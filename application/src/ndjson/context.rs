//! [`ContextViolation`] → NDJSON record conversion (v0.4 bounded-context
//! variants, plus the v0.5 verb-ownership cross-context variant).

use super::source::source_to_json;
use domain::{ContextViolation, SchemaVersion};
use serde_json::{json, Value};

pub(super) fn context_violation_to_record(v: &ContextViolation) -> Value {
    match v {
        ContextViolation::MembershipUnknown {
            concept,
            owned_unit,
            code_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "context_membership_unknown",
            "concept": concept,
            "owned_unit": owned_unit.0,
            "source": source_to_json(code_source),
        }),
        ContextViolation::CrossEdgeUnauthorized {
            concept,
            owning_context,
            edge_kind,
            target,
            target_context,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "cross_context_edge_unauthorized",
            "concept": concept,
            "owning_context": owning_context,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "target_context": target_context,
            "spec_source": source_to_json(spec_source),
        }),
        ContextViolation::CrossEdgeUndeclared {
            concept,
            owning_context,
            edge_kind,
            target,
            target_context,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "cross_context_edge_undeclared",
            "concept": concept,
            "owning_context": owning_context,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "target_context": target_context,
            "spec_source": source_to_json(spec_source),
        }),
        ContextViolation::CrossVerbUnauthorized {
            concept,
            qname,
            owning_context,
            target_context,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "cross_verb_unauthorized",
            "concept": concept,
            "qname": qname,
            "owning_context": owning_context,
            "target_context": target_context,
            "spec_source": source_to_json(spec_source),
        }),
        // Forward-compat: a v0.5 variant added upstream emits a generic
        // record rather than panicking. `#[non_exhaustive]` on
        // `ContextViolation` mandates this arm.
        _ => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "unknown_context_violation",
            "concept": v.concept(),
        }),
    }
}
