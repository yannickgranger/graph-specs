---
title: RFC-005 — graph-specs report --verb-coverage subcommand
status: Ratified (4-lens round 2 RATIFY 2026-05-26; 1 author-documented override per upstream CLAUDE.md §2.3 on `report_verb_coverage` placement; dry-run code-verification pass folded; ready for implementation issue filing per upstream §2.4)
date: 2026-05-26
authors: agentry-captain-2026-05-26 (drafted after spec-first + code discovery; 4-lens council round 1 verdicts in §5; one author-documented override per upstream CLAUDE.md §2.3 recorded in §5.5)
companion: agentry EPIC #793 (consumer-side ratified RFC at agentry:docs/rfc/RFC-verb-coverage-harvest.md, 2026-05-22 4-lens council)
consumer-issue: agentry tracking issue #1143; upstream request issue #95
---

# RFC-005 — `graph-specs report --verb-coverage` subcommand

## §1 — Problem

`graph-specs check` enforces concept ⟺ `pub type` name equivalence (per `specs/dialect.md` "What the Rust reader parses": only `pub struct`, `pub enum`, `pub trait`, `pub type`). This is the **noun vaccine**: it inoculates the type vocabulary and is structurally blind to **verbs** — `pub fn`, FSM transitions, run-loops, data-flow.

