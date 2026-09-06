use crate::{EdgeKind, OwnedUnit, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextViolation {
    MembershipUnknown {
        concept: String,
        owned_unit: OwnedUnit,
        code_source: Source,
    },
    CrossEdgeUnauthorized {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
    CrossEdgeUndeclared {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
    CrossVerbUnauthorized {
        concept: String,
        qname: String,
        owning_context: String,
        target_context: String,
        spec_source: Source,
    },
}

impl ContextViolation {
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
            provenance: crate::Provenance::empty(),
        }
    }

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/reader.md"),
            line: 12,
            context: None,
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
