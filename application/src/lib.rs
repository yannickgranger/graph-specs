use adapter_markdown::MarkdownReader;
use adapter_php::PhpAttributeReader;
use adapter_rust::{RustAnchorResolver, RustLoader, RustReader};
use domain::{
    diff, CheckInput, CheckOutcome, CohesionViolation, ConceptNode, ContextViolation,
    DeclaredSurface, DiffSide, EdgeKind, Graph, OwnershipAmbiguity, ResolvedAnchor, SignatureState,
    SourceWithSig, SpecTree, VerbDecl, VerbOwnership, Violation,
};
use ports::{
    AnchorResolver, CodeFacts, CodeLoader, CodeReader, ConceptAnchorReader, ContextReader,
    ReaderError, SpecLoader, SpecReader, SpecTreeReader, VerbAnchorReader, VerbReader,
};
use std::collections::HashMap;
use std::path::Path;

#[cfg(test)]
mod golden;
pub mod ndjson;
pub mod report;
mod report_ndjson;
mod report_text;
pub mod text;

const SIGNATURE_NORMALIZERS: &[&dyn ports::SignatureNormalizer] =
    &[&signature_norm::RustSignatures, &adapter_php::PhpSignatures];

fn union_spec_graphs(markdown: &mut Graph, attribute: Graph) -> Vec<Violation> {
    let mut drift = Vec::new();
    for node in attribute.nodes {
        match markdown.nodes.iter().find(|n| n.name == node.name) {
            None => markdown.nodes.push(node),
            Some(upstream) => {
                if let (SignatureState::Normalized(up), SignatureState::Normalized(down)) =
                    (&upstream.signature, &node.signature)
                {
                    if up != down {
                        drift.push(Violation::SignatureDriftWithinSide {
                            name: node.name.clone(),
                            side: DiffSide::Spec,
                            sources: vec![
                                SourceWithSig {
                                    source: upstream.source.clone(),
                                    sig: up.clone(),
                                },
                                SourceWithSig {
                                    source: node.source.clone(),
                                    sig: down.clone(),
                                },
                            ],
                        });
                    }
                }
            }
        }
    }
    markdown.edges.extend(attribute.edges);
    drift
}

fn code_inputs(
    code_dir: &Path,
) -> Result<(ports::CodeFileSet, adapter_rust::ParseCache), ReaderError> {
    let set = RustLoader.load(code_dir)?;
    let cache = adapter_rust::parse(code_dir, &set)?;
    Ok((set, cache))
}

fn declared_contexts_per_document(
    nodes: Vec<domain::ConceptNode>,
    trees: &[SpecTree],
) -> Vec<domain::ConceptNode> {
    let declared: HashMap<(&Path, &str), &str> = trees
        .iter()
        .flat_map(|tree| {
            tree.concept_declarations()
                .into_iter()
                .map(move |(name, context)| ((tree.file.as_path(), name), context))
        })
        .collect();
    nodes
        .into_iter()
        .map(|node| {
            let declared_context = declared
                .get(&(node.source.path(), node.name.as_str()))
                .map(|c| (*c).to_owned());
            match declared_context {
                None => node,
                context => node.with_declared_context(context),
            }
        })
        .collect()
}

