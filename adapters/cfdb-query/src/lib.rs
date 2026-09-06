use std::path::{Path, PathBuf};

use cfdb_core::fact::{Edge, Node, PropValue};
use cfdb_core::schema::{Label, SchemaVersion};
use domain::LocationKind;
use domain::Provenance;
use domain::{ConceptNode, DeclaredSurface, PubFnDecl, SignatureState, Source};
use ports::{CodeFacts, ReaderError, VerbReader};
use serde::Deserialize;

mod anchor_resolver;
mod php_edge_traversal;

pub use anchor_resolver::CfdbAnchorResolver;
pub use php_edge_traversal::PhpEdgeTraversal;

#[derive(Debug, Deserialize)]
struct KeyspaceFile {
    #[allow(dead_code)]
    schema_version: SchemaVersion,
    nodes: Vec<Node>,
    #[serde(default)]
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

#[derive(Debug, Clone, Default)]
pub struct CfdbQueryReader {
    keyspace: PathBuf,
    surface: DeclaredSurface,
}

impl CfdbQueryReader {
    #[must_use]
    pub fn new(keyspace: impl Into<PathBuf>) -> Self {
        Self {
            keyspace: keyspace.into(),
            surface: DeclaredSurface::default(),
        }
    }

    #[must_use]
    pub fn with_surface(mut self, surface: DeclaredSurface) -> Self {
        self.surface = surface;
        self
    }
}

impl CfdbQueryReader {
    fn load(&self) -> Result<KeyspaceFile, ReaderError> {
        let bytes = std::fs::read(&self.keyspace).map_err(|e| ReaderError::IoFailed {
            path: self.keyspace.clone(),
            cause: e.to_string(),
        })?;
        serde_json::from_slice(&bytes).map_err(|e| ReaderError::ParseFailed {
            path: self.keyspace.clone(),
            line: e.line(),
            message: e.to_string(),
        })
    }
}

impl CfdbQueryReader {
    pub fn concept_rung_items(&self) -> Result<usize, ReaderError> {
        let file = self.load()?;
        if discriminate(&self.keyspace, &file.nodes)? != Producer::Php {
            return Ok(0);
        }
        Ok(PhpEdgeTraversal::concept_rung_items(&file.nodes))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Producer {
    Rust,
    Php,
}

impl Producer {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust (`crate` / `bounded_context`)",
            Self::Php => "php (`php_construct`)",
        }
    }
}

fn mark_of(node: &Node) -> Option<Producer> {
    if prop(node, "php_construct").is_some() {
        return Some(Producer::Php);
    }
    if prop(node, "crate").is_some() || prop(node, "bounded_context").is_some() {
        return Some(Producer::Rust);
    }
    None
}

fn discriminate(keyspace: &Path, nodes: &[Node]) -> Result<Producer, ReaderError> {
    let items: Vec<&Node> = nodes
        .iter()
        .filter(|n| n.label.as_str() == Label::ITEM)
        .collect();
    if items.is_empty() {
        return Ok(Producer::Rust);
    }
    let mut seen: Option<Producer> = None;
    let mut unmarked: Option<&Node> = None;
    for item in &items {
        match mark_of(item) {
            None => unmarked = unmarked.or(Some(item)),
            Some(mark) => match seen {
                None => seen = Some(mark),
                Some(first) if first != mark => {
                    return Err(could_not_run(
                        keyspace,
                        &format!(
                            "the keyspace carries items of two producers — {} and {} — and graph-specs-011-php-ladder#4 invariant 7 rules one producer per keyspace, so neither side is read rather than one side dropped",
                            first.as_str(),
                            mark.as_str()
                        ),
                    ))
                }
                Some(_) => {}
            },
        }
    }
    match (seen, unmarked) {
        (Some(mark), None) => Ok(mark),
        (_, Some(node)) => Err(could_not_run(
            keyspace,
            &format!(
                "`{}` carries none of the marks a producer stamps — neither `php_construct` nor `crate`/`bounded_context` (cfdb-045-polyglot-relationship-edges#3.2, the mark sets measured at cfdb 0.8.0) — so which producer wrote this keyspace cannot be told, and the concept channel refuses rather than reading it as the other producer's shape",
                item_name(node)
            ),
        )),
        (None, None) => Ok(Producer::Rust),
    }
}

fn item_name(node: &Node) -> &str {
    prop(node, "qname")
        .or_else(|| prop(node, "name"))
        .unwrap_or(node.id.as_str())
}

fn could_not_run(keyspace: &Path, cause: &str) -> ReaderError {
    ReaderError::ParseFailed {
        path: keyspace.to_path_buf(),
        line: 0,
        message: format!("could not run the concept channel on this keyspace: {cause}"),
    }
}

impl VerbReader for CfdbQueryReader {
    fn extract_pub_fns(&self, _root: &Path) -> Result<Vec<PubFnDecl>, ReaderError> {
        let file = self.load()?;
        if discriminate(&self.keyspace, &file.nodes)? != Producer::Php {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for node in &file.nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            let Some(construct) = prop(node, "php_construct") else {
                continue;
            };
            if !matches!(construct, "method_declaration" | "function_definition") {
                continue;
            }
            let Some(qname) = prop(node, "qname") else {
                continue;
            };
            let Some(unit) = self.surface.unit_of(qname) else {
                continue;
            };
            let line = node
                .props
                .get("line")
                .and_then(PropValue::as_i64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(0);
            out.push(PubFnDecl {
                name: anchor_key(qname, construct),
                source: Source::Code {
                    path: PathBuf::from(namespace_of(qname)),
                    line,
                    provenance: Provenance::empty(),
                    location: LocationKind::Namespace,
                },
                owned_unit: Some(unit.to_owned()),
            });
        }
        Ok(out)
    }
}

fn anchor_key(qname: &str, construct: &str) -> String {
    if construct == "method_declaration" {
        let Some((class_path, method)) = qname.rsplit_once("::") else {
            return short_name(qname).to_owned();
        };
        return format!("{}::{method}", short_name(class_path));
    }
    short_name(qname).to_owned()
}

fn short_name(qname: &str) -> &str {
    qname.rsplit('\\').next().unwrap_or(qname)
}

fn namespace_of(qname: &str) -> String {
    let head = qname.split("::").next().unwrap_or(qname);
    match head.rsplit_once('\\') {
        Some((namespace, _)) => namespace.to_string(),
        None => head.to_string(),
    }
}

impl CodeFacts for CfdbQueryReader {
    fn relationships(&self, _root: &Path) -> Result<Vec<domain::Edge>, ReaderError> {
        let file = self.load()?;
        match discriminate(&self.keyspace, &file.nodes)? {
            Producer::Php => {
                PhpEdgeTraversal::new(self.surface.clone()).relationships(&file.nodes, &file.edges)
            }
            Producer::Rust => Ok(Vec::new()),
        }
    }

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

        if discriminate(&self.keyspace, &file.nodes)? == Producer::Php {
            return PhpEdgeTraversal::new(self.surface.clone()).concepts(&file.nodes, &file.edges);
        }

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
                provenance: Provenance::empty(),
                location: LocationKind::Path,
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
        assert_eq!(facts[0].unit(), Some("adapters/rust"));
        assert_eq!(facts[0].module_path(), Some("adapters/rust::edges"));
    }

    #[test]
    fn crate_root_collapses_to_unit() {
        let item = rust_item("Foo", "struct", "domain/src/lib.rs", "domain", "domain");
        let facts = load(&keyspace_with(&item));
        assert_eq!(facts[0].module_path(), Some("domain"));
        assert_eq!(facts[0].unit(), Some("domain"));
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
        assert_eq!(facts[0].unit(), Some("mycrate"));
        assert_eq!(facts[0].module_path(), Some("mycrate::bin::captain"));
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
