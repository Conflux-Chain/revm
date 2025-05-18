extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, GenericParam, ItemFn};

#[proc_macro_attribute]
pub fn use_dyn_host(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let vis = &input.vis;
    let sig = &mut input.sig;
    let block = &input.block;

    // Remove the Host generic parameter
    sig.generics.params = sig
        .generics
        .params
        .clone()
        .into_iter()
        .filter(|param| {
            if let GenericParam::Type(type_param) = param {
                type_param.ident != "H"
            } else {
                true
            }
        })
        .collect();

    // Add a lifetime parameter if it doesn't exist
    if !sig
        .generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Lifetime(_)))
    {
        sig.generics.params.push(parse_quote!('a));
    }

    // Modify the host parameter type
    for input in sig.inputs.iter_mut() {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Type::Reference(type_reference) = &mut *pat_type.ty {
                if let syn::Type::Path(type_path) = &mut *type_reference.elem {
                    if type_path
                        .path
                        .segments
                        .last()
                        .map_or(false, |seg| seg.ident == "H")
                    {
                        *pat_type.ty = parse_quote!(&mut (dyn Host + 'a));
                    }
                }
            }
        }
    }

    quote! {
        #vis #sig #block
    }
    .into()
}
