# RFC-006 — graph-specs opt-in verb-anchoring (`- verb:` bullet)

**Status:** Ratified 2026-05-26
**Date:** 2026-05-26
**Companion:** agentry EPIC #793 (consumer-side ratified RFC at agentry:docs/rfc/RFC-verb-coverage-harvest.md, 2026-05-22 4-lens council); upstream sibling RFC-005 (verb-coverage report)
**Consumer issue:** agentry tracking issue #1145; upstream request issue #96

## §1 — Problem

RFC-005 adds an informational `graph-specs report --verb-coverage` subcommand that computes `code_fns − cited_verbs` per bounded context. To compute `cited_verbs`, graph-specs must KNOW which `pub fn` items each spec section claims to own. Today there is no spec-side syntax for that — `specs/dialect.md` recognizes only the three bullet prefixes `- implements:`, `- depends on:`, `- returns:` (`adapters/markdown/src/lib.rs:233-237`), all of which model **type-to-type** relationships. There is no syntax for a **concept-to-function** anchor.

Without this RFC, the report's `cited_verbs` is always empty — every code `pub fn` shows up as "uncited," producing noise instead of signal.

The consumer-side RFC §8 (`agentry:docs/rfc/RFC-verb-coverage-harvest.md`) describes this as **B2 cross-dogfood**. Three agentry INV-* anchors carry the explicit `retire-when: B2-verb-anchoring-lands (#1145)` predicate: `INV-brief_lifecycle-late-event-no-op-warn`, `INV-brief_state_stream-crash-recovery-from-cursor`, and the consumer of `## EventSource` L2-verb anchoring. When this RFC's implementation lands and the consumer bumps `.cfdb/graph-specs.rev`, those three anchors atomically convert from prose-only waivers to real graph-specs L2-verb fences.

## §2 — Scope

In scope:

1. New opt-in bullet prefix `- verb: <bare-ident>` recognized by the markdown reader, parsed via a **separate handler** (NOT through `BULLET_PREFIXES` / `parse_bullet_edge`) — verbs are NOT edges; the type system enforces the distinction.
2. New L2 equivalence dimension: **concept-to-fn anchoring** living in a **new `VerbOwnership` aggregate carried by `CheckInput`** — NOT inside `Graph`. `Graph` remains the type-equivalence Published Language; `VerbOwnership` is the new behavioral-ownership aggregate. The Rust reader is extended to also collect `pub fn` items via the `VerbReader` port trait introduced by sibling RFC-005 (both CHECK and REPORT go through the same port; no inherent method).
3. Three new violation variants in the NDJSON wire schema (additive, no `schema_version` bump per `specs/ndjson-output.md:179`):
   - `verb_missing_in_code` — spec declares `- verb: X` but no `pub fn X` exists in code.
   - `verb_missing_in_spec` — code declares a `pub fn X` whose owning context has at least one `- verb:` bullet for some other verb, but no concept in that context declares `- verb: X`. **Opt-in per concept**: a concept with no `- verb:` bullets is not inspected at verb level.
   - `verb_target_unknown` — spec `- verb:` bullet names a `bare-ident` that does not appear in any code-side `pub fn` list **within the same context's owned units** (cross-context qname matches are silently routed here per §6, until a future cross-context-verb RFC).
4. Opt-in per spec, per concept: a verb-less spec stays L1-only; a concept with no `- verb:` bullets in an otherwise verb-anchored spec is L1-only at the concept level.
5. Slice A qname syntax is **bare fn identifiers only** (option (a) selected). Full module-path qname resolution is explicitly deferred to a future RFC; the existing per-file top-level walker has no module-path information and Slice A does not extend it.

Out of scope (§6 expands):

- The verb-coverage REPORT subcommand — separate RFC-005 (sibling). RFC-005 reads the same `- verb:` bullets RFC-006 introduces to compute the report.
- Function-signature-level equivalence (param types, return types, generic params). Future RFC.
- Trait-method anchoring across `impl Trait for Type` blocks. Future RFC.
- Cross-context `- verb:` references — currently emit `verb_target_unknown`. Future RFC adds `verb_cross_context_unauthorized` semantics.
- Const / static / macro_rules! anchoring.
- Full module-path qname resolution. Slice A uses bare identifiers; spec `- verb: handle_brief` matches code `pub fn handle_brief` within the concept's owning context's owned units.

