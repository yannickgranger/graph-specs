//! Bounded-context equivalence types — v0.4 per RFC-001.
//!
//! This module introduces the vocabulary for declaring bounded contexts,
//! their `Owns` / `Exports` / `Imports` surfaces, and the violation
//! variants emitted by the v0.4 diff context pass (landing in issue #25).
//!
//! The types are pure data — no diff algorithm here. The context pass
//! lives alongside the three existing passes in `diff.rs` and consumes
//! [`CheckInput`] as its spec-side argument.

use crate::{EdgeKind, Graph, Source};
use std::collections::HashMap;

/// A crate, npm package, Go module, or equivalent — named deliberately to
/// keep the domain model language-agnostic across future adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedUnit(pub String);

/// Parsed from `specs/contexts/<name>.md`. `exports` and `imports` model
/// the DDD context-mapping patterns in [`ContextPattern`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextDecl {
    pub name: String,
    pub owned_units: Vec<OwnedUnit>,
    pub exports: Vec<ContextExport>,
    pub imports: Vec<ContextImport>,
    pub source: Source,
}

impl ContextDecl {
    /// Required constructor outside the defining crate — `#[non_exhaustive]`
    /// prevents the struct-literal form in external callers (markdown
    /// adapter, downstream consumers).
    #[must_use]
    pub const fn new(
        name: String,
        owned_units: Vec<OwnedUnit>,
        exports: Vec<ContextExport>,
        imports: Vec<ContextImport>,
        source: Source,
    ) -> Self {
        Self {
            name,
            owned_units,
            exports,
            imports,
            source,
        }
    }
}

/// Export-centric framing (Evans Ch. 14): the supplying context is
/// authoritative about what it publishes. Asymmetric declarations fire
/// [`ContextViolation::CrossEdgeUndeclared`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExport {
    pub concept: String,
    pub pattern: ContextPattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextImport {
    pub from_context: String,
    pub pattern: ContextPattern,
    pub concept: String,
}

/// A DDD context-mapping pattern. v0.4 ships four; Anti-Corruption Layer,
/// Separate Ways, and Open Host Service are deferred to v0.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextPattern {
    SharedKernel,
    CustomerSupplier,
    Conformist,
    PublishedLanguage,
}

impl ContextPattern {
    /// Wire-form label used in violation messages and spec parsing.
    /// Stable across versions.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::SharedKernel => "SharedKernel",
            Self::CustomerSupplier => "CustomerSupplier",
            Self::Conformist => "Conformist",
            Self::PublishedLanguage => "PublishedLanguage",
        }
    }

    /// Canonical iterator over v0.4 variants — the single source of truth
    /// for parsers and error-message enumeration. Adding a v0.5 variant
    /// only requires updating this list and `as_label`.
    #[must_use]
    pub const fn variants() -> &'static [Self] {
        &[
            Self::SharedKernel,
            Self::CustomerSupplier,
            Self::Conformist,
            Self::PublishedLanguage,
        ]
    }
}

impl std::fmt::Display for ContextPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_label())
    }
}

/// The three context-level violation variants. Wrapped inside
/// [`crate::Violation::Context`] so consumers that do not opt into
/// context checking match one arm rather than three.
///
/// Every variant carries a `concept` field so the sort key in
/// `violation_key()` can extract a stable `&str` without destructuring.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextViolation {
    /// A `pub` type in code lives in a crate that is not listed under
    /// any declared context's `Owns` block.
    MembershipUnknown {
        concept: String,
        owned_unit: OwnedUnit,
        code_source: Source,
    },
    /// A v0.3 edge targets a concept in another context that is NOT
    /// listed in the owning context's `Imports`.
    CrossEdgeUnauthorized {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
    /// A v0.3 edge crosses a context boundary and IS listed in the
    /// importing context's `Imports`, but the target context does not
    /// declare the import back as an `Exports` entry (asymmetric
    /// declaration — invariant 5 from RFC-001 §4).
    CrossEdgeUndeclared {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
}

impl ContextViolation {
    /// Sort key used by `violation_key()` — every variant carries a
    /// `concept` field, and this accessor avoids per-variant destructure
    /// at every call site.
    #[must_use]
    pub const fn concept(&self) -> &str {
        match self {
            Self::MembershipUnknown { concept, .. }
            | Self::CrossEdgeUnauthorized { concept, .. }
            | Self::CrossEdgeUndeclared { concept, .. } => concept.as_str(),
        }
    }
}

/// Input to the v0.4 diff on the spec side — concept graph plus
/// declared bounded-context map.
///
/// Keeps [`Graph`] focused on concepts + edges (two reasons to change);
/// contexts are carried alongside (third reason to change) per
/// SOLID lens round-1 RC-1 in RFC-001.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckInput {
    pub graph: Graph,
    pub contexts: Vec<ContextDecl>,
}

impl CheckInput {
    /// An empty `contexts` list reduces v0.4 diff to v0.3 behavior (the
    /// context pass is a no-op).
    #[must_use]
    pub const fn new(graph: Graph, contexts: Vec<ContextDecl>) -> Self {
        Self { graph, contexts }
    }
}

