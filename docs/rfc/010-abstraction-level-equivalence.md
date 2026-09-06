# RFC-010 — Abstraction-level equivalence (the heading ladder)

- **Status:** FINAL — operator-ratified §13-A (cohesion fact-source routing: **(b)-MVP → (c)-clean**). Body §1–§11 integrates all binding dry-run resolutions; §12 is the condensed hardening trail. Ready to file §7 issues. Amendment 2026-09-06: §4 invariant 9, the binding pass's concept identity.
- **Date:** 2026-06-05
- **Authors:** Claude (session 2026-06-05, operator-prompted).
- **Companion:** yg/cfdb — the code-facts **database** and a co-evolving peer. graph-specs queries it as a first-class fact source through an **Anti-Corruption Layer** adapter (§3.3); the convergence step (c) is a small *paired cfdb RFC* (§8 OQ-8, §11) — cfdb already models multi-crate contexts (`.cfdb/concepts/*.toml`), so (c) needs **no cfdb schema change**.
- **Supersedes:** —
- **Related:** RFC-001 (`specs/contexts/` + `# <ctx>` H1=context — the **canonical-upstream** context definer, §3.4); RFC-004 (`LanguageBackend` seam; reserved the v3 NDJSON bump); RFC-005/006/007/008 (verb-anchoring — the downward concept→method rung; this RFC adds the upward concept→context rung); RFC-011 (PHP ladder — inherits this via an edge-traversal ACL path, §11.3).

---

## §1 — Problem

graph-specs exists to make one thing true: **a human can trust model-written code by reading abstractions instead of reading lines.** The trust chain:

```
RFC  →  specs  →  graph-specs ↔ code
```

A human ratifies intent (the RFC, 4-lens council). It graduates into an authoritative abstraction ladder in `specs/concepts/*.md`. graph-specs makes that ladder **load-bearing**: if code stops matching the declared abstraction, CI goes red. The human trusts the *abstraction*; graph-specs guarantees conformance.

cfdb is the **code-facts database** — the queryable store of what the code *is* (containment, calls, impls, signatures). Architectural bans are *one* query family over it; graph-specs is another consumer that asks cfdb what the code is and checks it against the spec ladder. The bias to resist is treating cfdb as a ban-*oracle* that only emits verdicts, when it is a *database* that can tell you about the code. So "graph-specs ↔ code" does not oblige graph-specs to *parse* code; it obliges graph-specs to *know the code's facts*, which are cfdb's product. (graph-specs ⇒ *the map matches the outline*; cfdb ⇒ *the queryable territory itself*.)

Today graph-specs enforces **one rung, flatly**: the markdown reader collapses `##`/`###` headings into a flat concept set; the Rust reader collapses top-level `pub` types into a flat set; the diff checks set-equality. Heading *depth* is read only to gate which headings count, then discarded (`#`/`####+` ignored; module containment dropped). But the abstraction a human authors is **structured** — bounded context ⊃ concept ⊃ sub-concept ⊃ member. Three trust gaps follow:

1. **The context rung is asserted-but-unchecked** — a file's H1 declares a bounded context the gate never verifies; its concepts can scatter in code while the flat check stays green.
2. **Concept ownership is unverified** — a type documented under the wrong context is invisible to a set-membership check.
3. **The ladder is not a ladder** — no machine-checked relationship between heading depth and the code abstraction it claims.

This RFC makes every rung load-bearing, and (council mandate, §11) reconciles the abstraction vocabulary across graph-specs, cfdb, and agentry into **one** architecture.

> **Operator framing (binding):** agentry is a dogfooded field *example*, not an oracle. Where its specs deviate from the model below, that is **drift the gate must catch** and a migration agentry performs — never a constraint the model bends to. The council is the architectural authority; consumers conform (§11).

## §2 — Scope

**Ships (Rust reference language):**
- **The 4-rung ladder in `domain`** (§3.1): `AbstractionLevel { Context, Concept, SubConcept, Member }`, `#[non_exhaustive]`, with a `from_heading_depth` constructor. Spec graph gains parent links; `ConceptNode` gains containment provenance (§3.3).
- **H1 = bounded-context identifier** (§3.2) with one normalization rule applied to **both** `specs/contexts/` and `specs/concepts/` H1s.
- **Code-side containment via a `CodeFacts` port** (§3.3) with two adapters — source-walking (`syn`) and a cfdb-query **ACL** — emitting **language-agnostic** fields `module_path`/`unit`/`context`.
- **Cohesion checks** (§3.5): `Violation::Cohesion(CohesionViolation)` — `ContextWithoutCohesionUnit`, `SubConceptOrphan`, `ConceptContextMismatch`. Split by **fact-dependency**: the first two fire spec-side (zero code facts); the third is code-fact-gated (§3.4 routing).
- **NDJSON schema → `"3"`** (§3.6); **self-dogfood** `core.md` split (§3.7).

