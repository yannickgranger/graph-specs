use std::path::Path;

fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .expect("read application/src")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let text = std::fs::read_to_string(&path).expect("read source");
                out.push((path.display().to_string(), text));
            }
        }
    }
    out
}

#[test]
fn application_names_no_adapter_markdown_type_outside_reader_construction() {
    let permitted = ["MarkdownReader"];
    let mut offenders = Vec::new();
    for (path, text) in sources() {
        for (n, line) in text.lines().enumerate() {
            let Some(rest) = line.split("adapter_markdown::").nth(1) else {
                continue;
            };
            let named: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !permitted.contains(&named.as_str()) {
                offenders.push(format!("{path}:{}: adapter_markdown::{named}", n + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "graph-specs-016 §7 S5: application names an adapter type that is not a reader struct it \
         constructs at the composition root — the port layer is what it should reach for:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_guard_fires_on_a_planted_adapter_type() {
    let planted = "use adapter_markdown::SpecTree;\nlet t: adapter_markdown::HeadingNode = x;\n";
    let permitted = ["MarkdownReader"];
    let mut offenders = Vec::new();
    for (n, line) in planted.lines().enumerate() {
        let Some(rest) = line.split("adapter_markdown::").nth(1) else {
            continue;
        };
        let named: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !permitted.contains(&named.as_str()) {
            offenders.push(format!("planted:{}: adapter_markdown::{named}", n + 1));
        }
    }
    assert_eq!(
        offenders,
        vec![
            "planted:1: adapter_markdown::SpecTree".to_string(),
            "planted:2: adapter_markdown::HeadingNode".to_string(),
        ],
        "the detection the guard runs must name a planted adapter type, or the guard proves nothing"
    );
}
