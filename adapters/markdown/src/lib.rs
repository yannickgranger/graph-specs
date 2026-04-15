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

use domain::{ConceptNode, Graph, Source};
use ports::{Reader, ReaderError};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
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

    let mut buf = String::new();
    let mut current_heading: Option<usize> = None;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2 | HeadingLevel::H3,
                ..
            }) => {
                buf.clear();
                current_heading = Some(line_of_offset(&line_starts, range.start));
            }
            Event::Text(s) if current_heading.is_some() => buf.push_str(&s),
            Event::Code(s) if current_heading.is_some() => buf.push_str(&s),
            Event::End(TagEnd::Heading(HeadingLevel::H2 | HeadingLevel::H3)) => {
                if let Some(line) = current_heading.take() {
                    let name = normalize_heading(&buf);
                    if !name.is_empty() {
                        out.push(ConceptNode {
                            name,
                            source: Source::Spec {
                                path: path.to_path_buf(),
                                line,
                            },
                        });
                    }
                }
            }
            _ => {}
        }
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
}
