use crate::ndjson::write_ndjson;
use crate::report_ndjson::emit_ndjson;
use crate::report_text::emit_text;
use crate::text::format_violation;
use domain::LocationKind;
use domain::Provenance;
use domain::{
    CheckOutcome, CohesionViolation, ContextPattern, ContextViolation, EdgeKind, HomonymAppearance,
    HomonymRecord, OwnedUnit, PubFnDecl, ReportOutput, Source, TierHistogramRecord, TierKind,
    VerbCoverageRecord, Violation,
};
use std::path::PathBuf;

fn spec(line: usize) -> Source {
    Source::Spec {
        path: PathBuf::from("specs/concepts/core.md"),
        line,
        context: None,
    }
}

fn code(line: usize) -> Source {
    Source::Code {
        path: PathBuf::from("domain/src/lib.rs"),
        line,
        provenance: Provenance::empty(),
        location: LocationKind::Path,
    }
}

fn all_violations() -> Vec<Violation> {
    let mut all = direct_violations();
    all.extend(wrapped_violations());
    all
}

fn direct_violations() -> Vec<Violation> {
    vec![
        Violation::MissingInCode {
            name: "Alpha".into(),
            spec_source: spec(1),
        },
        Violation::MissingInSpecs {
            name: "Beta".into(),
            code_source: code(2),
        },
        Violation::ForbiddenConceptReintroduced {
            name: "Gamma".into(),
            spec_source: spec(3),
            code_source: code(4),
        },
        Violation::SignatureDrift {
            name: "Delta".into(),
            spec_sig: "fn d(a: u8)".into(),
            code_sig: "fn d(a: u16)".into(),
            spec_source: spec(5),
            code_source: code(6),
        },
        Violation::SignatureMissingInSpec {
            name: "Epsilon".into(),
            code_sig: "fn e()".into(),
            code_source: code(7),
        },
        Violation::SignatureUnparseable {
            name: "Zeta".into(),
            raw: "fn (".into(),
            error: "expected identifier".into(),
            source: spec(8),
        },
        Violation::EdgeMissingInCode {
            concept: "Eta".into(),
            edge_kind: EdgeKind::Implements,
            target: "Theta".into(),
            spec_source: spec(9),
        },
        Violation::EdgeMissingInSpec {
            concept: "Iota".into(),
            edge_kind: EdgeKind::DependsOn,
            target: "Kappa".into(),
            code_source: code(10),
        },
        Violation::EdgeTargetUnknown {
            concept: "Lambda".into(),
            edge_kind: EdgeKind::Returns,
            target: "Mu".into(),
            spec_source: spec(11),
        },
    ]
}

fn wrapped_violations() -> Vec<Violation> {
    vec![
        Violation::Context(ContextViolation::MembershipUnknown {
            concept: "Nu".into(),
            owned_unit: OwnedUnit("crates/nu".into()),
            code_source: code(12),
        }),
        Violation::Context(ContextViolation::CrossEdgeUnauthorized {
            concept: "Xi".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::Implements,
            target: "Omicron".into(),
            target_context: "equivalence".into(),
            spec_source: spec(13),
        }),
        Violation::Context(ContextViolation::CrossEdgeUndeclared {
            concept: "Pi".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::DependsOn,
            target: "Rho".into(),
            target_context: "equivalence".into(),
            spec_source: spec(14),
        }),
        Violation::Context(ContextViolation::CrossVerbUnauthorized {
            concept: "Sigma".into(),
            qname: "sigma_run".into(),
            owning_context: "reading".into(),
            target_context: "equivalence".into(),
            spec_source: spec(15),
        }),
        Violation::VerbMissingInCode {
            concept: "Tau".into(),
            qname: "tau_run".into(),
            spec_source: spec(16),
        },
        Violation::VerbMissingInSpec {
            qname: "upsilon_run".into(),
            code_source: code(17),
        },
        Violation::VerbTargetUnknown {
            concept: "Phi".into(),
            qname: "phi_run".into(),
            spec_source: spec(18),
        },
        Violation::Cohesion(CohesionViolation::ContextWithoutCohesionUnit {
            context: "orchestration".into(),
            file: PathBuf::from("specs/concepts/orchestration.md"),
        }),
        Violation::Cohesion(CohesionViolation::SubConceptOrphan {
            sub_concept: "Chi".into(),
            file: PathBuf::from("specs/concepts/core.md"),
        }),
        Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
            concept: "Psi".into(),
            declared: "reading".into(),
            code_context: "equivalence".into(),
            spec_source: spec(19),
        }),
        Violation::DanglingAnchor {
            concept: "Omega".into(),
            target: "omega_item".into(),
            spec_source: spec(20),
        },
    ]
}

