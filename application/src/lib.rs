use adapter_markdown::{assemble_spec_trees, MarkdownReader, SpecTree};
use adapter_rust::{RustAnchorResolver, RustReader};
use domain::{
    diff, CheckInput, CheckOutcome, CohesionViolation, ConceptNode, DeclaredSurface, Graph,
    ResolvedAnchor, VerbDecl, VerbOwnership,
};
use ports::{AnchorResolver, CodeFacts, ContextReader, Reader, ReaderError, VerbReader};
use std::collections::HashMap;
use std::path::Path;

#[cfg(test)]
mod golden;
pub mod ndjson;
pub mod report;
mod report_ndjson;
mod report_text;
pub mod text;

pub fn run_check(
    specs_dir: &Path,
    code_dir: &Path,
    keyspace: Option<&Path>,
) -> Result<CheckOutcome, ReaderError> {
    let mut specs_graph = MarkdownReader.extract(specs_dir)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs_dir)?;
    let verb_anchors = MarkdownReader.extract_verb_anchors(specs_dir)?;
    let code_graph = match keyspace {
        None => RustReader.extract(code_dir)?,
        Some(keyspace) => Graph::new(
            code_facts(
                code_dir,
                Some(keyspace),
                &DeclaredSurface::from_contexts(&spec_contexts),
            )?,
            Vec::new(),
        ),
    };
    let pub_fn_decls = RustReader.extract_pub_fns(code_dir)?;
    let concept_anchors = MarkdownReader.extract_concept_anchors(specs_dir)?;

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

pub fn code_facts(
    code_dir: &Path,
    keyspace: Option<&Path>,
    surface: &DeclaredSurface,
) -> Result<Vec<ConceptNode>, ReaderError> {
    keyspace.map_or_else(
        || RustReader.concepts(code_dir),
        |keyspace| keyspace_facts(code_dir, keyspace, surface),
    )
}

#[cfg(feature = "codefacts")]
fn keyspace_facts(
    code_dir: &Path,
    keyspace: &Path,
    surface: &DeclaredSurface,
) -> Result<Vec<ConceptNode>, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace)
        .with_surface(surface.clone())
        .concepts(code_dir)
}

#[cfg(not(feature = "codefacts"))]
fn keyspace_facts(
    code_dir: &Path,
    _keyspace: &Path,
    _surface: &DeclaredSurface,
) -> Result<Vec<ConceptNode>, ReaderError> {
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
        let code = TempDir::new().unwrap();
        write(
            code.path(),
            "mycrate/Cargo.toml",
            "[package]\nname = \"mycrate\"\n",
        );
        write(code.path(), "mycrate/src/lib.rs", "pub struct Foo;");
        let via_router = code_facts(code.path(), None, &DeclaredSurface::default()).unwrap();
        let via_adapter = RustReader.concepts(code.path()).unwrap();
        assert_eq!(via_router, via_adapter);
        assert!(via_router.iter().any(|c| c.name == "Foo"));
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn code_facts_with_keyspace_routes_to_cfdb_query() {
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
        let facts = code_facts(
            Path::new("/ws"),
            Some(&keyspace),
            &DeclaredSurface::default(),
        )
        .unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].name, "Foo");
        assert_eq!(facts[0].unit.as_deref(), Some("domain"));
    }

    #[test]
    fn empty_trees_yield_no_violations() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        assert!(run_check(specs.path(), code.path(), None)
            .unwrap()
            .is_clean());
    }

    #[test]
    fn anchored_pub_crate_concept_resolves_end_to_end() {
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
        let v = run_check(specs.path(), code.path(), None)
            .unwrap()
            .violations;
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
        let v = run_check(specs.path(), code.path(), None)
            .unwrap()
            .violations;
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
        assert!(run_check(specs.path(), code.path(), None)
            .unwrap()
            .is_clean());
    }

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
        assert!(run_check(specs.path(), code.path(), None).is_ok());
    }
}
