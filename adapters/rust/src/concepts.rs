use crate::cfg_gate::is_test_gated;
use crate::{edges, normalize};
use domain::LocationKind;
use domain::Provenance;
use domain::{ConceptNode, Edge, SignatureState, Source};
use std::path::Path;
use syn::{Attribute, File, Visibility};

pub fn extract_from_entry(
    parsed: &File,
    unit: Option<&str>,
    module_path: Option<&str>,
    path: &Path,
    out: &mut Vec<ConceptNode>,
    edges_out: &mut Vec<Edge>,
) {
    for item in &parsed.items {
        visit_top_level_item(item, path, module_path, unit, out);
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
    out.push(
        ConceptNode::new(
            ident.to_string(),
            Source::Code {
                language: domain::CodeLanguage::Rust,
                path: path.to_path_buf(),
                line,
                provenance: Provenance::empty(),
                location: LocationKind::Path,
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
