use proc_macro::{TokenStream, TokenTree};

#[proc_macro]
pub fn declare_tuple_helpers(content: TokenStream) -> TokenStream {
    content
}
