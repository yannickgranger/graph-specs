# reading

Concept-level entries for the **reading** bounded context — the
concrete adapters that parse markdown specs and Rust source into the
equivalence context's graph model. Every type whose code lives under
`adapters/markdown/src/` or `adapters/rust/src/` (the context's
`Owns` block in `specs/contexts/reading.md`) appears here. Prose is
encouraged — it is ignored by the reader.

## MarkdownReader

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.2.3 anchor:"the markdown reader is the only adapter that parses markdown" -->

Concrete [Reader](#reader) and [ContextReader](#contextreader)
implementation for markdown spec files. Uses `pulldown-cmark`. Emits a
[ConceptNode](#conceptnode) for every `##` or `###` heading it encounters,
collects fenced `rust` blocks for signature-level comparison, and
recognises the v0.3 bullet prefixes (`- implements:`, `- depends on:`,
`- returns:`) as declared edges. Also implements
[ContextReader](#contextreader) for v0.4 — parses
`specs/contexts/<name>.md` files into [ContextDecl](#contextdecl) values.
Exposes `extract_invariant_annotations` (inherent method) for graph-specs-005-verb-coverage-report#3.2
— extracts `[enforced-by:]` / `[prose-only:]` annotations from
`#### Operational invariants` spec sections. Lives in `adapters/markdown`.

Per graph-specs-013-spec-state-marker#3.3 it no longer skips `status: draft` files: they are
parsed like any other spec, and every concept heading in one is marked
on the node it emits. The `extract_draft_concepts` side-index walk that
the previous design needed is retired.

- implements: Reader
- implements: ContextReader
- depends on: Graph
- depends on: ReaderError
- depends on: ContextDecl
- depends on: InvariantAnnotation
- depends on: VerbAnchor
- depends on: ConceptAnchor
- depends on: Violation

## RustBackend

<!-- parent:spec:LanguageBackend -->

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

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.2.2 anchor:"the doubled parse pays no cost" -->

Concrete [Reader](#reader) and [VerbReader](#verbreader) implementation
for Rust source files. Thin adapter over [RustBackend](#rustbackend):
pulls the [Extraction](#extraction), filters raw edges against the
discovered [ConceptNode](#conceptnode) set, and assembles a
[Graph](#graph) for the diff engine. Emits one [ConceptNode](#conceptnode)
per top-level `pub struct`, `pub enum`, `pub trait`, `pub type`, plus v0.2
signature normalisation via `adapter-rust::normalize` and v0.3 relationship
edges from struct fields, impl blocks, and trait method signatures.
`VerbReader::extract_pub_fns` uses a separate parallel walk (per graph-specs-005-verb-coverage-report#3.2
dry-run rust-systems-A); `check` invokes it to feed the verb-
anchoring pass with code-side `pub fn` declarations. Also implements
[CodeFacts](equivalence.md#codefacts) (graph-specs-010-abstraction-level-equivalence R10-6), returning the
graph's [ConceptNode](#conceptnode)s as the source-walk parity reference
the cfdb-query [CfdbQueryReader](#cfdbqueryreader) ACL must match. Lives in
`adapters/rust`.

- implements: Reader
- implements: VerbReader
- implements: CodeFacts
- depends on: Graph
- depends on: ConceptNode
- depends on: Edge
- depends on: EdgeKind
- depends on: ReaderError
- depends on: PubFnDecl

## RustAnchorResolver

<!-- parent:spec:AnchorResolver -->

Source-walk [AnchorResolver](equivalence.md#anchorresolver) implementation
(graph-specs-012-non-pub-spec-anchor#3.4 / R12-3). Builds an index of code items at **any** visibility
(the concept walk is `pub`-only) so a `- impl: <qname>` spec anchor can
resolve a concept whose canonical implementation is legitimately
`pub(crate)` (or a `fn` / `const`) — no manufactured `pub` ZST. The index
is built once from the code root; `resolve` is consulted only for the
anchor qnames, so the global concept set the [RustReader](#rustreader)
produces is unchanged. A dedicated struct (not an `impl` on
[RustReader](#rustreader)) because the port's `resolve(&self, qname)`
carries no root — the resolver must hold the pre-built index. Resolves
top-level types, `fn`s, `const`s, and `Type::method` impl methods; enum
variants are deferred to the cfdb-query path. Returns an
[AnchorTarget](equivalence.md#anchortarget). Lives in `adapters/rust`.

- implements: AnchorResolver
- depends on: AnchorTarget
- depends on: ReaderError
- depends on: RustAnchorResolver

## CfdbQueryReader

<!-- parent:spec:CodeFacts -->

The cfdb-query [CodeFacts](equivalence.md#codefacts) Anti-Corruption Layer
(graph-specs-010-abstraction-level-equivalence#3.3 / R10-6). Reads a cfdb keyspace JSON and translates `:Item`
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
- depends on: DeclaredSurface
- depends on: Edge
- depends on: EdgeKind
- depends on: PubFnDecl
- implements: VerbReader
- returns: CfdbQueryReader

### PhpEdgeTraversal
<!-- parent:spec:CfdbQueryReader -->

The PHP fact-extraction path of the [CfdbQueryReader](#cfdbqueryreader)
ACL (graph-specs-010-abstraction-level-equivalence#11.5: PHP `:Item` is
prop-less, so containment is read by traversing `IN_MODULE` / `IN_CRATE`
edges, never by prop reads). It yields a [ConceptNode](equivalence.md#conceptnode)
for every concept-rung PHP `:Item` — `class_declaration`,
`interface_declaration` and `enum_declaration` alike, told apart by the
`php_construct` property, the producer emitting `kind: "trait"` for a
class or an interface and `kind: "enum"` for an enum, a kind already
inside `cfdb-041-language-backend-trait#4`'s closed set, so no kind is
widened
(graph-specs-011-php-ladder#3.1; cfdb-045-polyglot-relationship-edges#3.2) —
and for nothing below that rung: a method `:Item` binds no heading and is
reached only through an anchor. The `unit` of the agnostic triple is the
namespace prefix of the qualified name (`\Ns\Class`), matched against the
[DeclaredSurface](equivalence.md#declaredsurface) before the node is
emitted; an item outside every declared prefix is not on the surface and
is not emitted. Lives in `adapters/cfdb-query`.

- depends on: ConceptNode
- depends on: DeclaredSurface
- depends on: ReaderError
- depends on: Edge
- returns: PhpEdgeTraversal

## CfdbAnchorResolver

<!-- parent:spec:AnchorResolver -->

cfdb-keyspace [AnchorResolver](equivalence.md#anchorresolver) (graph-specs-012-non-pub-spec-anchor#3.4
/ R12-6 — the OQ-1 parity path). The keyspace counterpart to the source-walk
[RustAnchorResolver](#rustanchorresolver): resolves a `- impl:` anchor
against the `:Item` facts a `cfdb extract` run already holds, for per-crate
repos whose keyspace is the code-fact source. Lifts the concept ACL's
`pub`-only filter — a `pub(crate)` item (cfdb reports it as `visibility:
"private"`) resolves at any visibility. Resolves type / fn / const kinds and
`Type::method` (reduced from cfdb's `crate::Type::method` qname); enum
variants are **not** resolvable because cfdb (v0.5.0) emits no `variant`
`:Item` kind — that remains deferred to a paired cfdb change. Returns an
[AnchorTarget](equivalence.md#anchortarget). Lives in `adapters/cfdb-query`.

- implements: AnchorResolver
- depends on: AnchorTarget
- depends on: ReaderError
- depends on: CfdbAnchorResolver

## PhpAttributeReader

- status: draft (per graph-specs-011-php-ladder#3.3)
<!-- parent:rfc:graph-specs-011-php-ladder#3.3 anchor:"it reads attributes" -->

The one reader that reads PHP source, and it reads attributes only: the
`#[Spec(...)]` attribute channel of
graph-specs-004-multi-language-adapter-contract#3.1, emitted as spec-side
facts whose [Source](equivalence.md#source) carries the inline-attribute
[SpecFormat](equivalence.md#specformat). It emits no code-side fact — the
PHP code side is the [CodeFacts](equivalence.md#codefacts) port reached
through [PhpEdgeTraversal](#phpedgetraversal), and graph-specs owns no PHP
structural parser (graph-specs-011-php-ladder#4 invariant 1). Markdown is
the canonical upstream of every attribute it reads; where the two
disagree the attribute is the conformist
(graph-specs-004-multi-language-adapter-contract#4 invariant 7). A `.php`
file routes to this reader and to no other (graph-specs-011-php-ladder#3.4).
Its parser backend is tree-sitter with the `tree-sitter-php` grammar the
cfdb PHP producer already pins, so one PHP syntax model runs in the
ecosystem; it is a leaf adapter and takes no dependency on any other
adapter (graph-specs-016-parse-once-reading-port#1). Lives in
`adapters/php`.

- implements: Reader
- depends on: Graph
- depends on: Source
- depends on: SpecFormat
- depends on: ReaderError
- returns: PhpAttributeReader

## SpecTree

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.2 anchor:"H1/parent-tree assembly" -->

The assembled heading tree for a single spec file (graph-specs-010-abstraction-level-equivalence#3.2 / R10-2) —
a parent-linked vector of [HeadingNode](#headingnode) in document order,
produced by the `assemble_tree` pass over the one dialect read the concept
reader also projects, so the tree's rungs and the graph's nodes are the
same list and cannot diverge (keel-dialect §12.1). Exposes `context_id`
(the file's single bounded-context identifier) and
`cohesion_violations`, which surfaces the spec-side
[CohesionViolation](#cohesionviolation)s the tree's shape reveals — an H1
context with no concept under it, and orphaned H3 sub-concepts. Wiring the
detection into the `check` diff is R10-3. Lives in `adapters/markdown`.

Marker-blind by construction (graph-specs-013-spec-state-marker#3.2 row 6): the assembler records
heading *depth*, so a marked `## Concept` is a `Concept` node like any
other and counts as its context's cohesion unit. Since graph-specs-013-spec-state-marker the walk
also no longer skips `status: draft` files — the doc-level structural
check applies to them on the same terms as any other doc.

### HeadingNode

<!-- parent:spec:SpecTree -->

One node of the abstraction-ladder tree (graph-specs-010-abstraction-level-equivalence#3.2 / R10-2) — a single
markdown heading, tagged with the [AbstractionLevel](#abstractionlevel) its
depth maps to (`H1 → Context`, `H2 → Concept`, `H3 → SubConcept`,
`H4+ → Member`), its trimmed text, the normalised context identifier for an
H1 node (`# AC verifier` → `ac-verifier`, `None` deeper), its 1-based line,
and the index of its parent one rung up (`None` for a context, or for an
orphaned sub-concept). Lives in `adapters/markdown`.
