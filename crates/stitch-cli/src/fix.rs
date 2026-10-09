//! Scrooge Systems Automated Struct Alignment Fixer (`stitch fix --scrooge`).
//!
//! Reorders struct fields in descending alignment order (align 8 -> align 4 -> align 2 -> align 1),
//! eliminating internal padding holes, reducing cache footprint, and optimizing memory layout.

use crate::config::{ScopeFilter, collect_rust_files};
use crate::metrics::estimate_size_align;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Fields, ItemStruct};

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
    scope: Option<ScopeFilter>,
}

impl<'a> FixEngine<'a> {
    pub fn new(root_dir: &'a Path) -> Self {
        Self {
            root_dir,
            scope: None,
        }
    }

    pub fn new_scoped(root_dir: &'a Path, scope: Option<ScopeFilter>) -> Self {
        Self { root_dir, scope }
    }

    pub fn run_scrooge(&self, dry_run: bool) -> Result<FixReport, String> {
        let mut report = FixReport::default();

        let rust_files = collect_rust_files(self.root_dir, self.scope.as_ref());

        for path in rust_files {
            let content = match std::fs::read_to_string(&path) {
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

            // Safety guard: Detect duplicate struct names in the same file to prevent ambiguous replacement
            let mut name_counts = std::collections::HashMap::new();
            for cand in &finder.candidates {
                *name_counts.entry(cand.name.clone()).or_insert(0) += 1;
            }
            let duplicate_names: HashSet<String> = name_counts
                .into_iter()
                .filter(|(_, count)| *count > 1)
                .map(|(name, _)| name)
                .collect();

            let mut modified_content = content.clone();
            let mut file_changed = false;

            // Process candidates in reverse order of position to avoid offset shifts
            finder.candidates.sort_by_key(|c| std::cmp::Reverse(c.line));

            for cand in finder.candidates {
                if cand.padding_saved == 0 {
                    continue;
                }

                // If struct name is duplicated in the file, refuse to modify to avoid corruption
                if duplicate_names.contains(&cand.name) {
                    eprintln!(
                        "Safety: Skipping struct `{}` in `{}:{}` due to duplicate name in same file.",
                        cand.name,
                        path.display(),
                        cand.line
                    );
                    continue;
                }

                if let Some(scope) = &self.scope
                    && !scope.is_empty()
                    && !scope.matches_path(&path)
                    && !scope.matches_name(&cand.name)
                {
                    continue;
                }

                if let Some(new_content) = reorder_struct_in_source(&modified_content, &cand) {
                    // Verify AST before accepting changes
                    if syn::parse_file(&new_content).is_ok() {
                        modified_content = new_content;
                        file_changed = true;
                        report.total_padding_saved += cand.padding_saved;
                        report.changes.push(StructFixChange {
                            struct_name: cand.name.clone(),
                            file_path: path.clone(),
                            line: cand.line,
                            declared_size_before: cand.declared_size_before,
                            declared_size_after: cand.declared_size_after,
                            padding_saved: cand.padding_saved,
                        });
                    } else {
                        eprintln!(
                            "Safety abort: Reordering struct `{}` in `{}` produced invalid AST. Skipping.",
                            cand.name,
                            path.display()
                        );
                    }
                }
            }

            if file_changed && !dry_run {
                // Final full-file AST validation before touching the disk
                if syn::parse_file(&modified_content).is_err() {
                    return Err(format!(
                        "Fatal: Candidate edits to `{}` failed final syntax tree parse validation. File left untouched.",
                        path.display()
                    ));
                }

                // Atomic write via temp file
                let tmp_path = path.with_extension("tmp.rs");
                std::fs::write(&tmp_path, &modified_content).map_err(|e| {
                    format!("Failed to write temp file `{}`: {e}", tmp_path.display())
                })?;

                if std::fs::rename(&tmp_path, &path).is_err() {
                    // Fallback on Windows if destination file exists
                    let _ = std::fs::remove_file(&path);
                    if let Err(e) = std::fs::rename(&tmp_path, &path) {
                        let _ = std::fs::remove_file(&tmp_path);
                        return Err(format!(
                            "Failed to atomically rename `{}` to `{}`: {e}",
                            tmp_path.display(),
                            path.display()
                        ));
                    }
                }

                // Format with rustfmt and verify status
                let status = std::process::Command::new("rustfmt")
                    .arg("--edition")
                    .arg("2024")
                    .arg(&path)
                    .status();

                match status {
                    Ok(st) if !st.success() => {
                        eprintln!(
                            "Warning: `rustfmt` returned non-zero exit status on `{}`",
                            path.display()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to execute `rustfmt` on `{}`: {e}",
                            path.display()
                        );
                    }
                    _ => {}
                }
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

        // FFI Safety: Skip all #[repr(C)] structs to preserve intentional C-ABI layouts
        for attr in &i.attrs {
            let attr_str = quote::quote!(#attr).to_string();
            if attr_str.contains("repr") && attr_str.contains('C') {
                return;
            }
        }

        if let Fields::Named(named) = &i.fields {
            // Skip structs with raw pointer fields (FFI interfaces)
            if named
                .named
                .iter()
                .any(|f| matches!(&f.ty, syn::Type::Ptr(_)))
            {
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

fn find_line_byte_offset(content: &str, target_line: usize) -> usize {
    let mut current_line = 1;
    for (idx, ch) in content.char_indices() {
        if current_line >= target_line {
            return idx;
        }
        if ch == '\n' {
            current_line += 1;
        }
    }
    content.len()
}

/// Syntax-aware brace finder that correctly ignores braces in comments, string literals, and char literals.
fn find_matching_brace_syntax_aware(content: &str, open_pos: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    if bytes.get(open_pos) != Some(&b'{') {
        return None;
    }

    let mut depth = 0;
    let mut i = open_pos;
    let len = bytes.len();

    while i < len {
        let b = bytes[i];

        // Line comment: //
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            let mut comment_depth = 1;
            while i + 1 < len && comment_depth > 0 {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    comment_depth -= 1;
                    i += 2;
                } else if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    comment_depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Raw string: r"..." or r#"..."#
        if b == b'r' && i + 1 < len && (bytes[i + 1] == b'"' || bytes[i + 1] == b'#') {
            i += 1;
            let mut hashes = 0;
            while i < len && bytes[i] == b'#' {
                hashes += 1;
                i += 1;
            }
            if i < len && bytes[i] == b'"' {
                i += 1;
                'raw_search: while i < len {
                    if bytes[i] == b'"' {
                        let mut end_hashes = 0;
                        while i + 1 + end_hashes < len
                            && bytes[i + 1 + end_hashes] == b'#'
                            && end_hashes < hashes
                        {
                            end_hashes += 1;
                        }
                        if end_hashes == hashes {
                            i += 1 + end_hashes;
                            break 'raw_search;
                        }
                    }
                    i += 1;
                }
                continue;
            }
        }

        // Normal string literal: "..."
        if b == b'"' {
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Char literal vs Lifetime
        if b == b'\'' {
            let is_char_lit = (i + 2 < len && bytes[i + 2] == b'\'' && bytes[i + 1] != b'\\')
                || (i + 3 < len && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'')
                || (i + 1 < len
                    && bytes[i + 1] == b'\\'
                    && bytes[i + 2..len.min(i + 12)].contains(&b'\''));

            if is_char_lit {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else if bytes[i] == b'\'' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            } else {
                // Lifetime like 'a or 'static: just skip the quote
                i += 1;
                continue;
            }
        }

        // Real code braces
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }

        i += 1;
    }

    None
}

fn reorder_struct_in_source(content: &str, cand: &StructCandidate) -> Option<String> {
    // Locate the line byte offset for cand.line
    let line_offset = find_line_byte_offset(content, cand.line);
    let search_start = line_offset.saturating_sub(256);

    let struct_pat = format!("struct {}", cand.name);
    let pos_in_window = content[search_start..].find(&struct_pat)?;
    let target_idx = search_start + pos_in_window;

    // Verify word boundary before `struct`
    let is_boundary = target_idx == 0
        || content.as_bytes()[target_idx - 1].is_ascii_whitespace()
        || content.as_bytes()[target_idx - 1] == b'>';
    if !is_boundary {
        return None;
    }

    // Find opening brace `{` after struct pat
    let brace_offset = content[target_idx..].find('{')?;
    let brace_open = target_idx + brace_offset;

    // Find matching closing brace `}` using syntax-aware scanner
    let brace_close = find_matching_brace_syntax_aware(content, brace_open)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matching_brace_syntax_aware_comments_and_strings() {
        let code = r#"struct Foo {
            // comment with { and }
            /* block comment { with nested } */
            field1: [u8; 8], // normal
            field2: &'static str, /* "string inside comment {" */
            field3: u16,
        }"#;

        let open_pos = code.find('{').unwrap();
        let close_pos = find_matching_brace_syntax_aware(code, open_pos);
        assert_eq!(close_pos, Some(code.len() - 1));
    }

    #[test]
    fn test_repr_c_struct_skipped() {
        let code = r#"
            #[repr(C)]
            pub struct ForeignAbiStruct {
                pub a: u8,
                pub b: u64,
                pub c: u16,
            }
        "#;
        let syntax_tree = syn::parse_file(code).unwrap();
        let mut finder = StructFinder {
            candidates: Vec::new(),
        };
        finder.visit_file(&syntax_tree);

        assert!(
            finder.candidates.is_empty(),
            "#[repr(C)] structs must NEVER be candidates for reordering (C-ABI protection)"
        );
    }

    #[test]
    fn test_reorder_struct_in_source_optimal_descending() {
        let code = "struct SuboptimalLayout {\n    a: u8,\n    b: u64,\n    c: u16,\n}\n";
        let syntax_tree = syn::parse_file(code).unwrap();
        let mut finder = StructFinder {
            candidates: Vec::new(),
        };
        finder.visit_file(&syntax_tree);

        assert_eq!(finder.candidates.len(), 1);
        let cand = &finder.candidates[0];
        let new_code = reorder_struct_in_source(code, cand).expect("Reordering failed");

        // Verify AST is valid
        let re_parsed = syn::parse_file(&new_code);
        assert!(
            re_parsed.is_ok(),
            "Reordered struct source must parse as valid Rust AST"
        );

        // Verify order: b (u64, align 8) -> c (u16, align 2) -> a (u8, align 1)
        let b_idx = new_code.find("b: u64").unwrap();
        let c_idx = new_code.find("c: u16").unwrap();
        let a_idx = new_code.find("a: u8").unwrap();
        assert!(b_idx < c_idx, "b (align 8) must precede c (align 2)");
        assert!(c_idx < a_idx, "c (align 2) must precede a (align 1)");
    }

    #[test]
    fn test_duplicate_struct_name_skipped() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_test_dup_struct_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("duplicate.rs");
        let code = r#"
            mod mod_a {
                pub struct SharedName {
                    pub a: u8,
                    pub b: u64,
                }
            }
            mod mod_b {
                pub struct SharedName {
                    pub x: u8,
                    pub y: u64,
                }
            }
        "#;
        std::fs::write(&file_path, code).unwrap();

        let engine = FixEngine::new(&temp_dir);
        let report = engine.run_scrooge(true).unwrap();

        // Must refuse to touch duplicate named structs
        assert_eq!(
            report.changes.len(),
            0,
            "Duplicate struct names must be refused to avoid ambiguous rewrites"
        );
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