pub fn run_check(
    specs_dir: &Path,
    code_dir: &Path,
    keyspace: Option<&Path>,
) -> Result<CheckOutcome, ReaderError> {
    let reader = MarkdownReader::new(SIGNATURE_NORMALIZERS);
    let spec_set = reader.load(specs_dir)?;
    let (code_set, cache) = code_inputs(code_dir)?;
    let mut specs_graph = reader.extract(&spec_set)?;
    let attribute_graph =
        PhpAttributeReader::new().extract(&PhpAttributeReader::new().load(code_dir)?)?;
    let mut within_side = union_spec_graphs(&mut specs_graph, attribute_graph);
    let spec_contexts = reader.extract_contexts(&spec_set)?;
    let verb_anchors = reader.extract_verb_anchors(&spec_set)?;
    let surface = match keyspace {
        None => DeclaredSurface::default(),
        Some(keyspace) => DeclaredSurface::from_contexts(&spec_contexts)
            .map_err(|a| ambiguous_ownership(keyspace, &a))?,
    };
    let code_graph = match keyspace {
        None => RustReader::new(cache.clone()).extract(&code_set)?,
        Some(keyspace) => {
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
            let relationships = code_relationships(code_dir, keyspace, &surface)?;
            eprintln!(
                "graph-specs: relationship channel read {} edge(s) on the declared surface",
                relationships.len()
            );
            Graph::new(facts, relationships)
        }
    };
    let pub_fn_decls = match keyspace {
        None => RustReader::new(cache.clone()).extract_pub_fns(code_dir)?,
        Some(keyspace) => keyspace_pub_fns(code_dir, keyspace, &surface)?,
    };
    let (concept_anchors, mut spec_findings) = reader.extract_concept_anchors(&spec_set)?;
    spec_findings.append(&mut PhpAttributeReader::new().extract_findings(code_dir)?);
    spec_findings.append(&mut within_side);

    let trees = reader.extract_spec_trees(&spec_set)?;
    specs_graph.nodes = declared_contexts_per_document(specs_graph.nodes, &trees);
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
        let resolve = anchor_resolver(code_dir, keyspace, &code_set, &cache)?;
        concept_anchors
            .into_iter()
            .map(|anchor| {
                let target = resolve(&anchor.target);
                ResolvedAnchor { anchor, target }
            })
            .collect()
    };

    let answerable = match keyspace {
        None => None,
        Some(keyspace) => Some(keyspace_answerable(code_dir, keyspace)?),
    };
    Ok(diff(
        CheckInput::new(specs_graph, spec_contexts, verb_ownership)
            .with_spec_cohesion(spec_cohesion)
            .with_spec_findings(spec_findings)
            .with_concept_anchors(resolved_anchors),
        code_graph,
        answerable.as_deref(),
    ))
}

#[cfg(feature = "codefacts")]
fn keyspace_answerable(code_dir: &Path, keyspace: &Path) -> Result<Vec<EdgeKind>, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace).answerable_relationships(code_dir)
}

#[cfg(not(feature = "codefacts"))]
fn keyspace_answerable(code_dir: &Path, _keyspace: &Path) -> Result<Vec<EdgeKind>, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
}

fn ambiguous_ownership(keyspace: &Path, ambiguity: &OwnershipAmbiguity) -> ReaderError {
    ReaderError::ParseFailed {
        path: keyspace.to_path_buf(),
        line: 0,
        message: format!(
            "could not run the declared surface: context `{}` owns `{}` and context `{}` owns `{}`, which nests inside it, so which context owns an item beneath the inner prefix has two answers; resolving it by length would pick one silently (graph-specs-011-php-ladder#3.2). Longest-wins still stands inside a single context's own Owns block",
            ambiguity.outer_context,
            ambiguity.outer.0,
            ambiguity.inner_context,
            ambiguity.inner.0
        ),
    }
}

type AnchorLookup = Box<dyn Fn(&str) -> Option<domain::AnchorTarget>>;

fn anchor_resolver(
    code_dir: &Path,
    keyspace: Option<&Path>,
    code_set: &ports::CodeFileSet,
    cache: &adapter_rust::ParseCache,
) -> Result<AnchorLookup, ReaderError> {
    match keyspace {
        None => {
            let resolver = RustAnchorResolver::index(code_set, cache)?;
            Ok(Box::new(move |qname: &str| resolver.resolve(qname)))
        }
        Some(keyspace) => keyspace_anchor_resolver(code_dir, keyspace),
    }
}

#[cfg(feature = "codefacts")]
fn keyspace_anchor_resolver(code_dir: &Path, keyspace: &Path) -> Result<AnchorLookup, ReaderError> {
    let resolver = adapter_cfdb_query::CfdbAnchorResolver::index(keyspace, code_dir)?;
    Ok(Box::new(move |qname: &str| resolver.resolve(qname)))
}

#[cfg(not(feature = "codefacts"))]
fn keyspace_anchor_resolver(
    code_dir: &Path,
    _keyspace: &Path,
) -> Result<AnchorLookup, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
}

