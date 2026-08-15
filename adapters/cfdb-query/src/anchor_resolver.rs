//! cfdb-query anchor resolver.
//!
//! The cfdb-keyspace counterpart to the source-walk `RustAnchorResolver`: it
//! resolves a `- impl: <qname>` anchor against the `:Item` facts a `cfdb
//! extract` run already holds — for **per-crate** repos (e.g. agentry) whose
//! keyspace is the code-fact source. Like the
//! source-walk resolver it builds an index once and `resolve` is a lookup, so
//! the concept set is unchanged.
//!
//! **Any visibility.** The concept ACL ([`crate::CfdbQueryReader`]) filters
//! `visibility == "pub"` for population parity; anchor resolution lifts that —
//! a `pub(crate)` item (cfdb reports it as `visibility == "private"`) resolves
//! just as a `pub` one does.
//!
//! **Kinds.** `struct`/`enum`/`trait`/`type_alias` → [`AnchorKind::Type`];
//! `fn`/`method` → [`AnchorKind::Fn`]; `const`/`static` → [`AnchorKind::Const`].
//! A `method` resolves under its `Type::method` form (the 2-segment qname the
//! `- impl:` grammar accepts), derived from cfdb's full `crate::Type::method`
//! qname. **Enum variants are NOT resolvable here:** cfdb does **not** emit a `variant` `:Item` kind — only the container
//! `enum`. Variant anchoring remains deferred, awaiting
//! a paired cfdb change that extracts variants as items. The source-walk path has the same MVP cut.

use crate::{prop, relativize, KeyspaceFile, EXCLUDED_DIRS};
use cfdb_core::fact::{Node, PropValue};
use cfdb_core::schema::Label;
use domain::{AnchorKind, AnchorTarget, Source};
use ports::{AnchorResolver, ReaderError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A cfdb-keyspace [`AnchorResolver`].
#[derive(Debug, Clone)]
pub struct CfdbAnchorResolver {
    index: HashMap<String, AnchorTarget>,
}

impl CfdbAnchorResolver {
    /// Build the any-visibility anchor index from the cfdb keyspace JSON at
    /// `keyspace`. `root` is the code root the keyspace was extracted from —
    /// used to relativize `:Item.file` paths so the same directory exclusions
    /// the source-walk resolver applies hold here too.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] when the keyspace cannot be read or
    /// [`ReaderError::ParseFailed`] when it cannot be deserialized.
    pub fn index(keyspace: &Path, root: &Path) -> Result<Self, ReaderError> {
        let bytes = std::fs::read(keyspace).map_err(|e| ReaderError::IoFailed {
            path: keyspace.to_path_buf(),
            cause: e.to_string(),
        })?;
        let file: KeyspaceFile =
            serde_json::from_slice(&bytes).map_err(|e| ReaderError::ParseFailed {
                path: keyspace.to_path_buf(),
                line: e.line(),
                message: e.to_string(),
            })?;
        let mut index = HashMap::new();
        for node in &file.nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            if let Some((key, target)) = index_entry(node, root) {
                index.entry(key).or_insert(target);
            }
        }
        Ok(Self { index })
    }
}

impl AnchorResolver for CfdbAnchorResolver {
    fn resolve(&self, qname: &str) -> Option<AnchorTarget> {
        self.index.get(qname).cloned()
    }
}

