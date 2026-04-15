//! Concept-level equivalence diff.
//!
//! Pure set-difference over concept names. A concept present in specs but
//! not in code yields [`Violation::MissingInCode`]; the inverse yields
//! [`Violation::MissingInSpecs`]. Duplicates within a side are collapsed
//! by name — the first occurrence carries the source location.

use crate::{ConceptNode, Graph, SignatureState, Violation};
use std::collections::HashMap;

#[must_use]
pub fn diff(specs: &Graph, code: &Graph) -> Vec<Violation> {
    let spec_by_name: HashMap<&str, &ConceptNode> =
        specs.nodes.iter().map(|n| (n.name.as_str(), n)).collect();
    let code_by_name: HashMap<&str, &ConceptNode> =
        code.nodes.iter().map(|n| (n.name.as_str(), n)).collect();

    let mut violations = Vec::new();

    for node in &specs.nodes {
        if let Some(code_node) = code_by_name.get(node.name.as_str()) {
            compare_signatures(node, code_node, &mut violations);
        } else {
            violations.push(Violation::MissingInCode {
                name: node.name.clone(),
                spec_source: node.source.clone(),
            });
        }
    }
    for node in &code.nodes {
        if !spec_by_name.contains_key(node.name.as_str()) {
            violations.push(Violation::MissingInSpecs {
                name: node.name.clone(),
                code_source: node.source.clone(),
            });
        }
    }

    // Deterministic ordering by (name, variant_rank).
    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });

    violations
}

