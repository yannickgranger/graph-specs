//! v0.5 verb-anchoring pass — fifth pass in [`crate::diff::diff`].
//!
//! Opt-in per context: a context with zero `- verb:` anchors is not
//! inspected. Once a context is verb-anchored (at least one concept in
//! it carries a `- verb:` bullet), all `pub fn` declarations in that
//! context must be claimed by at least one anchor.
//!
//! Emits three [`crate::Violation`] variants and one
//! [`crate::ContextViolation`] variant:
//! - `VerbMissingInCode` — anchor references a qname absent from code
//! - `VerbTargetUnknown` — qname exists in code but no context claims it
//! - `Context(CrossVerbUnauthorized)` — qname is in a different context
//! - `VerbMissingInSpec` — pub fn in verb-anchored context has no anchor

use crate::{
    context::context_for_concept, ContextDecl, ContextViolation, Graph, VerbAnchor, VerbDecl,
    VerbOwnership, Violation,
};
use std::collections::{HashMap, HashSet};

pub(super) fn verb_pass(
    verb_ownership: &VerbOwnership,
    code: &Graph,
    contexts: &[ContextDecl],
    out: &mut Vec<Violation>,
) {
    if verb_ownership.anchors.is_empty() {
        return;
    }

    let unit_to_context = build_unit_context_map(contexts);
    let decl_by_qname = build_decl_map(&verb_ownership.decls);

    let mut context_claimed_qnames: HashMap<&str, HashSet<&str>> = HashMap::new();

    for anchor in &verb_ownership.anchors {
        let concept_ctx =
            context_for_concept(code, contexts, &anchor.concept).map(|c| c.name.as_str());

        check_anchor(anchor, &decl_by_qname, &unit_to_context, concept_ctx, out);

        if let Some(ctx) = concept_ctx {
            context_claimed_qnames
                .entry(ctx)
                .or_default()
                .insert(anchor.qname.as_str());
        }
    }

    emit_missing_in_spec(
        &verb_ownership.decls,
        &unit_to_context,
        &context_claimed_qnames,
        out,
    );
}

fn build_unit_context_map(contexts: &[ContextDecl]) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for ctx in contexts {
        for unit in &ctx.owned_units {
            map.insert(unit.0.as_str(), ctx.name.as_str());
        }
    }
    map
}

fn build_decl_map(decls: &[VerbDecl]) -> HashMap<&str, Vec<&VerbDecl>> {
    let mut map: HashMap<&str, Vec<&VerbDecl>> = HashMap::new();
    for d in decls {
        map.entry(d.qname.as_str()).or_default().push(d);
    }
    map
}

fn check_anchor(
    anchor: &VerbAnchor,
    decl_by_qname: &HashMap<&str, Vec<&VerbDecl>>,
    unit_to_context: &HashMap<&str, &str>,
    concept_ctx: Option<&str>,
    out: &mut Vec<Violation>,
) {
    let Some(matching_decls) = decl_by_qname.get(anchor.qname.as_str()) else {
        out.push(Violation::VerbMissingInCode {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            spec_source: anchor.source.clone(),
        });
        return;
    };

    let decl_contexts: Vec<&str> = matching_decls
        .iter()
        .filter_map(|d| {
            d.owned_unit
                .as_deref()
                .and_then(|u| unit_to_context.get(u).copied())
        })
        .collect();

    if decl_contexts.is_empty() {
        out.push(Violation::VerbTargetUnknown {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            spec_source: anchor.source.clone(),
        });
        return;
    }

    let Some(concept_ctx_name) = concept_ctx else {
        return;
    };

    if decl_contexts.contains(&concept_ctx_name) {
        return;
    }

    let target_ctx = decl_contexts[0];
    out.push(Violation::Context(
        ContextViolation::CrossVerbUnauthorized {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            owning_context: concept_ctx_name.to_owned(),
            target_context: target_ctx.to_owned(),
            spec_source: anchor.source.clone(),
        },
    ));
}