**Deferred (OQs / follow-up RFCs):**
- **L4 (Member) diffing** — emitted only (OQ-2).
- **Universal cfdb-query cohesion on multi-crate-context repos** — the **(c)** step: a small paired cfdb RFC giving cfdb a `ContextSource` that reads `specs/contexts/` Owns (OQ-8). MVP is **(b)**: route each repo to the adapter whose context model matches (§3.4).
- **The PHP ladder (RFC-011)** — inherits the ladder; PHP `:Item` is prop-less, so the ACL needs an **edge-traversal** path (PHP is *not* "free" — §11.3).

**Out of scope:** changing the diff engine to branch on language; re-deriving RFC-001 cross-context *edges*; forcing consumers off `specs/contexts/`; any cfdb *schema* change (the (c) ContextSource reuses cfdb's existing multi-crate override machinery).

## §3 — Design

### §3.1 — The 4-rung ladder
Heading depth maps to a typed abstraction level. **Depth is authoritative**: in a conformant spec a heading's role *is* its depth; the tool enforces the mapping rather than inferring intent. A pub type written at H4 is drift the gate surfaces.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbstractionLevel { Context, Concept, SubConcept, Member }

impl AbstractionLevel {
    /// Adapters call this instead of `match`-ing the enum, so `#[non_exhaustive]`
    /// never forces a dead wildcard arm in adapter crates (dry-run §12 #E).
    #[must_use] pub const fn from_heading_depth(depth: u8) -> Self { /* 1→Context … 4+→Member */ }
}
```

H1=Context, H2=Concept (the diff unit), **H3=SubConcept** (a nested pub type, diffed at L2 — *not* collapsed into Member), H4+=Member (emitted, not diffed). A file with an H1 but **no H2** is malformed → `ContextWithoutCohesionUnit`.

### §3.2 — Spec side: H1 is a context identifier
The markdown reader treats a file's single H1 as a `Context` declaration whose text *is* the context identifier. One normalization rule — lowercase, internal whitespace→`-` (`# AC verifier` → `ac-verifier`) — is applied to **both** the `specs/contexts/`-side H1 (`contexts.rs`) and the `specs/concepts/`-side H1, so the two resolve to the same identifier (dry-run §12 #I; the single-word lowercase self-dogfood hid this). A descriptive H1 matching no `specs/contexts/` entry and not identifier-shaped is a reader error. H1/parent-tree assembly is a **separate `TreeAssembler` pass** (SRP: the existing `handle_event`/`SectionState` is at the complexity ceiling).

### §3.3 — Code side: containment through a `CodeFacts` port

#### §3.3.1 — CodeFacts
graph-specs needs each concept's containment — its `module_path`, `unit`, and resolved `context`. "graph-specs ↔ code" needs the code's *facts*, not parsing, so the code side is a **port**:

```rust
pub trait CodeFacts { fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>; }
```

#### §3.3.2 — ConceptNode

`ConceptNode` gains three **language-agnostic** `Option<String>` fields — **`module_path`, `unit`, `context`** — *not* cfdb's Rust-specific prop names, because cfdb's PHP `:Item` carries no such props (containment is edge-only). Two adapters:

- **Source-walking** (`RustReader` extended): keeps `syn`; derives `module_path` from the file path (collapsing `lib.rs`/`mod.rs`/`main.rs` to the crate root — dry-run §12 #H), `unit` from the owning crate **relative to the code root** (not the raw walked path — dry-run §12 #I), `context` from `specs/contexts/` Owns.
- **cfdb-query ACL** (`adapters/cfdb-query`, a **feature-gated** crate, `cfdb-core` **only** path dep — dry-run §12 #E/#G): reads `:Item` from a keyspace and **translates** cfdb's per-language representation into the agnostic fields (Rust = prop-reads `module_qpath`/`crate`/`bounded_context`; PHP = edge-traversal). It is an **Anti-Corruption Layer**, not a Conformist — cfdb's representation differs by language. It filters synthetic `:Item` stubs (absent `file`/`module_qpath`) and parses `schema_version` as a **struct** (dry-run §12 #F).

**Verified:** the agnostic ACL passes parity with source-walk (0 mismatches on `module_path`/`unit`) on a real 513-`:Item` keyspace; `application` does not transitively pull cfdb (opt-in leaf).

#### §3.3.3 — Adapter routing

**Adapter routing (the §13-A decision, MVP = (b)).** The two adapters are **not** universally interchangeable for `context`, because graph-specs' spec contexts are **multi-crate** (`equivalence` owns `domain`+`ports`) while cfdb's default `bounded_context` is **per-crate** (crate-prefix heuristic). The composition root routes by the repo's context model:

| Repo context model | `context` source | Adapter |
|---|---|---|
| Multi-crate (declared in `specs/contexts/` Owns; e.g. graph-specs) | `specs/contexts/` Owns | **source-walk** |
| One-per-crate (e.g. agentry) | cfdb `:Item.bounded_context` | **cfdb-query** |

This routing has **zero divergence** in practice — each repo uses the adapter whose model matches. The convergence step **(c)** — making cfdb-query coherent on multi-crate repos too — is deferred (OQ-8): cfdb already supports multi-crate grouping via `.cfdb/concepts/*.toml` `crates` lists, so (c) needs only a small cfdb `ContextSource` that reads `specs/contexts/` Owns (one source of truth, no split-brain), not a schema change. (The `.cfdb/concepts/*.toml`-mirror shortcut is rejected as a destination — it duplicates the grouping = split-brain.)

### §3.4 — Context resolution (two questions, not one chain)
A concept's owning context has one authority, but the **spec-side declaration** and the **code-side resolution** are distinct questions (dry-run §12 #B — conflating them makes the mismatch tautological):

- **Spec-side declaration (what the author claims):** the `concepts/` H1 (or a matching `specs/contexts/` `# <ctx>`). RFC-001's `specs/contexts/` is canonical-upstream when present.
- **Code-side resolution (what the code says):** **(A)** `specs/contexts/` Owns, or **(C)** the cfdb-query ACL's `context`. Per §3.3 routing, exactly one is used per repo. cfdb's `.cfdb/concepts/*.toml` is never read by graph-specs directly — it is the ACL's input only.

`ConceptContextMismatch` compares the spec-side declaration against the code-side resolution. The resolver is a **pure function in `domain/src/context.rs`** (extends `context_for_concept`), so the diff engine calls no reader I/O. `unit` is normalized relative to the code root before comparison (dry-run §12 #I).

### §3.5 — Cohesion violations
A new `CohesionViolation`, wrapped as `Violation::Cohesion(CohesionViolation)` — distinct from `Violation::Context(ContextViolation)` (RFC-001 cross-context edges). One new `violation_key` arm at rank 12.

```rust
pub enum CohesionViolation {
    // Spec-side — fire with ZERO code facts (source-walk OR cfdb-query):
    ContextWithoutCohesionUnit { context: String, file: PathBuf }, // H1 with no H2/H3 concept
    SubConceptOrphan          { sub_concept: String, file: PathBuf }, // H3 with no enclosing H2 (detected by R10-2 TreeAssembler)
    // Code-fact-gated — needs a code-side context (A or C per §3.4):
    ConceptContextMismatch    { concept: String, declared: String, code_context: String, spec_source: Source },
}
```

`ConceptContextMismatch` carries `spec_source` so its text rendering shows `path:line` like every other violation (dry-run §12 #B). The heterogeneous fields mean `violation_key` is a plain `fn`, **not** `const fn` (dry-run §12 #D — trivial, no runtime cost; the §3.1 enum match is what stays exhaustive). Exit code **1** for any cohesion violation — R10-3/R10-4 must verify `Violation::Cohesion` participates in the non-zero exit path (a pre-existing `check`-exits-0 gap, dry-run §12 #F); `2` stays reserved for unparseable input.

**Capability matrix (which variant fires under which fact source):**

| Variant | source-walk (no `specs/contexts/`) | source-walk (+ `specs/contexts/`) | cfdb-query |
|---|---|---|---|
| `ContextWithoutCohesionUnit` | ✅ | ✅ | ✅ |
| `SubConceptOrphan` | ✅ | ✅ | ✅ |
| `ConceptContextMismatch` | ✗ (no code-side context) | ✅ | ✅ (per-crate repos) |

### §3.6 — NDJSON schema v3
`schema_version` → `"3"`. Source objects gain `module_path`/`unit`/`context`; the `Cohesion` variants serialize with their fields. `SchemaVersion::CURRENT` → `V3`; the existing `"2"` assertions migrate. The cohesion emitter arms (`text.rs` + `ndjson.rs`) land **in the same slice that emits the violations** (dry-run §12 #G — otherwise findings render as `"unknown violation"` / are absent from NDJSON). qbot-core's `compare-spec-change` adds a v3 arm in lockstep (OQ-3).

### §3.7 — Self-dogfood: split `core.md`
`specs/concepts/core.md` (H1 `# Core concepts`, a title spanning three contexts) is malformed under §3.2. Atomically (R10-1): split into `equivalence.md`/`reading.md`/`orchestration.md` (each H1 = its `specs/contexts/` name); add the new pub types (`AbstractionLevel`, `CodeFacts`, `CohesionViolation`, the cfdb-query crate's surface) as H2 entries **and** their `Owns`/spec entries; **rewrite the existing `## CohesionViolation` spec block** (drop `depends on: Source`, restate fields — the §3.5 reshape falsifies it; dry-run §12 #C). Field-add touches ~20 construction sites. Dogfood → 0.

### §3.8 — cfdb is a queryable database; depth is a composition choice
The cfdb-query ACL is the proper exploitation of cfdb-the-database (don't re-walk source to recompute what a keyspace holds). What stays a composition choice is the **depth**, kept swappable by the `CodeFacts` port: source-walk floor → cfdb-query (per-crate repos now; multi-crate repos after (c)/OQ-8) → shared `<lang>-items` crate (#83, separate). Movement along the axis is an adapter swap, never a rewrite.

## §4 — Invariants
1. **One owning context per concept** — spec-side declaration (H1 / `specs/contexts/`) checked against code-side resolution (A or C, §3.4); `specs/contexts/` canonical-upstream.
2. **`ConceptNode ↔ cfdb `:Item`, NOT `:Concept`** (`:Concept` is an enrichment overlay whose `name` is a *context* name). Provenance fields are **language-agnostic** (`module_path`/`unit`/`context`), translated by the ACL — never cfdb's Rust-specific prop names.
3. **The cfdb-query adapter is an ACL; `domain`/`ports` link no cfdb crate.** The feature-gated `adapters/cfdb-query` crate links `cfdb-core` only. cfdb representation changes → the ACL changes, not the port.
4. **The diff engine stays language-agnostic** — cohesion is over `(concept, module_path, unit, context, H1-declared)` tuples, never `CodeLanguage`. PHP supplies the same tuples via ACL **edge-traversal** (not prop-reads).
5. **The flat concept-set check still runs**; H3 (SubConcept) stays diffed at L2 (no regression).
6. **Depth is authoritative; deviation is drift** — the tool enforces depth→level, never infers a heading's "real" level.
7. **Adapter routing has zero divergence** — each repo uses the adapter whose context model matches (§3.3); cfdb-query cohesion on multi-crate repos awaits (c)/OQ-8.
8. **NDJSON v2 → v3 is a hard break gated by `schema_version`** — no silent mis-parse.
9. **Concept identity in the binding pass is `(name, unit)`** — Amendment 2026-09-06: a surface item is bound or reported by its own name and its owning unit, never collapsed onto another item of the same name under another unit; a heading binds the item of its own context (§3.4, H1 = context), and a same-named item under another unit is a second concept — bound by a second heading in that context's document, or reported as undescribed. This is what §3.5's `ConceptContextMismatch` already presupposes (a mismatch is per item, not per name) and what `cascade-gate#3.2` and `cascade-gate#4` `TypeRealizedTwice` rule for the sibling instrument; the cascade ⟷ graph-specs listing equivalence a host fences holds only if both count the same items. Found 2026-09-06 on cours-coreen: `Clock`, `SubjectId`, `SystemClock` each realized under two declared prefixes and each reported once.

## §5 — Council review & hardening

## §6 — Non-goals
1. Not changing cfdb's *schema*; graph-specs queries it (ACL). The (c) `ContextSource` is a small paired cfdb RFC (OQ-8), not part of RFC-010's ship.
2. Not forcing consumers off `specs/contexts/`.
3. Not adding RFC-001 cross-context *edge* semantics (intra-context cohesion only).
4. Not diffing L4 (Member) — emitted only.
5. Not shipping the PHP backend (RFC-011) or the `LanguageBackend` registry.
6. Not auto-fixing drift.

## §7 — Issue decomposition
Vertical slices; `Tests:` per repo §2.5. **Target dogfood = agentry.**

| ID | Slice | Tests |
|---|---|---|
| **R10-1** | Domain: `AbstractionLevel`(4) + `from_heading_depth`; `ConceptNode` agnostic `module_path`/`unit`/`context` (`Option<String>`, ~20 sites); `Violation::Cohesion(CohesionViolation)` (§3.5 enum incl. `spec_source`) + `violation_key` rank 12 (non-`const`); resolver precedence pure-fn. **Self-dogfood `core.md` split + `## CohesionViolation` spec rewrite** (§3.7). | Unit: enum/provenance round-trip; resolver; `violation_key` rank. Self dogfood: 0 after split. Cross: cfdb unaffected. |
| **R10-2** | Markdown `TreeAssembler` (separate pass, <15): H1→Context (normalized identifier, both sides), H2→Concept, H3→SubConcept (+ orphan detection), H4→Member; parent links. | Unit: heading-tree fixtures (no-H2 malformed, descriptive-H1 error, `# AC verifier`→`ac-verifier` matches both sides). Self/target: agentry — one Context/file; 6 H1-only files surface malformed. |
| **R10-3** | Source-walk `CodeFacts` adapter (`module_path` crate-root collapse; `unit` relative to code root); the 3 cohesion variants **+ the `text.rs`/`ndjson.rs` `Cohesion` arms in the same slice** (§3.6/§12-G); verify non-zero exit (§12-F). | Unit: each variant; data-dependency split (with/without `specs/contexts/`); abs-vs-rel `--code`. Self: 0. **Target (agentry): report cohesion count — expected non-zero (AGE-1), reported not gated.** |
| **R10-4** | NDJSON v3 (`SchemaVersion::V3`; agnostic source fields; migrate `"2"` assertions); `specs/ndjson-output.md` authoritative. | Unit: emitter snapshots; version tripwire. Target: qbot-core v3-arm PR (OQ-3). |
| **R10-5** | `specs/dialect.md` §"Abstraction ladder" + normalization rule; README 4→5 levels. | Docs. Self: 0. |
| **R10-6** | cfdb-query **ACL** (`adapters/cfdb-query`, feature `codefacts`, `cfdb-core`-only): `:Item`→agnostic fields; stub filter; struct `schema_version`; crate-root + hyphen/underscore normalization. Composition root routes per §3.3. | Unit: parity vs source-walk on a real keyspace (0-mismatch); PHP empty-provenance probe. Target (agentry): run via cfdb-query against agentry's keyspace. |

## §8 — Open questions
| ID | Question | Status |
|---|---|---|
| OQ-1 | Canonical context resolver. | RESOLVED — §3.4 (spec-side declaration vs code-side A/C; routing §3.3). |
| OQ-2 | L4 (Member) diffing vs verb-anchoring. | OPEN — Member emitted-only. |
| OQ-3 | qbot-core v3-arm lockstep. | OPEN — block ship until ready. |
| OQ-4 | Sub-module cohesion granularity; non-test inline-`mod` divergence (§12-I). | OPEN — MVP is top-level/`is_test`-filtered. |
| OQ-6 | cfdb CF-1 (`:Context`/`:Concept` merge). | OPEN — paired cfdb RFC, advisory. |
| OQ-7 | Aggregate cohesion views (`ScatteredConcepts`/`SplitUnit`). | OPEN — derive from per-concept mismatch. |
| **OQ-8** | **(c) convergence:** paired cfdb RFC for a `ContextSource` reading `specs/contexts/` Owns, making cfdb-query cohesion coherent on multi-crate repos. | OPEN — **operator-approved as the post-MVP step (b→c)**; needs **no cfdb schema change** (cfdb already supports multi-crate grouping via `.cfdb/concepts/*.toml` `crates` lists); lockstep with graph-specs. |

## §9 — Ratification
**FINAL.** §13-A operator-ratified ((b)→(c)); §1–§11 integrate all binding resolutions. §7 may be filed as issues (repo §2.4). The (c) cfdb `ContextSource` (OQ-8) and CF-1/CF-2 (§11) are separate tracked work.

## §11 — Cross-fertilization (ecosystem)
The ladder maps onto cfdb's vocabulary (`H1 Context↔:Context`, `H2 Concept↔:Item`, `H3 SubConcept↔:Item`, `H4 Member↔:Field/:Variant/:Param`, `:Module`/`:Crate` containment). One vocabulary, two tools: cfdb extracts code-side containment; graph-specs enforces the spec-side ladder is coherent with it.

- **§11.1 graph-specs ← cfdb:** consumes `:Item` containment via the **ACL** (agnostic fields; translation, not verbatim adoption — PHP forces this). Queries `:Item`, never `:Concept`.
- **§11.2 cfdb ← graph-specs (paired RFCs):** **CF-1** merge cfdb's split-brained `:Context`/`:Concept` (OQ-6); **CF-2** strip `Spec:` from cfdb's own concept-file H1s; **(c)/OQ-8** add a `ContextSource` reading `specs/contexts/` Owns so cfdb's `bounded_context` reflects the multi-crate DDD grouping (one source of truth; the convergence target).
- **§11.3 agentry ← model:** **AGE-1** promote H4 pub-type declarations to H2 (6 files); **AGE-3** H1 → context identifier; agentry's per-crate keyspace already serves cfdb-query cohesion today (it *is* the one-per-crate case).
- **§11.4 cross-project invariant:** a code item's bounded context is true in **one** place and flows one direction — `cfdb/specs-contexts → context fact → ACL/source-walk → ConceptNode.context → cohesion vs H1`. No project re-derives another's knowledge.
- **§11.5 PHP (RFC-011):** *not* "nearly free" — PHP `:Item` is prop-less (edge-only containment), so the ACL needs a PHP-specific **edge-traversal** path. The ladder logic is reused; the fact-extraction path differs.
- **§11.6 the keyspace path says what it cannot read (keel-harness R1, §3.2; transcribed 2026-09-06):** on the keyspace input, a pass whose assumed shape is absent is a could-not-run naming the pass and the shape — never a clean empty, and never a finding that charges the specs with the reader's shortfall. The concept channel with no producer mark refuses (cascade-gate §10 aa, the same reading); the relationship channel reports the edge count it read, so zero edges is a stated zero; verb and impl anchors resolve through the cfdb-backed resolver, never a Rust source walk; a `- depends on:` or `- returns:` bullet on a heading whose producer emits no fact of that kind is unanswerable and says so, never `EdgeMissingInCode`; the run names the input it read and runs that input's passes only.

## §12 — Hardening trail (condensed)
Full detail in `council/rfc-010/` and this file's git history.

- **Round 1 council** (4× REQUEST CHANGES): caught the cfdb-as-database reality, the `:Item`≠`:Concept` homonym, the SubConcept rung, the SRP/CRP/violation-wrapping issues.
- **Round 2 council** (ddd-led synthesis): the converged 4-rung model + precedence + cfdb alignment + cross-fertilization (incl. the PHP-extractor check that forced **agnostic fields + ACL**, not verbatim/Conformist).
- **Dry-run 1** (3 coders, real keyspaces): proved cfdb-query is **required** (not optional) for code-context cohesion where there's no `specs/contexts/`; `cfdb-core`-only dep; struct `schema_version`; stub filtering; PHP prop-less.
- **Dry-run 2** (3 coders, §12-hardened model): **274 tests green, dogfood 0, agnostic-ACL parity 0-mismatch, real `ConceptContextMismatch` end-to-end into NDJSON.** New refinements folded into the body above: `ConceptContextMismatch` needs `spec_source` (§3.5); the §3.5 reshape rewrites the `## CohesionViolation` spec (§3.7); `violation_key` de-consts (§3.5); `from_heading_depth` avoids adapter wildcard (§3.1); `check`-exits-0 must be fixed (§3.5); cfdb-core path-dep is feature-gated (§3.3); crate-root `module_qpath` collapse + `unit`-relative-to-code-root normalization (§3.3/§3.4); no H1→concept link today so MVP uses the §3.7 filename invariant.
- **§13-A (operator-ratified):** cfdb's per-crate `bounded_context` ≠ graph-specs' multi-crate spec context. **Decision: (b)-MVP** (route each repo to the matching adapter — zero divergence) **→ (c)-clean** (paired cfdb `ContextSource` reading `specs/contexts/`; cfdb already supports multi-crate grouping, so no schema change — OQ-8). Cautious path, same destination.