The consumer-side evidence (agentry's ratified `RFC-verb-coverage-harvest.md` §0, council 4-lens 2026-05-22):

| crate | pub fn | pub type | LOC | fraction graph-specs sees |
|---|---|---|---|---|
| **orchestrator-runtime** | 144 | 49 | 16,331 | 49 / 193 |
| **agentry-role-runtime** | 123 | 28 | 10,476 | 28 / 151 |

agentry's `daemon.rs` is 2,643 LOC / 31 verb-fns / **0 pub type** — 100% invisible to the vaccine. Every repeat-offender outage in agentry (#574 FSM stall, #559 await-settled, #495b collapse, #748/#776 connection sharing) lived in unfenced verbs and shipped past green CI.

The consumer-side RFC §5 close condition is `grep + prose = 0` — measured by an **X-ray** that reports verb-coverage per bounded context. The X-ray must live in graph-specs (the sole spec parser; per the consumer RFC's clean-arch r2 resolution against pinned `cfdb 0.4.1`, cfdb's `:Concept` is materialized from `.cfdb/concepts/*.toml` — a Phase-A stub agentry has zero entries for — and cfdb's markdown scanner excludes `specs/` by design).

## §2 — Scope

In scope:

1. New read-only subcommand `graph-specs report --verb-coverage --specs <path> --code <path> [--format text|ndjson]`.
2. New `VerbReader` port trait in `ports/src/lib.rs` (sibling to `ContextReader`), implemented by `RustReader`. Mirrors the RFC-001 §3.6 precedent for adding new reader capabilities at the port layer.
3. Reuse the existing `MarkdownReader` AST walk infrastructure additively for invariant-annotation extraction (separate parser instance per file, see §3.2 / §3.3). The existing concept-level walks MUST NOT change shape.
4. Output three record kinds, **partitioned by bounded context** (using the v0.4 `ContextDecl` infrastructure from RFC-001):
   - **verb-coverage** — `pub fn` items in code that are not cited by any spec section in their owning context.
   - **tier histogram** — counts per `enforced-by:` annotation location, parsed from `[enforced-by: <artifact>; retire-when: <predicate>]` and `[prose-only: <why>]` bracketed annotations in spec `## Operational invariants` bullets. Tiers: `cypher` / `tier-0` / `script-fence` / `prose-only`. The histogram is **derived**, never asserted on the spec.
   - **homonym candidates** — pub-fn (or pub-type) names appearing in N contexts, **enriched with the sanctioning `ContextPattern`** (Shared Kernel / Published Language → sanctioned by doctrine; Conformist / Customer-Supplier → potential split-brain for council escalation per consumer RFC §4).
5. NDJSON output extends `specs/ndjson-output.md` v1 schema additively: new top-level `record` discriminator alongside the existing `violation` discriminator. Existing `violation` records carried unchanged. Consumers ignore unknown discriminators per existing schema-evolution rule.
6. Text format produces a tabular human-readable variant of the same data.
7. Exit code 0 on successful read; **never a gate**. Exit non-zero only on read/parse errors (mirrors `check` exit-2 semantics for reader errors).

Out of scope (§6 expands):

- `- verb:` bullet parsing as an **equivalence check** that BLOCKS the build. The reader collects the bullets; `report` lists discrepancies; `check` is unchanged. Adding verb-anchoring AS A GATE is the separate RFC-006 follow-up (= agentry issue #1145, B2 verb-anchoring).
- Modifying the existing `check` subcommand's behavior, exit codes, or NDJSON schema for existing records.
- Computing `[enforced-by:]`-tier validity (i.e., verifying that the cited artifact file actually exists) — that is the bijection meta-check, owned by the consumer (already live in agentry's `scripts/arch-check.sh` post agentry PR #1159).

## §3 — Design

### §3.1 — CLI surface

```
graph-specs report --verb-coverage --specs <path> --code <path> [--format text|ndjson]
```

The `report` subcommand will be added as a sibling to the existing `Check` variant in `application/src/main.rs`'s `Command` enum (currently a single-variant enum at lines 37-50; Slice B widens it). `--verb-coverage` is a flag (not a positional) so future report kinds (e.g. `--tier-histogram`-only) can be added without breaking the wire form.

### §3.2 — Port + reader extensions (additive)

**`VerbReader` port (new) — `ports/src/lib.rs`.** Sibling trait to `ContextReader`. Per the unanimous clean-arch + solid + rust-systems verdicts (§5.1 / §5.3 / §5.4), the new pub-fn extraction capability MUST live behind a port trait, not as an inherent impl on `RustReader`. This mirrors RFC-001 §3.6's introduction of `ContextReader` as a separate port:

```rust
pub trait VerbReader {
    /// Collect every top-level `pub fn` (signature + source) under `root`.
    /// Returns an empty Vec on adapters that do not extract verbs (no panic,
    /// no error). Independent of `Reader::extract` — never invoked by `check`.
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}
```

**`RustReader::extract_pub_fns` — `adapters/rust/src/lib.rs`.** Implements `VerbReader`. **Walk model (per rust-systems §5.4 blocker 1):** separate AST walk from `Reader::extract`. The current `extract` walks every `*.rs` file with `syn::parse_file`; `extract_pub_fns` re-parses the same files independently because (a) `Graph::nodes` (currently `Vec<ConceptNode>`) carries no fn arity/params/return-type, so it cannot be derived from the existing `Graph`; (b) the `report` subcommand is invoked deliberately, **never inside `check`**, so the doubled parse pays no cost on the `check` path. The existing `extract` is untouched.

**Implementation — separate walk function, NOT an extension of `visit_top_level_item`** (per dry-run verification rust-systems-A): the current `visit_top_level_item` (`adapters/rust/src/lib.rs:114-126`) is a closed match over `Item::Struct | Item::Enum | Item::Trait | Item::Type` with an explicit `_ => {}` catch-all whose comment names `Fn` as a deliberately-excluded item: *"All other items (Mod, Fn, Impl, Const, Static, Use, Macro, etc.) are not top-level concepts."* Extending that match to admit `Item::Fn` would contradict the documented invariant. Slice A MUST introduce a NEW parallel walk function `visit_top_level_fn` that exclusively handles `syn::Item::Fn`, driven from a sibling `for item in &file.items` loop. `visit_top_level_item` is not touched. The two walks share no state; they share only the file-iteration shape.

No new syn features required (current `syn = "2"` workspace pin with `["full", "parsing", "extra-traits"]` already supports `ItemFn.vis` + `ItemFn.sig.ident`; the workspace pin additionally lists `"visit"` but that feature is NOT exercised by the current manual `for item in file.items` walk model, and `extract_pub_fns` will NOT introduce a `syn::visit::Visit` impl — same manual walk shape).

**`MarkdownReader::extract_invariant_annotations` — `adapters/markdown/src/lib.rs`.** Inherent method (NOT on a new port trait — the markdown reader is the only adapter that parses markdown). **Parser model (per rust-systems §5.4 blocker 2 + dry-run rust-systems-C):** independent `Parser::new(source).into_offset_iter()` per file — a fresh `pulldown-cmark` parser instance with the offset iterator (mandatory for source-location attribution; `InvariantAnnotation.source` carries file:line, which the offset iter produces via `line_of_offset(range.start)` mirroring `adapters/markdown/src/lib.rs:145-169`). NOT a bare `Parser::new(source)` — that loses line numbers. NOT a shared event stream with the existing concept walk. The markdown source `&str` is already in memory (file-load cost paid once); the second parser allocation is the only cost. This is the RFC-001 §3.6 precedent (each marker-driven extraction owns its own parse) applied to invariant annotations.

**H4 heading recognition is structurally NEW (per dry-run rust-systems-D)** — the existing `handle_event` (`adapters/markdown/src/lib.rs:162-178`) matches only `HeadingLevel::H2 | HeadingLevel::H3` for concept dispatch (H4 falls through to the catch-all `_ => {}` arm at line 199, by deliberate design per `specs/dialect.md` "Level-1 and level-4+ headings"). The new `extract_invariant_annotations` parser loop introduces its own `HeadingLevel::H4` arm to recognize the `#### Operational invariants` section boundary. The existing `handle_event` MUST NOT be extended to recognize H4 — that would violate the dialect contract for concept dispatch.

The extractor scans `#### Operational invariants` sub-sections and parses each bullet for two bracketed annotation shapes:

- `[enforced-by: <artifact>; retire-when: <predicate>]` — real fence (tier derived from `<artifact>` location: `.cfdb/queries/*.cypher` → `Cypher`; `pub trait` / `pub fn` signature ref → `Tier0`; `scripts/*.sh` ref → `ScriptFence`)
- `[prose-only: <why>]` — explicit waiver (tier `ProseOnly`)

A new `InvariantAnnotation` domain type carries `inv_id`, `tier`, `artifact: Option<String>`, `retire_when: Option<String>`, `prose_only_why: Option<String>`, `source: Source`.

**Failure mode (per rust-systems §5.4 blocker 3):** tolerant-skip with diagnostic. A bullet line that LOOKS like an annotation (starts with `[enforced-by:` or `[prose-only:`) but fails to parse the bracket grammar emits a `tracing::warn!` with the file/line and the malformed text, and is dropped from the returned `Vec<InvariantAnnotation>`. The reader does NOT return `Err(_)` for grammar errors — `report` exit non-zero is reserved for I/O / fundamental parse failures (matching `Invariant 7` below). This honors the consumer RFC §4 "informational only — never a gate" stance: a malformed annotation should not fail the report.

The return type stays simple: `Result<Vec<InvariantAnnotation>, ReaderError>`. The `Err` arm is for I/O / pulldown-cmark catastrophic failure only.

**Both extensions are opt-in per spec.** A spec with no `#### Operational invariants` section yields an empty `Vec`. Equivalence-check behavior is unchanged.

**Bracket-grammar parser is new infrastructure (per dry-run rust-systems-E)** — the existing `parse_bullet_edge` (`adapters/markdown/src/lib.rs:242-255`) does prefix-matching on bullet text (`- implements:`, `- depends on:`, `- returns:`). The new annotation parser is structurally different: it scans `Event::Text` atoms inside `Event::Start(Tag::Item)` for embedded bracketed `[enforced-by:...]` / `[prose-only:...]` substrings, parses inner `key: value; key: value` fields, and emits an `InvariantAnnotation`. This is a new grammar parser, not additive prefix matching. Slice A scope includes the parser implementation.

**SDP coupling note (per solid §5.3 finding 4):** the bracketed-annotation grammar (`[enforced-by:...;retire-when:...]`, `[prose-only:...]`) is consumer-defined (agentry RFC §3). Embedding the grammar parser in `adapter-markdown` couples a low-level adapter crate to the consumer's policy grammar. The author's choice here is option (b) per the solid lens: **embed in `adapter-markdown` with explicit council sign-off on the coupling risk.** Rationale: (i) the agentry consumer RFC is ratified and the grammar is wire-stable (any future grammar change requires a consumer-side RFC bump — same discipline that protects `schema_version`); (ii) extracting the parser into `domain` (option a) would require `adapter-markdown` to expose raw bullet strings, which adds a stringly-typed boundary the type system cannot defend. Option (b) keeps the parser strongly typed at the boundary. Council ratification of this coupling is recorded in §5.3.

### §3.3 — Domain layer additions

`domain` gains seven new pure types and one pure function. Per the DDD lens §5.2 finding 2 (closing the open question), all new types belong in the existing `domain` crate, not a separate `domain-report` crate — they are pure value objects whose owning context is `equivalence`, the same context that owns `ConceptNode`, `Edge`, and `CheckInput`. Splitting them would create a dependency edge (`domain-report → domain`) that either inverts or duplicates types.

- `pub struct PubFnDecl { name: String, source: Source, owned_unit: Option<String> }` — code-side pub-fn fact.
- `pub struct InvariantAnnotation { inv_id: String, tier: TierKind, artifact: Option<String>, retire_when: Option<String>, prose_only_why: Option<String>, source: Source }` — spec-side annotation fact.
- `pub struct VerbCoverageRecord { context: Option<String>, pub_fn: PubFnDecl, cited: bool }` — `context: None` mirrors the parallel case in the equivalence check (per DDD §5.2 finding 4): the report-mode analog of `ContextViolation::MembershipUnknown` (`domain/src/context.rs:124-128`). A `None` context means the pub-fn lives in a crate not declared under any context's `Owns` block.
- `pub struct TierHistogramRecord { context: Option<String>, tier: TierKind, count: usize }`.
- `#[non_exhaustive] pub enum TierKind { Cypher, Tier0, ScriptFence, ProseOnly }` — **`#[non_exhaustive]` per solid §5.3 finding 3 and rust-systems §5.4 concurrence.** Mirrors `ContextPattern`'s `#[non_exhaustive]` (RFC-001 §3.7) for forward compatibility — RFC-006 may add `BehaviorTest`, etc.
- `pub struct HomonymRecord { name: String, contexts: Vec<HomonymAppearance> }` where `pub struct HomonymAppearance { context_name: String, sanctioned_by_pattern: Option<ContextPattern>, asymmetric: bool }`. Per DDD §5.2 finding 1: `sanctioned_by_pattern` is derived from the `CheckInput.contexts` `ContextImport`/`ContextExport` declarations. **Derivation algorithm (per dry-run DDD-B):** for context C and concept N, (i) prefer C's `ContextExport.pattern` if C exports N — exporting context is authoritative per Evans Ch. 14, the same export-centric framing RFC-001 cites for `ContextExport`; (ii) fall back to the importing context's `ContextImport.pattern` if no export exists; (iii) if both an export and an import exist for N in this context but with disagreeing patterns (the asymmetric-declaration case that RFC-001 §4 invariant 5 makes legal input), set `sanctioned_by_pattern` to the export's pattern AND set `asymmetric: true` to signal the disagreement to downstream consumers. `Some(PublishedLanguage)` or `Some(SharedKernel)` with `asymmetric: false` means the cross-context appearance is doctrine-sanctioned (no council attention warranted); `Some(Conformist)` / `Some(CustomerSupplier)` or `None` (undeclared) means the appearance is a potential split-brain (warrants council review). The NDJSON example in §3.4 reflects this enrichment.

  **New domain predicate `ContextPattern::is_doctrine_sanctioned() -> bool` (per dry-run DDD-C):** the report's sanctioned-vs-split-brain dispatch needs a predicate over `ContextPattern` that today's domain code does not expose. Slice A adds `pub fn is_doctrine_sanctioned(&self) -> bool` as an inherent method on `ContextPattern` returning `true` for `PublishedLanguage | SharedKernel`, `false` for `Conformist | CustomerSupplier` (the predicate is forward-compatible with `#[non_exhaustive]` because `ContextPattern` is already `#[non_exhaustive]` per RFC-001 §3.7 — new variants must return their own classification). Justification for `pub`: the report layer in `application` reads the predicate to drive the homonym record's split-brain flag (`pub` is the minimum visibility for this cross-crate use; no internal callers).
- `pub struct ReportOutput { verb_coverage: Vec<VerbCoverageRecord>, tier_histogram: Vec<TierHistogramRecord>, homonyms: Vec<HomonymRecord> }`.
- `pub fn report_verb_coverage(check_input: CheckInput, pub_fns: Vec<PubFnDecl>, annotations: Vec<InvariantAnnotation>) -> ReportOutput`.

**Placement of `report_verb_coverage` — author-documented override (per §5.5).** Clean-arch §5.1 and Solid §5.2 both REQUEST CHANGES to move this function from `domain` to `application`. The DDD §5.2 lens dissents, arguing precedent: `diff` (`domain/src/diff.rs`, re-exported at `domain/src/lib.rs:17`) is structurally identical — a pure function taking pre-materialized inputs (`CheckInput + Graph`) and returning pure outputs (`Vec<Violation>`), already RATIFIED in `domain` since the first equivalence check shipped. `report_verb_coverage` has the same shape: pre-materialized inputs in, pure records out, no I/O. The author records a single override per upstream `CLAUDE.md` §2.3 in §5.5 below: **`report_verb_coverage` STAYS in `domain`, parallel to `diff`.**

To address the clean-arch/solid concern that the function appears to "orchestrate" multiple reader outputs: §4 adds a **purity invariant (Invariant 8)** stating that `report_verb_coverage` MUST NOT invoke any reader and MUST receive pre-materialized inputs (DDD §5.2 finding 3). The orchestration that calls `VerbReader::extract_pub_fns` + `MarkdownReader::extract_invariant_annotations` + assembles `CheckInput` lives in `application/src/report.rs` (Slice B), mirroring how `application/src/lib.rs:run_check` orchestrates the equivalence check's readers around `domain::diff`.

### §3.4 — Output formats

Text format: a three-section human table per bounded context. Architects refine the exact column shape during round 2 review.

NDJSON format: new top-level discriminator `record` (sibling to existing `violation`). Schema:

```json
{"schema_version":"2","record":"verb_coverage","context":"reading","pub_fn":"MarkdownReader::new","cited":false,"source":{"kind":"code","path":"adapters/markdown/src/lib.rs","line":42}}
{"schema_version":"2","record":"tier_histogram","context":"reading","tier":"cypher","count":7}
{"schema_version":"2","record":"homonym","name":"Severity","contexts":[{"context":"validators","sanctioned_by_pattern":null,"asymmetric":false},{"context":"review","sanctioned_by_pattern":null,"asymmetric":false}]}
{"schema_version":"2","record":"homonym","name":"Graph","contexts":[{"context":"equivalence","sanctioned_by_pattern":"PublishedLanguage","asymmetric":false},{"context":"reading","sanctioned_by_pattern":"PublishedLanguage","asymmetric":false}]}
```

The two `homonym` examples illustrate the DDD §5.2 finding 1 distinction: the `Severity` case is a split-brain candidate (no sanctioning pattern — warrants council); the `Graph` case is doctrine-sanctioned (both contexts hold a `PublishedLanguage` export/import — no council needed).

Architects refine field names during round 2 review. `schema_version` stays at the existing value (`"2"`) because the new discriminator is additive (per `specs/ndjson-output.md` §Schema evolution rule).

## §4 — Invariants

1. **`check` behavior MUST NOT change.** `report` is purely additive. The existing four equivalence levels (concept, signature, edge, context) remain the only gating mechanism.
2. **Zero new spec parsers.** The Rust AST walk is extended once (one new `RustReader::extract_pub_fns` method behind the `VerbReader` port); no new tokenizer, no new file-discovery walker. The markdown annotation extractor reuses `pulldown-cmark` — same crate dependency, fresh `Parser::new(source)` instance per file (per rust-systems §5.4 blocker 2).
3. **Cross-dogfood is zero-false-positive.** Per RFC-002 §2.5: the new subcommand on cfdb's pinned tree must produce stable output. Round 2 architects prescribe the exact dogfood assertion.
4. **NDJSON schema_version handling extends additively.** New top-level `record` discriminator does NOT bump `schema_version`; existing `violation` records are unchanged. Consumers built against `schema_version: "2"` MUST ignore unknown top-level discriminators per the existing dispatch convention.
5. **No new violation variants.** `report` records are NOT violations — they are informational. The `violation` discriminator value-set is unchanged. The `report` subcommand exit code is 0 on success; non-zero only on read/parse errors (Invariant 7 below).
6. **Cross-fact locking covers wire-format discriminators.** Per solid §5.3 finding 5 and rust-systems §5.4 non-blocking 3: `cross-locked.json` (per agentry consumer RFC §8) locks the `record` discriminator value-set (`"verb_coverage"`, `"tier_histogram"`, `"homonym"`), the `TierKind` serialized variant strings, and the `HomonymAppearance` field shape. The `verb_coverage` record's `pub_fn` VALUES are NOT locked (they change with code edits); the `tier_histogram` and `homonym` record VALUES are NOT locked (they change with annotation edits). Only the SCHEMA is locked.
7. **Failure mode — tolerant-skip on annotation parse error.** Per rust-systems §5.4 blocker 3: a malformed bracketed annotation emits a `tracing::warn!` and is dropped from `extract_invariant_annotations`'s returned `Vec`. The reader returns `Ok(Vec<InvariantAnnotation>)`; `Err(ReaderError)` is reserved for I/O / fundamental parse failures only.
8. **`report_verb_coverage` is reader-free.** Per DDD §5.2 finding 3: the domain function MUST NOT invoke any reader. It receives pre-materialized `CheckInput + Vec<PubFnDecl> + Vec<InvariantAnnotation>` and returns `ReportOutput`. The orchestration that calls readers lives in `application/src/report.rs`.

## §5 — Architect lenses (round 1)

### §5.1 — Clean architecture

**REQUEST CHANGES** (round 1) — folded into round-2 revision below.

The clean-arch lens raised two BLOCKING findings:

1. `extract_pub_fns` must be a distinct port trait, not an inherent method on `RustReader`. **Resolved (§3.2):** `VerbReader` port added to `ports/src/lib.rs`, sibling to `ContextReader`. Mirrors RFC-001 §3.6 precedent verbatim.
2. `report_verb_coverage` belongs in `application`, not `domain`. **Contested fork — see §5.5 author override.** The DDD §5.2 lens dissents on this point citing the `diff` precedent (`domain/src/lib.rs:17`). The author override preserves domain placement and addresses the orchestration concern via the new Invariant 8 (reader-free purity).

Two NON-BLOCKING findings (confirmed correct in this revision):

3. `extract_invariant_annotations` lives in `adapters/markdown`, NOT a separate annotation adapter (preserves cohesion of the `reading` bounded context per RFC-001 §3.8).
4. New domain types live in the existing `domain` crate, NOT a `domain-report` crate (closed by DDD §5.2 finding 2).

Port-purity check (per lens evidence): `VerbReader::extract_pub_fns` → `Result<Vec<PubFnDecl>, ReaderError>` and `MarkdownReader::extract_invariant_annotations` → `Result<Vec<InvariantAnnotation>, ReaderError>` — both signatures use only domain/port types. No `syn`, `walkdir`, `pulldown-cmark`, `serde`, `tokio`, or other infrastructure type leaks. Port purity holds.

**ROUND 2 VERDICT (clean-arch): RATIFY.** Round-1 findings 1, 3, 4 RESOLVED. Author override on finding 2 ACCEPTED (the §5.5 softening removes the argument-from-silence; the `domain::diff` precedent at `domain/src/diff.rs:23` is verified live; Invariant 8 makes the purity contract load-bearing). Dry-run amendments verified honest against actual code. **One prescription folded into Slice A:** Invariant 8 (`report_verb_coverage` MUST NOT invoke any reader) MUST appear verbatim in Slice A's acceptance criteria, not just in §4. Without that, the purity contract is advisory; with it, the contract is enforceable.

### §5.2 — Domain-driven design

**REQUEST CHANGES** (round 1) — folded into round-2 revision below.

Four findings:

1. (BLOCKING) `HomonymRecord` must encode the sanctioning `ContextPattern` to honor the consumer RFC's "detect, never auto-resolve" guarantee. **Resolved (§3.3):** `HomonymRecord.contexts` is now `Vec<HomonymAppearance>` where `HomonymAppearance { context_name, sanctioned_by_pattern: Option<ContextPattern> }`. `Some(PublishedLanguage)` and `Some(SharedKernel)` mark doctrine-sanctioned cross-context appearances; `Some(Conformist)`, `Some(CustomerSupplier)`, and `None` mark split-brain candidates. The §3.4 NDJSON example illustrates the distinction.
2. (NON-BLOCKING) `InvariantAnnotation` and `PubFnDecl` belong in existing `domain` crate, not a separate `domain-report`. **Resolved (§3.3 lead paragraph):** explicitly cites the reasoning — the owning context is `equivalence`, splitting would create dependency edges that invert or duplicate.
3. (BLOCKING) `report_verb_coverage` placement in `domain` is correct (parallels `diff`), but the RFC must add an invariant that the function MUST NOT invoke any reader. **Resolved:** Invariant 8 added to §4.
4. (NON-BLOCKING) `VerbCoverageRecord.context: Option<String>` must cite the parallel case in `ContextViolation::MembershipUnknown`. **Resolved (§3.3):** explicit citation added — the `None` context is the report-mode analog of the check-mode `MembershipUnknown`.

The lens dissents from the clean-arch + solid recommendation to move `report_verb_coverage` to `application`. The author override in §5.5 records the resolution.

**ROUND 2 VERDICT (ddd): RATIFY.** All 4 round-1 findings RESOLVED + dry-run GAPs B (sanctioned-by derivation algorithm) and C (`is_doctrine_sanctioned()` predicate) RESOLVED. The exporter-wins algorithm (§3.3) is structurally consistent with RFC-001 §4 invariant 5 — `ContextExport`'s pattern is authoritative per Evans Ch. 14's export-centric framing. The `asymmetric: bool` flag preserves Export/Import disagreement signal without auto-resolving. The new `ContextPattern::is_doctrine_sanctioned()` predicate is correctly scoped (`PublishedLanguage | SharedKernel → true`; `Conformist | CustomerSupplier → false`); `pub` visibility minimum for cross-crate use; forward-compatible via existing `#[non_exhaustive]`. The amended `HomonymRecord` honors the consumer RFC's "detect, never auto-resolve" guarantee — `sanctioned_by_pattern: None` preserves ignorance honestly; `asymmetric: true` signals disagreement without choosing a winner.

### §5.3 — SOLID + component principles

**REQUEST CHANGES** (round 1) — folded into round-2 revision below.

Five findings:

1. (BLOCKING) `VerbReader` port trait. **Resolved (§3.2):** added.
2. (BLOCKING) `report_verb_coverage` to `application`. **Contested fork — see §5.5 author override.** The function stays in `domain`; the orchestration concern is addressed by Invariant 8.
3. (BLOCKING) `TierKind` needs `#[non_exhaustive]`. **Resolved (§3.3):** added.
4. (BLOCKING) SDP — annotation grammar coupling in `adapter-markdown`. **Resolved (§3.2 SDP coupling note):** option (b) selected — embed parser in `adapter-markdown` with explicit council sign-off; rationale (i) ratified consumer-side grammar is wire-stable; (ii) extracting to domain would force stringly-typed boundary. The SOLID lens is asked to RATIFY this coupling at round 2.
5. (BLOCKING) Invariant 6 for `cross-locked.json` scope. **Resolved (§4):** Invariant 6 added, distinguishing SCHEMA-locked fields from VALUE-volatile fields.

ADP / CCP checks per lens evidence: dependency direction `application → ports → domain` is preserved; CCP holds (the new pure types change together when the report schema changes). No ADP violation introduced.

**ROUND 2 VERDICT (solid): RATIFY.** Round-1 findings 1 (`VerbReader` port), 3 (`TierKind #[non_exhaustive]`), 5 (Invariant 6 `cross-locked.json` scope) RESOLVED. Finding 4 option (b) annotation-grammar coupling in `adapter-markdown` ACCEPTED with council sign-off — the consumer RFC governance protects wire-stability; the alternative (option a, extract to `domain`) would introduce a worse stringly-typed boundary; SDP direction is preserved (no new cross-crate arrow). Finding 2 author override ACCEPTED — the `domain::diff` SRP precedent is structurally sound; Invariant 8 closes the SRP drift risk. **One recorded note** (advisory, not blocking): if RFC-006 later adds verb-anchoring logic to `report_verb_coverage`, the SRP argument collapses and relocation to `application` becomes mandatory. That condition is not triggered by this RFC.

### §5.4 — Rust systems

**REQUEST CHANGES** (round 1) — folded into round-2 revision below.

Three BLOCKERS:

1. `extract_pub_fns` walk model clarity. **Resolved (§3.2 RustReader paragraph):** separate walk from `extract`, on-demand, pays own parse cost, never invoked by `check`. Explicit statement added.
2. `extract_invariant_annotations` parser model. **Resolved (§3.2 MarkdownReader paragraph):** fresh `Parser::new(source)` per file, NOT a shared event stream. RFC-001 §3.6 precedent cited.
3. Failure mode unspecified. **Resolved (§3.2 + Invariant 7):** tolerant-skip with `tracing::warn!`; return type stays `Result<Vec<InvariantAnnotation>, ReaderError>`; `Err` reserved for I/O / catastrophic parse.

Four non-blocking concerns (resolved or non-issues):

- syn version + `Item::Fn`: no feature flag changes needed (current pin is sufficient). Non-issue.
- `TierKind` `#[non_exhaustive]`: concurred with solid lens, **added (§3.3)**.
- `cross-locked.json` scope: lock the SCHEMA, not the values. **Reflected in Invariant 6.**
- Orphan rule / object safety / Cargo.toml churn: no violations; no new crates needed.

**ROUND 2 VERDICT (rust-systems): RATIFY.** All 3 round-1 BLOCKERS + all 5 dry-run code-contradictions RESOLVED. §3.2 RustReader paragraph correctly mandates the new parallel `visit_top_level_fn` walk and the no-touch of `visit_top_level_item` (verified at `adapters/rust/src/lib.rs:114-126`); correctly states the syn `"visit"` feature is pinned but not exercised. §3.2 MarkdownReader paragraph correctly mandates `Parser::new(source).into_offset_iter()` for line attribution; correctly mandates a separate H4 arm in the new parser loop (verified `handle_event` matches H2/H3 only at `adapters/markdown/src/lib.rs:162-178`); correctly characterizes the bracket-grammar parser as new infrastructure distinct from prefix-matching. Invariant 7 (tolerant-skip with `tracing::warn!`) is the right failure mode for an informational-only subcommand. Invariant 6's SCHEMA-locked / VALUE-volatile split is sound. No new rust-systems concerns introduced by the amendments — orphan rule, object safety, Cargo.toml churn all clean.

### §5.5 — Author override (per upstream CLAUDE.md §2.3)

Upstream CLAUDE.md §2.3 permits "a single author-documented override" when not all four lens verdicts are RATIFY. This RFC exercises that override exactly once:

**Override scope:** placement of `report_verb_coverage` in `domain` rather than `application`.

**Override rationale:**
- The clean-arch (§5.1 finding 2) and solid (§5.3 finding 2) lenses argue the function is application-layer orchestration. The DDD (§5.2 finding 3) lens argues it is a pure domain transformation parallel to `diff`.
- The decisive precedent is `domain::diff` (`domain/src/diff.rs:23`, re-exported `domain/src/lib.rs:17`), which takes pre-materialized `CheckInput + Graph` and returns `Vec<Violation>`. Verified signature: `pub fn diff(spec: CheckInput, code: Graph) -> Vec<Violation>`. The function has been in `domain` since the first equivalence check ratified. The clean-arch lens in RFC-001 did not adjudicate `diff`'s placement explicitly (the RFC-001 verdict ratified the overall design without per-function review), so this precedent is by-design-and-not-yet-contested rather than positively-ratified-by-the-lens — recorded honestly here per dry-run clean-arch finding. `report_verb_coverage` has structurally identical shape: pre-materialized inputs in, pure records out, no I/O, no reader calls.
- The clean-arch and solid concerns about "multi-reader orchestration" describe the CALLER, not the function. The caller — `application/src/report.rs` (Slice B) — IS in application. The function itself is a pure transformation.
- Invariant 8 (added per DDD §5.2 finding 3) makes the purity contract explicit: `report_verb_coverage` MUST NOT invoke any reader. This is the same invariant that protects `diff` from gaining reader calls.

**Override is bounded:** the override applies only to the placement of one function. All other clean-arch and solid findings are RESOLVED by edits, not overridden. The override is recorded inline here per the upstream §2.3 protocol; it does not consume the "one override" allowance for any future RFC in this repo.

### §5.6 — Dry-run code-contradiction findings (read-only verification, 2026-05-26)

After round 1 fold, three parallel read-only verifier sub-agents (rust-systems, clean-arch, ddd-specialist) dry-ran the revised draft against the actual graph-specs-rust code on disk. Findings drove 10 amendments folded into the revised §3 / §5.5 / §7 above. The verifier-discovered contradictions and their resolutions:

| # | Finding | Location | Resolution |
|---|---|---|---|
| 1 | `visit_top_level_item` cannot be extended — its `_ => {}` arm documents `Fn` as deliberately excluded | adapters/rust/src/lib.rs:121-123 | §3.2 RustReader paragraph: new parallel `visit_top_level_fn` walk function, untouched `visit_top_level_item` |
| 2 | `syn` `visit` feature is pinned but NOT exercised by current manual walk | Cargo.toml:20 vs adapters/rust/src/lib.rs:108-111 | §3.2 RustReader paragraph: removed `"visit"` from cited feature list; called out that the manual walk shape is preserved |
| 3 | `Parser::new(source)` loses line numbers without `.into_offset_iter()` | adapters/markdown/src/lib.rs:145 | §3.2 MarkdownReader paragraph: changed to `Parser::new(source).into_offset_iter()` with offset-attribution rationale |
| 4 | H4 headings are structurally invisible to existing `handle_event` (H2/H3-only) | adapters/markdown/src/lib.rs:162-178 + specs/dialect.md | §3.2 MarkdownReader paragraph: explicit statement that the new parser loop adds its own H4 arm AND the existing `handle_event` MUST NOT be extended |
| 5 | Bracket-grammar parser inside `Event::Text` is new infrastructure, not "additive bullet-recognition rules" | adapters/markdown/src/lib.rs:242-255 | §3.2 MarkdownReader paragraph: explicit statement that the bracket parser is new grammar, distinct from the prefix-matching `parse_bullet_edge` |
| 6 | Pin file is `.cfdb/cfdb.rev` (NOT `.cfdb/graph-specs.rev`) on this repo; agentry consumer has its own `.cfdb/graph-specs.rev` | .cfdb/ vs the RFC §8 cite | §8 + downstream cites: distinguish the upstream pin (`.cfdb/cfdb.rev`) from the consumer-side pin (`.cfdb/graph-specs.rev` in agentry) |
| 7 | Sanctioned-by derivation algorithm was unspecified — RFC-001 §4 invariant 5 makes Export/Import disagreement legal input | §3.3 (DDD verifier finding B) | §3.3: spelled out the exporter-wins algorithm with explicit `asymmetric: bool` flag for disagreement cases |
| 8 | `ContextPattern::is_doctrine_sanctioned()` predicate doesn't exist; report's sanctioned-vs-split-brain semantic needs new domain code | domain/src/context.rs (no such method) | §3.3 + §7 Slice A: new pure inherent method on `ContextPattern`; `pub fn`, justified for cross-crate report use |
| 9 | `report` subcommand "sits alongside `check`" was present-tense; current `Command` enum has only `Check` | application/src/main.rs:37-50 | §3.1: future tense — "will be added as a sibling" |
| 10 | `clean-arch lens has never contested [diff's] placement` is argument-from-silence (RFC-001 ratified the design without per-function review) | §5.5 author override | §5.5: softened to "by-design-and-not-yet-contested rather than positively-ratified-by-the-lens" |

Two clean-arch verifier sub-findings were FALSE POSITIVES (the original RFC was correct):

- `MembershipUnknown` line range cited as `124-128` IS correct (verified directly: variant body opens at 124, closes at `},` on 128). The clean-arch verifier had counted up to the `,` on 127. RFC unchanged.
- `## Schema evolution` section cited in `specs/ndjson-output.md` DOES exist (at line 179). The clean-arch verifier missed it on first read. RFC unchanged.

No CONTRADICTION-class findings remain unresolved. The two confirmed-by-DDD findings (sanctioned-by algorithm, predicate) are GAPS resolved by adding work to Slice A's scope (new derivation algorithm + new `is_doctrine_sanctioned()` predicate) — both folded above.

### §5.7 — Round 2 ask

Round 2 review: each lens re-reads §3 (revised twice — once for round 1 fold, once for dry-run amendments) + §4 + §5.5 (override) + §5.6 (dry-run findings folded). The ask is:
- Clean-arch: confirm round-1 findings 1, 3, 4 are RESOLVED; ratify or reject the author override on finding 2 (now softened per dry-run); confirm the dry-run-driven future-tense fix in §3.1 + the cite distinction in §8.
- DDD: confirm round-1 findings 1-4 are RESOLVED; confirm the new sanctioned-by derivation algorithm in §3.3 and the new `ContextPattern::is_doctrine_sanctioned()` predicate satisfy findings B + C from the dry-run.
- Solid: confirm findings 1, 3, 5 RESOLVED; ratify or reject the override on finding 2; **ratify the option-(b) annotation-grammar coupling on finding 4.**
- Rust-systems: confirm all three round-1 BLOCKERS RESOLVED; confirm dry-run findings A (visit_top_level_item separation), C (offset-iter), D (H4 arm), E (bracket parser as new infra) are correctly reflected in §3.2.

Round 2 verdicts are appended inline to §5.1–§5.4 above. RFC ratifies when all four lenses' round 2 verdicts read RATIFY (with the author override in §5.5 standing as the one-allowed exception on the contested fork).

## §6 — Non-goals

- `- verb:` bullet parsing as an **equivalence gate**. The reader collects the bullets (so `report` can list discrepancies); enforcing them as a `check` gate is RFC-006 follow-up (= agentry issue #1145, B2 verb-anchoring).
- Making verb-coverage a build gate. Per consumer RFC §4: "informational only — not a gate, not a baseline (nothing to bump; numbers trend to zero)."
- Altering the existing concept ⟺ `pub type` equivalence check.
- Modifying `specs/ndjson-output.md` v2 schema for existing `violation` records.
- A standalone X-ray binary, or a bash re-parse of `specs/` in any consumer's CI script.
- The B1 cfdb `:CallSite` receiver-type — separate cfdb-side RFC (= agentry issue #1144).
- A separate `domain-report` crate — closed per DDD §5.2 finding 2.
- Sibling-tool changes (cfdb extraction, agentry adapters) — those are downstream of this RFC, not in scope.

## §7 — Issue decomposition

Two vertical slices, each with a `Tests:` block per upstream `CLAUDE.md` §2.5. Round 2 architects prescribe the exact test surface; round 1 already converged on the minimum dogfood requirements.

### Slice A — port + domain types + reader extensions

**Scope:** the new `VerbReader` port in `ports/src/lib.rs`; the new domain types (`PubFnDecl`, `InvariantAnnotation`, `VerbCoverageRecord`, `TierHistogramRecord`, `TierKind` with `#[non_exhaustive]`, `HomonymRecord`, `HomonymAppearance` with `asymmetric: bool`, `ReportOutput`); the new domain predicate `ContextPattern::is_doctrine_sanctioned()` (per dry-run DDD-C); the pure `report_verb_coverage` function in `domain`; `RustReader: VerbReader` impl via a NEW `visit_top_level_fn` walk function (per dry-run rust-systems-A — `visit_top_level_item` is NOT touched); `MarkdownReader::extract_invariant_annotations` inherent method (NEW parser loop with its own H4 heading arm + bracket-grammar parser for `Event::Text` atoms, per dry-run rust-systems-D and rust-systems-E).

**Tests** (round 2 prescriptions; clean-arch added the Invariant-8 acceptance requirement):
- **Acceptance (clean-arch round 2 prescription):** Slice A's CI run MUST include a static-analysis or grep-based assertion that `report_verb_coverage`'s body contains zero calls to any `Reader::*`, `ContextReader::*`, `VerbReader::*`, or `MarkdownReader::extract_invariant_annotations` method. Invariant 8 (reader-free purity) is otherwise advisory; this assertion makes it enforceable. Without it, the SRP override in §5.5 loses its load-bearing safeguard.
- Unit: assertions on `report_verb_coverage` over crafted inputs covering (a) verb-coverage with `None` context; (b) tier histogram with all four `TierKind` values; (c) homonym with sanctioned vs unsanctioned appearances; (d) the exporter-wins-with-asymmetric-flag derivation algorithm on RFC-001 §4 invariant 5 inputs (Export/Import pattern disagreement).
- Self dogfood (graph-specs on graph-specs): `extract_pub_fns` on this repo's `application/` crate yields a non-zero list including `application::run_check`.
- Cross dogfood (graph-specs on cfdb at pinned SHA): `extract_invariant_annotations` returns `Ok(_)` on cfdb's `specs/concepts/`.
- Target dogfood (on agentry at pinned SHA): `extract_invariant_annotations` recognizes all 12 INV-brief_lifecycle-* / INV-brief_state_stream-* anchors landed by agentry PR #1146 / #1159, parsing tier correctly (2× `Cypher` + 1× `Tier0`-ish [graph-specs L1] + 9× `ProseOnly`).

### Slice B — `report` CLI subcommand + output formats + dogfood assertion

**Scope:** `application/src/main.rs` gains the `Report` subcommand; new `application/src/report.rs` orchestrates `VerbReader::extract_pub_fns` + `MarkdownReader::extract_invariant_annotations` + `MarkdownReader::extract` + `MarkdownReader::extract_contexts` → assembles `CheckInput` → calls `report_verb_coverage` → emits via new `application/src/report_text.rs` and `application/src/report_ndjson.rs` emitters. CI step added per upstream CLAUDE.md §3.

**Tests** (minimum prescribed by round 1):
- Unit: text + NDJSON emitter assertions on a crafted `ReportOutput`.
- Self dogfood: `graph-specs report --verb-coverage --specs specs/ --code .` exits 0 and emits a deterministic record set.
- Cross dogfood: report on cfdb's pinned tree exits 0; assert on a stable verb-coverage count.
- Target dogfood: report on agentry's pinned tree includes the 12 INV anchors in the tier histogram with their declared tiers; the homonym list contains the `Severity` / `Finding` split-brains the agentry council flagged (XC-01 / XC-02 per agentry RFC §7.1).

## §8 — Companion consumer

agentry's `scripts/arch-check.sh` will gain a new step running `graph-specs report --verb-coverage` informationally after the existing `==> graph-specs check` step, once this RFC ratifies and lands as `.cfdb/cfdb.rev (and the agentry consumer's `.cfdb/graph-specs.rev`)` bump. Per consumer RFC §6.3: "Fences are CI-time on a fresh extract — never consulted from the daemon's stale `#620` intake cache."

Consumer-side `retire-when:` predicates that fire when this RFC's implementation lands:
- (none directly — this RFC is purely additive informational tooling)
- The companion RFC-006 (B2 verb-anchoring) IS named in 3 agentry anchor `retire-when:` predicates (`INV-brief_lifecycle-late-event-no-op-warn`, `INV-brief_state_stream-crash-recovery-from-cursor`, and the consumer of `## EventSource` L2-verb anchoring). Those convert atomically when RFC-006 ratifies and ships.

## §9 — Cross-references

- Consumer-side ratified RFC: `agentry:docs/rfc/RFC-verb-coverage-harvest.md` (4-lens council, 2026-05-22).
- Consumer EPIC: https://agency.lab:3000/yg/agentry/issues/793 .
- Consumer tracking issue: https://agency.lab:3000/yg/agentry/issues/1143 .
- Upstream RFC request issue: https://agency.lab:3000/yg/graph-specs-rust/issues/95 .
- Sibling RFC-006 (verb-anchoring) request: https://agency.lab:3000/yg/graph-specs-rust/issues/96 ; sibling cfdb `:CallSite` RFC request: https://agency.lab:3000/yg/cfdb/issues/441 .
- RFC-001 (bounded-context equivalence) — context-partitioning vocabulary + `ContextReader`-as-separate-port precedent reused here.
- RFC-002 (cross-dogfood with cfdb) — `Tests:` discipline + `cross-locked.json` cross-fact locking.
- `specs/dialect.md` — markdown + Rust reader rules (extended additively here).
- `specs/ndjson-output.md` v2 — wire-contract this RFC extends with new `record` discriminator.
