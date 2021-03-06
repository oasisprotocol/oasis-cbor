use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Ident, Index, Member};

use crate::{
    common::{Codable, Field, Variant},
    util,
};

/// Derives the `Decode` trait.
pub fn derive(input: DeriveInput) -> TokenStream {
    let cbor_crate = util::cbor_crate_identifier();
    let dec = match Codable::from_derive_input(&input) {
        Ok(dec) => dec,
        Err(e) => return e.write_errors(),
    };

    let (dec_impl, include_dec_default) = match dec.data.as_ref() {
        darling::ast::Data::Enum(variants) => (derive_enum(&dec, variants), false),
        darling::ast::Data::Struct(fields) => {
            let inner = derive_struct(
                &dec.ident,
                dec.transparent.is_present(),
                dec.as_array.is_present(),
                dec.allow_unknown.is_present(),
                fields,
                quote!(Self),
            );
            (quote!(Ok({ #inner })), true)
        }
    };

    let dec_default_impl =
        if (include_dec_default && !dec.no_default.is_present()) || dec.with_default.is_present() {
            quote! {
                fn try_default() -> ::std::result::Result<Self, __cbor::DecodeError> {
                    Ok(Default::default())
                }
            }
        } else {
            quote!()
        };

    let dec_ty_ident = &dec.ident;
    let (imp, ty, wher) = dec.generics.split_for_impl();

    util::wrap_in_const(quote! {
        use #cbor_crate as __cbor;

        #[automatically_derived]
        impl #imp __cbor::Decode for #dec_ty_ident #ty #wher {
            #dec_default_impl

            fn try_from_cbor_value(value: __cbor::Value) -> ::std::result::Result<Self, __cbor::DecodeError> {
                #dec_impl
            }
        }
    })
}

fn field_decode_fn(field: &Field) -> TokenStream {
    if let Some(custom_decode_fn) = &field.deserialize_with {
        quote!((|v| __cbor::Decode::try_from_cbor_value_default(v).and_then(#custom_decode_fn)))
    } else {
        quote!(__cbor::Decode::try_from_cbor_value_default)
    }
}

fn derive_struct(
    ident: &Ident,
    transparent: bool,
    as_array: bool,
    allow_unknown: bool,
    fields: darling::ast::Fields<&Field>,
    self_ty: TokenStream,
) -> TokenStream {
    if transparent {
        // Transparently forward the implementation to the underlying type. This is only valid for
        // newtype structs.
        let decode_fn = quote_spanned!(ident.span()=> __cbor::Decode::try_from_cbor_value_default);
        quote!(Self(#decode_fn(value)?))
    } else {
        // Process all fields and decode the structure as a map or array.
        let as_array = fields.is_tuple() || fields.is_newtype() || as_array;

        let (extract_value, field_map_items): (_, Vec<_>) = if as_array {
            // Fields represented as an array.
            let extract_value = quote! {
                match value {
                    __cbor::Value::Array(array) => array,
                    _ => return Err(__cbor::DecodeError::UnexpectedType),
                }
            };

            let field_map_items = fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_ty = &field.ty;
                    let field_ident = match field.ident {
                        Some(ref ident) => Member::Named(ident.clone()),
                        None => Member::Unnamed(Index {
                            index: i as u32,
                            span: field_ty.span(),
                        }),
                    };

                    let field_value = if field.skip.is_present() {
                        // If the field should be skipped, always use Default::default() as value.
                        quote_spanned!(field_ty.span()=> ::std::default::Default::default())
                    } else {
                        let decode_fn = field_decode_fn(&field);
                        quote!(#decode_fn(it.next().ok_or(__cbor::DecodeError::MissingField)?)?)
                    };

                    quote! { #field_ident: #field_value }
                })
                .collect();

            (extract_value, field_map_items)
        } else if fields.is_unit() {
            // This is a unit struct with no fields.
            (quote! {Vec::<()>::new()}, vec![])
        } else {
            // Field represented as a map.
            let extract_value = quote! {
                match value {
                    // Sort map entries by CBOR keys.
                    __cbor::Value::Map(mut map) => { map.sort(); map },
                    _ => return Err(__cbor::DecodeError::UnexpectedType),
                }
            };

            // Sort struct fields by their CBOR keys to make destructure_cbor_map_peek_value_strict work.
            let mut fields = fields.fields;
            fields.sort_by(|a, b| a.to_cbor_key().partial_cmp(&b.to_cbor_key()).unwrap());

            let field_map_items = fields
                .iter()
                .map(|field| {
                    let field_ident = field.ident.as_ref().unwrap();
                    let field_ty = &field.ty;
                    let key = field.to_cbor_key_expr();

                    let field_value = if field.skip.is_present() {
                        // If the field should be skipped, always use Default::default() as value.
                        quote_spanned!(field_ty.span()=> ::std::default::Default::default())
                    } else {
                        let decode_fn = field_decode_fn(&field);
                        let destruct_fn = quote_spanned!(field_ty.span()=>
                            __cbor::macros::destructure_cbor_map_peek_value_strict);
                        let field_value = quote!({
                            let v: Option<__cbor::Value> = #destruct_fn(&mut it, #key)?;
                            #decode_fn(v.unwrap_or(__cbor::Value::Simple(__cbor::SimpleValue::NullValue)))?
                        });

                        field_value
                    };

                    quote! { #field_ident: #field_value }
                })
                .collect();

            (extract_value, field_map_items)
        };

        let handle_unknown_fields = if allow_unknown {
            quote!()
        } else {
            quote! {
                if it.next().is_some() {
                    return Err(__cbor::DecodeError::UnknownField);
                }
            }
        };

        quote! {
            let fields = #extract_value;
            let mut it = fields.into_iter().peekable();

            let v = #self_ty {
                #(#field_map_items),*
            };

            #handle_unknown_fields

            v
        }
    }
}

fn derive_enum(dec: &Codable, variants: Vec<&Variant>) -> TokenStream {
    // Make sure the transparent attribute cannot be used on an enum.
    if dec.transparent.is_present() {
        dec.ident
            .span()
            .unwrap()
            .error("cannot use transparent attribute on an enum".to_string())
            .emit();
        return quote!({});
    }

    // Make sure decoding of untagged enums is not supported.
    if dec.untagged.is_present() {
        dec.ident
            .span()
            .unwrap()
            .error("cannot derive decoder for untagged enum".to_string())
            .emit();
        return quote!({});
    }

    if variants.is_empty() {
        return quote! { Self };
    }

    // Generate decoders for all unit variants.
    let unit_decoders = variants.iter().filter_map(|variant| {
        if !variant.fields.is_unit() || variant.as_struct.is_present() || dec.tag.is_some() {
            return None;
        }
        if variant.skip.is_present() {
            return None;
        }
        if variant.embed.is_present() {
            return None;
        }

        let discriminant = match variant.discriminant {
            Some(ref expr) => {
                let encoder_fn =
                    quote_spanned!(variant.ident.span()=> __cbor::Encode::into_cbor_value);
                quote!(#encoder_fn(#expr))
            }
            None => variant.to_cbor_key_expr(),
        };

        let variant_ident = &variant.ident;

        Some(quote! {
            if value == #discriminant {
                return Ok(Self::#variant_ident);
            }
        })
    });

    // Generate decoders for all non-unit variants.
    let mut have_missing_variant = false;
    let non_unit_decoders: Vec<_> = variants
        .iter()
        .filter_map(|variant| {
            if variant.fields.is_unit() && !variant.as_struct.is_present() && dec.tag.is_none() {
                return None;
            }
            if variant.skip.is_present() {
                return None;
            }
            if variant.embed.is_present() {
                return None;
            }

            let variant_ident = &variant.ident;
            let key = variant.to_cbor_key_expr();

            let decoder = if variant.fields.is_newtype() {
                // Newtype variants map the key directly to the inner value as if transparent was used.
                let decode_fn =
                    quote_spanned!(variant.ident.span()=> __cbor::Decode::try_from_cbor_value_default);
                quote!(Self::#variant_ident(#decode_fn(value)?))
            } else {
                if dec.tag.is_some() && (variant.as_array.is_present() || variant.fields.is_tuple()) {
                    variant
                        .ident
                        .span()
                        .unwrap()
                        .error("cannot encode variant as array in internally tagged enums".to_string())
                        .emit();
                    return None;
                }

                // Derive inner decoder.
                let inner = derive_struct(
                    &variant.ident,
                    false,
                    variant.as_array.is_present(),
                    variant.allow_unknown.is_present(),
                    variant.fields.as_ref(),
                    quote!(Self::#variant_ident),
                );
                quote!({ #inner })
            };

            if variant.missing.is_present() {
                // This is a fallback variant for when the tag is missing.
                if have_missing_variant {
                    variant
                        .ident
                        .span()
                        .unwrap()
                        .error("multiple variants specify the missing attribute".to_string())
                        .emit();
                    return None;
                }
                have_missing_variant = true;

                Some(quote! {
                    if key == #key || key == __cbor::Value::Simple(__cbor::SimpleValue::Undefined) {
                        return Ok(#decoder);
                    }
                })
            } else {
                Some(quote! {
                    if key == #key {
                        return Ok(#decoder);
                    }
                })
            }
        })
        .collect();

    // Generate decoders for all embedded variants.
    let embedded_decoders: Vec<_> = variants
        .iter()
        .filter_map(|variant| {
            if variant.skip.is_present() {
                return None;
            }
            if !variant.embed.is_present() {
                return None;
            }

            if !variant.fields.is_newtype() {
                variant
                    .ident
                    .span()
                    .unwrap()
                    .error("cannot use embed attribute on non-newtype variant".to_string())
                    .emit();
                return None;
            }

            if dec.tag.is_some() {
                variant
                    .ident
                    .span()
                    .unwrap()
                    .error("cannot use embed attribute on internally tagged enum".to_string())
                    .emit();
                return None;
            }

            let variant_ident = &variant.ident;
            let decode_fn =
                quote_spanned!(variant.ident.span()=> __cbor::Decode::try_from_cbor_value_default);

            // TODO: Can we get rid of clone?
            Some(quote! {
                if let Ok(result) = #decode_fn(value.clone()) {
                    return Ok(Self::#variant_ident(result));
                }
            })
        })
        .collect();

    // Handle internally tagged enums.
    if let Some(tag) = &dec.tag {
        let tag = tag.to_cbor_key_expr();

        return quote! {
            match value {
                __cbor::Value::Map(mut map) => {
                    let key = if let Some((index, _)) = map
                        .iter()
                        .enumerate()
                        .find(|(_, v)| v.0 == #tag)
                    {
                        map.remove(index).1
                    } else {
                        __cbor::Value::Simple(__cbor::SimpleValue::Undefined)
                    };
                    let value = __cbor::Value::Map(map);

                    #(#non_unit_decoders)*

                    Err(__cbor::DecodeError::UnknownField)
                },
                _ => Err(__cbor::DecodeError::UnexpectedType)
            }
        };
    }

    // In case there are no non-unit decoders, just omit the match.
    if non_unit_decoders.is_empty() {
        quote! {
            #(#unit_decoders)*
            #(#embedded_decoders)*

            Err(__cbor::DecodeError::UnknownField)
        }
    } else {
        let embedded_decoders_map = if !embedded_decoders.is_empty() {
            quote! {
                let value = __cbor::Value::Map(vec![(key, value)]);
                #(#embedded_decoders)*
            }
        } else {
            quote!()
        };

        quote! {
            match value {
                __cbor::Value::Map(mut map) => {
                    if map.len() != 1 {
                        return Err(__cbor::DecodeError::UnknownField);
                    }

                    let (key, value) = map.pop().unwrap();

                    #(#non_unit_decoders)*
                    #embedded_decoders_map

                    Err(__cbor::DecodeError::UnknownField)
                },
                _ => {
                    #(#unit_decoders)*
                    #(#embedded_decoders)*

                    Err(__cbor::DecodeError::UnknownField)
                }
            }
        }
    }
}
