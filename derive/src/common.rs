//! Common definitions for both encoder and decoder.
use darling::{util::Flag, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use sk_cbor::values::IntoCborValue;
use syn::{Expr, ExprPath, Generics, Ident, Type};

#[derive(FromDeriveInput)]
#[darling(supports(any), attributes(cbor))]
pub struct Codable {
    pub ident: Ident,
    pub generics: Generics,

    pub data: darling::ast::Data<Variant, Field>,

    #[darling(default, rename = "transparent")]
    pub transparent: Flag,

    #[darling(default, rename = "untagged")]
    pub untagged: Flag,

    #[darling(default, rename = "as_array")]
    pub as_array: Flag,
}

#[derive(FromField)]
#[darling(attributes(cbor))]
pub struct Field {
    pub ident: Option<Ident>,
    pub ty: Type,

    #[darling(default, rename = "rename")]
    pub rename: Option<String>,

    #[darling(default, rename = "optional")]
    pub optional: Flag,

    #[darling(default, rename = "default")]
    pub default: Flag,

    #[darling(default, rename = "skip_serializing_if")]
    pub skip_serializing_if: Option<String>,
}

impl Field {
    pub fn to_cbor_key_expr(&self) -> TokenStream {
        // TODO: Support non-string keys.
        let key = self.ident.as_ref().unwrap().to_string();
        let key = self.rename.as_ref().unwrap_or(&key);
        quote!( __cbor::values::IntoCborValue::into_cbor_value(#key) )
    }

    pub fn to_cbor_key(&self) -> sk_cbor::Value {
        // TODO: Support non-string keys.
        let key = self.ident.as_ref().unwrap().to_string();
        let key = self.rename.as_ref().unwrap_or(&key);
        key.clone().into_cbor_value()
    }

    pub fn skip_serializing_if_expr(&self) -> Option<Result<ExprPath, syn::Error>> {
        self.skip_serializing_if
            .as_ref()
            .map(|s| syn::parse_str(&s))
    }
}

#[derive(FromVariant)]
#[darling(attributes(cbor))]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Option<Expr>,
    pub fields: darling::ast::Fields<Field>,

    #[darling(default, rename = "rename")]
    pub rename: Option<String>,

    #[darling(default, rename = "as_array")]
    pub as_array: Flag,
}

impl Variant {
    pub fn to_cbor_key_expr(&self) -> TokenStream {
        // TODO: Support non-string keys.
        let key = self.ident.to_string();
        let key = self.rename.as_ref().unwrap_or(&key);
        quote!( __cbor::values::IntoCborValue::into_cbor_value(#key) )
    }
}
