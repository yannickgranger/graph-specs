use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use cfdb_core::fact::{Edge, Node, PropValue};
use cfdb_core::schema::{EdgeLabel, Label};
use domain::LocationKind;
use domain::{
    ConceptNode, ConceptRef, DeclaredSurface, EdgeKind, OwnedUnit, Provenance, SignatureState,
    Source,
};
use ports::ReaderError;

const CONCEPT_RUNG: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
];

const KNOWN_CONSTRUCTS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "trait_declaration",
    "method_declaration",
    "function_definition",
];

const IN_MODULE: &str = "IN_MODULE";
const IMPORT: &str = "Import";
const CALLS: &str = "CALLS";
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

    #[must_use]
    pub fn concept_rung_items(nodes: &[Node]) -> usize {
        nodes
            .iter()
            .filter(|node| node.label.as_str() == Label::ITEM)
            .filter(|node| prop(node, "php_construct").is_some_and(|c| CONCEPT_RUNG.contains(&c)))
            .count()
    }

    pub fn concepts(
        &self,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<Vec<ConceptNode>, ReaderError> {
        let containers = containers(nodes, edges)?;
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
                        language: crate::node_language(node),
                        path: PathBuf::from(&module),
                        line,
                        provenance: Provenance::empty(),
                        location: LocationKind::Namespace,
                    },
                    SignatureState::Absent,
                )
                .with_provenance(Some(module), Some(unit.to_owned()), None),
            );
        }
        Ok(out)
    }
}

impl PhpEdgeTraversal {
    pub fn relationships(
        &self,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<Vec<domain::Edge>, ReaderError> {
        let containers = containers(nodes, edges)?;
        let mut by_id: HashMap<&str, (&str, Option<&str>)> = HashMap::new();
        for node in nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            let Some(construct) = prop(node, "php_construct") else {
                return Err(unknown_rung(node, None));
            };
            if !CONCEPT_RUNG.contains(&construct) {
                continue;
            }
            let (Some(name), Some(qname)) = (prop(node, "name"), prop(node, "qname")) else {
                continue;
            };
            by_id.insert(node.id.as_str(), (name, self.surface.unit_of(qname)));
        }

        let mut out = Vec::new();
        for edge in edges {
            if edge.label.as_str() != EdgeLabel::IMPLEMENTS {
                continue;
            }
            let (Some((src_name, Some(src_unit))), Some((dst_name, dst_unit))) = (
                by_id.get(edge.src.as_str()).copied(),
                by_id.get(edge.dst.as_str()).copied(),
            ) else {
                continue;
            };
            let module = containers.get(edge.src.as_str()).map_or(src_unit, |m| *m);
            out.push(php_edge(
                (src_name, src_unit),
                EdgeKind::Implements,
                (dst_name, dst_unit),
                module,
                0,
            ));
        }
        out.extend(self.crossings(nodes, edges, &containers));
        Ok(out)
    }

    fn crossings(
        &self,
        nodes: &[Node],
        edges: &[Edge],
        containers: &HashMap<&str, &str>,
    ) -> Vec<domain::Edge> {
        let mut classes: HashMap<&str, (&str, &str, &str)> = HashMap::new();
        let mut testers: HashMap<&str, (&str, &str, &str)> = HashMap::new();
        let mut in_file: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut qname_of: HashMap<&str, &str> = HashMap::new();
        for node in nodes {
            if node.label.as_str() != Label::ITEM {
                continue;
            }
            let Some(qname) = prop(node, "qname") else {
                continue;
            };
            qname_of.insert(node.id.as_str(), qname);
            if !prop(node, "php_construct").is_some_and(|c| CONCEPT_RUNG.contains(&c)) {
                continue;
            }
            let Some(name) = prop(node, "name") else {
                continue;
            };
            if let Some(unit) = self.surface.unit_of(qname) {
                classes.insert(qname, (node.id.as_str(), name, unit));
            } else if let Some(unit) = self.surface.test_unit_of(qname) {
                testers.insert(qname, (node.id.as_str(), name, unit));
            } else {
                continue;
            }
            if let Some(file) = prop(node, "file") {
                in_file.entry(file).or_default().push(qname);
            }
        }
        let owner = |qname: &str| -> Option<String> {
            let class = qname.split_once("::").map_or(qname, |(class, _)| class);
            (classes.contains_key(class) || testers.contains_key(class)).then(|| class.to_owned())
        };

        let mut pairs: BTreeSet<(String, String, usize)> = BTreeSet::new();
        for node in nodes {
            if node.label.as_str() != IMPORT {
                continue;
            }
            let (Some(fqn), Some(file)) = (prop(node, "fqn"), prop(node, "file")) else {
                continue;
            };
            let target = fqn.trim_start_matches('\\');
            if !classes.contains_key(target) {
                continue;
            }
            for source in in_file.get(file).into_iter().flatten() {
                pairs.insert((
                    (*source).to_owned(),
                    target.to_owned(),
                    prop_usize(node, "line"),
                ));
            }
        }
        for edge in edges {
            if edge.label.as_str() != CALLS {
                continue;
            }
            let (Some(src), Some(dst)) = (
                qname_of.get(edge.src.as_str()).and_then(|q| owner(q)),
                qname_of.get(edge.dst.as_str()).and_then(|q| owner(q)),
            ) else {
                continue;
            };
            pairs.insert((src, dst, 0));
        }

        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
        let mut out = Vec::new();
        for (src, dst, line) in &pairs {
            let source = classes
                .get(src.as_str())
                .or_else(|| testers.get(src.as_str()));
            let (Some(&(src_id, src_name, src_unit)), Some(&(_, dst_name, dst_unit))) =
                (source, classes.get(dst.as_str()))
            else {
                continue;
            };
            if src_unit == dst_unit || !seen.insert((src.as_str(), dst.as_str())) {
                continue;
            }
            let module = containers.get(src_id).map_or(src_unit, |m| *m);
            out.push(php_edge(
                (src_name, src_unit),
                EdgeKind::Uses,
                (dst_name, Some(dst_unit)),
                module,
                *line,
            ));
        }
        out
    }
}

