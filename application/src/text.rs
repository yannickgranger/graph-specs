use domain::{
    CheckOutcome, CohesionViolation, ContextViolation, PendingRecord, RealizedRecord,
    RetirementCompleteRecord, RetirementIncompleteRecord, Source, Violation,
};
use std::io::Write;
use std::path::Path;

pub fn format_pending(r: &PendingRecord, out: &mut impl Write) -> std::io::Result<()> {
    let (path, line) = source_pair(&r.spec_source);
    writeln!(out, "pending: {} ({}:{line})", r.concept, path.display())
}

pub fn format_realized(r: &RealizedRecord, out: &mut impl Write) -> std::io::Result<()> {
    let (path, line) = source_pair(&r.spec_source);
    writeln!(
        out,
        "realized — ratify: {} ({}:{line})",
        r.concept,
        path.display()
    )
}

pub fn format_retirement_incomplete(
    r: &RetirementIncompleteRecord,
    out: &mut impl Write,
) -> std::io::Result<()> {
    let (path, line) = source_pair(&r.spec_source);
    writeln!(
        out,
        "retirement incomplete: {} ({}:{line})",
        r.concept,
        path.display()
    )
}

pub fn format_retirement_complete(
    r: &RetirementCompleteRecord,
    out: &mut impl Write,
) -> std::io::Result<()> {
    let (path, line) = source_pair(&r.spec_source);
    writeln!(
        out,
        "retirement complete: {} ({}:{line})",
        r.concept,
        path.display()
    )
}

pub fn format_summary(outcome: &CheckOutcome, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "{} violations, {} pending, {} realized-unratified, {} retirement-incomplete, {} retirement-complete",
        outcome.violations.len(),
        outcome.pending.len(),
        outcome.realized.len(),
        outcome.retirement_incomplete.len(),
        outcome.retirement_complete.len()
    )
}

#[allow(clippy::too_many_lines)]
pub fn format_violation(v: &Violation, out: &mut impl Write) -> std::io::Result<()> {
    match v {
        Violation::MissingInCode { name, spec_source } => {
            let (path, line) = source_pair(spec_source);
            writeln!(out, "missing in code: {name} ({}:{line})", path.display())
        }
        Violation::MissingInSpecs { name, code_source } => {
            let (path, line) = source_pair(code_source);
            writeln!(out, "missing in specs: {name} ({}:{line})", path.display())
        }
        Violation::SignatureDrift {
            name,
            spec_sig,
            code_sig,
            spec_source,
            code_source,
        } => {
            let (spec_path, spec_line) = source_pair(spec_source);
            let (code_path, code_line) = source_pair(code_source);
            writeln!(
                out,
                "signature drift: {name}\n  spec ({}:{spec_line}): {spec_sig}\n  code ({}:{code_line}): {code_sig}",
                spec_path.display(),
                code_path.display()
            )
        }
        Violation::SignatureMissingInSpec {
            name,
            code_sig,
            code_source,
        } => {
            let (path, line) = source_pair(code_source);
            writeln!(
                out,
                "signature missing in spec: {name} ({}:{line})\n  code: {code_sig}",
                path.display()
            )
        }
        Violation::SignatureUnparseable {
            name,
            raw,
            error,
            source,
        } => {
            let (path, line) = source_pair(source);
            writeln!(
                out,
                "signature unparseable: {name} ({}:{line})\n  raw: {raw}\n  error: {error}",
                path.display()
            )
        }
        Violation::EdgeMissingInCode {
            concept,
            edge_kind,
            target,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "edge missing in code: {concept} --{edge_kind}--> {target} ({}:{line})",
                path.display()
            )
        }
        Violation::EdgeMissingInSpec {
            concept,
            edge_kind,
            target,
            code_source,
        } => {
            let (path, line) = source_pair(code_source);
            writeln!(
                out,
                "edge missing in spec: {concept} --{edge_kind}--> {target} ({}:{line})",
                path.display()
            )
        }
        Violation::EdgeTargetUnknown {
            concept,
            edge_kind,
            target,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "edge target unknown: {concept} --{edge_kind}--> {target} (not a concept in either graph) ({}:{line})",
                path.display()
            )
        }
        Violation::Context(ctx) => format_context_violation(ctx, out),
        Violation::VerbMissingInCode {
            concept,
            qname,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "verb missing in code: {concept} claims `{qname}` but no pub fn found ({}:{line})",
                path.display()
            )
        }
        Violation::VerbMissingInSpec { qname, code_source } => {
            let (path, line) = source_pair(code_source);
            writeln!(
                out,
                "verb missing in spec: `{qname}` is unclaimed in its context ({}:{line})",
                path.display()
            )
        }
        Violation::VerbTargetUnknown {
            concept,
            qname,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "verb target unknown: {concept} claims `{qname}` but fn belongs to no context ({}:{line})",
                path.display()
            )
        }
        Violation::ForbiddenConceptReintroduced {
            name,
            spec_source,
            code_source,
        } => {
            let (spec_path, spec_line) = source_pair(spec_source);
            let (code_path, code_line) = source_pair(code_source);
            writeln!(
                out,
                "forbidden concept reintroduced: {name}\n  expelled by ({}:{spec_line})\n  reintroduced at ({}:{code_line})",
                spec_path.display(),
                code_path.display()
            )
        }
        Violation::Cohesion(c) => format_cohesion_violation(c, out),
        Violation::DanglingAnchor {
            concept,
            target,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "dangling anchor: {concept} anchors `{target}` but no such code item exists ({}:{line})",
                path.display()
            )
        }
        _ => writeln!(out, "unknown violation"),
    }
}

