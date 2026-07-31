//! Application orchestration library.
//!
//! Thin glue between the readers (markdown + Rust), the pure diff engine
//! in [`domain`], and the CLI front-end in `main.rs`. Exposed as a library
//! so integration tests can drive the check end-to-end without going
//! through process boundaries when they choose to.

use adapter_markdown::{assemble_spec_trees, MarkdownReader, SpecTree};
use adapter_rust::{RustAnchorResolver, RustReader};
use domain::{
    diff, CheckInput, CheckOutcome, CohesionViolation, ConceptNode, ResolvedAnchor, VerbDecl,
    VerbOwnership,
};
use ports::{AnchorResolver, CodeFacts, ContextReader, Reader, ReaderError, VerbReader};
use std::collections::HashMap;
use std::path::Path;

pub mod ndjson;
pub mod report;
mod report_ndjson;
mod report_text;
pub mod text;

/// Run the full equivalence check across all levels configured in the
/// current domain (concept, signature, edge, and — when context files
/// are present — bounded context).
///
/// Returns the full [`CheckOutcome`] (RFC-013 §3.5): violations plus the
/// `pending` / `realized` marker records. The exit code is a function of
/// `violations` alone — a tree whose only findings are marker records
/// passes.
///
/// `specs_dir` is walked by both the concept reader and the context
/// reader. Each reader skips the other's subtree (`concepts/` vs
/// `contexts/`), so pointing `--specs specs/` at a v0.4 tree picks up
/// both sides; a v0.3 tree with no `contexts/` subdir yields an empty
/// context list and the pass is a no-op.
///
/// # Errors
///
/// Propagates any [`ReaderError`] from the underlying markdown or Rust
/// reader — typically I/O, parse, or directory-walk failures. Cyclic
/// import declarations surface as `ReaderError::ParseFailed` from the
/// context reader.
pub fn run_check(specs_dir: &Path, code_dir: &Path) -> Result<CheckOutcome, ReaderError> {
    let mut specs_graph = MarkdownReader.extract(specs_dir)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs_dir)?;
    let verb_anchors = MarkdownReader.extract_verb_anchors(specs_dir)?;
    let code_graph = RustReader.extract(code_dir)?;
    let pub_fn_decls = RustReader.extract_pub_fns(code_dir)?;
    let concept_anchors = MarkdownReader.extract_concept_anchors(specs_dir)?;

    // RFC-010 R10-3: the abstraction-ladder tree gives each concept its
    // spec-side *declared* context (the `concepts/` H1) and the spec-side
    // structural cohesion violations. Annotate the spec concepts with their
    // declared context so the diff's cohesion pass can compare it against
    // the code-resolved one; collect the structural violations alongside.
    let trees = assemble_spec_trees(specs_dir)?;
    let declared: HashMap<&str, &str> = trees
        .iter()
        .flat_map(SpecTree::concept_declarations)
        .collect();
    for node in &mut specs_graph.nodes {
        if let Some(ctx) = declared.get(node.name.as_str()) {
            node.context = Some((*ctx).to_owned());
        }
    }
    let spec_cohesion: Vec<CohesionViolation> = trees
        .iter()
        .flat_map(SpecTree::cohesion_violations)
        .collect();

    let verb_ownership = VerbOwnership {
        decls: pub_fn_decls.into_iter().map(VerbDecl::from).collect(),
        anchors: verb_anchors,
    };

    // RFC-012 R12-3: resolve each `- impl:` anchor's target against code,
    // at any visibility, through the `AnchorResolver` port. Built once;
    // consulted only for the anchor qnames (the global concept set is
    // unchanged). The diff stays pure — it receives verdicts, not a resolver.
    let resolved_anchors: Vec<ResolvedAnchor> = if concept_anchors.is_empty() {
        Vec::new()
    } else {
        let resolver = RustAnchorResolver::index(code_dir)?;
        concept_anchors
            .into_iter()
            .map(|anchor| {
                let target = resolver.resolve(&anchor.target);
                ResolvedAnchor { anchor, target }
            })
            .collect()
    };

    Ok(diff(
        CheckInput::new(specs_graph, spec_contexts, verb_ownership)
            .with_spec_cohesion(spec_cohesion)
            .with_concept_anchors(resolved_anchors),
        code_graph,
    ))
}

/// Resolve the code-side concept provenance through the `CodeFacts` adapter
/// the RFC-010 §3.3 routing rule selects:
///
/// - `keyspace: None` ⇒ the source-walking [`RustReader`] — the adapter for
///   multi-crate repos (e.g. graph-specs itself), whose bounded contexts span
///   several crates and resolve via `specs/contexts/` Owns.
/// - `keyspace: Some(path)` ⇒ the cfdb-query ACL reading that keyspace — the
///   adapter for one-per-crate repos (e.g. agentry), whose per-crate
///   `bounded_context` is coherent (RFC-010 §13-A (b)-MVP).
///
/// The cfdb-query branch is compiled only under the `codefacts` feature (the
/// opt-in leaf, Invariant 3); without it, a keyspace route is a configuration
/// error, never a silent fallback to source-walk.
///
/// # Errors
///
/// Propagates any [`ReaderError`] from the selected adapter; returns
/// [`ReaderError::WalkFailed`] when a keyspace route is requested in a build
/// compiled without the `codefacts` feature.
pub fn code_facts(
    code_dir: &Path,
    keyspace: Option<&Path>,
) -> Result<Vec<ConceptNode>, ReaderError> {
    keyspace.map_or_else(
        || RustReader.concepts(code_dir),
        |keyspace| keyspace_facts(code_dir, keyspace),
    )
}

