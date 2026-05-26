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

- verb: tokenise_target

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

## ContextReader

The v0.4 bounded-context port trait. Separate from [Reader](#reader)
because not every adapter parses context files — the rust adapter
implements only [Reader](#reader); the markdown adapter implements
both. Returns a list of [ContextDecl](#contextdecl) values or a
[ReaderError](#readererror) on malformed input. An empty list is a
valid result on v0.3 spec trees. Lives in `ports`.

```rust
pub trait ContextReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError>;
}
```

- depends on: ContextDecl
- depends on: ReaderError

## ReaderError

Failure modes of a [Reader](#reader) implementation. Describes
*reading operations* (I/O, parse, walk) rather than domain concerns,
which is why this type lives in the port layer rather than in `domain`.
Adapters map their language-specific failures onto `ReaderError` at the
port boundary.

## LanguageBackend

Lower-level code-side port: walks a source root in one pass and emits an
[Extraction](#extraction) of flat [ConceptNode](#conceptnode) values plus
raw [Edge](#edge) values, BEFORE the language-neutral known-concept edge
filter runs. Each `impl LanguageBackend for FooBackend` covers one source
language; [`detect`](#languagebackend) lets the CLI dispatch on marker
files (`Cargo.toml` for Rust, `composer.json` for PHP, `tsconfig.json`
for TypeScript). Roadmap (#83 reframe, RFC-005 / 006): each backend
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

## Extraction

Bundle returned by [LanguageBackend::extract](#languagebackend) — a flat
[ConceptNode](#conceptnode) vector and a flat [Edge](#edge) vector, the
latter unfiltered. Graph assembly (filtering raw edges against the
known-concept set) is performed by the calling [Reader](#reader)
adapter, in language-neutral code. Lives in `ports`.

- depends on: ConceptNode
- depends on: Edge

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
§3.2 dry-run rust-systems-A) and is never invoked by `check`. Lives in
`adapters/rust`.

- implements: Reader
- implements: VerbReader
- depends on: Graph
- depends on: ReaderError
- depends on: PubFnDecl

## OwnedUnit

A crate, npm package, Go module, or equivalent — the thing a bounded
context "owns" in the v0.4 context-mapping vocabulary per
[RFC-001](../../docs/rfc/001-bounded-context-equivalence.md).
Language-agnostic name so non-Rust adapters can interpret it under their
own build system. Lives in `domain`.

## ContextDecl

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

## ContextExport

A concept a context publishes under a named DDD pattern. Export-centric
framing (Evans Ch. 14) — the supplying context is authoritative about
what it publishes; importers reference exported concepts. Lives in
`domain`.

- depends on: ContextPattern

## ContextImport

A cross-context reference a context declares as sanctioned. Names the
supplier context, the [ContextPattern](#contextpattern) under which the
relationship is classified, and the concept being referenced. Lives in
`domain`.

- depends on: ContextPattern

## ContextPattern

A DDD context-mapping pattern. v0.4 ships four variants: Shared Kernel,
Customer-Supplier, Conformist, Published Language. Anti-Corruption
Layer, Separate Ways, and Open Host Service are deferred to v0.5 per
RFC-001 §2. Marked `#[non_exhaustive]` so future-variant additions are
non-breaking for downstream consumers. Lives in `domain`.

## ContextViolation

The three bounded-context-level violation variants, wrapped by
[Violation](#violation)'s `Context` arm so consumers that do not opt
into context checking match one arm rather than three. Each variant
carries a `concept` field so deterministic violation ordering continues
to work across all four equivalence levels. Marked `#[non_exhaustive]`.
Lives in `domain`.

- depends on: OwnedUnit
- depends on: EdgeKind
- depends on: Source

## CheckInput

Input envelope to the v0.5 diff on the spec side — concept graph plus
declared bounded-context map plus verb-anchoring data. Keeps
[Graph](#graph) focused on concepts and edges (SOLID SRP, per RFC-001
round-1 architect review); contexts and verb ownership are carried
alongside. An empty `contexts` list reduces diff behavior to v0.3
(context pass is a no-op); an empty `verb_ownership.anchors` skips
v0.5 entirely. Lives in `domain`.

- depends on: Graph
- depends on: ContextDecl
- depends on: VerbOwnership
- returns: CheckInput
- verb: diff
- verb: context_for_concept

## SchemaVersion

The NDJSON wire-contract version stamped on every record emitted by
`graph-specs check --format=ndjson`. Promoted from a serialization
literal to a domain-owned Published Language type so downstream
consumers (notably qbot-core's `compare-spec-change` pipeline, tracked
in `yg/qbot-core#4034`) import this type and dispatch parse behavior
against it rather than re-typing `"1"` / `"2"` magic strings per
consumer. The current production value is the associated constant
`SchemaVersion::CURRENT` (today: `V2`). Retaining `V1` keeps the
overlap-window reader path typed — consumers gating on this enum at
parse time get an exhaustiveness check the day a future RFC bumps
`CURRENT`. Marked `#[non_exhaustive]` so future-version additions
(v3+) are non-breaking for downstream consumers. Lives in `domain`.

See `specs/ndjson-output.md` §Schema evolution for the bump rules
(breaking changes bump; non-breaking additions do not) and
`docs/rfc/001-bounded-context-equivalence.md` §3.3 for the v1→v2
ratification decision.

## VerbReader

The v0.5 verb-extraction port trait. Sibling to [Reader](#reader) and
[ContextReader](#contextreader) — separate per RFC-005 §3.2 clean-arch
lens. Not every adapter extracts verbs (markdown has no code items);
returning an empty `Vec` is the correct implementation for adapters that
do not walk code. Only invoked by the `report` subcommand, never by
`check`. Lives in `ports`.

```rust
pub trait VerbReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}
```

- depends on: PubFnDecl
- depends on: ReaderError

## PubFnDecl

A top-level `pub fn` declaration found in code — the verb counterpart to
[ConceptNode](#conceptnode) (which captures pub types). Carries the
function name, a [Source](#source) pointing back to the declaration site,
and an optional `owned_unit` string for bounded-context membership lookup.
Per RFC-005 §3.3. Lives in `domain`.

- depends on: Source

## VerbDecl

A top-level `pub fn` declaration prepared for verb-anchoring: name
(`qname`), optional owning crate (`owned_unit`), and [Source](#source).
Convertible from [PubFnDecl](#pubfndecl) via `From`. Used by
[`VerbOwnership`](#verbownership) to represent the code side of the
verb-anchoring contract. Lives in `domain`.

- depends on: Source
- depends on: PubFnDecl
- returns: VerbDecl

## VerbAnchor

Spec-side anchor parsed from a `- verb: <ident>` bullet inside a concept
section. `concept` names the owning concept; `qname` is the bare
identifier; `raw_target` preserves the verbatim bullet text;
`source` points to the spec file line. Used by
[`VerbOwnership`](#verbownership) to represent the spec side of the
verb-anchoring contract. Lives in `domain`.

- depends on: Source

## VerbOwnership

Aggregates both sides of the verb-anchoring contract: `decls` (code
side, `Vec<VerbDecl>`) and `anchors` (spec side, `Vec<VerbAnchor>`).
Carried by [`CheckInput`](#checkinput) and consumed by the v0.5 verb
pass inside `diff`. `#[derive(Default)]` allows zero-cost construction
when no verb anchors are present (opt-in semantics). Lives in `domain`.

- depends on: VerbDecl
- depends on: VerbAnchor

## InvariantAnnotation

A parsed `[enforced-by:]` or `[prose-only:]` bracketed annotation
extracted from a spec `#### Operational invariants` bullet by
[MarkdownReader](#markdownreader). Carries `inv_id`, [TierKind](#tierkind),
`artifact`, `retire_when`, `prose_only_why`, and [Source](#source).
Per RFC-005 §3.3. Lives in `domain`.

- depends on: TierKind
- depends on: Source

## TierKind

Enforcement tier derived from an `[enforced-by:]` artifact path, or
`ProseOnly` for `[prose-only:]` waivers. Four variants in RFC-005 §3.3:
`Cypher` (`.cfdb/queries/*.cypher`), `Tier0` (pub trait/fn ref),
`ScriptFence` (`scripts/*.sh`), `ProseOnly` (explicit waiver). Marked
`#[non_exhaustive]` per RFC-005 §3.3 solid §5.3 finding 3 — RFC-006 may
add `BehaviorTest`. Lives in `domain`.

## VerbCoverageRecord

Report record: one `pub fn` in code, its bounded context (if known), and
whether any spec section cites it by name. `context: None` is the
report-mode analog of `ContextViolation::MembershipUnknown` — the fn
lives in a crate not declared under any context's `Owns` block.
Per RFC-005 §3.3. Lives in `domain`.

- depends on: PubFnDecl

## TierHistogramRecord

Report record: annotation count per [TierKind](#tierkind), partitioned by
bounded context. Per RFC-005 §3.3. Lives in `domain`.

- depends on: TierKind

## HomonymAppearance

A single context's appearance in a [HomonymRecord](#homonymrecord). Carries
`context_name`, `sanctioned_by_pattern` (derived via the exporter-wins
algorithm, RFC-005 §3.3 dry-run DDD-B), and `asymmetric` (set when export
and import patterns disagree for the same name, per RFC-001 §4 invariant
5). Per RFC-005 §3.3. Lives in `domain`.

- depends on: ContextPattern

## HomonymRecord

A name (pub fn or pub type) that appears in more than one bounded context.
Each appearance is a [HomonymAppearance](#homonymappearance) enriched with
the sanctioning [ContextPattern](#contextpattern). Per RFC-005 §3.3.
Lives in `domain`.

- depends on: HomonymAppearance

## ReportOutput

Aggregated output of the verb-coverage report: three record lists —
`verb_coverage` ([VerbCoverageRecord](#verbcoveragerecord) vec),
`tier_histogram` ([TierHistogramRecord](#tierhistogramrecord) vec), and
`homonyms` ([HomonymRecord](#homonymrecord) vec). Produced by
`report_verb_coverage`. Per RFC-005 §3.3. Lives in `domain`.

- depends on: VerbCoverageRecord
- depends on: TierHistogramRecord
- depends on: HomonymRecord
- verb: report_verb_coverage

## ReportFormat

Output format for `graph-specs report`. `text` is the human-readable
default; `ndjson` emits one JSON object per report record — see
`specs/ndjson-output.md` §Report records (v0.5) for the schema. Lives
in `application`.
