//! Shared [`Source`] → NDJSON serialization primitives, used by every
//! violation-record builder in [`super`].

use domain::{Provenance, Source};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn source_to_json(s: &Source) -> Value {
    let (kind, path, line) = match s {
        Source::Spec { path, line } => ("spec", path.as_path(), *line),
        Source::Code { path, line } => ("code", path.as_path(), *line),
    };
    json!({
        "kind": kind,
        "path": path_to_string(path),
        "line": line,
    })
}

/// [`source_to_json`] plus the RFC-010 §3.6 / #136 provenance triple:
/// when `s` is code-kind and the outcome's index knows the record's
/// concept, the present fields of the triple are added to the source
/// object. Additive and optional — absent fields are omitted, never
/// `null`, and a spec-kind source never carries them (provenance is a
/// code fact).
pub(super) fn code_source_to_json(s: &Source, provenance: Option<&Provenance>) -> Value {
    let mut record = source_to_json(s);
    if !matches!(s, Source::Code { .. }) {
        return record;
    }
    if let (Some(p), Value::Object(fields)) = (provenance, &mut record) {
        if let Some(m) = &p.module_path {
            fields.insert("module_path".to_owned(), json!(m));
        }
        if let Some(u) = &p.unit {
            fields.insert("unit".to_owned(), json!(u));
        }
        if let Some(c) = &p.context {
            fields.insert("context".to_owned(), json!(c));
        }
    }
    record
}

fn path_to_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