fn emit_missing_in_spec(
    decls: &[VerbDecl],
    unit_to_context: &HashMap<&str, &str>,
    context_claimed_qnames: &HashMap<&str, HashSet<&str>>,
    out: &mut Vec<Violation>,
) {
    for decl in decls {
        let Some(unit) = decl.owned_unit.as_deref() else {
            continue;
        };
        let Some(&decl_ctx) = unit_to_context.get(unit) else {
            continue;
        };
        let Some(claimed) = context_claimed_qnames.get(decl_ctx) else {
            continue;
        };
        if !claimed.contains(decl.qname.as_str()) {
            out.push(Violation::VerbMissingInSpec {
                qname: decl.qname.clone(),
                code_source: decl.source.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConceptNode, OwnedUnit, Source};
    use std::path::PathBuf;

    fn code_src(path: &str) -> Source {
        Source::Code {
            path: PathBuf::from(path),
            line: 1,
        }
    }

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/core.md"),
            line: 10,
        }
    }

    fn make_ctx(name: &str, units: &[&str]) -> ContextDecl {
        ContextDecl::new(
            name.to_owned(),
            units.iter().map(|u| OwnedUnit((*u).to_owned())).collect(),
            vec![],
            vec![],
            spec_src(),
        )
    }

    fn make_code_node(name: &str, unit: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_owned(),
            source: code_src(&format!("{unit}/src/lib.rs")),
            signature: crate::SignatureState::Absent,
        }
    }

    fn make_decl(qname: &str, unit: &str) -> VerbDecl {
        VerbDecl {
            qname: qname.to_owned(),
            owned_unit: Some(unit.to_owned()),
            source: code_src(&format!("{unit}/src/lib.rs")),
        }
    }

    fn make_anchor(concept: &str, qname: &str) -> VerbAnchor {
        VerbAnchor {
            concept: concept.to_owned(),
            qname: qname.to_owned(),
            raw_target: format!("verb: {qname}"),
            source: spec_src(),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn run(
        anchors: Vec<VerbAnchor>,
        decls: Vec<VerbDecl>,
        code_nodes: Vec<ConceptNode>,
        contexts: Vec<ContextDecl>,
    ) -> Vec<Violation> {
        let code = Graph::new(code_nodes, vec![]);
        let verb_ownership = VerbOwnership { decls, anchors };
        let mut out = Vec::new();
        verb_pass(&verb_ownership, &code, &contexts, &mut out);
        out
    }

    #[test]
    fn empty_anchors_is_noop() {
        let ctx = make_ctx("eq", &["domain"]);
        let decl = make_decl("diff", "domain");
        let node = make_code_node("Graph", "domain");
        let violations = run(vec![], vec![decl], vec![node], vec![ctx]);
        assert!(violations.is_empty());
    }

    #[test]
    fn verb_missing_in_code_when_no_decl_matches() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "nonexistent_fn");
        let node = make_code_node("Graph", "domain");
        let violations = run(vec![anchor], vec![], vec![node], vec![ctx]);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            Violation::VerbMissingInCode { qname, .. } if qname == "nonexistent_fn"
        ));
    }

    #[test]
    fn verb_target_unknown_when_decl_has_no_context() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "orphan_fn");
        let node = make_code_node("Graph", "domain");
        let decl = VerbDecl {
            qname: "orphan_fn".to_owned(),
            owned_unit: Some("unknown_crate".to_owned()),
            source: code_src("unknown_crate/src/lib.rs"),
        };
        let violations = run(vec![anchor], vec![decl], vec![node], vec![ctx]);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            Violation::VerbTargetUnknown { qname, .. } if qname == "orphan_fn"
        ));
    }

    #[test]
    fn cross_verb_unauthorized_when_decl_in_different_context() {
        let ctx_a = make_ctx("ctx_a", &["crate_a"]);
        let ctx_b = make_ctx("ctx_b", &["crate_b"]);
        let anchor = make_anchor("ConceptA", "fn_in_b");
        let node_a = make_code_node("ConceptA", "crate_a");
        let decl = make_decl("fn_in_b", "crate_b");
        let violations = run(vec![anchor], vec![decl], vec![node_a], vec![ctx_a, ctx_b]);
        assert_eq!(violations.len(), 1);
        assert!(
            matches!(
                &violations[0],
                Violation::Context(ContextViolation::CrossVerbUnauthorized {
                    qname, owning_context, target_context, ..
                }) if qname == "fn_in_b"
                    && owning_context == "ctx_a"
                    && target_context == "ctx_b"
            ),
            "expected CrossVerbUnauthorized, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn same_context_verb_anchor_is_valid() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl = make_decl("diff", "domain");
        let violations = run(vec![anchor], vec![decl], vec![node], vec![ctx]);
        assert!(
            violations.is_empty(),
            "expected no violations, got: {violations:?}"
        );
    }

    #[test]
    fn verb_missing_in_spec_when_unclaimed_fn_in_anchored_context() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl_diff = make_decl("diff", "domain");
        let decl_unclaimed = make_decl("unclaimed_fn", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_diff, decl_unclaimed],
            vec![node],
            vec![ctx],
        );
        assert_eq!(violations.len(), 1);
        assert!(
            matches!(&violations[0], Violation::VerbMissingInSpec { qname, .. } if qname == "unclaimed_fn"),
            "expected VerbMissingInSpec for unclaimed_fn, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn context_with_no_anchors_not_inspected_for_missing_in_spec() {
        let ctx_anchored = make_ctx("eq", &["domain"]);
        let ctx_not_anchored = make_ctx("other", &["other_crate"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl_diff = make_decl("diff", "domain");
        let decl_other = make_decl("other_fn", "other_crate");
        let violations = run(
            vec![anchor],
            vec![decl_diff, decl_other],
            vec![node],
            vec![ctx_anchored, ctx_not_anchored],
        );
        assert!(
            violations.is_empty(),
            "other_fn is in a non-anchored context and must not fire VerbMissingInSpec: {violations:?}"
        );
    }
}
