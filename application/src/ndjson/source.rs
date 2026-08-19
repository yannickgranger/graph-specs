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