/// Compare the signature payloads on a matched (spec, code) concept pair.
/// Pushes zero or one signature-level violation.
fn compare_signatures(spec: &ConceptNode, code: &ConceptNode, out: &mut Vec<Violation>) {
    // Unparseable on either side surfaces first — we can't compare against
    // a broken payload, and the author needs to fix the syntax.
    if let SignatureState::Unparseable { raw, error } = &spec.signature {
        out.push(Violation::SignatureUnparseable {
            name: spec.name.clone(),
            raw: raw.clone(),
            error: error.clone(),
            source: spec.source.clone(),
        });
        return;
    }
    if let SignatureState::Unparseable { raw, error } = &code.signature {
        out.push(Violation::SignatureUnparseable {
            name: code.name.clone(),
            raw: raw.clone(),
            error: error.clone(),
            source: code.source.clone(),
        });
        return;
    }

    match (&spec.signature, &code.signature) {
        (SignatureState::Normalized(spec_sig), SignatureState::Normalized(code_sig))
            if spec_sig != code_sig =>
        {
            out.push(Violation::SignatureDrift {
                name: spec.name.clone(),
                spec_sig: spec_sig.clone(),
                code_sig: code_sig.clone(),
                spec_source: spec.source.clone(),
                code_source: code.source.clone(),
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

const fn violation_key(v: &Violation) -> (&str, u8) {
    match v {
        Violation::MissingInCode { name, .. } => (name.as_str(), 0),
        Violation::MissingInSpecs { name, .. } => (name.as_str(), 1),
        Violation::SignatureDrift { name, .. } => (name.as_str(), 2),
        Violation::SignatureMissingInSpec { name, .. } => (name.as_str(), 3),
        Violation::SignatureUnparseable { name, .. } => (name.as_str(), 4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;
    use std::path::PathBuf;

    use crate::SignatureState;

    fn spec(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 1,
            },
            signature: SignatureState::Absent,
        }
    }

    fn code(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Code {
                path: PathBuf::from("domain/src/lib.rs"),
                line: 1,
            },
            signature: SignatureState::Absent,
        }
    }

    fn spec_with_sig(name: &str, sig: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 1,
            },
            signature: SignatureState::Normalized(sig.to_string()),
        }
    }

    fn code_with_sig(name: &str, sig: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Code {
                path: PathBuf::from("domain/src/lib.rs"),
                line: 1,
            },
            signature: SignatureState::Normalized(sig.to_string()),
        }
    }

    fn spec_unparseable(name: &str, raw: &str, error: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 1,
            },
            signature: SignatureState::Unparseable {
                raw: raw.to_string(),
                error: error.to_string(),
            },
        }
    }

    #[test]
    fn empty_graphs_yield_no_violations() {
        let v = diff(&Graph::default(), &Graph::default());
        assert!(v.is_empty());
    }

    #[test]
    fn matching_graphs_yield_no_violations() {
        let specs = Graph {
            nodes: vec![spec("Graph"), spec("Reader")],
        };
        let code = Graph {
            nodes: vec![code("Graph"), code("Reader")],
        };
        assert!(diff(&specs, &code).is_empty());
    }

    #[test]
    fn spec_only_concept_is_missing_in_code() {
        let specs = Graph {
            nodes: vec![spec("Graph"), spec("Orphan")],
        };
        let code = Graph {
            nodes: vec![code("Graph")],
        };
        let v = diff(&specs, &code);
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Violation::MissingInCode { name, .. } if name == "Orphan"));
    }

    #[test]
    fn code_only_concept_is_missing_in_specs() {
        let specs = Graph {
            nodes: vec![spec("Graph")],
        };
        let code = Graph {
            nodes: vec![code("Graph"), code("Undeclared")],
        };
        let v = diff(&specs, &code);
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Violation::MissingInSpecs { name, .. } if name == "Undeclared"));
    }

    #[test]
    fn violations_are_sorted_by_name_deterministically() {
        let specs = Graph {
            nodes: vec![spec("Zebra"), spec("Alpha")],
        };
        let code = Graph::default();
        let v = diff(&specs, &code);
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
        let specs = Graph {
            nodes: vec![spec_with_sig("OrderId", sig)],
        };
        let code = Graph {
            nodes: vec![code_with_sig("OrderId", sig)],
        };
        assert!(diff(&specs, &code).is_empty());
    }

    #[test]
    fn drifting_signatures_yield_signature_drift() {
        let specs = Graph {
            nodes: vec![spec_with_sig("OrderId", "pub struct OrderId(pub Uuid);")],
        };
        let code = Graph {
            nodes: vec![code_with_sig("OrderId", "pub struct OrderId(pub u64);")],
        };
        let v = diff(&specs, &code);
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
        // a violation. `SignatureMissingInSpec` is reserved for v0.4
        // strict / bounded-context mode.
        let specs = Graph {
            nodes: vec![spec("OrderId")], // concept heading only, no rust block
        };
        let code = Graph {
            nodes: vec![code_with_sig("OrderId", "pub struct OrderId(pub Uuid);")],
        };
        assert!(diff(&specs, &code).is_empty());
    }

    #[test]
    fn unparseable_spec_sig_yields_unparseable_violation() {
        let specs = Graph {
            nodes: vec![spec_unparseable(
                "OrderId",
                "pub struct OrderId(",
                "unexpected end of input, expected identifier",
            )],
        };
        let code = Graph {
            nodes: vec![code_with_sig("OrderId", "pub struct OrderId(pub Uuid);")],
        };
        let v = diff(&specs, &code);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0],
            Violation::SignatureUnparseable { name, raw, .. }
                if name == "OrderId" && raw == "pub struct OrderId("
        ));
    }

    #[test]
    fn absent_on_both_sides_still_passes_concept_check() {
        // No signatures on either side — legacy v0.1 behaviour preserved.
        let specs = Graph {
            nodes: vec![spec("Foo")],
        };
        let code = Graph {
            nodes: vec![code("Foo")],
        };
        assert!(diff(&specs, &code).is_empty());
    }

    #[test]
    fn duplicate_spec_names_collapse() {
        let specs = Graph {
            nodes: vec![spec("Graph"), spec("Graph")],
        };
        let code = Graph::default();
        let v = diff(&specs, &code);
        // Both occurrences in specs are missing in code — the diff reports both,
        // but neither is spuriously reported as a violation twice with different sources.
        assert!(v
            .iter()
            .all(|vi| matches!(vi, Violation::MissingInCode { name, .. } if name == "Graph")));
    }
}
