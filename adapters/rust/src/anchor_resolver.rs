use crate::cfg_gate::is_test_gated;
use crate::pub_fns::root_ident_of_self_ty;
use crate::walk::{is_excluded_dir, read_and_parse};
use domain::Provenance;
use domain::{AnchorKind, AnchorTarget, Source};
use ports::{AnchorResolver, ReaderError};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

pub struct RustAnchorResolver {
    index: HashMap<String, AnchorTarget>,
}

impl RustAnchorResolver {
    pub fn index(root: &Path) -> Result<Self, ReaderError> {
        let mut index = HashMap::new();
        let walker = WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_excluded_dir(e));
        for entry in walker {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let (parsed, path) = read_and_parse(entry.path().to_path_buf())?;
            for item in &parsed.items {
                index_item(item, &path, &mut index);
            }
        }
        Ok(Self { index })
    }
}

impl AnchorResolver for RustAnchorResolver {
    fn resolve(&self, qname: &str) -> Option<AnchorTarget> {
        self.index.get(qname).cloned()
    }
}

fn index_item(item: &syn::Item, path: &Path, index: &mut HashMap<String, AnchorTarget>) {
    match item {
        syn::Item::Struct(s) if !is_test_gated(&s.attrs) => {
            insert(index, &s.ident, AnchorKind::Type, path);
        }
        syn::Item::Enum(e) if !is_test_gated(&e.attrs) => {
            insert(index, &e.ident, AnchorKind::Type, path);
        }
        syn::Item::Trait(t) if !is_test_gated(&t.attrs) => {
            insert(index, &t.ident, AnchorKind::Type, path);
        }
        syn::Item::Type(t) if !is_test_gated(&t.attrs) => {
            insert(index, &t.ident, AnchorKind::Type, path);
        }
        syn::Item::Fn(f) if !is_test_gated(&f.attrs) => {
            insert(index, &f.sig.ident, AnchorKind::Fn, path);
        }
        syn::Item::Const(c) if !is_test_gated(&c.attrs) => {
            insert(index, &c.ident, AnchorKind::Const, path);
        }
        syn::Item::Static(s) if !is_test_gated(&s.attrs) => {
            insert(index, &s.ident, AnchorKind::Const, path);
        }
        syn::Item::Impl(item_impl) if !is_test_gated(&item_impl.attrs) => {
            index_impl_methods(item_impl, path, index);
        }
        _ => {}
    }
}

fn index_impl_methods(
    item_impl: &syn::ItemImpl,
    path: &Path,
    index: &mut HashMap<String, AnchorTarget>,
) {
    let Some(type_root) = root_ident_of_self_ty(&item_impl.self_ty) else {
        return;
    };
    for inner in &item_impl.items {
        let syn::ImplItem::Fn(method) = inner else {
            continue;
        };
        if is_test_gated(&method.attrs) {
            continue;
        }
        let ident = &method.sig.ident;
        let qname = format!("{type_root}::{ident}");
        let line = ident.span().start().line;
        index.entry(qname).or_insert_with(|| AnchorTarget {
            kind: AnchorKind::Fn,
            source: Source::Code {
                path: path.to_path_buf(),
                line,
                provenance: Provenance::empty(),
            },
        });
    }
}

fn insert(
    index: &mut HashMap<String, AnchorTarget>,
    ident: &syn::Ident,
    kind: AnchorKind,
    path: &Path,
) {
    let line = ident.span().start().line;
    index
        .entry(ident.to_string())
        .or_insert_with(|| AnchorTarget {
            kind,
            source: Source::Code {
                path: path.to_path_buf(),
                line,
                provenance: Provenance::empty(),
            },
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn resolver_over(src: &str) -> RustAnchorResolver {
        let d = TempDir::new().expect("tmp");
        let f = d.path().join("lib.rs");
        std::fs::File::create(&f)
            .expect("create")
            .write_all(src.as_bytes())
            .expect("write");
        RustAnchorResolver::index(d.path()).expect("index")
    }

    #[test]
    fn resolves_pub_crate_fn_at_any_visibility() {
        let r = resolver_over("pub(crate) fn validate_intake() {}");
        let t = r.resolve("validate_intake").expect("resolved");
        assert_eq!(t.kind, AnchorKind::Fn);
    }

    #[test]
    fn resolves_pub_crate_type_and_const() {
        let r = resolver_over("pub(crate) struct Hidden; const LIMIT: usize = 5;");
        assert_eq!(r.resolve("Hidden").expect("type").kind, AnchorKind::Type);
        assert_eq!(r.resolve("LIMIT").expect("const").kind, AnchorKind::Const);
    }

    #[test]
    fn resolves_private_fn_too() {
        let r = resolver_over("fn helper() {}");
        assert!(r.resolve("helper").is_some());
    }

    #[test]
    fn resolves_impl_method_qname() {
        let r = resolver_over("struct Foo; impl Foo { pub(crate) fn bar(&self) {} }");
        let t = r.resolve("Foo::bar").expect("method");
        assert_eq!(t.kind, AnchorKind::Fn);
    }

    #[test]
    fn unresolved_qname_is_none() {
        let r = resolver_over("pub fn present() {}");
        assert!(r.resolve("absent").is_none());
    }

    #[test]
    fn test_gated_items_are_not_indexed() {
        let r = resolver_over("#[cfg(test)]\nfn only_in_test() {}");
        assert!(r.resolve("only_in_test").is_none());
    }
}
