use crate::{Edge, EdgeKind, Violation};
use std::collections::{HashMap, HashSet};

pub(super) fn edge_diff(
    spec_edges: Vec<Edge>,
    code_edges: Vec<Edge>,
    known_concepts: &HashSet<String>,
    matched_concepts: &HashSet<String>,
    unpointable: &HashSet<String>,
    answerable: Option<&[EdgeKind]>,
    out: &mut Vec<Violation>,
) {
    let spec_by_concept = group_by_matched_concept(spec_edges, matched_concepts);
    let mut code_by_concept = group_by_matched_concept(code_edges, matched_concepts);

    for (concept, spec_for_concept) in spec_by_concept {
        let code_for_concept = code_by_concept.remove(&concept).unwrap_or_default();
        compare_edges(
            spec_for_concept,
            code_for_concept,
            known_concepts,
            unpointable,
            answerable,
            out,
        );
    }
}

fn group_by_matched_concept(
    edges: Vec<Edge>,
    matched_concepts: &HashSet<String>,
) -> HashMap<String, Vec<Edge>> {
    edges
        .into_iter()
        .filter(|e| matched_concepts.contains(e.source_concept.name.as_str()))
        .fold(HashMap::new(), |mut acc, e| {
            acc.entry(e.source_concept.name.clone())
                .or_default()
                .push(e);
            acc
        })
}

fn compare_edges(
    spec: Vec<Edge>,
    code: Vec<Edge>,
    known_concepts: &HashSet<String>,
    unpointable: &HashSet<String>,
    answerable: Option<&[EdgeKind]>,
    out: &mut Vec<Violation>,
) {
    let spec_matched: Vec<bool> = spec
        .iter()
        .map(|s| {
            code.iter()
                .any(|c| c.kind == s.kind && c.target.name == s.target.name)
        })
        .collect();
    let code_matched: Vec<bool> = code
        .iter()
        .map(|c| {
            spec.iter()
                .any(|s| s.kind == c.kind && s.target.name == c.target.name)
        })
        .collect();

    for (spec_edge, matched) in spec.into_iter().zip(spec_matched) {
        if unpointable.contains(spec_edge.target.name.as_str()) {
            continue;
        }
        if answerable.is_some_and(|kinds| !kinds.contains(&spec_edge.kind)) {
            out.push(Violation::EdgeUnanswerable {
                concept: spec_edge.source_concept.name,
                edge_kind: spec_edge.kind,
                target: spec_edge.target.name,
                spec_source: spec_edge.source,
            });
            continue;
        }
        if !known_concepts.contains(spec_edge.target.name.as_str()) {
            out.push(Violation::EdgeTargetUnknown {
                concept: spec_edge.source_concept.name,
                edge_kind: spec_edge.kind,
                target: spec_edge.target.name,
                spec_source: spec_edge.source,
            });
        } else if !matched {
            out.push(Violation::EdgeMissingInCode {
                concept: spec_edge.source_concept.name,
                edge_kind: spec_edge.kind,
                target: spec_edge.target.name,
                spec_source: spec_edge.source,
            });
        }
    }

    for (code_edge, matched) in code.into_iter().zip(code_matched) {
        if !matched {
            out.push(Violation::EdgeMissingInSpec {
                concept: code_edge.source_concept.name,
                edge_kind: code_edge.kind,
                target: code_edge.target.name,
                code_source: code_edge.source,
            });
        }
    }
}
