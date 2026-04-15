//! Markdown spec reader — concept-level.
//!
//! Walks a directory tree, parses each `*.md` file with `pulldown-cmark`,
//! and emits a [`ConceptNode`] for every `##` or `###` heading. Per the
//! dialect spec (`specs/dialect.md`), prose, tables, images, links,
//! bullets, and fenced blocks are all ignored — only `h2`/`h3` heading
//! text participates in the concept graph.
//!
//! Headings containing generic parameters are normalised: `## Graph<T>`
//! records the concept as `Graph`.

use domain::{ConceptNode, Graph, SignatureState, Source};
use ports::{Reader, ReaderError};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct MarkdownReader;

impl Reader for MarkdownReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();

        for entry in WalkDir::new(root) {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "md") {
                continue;
            }

            let path = entry.path();
            let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;

            extract_from_source(&source, path, &mut nodes);
        }

        Ok(Graph { nodes })
    }
}

fn extract_from_source(source: &str, path: &Path, out: &mut Vec<ConceptNode>) {
    let line_starts = compute_line_starts(source);
    let parser = Parser::new(source).into_offset_iter();

    let mut heading_buf = String::new();
    let mut in_heading_at: Option<usize> = None;
    // After a heading closes, `pending` holds the concept name+line until
    // the next heading (or EOF) — at which point we flush it with whatever
    // rust blocks were collected for that section.
    let mut pending: Option<(String, usize)> = None;
    let mut rust_blocks: Vec<String> = Vec::new();
    let mut in_rust_block = false;
    let mut block_buf = String::new();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2 | HeadingLevel::H3,
                ..
            }) => {
                // New section boundary — flush the previous section's concept.
                flush_pending(&mut pending, &rust_blocks, path, out);
                rust_blocks.clear();
                heading_buf.clear();
                in_heading_at = Some(line_of_offset(&line_starts, range.start));
            }
            Event::Text(s) if in_heading_at.is_some() => heading_buf.push_str(&s),
            Event::Code(s) if in_heading_at.is_some() => heading_buf.push_str(&s),
            Event::End(TagEnd::Heading(HeadingLevel::H2 | HeadingLevel::H3)) => {
                if let Some(line) = in_heading_at.take() {
                    let name = normalize_heading(&heading_buf);
                    if !name.is_empty() {
                        pending = Some((name, line));
                    }
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if pending.is_some() && lang.as_ref() == "rust" =>
            {
                in_rust_block = true;
                block_buf.clear();
            }
            Event::Text(s) if in_rust_block => block_buf.push_str(&s),
            Event::End(TagEnd::CodeBlock) if in_rust_block => {
                rust_blocks.push(std::mem::take(&mut block_buf));
                in_rust_block = false;
            }
            _ => {}
        }
    }

    flush_pending(&mut pending, &rust_blocks, path, out);
}

fn flush_pending(
    pending: &mut Option<(String, usize)>,
    rust_blocks: &[String],
    path: &Path,
    out: &mut Vec<ConceptNode>,
) {
    if let Some((name, line)) = pending.take() {
        out.push(ConceptNode {
            name,
            source: Source::Spec {
                path: path.to_path_buf(),
                line,
            },
            signature: signature_from_blocks(rust_blocks),
        });
    }
}

fn signature_from_blocks(blocks: &[String]) -> SignatureState {
    match blocks {
        [] => SignatureState::Absent,
        [only] => parse_single_block(only),
        many => {
            let count = many.len();
            SignatureState::Unparseable {
                raw: many.join("\n---\n"),
                error: format!(
                    "concept section contains {count} fenced rust blocks; at most one is allowed"
                ),
            }
        }
    }
}

fn parse_single_block(raw: &str) -> SignatureState {
    match syn::parse_str::<syn::Item>(raw) {
        Ok(item) => SignatureState::Normalized(adapter_rust::normalize(&item)),
        Err(e) => SignatureState::Unparseable {
            raw: raw.to_string(),
            error: e.to_string(),
        },
    }
}

