#![feature(proc_macro_diagnostic)]
#![deny(rust_2018_idioms)]

mod common;
mod decode;
mod encode;
mod util;

use proc_macro::TokenStream;

/// Derives the `Decode` trait.
#[proc_macro_derive(Decode, attributes(cbor))]
pub fn decode_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    decode::derive(input).into()
}

/// Derives the `Encode` trait.
#[proc_macro_derive(Encode, attributes(cbor))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    encode::derive(input).into()
}
