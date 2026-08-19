use super::types::{InvariantAnnotation, TierHistogramRecord, TierKind};
use std::collections::HashMap;

pub(super) fn build_tier_histogram(
    annotations: &[InvariantAnnotation],
) -> Vec<TierHistogramRecord> {
    let mut counts: HashMap<TierKind, usize> = HashMap::new();
    for ann in annotations {
        *counts.entry(ann.tier).or_insert(0) += 1;
    }
    let mut records: Vec<TierHistogramRecord> = counts
        .into_iter()
        .map(|(tier, count)| TierHistogramRecord {
            context: None,
            tier,
            count,
        })
        .collect();
    records.sort_by_key(|r| r.tier);
    records
}
