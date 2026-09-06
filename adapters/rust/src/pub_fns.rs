use crate::cfg_gate::is_test_gated;
use domain::LocationKind;
use domain::Provenance;
use domain::{PubFnDecl, Source};
use std::path::Path;
use syn::Visibility;

pub fn visit_top_level_fn(
    item: &syn::Item,
    path: &Path,
    owned_unit: Option<&str>,
    out: &mut Vec<PubFnDecl>,
) {
    if let syn::Item::Fn(f) = item {
        if !matches!(f.vis, Visibility::Public(_)) {
            return;
        }
        if is_test_gated(&f.attrs) {
            return;
        }
        let line = f.sig.ident.span().start().line;
        out.push(PubFnDecl {
            name: f.sig.ident.to_string(),
            source: Source::Code {
                path: path.to_path_buf(),
                line,
                provenance: Provenance::empty(),
                location: LocationKind::Path,
            },
            owned_unit: owned_unit.map(str::to_owned),
        });
    }
}

pub fn visit_impl_block(
    item: &syn::Item,
    path: &Path,
    owned_unit: Option<&str>,
    out: &mut Vec<PubFnDecl>,
) {
    let syn::Item::Impl(item_impl) = item else {
        return;
    };
    if is_test_gated(&item_impl.attrs) {
        return;
    }
    let Some(type_root) = root_ident_of_self_ty(&item_impl.self_ty) else {
        return;
    };
    let is_trait_impl = item_impl.trait_.is_some();
    for inner in &item_impl.items {
        let syn::ImplItem::Fn(method) = inner else {
            continue;
        };
        if is_test_gated(&method.attrs) {
            continue;
        }
        let is_public = matches!(method.vis, Visibility::Public(_)) || is_trait_impl;
        if !is_public {
            continue;
        }
        let method_ident = &method.sig.ident;
        let line = method_ident.span().start().line;
        out.push(PubFnDecl {
            name: format!("{type_root}::{method_ident}"),
            source: Source::Code {
                path: path.to_path_buf(),
                line,
                provenance: Provenance::empty(),
                location: LocationKind::Path,
            },
            owned_unit: owned_unit.map(str::to_owned),
        });
    }
}

pub fn root_ident_of_self_ty(ty: &syn::Type) -> Option<&syn::Ident> {
    let syn::Type::Path(tp) = ty else {
        return None;
    };
    if tp.qself.is_some() {
        return None;
    }
    tp.path.segments.first().map(|s| &s.ident)
}
