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
