# RFC-010 Council Synthesis — DDD-led

**Author:** DDD specialist (round-2 synthesis lead)
**Date:** 2026-06-05
**Cross-consultation:** clean-arch, solid-architect, rust-systems (DMs sent; findings below integrated with independent analysis where responses not yet received)

---

## Framing correction (operator mandate)

The synthesis does NOT accommodate corpus drift. agentry's current deviations (6/50 files with no H2, H3-as-sub-concept, `core.md` not named after its context) are **signals the gate should catch**, not constraints on the design. The correct model is defined here; corpus migration is a consequence, listed at the end.

---

## Q1 — The abstraction model: is "Hn = level n" architecturally sound?

**Short answer: yes, with one clarification — depth is the primary discriminator, and role must conform to depth, not the other way around.**

### The correct ladder

```
L1  H1  =  Bounded Context     (linguistic ownership unit; one per spec file)
L2  H2  =  Concept              (one pub type; the unit the gate enforces)
L3  H3  =  Sub-concept          (a pub type nested/subordinate to an L2; diffed at L2)
L4  H4  =  Member               (method, invariant annotation, or prose sub-policy; emitted, not diffed)
```

### Why depth wins over role

The "Hn = level n" principle is sound because it is a *convention that authors must follow*, not a heuristic the tool infers. The role of a heading IS its depth in a conformant spec. Deviation — writing a pub type at H4 when it should be an H2 — is drift the gate should surface. The tool is not responsible for guessing what an H4 really means; it is responsible for enforcing that H4 means Member.

The clean model has four rungs, not three. RFC-010 collapses H3 into `Member`, but that is architecturally wrong. H3 in a conformant spec is a **sub-concept** — a pub type whose ownership and co-location are subordinate to the enclosing H2. It is diffed as a concept (L2 semantics) but reported with its parent L2 context. Collapsing H3 with H4 throws away structural information that the cohesion check needs.

### The `Member` rung (L4)

L4/H4 is methods, invariant annotations, sub-policies — prose-oriented, not code-item-oriented. It is emitted (for provenance) but not diffed. The RFC's original claim that "H3/H4 = Member" is correct for H4; incorrect for H3.

**Revised AbstractionLevel enum:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbstractionLevel {
    /// H1 — a bounded context.
    Context,
    /// H2 — a concept (one pub type, the diff unit).
    Concept,
    /// H3 — a sub-concept: a pub type subordinate to an H2 concept.
    /// Diffed at concept level with parent context from its enclosing H2.
    SubConcept,
    /// H4+ — a member: method, invariant, or sub-policy. Emitted, not diffed.
    Member,
}
```

`#[non_exhaustive]` still applies. The variant count change (3→4) is the cost of the correct model.

### What a spec file must look like (conformant)

```markdown
# <context-name>           ← H1: exactly the context name (or conforms to a specs/contexts/ entry)

## PublicTypeName           ← H2: one pub type

### NestedType              ← H3: a pub type owned by the H2 concept above

#### some_method            ← H4: a method / invariant annotation
```

