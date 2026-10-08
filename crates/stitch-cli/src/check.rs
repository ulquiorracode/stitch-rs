//! Architecture AST & Topology scanner implementing `stitch check`.

use crate::config::{RuleSeverity, StitchConfig};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Fields, File, ItemImpl, ItemStruct, ItemTrait, Receiver, Type};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: String,
    pub severity: RuleSeverity,
    pub message: String,
    pub help: Option<String>,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub fn render_rustc(&self) -> String {
        let level = match self.severity {
            RuleSeverity::Deny => "error",
            RuleSeverity::Warn => "warning",
            RuleSeverity::Allow => return String::new(),
        };

        let file_display = self.file.display();
        let mut out = format!(
            "{level}[{}]: {}\n  --> {}:{}:{}\n",
            self.code, self.message, file_display, self.line, self.column
        );
        if let Some(help) = &self.help {
            out.push_str(&format!("   = help: {help}\n"));
        }
        out
    }
}

pub struct CheckRunner<'a> {
    config: &'a StitchConfig,
    root_dir: &'a Path,
}

impl<'a> CheckRunner<'a> {
    pub fn new(config: &'a StitchConfig, root_dir: &'a Path) -> Self {
        Self { config, root_dir }
    }

    pub fn run(&self) -> Vec<Diagnostic> {
        let rust_files = self.collect_rust_files();

        let mut diagnostics: Vec<Diagnostic> = rust_files
            .par_iter()
            .flat_map(|path| self.scan_file(path))
            .collect();

        // Cross-file boundary checks
        diagnostics.extend(self.check_boundaries());

        diagnostics
    }

    fn collect_rust_files(&self) -> Vec<PathBuf> {
        WalkDir::new(self.root_dir)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                let path_str = entry.path().to_string_lossy();
                entry.depth() == 0
                    || (name != "target"
                        && name != ".git"
                        && !name.starts_with('.')
                        && !path_str.contains("tests/ui")
                        && !path_str.contains("tests\\ui"))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .map(|e| e.into_path())
            .collect()
    }

    fn scan_file(&self, path: &Path) -> Vec<Diagnostic> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let syntax_tree: File = match syn::parse_file(&content) {
            Ok(tree) => tree,
            Err(_) => return Vec::new(),
        };

        let mut visitor = AstScanner {
            config: self.config,
            file_path: path,
            diagnostics: Vec::new(),
            in_hot_path_fn: false,
            in_pipeline_impl: false,
        };

        visitor.visit_file(&syntax_tree);
        visitor.diagnostics
    }

    fn check_boundaries(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let cargo_tomls = WalkDir::new(self.root_dir)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "Cargo.toml");

        for entry in cargo_tomls {
            let manifest_path = entry.path();
            let content = match std::fs::read_to_string(manifest_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let toml: toml::Value = match toml::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let pkg_name = match toml.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                Some(n) => n,
                None => continue,
            };

            let deps = toml.get("dependencies").and_then(|d| d.as_table());

            for forbidden in &self.config.boundaries.forbidden_dependencies {
                let from_crates = match forbidden.from.as_str() {
                    "core_crates" => &self.config.boundaries.core_crates,
                    "port_crates" => &self.config.boundaries.port_crates,
                    "service_crates" => &self.config.boundaries.service_crates,
                    _ => continue,
                };

                let to_crates = match forbidden.to.as_str() {
                    "adapter_crates" => &self.config.boundaries.adapter_crates,
                    _ => continue,
                };

                if from_crates.iter().any(|c| c == pkg_name) {
                    if let Some(deps_table) = deps {
                        for to_crate in to_crates {
                            if deps_table.contains_key(to_crate) {
                                diags.push(Diagnostic {
                                    code: "SMA-BOUND-030".to_string(),
                                    severity: RuleSeverity::Deny,
                                    message: format!("Dependency Inversion Violation: Crate `{pkg_name}` directly references adapter crate `{to_crate}` in Cargo.toml."),
                                    help: Some("Core and Port crates must remain purely abstract without vendor FFI adapter dependencies.".to_string()),
                                    file: manifest_path.to_path_buf(),
                                    line: 1,
                                    column: 1,
                                });
                            }
                        }
                    }
                }
            }
        }

        diags
    }
}