## §3 — Design

### §3.0 — Implementation dependency on RFC-005 (per dry-run rust-systems-F + clean-arch-A)

RFC-006 references `VerbReader::extract_pub_fns`, `PubFnDecl`, and the `walk_pub_fns` private helper as if they exist; they DO NOT — they are RFC-005 Slice A deliverables. RFC-006 Slice A CANNOT merge before RFC-005 Slice A lands on upstream `develop`. The two RFCs are sequenced: RFC-005 merges → RFC-006 rebases on RFC-005's port + types → RFC-006 Slice A merges. The single-PR alternative is a combined RFC-005+006 Slice A — author preference is the sequenced shape (smaller PRs, separately reviewable), but operators MAY combine if landing speed matters.

### §3.1 — Spec-side syntax (markdown extension — separate handler, NOT BULLET_PREFIXES)

`BULLET_PREFIXES: &[(&str, EdgeKind)]` at `adapters/markdown/src/lib.rs:233-237` is typed to produce `Edge`s. Routing `- verb:` bullets through it would either corrupt `EdgeKind` (which is a closed type-to-type relationship enum at `domain/src/lib.rs:144-152`) or produce spurious `Edge` records in the graph.

The correct shape: a **new parallel handler `parse_verb_bullet(text: &str) -> Option<VerbAnchor>`** in `adapters/markdown/src/lib.rs` that never touches `EdgeKind` or `BULLET_PREFIXES`. The handler:

- Trims the bullet text.
- Strips the literal prefix `"verb:"`.
- Trims the remaining target. Empty target → `None` (mirrors `parse_bullet_edge` empty-target behavior).
- Slice A grammar: the target MUST be a bare identifier (regex `^[A-Za-z_][A-Za-z0-9_]*$`). Any whitespace, `::`, or special character → `None` with `tracing::warn!` (tolerant-skip; future RFC widens to qnames).
- Returns `VerbAnchor { qname: trimmed_ident.to_string(), raw_target: <verbatim>, source: Source::Spec { ... } }`.

The markdown walk's bullet dispatch becomes two parallel calls per `Event::Text` inside a `Tag::Item`: first try `parse_bullet_edge` (existing edge prefixes); if that returns `None`, try `parse_verb_bullet`; if both return `None`, the bullet is prose. The two collection paths populate `Vec<Edge>` and `Vec<VerbAnchor>` independently — `Vec<VerbAnchor>` is a new field on the spec-side reader output.

The dialect spec (`specs/dialect.md`) gains a new section under "## What the markdown reader parses": "Bullets with the prefix `- verb: <bare-ident>` — concept-to-fn anchoring (v0.5; bare-identifier only in Slice A; full qnames deferred to a future RFC)."

### §3.2 — Code-side extension (Rust adapter — port-only, no inherent method)

RFC-005 ratified `VerbReader::extract_pub_fns` as a port trait for the REPORT path. RFC-006 reuses the same port for the CHECK path — NO new inherent method on `RustReader`.

The shared private helper `walk_pub_fns(file: &syn::File) -> impl Iterator<Item = (syn::Ident, Span)>` in `adapters/rust/src/lib.rs` is called from the single `impl VerbReader for RustReader { fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError> { ... } }` impl. The CHECK path acquires `Vec<PubFnDecl>` from the port and converts to `Vec<VerbDecl>` at the application-layer composition root (mapping is one-to-one for Slice A — `PubFnDecl { name, source, owned_unit }` → `VerbDecl { qname: name, owned_unit, source }`).

Why two types instead of unifying — `PubFnDecl` is the REPORT-path raw fact (includes the `owned_unit` for histogram partitioning); `VerbDecl` is the CHECK-path matching-key (qname + source for diff matching). They diverge in future RFCs (REPORT may add fn-arity counts, CHECK may add signature-level fields). Slice A keeps them as separate types with a documented `From<PubFnDecl> for VerbDecl` impl living in the application layer (the layer that owns the orchestration).