A spec file with NO H2 (e.g. agentry's `boundary_signaling.md`, `secret_resolver.md`) is **malformed under the correct model**. The gate should flag it as `ContextWithoutCohesionUnit` (H1 with no H2 children). The author must migrate: promote H4 types to H2 or H3.

---

## Q2 — The "context" homonym: canonical resolver and precedence

**Three definers exist; one must be canonical and the others must delegate.**

### Which is canonical: (A) `specs/contexts/*.md`, (B) `specs/concepts/*.md` H1, or (C) `.cfdb/concepts/*.toml`?

**(A) `specs/contexts/` is canonical-upstream. This is not up for debate.**

Reason: (A) is the only form that carries machine-readable Owns/Exports/Imports — the structural declaration that makes context mapping enforceable. RFC-001 ratified this. (B) is an inference from H1 text. (C) is a crate-level heuristic from a different tool. Canonical means: when (A) exists for a context, all other definers must agree with (A) or be flagged as drift.

### The precedence rule (full specification)

1. **`specs/contexts/<name>.md` wins** if present. Its `# <name>` H1 IS the authoritative context name. Its `Owns` block IS the authoritative list of owned units. A `specs/concepts/<file>.md` H1 that names the same context is a Conformist — it conforms to (A).

2. **`specs/concepts/<file>.md` H1 is a context declaration** only when NO `specs/contexts/` file names that context. It is inferred-from-H1: the H1 text IS the context name (not a description). The inference requires that H1 text equal a context name exactly — title-case, no spaces in the context name, matching the convention (use `-` not spaces for multi-word names). The cohesion check provides the enforcement teeth: H1-declared context owns the module its H2 concepts co-locate in.

3. **`.cfdb/concepts/*.toml`** is NEVER consulted by graph-specs directly. The cfdb-query adapter reads it — but that is the adapter's concern. The adapter translates cfdb's `bounded_context` prop on `:Item` nodes into graph-specs' `ConceptNode.context` field. That translation is the adapter's ACL (Anti-Corruption Layer), not a third context definer.

### The core.md collision — prescription

`specs/concepts/core.md:1` has H1 `"Core concepts"`. This must change. The correct model DOES NOT support a single concepts file spanning multiple bounded contexts. Graph-specs-rust currently puts concepts from `equivalence`, `reading`, and `orchestration` all in one file. Under the correct model there are two conformant options:

**Option A (preferred): Split into per-context concept files.**
- `specs/concepts/equivalence.md` with H1 `# equivalence` — contains `## Graph`, `## ConceptNode`, etc.
- `specs/concepts/reading.md` with H1 `# reading` — contains `## MarkdownReader`, `## RustReader`, etc.
- `specs/concepts/orchestration.md` with H1 `# orchestration` — contains `## ReportFormat`.
- Delete `core.md`.

The `specs/contexts/<name>.md` files (A, B, C for equivalence/reading/orchestration) already exist and are authoritative. These new concept files conform to them by matching H1 name.

**Option B (acceptable): Keep `core.md` but make each H1 a context name.**

This requires `core.md` to have MULTIPLE H1s — one per context — and the markdown reader parses each H1 as introducing a new context scope. This is architecturally valid under the "H1 = context" rule, but conflicts with the standard markdown convention that a document has one H1. It is acceptable mechanically but confusing to authors.

Option A is the correct choice. The migration creates three small files (the `specs/contexts/` files already carry the Owns/Exports/Imports; the concept files carry only H1 + H2/H3 listings). The gate then dogfoods cleanly.

### H1 text = context name (NOT a description)

This must be stated as an invariant. In agentry, `# AC verifier` (filename: `ac_verifier.md`) — note the space. Context names are identifiers. The invariant: H1 text must be either (a) an exact match for a `specs/contexts/<name>.md` filename stem, OR (b) a valid identifier form (kebab-case or snake_case or PascalCase, tool normalizes). A descriptive H1 like `# Core concepts` is a reader error, not a context declaration.

---

## Q3 — cfdb vocabulary alignment

### Correct mapping (replacing RFC-010 Invariant 2)

| graph-specs concept | cfdb label/prop |
|---|---|
| `ConceptNode` (a pub type) | `:Item` node |
| `ConceptNode.name` | `:Item.props["name"]` (last segment of qname) |
| `ConceptNode.module` (proposed new field) | `:Item.props["module_qpath"]` (set by extractor at `/var/mnt/workspaces/cfdb/crates/cfdb-extractor/src/item_visitor/emit/mod.rs:262`) |
| `ConceptNode.crate` (proposed new field) | `:Item.props["crate"]` |
| `ConceptNode.context` (proposed new field) | `:Item.props["bounded_context"]` |
| Graph-specs "context" (linguistic boundary) | `:Context` node (cfdb) |
| Graph-specs "owned unit" / crate | `:Crate` node (cfdb) |

**cfdb `:Concept` is NOT graph-specs `ConceptNode`.** cfdb `:Concept` is an enrichment-overlay node assigned by `enrich_concepts` from TOML — a named business concept label, not a code item. Invariant 2 in the RFC must be rewritten to state this distinction explicitly.

### Relationship pattern: the cfdb-query adapter is Conformist

The cfdb-query adapter (`CodeFacts` port, R10-6) reads cfdb's `:Item` nodes with their `module_qpath`, `crate`, `bounded_context` props and maps them to `ConceptNode`. It adopts cfdb's vocabulary without translation. That is the **Conformist** pattern (Evans Ch. 14): the adapter conforms to cfdb's Published Language schema.

The clean-arch implication (from consultation): the cfdb-query adapter lives in `adapters/cfdb-query` (not in `domain/`); it depends on the `CodeFacts` port (in `ports/`); it does NOT pull cfdb-core as a domain-layer dependency. The adapter crate links cfdb-core; `domain/` and `ports/` do not.

This must be stated in Invariant 3 as: "The cfdb-query adapter is Conformist to cfdb's Published Language. If cfdb's `:Item` schema changes, the adapter updates — not the port."

---

## Q4 — Member/L3: keep with corrected semantics

**Keep L4/Member in this RFC, with the H3/H4 split.**

The architectural decision: the ladder has four rungs because the domain has four distinct abstraction levels in practice. Deferring to a separate RFC just delays the enum definition and forces a breaking change later. The `#[non_exhaustive]` tag means adding `SubConcept` (H3) now is non-breaking for external crates; the diff engine adds one new match arm (for SubConcept → treated as L2 in the cohesion check).

`Member` (H4) is emitted-not-diffed in this RFC. That stays correct. The OQ-2 question (how L4 diffing reconciles with verb-anchoring) is genuinely deferred and nothing in the four-rung enum forecloses it.

---

## Converged model summary

```
Abstraction ladder:
  H1 = L1 = Context       (bounded context; H1 text IS the context name as identifier)
  H2 = L2 = Concept        (pub type; the diff unit)
  H3 = L3 = SubConcept     (pub type nested under L2; diffed at L2 granularity)
  H4 = L4 = Member         (method/invariant; emitted, not diffed)

Context resolver precedence:
  (A) specs/contexts/*.md  — canonical-upstream (has Owns/Exports/Imports)
  (B) specs/concepts/*.md H1  — inferred context when no (A) present; name must be identifier
  (C) cfdb .cfdb/concepts/*.toml  — adapter-internal; never consulted directly by graph-specs

cfdb vocabulary:
  ConceptNode  ↔  :Item  (NOT :Concept)
  module       ↔  :Item.module_qpath
  crate        ↔  :Item.crate
  context      ↔  :Item.bounded_context
  cfdb-query adapter relationship to cfdb: Conformist

AbstractionLevel enum: 4 variants (Context/Concept/SubConcept/Member), all #[non_exhaustive]

Violation wrapping: cohesion violations wrap inside Violation::Context(CohesionViolation)
  — a new CohesionViolation enum distinct from ContextViolation (which covers RFC-001
  cross-context edge violations). This is the SOLID/ISP answer: separate the two
  violation families so consumers can match only what they need.
```

---

## Required corpus migrations (consequences of the correct model)

### graph-specs-rust own specs/

1. **Split `specs/concepts/core.md`** into three per-context concept files:
   - `specs/concepts/equivalence.md` (H1: `# equivalence`)
   - `specs/concepts/reading.md` (H1: `# reading`)
   - `specs/concepts/orchestration.md` (H1: `# orchestration`)
   Each H1 matches its corresponding `specs/contexts/<name>.md` file.

2. **Add `AbstractionLevel`, `CodeFacts`, `CohesionViolation`, `SubConcept`** to `specs/concepts/equivalence.md` as new H2 entries (required for self-dogfood at 0 violations).

### agentry migrations (consequences — not gate-blocked until agentry opts into v3 checks)

3. **Files with H1 but no H2** (`boundary_signaling.md`, `fsm_merge_rail.md`, `git_operator.md`, `refusal.md`, `secret_resolver.md`, `secrets.md`): promote H4 types to H2 or H3. The gate will emit `ContextWithoutCohesionUnit` on these files once v3 checks are active.

4. **Files using H4 for pub types** (`secret_resolver.md` — `#### SecretResolver`, `#### OrgKey`, etc.): promote to H2. These are L2 concepts authored at H4; they must move to H2 for the ladder to be load-bearing.

5. **Files using H3 for sub-concepts** (`captain_cli.md` — `### UnsatisfiedRolePrecondition`, `### QueryResults`): these are ALREADY CORRECT under the four-rung model. H3 = SubConcept = diffed as L2. No migration needed; the four-rung model handles these correctly.

6. **H1 text must be identifier-form context names** for any file that lacks a matching `specs/contexts/` file. Files like `# AC verifier` (with a space) may need normalization — the tool should normalize to `ac_verifier` / `ac-verifier` per a stated convention, OR require authors to use `# AcVerifier` or `# ac-verifier` in H1.

---

## Cross-lens action items for RFC revision

| Item | Lens | Action |
|---|---|---|
| Rename `AbstractionLevel::Member` to add `SubConcept` variant for H3 | DDD + SOLID | RFC §3.1 revision |
| Rewrite Invariant 2: `ConceptNode ↔ :Item`, not `:Concept` | DDD + Rust-systems | RFC §4 Invariant 2 |
| State cfdb-query adapter is Conformist | DDD | RFC §3.3 + Invariant 3 |
| Prescribe core.md split in RFC §3.7 self-dogfood | DDD + Clean-arch | RFC §3.7, §7 R10-1 |
| H1 text must be identifier-form (not descriptive title) | DDD | RFC §3.2 + new invariant |
| `CohesionViolation` as separate wrapper from `ContextViolation` | SOLID + DDD | RFC §3.5 |
| Context resolver precedence: (A)>(B)>(adapter) | DDD | RFC §3.4 + OQ-1 resolution |
| Three-file split in §7 issue decomposition | DDD + Clean-arch | RFC §7 R10-1 expanded |

---

## Cross-fertilization — ecosystem architecture

The council is the oracle. The converged model defines the correct vocabulary. All three projects adopt it. This section states what each project gains and what it must change.

---

### The unified containment vocabulary (shared by all three projects)

The converged ladder maps directly onto cfdb's node vocabulary. This is not a coincidence — cfdb's containment graph IS the code-side realization of the abstraction ladder:

```
Spec side (graph-specs)        Code side (cfdb graph labels)
─────────────────────          ──────────────────────────────
H1  Context                 ↔  :Context  (name, owning crates via BELONGS_TO)
H2  Concept (pub type)      ↔  :Item     (qname, module_qpath, crate, bounded_context)
H3  SubConcept (nested type)↔  :Item     (same label; discriminated by depth provenance)
H4  Member (method/inv.)    ↔  :Field / :Variant / :Param  (or absent for prose-only)
    —                       ↔  :Module   (structural containment unit)
    —                       ↔  :Crate    (owned unit)
```

graph-specs' `ConceptNode` maps to cfdb's `:Item`. graph-specs' `OwnedUnit` maps to cfdb's `:Crate`. graph-specs' bounded context name maps to cfdb's `:Context.name`.

**One vocabulary. Two tools. Each provides what the other lacks**: cfdb extracts the code-side containment facts; graph-specs enforces that the spec-side abstraction ladder is coherent with those facts.

---

### graph-specs adopts from cfdb

1. **Containment vocabulary for `ConceptNode`.** The `module`, `crate`, `context` fields proposed in RFC-010 adopt cfdb's prop names directly: `module_qpath` → `ConceptNode.module`, `:Item.crate` → `ConceptNode.crate`, `:Item.bounded_context` → `ConceptNode.context`. No renaming, no translation. This is the Conformist pattern: graph-specs' cfdb-query adapter adopts cfdb's `:Item` property vocabulary.

2. **The cfdb-query `CodeFacts` adapter (R10-6) queries `:Item` nodes, not `:Concept` nodes.** Confirmed: cfdb's `:Concept` is an enrichment-overlay node whose `name` IS the context name (not a pub-type name). graph-specs cannot use `:Concept` as its data source without misreading context names as type names.

3. **Correction to RFC-010 Invariant 2** (required before ratification): rewrite to state the `:Item`/`:Concept` distinction explicitly, as detailed in Q3 above.

---

### cfdb adopts from graph-specs

RFC-010's ratification implies two consequences for cfdb:

**CF-1 [cfdb advisory, not a blocker for RFC-010]: Rationalize the `:Context` / `:Concept` split.**

cfdb currently emits both `:Context` nodes (from the extractor — one per bounded context, linked to `:Crate` via `BELONGS_TO`) AND `:Concept` nodes (from `enrich_concepts` — one per bounded context, linked to `:Item` via `LABELED_AS`/`CANONICAL_FOR`). Both node types carry `name` = the bounded-context name. They represent the SAME semantic entity (a bounded context) at different enrichment layers.

In Evans' model, this is a split-brain: two nodes representing the same aggregate root. The clean design is one `:Context` node that accumulates both the `BELONGS_TO` (crate containment) and `LABELED_AS` (item labeling) edges. cfdb should merge `:Concept` into `:Context`.

The migration: `enrich_concepts` emits `LABELED_AS` edges from `:Item` to `:Context` (not to a separate `:Concept` node). The `:Concept` label is retired. This is a schema bump in cfdb (`SchemaVersion` patch, lockstep PR with graph-specs per the existing protocol).

Why this is a graph-specs consequence: once graph-specs' cfdb-query adapter reads `:Context` for context provenance (which it already should, since cfdb's `:Context` carries the bounded-context name), there is no need for cfdb's separate `:Concept` overlay node. The query becomes: `(:Item)-[:IN_MODULE]->(:Module), (:Item.crate = :Crate.name), (:Crate)-[:BELONGS_TO]->(:Context)` — a pure structural traversal without the enrichment-time overlay.

