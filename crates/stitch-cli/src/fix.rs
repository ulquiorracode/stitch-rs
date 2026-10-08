//! Scrooge Systems Automated Struct Alignment Fixer (`stitch fix --scrooge` / `sma fix --scrooge`).
//!
//! Reorders struct fields in descending alignment order (align 8 -> align 4 -> align 2 -> align 1),
//! eliminating internal padding holes, reducing cache footprint, and optimizing memory layout.

use crate::metrics::estimate_size_align;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Fields, ItemStruct};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct StructFixChange {
    pub struct_name: String,
    pub file_path: PathBuf,
    pub line: usize,
    pub declared_size_before: usize,
    pub declared_size_after: usize,
    pub padding_saved: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FixReport {
    pub changes: Vec<StructFixChange>,
    pub total_padding_saved: usize,
}

pub struct FixEngine<'a> {
    root_dir: &'a Path,
}

impl<'a> FixEngine<'a> {
    pub fn new(root_dir: &'a Path) -> Self {
        Self { root_dir }
    }

    pub fn run_scrooge(&self, dry_run: bool) -> Result<FixReport, String> {
        let mut report = FixReport::default();

        let rust_files = WalkDir::new(self.root_dir)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                let path_str = entry.path().to_string_lossy();
                entry.depth() == 0
                    || (name != "target"
                        && name != ".git"
                        && !name.starts_with('.')
                        && !path_str.contains("tests/ui")
                        && !path_str.contains("tests\\ui")
                        && !path_str.contains("goldsrc-sys")
                        && !name.ends_with("bindings_pregenerated.rs"))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"));

        for entry in rust_files {
            let path = entry.path();
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let syntax_tree = match syn::parse_file(&content) {
                Ok(st) => st,
                Err(_) => continue,
            };

            let mut finder = StructFinder {
                candidates: Vec::new(),
            };
            finder.visit_file(&syntax_tree);

            if finder.candidates.is_empty() {
                continue;
            }

            let mut modified_content = content.clone();
            let mut file_changed = false;

            // Process candidates in reverse order of position to avoid offset shifts
            finder.candidates.sort_by_key(|c| std::cmp::Reverse(c.line));

            for cand in finder.candidates {
                if cand.padding_saved == 0 {
                    continue;
                }

                if let Some(new_content) = reorder_struct_in_source(&modified_content, &cand) {
                    modified_content = new_content;
                    file_changed = true;
                    report.total_padding_saved += cand.padding_saved;
                    report.changes.push(StructFixChange {
                        struct_name: cand.name.clone(),
                        file_path: path.to_path_buf(),
                        line: cand.line,
                        declared_size_before: cand.declared_size_before,
                        declared_size_after: cand.declared_size_after,
                        padding_saved: cand.padding_saved,
                    });
                }
            }

            if file_changed && !dry_run {
                std::fs::write(path, &modified_content)
                    .map_err(|e| format!("Failed to write to `{}`: {e}", path.display()))?;

                // Format with rustfmt
                let _ = std::process::Command::new("rustfmt").arg(path).status();
            }
        }

        Ok(report)
    }
}

struct StructCandidate {
    name: String,
    line: usize,
    fields: Vec<syn::Field>,
    declared_size_before: usize,
    declared_size_after: usize,
    padding_saved: usize,
}

struct StructFinder {
    candidates: Vec<StructCandidate>,
}

impl<'ast> Visit<'ast> for StructFinder {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let struct_name = i.ident.to_string();
        // Skip foreign C-ABI structs (universal convention _t / _s)
        if struct_name.ends_with("_t") || struct_name.ends_with("_s") {
            return;
        }

