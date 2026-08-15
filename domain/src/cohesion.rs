//! Cohesion violations.
//!
//! The upward concept→context rung of the ladder. Wrapped by
//! [`crate::Violation::Cohesion`] so consumers that do not opt into
//! cohesion checking match one arm rather than three.
//!
//! Split by **fact-dependency**: the first two variants fire spec-side
//! with zero code facts (source-walk OR cfdb-query); the third is
//! code-fact-gated — it needs a code-side context resolution. The
//! *detection* logic that emits these is separate; this module is
//! the type only.

use crate::Source;
use std::path::PathBuf;

/// A cohesion violation between the declared abstraction ladder and the
/// code's containment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CohesionViolation {
    /// An `H1` context heading with no `H2`/`H3` concept under it — a
    /// bounded context that declares no cohesion unit. Fires spec-side.
    ContextWithoutCohesionUnit { context: String, file: PathBuf },
    /// An `H3` sub-concept with no enclosing `H2` concept (detected by the
    /// R10-2 `TreeAssembler`). Fires spec-side.
    SubConceptOrphan { sub_concept: String, file: PathBuf },
    /// A concept's spec-side declared owning context disagrees with the
    /// context the code resolves it to. `spec_source` is carried so the
    /// text renderer can show `path:line` like every other violation.
    /// Code-fact-gated.
    ConceptContextMismatch {
        concept: String,
        declared: String,
        code_context: String,
        spec_source: Source,
    },
}

impl CohesionViolation {
    /// Stable `&str` sort key used by `violation_key()`. The variants
    /// carry differently-named leading identifiers (`context` /
    /// `sub_concept` / `concept`), so this accessor normalises them to a
    /// single key rather than forcing a per-variant destructure at the
    /// call site. `str::as_str` is `const` on this toolchain, so this
    /// stays `const fn`, which `clippy::nursery` requires.
    #[must_use]
    pub const fn key(&self) -> &str {
        match self {
            Self::ContextWithoutCohesionUnit { context, .. } => context.as_str(),
            Self::SubConceptOrphan { sub_concept, .. } => sub_concept.as_str(),
            Self::ConceptContextMismatch { concept, .. } => concept.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/equivalence.md"),
            line: 42,
        }
    }

    #[test]
    fn context_without_cohesion_unit_key_is_context() {
        let v = CohesionViolation::ContextWithoutCohesionUnit {
            context: "equivalence".to_owned(),
            file: PathBuf::from("specs/concepts/equivalence.md"),
        };
        assert_eq!(v.key(), "equivalence");
    }

    #[test]
    fn sub_concept_orphan_key_is_sub_concept() {
        let v = CohesionViolation::SubConceptOrphan {
            sub_concept: "InnerThing".to_owned(),
            file: PathBuf::from("specs/concepts/equivalence.md"),
        };
        assert_eq!(v.key(), "InnerThing");
    }

    #[test]
    fn concept_context_mismatch_key_is_concept() {
        let v = CohesionViolation::ConceptContextMismatch {
            concept: "MarkdownReader".to_owned(),
            declared: "reading".to_owned(),
            code_context: "equivalence".to_owned(),
            spec_source: spec_src(),
        };
        assert_eq!(v.key(), "MarkdownReader");
    }
}
