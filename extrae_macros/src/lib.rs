extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn profile(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = input_fn.sig.ident.to_string();
    let fn_block = input_fn.block;
    let fn_vis = input_fn.vis;
    let fn_sig = input_fn.sig;

    let expanded = quote! {
        #fn_vis #fn_sig {
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            let id: u16 = *PROFILER_ONCE.get_or_init(|| extrae_rs::GlobalInfo::register_event_name(#fn_name, file!(), line!(), 0));
            crate::ThreadInfo::emplace_event(id, 1);
            let result = #fn_block;
            crate::ThreadInfo::emplace_event(id, 0);
            result
        }
    };

    TokenStream::from(expanded)
}