/// Normalise a heading's collected text into a concept name.
/// Strips generics (`Foo<T>` → `Foo`) and trims whitespace.
fn normalize_heading(raw: &str) -> String {
    let trimmed = raw.trim();
    trimmed
        .find('<')
        .map_or_else(|| trimmed.to_string(), |i| trimmed[..i].trim().to_string())
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn line_of_offset(starts: &[usize], offset: usize) -> usize {
    match starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let full = dir.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&full).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn extract(dir: &Path) -> Vec<String> {
        let g = MarkdownReader.extract(dir).unwrap();
        let mut names: Vec<String> = g.nodes.into_iter().map(|n| n.name).collect();
        names.sort();
        names
    }

    #[test]
    fn captures_h2_and_h3_headings() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n### Bar\n");
        assert_eq!(extract(d.path()), vec!["Bar", "Foo"]);
    }

    #[test]
    fn ignores_h1_and_deeper_than_h3() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "# Title\n## Keep\n#### Skip\n##### Deep\n",
        );
        assert_eq!(extract(d.path()), vec!["Keep"]);
    }

    #[test]
    fn ignores_prose_and_lists() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Concept\n\nSome prose about Concept.\n\n- a bullet mentioning Other\n- another\n\nMore prose.\n",
        );
        assert_eq!(extract(d.path()), vec!["Concept"]);
    }

    #[test]
    fn ignores_fenced_code_blocks() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Hidden;\n```\n\n```\npub struct AlsoHidden;\n```\n",
        );
        assert_eq!(extract(d.path()), vec!["Foo"]);
    }

    #[test]
    fn strips_generics_from_heading() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Graph<T>\n");
        assert_eq!(extract(d.path()), vec!["Graph"]);
    }

    #[test]
    fn handles_inline_backticks_in_heading() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## `Reader`\n");
        assert_eq!(extract(d.path()), vec!["Reader"]);
    }

    #[test]
    fn records_correct_line_number() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "# Title\n\n## OnLine3\n");
        let g = MarkdownReader.extract(d.path()).unwrap();
        match &g.nodes[0].source {
            Source::Spec { line, .. } => assert_eq!(*line, 3),
            Source::Code { .. } => panic!("expected Spec source"),
        }
    }

    #[test]
    fn non_md_files_are_ignored() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## FromMd\n");
        write(d.path(), "a.txt", "## FromTxt\n");
        assert_eq!(extract(d.path()), vec!["FromMd"]);
    }

    // --- v0.2 signature-level tests ---

    fn extract_sig(dir: &Path, concept: &str) -> SignatureState {
        let g = MarkdownReader.extract(dir).unwrap();
        g.nodes
            .into_iter()
            .find(|n| n.name == concept)
            .unwrap_or_else(|| panic!("concept {concept} not found"))
            .signature
    }

    #[test]
    fn concept_without_rust_block_has_absent_signature() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n\nJust prose.\n");
        assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
    }

    #[test]
    fn concept_with_rust_block_has_normalized_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\nProse.\n\n```rust\npub struct Foo(pub u32);\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Normalized(s) => {
                assert!(s.contains("pub struct Foo"));
                assert!(s.contains("u32"));
            }
            other => panic!("expected Normalized, got {other:?}"),
        }
    }

    #[test]
    fn non_rust_fenced_blocks_do_not_populate_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```python\npub struct Foo;\n```\n",
        );
        assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
    }

    #[test]
    fn unparseable_rust_block_becomes_unparseable_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo(\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Unparseable { raw, error } => {
                assert!(raw.contains("pub struct Foo("));
                assert!(!error.is_empty());
            }
            other => panic!("expected Unparseable, got {other:?}"),
        }
    }

    #[test]
    fn multiple_rust_blocks_in_one_concept_is_unparseable() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo;\n```\n\n```rust\npub struct Bar;\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Unparseable { error, .. } => {
                assert!(error.to_lowercase().contains("multiple") || error.contains('2'));
            }
            other => panic!("expected Unparseable (multiple rust blocks), got {other:?}"),
        }
    }

    #[test]
    fn rust_block_scoped_to_current_concept_only() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo;\n```\n\n## Bar\n\nNo block here.\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Normalized(_) => {}
            other => panic!("Foo should have Normalized sig, got {other:?}"),
        }
        assert_eq!(extract_sig(d.path(), "Bar"), SignatureState::Absent);
    }

    #[test]
    fn rust_block_before_first_heading_is_ignored() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "```rust\npub struct Orphan;\n```\n\n## Foo\n",
        );
        // Orphan must not be extracted as a concept; Foo exists with no signature.
        let g = MarkdownReader.extract(d.path()).unwrap();
        let names: Vec<&str> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["Foo"]);
        assert_eq!(g.nodes[0].signature, SignatureState::Absent);
    }
}