pub fn code_src() -> Source {
    Source::Code {
        path: PathBuf::from("application/src/lib.rs"),
        line: 33,
        provenance: Provenance::empty(),
        location: LocationKind::Path,
    }
}

pub fn make_report() -> ReportOutput {
    ReportOutput {
        verb_coverage: vec![VerbCoverageRecord {
            context: Some("equivalence".to_owned()),
            pub_fn: PubFnDecl {
                name: "run_check".to_owned(),
                source: code_src(),
                owned_unit: Some("application".to_owned()),
            },
            cited: true,
        }],
        tier_histogram: vec![TierHistogramRecord {
            context: None,
            tier: TierKind::Cypher,
            count: 3,
        }],
        homonyms: vec![HomonymRecord {
            name: "Foo".to_owned(),
            contexts: vec![
                HomonymAppearance {
                    context_name: "ctx_a".to_owned(),
                    sanctioned_by_pattern: Some(ContextPattern::PublishedLanguage),
                    asymmetric: false,
                },
                HomonymAppearance {
                    context_name: "ctx_b".to_owned(),
                    sanctioned_by_pattern: None,
                    asymmetric: true,
                },
            ],
        }],
    }
}

const NDJSON_PLACEHOLDERS: [&str; 3] = [
    "unknown_violation",
    "unknown_context_violation",
    "unknown_cohesion_violation",
];

const TEXT_PLACEHOLDERS: [&str; 3] = [
    "unknown violation",
    "unknown context violation",
    "unknown cohesion violation",
];

#[test]
fn every_violation_variant_renders_ndjson_without_placeholder() {
    let violations = all_violations();
    let expected = violations.len();
    let outcome = CheckOutcome {
        violations,
        ..CheckOutcome::default()
    };
    let mut buf = Vec::new();
    write_ndjson(&outcome, &mut buf).unwrap();
    let rendered = String::from_utf8(buf).unwrap();
    assert_eq!(rendered.lines().count(), expected);
    for placeholder in NDJSON_PLACEHOLDERS {
        assert!(
            !rendered.contains(placeholder),
            "ndjson emitter fell through to `{placeholder}`:\n{rendered}"
        );
    }
}

#[test]
fn every_violation_variant_renders_text_without_placeholder() {
    for violation in all_violations() {
        let mut buf = Vec::new();
        format_violation(&violation, &mut buf).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert!(!rendered.trim().is_empty());
        for placeholder in TEXT_PLACEHOLDERS {
            assert!(
                !rendered.contains(placeholder),
                "text emitter fell through to `{placeholder}` for {violation:?}"
            );
        }
    }
}

fn all_tiers_report() -> ReportOutput {
    let tiers = [
        TierKind::Cypher,
        TierKind::Tier0,
        TierKind::ScriptFence,
        TierKind::ProseOnly,
    ];
    ReportOutput {
        verb_coverage: vec![VerbCoverageRecord {
            context: Some("reading".into()),
            pub_fn: PubFnDecl {
                name: "alpha_run".into(),
                source: code(1),
                owned_unit: Some("crates/alpha".into()),
            },
            cited: true,
        }],
        tier_histogram: tiers
            .iter()
            .enumerate()
            .map(|(i, tier)| TierHistogramRecord {
                context: Some("reading".into()),
                tier: *tier,
                count: i + 1,
            })
            .collect(),
        homonyms: vec![HomonymRecord {
            name: "Shared".into(),
            contexts: vec![
                HomonymAppearance {
                    context_name: "reading".into(),
                    sanctioned_by_pattern: Some(ContextPattern::PublishedLanguage),
                    asymmetric: false,
                },
                HomonymAppearance {
                    context_name: "equivalence".into(),
                    sanctioned_by_pattern: Some(ContextPattern::SharedKernel),
                    asymmetric: true,
                },
            ],
        }],
    }
}

#[test]
fn every_tier_renders_report_ndjson_without_placeholder() {
    let report = all_tiers_report();
    let mut buf = Vec::new();
    emit_ndjson(&mut buf, &report).unwrap();
    let rendered = String::from_utf8(buf).unwrap();
    assert!(
        !rendered.contains("\"tier\":\"unknown\""),
        "report ndjson emitter fell through to tier `unknown`:\n{rendered}"
    );
}

#[test]
fn every_tier_renders_report_text_without_placeholder() {
    let report = all_tiers_report();
    let mut buf = Vec::new();
    emit_text(&mut buf, &report).unwrap();
    let rendered = String::from_utf8(buf).unwrap();
    assert!(
        !rendered.contains("unknown"),
        "report text emitter fell through to tier `unknown`:\n{rendered}"
    );
}
