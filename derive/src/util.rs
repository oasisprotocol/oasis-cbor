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
    let found_crate =
        crate_name("oasis-cbor").expect("oasis-cbor should be imported in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(::oasis_cbor),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( ::#ident )
        }
    }
}
