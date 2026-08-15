//! Spec anchors — non-`pub` spec anchors.
//!
//! An **anchor** lets a spec heading bind to a named code item the concept
//! walk would not otherwise surface — a `pub(crate)` type, a `fn`, or a
//! `const` — so a context can document a concept whose canonical
//! implementation is legitimately not a top-level `pub` type. It is a
//! *redirection of the equivalence target*, never a suppression: the named
//! item must still resolve, and an anchor naming a nonexistent item fires
//! [`crate::Violation::DanglingAnchor`].
//!
//! This module owns the domain types and the two pure decision functions.
//! Parsing the `- impl:` bullet is the markdown adapter's concern; resolving
//! the qname against code is the `AnchorResolver` port's responsibility.

use crate::{Source, Violation};

/// A concept heading explicitly bound to a named code item.
///
/// Shares the verb-bullet qname grammar with [`crate::VerbAnchor`] but is a
/// **distinct** type: a `VerbAnchor` *attributes* a `pub fn` to a context,
/// while a `ConceptAnchor` *redirects* a concept's equivalence target.
/// `concept` names the owning heading; `target` is the qname of the code
/// item; `source` is the spec site for `path:line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptAnchor {
    pub concept: String,
    pub target: String,
    pub source: Source,
}

/// The kind of code item an anchor resolved to.
///
/// The MVP resolves `Type` / `Fn` / `Const` — each a `syn::Item` the reader
/// already visits. Enum-variant resolution is deferred to a later phase
/// (where `kind:"variant"` is native), so the enum is `#[non_exhaustive]` to
/// admit it without a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnchorKind {
    /// A `struct` / `enum` / `trait` / `type` at any visibility.
    Type,
    /// A free `fn` or an impl method at any visibility.
    Fn,
    /// A `const` / `static` at any visibility.
    Const,
}

/// A resolved anchor target — the code item an anchor resolver found for
/// a qname, at any visibility.
///
/// A pure domain type by construction: it carries **no** infrastructure
/// representation — the resolving adapter translates into this shape,
/// keeping the dependency arrow pointing inward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorTarget {
    pub kind: AnchorKind,
    pub source: Source,
}

/// A concept anchor paired with its code-side resolution verdict.
///
/// Built by the application — which resolves each anchor's `target` through
/// the `AnchorResolver` port — and handed to the diff, so the diff engine
/// stays pure and calls no resolver itself. `target` is `Some` when the
/// named item exists in code (at any visibility), `None` when it does not
/// (→ [`Violation::DanglingAnchor`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAnchor {
    pub anchor: ConceptAnchor,
    pub target: Option<AnchorTarget>,
}

/// The pure anchor-match decision.
///
/// Given a concept anchor and the resolver's verdict for its target, return
/// a [`Violation::DanglingAnchor`] when the target did not resolve, or `None`
/// when it did (the concept is satisfied — no `MissingInCode`). Pure: the
/// resolver I/O happens in the adapter; the diff engine calls this with a
/// pre-computed verdict.
#[must_use]
pub fn anchor_violation(
    anchor: &ConceptAnchor,
    resolved: Option<&AnchorTarget>,
) -> Option<Violation> {
    if resolved.is_some() {
        return None;
    }
    Some(Violation::DanglingAnchor {
        concept: anchor.concept.clone(),
        target: anchor.target.clone(),
        spec_source: anchor.source.clone(),
    })
}

/// Whether a `cohesion: behavioral` declaration exempts a context from
/// [`crate::CohesionViolation::ContextWithoutCohesionUnit`].
///
/// The marker is honored **only** when the context carries machine-checkable
/// behavioral substance — at least one `- impl:` / `- verb:` anchor or one
/// `[enforced-by:]` / `[prose-only:]` invariant annotation. A behavioral
/// declaration over an empty file is *not* exempt: it stays a violation, so
/// the marker buys an exemption against demonstrated content, never against
/// emptiness. Pure; the adapter supplies both booleans.
#[must_use]
pub const fn behavioral_exemption_applies(declared_behavioral: bool, has_substance: bool) -> bool {
    declared_behavioral && has_substance
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec_src(line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/intake_validation.md"),
            line,
        }
    }

    fn code_src() -> Source {
        Source::Code {
            path: PathBuf::from("domain/src/intake.rs"),
            line: 7,
        }
    }

    fn anchor() -> ConceptAnchor {
        ConceptAnchor {
            concept: "ValidateIntakeFull".to_owned(),
            target: "validate_intake".to_owned(),
            source: spec_src(3),
        }
    }

    #[test]
    fn concept_anchor_round_trips_its_fields() {
        let a = anchor();
        assert_eq!(a.concept, "ValidateIntakeFull");
        assert_eq!(a.target, "validate_intake");
        assert_eq!(a.source, spec_src(3));
        // Clone + Eq hold.
        assert_eq!(a.clone(), a);
    }

    #[test]
    fn resolved_anchor_yields_no_violation() {
        let target = AnchorTarget {
            kind: AnchorKind::Fn,
            source: code_src(),
        };
        assert_eq!(anchor_violation(&anchor(), Some(&target)), None);
    }

    #[test]
    fn unresolved_anchor_yields_dangling_anchor() {
        let v = anchor_violation(&anchor(), None).expect("dangling");
        match v {
            Violation::DanglingAnchor {
                concept,
                target,
                spec_source,
            } => {
                assert_eq!(concept, "ValidateIntakeFull");
                assert_eq!(target, "validate_intake");
                assert_eq!(spec_source, spec_src(3));
            }
            other => panic!("expected DanglingAnchor, got {other:?}"),
        }
    }

    #[test]
    fn anchor_kind_resolves_each_mvp_kind() {
        // The three source-walk MVP kinds are distinct and Copy.
        for kind in [AnchorKind::Type, AnchorKind::Fn, AnchorKind::Const] {
            let t = AnchorTarget {
                kind,
                source: code_src(),
            };
            assert_eq!(anchor_violation(&anchor(), Some(&t)), None);
        }
    }

    #[test]
    fn behavioral_exemption_requires_marker_and_substance() {
        // The truth table — only marker AND substance exempts.
        assert!(behavioral_exemption_applies(true, true));
        assert!(!behavioral_exemption_applies(true, false)); // empty doctrine file stays a violation
        assert!(!behavioral_exemption_applies(false, true)); // substance without the marker is not exemption
        assert!(!behavioral_exemption_applies(false, false));
    }
}
