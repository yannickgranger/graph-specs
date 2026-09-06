use adapter_markdown::{assemble_spec_trees, MarkdownReader, SpecTree};
use adapter_rust::{RustAnchorResolver, RustReader};
use domain::{
    diff, CheckInput, CheckOutcome, CohesionViolation, ConceptNode, ContextViolation,
    DeclaredSurface, Graph, ResolvedAnchor, VerbDecl, VerbOwnership, Violation,
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
        Some(keyspace) => {
            let surface = DeclaredSurface::from_contexts(&spec_contexts);
            let facts = code_facts(code_dir, Some(keyspace), &surface)?;
            if facts.is_empty() {
                let concept_rung_items = concept_rung_items(code_dir, keyspace)?;
                if concept_rung_items > 0 {
                    return Ok(CheckOutcome::new(
                        vec![Violation::Context(ContextViolation::SurfaceAdmitsNothing {
                            declared_prefixes: spec_contexts
                                .iter()
                                .flat_map(|c| c.owned_units.iter().cloned())
                                .collect(),
                            concept_rung_items,
                            keyspace: keyspace.to_path_buf(),
                        })],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    ));
                }
            }
            Graph::new(facts, code_relationships(code_dir, keyspace, &surface)?)
        }
    };
    let pub_fn_decls = RustReader.extract_pub_fns(code_dir)?;
    let concept_anchors = MarkdownReader.extract_concept_anchors(specs_dir)?;

    let trees = assemble_spec_trees(specs_dir)?;
    let declared: HashMap<&str, &str> = trees
        .iter()
        .flat_map(SpecTree::concept_declarations)
        .collect();
    specs_graph.nodes = specs_graph
        .nodes
        .into_iter()
        .map(|node| {
            let declared_context = declared.get(node.name.as_str()).map(|c| (*c).to_owned());
            match declared_context {
                None => node,
                context => node.with_declared_context(context),
            }
        })
        .collect();
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
fn concept_rung_items(_code_dir: &Path, keyspace: &Path) -> Result<usize, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace).concept_rung_items()
}

#[cfg(not(feature = "codefacts"))]
fn concept_rung_items(code_dir: &Path, _keyspace: &Path) -> Result<usize, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
}

#[cfg(feature = "codefacts")]
fn code_relationships(
    code_dir: &Path,
    keyspace: &Path,
    surface: &DeclaredSurface,
) -> Result<Vec<domain::Edge>, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace)
        .with_surface(surface.clone())
        .relationships(code_dir)
}

