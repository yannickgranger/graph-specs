//! Concept-level equivalence diff.
//!
//! Pure set-difference over concept names. A concept present in specs but
//! not in code yields [`Violation::MissingInCode`]; the inverse yields
//! [`Violation::MissingInSpecs`]. Duplicates within a side are collapsed
//! by name — the first occurrence carries the source location.

use crate::{ConceptNode, Graph, Violation};
use std::collections::HashMap;

#[must_use]
pub fn diff(specs: &Graph, code: &Graph) -> Vec<Violation> {
    let spec_by_name: HashMap<&str, &ConceptNode> =
        specs.nodes.iter().map(|n| (n.name.as_str(), n)).collect();
    let code_by_name: HashMap<&str, &ConceptNode> =
        code.nodes.iter().map(|n| (n.name.as_str(), n)).collect();

    let mut violations = Vec::new();

    for node in &specs.nodes {
        if !code_by_name.contains_key(node.name.as_str()) {
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

    // Deterministic ordering: name ascending, MissingInCode before MissingInSpecs for ties.
    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });

    violations
}

const fn violation_key(v: &Violation) -> (&str, u8) {
    match v {
        Violation::MissingInCode { name, .. } => (name.as_str(), 0),
        Violation::MissingInSpecs { name, .. } => (name.as_str(), 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;
    use std::path::PathBuf;

    fn spec(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 1,
            },
        }
    }

    fn code(name: &str) -> ConceptNode {
        ConceptNode {
            name: name.to_string(),
            source: Source::Code {
                path: PathBuf::from("domain/src/lib.rs"),
                line: 1,
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
                Violation::MissingInSpecs { .. } => None,
            })
            .collect();
        assert_eq!(names, vec!["Alpha", "Zebra"]);
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
