//! Source-walk anchor resolver — RFC-012 §3.4 / R12-3.
//!
//! Builds an index of code items at **any** visibility (the concept walk is
//! `pub`-only) so a `- impl: <qname>` spec anchor can resolve a concept
//! whose canonical implementation is legitimately `pub(crate)` (or a `fn` /
//! `const`) without manufacturing a caller-less `pub` ZST.
//!
//! Lazy by use: the index is built once from the code root, and the diff
//! consults `resolve` **only** for the qnames an anchor references — so the
//! global concept set the [`crate::RustReader`] produces is untouched
//! (RFC-012 §4 I1). A dedicated struct (not `impl AnchorResolver for
//! RustReader`) because the port's `resolve(&self, qname)` carries no root:
//! the resolver must hold the pre-built index.
//!
//! Kinds (DD-7): top-level `struct`/`enum`/`trait`/`type` → [`AnchorKind::Type`];
//! `fn` → [`AnchorKind::Fn`]; `const`/`static` → [`AnchorKind::Const`]; impl
//! methods (`Type::method`) → [`AnchorKind::Fn`]. Enum **variants** are
//! deferred to R12-6 (cfdb-query, where `kind:"variant"` is native).

use crate::cfg_gate::is_test_gated;
use crate::pub_fns::root_ident_of_self_ty;
use crate::walk::{is_excluded_dir, read_and_parse};
use domain::{AnchorKind, AnchorTarget, Source};
use ports::{AnchorResolver, ReaderError};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// A source-walk [`AnchorResolver`] (RFC-012 §3.4) over a code root.
pub struct RustAnchorResolver {
    index: HashMap<String, AnchorTarget>,
}

impl RustAnchorResolver {
    /// Build the any-visibility item index by walking every `*.rs` file under
    /// `root`, excluding the same directories the concept walk skips
    /// (`target/`, `tests/`, …) and `#[cfg(test)]`-gated items.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::WalkFailed`] on directory-traversal failure, or
    /// [`ReaderError::IoFailed`] / [`ReaderError::ParseFailed`] when a source
    /// file cannot be read or parsed.
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

/// Index one top-level item (any visibility) plus its impl methods. The first
/// occurrence of a qname wins (resolution only cares Some/None; the `Source`
/// of a duplicate name is otherwise immaterial).
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

/// Index every `Type::method` of an inherent or trait impl (any visibility).
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
        // `d` stays alive through the walk — it drops only when this fn returns.
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
        // Any visibility — even a bare private `fn` resolves (it exists).
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
