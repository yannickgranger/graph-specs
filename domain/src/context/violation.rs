//! The context-level violation variants (RFC-001 §4).

use crate::{EdgeKind, OwnedUnit, Source};

/// The context-level violation variants. Wrapped inside
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
    /// A `- verb: <qname>` anchor's concept lives in `owning_context`
    /// but the `pub fn` named `qname` belongs to `target_context`
    /// (cross-context verb routing without a matching `Imports` entry).
    CrossVerbUnauthorized {
        concept: String,
        qname: String,
        owning_context: String,
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
            | Self::CrossEdgeUndeclared { concept, .. }
            | Self::CrossVerbUnauthorized { concept, .. } => concept.as_str(),
        }
    }
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
