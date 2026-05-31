//! Human-readable text format for `graph-specs check`.
//!
//! Mirrors the NDJSON module's shape (pure function, writes to `impl
//! Write`) so unit tests can exercise each variant without going
//! through stdout. The CLI's `--format=text` dispatch calls this.

use domain::{ContextViolation, Source, Violation};
use std::io::Write;
use std::path::Path;

/// Write one violation as a human-readable line (or block for
/// multi-field variants). Lines end with `\n`.
///
/// # Errors
///
/// Propagates any [`std::io::Error`] from the underlying writer.
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
        Violation::ImplementsDraftConcept { name, draft_source } => {
            let (path, line) = source_pair(draft_source);
            writeln!(
                out,
                "implements draft spec: {name} ({}:{line}) — promote the draft (flip status:, set code_landing_pr) or remove the code",
                path.display()
            )
        }
        _ => writeln!(out, "unknown violation"),
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
    use domain::{ContextViolation, EdgeKind, OwnedUnit, Source, Violation};
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
        // Regression: existing text shape preserved.
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
}
