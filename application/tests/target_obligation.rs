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

fn cargo_toml(dir: &Path) {
    write(
        dir,
        "Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
}

fn edge_findings(outcome: &domain::CheckOutcome) -> usize {
    outcome
        .violations
        .iter()
        .filter(|v| matches!(v, Violation::EdgeMissingInCode { .. }))
        .count()
}

#[test]
fn the_motivating_shape_reaches_zero_violations() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/contract.md",
        "# contract\n\n## Assertion\n\n- depends on: AssertionScope\n\n## AssertionScope\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Assertion;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert!(
        outcome.is_clean(),
        "the retirement's intermediate commit is legal: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.retirement_complete.len(), 1);
}

#[test]
fn the_same_shape_with_the_item_still_present_stays_armed() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/contract.md",
        "# contract\n\n## Assertion\n\n- depends on: AssertionScope\n\n## AssertionScope\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(
        code.path(),
        "src/lib.rs",
        "pub struct Assertion;\npub struct AssertionScope;\n",
    );

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        edge_findings(&outcome),
        1,
        "the item is there, so the edge is satisfiable and must stay armed: {:?}",
        outcome.violations
    );
    assert_eq!(
        outcome.retirement_incomplete.len(),
        1,
        "and the retirement is reported incomplete"
    );
}

#[test]
fn a_name_is_pointable_when_any_heading_carrying_it_is() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "# alpha\n\n## S\n\n- depends on: T\n\n## T\n<!-- parent:spec:S polarity:illustrative -->\n\nProse.\n",
    );
    write(
        specs.path(),
        "concepts/beta.md",
        "# beta\n\n## T\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\npub struct T;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        edge_findings(&outcome),
        1,
        "the declared heading owns a real item, so the edge is satisfiable: {:?}",
        outcome.violations
    );
    assert!(
        !outcome.is_clean(),
        "a permissive per-name key would take this to exit 0"
    );
}

#[test]
fn a_name_is_unpointable_only_when_every_heading_carrying_it_is() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "# alpha\n\n## S\n\n- depends on: T\n\n## T\n<!-- parent:spec:S polarity:illustrative -->\n\nProse.\n",
    );
    write(
        specs.path(),
        "concepts/beta.md",
        "# beta\n\n## T\n<!-- parent:spec:S polarity:illustrative -->\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        edge_findings(&outcome),
        0,
        "every heading carrying `T` is unpointable, so the edge is suppressed: {:?}",
        outcome.violations
    );
}

#[test]
fn one_unpointable_heading_does_not_make_a_shared_name_unpointable() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "# alpha\n\n## S\n\n- depends on: T\n\n## T\n<!-- parent:spec:S polarity:illustrative -->\n\nProse.\n",
    );
    write(
        specs.path(),
        "concepts/beta.md",
        "# beta\n\n## T\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        edge_findings(&outcome),
        1,
        "one unpointable heading must not carry the name: {:?}",
        outcome.violations
    );
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "T")),
        "and row 1 still fires on the declared heading: {:?}",
        outcome.violations
    );
}
