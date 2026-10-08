//! Procedural macro attribute suite and compile-time architecture linter for SMA.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, ItemStruct};

mod rules;
mod scrooge;
mod taxonomy;
mod visitor;

use rules::*;
use scrooge::*;
use taxonomy::*;

#[proc_macro_attribute]
pub fn layer(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    match &item {
        Item::Struct(s) => {
            if let Err(e) = verify_suffix(&s.ident, "Layer", SMA_TAXO_001) {
                return e.to_compile_error().into();
            }
            if let Err(e) = inspect_struct_fields(&s.fields) {
                return e.to_compile_error().into();
            }
            quote! { #item }.into()
        }
        Item::Impl(item_impl) => {
            if let Err(e) = verify_layer_impl(item_impl) {
                return e.to_compile_error().into();
            }
            quote! { #item }.into()
        }
        _ => quote! { #item }.into(),
    }
}

#[proc_macro_attribute]
pub fn terminal(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    match &item {
        Item::Struct(s) => {
            if let Err(e) = verify_suffix(&s.ident, "Terminal", SMA_TAXO_002) {
                return e.to_compile_error().into();
            }
            if let Err(e) = inspect_struct_fields(&s.fields) {
                return e.to_compile_error().into();
            }
            quote! { #item }.into()
        }
        Item::Impl(item_impl) => {
            if let Err(e) = verify_terminal_impl(item_impl) {
                return e.to_compile_error().into();
            }
            quote! { #item }.into()
        }
        _ => quote! { #item }.into(),
    }
}

#[proc_macro_attribute]
pub fn port(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Trait(t) = &item {
        match verify_suffix(&t.ident, "Port", SMA_TAXO_003) {
            Ok(()) => quote! { #item }.into(),
            Err(e) => e.to_compile_error().into(),
        }
    } else {
        quote! { #item }.into()
    }
}

#[proc_macro_attribute]
pub fn adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Struct(s) = &item {
        match verify_suffix(&s.ident, "Adapter", SMA_TAXO_004) {
            Ok(()) => quote! { #item }.into(),
            Err(e) => e.to_compile_error().into(),
        }
    } else {
        quote! { #item }.into()
    }
}

#[proc_macro_attribute]
pub fn hub(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Struct(s) = &item {
        if let Err(e) = verify_suffix(&s.ident, "Hub", SMA_TAXO_005) {
            return e.to_compile_error().into();
        }
        if let Err(e) = inspect_struct_fields(&s.fields) {
            return e.to_compile_error().into();
        }
    }
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn token(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    if let Err(e) = verify_suffix(&item.ident, "Token", SMA_TAXO_006) {
        return e.to_compile_error().into();
    }
    if let Err(e) = verify_transparent(&item.attrs, item.ident.span()) {
        return e.to_compile_error().into();
    }
    let assertion = synthesize_register_size_assertion(&item.ident);
    quote! {
        #item
        #assertion
    }
    .into()
}

#[proc_macro_attribute]
pub fn id(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    if let Err(e) = verify_suffix(&item.ident, "Id", SMA_TAXO_007) {
        return e.to_compile_error().into();
    }
    if let Err(e) = verify_transparent(&item.attrs, item.ident.span()) {
        return e.to_compile_error().into();
    }
    let assertion = synthesize_register_size_assertion(&item.ident);
    quote! {
        #item
        #assertion
    }
    .into()
}

#[proc_macro_attribute]
pub fn blackboard(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    if let Err(e) = verify_blackboard_suffix(&item.ident) {
        return e.to_compile_error().into();
    }
    if let Err(e) = verify_cache_alignment(&item.attrs, item.ident.span()) {
        return e.to_compile_error().into();
    }
    if let Err(e) = inspect_struct_fields(&item.fields) {
        return e.to_compile_error().into();
    }
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn entity(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn value_object(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Struct(s) = &item {
        match inspect_struct_fields(&s.fields) {
            Ok(()) => quote! { #item }.into(),
            Err(e) => e.to_compile_error().into(),
        }
    } else {
        quote! { #item }.into()
    }
}

#[proc_macro_attribute]
pub fn event(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Struct(s) = &item {
        match inspect_struct_fields(&s.fields) {
            Ok(()) => quote! { #item }.into(),
            Err(e) => e.to_compile_error().into(),
        }
    } else {
        quote! { #item }.into()
    }
}

#[proc_macro_attribute]
pub fn data(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    quote! { #item }.into()
}