**CF-2 [cfdb advisory]: cfdb's `specs/concepts/` H1 convention must conform to the ladder model.**

cfdb uses `# Spec: cfdb-core` as H1 for all its concept files (`/var/mnt/workspaces/cfdb/specs/concepts/cfdb-core.md:1`, `/var/mnt/workspaces/cfdb/specs/concepts/cfdb-concepts.md:1`). The converged model requires H1 text to be an identifier-form context name, not a descriptive title. cfdb's H1 text must change to match the bounded-context name: `# cfdb-core` (since cfdb-core IS its own bounded context per the `.cfdb/concepts/cfdb.toml` TOML structure, or more precisely the context is `"cfdb"` if the TOML declares it so).

cfdb has no `specs/contexts/` directory. Per the converged model's precedence rule, that means the H1 of each `specs/concepts/*.md` file IS the authoritative context declaration. The migration for cfdb is therefore: strip the `"Spec: "` prefix from every H1 — the context name IS the crate name (conformant to cfdb's own `.cfdb/concepts/*.toml` naming). This is a mechanical migration, not a design change.

---

### agentry adopts from the converged model

agentry is the primary dogfood target for RFC-010. The converged model implies the following migrations, in priority order:

**AGE-1 [required for v3 gate to pass — post-RFC-010 ship]:** Six files have H1 but zero H2 headings:
- `/var/mnt/workspaces/agentry/specs/concepts/boundary_signaling.md` (6 H4s)
- `/var/mnt/workspaces/agentry/specs/concepts/fsm_merge_rail.md` (6 H4s)
- `/var/mnt/workspaces/agentry/specs/concepts/git_operator.md` (9 H4s)
- `/var/mnt/workspaces/agentry/specs/concepts/refusal.md` (1 H4)
- `/var/mnt/workspaces/agentry/specs/concepts/secret_resolver.md` (16 H4s — many are pub-type declarations)
- `/var/mnt/workspaces/agentry/specs/concepts/secrets.md` (3 H4s)

