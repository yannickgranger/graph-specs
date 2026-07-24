//! Verb-coverage report types and pure transformation — per RFC-005 §3.3.
//!
//! All types here are pure value objects whose owning context is
//! `equivalence`, parallel to [`crate::ConceptNode`] / [`crate::Edge`].
//! The orchestration that calls readers lives in `application::report`
//! (Slice B); this module receives pre-materialised inputs only.

mod context_map;
mod coverage;
mod homonym;
mod tier_histogram;
mod types;

#[cfg(test)]
mod tests;

use crate::CheckInput;
use context_map::build_unit_context_map;
use coverage::build_verb_coverage;
use homonym::build_homonyms;
use tier_histogram::build_tier_histogram;

pub use types::{
    HomonymAppearance, HomonymRecord, InvariantAnnotation, PubFnDecl, ReportOutput,
    TierHistogramRecord, TierKind, VerbCoverageRecord,
};

/// Pure transformation — per RFC-005 §4 Invariant 8, MUST NOT invoke any reader.
///
/// Receives pre-materialised inputs assembled by the application orchestrator
/// (`application::report`, Slice B), parallel to [`crate::diff`].
///
/// # Arguments
///
/// * `check_input` — spec graph + bounded-context declarations (assembled
///   by the orchestrator before calling this function, same as for `diff`).
/// * `pub_fns` — all top-level `pub fn` declarations from the code tree,
///   as produced by `VerbReader::extract_pub_fns`.
/// * `annotations` — all `[enforced-by:]` / `[prose-only:]` annotations
///   from spec `#### Operational invariants` sections, as produced by
///   `MarkdownReader::extract_invariant_annotations`.
#[must_use]
pub fn report_verb_coverage(
    check_input: &CheckInput,
    pub_fns: &[PubFnDecl],
    annotations: &[InvariantAnnotation],
) -> ReportOutput {
    let unit_to_context = build_unit_context_map(&check_input.contexts);

    let spec_names: std::collections::HashSet<&str> = check_input
        .graph
        .nodes
        .iter()
        .map(|n| n.name.as_str())
        .collect();

    let verb_coverage = build_verb_coverage(pub_fns, &unit_to_context, &spec_names);
    let tier_histogram = build_tier_histogram(annotations);
    let homonyms = build_homonyms(pub_fns, &check_input.contexts, &unit_to_context);

    ReportOutput {
        verb_coverage,
        tier_histogram,
        homonyms,
    }
}
