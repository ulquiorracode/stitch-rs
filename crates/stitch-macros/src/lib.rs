//! Procedural macro attribute suite and compile-time architecture linter for SMA.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, parse_macro_input};

macro_rules! define_passthrough_marker {
    ($name:ident, $suffix:expr, $err_code:expr) => {
        #[proc_macro_attribute]
        pub fn $name(_args: TokenStream, input: TokenStream) -> TokenStream {
            let item = parse_macro_input!(input as Item);
            let ident_opt = match &item {
                Item::Struct(s) => Some(&s.ident),
                Item::Enum(e) => Some(&e.ident),
                Item::Trait(t) => Some(&t.ident),
                _ => None,
            };

            if let Some(ident) = ident_opt {
                let name_str = ident.to_string();
                if !name_str.ends_with($suffix) {
                    let err = syn::Error::new(
                        ident.span(),
                        format!(
                            "[{}] Suffix violation: Item `{}` decorated with #[stitch::{}] must have suffix `{}`.",
                            $err_code, name_str, stringify!($name), $suffix
                        ),
                    );
                    let compile_err = err.to_compile_error();
                    return quote! {
                        #compile_err
                        #item
                    }
                    .into();
                }
            }

            quote! { #item }.into()
        }
    };
}

define_passthrough_marker!(layer, "Layer", "SMA-TAXO-001");
define_passthrough_marker!(terminal, "Terminal", "SMA-TAXO-002");
define_passthrough_marker!(port, "Port", "SMA-TAXO-003");
define_passthrough_marker!(adapter, "Adapter", "SMA-TAXO-004");
define_passthrough_marker!(hub, "Hub", "SMA-TAXO-005");
define_passthrough_marker!(token, "Token", "SMA-TAXO-006");
define_passthrough_marker!(id, "Id", "SMA-TAXO-007");

#[proc_macro_attribute]
pub fn blackboard(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    if let Item::Struct(s) = &item {
        let name_str = s.ident.to_string();
        if !name_str.ends_with("Context") && !name_str.ends_with("Blackboard") {
            let err = syn::Error::new(
                s.ident.span(),
                format!(
                    "[SMA-TAXO-008] Suffix violation: Struct `{}` decorated with #[stitch::blackboard] must have suffix `Context` or `Blackboard`.",
                    name_str
                ),
            );
            let compile_err = err.to_compile_error();
            return quote! {
                #compile_err
                #item
            }
            .into();
        }
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
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn event(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn data(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    quote! { #item }.into()
}