fn format_cohesion_violation(v: &CohesionViolation, out: &mut impl Write) -> std::io::Result<()> {
    match v {
        CohesionViolation::ContextWithoutCohesionUnit { context, file } => writeln!(
            out,
            "context without cohesion unit: `{context}` declares no concept under its H1 ({})",
            file.display()
        ),
        CohesionViolation::SubConceptOrphan { sub_concept, file } => writeln!(
            out,
            "sub-concept orphan: `{sub_concept}` has no enclosing concept (H3 without an H2) ({})",
            file.display()
        ),
        CohesionViolation::ConceptContextMismatch {
            concept,
            declared,
            code_context,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "concept context mismatch: {concept} declared in `{declared}` but code resolves to `{code_context}` ({}:{line})",
                path.display()
            )
        }
        _ => writeln!(out, "unknown cohesion violation"),
    }
}

fn format_context_violation(v: &ContextViolation, out: &mut impl Write) -> std::io::Result<()> {
    match v {
        ContextViolation::MembershipUnknown {
            concept,
            owned_unit,
            code_source,
        } => {
            let (path, line) = source_pair(code_source);
            writeln!(
                out,
                "context membership unknown: {concept} in `{}` ({}:{line})",
                owned_unit.0,
                path.display()
            )
        }
        ContextViolation::CrossEdgeUnauthorized {
            concept,
            owning_context,
            edge_kind,
            target,
            target_context,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "cross-context edge unauthorized: {concept} ({owning_context}) --{edge_kind}--> {target} ({target_context}) at {}:{line}",
                path.display()
            )
        }
        ContextViolation::CrossEdgeUndeclared {
            concept,
            owning_context,
            edge_kind,
            target,
            target_context,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "cross-context edge undeclared: {concept} ({owning_context}) --{edge_kind}--> {target} ({target_context}) at {}:{line}",
                path.display()
            )
        }
        ContextViolation::CrossVerbUnauthorized {
            concept,
            qname,
            owning_context,
            target_context,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            writeln!(
                out,
                "cross-context verb unauthorized: {concept} ({owning_context}) claims `{qname}` which belongs to {target_context} ({}:{line})",
                path.display()
            )
        }
        _ => writeln!(out, "unknown context violation for {}", v.concept()),
    }
}