/// The `Some(keyspace)` arm of [`code_facts`]: read code facts from a cfdb
/// keyspace via the cfdb-query ACL. Compiled only under the `codefacts`
/// feature (the opt-in leaf, RFC-010 Invariant 3).
#[cfg(feature = "codefacts")]
fn keyspace_facts(code_dir: &Path, keyspace: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace).concepts(code_dir)
}

/// Without the `codefacts` feature, requesting a keyspace route is a
/// configuration error — never a silent fallback to source-walk.
#[cfg(not(feature = "codefacts"))]
fn keyspace_facts(code_dir: &Path, _keyspace: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Violation;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let full = dir.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&full).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn code_facts_without_keyspace_routes_to_source_walk() {
        // RFC-010 §3.3 routing: `None` keyspace selects the source-walk adapter
        // — `code_facts` must return exactly what `RustReader` does.
        let code = TempDir::new().unwrap();
        write(
            code.path(),
            "mycrate/Cargo.toml",
            "[package]\nname = \"mycrate\"\n",
        );
        write(code.path(), "mycrate/src/lib.rs", "pub struct Foo;");
        let via_router = code_facts(code.path(), None).unwrap();
        let via_adapter = RustReader.concepts(code.path()).unwrap();
        assert_eq!(via_router, via_adapter);
        assert!(via_router.iter().any(|c| c.name == "Foo"));
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn code_facts_with_keyspace_routes_to_cfdb_query() {
        // RFC-010 §3.3 routing: `Some(keyspace)` selects the cfdb-query ACL.
        let dir = TempDir::new().unwrap();
        let keyspace = dir.path().join("ks.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":5,"patch":0},"nodes":[
                {"id":"item:domain::Foo","label":"Item","props":{
                    "name":"Foo","kind":"struct","crate":"domain",
                    "bounded_context":"domain","module_qpath":"domain",
                    "file":"/ws/domain/src/lib.rs","visibility":"pub",
                    "is_test":false,"line":1}}],"edges":[]}"#,
        )
        .unwrap();
        let facts = code_facts(Path::new("/ws"), Some(&keyspace)).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].name, "Foo");
        assert_eq!(facts[0].unit.as_deref(), Some("domain"));
    }

    #[test]
    fn empty_trees_yield_no_violations() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        assert!(run_check(specs.path(), code.path()).unwrap().is_clean());
    }

    // --- RFC-012 R12-3 — anchored concepts resolve end-to-end ---

    #[test]
    fn anchored_pub_crate_concept_resolves_end_to_end() {
        // The motivating case: a concept whose impl is `pub(crate)` (no pub
        // type) is satisfied by its `- impl:` anchor — no ZST, no MissingInCode.
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/intake.md",
            "## ValidateIntakeFull\n\n- impl: validate_intake\n",
        );
        write(
            code.path(),
            "src/lib.rs",
            "pub(crate) fn validate_intake() {}",
        );
        let v = run_check(specs.path(), code.path()).unwrap().violations;
        assert!(
            v.is_empty(),
            "anchored pub(crate) concept must resolve: {v:?}"
        );
    }

    #[test]
    fn dangling_anchor_end_to_end() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/intake.md",
            "## ValidateIntakeFull\n\n- impl: nonexistent_fn\n",
        );
        write(code.path(), "src/lib.rs", "pub fn other() {}");
        let v = run_check(specs.path(), code.path()).unwrap().violations;
        assert!(
            v.iter().any(|x| matches!(
                x,
                Violation::DanglingAnchor { target, .. } if target == "nonexistent_fn"
            )),
            "expected DanglingAnchor: {v:?}"
        );
        assert!(
            !v.iter()
                .any(|x| matches!(x, Violation::MissingInCode { .. })),
            "anchored concept must not also be MissingInCode"
        );
    }

    #[test]
    fn matching_tree_yields_no_violations() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(specs.path(), "a.md", "## Foo\n## Bar\n");
        write(
            code.path(),
            "src/lib.rs",
            "pub struct Foo; pub enum Bar { X }",
        );
        assert!(run_check(specs.path(), code.path()).unwrap().is_clean());
    }

    /// v0.4: `--specs specs/` with both `concepts/` and `contexts/`
    /// subdirs. The two readers scope themselves so neither trips on
    /// the other's dialect. This test asserts the wiring returns `Ok`
    /// — unit-matching semantics are covered in
    /// `domain::diff::context_tests`; dogfood against this repo's own
    /// specs is the end-to-end integration proof.
    #[test]
    fn v04_layout_does_not_collide_on_shared_specs_root() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(specs.path(), "concepts/core.md", "## Foo\n## Bar\n");
        write(
            specs.path(),
            "contexts/only.md",
            "# only\n\n## Owns\n\n- fixture\n",
        );
        write(
            code.path(),
            "fixture/src/lib.rs",
            "pub struct Foo; pub enum Bar { X }",
        );
        assert!(run_check(specs.path(), code.path()).is_ok());
    }
}
