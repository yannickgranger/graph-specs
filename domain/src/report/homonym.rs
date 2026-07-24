//! Homonym pass: names that appear in more than one bounded context, with
//! exporter-wins pattern sanctioning (RFC-005 §3.3 dry-run DDD-B).

use super::types::{HomonymAppearance, HomonymRecord, PubFnDecl};
use crate::{ContextDecl, ContextPattern};
use std::collections::{HashMap, HashSet};

pub(super) fn build_homonyms(
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
        let unique_ctxs: HashSet<&str> = ctx_names.into_iter().collect();
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
