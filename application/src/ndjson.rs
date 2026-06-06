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

use domain::{CohesionViolation, ContextViolation, SchemaVersion, Source, Violation};
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

fn cohesion_violation_to_record(v: &CohesionViolation) -> Value {
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
        } => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "concept_context_mismatch",
            "concept": concept,
            "declared": declared,
            "code_context": code_context,
            "spec_source": source_to_json(spec_source),
        }),
        // Forward-compat with `#[non_exhaustive]`: a future variant emits a
        // generic record rather than panicking.
        _ => json!({
            "schema_version": SchemaVersion::CURRENT.as_str(),
            "violation": "unknown_cohesion_violation",
        }),
    }
}

fn context_violation_to_record(v: &ContextViolation) -> Value {
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
    use domain::{CohesionViolation, EdgeKind};
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
        assert_eq!(r["schema_version"], "3");
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
    fn each_record_has_schema_version_three() {
        let v = Violation::MissingInCode {
            name: "X".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("x.md"),
                line: 1,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["schema_version"], "3");
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
        assert_eq!(r["schema_version"], "3");
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

    // --- v0.5 verb violation records -----------------------------------

    #[test]
    fn verb_missing_in_code_record() {
        let v = Violation::VerbMissingInCode {
            concept: "Graph".into(),
            qname: "diff".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 10,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["schema_version"], "3");
        assert_eq!(r["violation"], "verb_missing_in_code");
        assert_eq!(r["concept"], "Graph");
        assert_eq!(r["qname"], "diff");
        assert_eq!(r["spec_source"]["kind"], "spec");
    }

    #[test]
    fn verb_missing_in_spec_record() {
        let v = Violation::VerbMissingInSpec {
            qname: "orphan_fn".into(),
            code_source: Source::Code {
                path: PathBuf::from("domain/src/lib.rs"),
                line: 42,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "verb_missing_in_spec");
        assert_eq!(r["qname"], "orphan_fn");
        assert_eq!(r["code_source"]["kind"], "code");
    }

    #[test]
    fn verb_target_unknown_record() {
        let v = Violation::VerbTargetUnknown {
            concept: "Graph".into(),
            qname: "ghost_fn".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 5,
            },
        };
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "verb_target_unknown");
        assert_eq!(r["concept"], "Graph");
        assert_eq!(r["qname"], "ghost_fn");
    }

    #[test]
    fn cross_verb_unauthorized_record() {
        let v = Violation::Context(ContextViolation::CrossVerbUnauthorized {
            concept: "Graph".into(),
            qname: "diff".into(),
            owning_context: "equivalence".into(),
            target_context: "reading".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/core.md"),
                line: 15,
            },
        });
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "cross_verb_unauthorized");
        assert_eq!(r["concept"], "Graph");
        assert_eq!(r["qname"], "diff");
        assert_eq!(r["owning_context"], "equivalence");
        assert_eq!(r["target_context"], "reading");
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

    // --- RFC-010 §3.5 / R10-3 cohesion records (§12-G) ---

    #[test]
    fn concept_context_mismatch_record() {
        let v = Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
            concept: "Widget".into(),
            declared: "reading".into(),
            code_context: "equivalence".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/reading.md"),
                line: 7,
            },
        });
        let r = record(&render_one(v));
        assert_eq!(r["violation"], "concept_context_mismatch");
        assert_eq!(r["concept"], "Widget");
        assert_eq!(r["declared"], "reading");
        assert_eq!(r["code_context"], "equivalence");
        assert_eq!(r["spec_source"]["line"], 7);
    }

    #[test]
    fn spec_side_cohesion_records_carry_file() {
        let cwc = record(&render_one(Violation::Cohesion(
            CohesionViolation::ContextWithoutCohesionUnit {
                context: "lonely".into(),
                file: PathBuf::from("specs/concepts/lonely.md"),
            },
        )));
        assert_eq!(cwc["violation"], "context_without_cohesion_unit");
        assert_eq!(cwc["context"], "lonely");
        assert_eq!(cwc["file"], "specs/concepts/lonely.md");

        let orphan = record(&render_one(Violation::Cohesion(
            CohesionViolation::SubConceptOrphan {
                sub_concept: "Inner".into(),
                file: PathBuf::from("specs/concepts/x.md"),
            },
        )));
        assert_eq!(orphan["violation"], "sub_concept_orphan");
        assert_eq!(orphan["sub_concept"], "Inner");
    }

    // --- RFC-012 §3.6 / R12-1 dangling-anchor record (additive, v3) ---

    #[test]
    fn dangling_anchor_record_is_additive_v3() {
        let v = Violation::DanglingAnchor {
            concept: "ValidateIntakeFull".into(),
            target: "validate_intake".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/intake_validation.md"),
                line: 3,
            },
        };
        let r = record(&render_one(v));
        // Additive: stays schema_version "3", no bump (DD-6).
        assert_eq!(r["schema_version"], "3");
        assert_eq!(r["violation"], "dangling_anchor");
        assert_eq!(r["concept"], "ValidateIntakeFull");
        assert_eq!(r["target"], "validate_intake");
        assert_eq!(r["source"]["kind"], "spec");
        assert_eq!(r["source"]["line"], 3);
        // §12-G: must not fall through to the generic record.
        assert_ne!(r["violation"], "unknown_violation");
    }
}
