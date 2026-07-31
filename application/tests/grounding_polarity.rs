//! Integration tests for RFC-014 grounding polarity, end to end through the
//! real markdown + Rust readers.
//!
//! The proof lives here rather than against an external grounded tree so it
//! is reproducible in CI: `yg/Bosun` is the corpus that motivated #168, but
//! it is not a dependency of this repo's gate.

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

/// One heading carrying `polarity`, crossed with the code item existing.
fn run_case(polarity: &str, code_present: bool) -> domain::CheckOutcome {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/topology.md",
        &format!("## Member\n<!-- parent:spec:Unit polarity:{polarity} -->\n\nProse.\n"),
    );
    cargo_toml(code.path());
    write(
        code.path(),
        "src/lib.rs",
        if code_present {
            "pub struct Member;\n"
        } else {
            ""
        },
    );
    application::run_check(specs.path(), code.path()).unwrap()
}

/// The §3.3 table, end to end.
#[test]
fn polarity_presence_matrix_end_to_end() {
    // declared — the ordinary obligation, unchanged in both directions.
    let o = run_case("declared", false);
    assert!(
        o.violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Member")),
        "declared + absent must still fire: {:?}",
        o.violations
    );
    assert!(run_case("declared", true).is_clean());

    // forbidden — absence is clean; presence is the finding.
    let o = run_case("forbidden", false);
    assert!(
        o.is_clean(),
        "forbidden + absent is clean — this is the 7-false-findings direction of #168: {:?}",
        o.violations
    );
    let o = run_case("forbidden", true);
    assert!(
        matches!(
            o.violations.as_slice(),
            [Violation::ForbiddenConceptReintroduced { name, .. }] if name == "Member"
        ),
        "expected exactly one forbidden_concept_reintroduced, got: {:?}",
        o.violations
    );
    assert!(
        !o.violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { .. })),
        "and zero missing_in_code"
    );

    // illustrative — absence is clean; presence falls through to the orphan
    // sweep, so the marker cannot launder unspecced public surface.
    assert!(run_case("illustrative", false).is_clean());
    let o = run_case("illustrative", true);
    assert!(
        matches!(
            o.violations.as_slice(),
            [Violation::MissingInSpecs { name, .. }] if name == "Member"
        ),
        "expected exactly one missing_in_specs, got: {:?}",
        o.violations
    );
    assert!(
        !o.violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { .. })),
        "and zero missing_in_code"
    );
}

#[test]
fn the_168_reproduction_both_directions() {
    // The acceptance proof. On `develop` this fixture produced a false
    // `missing in code: Member` with the item absent, and — the defect that
    // matters — went SILENT at exit 0 with the expelled name reintroduced.
    let absent = run_case("forbidden", false);
    assert!(
        absent.is_clean(),
        "over-report direction: {:?}",
        absent.violations
    );

    let present = run_case("forbidden", true);
    assert!(
        !present.is_clean(),
        "under-report direction: reintroducing an expelled name must make the tool LOUDER"
    );
}

#[test]
fn an_unreadable_polarity_leaves_the_obligation_armed() {
    // The fallback direction is the point (§3.2): a typo must not silently
    // narrow an obligation somebody deliberately wrote down.
    let o = run_case("frobidden", false);
    assert!(
        o.violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Member")),
        "a typo'd value falls back to declared: {:?}",
        o.violations
    );
}

#[test]
fn ndjson_carries_the_additive_variant_at_the_unchanged_schema_version() {
    let outcome = run_case("forbidden", true);
    let mut buf = Vec::new();
    application::ndjson::write_ndjson(&outcome, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    let r: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(r["violation"], "forbidden_concept_reintroduced");
    assert_eq!(r["concept"], "Member");
    assert_eq!(
        r["schema_version"], "4",
        "additive variant — no schema bump (§3.5)"
    );
    assert_eq!(r["spec_source"]["kind"], "spec");
    assert_eq!(r["code_source"]["kind"], "code");
}

#[test]
fn a_verb_anchor_under_a_non_declared_heading_imposes_no_obligation() {
    // RFC-014 §3.3 uniform obligation-skip, through the verb pass — which
    // needs a declared bounded context to be armed at all.
    for polarity in ["forbidden", "illustrative"] {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/topology.md",
            &format!("# topology\n\n## Member\n<!-- polarity:{polarity} -->\n\n- verb: reclaim\n"),
        );
        write(
            specs.path(),
            "contexts/topology.md",
            "# topology\n\n## Owns\n\n- fixture\n",
        );
        write(
            code.path(),
            "fixture/Cargo.toml",
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        );
        write(code.path(), "fixture/src/lib.rs", "");

        let outcome = application::run_check(specs.path(), code.path()).unwrap();
        assert!(
            !outcome
                .violations
                .iter()
                .any(|v| matches!(v, Violation::VerbMissingInCode { .. })),
            "{polarity} heading compels nothing, so its verb anchor cannot be missing: {:?}",
            outcome.violations
        );
    }
}

#[test]
fn a_dangling_impl_anchor_under_a_non_declared_heading_fires_nothing() {
    for polarity in ["forbidden", "illustrative"] {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/topology.md",
            &format!("## Member\n<!-- polarity:{polarity} -->\n\n- impl: gone_fn\n"),
        );
        cargo_toml(code.path());
        write(code.path(), "src/lib.rs", "");

        let outcome = application::run_check(specs.path(), code.path()).unwrap();
        assert!(
            outcome.is_clean(),
            "{polarity} heading's anchor cannot dangle: {:?}",
            outcome.violations
        );
    }
}

#[test]
fn the_spec_state_marker_is_inert_under_a_non_declared_heading() {
    // RFC-014 §3.3 precedence, end to end. `realized — ratify` on an
    // expelled name would tell a consumer two opposite things.
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/topology.md",
        "---\nstatus: draft\n---\n\n## Member\n<!-- polarity:forbidden -->\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Member;\n");

    let outcome = application::run_check(specs.path(), code.path()).unwrap();
    assert!(
        outcome.realized.is_empty(),
        "no realized record on an expelled name"
    );
    assert!(outcome.pending.is_empty());
    assert!(
        matches!(
            outcome.violations.as_slice(),
            [Violation::ForbiddenConceptReintroduced { .. }]
        ),
        "the ban still fires: {:?}",
        outcome.violations
    );
}
