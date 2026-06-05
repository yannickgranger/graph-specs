# reading

Concept-level entries for the **reading** bounded context — the
concrete adapters that parse markdown specs and Rust source into the
equivalence context's graph model. Every type whose code lives under
`adapters/markdown/src/` or `adapters/rust/src/` (the context's
`Owns` block in `specs/contexts/reading.md`) appears here. Prose is
encouraged — it is ignored by the reader.

## MarkdownReader

Concrete [Reader](#reader) and [ContextReader](#contextreader)
implementation for markdown spec files. Uses `pulldown-cmark`. Emits a
[ConceptNode](#conceptnode) for every `##` or `###` heading it encounters,
collects fenced `rust` blocks for signature-level comparison, and
recognises the v0.3 bullet prefixes (`- implements:`, `- depends on:`,
`- returns:`) as declared edges. Also implements
[ContextReader](#contextreader) for v0.4 — parses
`specs/contexts/<name>.md` files into [ContextDecl](#contextdecl) values.
Exposes `extract_invariant_annotations` (inherent method) for RFC-005
§3.2 — extracts `[enforced-by:]` / `[prose-only:]` annotations from
`#### Operational invariants` spec sections. Lives in `adapters/markdown`.

- implements: Reader
- implements: ContextReader
- depends on: Graph
- depends on: ConceptNode
- depends on: ReaderError
- depends on: ContextDecl
- depends on: InvariantAnnotation
- depends on: VerbAnchor

## RustBackend

Concrete [LanguageBackend](#languagebackend) implementation for Rust
source files. Uses `syn`. Walks the source tree once (skipping `target/`,
`.git/`, `.claude/`, `.proofs/`, per-crate `tests/` / `benches/` /
`examples/` and `node_modules/`), parses each `*.rs` file, and emits
flat [ConceptNode](#conceptnode) + raw [Edge](#edge) into an
[Extraction](#extraction). Detects via `Cargo.toml` at the root.
[RustReader](#rustreader) wraps it for the [Reader](#reader) port.
Lives in `adapters/rust`.

- implements: LanguageBackend
- depends on: Extraction
- depends on: ReaderError

## RustReader

Concrete [Reader](#reader) and [VerbReader](#verbreader) implementation
for Rust source files. Thin adapter over [RustBackend](#rustbackend):
pulls the [Extraction](#extraction), filters raw edges against the
discovered [ConceptNode](#conceptnode) set, and assembles a
[Graph](#graph) for the diff engine. Emits one [ConceptNode](#conceptnode)
per top-level `pub struct`, `pub enum`, `pub trait`, `pub type`, plus v0.2
signature normalisation via `adapter-rust::normalize` and v0.3 relationship
edges from struct fields, impl blocks, and trait method signatures.
`VerbReader::extract_pub_fns` uses a separate parallel walk (per RFC-005
§3.2 dry-run rust-systems-A); `check` invokes it to feed the verb-
anchoring pass with code-side `pub fn` declarations. Lives in
`adapters/rust`.

- implements: Reader
- implements: VerbReader
- depends on: Graph
- depends on: ReaderError
- depends on: PubFnDecl