/// Detect a cycle in the import graph over `contexts`, excluding edges
/// classified as [`ContextPattern::SharedKernel`] (RFC-001 §4 invariant 4
/// — Shared Kernel is the one legal form of mutual reference).
///
/// Returns `Some(cycle)` with the names on the cycle (in traversal
/// order), or `None` if the graph is acyclic under the exclusion rule.
/// Callers (the adapter-side `walk_contexts`) surface the cycle as a
/// reader error per invariant 7.
#[must_use]
pub fn detect_import_cycle(contexts: &[ContextDecl]) -> Option<Vec<String>> {
    use std::collections::HashSet;

    let adj: HashMap<&str, Vec<&str>> = contexts
        .iter()
        .map(|c| {
            let edges: Vec<&str> = c
                .imports
                .iter()
                .filter(|i| i.pattern != ContextPattern::SharedKernel)
                .map(|i| i.from_context.as_str())
                .collect();
            (c.name.as_str(), edges)
        })
        .collect();

    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for start in adj.keys() {
        if visited.contains(start) {
            continue;
        }
        if let Some(cycle) = dfs_cycle(start, &adj, &mut visited, &mut stack, &mut path) {
            return Some(cycle.into_iter().map(String::from).collect());
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    visited.insert(node);
    stack.insert(node);
    path.push(node);
    if let Some(neighbours) = adj.get(node) {
        for &next in neighbours {
            if let Some(cycle) = visit_neighbour(next, adj, visited, stack, path) {
                return Some(cycle);
            }
        }
    }
    stack.remove(node);
    path.pop();
    None
}

fn visit_neighbour<'a>(
    next: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    // Import names a context not in the declared set — not a cycle issue;
    // left for the context pass to flag separately.
    if !adj.contains_key(next) {
        return None;
    }
    if stack.contains(next) {
        let start = path.iter().position(|&n| n == next).unwrap_or(0);
        return Some(path[start..].to_vec());
    }
    if visited.contains(next) {
        return None;
    }
    dfs_cycle(next, adj, visited, stack, path)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn src() -> Source {
        Source::Code {
            path: PathBuf::from("some-crate/src/lib.rs"),
            line: 3,
        }
    }

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/reader.md"),
            line: 12,
        }
    }

    #[test]
    fn owned_unit_constructs_and_compares() {
        let a = OwnedUnit("domain".to_string());
        let b = OwnedUnit("domain".to_string());
        let c = OwnedUnit("ports".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn context_pattern_as_label_stable() {
        assert_eq!(ContextPattern::SharedKernel.as_label(), "SharedKernel");
        assert_eq!(
            ContextPattern::CustomerSupplier.as_label(),
            "CustomerSupplier"
        );
        assert_eq!(ContextPattern::Conformist.as_label(), "Conformist");
        assert_eq!(
            ContextPattern::PublishedLanguage.as_label(),
            "PublishedLanguage"
        );
    }

    #[test]
    fn context_pattern_display_matches_label() {
        assert_eq!(format!("{}", ContextPattern::SharedKernel), "SharedKernel");
    }

    #[test]
    fn context_decl_constructs_with_all_sections() {
        let decl = ContextDecl {
            name: "equivalence".to_string(),
            owned_units: vec![
                OwnedUnit("domain".to_string()),
                OwnedUnit("ports".to_string()),
            ],
            exports: vec![ContextExport {
                concept: "Graph".to_string(),
                pattern: ContextPattern::PublishedLanguage,
            }],
            imports: vec![],
            source: spec_src(),
        };
        assert_eq!(decl.name, "equivalence");
        assert_eq!(decl.owned_units.len(), 2);
        assert_eq!(decl.exports[0].concept, "Graph");
    }

    #[test]
    fn membership_unknown_concept_accessor() {
        let v = ContextViolation::MembershipUnknown {
            concept: "Foo".to_string(),
            owned_unit: OwnedUnit("some-crate".to_string()),
            code_source: src(),
        };
        assert_eq!(v.concept(), "Foo");
    }

    #[test]
    fn cross_edge_unauthorized_concept_accessor() {
        let v = ContextViolation::CrossEdgeUnauthorized {
            concept: "MarkdownReader".to_string(),
            owning_context: "reading".to_string(),
            edge_kind: EdgeKind::DependsOn,
            target: "TradingPort".to_string(),
            target_context: "trading".to_string(),
            spec_source: spec_src(),
        };
        assert_eq!(v.concept(), "MarkdownReader");
    }

    #[test]
    fn cross_edge_undeclared_concept_accessor() {
        let v = ContextViolation::CrossEdgeUndeclared {
            concept: "MarkdownReader".to_string(),
            owning_context: "reading".to_string(),
            edge_kind: EdgeKind::Implements,
            target: "Reader".to_string(),
            target_context: "equivalence".to_string(),
            spec_source: spec_src(),
        };
        assert_eq!(v.concept(), "MarkdownReader");
    }

    #[test]
    fn check_input_default_is_empty() {
        let ci = CheckInput::default();
        assert!(ci.graph.nodes.is_empty());
        assert!(ci.graph.edges.is_empty());
        assert!(ci.contexts.is_empty());
    }

    #[test]
    fn check_input_new_wraps_arguments() {
        let g = Graph::empty();
        let ctxs = vec![ContextDecl {
            name: "x".to_string(),
            owned_units: vec![],
            exports: vec![],
            imports: vec![],
            source: spec_src(),
        }];
        let ci = CheckInput::new(g, ctxs);
        assert_eq!(ci.contexts.len(), 1);
        assert_eq!(ci.contexts[0].name, "x");
    }

    #[test]
    fn violation_context_wraps_context_violation() {
        use crate::Violation;
        let inner = ContextViolation::MembershipUnknown {
            concept: "Foo".to_string(),
            owned_unit: OwnedUnit("some-crate".to_string()),
            code_source: src(),
        };
        let outer = Violation::Context(inner.clone());
        assert_eq!(outer, Violation::Context(inner));
    }
}
