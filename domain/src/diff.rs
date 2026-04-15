//! Equivalence diff — concept, signature, and relationship levels.
//!
//! Three passes over the spec and code graphs:
//!
//! 1. **Concept** — set-difference over concept names. A concept present
//!    in specs but not in code yields [`Violation::MissingInCode`]; the
//!    inverse yields [`Violation::MissingInSpecs`]. Duplicates within a
//!    side are collapsed by name — the first occurrence carries the
//!    source location.
//! 2. **Signature** — per matched concept, the v0.2 signature payload is
//!    compared via [`compare_signatures`]. Absent on the spec side is
//!    *opt-out* (no comparison). `SignatureUnparseable` short-circuits a
//!    concept's comparison.
//! 3. **Edge** (v0.3) — per matched concept that has ≥1 spec-side edge,
//!    edge sets are compared. Opt-in per concept: no spec edges means no
//!    edge check for that concept.

use crate::{ConceptNode, Edge, Graph, SignatureState, Violation};
use std::collections::{HashMap, HashSet};

#[must_use]
pub fn diff(specs: Graph, code: Graph) -> Vec<Violation> {
    let Graph {
        nodes: spec_nodes,
        edges: spec_edges,
    } = specs;
    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

    // Index code by name, consuming code_nodes — later lookups remove the
    // match so the remainder is "code-only" (missing in specs).
    let mut code_by_name: HashMap<String, ConceptNode> = code_nodes
        .into_iter()
        .map(|n| (n.name.clone(), n))
        .collect();

    // Name-sets are needed by the edge pass, which runs after spec_nodes
    // is consumed. Snapshot them before the concept/signature loop.
    let matched_concepts: HashSet<String> = spec_nodes
        .iter()
        .filter(|n| code_by_name.contains_key(&n.name))
        .map(|n| n.name.clone())
        .collect();
    let known_concepts: HashSet<String> = spec_nodes
        .iter()
        .map(|n| n.name.as_str())
        .chain(code_by_name.keys().map(String::as_str))
        .map(str::to_owned)
        .collect();

    let mut violations = Vec::new();

    for spec_node in spec_nodes {
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            compare_signatures(spec_node, code_node, &mut violations);
        } else {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
    }
    for (_, code_node) in code_by_name {
        violations.push(Violation::MissingInSpecs {
            name: code_node.name,
            code_source: code_node.source,
        });
    }

    edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &mut violations,
    );

    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });

    violations
}

