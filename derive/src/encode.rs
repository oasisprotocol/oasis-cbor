use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Ident, Index, Member};

use crate::{
    common::{Codable, Field, Variant},
    util,
};

struct DeriveResult {
    enc_impl: TokenStream,
    opt_enc_impl: TokenStream,
    encode_as_map: bool,
}

impl DeriveResult {
    fn empty() -> Self {
        Self {
            enc_impl: quote!({}),
            opt_enc_impl: quote!({}),
            encode_as_map: false,
        }
    }
}

/// Derives the `Encode` trait.
pub fn derive(input: DeriveInput) -> TokenStream {
    let cbor_crate = util::cbor_crate_identifier();
    let enc = match Codable::from_derive_input(&input) {
        Ok(enc) => enc,
        Err(e) => return e.write_errors(),
    };

    let derived = match enc.data.as_ref() {
        darling::ast::Data::Enum(variants) => derive_enum(&enc, variants),
        darling::ast::Data::Struct(fields) => derive_struct(
            &enc.ident,
            enc.transparent.is_present(),
            enc.as_array.is_present(),
            false,
            fields,
            None,
        ),
    };

    let enc_ty_ident = &enc.ident;
    let (imp, ty, wher) = enc.generics.split_for_impl();
    let enc_impl = derived.enc_impl;
    let opt_enc_impl = derived.opt_enc_impl;

    // Implement the EncodeAsMap marker trait in case the type is known to encode as a map. This
    // allows operations to only operate on such types.
    let encode_as_map = if derived.encode_as_map {
        quote! {
            #[automatically_derived]
            impl #imp __cbor::EncodeAsMap for #enc_ty_ident #ty #wher {}
        }
    } else {
        quote!()
    };

    util::wrap_in_const(quote! {
        use #cbor_crate as __cbor;

        #[automatically_derived]
        impl #imp __cbor::Encode for #enc_ty_ident #ty #wher {
            fn into_cbor_value(self) -> __cbor::Value {
                #enc_impl
            }

             fn into_optional_cbor_value(self) -> Option<__cbor::Value> {
                #opt_enc_impl
             }
        }

        #encode_as_map
    })
}

