//! Marker records and the widened check outcome — RFC-013 §3.4 / §3.5,
//! widened by RFC-015 §3.2 / §3.5.
//!
//! A **marker record** is not a failure. It reports the state of a concept
//! heading carrying a [`Marker`] (on the heading, or file-wide via front
//! matter). Four kinds, mutually exclusive per heading — the marker value
//! picks the pair, the backing item picks the member.
//!
//! Under `- status: draft`, the concept is declared ahead of its code and
//! ratification is pending:
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
//! Under `- status: retired`, the code is owed to be *gone*, and the marker
//! line is never deleted:
//!
//! - [`RetirementIncompleteRecord`] — retired heading, backing item still
//!   present. Emitted *in addition to* full equivalence enforcement, exactly
//!   as [`RealizedRecord`] is. The window every correct retirement opens.
//! - [`RetirementCompleteRecord`] — retired heading, backing item gone.
//!   Emitted *instead of* [`crate::Violation::MissingInCode`], and carrying
//!   [`PendingRecord`]'s obligation skip in full.
//!
//! "Marker record", never "report record": `report` already names the
//! RFC-005 verb-coverage subcommand and its [`crate::ReportOutput`]
//! aggregate in this same bounded context (RFC-013 §3.4, DDD lens).
//!
//! Both kinds are siblings of [`crate::Violation`] and are produced by
//! [`crate::diff`] itself — the pending-vs-realized decision is the same
//! concept/code matching the diff already performs, so deriving it in the
//! application layer would be a split-brain on one decision.

use crate::{Provenance, Source, Violation};
use std::collections::BTreeMap;
use std::path::Path;

/// Which spec-state marker a concept heading carries (RFC-015 §3.1).
///
/// Two legal values, and **neither transitions to the other**: `draft`
/// declares code owed to *exist* and is deleted at ratification; `retired`
/// declares code owed to be *gone*, is written while the backing item is
/// still present, and is never deleted. RFC-013 §3.1's rationale — *"a
/// presence flag, never a state machine"* — survives the second value
/// intact, because the progress axis is the **code**, not the marker.
///
/// An enum rather than the original `bool` because the concept pass
/// dispatches on the *value* (rows 3/4 versus rows 7/8), while other sites
/// ask only whether a marker is present at all (RFC-015 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Marker {
    /// No marker bullet, and no `status:` front matter — the ordinary
    /// heading. Rows 1 and 2.
    #[default]
    Unmarked,
    /// `- status: draft` — declared ahead of its code. Rows 3 and 4.
    Draft,
    /// `- status: retired` — its code is owed to be gone. Rows 7 and 8.
    Retired,
}

impl Marker {
    /// Whether the heading carries a marker at all.
    ///
    /// The question the anchor-suppression set asks (RFC-015 §3.3): an
    /// unresolved `- impl:` target under **either** value is the state the
    /// marker announces, not a dangling anchor.
    #[must_use]
    pub const fn is_marked(self) -> bool {
        !matches!(self, Self::Unmarked)
    }
}

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

/// A `retired` heading whose backing code item is still present
/// (RFC-015 §3.2 row 7).
///
/// The retirement was announced and the code has not gone yet. Emitted *in
/// addition to* fully enforced equivalence for the pair — a marker never
/// parks a divergence, so a retired heading whose backing item diverges
/// still produces that ordinary violation. Marker/code co-presence is not
/// itself a contradiction: it is the window every correct retirement opens.
///
/// A cleanliness term (RFC-015 §3.5): a clean tree carries none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementIncompleteRecord {
    pub concept: String,
    pub spec_source: Source,
}

/// A `retired` heading with no backing code item (RFC-015 §3.2 row 8).
///
/// The retirement is complete. Emitted *instead of*
/// [`crate::Violation::MissingInCode`], and the heading's own
/// code-obligating declarations impose nothing — row 8 carries row 3's
/// obligation skip in full (RFC-015 §3.2).
///
/// Rendered but **not** a cleanliness term: the marker line is never
/// deleted, so this list never drains, and a never-draining term inside the
/// clean state would make the clean state unreachable (RFC-015 §3.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementCompleteRecord {
    pub concept: String,
    pub spec_source: Source,
}

