use {
    proc_macro::TokenStream,
    quote::quote,
    syn::*,
};

#[proc_macro_derive(Error)]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = input.ident;
    TokenStream::from(quote! {
        impl<'r> ::rocket_util::rocket::response::Responder<'r, 'static> for #ty {
            fn respond_to(self, request: &'r ::rocket_util::rocket::Request<'_>) -> ::rocket_util::rocket::response::Result<'static> {
                //TODO also print Display?
                ::rocket_util::rocket::response::Responder::respond_to(::rocket_util::rocket::response::Debug(self), request)
            }
        }
    })
}