The new walk function `visit_top_level_fn` is a **separate parallel function** (NOT extending `visit_top_level_item` at `adapters/rust/src/lib.rs:114-126`). Both are driven from sibling `for item in &file.items` loops in `extract_from_file`, each handling its own `Item::*` variants.

**Slice A qname construction (option (a) selected):** the `VerbDecl.qname` is the bare fn identifier (`f.sig.ident.to_string()`). NO module path prefix in Slice A. The spec `- verb:` syntax is correspondingly restricted to bare identifiers (see §3.1). Full module-path qname construction requires walking `pub mod foo { ... }` blocks (not currently supported by the per-file top-level walker) AND a deterministic file-path → module-segment derivation algorithm. Both are explicit non-goals (§6); a future RFC widens the grammar when the walker gains module-path resolution.

### §3.3 — Domain types — new `VerbOwnership` aggregate (NOT in `Graph`)

`VerbAnchor` and `VerbDecl` belong to a NEW aggregate `VerbOwnership`, NOT to `Graph`. `Graph` is the Published Language of the `equivalence` context's type-level equivalence; verb ownership is a categorically distinct concept ("a concept claims this fn") that must not corrupt `Graph`'s bounded context.

`domain` gains:

- `pub struct VerbDecl { qname: String, owned_unit: Option<String>, source: Source }` — code-side fact. `owned_unit` resolved via existing `ContextDecl.owned_units` matching.
- `pub struct VerbAnchor { concept: String, qname: String, raw_target: String, source: Source }` — spec-side fact. `raw_target` preserves the verbatim bullet text for display; `qname` is the tokenized match key (same SRP shape as `Edge`).
- `#[derive(Debug, Default, Clone, PartialEq, Eq)] pub struct VerbOwnership { pub decls: Vec<VerbDecl>, pub anchors: Vec<VerbAnchor> }` — new aggregate. **`Default` derive required** so that `CheckInput`'s existing `#[derive(Default)]` continues to compile.
- `CheckInput` (verified at `domain/src/context.rs:174-177`) gains a new field: `pub verb_ownership: VerbOwnership`. **Migration plan:** `CheckInput::new` (`domain/src/context.rs:183`, `const fn`) gains the new field as a third positional arg; ALL existing callers update in the same PR (audit: `application/src/lib.rs:37` is the sole production caller; the inline test at `context.rs:393` and any other test calls update in lockstep). For ergonomic addition, Slice A also adds `pub const fn with_graph_and_contexts(graph: Graph, contexts: Vec<ContextDecl>) -> Self` constructor that defaults `verb_ownership` to empty — preserves the v0.4 call-site shape for callers not yet adopting verb-anchoring. An empty `VerbOwnership` reduces v0.5 verb-pass behavior to v0.4 (verb pass is a no-op when no `- verb:` bullets exist anywhere).
- Three new `Violation` enum variants (extending the existing `Violation` enum in `domain/src/lib.rs:182-244`):
  - `Violation::VerbMissingInCode { concept: String, qname: String, spec_source: Source }`
  - `Violation::VerbMissingInSpec { qname: String, code_source: Source }`
  - `Violation::VerbTargetUnknown { concept: String, qname: String, spec_source: Source }` — fires ONLY when the qname exists in NO context. Cross-context cases go through `ContextViolation::CrossVerbUnauthorized` below.
- One new `ContextViolation` variant (extending the existing `#[non_exhaustive]` enum at `domain/src/context.rs:120-138`):
  - `ContextViolation::CrossVerbUnauthorized { concept: String, qname: String, owning_context: String, target_context: String, spec_source: Source }` — parity with `CrossEdgeUnauthorized`. Routed via the existing `Violation::Context(ContextViolation)` wrapper at `domain/src/lib.rs:243`.

