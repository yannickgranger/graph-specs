//! Concept-level pass — the first pass of [`crate::diff`].
//!
//! Set-difference over concept names, widened by RFC-013 §3.2 into the
//! four spec-side rows of the enforcement matrix:
//!
//! | Heading | Backing item | Result |
//! |---|---|---|
//! | unmarked | present | signature comparison (unchanged) |
//! | unmarked | absent | [`Violation::MissingInCode`] (unchanged) |
//! | `draft` | present | signature comparison **and** a [`RealizedRecord`] |
//! | `draft` | absent | a [`PendingRecord`], **no** `MissingInCode` |
//! | `retired` | present | signature comparison **and** a [`RetirementIncompleteRecord`] |
//! | `retired` | absent | a [`RetirementCompleteRecord`], **no** `MissingInCode` |
//!
//! The two marker values pick the *pair*; the backing item picks the
//! member. Rows 7 and 8 are RFC-015 §3.2; rows 3 and 4 are unchanged.
//!
//! "Backing item present" is one fact with two spellings: a name-matched
//! code node, or a resolved `- impl:` anchor (RFC-012 §3.4). Row 3 vs row 4
//! and the anchor's resolution outcome are the same question, not two
//! (RFC-013 §3.4).
//!
//! Ahead of all of that sits RFC-014's grounding-polarity guard, which is
//! **terminal**: see [`polarity_guard`].

use super::signature;
use crate::{
    ConceptNode, Marker, PendingRecord, Polarity, RealizedRecord, RetirementCompleteRecord,
    RetirementIncompleteRecord, Source, Violation,
};
use std::collections::HashMap;

/// Whether each anchored concept's `- impl:` targets all resolved — the
/// output of [`super::anchor_pass`]. Absence from the map means the concept
/// declares no anchor at all.
pub(super) type AnchorResolutions = HashMap<String, bool>;

/// The four marker-record lists the concept pass produces, grouped so
/// [`concept_pass`] keeps its parameter list inside the lint ceiling and so
/// [`super::diff`] carries one binding instead of four.
///
/// Deliberately **not** `pub`: this is a collection of sinks, not a domain
/// concept, and a `pub` struct here would owe `specs/concepts/` a heading
/// for plumbing. [`crate::CheckOutcome`] is where these become public, one
/// list per field, which is the shape the wire format already describes.
#[derive(Default)]
pub(super) struct MarkerRecords {
    pub pending: Vec<PendingRecord>,
    pub realized: Vec<RealizedRecord>,
    pub retirement_incomplete: Vec<RetirementIncompleteRecord>,
    pub retirement_complete: Vec<RetirementCompleteRecord>,
}

impl MarkerRecords {
    /// Rows 4 and 7 — a marked heading whose backing item is present. The
    /// record rides *alongside* full equivalence enforcement in both cases;
    /// a marker never parks a divergence.
    fn push_backed(&mut self, marker: Marker, name: &str, source: &Source) {
        match marker {
            Marker::Unmarked => {}
            Marker::Draft => self.realized.push(RealizedRecord {
                concept: name.to_owned(),
                spec_source: source.clone(),
            }),
            Marker::Retired => self.retirement_incomplete.push(RetirementIncompleteRecord {
                concept: name.to_owned(),
                spec_source: source.clone(),
            }),
        }
    }

    /// Rows 3 and 8 — a marked heading with no backing item. Both are
    /// emitted *instead of* [`Violation::MissingInCode`].
    fn push_unbacked(&mut self, marker: Marker, spec_node: ConceptNode) {
        match marker {
            Marker::Unmarked => {}
            Marker::Draft => self.pending.push(PendingRecord {
                concept: spec_node.name,
                spec_source: spec_node.source,
            }),
            Marker::Retired => self.retirement_complete.push(RetirementCompleteRecord {
                concept: spec_node.name,
                spec_source: spec_node.source,
            }),
        }
    }

