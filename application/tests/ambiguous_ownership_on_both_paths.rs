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

fn nested_declarations() -> (TempDir, TempDir) {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "# outer\n\n## Clock\n\nThe clock.\n",
    );
    write(
        specs.path(),
        "contexts/outer.md",
        "# outer\n\n## Owns\n\n- domain\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        specs.path(),
        "contexts/inner.md",
        "# inner\n\n## Owns\n\n- domain/enrolment\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        code.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"domain/enrolment\"]\n",
    );
    write(
        code.path(),
        "domain/enrolment/Cargo.toml",
        "[package]\nname = \"enrolment\"\nversion = \"0.1.0\"\n",
    );
    write(
        code.path(),
        "domain/enrolment/src/lib.rs",
        "pub struct Clock;\n",
    );
    (specs, code)
}

#[test]
fn the_source_walk_path_refuses_nested_prefixes() {
    let (specs, code) = nested_declarations();
    let error = application::run_check(specs.path(), code.path(), None)
        .expect_err("two contexts nesting their prefixes have no declared surface");
    let message = error.to_string();
    assert!(
        message.contains("could not run the declared surface"),
        "the source-walk path refuses with the same sentence the keyspace path prints: {message}"
    );
    assert!(
        message.contains("`outer`") && message.contains("`inner`"),
        "and names both contexts: {message}"
    );
    assert!(
        message.contains("`domain`") && message.contains("`domain/enrolment`"),
        "and both prefixes: {message}"
    );
}

#[test]
fn a_well_formed_declaration_still_runs_on_the_source_walk_path() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "# outer\n\n## Clock\n\nThe clock.\n",
    );
    write(
        specs.path(),
        "contexts/outer.md",
        "# outer\n\n## Owns\n\n- domain\n\n## Exports (Published Language)\n\n## Imports\n",
    );
    write(
        code.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"domain/enrolment\"]\n",
    );
    write(
        code.path(),
        "domain/enrolment/Cargo.toml",
        "[package]\nname = \"enrolment\"\nversion = \"0.1.0\"\n",
    );
    write(
        code.path(),
        "domain/enrolment/src/lib.rs",
        "pub struct Clock;\n",
    );

    let outcome = application::run_check(specs.path(), code.path(), None)
        .expect("one context owning a prefix is not an ambiguity");
    assert!(outcome.is_clean(), "{:?}", outcome.violations);
}
