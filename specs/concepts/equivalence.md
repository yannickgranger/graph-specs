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

A single named concept located at a specific source site. Carries the
concept's name, a [Source](#source) pointing back to where the reader
found it, and an optional [SignatureState](#signaturestate) payload for
v0.2 signature-level equivalence. Since v0.6 (RFC-010 §3.3) it also
carries the language-agnostic containment triple `module_path` / `unit` /
`context` (each `Option<String>`), populated by a code-facts adapter and
left `None` on the spec side. `new` is the no-provenance constructor;
`with_provenance` is the builder that attaches the triple. Lives in
`domain`.

- depends on: Source
- depends on: SignatureState
- returns: ConceptNode

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
be sorted deterministically regardless of violation kind. The variant
set includes `ImplementsDraftConcept` for the case where a code item
implements a concept whose spec heading is still `status: draft`, and
`DanglingAnchor` (RFC-012 §3.5) for the case where a `- impl:` anchor
names a code item that does not exist — a **top-level** arm (not nested
in `Cohesion`) so opting out of cohesion checking cannot suppress
broken-anchor detection.

## Edge

A declared relationship between two concepts (v0.3): `implements`,
`depends on`, or `returns`. Each edge owns a tokenised matching target
plus the raw textual form preserved for display in drift messages.

- verb: tokenise_target

## EdgeKind

The relationship kind of an [Edge](#edge). Closed set for v0.3;
future dialect growth adds variants here.

- verb: EdgeKind::as_label
- verb: EdgeKind::fmt

## Reader

The language-neutral port trait. Concrete readers (markdown specs,
Rust code, later PHP / TypeScript) implement it and produce graphs with
identical shape. Lives in `ports`.

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}
```

## CodeFacts

The code-side containment port (RFC-010 §3.3). Where [Reader](#reader)
produces a full type-equivalence [Graph](#graph), `CodeFacts` answers the
narrower question of which concepts the code contains and each one's
language-agnostic containment provenance — the `module_path` / `unit` /
`context` triple on [ConceptNode](#conceptnode) that the cohesion pass
reads. Two adapters implement it under the §3.3 routing rule: the
source-walking `RustReader` for multi-crate repos (graph-specs itself) and
the cfdb-query `CfdbQueryReader` ACL for one-per-crate repos (agentry). Both
emit the agnostic triple, never cfdb's Rust-specific prop names, so the diff
engine stays language-neutral. Lives in `ports`.

```rust
pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
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
- verb: ContextDecl::new

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

- verb: ContextPattern::as_label
- verb: ContextPattern::variants
- verb: ContextPattern::is_doctrine_sanctioned
- verb: ContextPattern::fmt

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
- verb: ContextViolation::concept

## CheckInput

Input envelope to the v0.5 diff on the spec side — concept graph plus
declared bounded-context map plus verb-anchoring data. Keeps
[Graph](#graph) focused on concepts and edges (SOLID SRP, per RFC-001
round-1 architect review); contexts and verb ownership are carried
alongside. An empty `contexts` list reduces diff behavior to v0.3
(context pass is a no-op); an empty `verb_ownership.anchors` skips
v0.5 entirely. Lives in `domain`.

- depends on: Graph
- depends on: ConceptNode
- depends on: ContextDecl
- depends on: VerbOwnership
- depends on: CohesionViolation
- depends on: ResolvedAnchor
- returns: CheckInput
- verb: diff
- verb: context_for_concept
- verb: resolve_declared_context
- verb: CheckInput::new
- verb: CheckInput::with_graph_and_contexts
- verb: CheckInput::with_draft_concepts
- verb: CheckInput::with_spec_cohesion
- verb: CheckInput::with_concept_anchors

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

- verb: SchemaVersion::as_str
- verb: SchemaVersion::fmt

## VerbReader

The v0.5 verb-extraction port trait. Sibling to [Reader](#reader) and
[ContextReader](#contextreader) — separate per RFC-005 §3.2 clean-arch
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

A public function (top-level free `pub fn` OR public method inside an
impl block) found in code — the verb counterpart to
[ConceptNode](#conceptnode) (which captures pub types). Carries the
function name, a [Source](#source) pointing back to the declaration site,
and an optional `owned_unit` string for bounded-context membership lookup.
Per RFC-005 §3.3. Lives in `domain`.

- depends on: Source

## VerbDecl

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

## VerbAnchor

Spec-side anchor parsed from a `- verb: <qname>` bullet inside a concept
section. `concept` names the owning concept; `qname` is either a bare
identifier (matching a top-level `pub fn`) or a `Type::method` two-segment
name (matching an impl-block method); `raw_target` preserves the verbatim
bullet text; `source` points to the spec file line. Used by
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

## AbstractionLevel

One rung of the four-level abstraction ladder (RFC-010 §3.1): `Context`
(H1), `Concept` (H2 — the diff unit), `SubConcept` (H3, diffed at L2),
`Member` (H4+, emitted not diffed). Depth is authoritative — a heading's
role *is* its depth. `from_heading_depth` maps a markdown heading depth
to its rung; adapters call it rather than `match`-ing the enum, so
`#[non_exhaustive]` never forces a dead wildcard arm in adapter crates.
Marked `#[non_exhaustive]`. Lives in `domain`.

- verb: AbstractionLevel::from_heading_depth

## CohesionViolation

The abstraction-ladder cohesion violations (RFC-010 §3.5), wrapped by
[Violation](#violation)'s `Cohesion` arm so consumers that do not opt
into cohesion checking match one arm rather than three — distinct from
[ContextViolation](#contextviolation) (RFC-001 cross-context edges).
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

A concept heading explicitly bound to a named code item the concept walk
would not otherwise surface (RFC-012 §3.2) — a `pub(crate)` type, a `fn`,
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

## AnchorKind

The kind of code item an anchor resolved to (RFC-012 §3.4): `Type`, `Fn`,
or `Const` — the three the source-walk MVP resolves, each a `syn::Item`
the reader already visits. Enum-variant resolution is deferred to R12-6
(cfdb-query, where `kind:"variant"` is native), so the enum is
`#[non_exhaustive]` to admit it without a breaking change. Lives in
`domain`.

## AnchorTarget

A resolved anchor target — the code item an `AnchorResolver` found for a
qname, at any visibility (RFC-012 §3.4). A pure domain type by
construction: it carries no infrastructure representation (`syn::Item`,
cfdb `Node`/`PropValue`); the resolving adapter translates into this
shape, keeping the dependency arrow pointing inward. Pairs an
[AnchorKind](#anchorkind) with a [Source](#source). Lives in `domain`.

## ResolvedAnchor

A [ConceptAnchor](#conceptanchor) paired with its code-side resolution
verdict (RFC-012 §3.4) — `Some(`[AnchorTarget](#anchortarget)`)` when the
named item exists in code, `None` when it does not. Built by the
application (which resolves each target through the
[AnchorResolver](#anchorresolver) port) and handed to the diff, so the
diff engine stays pure and calls no resolver itself. An anchored concept
is exempt from `MissingInCode`; an unresolved anchor becomes
[Violation](#violation)'s `DanglingAnchor` arm. Lives in `domain`.

## AnchorResolver

The anchor-resolution port (RFC-012 §3.4) — a **separate** trait from
[CodeFacts](#codefacts) (ISP): not every code adapter resolves anchors, so
widening `CodeFacts` would force a deferred adapter to ship a stub. It
answers one question the concept walk does not: *does an item named
`qname` exist anywhere in the code, at **any** visibility?* — so a concept
whose canonical implementation is `pub(crate)` (or a `fn` / `const`) can be
a spec concept without a manufactured `pub` type. Resolution is consulted
only for the qnames an anchor references, so the global concept set is
unchanged. Lives in `ports`.
