//! Verb-coverage report types and pure transformation — per RFC-005 §3.3.
//!
//! All types here are pure value objects whose owning context is
//! `equivalence`, parallel to [`crate::ConceptNode`] / [`crate::Edge`].
//! The orchestration that calls readers lives in `application::report`
//! (Slice B); this module receives pre-materialised inputs only.

use crate::{CheckInput, ContextDecl, ContextPattern, Source};
use std::collections::HashMap;

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

fn build_unit_context_map(contexts: &[ContextDecl]) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for ctx in contexts {
        for unit in &ctx.owned_units {
            map.insert(unit.0.as_str(), ctx.name.as_str());
        }
    }
    map
}

fn build_verb_coverage(
    pub_fns: &[PubFnDecl],
    unit_to_context: &HashMap<&str, &str>,
    spec_names: &std::collections::HashSet<&str>,
) -> Vec<VerbCoverageRecord> {
    pub_fns
        .iter()
        .map(|f| {
            let context = f
                .owned_unit
                .as_deref()
                .and_then(|u| unit_to_context.get(u).copied())
                .map(str::to_owned);
            let cited = spec_names.contains(f.name.as_str());
            VerbCoverageRecord {
                context,
                pub_fn: f.clone(),
                cited,
            }
        })
        .collect()
}

