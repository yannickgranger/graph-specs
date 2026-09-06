# equivalence

Concept-level entries for the **equivalence** bounded context — the
domain of the checker itself: the graph model, the diff engine, the
port contracts readers implement, and the violation vocabulary
downstream consumers observe. Every type whose code lives under
`domain/src/` or `ports/src/` (the context's `Owns` block in
`specs/contexts/equivalence.md`) appears here. Every top-level `pub`
type in those crates must have a heading; every heading must
correspond to such a type. Prose is encouraged — it is ignored by
the reader.

## Graph

<!-- parent:rfc:graph-specs-017-founding-graph-model#3.1 anchor:"everything one reader found on one side of the check" -->

A collection of concept nodes and declared relationship edges extracted
from one side of the equivalence check (specs or code). Two graphs are
equivalent at concept level iff their node sets carry the same names;
equivalent at relationship level iff their edge sets also align after
the v0.3 opt-in rules apply. Lives in `domain`.

- depends on: ConceptNode
- depends on: Edge
- returns: Graph
- verb: Graph::new
- verb: Graph::empty

## ConceptNode

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.3.2 anchor:"cfdb's PHP `:Item` carries no such props" -->

A single named concept located at a specific source site. Carries the
concept's name, a [Source](#source) pointing back to where the reader
found it, and an optional [SignatureState](#signaturestate) payload for
v0.2 signature-level equivalence. The language-agnostic containment
triple `module_path` / `unit` / `context` is not a second copy on the
node: the node reads it from its own [Source](#source), which is the one
writer (graph-specs-010-abstraction-level-equivalence#4 invariant 9).
`new` is the no-provenance constructor; `with_provenance` is the builder
that writes the resolved triple into a code source, and
`with_declared_context` the builder that writes the declared context
into a spec source — one builder per side, neither reaching into the
other's. `module_path`, `unit` and `context` read whichever the node's
own source holds. Lives in `domain`.

Since graph-specs-013-spec-state-marker#3.3 it also carries the spec-state [Marker](#marker),
set by the markdown reader from the heading's own `- status:` bullet or
from the file's front matter. graph-specs-015-spec-retirement-state#3.3 widened it from a `bool` to
a value: two legal values now exist, and the sites that read it ask
different questions — the concept pass dispatches on **which** value,
while the anchor-suppression set asks only **whether** a marker is
present. Always unmarked on the code side.

Since graph-specs-014-grounding-polarity#3.4 it also carries [Polarity](#polarity), attached by the
`with_polarity` builder (mirroring `with_provenance` — not a positional
argument on `new`, which deliberately does not derive `Default`).
Spec-side only; the code side is a fact, not a declaration.

The marker and the polarity are two independent fields rather than one
fused carrier: different upstream sources, different grammars, different
extension seams.

- depends on: Source
- depends on: SignatureState
- depends on: Polarity
- depends on: Marker
- returns: ConceptNode
- verb: ConceptNode::new
- verb: ConceptNode::with_provenance
- verb: ConceptNode::with_declared_context
- verb: ConceptNode::with_polarity
- verb: ConceptNode::module_path
- verb: ConceptNode::unit
- verb: ConceptNode::context

### SignatureState

<!-- parent:spec:ConceptNode -->

The signature-level payload on a [ConceptNode](#conceptnode). `Absent`
means the reader produced no signature (v0.1 concept-only mode).
`Normalized` carries the byte-equal comparison target — the output of the language's normalizer, `adapter-rust::normalize` on a `syn::Item` or `adapter-php::normalize` on a tree-sitter declaration. `Unparseable` surfaces a spec-side fenced block, `rust` or `php`, that failed to parse, or a section with more than one fenced block of that language, the fence tag naming the language (graph-specs-004-multi-language-adapter-contract#3.6).

## Polarity

<!-- parent:rfc:graph-specs-014-grounding-polarity#3.1 anchor:"it tracks that definition and does not fork it" -->

The grounding-polarity payload on a [ConceptNode](#conceptnode) — which
direction a spec heading's obligation points (graph-specs-014-grounding-polarity#3.1). `Declared`
(the default) is the ordinary obligation: the concept must exist in code.
`Forbidden` expels the name — code must **not** bear it. `Illustrative`
names an example, so the heading neither compels nor satisfies a code
item. Lives in `domain`.

**This concept is imported, not defined here.** `polarity` is owned
upstream: defined in agentry's ratified `agentry-vocabulary`, authored via
Bosun's grounding key, realized as `cascade::Polarity`. graph-specs is a
**Conformist** — it tracks that definition and does not fork it. The three
values and their meanings are cited from cascade's `resolve_polarity`, not
re-derived; if upstream adds a value, that is the seam that changes, which
is why the enum is `#[non_exhaustive]`.

"Conformist" here is prose, not a wired relationship: this is *not*
[ContextPattern](#contextpattern)`::Conformist`, which is a formal enum
scoped to this repo's own bounded contexts (graph-specs-001-bounded-context-equivalence#6). Nothing here
formalises a cross-repo import.

**Disambiguation.** This is the *concept-grounding* sense of the word —
which way a heading's obligation points. It is not the vocabulary system's
word-polarity; cascade itself keeps the two apart (`WordPolarity`,
"Distinct from `Polarity`").

Data only, no predicate methods: the branch table lives at its single call
site in the diff, matching upstream, whose own `Polarity` has zero methods.

## CodeLanguage

<!-- parent:rfc:graph-specs-004-multi-language-adapter-contract#3.1.1 anchor:"The runtime / toolchain that owns a code fact" -->

The runtime or toolchain that owns a code fact, carried on the code
variant of [Source](#source). Markdown is not a member: markdown is a
spec format, not a code language. The set is open — a new language is
added here, and the diff engine never branches on the value.

## SpecFormat

<!-- parent:rfc:graph-specs-004-multi-language-adapter-contract#3.1.2 anchor:"The authoring format of a spec fact" -->

The authoring format of a spec fact, carried on the spec variant of
[Source](#source). The markdown value covers every spec file this
checker reads, concepts and contexts alike — the subdirectory split is a
reader detail, not a domain concept. The inline-attribute value covers
the attribute and decorator forms a language reader extracts. The set is
open.

## SignatureNormalizer

<!-- parent:rfc:graph-specs-004-multi-language-adapter-contract#3.6 anchor:"through a normalizer port supplied at the composition root" -->

The port by which the markdown reader normalizes a fenced signature of any
language without depending on an adapter crate: it takes the fence tag and the
block and answers the byte-equal comparison target, or the `Unparseable` state
with the tag naming the language. The composition root supplies the
implementation per fence tag — [RustSignatures](reading.md#rustsignatures)
for `rust`, [PhpSignatures](reading.md#phpsignatures) for `php`.

## Source

<!-- parent:rfc:graph-specs-004-multi-language-adapter-contract#3.1.3 anchor:"variants gain a typed payload, NOT a struct rewrite" -->

Where a concept was found — either in a spec file or a code file. Used
for error messages that point back at the file and line the violation
came from. Each variant carries the facts its own side resolves, and
neither reads the other's (graph-specs-010-abstraction-level-equivalence#3.4:
the spec-side declaration and the code-side resolution are two
questions, never one chain). The spec variant carries, beside its
location and line, the context the document declares — the author's
claim. The code variant carries, beside its location and line, the
containment triple the cohesion pass reads — `module_path`, `unit`,
`context` — as the NDJSON source object already does
(`specs/ndjson-output.md`), so a code fact identifies itself by name and
unit at the source and no side index keyed on a bare name stands between
a record and its provenance (graph-specs-010-abstraction-level-equivalence#4
invariant 9); a context mismatch is a comparison of the two sources. One
writer per side: the composition root writes the declared context into
the spec node's source, the code adapter writes the resolved triple into
the code node's. For a fact read from a code-fact keyspace whose graph
carries no file node, the location is the containing module's qualified
name — the only coordinate the graph affords
(graph-specs-011-php-ladder#3.2; cfdb-045-polyglot-relationship-edges#3.4)
— and every message that prints it labels it a namespace, never a path.

The code variant also carries the kind of its location — a path, or a namespace — set by the reader that produced the fact: the keyspace reader sets namespace for a fact whose graph carries no file node, every other reader sets path; a message that prints the location labels it by that kind, and never infers the kind from the value or from the run (graph-specs-010-abstraction-level-equivalence#3.4: the code-side resolution answers with its own facts).

### LocationKind

<!-- parent:spec:Source -->

What kind of location a code-side [Source](#source) carries — a `Path` or
a `Namespace`. Set by the reader that produced the fact: the keyspace
reader sets `Namespace` for a fact whose graph carries no file node,
every other reader sets `Path`. A message that prints the location
labels it by this kind and never infers the kind from the value or from
the run, so a namespace that happens to contain a slash is still a
namespace and a path that happens to contain a backslash is still a
path. Pure value, `Path` by default. Lives in `domain`.

- verb: LocationKind::as_label

### SourceWithSig

<!-- parent:spec:Source -->

One reader's answer about a concept's signature, paired with the
[Source](#source) it read it from. Carried in the `sources` list of the
intra-side drift [Violation](#violation), which reports every disagreeing
reader rather than choosing between them: the pair is what makes the report
actionable, since a signature without the site it came from names no file to
open. Pure value. Lives in `domain`.

### DiffSide

<!-- parent:spec:Violation -->

Which side of the diff a [Violation](#violation) is about — the spec side or
the code side. Used only by the intra-side drift variant, where the
disagreement is among readers of one side and the side is the thing that must
be said; every other variant names its sides by carrying a `spec_source` and a
`code_source`, and needs no value for it. Pure value. Lives in `domain`.

- verb: DiffSide::as_label

## Violation

<!-- parent:rfc:graph-specs-012-non-pub-spec-anchor#3.5 anchor:"an anchor naming a nonexistent item" -->

A single equivalence violation between spec and code graphs. Concept-,
signature-, and relationship-level variants share the convention that
the first-carried field is the concept or owner name, so CLI output can
be sorted deterministically regardless of violation kind. The variant set
includes `DanglingAnchor` (graph-specs-012-non-pub-spec-anchor#3.5) for the case where a `- impl:`
anchor
names a code item that does not exist — a **top-level** arm (not nested
in `Cohesion`) so opting out of cohesion checking cannot suppress
broken-anchor detection.

graph-specs-013-spec-state-marker#3.4 **retired** `ImplementsDraftConcept`. A code item backing
a marked heading is the normal mid-arc state, not a failure; it is
reported as a [RealizedRecord](#realizedrecord) instead. The variant's
sort slot (13) is retired, not reused — existing slots are not
renumbered.

graph-specs-014-grounding-polarity#3.4 adds `ForbiddenConceptReintroduced { name, spec_source,
code_source }` — a code item bearing a name its heading expelled
([Polarity](#polarity)`::Forbidden`). Both sites are carried, so the
finding names what expelled the name *and* what reintroduced it. Sort
slot 15, appended after `DanglingAnchor` (14).

## Edge

<!-- parent:rfc:graph-specs-017-founding-graph-model#3.2 anchor:"a relationship one concept declares about another" -->

A declared relationship between two concepts (v0.3): `implements`,
`depends on`, or `returns`. Each edge owns a tokenised matching target
plus the raw textual form preserved for display in drift messages.

- verb: tokenise_target

### ConceptRef

<!-- parent:spec:Edge -->

A concept-rung endpoint of an [Edge](#edge), carrying three facts: the
concept's name; its context, always — the spec side resolves it through
the document that authored the edge (its own concept, or a concept the
document's Imports block sanctions from a named supplier; a name that is
neither is unresolved and reported), the code side through the unit
index (graph-specs-010-abstraction-level-equivalence#3.4, both
questions); and its owning unit where the side resolves one — the code
side, whose identity is `(name, unit)` under
graph-specs-010-abstraction-level-equivalence#4 invariant 9; the spec
side declares a context and never a unit, so an absent unit there is
that side's truth. Two endpoints compare by `(name, context)` at the
crossing rung. An endpoint carrying a name alone cannot say which of two
same-named concepts under two units it means, so a crossing is attributed
to whichever won the name; carrying the context and, where known, the
unit makes the edge say which `Clock` it means. A relationship edge whose
far end is an item outside every declared prefix is reported as a
crossing out of the declared surface, never dropped
(graph-specs-011-php-ladder#4 invariant 3 rules the item, not the edge).
Pure value. Lives in `domain`.

- depends on: OwnedUnit
- returns: ConceptRef
- verb: ConceptRef::named
- verb: ConceptRef::resolved

## EdgeKind

<!-- parent:rfc:graph-specs-017-founding-graph-model#3.3 anchor:"The kind of an edge is the kind of relationship it states" -->

The relationship kind of an [Edge](#edge). Closed set for v0.3;
future dialect growth adds variants here.

- verb: EdgeKind::as_label
- verb: EdgeKind::fmt

## Reader

- status: retired (per graph-specs-016-parse-once-reading-port#3.3)
<!-- parent:rfc:graph-specs-004-multi-language-adapter-contract#3.2 anchor:"The `Reader` trait (`ports/src/lib.rs:15`) stays" -->

The language-neutral port trait. Concrete readers (markdown specs,
Rust code, later PHP / TypeScript) implement it and produce graphs with
identical shape. Lives in `ports`.

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}
```

## SpecReader

<!-- parent:rfc:graph-specs-016-parse-once-reading-port#3.3.1 anchor:"`Reader` splits into `SpecReader` and `CodeReader` and the shared name retires" -->

The spec-side capability port: `extract` over a [SpecFileSet](#specfileset)
answers the [Graph](#graph) of concept nodes and declared edges, or a
[ReaderError](#readererror) — structurally `ParseFailed` alone, the walk's
`IoFailed` and `WalkFailed` being the loader's province. It takes the name
[Reader](#reader) could not keep: two nominal input types cannot share one
non-generic method signature, so the shared trait splits by side and the
shared name retires. One capability per trait, as every port here.
Implemented by [MarkdownReader](reading.md#markdownreader) and
[PhpAttributeReader](reading.md#phpattributereader). Lives in `ports`.

### CodeReader

<!-- parent:spec:SpecReader -->

The code-side twin: `extract` over a [CodeFileSet](#codefileset) answers the
code-side [Graph](#graph), implemented by [RustReader](reading.md#rustreader),
which holds the code root it derives provenance from and, once the parse
cache of graph-specs-016-parse-once-reading-port#3.4 lands, serves from that
cache
instead of walking again.


### VerbAnchorReader

<!-- parent:spec:SpecReader -->

The capability that reads the verb anchors of a spec set — the `- verb:`
bullets of graph-specs-006-verb-anchoring — over a
[SpecFileSet](#specfileset), answering [VerbAnchor](#verbanchor) values;
formerly an inherent method of the markdown reader, now a sibling port of
the same one-capability shape, performing no I/O because its input carries
the text. Implemented by [MarkdownReader](reading.md#markdownreader). Lives
in `ports`.

### ConceptAnchorReader

<!-- parent:spec:SpecReader -->

The capability that reads the concept anchors of a spec set — the anchor
bullet of graph-specs-012-non-pub-spec-anchor#3.2 — over a
[SpecFileSet](#specfileset), answering [ConceptAnchor](#conceptanchor)
values. Same shape, same implementor, no I/O. Lives in `ports`.

### AnnotationReader

<!-- parent:spec:SpecReader -->

The capability that reads the invariant annotations of a spec set over a
[SpecFileSet](#specfileset), answering
[InvariantAnnotation](#invariantannotation) values. Same shape, same
implementor, no I/O. Lives in `ports`.

### SpecTreeReader

<!-- parent:spec:SpecReader -->

The capability that assembles the heading trees of a spec set over a
[SpecFileSet](#specfileset), answering one [SpecTree](#spectree) per file:
the markdown event walk and the [ReaderError](#readererror) it can produce
stay adapter-side behind this port, while the tree itself and its methods
live in `domain` (graph-specs-016-parse-once-reading-port#3.3.3). It
replaces the adapter free function that answered an adapter type. Lives in
`ports`.
## LoadedFile

<!-- parent:rfc:graph-specs-016-parse-once-reading-port#3.1 anchor:"a loaded artifact, path + text, no line, no side discriminant of its own" -->

One file as the loader handed it on: its path and its text, nothing more. A
different pipeline stage from [Source](#source), which is where a concept was
found — side, path, line — and kept off that stem so the context's published
language carries no same-stem homonym. Standard-library types only; no `syn`
type enters `ports`. Lives in `ports`.

### SpecFileSet

<!-- parent:spec:LoadedFile -->

The spec-side aggregate of loaded files — an aggregate, not a bag: its files
are private, the constructor sorts them by path, and reading is a slice in
that order. The order is load-bearing; the byte-stability invariant of
graph-specs-016-parse-once-reading-port#4 rests on it.

- depends on: LoadedFile
- returns: SpecFileSet

### CodeFileSet

<!-- parent:spec:LoadedFile -->

The code-side aggregate of loaded files, of the same shape as
[SpecFileSet](#specfileset): private files, a sorting constructor, an ordered
slice out. The nominal difference is what lets a capability trait take one
side and never the other.

- depends on: LoadedFile
- returns: CodeFileSet

## SpecLoader

<!-- parent:rfc:graph-specs-016-parse-once-reading-port#3.2 anchor:"The loader owns the single walk" -->

The port that owns the single walk over a spec tree: directory traversal, the
extension filter, the read of each file, then the sorting constructor of
[SpecFileSet](#specfileset). Partitioning `concepts/` from `contexts/` is no
longer the walker's: capability extractors filter by path prefix in memory,
through one shared predicate in the markdown adapter. A monomorphic trait
with exactly one implementor, [MarkdownReader](reading.md#markdownreader);
`IoFailed` and `WalkFailed` are its errors and no capability trait's. Lives
in `ports`.

### CodeLoader

<!-- parent:spec:SpecLoader -->

The code-side twin: the same single walk over a code tree, ending in a
[CodeFileSet](#codefileset); one trait of its own rather than a generic one
because no polymorphic call site exists. Its one implementor is
[RustLoader](reading.md#rustloader).

## CodeFacts

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.3.1 anchor:"fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>" -->

The code-side containment port (graph-specs-010-abstraction-level-equivalence#3.3). Where [Reader](#reader)
produces a full type-equivalence [Graph](#graph), `CodeFacts` answers the
narrower question of which concepts the code contains and each one's
language-agnostic containment provenance — the `module_path` / `unit` /
`context` triple on [ConceptNode](#conceptnode) that the cohesion pass
reads. Two adapters implement it under the §3.3 routing rule: the
source-walking `RustReader` for multi-crate repos (graph-specs itself) and
the cfdb-query `CfdbQueryReader` ACL for one-per-crate repos (agentry). Both
emit the agnostic triple, never cfdb's Rust-specific prop names, so the diff
engine stays language-neutral. Lives in `ports`.

The port answers the relationship facts of the tree beside its concepts —
`IMPLEMENTS` today — because edges are cfdb's facts like items are
(graph-specs-011-php-ladder#4 invariant 6;
cfdb-045-polyglot-relationship-edges#3.2). The source-walking adapter
answers from its own edge walk, the cfdb-query ACL from the keyspace's
`IMPLEMENTS` edges, both endpoints a [ConceptRef](#conceptref).

The port also states **which** relationship kinds it can answer for the
input it read, because a kind the input carries no fact of is unanswered
rather than unmet (graph-specs-010-abstraction-level-equivalence#11.6).
The source walk answers all three; the cfdb-query ACL answers
`IMPLEMENTS` on a PHP keyspace and, until it translates cfdb's Rust
relationship facts, nothing on a Rust one. The set is the reader's own
statement about its input, never a constant the composition root holds.

```rust
pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
    fn relationships(&self, root: &Path) -> Result<Vec<Edge>, ReaderError>;
    fn answerable_relationships(&self, root: &Path) -> Result<Vec<EdgeKind>, ReaderError>;
}
```

## ContextReader

<!-- parent:rfc:graph-specs-001-bounded-context-equivalence#3.6 anchor:"A new port, `ContextReader`" -->

The v0.4 bounded-context port trait. Separate from [Reader](#reader)
because not every adapter parses context files — the rust adapter
implements only [Reader](#reader); the markdown adapter implements
both. Returns a list of [ContextDecl](#contextdecl) values or a
[ReaderError](#readererror) on malformed input. An empty list is a
valid result on v0.3 spec trees. Lives in `ports`.

```rust
pub trait ContextReader {
    fn extract_contexts(&self, files: &SpecFileSet) -> Result<Vec<ContextDecl>, ReaderError>;
}
```

- depends on: ContextDecl
- depends on: ReaderError
- depends on: SpecFileSet

### ReaderError

<!-- parent:spec:Reader -->

Failure modes of a [Reader](#reader) implementation. Describes
*reading operations* (I/O, parse, walk) rather than domain concerns,
which is why this type lives in the port layer rather than in `domain`.
Adapters map their language-specific failures onto `ReaderError` at the
port boundary.

## LanguageBackend

<!-- parent:rfc:graph-specs-016-parse-once-reading-port#3.3.2 anchor:"recorded here as an RFC-004 amendment" -->

Lower-level code-side port: walks a source root in one pass and emits an
[Extraction](#extraction) of flat [ConceptNode](#conceptnode) values plus
raw [Edge](#edge) values, BEFORE the language-neutral known-concept edge
filter runs. Each `impl LanguageBackend for FooBackend` covers one source
language; [`detect`](#languagebackend) lets the CLI dispatch on marker
files (`Cargo.toml` for Rust, `composer.json` for PHP, `tsconfig.json`
for TypeScript). Roadmap (#83 reframe, graph-specs-005-verb-coverage-report / graph-specs-006-verb-anchoring): each backend
becomes a thin wrapper over a per-language `<lang>-items` shared crate
also consumed by cfdb. Lives in `ports`.

```rust
pub trait LanguageBackend {
    fn detect(&self, code_root: &Path) -> bool;
    fn extract(&self, code_root: &Path) -> Result<Extraction, ReaderError>;
}
```

- depends on: Extraction
- depends on: ReaderError

### Extraction

<!-- parent:spec:LanguageBackend -->

Bundle returned by [LanguageBackend::extract](#languagebackend) — a flat
[ConceptNode](#conceptnode) vector and a flat [Edge](#edge) vector, the
latter unfiltered. Graph assembly (filtering raw edges against the
known-concept set) is performed by the calling [Reader](#reader)
adapter, in language-neutral code. Lives in `ports`.

- depends on: ConceptNode
- depends on: Edge

## ContextDecl

<!-- parent:rfc:graph-specs-001-bounded-context-equivalence#3.7 anchor:"Six additions in `domain::`" -->

Declaration of a bounded context as parsed from `specs/contexts/<name>.md`.
Carries the context name, its [OwnedUnit](#ownedunit) set, its exports
(published concepts), its imports (sanctioned cross-context references),
and the source location the declaration came from. Exports and imports
both reference [ContextPattern](#contextpattern) for the DDD mapping
pattern that applies. Lives in `domain`.

- depends on: OwnedUnit
- depends on: ContextExport
- depends on: ContextImport
- depends on: Source
- returns: ContextDecl
- verb: detect_import_cycle
- verb: ContextDecl::new

### OwnedUnit

<!-- parent:spec:ContextDecl -->

A crate, npm package, Go module, or equivalent — the thing a bounded
context "owns" in the v0.4 context-mapping vocabulary per
graph-specs-001-bounded-context-equivalence.
Language-agnostic name so non-Rust adapters can interpret it under their
own build system. Lives in `domain`.

### ContextExport

<!-- parent:spec:ContextDecl -->

A concept a context publishes under a named DDD pattern. Export-centric
framing (Evans Ch. 14) — the supplying context is authoritative about
what it publishes; importers reference exported concepts. Lives in
`domain`.

- depends on: ContextPattern

### ContextImport

<!-- parent:spec:ContextDecl -->

A cross-context reference a context declares as sanctioned. Names the
supplier context, the [ContextPattern](#contextpattern) under which the
relationship is classified, and the concept being referenced. Lives in
`domain`.

- depends on: ContextPattern

## DeclaredSurface
<!-- parent:rfc:graph-specs-011-php-ladder#3.2 anchor:"whose qualified name begins with a declared" -->

The concept surface of a code tree whose language affords no per-item
visibility. Rust's surface test is `pub`, a property of each item; PHP has
none, and every-class-under-the-autoload-root is refused as the relaxed
filter graph-specs-012-non-pub-spec-anchor#3.1 already rejects. The
surface is declared, never derived: the set of [OwnedUnit](#ownedunit)
prefixes the repository's `specs/contexts/` declares, read by a
[CodeFacts](#codefacts) adapter as the admission test on a qualified
name — an item is on the surface exactly when its qualified name begins
with a declared prefix, and an item outside every prefix binds no heading
and demands none. The owned-unit string stays opaque and language-agnostic
(graph-specs-004-multi-language-adapter-contract#2, invariant 6): a PHP
repository writes a namespace prefix where a Rust repository writes a
crate name, and no marked form distinguishes the two — that question is
RFC-007's (graph-specs-011-php-ladder#6 item 2). Pure value, no IO. Lives
in `domain`.

- depends on: ContextDecl
- depends on: DeclaredSurface
- depends on: OwnershipAmbiguity
- verb: DeclaredSurface::from_contexts
- verb: DeclaredSurface::admits
- verb: DeclaredSurface::unit_of
- verb: DeclaredSurface::is_empty

### OwnershipAmbiguity

<!-- parent:spec:DeclaredSurface -->

Two contexts whose declarations both own one item — an outer prefix and
its context, an inner prefix and its context. A value the declared
surface returns instead of a surface when two contexts nest their
prefixes; the composition root turns it into a could-not-run naming both
prefixes and both contexts, never a resolution by length across contexts
(graph-specs-011-php-ladder#3.2: one declared surface per context;
graph-specs-010-abstraction-level-equivalence#11.6: a shape the reader
cannot decide says so). Within one context's own Owns block the longest
declared prefix wins and no ambiguity arises, because no question of
which context owns the item exists there.

## ContextPattern

<!-- parent:rfc:graph-specs-001-bounded-context-equivalence#2 anchor:"Four DDD patterns" -->

A DDD context-mapping pattern. v0.4 ships four variants: Shared Kernel,
Customer-Supplier, Conformist, Published Language. Anti-Corruption
Layer, Separate Ways, and Open Host Service are deferred to v0.5 per
graph-specs-001-bounded-context-equivalence#2. Marked `#[non_exhaustive]` so future-variant additions are
non-breaking for downstream consumers. Lives in `domain`.

- verb: ContextPattern::as_label
- verb: ContextPattern::variants
- verb: ContextPattern::is_doctrine_sanctioned
- verb: ContextPattern::fmt

## ContextViolation

<!-- parent:rfc:graph-specs-001-bounded-context-equivalence#3.2 anchor:"wrapped under a new `ContextViolation` enum" -->

The bounded-context-level violation variants, wrapped by
[Violation](#violation)'s `Context` arm so consumers that do not opt
into context checking match one arm rather than three. Each variant
carries a `concept` field so deterministic violation ordering continues
to work across all four equivalence levels. Marked `#[non_exhaustive]`.
Lives in `domain`.

- depends on: OwnedUnit
- depends on: EdgeKind
- depends on: Source
- verb: ContextViolation::concept

## CheckInput

<!-- parent:rfc:graph-specs-009-draft-implementation-diagnostic#3.2 anchor:"pub draft_concepts: Vec<ConceptNode>" -->

Input envelope to the v0.5 diff on the spec side — concept graph plus
declared bounded-context map plus verb-anchoring data. Keeps
[Graph](#graph) focused on concepts and edges (SOLID SRP, per graph-specs-001-bounded-context-equivalence
round-1 architect review); contexts and verb ownership are carried
alongside. An empty `contexts` list reduces diff behavior to v0.3
(context pass is a no-op); an empty `verb_ownership.anchors` skips
v0.5 entirely. Lives in `domain`.

Carries no draft-concept side index: graph-specs-013-spec-state-marker#3.3 consolidated
spec-state onto [ConceptNode](#conceptnode)'s `marked` field, so the
graph is the single carrier and there is no second object graph joined
by name.

- depends on: Graph
- depends on: ContextDecl
- depends on: VerbOwnership
- depends on: CohesionViolation
- depends on: ResolvedAnchor
- depends on: Violation
- returns: CheckInput
- verb: diff
- verb: context_for_concept
- verb: context_for_code_node
- verb: resolve_declared_context
- verb: CheckInput::new
- verb: CheckInput::with_graph_and_contexts
- verb: CheckInput::with_spec_cohesion
- verb: CheckInput::with_spec_findings
- verb: CheckInput::with_concept_anchors

### ResolvedAnchor

<!-- parent:spec:CheckInput -->

A [ConceptAnchor](#conceptanchor) paired with its code-side resolution
verdict (graph-specs-012-non-pub-spec-anchor#3.4) — `Some(`[AnchorTarget](#anchortarget)`)` when the
named item exists in code, `None` when it does not. Built by the
application (which resolves each target through the
[AnchorResolver](#anchorresolver) port) and handed to the diff, so the
diff engine stays pure and calls no resolver itself. An anchored concept
is exempt from `MissingInCode`; an unresolved anchor becomes
[Violation](#violation)'s `DanglingAnchor` arm. Lives in `domain`.

## Provenance

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.6 anchor:"Source objects gain" -->

The containment-provenance record rendered into NDJSON code-kind source
objects (graph-specs-010-abstraction-level-equivalence#3.6 / #136) — the emitter-facing form of the agnostic
triple that [ConceptNode](#conceptnode) carries as three loose `Option`
fields: `module_path` (crate-root-collapsed module path), `unit` (the
owning crate, relative to the code root) and `context` (the bounded
context whose `specs/contexts/` Owns block owns `unit`). Pure data;
every field is optional because each is independently unavailable.

The diff snapshots one record per code concept, keyed by concept name,
into `CheckOutcome::provenance` before the code nodes are consumed —
`context` resolves through the same Owns lookup the cohesion pass uses,
so the emitter never re-derives it (that would split-brain the
resolution, which is why #136 was split out of #128 in the first
place). Lives in `domain`.

## Marker

<!-- parent:rfc:graph-specs-015-spec-retirement-state#3.1 anchor:"no third value exists" -->

Which spec-state marker a concept heading carries (graph-specs-013-spec-state-marker#3.1,
widened by graph-specs-015-spec-retirement-state#3.1). Read by the markdown reader from the
heading's first content line, or inherited from the file's front
matter, and carried on [ConceptNode](#conceptnode).

Two legal values, and **neither transitions to the other**. `draft`
declares code owed to *exist*, and ratification is deletion of the
line. `retired` declares code owed to be *gone*: it is written while
the backing item is still present, and it is never deleted. A third
state — unmarked — is the ordinary heading.

Still a presence flag per value, never a state machine, because the
progress axis is the code rather than the marker: nothing in the tree
rewrites one value into the other. `is_marked` answers the narrower
question of whether any marker is present at all, which is what the
anchor-suppression set asks — an unresolved `- impl:` target under
either value is the state the marker announces, not a dangling anchor.
Lives in `domain`.

Predicate only, no data beyond the three values — `is_marked` sits on
the type because two call sites ask that same narrower question, and a
`matches!` copied to each is the split-brain that invites a third
spelling.

## CheckOutcome

<!-- parent:rfc:graph-specs-015-spec-retirement-state#3.5 anchor:"two rules, and only one of them is" -->

The full result of one equivalence check (graph-specs-013-spec-state-marker#3.5, widened by
graph-specs-015-spec-retirement-state#3.5): the violation list plus the four marker-record lists —
`pending` and `realized` for the `draft` value, `retirement_incomplete`
and `retirement_complete` for `retired`. The diff produces all of them,
because the record decision is the same concept/code matching it
already performs for the unmarked rows — deriving it in the application
layer would split-brain one decision across two places.

Since graph-specs-010-abstraction-level-equivalence#3.6 / #136 it also carries `provenance` — the
[Provenance](#provenance) index (concept name → triple) the NDJSON
emitter reads to render code-kind source objects. A side index on the
outcome rather than fields on [Violation](#violation), so the stable
violation enum is untouched across its construction sites.

The exit code is a function of `violations` alone: a tree whose only
findings are marker records passes, under either marker value.
`is_clean` names that rule so no consumer has to re-derive it. `new` is
the assembling constructor — it sorts every marker list into the same
stable order (concept name, then spec site) so record order is
deterministic for a fixed input tree. Lives in `domain`.

Rendering and cleanliness are two different rules, and only one of them
is about zero. Every count is always rendered, even at zero, because an
absent segment is indistinguishable from a formatter that forgot it.
Cleanliness is the narrower question of which counts must be zero, and
two of these lists are not cleanliness terms: `pending` is a worklist,
and `retirement_complete` never drains, because the `retired` marker
line is never deleted. A never-draining term inside the clean state
would make the clean state unreachable.

- depends on: Violation
- depends on: PendingRecord
- depends on: RealizedRecord
- depends on: RetirementIncompleteRecord
- depends on: RetirementCompleteRecord
- returns: CheckOutcome
- verb: diff
- verb: CheckOutcome::new
- verb: CheckOutcome::empty
- verb: CheckOutcome::is_clean

### PendingRecord

<!-- parent:spec:CheckOutcome -->

A marked concept heading with no backing code item — row 3 of the
graph-specs-013-spec-state-marker#3.2 enforcement matrix. Emitted **instead of**
`Violation::MissingInCode`: the marker announces that the concept is
declared ahead of its code, so no code-existence obligation applies.

The skip that follows is `unobliged`, and it is **cited, not restated**:
it is stated once in `specs/dialect.md`, under what a heading obliges.
Written out here it would read as a rule about this record kind, and it
is not one — row 8 and both polarity values are members too, and after
graph-specs-015-spec-retirement-state a rule scoped to `PendingRecord` is false where it sits.

Not a failure. The pending list is the transcription worklist the
ratification workflow reads every run — a state field with a producer
and no reader rots. Lives in `domain`.

- depends on: Source

### RealizedRecord

<!-- parent:spec:CheckOutcome -->

A marked concept heading whose backing code item exists — row 4 of the
graph-specs-013-spec-state-marker#3.2 enforcement matrix, by name match or by `- impl:` anchor
resolution, exactly as an unmarked heading binds.

Emitted **in addition to** the normal, fully enforced equivalence
checks for that pair: a marker never parks a divergence, so drift under
a marked heading still fires its ordinary violation. The record is the
ratification signal — ratification is deletion of the marker line,
performed by a human. Lives in `domain`.

- depends on: Source

### RetirementIncompleteRecord

<!-- parent:spec:CheckOutcome -->

A `retired` heading whose backing code item is still present — row 7 of
the graph-specs-015-spec-retirement-state#3.2 enforcement matrix.

The retirement was announced and the code has not gone yet. Emitted
**in addition to** the normal, fully enforced equivalence checks for
that pair, exactly as [RealizedRecord](#realizedrecord) is: a marker
never parks a divergence, so drift under a retired heading still fires
its ordinary violation. Marker/code co-presence is not itself the
contradiction — it is the window every correct retirement opens.

Not a failure, and a cleanliness term: a clean tree carries none. Lives
in `domain`.

- depends on: Source

### RetirementCompleteRecord

<!-- parent:spec:CheckOutcome -->

A `retired` heading with no backing code item — row 8 of the graph-specs-015-spec-retirement-state#3.2
enforcement matrix. The retirement is done.

Emitted **instead of** `Violation::MissingInCode`, and carrying
[PendingRecord](#pendingrecord)'s obligation skip in full: a row-8
heading imposes nothing through its edge bullets, its verb anchors, or
its `- impl:` anchors. That is stated rather than inherited, because
silence resolves to armed — the skip is a set the passes are handed,
and a row-8 concept that nobody put in it is enforced.

Rendered like every other record, but **not** a cleanliness term: the
marker line is never deleted, so this list never drains, and a
never-draining term inside the clean state would make the clean state
unreachable. Lives in `domain`.

- depends on: Source

## SchemaVersion

<!-- parent:rfc:graph-specs-013-spec-state-marker#3.5.4 anchor:"a discriminator that silently stops appearing" -->

The NDJSON wire-contract version stamped on every record emitted by
`graph-specs check --format=ndjson`. Promoted from a serialization
literal to a domain-owned Published Language type so downstream
consumers (notably qbot-core's `compare-spec-change` pipeline, tracked
in `yg/qbot-core#4034`) import this type and dispatch parse behavior
against it rather than re-typing `"1"` / `"2"` magic strings per
consumer. The current production value is the associated constant
`SchemaVersion::CURRENT` (today: `V4`, per graph-specs-013-spec-state-marker#3.5). Retaining the
superseded variants keeps the
overlap-window reader path typed — consumers gating on this enum at
parse time get an exhaustiveness check the day a future RFC bumps
`CURRENT`. Marked `#[non_exhaustive]` so future-version additions
(v3+) are non-breaking for downstream consumers. Lives in `domain`.

See `specs/ndjson-output.md` §Schema evolution for the bump rules
(breaking changes bump; non-breaking additions do not) and
`docs/rfc/001-bounded-context-equivalence.md` §3.3 for the v1→v2
ratification decision.

- verb: SchemaVersion::as_str
- verb: SchemaVersion::fmt

## VerbReader

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.2.1 anchor:"Sibling trait to `ContextReader`" -->

The v0.5 verb-extraction port trait. Sibling to [Reader](#reader) and
[ContextReader](#contextreader) — separate per graph-specs-005-verb-coverage-report#3.2 clean-arch
lens. Not every adapter extracts verbs (markdown has no code items);
returning an empty `Vec` is the correct implementation for adapters that
do not walk code. Invoked by `check` (it feeds the v0.5/v0.6 verb-
anchoring pass with the code-side `pub fn` declarations) and by the
`report` subcommand. Lives in `ports`.

```rust
pub trait VerbReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}
```

- depends on: PubFnDecl
- depends on: ReaderError

## PubFnDecl

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.1 anchor:"code-side pub-fn fact" -->

A public function (top-level free `pub fn` OR public method inside an
impl block) found in code — the verb counterpart to
[ConceptNode](#conceptnode) (which captures pub types). Carries the
function name, a [Source](#source) pointing back to the declaration site,
and an optional `owned_unit` string for bounded-context membership lookup.
Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: Source

## VerbOwnership

<!-- parent:rfc:graph-specs-006-verb-anchoring#3.3 anchor:"pub decls: Vec<VerbDecl>, pub anchors: Vec<VerbAnchor>" -->

Aggregates both sides of the verb-anchoring contract: `decls` (code
side, `Vec<VerbDecl>`) and `anchors` (spec side, `Vec<VerbAnchor>`).
Carried by [`CheckInput`](#checkinput) and consumed by the v0.5 verb
pass inside `diff`. `#[derive(Default)]` allows zero-cost construction
when no verb anchors are present (opt-in semantics). Lives in `domain`.

- depends on: VerbDecl
- depends on: VerbAnchor

### VerbDecl

<!-- parent:spec:VerbOwnership -->

A public function (top-level free `pub fn` OR public impl-block method)
prepared for verb-anchoring. `qname` is a bare identifier for top-level
fns or a `Type::method` two-segment name for impl methods. Carries an
optional `owned_unit` string and a [Source](#source). Convertible from
[PubFnDecl](#pubfndecl) via `From`. Used by
[`VerbOwnership`](#verbownership) to represent the code side of the
verb-anchoring contract. Lives in `domain`.

- depends on: Source
- depends on: PubFnDecl
- returns: VerbDecl
- verb: VerbDecl::from

### VerbAnchor

<!-- parent:spec:VerbOwnership -->

Spec-side anchor parsed from a `- verb: <qname>` bullet inside a concept
section. `concept` names the owning concept; `qname` is either a bare
identifier (matching a top-level `pub fn`) or a `Type::method` two-segment
name (matching an impl-block method); `raw_target` preserves the verbatim
bullet text; `source` points to the spec file line. Used by
[`VerbOwnership`](#verbownership) to represent the spec side of the
verb-anchoring contract. Lives in `domain`.

- depends on: Source

## InvariantAnnotation

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.2 anchor:"spec-side annotation fact" -->

A parsed `[enforced-by:]` or `[prose-only:]` bracketed annotation
extracted from a spec `#### Operational invariants` bullet by
[MarkdownReader](#markdownreader). Carries `inv_id`, [TierKind](#tierkind),
`artifact`, `retire_when`, `prose_only_why`, and [Source](#source).
Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: TierKind
- depends on: Source

## TierKind

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.5 anchor:"Cypher, Tier0, ScriptFence, ProseOnly" -->

Enforcement tier derived from an `[enforced-by:]` artifact path, or
`ProseOnly` for `[prose-only:]` waivers. Four variants in graph-specs-005-verb-coverage-report#3.3:
`Cypher` (`.cfdb/queries/*.cypher`), `Tier0` (pub trait/fn ref),
`ScriptFence` (`scripts/*.sh`), `ProseOnly` (explicit waiver). Marked
`#[non_exhaustive]` per graph-specs-005-verb-coverage-report#3.3 solid §5.3 finding 3 — graph-specs-006-verb-anchoring may
add `BehaviorTest`. Lives in `domain`.

## VerbCoverageRecord

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.3 anchor:"pub_fn: PubFnDecl, cited: bool" -->

Report record: one `pub fn` in code, its bounded context (if known), and
whether any spec section cites it by name. `context: None` is the
report-mode analog of `ContextViolation::MembershipUnknown` — the fn
lives in a crate not declared under any context's `Owns` block.
Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: PubFnDecl

## TierHistogramRecord

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.4 anchor:"tier: TierKind, count: usize" -->

Report record: annotation count per [TierKind](#tierkind), partitioned by
bounded context. Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: TierKind

## HomonymRecord

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.6 anchor:"name: String, contexts: Vec<HomonymAppearance>" -->

A name (pub fn or pub type) that appears in more than one bounded context.
Each appearance is a [HomonymAppearance](#homonymappearance) enriched with
the sanctioning [ContextPattern](#contextpattern). Per graph-specs-005-verb-coverage-report#3.3.
Lives in `domain`.

- depends on: HomonymAppearance

### HomonymAppearance

<!-- parent:spec:HomonymRecord -->

A single context's appearance in a [HomonymRecord](#homonymrecord). Carries
`context_name`, `sanctioned_by_pattern` (derived via the exporter-wins
algorithm, graph-specs-005-verb-coverage-report#3.3 dry-run DDD-B), and `asymmetric` (set when export
and import patterns disagree for the same name, per graph-specs-001-bounded-context-equivalence#4 invariant
5). Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: ContextPattern

## ReportOutput

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.3.7 anchor:"verb_coverage: Vec<VerbCoverageRecord>" -->

Aggregated output of the verb-coverage report: three record lists —
`verb_coverage` ([VerbCoverageRecord](#verbcoveragerecord) vec),
`tier_histogram` ([TierHistogramRecord](#tierhistogramrecord) vec), and
`homonyms` ([HomonymRecord](#homonymrecord) vec). Produced by
`report_verb_coverage`. Per graph-specs-005-verb-coverage-report#3.3. Lives in `domain`.

- depends on: VerbCoverageRecord
- depends on: TierHistogramRecord
- depends on: HomonymRecord
- verb: report_verb_coverage

## AbstractionLevel

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.1 anchor:"Heading depth maps to a typed abstraction level" -->

One rung of the four-level abstraction ladder (graph-specs-010-abstraction-level-equivalence#3.1): `Context`
(H1), `Concept` (H2 — the diff unit), `SubConcept` (H3, diffed at L2),
`Member` (H4+, emitted not diffed). Depth is authoritative — a heading's
role *is* its depth. `from_heading_depth` maps a markdown heading depth
to its rung; adapters call it rather than `match`-ing the enum, so
`#[non_exhaustive]` never forces a dead wildcard arm in adapter crates.
Marked `#[non_exhaustive]`. Lives in `domain`.

- verb: AbstractionLevel::from_heading_depth

## CohesionViolation

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.5 anchor:"carries `spec_source` so its text rendering" -->

The abstraction-ladder cohesion violations (graph-specs-010-abstraction-level-equivalence#3.5), wrapped by
[Violation](#violation)'s `Cohesion` arm so consumers that do not opt
into cohesion checking match one arm rather than three — distinct from
[ContextViolation](#contextviolation) (graph-specs-001-bounded-context-equivalence cross-context edges).
Three variants: `ContextWithoutCohesionUnit` (an H1 context with no
H2/H3 concept under it) and `SubConceptOrphan` (an H3 with no enclosing
H2) fire spec-side with zero code facts; `ConceptContextMismatch` (the
spec-side declared owning context disagrees with the code-resolved one)
is code-fact-gated and carries a [Source](#source) so its rendering
shows `path:line` like every other violation. Marked `#[non_exhaustive]`.
The *detection* logic that emits these lands in R10-3; this entry covers
the type. Lives in `domain`.

- verb: CohesionViolation::key

## ConceptAnchor

<!-- parent:rfc:graph-specs-012-non-pub-spec-anchor#3.2 anchor:"the anchor is a bullet directive" -->

A concept heading explicitly bound to a named code item the concept walk
would not otherwise surface (graph-specs-012-non-pub-spec-anchor#3.2) — a `pub(crate)` type, a `fn`,
or a `const`. Parsed from a `- impl: <qname>` bullet, it *redirects* the
concept's equivalence target to the resolved item rather than requiring a
top-level `pub` type named like the heading. Shares the verb-bullet qname
grammar with [VerbAnchor](#verbanchor) (one grammar) but is a distinct
type: a `VerbAnchor` attributes a `pub fn` to a context, a `ConceptAnchor`
redirects a concept's equivalence target. An anchor naming a nonexistent
item fires [Violation](#violation)'s `DanglingAnchor` arm, so the link
stays two-way and zero-baseline. Lives in `domain`.

- verb: anchor_violation
- verb: behavioral_exemption_applies

## AnchorTarget

<!-- parent:rfc:graph-specs-012-non-pub-spec-anchor#3.4.1 anchor:"zero infrastructure imports" -->

A resolved anchor target — the code item an `AnchorResolver` found for a
qname, at any visibility (graph-specs-012-non-pub-spec-anchor#3.4). A pure domain type by
construction: it carries no infrastructure representation (`syn::Item`,
cfdb `Node`/`PropValue`); the resolving adapter translates into this
shape, keeping the dependency arrow pointing inward. Pairs an
[AnchorKind](#anchorkind) with a [Source](#source). Lives in `domain`.

### AnchorKind

<!-- parent:spec:AnchorTarget -->

The kind of code item an anchor resolved to (graph-specs-012-non-pub-spec-anchor#3.4): `Type`, `Fn`,
or `Const` — the three the source-walk MVP resolves, each a `syn::Item`
the reader already visits. Enum-variant resolution is deferred to R12-6
(cfdb-query, where `kind:"variant"` is native), so the enum is
`#[non_exhaustive]` to admit it without a breaking change. Lives in
`domain`.

## AnchorResolver

<!-- parent:rfc:graph-specs-012-non-pub-spec-anchor#3.4.3 anchor:"fn resolve(&self, qname: &str) -> Option<AnchorTarget>" -->

The anchor-resolution port (graph-specs-012-non-pub-spec-anchor#3.4) — a **separate** trait from
[CodeFacts](#codefacts) (ISP): not every code adapter resolves anchors, so
widening `CodeFacts` would force a deferred adapter to ship a stub. It
answers one question the concept walk does not: *does an item named
`qname` exist anywhere in the code, at **any** visibility?* — so a concept
whose canonical implementation is `pub(crate)` (or a `fn` / `const`) can be
a spec concept without a manufactured `pub` type. Resolution is consulted
only for the qnames an anchor references, so the global concept set is
unchanged. Lives in `ports`.

## SpecTree

<!-- parent:rfc:graph-specs-010-abstraction-level-equivalence#3.2 anchor:"H1/parent-tree assembly" -->

The assembled heading tree for a single spec file (graph-specs-010-abstraction-level-equivalence#3.2 / R10-2) —
a parent-linked vector of [HeadingNode](#headingnode) in document order,
assembled behind the [SpecTreeReader](#spectreereader) port over the one dialect read the concept
reader also projects, so the tree's rungs and the graph's nodes are the
same list and cannot diverge (keel-dialect §12.1). Exposes `context_id`
(the file's single bounded-context identifier) and
`cohesion_violations`, which surfaces the spec-side
[CohesionViolation](#cohesionviolation)s the tree's shape reveals — an H1
context with no concept under it, and orphaned H3 sub-concepts. Wiring the
detection into the `check` diff is R10-3. Lives in `domain` since
graph-specs-016-parse-once-reading-port#3.3.3, with its four methods; only
the assembly stays adapter-side.

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
orphaned sub-concept). Lives in `domain` (graph-specs-016-parse-once-reading-port#3.3.3).
