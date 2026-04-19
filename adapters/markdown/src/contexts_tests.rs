use super::*;
use std::path::PathBuf;

fn p() -> PathBuf {
    PathBuf::from("specs/contexts/x.md")
}

fn parse(source: &str) -> Result<ContextDecl, ReaderError> {
    parse_context_file(&p(), source)
}

#[test]
fn minimal_h1_only_yields_empty_sections() {
    let decl = parse("# equivalence\n").expect("parse");
    assert_eq!(decl.name, "equivalence");
    assert!(decl.owned_units.is_empty());
    assert!(decl.exports.is_empty());
    assert!(decl.imports.is_empty());
}

#[test]
fn parses_owns_section() {
    let src = "# equivalence\n\n## Owns\n\n- domain\n- ports\n";
    let decl = parse(src).expect("parse");
    assert_eq!(
        decl.owned_units,
        vec![OwnedUnit("domain".into()), OwnedUnit("ports".into())]
    );
}

#[test]
fn parses_exports_section_with_each_pattern() {
    let src = "\
# equivalence

## Exports

- Graph (PublishedLanguage)
- Edge (SharedKernel)
- Reader (Conformist)
- Bar (CustomerSupplier)
";
    let decl = parse(src).expect("parse");
    assert_eq!(decl.exports.len(), 4);
    assert_eq!(decl.exports[0].concept, "Graph");
    assert_eq!(decl.exports[0].pattern, ContextPattern::PublishedLanguage);
    assert_eq!(decl.exports[1].pattern, ContextPattern::SharedKernel);
    assert_eq!(decl.exports[2].pattern, ContextPattern::Conformist);
    assert_eq!(decl.exports[3].pattern, ContextPattern::CustomerSupplier);
}

#[test]
fn parses_imports_section() {
    let src = "\
# reading

## Imports

- Graph from equivalence (PublishedLanguage)
- Reader from equivalence (Conformist)
";
    let decl = parse(src).expect("parse");
    assert_eq!(decl.imports.len(), 2);
    assert_eq!(decl.imports[0].concept, "Graph");
    assert_eq!(decl.imports[0].from_context, "equivalence");
    assert_eq!(decl.imports[0].pattern, ContextPattern::PublishedLanguage);
    assert_eq!(decl.imports[1].pattern, ContextPattern::Conformist);
}

#[test]
fn parses_all_four_sections() {
    let src = "\
# reading

## Owns

- adapter-markdown
- adapter-rust

## Exports

- MarkdownReader (PublishedLanguage)
- RustReader (PublishedLanguage)

## Imports

- Graph from equivalence (PublishedLanguage)
- Reader from equivalence (Conformist)

## Concepts

See specs/concepts/core.md — this section is prose-only.
";
    let decl = parse(src).expect("parse");
    assert_eq!(decl.name, "reading");
    assert_eq!(decl.owned_units.len(), 2);
    assert_eq!(decl.exports.len(), 2);
    assert_eq!(decl.imports.len(), 2);
}

#[test]
fn unknown_pattern_in_exports_yields_parse_error() {
    let src = "# foo\n\n## Exports\n\n- Foo (NotARealPattern)\n";
    let err = parse(src).expect_err("should fail");
    match err {
        ReaderError::ParseFailed { message, .. } => {
            assert!(
                message.contains("NotARealPattern"),
                "error should name the offending token, got: {message}"
            );
        }
        other => panic!("expected ParseFailed, got {other:?}"),
    }
}

#[test]
fn unknown_pattern_in_imports_yields_parse_error() {
    let src = "# foo\n\n## Imports\n\n- Bar from baz (Gibberish)\n";
    let err = parse(src).expect_err("should fail");
    assert!(matches!(err, ReaderError::ParseFailed { .. }));
}

#[test]
fn malformed_export_bullet_yields_parse_error() {
    let src = "# foo\n\n## Exports\n\n- Graph\n";
    let err = parse(src).expect_err("should fail");
    assert!(matches!(err, ReaderError::ParseFailed { .. }));
}

#[test]
fn malformed_import_bullet_yields_parse_error() {
    let src = "# foo\n\n## Imports\n\n- Graph (PublishedLanguage)\n";
    // Missing "from <Context>"
    let err = parse(src).expect_err("should fail");
    assert!(matches!(err, ReaderError::ParseFailed { .. }));
}

#[test]
fn missing_h1_yields_parse_error() {
    let src = "## Owns\n\n- domain\n";
    let err = parse(src).expect_err("should fail");
    match err {
        ReaderError::ParseFailed { message, .. } => {
            assert!(message.contains("# ContextName"));
        }
        other => panic!("expected ParseFailed, got {other:?}"),
    }
}

#[test]
fn two_h1_headings_yields_parse_error() {
    let src = "# first\n\n# second\n";
    let err = parse(src).expect_err("should fail");
    assert!(matches!(err, ReaderError::ParseFailed { .. }));
}

#[test]
fn concepts_section_is_ignored() {
    let src = "\
# foo

## Concepts

- some bullet
- another
";
    let decl = parse(src).expect("parse");
    assert!(decl.owned_units.is_empty());
    assert!(decl.exports.is_empty());
    assert!(decl.imports.is_empty());
}

#[test]
fn source_line_points_at_h1() {
    let src = "\n\n# equivalence\n";
    let decl = parse(src).expect("parse");
    match decl.source {
        Source::Spec { line, .. } => assert_eq!(line, 3),
        Source::Code { .. } => panic!("expected Spec source"),
    }
}

#[test]
fn heading_annotations_do_not_affect_section_dispatch() {
    // RFC §3.1 allows annotations on section headings.
    let src = "\
# foo

## Exports (Published Language — what this context publishes)

- Graph (PublishedLanguage)
";
    let decl = parse(src).expect("parse");
    assert_eq!(decl.exports.len(), 1);
}
