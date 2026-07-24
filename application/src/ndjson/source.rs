//! Shared [`Source`] → NDJSON serialization primitives, used by every
//! violation-record builder in [`super`].

use domain::Source;
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

fn path_to_string(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
