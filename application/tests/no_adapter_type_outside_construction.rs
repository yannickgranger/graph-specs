use std::path::Path;

const PERMITTED: [&str; 1] = ["MarkdownReader"];
const CRATE: &str = "adapter_markdown";

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

fn offenders_in(sources: &[(String, String)]) -> Vec<String> {
    let mut offenders = Vec::new();
    for (path, text) in sources {
        for (n, line) in text.lines().enumerate() {
            for reference in crate_references(line) {
                if !matches!(&reference, Reference::Item(name) if PERMITTED.contains(&name.as_str()))
                {
                    offenders.push(format!("{path}:{}: {}", n + 1, reference.render()));
                }
            }
        }
    }
    offenders
}

enum Reference {
    Item(String),
    Alias(String),
    Bare,
}

impl Reference {
    fn render(&self) -> String {
        match self {
            Self::Item(name) => format!("{CRATE}::{name}"),
            Self::Alias(alias) => format!("{CRATE} as {alias}"),
            Self::Bare => CRATE.to_string(),
        }
    }
}

fn crate_references(line: &str) -> Vec<Reference> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(CRATE) {
        let before = rest[..at].chars().next_back();
        let after = &rest[at + CRATE.len()..];
        rest = after;
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        if let Some(tail) = after.strip_prefix("::") {
            out.push(Reference::Item(identifier(tail)));
        } else if let Some(tail) = after.strip_prefix(" as ") {
            out.push(Reference::Alias(identifier(tail)));
        } else {
            out.push(Reference::Bare);
        }
    }
    out
}

fn identifier(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

#[test]
fn application_names_no_adapter_markdown_type_outside_reader_construction() {
    let sources = sources();
    assert!(
        !sources.is_empty(),
        "the guard scanned no source at all — a walk that reads nothing cannot refuse anything"
    );
    let offenders = offenders_in(&sources);
    assert!(
        offenders.is_empty(),
        "graph-specs-016 §7 S5: application names an adapter type that is not a reader struct it \
         constructs at the composition root — the port layer is what it should reach for:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_guard_fires_on_a_planted_adapter_type() {
    let resolves: fn(&ports::SpecFileSet) -> Vec<&ports::LoadedFile> =
        adapter_markdown::concept_files;
    assert!(
        resolves(&ports::SpecFileSet::new(Vec::new())).is_empty(),
        "the planted name must be an item the adapter really publishes, or the guard is planted \
         against a type that could never appear in a source it walks"
    );

    let planted = vec![(
        "planted".to_string(),
        "use adapter_markdown::concept_files;\n\
         use adapter_markdown as md;\n\
         let r = adapter_markdown::MarkdownReader::new(&[]);\n"
            .to_string(),
    )];

    assert_eq!(
        offenders_in(&planted),
        vec![
            "planted:1: adapter_markdown::concept_files".to_string(),
            "planted:2: adapter_markdown as md".to_string(),
        ],
        "the guard must name a planted adapter item and a planted alias, and must not name the \
         reader it permits — otherwise it proves nothing about the sources it walks"
    );
}