fn build_tier_histogram(annotations: &[InvariantAnnotation]) -> Vec<TierHistogramRecord> {
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

fn build_homonyms(
    pub_fns: &[PubFnDecl],
    contexts: &[ContextDecl],
    unit_to_context: &HashMap<&str, &str>,
) -> Vec<HomonymRecord> {
    // Collect all (name, context_name) pairs from pub_fns.
    let mut name_to_ctxs: HashMap<&str, Vec<&str>> = HashMap::new();
    for f in pub_fns {
        if let Some(ctx) = f
            .owned_unit
            .as_deref()
            .and_then(|u| unit_to_context.get(u).copied())
        {
            name_to_ctxs.entry(f.name.as_str()).or_default().push(ctx);
        }
    }

    let mut records = Vec::new();
    for (name, ctx_names) in name_to_ctxs {
        let unique_ctxs: std::collections::HashSet<&str> = ctx_names.into_iter().collect();
        if unique_ctxs.len() <= 1 {
            continue;
        }
        let mut appearances: Vec<HomonymAppearance> = unique_ctxs
            .iter()
            .map(|&ctx_name| {
                let (sanctioned_by_pattern, asymmetric) = sanctioned_by(name, ctx_name, contexts);
                HomonymAppearance {
                    context_name: ctx_name.to_owned(),
                    sanctioned_by_pattern,
                    asymmetric,
                }
            })
            .collect();
        appearances.sort_by(|a, b| a.context_name.cmp(&b.context_name));
        records.push(HomonymRecord {
            name: name.to_owned(),
            contexts: appearances,
        });
    }
    records.sort_by(|a, b| a.name.cmp(&b.name));
    records
}

/// Exporter-wins derivation (RFC-005 §3.3 dry-run DDD-B):
///
/// (i) prefer context C's `ContextExport.pattern` if C exports name N —
///     exporting context is authoritative per Evans Ch. 14;
/// (ii) fall back to C's `ContextImport.pattern` if C imports N;
/// (iii) if both present for N in C but with disagreeing patterns, use the
///      export's pattern and set `asymmetric = true`.
fn sanctioned_by(
    name: &str,
    ctx_name: &str,
    contexts: &[ContextDecl],
) -> (Option<ContextPattern>, bool) {
    let Some(ctx) = contexts.iter().find(|c| c.name == ctx_name) else {
        return (None, false);
    };

    let export_pattern = ctx
        .exports
        .iter()
        .find(|e| e.concept == name)
        .map(|e| e.pattern);
    let import_pattern = ctx
        .imports
        .iter()
        .find(|i| i.concept == name)
        .map(|i| i.pattern);

    match (export_pattern, import_pattern) {
        (Some(ep), Some(ip)) if ep != ip => (Some(ep), true),
        (Some(ep), _) => (Some(ep), false),
        (None, Some(ip)) => (Some(ip), false),
        (None, None) => (None, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CheckInput, ContextDecl, ContextExport, ContextImport, ContextPattern, Graph, OwnedUnit,
        Source, VerbOwnership,
    };
    use std::path::PathBuf;

    fn code_src(line: usize) -> Source {
        Source::Code {
            path: PathBuf::from("some-crate/src/lib.rs"),
            line,
        }
    }

    fn spec_src(line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/core.md"),
            line,
        }
    }

    fn make_fn(name: &str, owned_unit: Option<&str>) -> PubFnDecl {
        PubFnDecl {
            name: name.to_owned(),
            source: code_src(1),
            owned_unit: owned_unit.map(str::to_owned),
        }
    }

    fn make_ann(tier: TierKind) -> InvariantAnnotation {
        InvariantAnnotation {
            inv_id: "INV-test".to_owned(),
            tier,
            artifact: None,
            retire_when: None,
            prose_only_why: None,
            source: spec_src(10),
        }
    }

    fn empty_input() -> CheckInput {
        CheckInput::default()
    }

    // (a) verb-coverage with None context
    #[test]
    fn verb_coverage_none_context_when_unit_unmapped() {
        let input = empty_input();
        let pub_fns = vec![make_fn("run_check", Some("application"))];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert_eq!(out.verb_coverage.len(), 1);
        assert_eq!(out.verb_coverage[0].context, None);
        assert!(!out.verb_coverage[0].cited);
        assert_eq!(out.verb_coverage[0].pub_fn.name, "run_check");
    }

    // (a continued) verb-coverage with a mapped context
    #[test]
    fn verb_coverage_context_resolved_from_owned_unit() {
        let ctx = ContextDecl::new(
            "orchestration".to_owned(),
            vec![OwnedUnit("application".to_owned())],
            vec![],
            vec![],
            spec_src(1),
        );
        let input = CheckInput::new(Graph::default(), vec![ctx], VerbOwnership::default());
        let pub_fns = vec![make_fn("run_check", Some("application"))];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert_eq!(
            out.verb_coverage[0].context,
            Some("orchestration".to_owned())
        );
    }

    // (a) cited=true when fn name appears as a spec concept node
    #[test]
    fn verb_coverage_cited_when_spec_has_matching_concept() {
        use crate::{ConceptNode, SignatureState};
        let node = ConceptNode::new("run_check".to_owned(), spec_src(5), SignatureState::Absent);
        let graph = Graph::new(vec![node], vec![]);
        let input = CheckInput::new(graph, vec![], VerbOwnership::default());
        let pub_fns = vec![make_fn("run_check", None)];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert!(out.verb_coverage[0].cited);
    }

    // (b) tier histogram with all four TierKind values
    #[test]
    fn tier_histogram_all_four_tiers() {
        let input = empty_input();
        let annotations = vec![
            make_ann(TierKind::Cypher),
            make_ann(TierKind::Cypher),
            make_ann(TierKind::Tier0),
            make_ann(TierKind::ScriptFence),
            make_ann(TierKind::ProseOnly),
            make_ann(TierKind::ProseOnly),
            make_ann(TierKind::ProseOnly),
        ];
        let out = report_verb_coverage(&input, &[], &annotations);
        assert_eq!(out.tier_histogram.len(), 4);
        let cypher = out
            .tier_histogram
            .iter()
            .find(|r| r.tier == TierKind::Cypher)
            .unwrap();
        assert_eq!(cypher.count, 2);
        let tier0 = out
            .tier_histogram
            .iter()
            .find(|r| r.tier == TierKind::Tier0)
            .unwrap();
        assert_eq!(tier0.count, 1);
        let script = out
            .tier_histogram
            .iter()
            .find(|r| r.tier == TierKind::ScriptFence)
            .unwrap();
        assert_eq!(script.count, 1);
        let prose = out
            .tier_histogram
            .iter()
            .find(|r| r.tier == TierKind::ProseOnly)
            .unwrap();
        assert_eq!(prose.count, 3);
        // Records sorted by tier discriminant
        assert!(out.tier_histogram[0].tier <= out.tier_histogram[1].tier);
    }

    // (c) homonym with sanctioned vs unsanctioned
    #[test]
    fn homonym_sanctioned_published_language() {
        let ctx_a = ContextDecl::new(
            "ctx_a".to_owned(),
            vec![OwnedUnit("crate_a".to_owned())],
            vec![ContextExport {
                concept: "Foo".to_owned(),
                pattern: ContextPattern::PublishedLanguage,
            }],
            vec![],
            spec_src(1),
        );
        let ctx_b = ContextDecl::new(
            "ctx_b".to_owned(),
            vec![OwnedUnit("crate_b".to_owned())],
            vec![ContextExport {
                concept: "Foo".to_owned(),
                pattern: ContextPattern::PublishedLanguage,
            }],
            vec![],
            spec_src(2),
        );
        let input = CheckInput::new(
            Graph::default(),
            vec![ctx_a, ctx_b],
            VerbOwnership::default(),
        );
        let pub_fns = vec![
            make_fn("Foo", Some("crate_a")),
            make_fn("Foo", Some("crate_b")),
        ];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert_eq!(out.homonyms.len(), 1);
        let rec = &out.homonyms[0];
        assert_eq!(rec.name, "Foo");
        assert_eq!(rec.contexts.len(), 2);
        for app in &rec.contexts {
            assert_eq!(
                app.sanctioned_by_pattern,
                Some(ContextPattern::PublishedLanguage)
            );
            assert!(!app.asymmetric);
        }
    }

    #[test]
    fn homonym_unsanctioned_no_declaration() {
        let ctx_a = ContextDecl::new(
            "ctx_a".to_owned(),
            vec![OwnedUnit("crate_a".to_owned())],
            vec![],
            vec![],
            spec_src(1),
        );
        let ctx_b = ContextDecl::new(
            "ctx_b".to_owned(),
            vec![OwnedUnit("crate_b".to_owned())],
            vec![],
            vec![],
            spec_src(2),
        );
        let input = CheckInput::new(
            Graph::default(),
            vec![ctx_a, ctx_b],
            VerbOwnership::default(),
        );
        let pub_fns = vec![
            make_fn("Bar", Some("crate_a")),
            make_fn("Bar", Some("crate_b")),
        ];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert_eq!(out.homonyms.len(), 1);
        let rec = &out.homonyms[0];
        for app in &rec.contexts {
            assert_eq!(app.sanctioned_by_pattern, None);
            assert!(!app.asymmetric);
        }
    }

    // (d) exporter-wins with asymmetric flag on Export/Import pattern disagreement
    #[test]
    fn homonym_asymmetric_export_import_disagreement() {
        let ctx_a = ContextDecl::new(
            "ctx_a".to_owned(),
            vec![OwnedUnit("crate_a".to_owned())],
            vec![ContextExport {
                concept: "Baz".to_owned(),
                pattern: ContextPattern::PublishedLanguage,
            }],
            vec![ContextImport {
                from_context: "ctx_b".to_owned(),
                pattern: ContextPattern::Conformist,
                concept: "Baz".to_owned(),
            }],
            spec_src(1),
        );
        let ctx_b = ContextDecl::new(
            "ctx_b".to_owned(),
            vec![OwnedUnit("crate_b".to_owned())],
            vec![],
            vec![],
            spec_src(2),
        );
        let input = CheckInput::new(
            Graph::default(),
            vec![ctx_a, ctx_b],
            VerbOwnership::default(),
        );
        let pub_fns = vec![
            make_fn("Baz", Some("crate_a")),
            make_fn("Baz", Some("crate_b")),
        ];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert_eq!(out.homonyms.len(), 1);
        let app_a = out.homonyms[0]
            .contexts
            .iter()
            .find(|a| a.context_name == "ctx_a")
            .unwrap();
        // Export wins; asymmetric because import disagrees
        assert_eq!(
            app_a.sanctioned_by_pattern,
            Some(ContextPattern::PublishedLanguage)
        );
        assert!(app_a.asymmetric);
        let app_b = out.homonyms[0]
            .contexts
            .iter()
            .find(|a| a.context_name == "ctx_b")
            .unwrap();
        assert_eq!(app_b.sanctioned_by_pattern, None);
        assert!(!app_b.asymmetric);
    }

    #[test]
    fn single_context_fn_is_not_a_homonym() {
        let ctx = ContextDecl::new(
            "solo".to_owned(),
            vec![OwnedUnit("solo_crate".to_owned())],
            vec![],
            vec![],
            spec_src(1),
        );
        let input = CheckInput::new(Graph::default(), vec![ctx], VerbOwnership::default());
        let pub_fns = vec![make_fn("only_here", Some("solo_crate"))];
        let out = report_verb_coverage(&input, &pub_fns, &[]);
        assert!(out.homonyms.is_empty());
    }
}