For each file: determine which H4s are pub-type declarations (promote to H2) and which are prose sub-policies (leave at H4 under the promoted H2). `secret_resolver.md` is the most work — 8 H4 entries (`#### SecretResolver`, `#### CompositionRoot`, `#### ResolveCtx`, `#### ResolvedSecrets`, `#### ResolveError`, `#### OrgKey`, `#### ProjectKey`, `#### DeriveProjectKey`) are pub-type declarations that must become H2. The remaining H4s (`#### Architecture`, `#### Resolution flow`, etc.) are prose and stay at H4.

The gate will fire `ContextWithoutCohesionUnit` for these files under v3 checks.

**AGE-2 [conformant already — no migration needed]:** Files using H3 for sub-concepts (e.g., `captain_cli.md:123` `### UnsatisfiedRolePrecondition`, `### QueryResults`, `### IntentHit`, `### LessonHit`) are CORRECT under the four-rung model. H3 = SubConcept = diffed at L2. No migration.

**AGE-3 [required for correct context name resolution]:** agentry has no `specs/contexts/` directory. The converged model's B-path (H1 as inferred context) applies. The H1 text must be an identifier-form context name. All 50 files use `# <Name>` with PascalCase or snake_case spacing (e.g., `# AC verifier` with a space, `# Brief contract` with a space). The tool must define its normalization rule: either authors write `# ac-verifier` (kebab-case) and the tool accepts it, or the tool normalizes `# AC verifier` → `"ac-verifier"` by lowercasing and replacing spaces with hyphens. The normalization rule must be stated in RFC-010 §3.2 and in `specs/dialect.md`.

