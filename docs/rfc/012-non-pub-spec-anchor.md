# RFC-012 — Non-`pub` spec anchors (heading → non-public code item)

- **Status:** **RATIFIED** — round-1: 4× REQUEST CHANGES (0 rejects); all blockers folded into §3/§7; round-2 confirmation: **4× RATIFY** (clean-arch, ddd, solid, rust-systems). §7 may be filed as issues (repo §2.4).
- **Date:** 2026-06-06
- **Authors:** Claude (session 2026-06-06, operator-prompted from issue #144).
- **Numbering:** RFC-011 is verbally reserved by RFC-010 (§2/§11.5) for *the PHP ladder*; this RFC takes **012** to respect that reservation.
- **Companion:** yg/cfdb — the code-facts database. RFC-010's cfdb-query ACL (R10-6, #130/#142, landed on `develop`) is the natural anchor-resolution path for the (c)-clean step; the MVP resolves anchors source-side (§3.4).
- **Supersedes:** —
- **Resolves:** #144 (concept whose canonical impl is `pub(crate)`), #143 (context that owns no `pub` type by design). The two issues are **one** design question — *how does a spec heading anchor to a code item that is not a top-level `pub` type?* — split across the two ladder rungs RFC-010 made load-bearing (concept rung / context rung). Resolving them in separate PRs would ship two divergent exemption mechanisms = split-brain; this RFC unifies them under one **anchor** primitive.
- **Related:** RFC-001 (concept-equivalence — the flat `## Concept` ↔ top-level `pub` type check); RFC-005/006/008 (verb-anchoring — `- verb: <qname>`, the existing precedent for a spec bullet that resolves to a *non-type* code symbol; this RFC lifts that pattern one rung); RFC-010 (the 4-rung ladder + `CohesionViolation::ContextWithoutCohesionUnit`, the rule #143 hits).

---

## §1 — Problem

graph-specs makes one rule load-bearing: **every `## Concept` heading is backed by a top-level `pub` type, and every `# Context` H1 eventually owns one.** The Rust reader collects only `pub struct/enum/trait/type` at file root (`adapters/rust/src/lib.rs:370` — `if !matches!(vis, Visibility::Public(_)) { return; }`); the diff fires `Violation::MissingInCode` for any spec concept with no code match (`domain/src/diff.rs:108`); RFC-010's `TreeAssembler` fires `CohesionViolation::ContextWithoutCohesionUnit` for an H1 with no H2/H3 under it (`adapters/markdown/src/tree.rs:108`).

That rule is correct for the common case — the public API *is* the spec surface. But two legitimate shapes have no honest way to satisfy it, and both **hard-block downstream consumers** (agentry, which has removed its allowlist escape hatch under a no-baseline / no-ratchet policy).

### §1.1 — The concept case (#144)

A bounded context legitimately owns a concept whose canonical implementation must stay `pub(crate)`. The motivating case: agentry's `specs/concepts/intake_validation.md ## ValidateIntakeFull` — the full gate-1..6 intake chain, implemented by `validate_intake`, kept **`pub(crate)`** per council CP-5 so `&mut ConnectionManager` never leaks through a `pub` signature.

The only way to satisfy graph-specs today is to manufacture an empty anchor type — `pub struct ValidateIntakeFull;` — a ZST whose **only** purpose is to back the heading. By construction it has **zero callers**, so it manufactures orphan public-API surface that other (correct) gates — agentry's `orphan-pub` / `dead-pub` cfdb rules — then flag. **Two correct gates in direct conflict**, resolvable today only by a forbidden allowlist or a fake `pub`.

### §1.2 — The context case (#143)

RFC-010's `ContextWithoutCohesionUnit` assumes every `# H1` context eventually owns an H2/H3 concept (a top-level `pub` type). But **behavioral / doctrine contexts own no `pub` type by design** — they are realized as `pub const` + `pub fn` + enum *variants* + cfdb fences, exactly the surfaces the Rust reader ignores. Per-file archaeology on agentry's AGE-1 set found five genuinely type-free contexts:

| Context | Realized as | Owns a pub type? |
|---|---|---|
| `boundary_signaling.md` | SHAPE registry over tokio types; fence-enforced | No — never |
| `fsm_merge_rail.md` | 2× `pub const` + 2× `pub async fn` + fences; H1 states verbatim *"No pub types are exported by this concept"* | No — by design |
| `git_operator.md` | Behavioral prose; its 5 pub types are `##`-owned in sibling contexts | No — owned elsewhere |
| `refusal.md` | `EventKind::ToolRefused` **variant** + `parse_tool_refusal`/`emit_tool_refused` fns | No — variant + fns |
| `secrets.md` | Hygiene-ledger doctrine; H1 declares *"no port, no provider, no runtime abstraction"*; an invariant forbids a type until a future RFC | No — by charter |

For these, `ContextWithoutCohesionUnit` is **unsatisfiable by any honest edit** — the only way to add a code-backed `##` is to manufacture a phantom `pub` type purely to placate the linter (a "production stub / lie as default" anti-pattern, a metric-gaming move).

### §1.3 — One wall, two rungs

Both cases hit the same wall — *a spec heading must map to a top-level `pub` type* — at two different rungs of the RFC-010 ladder (§1.1 = the **Concept** rung; §1.2 = the **Context** rung). The fix is one primitive: an **anchor** that redirects a heading's equivalence target from "a top-level `pub` type named like the heading" to a **named code item, resolvable at any visibility** (and, for the doctrine subset of §1.2, a declared *absence* of any owned type). Crucially, an anchor is **not** a suppression: it names a concrete code item the tool must still find. Delete or rename the item and the gate re-arms. The heading↔surface equivalence stays **two-way and zero-baseline**; the backing surface simply isn't required to be `pub`.

### §1.4 — Sequencing constraint (downstream)

agentry pins graph-specs behind the cohesion rule, so these violations currently fire only on a local newer build (#143: *"pre-emptive, not a live CI break"*). The exemption **MUST land here before** agentry bumps `.cfdb/graph-specs.rev` past the cohesion rule; agentry forbids baselines/allowlists/ratchets, so "just suppress it" is not available there.

---

## §2 — Scope

**Ships (Rust reference language):**

1. **Concept anchor** (§3.2) — an opt-in bullet under an H2/H3 that names the code item realizing the concept, e.g.
   ```
   ## ValidateIntakeFull
   - impl: validate_intake
   ```
   The concept's equivalence target becomes the named item — resolvable at **any visibility** and across the kinds the Rust reader otherwise ignores for concepts (`pub(crate)` type, `fn` / `pub(crate) fn`, enum variant, `const`). Absent an anchor, the existing top-level-`pub`-type rule is unchanged.
2. **Behavioral-context declaration** (§3.3) — a front-matter key (sibling to `status: draft`), `cohesion: behavioral`, that satisfies `ContextWithoutCohesionUnit` for a genuinely type-free context — **gated** against gaming (§3.3.1), not a free pass.
3. **Anchor resolution in the `CodeFacts` port** (§3.4) — a bounded, *selective* lift of the `pub`-only filter: only items **named by an anchor** are resolved at non-`pub` visibility. The global concept walk is untouched.
4. **A `DanglingAnchor` violation** (§3.5) — fires when an anchor names an item the code does not contain, preserving two-way equivalence.
5. **NDJSON + dialect** (§3.6) — the new surfaces serialize; `specs/dialect.md` gains an "Anchors" section.

**Deferred (OQs / follow-up):**
- **cfdb-query anchor parity** — resolving anchors through the RFC-010 ACL (the keyspace already holds every `:Item` at all visibilities; only `adapters/cfdb-query/src/lib.rs:127` filters them out). MVP resolves source-side so the dual-control gate needs no keyspace (OQ-1).
- **Behavioral-anchor *inference*** (#143 option 2 — exempt a context whose members are all `enforced-by: prose-only`/`cfdb-query`) as an alternative/complement to the explicit marker (OQ-2).
- **PHP** — the anchor logic is reused; the PHP fact path differs (edge-traversal), tracked under RFC-011.

**Out of scope (§6):** relaxing the *global* `pub`-only filter; auto-fixing drift; an out-of-band allowlist/baseline of any kind; re-deriving RFC-001 cross-context edges.

---

## §3 — Design

### §3.1 — Why an anchor, not a relaxed filter

The blunt fix for §1.1 — "let the reader collect `pub(crate)` types too" — is rejected (§6 non-goal 1). It widens the *whole* concept surface (every `pub(crate)` type in the tree becomes a concept the spec must now document), inverting the gate from opt-out to mandatory and breaking the "public API is the spec surface" principle. An **anchor** is surgical: only the explicitly-anchored heading changes target; everything else keeps the top-level-`pub`-type rule. And because an anchor names a concrete item the tool resolves, it is a *redirection of the equivalence target*, never a suppression — which keeps it inside the methodology's no-ratchet rule (§4 I2).

### §3.2 — Concept anchor (the Concept rung — #144)

**DD-1 ruled (council unanimous on syntax; ddd split-brain blocker folded):** the anchor is a bullet directive, `- impl: <qname>`, under an H2/H3. HTML comments — issue #144's literal `<!-- graph-specs:anchor=… -->` proposal — are **explicitly ignored** by the dialect (`specs/dialect.md`, "What the markdown reader ignores"), so adopting them would mean a one-off exception to comment-skipping; the bullet form is dialect-consistent (verb bullets are the established precedent for a concept→non-type-symbol link) and lives *inside* the concept's section so ownership is unambiguous.

```
## ValidateIntakeFull
- impl: validate_intake
```

**One grammar, not two (ddd Finding 1 — split-brain blocker).** `- impl:` and the existing `- verb:` are, at the bullet-grammar level, the *same* thing: a spec bullet naming a code symbol. They MUST NOT carry two independent qname parsers. The single qname grammar — today `parse_verb_bullet`'s validator (`adapters/markdown/src/lib.rs:518`) over `VERB_QNAME_RE` (`:504`) — is **extracted into one shared `parse_anchor_qname` function** consumed by both prefixes. Invariant (§4 I7): the qname grammar has exactly one definition; a future grammar change (e.g. three-segment paths) touches both prefixes by construction. The `impl:` key does **not** prefix-collide with `implements:` in `BULLET_PREFIXES` (`:494`) — `parse_bullet_edge` falls through first, verified (rust-systems A-1).

`ConceptAnchor` is a **distinct domain type** from `VerbAnchor` — they share the grammar but not the semantics: `VerbAnchor` *attributes* a `pub fn` to a context (ownership); `ConceptAnchor` *redirects* a concept's equivalence target (the diff unit). Distinct change-axes, distinct types, one grammar. The type lives in `domain` (consumed by the diff engine, like `VerbAnchor` at `domain/src/lib.rs:264`); the `- impl:` parser lives in the markdown adapter (solid A-2 / ddd):

```
domain (new):
/// A concept heading explicitly bound to a named code item the concept
/// walk would not otherwise surface (a non-`pub` type, fn, or const).
/// Shares the verb-bullet qname grammar; carries spec source for `path:line`.
pub struct ConceptAnchor { pub concept: String, pub target: String, pub source: Source }
```

The anchored concept's diff target is the **resolved item** (§3.4), not a name-matched top-level `pub` type. If the item resolves, the concept is satisfied (no `MissingInCode`); if it does not, `DanglingAnchor` fires (§3.5).

**Anchorable kinds — MVP cut (DD-7, rust-systems B-2 blocker).** The source-walk MVP resolves anchors to non-`pub` **type / fn / const** only — each maps directly to a `syn::Item` the reader already visits (`adapters/rust/src/lib.rs` `visit_top_level_item`/`extract_pub_fns`), so lifting the `pub` guard for anchor-named items is a direct extension. **Enum variants are deferred to R12-6 (cfdb-query).** A variant is not a `syn::Item` — resolving `EventKind::ToolRefused` means a nested `ItemEnum.variants` walk *and* disambiguation against the identically-shaped `Type::method` qname, disproportionate for the source-walk path; cfdb already emits variants as first-class `:Item` nodes with `kind: "variant"`, so the keyspace path resolves them cheaply. The motivating variant case (#143 `refusal.md`) is **already covered in MVP** via the `cohesion: behavioral` marker (§3.3) — it owns no `pub` type regardless; only an explicit `- impl: Enum::Variant` *concept* anchor awaits R12-6.

### §3.3 — Behavioral-context declaration (the Context rung — #143)

A spec file may declare, in leading front-matter (the same block `status: draft` already uses):

```
---
cohesion: behavioral
---

# secrets
```

This satisfies `ContextWithoutCohesionUnit` for that file: the context is asserted type-free **by design**, visible to any reader. It does **not** affect concept-equivalence — a concept under a behavioral context that *does* carry an `## H2` still resolves normally.

#### §3.3.1 — Anti-gaming gate (DD-3 — ruled)

A bare `cohesion: behavioral` marker is an *assertion*, and the methodology's split-brain/anti-gaming discipline asks: *what stops slapping it on a context that should own a `pub` type, to dodge the gate?* **DD-3 ruled (council unanimous): option (ii) — require machine-checkable behavioral substance.** Option (i) assertion-only is rejected (an unguarded suppression, contradicts §4 I2); option (iii) inference-no-marker is rejected (an invisible exemption — a context the author *intends* to grow a type but hasn't yet would silently self-exempt) and kept only as OQ-2.

**Substance set — enumerated (ddd Finding 2 blocker; the set is closed and machine-detectable):** `cohesion: behavioral` is honored for a file iff its context (H1) owns **≥1** of:

1. a `- impl:` concept anchor (§3.2),
2. a `- verb:` verb anchor (RFC-005/006/008),
3. a `[enforced-by: …]` or `[prose-only: …]` invariant annotation (`adapters/markdown/src/lib.rs` `extract_invariant_annotations`).

**The `secrets.md`-shaped case — ruled (ddd Finding 2):** a doctrine context with **no** `## H2`, realized purely as invariant annotations, **is exempted** — its `[prose-only:]` / `[enforced-by:]` annotations are substance (case 3). A context with `cohesion: behavioral` and **none** of the three (a genuinely empty file) **stays a `ContextWithoutCohesionUnit` violation**. This is the deterministic boundary: the marker buys an exemption only against demonstrated behavioral content, never against emptiness.

**The `git_operator.md`-shaped case ("types owned elsewhere") — scoped out (ddd Finding 4):** a context whose types are `##`-owned in *sibling* contexts is, in DDD terms, a Supporting/Conformist context; that cross-context ownership is declared in the siblings' `Imports`/`Exports` blocks (RFC-001), an **existing** concern this RFC does not re-derive. `cohesion: behavioral` + its own behavioral substance (prose-only annotations) satisfies its cohesion obligation here.

### §3.4 — Anchor resolution through an `AnchorResolver` port

Resolving an anchor asks a question the concept walk does not: *does an item named `<qname>` exist anywhere in the code, at any visibility?* The data already exists on both adapters — the source-walk `syn` AST visits every item before the `pub` filter drops non-`pub` ones (`adapters/rust/src/lib.rs:370`); the cfdb keyspace carries every `:Item` with a `visibility` prop (the ACL drops non-`pub` at `adapters/cfdb-query/src/lib.rs:127`). RFC-012 adds a **resolution query** that consults that data *only for names an anchor references* — the global concept set is unchanged (§4 I1).

**The resolution result type — `AnchorTarget` (clean-arch B-1 blocker; defined here, lives in `domain`, zero infrastructure imports):**

```
domain (new) — no `syn`, no cfdb, no `PropValue`:
pub struct AnchorTarget { pub kind: AnchorKind, pub source: Source }

#[non_exhaustive]
pub enum AnchorKind { Type, Fn, Const }   // Variant deferred to R12-6 (DD-7)
```

**Locus — DD-2 ruled (council unanimous): source-walk MVP; cfdb-query deferred.**
- **MVP = source-walk.** The `RustReader` gains a *lazy* anchor-resolution pass (mirroring `extract_verb_anchors` / `VerbReader::extract_pub_fns`) that resolves **only the qnames an anchor references** — not every non-`pub` item in the tree (rust-systems §3.4: lazy, not eager, so the global concept set is untouched and cost is bounded). The dual-control gate (`graph-specs check`) needs **no keyspace** — it stays a pure source check.
- **(c)-clean = cfdb-query.** Lift the `adapters/cfdb-query/src/lib.rs:127` filter behind a resolution method so per-crate repos (agentry) resolve anchors through the ACL too, and pick up enum-variant kinds natively (OQ-1 / R12-6).

**Port shape — DD-4 ruled (clean-arch + solid, blocking): a separate `AnchorResolver` trait, NOT a widened `CodeFacts`.** `ports::CodeFacts` has one method (`concepts`) implemented by both adapters; adding `resolve` to it would force `CfdbQueryReader` — whose anchor capability is explicitly deferred to R12-6 — to ship a `None`-returning stub for the whole MVP window, violating ISP and the methodology's "no production stubs" rule (global §6). Instead, a peer trait in `ports` (mirroring how `VerbReader` was split from `Reader`, RFC-005 §3.2):

```
ports (new):
pub trait AnchorResolver { fn resolve(&self, qname: &str) -> Option<AnchorTarget>; }
```

`impl AnchorResolver for RustReader` lands in R12-3; `impl AnchorResolver for CfdbQueryReader` in R12-6 — each independently ratifiable, no stub. Object-safe (rust-systems A-3): no generics, no `Self` return. The diff engine stays language-agnostic: it compares `(concept, anchor-target, resolved?)` tuples, never a `CodeLanguage`, and calls no reader I/O (resolution results are pre-computed into `CheckInput`, like every other fact).

### §3.5 — `DanglingAnchor` violation (DD-5 — ruled, contested resolution recorded)

Two-way equivalence requires that an anchor naming a nonexistent item fail just as a missing `pub` type does. **DD-5 ruled: a top-level `Violation` arm, NOT a `CohesionViolation` variant.**

```
domain (new) — a top-level Violation arm at violation_key rank 14:
Violation::DanglingAnchor { concept: String, target: String, spec_source: Source }
// violation_key: Violation::DanglingAnchor { concept, .. } => (concept.as_str(), 14)
// `str::as_str` is const → violation_key stays `const fn` (domain/src/diff.rs:171).
```

**Contested resolution (council merge-rationale).** ddd + solid ruled **top-level arm**; rust-systems preferred nesting it in the already-`#[non_exhaustive]` `CohesionViolation` to minimize churn (the `Violation::Cohesion(c) => (c.key(), 12)` arm and the emitters' `_ => unknown` wildcards would absorb it for free). **Taxonomy + safety win over churn:** a dangling anchor is a *concept-equivalence* defect (the analog of `MissingInCode`), not a *ladder-shape* cohesion defect; `Violation::Cohesion` is precisely the arm a consumer matches to *opt out of cohesion checking*, so nesting would let opting-out silently suppress broken-anchor detection — a footgun (ddd). The "free" emitter wildcard rust-systems cites is itself **undesirable**: rendering a real, expected violation through `_ => unknown_violation` is the exact "unknown violation" trap RFC-010 §12-G warned against — `DanglingAnchor` gets **explicit** text + NDJSON emitter arms. The const-fn constraint rust-systems required is preserved either way (above).

This is what keeps an anchor honest: rename `validate_intake` and the spec must follow, exactly as renaming a `pub` type forces a spec edit today. Exit code 1 (joins the existing non-zero exit path).

### §3.6 — NDJSON schema + dialect

- NDJSON: **DD-6 ruled (council unanimous): additive, no version bump.** `schema_version` stays `"3"`. `DanglingAnchor` is a new record kind absent from existing records; the anchored-concept fields are absent (default) on every existing concept — neither reshapes an existing field, exactly the `ImplementsDraftConcept` precedent (`domain/src/lib.rs:307`, which shipped without a bump). The `SchemaVersion::CURRENT == V3` tripwire (`domain/src/lib.rs`) guards against an accidental bump. No lockstep consumer PR (qbot-core) is required (OQ-3 closed).
- `specs/dialect.md`: a new "Anchors" section documenting `- impl:` and `cohesion: behavioral`, plus an update to "What the Rust reader ignores" (non-`pub` items are ignored *for the concept walk*, but resolvable *by anchor*).

### §3.7 — Self-dogfood

graph-specs' own `specs/` owns no `pub(crate)` concept and no type-free context today, so the primary signal is a **constructed integration fixture** (a synthetic `specs/` + crafted `.rs` exercising both anchor kinds) plus a **cross-dogfood** assertion that the companion (cfdb) stays at 0 findings. The target-dogfood signal is agentry: the AGE-1 set resolves to 0 once anchored/marked.

---

## §4 — Invariants

1. **The global concept walk is unchanged.** Only headings carrying an explicit anchor change their equivalence target; only files carrying `cohesion: behavioral` change their cohesion obligation. A spec with neither behaves exactly as today (regression-proof against the existing dogfood = 0).
2. **No suppression, no baseline (methodology §6 no-ratchet).** An anchor names a concrete code item the tool resolves; `cohesion: behavioral` is an in-spec authored declaration, machine-checked under §3.3.1(ii). Neither is an out-of-band allowlist/ceiling file. A `--update-baseline`-style escape is explicitly forbidden.
3. **Two-way and zero-baseline preserved.** A dangling anchor fires (`DanglingAnchor`); a missing behavioral substance fires (§3.3.1(ii)). Deleting/renaming an anchored item re-arms the gate.
4. **The diff engine stays language-agnostic.** Anchor resolution is a `CodeFacts`-port concern; the diff compares tuples, never `CodeLanguage`.
5. **Default-deny is retained.** Absent an anchor, a non-`pub` item is still *not* a concept; absent the marker, a type-free context is still a `ContextWithoutCohesionUnit`. The opt-in surfaces the intent.
6. **Stable wire schema.** NDJSON change is additive (`schema_version` stays `"3"`, DD-6) — never a silent reshape; the `SchemaVersion::CURRENT` tripwire enforces it.
7. **One qname grammar.** The `- impl:` and `- verb:` bullet prefixes share exactly one qname-validation function (`parse_anchor_qname`); no second parser may be introduced (anti-split-brain, ddd Finding 1). A grammar change touches both prefixes by construction.

---

## §5 — Architect lenses

Each lens returns RATIFY / REQUEST CHANGES / REJECT with evidence and prescribes the §7 `Tests:` rows (§2.3). Not ratified until all four RATIFY or a single author-documented override is recorded. Artifacts: this section + the agent transcripts.

**Round 1 — 4× REQUEST CHANGES, 0 REJECT.** The design (anchor-not-suppression framing, source-walk MVP locus, the #143+#144 unification) was judged fundamentally sound by every lens; the blockers were all RFC-text closures of the open design decisions, folded into §3/§7. **Round 2 — 4× RATIFY** (each lens re-read the resolved text and confirmed its blockers closed). Ratified.

### §5.1 — Clean architecture (`clean-arch`) — round 1: REQUEST CHANGES → round 2: **RATIFY**
Blockers (resolved): **B-1** define `AnchorTarget` concretely in `domain` with no infra imports → §3.4. **B-2** close DD-5 + confirm `const fn` → §3.5. Rulings: DD-2 source-walk sound (diff engine verified I/O-free, `domain/src/diff.rs:49`); DD-4 separate `AnchorResolver` trait (the `VerbReader` precedent); unification §1.3 sound — same problem framing, two *independent* code-path modifications (concept-diff vs cohesion pass), not split-brain. Round 2: "No remaining dependency-direction or port-purity objection."

### §5.2 — Domain-driven design (`ddd-specialist`) — round 1: REQUEST CHANGES → round 2: **RATIFY**
Blockers (resolved): **Finding 1** homonym `- impl:`/`- verb:` — one shared grammar, distinct types → §3.2 + §4 I7. **Finding 2** enumerate the behavioral-substance set + rule the `secrets.md` shape → §3.3.1. Rulings: DD-3 option (ii); DD-5 top-level arm (cohesion-nesting is an opt-out footgun); behavioral/doctrine context is a legitimate DDD Supporting context (Finding 4). Round 2: "One owning concept, one canonical resolver, zero split-brain risk."

### §5.3 — SOLID + component principles (`solid-architect`) — round 1: REQUEST CHANGES → round 2: **RATIFY**
Blockers (resolved): **B-1** DD-4 separate `AnchorResolver` trait — widening `CodeFacts` forces a `CfdbQueryReader` stub for the MVP window (ISP + global §6 "no production stubs") → §3.4. **B-2** DD-5 top-level arm — `DanglingAnchor` and `CohesionViolation` have different change-axes (CCP) → §3.5. Advisory: R12-3 has 3 reasons-to-change (SRP) — addressed in §7 by extracting the resolver as its own module within the one vertical slice (splitting into a separate issue would ship an `AnchorResolver` with no observable failure behavior — methodology §6 rule 2). `domain` stays maximally stable (I≈0); no zone-of-pain risk.

### §5.4 — Rust systems (`rust-systems`) — round 1: REQUEST CHANGES → round 2: **RATIFY**
Blockers (resolved): **B-2** DD-7 cut enum variants from the source-walk MVP (a variant is not a `syn::Item`; ambiguous with `Type::method`) → defer to R12-6 where cfdb's `kind:"variant"` is native → §3.2. **B-1** close DD-5 + preserve `const fn` → §3.5. Rulings: DD-2 source-walk lazy (resolve-only-named) — bounded cost; no new crate or feature flag; `AnchorResolver` object-safe, no orphan-rule issue; `impl:` ≠ `implements:` prefix (no `BULLET_PREFIXES` collision). Round 2: the top-level-arm overrule of its own churn preference is "a systems efficiency preference, not a correctness constraint" — RATIFY.

### §5.5 — Consolidated design-decision rulings

| DD | Question | Ruling | Vote |
|---|---|---|---|
| **DD-1** | anchor syntax | `- impl:` bullet; **one** shared qname grammar (§3.2, I7) | unanimous (ddd: +homonym fix) |
| **DD-2** | resolver locus | source-walk MVP; cfdb-query deferred to R12-6 | unanimous |
| **DD-3** | behavioral anti-gaming | option (ii) require-substance; set enumerated; `secrets.md` exempted via prose-only annotations | unanimous |
| **DD-4** | port shape | separate `ports::AnchorResolver` trait (not a widened `CodeFacts`) | clean-arch + solid (rust: either object-safe) |
| **DD-5** | `DanglingAnchor` placement | **top-level `Violation` arm, rank 14** (const-fn preserved) | ddd + solid **over** rust-systems (churn) — *contested, resolved §3.5* |
| **DD-6** | NDJSON | additive, `schema_version` stays `"3"` | unanimous |
| **DD-7** | anchorable kinds | MVP = type / fn / const; **variant deferred to R12-6** | rust-systems (others defer) |

---

## §6 — Non-goals

1. **Not relaxing the global `pub`-only filter** (§3.1) — anchors are by-name and surgical; the blanket-`pub(crate)` alternative is rejected.
2. **Not an allowlist / baseline / ceiling** of any form (§4 I2). The methodology's no-ratchet rule is binding; the fix for a real violation is an anchor (which still resolves) or honest spec/code work, never suppression.
3. **Not auto-fixing** drift.
4. **Not shipping the PHP backend** (RFC-011); the anchor logic is reused there later via an edge-traversal fact path.
5. **Not re-deriving RFC-001 cross-context edges** — intra-context cohesion + concept-equivalence only.
6. **Not making the marker retroactive** — like `status: draft`, it consults only leading front-matter.

---

## §7 — Issue decomposition

Vertical slices; one issue each; `Tests:` per repo §2.5, prescribed by the council (§5). **Target dogfood = agentry** (the AGE-1 set + `intake_validation`) at a pinned SHA. Sequencing: R12-1 → {R12-2 → R12-3, R12-2 → R12-4} → R12-5; R12-6 deferred (OQ-1).

#### R12-1 — Domain types + top-level violation
`ConceptAnchor`, `AnchorTarget`/`AnchorKind` (§3.4), behavioral-context flag on the spec graph, pure anchor-match fn, and `Violation::DanglingAnchor` (top-level, `violation_key` rank 14, DD-5). Because `Violation` is matched exhaustively in `application` (text + NDJSON), this slice **includes the minimal `DanglingAnchor` emitter arms** so the workspace compiles and cross-dogfood does not panic — the *authoritative* NDJSON schema doc + dialect land in R12-5. No reader/adapter change.
```
Tests:
  - Unit: ConceptAnchor round-trips concept/target/source; violation_key places DanglingAnchor at rank 14 (after ImplementsDraftConcept=13) and the fn still compiles `const`; pure resolve_anchor(anchors, resolved_items) returns Some for a present target, None (→DanglingAnchor) for absent; behavioral-flag toggles the cohesion obligation in a pure fn (no reader); qname grammar rejects multi-segment / empty; SchemaVersion::CURRENT stays V3.
  - Self dogfood (graph-specs on graph-specs): `check --specs specs/ --code .` exits 0 — this repo carries no `- impl:`/`cohesion: behavioral`; the new domain types must not regress the existing baseline.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0 — the new top-level arm's emitter arms ship in this slice, so the NDJSON run does not panic on cfdb's output.
  - Target dogfood (agentry at pinned SHA): none — rationale: no reader/diff wiring yet; AGE-1 anchors cannot be authored until R12-2/R12-3. Mandatory at R12-3.
```

#### R12-2 — Markdown reader: `- impl:` + `cohesion: behavioral`
Extract the single `parse_anchor_qname` grammar (§4 I7) and dispatch `- impl: <qname>` through it (reusing `VERB_QNAME_RE`); parse `cohesion: behavioral` via the `status: draft` front-matter path (`is_draft` precedent, `adapters/markdown/src/lib.rs:317`); thread both into the spec graph / `TreeAssembler`.
```
Tests:
  - Unit: parse_anchor_qname accepts `impl: validate_intake` and `impl: Type::method`; `implements: Foo` → None (no BULLET_PREFIXES false-positive); one regex feeds both prefixes (verb + impl parse the same qname identically); is_behavioral_context across the five front-matter shapes the is_draft suite covers (fence-only / no-fence / closed-before-key / present / absent).
  - Self dogfood (graph-specs on graph-specs): exits 0 — parser added, no behavioral context or anchor in this repo's specs, so output is unchanged.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0 — the new bullet key must not false-positive on any existing cfdb spec bullet.
  - Target dogfood (agentry at pinned SHA): none — rationale: the reader parses but nothing resolves (R12-3) or exempts (R12-4) yet; carried forward to those slices.
```

#### R12-3 — Source-walk `AnchorResolver` + diff wiring + `DanglingAnchor`
`impl AnchorResolver for RustReader` as its **own module** (solid SRP advisory — a lazy resolve-only-named pass mirroring `extract_verb_anchors`, resolving non-`pub` **type/fn/const**, DD-7); wire the anchored concept to diff against the resolved item; emit `DanglingAnchor` + non-zero exit. One vertical slice (resolution without its failure path is half a feature).
```
Tests:
  - Unit: fixture `.rs` with `pub(crate) fn validate_intake` → resolve Some(Fn); `pub(crate) struct X` → Some(Type); `pub const LIMIT` → Some(Const); nonexistent → None. Full run_check: an anchored concept whose target resolves emits no MissingInCode; `- impl: renamed_fn` with no such item emits exactly one DanglingAnchor{concept,target}; an un-anchored `pub(crate)` item is NOT surfaced as a concept (§4 I1 regression); a DanglingAnchor drives process exit 1 (CLI fixture, application/tests/cli.rs).
  - Self dogfood (graph-specs on graph-specs): exits 0; plus an in-repo synthetic fixture with `- impl: <a pub(crate) fn>` passes (no DanglingAnchor, no MissingInCode).
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0 — lazy resolve-only-named adds no global concept nodes, preserving cfdb's zero-findings baseline.
  - Target dogfood (agentry at pinned SHA): anchor `intake_validation.md ## ValidateIntakeFull` with `- impl: validate_intake` → 0 violations for that concept (was MissingInCode). Report the before/after total-violation count in the PR body.
```

#### R12-4 — Cohesion exemption + anti-gaming gate
`cohesion: behavioral` satisfies `ContextWithoutCohesionUnit` (`adapters/markdown/src/tree.rs` `has_cohesion_unit`) **only** when the §3.3.1 substance set is non-empty (DD-3 option ii).
```
Tests:
  - Unit: SpecTree::cohesion_violations() is empty for a behavioral-marked file WITH substance (≥1 of {`- impl:`, `- verb:`, `[enforced-by:]`/`[prose-only:]`}); STILL emits ContextWithoutCohesionUnit for a behavioral-marked file with ZERO substance (anti-gaming — a test, not a comment); the `secrets.md` shape (only prose-only annotations, no `## H2`) is exempted; the substance predicate enumerates exactly the three cases.
  - Self dogfood (graph-specs on graph-specs): exits 0; the existing self-dogfood tree test (`tree.rs` `self_dogfood_concept_specs_…`) still passes — no accidental behavioral false-positive on this repo's specs.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0 — no cfdb file acquires an unintended exemption.
  - Target dogfood (agentry at pinned SHA): the five type-free contexts (boundary_signaling, fsm_merge_rail, git_operator, refusal, secrets) marked `cohesion: behavioral` drop ContextWithoutCohesionUnit 5 → 0; other cohesion violations unaffected. Report before/after in the PR body.
```

#### R12-5 — NDJSON surfaces + dialect + README
Authoritative `DanglingAnchor` + anchored-concept NDJSON serialization (additive, `schema_version` `"3"`, DD-6); explicit text emitter arm (no `_ => unknown`, §12-G); `specs/dialect.md` "Anchors" section + the "What the Rust reader ignores" amendment; README.
```
Tests:
  - Unit: NDJSON emits DanglingAnchor fields (concept/target/spec_source) under schema_version "3"; SchemaVersion::CURRENT stays V3 (tripwire); text renderer shows DanglingAnchor as `path:line` (asserts it is NOT rendered as "unknown violation"); an anchored concept's NDJSON record carries the resolved-target fields.
  - Self dogfood (graph-specs on graph-specs): `check --format ndjson` on this repo emits no DanglingAnchor record (no anchors here).
  - Cross dogfood (graph-specs on cfdb at pinned SHA): `--format ndjson` → 0 findings, no DanglingAnchor records.
  - Target dogfood (agentry at pinned SHA): `--format ndjson` on the AGE-1 fixture — the new anchor fields appear for ≥1 anchored concept; report field presence in the PR body.
```

#### R12-6 *(deferred / OQ-1)* — cfdb-query `AnchorResolver` parity
`impl AnchorResolver for CfdbQueryReader`: lift the `adapters/cfdb-query/src/lib.rs:127` filter behind the resolve path (the `CONCEPT_KINDS`/`concepts()` population is untouched) so per-crate repos resolve anchors through the ACL, and resolve **enum-variant** kinds natively (cfdb `kind:"variant"`, the DD-7 deferral).
```
Tests:
  - Unit: a keyspace fixture node `visibility:"pub(crate)", kind:"fn", name:"validate_intake"` → resolve Some(Fn); `kind:"variant"` → resolve Some (the deferred variant case); concepts() still returns None for the non-`pub` node (resolve is a separate path); contract test (double-faithfulness §6-1): source-walk and cfdb-query resolve identically on the same fixture.
  - Self dogfood (graph-specs on graph-specs): exits 0 — this repo uses source-walk; R12-6 is a no-op for self.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0 — resolve is opt-in (anchor-named only), global population unchanged.
  - Target dogfood (agentry at pinned SHA, the per-crate cfdb-query path): resolve("validate_intake") and resolve("EventKind::ToolRefused") both Some; the AGE-1 anchor set resolves identically to R12-3's source-walk (parity delta 0). Report in the PR body.
```

---

## §8 — Open questions

| ID | Question | Status |
|---|---|---|
| OQ-1 | cfdb-query anchor parity (resolve through the R10-6 ACL). | OPEN — deferred; MVP is source-walk (§3.4). |
| OQ-2 | Behavioral-anchor *inference* (#143 option 2) as an alternative to the explicit marker. | OPEN — §3.3.1(iii); council may fold into DD-3. |
| OQ-3 | Does any consumer (qbot-core `compare-spec-change`) need a lockstep arm if NDJSON bumps? | OPEN — avoided iff DD-6 = additive-no-bump. |

---

## §9 — Ratification

**RATIFIED** (2026-06-06). All four lenses RATIFY (§5); every §7 slice carries a prescribed `Tests:` block. §7 may now be filed as issues per repo §2.4 — each linking `Refs: docs/rfc/012-non-pub-spec-anchor.md`, carrying its prescribed `Tests:` block verbatim, and `Resolves:`/`Refs:` the originating issue (#144 for R12-1/2/3/5/6; #143 for R12-1/2/4) — worked via `/work-issue-lib`. R12-6 is filed as deferred (OQ-1). No code is written until the slice issues exist (RFC-first, repo §1).
