//! AST statement visitor scanning method bodies for hot-path anti-patterns and heap allocations.

use crate::rules::*;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Error, ExprCall, ExprMethodCall, ImplItemFn, Result};

pub struct HotPathAstVisitor {
    pub violations: Vec<Error>,
}

impl HotPathAstVisitor {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    pub fn validate_fn(&mut self, method: &ImplItemFn) -> Result<()> {
        self.visit_impl_item_fn(method);
        if let Some(first) = self.violations.first() {
            let mut combined_err = first.clone();
            for extra in self.violations.iter().skip(1) {
                combined_err.combine(extra.clone());
            }
            return Err(combined_err);
        }
        Ok(())
    }
}

impl Default for HotPathAstVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for HotPathAstVisitor {
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let macro_name = i
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        match macro_name.as_str() {
            "format" | "vec" | "println" | "eprintln" | "panic" | "dbg" | "todo"
            | "unimplemented" => {
                self.violations.push(Error::new(
                    i.path.span(),
                    format!(
                        "[{}] Scrooge Violation: Macro `{}!` performs heap allocations or I/O blocking inside hot path.",
                        SMA_HOTPATH_020, macro_name
                    ),
                ));
            }
            _ => {}
        }
        syn::visit::visit_macro(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        match method_name.as_str() {
            "to_string" | "to_owned" | "clone_into" => {
                self.violations.push(Error::new(
                    i.method.span(),
                    format!(
                        "[{}] Scrooge Violation: Method call `.{method_name}()` incurs dynamic heap allocation in hot path.",
                        SMA_HOTPATH_021
                    ),
                ));
            }
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_expr_call(&mut self, i: &'ast ExprCall) {
        let func_str = quote::quote!(#i.func).to_string();
        if func_str.contains("Box :: new")
            || func_str.contains("Arc :: new")
            || func_str.contains("Rc :: new")
            || func_str.contains("Vec :: new")
            || func_str.contains("Vec :: with_capacity")
            || func_str.contains("HashMap :: new")
            || func_str.contains("BTreeMap :: new")
            || func_str.contains("String :: from")
            || func_str.contains("String :: new")
        {
            self.violations.push(Error::new(
                i.func.span(),
                format!(
                    "[{}] Scrooge Violation: Heap constructor `{func_str}` is forbidden in hot path.",
                    SMA_HOTPATH_022
                ),
            ));
        }
        syn::visit::visit_expr_call(self, i);
    }
}