/// Translate one `:Item` into an `(index-key, target)` pair, at any
/// visibility, or `None` when the item is not anchorable (a stub, an excluded
/// dir, test-gated, or a non-resolvable kind such as `impl_block`).
fn index_entry(node: &Node, root: &Path) -> Option<(String, AnchorTarget)> {
    if node
        .props
        .get("is_test")
        .and_then(PropValue::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    // Stub filter (RFC-010 §12-F): a synthetic `:Item` carries no `file`.
    let file = prop(node, "file")?;
    let rel = relativize(file, root);
    if rel.split('/').any(|seg| EXCLUDED_DIRS.contains(&seg)) {
        return None;
    }
    let kind = prop(node, "kind")?;
    let anchor_kind = match kind {
        "struct" | "enum" | "trait" | "type_alias" => AnchorKind::Type,
        "fn" | "method" => AnchorKind::Fn,
        "const" | "static" => AnchorKind::Const,
        _ => return None, // impl_block, and any future kind, are not anchorable
    };
    let key = if kind == "method" {
        method_key(prop(node, "qname")?)?
    } else {
        prop(node, "name")?.to_string()
    };
    let line = node
        .props
        .get("line")
        .and_then(PropValue::as_i64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0);
    Some((
        key,
        AnchorTarget {
            kind: anchor_kind,
            source: Source::Code {
                path: PathBuf::from(file),
                line,
            },
        },
    ))
}

/// Reduce a method qname `crate::Type::method` to the `Type::method` form the
/// `- impl:` grammar accepts (the same 2-segment shape `- verb:` uses).
fn method_key(qname: &str) -> Option<String> {
    let mut segments: Vec<&str> = qname.split("::").collect();
    let method = segments.pop()?;
    let ty = segments.pop()?;
    Some(format!("{ty}::{method}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    const ROOT: &str = "/ws";

    /// Write a keyspace JSON with the given `:Item` prop objects and return its
    /// path (kept alive by the returned `TempDir`).
    fn keyspace(items: &[&str]) -> (TempDir, PathBuf) {
        let d = TempDir::new().expect("tmp");
        let path = d.path().join("ks.json");
        let nodes = items
            .iter()
            .enumerate()
            .map(|(i, props)| format!(r#"{{"id":"item:{i}","label":"Item","props":{props}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            r#"{{"schema_version":{{"major":0,"minor":5,"patch":0}},"nodes":[{nodes}],"edges":[]}}"#
        );
        std::fs::File::create(&path)
            .expect("create")
            .write_all(json.as_bytes())
            .expect("write");
        (d, path)
    }

    fn item(name: &str, kind: &str, vis: &str, qname: &str) -> String {
        format!(
            r#"{{"name":"{name}","kind":"{kind}","visibility":"{vis}","qname":"{qname}","file":"{ROOT}/src/lib.rs","is_test":false,"line":7}}"#
        )
    }

    #[test]
    fn resolves_pub_crate_fn_at_any_visibility() {
        // cfdb reports `pub(crate)` as `visibility:"private"` — resolution
        // lifts the pub-only filter, so it still resolves.
        let (_d, ks) = keyspace(&[&item(
            "validate_intake",
            "fn",
            "private",
            "krate::validate_intake",
        )]);
        let r = CfdbAnchorResolver::index(&ks, Path::new(ROOT)).expect("index");
        assert_eq!(
            r.resolve("validate_intake").expect("resolved").kind,
            AnchorKind::Fn
        );
    }

    #[test]
    fn resolves_type_and_const_kinds() {
        let (_d, ks) = keyspace(&[
            &item("Hidden", "struct", "private", "krate::Hidden"),
            &item("LIMIT", "const", "private", "krate::LIMIT"),
        ]);
        let r = CfdbAnchorResolver::index(&ks, Path::new(ROOT)).expect("index");
        assert_eq!(r.resolve("Hidden").expect("ty").kind, AnchorKind::Type);
        assert_eq!(r.resolve("LIMIT").expect("const").kind, AnchorKind::Const);
    }

    #[test]
    fn resolves_method_under_two_segment_key() {
        let (_d, ks) = keyspace(&[&item("bar", "method", "private", "krate::Foo::bar")]);
        let r = CfdbAnchorResolver::index(&ks, Path::new(ROOT)).expect("index");
        assert_eq!(r.resolve("Foo::bar").expect("method").kind, AnchorKind::Fn);
        // The full crate-qualified qname is NOT the anchor grammar's form.
        assert!(r.resolve("krate::Foo::bar").is_none());
    }

    #[test]
    fn impl_block_and_absent_are_not_resolvable() {
        let (_d, ks) = keyspace(&[&item(
            "impl Foo",
            "impl_block",
            "private",
            "krate::Foo::impl",
        )]);
        let r = CfdbAnchorResolver::index(&ks, Path::new(ROOT)).expect("index");
        assert!(r.resolve("impl Foo").is_none());
        assert!(r.resolve("absent").is_none());
    }

    #[test]
    fn test_gated_and_stub_items_are_skipped() {
        let test_gated = format!(
            r#"{{"name":"only_in_test","kind":"fn","visibility":"pub","qname":"krate::only_in_test","file":"{ROOT}/src/lib.rs","is_test":true,"line":1}}"#
        );
        let stub = r#"{"name":"Synthetic","kind":"struct","visibility":"pub"}"#.to_owned();
        let (_d, ks) = keyspace(&[&test_gated, &stub]);
        let r = CfdbAnchorResolver::index(&ks, Path::new(ROOT)).expect("index");
        assert!(r.resolve("only_in_test").is_none());
        assert!(r.resolve("Synthetic").is_none());
    }
}
