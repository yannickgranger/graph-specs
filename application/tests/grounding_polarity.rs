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
    application::run_check(specs.path(), code.path(), None).unwrap()
}

#[test]
fn polarity_presence_matrix_end_to_end() {
    let o = run_case("declared", false);
    assert!(
        o.violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Member")),
        "declared + absent must still fire: {:?}",
        o.violations
    );
    assert!(run_case("declared", true).is_clean());

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
fn an_unknown_polarity_value_is_malformed_and_never_a_silent_default() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/topology.md",
        "## Member\n<!-- parent:spec:Unit polarity:frobidden -->\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "");

    let err = application::run_check(specs.path(), code.path(), None)
        .expect_err("a typo'd value must refuse, never fall back to declared");
    assert!(
        matches!(&err, ports::ReaderError::ParseFailed { message, .. } if message.contains("unknown polarity")),
        "got {err:?}"
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
    for polarity in ["forbidden", "illustrative"] {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(
            specs.path(),
            "concepts/topology.md",
            &format!("# topology\n\n## Member\n<!-- parent:spec:Unit polarity:{polarity} -->\n\n- verb: reclaim\n"),
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

        let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
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
            &format!(
                "## Member\n<!-- parent:spec:Unit polarity:{polarity} -->\n\n- impl: gone_fn\n"
            ),
        );
        cargo_toml(code.path());
        write(code.path(), "src/lib.rs", "");

        let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
        assert!(
            outcome.is_clean(),
            "{polarity} heading's anchor cannot dangle: {:?}",
            outcome.violations
        );
    }
}

#[test]
fn the_spec_state_marker_is_inert_under_a_non_declared_heading() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/topology.md",
        "---\nstatus: draft\n---\n\n## Member\n<!-- parent:spec:Unit polarity:forbidden -->\n\nProse.\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct Member;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
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