struct AstScanner<'a> {
    config: &'a StitchConfig,
    file_path: &'a Path,
    diagnostics: Vec<Diagnostic>,
    in_hot_path_fn: bool,
    in_pipeline_impl: bool,
}

impl<'a, 'ast> Visit<'ast> for AstScanner<'a> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let ident_str = i.ident.to_string();
        let span = i.ident.span();

        // 1. Blackboard Check
        if has_attr(&i.attrs, "blackboard")
            || ident_str.ends_with("Blackboard")
            || ident_str.ends_with("Context")
        {
            if !ident_str.ends_with("Context") && !ident_str.ends_with("Blackboard") {
                self.record("SMA-TAXO-008", span, format!("Type `{ident_str}` decorated with #[stitch::blackboard] must end with `Context` or `Blackboard`."), None);
            }
            if !has_cache_align(&i.attrs) {
                self.record("SMA-SCROOGE-012", span, format!("Context Blackboard `{ident_str}` missing mandatory `#[repr(C, align(64))]` cache-line attribute."), Some("Add `#[repr(C, align(64))]` to align with L1D cache line.".to_string()));
            }
            self.check_heap_fields(&i.fields, &ident_str);
        }

        // 2. Token Check
        if has_attr(&i.attrs, "token") || ident_str.ends_with("Token") {
            if !ident_str.ends_with("Token") {
                self.record("SMA-TAXO-006", span, format!("Type `{ident_str}` decorated with #[stitch::token] must have suffix `Token`."), None);
            }
            if !has_repr_transparent(&i.attrs) {
                self.record("SMA-SCROOGE-013", span, format!("Token `{ident_str}` must have `#[repr(transparent)]` to guarantee 8-byte register layout."), None);
            }
        }

        // 3. Id Check
        if (has_attr(&i.attrs, "id")
            || (ident_str.ends_with("Id") && !ident_str.starts_with("TypeId")))
            && !has_repr_transparent(&i.attrs)
            && has_attr(&i.attrs, "id")
        {
            self.record("SMA-SCROOGE-013", span, format!("Id `{ident_str}` must have `#[repr(transparent)]` to guarantee register-passing layout."), None);
        }

        // 4. Layer Check
        if has_attr(&i.attrs, "layer") {
            if !ident_str.ends_with("Layer") {
                self.record(
                    "SMA-TAXO-001",
                    span,
                    format!("Layer `{ident_str}` must have suffix `Layer`."),
                    None,
                );
            }
            self.check_heap_fields(&i.fields, &ident_str);
        }

        // 5. Terminal Check
        if has_attr(&i.attrs, "terminal") {
            if !ident_str.ends_with("Terminal") {
                self.record(
                    "SMA-TAXO-002",
                    span,
                    format!("Terminal `{ident_str}` must have suffix `Terminal`."),
                    None,
                );
            }
            self.check_heap_fields(&i.fields, &ident_str);
        }

        syn::visit::visit_item_struct(self, i);
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let ident_str = i.ident.to_string();
        if has_attr(&i.attrs, "port") && !ident_str.ends_with("Port") {
            self.record(
                "SMA-TAXO-003",
                i.ident.span(),
                format!("Port trait `{ident_str}` must have suffix `Port`."),
                None,
            );
        }
        syn::visit::visit_item_trait(self, i);
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let is_pipeline = has_attr(&i.attrs, "layer")
            || has_attr(&i.attrs, "terminal")
            || is_impl_of(i, "Layer")
            || is_impl_of(i, "Terminal")
            || is_impl_of(i, "PipelineChain")
            || is_impl_of(i, "ChatLayer")
            || is_impl_of(i, "TakeDamageLayer");

        if has_attr(&i.attrs, "layer") || is_impl_of(i, "Layer") {
            for item in &i.items {
                if let syn::ImplItem::Fn(m) = item {
                    let fn_name = m.sig.ident.to_string();
                    if (fn_name == "on_enter" || fn_name == "on_exit") && is_receiver_mut(&m.sig) {
                        self.record(
                            "SMA-CONCUR-040",
                            m.sig.span(),
                            format!("Layer method `{fn_name}` has mutable receiver `&mut self`. Must be immutable `&self`."),
                            Some("Use immutable `&self` to guarantee thread-safe re-entrancy during U-cycle traversal.".to_string()),
                        );
                    }
                }
            }
        }

        let was_pipeline = self.in_pipeline_impl;
        self.in_pipeline_impl = is_pipeline;
        syn::visit::visit_item_impl(self, i);
        self.in_pipeline_impl = was_pipeline;
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        let fn_name = i.sig.ident.to_string();
        let was_hot = self.in_hot_path_fn;
        if fn_name == "on_enter"
            || fn_name == "on_exit"
            || (self.in_pipeline_impl && fn_name == "execute")
        {
            self.in_hot_path_fn = true;
        }
        syn::visit::visit_impl_item_fn(self, i);
        self.in_hot_path_fn = was_hot;
    }

    fn visit_expr_macro(&mut self, i: &'ast syn::ExprMacro) {
        let macro_name = i
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        let is_hotpath = self.in_hot_path_fn || self.config.is_strict_hotpath(self.file_path);
        if is_hotpath && matches!(macro_name.as_str(), "format" | "vec" | "panic") {
            self.record(
                "SMA-HOTPATH-020",
                i.mac.path.span(),
                format!("Macro `{}!` allocates or panics in hot path.", macro_name),
                Some(
                    "Use static fixed buffers or typed Error return instead of heap allocation."
                        .to_string(),
                ),
            );
        }
        syn::visit::visit_expr_macro(self, i);
    }
}