fn derive_struct(
    ident: &Ident,
    transparent: bool,
    as_array: bool,
    unit_as_struct: bool,
    fields: darling::ast::Fields<&Field>,
    field_bindings: Option<Vec<Ident>>,
) -> DeriveResult {
    if fields.is_unit() && !unit_as_struct {
        return DeriveResult {
            enc_impl: quote! { __cbor::Value::Simple(__cbor::SimpleValue::NullValue) },
            opt_enc_impl: quote! { None },
            encode_as_map: false,
        };
    }

    if transparent {
        // Transparently forward the implementation to the underlying type. This is only valid for
        // newtype structs.
        if !fields.is_newtype() {
            ident
                .span()
                .unwrap()
                .error("transparent attribute can only be used for newtype structs".to_string())
                .emit();
            return DeriveResult::empty();
        }

        let encode_fn = quote_spanned!(ident.span()=> __cbor::Encode::into_cbor_value);
        let opt_encode_fn = quote_spanned!(ident.span()=> __cbor::Encode::into_optional_cbor_value);

        DeriveResult {
            enc_impl: quote!(#encode_fn(self.0)),
            opt_enc_impl: quote!(#opt_encode_fn(self.0)),
            encode_as_map: false, // We cannot be sure that the inner type encodes as map.
        }
    } else {
        // Process all fields and encode the structure as a map or array.
        let as_array = fields.is_tuple() || fields.is_newtype() || as_array;

        let field_map_items: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                if field.skip.is_present() {
                    // Skip serializing this field.
                    return quote!();
                }

                let field_ty = &field.ty;

                let field_binding = match field_bindings {
                    Some(ref field_bindings) => {
                        let field_ident = &field_bindings[i];
                        quote!( #field_ident )
                    }
                    None => field
                        .ident
                        .as_ref()
                        .map(|f| quote!( self.#f ))
                        .unwrap_or_else(|| {
                            let index = syn::Index::from(i);
                            quote!( self.#index )
                        }),
                };

                let field_value = if let Some(custom_encode_fn) = &field.serialize_with {
                    quote_spanned!(field_ty.span()=> __cbor::Encode::into_cbor_value(#custom_encode_fn(&#field_binding)))
                } else {
                    quote_spanned!(field_ty.span()=> __cbor::Encode::into_cbor_value(#field_binding))
                };

                if as_array {
                    // Output the fields as a CBOR array.
                    if field.skip_serializing_if.is_some() {
                        field
                            .ident
                            .span()
                            .unwrap()
                            .error("cannot use skip_serializing_if attribute in arrays".to_string())
                            .emit();
                        return quote!({});
                    }

                    quote! { fields.push(#field_value); }
                } else {
                    // Output the fields as a CBOR map.
                    let key = field.to_cbor_key_expr();

                    if field.optional.is_present() {
                        // If the field is optional then we can omit it when it is equal to the
                        // null value.
                        match &field.skip_serializing_if {
                            None => {
                                quote! {
                                    if let Some(value) = __cbor::Encode::into_optional_cbor_value(#field_binding) {
                                        fields.push((#key, value));
                                    }
                                }
                            }
                            Some(skip_serializing_if) => {
                                quote! {
                                    if !#skip_serializing_if(&#field_binding) {
                                        fields.push((#key, #field_value));
                                    }
                                }
                            }
                        }
                    } else {
                        // Otherwise always include it.
                        quote! { fields.push((#key, #field_value)); }
                    }
                }
            })
            .collect();

        let value_ty = if as_array {
            quote! { __cbor::Value::Array(fields) }
        } else {
            quote! { __cbor::Value::Map(fields) }
        };

        let num_fields = field_map_items.len();

        DeriveResult {
            enc_impl: quote! {
                let mut fields = ::std::vec::Vec::with_capacity(#num_fields);
                #(#field_map_items)*

                #value_ty
            },
            opt_enc_impl: quote!(match self.into_cbor_value() {
                __cbor::Value::Map(fields) if fields.is_empty() => None,
                v => Some(v),
            }),
            encode_as_map: !as_array,
        }
    }
}

fn derive_enum(enc: &Codable, variants: Vec<&Variant>) -> DeriveResult {
    // Make sure the transparent attribute cannot be used on an enum.
    if enc.transparent.is_present() {
        enc.ident
            .span()
            .unwrap()
            .error("cannot use transparent attribute on an enum".to_string())
            .emit();
        return DeriveResult::empty();
    }

    if variants.is_empty() {
        return DeriveResult {
            enc_impl: quote! { __cbor::Value::Simple(__cbor::SimpleValue::NullValue) },
            opt_enc_impl: quote! { Some(self.into_cbor_value()) },
            encode_as_map: false,
        };
    }

    let maybe_wrap_map = |variant: &Variant, key, inner| {
        if enc.untagged.is_present() || variant.missing.is_present() {
            // Untagged enum with just the inner type.
            quote!( #inner )
        } else if let Some(tag) = &enc.tag {
            // Internally tagged enum.
            let tag = tag.to_cbor_key_expr();

            quote!({
                let mut items = match #inner {
                    __cbor::Value::Map(items) => items,
                    _ => unreachable!(),
                };
                items.push((#tag, #key));
                __cbor::Value::Map(items)
            })
        } else {
            // Regular tagged enum where the tag is stored as a map key.
            quote!(__cbor::Value::Map(vec![(#key, #inner)]))
        }
    };

    let (match_arms, maybe_encode_as_map): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let key = variant.to_cbor_key_expr();

            let encode_fn = if enc.tag.is_some() {
                quote_spanned!(variant.ident.span()=> __cbor::EncodeAsMap::into_cbor_value_map)
            } else {
                quote_spanned!(variant.ident.span()=> __cbor::Encode::into_cbor_value)
            };

            if variant.skip.is_present() {
                // If we need to skip serializing this variant, serialize into undefined.
                return (quote! { Self::#variant_ident { .. } => __cbor::Value::Simple(__cbor::SimpleValue::Undefined), }, true);
            }

            if variant.embed.is_present() {
                // If we need to embed this variant, just serialize the embedded enum directly.
                if !variant.fields.is_newtype() {
                    variant.ident.span().unwrap().error("cannot use embed attribute on non-newtype variant".to_string()).emit();
                    return (quote!(), false);
                }
                if enc.tag.is_some() {
                    variant.ident.span().unwrap().error("cannot use embed attribute on internally tagged enum".to_string()).emit();
                    return (quote!(), false);
                }
                // TODO: It would be great if this somehow ensured that there was no overlap etc.
                return (quote! { Self::#variant_ident(inner) => #encode_fn(inner), }, true);
            }

            if variant.fields.is_unit() && !variant.as_struct.is_present() && enc.tag.is_none() {
                // For unit variants, just return the CBOR-encoded key or the discriminant if any.
                match variant.discriminant {
                    Some(ref expr) => {
                        let inner = quote!(#encode_fn(#expr));
                        (quote! { Self::#variant_ident => #inner, }, false)
                    }
                    None => (quote! { Self::#variant_ident => #key, }, false),
                }
            } else {
                // For others, encode as a map.
                if variant.fields.is_newtype() {
                    // Newtype variants map the key directly to the inner value as if transparent was used.
                    let inner = quote!(#encode_fn(inner));
                    let wrapper = maybe_wrap_map(variant, key, inner);

                    (quote! { Self::#variant_ident(inner) => #wrapper, }, true)
                } else {
                    // Generate field bindings as we need to destructure the enum variant.
                    let (bindings, idents): (Vec<_>, Vec<_>) = variant
                        .fields
                        .as_ref()
                        .iter()
                        .enumerate()
                        .map(|(i, field)| {
                            let pat = match field.ident {
                                Some(ref ident) => Member::Named(ident.clone()),
                                None => Member::Unnamed(Index {
                                    index: i as u32,
                                    span: variant_ident.span(),
                                }),
                            };
                            let ident = Ident::new(&format!("__a{}", i), variant_ident.span());
                            let binding = quote!( #pat: #ident, );

                            (binding, ident)
                        })
                        .unzip();

                    if enc.tag.is_some() && (variant.as_array.is_present() || variant.fields.is_tuple()) {
                        variant.ident.span().unwrap().error("cannot encode variant as array in internally tagged enums".to_string()).emit();
                        return (quote!(), false);
                    }

                    // Derive encoder and wrap it in a map.
                    let derived = derive_struct(
                        &variant.ident,
                        false,
                        variant.as_array.is_present(),
                        variant.as_struct.is_present(),
                        variant.fields.as_ref(),
                        Some(idents),
                    );
                    let inner = derived.enc_impl;
                    let wrapper = maybe_wrap_map(variant, key, quote!( {#inner} ));

                    (
                        quote! {
                            Self::#variant_ident { #(#bindings)* } => #wrapper,
                        },
                        true,
                    )
                }
            }
        })
        .unzip();

    // Check if all variants encode as a map.
    let all_encode_as_map = !enc.untagged.is_present() && maybe_encode_as_map.iter().all(|x| *x);

    DeriveResult {
        enc_impl: quote! {
            match self {
                #(#match_arms)*
            }
        },
        opt_enc_impl: quote! { Some(self.into_cbor_value()) },
        encode_as_map: all_encode_as_map,
    }
}