#[cfg(feature = "codefacts")]
fn keyspace_pub_fns(
    code_dir: &Path,
    keyspace: &Path,
    surface: &DeclaredSurface,
) -> Result<Vec<domain::PubFnDecl>, ReaderError> {
    adapter_cfdb_query::CfdbQueryReader::new(keyspace)
        .with_surface(surface.clone())
        .extract_pub_fns(code_dir)
}

#[cfg(not(feature = "codefacts"))]
fn keyspace_pub_fns(
    code_dir: &Path,
    _keyspace: &Path,
    _surface: &DeclaredSurface,
) -> Result<Vec<domain::PubFnDecl>, ReaderError> {
    Err(ReaderError::WalkFailed {
        root: code_dir.to_path_buf(),
        cause: "cfdb-query keyspace routing requires the `codefacts` feature".to_owned(),
    })
}

pub fn code_facts(
    code_dir: &Path,
    keyspace: Option<&Path>,
    surface: &DeclaredSurface,
) -> Result<Vec<ConceptNode>, ReaderError> {
    keyspace.map_or_else(
        || {
            let (_set, cache) = code_inputs(code_dir)?;
            RustReader::new(cache).concepts(code_dir)
        },
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

    fn malformed_run(bullet: &str) -> Vec<Violation> {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/catalogue.md",
            &format!("# catalogue\n\n## Course\n\n{bullet}\n"),
        );
        write(code.path(), "Cargo.toml", "[package]\nname = \"c\"\n");
        write(code.path(), "src/lib.rs", "pub struct Course;\n");
        run_check(specs.path(), code.path(), None)
            .unwrap()
            .violations
    }

    fn malformed(violations: &[Violation]) -> Vec<(&str, &str, &str)> {
        violations
            .iter()
            .filter_map(|v| match v {
                Violation::MalformedAnchorBullet {
                    concept,
                    bullet,
                    qname,
                    ..
                } => Some((concept.as_str(), bullet.as_str(), qname.as_str())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_namespace_qualified_verb_bullet_is_reported_not_discarded() {
        let violations = malformed_run("- verb: App\\Catalogue\\Course::rename");
        assert_eq!(
            malformed(&violations),
            vec![("Course", "verb", "App\\Catalogue\\Course::rename")],
            "the bullet, its heading and its qname are named: {violations:?}"
        );
    }

    #[test]
    fn a_namespace_qualified_impl_bullet_is_reported_not_discarded() {
        let violations = malformed_run("- impl: App\\Catalogue\\Course::rename");
        assert_eq!(
            malformed(&violations),
            vec![("Course", "impl", "App\\Catalogue\\Course::rename")]
        );
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::DanglingAnchor { .. })),
            "a bullet the grammar cannot read is malformed, not a dangling anchor: {violations:?}"
        );
    }

    #[test]
    fn a_bullet_the_grammar_reads_is_not_malformed() {
        let bare = malformed_run("- verb: rename");
        assert!(
            malformed(&bare).is_empty(),
            "a bare identifier is one of the two admitted forms: {bare:?}"
        );
        let typed = malformed_run("- impl: Course::rename");
        assert!(
            malformed(&typed).is_empty(),
            "`Type::method` is the other: {typed:?}"
        );
    }

    #[test]
    fn an_empty_anchor_bullet_is_malformed_too() {
        let violations = malformed_run("- verb:");
        assert_eq!(malformed(&violations), vec![("Course", "verb", "")]);
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
        let set = ports::CodeLoader::load(&RustLoader, code.path()).unwrap();
        let cache = adapter_rust::parse(code.path(), &set).unwrap();
        let via_adapter = RustReader::new(cache).concepts(code.path()).unwrap();
        assert_eq!(via_router, via_adapter);
        assert!(via_router.iter().any(|c| c.name == "Foo"));
    }

    #[cfg(feature = "codefacts")]
    fn php_keyspace(dir: &Path) -> std::path::PathBuf {
        let keyspace = dir.join("coreen.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
            {"id":"module:App\\Catalogue","label":"Module","props":{"name":"App\\Catalogue"}},
            {"id":"item:App\\Catalogue\\Course","label":"Item","props":{"kind":"trait","line":3,
             "name":"Course","php_construct":"class_declaration","qname":"App\\Catalogue\\Course"}},
            {"id":"item:App\\Catalogue\\Course::rename","label":"Item","props":{"kind":"fn","line":7,
             "name":"rename","php_construct":"method_declaration",
             "qname":"App\\Catalogue\\Course::rename"}}
            ],"edges":[
            {"src":"item:App\\Catalogue\\Course","dst":"module:App\\Catalogue","label":"IN_MODULE"}
            ]}"#,
        )
        .unwrap();
        keyspace
    }

    #[cfg(feature = "codefacts")]
    fn catalogue_specs(specs: &Path) {
        write(
            specs,
            "contexts/catalogue.md",
            "# catalogue\n\n## Owns\n\n- App\\Catalogue\n",
        );
    }

    #[cfg(feature = "codefacts")]
    fn rust_keyspace(dir: &Path) -> std::path::PathBuf {
        let keyspace = dir.join("rust.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":5,"patch":0},"nodes":[
            {"id":"item:domain::Reader","label":"Item","props":{"kind":"trait","name":"Reader",
             "visibility":"pub","is_test":false,"line":1,"file":"/ws/domain/src/lib.rs",
             "module_qpath":"domain","crate":"domain","bounded_context":"equivalence"}},
            {"id":"item:domain::Walker","label":"Item","props":{"kind":"struct","name":"Walker",
             "visibility":"pub","is_test":false,"line":9,"file":"/ws/domain/src/lib.rs",
             "module_qpath":"domain","crate":"domain","bounded_context":"equivalence"}}
            ],"edges":[]}"#,
        )
        .unwrap();
        keyspace
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_implements_bullet_on_a_rust_keyspace_is_answered_now_not_excused() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/equivalence.md",
            "# equivalence\n\n## Walker\n\n- implements: Reader\n\n## Reader\n",
        );
        let keyspace = dir.path().join("rust.json");
        std::fs::write(
            &keyspace,
            r#"{"schema_version":{"major":0,"minor":5,"patch":0},"nodes":[
            {"id":"item:domain::Reader","label":"Item","props":{"kind":"trait","name":"Reader",
             "qname":"domain::Reader","visibility":"pub","is_test":false,"line":1,
             "file":"/ws/domain/src/lib.rs","module_qpath":"domain","crate":"domain",
             "bounded_context":"equivalence"}},
            {"id":"item:domain::Walker","label":"Item","props":{"kind":"struct","name":"Walker",
             "qname":"domain::Walker","visibility":"pub","is_test":false,"line":9,
             "file":"/ws/domain/src/lib.rs","module_qpath":"domain","crate":"domain",
             "bounded_context":"equivalence"}},
            {"id":"item:domain::Walker::impl_Reader","label":"Item","props":{"kind":"impl_block",
             "name":"impl","qname":"domain::Walker::impl_Reader","visibility":"pub","is_test":false,
             "line":12,"file":"/ws/domain/src/lib.rs","module_qpath":"domain","crate":"domain",
             "bounded_context":"equivalence"}}
            ],"edges":[
            {"src":"item:domain::Walker::impl_Reader","dst":"item:domain::Walker","label":"IMPLEMENTS_FOR"},
            {"src":"item:domain::Walker::impl_Reader","dst":"item:domain::Reader","label":"IMPLEMENTS"}
            ]}"#,
        )
        .unwrap();

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        assert!(
            violations.is_empty(),
            "the impl-block join answers the bullet; nothing is excused and nothing is unmet: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_implements_bullet_the_keyspace_contradicts_is_reported_not_excused() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/equivalence.md",
            "# equivalence\n\n## Walker\n\n- implements: Reader\n\n## Reader\n",
        );
        let keyspace = rust_keyspace(dir.path());

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        assert!(
            violations
                .iter()
                .any(|v| matches!(v, Violation::EdgeMissingInCode { concept, .. } if concept == "Walker")),
            "a keyspace that carries no such impl gives the bullet its ordinary verdict: {violations:?}"
        );
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::EdgeUnanswerable { .. })),
            "and nothing is unanswerable, because the reader answers IMPLEMENTS now: {violations:?}"
        );
    }

    #[test]
    fn the_same_implements_bullet_on_the_source_walk_keeps_its_ordinary_verdict() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/equivalence.md",
            "# equivalence\n\n## Walker\n\n- implements: Reader\n\n## Reader\n",
        );
        write(code.path(), "Cargo.toml", "[package]\nname = \"c\"\n");
        write(
            code.path(),
            "src/lib.rs",
            "pub trait Reader {}\npub struct Walker;\n",
        );

        let violations = run_check(specs.path(), code.path(), None)
            .unwrap()
            .violations;
        assert!(
            violations.iter().any(
                |v| matches!(v, Violation::EdgeMissingInCode { concept, .. } if concept == "Walker")
            ),
            "the walk answers all three kinds, so an unmet bullet is unmet: {violations:?}"
        );
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::EdgeUnanswerable { .. })),
            "and nothing is unanswerable there: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn a_bullet_the_producer_cannot_answer_is_unanswerable_not_missing_in_code() {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        catalogue_specs(specs.path());
        write(
            specs.path(),
            "concepts/catalogue.md",
            "# catalogue\n\n## Course\n\n- depends on: Clock\n\n## Clock\n",
        );
        let keyspace = php_keyspace(dir.path());

        let violations = run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations;
        assert!(
            violations.iter().any(
                |v| matches!(v, Violation::EdgeUnanswerable { concept, edge_kind, .. }
                    if concept == "Course" && *edge_kind == EdgeKind::DependsOn)
            ),
            "the producer emits no field-type fact, so the bullet is unanswered: {violations:?}"
        );
        assert!(
            !violations.iter().any(
                |v| matches!(v, Violation::EdgeMissingInCode { concept, .. } if concept == "Course")
            ),
            "never charged to the specs as unmet: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    fn keyspace_violations(specs_body: &str) -> Vec<Violation> {
        let specs = TempDir::new().unwrap();
        let dir = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        catalogue_specs(specs.path());
        write(specs.path(), "concepts/catalogue.md", specs_body);
        let keyspace = php_keyspace(dir.path());
        run_check(specs.path(), code.path(), Some(&keyspace))
            .unwrap()
            .violations
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn a_verb_bullet_in_the_dialects_own_form_resolves_against_the_keyspace() {
        let violations =
            keyspace_violations("# catalogue\n\n## Course\n\n- verb: Course::rename\n");
        assert!(
            !violations
                .iter()
                .any(|v| matches!(v, Violation::VerbMissingInCode { .. })),
            "`Type::method` is one of the two forms specs/dialect.md admits, and the keyspace carries the method: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn a_verb_bullet_naming_a_method_the_keyspace_lacks_is_reported() {
        let violations =
            keyspace_violations("# catalogue\n\n## Course\n\n- verb: Course::missing\n");
        assert!(
            violations
                .iter()
                .any(|v| matches!(v, Violation::VerbMissingInCode { qname, .. } if qname == "Course::missing")),
            "the pass runs: an unmatched verb in a readable form is reported: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_impl_anchor_in_the_dialects_own_form_resolves_against_the_keyspace() {
        let violations = keyspace_violations(
            "# catalogue\n\n## Course\n\n## Renaming\n\n- impl: Course::rename\n",
        );
        assert!(
            !violations.iter().any(
                |v| matches!(v, Violation::DanglingAnchor { concept, .. } if concept == "Renaming")
            ),
            "the cfdb-backed resolver answers the anchor: {violations:?}"
        );
    }

    #[cfg(feature = "codefacts")]
    #[test]
    fn an_impl_anchor_naming_nothing_dangles_on_the_keyspace_path() {
        let violations =
            keyspace_violations("# catalogue\n\n## Course\n\n## Bogus\n\n- impl: nonexistent\n");
        assert!(
            violations
                .iter()
                .any(|v| matches!(v, Violation::DanglingAnchor { concept, .. } if concept == "Bogus")),
            "the pass runs: an anchor naming nothing dangles, so the absence above is evidence: {violations:?}"
        );
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
