//! NDJSON output format for `graph-specs check`.
//!
//! Emits one line-delimited JSON object per [`Violation`]. The format is
//! designed as a stable wire contract for downstream comparators
//! (e.g. qbot-core's Study 002 Phase A1 pipeline). See
//! `specs/ndjson-output.md` for the authoritative schema.
//!
//! Schema v2 invariants:
//! - every record carries `"schema_version":"2"` at the top level
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

use domain::{ContextViolation, Source, Violation};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

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

fn violation_to_record(v: &Violation) -> Value {
    match v {
        Violation::MissingInCode { name, spec_source } => json!({
            "schema_version": "2",
            "violation": "missing_in_code",
            "concept": name,
            "source": source_to_json(spec_source),
        }),
        Violation::MissingInSpecs { name, code_source } => json!({
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
            "violation": "edge_target_unknown",
            "concept": concept,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "spec_source": source_to_json(spec_source),
        }),
        Violation::Context(ctx) => context_violation_to_record(ctx),
    }
}

fn context_violation_to_record(v: &ContextViolation) -> Value {
    match v {
        ContextViolation::MembershipUnknown {
            concept,
            owned_unit,
            code_source,
        } => json!({
            "schema_version": "2",
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
            "schema_version": "2",
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
            "schema_version": "2",
            "violation": "cross_context_edge_undeclared",
            "concept": concept,
            "owning_context": owning_context,
            "edge_kind": edge_kind.as_label(),
            "target": target,
            "target_context": target_context,
            "spec_source": source_to_json(spec_source),
        }),
        // Forward-compat: a v0.5 variant added upstream emits a generic
        // record rather than panicking. `#[non_exhaustive]` on
        // `ContextViolation` mandates this arm.
        _ => json!({
            "schema_version": "2",
            "violation": "unknown_context_violation",
            "concept": v.concept(),
        }),
    }
}

