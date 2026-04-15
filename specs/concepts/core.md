# Core concepts

The concepts currently exposed by the public Rust surface of
`graph-specs-rust`. Every top-level `pub` type in the workspace must
appear here; every heading here must correspond to a top-level `pub`
type in the workspace. Prose is encouraged — it is ignored by the
reader.

## Graph

A collection of concept nodes and declared relationship edges extracted
from one side of the equivalence check (specs or code). Two graphs are
equivalent at concept level iff their node sets carry the same names;
equivalent at relationship level iff their edge sets also align after
the v0.3 opt-in rules apply. Lives in `domain`.

- depends on: ConceptNode
- depends on: Edge
- returns: Graph

## ConceptNode

A single named concept located at a specific source site. Carries the
concept's name, a [Source](#source) pointing back to where the reader
found it, and an optional [SignatureState](#signaturestate) payload for
v0.2 signature-level equivalence.

- depends on: Source
- depends on: SignatureState

## SignatureState

The signature-level payload on a [ConceptNode](#conceptnode). `Absent`
means the reader produced no signature (v0.1 concept-only mode).
`Normalized` carries the byte-equal comparison target — the output of
`adapter-rust::normalize` on a `syn::Item`. `Unparseable` surfaces a
spec-side fenced `rust` block that failed to parse, or a section with
more than one fenced `rust` block.

## Source

Where a concept was found — either in a spec file or a code file. Used
for error messages that point back at the file and line the violation
came from.

## Violation

A single equivalence violation between spec and code graphs. Concept-,
signature-, and relationship-level variants share the convention that
the first-carried field is the concept or owner name, so CLI output can
be sorted deterministically regardless of violation kind.

## Edge

A declared relationship between two concepts (v0.3): `implements`,
`depends on`, or `returns`. Each edge owns a tokenised matching target
plus the raw textual form preserved for display in drift messages.

## EdgeKind

The relationship kind of an [Edge](#edge). Closed set for v0.3;
future dialect growth adds variants here.

## Reader

The language-neutral port trait. Concrete readers (markdown specs,
Rust code, later PHP / TypeScript) implement it and produce graphs with
identical shape. Lives in `ports`.

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}
```

## ReaderError

Failure modes of a [Reader](#reader) implementation. Describes
*reading operations* (I/O, parse, walk) rather than domain concerns,
which is why this type lives in the port layer rather than in `domain`.
Adapters map their language-specific failures onto `ReaderError` at the
port boundary.

## MarkdownReader

Concrete [Reader](#reader) implementation for markdown spec files. Uses
`pulldown-cmark`. Emits a [ConceptNode](#conceptnode) for every `##` or
`###` heading it encounters, collects fenced `rust` blocks for
signature-level comparison, and recognises the v0.3 bullet prefixes
(`- implements:`, `- depends on:`, `- returns:`) as declared edges.
Lives in `adapters/markdown`.

- implements: Reader
- depends on: Graph
- depends on: ReaderError

## RustReader

Concrete [Reader](#reader) implementation for Rust source files. Uses
`syn`. Emits a [ConceptNode](#conceptnode) for every top-level
`pub struct`, `pub enum`, `pub trait`, `pub type`, plus v0.2 signature
normalisation via `adapter-rust::normalize` and v0.3 relationship edges
from struct fields, impl blocks, and trait method signatures. Lives in
`adapters/rust`.

- implements: Reader
- depends on: Graph
- depends on: ReaderError
