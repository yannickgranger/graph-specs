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

// --- v0.3 relationship-level inject-bite (AC #6) ---

#[test]
fn injectbite_spec_implements_without_code_impl_emits_edge_missing_in_code() {
    // Spec: MarkdownReader section declares `- implements: Reader`.
    // Code: MarkdownReader is a pub struct, but Reader is a trait with no
    // `impl Reader for MarkdownReader` block. Result: EdgeMissingInCode.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Reader\n\n## MarkdownReader\n\n- implements: Reader\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct MarkdownReader; pub trait Reader {}",
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
        .stdout(predicate::str::contains(
            "edge missing in code: MarkdownReader --IMPLEMENTS--> Reader",
        ));
}

#[test]
fn injectbite_code_impl_without_spec_bullet_emits_edge_missing_in_spec() {
    // Spec: MarkdownReader has at least one bullet (opts in) but omits
    // the Writer one. Code: `impl Writer for MarkdownReader` exists.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Reader\n\n## Writer\n\n## MarkdownReader\n\n- implements: Reader\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct MarkdownReader; pub trait Reader {} pub trait Writer {} impl Reader for MarkdownReader {} impl Writer for MarkdownReader {}",
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
        .stdout(predicate::str::contains(
            "edge missing in spec: MarkdownReader --IMPLEMENTS--> Writer",
        ));
}

#[test]
fn injectbite_spec_edge_target_unknown_concept_emits_target_unknown() {
    // Spec references a concept that exists on neither side.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## MarkdownReader\n\n- implements: NotAConcept\n",
    );
    write_file(code.path(), "src/lib.rs", "pub struct MarkdownReader;");

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
        .stdout(predicate::str::contains(
            "edge target unknown: MarkdownReader --IMPLEMENTS--> NotAConcept",
        ));
}

#[test]
fn injectbite_field_rename_pair_emits_missing_in_code_and_spec() {
    // Spec: Container depends on Graph. Code: Container's field is Node,
    // not Graph. Both EdgeMissingInCode (Graph) and EdgeMissingInSpec (Node)
    // fire for the same concept.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Graph\n\n## Node\n\n## Container\n\n- depends on: Graph\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct Graph; pub struct Node; pub struct Container { pub n: Node }",
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
        .stdout(predicate::str::contains(
            "edge missing in code: Container --DEPENDS_ON--> Graph",
        ))
        .stdout(predicate::str::contains(
            "edge missing in spec: Container --DEPENDS_ON--> Node",
        ));
}

#[test]
fn v03_matching_edges_produce_no_violations() {
    // Regression: a spec that declares the same edges the code has must
    // still produce 0 violations.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "core.md",
        "## Reader\n\n## MarkdownReader\n\n- implements: Reader\n",
    );
    write_file(
        code.path(),
        "src/lib.rs",
        "pub struct MarkdownReader; pub trait Reader {} impl Reader for MarkdownReader {}",
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

// --- NDJSON output (issue #13) --------------------------------------------

fn run_ndjson(specs: &Path, code: &Path) -> std::process::Output {
    bin()
        .args([
            "check",
            "--specs",
            specs.to_str().unwrap(),
            "--code",
            code.to_str().unwrap(),
            "--format",
            "ndjson",
        ])
        .output()
        .expect("run")
}

fn parse_ndjson(stdout: &[u8]) -> Vec<serde_json::Value> {
    let s = std::str::from_utf8(stdout).expect("utf8");
    s.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("valid json line"))
        .collect()
}

#[test]
fn ndjson_on_clean_tree_emits_empty_stdout_and_exit_zero() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n");
    write_file(code.path(), "src/lib.rs", "pub struct Foo;");

    let out = run_ndjson(specs.path(), code.path());
    assert_eq!(out.status.code(), Some(0));
    assert!(
        out.stdout.is_empty(),
        "ndjson on clean tree must emit no stdout; got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn ndjson_missing_in_code_emits_one_record_exit_one() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## OnlySpec\n");
    write_file(code.path(), "src/lib.rs", "");

    let out = run_ndjson(specs.path(), code.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["schema_version"], "3");
    assert_eq!(records[0]["violation"], "missing_in_code");
    assert_eq!(records[0]["concept"], "OnlySpec");
    assert_eq!(records[0]["source"]["kind"], "spec");
}

