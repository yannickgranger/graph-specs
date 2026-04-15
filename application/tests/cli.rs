//! End-to-end CLI tests via `assert_cmd`.
//!
//! Builds the real `graph-specs` binary and drives it against temporary
//! fixture directories. Covers the four AC scenarios for issue #3:
//! empty, matching, spec-only concept (`MissingInCode`), code-only concept
//! (`MissingInSpecs`).

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn write_file(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(&full).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

fn bin() -> Command {
    Command::cargo_bin("graph-specs").expect("graph-specs binary built")
}

#[test]
fn empty_specs_and_empty_code_pass() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 violations"));
}

#[test]
fn matching_specs_and_code_pass() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n## Bar\n");
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct Foo; pub enum Bar { X }",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 violations"));
}

#[test]
fn spec_only_concept_exits_1_with_missing_in_code() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n## Orphan\n");
    write_file(code.path(), "src/lib.rs", "pub struct Foo;");

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("missing in code: Orphan"))
        .stdout(predicate::str::contains("1 violation"));
}

// --- v0.2 signature-level integration + inject-bite ---

#[test]
fn injectbite_rename_field_in_spec_only() {
    // Spec says the field is `pub uuid: Uuid`; code says `pub id: Uuid`.
    // The concept name matches, but the normalised signatures diverge.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## OrderId\n\n```rust\npub struct OrderId { pub uuid: Uuid }\n```\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct OrderId { pub id: Uuid }",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("signature drift: OrderId"))
        .stdout(predicate::str::contains("uuid"))
        .stdout(predicate::str::contains("id"));
}

#[test]
fn injectbite_add_variant_in_code_only() {
    // Spec has enum with one variant; code adds a second.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Status\n\n```rust\npub enum Status { Open }\n```\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub enum Status { Open, Closed }",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("signature drift: Status"))
        .stdout(predicate::str::contains("Closed"));
}

#[test]
fn injectbite_change_generic_bound_in_spec_only() {
    // Spec says `T: Copy`; code says `T: Clone`.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Holder\n\n```rust\npub struct Holder<T: Copy>(pub T);\n```\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct Holder<T: Clone>(pub T);",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("signature drift: Holder"));
}

#[test]
fn matching_signatures_yield_no_violations() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## OrderId\n\n```rust\npub struct OrderId(pub u32);\n```\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct OrderId(pub u32);",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 violations"));
}

#[test]
fn unparseable_spec_rust_block_exits_2() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## OrderId\n\n```rust\npub struct OrderId(\n```\n",
    );
    write_file(code.path(), "src/lib.rs", "pub struct OrderId(pub u32);");

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("signature unparseable: OrderId"));
}

#[test]
fn concept_only_spec_does_not_emit_signature_violation() {
    // Backward compat: a v0.1-style spec with no rust block coexists with
    // code that has a signature. Should pass (opt-in semantics).
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## OrderId\n");
    write_file(code.path(), "src/lib.rs", "pub struct OrderId(pub u32);");

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 violations"));
}

#[test]
fn code_only_concept_exits_1_with_missing_in_specs() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n");
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct Foo; pub struct Undeclared;",
    );

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("missing in specs: Undeclared"))
        .stdout(predicate::str::contains("1 violation"));
}
