//! Integration test: a `status: draft` spec implemented by a `pub` item
//! yields `ImplementsDraftConcept` and no `MissingInSpecs` for that name.

use domain::Violation;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn write(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(&full).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

#[test]
fn draft_implementation_reports_implements_draft_concept() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/widget.md",
        "---\nstatus: draft\n---\n\n## Widget\n",
    );

    write(
        code.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    write(code.path(), "src/lib.rs", "pub struct Widget;\n");

    let violations = application::run_check(specs.path(), code.path()).unwrap();

    let implements_draft_count = violations
        .iter()
        .filter(|v| matches!(v, Violation::ImplementsDraftConcept { name, .. } if name == "Widget"))
        .count();
    assert_eq!(
        implements_draft_count, 1,
        "expected exactly one ImplementsDraftConcept for Widget, got violations: {violations:?}"
    );

    let missing_in_specs_count = violations
        .iter()
        .filter(|v| matches!(v, Violation::MissingInSpecs { name, .. } if name == "Widget"))
        .count();
    assert_eq!(
        missing_in_specs_count, 0,
        "Widget must not appear as MissingInSpecs"
    );
}
