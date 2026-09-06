use crate::{prop, relativize, KeyspaceFile, EXCLUDED_DIRS};
use cfdb_core::fact::{Node, PropValue};
use cfdb_core::schema::Label;
use domain::LocationKind;
use domain::Provenance;
use domain::{AnchorKind, AnchorTarget, Source};
use ports::{AnchorResolver, ReaderError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CfdbAnchorResolver {
    index: HashMap<String, AnchorTarget>,
}

impl CfdbAnchorResolver {
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

fn index_entry(node: &Node, root: &Path) -> Option<(String, AnchorTarget)> {
    if node
        .props
        .get("is_test")
        .and_then(PropValue::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let php_construct = prop(node, "php_construct");
    let (path, location) = match (prop(node, "file"), php_construct) {
        (Some(file), _) => {
            let rel = relativize(file, root);
            if rel.split('/').any(|seg| EXCLUDED_DIRS.contains(&seg)) {
                return None;
            }
            (PathBuf::from(file), LocationKind::Path)
        }
        (None, Some(_)) => (
            PathBuf::from(namespace_of(prop(node, "qname")?)),
            LocationKind::Namespace,
        ),
        (None, None) => return None,
    };
    let kind = prop(node, "kind")?;
    let anchor_kind = match kind {
        "struct" | "enum" | "trait" | "type_alias" => AnchorKind::Type,
        "fn" | "method" => AnchorKind::Fn,
        "const" | "static" => AnchorKind::Const,
        _ => return None,
    };
    let key = if matches!(php_construct, Some("method_declaration")) {
        php_method_key(prop(node, "qname")?)?
    } else if kind == "method" {
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
                path,
                line,
                provenance: Provenance::empty(),
                location,
            },
        },
    ))
}

fn namespace_of(qname: &str) -> String {
    match qname.rsplit_once('\\') {
        Some((namespace, _)) => namespace.to_string(),
        None => qname.to_string(),
    }
}

fn php_method_key(qname: &str) -> Option<String> {
    let (class_path, method) = qname.rsplit_once("::")?;
    let class = class_path.rsplit('\\').next().unwrap_or(class_path);
    Some(format!("{class}::{method}"))
}

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
