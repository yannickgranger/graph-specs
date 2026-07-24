//! Verb-coverage report record types — per RFC-005 §3.3.

use crate::{ContextPattern, Source};

/// A top-level `pub fn` declaration found in code — the verb counterpart
/// to [`crate::ConceptNode`] (which captures pub types). Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubFnDecl {
    pub name: String,
    pub source: Source,
    /// The `OwnedUnit` key (crate path) for context-membership lookup.
    /// `None` when the file is outside any discovered crate root.
    pub owned_unit: Option<String>,
}

/// Enforcement tier derived from an `[enforced-by:]` artifact path, or
/// `ProseOnly` for `[prose-only:]` waivers. Per RFC-005 §3.3.
///
/// `#[non_exhaustive]` per RFC-005 §3.3 + solid §5.3 finding 3 — mirrors
/// `ContextPattern`'s forward-compatibility stance (RFC-001 §3.7).
/// RFC-006 may add `BehaviorTest`; downstream `match` arms require `_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TierKind {
    Cypher,
    Tier0,
    ScriptFence,
    ProseOnly,
}

/// A parsed `[enforced-by:...]` or `[prose-only:...]` bracketed annotation
/// extracted from a spec `#### Operational invariants` bullet. Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantAnnotation {
    /// Invariant identifier — text preceding the bracket in the bullet
    /// (trimmed). May be empty when the bracket begins the bullet.
    pub inv_id: String,
    pub tier: TierKind,
    /// Artifact path cited in `[enforced-by: <artifact>; ...]`. `None` for
    /// `prose-only` waivers.
    pub artifact: Option<String>,
    /// `retire-when` predicate from `[enforced-by: ...; retire-when: ...]`.
    pub retire_when: Option<String>,
    /// Waiver rationale from `[prose-only: <why>]`.
    pub prose_only_why: Option<String>,
    pub source: Source,
}

/// Report record: one `pub fn` in code, its bounded context (if known),
/// and whether any spec section cites it by name. Per RFC-005 §3.3.
///
/// `context: None` is the report-mode analog of
/// `ContextViolation::MembershipUnknown` — the fn lives in a crate not
/// declared under any context's `Owns` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbCoverageRecord {
    pub context: Option<String>,
    pub pub_fn: PubFnDecl,
    pub cited: bool,
}

/// Report record: annotation count per enforcement tier, partitioned by
/// bounded context. Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierHistogramRecord {
    pub context: Option<String>,
    pub tier: TierKind,
    pub count: usize,
}

/// Single context appearance in a [`HomonymRecord`]. Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomonymAppearance {
    pub context_name: String,
    /// Derived via the exporter-wins algorithm (RFC-005 §3.3 dry-run DDD-B):
    /// the exporting context's pattern is authoritative; `None` means the
    /// name is undeclared in either direction for this context.
    pub sanctioned_by_pattern: Option<ContextPattern>,
    /// `true` when a context exports and imports the same name under
    /// disagreeing patterns — per RFC-001 §4 invariant 5, asymmetric
    /// declarations are legal input; this flag preserves the signal without
    /// auto-resolving.
    pub asymmetric: bool,
}

/// A name (pub fn or pub type) that appears in more than one bounded
/// context. Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomonymRecord {
    pub name: String,
    pub contexts: Vec<HomonymAppearance>,
}

/// Aggregated output of the verb-coverage report. Per RFC-005 §3.3.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportOutput {
    pub verb_coverage: Vec<VerbCoverageRecord>,
    pub tier_histogram: Vec<TierHistogramRecord>,
    pub homonyms: Vec<HomonymRecord>,
}
