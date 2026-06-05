# Clean Architecture Lens — RFC-010 Review

**Verdict: REQUEST CHANGES**

Two blocking items; two advisory items.

---

## Factual claim verification

### Claim 1: graph-specs ignores H1 and discards module containment

CONFIRMED TRUE.

- `adapters/markdown/src/lib.rs:285-293`: `handle_event` matches only `HeadingLevel::H2 | HeadingLevel::H3` for concept nodes. H1 generates no `ConceptNode` — it falls through the `_ => {}` arm silently.
- `specs/dialect.md:70`: explicitly documents "Level-1 and level-4+ headings" as ignored.
- `adapters/rust/src/lib.rs:231-243`: `visit_top_level_item` walks only `Item::Struct | Item::Enum | Item::Trait | Item::Type` — there is no `mod` arm. The comment at line 240 explicitly documents this: "Inline mod contents are intentionally not recursed — per-file top-level only." Module containment is discarded.
- `adapters/rust/src/lib.rs:162-183`: `find_owned_unit` already walks up to the nearest `Cargo.toml` to find crate-level provenance, but this is used only for `VerbReader::extract_pub_fns`, not for `ConceptNode` emission. `ConceptNode` carries only `name`, `source`, `signature` — no module or crate field (`domain/src/lib.rs:101-105`).

### Claim 2: cfdb models IN_MODULE/IN_CRATE containment

CONFIRMED TRUE.

- `crates/cfdb-core/src/schema/labels.rs:105-106`: `EdgeLabel::IN_CRATE` and `EdgeLabel::IN_MODULE` are defined constants.
- `crates/cfdb-extractor/src/item_visitor/emit/mod.rs:148-174,300,385-388`: the Rust extractor emits both `IN_MODULE` and `IN_CRATE` edges per item.
- `crates/cfdb-extractor-php/src/lib.rs:101-102,276,332,367`: the PHP extractor also emits `IN_CRATE` and `IN_MODULE` edges. The RFC claim "cfdb already extracts it (namespace→Module, class/interface/trait→Item)" is verified at `cfdb-extractor-php/src/lib.rs:307` (namespace→Module traversal).

### Claim 3: cfdb is a library API, not CLI-only

CONFIRMED — cfdb exposes multiple library crates:
- `crates/cfdb-query` is a pure Rust library (`[lib]` only, `Cargo.toml:7`) with a public API including `parse`, `QueryBuilder`, and `compute_diff`.
- `crates/cfdb-petgraph` exposes `PetgraphStore` with `execute_explained` and implements `StoreBackend`.
- `crates/cfdb-cli/src/lib.rs:1-49`: cfdb-cli itself is a `[lib] + [[bin]]` combination — command logic is callable without spawning a subprocess.
- The cfdb-query adapter could be implemented against `cfdb-core::StoreBackend + cfdb-petgraph::PetgraphStore` using in-process Cypher queries. This is a real API surface, not just a CLI.

### Claim 4: agentry has 38 of ~50 files with H1=bounded-context

CONFIRMED: 38 of 50 agentry `specs/concepts/*.md` files contain "bounded context" in their text. All 50 files have an H1 heading. agentry has no `specs/contexts/` directory — context information lives exclusively in H1 of concept files, validating the RFC's key motivation.

---

## RC-1 [BLOCKING]: `AbstractionLevel` enum belongs in `domain` but the RFC's `ConceptNode.module` and `ConceptNode.crate` fields introduce a vocabulary collision with cfdb's `Module`/`Crate` node types

**Finding:** The RFC proposes adding `module` and `crate` String fields to `ConceptNode` (§3.3, R10-1). `ConceptNode` lives in `domain/src/lib.rs:101-105`. The RFC argues this is not a Dependency Rule violation because "containment is a domain abstraction concern."

That argument holds for `AbstractionLevel` — heading depth as a domain concept is analogous to `EdgeKind` (already in domain). The enum has no infrastructure signature; it classifies _abstractions_, not build artifacts.

The argument **fails** for `module` and `crate` as `String` fields on `ConceptNode`. The precedent is RFC-004's RC-1 rejection of `BuildSystemKind` (`domain/src/context.rs:17` rationale comment: "named deliberately to keep the domain model language-agnostic"). The exact same principle applies:

- `"module"` is a Rust/PHP module system concept — a file-system or namespace artifact from the adapter tier. In a language with no modules (hypothetical future plain C or single-file DSL), the field becomes meaningless noise.
- `"crate"` is explicitly a Cargo/Rust concept. The RFC's §4 Invariant 6 says "diff engine stays language-agnostic... never over `CodeLanguage`," but a `crate` field IS a language-specific containment concept. PHP has namespaces, not crates.

