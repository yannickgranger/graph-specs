use super::*;
use crate::Provenance;
use crate::{
    CheckInput, ContextDecl, ContextExport, ContextImport, ContextPattern, Graph, OwnedUnit,
    Source, VerbOwnership,
};
use std::path::PathBuf;

fn code_src(line: usize) -> Source {
    Source::Code {
        path: PathBuf::from("some-crate/src/lib.rs"),
        line,
        provenance: Provenance::empty(),
    }
}

fn spec_src(line: usize) -> Source {
    Source::Spec {
        path: PathBuf::from("specs/concepts/core.md"),
        line,
        context: None,
    }
}

fn make_fn(name: &str, owned_unit: Option<&str>) -> PubFnDecl {
    PubFnDecl {
        name: name.to_owned(),
        source: code_src(1),
        owned_unit: owned_unit.map(str::to_owned),
    }
}

fn make_ann(tier: TierKind) -> InvariantAnnotation {
    InvariantAnnotation {
        inv_id: "INV-test".to_owned(),
        tier,
        artifact: None,
        retire_when: None,
        prose_only_why: None,
        source: spec_src(10),
    }
}

fn empty_input() -> CheckInput {
    CheckInput::default()
}

#[test]
fn verb_coverage_none_context_when_unit_unmapped() {
    let input = empty_input();
    let pub_fns = vec![make_fn("run_check", Some("application"))];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert_eq!(out.verb_coverage.len(), 1);
    assert_eq!(out.verb_coverage[0].context, None);
    assert!(!out.verb_coverage[0].cited);
    assert_eq!(out.verb_coverage[0].pub_fn.name, "run_check");
}

#[test]
fn verb_coverage_context_resolved_from_owned_unit() {
    let ctx = ContextDecl::new(
        "orchestration".to_owned(),
        vec![OwnedUnit("application".to_owned())],
        vec![],
        vec![],
        spec_src(1),
    );
    let input = CheckInput::new(Graph::default(), vec![ctx], VerbOwnership::default());
    let pub_fns = vec![make_fn("run_check", Some("application"))];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert_eq!(
        out.verb_coverage[0].context,
        Some("orchestration".to_owned())
    );
}

#[test]
fn verb_coverage_cited_when_spec_has_matching_concept() {
    use crate::{ConceptNode, SignatureState};
    let node = ConceptNode::new("run_check".to_owned(), spec_src(5), SignatureState::Absent);
    let graph = Graph::new(vec![node], vec![]);
    let input = CheckInput::new(graph, vec![], VerbOwnership::default());
    let pub_fns = vec![make_fn("run_check", None)];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert!(out.verb_coverage[0].cited);
}

#[test]
fn tier_histogram_all_four_tiers() {
    let input = empty_input();
    let annotations = vec![
        make_ann(TierKind::Cypher),
        make_ann(TierKind::Cypher),
        make_ann(TierKind::Tier0),
        make_ann(TierKind::ScriptFence),
        make_ann(TierKind::ProseOnly),
        make_ann(TierKind::ProseOnly),
        make_ann(TierKind::ProseOnly),
    ];
    let out = report_verb_coverage(&input, &[], &annotations);
    assert_eq!(out.tier_histogram.len(), 4);
    let cypher = out
        .tier_histogram
        .iter()
        .find(|r| r.tier == TierKind::Cypher)
        .expect("cypher tier present");
    assert_eq!(cypher.count, 2);
    let tier0 = out
        .tier_histogram
        .iter()
        .find(|r| r.tier == TierKind::Tier0)
        .expect("tier0 tier present");
    assert_eq!(tier0.count, 1);
    let script = out
        .tier_histogram
        .iter()
        .find(|r| r.tier == TierKind::ScriptFence)
        .expect("script-fence tier present");
    assert_eq!(script.count, 1);
    let prose = out
        .tier_histogram
        .iter()
        .find(|r| r.tier == TierKind::ProseOnly)
        .expect("prose-only tier present");
    assert_eq!(prose.count, 3);
    assert!(out.tier_histogram[0].tier <= out.tier_histogram[1].tier);
}

