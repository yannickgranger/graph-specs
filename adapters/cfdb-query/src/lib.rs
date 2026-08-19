use std::path::{Path, PathBuf};

use cfdb_core::fact::{Edge, Node, PropValue};
use cfdb_core::schema::{Label, SchemaVersion};
use domain::{ConceptNode, SignatureState, Source};
use ports::{CodeFacts, ReaderError};
use serde::Deserialize;

mod anchor_resolver;

pub use anchor_resolver::CfdbAnchorResolver;

#[derive(Debug, Deserialize)]
struct KeyspaceFile {
    #[allow(dead_code)]
    schema_version: SchemaVersion,
    nodes: Vec<Node>,
    #[serde(default)]
    #[allow(dead_code)]
    edges: Vec<Edge>,
}

const CONCEPT_KINDS: &[&str] = &["struct", "enum", "trait", "type_alias"];

const EXCLUDED_DIRS: &[&str] = &[
    "target",
    ".git",
    ".claude",
    ".proofs",
    "tests",
    "benches",
    "examples",
    "node_modules",
];

#[derive(Debug, Clone)]
pub struct CfdbQueryReader {
    keyspace: PathBuf,
}

impl CfdbQueryReader {
    #[must_use]
    pub fn new(keyspace: impl Into<PathBuf>) -> Self {
        Self {
            keyspace: keyspace.into(),
        }
    }
}

impl CodeFacts for CfdbQueryReader {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
        let bytes = std::fs::read(&self.keyspace).map_err(|e| ReaderError::IoFailed {
            path: self.keyspace.clone(),
            cause: e.to_string(),
        })?;
        let file: KeyspaceFile =
            serde_json::from_slice(&bytes).map_err(|e| ReaderError::ParseFailed {
                path: self.keyspace.clone(),
                line: e.line(),
                message: e.to_string(),
            })?;

        let mut out = Vec::new();
        for node in &file.nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            if let Some(concept) = item_to_concept(node, root) {
                out.push(concept);
            }
        }
        Ok(out)
    }
}

fn prop<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.props.get(key).and_then(PropValue::as_str)
}

