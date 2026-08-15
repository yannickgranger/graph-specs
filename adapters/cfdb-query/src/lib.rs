//! cfdb-query `CodeFacts` Anti-Corruption Layer.
//!
//! Reads a cfdb keyspace JSON and translates `:Item` nodes into the
//! language-agnostic [`ConceptNode`] provenance triple (`module_path` /
//! `unit` / `context`). It is an **ACL, not a Conformist**: cfdb's
//! Rust-specific props (`crate` / `module_qpath` / `bounded_context`) are
//! *translated* into graph-specs' agnostic fields, never adopted verbatim —
//! and crucially `unit` / `module_path` are **reconstructed from the `file`
//! prop** to match the source-walk adapter's derivation (the dir-path `unit`
//! `find_owned_unit` produces differs from cfdb's package-name `crate`).
//!
//! Routing: the composition root selects this adapter for
//! one-per-crate repos (e.g. agentry) and the source-walk `RustReader` for
//! multi-crate repos (e.g. graph-specs itself). The parity contract is
//! 0-mismatch on `module_path` / `unit` vs source-walk on a real keyspace.
//!
//! Dependency surface is `cfdb-core` only (plus serde) — the keyspace
//! deserializes with cfdb-core's `fact`/`schema` types; no cfdb-petgraph
//! query engine is linked.

use std::path::{Path, PathBuf};

use cfdb_core::fact::{Edge, Node, PropValue};
use cfdb_core::schema::{Label, SchemaVersion};
use domain::{ConceptNode, SignatureState, Source};
use ports::{CodeFacts, ReaderError};
use serde::Deserialize;

mod anchor_resolver;

pub use anchor_resolver::CfdbAnchorResolver;

/// Local mirror of `cfdb_petgraph::persist::KeyspaceFile`, deserialized with
/// cfdb-core types ONLY.
///
/// `schema_version` is a **struct** (`{major, minor, patch}`), not the string
/// the stale `persist.rs` doc-comment example shows — declaring it as
/// [`SchemaVersion`] is what makes the deserializer accept the real on-disk
/// shape. `edges` are retained for round-trip fidelity but
/// the ACL reads only `:Item` nodes.
#[derive(Debug, Deserialize)]
struct KeyspaceFile {
    #[allow(dead_code)]
    schema_version: SchemaVersion,
    nodes: Vec<Node>,
    #[serde(default)]
    #[allow(dead_code)]
    edges: Vec<Edge>,
}

/// The `:Item` kinds the source-walk adapter emits as concepts (`pub
/// struct`/`enum`/`trait`/`type`). cfdb's richer population also carries
/// `fn`/`method`/`impl_block`/`const`/`static`; filtering to these four keeps
/// the ACL's concept set in parity. cfdb `type_alias` == Rust `pub type`.
const CONCEPT_KINDS: &[&str] = &["struct", "enum", "trait", "type_alias"];

/// Directory names the source-walk adapter skips (`adapter-rust`'s
/// `EXCLUDED_DIRS`). cfdb extracts the whole workspace including these, so the
/// ACL re-applies the exclusion for population parity.
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

/// cfdb-query `CodeFacts` adapter — reads `:Item` facts from a keyspace JSON.
#[derive(Debug, Clone)]
pub struct CfdbQueryReader {
    keyspace: PathBuf,
}

impl CfdbQueryReader {
    /// Construct an adapter that reads the cfdb keyspace JSON at `keyspace`
    /// (the file a `cfdb extract --db <dir> --keyspace <name>` run writes to
    /// `<dir>/<name>.json`).
    #[must_use]
    pub fn new(keyspace: impl Into<PathBuf>) -> Self {
        Self {
            keyspace: keyspace.into(),
        }
    }
}

impl CodeFacts for CfdbQueryReader {
    /// Read the keyspace and translate every concept `:Item` into an agnostic
    /// [`ConceptNode`]. `root` is the code root the keyspace was extracted
    /// from — used to relativize the absolute `:Item.file` paths so the
    /// derived `unit` / `module_path` match the source-walk adapter.
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

/// Translate one `:Item` into an agnostic [`ConceptNode`], or `None` when the
/// item is not a concept the source-walk adapter would emit (population
/// parity).
fn item_to_concept(node: &Node, root: &Path) -> Option<ConceptNode> {
    // --- concept-population filter (parity with source-walk) ---
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
    // Stub filter (RFC-010 §12-F): a synthetic `:Item` carries no `file`.
    let file = prop(node, "file")?;
    let rel = relativize(file, root);
    if rel.split('/').any(|seg| EXCLUDED_DIRS.contains(&seg)) {
        return None;
    }

    // --- agnostic provenance, reconstructed from the file path ---
    let unit = crate_dir_of(&rel, root)?;
    // Top-level-of-file parity: the source-walk adapter visits only each
    // file's top-level items, so an `:Item` nested inside an inline `mod`
    // (a deeper `module_qpath`) is skipped.
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
            // The ACL provides containment provenance, not type signatures —
            // signature equivalence stays the source-walk adapter's job.
            SignatureState::Absent,
        )
        .with_provenance(Some(module_path), Some(unit), context),
    )
}