fn source_pair(s: &Source) -> (&Path, usize) {
    match s {
        Source::Spec { path, line } | Source::Code { path, line } => (path.as_path(), *line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{CohesionViolation, ContextViolation, EdgeKind, OwnedUnit, Source, Violation};
    use std::path::PathBuf;

    fn render(v: &Violation) -> String {
        let mut buf = Vec::new();
        format_violation(v, &mut buf).expect("write");
        String::from_utf8(buf).expect("utf8")
    }

    fn code_src() -> Source {
        Source::Code {
            path: PathBuf::from("some-crate/src/lib.rs"),
            line: 3,
        }
    }

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/contexts/reading.md"),
            line: 12,
        }
    }

    #[test]
    fn context_membership_unknown_text() {
        let v = Violation::Context(ContextViolation::MembershipUnknown {
            concept: "Foo".into(),
            owned_unit: OwnedUnit("stray-crate".into()),
            code_source: code_src(),
        });
        let out = render(&v);
        assert!(out.ends_with('\n'));
        assert!(
            out.contains("context membership unknown: Foo"),
            "got: {out}"
        );
        assert!(out.contains("`stray-crate`"));
        assert!(out.contains("some-crate/src/lib.rs:3"));
    }

    #[test]
    fn cross_context_edge_unauthorized_text() {
        let v = Violation::Context(ContextViolation::CrossEdgeUnauthorized {
            concept: "MarkdownReader".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::DependsOn,
            target: "TradingPort".into(),
            target_context: "trading".into(),
            spec_source: spec_src(),
        });
        let out = render(&v);
        assert!(
            out.starts_with("cross-context edge unauthorized: MarkdownReader (reading) --DEPENDS_ON--> TradingPort (trading)"),
            "got: {out}"
        );
        assert!(out.contains("specs/contexts/reading.md:12"));
    }

    #[test]
    fn cross_context_edge_undeclared_text() {
        let v = Violation::Context(ContextViolation::CrossEdgeUndeclared {
            concept: "MarkdownReader".into(),
            owning_context: "reading".into(),
            edge_kind: EdgeKind::Implements,
            target: "Reader".into(),
            target_context: "equivalence".into(),
            spec_source: spec_src(),
        });
        let out = render(&v);
        assert!(
            out.starts_with("cross-context edge undeclared: MarkdownReader (reading) --IMPLEMENTS--> Reader (equivalence)"),
            "got: {out}"
        );
    }

    #[test]
    fn verb_missing_in_code_text() {
        let v = Violation::VerbMissingInCode {
            concept: "Graph".into(),
            qname: "diff".into(),
            spec_source: spec_src(),
        };
        let out = render(&v);
        assert!(out.ends_with('\n'));
        assert!(
            out.contains("verb missing in code: Graph claims `diff`"),
            "got: {out}"
        );
        assert!(out.contains("specs/contexts/reading.md:12"));
    }

    #[test]
    fn verb_missing_in_spec_text() {
        let v = Violation::VerbMissingInSpec {
            qname: "orphan_fn".into(),
            code_source: code_src(),
        };
        let out = render(&v);
        assert!(out.ends_with('\n'));
        assert!(
            out.contains("verb missing in spec: `orphan_fn` is unclaimed"),
            "got: {out}"
        );
        assert!(out.contains("some-crate/src/lib.rs:3"));
    }

    #[test]
    fn verb_target_unknown_text() {
        let v = Violation::VerbTargetUnknown {
            concept: "Graph".into(),
            qname: "ghost_fn".into(),
            spec_source: spec_src(),
        };
        let out = render(&v);
        assert!(out.ends_with('\n'));
        assert!(
            out.contains("verb target unknown: Graph claims `ghost_fn`"),
            "got: {out}"
        );
    }

    #[test]
    fn cross_verb_unauthorized_text() {
        let v = Violation::Context(ContextViolation::CrossVerbUnauthorized {
            concept: "Graph".into(),
            qname: "diff".into(),
            owning_context: "equivalence".into(),
            target_context: "reading".into(),
            spec_source: spec_src(),
        });
        let out = render(&v);
        assert!(out.ends_with('\n'));
        assert!(
            out.contains("cross-context verb unauthorized: Graph (equivalence) claims `diff` which belongs to reading"),
            "got: {out}"
        );
    }

    #[test]
    fn v03_missing_in_code_unchanged() {
        let v = Violation::MissingInCode {
            name: "Foo".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/a.md"),
                line: 1,
            },
        };
        let out = render(&v);
        assert_eq!(out, "missing in code: Foo (specs/a.md:1)\n");
    }

    #[test]
    fn concept_context_mismatch_text_renders_path_line() {
        let v = Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
            concept: "Widget".into(),
            declared: "reading".into(),
            code_context: "equivalence".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/reading.md"),
                line: 7,
            },
        });
        let out = render(&v);
        assert!(out.contains("concept context mismatch: Widget"));
        assert!(out.contains("declared in `reading`"));
        assert!(out.contains("code resolves to `equivalence`"));
        assert!(out.contains("specs/concepts/reading.md:7"));
        assert!(!out.contains("unknown violation"));
    }

    #[test]
    fn context_without_cohesion_unit_text() {
        let v = Violation::Cohesion(CohesionViolation::ContextWithoutCohesionUnit {
            context: "lonely".into(),
            file: PathBuf::from("specs/concepts/lonely.md"),
        });
        let out = render(&v);
        assert!(out.contains("context without cohesion unit: `lonely`"));
        assert!(out.contains("specs/concepts/lonely.md"));
    }

    #[test]
    fn sub_concept_orphan_text() {
        let v = Violation::Cohesion(CohesionViolation::SubConceptOrphan {
            sub_concept: "Inner".into(),
            file: PathBuf::from("specs/concepts/x.md"),
        });
        let out = render(&v);
        assert!(out.contains("sub-concept orphan: `Inner`"));
    }

    #[test]
    fn dangling_anchor_text_renders_path_line_not_unknown() {
        let v = Violation::DanglingAnchor {
            concept: "ValidateIntakeFull".into(),
            target: "validate_intake".into(),
            spec_source: Source::Spec {
                path: PathBuf::from("specs/concepts/intake_validation.md"),
                line: 3,
            },
        };
        let out = render(&v);
        assert!(out.contains("dangling anchor: ValidateIntakeFull"));
        assert!(out.contains("anchors `validate_intake`"));
        assert!(out.contains("specs/concepts/intake_validation.md:3"));
        assert!(!out.contains("unknown violation"));
    }
}
