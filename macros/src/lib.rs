extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn measured(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_visibility = &input_fn.vis;
    let fn_block = &input_fn.block;
    let inputs = &input_fn.sig.inputs;
    let attrs = &input_fn.attrs;

    let output = quote! {
        #(#attrs)*
        #fn_visibility fn #fn_name(mut performance: ResMut<Performance>, #inputs) {
            let start = std::time::Instant::now();
            #fn_block
            let duration = start.elapsed();
            performance.add_duration(stringify!(#fn_name), duration);
        }
    };

    TokenStream::from(output)
}