        if let Fields::Named(named) = &i.fields {
            // Skip structs with raw pointer fields (FFI interfaces)
            if named.named.iter().any(|f| matches!(&f.ty, syn::Type::Ptr(_))) {
                return;
            }

            let mut current_offset = 0;
            let mut internal_holes = 0;
            let mut max_align = 1;
            let mut fields_with_align = Vec::new();

            for field in &named.named {
                let (size, align) = estimate_size_align(&field.ty);
                max_align = max_align.max(align);

                let pad = (align - (current_offset % align)) % align;
                internal_holes += pad;
                current_offset += pad + size;

                fields_with_align.push((field.clone(), size, align));
            }

            let tail_pad = (max_align - (current_offset % max_align)) % max_align;
            let total_size_before = current_offset + tail_pad;

            // Check if fields are already sorted by descending alignment
            let is_already_sorted = fields_with_align.windows(2).all(|w| w[0].2 >= w[1].2);

            if !is_already_sorted {
                // Sort stably by descending alignment
                let mut sorted_fields = fields_with_align.clone();
                sorted_fields.sort_by_key(|f| std::cmp::Reverse(f.2));

                let mut opt_offset = 0;
                for (_, size, align) in &sorted_fields {
                    let pad = (align - (opt_offset % align)) % align;
                    opt_offset += pad + size;
                }
                let opt_tail = (max_align - (opt_offset % max_align)) % max_align;
                let total_size_after = opt_offset + opt_tail;

                let padding_saved = if total_size_before > total_size_after {
                    total_size_before - total_size_after
                } else {
                    internal_holes
                };

                if padding_saved > 0 {
                    let sorted_syn_fields: Vec<syn::Field> =
                        sorted_fields.into_iter().map(|(f, _, _)| f).collect();

                    self.candidates.push(StructCandidate {
                        name: i.ident.to_string(),
                        line: i.ident.span().start().line,
                        fields: sorted_syn_fields,
                        declared_size_before: total_size_before,
                        declared_size_after: total_size_after,
                        padding_saved,
                    });
                }
            }
        }

        syn::visit::visit_item_struct(self, i);
    }
}

fn reorder_struct_in_source(content: &str, cand: &StructCandidate) -> Option<String> {
    // Locate `struct <cand.name>`
    let struct_pat = format!("struct {}", cand.name);
    let mut search_idx = 0;

    let target_idx = loop {
        if let Some(pos) = content[search_idx..].find(&struct_pat) {
            let actual_pos = search_idx + pos;
            // Verify word boundary before `struct`
            let is_boundary = actual_pos == 0
                || content.as_bytes()[actual_pos - 1].is_ascii_whitespace()
                || content.as_bytes()[actual_pos - 1] == b'>';
            if is_boundary {
                break actual_pos;
            }
            search_idx = actual_pos + struct_pat.len();
        } else {
            return None;
        }
    };

    // Find opening brace `{` after struct pat
    let brace_open = content[target_idx..].find('{')? + target_idx;

    // Find matching closing brace `}`
    let mut depth = 0;
    let mut brace_close = None;

    for (idx, ch) in content[brace_open..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                brace_close = Some(brace_open + idx);
                break;
            }
        }
    }

    let brace_close = brace_close?;

    // Build replacement body
    let mut body = String::from("\n");
    for field in &cand.fields {
        for attr in &field.attrs {
            if attr.path().is_ident("doc")
                && let syn::Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value
            {
                let val = lit.value();
                if val.starts_with(' ') {
                    body.push_str(&format!("    ///{val}\n"));
                } else {
                    body.push_str(&format!("    /// {val}\n"));
                }
                continue;
            }
            let attr_tok = quote::quote!(#attr).to_string();
            body.push_str(&format!("    {attr_tok}\n"));
        }

        let vis = &field.vis;
        let vis_tok = quote::quote!(#vis).to_string();
        let ident = field.ident.as_ref()?;
        let ty = &field.ty;
        let ty_tok = quote::quote!(#ty).to_string();

        if vis_tok.is_empty() {
            body.push_str(&format!("    {ident}: {ty_tok},\n"));
        } else {
            body.push_str(&format!("    {vis_tok} {ident}: {ty_tok},\n"));
        }
    }

    let mut result = String::with_capacity(content.len() + body.len());
    result.push_str(&content[..=brace_open]);
    result.push_str(&body);
    result.push_str(&content[brace_close..]);

    Some(result)
}
