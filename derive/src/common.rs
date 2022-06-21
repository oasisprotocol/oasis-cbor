//! Common definitions for both encoder and decoder.
use darling::{util::Flag, Error, FromDeriveInput, FromField, FromVariant, Result};
use proc_macro2::TokenStream;
use quote::quote;
use sk_cbor::values::IntoCborValue;
use syn::{Expr, Generics, Ident, Lit, Path, Type};

#[derive(FromDeriveInput)]
#[darling(supports(any), attributes(cbor))]
#[darling(and_then = "Self::exclusive_default_and_with_default")]
pub struct Codable {
    pub ident: Ident,
    pub generics: Generics,

    pub data: darling::ast::Data<Variant, Field>,

    #[darling(rename = "transparent")]
    pub transparent: Flag,

    #[darling(rename = "untagged")]
    pub untagged: Flag,

    #[darling(rename = "as_array")]
    pub as_array: Flag,

    #[darling(rename = "no_default")]
    pub no_default: Flag,

    #[darling(rename = "with_default")]
    pub with_default: Flag,

    #[darling(rename = "allow_unknown")]
    pub allow_unknown: Flag,
}

impl Codable {
    fn exclusive_default_and_with_default(self) -> Result<Self> {
        if self.no_default.is_present() && self.with_default.is_present() {
            Err(Error::custom("Cannot set no_default and with_default")
                .with_span(&self.with_default))
        } else {
            Ok(self)
        }
    }
}

pub enum Key {
    String(String),
    Integer(u64),
}

impl Key {
    fn to_cbor_key_expr(&self) -> TokenStream {
        match self {
            Key::String(ref v) => {
                quote!( __cbor::values::IntoCborValue::into_cbor_value(#v) )
            }
            Key::Integer(ref v) => {
                quote!( __cbor::values::IntoCborValue::into_cbor_value(#v) )
            }
        }
    }

    fn to_cbor_key(&self) -> sk_cbor::Value {
        match self {
            Key::String(ref v) => v.clone().into_cbor_value(),
            Key::Integer(ref v) => v.into_cbor_value(),
        }
    }
}

impl darling::FromMeta for Key {
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(Self::String(value.to_string()))
    }

    fn from_value(value: &Lit) -> darling::Result<Self> {
        (match *value {
            Lit::Str(ref s) => Self::from_string(&s.value()),
            Lit::Int(ref s) => Ok(Self::Integer(s.base10_parse().unwrap())),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        })
        .map_err(|e| e.with_span(value))
    }
}

#[derive(FromField)]
#[darling(attributes(cbor))]
pub struct Field {
    pub ident: Option<Ident>,
    pub ty: Type,

    #[darling(rename = "rename")]
    pub rename: Option<Key>,

    #[darling(rename = "optional")]
    pub optional: Flag,

    #[darling(rename = "skip")]
    pub skip: Flag,

    #[darling(rename = "skip_serializing_if")]
    pub skip_serializing_if: Option<Path>,

    #[darling(rename = "serialize_with")]
    pub serialize_with: Option<Path>,

    #[darling(rename = "deserialize_with")]
    pub deserialize_with: Option<Path>,
}

impl Field {
    pub fn to_cbor_key_expr(&self) -> TokenStream {
        self.rename
            .as_ref()
            .map(Key::to_cbor_key_expr)
            .unwrap_or_else(|| {
                // No explicit rename, use identifier name.
                let ident = self.ident.as_ref().unwrap().to_string();
                quote!( __cbor::values::IntoCborValue::into_cbor_value(#ident) )
            })
    }

    pub fn to_cbor_key(&self) -> sk_cbor::Value {
        self.rename
            .as_ref()
            .map(Key::to_cbor_key)
            .unwrap_or_else(|| {
                // No explicit rename, use identifier name.
                let ident = self.ident.as_ref().unwrap().to_string();
                ident.into_cbor_value()
            })
    }
}

#[derive(FromVariant)]
#[darling(attributes(cbor))]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Option<Expr>,
    pub fields: darling::ast::Fields<Field>,

    #[darling(rename = "rename")]
    pub rename: Option<Key>,

    #[darling(rename = "as_array")]
    pub as_array: Flag,

    #[darling(rename = "as_struct")]
    pub as_struct: Flag,

    #[darling(rename = "skip")]
    pub skip: Flag,

    #[darling(rename = "embed")]
    pub embed: Flag,

    #[darling(rename = "allow_unknown")]
    pub allow_unknown: Flag,
}

impl Variant {
    pub fn to_cbor_key_expr(&self) -> TokenStream {
        self.rename
            .as_ref()
            .map(Key::to_cbor_key_expr)
            .unwrap_or_else(|| {
                // No explicit rename, use identifier name.
                let ident = self.ident.to_string();
                quote!( __cbor::values::IntoCborValue::into_cbor_value(#ident) )
            })
    }
}