#[cfg(not(feature = "codefacts"))]
fn code_relationships(
    code_dir: &Path,
    _keyspace: &Path,
    _surface: &DeclaredSurface,
) -> Result<Vec<domain::Edge>, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
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
    fn two_same_named_types_under_two_units_bind_once_and_report_once() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/enrolment.md",
            "# enrolment\n\n## Owns\n\n- enrolment\n",
        );
        write(
            specs.path(),
            "contexts/privacy.md",
            "# privacy\n\n## Owns\n\n- privacy\n",
        );
        write(
            specs.path(),
            "concepts/enrolment.md",
            "# enrolment\n\n## Clock\n",
        );
        write(
            code.path(),
            "enrolment/Cargo.toml",
            "[package]\nname = \"enrolment\"\n",
        );
        write(code.path(), "enrolment/src/lib.rs", "pub struct Clock;");
        write(
            code.path(),
            "privacy/Cargo.toml",
            "[package]\nname = \"privacy\"\n",
        );
        write(code.path(), "privacy/src/lib.rs", "pub struct Clock;");

        let violations = run_check(specs.path(), code.path(), None)
            .unwrap()
            .violations;
        let reported: Vec<&Violation> = violations
            .iter()
            .filter(|v| matches!(v, Violation::MissingInSpecs { name, .. } if name == "Clock"))
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "the unclaimed Clock is reported exactly once: {violations:?}"
        );
        let Violation::MissingInSpecs { code_source, .. } = reported[0] else {
            unreachable!()
        };
        assert_eq!(
            code_source.unit(),
            Some("privacy"),
            "the reported Clock is the one no heading claimed"
        );
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Clock")),
            "the heading binds its own context's Clock: {violations:?}"
        );
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
    fn a_surface_that_admits_nothing_says_so_instead_of_wearing_the_missing_in_code_costume() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/enrolment.md",
            "# enrolment\n\n## Owns\n\n- App\\Enrolment\n",
        );
        write(
            specs.path(),
            "concepts/enrolment.md",
            "# enrolment\n\n## Course\n\n## Teachable\n",
        );
        let keyspace = dir.path().join("coreen.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
            {"id":"item:Other\\Course","label":"Item","props":{"kind":"trait","line":3,
             "name":"Course","php_construct":"class_declaration","qname":"Other\\Course"}},
            {"id":"item:Other\\Teachable","label":"Item","props":{"kind":"trait","line":3,
             "name":"Teachable","php_construct":"interface_declaration","qname":"Other\\Teachable"}}
            ],"edges":[]}"#,
        )
        .unwrap();

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        assert_eq!(
            violations.len(),
            1,
            "one line about the surface, not one per heading: {violations:?}"
        );
        let Violation::Context(ContextViolation::SurfaceAdmitsNothing {
            declared_prefixes,
            concept_rung_items,
            ..
        }) = &violations[0]
        else {
            panic!("expected SurfaceAdmitsNothing, got {:?}", violations[0])
        };
        assert_eq!(*concept_rung_items, 2, "the keyspace holds N and says N");
        assert_eq!(
            declared_prefixes
                .iter()
                .map(|u| u.0.as_str())
                .collect::<Vec<_>>(),
            vec!["App\\Enrolment"]
        );
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::MissingInCode { .. })),
            "no heading is blamed for a surface that admitted nothing"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn membership_and_binding_report_one_context_not_the_adapters_guess() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/equivalence.md",
            "# equivalence\n\n## Owns\n\n- domain\n",
        );
        write(specs.path(), "concepts/equivalence.md", "# equivalence\n");
        let keyspace = dir.path().join("ks.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":5,"patch":0},"nodes":[
            {"id":"item:marketing::Flyer","label":"Item","props":{"kind":"struct","name":"Flyer",
             "visibility":"pub","is_test":false,"line":1,"file":"/ws/marketing/src/lib.rs",
             "module_qpath":"marketing","bounded_context":"marketing"}}
            ],"edges":[]}"#,
        )
        .unwrap();

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        let membership: Vec<&Violation> = violations
            .iter()
            .filter(|v| {
                matches!(
                    v,
                    Violation::Context(domain::ContextViolation::MembershipUnknown { .. })
                )
            })
            .collect();
        assert_eq!(membership.len(), 1, "{violations:?}");
        let Violation::Context(domain::ContextViolation::MembershipUnknown { code_source, .. }) =
            membership[0]
        else {
            unreachable!()
        };
        assert_eq!(
            code_source.context(),
            None,
            "no declared context owns `marketing`; the adapter's bounded_context is not an answer to that question"
        );
        let orphan = violations
            .iter()
            .find_map(|v| match v {
                Violation::MissingInSpecs { code_source, .. } => Some(code_source),
                _ => None,
            })
            .expect("Flyer is undescribed");
        assert_eq!(
            orphan.context(),
            code_source.context(),
            "membership and binding read the same source"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_implements_edge_across_two_contexts_is_classified_by_name_and_context() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/enrolment.md",
            "# enrolment\n\n## Owns\n\n- App\\Enrolment\n",
        );
        write(
            specs.path(),
            "contexts/privacy.md",
            "# privacy\n\n## Owns\n\n- App\\Privacy\n",
        );
        write(
            specs.path(),
            "concepts/enrolment.md",
            "# enrolment\n\n## Enrolling\n",
        );
        write(
            specs.path(),
            "concepts/privacy.md",
            "# privacy\n\n## Erasable\n",
        );
        let keyspace = dir.path().join("coreen.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
            {"id":"module:App\\Enrolment","label":"Module","props":{"name":"App\\Enrolment"}},
            {"id":"module:App\\Privacy","label":"Module","props":{"name":"App\\Privacy"}},
            {"id":"item:App\\Enrolment\\Enrolling","label":"Item","props":{"kind":"trait","line":3,
             "name":"Enrolling","php_construct":"class_declaration","qname":"App\\Enrolment\\Enrolling"}},
            {"id":"item:App\\Privacy\\Erasable","label":"Item","props":{"kind":"trait","line":3,
             "name":"Erasable","php_construct":"interface_declaration","qname":"App\\Privacy\\Erasable"}}
            ],"edges":[
            {"src":"item:App\\Enrolment\\Enrolling","dst":"module:App\\Enrolment","label":"IN_MODULE"},
            {"src":"item:App\\Privacy\\Erasable","dst":"module:App\\Privacy","label":"IN_MODULE"},
            {"src":"item:App\\Enrolment\\Enrolling","dst":"item:App\\Privacy\\Erasable","label":"IMPLEMENTS"}
            ]}"#,
        )
        .unwrap();

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        let crossing: Vec<&Violation> = violations
            .iter()
            .filter(|v| {
                matches!(
                    v,
                    Violation::Context(domain::ContextViolation::CrossEdgeUnauthorized { .. })
                )
            })
            .collect();
        assert_eq!(
            crossing.len(),
            1,
            "an undeclared crossing between two contexts is reported: {violations:?}"
        );
        let Violation::Context(domain::ContextViolation::CrossEdgeUnauthorized {
            owning_context,
            target_context,
            ..
        }) = crossing[0]
        else {
            unreachable!()
        };
        assert_eq!(owning_context, "enrolment");
        assert_eq!(target_context, "privacy");
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_implements_edge_off_the_declared_surface_is_reported_as_a_crossing() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/catalogue.md",
            "# catalogue\n\n## Owns\n\n- App\\Catalogue\n",
        );
        write(
            specs.path(),
            "concepts/catalogue.md",
            "# catalogue\n\n## Course\n\n## Teachable\n",
        );
        let keyspace = dir.path().join("coreen.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
            {"id":"module:App\\Catalogue","label":"Module","props":{"name":"App\\Catalogue"}},
            {"id":"item:App\\Catalogue\\Course","label":"Item","props":{"kind":"trait","line":3,
             "name":"Course","php_construct":"class_declaration","qname":"App\\Catalogue\\Course"}},
            {"id":"item:App\\Catalogue\\Teachable","label":"Item","props":{"kind":"trait","line":3,
             "name":"Teachable","php_construct":"interface_declaration","qname":"App\\Catalogue\\Teachable"}},
            {"id":"item:Vendor\\Serializable","label":"Item","props":{"kind":"trait","line":3,
             "name":"Serializable","php_construct":"interface_declaration","qname":"Vendor\\Serializable"}}
            ],"edges":[
            {"src":"item:App\\Catalogue\\Course","dst":"module:App\\Catalogue","label":"IN_MODULE"},
            {"src":"item:App\\Catalogue\\Course","dst":"item:App\\Catalogue\\Teachable","label":"IMPLEMENTS"},
            {"src":"item:App\\Catalogue\\Course","dst":"item:Vendor\\Serializable","label":"IMPLEMENTS"}
            ]}"#,
        )
        .unwrap();

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        let off: Vec<&Violation> = violations
            .iter()
            .filter(|v| {
                matches!(
                    v,
                    Violation::Context(domain::ContextViolation::CrossEdgeOffSurface { .. })
                )
            })
            .collect();
        assert_eq!(
            off.len(),
            1,
            "the crossing out of the declared surface is reported once: {violations:?}"
        );
        let Violation::Context(domain::ContextViolation::CrossEdgeOffSurface {
            concept,
            target,
            ..
        }) = off[0]
        else {
            unreachable!()
        };
        assert_eq!(concept, "Course");
        assert_eq!(target, "Serializable");
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn two_same_named_php_classes_under_two_prefixes_bind_once_and_report_once() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        write(
            specs.path(),
            "contexts/enrolment.md",
            "# enrolment\n\n## Owns\n\n- App\\Enrolment\n",
        );
        write(
            specs.path(),
            "contexts/privacy.md",
            "# privacy\n\n## Owns\n\n- App\\Privacy\n",
        );
        write(
            specs.path(),
            "concepts/enrolment.md",
            "# enrolment\n\n## Clock\n",
        );
        let keyspace = dir.path().join("coreen.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
            {"id":"module:App\\Enrolment","label":"Module","props":{"name":"App\\Enrolment"}},
            {"id":"module:App\\Privacy","label":"Module","props":{"name":"App\\Privacy"}},
            {"id":"item:App\\Enrolment\\Clock","label":"Item","props":{"kind":"trait","line":3,
             "name":"Clock","php_construct":"class_declaration","qname":"App\\Enrolment\\Clock"}},
            {"id":"item:App\\Privacy\\Clock","label":"Item","props":{"kind":"trait","line":3,
             "name":"Clock","php_construct":"class_declaration","qname":"App\\Privacy\\Clock"}}
            ],"edges":[
            {"src":"item:App\\Enrolment\\Clock","dst":"module:App\\Enrolment","label":"IN_MODULE"},
            {"src":"item:App\\Privacy\\Clock","dst":"module:App\\Privacy","label":"IN_MODULE"}
            ]}"#,
        )
        .unwrap();

        let code = TempDir::new().unwrap();
        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        let reported: Vec<&Violation> = violations
            .iter()
            .filter(|v| matches!(v, Violation::MissingInSpecs { name, .. } if name == "Clock"))
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "one heading claims one Clock; the other is reported: {violations:?}"
        );
        let Violation::MissingInSpecs { code_source, .. } = reported[0] else {
            unreachable!()
        };
        assert_eq!(code_source.unit(), Some("App\\Privacy"));
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
        assert_eq!(facts[0].unit(), Some("domain"));
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
