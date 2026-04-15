# Spec dialect

This document describes the markdown dialect `graph-specs-rust` parses.
It is a meta-spec: it describes the format of spec files, not any concept
of the tool itself. The tool's CLI `--specs` flag is pointed at
`specs/concepts/` (not `specs/`), so this file's own headings are never
walked as concepts.

## Registry boundary

`specs/` is the **spec registry** — the directory tree that holds all
authoritative specifications for this project. `docs/` holds non-spec
content (roadmap, operational notes, rationale) and is never walked by
the tool. Moving a document between `specs/` and `docs/` is a meaningful
action: it brings the document under (or removes it from) the gate.

Within `specs/`, the concept-declaration subdirectory is `specs/concepts/`.
The tool's `--specs` flag should point at that subdirectory when running
the dogfood check. Other files under `specs/` (like this one) are
meta-specs that describe the system; they are not walked by the CLI and
their headings do not become concept nodes.

## What the markdown reader parses

Only **structural** elements contribute to the concept graph.

- Level-2 and level-3 headings (`##` and `###`) — the heading text
  becomes a concept node. Heading text is normalised: inline backticks
  are stripped (CommonMark's plain-text rendering), leading/trailing
  whitespace is trimmed, and generic parameters are removed
  (`## Graph<T>` records the concept as `Graph`).
- Fenced `rust` code blocks inside a concept's section — reserved for
  signature-level extraction in a later issue. Currently parsed but not
  diffed.
- Bullets with recognised prefixes (`- implements: X`, `- depends on: X`)
  — reserved for relationship-level extraction in a later issue.
  Currently parsed but not diffed.

## What the markdown reader ignores

Prose changes never affect the graph. The reader does not see:

- Paragraphs, blockquotes, emphasis, strong, strikethrough
- Level-1 and level-4+ headings
- Fenced blocks without a recognised language tag (untagged or `txt` or
  similar)
- Bullets without a recognised prefix
- Ordered lists
- Tables, images, links, raw HTML blocks, HTML comments
- Files outside the directory passed to `--specs`
- Any file whose extension is not `.md`

## What the Rust reader parses

Only **top-level public declarations** contribute to the concept graph.

- `pub struct`, `pub enum`, `pub trait`, `pub type` at the root of each
  `*.rs` file. The identifier is the concept name. The file path and
  start line of the identifier are the source location.

## What the Rust reader ignores

The code-side filter rules are:

- Non-`pub` items
- Items gated by `#[cfg(test)]` or `#[cfg(feature = "…test…")]`
- Declarations nested inside `pub mod foo { … }` (top-level only)
- `impl` blocks, `fn`, `const`, `static`, `use`, `macro_rules!`, `mod`
- Per-crate `tests/`, `benches/`, `examples/` directories
- `target/`, `.git/`, `.claude/`, `.proofs/`, `node_modules/` directories
- Any file whose extension is not `.rs`

## Meta note

This dialect spec is itself written in the dialect it describes: `##` and
`###` headings name the subsections, but because this file lives at
`specs/dialect.md` (not under `specs/concepts/`), those headings are not
parsed as concept declarations. The separation is enforced operationally
by the CLI flag, not structurally by the markdown.