**AGE-4 [recommended — aligns agentry with three-project ecosystem]:** agentry has no `.cfdb/concepts/*.toml` files. Adding them would allow cfdb's `enrich_concepts` to emit `LABELED_AS` edges for agentry's bounded contexts. The converged model's CF-1 proposal (merge `:Concept` into `:Context`) makes this moot if cfdb adopts it — `BELONGS_TO` edges already link `:Crate` to `:Context` without TOML-declared concepts. Either way, agentry's cfdb keyspace already carries `bounded_context` on every `:Item` via the crate-prefix heuristic in `cfdb-concepts`. The cfdb-query `CodeFacts` adapter can serve agentry without any `.toml` addition.

---

### Summary table

| Project | Adopts | Changes forced |
|---|---|---|
| **graph-specs** | cfdb containment vocabulary (`:Item` props as `ConceptNode` fields); Conformist to cfdb Published Language | Rewrite Invariant 2; split `core.md` into per-context files; add `SubConcept` variant; name cfdb-query adapter Conformist |
| **cfdb** | graph-specs' spec-ladder coherence discipline (H1=context, H2=pub type); four-rung abstraction vocabulary | Rationalize `:Context`/`:Concept` split (CF-1 — advisory, separate RFC); strip `"Spec: "` prefix from all H1s (mechanical migration) |
| **agentry** | Four-rung abstraction ladder; H1=context identifier | Promote H4 pub types to H2 in 6 files (AGE-1); define H1 normalization rule (AGE-3) |

---

### The cross-project invariant that ties the model together

A fact about a code item's bounded context is TRUE in exactly ONE place and flows in ONE direction:

```
cfdb extractor → :Item.bounded_context  (the authoritative code-side fact)
       ↓  (cfdb-query CodeFacts adapter, Conformist)
graph-specs ConceptNode.context          (the spec-checking consumer)
       ↓  (cohesion violation if mismatches H1)
specs/concepts/<name>.md H1              (the spec-side declaration)
```

No project re-derives what another project already knows. cfdb knows the code containment. graph-specs knows the spec abstraction. The adapter translates. The gate enforces the match.


---

## Rust-systems consultation — confirmed findings

*Response received from rust-systems lens; integrated here.*

**R10-6 implementation precision: crate-root `module_qpath` convention.**

cfdb `:Item` nodes carry `module_qpath` as a prop even for items at the crate root (`/var/mnt/workspaces/cfdb/crates/cfdb-extractor/src/item_visitor/emit/mod.rs:140–177`). However, at the crate root (`module_stack.len() == 1`), `emit_in_module_edge` is a no-op — no `IN_MODULE` edge is emitted, only `IN_CRATE`. The `module_qpath` prop for a crate-root item equals the crate name with no `::` separator (e.g., `"domain"` not `"domain::some_mod"`).

The source-walking `CodeFacts` adapter must produce the same convention for the R10-6 parity test to pass. Specifically: when a `ConceptNode` is emitted for a pub type in a crate's `src/lib.rs` or `src/main.rs` with no enclosing `mod` block, `ConceptNode.module` should equal `ConceptNode.crate` (the crate name). This rule must be stated in the R10-6 issue body to prevent a parity failure on crate-root items.