#[test]
fn ndjson_missing_in_specs_emits_one_record_exit_one() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "");
    write_file(code.path(), "src/lib.rs", "pub struct OnlyCode;");

    let out = run_ndjson(specs.path(), code.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["violation"], "missing_in_specs");
    assert_eq!(records[0]["concept"], "OnlyCode");
    assert_eq!(records[0]["source"]["kind"], "code");
}

#[test]
fn ndjson_signature_unparseable_exits_two() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n\n```rust\nfn foo(\n```\n");
    write_file(code.path(), "src/lib.rs", "pub struct Foo;");

    let out = run_ndjson(specs.path(), code.path());
    assert_eq!(out.status.code(), Some(2));
    let records = parse_ndjson(&out.stdout);
    assert!(
        records
            .iter()
            .any(|r| r["violation"] == "signature_unparseable"),
        "expected signature_unparseable record, got: {records:?}"
    );
}

#[test]
fn ndjson_multiple_violations_newline_delimited_each_parseable() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## SpecOnly\n");
    write_file(code.path(), "src/lib.rs", "pub struct CodeOnly;");

    let out = run_ndjson(specs.path(), code.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert_eq!(records.len(), 2);
    // Each record must parse independently — the invariant of NDJSON.
    // parse_ndjson already asserts this (it would panic on invalid line).
    let violations: Vec<&str> = records
        .iter()
        .map(|r| r["violation"].as_str().unwrap())
        .collect();
    assert!(violations.contains(&"missing_in_code"));
    assert!(violations.contains(&"missing_in_specs"));
}

#[test]
fn ndjson_text_format_unchanged_by_flag_absence() {
    // Regression: default output (no --format) must match legacy text.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(specs.path(), "core.md", "## Foo\n");
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
        .success()
        .stdout(predicate::str::contains("0 violations"));
}

// --- v0.4 bounded-context inject-bite (issue #28 AC) ---------------------
//
// Runs the CLI with `--specs <tmp>/specs --code <tmp>/code` and asserts
// each `ContextViolation` variant surfaces end-to-end. These tests were
// deferred from #26 (NDJSON) and #27 (text) because `run_check` did not
// yet load context declarations. With #28 wiring both readers, the three
// variants can finally be exercised through the real CLI.

/// Build a canonical v0.4 layout under a tmpdir:
///   <root>/specs/concepts/core.md  (v0.1 concept headings)
///   <root>/specs/contexts/*.md     (v0.4 context declarations)
///   <root>/<unit>/src/lib.rs       (code — `--code .` picks it up)
///
/// Returns the `TempDir` guard; the caller runs the binary with
/// `current_dir(root)` so the source paths resolve to `./<unit>/src/...`,
/// which after `trim_start_matches("./")` + `split("/src/")` yields
/// unit strings that can be matched against context `Owns` entries.
fn v04_fixture(concepts: &str, contexts: &[(&str, &str)], code_files: &[(&str, &str)]) -> TempDir {
    let root = TempDir::new().unwrap();
    write_file(root.path(), "specs/concepts/core.md", concepts);
    for (name, body) in contexts {
        write_file(root.path(), &format!("specs/contexts/{name}.md"), body);
    }
    for (rel, body) in code_files {
        write_file(root.path(), rel, body);
    }
    root
}

fn run_v04_ndjson(root: &Path) -> std::process::Output {
    bin()
        .current_dir(root)
        .args([
            "check", "--specs", "specs/", "--code", ".", "--format", "ndjson",
        ])
        .output()
        .expect("run")
}

fn run_v04_text(root: &Path) -> std::process::Output {
    bin()
        .current_dir(root)
        .args(["check", "--specs", "specs/", "--code", "."])
        .output()
        .expect("run")
}

