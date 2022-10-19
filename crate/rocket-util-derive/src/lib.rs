#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    proc_macro::TokenStream,
    quote::quote,
    syn::*,
};

mod html;

/// Generates HTML code. Similar to the macros from [`horrorshow`](https://docs.rs/horrorshow) with the following differences:
///
/// * This macro expands to an expression of type `RawHtml<String>` rather than `FnRenderer`. This also means that any expressions used in the macro are eagerly evaluated.
/// * This macro supports `@let`.
/// * This macro supports `@match`. Don't use commas to separate the match arms.
/// * This macro also supports `@unimplemented` and `@unreachable` to work around type inference issues with using `: unimplemented!();` or `: unreachable!();`.
/// * HTML attributes with hyphens should be written with underscores instead, e.g. write `data_foo` instead of `data-foo`.
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    html::mac(input, false)
}

#[doc(hidden)]
#[proc_macro]
pub fn html_internal(input: TokenStream) -> TokenStream {
    html::mac(input, true)
}

#[proc_macro_derive(CsrfForm)]
pub fn derive_csrf_form(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = input.ident;
    TokenStream::from(quote! {
        impl ::rocket_util::CsrfForm for #ty {
            fn csrf(&self) -> &::std::string::String { &self.csrf }
        }
    })
}

#[proc_macro_derive(Error)]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = input.ident;
    TokenStream::from(quote! {
        impl<'r> ::rocket_util::rocket::response::Responder<'r, 'static> for #ty {
            fn respond_to(self, request: &'r ::rocket_util::rocket::Request<'_>) -> ::rocket_util::rocket::response::Result<'static> {
                ::rocket_util::rocket::response::Responder::respond_to(::rocket_util::Error(self), request)
            }
        }
    })
}
