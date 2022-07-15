use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::Ident;

pub fn wrap_in_const(tokens: TokenStream) -> TokenStream {
    quote! {
        #[doc(hidden)]
        const _: () = {
            #tokens
        };
    }
}

pub fn cbor_crate_identifier() -> TokenStream {
    let found_crate = crate_name("oasis-cbor");

    match found_crate {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( ::#ident )
        }
        Err(proc_macro_crate::Error::CrateNotFound { .. }) => {
            let ident = Ident::new("oasis_cbor", Span::call_site());
            quote!(#ident)
        }
        Err(_) => panic!("oasis-cbor should be imported in `Cargo.toml`"),
    }
}