fn php_edge(
    (src_name, src_unit): (&str, &str),
    kind: EdgeKind,
    (dst_name, dst_unit): (&str, Option<&str>),
    module: &str,
    line: usize,
) -> domain::Edge {
    domain::Edge {
        source_concept: ConceptRef::resolved(
            src_name.to_owned(),
            None,
            Some(OwnedUnit(src_unit.to_owned())),
        ),
        kind,
        target: ConceptRef::resolved(
            dst_name.to_owned(),
            None,
            dst_unit.map(|u| OwnedUnit(u.to_owned())),
        ),
        raw_target: dst_name.to_owned(),
        source: Source::Code {
            language: domain::CodeLanguage::Php,
            path: PathBuf::from(module),
            line,
            provenance: Provenance {
                module_path: Some(module.to_owned()),
                unit: Some(src_unit.to_owned()),
                context: None,
            },
            location: LocationKind::Namespace,
        },
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

fn containers<'a>(
    nodes: &'a [Node],
    edges: &'a [Edge],
) -> Result<HashMap<&'a str, &'a str>, ReaderError> {
    let named: HashMap<&str, &str> = nodes
        .iter()
        .filter(|n| n.label.as_str() != Label::ITEM)
        .filter_map(|n| prop(n, "name").map(|name| (n.id.as_str(), name)))
        .collect();
    let by_id: HashMap<&str, &Node> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out: HashMap<&str, &str> = HashMap::new();
    let mut modules_seen: HashMap<&str, usize> = HashMap::new();
    for edge in edges {
        let label = edge.label.as_str();
        if label != IN_MODULE && label != IN_CRATE {
            continue;
        }
        let Some(container) = named.get(edge.dst.as_str()) else {
            return Err(malformed_containment(
                by_id.get(edge.src.as_str()).copied(),
                edge.src.as_str(),
                &format!(
                    "its `{label}` edge points at `{}`, which the keyspace's node set does not carry",
                    edge.dst
                ),
            ));
        };
        if label == IN_MODULE {
            let count = modules_seen.entry(edge.src.as_str()).or_insert(0);
            *count += 1;
            if *count > 1 {
                return Err(malformed_containment(
                    by_id.get(edge.src.as_str()).copied(),
                    edge.src.as_str(),
                    "it carries more than one `IN_MODULE` edge, so which module contains it has no answer",
                ));
            }
            out.insert(edge.src.as_str(), container);
        } else {
            out.entry(edge.src.as_str()).or_insert(container);
        }
    }
    Ok(out)
}

fn malformed_containment(node: Option<&Node>, id: &str, cause: &str) -> ReaderError {
    let named = node
        .and_then(|n| prop(n, "qname").or_else(|| prop(n, "name")))
        .unwrap_or(id);
    ReaderError::ParseFailed {
        path: PathBuf::from(named),
        line: 0,
        message: format!(
            "could not run the containment traversal for `{named}`: {cause}; containment on the PHP path is read by traversing `IN_MODULE` and `IN_CRATE` because the PHP `:Item` is prop-less (graph-specs-010-abstraction-level-equivalence#11.5), and a malformed traversal is a could-not-run rather than a silent fallback"
        ),
    }
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