fn item_to_concept(node: &Node, root: &Path) -> Option<ConceptNode> {
    if !CONCEPT_KINDS.contains(&prop(node, "kind")?) {
        return None;
    }
    if prop(node, "visibility") != Some("pub") {
        return None;
    }
    if node
        .props
        .get("is_test")
        .and_then(PropValue::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let file = prop(node, "file")?;
    let rel = relativize(file, root);
    if rel.split('/').any(|seg| EXCLUDED_DIRS.contains(&seg)) {
        return None;
    }

    let unit = crate_dir_of(&rel, root)?;
    if !is_top_level(node, &rel, &unit) {
        return None;
    }
    let module_path = module_path_of(&rel, &unit);
    let context = prop(node, "bounded_context").map(str::to_owned);
    let line = node
        .props
        .get("line")
        .and_then(PropValue::as_i64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0);

    Some(
        ConceptNode::new(
            prop(node, "name")?.to_string(),
            Source::Code {
                path: PathBuf::from(file),
                line,
            },
            SignatureState::Absent,
        )
        .with_provenance(Some(module_path), Some(unit), context),
    )
}

fn relativize(file: &str, root: &Path) -> String {
    let file_norm = file.replace('\\', "/");
    let root_norm = root.to_string_lossy().replace('\\', "/");
    let root_trim = root_norm.trim_end_matches('/');
    let rel = file_norm.strip_prefix(root_trim).unwrap_or(&file_norm);
    rel.trim_start_matches('/').to_string()
}

fn crate_dir_of(rel: &str, root: &Path) -> Option<String> {
    if let Some(idx) = rel.find("/src/") {
        return Some(rel[..idx].to_string());
    }
    if rel.starts_with("src/") {
        return root.file_name().and_then(|n| n.to_str()).map(str::to_owned);
    }
    None
}

fn module_path_of(rel: &str, unit: &str) -> String {
    let segments = file_module_segments(rel, unit);
    if segments.is_empty() {
        unit.to_string()
    } else {
        format!("{unit}::{}", segments.join("::"))
    }
}

fn file_module_segments(rel: &str, unit: &str) -> Vec<String> {
    let after_unit = rel
        .strip_prefix(unit)
        .unwrap_or(rel)
        .trim_start_matches('/');
    let after_src = after_unit.strip_prefix("src/").unwrap_or(after_unit);
    let stem = after_src.strip_suffix(".rs").unwrap_or(after_src);
    let mut segments: Vec<String> = stem
        .split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if matches!(
        segments.last().map(String::as_str),
        Some("lib" | "mod" | "main")
    ) {
        segments.pop();
    }
    segments
}

fn is_top_level(node: &Node, rel: &str, unit: &str) -> bool {
    let Some(mq) = prop(node, "module_qpath") else {
        return true;
    };
    let mq_tail: Vec<&str> = mq.split("::").skip(1).collect();
    let file_segs = file_module_segments(rel, unit);
    !(mq_tail.len() > file_segs.len() && mq_tail[..file_segs.len()] == file_segs[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/ws";

    fn write_keyspace(json: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("keyspace.json");
        std::fs::write(&path, json).expect("write keyspace fixture");
        (dir, path)
    }

    fn rust_item(name: &str, kind: &str, file: &str, crate_name: &str, mq: &str) -> String {
        format!(
            r#"{{ "id": "item:{name}", "label": "Item", "props": {{
                "name": "{name}", "qname": "{crate_name}::{name}", "kind": "{kind}",
                "crate": "{crate_name}", "bounded_context": "{crate_name}",
                "module_qpath": "{mq}", "file": "{ROOT}/{file}",
                "visibility": "pub", "is_test": false, "line": 7 }} }}"#
        )
    }

    fn keyspace_with(nodes: &str) -> String {
        format!(
            r#"{{ "schema_version": {{ "major": 0, "minor": 5, "patch": 0 }},
                "nodes": [{nodes}], "edges": [] }}"#
        )
    }

    fn load(json: &str) -> Vec<ConceptNode> {
        let (_dir, path) = write_keyspace(json);
        CfdbQueryReader::new(path)
            .concepts(Path::new(ROOT))
            .expect("load fixture keyspace")
    }

    #[test]
    fn struct_schema_version_deserializes() {
        let facts = load(&keyspace_with(""));
        assert!(facts.is_empty());
    }

    #[test]
    fn string_schema_version_is_rejected() {
        let (_dir, path) =
            write_keyspace(r#"{ "schema_version": "0.1.0", "nodes": [], "edges": [] }"#);
        let err = CfdbQueryReader::new(path)
            .concepts(Path::new(ROOT))
            .unwrap_err();
        assert!(
            matches!(err, ReaderError::ParseFailed { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn stub_without_file_is_filtered() {
        let stub = r#"{ "id": "item:Stub", "label": "Item", "props": {
            "name": "Stub", "kind": "struct", "crate": "domain",
            "module_qpath": "domain", "visibility": "pub", "is_test": false } }"#;
        let real = rust_item("Real", "struct", "domain/src/lib.rs", "domain", "domain");
        let facts = load(&keyspace_with(&format!("{stub}, {real}")));
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].name, "Real");
    }

    #[test]
    fn unit_is_dir_path_not_package_name() {
        let item = rust_item(
            "Edge",
            "struct",
            "adapters/rust/src/edges.rs",
            "adapter-rust",
            "adapter_rust::edges",
        );
        let facts = load(&keyspace_with(&item));
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].unit.as_deref(), Some("adapters/rust"));
        assert_eq!(
            facts[0].module_path.as_deref(),
            Some("adapters/rust::edges")
        );
    }

    #[test]
    fn crate_root_collapses_to_unit() {
        let item = rust_item("Foo", "struct", "domain/src/lib.rs", "domain", "domain");
        let facts = load(&keyspace_with(&item));
        assert_eq!(facts[0].module_path.as_deref(), Some("domain"));
        assert_eq!(facts[0].unit.as_deref(), Some("domain"));
    }

    #[test]
    fn inline_mod_item_is_filtered() {
        let item = rust_item(
            "Inner",
            "struct",
            "domain/src/lib.rs",
            "domain",
            "domain::inner",
        );
        assert!(load(&keyspace_with(&item)).is_empty());
    }

    #[test]
    fn bin_target_item_is_top_level() {
        let item = rust_item(
            "Cmd",
            "struct",
            "mycrate/src/bin/captain.rs",
            "mycrate",
            "mycrate::captain",
        );
        let facts = load(&keyspace_with(&item));
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].unit.as_deref(), Some("mycrate"));
        assert_eq!(
            facts[0].module_path.as_deref(),
            Some("mycrate::bin::captain")
        );
    }

    #[test]
    fn item_in_excluded_dir_is_filtered() {
        let item = rust_item(
            "Fixture",
            "struct",
            "application/tests/cli.rs",
            "application",
            "application::cli",
        );
        assert!(load(&keyspace_with(&item)).is_empty());
    }

    #[test]
    fn non_concept_kinds_are_filtered() {
        let f = rust_item("do_it", "fn", "domain/src/lib.rs", "domain", "domain");
        assert!(load(&keyspace_with(&f)).is_empty());
    }

    #[test]
    fn non_pub_item_is_filtered() {
        let item = r#"{ "id": "item:Priv", "label": "Item", "props": {
            "name": "Priv", "kind": "struct", "crate": "domain",
            "module_qpath": "domain", "file": "/ws/domain/src/lib.rs",
            "visibility": "inherited", "is_test": false } }"#;
        assert!(load(&keyspace_with(item)).is_empty());
    }
}
