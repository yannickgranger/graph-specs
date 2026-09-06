use super::source::source_to_json;
use domain::{CohesionViolation, SchemaVersion};
use serde_json::{json, Value};

pub(super) fn cohesion_violation_to_record(v: &CohesionViolation) -> Value {
    match v {
        CohesionViolation::ContextWithoutCohesionUnit { context, file } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "context_without_cohesion_unit",
            "context": context,
            "file": file.display().to_string(),
        }),
        CohesionViolation::SubConceptOrphan { sub_concept, file } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "sub_concept_orphan",
            "sub_concept": sub_concept,
            "file": file.display().to_string(),
        }),
        CohesionViolation::ConceptContextMismatch {
            concept,
            declared,
            code_context,
            spec_source,
            code_source,
        } => {
            let mut record = json!({
                "schema_version": SchemaVersion::CURRENT.as_str(),
                "violation": "concept_context_mismatch",
                "concept": concept,
                "declared": declared,
                "code_context": code_context,
                "spec_source": source_to_json(spec_source),
            });
            if let Some(code_source) = code_source {
                record["code_source"] = source_to_json(code_source);
            }
            record
        }
        _ => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "unknown_cohesion_violation",
        }),
    }
}
