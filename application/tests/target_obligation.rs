//! Integration tests for RFC-015 §3.4's target-side rule, end to end through
//! the real markdown + Rust readers.
//!
//! Two things these fixtures prove that no unit matrix can. The unit
//! matrices are all single-heading, so they never reach the per-**name**
//! conjunction at all — and the name collision they test is not exotic: a
//! heading in one context illustrating a type really declared in another is
//! the canonical use of `illustrative`.

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

/// The §1 shape: a live concept depends on a name whose heading is retired,
/// and the retirement is complete. This is the commit that had no legal
/// intermediate state before RFC-015.
#[test]
fn the_motivating_shape_reaches_zero_violations() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/contract.md",
        "## Assertion\n\n- depends on: AssertionScope\n\n## AssertionScope\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Assertion;\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

    assert!(
        outcome.is_clean(),
        "the retirement's intermediate commit is legal: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.retirement_complete.len(), 1);
}

/// The same fixture with the target's item still PRESENT. This is what
/// proves the key is `unpointable` — marked AND absent — and not the marker
/// alone: with the item there, the spec edge is satisfiable and stays armed.
#[test]
fn the_same_shape_with_the_item_still_present_stays_armed() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/contract.md",
        "## Assertion\n\n- depends on: AssertionScope\n\n## AssertionScope\n\n- status: retired\n",
    );
    cargo_toml(code.path());
    write(
        code.path(),
        "src/lib.rs",
        "pub struct Assertion;\npub struct AssertionScope;\n",
    );

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

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

/// D12, the fire direction. Two headings share the name `T`: one
/// `illustrative`, one declared and owning the code item. A permissive
/// per-name key suppresses the edge finding and takes the tree to exit 0
/// with a satisfiable divergence behind it.
///
/// `missing in specs: T` deliberately does **not** co-fire here — the
/// declared heading in `beta` consumes the code node, so the orphan sweep
/// never sees it. The edge finding is the only violation, which is what
/// makes this the cell where suppression alone decides the gate colour.
#[test]
fn a_name_is_pointable_when_any_heading_carrying_it_is() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "## S\n\n- depends on: T\n\n## T\n<!-- polarity:illustrative -->\n\nProse.\n",
    );
    write(specs.path(), "concepts/beta.md", "## T\n\nProse.\n");
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\npub struct T;\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

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

/// D12, the MIRROR — the suppress direction, and **the only test in the
/// slice that can distinguish the collision rule from a no-op.**
///
/// The fire-direction case above reaches the per-name conjunction once, in
/// the one direction where a correct implementation and a no-op agree: its
/// correct answer is also "not unpointable". So without this mirror, an
/// implementation correct on single-heading names that always answers "not
/// unpointable" on a collision passes the entire slice.
///
/// The slip it catches is plausible rather than contrived: "if any heading
/// disagrees, don't suppress" over-simplifies to "if there's more than one
/// heading, don't suppress" in a single edit.
#[test]
fn a_name_is_unpointable_only_when_every_heading_carrying_it_is() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "## S\n\n- depends on: T\n\n## T\n<!-- polarity:illustrative -->\n\nProse.\n",
    );
    write(
        specs.path(),
        "concepts/beta.md",
        "## T\n<!-- polarity:illustrative -->\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

    assert_eq!(
        edge_findings(&outcome),
        0,
        "every heading carrying `T` is unpointable, so the edge is suppressed: {:?}",
        outcome.violations
    );
}

/// The cell that separates the conservative fold from a **permissive** one,
/// which the prescribed D12 pair does not reach.
///
/// Presence is keyed by name, so every heading carrying `T` sees the same
/// `item_present` — which means the two prescribed fixtures can only vary
/// polarity, and in both of them the headings agree. Here they disagree:
/// with the item absent, the `illustrative` heading IS unpointable and the
/// declared one is NOT (matrix row 1 — nothing accounts for that absence,
/// because the absence is itself the finding).
///
/// Conservative (`every` heading): `T` stays pointable, the edge fires, and
/// `missing in code: T` fires beside it. Permissive (`any` heading): the
/// edge is suppressed and a real divergence is parked.
#[test]
fn one_unpointable_heading_does_not_make_a_shared_name_unpointable() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/alpha.md",
        "## S\n\n- depends on: T\n\n## T\n<!-- polarity:illustrative -->\n\nProse.\n",
    );
    write(specs.path(), "concepts/beta.md", "## T\n\nProse.\n");
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct S;\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();

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
