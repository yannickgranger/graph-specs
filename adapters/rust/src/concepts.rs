//! Concept-level item visiting — the `pub struct` / `pub enum` / `pub trait`
//! / `pub type` walk that backs [`crate::RustBackend`].

use crate::cfg_gate::is_test_gated;
use crate::provenance::{find_owned_unit, module_path_of};
use crate::{edges, normalize};
use domain::{ConceptNode, Edge, SignatureState, Source};
use std::path::Path;
use syn::{Attribute, File, Visibility};

pub(super) fn extract_from_file(
    file: &File,
    path: &Path,
    root: &Path,
    out: &mut Vec<ConceptNode>,
    edges_out: &mut Vec<Edge>,
) {
    // Containment provenance (RFC-010 §3.3) is per-file, so derive it once
    // and share across the file's top-level items: `unit` is the owning
    // crate relative to the code root (§12-I — NOT the raw walked path);
    // `module_path` is the crate-root-collapsed module path (§12-H).
    let unit = find_owned_unit(path, root);
    let module_path = module_path_of(path, root, unit.as_deref());
    for item in &file.items {
        visit_top_level_item(item, path, module_path.as_deref(), unit.as_deref(), out);
        edges::emit_for_item(item, path, edges_out);
    }
}

fn visit_top_level_item(
    item: &syn::Item,
    path: &Path,
    module_path: Option<&str>,
    unit: Option<&str>,
    out: &mut Vec<ConceptNode>,
) {
    use syn::Item;
    match item {
        Item::Struct(s) => emit(
            &s.vis,
            &s.ident,
            &s.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Enum(e) => emit(
            &e.vis,
            &e.ident,
            &e.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Trait(t) => emit(
            &t.vis,
            &t.ident,
            &t.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Type(t) => emit(
            &t.vis,
            &t.ident,
            &t.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        // All other items (Mod, Fn, Impl, Const, Static, Use, Macro, etc.) are
        // not top-level concepts. Inline `mod` contents are intentionally not
        // recursed — per-file top-level only.
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn emit(
    vis: &Visibility,
    ident: &syn::Ident,
    attrs: &[Attribute],
    item: &syn::Item,
    path: &Path,
    module_path: Option<&str>,
    unit: Option<&str>,
    out: &mut Vec<ConceptNode>,
) {
    if !matches!(vis, Visibility::Public(_)) {
        return;
    }
    if is_test_gated(attrs) {
        return;
    }
    let line = ident.span().start().line;
    // `context` stays `None` on the reader side — it needs `specs/contexts/`
    // Owns, resolved by the cohesion pass (RFC-010 §3.4). The cfdb-query ACL
    // (R10-6) populates `context` directly.
    out.push(
        ConceptNode::new(
            ident.to_string(),
            Source::Code {
                path: path.to_path_buf(),
                line,
            },
            SignatureState::Normalized(normalize(item)),
        )
        .with_provenance(
            module_path.map(str::to_owned),
            unit.map(str::to_owned),
            None,
        ),
    );
}
