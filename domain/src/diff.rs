mod code_index;
mod cohesion;
mod concept;
mod context;
mod edge;
mod obligation;
mod signature;
mod verb;

#[cfg(test)]
mod tests;

use crate::context::context_for_code_node;
use crate::{
    anchor_violation, CheckInput, CheckOutcome, ConceptNode, ContextDecl, Graph, Polarity, Source,
    Violation,
};
use code_index::CodeIndex;
use concept::{concept_pass, AnchorResolutions, MarkerRecords};
use std::collections::HashSet;

fn resolve_code_contexts(nodes: Vec<ConceptNode>, contexts: &[ContextDecl]) -> Vec<ConceptNode> {
    nodes
        .into_iter()
        .map(|mut node| {
            let resolved = context_for_code_node(&node, contexts).map(|c| c.name.clone());
            if let Source::Code { provenance, .. } = &mut node.source {
                provenance.context = resolved;
            }
            node
        })
        .collect()
}

fn snapshot_declared_contexts(
    spec_nodes: &[ConceptNode],
    spec_contexts: &[ContextDecl],
) -> Vec<(String, String, Source)> {
    if spec_contexts.is_empty() {
        return Vec::new();
    }
    spec_nodes
        .iter()
        .filter_map(|n| {
            n.context()
                .map(|c| (n.name.clone(), c.to_owned(), n.source.clone()))
        })
        .collect()
}

#[must_use]
pub fn diff(spec: CheckInput, code: Graph, answerable: Option<&[crate::EdgeKind]>) -> CheckOutcome {
    let CheckInput {
        graph: specs,
        contexts: spec_contexts,
        surface,
        verb_ownership: spec_verb_ownership,
        spec_cohesion,
        concept_anchors,
        spec_findings,
    } = spec;
    let Graph {
        nodes: spec_nodes,
        edges: spec_edges,
    } = specs;
    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

    let code_nodes = resolve_code_contexts(code_nodes, &spec_contexts);

    let code_for_context = if spec_contexts.is_empty() {
        Graph::default()
    } else {
        Graph::new(code_nodes.clone(), code_edges.clone())
    };
    let code_for_verb = if spec_verb_ownership.anchors.is_empty() {
        Graph::default()
    } else {
        Graph::new(code_nodes.clone(), Vec::new())
    };

    let declared_contexts = snapshot_declared_contexts(&spec_nodes, &spec_contexts);

    let mut code_by_name = CodeIndex::new(code_nodes, !spec_contexts.is_empty());

    let unobliged_concepts: HashSet<String> = spec_nodes
        .iter()
        .filter(|n| n.polarity != Polarity::Declared)
        .map(|n| n.name.clone())
        .collect();

    let (matched_concepts, known_concepts) = snapshot_name_sets(&spec_nodes, &code_by_name);

    let mut violations = spec_findings;
    let mut records = MarkerRecords::default();

    let anchored_concepts = run_anchor_pass(
        concept_anchors,
        &spec_nodes,
        &code_by_name,
        &unobliged_concepts,
        &mut violations,
    );

    let keys = obligation::obligation_keys(&spec_nodes, &code_by_name, &anchored_concepts);

    concept_pass(
        spec_nodes,
        &mut code_by_name,
        &anchored_concepts,
        &mut violations,
        &mut records,
    );

    orphan_pass(code_by_name, &mut violations);

    edge::edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &keys.unpointable,
        answerable,
        &mut violations,
    );

    verb::verb_pass(
        spec_verb_ownership,
        &code_for_verb,
        &spec_contexts,
        &keys.unobliged,
        &mut violations,
    );

    cohesion::cohesion_pass(
        spec_cohesion,
        declared_contexts,
        &code_for_context,
        &spec_contexts,
        &mut violations,
    );

    context::context_pass(spec_contexts, code_for_context, &surface, &mut violations);

    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });
    CheckOutcome::new(
        violations,
        records.pending,
        records.realized,
        records.retirement_incomplete,
        records.retirement_complete,
    )
}