    /// Route a marked, anchored concept to the record its anchor resolution
    /// selects — resolution is the backing-item fact for an anchored
    /// heading (RFC-012 §3.4).
    fn push_anchored(&mut self, resolved: bool, marker: Marker, spec_node: ConceptNode) {
        if resolved {
            self.push_backed(marker, &spec_node.name, &spec_node.source);
        } else {
            self.push_unbacked(marker, spec_node);
        }
    }
}

/// Run the concept pass, consuming `spec_nodes` and removing each matched
/// name from `code_by_name` (the remainder is the code-only orphan set).
pub(super) fn concept_pass(
    spec_nodes: Vec<ConceptNode>,
    code_by_name: &mut HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
    violations: &mut Vec<Violation>,
    sinks: &mut MarkerRecords,
) {
    for spec_node in spec_nodes {
        // RFC-014 §3.3 — evaluated first, and terminal. A non-`declared`
        // heading never reaches the marked dispatch below, under EITHER
        // marker value (RFC-015 §3.2, the 3×3 product).
        let Some(spec_node) = polarity_guard(spec_node, code_by_name, violations) else {
            continue;
        };
        let marker = spec_node.marker;
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            sinks.push_backed(marker, &spec_node.name, &spec_node.source);
            signature::compare_signatures(spec_node, code_node, violations);
        } else if let Some(&resolved) = anchored.get(&spec_node.name) {
            // Anchored: existence is governed by the anchor, so the anchor's
            // resolution IS the row-3/row-4 (and row-7/row-8) fact. Unmarked
            // anchored concepts are unchanged — a `DanglingAnchor` was
            // already emitted above if the target did not resolve.
            sinks.push_anchored(resolved, marker, spec_node);
        } else if marker.is_marked() {
            sinks.push_unbacked(marker, spec_node);
        } else {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
    }
}

/// The RFC-014 §3.3 grounding-polarity guard — **terminal**, and evaluated
/// before anything else in the loop.
///
/// Returns `Some(spec_node)` to hand a `declared` heading on to the ordinary
/// dispatch, or `None` when polarity has fully decided the outcome:
///
/// | polarity | code absent | code present |
/// |---|---|---|
/// | `forbidden` | clean | [`Violation::ForbiddenConceptReintroduced`] |
/// | `illustrative` | clean | [`Violation::MissingInSpecs`], via the orphan sweep |
///
/// **`illustrative` is a match-attempt gate, not a post-match dispatch.**
/// It must leave `code_by_name` untouched so the code node falls through to
/// the orphan sweep, exactly as upstream filters illustrative names out of
/// the type-binding set *before* matching. Reaching this outcome by removing
/// the node and re-emitting would be a different thing wearing the same
/// output.
///
/// Terminality is why a marked heading with non-`declared` polarity emits no
/// marker record. That is not an arbitrary tiebreak: `marked` exists to relax
/// the code-existence obligation a `declared` heading carries, and neither
/// `forbidden` nor `illustrative` carries one — absence is clean by
/// definition for both. There is nothing for `marked` to relax, so it is
/// structurally inert here rather than out-competed.
fn polarity_guard(
    spec_node: ConceptNode,
    code_by_name: &mut HashMap<String, ConceptNode>,
    violations: &mut Vec<Violation>,
) -> Option<ConceptNode> {
    match spec_node.polarity {
        Polarity::Declared => Some(spec_node),
        Polarity::Forbidden => {
            // Consumed on a hit so the orphan sweep does not *also* report it
            // as undocumented — the heading documents it, as banned.
            if let Some(code_node) = code_by_name.remove(&spec_node.name) {
                violations.push(Violation::ForbiddenConceptReintroduced {
                    name: spec_node.name,
                    spec_source: spec_node.source,
                    code_source: code_node.source,
                });
            }
            None
        }
        // No `remove` — that is the whole point (see the doc comment).
        Polarity::Illustrative => None,
    }
}