fn source_to_json(s: &Source) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::EdgeKind;
    use serde_json::Value;
    use std::path::PathBuf;

    fn render_one(v: Violation) -> String {
        let mut buf = Vec::new();
        write_ndjson(&[v], &mut buf).expect("write");
        String::from_utf8(buf).expect("utf8")
    }

    fn record(line: &str) -> Value {
        serde_json::from_str(line.trim_end_matches('\n')).expect("valid json")
    }

    #[test]
    fn missing_in_code_record() {
        let v = Violation::MissingInCode {
            name: "Foo".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/a.md"),
                line: 12,
            },
        };
        let out = render_one(v);
        assert!(out.ends_with('\n'));
        let r = record(&out);
        assert_eq!(r["schema_version"], "2");
        assert_eq!(r["violation"], "missing_in_code");
        assert_eq!(r["concept"], "Foo");
        assert_eq!(r["source"]["kind"], "spec");
        assert_eq!(r["source"]["path"], "specs/a.md");
        assert_eq!(r["source"]["line"], 12);
    }

    #[test]
    fn missing_in_specs_record() {
        let v = Violation::MissingInSpecs {
            name: "Bar".into(),
            code_source: Source::Code {
                path: PathBuf::from("src/lib.rs"),
                line: 3,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "missing_in_specs");
        assert_eq!(r["concept"], "Bar");
        assert_eq!(r["source"]["kind"], "code");
        assert_eq!(r["source"]["path"], "src/lib.rs");
        assert_eq!(r["source"]["line"], 3);
    }

    #[test]
    fn signature_drift_record() {
        let v = Violation::SignatureDrift {
            name: "Reader".into(),
            spec_sig: "fn extract(&self)".into(),
            code_sig: "fn extract(&self, root: &Path)".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/core.md"),
                line: 44,
            },
            code_source: Source::Code {
                path: PathBuf::from("ports/src/lib.rs"),
                line: 15,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "signature_drift");
        assert_eq!(r["concept"], "Reader");
        assert_eq!(r["spec_sig"], "fn extract(&self)");
        assert_eq!(r["code_sig"], "fn extract(&self, root: &Path)");
        assert_eq!(r["spec_source"]["kind"], "spec");
        assert_eq!(r["spec_source"]["line"], 44);
        assert_eq!(r["code_source"]["kind"], "code");
        assert_eq!(r["code_source"]["line"], 15);
    }

    #[test]
    fn signature_missing_in_spec_record() {
        let v = Violation::SignatureMissingInSpec {
            name: "Reader".into(),
            code_sig: "fn extract(&self, root: &Path)".into(),
            code_source: Source::Code {
                path: PathBuf::from("ports/src/lib.rs"),
                line: 15,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "signature_missing_in_spec");
        assert_eq!(r["concept"], "Reader");
        assert_eq!(r["code_sig"], "fn extract(&self, root: &Path)");
        assert_eq!(r["code_source"]["kind"], "code");
    }

    #[test]
    fn signature_unparseable_record() {
        let v = Violation::SignatureUnparseable {
            name: "Broken".into(),
            raw: "fn foo(".into(),
            error: "expected `)`".into(),
            source: Source::Spec {
                path: PathBuf::from("specs/broken.md"),
                line: 9,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "signature_unparseable");
        assert_eq!(r["concept"], "Broken");
        assert_eq!(r["raw"], "fn foo(");
        assert_eq!(r["error"], "expected `)`");
        assert_eq!(r["source"]["kind"], "spec");
    }

    #[test]
    fn edge_missing_in_code_record() {
        let v = Violation::EdgeMissingInCode {
            concept: "MarkdownReader".into(),
            edge_kind: EdgeKind::Implements,
            target: "Reader".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/core.md"),
                line: 7,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "edge_missing_in_code");
        assert_eq!(r["concept"], "MarkdownReader");
        assert_eq!(r["edge_kind"], "IMPLEMENTS");
        assert_eq!(r["target"], "Reader");
        assert_eq!(r["spec_source"]["kind"], "spec");
    }

    #[test]
    fn edge_missing_in_spec_record() {
        let v = Violation::EdgeMissingInSpec {
            concept: "MarkdownReader".into(),
            edge_kind: EdgeKind::DependsOn,
            target: "Graph".into(),
            code_source: Source::Code {
                path: PathBuf::from("adapters/markdown/src/lib.rs"),
                line: 42,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "edge_missing_in_spec");
        assert_eq!(r["edge_kind"], "DEPENDS_ON");
        assert_eq!(r["target"], "Graph");
        assert_eq!(r["code_source"]["kind"], "code");
    }

    #[test]
    fn edge_target_unknown_record() {
        let v = Violation::EdgeTargetUnknown {
            concept: "MarkdownReader".into(),
            edge_kind: EdgeKind::Returns,
            target: "Frobnicator".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/core.md"),
                line: 50,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "edge_target_unknown");
        assert_eq!(r["edge_kind"], "RETURNS");
        assert_eq!(r["target"], "Frobnicator");
        assert_eq!(r["spec_source"]["kind"], "spec");
    }

    #[test]
    fn empty_violations_writes_nothing() {
        let mut buf = Vec::new();
        write_ndjson(&[], &mut buf).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn multiple_violations_are_newline_delimited() {
        let v1 = Violation::MissingInCode {
            name: "Foo".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("a.md"),
                line: 1,
            },
        };
        let v2 = Violation::MissingInSpecs {
            name: "Bar".into(),
            code_source: Source::Code {
                path: PathBuf::from("b.rs"),
                line: 2,
            },
        };
        let mut buf = Vec::new();
        write_ndjson(&[v1, v2], &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            serde_json::from_str::<Value>(lines[0]).unwrap()["concept"],
            "Foo"
        );
        assert_eq!(
            serde_json::from_str::<Value>(lines[1]).unwrap()["concept"],
            "Bar"
        );
    }

    #[test]
    fn each_record_has_schema_version_two() {
        let v = Violation::MissingInCode {
            name: "X".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("x.md"),
                line: 1,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["schema_version"], "2");
    }

    // --- v0.4 context violation records (#26) -------------------------

    use domain::OwnedUnit;

    #[test]
    fn context_membership_unknown_record() {
        let v = Violation::Context(ContextViolation::MembershipUnknown {
            concept: "Orphan".into(),
            owned_unit: OwnedUnit("stray-crate".into()),
            code_source: Source::Code {
                path: PathBuf::from("stray-crate/src/lib.rs"),
                line: 3,
            },
        });
        let r = record(&render_one(v));
        assert_eq!(r["schema_version"], "2");
        assert_eq!(r["violation"], "context_membership_unknown");
        assert_eq!(r["concept"], "Orphan");
        assert_eq!(r["owned_unit"], "stray-crate");
        assert_eq!(r["source"]["kind"], "code");
    }

    #[test]
    fn cross_context_edge_unauthorized_record() {
        let v = Violation::Context(ContextViolation::CrossEdgeUnauthorized {
            concept: "MarkdownReader".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::DependsOn,
            target: "TradingPort".into(),
            target_context: "trading".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/contexts/reading.md"),
                line: 12,
            },
        });
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "cross_context_edge_unauthorized");
        assert_eq!(r["concept"], "MarkdownReader");
        assert_eq!(r["owning_context"], "reading");
        assert_eq!(r["edge_kind"], "DEPENDS_ON");
        assert_eq!(r["target"], "TradingPort");
        assert_eq!(r["target_context"], "trading");
        assert_eq!(r["spec_source"]["kind"], "spec");
    }

    #[test]
    fn cross_context_edge_undeclared_record() {
        let v = Violation::Context(ContextViolation::CrossEdgeUndeclared {
            concept: "MarkdownReader".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::Implements,
            target: "Reader".into(),
            target_context: "equivalence".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/contexts/reading.md"),
                line: 12,
            },
        });
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "cross_context_edge_undeclared");
        assert_eq!(r["edge_kind"], "IMPLEMENTS");
        assert_eq!(r["target"], "Reader");
        assert_eq!(r["target_context"], "equivalence");
    }
}
