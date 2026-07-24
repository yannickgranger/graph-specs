use super::write_ndjson;
use domain::{CohesionViolation, ContextViolation, EdgeKind, OwnedUnit, Source, Violation};
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
    write_ndjson(&[], &mut buf).expect("write");
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
    write_ndjson(&[v1, v2], &mut buf).expect("write");
    let out = String::from_utf8(buf).expect("utf8");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        serde_json::from_str::<Value>(lines[0]).expect("valid json")["concept"],
        "Foo"
    );
    assert_eq!(
        serde_json::from_str::<Value>(lines[1]).expect("valid json")["concept"],
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
