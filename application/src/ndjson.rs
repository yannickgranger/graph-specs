//! NDJSON output format for `graph-specs check`.
//!
//! Emits one line-delimited JSON object per [`Violation`]. The format is
//! designed as a stable wire contract for downstream comparators
//! (e.g. qbot-core's Study 002 Phase A1 pipeline). See
//! `specs/ndjson-output.md` for the authoritative schema.
//!
//! Schema invariants:
//! - every record carries `"schema_version":"4"` at the top level
//! - `violation` is the `snake_case` variant discriminator on a finding
//! - `marker` is the discriminator on an RFC-013 marker record — a
//!   deliberately **separate** key, so the existing violation-filtered
//!   stream is unchanged and the marker-filtered stream is the upstream
//!   ratification worklist
//! - record order matches the `violations` argument order
//! - no trailing comma, no final newline suppression — each record
//!   ends in `\n`
//! - path strings are emitted via [`std::path::Path::to_string_lossy`]
//!
//! v2 adds three variants over v1: `context_membership_unknown`,
//! `cross_context_edge_unauthorized`, `cross_context_edge_undeclared`.
//! All v1 records are structurally unchanged except for the version
//! bump. Consumers pin on `schema_version` and select a variant set.
//!
//! v4 (RFC-013 §3.5) adds the `marker` record kinds and **retires** the
//! `implements_draft_concept` violation kind. The retirement is what makes
//! the bump breaking rather than additive.

mod cohesion;
mod context;
mod source;
#[cfg(test)]
mod tests;

use cohesion::cohesion_violation_to_record;
use context::context_violation_to_record;
use domain::{CheckOutcome, PendingRecord, RealizedRecord, SchemaVersion, Violation};
use serde_json::{json, Value};
use source::source_to_json;
use std::io::Write;

/// Write a check outcome as NDJSON to `out` — every violation, then every
/// pending record, then every realized record.
///
/// Grouping by kind rather than interleaving keeps a consumer that reads
/// only the `violation`-keyed prefix working byte-for-byte as before.
///
/// # Errors
///
/// Propagates any [`std::io::Error`] from the underlying writer —
/// typically a broken pipe when stdout is closed downstream.
pub fn write_ndjson(outcome: &CheckOutcome, out: &mut impl Write) -> std::io::Result<()> {
    for v in &outcome.violations {
        write_record(&violation_to_record(v), out)?;
    }
    for p in &outcome.pending {
        write_record(&pending_to_record(p), out)?;
    }
    for r in &outcome.realized {
        write_record(&realized_to_record(r), out)?;
    }
    Ok(())
}

fn write_record(record: &Value, out: &mut impl Write) -> std::io::Result<()> {
    serde_json::to_writer(&mut *out, record)?;
    out.write_all(b"\n")
}

/// RFC-013 §3.5 — a `pending` marker record. Keyed `marker`, never
/// `report`: the `report` subcommand's emitter already owns a `"record"`
/// discriminator for `verb_coverage` / `tier_histogram` / `homonym`.
fn pending_to_record(r: &PendingRecord) -> Value {
    json!({
        "schema_version": SchemaVersion::CURRENT.as_str(),
        "marker": "pending",
        "concept": r.concept,
        "source": source_to_json(&r.spec_source),
    })
}

/// RFC-013 §3.5 — a `realized` marker record. See [`pending_to_record`].
fn realized_to_record(r: &RealizedRecord) -> Value {
    json!({
        "schema_version": SchemaVersion::CURRENT.as_str(),
        "marker": "realized",
        "concept": r.concept,
        "source": source_to_json(&r.spec_source),
    })
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
