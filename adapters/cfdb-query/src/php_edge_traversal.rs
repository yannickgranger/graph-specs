use std::collections::HashMap;
use std::path::PathBuf;

use cfdb_core::fact::{Edge, Node, PropValue};
use cfdb_core::schema::Label;
use domain::{ConceptNode, DeclaredSurface, SignatureState, Source};
use ports::ReaderError;

const CONCEPT_RUNG: &[&str] = &["class_declaration", "interface_declaration"];

const KNOWN_CONSTRUCTS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "trait_declaration",
    "method_declaration",
    "function_definition",
];

const IN_MODULE: &str = "IN_MODULE";
const IN_CRATE: &str = "IN_CRATE";

#[derive(Debug, Clone)]
pub struct PhpEdgeTraversal {
    surface: DeclaredSurface,
}

impl PhpEdgeTraversal {
    #[must_use]
    pub const fn new(surface: DeclaredSurface) -> Self {
        Self { surface }
    }

    #[must_use]
    pub fn declares_php(nodes: &[Node]) -> bool {
        nodes
            .iter()
            .any(|node| prop(node, "php_construct").is_some())
    }

    pub fn concepts(
        &self,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<Vec<ConceptNode>, ReaderError> {
        let containers = containers(nodes, edges);
        let mut out = Vec::new();
        for node in nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            let Some(construct) = prop(node, "php_construct") else {
                return Err(unknown_rung(node, None));
            };
            if !KNOWN_CONSTRUCTS.contains(&construct) {
                return Err(unknown_rung(node, Some(construct)));
            }
            if !CONCEPT_RUNG.contains(&construct) {
                continue;
            }
            let Some(qname) = prop(node, "qname") else {
                continue;
            };
            let Some(unit) = self.surface.unit_of(qname) else {
                continue;
            };
            let Some(name) = prop(node, "name") else {
                continue;
            };
            let module = containers
                .get(node.id.as_str())
                .map_or_else(|| unit.to_owned(), |m| (*m).to_owned());
            let line = prop_usize(node, "line");
            out.push(
                ConceptNode::new(
                    name.to_owned(),
                    Source::Code {
                        path: PathBuf::from(&module),
                        line,
                    },
                    SignatureState::Absent,
                )
                .with_provenance(Some(module), Some(unit.to_owned()), None),
            );
        }
        Ok(out)
    }
}

fn unknown_rung(node: &Node, construct: Option<&str>) -> ReaderError {
    let named = prop(node, "qname")
        .or_else(|| prop(node, "name"))
        .unwrap_or("<unnamed>");
    let cause = construct.map_or_else(
        || format!("php item `{named}` carries no `php_construct`"),
        |c| format!("php item `{named}` carries the unknown `php_construct` `{c}`"),
    );
    ReaderError::ParseFailed {
        path: PathBuf::from(named),
        line: 0,
        message: format!(
            "{cause}; the concept rung is told by `php_construct` alone (graph-specs-011-php-ladder#3.1) over the producer vocabulary cfdb-045-polyglot-relationship-edges#3.2 records, and a value outside it is refused rather than dropped off the rung in silence"
        ),
    }
}

fn containers<'a>(nodes: &'a [Node], edges: &'a [Edge]) -> HashMap<&'a str, &'a str> {
    let named: HashMap<&str, &str> = nodes
        .iter()
        .filter(|n| n.label.as_str() != Label::ITEM)
        .filter_map(|n| prop(n, "name").map(|name| (n.id.as_str(), name)))
        .collect();
    let mut out: HashMap<&str, &str> = HashMap::new();
    for edge in edges {
        let label = edge.label.as_str();
        if label != IN_MODULE && label != IN_CRATE {
            continue;
        }
        let Some(container) = named.get(edge.dst.as_str()) else {
            continue;
        };
        if label == IN_MODULE {
            out.insert(edge.src.as_str(), container);
        } else {
            out.entry(edge.src.as_str()).or_insert(container);
        }
    }
    out
}

fn prop<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.props.get(key).and_then(PropValue::as_str)
}

fn prop_usize(node: &Node, key: &str) -> usize {
    node.props
        .get(key)
        .and_then(PropValue::as_i64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "php_edge_traversal_tests.rs"]
mod tests;
