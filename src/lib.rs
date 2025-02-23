use proc_macro::TokenStream;

mod impl_body;
use impl_body::body;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive_custom_debug(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    body(&ast).into()
}
