# RFC-001 — v0.4 bounded-context equivalence

- **Status:** RATIFIED (2026-04-19, round 2)
- **Authors:** Claude (session 2026-04-19)
- **Supersedes:** —
- **Superseded by:** —
- **Tracking issue:** —

## §1 — Problem

graph-specs-rust today checks equivalence at three levels: concept (v0.1), signature (v0.2), and declared relationship (v0.3). All three operate on a **flat** concept graph — every `pub` type in the workspace is a node in the same graph. The tool has no notion of "these types belong to context A; those belong to context B; the boundary between A and B is a contract."

This creates three concrete gaps:

1. **Cross-context drift is invisible.** A new `depends on:` edge from a type in `domain` to a type in `adapter-markdown` is a hexagonal-boundary violation, but today it reads to graph-specs as just another declared edge.
2. **Context ownership is implicit.** There is no machine-readable statement "context `reading` owns MarkdownReader + RustReader."
3. **Study 002 v4.2 §6 Level 4 + §41 Phase E is blocked on us.** qbot-core's anti-drift pipeline cites bounded-context equivalence as the capability that makes cross-context contract enforcement possible.

DDD names this enforcement surface "context mapping" — Evans Chapter 14 — and enumerates seven stable patterns: Shared Kernel, Customer-Supplier, Conformist, Anti-Corruption Layer, Separate Ways, Open Host Service, Published Language. Graph-specs does not need to implement all seven; it needs a vocabulary rich enough to express "this cross-context reference is sanctioned" vs "this cross-context reference is drift."

## §2 — Scope

**Ships in v0.4:**

- Spec-side vocabulary for declaring bounded contexts, **Exports** (supplier-side Published Language), **Imports** (consumer-side references), and **owned units** (crates in a Cargo workspace)
- Four DDD patterns: **Shared Kernel**, **Customer-Supplier**, **Conformist**, **Published Language** (Conformist added per DDD lens round-1 RC-1)
- Three new violation variants wrapped under `Violation::Context(ContextViolation)`: `ContextMembershipUnknown`, `CrossContextEdgeUnauthorized`, `CrossContextEdgeUndeclared`
- A new `ContextReader` port; markdown adapter implements both `Reader` and `ContextReader`
- NDJSON schema bumped to `schema_version: "2"` (version-explicit per rust-systems lens round-1 RC-2)
- Self-dogfood: graph-specs-rust declares its own bounded contexts

**Deferred to v0.5:**

- **Anti-Corruption Layer**, **Separate Ways**, **Open Host Service** patterns
- Semantic inference of context membership from crate layout (membership stays declared)
- Migration tooling that infers context declarations from existing spec trees

**Out of scope for this RFC (but related, tracked as follow-up):**

- Exit-code taxonomy refinement (Study 002 v4.2 §14 integer pinning)
- Determinism / performance guarantees in CI (Study 002 v4.2 §41 A3 budget)
- `check --format=ndjson` filter/query flags (Study 003 v2 §15 C2/C6)

## §3 — Design

### §3.1 — Spec surface

A new directory `specs/contexts/` holds one markdown file per bounded context. Filename is the canonical context name (`specs/contexts/reading.md`, etc.).

Every context file has four required sections:

```markdown
# <ContextName>

## Owns

- unit: domain
- unit: ports

## Exports (Published Language — what this context publishes)

- Graph (Published Language)
- ConceptNode (Published Language)
- Edge (Shared Kernel)

## Imports (sanctioned cross-context references)

- from: equivalence
  patterns:
    - Published Language: Graph
    - Published Language: ConceptNode

## Concepts

(references concepts in specs/concepts/ that live in this context — derived from the `Owns` block)
```

**Vocabulary changes from round-1:**

- `Owns` uses `unit:` (not `crate:`) per clean-arch lens RC-1 — the same word covers Cargo crates, npm packages, Go modules, or any future language-specific "owned unit."
- `Exports` is required (not just `Imports`) per DDD lens RC-2 — Published Language is export-centric; the supplying context declares its stable language, importers reference it.
- Import entries name a single pattern per entry (not a list) — the `patterns:` block holds one entry per (pattern, concept) pair for clarity.

### §3.2 — Violation variants

Three new variants, wrapped under a new `ContextViolation` enum (SOLID lens round-1 RC-2):

```rust
#[non_exhaustive]
pub enum ContextViolation {
    MembershipUnknown {
        concept: String,
        owned_unit: OwnedUnit,
        code_source: Source,
    },
    CrossEdgeUnauthorized {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
    CrossEdgeUndeclared {
        concept: String,
        owning_context: String,
        edge_kind: EdgeKind,
        target: String,
        target_context: String,
        spec_source: Source,
    },
}
```