The RFC's own Invariant 2 acknowledges this: "Ladder levels reuse cfdb's vocabulary... `Context`/`Module`/concept provenance names match cfdb's." But cfdb deliberately chose `Module` and `Crate` as _language-agnostic_ names in its schema (`labels.rs:18-19`: "Module" and "Crate" are plain strings — cfdb-extractor-php maps PHP namespaces to `Module`, npm packages to `Crate`). The fields on `ConceptNode` must carry the same language-agnostic semantics explicitly documented, not implicit via "we're aligning with cfdb's vocabulary."

**Resolution:** One of two paths:
1. Rename `ConceptNode.crate` → `ConceptNode.unit` (matching `OwnedUnit`'s language-agnostic naming established in `domain/src/context.rs:17`) and `ConceptNode.module` → `ConceptNode.container` or `ConceptNode.scope`. Document in the field doccomment that for Rust this is the module path, for PHP this is the namespace. This keeps `domain` language-agnostic per the existing rationale.
2. Alternatively: do not add `module`/`crate` as fields on `ConceptNode` at all — carry them only in the `CodeFacts` port output as a separate `ProvenanceRecord` type that wraps a `ConceptNode`. The diff engine would receive `(ConceptNode, Option<ProvenanceRecord>)` tuples — the domain type stays clean; provenance is an overlay. This matches how RFC-004 kept `Source::Spec`/`Source::Code` clean without embedding language info.

---

## RC-2 [BLOCKING]: The `CodeFacts` port (§3.3) is not placed in `ports/`, and the RFC does not say where it lives — this is a placement ambiguity that will land in `domain` by default

**Finding:** The RFC defines the `CodeFacts` trait at §3.3 but never names the crate it lives in. The existing port traits are in `ports/src/lib.rs`:
- `Reader` (line 24)
- `VerbReader` (line 44)
- `ContextReader` (line 60)

These are all in `ports/`. The RFC's §3.3 code block defines `pub trait CodeFacts` as if it were domain code (the surrounding prose puts it adjacent to `ConceptNode` discussion). If the RFC author expects `CodeFacts` to live in `domain/`, that would be a Dependency Rule violation: the diff engine in `domain/src/diff.rs` would then depend on `CodeFacts`, which means `domain` carries an I/O-shaped trait (`fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>`). The `ReaderError` type already lives in `ports/` (`ports/src/lib.rs:77`); `domain` cannot depend on `ports`.

If `CodeFacts` lives in `ports/`, that is architecturally correct — but then the RFC's claim that "the diff engine checks... against the `CodeFacts` port" is wrong: the diff engine receives already-materialized `Vec<ConceptNode>` from `application`, not a port reference. The port is resolved at the `application` composition root (`application/src/lib.rs:36-50`).

The RFC must specify: `CodeFacts` lives in `ports/`. The diff engine (`domain/src/diff.rs`) never calls the port — it receives the already-resolved `Vec<ConceptNode>` (with provenance fields) passed in via `CheckInput`. `application/src/lib.rs::run_check` is where the port is called and the result materialized, exactly as `Reader` is called today.

**Resolution:** Add to R10-1 prescription: "`CodeFacts` trait definition goes in `ports/src/lib.rs` alongside `Reader`, `VerbReader`, `ContextReader`. The diff engine receives `Vec<ConceptNode>` with provenance fields from `CheckInput`, not a trait reference. No change to `domain/src/diff.rs` call sites."

---

## Advisory-1: Context resolver (§3.4) must live in `domain`, not in a reader adapter

**Finding:** The RFC proposes option (A) as the context resolver: infer a concept's owning context from its module co-location with other concepts under the same H1. This resolver must run _after_ both the spec side (H1 tree) and code side (containment provenance) are materialized — it is a pure function over `(Vec<ConceptNode with provenance>, Vec<ContextDecl>)`.

The RFC does not specify where the resolver lives. Three candidates exist:

1. `domain/src/context.rs` — correct tier. The existing `context_for_concept` function at `domain/src/context.rs:238` already does a simpler version of this (maps a concept's `Source::Code.path` to a context via `OwnedUnit` prefix matching). The new resolver extends this logic to use module provenance instead of path prefix. This is pure domain logic.

2. `adapters/markdown/src/lib.rs` — WRONG. Mixing context resolution with markdown parsing violates SRP and pulls I/O reasoning into the resolution layer.

3. `application/src/lib.rs` — acceptable as a fallback (composition root can orchestrate), but the logic should be domain-pure and unit-testable.

The RFC's §3.4 correctly identifies three "candidate definers" but does not prescribe the resolution call site. The council should prescribe: the context resolution algorithm lives in `domain/src/context.rs` as a pure function. `application/src/lib.rs::run_check` calls the domain resolver after materializing both sides, then passes resolved provenance into `diff()`.

This is not blocking — the RFC does not propose a wrong placement, it omits the placement. The implementer will discover this naturally during R10-3, but it is worth naming explicitly.

---

## Advisory-2: Vocabulary alignment with cfdb (§3.8 / Invariant 2) must be naming-only, not a structural dependency

**Finding:** Invariant 2 states "Ladder levels reuse cfdb's vocabulary, never a parallel one." This is safe as stated — it is naming alignment only. Verified by checking that `cfdb-query` is a library crate with no dependency on any graph-specs type and vice versa. No circular dependency exists or is proposed.

However, the RFC introduces a subtle risk at R10-6: the cfdb-query `CodeFacts` adapter. When R10-6 ships, `adapters/cfdb-query` will `use cfdb_core::...` and `cfdb_petgraph::...`. This is a legitimate adapter-tier dependency (`adapters/` → external library). The critical invariant is:

- `cfdb_core` or `cfdb_petgraph` must NOT appear in `domain/Cargo.toml` or `ports/Cargo.toml`. Only in `adapters/cfdb-query/Cargo.toml` (a new crate for R10-6) or `application/Cargo.toml`.
- The `CodeFacts` trait signature must contain zero cfdb types — only `domain::ConceptNode`, `std::path::Path`, and `ports::ReaderError`. Verified: the RFC's §3.3 signature uses only `Path`, `Vec<ConceptNode>`, and `ReaderError`. This is port-pure.

R10-6 should explicitly state in its prescription: "add `cfdb-core` and `cfdb-petgraph` to `adapters/cfdb-query/Cargo.toml` only; verify neither appears in `domain/Cargo.toml` or `ports/Cargo.toml` after this slice."

---

## Dependency Rule summary

| Layer | Proposed change | Verdict |
|---|---|---|
| `domain` | `AbstractionLevel` enum | CLEAN — pure concept enum, no infra dependency |
| `domain` | `ConceptNode.module: String` + `.crate: String` field names | LANGUAGE-BIASED — rename to language-agnostic terms |
| `ports` | `CodeFacts` trait (after RC-2 resolution) | CLEAN — port-pure signature confirmed |
| `adapters/markdown` | H1→Context tree assembly | CLEAN — adapter-tier I/O change |
| `adapters/rust` | emit module/crate provenance | CLEAN — adapter-tier |
| `adapters/cfdb-query` (R10-6) | cfdb-petgraph dependency | CLEAN — adapter-tier only |
| `application` | composition root selects adapter | CLEAN — composition root is the right place |
| `domain/src/diff.rs` | four new cohesion violation arms | CLEAN — diff engine stays language-agnostic |

## Screaming architecture assessment

The four new violation names (`ContextOwnsScatteredConcepts`, `ConceptMisfiled`, `ContextWithoutCohesionUnit`, `ModuleSplitAcrossContexts`) scream their responsibility: intra-context cohesion enforcement. They do not name a layer. Approved.

`CodeFacts` screams "code-side fact source" — acceptable. Less precise than `CodeContainmentReader` or `ContainmentPort` but within the naming pattern established by `Reader`, `VerbReader`, `ContextReader`.

`AbstractionLevel` names the concept correctly.

## Composition root impact

No change to today's composition root shape. `application/src/lib.rs::run_check` remains the wiring point. R10-6 adds a new adapter choice but the dispatch pattern (caller passes the concrete adapter at construction time) matches the existing `MarkdownReader`/`RustReader` pattern. No DI framework changes needed.

## Use case coupling map

R10-1 (domain types) → R10-2 (spec reader) and R10-3 (code reader) are correctly sequenced as dependencies. R10-4 (NDJSON schema) depends on R10-3 having the violation variants defined. R10-6 (cfdb-query adapter) is independent of R10-2 through R10-5 — it can land any time after R10-3 proves the violation rule. No unexpected coupling across use cases.

---

**RC list:**

1. [BLOCKING] RC-1: Rename `ConceptNode.module` and `ConceptNode.crate` to language-agnostic field names (e.g. `container: Option<String>` and `unit: Option<String>`) per RFC-004's `BuildSystemKind` rejection precedent (`domain/src/context.rs:17`). "Module" and "crate" are Rust/Cargo terminology; future PHP/TS backends should not confront `crate_name: None` on their concept nodes.

2. [BLOCKING] RC-2: Specify that `CodeFacts` lives in `ports/src/lib.rs` (alongside `Reader`, `VerbReader`, `ContextReader`), not adjacent to `ConceptNode` in `domain`. Add to R10-1 prescription: the diff engine receives provenance as fields on `ConceptNode` passed into `CheckInput`, never calls the port directly.

3. [advisory] A-1: Prescribe that the context resolver algorithm (§3.4 option A) is a pure function in `domain/src/context.rs`, not embedded in any adapter. Analogous to `context_for_concept` at `domain/src/context.rs:238` — extend that function rather than duplicating it in the markdown adapter.

4. [advisory] A-2: Add to R10-6 prescription: verify `cfdb-core` and `cfdb-petgraph` appear in `adapters/cfdb-query/Cargo.toml` only; confirm absence from `domain/Cargo.toml` and `ports/Cargo.toml` via a CI cargo-tree check.
