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
    encode_as_map: bool,
}

impl DeriveResult {
    fn empty() -> Self {
        Self {
            enc_impl: quote!({}),
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
            enc.transparent.is_some(),
            enc.as_array.is_some(),
            fields,
            None,
        ),
    };

    let enc_ty_ident = &enc.ident;
    let (imp, ty, wher) = enc.generics.split_for_impl();
    let enc_impl = derived.enc_impl;

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
        }

        #encode_as_map
    })
}

fn derive_struct(
    ident: &Ident,
    transparent: bool,
    as_array: bool,
    fields: darling::ast::Fields<&Field>,
    mut field_bindings: Option<Vec<Ident>>,
) -> DeriveResult {
    if fields.is_unit() {
        return DeriveResult {
            enc_impl: quote! { __cbor::Value::Simple(__cbor::SimpleValue::NullValue) },
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

        DeriveResult {
            enc_impl: quote!(#encode_fn(self.0)),
            encode_as_map: false, // We cannot be sure that the inner type encodes as map.
        }
    } else {
        // Process all fields and encode the structure as a map or array.
        let as_array = fields.is_tuple() || fields.is_newtype() || as_array;

        // Sort fields by their CBOR keys. This makes sure that fields are ordered correctly even
        // when encoded into intermediate cbor::Value types (since writer also sorts).
        let mut fields = fields.fields;
        if !as_array {
            // First sort any field bindings.
            field_bindings = field_bindings.map(|bindings| {
                let mut field_bindings_idx: Vec<_> = bindings.iter().enumerate().collect();
                field_bindings_idx.sort_by_key(|(i, _)| fields[*i].to_cbor_key());
                field_bindings_idx
                    .into_iter()
                    .map(|(_, f)| f.clone())
                    .collect()
            });
            // Then sort the fields.
            fields.sort_by_key(|f| f.to_cbor_key());
        }

        let field_map_items: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                // Perform early validation for option compatibility.
                if field.optional.is_some() && as_array {
                    field
                        .ident
                        .span()
                        .unwrap()
                        .error("cannot use optional attribute in arrays".to_string())
                        .emit();
                    return quote!({});
                }

                if field.skip.is_some() {
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

                let encode_fn = quote_spanned!(field_ty.span()=> __cbor::Encode::into_cbor_value);

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

                    let field_value = quote!(#encode_fn(#field_binding));

                    quote! { fields.push(#field_value); }
                } else {
                    // Output the fields as a CBOR map.
                    let key = field.to_cbor_key_expr();
                    let field_value = quote!(#encode_fn(#field_binding) );

                    if field.optional.is_some() {
                        // If the field is optional then we can omit it when it is equal to the
                        // null value.
                        match &field.skip_serializing_if {
                            None => {
                                quote! {
                                    let fv = #field_value;
                                    if fv != __cbor::Value::Simple(__cbor::SimpleValue::NullValue) {
                                        fields.push((#key, fv));
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
            encode_as_map: !as_array,
        }
    }
}

fn derive_enum(enc: &Codable, variants: Vec<&Variant>) -> DeriveResult {
    // Make sure the transparent attribute cannot be used on an enum.
    if enc.transparent.is_some() {
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
            encode_as_map: false,
        };
    }

    let maybe_wrap_map = |key, inner| {
        if enc.untagged.is_some() {
            // Untagged enum with just the inner type.
            quote!( #inner )
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

            let encode_fn = quote_spanned!(variant.ident.span()=> __cbor::Encode::into_cbor_value);

            if variant.skip.is_some() {
                // If we need to skip serializing this variant, serialize into undefined.
                return (quote! { Self::#variant_ident { .. } => __cbor::Value::Simple(__cbor::SimpleValue::Undefined), }, true);
            }

            if variant.fields.is_unit() {
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
                    let wrapper = maybe_wrap_map(key, inner);

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

                    // Derive encoder and wrap it in a map.
                    let derived = derive_struct(
                        &variant.ident,
                        false,
                        variant.as_array.is_some(),
                        variant.fields.as_ref(),
                        Some(idents),
                    );
                    let inner = derived.enc_impl;
                    let wrapper = maybe_wrap_map(key, quote!( {#inner} ));

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
    let all_encode_as_map = enc.untagged.is_none() && maybe_encode_as_map.iter().all(|x| *x);

    DeriveResult {
        enc_impl: quote! {
            match self {
                #(#match_arms)*
            }
        },
        encode_as_map: all_encode_as_map,
    }
}
