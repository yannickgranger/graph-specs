//! NDJSON output format for `graph-specs check`.
//!
//! Emits one line-delimited JSON object per [`Violation`]. The format is
//! designed as a stable wire contract for downstream comparators
//! (e.g. qbot-core's Study 002 Phase A1 pipeline). See
//! `specs/ndjson-output.md` for the authoritative schema.
//!
//! Schema v2 invariants:
//! - every record carries `"schema_version":"3"` at the top level
//! - `violation` is the `snake_case` variant discriminator
//! - record order matches the `violations` argument order
//! - no trailing comma, no final newline suppression — each record
//!   ends in `\n`
//! - path strings are emitted via [`std::path::Path::to_string_lossy`]
//!
//! v2 adds three variants over v1: `context_membership_unknown`,
//! `cross_context_edge_unauthorized`, `cross_context_edge_undeclared`.
//! All v1 records are structurally unchanged except for the version
//! bump. Consumers pin on `schema_version` and select a variant set.

mod cohesion;
mod context;
mod source;
#[cfg(test)]
mod tests;

use cohesion::cohesion_violation_to_record;
use context::context_violation_to_record;
use domain::{SchemaVersion, Violation};
use serde_json::{json, Value};
use source::source_to_json;
use std::io::Write;

/// Write violations as NDJSON to `out`.
///
/// # Errors
///
/// Propagates any [`std::io::Error`] from the underlying writer —
/// typically a broken pipe when stdout is closed downstream.
pub fn write_ndjson(violations: &[Violation], out: &mut impl Write) -> std::io::Result<()> {
    for v in violations {
        let record = violation_to_record(v);
        serde_json::to_writer(&mut *out, &record)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn violation_to_record(v: &Violation) -> Value {
    match v {
        Violation::MissingInCode { name, spec_source } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "missing_in_code",
            "concept": name,
            "source": source_to_json(spec_source),
        }),
        Violation::MissingInSpecs { name, code_source } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "missing_in_specs",
            "concept": name,
            "source": source_to_json(code_source),
        }),
        Violation::SignatureDrift {
            name,
            spec_sig,
            code_sig,
            spec_source,
            code_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "signature_drift",
            "concept": name,
            "spec_sig": spec_sig,
            "code_sig": code_sig,
            "spec_source": source_to_json(spec_source),
            "code_source": source_to_json(code_source),
        }),
        Violation::SignatureMissingInSpec {
            name,
            code_sig,
            code_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "signature_missing_in_spec",
            "concept": name,
            "code_sig": code_sig,
            "code_source": source_to_json(code_source),
        }),
        Violation::SignatureUnparseable {
            name,
            raw,
            error,
            source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "signature_unparseable",
            "concept": name,
            "raw": raw,
            "error": error,
            "source": source_to_json(source),
        }),
        Violation::EdgeMissingInCode {
            concept,
            edge_kind,
            target,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "edge_missing_in_code",
            "concept": concept,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "spec_source": source_to_json(spec_source),
        }),
        Violation::EdgeMissingInSpec {
            concept,
            edge_kind,
            target,
            code_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "edge_missing_in_spec",
            "concept": concept,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "code_source": source_to_json(code_source),
        }),
        Violation::EdgeTargetUnknown {
            concept,
            edge_kind,
            target,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "edge_target_unknown",
            "concept": concept,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "spec_source": source_to_json(spec_source),
        }),
        Violation::Context(ctx) => context_violation_to_record(ctx),
        Violation::VerbMissingInCode {
            concept,
            qname,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "verb_missing_in_code",
            "concept": concept,
            "qname": qname,
            "spec_source": source_to_json(spec_source),
        }),
        Violation::VerbMissingInSpec { qname, code_source } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "verb_missing_in_spec",
            "qname": qname,
            "code_source": source_to_json(code_source),
        }),
        Violation::VerbTargetUnknown {
            concept,
            qname,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "verb_target_unknown",
            "concept": concept,
            "qname": qname,
            "spec_source": source_to_json(spec_source),
        }),
        Violation::ImplementsDraftConcept { name, draft_source } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "implements_draft_concept",
            "name": name,
            "draft_source": source_to_json(draft_source),
        }),
        Violation::Cohesion(c) => cohesion_violation_to_record(c),
        Violation::DanglingAnchor {
            concept,
            target,
            spec_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "dangling_anchor",
            "concept": concept,
            "target": target,
            "source": source_to_json(spec_source),
        }),
        _ => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "unknown_violation",
        }),
    }
}
