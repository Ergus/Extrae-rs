extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

struct ProfileAttr {
    level: u32,
    name: String,
    value: u16,
}


impl ProfileAttr {
    fn new(name: String) -> Self
    {
        Self { level: 0, name , value: 0 }
    }

    fn parse(&mut self, meta: syn::meta::ParseNestedMeta) -> syn::Result<()>
    {
        if meta.path.is_ident("level") {
            self.level = meta.value()?.parse::<syn::LitInt>()?.base10_parse::<u32>()?;
            Ok(())
        } else if meta.path.is_ident("name") {
            self.name = meta.value()?.parse::<syn::LitStr>()?.value();
            Ok(())
        } else if meta.path.is_ident("value") {
            self.value = meta.value()?.parse::<syn::LitInt>()?.base10_parse::<u16>()?;
            Ok(())
        } else {
            Err(meta.error("unsupported profile property"))
        }
    }
}

#[proc_macro_attribute]
pub fn extrae_profile(args: TokenStream, item: TokenStream) -> TokenStream
{
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = input_fn.sig.ident.to_string();
    let fn_block = input_fn.block;
    let fn_vis = input_fn.vis;
    let fn_sig = input_fn.sig;

    // https://docs.rs/syn/latest/syn/meta/fn.parser.html#example
    let mut attrs = ProfileAttr::new(fn_name);
    let profile_parser = syn::meta::parser(|meta| attrs.parse(meta));
    parse_macro_input!(args with profile_parser);

    let fn_name = attrs.name.clone();
    let value = attrs.value;

    let expanded = quote! {
        #fn_vis #fn_sig {
            extrae_rs::instrument_function!(#fn_name, #value);
            #fn_block
        }
    };

    TokenStream::from(expanded)
}












