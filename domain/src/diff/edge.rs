//! Edge-level (v0.3 relationship) comparison on matched concepts.
//!
//! Two obligation questions meet here and only one of them is this pass's
//! own. The **source** side — whose edges participate at all — is the
//! `matched_concepts` grouping below. The **target** side is RFC-015 §3.4's
//! `unpointable`, stated in [`super::obligation`] and consumed here: no
//! heading bears a code-existence demand made of it by *another* heading's
//! declarations.

use crate::{Edge, Violation};
use std::collections::{HashMap, HashSet};

/// Compare edge sets per matched concept. Opt-in semantics: only concepts
/// with ≥1 spec-side edge participate. Spec-declared targets that are not
/// in `known_concepts` short-circuit to [`Violation::EdgeTargetUnknown`]
/// (they cannot be "missing in code" — the target is a project mirage).
pub(super) fn edge_diff(
    spec_edges: Vec<Edge>,
    code_edges: Vec<Edge>,
    known_concepts: &HashSet<String>,
    matched_concepts: &HashSet<String>,
    unpointable: &HashSet<String>,
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
            out,
        );
    }
}

/// Group edges by their `source_concept`, filtering out any whose owner is
/// not in `matched_concepts` — edges on spec-only or code-only concepts are
/// suppressed because the concept-level violation already reports the gap,
/// and emitting edge violations on top would double-count.
fn group_by_matched_concept(
    edges: Vec<Edge>,
    matched_concepts: &HashSet<String>,
) -> HashMap<String, Vec<Edge>> {
    edges
        .into_iter()
        .filter(|e| matched_concepts.contains(&e.source_concept))
        .fold(HashMap::new(), |mut acc, e| {
            acc.entry(e.source_concept.clone()).or_default().push(e);
            acc
        })
}

/// Compare the two per-concept edge sets. Runs once per concept; keeps the
/// outer [`edge_diff`] under the complexity ceiling.
fn compare_edges(
    spec: Vec<Edge>,
    code: Vec<Edge>,
    known_concepts: &HashSet<String>,
    unpointable: &HashSet<String>,
    out: &mut Vec<Violation>,
) {
    let spec_matched: Vec<bool> = spec
        .iter()
        .map(|s| {
            code.iter()
                .any(|c| c.kind == s.kind && c.target == s.target)
        })
        .collect();
    let code_matched: Vec<bool> = code
        .iter()
        .map(|c| {
            spec.iter()
                .any(|s| s.kind == c.kind && s.target == c.target)
        })
        .collect();

    for (spec_edge, matched) in spec.into_iter().zip(spec_matched) {
        if unpointable.contains(&spec_edge.target) {
            // RFC-015 §3.4, target side. Checked BEFORE the known-target
            // short-circuit: an unpointable name is declared in specs, so it
            // is in `known_concepts` and would otherwise fall straight to
            // `EdgeMissingInCode`. `known_concepts` itself is never filtered
            // — the name is still declared, so a bullet aimed at it is not
            // aiming at a mirage, and `EdgeTargetUnknown` keeps its meaning.
            continue;
        }
        if !known_concepts.contains(&spec_edge.target) {
            out.push(Violation::EdgeTargetUnknown {
                concept: spec_edge.source_concept,
                edge_kind: spec_edge.kind,
                target: spec_edge.target,
                spec_source: spec_edge.source,
            });
        } else if !matched {
            out.push(Violation::EdgeMissingInCode {
                concept: spec_edge.source_concept,
                edge_kind: spec_edge.kind,
                target: spec_edge.target,
                spec_source: spec_edge.source,
            });
        }
    }

    // Invariant 5 — `EdgeMissingInSpec` is untouched by the exemption, on
    // either endpoint. The rule is one-directional: code may not carry a
    // relationship the specs do not declare, whatever the target's marker or
    // polarity says.
    for (code_edge, matched) in code.into_iter().zip(code_matched) {
        if !matched {
            out.push(Violation::EdgeMissingInSpec {
                concept: code_edge.source_concept,
                edge_kind: code_edge.kind,
                target: code_edge.target,
                code_source: code_edge.source,
            });
        }
    }
}
