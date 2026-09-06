use super::write_ndjson;
use domain::{
    CheckOutcome, CohesionViolation, ContextViolation, EdgeKind, OwnedUnit, PendingRecord,
    Provenance, RealizedRecord, Source, Violation,
};
use serde_json::Value;
use std::path::PathBuf;

fn render(outcome: &CheckOutcome) -> String {
    let mut buf = Vec::new();
    write_ndjson(outcome, &mut buf).expect("write");
    String::from_utf8(buf).expect("utf8")
}

fn render_one(v: Violation) -> String {
    render(&CheckOutcome {
        violations: vec![v],
        ..CheckOutcome::empty()
    })
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
            context: None,
        },
    };
    let out = render_one(v);
    assert!(out.ends_with('\n'));
    let r = record(&out);
    assert_eq!(r["schema_version"], "4");
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
            provenance: Provenance::empty(),
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
            context: None,
        },
        code_source: Source::Code {
            path: PathBuf::from("ports/src/lib.rs"),
            line: 15,
            provenance: Provenance::empty(),
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
            provenance: Provenance::empty(),
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
            context: None,
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
            context: None,
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
            provenance: Provenance::empty(),
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
            context: None,
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
    write_ndjson(&CheckOutcome::empty(), &mut buf).expect("write");
    assert!(buf.is_empty());
}

#[test]
fn multiple_violations_are_newline_delimited() {
    let v1 = Violation::MissingInCode {
        name: "Foo".into(),
        spec_source: Source::Spec {
            path: PathBuf::from("a.md"),
            line: 1,
            context: None,
        },
    };
    let v2 = Violation::MissingInSpecs {
        name: "Bar".into(),
        code_source: Source::Code {
            path: PathBuf::from("b.rs"),
            line: 2,
            provenance: Provenance::empty(),
        },
    };
    let mut buf = Vec::new();
    write_ndjson(
        &CheckOutcome {
            violations: vec![v1, v2],
            ..CheckOutcome::empty()
        },
        &mut buf,
    )
    .expect("write");
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
fn each_record_has_schema_version_four() {
    let v = Violation::MissingInCode {
        name: "X".into(),
        spec_source: Source::Spec {
            path: PathBuf::from("x.md"),
            line: 1,
            context: None,
        },
    };
    let r = record(&render_one(v));
    assert_eq!(r["schema_version"], "4");
}

#[test]
fn context_membership_unknown_record() {
    let v = Violation::Context(ContextViolation::MembershipUnknown {
        concept: "Orphan".into(),
        owned_unit: OwnedUnit("stray-crate".into()),
        code_source: Source::Code {
            path: PathBuf::from("stray-crate/src/lib.rs"),
            line: 3,
            provenance: Provenance::empty(),
        },
    });
    let r = record(&render_one(v));
    assert_eq!(r["schema_version"], "4");
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
            context: None,
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
fn verb_missing_in_code_record() {
    let v = Violation::VerbMissingInCode {
        concept: "Graph".into(),
        qname: "diff".into(),
        spec_source: Source::Spec {
            path: PathBuf::from("specs/concepts/core.md"),
            line: 10,
            context: None,
        },
    };
    let r = record(&render_one(v));
    assert_eq!(r["schema_version"], "4");
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
            provenance: Provenance::empty(),
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
            context: None,
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
            context: None,
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
            context: None,
        },
    });
    let r = record(&render_one(v));
    assert_eq!(r["violation"], "cross_context_edge_undeclared");
    assert_eq!(r["edge_kind"], "IMPLEMENTS");
    assert_eq!(r["target"], "Reader");
    assert_eq!(r["target_context"], "equivalence");
}

