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

fn report_records() -> Vec<serde_json::Value> {
    let root = workspace_root();
    let specs = root.join("specs");
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
    std::str::from_utf8(&out.stdout)
        .expect("utf8")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("valid json line"))
        .collect()
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

#[test]
fn a_report_record_source_keeps_the_frozen_shape_the_contract_documents() {
    let contract = std::fs::read_to_string("../specs/ndjson-output.md")
        .expect("the wire contract is beside the code it contracts");
    let example: serde_json::Value = contract
        .lines()
        .find(|l| l.contains("\"record\":\"verb_coverage\""))
        .map(|l| serde_json::from_str(l).expect("the example is a record"))
        .expect("the contract carries a verb_coverage example");

    let documented: Vec<&str> = example["pub_fn"]["source"]
        .as_object()
        .expect("the example's source is an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        documented,
        ["kind", "line", "path"],
        "report records version independently and stand at \"2\": a field reaching this source \
         through a shared emitter is an undeclared change to a frozen schema, and this is the \
         record that catches it"
    );
    assert_eq!(example["schema_version"], "2");

    let out = report_records();
    let source = out
        .iter()
        .find(|r| r["record"] == "verb_coverage")
        .expect("a verb_coverage record")["pub_fn"]["source"]
        .as_object()
        .expect("source object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        source, documented,
        "the emitted report source and the contract's"
    );
}