#[test]
fn injectbite_v04_membership_unknown_surfaces_in_text_and_ndjson() {
    // Code has a concept in `beta-unit/src/...`, but no context declares
    // `beta-unit` under `Owns` — alpha only owns `alpha-unit`. Tool must
    // flag MembershipUnknown for the stray concept.
    let root = v04_fixture(
        "## Stray\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[("beta-unit/src/lib.rs", "pub struct Stray;")],
    );

    // Text output
    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("context membership unknown: Stray"),
        "text: {stdout}"
    );

    // NDJSON output
    let out = run_v04_ndjson(root.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert!(
        records
            .iter()
            .any(|r| r["violation"] == "context_membership_unknown"
                && r["concept"] == "Stray"
                && r["owned_unit"] == "beta-unit"),
        "ndjson: {records:?}"
    );
}

#[test]
fn injectbite_v04_cross_edge_unauthorized_surfaces_in_text_and_ndjson() {
    // Two contexts, no imports between them. Code has an `impl Foo for
    // Impl` spanning alpha and beta. The cross-context edge lacks any
    // Imports entry in beta, so CrossEdgeUnauthorized must fire.
    let root = v04_fixture(
        "## Foo\n## Impl\n",
        &[
            ("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n"),
            ("beta", "# beta\n\n## Owns\n\n- beta-unit\n"),
        ],
        &[
            ("alpha-unit/src/lib.rs", "pub trait Foo {}"),
            (
                "beta-unit/src/lib.rs",
                "use alpha_unit::Foo; pub struct Impl; impl Foo for Impl {}",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("cross-context edge unauthorized: Impl"),
        "text: {stdout}"
    );

    let out = run_v04_ndjson(root.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert!(
        records
            .iter()
            .any(|r| r["violation"] == "cross_context_edge_unauthorized"
                && r["concept"] == "Impl"
                && r["owning_context"] == "beta"
                && r["target"] == "Foo"
                && r["target_context"] == "alpha"),
        "ndjson: {records:?}"
    );
}

// --- v0.5 verb-anchoring inject-bite (issue #103 AC) ---------------------
//
// Six cases: match, VerbMissingInCode, VerbMissingInSpec, VerbTargetUnknown,
// CrossVerbUnauthorized, and the zero-anchor no-op. Each drives the real
// binary with `--specs specs/ --code .` against a v0.4-style tmpdir layout.
// Cargo.toml stubs in unit directories are required so `find_owned_unit`
// resolves the workspace-relative unit name.

#[test]
fn injectbite_v05_verb_match_produces_no_violations() {
    // Spec: ConceptA anchors `- verb: my_fn`. Code: pub fn my_fn in alpha-unit
    // owned by context alpha. Exact match → no violations.
    let root = v04_fixture(
        "## ConceptA\n\n- verb: my_fn\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            (
                "alpha-unit/src/lib.rs",
                "pub struct ConceptA; pub fn my_fn() {}",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert_eq!(text.status.code(), Some(0), "text: {stdout}");
    assert!(stdout.contains("0 violations"), "text: {stdout}");
}

#[test]
fn injectbite_v05_verb_missing_in_code_surfaces_in_text() {
    // Spec: ConceptA anchors `- verb: absent_fn`. Code: no pub fn absent_fn.
    // Expects VerbMissingInCode.
    let root = v04_fixture(
        "## ConceptA\n\n- verb: absent_fn\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            ("alpha-unit/src/lib.rs", "pub struct ConceptA;"),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("verb missing in code: ConceptA claims `absent_fn`"),
        "text: {stdout}"
    );
}

#[test]
fn injectbite_v05_verb_missing_in_spec_surfaces_in_text() {
    // Spec: ConceptA anchors `- verb: claimed_fn` but NOT unclaimed_fn.
    // Code: alpha-unit exposes both. Expects VerbMissingInSpec for unclaimed_fn.
    let root = v04_fixture(
        "## ConceptA\n\n- verb: claimed_fn\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            (
                "alpha-unit/src/lib.rs",
                "pub struct ConceptA; pub fn claimed_fn() {} pub fn unclaimed_fn() {}",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("verb missing in spec: `unclaimed_fn` is unclaimed"),
        "text: {stdout}"
    );
}

#[test]
fn injectbite_v05_verb_target_unknown_surfaces_in_text() {
    // Spec: ConceptA anchors `- verb: ghost_fn`. Code: pub fn ghost_fn exists
    // in orphan-unit which has no Cargo.toml → owned_unit is None → no context
    // owns it. Expects VerbTargetUnknown.
    let root = v04_fixture(
        "## ConceptA\n\n- verb: ghost_fn\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            ("alpha-unit/src/lib.rs", "pub struct ConceptA;"),
            ("orphan-unit/src/lib.rs", "pub fn ghost_fn() {}"),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("verb target unknown: ConceptA claims `ghost_fn`"),
        "text: {stdout}"
    );
}

#[test]
fn injectbite_v05_cross_verb_unauthorized_surfaces_in_text() {
    // Spec: ConceptA (in alpha context) anchors `- verb: cross_fn`.
    // Code: cross_fn is pub fn in beta-unit (beta context). No import declared.
    // Expects CrossVerbUnauthorized.
    let root = v04_fixture(
        "## ConceptA\n\n- verb: cross_fn\n",
        &[
            ("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n"),
            ("beta", "# beta\n\n## Owns\n\n- beta-unit\n"),
        ],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            ("alpha-unit/src/lib.rs", "pub struct ConceptA;"),
            (
                "beta-unit/Cargo.toml",
                "[package]\nname = \"beta-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            ("beta-unit/src/lib.rs", "pub fn cross_fn() {}"),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains(
            "cross-context verb unauthorized: ConceptA (alpha) claims `cross_fn` which belongs to beta"
        ),
        "text: {stdout}"
    );
}

#[test]
fn injectbite_v06_impl_method_verb_anchor_matches_impl_block() {
    // Spec: Foo anchors `- verb: Foo::bar`. Code: impl Foo { pub fn bar() {} }.
    // After v0.6 impl-method walk, Foo::bar is in the decl set → exit 0.
    let root = v04_fixture(
        "## Foo\n\n- verb: Foo::bar\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            (
                "alpha-unit/src/lib.rs",
                "pub struct Foo; impl Foo { pub fn bar() {} }",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert_eq!(text.status.code(), Some(0), "text: {stdout}");
    assert!(stdout.contains("0 violations"), "text: {stdout}");
}

#[test]
fn v05_zero_verb_bullets_verb_pass_is_noop() {
    // Spec has no `- verb:` bullets → verb pass skipped entirely.
    // Code: alpha-unit exposes pub fn any_fn. No VerbMissingInSpec should fire.
    let root = v04_fixture(
        "## ConceptA\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            (
                "alpha-unit/src/lib.rs",
                "pub struct ConceptA; pub fn any_fn() {}",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert_eq!(text.status.code(), Some(0), "text: {stdout}");
    assert!(stdout.contains("0 violations"), "text: {stdout}");
}

#[test]
fn injectbite_v06_hybrid_opt_in_impl_method_vs_free_fn() {
    // Spec: ## Foo with `- verb: bar` (bare-ident); ## Other with no anchors.
    // Code: pub fn bar (claimed free fn), impl Foo { pub fn baz } (unclaimed
    // impl-method), pub fn loose_fn (unclaimed free fn), impl Other { pub fn
    // quux } (impl-method for non-opted-in concept).
    //
    // Expected VerbMissingInSpec: Foo::baz (impl-method branch) and loose_fn
    // (free-fn branch). Other::quux must NOT fire (per-concept narrowing).
    let root = v04_fixture(
        "## Foo\n\n- verb: bar\n\n## Other\n",
        &[("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n")],
        &[
            (
                "alpha-unit/Cargo.toml",
                "[package]\nname = \"alpha-unit\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            ),
            (
                "alpha-unit/src/lib.rs",
                "pub struct Foo; pub struct Other; \
                 pub fn bar() {} \
                 impl Foo { pub fn baz() {} } \
                 pub fn loose_fn() {} \
                 impl Other { pub fn quux() {} }",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("verb missing in spec: `Foo::baz` is unclaimed"),
        "impl-method branch: expected VerbMissingInSpec for Foo::baz; text: {stdout}"
    );
    assert!(
        stdout.contains("verb missing in spec: `loose_fn` is unclaimed"),
        "free-fn branch: expected VerbMissingInSpec for loose_fn; text: {stdout}"
    );
    assert!(
        !stdout.contains("Other::quux"),
        "per-concept narrowing: Other::quux must not fire (Other has no anchors); text: {stdout}"
    );
}

#[test]
fn injectbite_v04_cross_edge_undeclared_surfaces_in_text_and_ndjson() {
    // Beta imports `Foo from alpha (PublishedLanguage)`, but alpha does
    // NOT export `Foo`. The edge is authorized on the consumer side but
    // unsatisfied on the supplier side — CrossEdgeUndeclared fires.
    let root = v04_fixture(
        "## Foo\n## Impl\n",
        &[
            ("alpha", "# alpha\n\n## Owns\n\n- alpha-unit\n"),
            (
                "beta",
                "# beta\n\n## Owns\n\n- beta-unit\n\n## Imports\n\n- Foo from alpha (PublishedLanguage)\n",
            ),
        ],
        &[
            ("alpha-unit/src/lib.rs", "pub trait Foo {}"),
            (
                "beta-unit/src/lib.rs",
                "use alpha_unit::Foo; pub struct Impl; impl Foo for Impl {}",
            ),
        ],
    );

    let text = run_v04_text(root.path());
    assert_eq!(text.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("cross-context edge undeclared: Impl"),
        "text: {stdout}"
    );

    let out = run_v04_ndjson(root.path());
    assert_eq!(out.status.code(), Some(1));
    let records = parse_ndjson(&out.stdout);
    assert!(
        records
            .iter()
            .any(|r| r["violation"] == "cross_context_edge_undeclared"
                && r["concept"] == "Impl"
                && r["target"] == "Foo"),
        "ndjson: {records:?}"
    );
}

// --- RFC-010 §3.5 / R10-3 cohesion end-to-end (§12-F non-zero exit) ---

#[test]
fn spec_side_cohesion_violation_exits_non_zero() {
    // A `concepts/` file with an H1 but no concept under it is malformed —
    // ContextWithoutCohesionUnit. It must drive a non-zero exit and render
    // (not "unknown violation").
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "concepts/lonely.md",
        "# lonely\n\nprose only, no concept.\n",
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
        .failure()
        .code(1)
        .stdout(
            predicate::str::contains("context without cohesion unit: `lonely`")
                .and(predicate::str::contains("unknown violation").not()),
        );
}

#[test]
fn concept_context_mismatch_exits_non_zero_end_to_end() {
    // `Widget` is documented under H1 `reading` (concepts/reading.md) but
    // its code lives in crate `domain`, which `equivalence` Owns — a real
    // ConceptContextMismatch resolved through specs/contexts/ Owns.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_file(
        specs.path(),
        "contexts/equivalence.md",
        "# equivalence\n\n## Owns\n\n- domain\n",
    );
    write_file(
        specs.path(),
        "contexts/reading.md",
        "# reading\n\n## Owns\n\n- adapters/markdown\n",
    );
    write_file(
        specs.path(),
        "concepts/reading.md",
        "# reading\n\n## Widget\n",
    );
    write_file(
        code.path(),
        "domain/Cargo.toml",
        "[package]\nname = \"domain\"\n",
    );
    write_file(code.path(), "domain/src/lib.rs", "pub struct Widget;");

    bin()
        .args([
            "check",
            "--specs",
            specs.path().to_str().unwrap(),
            "--code",
            code.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(
            predicate::str::contains("concept context mismatch: Widget")
                .and(predicate::str::contains("declared in `reading`"))
                .and(predicate::str::contains("code resolves to `equivalence`")),
        );
}