The `Violation` enum becomes `#[non_exhaustive]` in Slice A. **Blast-radius audit.** workspace grep shows TWO sites that exhaustively `match` on `Violation`: `application/src/text.rs:18` and `application/src/ndjson.rs:42`. Both live in the same workspace as the `domain` crate; both are updated in the SAME Slice A PR — NOT split across Slice A and Slice B — to avoid the inter-slice compile-safety gap: once `Violation` becomes `#[non_exhaustive]` the compiler no longer enforces exhaustive match, so adding the variants + the `#[non_exhaustive]` attribute + the new arms at both emitter sites MUST land atomically. External consumers: agentry uses the CLI/NDJSON wire only (zero `use domain::` imports in agentry's tree, verified via `grep -rE 'use .*::Violation|use domain::' /var/mnt/workspaces/agentry/crates/` returning empty). qbot-core (per RFC-002) consumes the NDJSON wire. **External coordination required: none.** The schema_version invariant holds (`"2"` stays; new variants are additive per `specs/ndjson-output.md:179-186`).

### §3.4 — Diff extension — fourth pass operating on `CheckInput.verb_ownership`

`domain::diff(spec: CheckInput, code: Graph) -> Vec<Violation>` (verified signature at `domain/src/diff.rs:23`) gains a fourth pass after the existing concept / signature / edge passes. `VerbDecl`s arrive via `spec.verb_ownership.decls` (the application-layer composition root pre-populates `CheckInput.verb_ownership.decls` by calling `VerbReader::extract_pub_fns` and mapping `PubFnDecl → VerbDecl`). The `diff` signature does NOT gain a new parameter. The `code: Graph` parameter is untouched; `VerbOwnership` is carried entirely inside `CheckInput`.

The pass:

- Reads `spec.verb_ownership.anchors` (spec-declared) and `spec.verb_ownership.decls` (code-declared, pre-loaded by app layer).
- **Concept→context membership lookup:** `ContextDecl` does NOT carry a `concepts: Vec<String>` field. The lookup is a two-hop join: (i) find the `ConceptNode` for the anchor's `concept` in `spec.graph.nodes` to get its `Source::Code { path, .. }`; (ii) find the `ContextDecl` whose `owned_units: Vec<OwnedUnit>` contains a prefix of that path. For spec-side concepts whose source is `Source::Spec`, this hop falls through (specs don't have an owned_unit; the membership is via the spec-file's path prefix matching). The two-hop algorithm is documented as the canonical lookup for ALL context-membership queries — Slice A adds a `pub fn context_for_concept(graph: &Graph, contexts: &[ContextDecl], concept_name: &str) -> Option<&ContextDecl>` helper in `domain::context` to encapsulate it (called from the new diff pass and available for future passes).
- **`VerbDecl.owned_unit` membership:** for each `VerbDecl`, walk `spec.contexts`; the context whose `owned_units` contains a path-prefix matching `VerbDecl.owned_unit` (or whose `OwnedUnit(String)` matches when `owned_unit` is `Some`) owns this fn. A `VerbDecl.owned_unit == None` (no path information) is treated as orphaned — emits `verb_missing_in_spec` if any context has verb-anchored concepts, never matches an anchor (a `None` cannot prove same-context membership).
- If the anchor's concept and the matching `VerbDecl` are in the SAME context → match → no violation.
- If the qname exists in NO context → `verb_target_unknown`.
- **If the qname exists ONLY in a DIFFERENT context → `Violation::Context(ContextViolation::CrossVerbUnauthorized { ... })` (Invariant 8):** parity with how `cross_context_edge_unauthorized` handles the analogous case for edges (`domain/src/lib.rs:243` + `domain/src/context.rs:130-138`). The new `ContextViolation::CrossVerbUnauthorized` variant is added in Slice A (`ContextViolation` is already `#[non_exhaustive]` per RFC-001 §3.2 so this is non-breaking).
- For each `VerbDecl` in a context whose anchors exist (opt-in): if no anchor claims this qname → `verb_missing_in_spec`.

The pass is **opt-in per concept** AND **opt-in per context**: a context with zero `- verb:` anchored concepts is not inspected at all.

### §3.5 — NDJSON wire format

Three new records (additive; `schema_version` stays at `"2"`):

```json
{"schema_version":"2","violation":"verb_missing_in_code","concept":"BriefLifecycle","qname":"is_late_event","spec_source":{"kind":"spec","path":"specs/concepts/brief_lifecycle.md","line":42}}
{"schema_version":"2","violation":"verb_missing_in_spec","qname":"handle_brief","code_source":{"kind":"code","path":"src/lifecycle.rs","line":17}}
{"schema_version":"2","violation":"verb_target_unknown","concept":"BriefLifecycle","qname":"phantom_fn","spec_source":{"kind":"spec","path":"specs/concepts/brief_lifecycle.md","line":50}}
```

Per `specs/ndjson-output.md:182-186` "Adding a new variant to the `violation` enum" is non-breaking. Consumers built against `schema_version: "2"` MUST ignore unknown violation discriminators.

## §4 — Invariants

1. **`check` exit codes unchanged.** New verb violations contribute to exit code 1 (one or more violations). They do NOT trigger exit code 2.
2. **Opt-in per spec and per concept.** A spec with no `- verb:` bullets, OR a concept with no `- verb:` bullets within a verb-anchored spec, is L1-only at the verb level.
3. **Concept-to-fn name-only anchoring.** The match is between spec `qname` (bare ident) and code `f.sig.ident.to_string()`. Signature-level verb equivalence is out of scope (§6).
4. **Per-concept ownership; hybrid opt-in for `verb_missing_in_spec` (amended 2026-05-27 per RFC-008).** A `pub fn` is "owned" by a concept iff the concept declares `- verb: X` matching the fn's qname. Activation of `verb_missing_in_spec` for an unowned `pub fn` depends on the qname grammar:
   - `Type::method` qname (impl method, per RFC-007): the decl is inspected iff a concept `## Type` exists IN THE DECL'S OWN BOUNDED CONTEXT with at least one `- verb:` bullet. Per-concept opt-in; context-scoped to prevent cross-context type-name homonyms from falsely firing.
   - Bare-identifier qname (top-level free `pub fn`): the decl is inspected iff its owning bounded context has at least one opt-in concept (the original per-context activation, preserved). Free fns have no Type root and no natural concept-scoped owner; coverage stays a context-level concern.

   See RFC-008 §3.1 for the implementation predicate. The amendment is bounded — only Invariant 4 changes; Invariant 2 stays accurate (the hybrid preserves its spec promise for free-fn-heavy contexts).
5. **Zero new spec parsers.** The new bullet prefix uses the existing `pulldown-cmark` walk via a **separate `parse_verb_bullet` handler** (NOT extending `BULLET_PREFIXES`/`parse_bullet_edge`). The new code-side walk extends `RustReader` with `visit_top_level_fn` parallel to `visit_top_level_item` (RFC-005 precedent: NOT extending the existing match arm).
6. **`Violation` becomes `#[non_exhaustive]`.** One-time OCP-correct shape; blast radius is two same-crate sites (`application/src/text.rs:18` + `application/src/ndjson.rs:42`); **both updated in Slice A** (per §3.3 atomicity rule — `#[non_exhaustive]` + new variants + emitter arms all land in Slice A to close the inter-slice compile-safety gap). External consumers (agentry, qbot-core) consume the NDJSON wire — not the Rust type — so are unaffected. The schema_version `"2"` invariant holds.
7. **Cross-fact locking covers new violation variants.** Per RFC-002 §3: the three new discriminator strings (`verb_missing_in_code`, `verb_missing_in_spec`, `verb_target_unknown`) are SCHEMA-locked in `cross-locked.json`. Values (concept names, qnames) are NOT locked.
8. **Context-local verb anchoring.** A `- verb:` anchor is context-local: the anchoring concept and the matching `pub fn` MUST belong to the same `ContextDecl.owned_units`. Routing of failures per §3.4: qname exists in NO context → `Violation::VerbTargetUnknown`; qname exists in a DIFFERENT context → `Violation::Context(ContextViolation::CrossVerbUnauthorized)` (NOT silent pass, NOT auto-resolution; parity with `CrossEdgeUnauthorized`). The future-RFC item is broader cross-context-verb semantics (e.g., declared `verb-imports`), not the basic cross-context detection that ships in Slice A.
9. **CHECK and REPORT share `VerbReader::extract_pub_fns`.** No inherent method on `RustReader` for pub-fn extraction. The application-layer composition root acquires `Vec<PubFnDecl>` from the port and maps to `Vec<VerbDecl>` via a documented `From<PubFnDecl> for VerbDecl` impl living in `application/src/`. The shared private helper `walk_pub_fns` inside `adapters/rust/src/lib.rs` is called only from `impl VerbReader for RustReader`.

## §5 — Architect lenses (round 1 verdicts folded)

### §5.1 — Clean architecture

**ROUND 2 VERDICT (clean-arch): RATIFY.**

### §5.2 — Domain-driven design

**ROUND 2 VERDICT (ddd): RATIFY.**

### §5.3 — SOLID + component principles

**ROUND 2 VERDICT (solid): RATIFY.**

### §5.4 — Rust systems

**ROUND 2 VERDICT (rust-systems): RATIFY.**

## §6 — Non-goals

- Signature-level verb equivalence (param types, return types, generic params). Future RFC.
- Trait-method anchoring across `impl Trait for Type` blocks. Future RFC.
- Cross-context `- verb:` references — currently emit `verb_target_unknown` (Invariant 8); future RFC adds `verb_cross_context_unauthorized` semantics.
- Const / static / macro_rules! anchoring.
- Full module-path qname resolution. Slice A bare-identifier only.
- A separate report subcommand — that is RFC-005. The two RFCs compose.

## §7 — Issue decomposition

Two vertical slices.

### Slice A — domain types + reader extensions + `#[non_exhaustive]` migration + atomic emitter arms

**Scope:** new domain types (`VerbDecl`, `VerbAnchor`, `VerbOwnership`, three new `Violation::Verb*` variants, one new `ContextViolation::CrossVerbUnauthorized` variant); `#[non_exhaustive]` on `Violation` (per Invariant 6); new emitter arms in `application/src/text.rs:18` and `application/src/ndjson.rs:42` for all four new variants (atomicity rule — must land in the same PR as the `#[non_exhaustive]` flip); new `parse_verb_bullet` handler in `adapters/markdown/src/lib.rs` (NOT extending `BULLET_PREFIXES`); new collection path for `Vec<VerbAnchor>` parallel to `Vec<Edge>` (and `finish_bullet` gains a `verb_anchors: &mut Vec<VerbAnchor>` out-param); new `visit_top_level_fn` walk function in `adapters/rust/src/lib.rs` (NOT modifying `visit_top_level_item`); shared `walk_pub_fns` helper in `adapters/rust/src/lib.rs` called only from `impl VerbReader for RustReader`; application-layer `From<PubFnDecl> for VerbDecl` mapping; new `pub fn context_for_concept(graph: &Graph, contexts: &[ContextDecl], concept_name: &str) -> Option<&ContextDecl>` helper in `domain::context` (two-hop algorithm); `CheckInput::new` updated + new `with_graph_and_contexts` ergonomic constructor.

**Tests**:
- **Acceptance (via cfdb Cypher rules):** the existing CI mechanism is `cfdb-check` job running `.cfdb/queries/*.cypher` ban rules — that is the correct landing pad, NOT new shell scripts.
  (a) New `.cfdb/queries/arch-ban-multiple-walk-pub-fns-callers.cypher` — assert EXACTLY ONE `:CallSite` to `walk_pub_fns` exists, and that its caller is `impl VerbReader for RustReader::extract_pub_fns`. (Note: depends on cfdb `:CallSite` data for graph-specs-rust — verify pin in `.cfdb/cfdb.rev` supports this query before Slice A merges.)
  (b) New `.cfdb/queries/arch-ban-verb-in-bullet-prefixes.cypher` — assert the `BULLET_PREFIXES` const-table does NOT contain a `verb:` entry and that `EdgeKind` does not gain a `Verb` variant. (Uses cfdb's literal-extraction RFC-041 facts.)
  Both Cypher rules land in the same PR as Slice A. Without them, the CHECK/REPORT shared-helper and type-system-lie guarantees remain advisory.
- Unit: pure-function assertions on the new diff pass covering all three violation variants AND the context-local rule (Invariant 8) — including a test case where the qname exists in a different context's owned unit and verifying `verb_target_unknown` fires.
- Self dogfood: add `- verb:` bullets to `specs/concepts/core.md` for at least one concept (e.g., `MarkdownReader` declaring `- verb: extract`); `graph-specs check` exits 0 with no new violations.
- Cross dogfood (graph-specs on cfdb at pinned SHA): cfdb's tree without `- verb:` bullets produces zero new violations (opt-in default).
- Target dogfood (on agentry at pinned SHA): once consumer adds `- verb:` bullets to the three `retire-when: B2-verb-anchoring-lands` anchored concepts, the pass converts those prose-only waivers to enforced verb anchors.

### Slice B — `check` wiring + dialect doc + downstream consumer update

**Scope (emitter ARMS moved into Slice A per atomicity rule; Slice B handles non-blast-radius wiring only):** `application/src/lib.rs::run_check` orchestrates the new pass (calls `VerbReader::extract_pub_fns`, applies `From<PubFnDecl>`, threads `VerbOwnership` into `domain::diff`); `specs/dialect.md` documents the `- verb:` bullet syntax + the bare-ident-only Slice A restriction; CI step added per upstream §3. **NOT in Slice B:** the new `Violation` variant arms in `application/src/text.rs` and `application/src/ndjson.rs` — those land atomically in Slice A with the `#[non_exhaustive]` flip (per Invariant 6 + §3.3).

**Tests**:
- Unit: integration assertions that the new pass triggers all three new violation variants end-to-end through `run_check`.
- Self dogfood: `graph-specs check` on this repo exits 0 after the `- verb:` bullets in `specs/concepts/core.md`.
- Cross dogfood: exits 0 on cfdb pinned tree.
- Target dogfood: agentry pinned tree shows zero `verb_missing_in_code` / `verb_target_unknown` for the three migrated anchors after the consumer migration brief lands.

## §8 — Companion consumer + retire-when

agentry's three `retire-when: B2-verb-anchoring-lands (#1145)` anchored INV-* anchors convert when this RFC's implementation lands and `.cfdb/graph-specs.rev` bumps on the consumer side:

- `INV-brief_lifecycle-late-event-no-op-warn` → `enforced-by: graph-specs L2-verb (## LateEventFence ↔ lifecycle_driver::is_late_event)`
- `INV-brief_state_stream-crash-recovery-from-cursor` → `enforced-by: graph-specs L2-verb (## RedisEventSource ↔ resume_from)`
- The consumer of `## EventSource` L2-verb anchoring

Lockstep PR per RFC-002 §3 cross-fact locking.

## §9 — Cross-references

- Consumer-side ratified RFC: `agentry:docs/rfc/RFC-verb-coverage-harvest.md` (council, 2026-05-22).
- Consumer EPIC: https://agency.lab:3000/yg/agentry/issues/793 .
- Consumer tracking issue (B2): https://agency.lab:3000/yg/agentry/issues/1145 .
- Upstream RFC request issue: https://agency.lab:3000/yg/graph-specs-rust/issues/96 .
- Sibling RFC-005 (verb-coverage report).
- Sibling cfdb `:CallSite` argument-type RFC request: https://agency.lab:3000/yg/cfdb/issues/441 .
- RFC-001 (bounded-context equivalence) — `ContextDecl.owned_units` reused.
- RFC-002 (cross-dogfood) — `Tests:` + `cross-locked.json` discipline.
- `specs/dialect.md` — extended additively.
- `specs/ndjson-output.md` v2 — `## Schema evolution` (line 179) authorizes additive new variants without `schema_version` bump.
