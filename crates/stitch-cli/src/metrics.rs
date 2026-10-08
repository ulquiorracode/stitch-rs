//! Scrooge Systems Memory & Alignment Auditor implementing `stitch metrics`.

use std::path::Path;
use syn::visit::Visit;
use syn::{Fields, ItemStruct, Type};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct StructLayoutReport {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub fields: Vec<FieldMetric>,
    pub total_size: usize,
    pub total_padding: usize,
    pub optimal_padding: usize,
    pub preventable_padding: usize,
    pub crosses_cache_line: bool,
}

#[derive(Debug, Clone)]
pub struct FieldMetric {
    pub name: String,
    pub type_str: String,
    pub size: usize,
    pub align: usize,
    pub offset: usize,
    pub padding_before: usize,
}

pub struct MetricsAuditor<'a> {
    root_dir: &'a Path,
}

impl<'a> MetricsAuditor<'a> {
    pub fn new(root_dir: &'a Path) -> Self {
        Self { root_dir }
    }

    pub fn audit(&self) -> Vec<StructLayoutReport> {
        let mut reports = Vec::new();

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
                        && !name.ends_with("bindings_pregenerated.rs"))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"));

        for entry in rust_files {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path)
                && let Ok(syntax_tree) = syn::parse_file(&content)
            {
                let mut visitor = StructLayoutVisitor {
                    file_path: path.to_string_lossy().to_string(),
                    reports: &mut reports,
                };
                visitor.visit_file(&syntax_tree);
            }
        }

        reports
    }
}

struct StructLayoutVisitor<'a> {
    file_path: String,
    reports: &'a mut Vec<StructLayoutReport>,
}

impl<'a, 'ast> Visit<'ast> for StructLayoutVisitor<'a> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if let Fields::Named(named) = &i.fields {
            let mut current_offset = 0;
            let mut total_padding = 0;
            let mut fields_metrics = Vec::new();
            let mut max_align = 1;

            for field in &named.named {
                let field_name = field
                    .ident
                    .as_ref()
                    .map(|i| i.to_string())
                    .unwrap_or_default();
                let (size, align) = estimate_size_align(&field.ty);
                max_align = max_align.max(align);

                let padding_needed = (align - (current_offset % align)) % align;
                total_padding += padding_needed;
                current_offset += padding_needed;

                fields_metrics.push(FieldMetric {
                    name: field_name,
                    type_str: quote::quote!(#field.ty).to_string(),
                    size,
                    align,
                    offset: current_offset,
                    padding_before: padding_needed,
                });

                current_offset += size;
            }

            // Tail padding to round up to max alignment
            let tail_padding = (max_align - (current_offset % max_align)) % max_align;
            total_padding += tail_padding;
            let total_size = current_offset + tail_padding;

            // Optimal layout: sort fields by alignment descending
            let mut optimal_fields = fields_metrics.clone();
            optimal_fields.sort_by_key(|b| std::cmp::Reverse(b.align));
            let mut optimal_offset = 0;
            let mut optimal_padding = 0;
            for f in &optimal_fields {
                let pad = (f.align - (optimal_offset % f.align)) % f.align;
                optimal_padding += pad;
                optimal_offset += pad + f.size;
            }
            let opt_tail = (max_align - (optimal_offset % max_align)) % max_align;
            optimal_padding += opt_tail;

            let crosses_cache_line =
                total_size > 64 || (current_offset / 64 != (current_offset.saturating_sub(1)) / 64);

            let preventable_padding = total_padding.saturating_sub(optimal_padding);

            self.reports.push(StructLayoutReport {
                name: i.ident.to_string(),
                file: self.file_path.clone(),
                line: i.ident.span().start().line,
                fields: fields_metrics,
                total_size,
                total_padding,
                optimal_padding,
                preventable_padding,
                crosses_cache_line,
            });
        }
        syn::visit::visit_item_struct(self, i);
    }
}

pub fn estimate_size_align(ty: &Type) -> (usize, usize) {
    match ty {
        Type::Path(p) => {
            let seg = p
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            match seg.as_str() {
                "u8" | "i8" | "bool" => (1, 1),
                "u16" | "i16" => (2, 2),
                "u32" | "i32" | "f32" => (4, 4),
                "u64" | "i64" | "f64" | "usize" | "isize" => (8, 8),
                "u128" | "i128" => (16, 16),
                _ if seg.ends_with("Token") || seg.ends_with("Id") => (8, 8),
                _ => (8, 8), // Default pointer / opaque estimate
            }
        }
        Type::Array(arr) => {
            let (elem_size, elem_align) = estimate_size_align(&arr.elem);
            let len = if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit),
                ..
            }) = &arr.len
            {
                lit.base10_parse::<usize>().unwrap_or(1)
            } else {
                1
            };
            (elem_size * len, elem_align)
        }
        Type::Reference(_) | Type::Ptr(_) => (8, 8),
        _ => (8, 8),
    }
}