#[test]
fn homonym_sanctioned_published_language() {
    let ctx_a = ContextDecl::new(
        "ctx_a".to_owned(),
        vec![OwnedUnit("crate_a".to_owned())],
        vec![ContextExport {
            concept: "Foo".to_owned(),
            pattern: ContextPattern::PublishedLanguage,
        }],
        vec![],
        spec_src(1),
    );
    let ctx_b = ContextDecl::new(
        "ctx_b".to_owned(),
        vec![OwnedUnit("crate_b".to_owned())],
        vec![ContextExport {
            concept: "Foo".to_owned(),
            pattern: ContextPattern::PublishedLanguage,
        }],
        vec![],
        spec_src(2),
    );
    let input = CheckInput::new(
        Graph::default(),
        vec![ctx_a, ctx_b],
        VerbOwnership::default(),
    );
    let pub_fns = vec![
        make_fn("Foo", Some("crate_a")),
        make_fn("Foo", Some("crate_b")),
    ];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert_eq!(out.homonyms.len(), 1);
    let rec = &out.homonyms[0];
    assert_eq!(rec.name, "Foo");
    assert_eq!(rec.contexts.len(), 2);
    for app in &rec.contexts {
        assert_eq!(
            app.sanctioned_by_pattern,
            Some(ContextPattern::PublishedLanguage)
        );
        assert!(!app.asymmetric);
    }
}

#[test]
fn homonym_unsanctioned_no_declaration() {
    let ctx_a = ContextDecl::new(
        "ctx_a".to_owned(),
        vec![OwnedUnit("crate_a".to_owned())],
        vec![],
        vec![],
        spec_src(1),
    );
    let ctx_b = ContextDecl::new(
        "ctx_b".to_owned(),
        vec![OwnedUnit("crate_b".to_owned())],
        vec![],
        vec![],
        spec_src(2),
    );
    let input = CheckInput::new(
        Graph::default(),
        vec![ctx_a, ctx_b],
        VerbOwnership::default(),
    );
    let pub_fns = vec![
        make_fn("Bar", Some("crate_a")),
        make_fn("Bar", Some("crate_b")),
    ];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert_eq!(out.homonyms.len(), 1);
    let rec = &out.homonyms[0];
    for app in &rec.contexts {
        assert_eq!(app.sanctioned_by_pattern, None);
        assert!(!app.asymmetric);
    }
}

#[test]
fn homonym_asymmetric_export_import_disagreement() {
    let ctx_a = ContextDecl::new(
        "ctx_a".to_owned(),
        vec![OwnedUnit("crate_a".to_owned())],
        vec![ContextExport {
            concept: "Baz".to_owned(),
            pattern: ContextPattern::PublishedLanguage,
        }],
        vec![ContextImport {
            from_context: "ctx_b".to_owned(),
            pattern: ContextPattern::Conformist,
            concept: "Baz".to_owned(),
        }],
        spec_src(1),
    );
    let ctx_b = ContextDecl::new(
        "ctx_b".to_owned(),
        vec![OwnedUnit("crate_b".to_owned())],
        vec![],
        vec![],
        spec_src(2),
    );
    let input = CheckInput::new(
        Graph::default(),
        vec![ctx_a, ctx_b],
        VerbOwnership::default(),
    );
    let pub_fns = vec![
        make_fn("Baz", Some("crate_a")),
        make_fn("Baz", Some("crate_b")),
    ];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert_eq!(out.homonyms.len(), 1);
    let app_a = out.homonyms[0]
        .contexts
        .iter()
        .find(|a| a.context_name == "ctx_a")
        .expect("ctx_a appearance present");
    assert_eq!(
        app_a.sanctioned_by_pattern,
        Some(ContextPattern::PublishedLanguage)
    );
    assert!(app_a.asymmetric);
    let app_b = out.homonyms[0]
        .contexts
        .iter()
        .find(|a| a.context_name == "ctx_b")
        .expect("ctx_b appearance present");
    assert_eq!(app_b.sanctioned_by_pattern, None);
    assert!(!app_b.asymmetric);
}

#[test]
fn single_context_fn_is_not_a_homonym() {
    let ctx = ContextDecl::new(
        "solo".to_owned(),
        vec![OwnedUnit("solo_crate".to_owned())],
        vec![],
        vec![],
        spec_src(1),
    );
    let input = CheckInput::new(Graph::default(), vec![ctx], VerbOwnership::default());
    let pub_fns = vec![make_fn("only_here", Some("solo_crate"))];
    let out = report_verb_coverage(&input, &pub_fns, &[]);
    assert!(out.homonyms.is_empty());
}