/// `:Item.file` is an absolute path; make it relative to the code `root` and
/// `/`-normalized, so the derived `unit` / `module_path` are workspace-relative
/// like the source-walk adapter's.
fn relativize(file: &str, root: &Path) -> String {
    let file_norm = file.replace('\\', "/");
    let root_norm = root.to_string_lossy().replace('\\', "/");
    let root_trim = root_norm.trim_end_matches('/');
    let rel = file_norm.strip_prefix(root_trim).unwrap_or(&file_norm);
    rel.trim_start_matches('/').to_string()
}

/// The owning crate as the source-walk `find_owned_unit` reports it: the
/// workspace-relative directory of the crate's `Cargo.toml` — i.e. the path
/// segment before `/src/` (`adapters/rust/src/lib.rs` → `adapters/rust`). For
/// a single-crate repo whose `src/` is at the root, the unit is the root
/// directory name. Returns `None` for a file with no `src/` component.
fn crate_dir_of(rel: &str, root: &Path) -> Option<String> {
    if let Some(idx) = rel.find("/src/") {
        return Some(rel[..idx].to_string());
    }
    if rel.starts_with("src/") {
        return root.file_name().and_then(|n| n.to_str()).map(str::to_owned);
    }
    None
}

/// Reproduce the source-walk `module_path_of` derivation: crate-root-collapsed
/// module path (`adapters/rust/src/edges.rs` → `adapters/rust::edges`;
/// `domain/src/lib.rs` → `domain`).
fn module_path_of(rel: &str, unit: &str) -> String {
    let segments = file_module_segments(rel, unit);
    if segments.is_empty() {
        unit.to_string()
    } else {
        format!("{unit}::{}", segments.join("::"))
    }
}

/// The module segments between `<unit>/src/` and the file, with a trailing
/// `lib` / `mod` / `main` collapsed to the crate root.
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

/// True when the `:Item` is declared at its file's top level — the items the
/// source-walk adapter emits. cfdb's `module_qpath` is `<pkg_underscore>[::seg]*`;
/// dropping the crate-root segment leaves the item's module tail.
///
/// An item declared inside an inline `mod` has a `module_qpath` that **extends**
/// the file's own module path with extra segments — that is the only case we
/// skip. cfdb may also *shorten* the tail relative to the file path (e.g. for a
/// `src/bin/<name>.rs` binary it normalizes away the `bin` segment), but such an
/// item is still top-level in its file and the source-walk adapter emits it — so
/// we treat it as top-level. Items without a `module_qpath` prop are top-level.
fn is_top_level(node: &Node, rel: &str, unit: &str) -> bool {
    let Some(mq) = prop(node, "module_qpath") else {
        return true;
    };
    let mq_tail: Vec<&str> = mq.split("::").skip(1).collect();
    let file_segs = file_module_segments(rel, unit);
    // Skip only when the cfdb module path strictly extends the file's module
    // path (genuine inline-`mod` nesting).
    !(mq_tail.len() > file_segs.len() && mq_tail[..file_segs.len()] == file_segs[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/ws";

    /// Write a keyspace fixture to a fresh tempdir and return both — the
    /// `TempDir` guard must outlive the read in `concepts()`. Per-call dirs
    /// keep parallel tests from racing on a shared path.
    fn write_keyspace(json: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("keyspace.json");
        std::fs::write(&path, json).expect("write keyspace fixture");
        (dir, path)
    }

    /// One concept `:Item` with the full Rust prop set, file under `<ROOT>`.
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
        // RFC-010 §12-F: schema_version is a STRUCT, not the string "0.5.0".
        let facts = load(&keyspace_with(""));
        assert!(facts.is_empty());
    }

    #[test]
    fn string_schema_version_is_rejected() {
        // The stale `persist.rs` doc-comment example "0.1.0" must NOT parse.
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
        // §12-F stub filter: a synthetic `:Item` has no `file` prop.
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
        // The central translation: source-walk `unit` is the crate DIRECTORY
        // path ("adapters/rust"), NOT cfdb's `crate` package name
        // ("adapter-rust"). Derived from the `file` prop.
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
        // lib.rs/mod.rs/main.rs collapse to the crate root.
        let item = rust_item("Foo", "struct", "domain/src/lib.rs", "domain", "domain");
        let facts = load(&keyspace_with(&item));
        assert_eq!(facts[0].module_path.as_deref(), Some("domain"));
        assert_eq!(facts[0].unit.as_deref(), Some("domain"));
    }

    #[test]
    fn inline_mod_item_is_filtered() {
        // module_qpath deeper than the file's module ⇒ declared inside an
        // inline `mod`, which the source-walk adapter does not visit.
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
        // cfdb normalizes `src/bin/<name>.rs` to module `<crate>::<name>` (no
        // `bin`), but source-walk's path-based module_path keeps `bin`. The
        // item is top-level in its file, so the ACL must emit it — with the
        // source-walk-matching module_path (regression: agentry target dogfood).
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
        // Items under tests/ / benches/ / examples/ are skipped (parity with
        // the source-walk adapter's EXCLUDED_DIRS).
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
        // fn / method / impl_block / const / static are not concepts.
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