#[test]
fn concept_context_mismatch_record() {
    let v = Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
        concept: "Widget".into(),
        declared: "reading".into(),
        code_context: "equivalence".into(),
        spec_source: Source::Spec {
            path: PathBuf::from("specs/concepts/reading.md"),
            line: 7,
            context: None,
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

#[test]
fn dangling_anchor_record_is_additive_v3() {
    let v = Violation::DanglingAnchor {
        concept: "ValidateIntakeFull".into(),
        target: "validate_intake".into(),
        spec_source: Source::Spec {
            path: PathBuf::from("specs/concepts/intake_validation.md"),
            line: 3,
            context: None,
        },
    };
    let r = record(&render_one(v));
    assert_eq!(r["schema_version"], "4");
    assert_eq!(r["violation"], "dangling_anchor");
    assert_eq!(r["concept"], "ValidateIntakeFull");
    assert_eq!(r["target"], "validate_intake");
    assert_eq!(r["source"]["kind"], "spec");
    assert_eq!(r["source"]["line"], 3);
    assert_ne!(r["violation"], "unknown_violation");
}

fn spec_at(path: &str, line: usize) -> Source {
    Source::Spec {
        path: PathBuf::from(path),
        line,
        context: None,
    }
}

#[test]
fn pending_marker_record() {
    let out = render(&CheckOutcome {
        pending: vec![PendingRecord {
            concept: "Digest".into(),
            spec_source: spec_at("specs/concepts/execution.md", 41),
        }],
        ..CheckOutcome::empty()
    });
    let r = record(&out);
    assert_eq!(r["schema_version"], "4");
    assert_eq!(r["marker"], "pending");
    assert_eq!(r["concept"], "Digest");
    assert_eq!(r["source"]["kind"], "spec");
    assert_eq!(r["source"]["path"], "specs/concepts/execution.md");
    assert_eq!(r["source"]["line"], 41);
    assert!(
        r.get("violation").is_none(),
        "a marker record carries no `violation` key — the two streams are          filtered separately"
    );
}

#[test]
fn realized_marker_record() {
    let out = render(&CheckOutcome {
        realized: vec![RealizedRecord {
            concept: "InboundAcl".into(),
            spec_source: spec_at("specs/concepts/fleet_supervision.md", 120),
        }],
        ..CheckOutcome::empty()
    });
    let r = record(&out);
    assert_eq!(r["marker"], "realized");
    assert_eq!(r["concept"], "InboundAcl");
    assert_eq!(r["source"]["line"], 120);
}

#[test]
fn violations_precede_markers_in_the_stream() {
    let out = render(&CheckOutcome {
        violations: vec![Violation::MissingInCode {
            name: "Foo".into(),
            spec_source: spec_at("specs/a.md", 1),
        }],
        pending: vec![PendingRecord {
            concept: "Bar".into(),
            spec_source: spec_at("specs/a.md", 5),
        }],
        realized: vec![RealizedRecord {
            concept: "Baz".into(),
            spec_source: spec_at("specs/a.md", 9),
        }],
        ..CheckOutcome::empty()
    });
    let kinds: Vec<String> = out
        .lines()
        .map(|l| {
            let r = record(l);
            r.get("violation")
                .or_else(|| r.get("marker"))
                .and_then(Value::as_str)
                .expect("each record carries one discriminator")
                .to_owned()
        })
        .collect();
    assert_eq!(kinds, vec!["missing_in_code", "pending", "realized"]);
}

fn code_at(path: &str, line: usize) -> Source {
    Source::Code {
        path: PathBuf::from(path),
        line,
        provenance: Provenance::empty(),
    }
}

fn prov_full() -> Provenance {
    Provenance {
        module_path: Some("domain".to_owned()),
        unit: Some("domain".to_owned()),
        context: Some("equivalence".to_owned()),
    }
}

fn code_prov(path: &str, line: usize, provenance: Provenance) -> Source {
    Source::Code {
        path: PathBuf::from(path),
        line,
        provenance,
    }
}

fn assert_triple(source: &Value) {
    assert_eq!(source["module_path"], "domain");
    assert_eq!(source["unit"], "domain");
    assert_eq!(source["context"], "equivalence");
}

#[test]
fn missing_in_specs_source_carries_provenance_triple() {
    let v = Violation::MissingInSpecs {
        name: "Bar".into(),
        code_source: code_prov("domain/src/lib.rs", 3, prov_full()),
    };
    let r = record(&render_one(v));
    assert_eq!(r["source"]["kind"], "code");
    assert_triple(&r["source"]);
}

#[test]
fn signature_drift_triple_lands_on_code_source_only() {
    let v = Violation::SignatureDrift {
        name: "Reader".into(),
        spec_sig: "fn extract(&self)".into(),
        code_sig: "fn extract(&self, root: &Path)".into(),
        spec_source: spec_at("specs/core.md", 44),
        code_source: code_prov("ports/src/lib.rs", 15, prov_full()),
    };
    let r = record(&render_one(v));
    assert_triple(&r["code_source"]);
    assert!(r["spec_source"]["module_path"].is_null());
    assert!(r["spec_source"]["unit"].is_null());
    assert!(r["spec_source"]["context"].is_null());
}

#[test]
fn signature_missing_in_spec_source_carries_provenance_triple() {
    let v = Violation::SignatureMissingInSpec {
        name: "Reader".into(),
        code_sig: "fn extract(&self, root: &Path)".into(),
        code_source: code_prov("ports/src/lib.rs", 15, prov_full()),
    };
    let r = record(&render_one(v));
    assert_triple(&r["code_source"]);
}

#[test]
fn edge_missing_in_spec_source_carries_provenance_triple() {
    let v = Violation::EdgeMissingInSpec {
        concept: "MarkdownReader".into(),
        edge_kind: EdgeKind::DependsOn,
        target: "Graph".into(),
        code_source: code_prov("adapters/markdown/src/lib.rs", 42, prov_full()),
    };
    let r = record(&render_one(v));
    assert_triple(&r["code_source"]);
}

#[test]
fn context_membership_unknown_source_carries_provenance_triple() {
    let v = Violation::Context(ContextViolation::MembershipUnknown {
        concept: "Stray".into(),
        owned_unit: OwnedUnit("straycrate".into()),
        code_source: code_prov("straycrate/src/lib.rs", 1, prov_full()),
    });
    let r = record(&render_one(v));
    assert_eq!(r["violation"], "context_membership_unknown");
    assert_triple(&r["source"]);
}

#[test]
fn forbidden_concept_reintroduced_code_source_carries_provenance_triple() {
    let v = Violation::ForbiddenConceptReintroduced {
        name: "LegacyThing".into(),
        spec_source: spec_at("specs/core.md", 9),
        code_source: code_prov("domain/src/lib.rs", 80, prov_full()),
    };
    let r = record(&render_one(v));
    assert_triple(&r["code_source"]);
    assert!(r["spec_source"]["module_path"].is_null());
}

#[test]
fn partial_triple_renders_only_present_fields() {
    let v = Violation::MissingInSpecs {
        name: "Bar".into(),
        code_source: code_prov(
            "domain/src/lib.rs",
            3,
            Provenance {
                module_path: None,
                unit: Some("domain".to_owned()),
                context: None,
            },
        ),
    };
    let r = record(&render_one(v));
    assert_eq!(r["source"]["unit"], "domain");
    let source = r["source"].as_object().expect("source object");
    assert!(!source.contains_key("module_path"));
    assert!(!source.contains_key("context"));
}

#[test]
fn unknown_concept_renders_plain_source_object() {
    let v = Violation::MissingInSpecs {
        name: "Bar".into(),
        code_source: code_at("domain/src/lib.rs", 3),
    };
    let r = record(&render_one(v));
    let source = r["source"].as_object().expect("source object");
    assert_eq!(source.len(), 3, "kind/path/line only: {source:?}");
}

#[test]
fn spec_kind_source_never_carries_the_triple() {
    let v = Violation::MissingInCode {
        name: "Foo".into(),
        spec_source: spec_at("specs/a.md", 12),
    };
    let r = record(&render_one(v));
    let source = r["source"].as_object().expect("source object");
    assert_eq!(source.len(), 3, "kind/path/line only: {source:?}");
}
