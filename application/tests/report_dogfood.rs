use assert_cmd::Command;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("workspace root").to_path_buf()
}

fn bin() -> Command {
    Command::cargo_bin("graph-specs").expect("graph-specs binary built")
}

#[test]
fn dogfood_report_exits_zero() {
    let root = workspace_root();
    let specs = root.join("specs");
    if !specs.exists() {
        return;
    }

    bin()
        .args([
            "report",
            "--verb-coverage",
            "--specs",
            specs.to_str().unwrap(),
            "--code",
            root.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn dogfood_report_ndjson_contains_run_check() {
    let root = workspace_root();
    let specs = root.join("specs");
    if !specs.exists() {
        return;
    }

    let out = bin()
        .args([
            "report",
            "--verb-coverage",
            "--specs",
            specs.to_str().unwrap(),
            "--code",
            root.to_str().unwrap(),
            "--format",
            "ndjson",
        ])
        .output()
        .expect("run");

    assert_eq!(out.status.code(), Some(0), "report must exit 0");

    let stdout = std::str::from_utf8(&out.stdout).expect("utf8");
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("valid json line"))
        .collect();

    assert!(
        records
            .iter()
            .any(|r| r["record"] == "verb_coverage" && r["pub_fn"]["name"] == "run_check"),
        "expected verb_coverage record for run_check; first few records: {:?}",
        records.iter().take(5).collect::<Vec<_>>()
    );
}
