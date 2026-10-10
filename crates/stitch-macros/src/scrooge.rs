//! Scrooge Memory Invariant checkers: zero-alloc hot paths, cache line alignment, register sizing.

use crate::rules::*;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Fields, Ident, LitInt, Result, Type};

/// Inspects struct fields to ensure no heap-allocated types or trait objects exist on hot paths.
pub fn inspect_struct_fields(fields: &Fields) -> Result<()> {
    for field in fields {
        inspect_type(&field.ty, field.ident.as_ref())?;
    }
    Ok(())
}

fn inspect_type(ty: &Type, field_ident: Option<&Ident>) -> Result<()> {
    match ty {
        Type::Path(type_path) => {
            for segment in &type_path.path.segments {
                let type_name = segment.ident.to_string();
                if FORBIDDEN_HEAP_TYPES.contains(&type_name.as_str()) {
                    let field_name =
                        field_ident.map_or_else(|| "<unnamed>".to_string(), |i| i.to_string());
                    return Err(syn::Error::new(
                        segment.ident.span(),
                        format!(
                            "[{}] Scrooge Violation: Prohibited heap-allocated type `{}` in hot-path field `{}`. Use static buffers, ArrayString, or inline slices.",
                            SMA_SCROOGE_010, type_name, field_name
                        ),
                    ));
                }

                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        for arg in &args.args {
                            match arg {
                                syn::GenericArgument::Type(inner_ty) => {
                                    inspect_type(inner_ty, field_ident)?;
                                }
                                syn::GenericArgument::AssocType(assoc) => {
                                    inspect_type(&assoc.ty, field_ident)?;
                                }
                                _ => {}
                            }
                        }
                    }
                    syn::PathArguments::Parenthesized(paren) => {
                        for input in &paren.inputs {
                            inspect_type(input, field_ident)?;
                        }
                        if let syn::ReturnType::Type(_, output) = &paren.output {
                            inspect_type(output, field_ident)?;
                        }
                    }
                    syn::PathArguments::None => {}
                }
            }
        }
        Type::Array(arr) => {
            inspect_type(&arr.elem, field_ident)?;
        }
        Type::Slice(slice) => {
            inspect_type(&slice.elem, field_ident)?;
        }
        Type::Tuple(tup) => {
            for elem in &tup.elems {
                inspect_type(elem, field_ident)?;
            }
        }
        Type::Reference(r) => {
            inspect_type(&r.elem, field_ident)?;
        }
        Type::Ptr(p) => {
            inspect_type(&p.elem, field_ident)?;
        }
        Type::Paren(paren) => {
            inspect_type(&paren.elem, field_ident)?;
        }
        Type::Group(g) => {
            inspect_type(&g.elem, field_ident)?;
        }
        Type::TraitObject(_) | Type::ImplTrait(_) => {
            return Err(syn::Error::new(
                ty.span(),
                format!(
                    "[{}] Scrooge Violation: Trait object (`dyn ...`) is strictly forbidden on hot paths. Use monomorphic generic parameters.",
                    SMA_SCROOGE_011
                ),
            ));
        }
        _ => {}
    }
    Ok(())
}

/// Verifies that a context struct possesses `#[repr(..., align(N))]` with N >= 64.
pub fn verify_cache_alignment(attrs: &[Attribute], span: proc_macro2::Span) -> Result<()> {
    let mut has_align_64_or_more = false;

    for attr in attrs {
        if attr.path().is_ident("repr") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("align") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: LitInt = content.parse()?;
                    if lit.base10_parse::<u64>()? >= 64 {
                        has_align_64_or_more = true;
                    }
                }
                Ok(())
            });
        }
    }

    if !has_align_64_or_more {
        return Err(syn::Error::new(
            span,
            format!(
                "[{}] Cache Alignment Violation: Context Blackboards must be annotated with `#[repr(C, align(64))]` (or higher) to eliminate false sharing.",
                SMA_SCROOGE_012
            ),
        ));
    }
    Ok(())
}

/// Verifies that a token/id struct possesses `#[repr(transparent)]`.
pub fn verify_transparent(attrs: &[Attribute], span: proc_macro2::Span) -> Result<()> {
    let mut has_transparent = false;

    for attr in attrs {
        if attr.path().is_ident("repr") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("transparent") {
                    has_transparent = true;
                }
                Ok(())
            });
        }
    }

    if !has_transparent {
        return Err(syn::Error::new(
            span,
            format!(
                "[{}] Memory Layout Violation: Tokens and IDs must be annotated with `#[repr(transparent)]` to guarantee register-passable layout.",
                SMA_SCROOGE_013
            ),
        ));
    }
    Ok(())
}

/// Synthesizes static compile-time assertions verifying 8-byte size and target u64 alignment for Tokens and IDs.
pub fn synthesize_register_size_assertion(ident: &Ident) -> proc_macro2::TokenStream {
    quote! {
        const _: () = {
            assert!(
                ::core::mem::size_of::<#ident>() == 8,
                concat!("SMA-SCROOGE-013: Struct `", stringify!(#ident), "` size must be exactly 8 bytes.")
            );
            assert!(
                ::core::mem::align_of::<#ident>() == ::core::mem::align_of::<u64>(),
                concat!("SMA-SCROOGE-013: Struct `", stringify!(#ident), "` alignment must match target u64 alignment.")
            );
        };
    }
}
