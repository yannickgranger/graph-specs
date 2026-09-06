# reading

Concept-level entries for the **reading** bounded context — the
concrete adapters that parse markdown specs and Rust source into the
equivalence context's graph model. Every type whose code lives under
`adapters/markdown/src/` or `adapters/rust/src/` (the context's
`Owns` block in `specs/contexts/reading.md`) appears here. Prose is
encouraged — it is ignored by the reader.

## MarkdownReader

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.2.3 anchor:"the markdown reader is the only adapter that parses markdown" -->
Concrete [SpecLoader](equivalence.md#specloader), [SpecReader](equivalence.md#specreader)
and [ContextReader](equivalence.md#contextreader) implementation for markdown
spec files, and the implementor of the four sibling capabilities —
[VerbAnchorReader](equivalence.md#verbanchorreader),
[ConceptAnchorReader](equivalence.md#conceptanchorreader),
[AnnotationReader](equivalence.md#annotationreader),
[SpecTreeReader](equivalence.md#spectreereader). Uses `pulldown-cmark`.

It loads once and extracts many times (graph-specs-016-parse-once-reading-port#3.6):
`load` is the one walk over the spec tree — traversal, the `.md` filter, the
read — answering a [SpecFileSet](equivalence.md#specfileset) sorted by
path; every capability reads the set it is handed and performs no I/O, so
the only failure a capability can produce is a parse failure, the walk's
failures being the loader's alone (graph-specs-016-parse-once-reading-port#3.3.1).
The `concepts/` and `contexts/` partition is an in-memory predicate over
the loaded set, the one owner of that rule. From the set it emits a
[ConceptNode](equivalence.md#conceptnode) for every `##` or `###` heading,
collects fenced signature blocks and normalizes them through the
[SignatureNormalizer](equivalence.md#signaturenormalizer) port supplied per
fence tag, recognises the bullet prefixes (`- implements:`, `- depends on:`,
`- returns:`) as declared edges, parses `specs/contexts/<name>.md` into
[ContextDecl](equivalence.md#contextdecl) values, reads the verb anchors,
the concept anchors and the `[enforced-by:]` / `[prose-only:]` annotations,
and assembles one [SpecTree](equivalence.md#spectree) per file. Lives in
`adapters/markdown`.

Per graph-specs-013-spec-state-marker#3.3 it does not skip `status: draft`
files: they are read like any other spec, and every concept heading in one
is marked on the node it emits.

- implements: SpecReader
- implements: SpecLoader
- implements: ContextReader
- implements: VerbAnchorReader
- implements: ConceptAnchorReader
- implements: AnnotationReader
- implements: SpecTreeReader
- depends on: SignatureNormalizer
- depends on: Graph
- depends on: ReaderError
- depends on: ContextDecl
- depends on: InvariantAnnotation
- depends on: VerbAnchor
- depends on: ConceptAnchor
- depends on: Violation
- depends on: SpecFileSet
- depends on: SpecTree

### RustBackend

<!-- parent:spec:LanguageBackend -->
Concrete [LanguageBackend](equivalence.md#languagebackend) implementation for
Rust source, cache-holding since graph-specs-016-parse-once-reading-port#3.4:
constructed with the [ParseCache](#parsecache) of the run, it reads every
parsed file from it and emits flat [ConceptNode](equivalence.md#conceptnode)
plus raw [Edge](equivalence.md#edge) values into an
[Extraction](equivalence.md#extraction), BEFORE the language-neutral
known-concept edge filter runs. The walk it once owned — skipping
`target/`, `.git/`, `.claude/`, `.proofs/`, per-crate `tests/` /
`benches/` / `examples/` and `node_modules/` — is [RustLoader](#rustloader)'s.
Detects via `Cargo.toml` at the root. [RustReader](#rustreader) wraps it for
the [CodeReader](equivalence.md#codereader) port. Lives in `adapters/rust`.

- implements: LanguageBackend
- depends on: Extraction
- depends on: ReaderError

## RustReader

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.2.2 anchor:"the doubled parse pays no cost" -->
Concrete [CodeReader](equivalence.md#codereader), [VerbReader](equivalence.md#verbreader)
and [CodeFacts](equivalence.md#codefacts) implementation for Rust source,
constructed at the composition root with the [ParseCache](#parsecache) of
the run (graph-specs-016-parse-once-reading-port#3.4) and holding nothing
else: the walk is [RustLoader](#rustloader)'s, the parse happens once, and
every capability reads the cache it holds. `extract` over a
[CodeFileSet](equivalence.md#codefileset) pulls the
[Extraction](equivalence.md#extraction) through [RustBackend](#rustbackend),
filters raw edges against the discovered
[ConceptNode](equivalence.md#conceptnode) set and assembles a
[Graph](equivalence.md#graph) for the diff engine — one node per top-level
`pub struct`, `pub enum`, `pub trait`, `pub type`, signatures normalized by
`signature-norm` (graph-specs-016-parse-once-reading-port#3.5), relationship
edges from struct fields, impl blocks and function signatures, trait objects
and impl-trait parameters included (graph-specs-017-founding-graph-model#3.3).
`extract_pub_fns` feeds the verb-anchoring pass from the same cache; the
separate walk graph-specs-005-verb-coverage-report#3.2 once budgeted is
collapsed into the one load (graph-specs-016-parse-once-reading-port#3.6).
`concepts` keeps its root-taking contract and is served from the cache —
the source-walk parity reference the cfdb-query
[CfdbQueryReader](#cfdbqueryreader) ACL must match
(graph-specs-010-abstraction-level-equivalence R10-6). Lives in `adapters/rust`.

- implements: CodeReader
- implements: VerbReader
- implements: CodeFacts
- depends on: Graph
- depends on: ConceptNode
- depends on: Edge
- depends on: EdgeKind
- depends on: ReaderError
- depends on: PubFnDecl
- depends on: CodeFileSet
- returns: RustReader

### RustLoader

<!-- parent:spec:RustReader -->

The unit struct that implements [CodeLoader](equivalence.md#codeloader) for a
Rust tree: the one walk — traversal, the `.rs` filter, the read — ending in
the sorting constructor of [CodeFileSet](equivalence.md#codefileset).
Constructible before any parse state exists, since the cache-holding readers
are built after the load. Lives in `adapters/rust`.

- implements: CodeLoader
- depends on: CodeFileSet
- depends on: ReaderError

### RustAnchorResolver

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

### CfdbQueryReader

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

### CfdbAnchorResolver

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

## ParseCache

- status: draft (per graph-specs-016-parse-once-reading-port#3.4)
<!-- parent:rfc:graph-specs-016-parse-once-reading-port#3.4 anchor:"built once per run by" -->

The Rust adapter's parse cache: a handle over the parsed files of one
[CodeFileSet](equivalence.md#codefileset), built once per run from the root
and the loaded set, each entry bundling the parsed file with its
pre-computed containment provenance so that no consumer needs the root
after construction. It is the adapter's and never crosses `ports` — `syn`
stays behind it, its inner field is private, and it is a cloneable owned
handle to one shared table, never a process-global: every run constructs
its own. [RustReader](#rustreader), [RustBackend](#rustbackend) and
[RustAnchorResolver](#rustanchorresolver) take it at construction, at the
composition root, and the three code walks collapse to the one the loader
makes. Lives in `adapters/rust`.

## PhpAttributeReader

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
`adapters/php`. An attribute key outside the set the contract fixes —
`implements`, `extends`, `signature`
(graph-specs-004-multi-language-adapter-contract#3.5) — is a
[Violation](equivalence.md#violation) naming the key and the concept, never
a skip; the reader carries the inline-attribute format on every fact it
emits and declares as bullets only what the source walk answers, a type
named by a signature.

- implements: SpecReader
- implements: SpecLoader
- depends on: Graph
- depends on: ReaderError
- depends on: Violation
- returns: PhpAttributeReader
- depends on: SpecFileSet

### RustSignatures

<!-- parent:spec:SignatureNormalizer -->

The [SignatureNormalizer](equivalence.md#signaturenormalizer) implementation
for the `rust` fence tag. It carries no state and answers by the `normalize`
of graph-specs-016-parse-once-reading-port#3.5 — the `syn`-specific
normalizer moved verbatim into `signature-norm` — so the comparison target
of a `rust` fence is the same bytes the Rust code side emits. It is not a
reader and reads no tree. Lives in `adapters/signature`; the composition
root supplies it for `rust` and for no other tag.

- implements: SignatureNormalizer

### PhpSignatures

<!-- parent:spec:SignatureNormalizer -->

The [SignatureNormalizer](equivalence.md#signaturenormalizer) implementation
for the `php` fence tag. It carries no state and answers by
`adapter-php::normalize` of
graph-specs-004-multi-language-adapter-contract#3.6: the fence parsed by the
pinned `tree-sitter-php` grammar (graph-specs-011-php-ladder#3.3), its
tokens re-printed single-spaced with comments, attributes and body dropped,
byte-equal to the target the PHP code side carries; a fence the grammar
cannot parse, or a section carrying two `php` fences, answers the
`Unparseable` state with the tag naming the language. It is not a reader:
[PhpAttributeReader](#phpattributereader) reads attributes and implements
no second port. Lives in `adapters/php`; the composition root supplies it
for `php` and for no other tag.

- implements: SignatureNormalizer