fn snapshot_name_sets(
    spec_nodes: &[ConceptNode],
    code_by_name: &CodeIndex,
) -> (HashSet<String>, HashSet<String>) {
    let matched = spec_nodes
        .iter()
        .filter(|n| n.polarity == Polarity::Declared && code_by_name.contains(&n.name))
        .map(|n| n.name.clone())
        .collect();
    let known = spec_nodes
        .iter()
        .map(|n| n.name.as_str())
        .chain(code_by_name.names())
        .map(str::to_owned)
        .collect();
    (matched, known)
}

fn run_anchor_pass(
    concept_anchors: Vec<crate::ResolvedAnchor>,
    spec_nodes: &[ConceptNode],
    code_by_name: &CodeIndex,
    non_declared: &HashSet<String>,
    violations: &mut Vec<Violation>,
) -> AnchorResolutions {
    let mut suppressed: HashSet<&str> = spec_nodes
        .iter()
        .filter(|n| n.marker.is_marked() && !code_by_name.contains(&n.name))
        .map(|n| n.name.as_str())
        .collect();
    suppressed.extend(non_declared.iter().map(String::as_str));
    anchor_pass(concept_anchors, &suppressed, violations)
}

fn orphan_pass(code_by_name: CodeIndex, violations: &mut Vec<Violation>) {
    for code_node in code_by_name.into_remaining() {
        violations.push(Violation::MissingInSpecs {
            name: code_node.name,
            code_source: code_node.source,
        });
    }
}

fn anchor_pass(
    concept_anchors: Vec<crate::ResolvedAnchor>,
    marked_without_code: &HashSet<&str>,
    violations: &mut Vec<Violation>,
) -> AnchorResolutions {
    let mut anchored = AnchorResolutions::new();
    for resolved in concept_anchors {
        let violation = anchor_violation(&resolved.anchor, resolved.target.as_ref());
        let this_resolved = violation.is_none();
        anchored
            .entry(resolved.anchor.concept.clone())
            .and_modify(|all_resolved| *all_resolved &= this_resolved)
            .or_insert(this_resolved);
        if let Some(v) = violation {
            if !marked_without_code.contains(resolved.anchor.concept.as_str()) {
                violations.push(v);
            }
        }
    }
    anchored
}

const fn violation_key(v: &Violation) -> (&str, u8) {
    match v {
        Violation::MissingInCode { name, .. } => (name.as_str(), 0),
        Violation::MissingInSpecs { name, .. } => (name.as_str(), 1),
        Violation::SignatureDrift { name, .. } => (name.as_str(), 2),
        Violation::SignatureMissingInSpec { name, .. } => (name.as_str(), 3),
        Violation::SignatureUnparseable { name, .. } => (name.as_str(), 4),
        Violation::EdgeMissingInCode { concept, .. } => (concept.as_str(), 5),
        Violation::EdgeMissingInSpec { concept, .. } => (concept.as_str(), 6),
        Violation::EdgeTargetUnknown { concept, .. } => (concept.as_str(), 7),
        Violation::EdgeUnanswerable { concept, .. } => (concept.as_str(), 16),
        Violation::MalformedAnchorBullet { concept, .. } => (concept.as_str(), 17),
        Violation::Context(ctx) => (ctx.concept(), 8),
        Violation::VerbMissingInCode { concept, .. } => (concept.as_str(), 9),
        Violation::VerbMissingInSpec { qname, .. } => (qname.as_str(), 10),
        Violation::VerbTargetUnknown { concept, .. } => (concept.as_str(), 11),
        Violation::Cohesion(c) => (c.key(), 12),
        Violation::SignatureDriftWithinSide { name, .. } => (name.as_str(), 13),
        Violation::UnknownAttributeKey { concept, .. } => (concept.as_str(), 18),
        Violation::DanglingAnchor { concept, .. } => (concept.as_str(), 14),
        Violation::ForbiddenConceptReintroduced { name, .. } => (name.as_str(), 15),
    }
}