/// Compare the signature payloads on a matched (spec, code) concept pair.
/// Consumes both sides — each field is moved into the emitted violation
/// rather than cloned.
fn compare_signatures(spec: ConceptNode, code: ConceptNode, out: &mut Vec<Violation>) {
    // Unparseable on either side surfaces first — we can't compare against
    // a broken payload, and the author needs to fix the syntax. Spec side
    // wins the race because broken spec markup is the more common cause.
    if matches!(spec.signature, SignatureState::Unparseable { .. }) {
        if let SignatureState::Unparseable { raw, error } = spec.signature {
            out.push(Violation::SignatureUnparseable {
                name: spec.name,
                raw,
                error,
                source: spec.source,
            });
        }
        return;
    }
    if matches!(code.signature, SignatureState::Unparseable { .. }) {
        if let SignatureState::Unparseable { raw, error } = code.signature {
            out.push(Violation::SignatureUnparseable {
                name: code.name,
                raw,
                error,
                source: code.source,
            });
        }
        return;
    }

    match (spec.signature, code.signature) {
        (SignatureState::Normalized(spec_sig), SignatureState::Normalized(code_sig))
            if spec_sig != code_sig =>
        {
            out.push(Violation::SignatureDrift {
                name: spec.name,
                spec_sig,
                code_sig,
                spec_source: spec.source,
                code_source: code.source,
            });
        }
        // No-op cases:
        //   - Both Absent → concept-only match, v0.1 semantics preserved.
        //   - Both Normalized and equal → signature match.
        //   - Absent vs Normalized (either direction) → spec has not opted
        //     into signature-level for this concept. No comparison is
        //     performed. `SignatureMissingInSpec` is reserved for v0.4
        //     strict / bounded-context mode and is not emitted in v0.2.
        _ => {}
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

/// Compare edge sets per matched concept. Opt-in semantics: only concepts
/// with ≥1 spec-side edge participate. Spec-declared targets that are not
/// in `known_concepts` short-circuit to [`Violation::EdgeTargetUnknown`]
/// (they cannot be "missing in code" — the target is a project mirage).
fn edge_diff(
    spec_edges: Vec<Edge>,
    code_edges: Vec<Edge>,
    known_concepts: &HashSet<String>,
    matched_concepts: &HashSet<String>,
    out: &mut Vec<Violation>,
) {
    let spec_by_concept = group_by_matched_concept(spec_edges, matched_concepts);
    let mut code_by_concept = group_by_matched_concept(code_edges, matched_concepts);

    for (concept, spec_for_concept) in spec_by_concept {
        let code_for_concept = code_by_concept.remove(&concept).unwrap_or_default();
        compare_edges(spec_for_concept, code_for_concept, known_concepts, out);
    }
}

/// Compare the two per-concept edge sets. Runs once per concept; keeps the
/// outer [`edge_diff`] under the complexity ceiling.
fn compare_edges(
    spec: Vec<Edge>,
    code: Vec<Edge>,
    known_concepts: &HashSet<String>,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeKind, SignatureState, Source};
    use std::path::PathBuf;

    fn spec_path() -> PathBuf {
        PathBuf::from("specs/concepts/core.md")
    }
    fn code_path() -> PathBuf {
        PathBuf::from("domain/src/lib.rs")
    }

    fn spec(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 1,
            },
            signature: SignatureState::Absent,
        }
    }
    fn code(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Code {
                path: code_path(),
                line: 1,
            },
            signature: SignatureState::Absent,
        }
    }
    fn spec_with_sig(name: &str, sig: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 1,
            },
            signature: SignatureState::Normalized(sig.to_string()),
        }
    }
    fn code_with_sig(name: &str, sig: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Code {
                path: code_path(),
                line: 1,
            },
            signature: SignatureState::Normalized(sig.to_string()),
        }
    }
    fn spec_unparseable(name: &str, raw: &str, error: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 1,
            },
            signature: SignatureState::Unparseable {
                raw: raw.to_string(),
                error: error.to_string(),
            },
        }
    }

    fn spec_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
        Edge {
            source_concept: concept.to_string(),
            kind,
            target: target.to_string(),
            raw_target: target.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 10,
            },
        }
    }
    fn code_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
        Edge {
            source_concept: concept.to_string(),
            kind,
            target: target.to_string(),
            raw_target: target.to_string(),
            source: Source::Code {
                path: code_path(),
                line: 10,
            },
        }
    }

    fn nodes(ns: Vec<ConceptNode>) -> Graph {
        Graph {
            nodes: ns,
            edges: vec![],
        }
    }

    #[test]
    fn empty_graphs_yield_no_violations() {
        let v = diff(Graph::default(), Graph::default());
        assert!(v.is_empty());
    }

    #[test]
    fn matching_graphs_yield_no_violations() {
        let specs = nodes(vec![spec("Graph"), spec("Reader")]);
        let code = nodes(vec![code("Graph"), code("Reader")]);
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn spec_only_concept_is_missing_in_code() {
        let specs = nodes(vec![spec("Graph"), spec("Orphan")]);
        let code = nodes(vec![code("Graph")]);
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Violation::MissingInCode { name, .. } if name == "Orphan"));
    }

    #[test]
    fn code_only_concept_is_missing_in_specs() {
        let specs = nodes(vec![spec("Graph")]);
        let code = nodes(vec![code("Graph"), code("Undeclared")]);
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Violation::MissingInSpecs { name, .. } if name == "Undeclared"));
    }

    #[test]
    fn violations_are_sorted_by_name_deterministically() {
        let specs = nodes(vec![spec("Zebra"), spec("Alpha")]);
        let code = Graph::default();
        let v = diff(specs, code);
        let names: Vec<&str> = v
            .iter()
            .filter_map(|vi| match vi {
                Violation::MissingInCode { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["Alpha", "Zebra"]);
    }

    // --- v0.2 signature-level tests ---

    #[test]
    fn matching_signatures_yield_no_violations() {
        let sig = "pub struct OrderId(pub Uuid);";
        let specs = nodes(vec![spec_with_sig("OrderId", sig)]);
        let code = nodes(vec![code_with_sig("OrderId", sig)]);
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn drifting_signatures_yield_signature_drift() {
        let specs = nodes(vec![spec_with_sig(
            "OrderId",
            "pub struct OrderId(pub Uuid);",
        )]);
        let code = nodes(vec![code_with_sig(
            "OrderId",
            "pub struct OrderId(pub u64);",
        )]);
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::SignatureDrift { name, spec_sig, code_sig, .. }
                if name == "OrderId"
                    && spec_sig == "pub struct OrderId(pub Uuid);"
                    && code_sig == "pub struct OrderId(pub u64);"
        ));
    }

    #[test]
    fn code_sig_without_spec_sig_is_not_a_v02_violation() {
        // v0.2 semantics: specs opt-in per-concept via a fenced `rust`
        // block. Absence on the spec side means "do not compare" — not
        // a violation.
        let specs = nodes(vec![spec("OrderId")]);
        let code = nodes(vec![code_with_sig(
            "OrderId",
            "pub struct OrderId(pub Uuid);",
        )]);
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn unparseable_spec_sig_yields_unparseable_violation() {
        let specs = nodes(vec![spec_unparseable(
            "OrderId",
            "pub struct OrderId(",
            "unexpected end of input, expected identifier",
        )]);
        let code = nodes(vec![code_with_sig(
            "OrderId",
            "pub struct OrderId(pub Uuid);",
        )]);
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::SignatureUnparseable { name, raw, .. }
                if name == "OrderId" && raw == "pub struct OrderId("
        ));
    }

    #[test]
    fn absent_on_both_sides_still_passes_concept_check() {
        let specs = nodes(vec![spec("Foo")]);
        let code = nodes(vec![code("Foo")]);
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn duplicate_spec_names_collapse() {
        let specs = nodes(vec![spec("Graph"), spec("Graph")]);
        let code = Graph::default();
        let v = diff(specs, code);
        assert!(v
            .iter()
            .all(|vi| matches!(vi, Violation::MissingInCode { name, .. } if name == "Graph")));
    }

    // --- v0.3 edge-level tests ---

    #[test]
    fn concept_without_spec_edges_skips_edge_comparison() {
        // Reader is on both sides, but spec declares no bullet edges.
        // Code emits an IMPLEMENTS edge. Opt-in semantics: no comparison.
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("MarkdownReader")],
            edges: vec![],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("MarkdownReader")],
            edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn matching_edges_yield_no_violations() {
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("MarkdownReader")],
            edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("MarkdownReader")],
            edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        assert!(diff(specs, code).is_empty());
    }

    #[test]
    fn spec_edge_without_code_match_yields_edge_missing_in_code() {
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("MarkdownReader")],
            edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("MarkdownReader")],
            edges: vec![],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::EdgeMissingInCode { concept, edge_kind, target, .. }
                if concept == "MarkdownReader"
                    && *edge_kind == EdgeKind::Implements
                    && target == "Reader"
        ));
    }

    #[test]
    fn code_edge_without_spec_match_yields_edge_missing_in_spec() {
        // MarkdownReader opts in via spec edge → code side's extra edge is
        // reported as missing in spec.
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("MarkdownReader"), spec("Parser")],
            edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("MarkdownReader"), code("Parser")],
            edges: vec![
                code_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
                code_edge("MarkdownReader", EdgeKind::DependsOn, "Parser"),
            ],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::EdgeMissingInSpec { concept, edge_kind, target, .. }
                if concept == "MarkdownReader"
                    && *edge_kind == EdgeKind::DependsOn
                    && target == "Parser"
        ));
    }

    #[test]
    fn spec_edge_with_unknown_target_yields_edge_target_unknown() {
        let specs = Graph {
            nodes: vec![spec("MarkdownReader")],
            edges: vec![spec_edge(
                "MarkdownReader",
                EdgeKind::Implements,
                "NotAConcept",
            )],
        };
        let code = Graph {
            nodes: vec![code("MarkdownReader")],
            edges: vec![],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::EdgeTargetUnknown { concept, target, .. }
                if concept == "MarkdownReader" && target == "NotAConcept"
        ));
    }

    #[test]
    fn edge_target_unknown_suppresses_missing_in_code() {
        // When the target is unknown we emit EdgeTargetUnknown only —
        // reporting both EdgeTargetUnknown and EdgeMissingInCode for the
        // same bullet would double-count.
        let specs = Graph {
            nodes: vec![spec("Reader")],
            edges: vec![spec_edge("Reader", EdgeKind::Implements, "Iterator")],
        };
        let code = Graph {
            nodes: vec![code("Reader")],
            edges: vec![],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Violation::EdgeTargetUnknown { .. }));
    }

    #[test]
    fn edges_on_spec_only_concept_do_not_double_count() {
        // MarkdownReader is spec-only (MissingInCode already fires).
        // Its bullet edges must not also produce EdgeMissingInCode noise.
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("MarkdownReader")],
            edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        let code = Graph {
            nodes: vec![code("Reader")],
            edges: vec![],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::MissingInCode { name, .. } if name == "MarkdownReader"
        ));
    }

    #[test]
    fn edges_on_code_only_concept_are_ignored() {
        // MarkdownReader is code-only. Its emitted edges have no spec side
        // to compare against → no EdgeMissingInSpec. Only MissingInSpecs.
        let specs = Graph {
            nodes: vec![spec("Reader")],
            edges: vec![],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("MarkdownReader")],
            edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::MissingInSpecs { name, .. } if name == "MarkdownReader"
        ));
    }

    #[test]
    fn multi_edge_per_concept_is_diffed_independently() {
        // MarkdownReader specs: implements: Reader + depends on: Parser.
        // Code: implements: Reader + depends on: Tokenizer (different).
        let specs = Graph {
            nodes: vec![
                spec("Reader"),
                spec("Parser"),
                spec("Tokenizer"),
                spec("MarkdownReader"),
            ],
            edges: vec![
                spec_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
                spec_edge("MarkdownReader", EdgeKind::DependsOn, "Parser"),
            ],
        };
        let code = Graph {
            nodes: vec![
                code("Reader"),
                code("Parser"),
                code("Tokenizer"),
                code("MarkdownReader"),
            ],
            edges: vec![
                code_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
                code_edge("MarkdownReader", EdgeKind::DependsOn, "Tokenizer"),
            ],
        };
        let v = diff(specs, code);
        assert_eq!(v.len(), 2);
        // Deterministic ordering: concept name "MarkdownReader" tied,
        // variant rank 5 (EdgeMissingInCode) before 6 (EdgeMissingInSpec).
        assert!(matches!(
            &v[0],
            Violation::EdgeMissingInCode { target, .. } if target == "Parser"
        ));
        assert!(matches!(
            &v[1],
            Violation::EdgeMissingInSpec { target, .. } if target == "Tokenizer"
        ));
    }

    #[test]
    fn same_kind_different_targets_treated_as_separate_edges() {
        let specs = Graph {
            nodes: vec![spec("Reader"), spec("Parser"), spec("Thing")],
            edges: vec![
                spec_edge("Thing", EdgeKind::DependsOn, "Reader"),
                spec_edge("Thing", EdgeKind::DependsOn, "Parser"),
            ],
        };
        let code = Graph {
            nodes: vec![code("Reader"), code("Parser"), code("Thing")],
            edges: vec![
                code_edge("Thing", EdgeKind::DependsOn, "Reader"),
                code_edge("Thing", EdgeKind::DependsOn, "Parser"),
            ],
        };
        assert!(diff(specs, code).is_empty());
    }
}