impl<'a> AstScanner<'a> {
    fn record(&mut self, code: &str, span: proc_macro2::Span, msg: String, help: Option<String>) {
        let severity = self.config.severity_for_path(self.file_path, code);
        if severity == RuleSeverity::Allow {
            return;
        }
        let start = span.start();
        self.diagnostics.push(Diagnostic {
            code: code.to_string(),
            severity,
            message: msg,
            help,
            file: self.file_path.to_path_buf(),
            line: start.line,
            column: start.column,
        });
    }

    fn check_heap_fields(&mut self, fields: &Fields, struct_name: &str) {
        for field in fields {
            if let Type::Path(type_path) = &field.ty
                && let Some(seg) = type_path.path.segments.last()
            {
                let name = seg.ident.to_string();
                if self.config.scrooge.forbid_heap_types.contains(&name) {
                    let field_name = field
                        .ident
                        .as_ref()
                        .map_or_else(|| "<unnamed>".to_string(), |i| i.to_string());
                    self.record(
                        "SMA-SCROOGE-010",
                        seg.ident.span(),
                        format!("Prohibited heap-allocated type `{name}` in hot-path field `{struct_name}.{field_name}`."),
                        Some("Use fixed-capacity array `[u8; N]`, ArrayString, or inline slice.".to_string()),
                    );
                }
            }
        }
    }
}

fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident(name) || a.path().segments.last().is_some_and(|s| s.ident == name)
    })
}

fn has_cache_align(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        if a.path().is_ident("repr") {
            let mut found = false;
            let _ = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("align") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    if let Ok(lit) = content.parse::<syn::LitInt>()
                        && lit.base10_parse::<u64>().unwrap_or(0) >= 64
                    {
                        found = true;
                    }
                }
                Ok(())
            });
            found
        } else {
            false
        }
    })
}

fn has_repr_transparent(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        if a.path().is_ident("repr") {
            let mut found = false;
            let _ = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("transparent") {
                    found = true;
                }
                Ok(())
            });
            found
        } else {
            false
        }
    })
}

fn is_impl_of(item_impl: &ItemImpl, trait_name: &str) -> bool {
    if let Some((_, path, _)) = &item_impl.trait_ {
        path.segments.last().is_some_and(|s| s.ident == trait_name)
    } else {
        false
    }
}

fn is_receiver_mut(sig: &syn::Signature) -> bool {
    matches!(
        sig.receiver(),
        Some(Receiver {
            reference: Some(_),
            mutability: Some(_),
            ..
        })
    )
}
