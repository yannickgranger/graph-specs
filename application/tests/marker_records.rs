//! Integration tests for the RFC-013 spec-state marker, end to end through
//! `run_check` and the NDJSON emitter.
//!
//! Rewritten from `draft_diagnostic.rs`, which asserted the retired
//! `ImplementsDraftConcept` variant (RFC-013 §7 Slice A: rewritten, not
//! deleted). Its case — a `status: draft` spec implemented by a `pub` item —
//! survives below with the polarity flipped: that condition is now a
//! `realized` marker record, not a violation.

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

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

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

/// The fixture the RFC prescribes: a draft-front-matter file plus a ratified
/// file carrying one per-heading-marked concept with no code and one with.
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

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
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
    assert_eq!(markers[2]["schema_version"], "4");
}

#[test]
fn a_real_divergence_under_a_marked_heading_still_reds_the_gate() {
    // RFC-013 §4 invariant 1 — a marker never parks a divergence. The spec
    // declares `pub struct Widget;`; the code declares an enum.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write_mixed_fixture(specs.path(), code.path(), "pub enum Widget { A }\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
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
    // RFC-013 §3.1 fail-loud: the marker bullet is not the first non-blank
    // content line, so it is inert and `MissingInCode` fires as usual.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "## Widget\n\nSome prose first.\n\n- status: draft\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
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
    // RFC-013 §3.4 uniform obligation skip, end to end.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/core.md",
        "## Reconciler\n\n- status: draft\n- verb: reconcile\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
    assert!(
        outcome.is_clean(),
        "a pending concept's verb anchor imposes nothing: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
}

// --- RFC-013 §3.2 row 6 — cohesion tightening (Slice B) ---

#[test]
fn an_h1_only_draft_doc_reds_the_check() {
    // Before RFC-013 a doc could enter the enforced surface carrying no
    // cohesion unit at all by declaring itself draft. That channel is closed:
    // marking relaxes a concept's code-existence obligation, never the
    // doc-level structural check.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/reading.md",
        "---\nstatus: draft\n---\n\n# reading\n\nJust prose, no concept.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
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
    // A marked heading COUNTS as a cohesion unit — the assembler records
    // heading depth and is marker-blind by construction.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/reading.md",
        "---\nstatus: draft\n---\n\n# reading\n\n## Widget\n\nNot built yet.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
    assert!(
        outcome.is_clean(),
        "a marked heading satisfies its context: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
    assert_eq!(outcome.pending[0].concept, "Widget");
}
