use crate::{ContextPattern, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubFnDecl {
    pub name: String,
    pub source: Source,
    pub owned_unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TierKind {
    Cypher,
    Tier0,
    ScriptFence,
    ProseOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantAnnotation {
    pub inv_id: String,
    pub tier: TierKind,
    pub artifact: Option<String>,
    pub retire_when: Option<String>,
    pub prose_only_why: Option<String>,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbCoverageRecord {
    pub context: Option<String>,
    pub pub_fn: PubFnDecl,
    pub cited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierHistogramRecord {
    pub context: Option<String>,
    pub tier: TierKind,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomonymAppearance {
    pub context_name: String,
    pub sanctioned_by_pattern: Option<ContextPattern>,
    pub asymmetric: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomonymRecord {
    pub name: String,
    pub contexts: Vec<HomonymAppearance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportOutput {
    pub verb_coverage: Vec<VerbCoverageRecord>,
    pub tier_histogram: Vec<TierHistogramRecord>,
    pub homonyms: Vec<HomonymRecord>,
}
