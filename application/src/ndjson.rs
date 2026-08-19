mod cohesion;
mod context;
mod source;
#[cfg(test)]
mod tests;

use cohesion::cohesion_violation_to_record;
use context::context_violation_to_record;
use domain::{
    CheckOutcome, PendingRecord, Provenance, RealizedRecord, SchemaVersion, Source, Violation,
};
use serde_json::{json, Value};
use source::{code_source_to_json, source_to_json};
use std::collections::BTreeMap;
use std::io::Write;

type ProvenanceIndex = BTreeMap<String, Provenance>;

pub fn write_ndjson(outcome: &CheckOutcome, out: &mut impl Write) -> std::io::Result<()> {
    for v in &outcome.violations {
        write_record(&violation_to_record(v, &outcome.provenance), out)?;
    }
    for p in &outcome.pending {
        write_record(&pending_to_record(p), out)?;
    }
    for r in &outcome.realized {
        write_record(&realized_to_record(r), out)?;
    }
    for r in &outcome.retirement_incomplete {
        write_record(
            &retirement_record("retirement_incomplete", &r.concept, &r.spec_source),
            out,
        )?;
    }
    for r in &outcome.retirement_complete {
        write_record(
            &retirement_record("retirement_complete", &r.concept, &r.spec_source),
            out,
        )?;
    }
    Ok(())
}

fn write_record(record: &Value, out: &mut impl Write) -> std::io::Result<()> {
    serde_json::to_writer(&mut *out, record)?;
    out.write_all(b"\n")
}

fn pending_to_record(r: &PendingRecord) -> Value {
    json!({
        "schema_version": SchemaVersion::CURRENT.as_str(),
        "marker": "pending",
        "concept": r.concept,
        "source": source_to_json(&r.spec_source),
    })
}

fn retirement_record(marker: &str, concept: &str, source: &Source) -> Value {
    json!({
        "schema_version": SchemaVersion::CURRENT.as_str(),
        "marker": marker,
        "concept": concept,
        "source": source_to_json(source),
    })
}

fn realized_to_record(r: &RealizedRecord) -> Value {
    json!({
        "schema_version": SchemaVersion::CURRENT.as_str(),
        "marker": "realized",
        "concept": r.concept,
        "source": source_to_json(&r.spec_source),
    })
}

#[allow(clippy::too_many_lines)]
fn violation_to_record(v: &Violation, provenance: &ProvenanceIndex) -> Value {
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
            "source": code_source_to_json(code_source, provenance.get(name)),
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
            "code_source": code_source_to_json(code_source, provenance.get(name)),
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
            "code_source": code_source_to_json(code_source, provenance.get(name)),
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
            "code_source": code_source_to_json(code_source, provenance.get(concept)),
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
        Violation::Context(ctx) => context_violation_to_record(ctx, provenance),
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
            "code_source": code_source_to_json(code_source, provenance.get(qname)),
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
        Violation::ForbiddenConceptReintroduced {
            name,
            spec_source,
            code_source,
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "forbidden_concept_reintroduced",
            "concept": name,
            "spec_source": source_to_json(spec_source),
            "code_source": code_source_to_json(code_source, provenance.get(name)),
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
