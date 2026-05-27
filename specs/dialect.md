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
- Bullets with recognised prefixes (`- implements: X`, `- depends on: X`,
  `- returns: X`, `- verb: <qname>`) — the first three are
  relationship-level anchors; `- verb:` is a function-ownership anchor
  handled by a separate parser path (see [Verb bullets](#verb-bullets)
  below). **Note:** `verb:` is NOT in `BULLET_PREFIXES` code-side; the
  parser dispatches it via a dedicated handler.

### Verb bullets

A `- verb: <qname>` bullet inside a concept section anchors a public
function to that concept's owning bounded context (v0.5).

**Bullet shape:**

```
- verb: <qname>
```

**Qname forms (v0.6):** two forms are accepted, syntactically disjoint:

- **Bare identifier** — `^[A-Za-z_][A-Za-z0-9_]*$`. Matches a top-level
  `pub fn` declaration at the root of a `.rs` file (e.g. `- verb: diff`
  anchors `pub fn diff(...)`). No auto-fallback to impl methods.
- **`Type::method`** — `^[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*$`.
  Matches a public method inside an impl block: either an inherent impl
  (`impl Foo { pub fn bar }`) or a trait impl (`impl Trait for Foo { fn bar }`).
  Trait-impl methods are considered public even without an explicit `pub`
  keyword. The `::` distinguishes this form from a bare identifier —
  no auto-fallback to top-level fns applies.

Multi-segment paths (`a::b::c`), leading `::`, trailing `::`, and
non-identifier characters are rejected with a `tracing::warn!` log
(tolerant-skip).

**Opt-in semantics:** a concept section with no `- verb:` bullets is
never inspected by the verb pass. The pass activates only when at least
one concept in a bounded context carries a verb anchor. Both
concept-level specs (`specs/concepts/`) and context-level specs
(`specs/contexts/`) may carry verb bullets.

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
- **Impl-method extraction (v0.6, verb-anchoring only):** public methods
  in impl blocks are extracted as `Type::method` qnames for verb-anchor
  matching. This walk is separate from concept extraction and only feeds
  the verb-anchoring pass (`VerbReader::extract_pub_fns`). Both inherent
  impls (`impl Foo { pub fn bar }`) and trait impls
  (`impl Trait for Foo { fn bar }`) are covered.

## What the Rust reader ignores

The code-side filter rules are:

- Non-`pub` items
- Items gated by `#[cfg(test)]` or `#[cfg(feature = "…test…")]`
- Declarations nested inside `pub mod foo { … }` (top-level only)
- `impl` blocks (except for verb-anchoring purposes — see ## What the Rust reader parses), `fn`, `const`, `static`, `use`, `macro_rules!`, `mod`
- Per-crate `tests/`, `benches/`, `examples/` directories
- `target/`, `.git/`, `.claude/`, `.proofs/`, `node_modules/` directories
- Any file whose extension is not `.rs`

## Meta note

This dialect spec is itself written in the dialect it describes: `##` and
`###` headings name the subsections, but because this file lives at
`specs/dialect.md` (not under `specs/concepts/`), those headings are not
parsed as concept declarations. The separation is enforced operationally
by the CLI flag, not structurally by the markdown.