`Violation` gains a single new variant:

```rust
pub enum Violation {
    // ... existing 8 variants unchanged ...
    Context(ContextViolation),
}
```

Consumers that do not opt into context checking match one new arm. Consumers that do opt in can match exhaustively on `ContextViolation`.

Every `ContextViolation` variant carries a `concept: String` field (rust-systems RC-1) so `violation_key()`'s `const fn` sort key continues to work without allocation or destructuring changes.

All three are fatal (exit code 1), matching existing v0.3 variants.

### §3.3 — NDJSON schema

**Schema version bumps to `"2"`** (rust-systems RC-2). Rationale: consumers using `#[serde(tag = "violation")]` without `#[serde(other)]` fail on unknown variant names. A version bump gives downstream an unambiguous gate: "if `schema_version == "1"`, use v1 enum; if `"2"`, use v2 enum."

All existing v1 record shapes keep their structure with `schema_version: "2"`. Three new record shapes:

```json
{"schema_version":"2","violation":"context_membership_unknown","concept":"Foo","owned_unit":"some-crate","source":{"kind":"code","path":"some-crate/src/lib.rs","line":3}}
{"schema_version":"2","violation":"cross_context_edge_unauthorized","concept":"MarkdownReader","owning_context":"reading","edge_kind":"DEPENDS_ON","target":"TradingPort","target_context":"trading","spec_source":{"kind":"spec","path":"specs/concepts/reader.md","line":12}}
{"schema_version":"2","violation":"cross_context_edge_undeclared","concept":"MarkdownReader","owning_context":"reading","edge_kind":"DEPENDS_ON","target":"Graph","target_context":"equivalence","spec_source":{"kind":"spec","path":"specs/concepts/reader.md","line":12}}
```

