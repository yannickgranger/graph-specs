use std::process::Command;

const ABSENT: &str = "unknown";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Some(head) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={head}/HEAD");
    }
    let rev = git(&["rev-parse", "HEAD"])
        .filter(|rev| rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit()));
    println!(
        "cargo:rustc-env=GRAPH_SPECS_BUILD_REV={}",
        rev.as_deref().unwrap_or(ABSENT)
    );
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}
