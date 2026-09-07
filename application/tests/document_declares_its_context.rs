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

fn tree(h1: Option<&str>) -> (TempDir, TempDir) {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    let heading = h1.map_or_else(String::new, |text| format!("{text}\n\n"));
    write(
        specs.path(),
        "concepts/core.md",
        &format!("{heading}## Clock\n\nThe clock.\n"),
    );
    write(
        specs.path(),
        "contexts/scheduling.md",
        "# scheduling\n\n## Owns\n\n- crates/scheduling\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        code.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/scheduling\"]\n",
    );
    write(
        code.path(),
        "crates/scheduling/Cargo.toml",
        "[package]\nname = \"scheduling\"\nversion = \"0.1.0\"\n",
    );
    write(
        code.path(),
        "crates/scheduling/src/lib.rs",
        "pub struct Clock;\n",
    );
    (specs, code)
}

#[test]
fn a_declared_document_runs() {
    let (specs, code) = tree(Some("# scheduling"));
    let outcome = application::run_check(specs.path(), code.path(), None)
        .expect("a document naming a declared context runs");
    assert!(outcome.is_clean(), "{:?}", outcome.violations);
}

#[test]
fn a_document_with_no_context_heading_is_a_reader_error() {
    let (specs, code) = tree(None);
    let error = application::run_check(specs.path(), code.path(), None)
        .expect_err("a document declaring no context cannot be read against a tree that declares");
    let message = error.to_string();
    assert!(
        message.contains("core.md"),
        "the refusal names the document: {message}"
    );
    assert!(
        message.contains("carries no `#` heading"),
        "and says what it found: {message}"
    );
    assert!(
        message.contains("scheduling"),
        "and names the declared context(s) it could have matched: {message}"
    );
}

#[test]
fn a_descriptive_heading_matching_no_declared_context_is_a_reader_error() {
    let (specs, code) = tree(Some("# Core concepts: the equivalence layer"));
    let error = application::run_check(specs.path(), code.path(), None)
        .expect_err("a descriptive H1 declares no context and must not bind by name alone");
    let message = error.to_string();
    assert!(
        message.contains("core.md"),
        "the refusal names the document: {message}"
    );
    assert!(
        message.contains("Core concepts: the equivalence layer"),
        "and quotes the heading it read: {message}"
    );
    assert!(
        message.contains("scheduling"),
        "and names the declared context(s): {message}"
    );
}

#[test]
fn an_identifier_shaped_heading_matching_nothing_declares_a_context_owning_no_unit() {
    let (specs, code) = tree(Some("# privacy"));
    let outcome = application::run_check(specs.path(), code.path(), None)
        .expect("an identifier-shaped H1 declares a context, so the run is not refused");
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, domain::Violation::MissingInCode { name, .. } if name == "Clock")),
        "the context it declares owns no unit, so its heading reads missing in code rather than \
         binding by name alone: {:?}",
        outcome.violations
    );
}