`specs/ndjson-output.md` is updated to document the v2 variants. v1 records continue to be produced by v0.3 callers; v2 records are produced by v0.4 callers. The tool emits one version per run (whichever the caller's tree supports), never mixing.

### §3.4 — CLI surface

No new flags. `--format={text,ndjson}` gains three new violation shapes in each format.

### §3.5 — Exit codes

Unchanged. v0.4 violations all map to exit code 1.

### §3.6 — Reader ports

A new port, `ContextReader`, in `ports/src/lib.rs` (clean-arch RC-2):

```rust
pub trait ContextReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError>;
}
```

The markdown adapter implements both `Reader` (concepts + edges from `specs/concepts/`) and `ContextReader` (contexts from `specs/contexts/`). The rust adapter implements only `Reader`.

Application wires both ports at the composition root:

```rust
pub fn run_check(specs: &Path, code: &Path) -> Result<Vec<Violation>, ReaderError> {
    let specs_graph = MarkdownReader.extract(specs)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs)?;
    let code_graph = RustReader.extract(code)?;
    Ok(diff(CheckInput::new(specs_graph, spec_contexts), code_graph))
}
```

**Parser non-generalization (rust-systems RC-5).** `parse_context_file()` is a new independent state machine. It reuses only `compute_line_starts` and `line_of_offset` from the existing concept parser, extracted into a shared `markdown_utils` submodule. `SectionState` is not generalized — its existing `H2|H3 + fenced rust + bullet-prefix` dispatch is not the right shape for context files (which have `H1 name + three H2 subsections with structured list syntax`).

### §3.7 — Domain changes

Six additions in `domain::`:

```rust
/// A crate, npm package, Go module, or equivalent — the thing a context "owns."
/// Named deliberately to keep the domain model language-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedUnit(pub String);

/// Declaration of a bounded context.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextDecl {
    pub name: String,
    pub owned_units: Vec<OwnedUnit>,
    pub exports: Vec<ContextExport>,
    pub imports: Vec<ContextImport>,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExport {
    pub concept: String,
    pub pattern: ContextPattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextImport {
    pub from_context: String,
    pub pattern: ContextPattern,
    pub concept: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextPattern {
    SharedKernel,
    CustomerSupplier,
    Conformist,
    PublishedLanguage,
    // AntiCorruptionLayer, SeparateWays, OpenHostService deferred to v0.5
}
```

**`Graph` is unchanged** (SOLID RC-1 + clean-arch RC-3). `ContextDecl`s do NOT live inside `Graph`. Instead:

```rust
/// Input to the v0.4 diff — concept graph plus optional bounded-context map.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckInput {
    pub graph: Graph,
    pub contexts: Vec<ContextDecl>,
}

impl CheckInput {
    #[must_use]
    pub fn new(graph: Graph, contexts: Vec<ContextDecl>) -> Self {
        Self { graph, contexts }
    }
}
```

`diff::diff()` gains a signature change:

```rust
pub fn diff(spec: CheckInput, code: Graph) -> Vec<Violation>;
```

When `spec.contexts.is_empty()`, diff behaves exactly as v0.3 — the fourth pass is a no-op, no `Violation::Context(_)` is emitted.

**`Graph` and `ContextPattern` receive `#[non_exhaustive]`** (rust-systems RC-3, RC-4) so future field/variant additions are non-breaking for downstream consumers.

**Context-pass ordering** (clean-arch RC-4). The context pass is independent of the edge pass: it re-derives cross-context candidate edges from `spec.graph.edges` + `code.edges` and cross-references them against `spec.contexts`. The three existing passes remain unchanged; pass 4 is a pure function of `(CheckInput, Graph)`, not of the intermediate state from passes 1–3. This keeps passes composable and order-independent.

### §3.8 — Self-dogfood

graph-specs-rust declares its own bounded contexts:

- `equivalence` — owns: `domain`, `ports`. Exports `Graph`, `ConceptNode`, `Edge`, `Source`, `Violation`, `ContextDecl`, `Reader`, `ContextReader` as Published Language.
- `reading` — owns: `adapter-markdown`, `adapter-rust`. Imports the above from `equivalence` as Published Language. Implements `Reader` and (for markdown) `ContextReader` — i.e. Conformist to the port contracts. Exports `MarkdownReader`, `RustReader` as Published Language.
- `orchestration` — owns: `application`. Imports from `equivalence` (Published Language) and from `reading` (Customer-Supplier on concrete readers).

The v0.4 dogfood proves the rules on the tool's own source before shipping.

## §4 — Invariants

1. **Backward compatibility with v0.3 spec trees.** A repo with no `specs/contexts/` directory continues to pass unchanged. `spec.contexts.is_empty()` ⇒ `Violation::Context(_)` never fires.
2. **NDJSON v1 consumers keep working.** v0.3 callers emit `schema_version: "1"`; v0.4 callers emit `"2"`. Consumers gate on the first-line `schema_version`.
3. **Exactly-one-context per owned unit.** An owned unit (crate) appears under `Owns` in exactly one context. Duplicates are a reader error — not a level-4 violation.
4. **Shared Kernel ownership** (DDD RC-3). A crate declared as Shared Kernel appears in the `Owns` block of exactly one designated context; other participants reference it through `Imports: SharedKernel`. This preserves invariant 3 even for jointly-developed crates.
5. **Imports are symmetric on both sides.** If consumer A declares `Imports: PublishedLanguage Graph from equivalence`, supplier context `equivalence` must declare `Exports: Graph (PublishedLanguage)`. Asymmetric declarations fire `CrossEdgeUndeclared`.
6. **Transitive cross-context references are forbidden** (DDD RC-4). If A imports X from B and B imports Y from C, a type in A's code cannot reference Y. Every cross-context edge needs an individual `Imports` entry in the referencing context's spec.
7. **Cyclic import declarations are a reader error** (SOLID RC-3, clean-arch RC-5). If A declares `Imports from B` and B declares `Imports from A` (not Shared Kernel), parsing fails with a reader error. Shared Kernel is the one legal form of mutual reference, and only one side owns the kernel (invariant 4).
8. **Deterministic output order.** Record ordering follows the existing `violation_key()` sort. `Violation::Context(_)` takes rank 8 (after existing 0–7); internal ordering within `ContextViolation` uses the `concept: String` field that every variant carries.
9. **Context pass is order-independent from edge pass.** Pass 4 re-derives cross-context candidate edges from `spec.graph.edges` + `code.edges`; it does not read pass 3's `Violation` output (clean-arch RC-4).

## §5 — Architect lenses

### §5.0 — Round 1 (DRAFT) — verdicts captured, all REQUEST CHANGES

| Lens | Verdict | Key asks addressed in this revision |
|---|---|---|
| Clean architecture | REQUEST CHANGES | RC-1 (`crates` → `owned_units`); RC-2 (`ContextReader` port); RC-3 (contexts out of `Graph`, into `CheckInput`); RC-4 (context pass order-independent); RC-5 (cycle = reader error) |
| DDD | REQUEST CHANGES | RC-1 (Conformist in v0.4); RC-2 (Exports section); RC-3 (Shared Kernel ownership invariant); RC-4 (transitive forbidden) |
| SOLID + components | REQUEST CHANGES | RC-1 (contexts out of `Graph`); RC-2 (wrap in `Violation::Context`); RC-3 (cycle = reader error) |
| Rust systems | REQUEST CHANGES | RC-1 (new variants carry `concept: String`); RC-2 (schema_version "2"); RC-3 (`#[non_exhaustive]` on `Graph`); RC-4 (`#[non_exhaustive]` on `ContextPattern`); RC-5 (parser does not generalize `SectionState`) |

All 15 asks integrated.

### §5.1 — Round 2 — RATIFY ×4

All four lenses re-invoked against this revision. All returned RATIFY.

- **Clean architecture:** RATIFY. "All five round-1 asks are materially closed. No new clean-arch concerns introduced. The `ContextReader` port signature is pure, the composition root wiring is correctly placed in `application`, and `Graph` remains a clean domain type with no layer bleed."
- **DDD:** RATIFY. "All four RC items are materially and correctly addressed. On OQ-4: markdown wins on ubiquitous language grounds — a context map should be readable by non-engineers involved in boundary discussions."
- **SOLID + components:** RATIFY. "The `CheckInput` envelope is genuine, not cosmetic. The `Violation::Context` wrapper is as close to closed-for-modification as a Rust enum permits. The cycle-as-reader-error rule is consistent with the Shared Kernel carve-out."
- **Rust systems:** RATIFY. "All five RC items present and correctly stated. `violation_key()`'s `const fn` remains valid — `&str` from a `String` field is available in a `const fn` match arm since Rust 1.65. `#[non_exhaustive]` on `Graph` does not affect internal destructure in `diff.rs` since it only restricts exhaustive struct patterns in external crates. No orphan, coherence, or serialisation issue."

**OQ-4 resolved:** markdown (per DDD lens ubiquitous-language reasoning; no blocker from other lenses).

## §6 — Non-goals

- Inferring context membership from crate layout — membership is declared, full stop.
- Implementing all seven Evans context-mapping patterns in v0.4 — four is the minimum viable set for hexagonal architectures (three load-bearing for port contracts + one for cross-kernel sharing).
- Changing v0.1 / v0.2 / v0.3 detection — pure addition.
- Building tooling to migrate existing spec trees — manual adoption per downstream.
- Graceful degradation for partial context coverage — all owned units in scope must belong to a declared context, or all context declarations are ignored (all-or-nothing v0.4 opt-in).
- Cross-repo context mapping (e.g. graph-specs-rust's contexts referencing qbot-core's contexts) — single-repo scope.
- Publishing v0.4 before qbot-core's comparator (`compare-spec-delta`) is ready to consume schema v2 — coordinate the bump.

## §7 — Issue decomposition (fills in after ratification)

Proposed vertical slices (subject to round-2 revision):

1. **Domain types + `CheckInput` + test fixtures.** `OwnedUnit`, `ContextDecl`, `ContextExport`, `ContextImport`, `ContextPattern`, `ContextViolation`. `CheckInput` envelope. `Violation::Context(_)` wrapping. `#[non_exhaustive]` on `Graph`, `ContextPattern`, `ContextViolation`. Unit tests.
2. **`ContextReader` port.** New trait in `ports`. `MarkdownReader` implements both.
3. **Markdown reader — context file parser.** `parse_context_file()` + `markdown_utils` submodule extraction. Tests for all four sections (Owns / Exports / Imports / Concepts).
4. **Domain diff — context pass.** Fourth pass in `diff::diff()` with new signature `diff(spec: CheckInput, code: Graph)`. Tests for all three `ContextViolation` arms including cycle-detection reader error (invariant 7) and transitive forbidding (invariant 6).
5. **NDJSON schema v2.** Bump version, add variant records, update `specs/ndjson-output.md` as authoritative contract.
6. **CLI text output.** New `print_violation()` arms for `Violation::Context(_)`.
7. **Self-dogfood.** Add `specs/contexts/{equivalence,reading,orchestration}.md`. Fix any violations the tool finds on its own source.
8. **cfdb ban rules.** Add `.cfdb/queries/arch-context-*.cypher` capturing context-boundary invariants at the syn-level (belt + suspenders).
9. **Downstream coordination.** File a qbot-core issue pinning schema v2 consumption timing; update Study 002 v4.2 §10 reference.

## §8 — Open questions (all resolved)

Round-1 resolved OQ-1 (schema bump to v2), OQ-2 (add Exports / export-centric), OQ-3 (add Conformist to v0.4), OQ-5 (transitive forbidden as invariant 6). Round-2 resolved OQ-4 (markdown syntax, per DDD ubiquitous-language reasoning).

## §9 — Ratification

**RATIFIED 2026-04-19.** All four architect lenses returned RATIFY in round 2 (see §5.1).

§7 is now the concrete backlog. Each vertical slice is filed as a forge issue with body linking back to this RFC (`Refs: docs/rfc/001-bounded-context-equivalence.md`). Issues are worked via `/work-issue-lib` under the dual-control regime defined in `CLAUDE.md` §3 (graph-specs check + cfdb violations).