/// The full result of one equivalence check — violations plus the four
/// marker-record kinds (RFC-013 §3.5, widened by RFC-015 §3.5).
///
/// The exit code is a function of `violations` alone: a tree whose only
/// findings are pending/realized records exits 0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckOutcome {
    pub violations: Vec<Violation>,
    pub pending: Vec<PendingRecord>,
    pub realized: Vec<RealizedRecord>,
    pub retirement_incomplete: Vec<RetirementIncompleteRecord>,
    pub retirement_complete: Vec<RetirementCompleteRecord>,
    /// Containment provenance per code concept, keyed by concept name
    /// (RFC-010 §3.6 / #136). Snapshotted by the diff before the code
    /// nodes are consumed; read by the NDJSON emitter to render the
    /// agnostic triple inside code-kind source objects. A side index on
    /// the outcome rather than fields on [`Violation`] — the enum stays
    /// stable across its ~35 construction sites. A `BTreeMap` so
    /// [`CheckOutcome::empty`] stays `const` and iteration order is
    /// deterministic.
    pub provenance: BTreeMap<String, Provenance>,
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
        mut retirement_incomplete: Vec<RetirementIncompleteRecord>,
        mut retirement_complete: Vec<RetirementCompleteRecord>,
    ) -> Self {
        sort_records(&mut pending);
        sort_records(&mut realized);
        sort_records(&mut retirement_incomplete);
        sort_records(&mut retirement_complete);
        Self {
            violations,
            pending,
            realized,
            retirement_incomplete,
            retirement_complete,
            provenance: BTreeMap::new(),
        }
    }

    /// A check that found nothing at all.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            violations: Vec::new(),
            pending: Vec::new(),
            realized: Vec::new(),
            retirement_incomplete: Vec::new(),
            retirement_complete: Vec::new(),
            provenance: BTreeMap::new(),
        }
    }

    /// `true` when the check found no violations. Pending and realized
    /// records are deliberately not consulted — they never fail a gate.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// The one shape every marker record has: a concept name and the spec site
/// that declared it. Private — it exists so the four kinds sort through one
/// implementation, not to name a domain concept, and a `pub` trait here
/// would put a fifth heading in `specs/concepts/` for plumbing.
trait MarkerRecord {
    fn sort_key(&self) -> (&str, &Path, usize);
}

macro_rules! marker_record {
    ($($t:ty),+ $(,)?) => {$(
        impl MarkerRecord for $t {
            fn sort_key(&self) -> (&str, &Path, usize) {
                record_key(&self.concept, &self.spec_source)
            }
        }
    )+};
}

marker_record!(
    PendingRecord,
    RealizedRecord,
    RetirementIncompleteRecord,
    RetirementCompleteRecord,
);

/// Sort one marker list into its stable order (concept name, then spec site
/// — two headings may share a name across files).
fn sort_records<T: MarkerRecord>(records: &mut [T]) {
    records.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
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
            pending: vec![PendingRecord {
                concept: "Digest".to_owned(),
                spec_source: spec_at("specs/concepts/execution.md", 41),
            }],
            realized: vec![RealizedRecord {
                concept: "InboundAcl".to_owned(),
                spec_source: spec_at("specs/concepts/fleet.md", 120),
            }],
            ..CheckOutcome::empty()
        };
        assert!(outcome.is_clean());
    }

    #[test]
    fn retirement_records_never_make_an_outcome_unclean() {
        // RFC-015 §4 — the exit code stays a function of violations alone,
        // and BOTH retirement kinds are non-violations. Row 7 is a
        // cleanliness term for the human reading the summary; it is not an
        // exit-code term, and that distinction is the whole of §3.5.
        let outcome = CheckOutcome {
            retirement_incomplete: vec![RetirementIncompleteRecord {
                concept: "AssertionScope".to_owned(),
                spec_source: spec_at("specs/concepts/brief_contract.md", 56),
            }],
            retirement_complete: vec![RetirementCompleteRecord {
                concept: "PrePushRebaseDecision".to_owned(),
                spec_source: spec_at("specs/concepts/agent_contract.md", 665),
            }],
            ..CheckOutcome::empty()
        };
        assert!(outcome.is_clean());
    }

    #[test]
    fn marker_is_unmarked_by_default_and_both_values_read_as_marked() {
        // The anchor-suppression set asks only this question (RFC-015 §3.3).
        assert_eq!(Marker::default(), Marker::Unmarked);
        assert!(!Marker::Unmarked.is_marked());
        assert!(Marker::Draft.is_marked());
        assert!(Marker::Retired.is_marked());
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
        let outcome = CheckOutcome::new(Vec::new(), records, Vec::new(), Vec::new(), Vec::new());
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
