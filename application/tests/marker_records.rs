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

#[test]
fn draft_implementation_reports_a_realized_marker_record() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/widget.md",
        "---\nstatus: draft\n---\n\n## Widget\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Widget;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        outcome.realized.len(),
        1,
        "expected exactly one realized record for Widget, got: {outcome:?}"
    );
    assert_eq!(outcome.realized[0].concept, "Widget");
    assert!(
        !outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInSpecs { name, .. } if name == "Widget")),
        "a marked heading satisfies its pub item: {:?}",
        outcome.violations
    );
    assert!(
        outcome.is_clean(),
        "code backing a marked heading is the happy path, not a red gate: {:?}",
        outcome.violations
    );
}

fn write_mixed_fixture(specs: &Path, code: &Path, widget_decl: &str) {
    write(
        specs,
        "concepts/legacy.md",
        "---\nstatus: draft\n---\n\n## Reconciler\n",
    );
    write(
        specs,
        "concepts/core.md",
        "## Widget\n\n- status: draft\n\n```rust\npub struct Widget;\n```\n\n## Digest\n\n- status: draft\n\nNot built yet.\n",
    );
    cargo_toml(code);
    write(code, "src/lib.rs", widget_decl);
}

#[test]
fn mixed_fixture_yields_one_pending_one_realized_and_exits_zero() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_mixed_fixture(specs.path(), code.path(), "pub struct Widget;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome.is_clean(),
        "marker records never fail the gate: {:?}",
        outcome.violations
    );

    let realized: Vec<&str> = outcome
        .realized
        .iter()
        .map(|r| r.concept.as_str())
        .collect();
    assert_eq!(realized, vec!["Widget"]);

    let pending: Vec<&str> = outcome.pending.iter().map(|p| p.concept.as_str()).collect();
    assert_eq!(
        pending,
        vec!["Digest", "Reconciler"],
        "both the per-heading marker and the file-scope marker produce pending records"
    );

    let mut buf = Vec::new();
    application::ndjson::write_ndjson(&outcome, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    let markers: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|r| r.get("marker").is_some())
        .collect();
    assert_eq!(markers.len(), 3);
    assert_eq!(markers[0]["marker"], "pending");
    assert_eq!(markers[0]["concept"], "Digest");
    assert_eq!(markers[2]["marker"], "realized");
    assert_eq!(markers[2]["schema_version"], "5");
}

#[test]
fn a_real_divergence_under_a_marked_heading_still_reds_the_gate() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_mixed_fixture(specs.path(), code.path(), "pub enum Widget { A }\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::SignatureDrift { name, .. } if name == "Widget")),
        "expected SignatureDrift under the marked heading, got: {:?}",
        outcome.violations
    );
    assert!(!outcome.is_clean());
    assert_eq!(
        outcome.realized.len(),
        1,
        "the realized record still rides alongside the violation"
    );
}

#[test]
fn a_misplaced_marker_leaves_the_anti_invention_check_armed() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "## Widget\n\nSome prose first.\n\n- status: draft\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Widget")),
        "a misplaced marker must fail loud, not silently suppress: {:?}",
        outcome.violations
    );
    assert!(outcome.pending.is_empty());
}

#[test]
fn a_pending_concepts_verb_anchor_imposes_no_obligation() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "## Reconciler\n\n- status: draft\n- verb: reconcile\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome.is_clean(),
        "a pending concept's verb anchor imposes nothing: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
}

#[test]
fn an_h1_only_draft_doc_reds_the_check() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/reading.md",
        "---\nstatus: draft\n---\n\n# reading\n\nJust prose, no concept.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome.violations.iter().any(|v| matches!(
            v,
            Violation::Cohesion(domain::CohesionViolation::ContextWithoutCohesionUnit {
                context,
                ..
            }) if context == "reading"
        )),
        "expected context_without_cohesion_unit on the draft doc, got: {:?}",
        outcome.violations
    );
    assert!(!outcome.is_clean());
}

#[test]
fn adding_a_marked_heading_greens_it_back_to_pending_only() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/reading.md",
        "---\nstatus: draft\n---\n\n# reading\n\n## Widget\n\nNot built yet.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        outcome.is_clean(),
        "a marked heading satisfies its context: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
    assert_eq!(outcome.pending[0].concept, "Widget");
}

#[test]
fn both_retirement_records_emit_end_to_end_and_the_tree_exits_clean() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/widget.md",
        "## Widget\n\n- status: retired\n\n## Gadget\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Widget;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();

    assert_eq!(
        outcome.retirement_incomplete.len(),
        1,
        "Widget's item is still present — row 7: {outcome:?}"
    );
    assert_eq!(outcome.retirement_incomplete[0].concept, "Widget");
    assert_eq!(
        outcome.retirement_complete.len(),
        1,
        "Gadget's item is gone — row 8: {outcome:?}"
    );
    assert_eq!(outcome.retirement_complete[0].concept, "Gadget");
    assert!(
        outcome.pending.is_empty() && outcome.realized.is_empty(),
        "the `draft` pair must stay empty — the values do not bleed"
    );
    assert!(
        outcome.is_clean(),
        "neither retirement record moves the exit code: {:?}",
        outcome.violations
    );
}

#[test]
fn retirement_records_carry_the_current_schema_version_on_the_wire() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/widget.md",
        "## Widget\n\n- status: retired\n\n## Gadget\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Widget;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let mut buf = Vec::new();
    application::ndjson::write_ndjson(&outcome, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();

    for marker in ["retirement_incomplete", "retirement_complete"] {
        let line = out
            .lines()
            .find(|l| l.contains(&format!("\"marker\":\"{marker}\"")))
            .unwrap_or_else(|| panic!("no {marker} record in:\n{out}"));
        assert!(
            line.contains(&format!(
                "\"schema_version\":\"{}\"",
                domain::SchemaVersion::CURRENT.as_str()
            )),
            "{marker} rides the current schema version and carries none of its own: {line}"
        );
        assert!(
            !line.contains("\"violation\""),
            "exactly one of violation/marker is present: {line}"
        );
    }
}

#[test]
fn the_text_summary_renders_every_list_even_at_zero() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(specs.path(), "concepts/widget.md", "## Widget\n");
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Widget;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let mut buf = Vec::new();
    application::text::format_summary(&outcome, &mut buf).unwrap();

    assert_eq!(
        String::from_utf8(buf).unwrap().trim_end(),
        "0 violations, 0 pending, 0 realized-unratified, 0 retirement-incomplete, 0 retirement-complete"
    );
}
