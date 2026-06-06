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
- depends on: ConceptAnchor

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
anchoring pass with code-side `pub fn` declarations. Also implements
[CodeFacts](equivalence.md#codefacts) (RFC-010 R10-6), returning the
graph's [ConceptNode](#conceptnode)s as the source-walk parity reference
the cfdb-query [CfdbQueryReader](#cfdbqueryreader) ACL must match. Lives in
`adapters/rust`.

- implements: Reader
- implements: VerbReader
- implements: CodeFacts
- depends on: Graph
- depends on: ConceptNode
- depends on: ReaderError
- depends on: PubFnDecl

## CfdbQueryReader

The cfdb-query [CodeFacts](equivalence.md#codefacts) Anti-Corruption Layer
(RFC-010 §3.3 / R10-6). Reads a cfdb keyspace JSON and translates `:Item`
nodes into agnostic [ConceptNode](#conceptnode)s — `unit` / `module_path`
reconstructed from the `:Item.file` prop to match the source-walk
[RustReader](#rustreader)'s derivation (the parity contract), `context`
from cfdb's per-crate `bounded_context`. It is an ACL, not a Conformist:
cfdb's Rust-specific props are translated, never adopted verbatim, so a
prop-less PHP `:Item` yields empty provenance rather than a crash. Depends
only on `cfdb-core` (plus serde) for the keyspace wire shape; compiled into
the application solely behind the `codefacts` feature (the opt-in leaf).
Lives in `adapters/cfdb-query`.

- implements: CodeFacts
- depends on: ConceptNode
- depends on: ReaderError
- returns: CfdbQueryReader

## HeadingNode

One node of the abstraction-ladder tree (RFC-010 §3.2 / R10-2) — a single
markdown heading, tagged with the [AbstractionLevel](#abstractionlevel) its
depth maps to (`H1 → Context`, `H2 → Concept`, `H3 → SubConcept`,
`H4+ → Member`), its trimmed text, the normalised context identifier for an
H1 node (`# AC verifier` → `ac-verifier`, `None` deeper), its 1-based line,
and the index of its parent one rung up (`None` for a context, or for an
orphaned sub-concept). Lives in `adapters/markdown`.

## SpecTree

The assembled heading tree for a single spec file (RFC-010 §3.2 / R10-2) —
a parent-linked vector of [HeadingNode](#headingnode) in document order,
produced by the `assemble_tree` pass (a separate `pulldown-cmark` walk from
the concept reader's `handle_event`, which is at the complexity ceiling).
Exposes `context_id` (the file's single bounded-context identifier) and
`cohesion_violations`, which surfaces the spec-side
[CohesionViolation](#cohesionviolation)s the tree's shape reveals — an H1
context with no concept under it, and orphaned H3 sub-concepts. Wiring the
detection into the `check` diff is R10-3. Lives in `adapters/markdown`.
