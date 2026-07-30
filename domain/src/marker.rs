//! Marker records and the widened check outcome — RFC-013 §3.4 / §3.5.
//!
//! A **marker record** is not a failure. It reports the state of a concept
//! heading carrying the `- status: draft` marker (or living in a
//! `status: draft` file): the concept is declared ahead of its code, and
//! ratification is pending. Two kinds, mutually exclusive per heading:
//!
//! - [`PendingRecord`] — marked heading, no backing item. Emitted *instead
//!   of* [`crate::Violation::MissingInCode`]; the heading's own
//!   code-obligating declarations (edge bullets, verb anchors, `- impl:`
//!   anchors) impose nothing while it is pending.
//! - [`RealizedRecord`] — marked heading *with* a backing item. Emitted *in
//!   addition to* the normal, fully enforced equivalence checks for that
//!   pair. This is the ratification signal: the marker line is now ready to
//!   be deleted by a human upstream.
//!
//! "Marker record", never "report record": `report` already names the
//! RFC-005 verb-coverage subcommand and its [`crate::ReportOutput`]
//! aggregate in this same bounded context (RFC-013 §3.4, DDD lens).
//!
//! Both kinds are siblings of [`crate::Violation`] and are produced by
//! [`crate::diff`] itself — the pending-vs-realized decision is the same
//! concept/code matching the diff already performs, so deriving it in the
//! application layer would be a split-brain on one decision.

use crate::{Source, Violation};
use std::path::Path;

/// A marked concept heading with no backing code item (RFC-013 §3.2 row 3).
///
/// Emitted instead of [`crate::Violation::MissingInCode`]. The pending list
/// is the transcription worklist the upstream ratification workflow reads
/// every run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRecord {
    pub concept: String,
    pub spec_source: Source,
}

/// A marked concept heading whose backing code item exists (RFC-013 §3.2
/// row 4) — by name match or by `- impl:` anchor resolution, exactly as an
/// unmarked heading binds.
///
/// Emitted *alongside* full equivalence enforcement for the pair: a marker
/// never parks a divergence (RFC-013 §4 invariant 1). The record is the
/// "ready to ratify" signal — ratification is deletion of the marker line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealizedRecord {
    pub concept: String,
    pub spec_source: Source,
}

/// The full result of one equivalence check — violations plus the two
/// marker-record kinds (RFC-013 §3.5).
///
/// The exit code is a function of `violations` alone: a tree whose only
/// findings are pending/realized records exits 0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckOutcome {
    pub violations: Vec<Violation>,
    pub pending: Vec<PendingRecord>,
    pub realized: Vec<RealizedRecord>,
}

impl CheckOutcome {
    /// Assemble an outcome, sorting the two marker lists into their stable
    /// order (concept name, then spec site — two headings may share a name
    /// across files).
    ///
    /// `violations` arrive already ordered by the diff's own violation key,
    /// which ranks by variant as well as by name; sorting them here would
    /// need that key and does not belong in this module.
    #[must_use]
    pub fn new(
        violations: Vec<Violation>,
        mut pending: Vec<PendingRecord>,
        mut realized: Vec<RealizedRecord>,
    ) -> Self {
        pending.sort_by(|a, b| {
            record_key(&a.concept, &a.spec_source).cmp(&record_key(&b.concept, &b.spec_source))
        });
        realized.sort_by(|a, b| {
            record_key(&a.concept, &a.spec_source).cmp(&record_key(&b.concept, &b.spec_source))
        });
        Self {
            violations,
            pending,
            realized,
        }
    }

    /// A check that found nothing at all.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            violations: Vec::new(),
            pending: Vec::new(),
            realized: Vec::new(),
        }
    }

    /// `true` when the check found no violations. Pending and realized
    /// records are deliberately not consulted — they never fail a gate.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Stable ordering key for a marker record: concept name first, then the
/// spec site as tiebreak.
fn record_key<'a>(concept: &'a str, source: &'a Source) -> (&'a str, &'a Path, usize) {
    match source {
        Source::Spec { path, line } | Source::Code { path, line } => {
            (concept, path.as_path(), *line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec_at(path: &str, line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from(path),
            line,
        }
    }

    #[test]
    fn empty_outcome_is_clean() {
        assert!(CheckOutcome::empty().is_clean());
    }

    #[test]
    fn marker_records_never_make_an_outcome_unclean() {
        // RFC-013 §4 invariant 3 — exit code is a function of violations only.
        let outcome = CheckOutcome {
            violations: Vec::new(),
            pending: vec![PendingRecord {
                concept: "Digest".to_owned(),
                spec_source: spec_at("specs/concepts/execution.md", 41),
            }],
            realized: vec![RealizedRecord {
                concept: "InboundAcl".to_owned(),
                spec_source: spec_at("specs/concepts/fleet.md", 120),
            }],
        };
        assert!(outcome.is_clean());
    }

    #[test]
    fn pending_sorts_by_concept_then_source() {
        let records = vec![
            PendingRecord {
                concept: "Beta".to_owned(),
                spec_source: spec_at("b.md", 1),
            },
            PendingRecord {
                concept: "Alpha".to_owned(),
                spec_source: spec_at("z.md", 9),
            },
            PendingRecord {
                concept: "Alpha".to_owned(),
                spec_source: spec_at("a.md", 3),
            },
        ];
        let outcome = CheckOutcome::new(Vec::new(), records, Vec::new());
        let seen: Vec<_> = outcome
            .pending
            .iter()
            .map(|r| {
                let (concept, path, _) = record_key(&r.concept, &r.spec_source);
                (concept, path)
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                ("Alpha", Path::new("a.md")),
                ("Alpha", Path::new("z.md")),
                ("Beta", Path::new("b.md")),
            ]
        );
    }
}
