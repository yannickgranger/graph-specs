use domain::{CohesionViolation, Violation};
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

fn tree() -> (TempDir, TempDir) {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "contexts/scheduling.md",
        "# scheduling\n\n## Owns\n\n- crates/scheduling\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        specs.path(),
        "contexts/privacy.md",
        "# privacy\n\n## Owns\n\n- crates/privacy\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        specs.path(),
        "concepts/scheduling.md",
        "# scheduling\n\n## Clock\n\nThe clock this context holds for itself.\n",
    );
    write(
        specs.path(),
        "concepts/privacy.md",
        "# privacy\n\n## Clock\n\nThe clock this context holds for itself.\n",
    );

    write(
        code.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/scheduling\", \"crates/privacy\"]\n",
    );
    for unit in ["scheduling", "privacy"] {
        write(
            code.path(),
            &format!("crates/{unit}/Cargo.toml"),
            &format!("[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\n"),
        );
        write(
            code.path(),
            &format!("crates/{unit}/src/lib.rs"),
            "pub struct Clock;\n",
        );
    }
    (specs, code)
}

#[test]
fn two_documents_declaring_one_name_each_keep_their_own_context() {
    let (specs, code) = tree();
    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    let mismatches: Vec<_> = outcome
        .violations
        .iter()
        .filter(|v| {
            matches!(
                v,
                Violation::Cohesion(CohesionViolation::ConceptContextMismatch { .. })
            )
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "each document's Clock is declared by its own document, so each binds the item of its own \
         unit: {mismatches:?}"
    );
    assert!(
        outcome.is_clean(),
        "and nothing else is reported: {:?}",
        outcome.violations
    );
}

#[test]
fn a_tree_declaring_no_contexts_still_binds_by_name() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/reading.md",
        "# reading\n\n## Clock\n\nThe clock.\n",
    );
    write(
        code.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    write(code.path(), "src/lib.rs", "pub struct Clock;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert!(
        outcome.is_clean(),
        "with no specs/contexts nothing declares a unit, so no code fact can carry a context and \
         binding must not ask for one: {:?}",
        outcome.violations
    );
}
