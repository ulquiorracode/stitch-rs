//! Suffix validation and trait implementation verifiers for SMA components.

use crate::rules::*;
use crate::visitor::HotPathAstVisitor;
use syn::spanned::Spanned;
use syn::{Error, Ident, ImplItem, ItemImpl, Receiver, Result, ReturnType, Signature};

/// Verifies that an identifier ends with the mandatory architectural suffix.
pub fn verify_suffix(ident: &Ident, expected_suffix: &str, error_code: &str) -> Result<()> {
    let name = ident.to_string();
    if !name.ends_with(expected_suffix) {
        return Err(Error::new(
            ident.span(),
            format!(
                "[{error_code}] Architectural violation: Type `{name}` must have suffix `{expected_suffix}`.",
            ),
        ));
    }
    Ok(())
}

/// Verifies that a blackboard struct ends with `Context` or `Blackboard`.
pub fn verify_blackboard_suffix(ident: &Ident) -> Result<()> {
    let name = ident.to_string();
    if !name.ends_with("Context") && !name.ends_with("Blackboard") {
        return Err(Error::new(
            ident.span(),
            format!(
                "[{SMA_TAXO_008}] Architectural violation: Context Blackboard `{name}` must have suffix `Context` or `Blackboard`.",
            ),
        ));
    }
    Ok(())
}

/// Verifies Layer trait implementation: receiver immutability, FlowControl return, and hot-path body statements.
pub fn verify_layer_impl(item_impl: &ItemImpl) -> Result<()> {
    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();
            if fn_name == "on_enter" {
                verify_receiver_immutable(&method.sig)?;
                verify_return_flow_control(&method.sig)?;
                let mut visitor = HotPathAstVisitor::new();
                visitor.validate_fn(method)?;
            } else if fn_name == "on_exit" {
                verify_receiver_immutable(&method.sig)?;
                let mut visitor = HotPathAstVisitor::new();
                visitor.validate_fn(method)?;
            }
        }
    }
    Ok(())
}

/// Verifies Terminal trait implementation: hot-path body statements.
pub fn verify_terminal_impl(item_impl: &ItemImpl) -> Result<()> {
    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();
            if fn_name == "execute" {
                let mut visitor = HotPathAstVisitor::new();
                visitor.validate_fn(method)?;
            }
        }
    }
    Ok(())
}

/// Verifies Query trait implementation: strictly immutable context `&TCtx` and no mutation statements.
pub fn verify_query_impl(item_impl: &ItemImpl) -> Result<()> {
    // SMA-CQS-051: Reject hybrid types implementing both Query and Command
    if let Some((_, path, _)) = &item_impl.trait_ {
        let trait_name = path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if trait_name.contains("Command") {
            return Err(Error::new(
                item_impl.self_ty.span(),
                format!(
                    "[{SMA_CQS_051}] CQS Hybrid Violation: A type cannot implement both `Query` and `Command`. Keep intent read/write roles strictly segregated.",
                ),
            ));
        }
    }

    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();
            if fn_name == "query" {
                // Ensure receiver is &self
                verify_receiver_immutable(&method.sig)?;

                // Ensure ctx argument is not mutable &mut
                for input in &method.sig.inputs {
                    if let syn::FnArg::Typed(pat_type) = input {
                        let ty_str = quote::quote!(#pat_type).to_string();
                        if ty_str.contains("& mut") {
                            return Err(Error::new(
                                pat_type.span(),
                                format!(
                                    "[{SMA_CQS_050}] CQS Violation: Query method `query` cannot accept mutable references `&mut`. Queries must be pure reads.",
                                ),
                            ));
                        }
                    }
                }

                // Check hot-path body statements
                let mut visitor = HotPathAstVisitor::new();
                visitor.validate_fn(method)?;
            }
        }
    }
    Ok(())
}

/// Verifies Command trait implementation: hot-path body statements.
pub fn verify_command_impl(item_impl: &ItemImpl) -> Result<()> {
    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();
            if fn_name == "execute" {
                verify_receiver_immutable(&method.sig)?;
                let mut visitor = HotPathAstVisitor::new();
                visitor.validate_fn(method)?;
            }
        }
    }
    Ok(())
}

fn verify_receiver_immutable(sig: &Signature) -> Result<()> {
    match sig.receiver() {
        Some(Receiver {
            reference: Some(_),
            mutability: None,
            ..
        }) => Ok(()),
        _ => Err(Error::new(
            sig.span(),
            format!(
                "[{SMA_CONCUR_040}] Concurrency contract violation: Layer traversal methods must receive immutable `&self` to guarantee stateless re-entrancy.",
            ),
        )),
    }
}

fn verify_return_flow_control(sig: &Signature) -> Result<()> {
    match &sig.output {
        ReturnType::Type(_, ty) => {
            let ty_str = quote::quote!(#ty).to_string();
            if !ty_str.contains("FlowControl") {
                return Err(Error::new(
                    ty.span(),
                    format!(
                        "[{SMA_HOTPATH_024}] Contract violation: Method `on_enter` must return `FlowControl<...>`, found `{ty_str}`.",
                    ),
                ));
            }
            Ok(())
        }
        ReturnType::Default => Err(Error::new(
            sig.span(),
            format!(
                "[{SMA_HOTPATH_024}] Contract violation: Method `on_enter` must return `FlowControl<...>`, found `()`.",
            ),
        )),
    }
}