**`ConceptNode` field-add: zero application-layer churn, confirmed.**

`application/src/` has zero `ConceptNode` destructure sites. The application layer accesses `ConceptNode` only transitively via `Violation` variants. Adding `module: Option<String>` and `crate_name: Option<String>` requires `None` fill-ins at construction sites in `adapters/rust/src/lib.rs` and `adapters/markdown/src/lib.rs`, plus ~10 domain test helpers — all in adapter/domain crates, not in `application/`. This is additive-with-Option: no breaking churn.


---

## Rust-systems round-2 constraints — locked for implementation

*Three constraints that must be stated in the RFC before implementation begins, to prevent dry-run failures.*

### Constraint 1: CohesionViolation wrapping shape and violation_key rank — locked in R10-1

The current `violation_key` function at `domain/src/diff.rs:120` holds ranks 0–11. `Violation::Context(ContextViolation)` occupies rank 8. The next free rank is 12.

**Decision: wrap the four cohesion violations in `Violation::Cohesion(CohesionViolation)` at rank 12**, mirroring the `Violation::Context(ContextViolation)` pattern exactly. This is the correct SOLID/ISP choice (rust-systems ratifies; SOLID lens to confirm): consumers that do not opt into cohesion checking match one new arm, not four. The `CohesionViolation` enum carries a `context: String` field on every variant (parallel to `ContextViolation`'s `concept: String`) so the sort key accessor delegates cleanly:

```rust
Violation::Cohesion(coh) => (coh.context(), 12),
```

The flat 4-arm alternative (ranks 12–15) also compiles but fragments the opt-in surface and violates the established wrapping precedent. Flat is rejected.

This shape MUST be committed in R10-1 (domain types slice) because the `violation_key` `const fn` is exhaustive at compile time — all match arms must exist before any downstream crate compiles. R10-3 and R10-4 add no new `Violation` variants; they implement the logic that produces `Violation::Cohesion(_)` values.

### Constraint 2: module_qpath derivation — R10-3 must replace owning_unit_str

The existing `owning_unit_str` function at `domain/src/diff/context.rs:204–214` extracts only the **crate directory** (everything before `/src/`). For example, `"domain/src/diff/context.rs"` → `"domain"`. This is crate-granular, not module-granular.

RFC-010 intends cohesion at **module granularity** (`module_qpath` = `"domain::diff::context"`, not just `"domain"`). If R10-3 reuses `owning_unit_str` unchanged, `ContextOwnsScatteredConcepts` fires only when concepts scatter across **crates**, not across modules within a crate — substantially weaker than the RFC's stated intent.

R10-3 must add a `module_qpath_from_source` helper that derives the full module qpath from a `Source::Code { path, .. }` value:

```rust
fn module_qpath_from_source(path: &Path, crate_name: &str) -> String {
    // 1. strip workspace-root prefix and leading "./"
    // 2. strip "<crate_name>/src/" prefix
    // 3. strip trailing filename (.rs)
    // 4. replace "/" with "::"
    // 5. if result is empty (file was src/lib.rs or src/main.rs) → crate_name
    // 6. prepend crate_name + "::" otherwise
}
```

The convention for crate-root items (step 5) must match cfdb's: `module_qpath = crate_name` when the item lives directly in `src/lib.rs` or `src/main.rs`. This is load-bearing for the R10-6 parity test: the cfdb-query adapter returns `module_qpath = "domain"` for a crate-root item; the source-walking adapter must return the same string.

`owning_unit_str` is used in the existing v0.4 context pass (membership check). It stays as-is for that pass. The new cohesion pass (R10-3) uses `module_qpath_from_source` for finer-grained grouping.

### Constraint 3: cfdb-query adapter — path dep, feature-gated, post-R10-3

The cfdb-query `CodeFacts` adapter (R10-6) requires path dependencies to four cfdb crates (`cfdb-core`, `cfdb-petgraph`, `cfdb-query`, `cfdb-concepts`) plus transitives (`petgraph`, `chumsky`, `regex`, `indexmap`). The path dep approach is correct — cfdb has `publish = false` and the repos are co-located at `/var/mnt/workspaces/cfdb`.

The adapter lives in a new `adapters/cfdb-query` crate. The `application` crate activates it via a Cargo feature (e.g. `features = ["cfdb-query"]`). This keeps the default build standalone (no cfdb dep) and makes the cfdb path dep opt-in, exactly as §3.8 intends.

The keyspace file (`<workspace>/.cfdb/db/<name>.json`) must exist before `graph-specs check --code-facts=cfdb:<name>` runs. CI is fine (cfdb extract runs first in `arch.yml`). Local dev needs a clear error if the keyspace is absent — not a silent fall-back to source-walking, which would mask a misconfigured CI.

R10-6 is correctly sequenced after R10-3: the cohesion rule must be proven correct against the source-walking adapter before the cfdb-query adapter is added as a second code-path.


---

## SOLID round-2 constraints — locked for implementation

*Prescriptions received from SOLID lens; one correction applied from source verification.*

### A. SRP: Two-pass markdown reader — locked for R10-2

The markdown reader gains a second parser pass for heading-tree extraction. The existing `SectionState`/`handle_event` machinery is NOT touched.

New type: `ContextTreeState` with fields `current_context: Option<String>`, `parent_links: Vec<(concept_name, context_name)>`, `heading_buf`, `current_level`. New function: `extract_context_tree_from_source` — a fresh `Parser::new(source).into_offset_iter()` call per file, handling only `Event::Start(Tag::Heading)`, `Event::End(TagEnd::Heading)`, `Event::Text`. The exact precedent is `extract_annotations_from_source` at `adapters/markdown/src/lib.rs:473–474`.

Integration in `MarkdownReader::extract`: Pass 1 (unchanged) produces `Vec<ConceptNode>`; Pass 2 produces `Vec<(concept_name, context_name)>` parent links; post-processing attaches context provenance. No shared mutable state between passes.

R10-2 issue body must specify: "implement `ContextTreeState` + `extract_context_tree_from_source` as a separate parser pass; do not modify `SectionState` or `handle_event`; verify with ra-query that both score ≤ 14 post-implementation."

### B. Violation taxonomy — locked, converged with rust-systems

`Violation::Cohesion(CohesionViolation)` at rank 12. Four variants, using the compact register matching existing `ContextViolation` naming style:

```rust
pub enum CohesionViolation {
    ScatteredConcepts {
        context: String,
        modules: Vec<String>,
        spec_source: Source,
    },
    MisfiledConcept {
        concept: String,
        declared_context: String,
        actual_unit: String,
        spec_source: Source,
        code_source: Source,
    },
    UnitlessContext {
        context: String,
        spec_source: Source,
    },
    SplitUnit {
        unit: String,
        contexts: Vec<String>,
        code_source: Source,
    },
}
```

`CohesionViolation` lives in `domain/src/context.rs` alongside `ContextViolation`. It exposes `context_name() -> &str` as a `const fn` method (parallel to `ContextViolation::concept()`). `violation_key` gains one arm: `Violation::Cohesion(coh) => (coh.context_name(), 12)`. Committed in R10-1 — `violation_key` is exhaustive at compile time; all downstream crates depend on it.

### C. Module granularity: file-relative path, not inline-mod-stack — locked for R10-3

The canonical `container` value (source-walking adapter) is the **file-relative module path**, derived from the source file path:

```
container = strip_prefix(src_root) → strip_suffix(".rs") → replace('/', "::") → prepend crate_name + "::"
# lib.rs / main.rs edge case: result is empty after strip → container = crate_name
```

This matches the one-file-per-module convention graph-specs already relies on (`adapters/rust/src/lib.rs:11`). It does NOT require tracking inline `mod` nesting depth.

**Topology correction (from source verification):** SOLID proposed traversing `(:Item)-[:IN_MODULE]->(:File)` in the cfdb-query adapter. That edge does not exist in cfdb's graph. The actual edges are:

- `(:Item) -[:IN_MODULE]-> (:Module)` — for items in nested modules (via `emit_in_module_edge`)
- `(:File) -[:IN_MODULE]-> (:Module)` — for the file node (via `file_walker.rs:118–127`)
- `(:Item) -[:IN_CRATE]-> (:Crate)` — for crate membership

The cfdb-query adapter gets file granularity by reading **`:Item.file`** and **`:Item.module_qpath`** props directly from the `:Item` node — no edge traversal required. Both props are set by the extractor on every `:Item` node (`cfdb-core/src/schema/describe/nodes.rs:105,112`). The parity guarantee is sound: source-walking adapter derives `container` from `Source::Code { path }` using the file-path formula above; cfdb-query adapter reads `:Item.module_qpath` directly. Both produce the same string for single-file modules.

### D. CodeFacts port: feature-gated in `ports`, minimal dep chain — locked

The `CodeFacts` trait is compiled only when the `codefacts` Cargo feature is enabled in `ports/Cargo.toml`:

```toml
[features]
codefacts = []
```

`adapters/cfdb-query` enables `ports = { ..., features = ["codefacts"] }`. Adapters that do not implement `CodeFacts` do not enable the feature. This avoids making `CodeFacts` mandatory for the existing `RustReader`/`MarkdownReader` ecosystem.

**Dep chain correction (from source verification):** The cfdb-query adapter does NOT need petgraph or chumsky. The keyspace JSON (`persist.rs:36–40`) deserializes to `KeyspaceFile { nodes: Vec<Node>, edges: Vec<Edge> }` using only cfdb-core types and serde_json. The `PetgraphStore::load` path uses petgraph for graph indexing, but the cfdb-query adapter can deserialize the same JSON directly via `serde_json::from_slice::<KeyspaceFile>(&bytes)` into cfdb-core's `Node`/`Edge` types, then iterate the flat `Vec`s in memory. No graph engine needed for simple prop reads.

Minimum dep chain for `adapters/cfdb-query`:
- `cfdb-core` (path dep, `publish = false`)
- `serde_json`
- `domain` (internal)
- `ports` (internal, `features = ["codefacts"]`)

cfdb-core has no petgraph in its `[dependencies]` (confirmed: `/var/mnt/workspaces/cfdb/crates/cfdb-core/Cargo.toml` — serde_json is the only non-workspace dep). This keeps cold-build time under control.

Invariant 3 text: "domain and ports link no cfdb crate; `adapters/cfdb-query` is the single authorized point of cfdb-core linkage in the graph-specs workspace."


---

## Clean-arch round-2 constraints — integrated with one correction

*Response received from clean-arch lens; one naming correction applied from source verification.*

### CA-1: Context resolver — pure function in domain/src/context.rs

The A-wins-over-B precedence rule is a business rule, not a composition concern. It belongs in `domain/src/context.rs` alongside `context_for_concept` (line 238) and `detect_import_cycle`.

The matching criterion: an explicit `ContextDecl` from source (A) claims ownership if its `owned_units` covers the spec file path that the H1-inferred context (B) was read from. This is a pure set-membership check over two `Vec<ContextDecl>` values — zero I/O, no string comparison between two sources. The function signature:

```rust
pub fn resolve_context_precedence<'a>(
    explicit: &'a [ContextDecl],   // from specs/contexts/
    inferred: &'a [ContextDecl],   // from specs/concepts/ H1s
) -> Vec<&'a ContextDecl>
```

Returns the winning `ContextDecl` per concept file: explicit wins when its `owned_units` contains the spec path; inferred wins when no explicit entry matches. The application layer assembles both slices and calls this function; domain decides the winner. The rule does NOT compare context name strings — it compares path membership, which is why the `core.md` collision was a symptom of a missing path-ownership declaration, not a string mismatch.

### CA-2: ConceptNode field names — language-agnostic domain vocabulary (not cfdb prop names)

Clean-arch's prescription that `module_qpath`/`crate_name`/`bounded_context` should align with cfdb's `:Item` prop names is **partially correct** but does not survive the polyglot test.

Source verification shows the cfdb PHP extractor (`cfdb-extractor-php/src/lib.rs:255–266`) does NOT set `module_qpath`, `crate`, or `bounded_context` props on its `:Item` nodes. Those props are Rust-extractor-specific. The cfdb-query adapter therefore cannot read `:Item.module_qpath` for PHP keyspaces — it must traverse `IN_MODULE`/`IN_CRATE` edges for PHP containment.

This means the `ConceptNode` provenance fields are graph-specs' Published Language and must use graph-specs' own domain vocabulary, not cfdb's internal Rust prop names. The cfdb-query adapter is an ACL (Anti-Corruption Layer) that translates cfdb's language-specific internals into graph-specs' stable language-agnostic vocabulary at the adapter boundary.

**Authoritative field names for `ConceptNode` (resolving SOLID vs clean-arch disagreement):**

```rust
pub struct ConceptNode {
    pub name: String,
    pub source: Source,
    pub signature: SignatureState,
    // RFC-010 additions:
    pub module_path: Option<String>,   // :: qualified scope path (Rust mod, PHP namespace, …)
    pub unit: Option<String>,          // owning unit name — crate, package, or module (aligns with OwnedUnit)
    pub context: Option<String>,       // bounded context name (aligns with ContextDecl::name)
}
```

- `module_path` — the `::` qualified scope path. For Rust this is `module_qpath`; for PHP this is the namespace path (derived from `IN_MODULE` edge traversal). The name is domain-level, not language-level.
- `unit` — aligns with the existing `OwnedUnit(String)` vocabulary in `domain/src/context.rs:17`. Intentionally abstract: Rust crate, PHP composer package, future language unit.
- `context` — the bounded context name. Direct string; aligns with `ContextDecl::name`.

The cfdb-query adapter populates these by:
- `module_path`: reads `:Item.module_qpath` for Rust keyspaces; traverses `(:Item)-[:IN_MODULE]->(:Module {qpath})` for PHP keyspaces where the prop is absent.
- `unit`: reads `:Item.crate` for Rust; `(:Item)-[:IN_CRATE]->(:Crate {name})` edge for PHP.
- `context`: reads `:Item.bounded_context` for Rust; derives from `(:Crate)-[:BELONGS_TO]->(:Context {name})` for PHP.

The source-walking adapter populates `module_path` via the `module_qpath_from_source` formula locked in rust-systems constraint 2. `unit` is the crate name extracted by `owning_unit_str`. `context` is populated post-resolution by `resolve_context_precedence`.

**Note on SOLID's `unit`/`container` names:** SOLID's `unit` maps correctly to this synthesis (`unit` = OwnedUnit-aligned). SOLID's `container` maps to `module_path` here. The renaming is purely cosmetic from SOLID's perspective — the semantics are identical.

