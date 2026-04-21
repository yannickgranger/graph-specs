---
title: RFC-004 — Multi-language adapter contract
status: DRAFT (round 1, awaiting architect-team review)
date: 2026-04-21
authors: Claude (session 2026-04-21, EPIC umbrella for OSS + multi-language)
companion: yg/cfdb (visibility mirror only — RFC-004 has no cross-tool wire impact)
supersedes: —
related: RFC-003 §9 (forward-looking workflow modes); RFC-005 (PHP adapter, blocked on this); RFC-006 (TypeScript adapter, blocked on this)
---

# RFC-004 — Multi-language adapter contract

## §1 — Problem

`graph-specs-rust` ships v0.4 with a single language pair:

- **Spec side:** markdown (`MarkdownReader`) — emits concept nodes from `##`/`###` headings + signature blocks from fenced ```rust + edges from `- implements:` / `- depends on:` / `- returns:` bullets + bounded-context declarations from `specs/contexts/*.md`.
- **Code side:** Rust (`RustReader`) — emits concept nodes from top-level `pub struct` / `pub enum` / `pub trait` / `pub type` + signatures from syn AST + edges from impl blocks, struct fields, and trait method signatures.

Both readers implement the same `Reader` port (`ports/src/lib.rs:15`) and produce graphs of identical shape (`Graph` in `domain/src/lib.rs`). The diff engine in `domain/src/diff.rs` consumes the two graphs and reports four levels of equivalence violations.

This shape **assumes a one-to-one mapping**: one spec source language (markdown), one code source language (Rust). The README §"Multi-language projects (planned)" line 109 already names PHP + TypeScript adapters as future work; RFC-003 §9 ("Forward-looking workflow modes") committed to a richer model:

> **Spec sources are additive per language.** Markdown is universal and mandatory; inline attributes/decorators are an optional augmentation that strengthens the gate when the codebase culture supports it. The diff engine unions all spec sources before comparing against the structural code graph. RFC-004 formalizes the `Vec<Reader>` per side; RFC-005 / RFC-006 ship the language-specific readers.

Three concrete gaps prevent RFC-005 (PHP) and RFC-006 (TS) from landing today:

1. **No language tagging.** The `Source` type carries `path` + `line` but no language discriminant. A future PHP `OwnedUnit` (composer package) and a future TS `OwnedUnit` (npm package) cannot be distinguished from a Rust crate inside the diff engine; the cross-context level-4 logic from RFC-001 would silently flatten cross-language edges into same-language ones.
2. **No file-extension dispatch.** The application's composition root (`application/src/main.rs`) instantiates exactly two readers — `MarkdownReader` and `RustReader` — and hands them fixed paths. There is no notion of "walk the tree and route each file by extension to the right adapter."
3. **No multi-source spec composition.** The `diff()` signature takes a single `Graph` per side (`fn diff(spec: CheckInput, code: Graph) -> Vec<Violation>`). There is no way to express "the spec graph for this PR is the union of (markdown spec files) ∪ (PHP `#[Spec]` attributes) ∪ (TS `@Spec` decorators)."

This RFC is **infra-only**. It ships the framework; it ships **no new adapter**. RFC-005 and RFC-006 land the PHP and TS readers against the framework this RFC ratifies.

## §2 — Scope

**Ships in v0.5:**

- **Domain additions.** `Language` enum (`Rust`, `Php`, `TypeScript`, `Markdown`, marked `#[non_exhaustive]`); `Source.language: Language` field; `OwnedUnit.build_system: BuildSystemKind` field with a `BuildSystemKind` enum (`CargoCrate`, `ComposerPackage`, `NpmPackage`, marked `#[non_exhaustive]`); `SpecSourceKind` enum (`MarkdownConcept`, `MarkdownContext`, `InlineAttribute`, marked `#[non_exhaustive]`).
- **Composition-root extension.** `application::run_check` grows from "two readers" to **`Vec<Box<dyn Reader>>` per side**: `spec_readers: Vec<Box<dyn Reader>>` and `code_readers: Vec<Box<dyn Reader>>`. The diff engine consumes a `CheckInput { spec_graph, contexts, code_graph }` where each graph is the **union** of all readers' outputs, deduplicated by `(name, source.path, source.line)`.
- **File-extension dispatch table.** `application::dispatch::route_by_extension(path: &Path) -> Option<ReaderTarget>` returns which reader(s) should ingest a file: `*.md` → markdown; `*.rs` → rust; (future RFCs add `*.php` → php-attribute + php-structural; `*.ts`/`*.tsx` → ts-decorator + ts-structural).
- **`--lang` CLI hint.** New optional flag `graph-specs check --lang <rust|php|ts>` for trees where extension dispatch is ambiguous (e.g., a polyglot monorepo where the user wants to scope a single check). Default behavior unchanged: walks the tree, dispatches by extension, runs every adapter that has a registered file type.
- **NDJSON schema bumps to `"3"`.** Every record gains a top-level `language` field on `source` objects (or `spec_source` / `code_source` in multi-source variants). v2 records continue to be produced by v0.4 callers; v3 is produced by v0.5 callers. The tool emits one version per run, never mixing.
- **`specs/dialect.md` extension.** New §"Multi-language fenced blocks" documents which fence languages route to which adapter: ```rust → rust adapter; ```php → php adapter (v0.6); ```ts → ts adapter (v0.6). Markdown stays universal — every adapter's spec source can include markdown concept declarations alongside its code-side extraction.
- **Self-dogfood.** graph-specs' own `specs/concepts/core.md` gains explicit `language: rust` annotations where the v3 NDJSON schema demands them; the existing dogfood gate stays green throughout.

**Deferred to v0.6 (RFC-005 + RFC-006):**

- The `adapters/php` crate (PHP attribute reader + PHP structural reader behind one composition).
- The `adapters/typescript` crate (TS decorator reader + TS structural reader behind one composition).
- File-type registrations for `*.php`, `*.ts`, `*.tsx` in the dispatch table.

**Deferred to v0.7 (RFC-007):**

- Cross-language bounded contexts (a single `specs/contexts/<name>.md` declaring owned units across Rust + PHP + TS). Lands `ContextPattern::AntiCorruptionLayer` (deferred from RFC-001 §2).

**Out of scope (explicit non-goals):**

- Inline doc-comment spec sources (PHPDoc `@spec`, TSDoc `@spec`). User decision recorded in session 2026-04-21: markdown stays the universal spec source; PHP/TS get attribute/decorator augmentation but no doc-comment third channel.
- Tree-sitter as the parsing backend (deferred to RFC-005/006 — those RFCs choose between tree-sitter and language-native parsers).
- crates.io publishing (deferred to RFC-008).
- Migration tooling for existing single-language graph-specs consumers — the `--lang` flag plus extension dispatch is the migration story.
- Re-architecting the bounded-context spec format (`specs/contexts/`) to be language-aware. Bounded contexts stay markdown for all languages; cross-language context declarations are RFC-007 territory.

## §3 — Design

### §3.1 — Domain additions

Five new types in `domain/src/lib.rs`. All carry `#[non_exhaustive]` per the RFC-001 §3.7 invariant for downstream-consumer-friendly evolution.

```rust
/// The source language of a code fact OR the format of a spec fact.
///
/// `Markdown` is the universal spec format — every adapter's spec source can
/// be markdown. `Rust` / `Php` / `TypeScript` are code source languages
/// (and, for PHP / TS, also the spec source language when inline attributes
/// or decorators carry the spec payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    Rust,
    Php,
    TypeScript,
    Markdown,
}

/// The build-system flavour of an `OwnedUnit`.
///
/// Each language has a canonical build-system kind. `BuildSystemKind` lets
/// the bounded-context layer (RFC-001 §3.7) interpret an owned-unit string
/// under the right namespace convention (Cargo crate name vs Composer
/// vendor/package vs npm scoped name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BuildSystemKind {
    CargoCrate,
    ComposerPackage,
    NpmPackage,
}

/// Discriminator for the spec-side origin of a fact.
///
/// `MarkdownConcept` — fact came from a `specs/concepts/*.md` heading.
/// `MarkdownContext` — fact came from a `specs/contexts/*.md` declaration.
/// `InlineAttribute` — fact came from an inline `#[Spec(...)]` attribute or
///                     `@Spec(...)` decorator extracted by a language-specific
///                     attribute-reader (RFC-005 / RFC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpecSourceKind {
    MarkdownConcept,
    MarkdownContext,
    InlineAttribute,
}
```

`Source` gains a `language` field. `OwnedUnit` gains a `build_system` field. Both are additive; existing v0.4 consumers see the new fields as serde-default-able when reading v3 NDJSON, but the producer always emits them.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Source {
    pub kind: SourceKind, // existing: Spec | Code
    pub path: PathBuf,
    pub line: usize,
    pub language: Language,    // NEW (v0.5)
    pub spec_kind: Option<SpecSourceKind>, // NEW (v0.5) — Some when kind == Spec
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct OwnedUnit {
    pub name: String,
    pub build_system: BuildSystemKind, // NEW (v0.5)
}
```

The existing `OwnedUnit(pub String)` tuple-struct shape is REPLACED by the named-field shape above. Migration path for v0.4 callers: `OwnedUnit::cargo_crate(name: impl Into<String>) -> Self` constructor preserves the old call-site shape with an inferred `build_system`.

**Why `SpecSourceKind` is a separate enum from `Language`:** A markdown spec file IS a markdown file (`Language::Markdown`), but the content it carries is a description of Rust / PHP / TS code. Conversely, an inline `#[Spec(...)]` attribute is in a PHP file (`Language::Php`) but its payload is the spec for a PHP class. The two discriminants answer different questions: `Language` asks "what is the file format?", `SpecSourceKind` asks "what kind of spec extraction did this fact come from?". Conflating them would force `Language::Markdown` to mean "this fact came from markdown", losing the ability to express "this fact came from a markdown file describing PHP code" (the greenfield workflow per RFC-003 §9.1).

### §3.2 — `Reader` port: stays single-method, but composition extends

The `Reader` trait (`ports/src/lib.rs:15`) is **unchanged**:

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}
```

What changes is the **composition root** in `application/src/lib.rs`. Today's `run_check`:

```rust
// v0.4
pub fn run_check(specs: &Path, code: &Path) -> Result<Vec<Violation>, ReaderError> {
    let specs_graph = MarkdownReader.extract(specs)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs)?;
    let code_graph = RustReader.extract(code)?;
    Ok(diff(CheckInput::new(specs_graph, spec_contexts), code_graph))
}
```

Becomes:

```rust
// v0.5
pub struct CheckRequest {
    pub spec_root: PathBuf,
    pub code_root: PathBuf,
    pub lang_hint: Option<Language>, // --lang flag
    pub spec_readers: Vec<Box<dyn Reader>>,    // NEW
    pub code_readers: Vec<Box<dyn Reader>>,    // NEW
    pub context_reader: Box<dyn ContextReader>, // markdown only, for now
}

pub fn run_check(req: CheckRequest) -> Result<Vec<Violation>, ReaderError> {
    let spec_graph = union_graphs(req.spec_readers.iter().map(|r| r.extract(&req.spec_root)).collect::<Result<Vec<_>, _>>()?);
    let spec_contexts = req.context_reader.extract_contexts(&req.spec_root)?;
    let code_graph = union_graphs(req.code_readers.iter().map(|r| r.extract(&req.code_root)).collect::<Result<Vec<_>, _>>()?);
    Ok(diff(CheckInput::new(spec_graph, spec_contexts), code_graph))
}

/// Deduplicates by (concept-name, source.path, source.line). Conflicts
/// (same concept, different signatures across readers on the SAME side) are
/// surfaced as a new SignatureDriftWithinSide variant — see §3.5.
fn union_graphs(graphs: Vec<Graph>) -> Graph { /* ... */ }
```

The composition root in `application/src/main.rs` builds the `CheckRequest` based on what file extensions are present in the tree (or what `--lang` was passed). Default registration:

```rust
// v0.5 default — spec readers and code readers
let spec_readers: Vec<Box<dyn Reader>> = vec![Box::new(MarkdownReader)];
let code_readers: Vec<Box<dyn Reader>> = vec![Box::new(RustReader)];

// v0.6 (post-RFC-005/006) example:
// let spec_readers = vec![
//     Box::new(MarkdownReader),
//     Box::new(PhpAttributeReader),    // extracts spec graph from #[Spec(...)]
//     Box::new(TsDecoratorReader),     // extracts spec graph from @Spec(...)
// ];
// let code_readers = vec![
//     Box::new(RustReader),
//     Box::new(PhpStructuralReader),   // extracts code graph from class declarations
//     Box::new(TsStructuralReader),    // extracts code graph from class declarations
// ];
```

**Note on `Box<dyn Reader>` vs static dispatch.** v0.5 uses `Box<dyn Reader>` for two reasons: (a) the reader set is determined at runtime by file-extension dispatch, not at compile time; (b) the composition root ergonomics (push-back into a `Vec`) are far cleaner than enum-dispatch. The performance cost is one virtual call per file walked, which is dominated by I/O. If profiling later shows this matters, the rust-systems lens can revisit — but it should not block v0.5.

### §3.3 — File-extension dispatch

A new module `application/src/dispatch.rs` owns the routing table:

```rust
pub struct ReaderTarget {
    pub reader_index: usize,        // index into spec_readers OR code_readers
    pub side: ReaderSide,           // Spec | Code
}

pub enum ReaderSide { Spec, Code }

pub fn route_by_extension(path: &Path, registry: &ReaderRegistry) -> Vec<ReaderTarget> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md")  => vec![ReaderTarget { reader_index: registry.markdown(), side: ReaderSide::Spec }],
        Some("rs")  => vec![ReaderTarget { reader_index: registry.rust(),     side: ReaderSide::Code }],
        // RFC-005 adds: Some("php") => vec![Spec(php_attr), Code(php_struct)]
        // RFC-006 adds: Some("ts") | Some("tsx") => vec![Spec(ts_decor), Code(ts_struct)]
        _ => vec![], // unknown extension → silently skip
    }
}
```

Note that `*.php` and `*.ts` files dispatch to **two readers** each (one per side), reflecting the §3.2 design where each language has both a spec-source reader (attribute/decorator) and a code-source reader (structural class declarations).

The `ReaderRegistry` is a tiny indirection that maps a logical reader name to its index in the `spec_readers` / `code_readers` vec. v0.5 only registers `markdown` and `rust`; RFC-005/006 add `php_attribute`, `php_structural`, `ts_decorator`, `ts_structural` to the registry.

**Unknown extensions silently skip.** This matches today's behavior (`adapter-rust` ignores `*.toml`; `adapter-markdown` ignores `*.txt`). The dialect spec already documents this exclusion list (`specs/dialect.md` §"What the rust reader ignores").

### §3.4 — `--lang` CLI hint

Optional flag for tree-scoping in polyglot monorepos:

```bash
graph-specs check --specs specs/ --code . --lang rust   # only Rust adapters fire
graph-specs check --specs specs/ --code . --lang php    # only PHP adapters fire (post-RFC-005)
graph-specs check --specs specs/ --code .               # default: all registered adapters fire
```

Implementation: `--lang` filters the `spec_readers` and `code_readers` vecs in `CheckRequest` to only those whose `language()` method returns the requested variant. Adapters gain a tiny method:

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;

    /// Which language this reader handles. Used by the --lang flag for
    /// tree-scoped checks; default impl is REQUIRED so no existing
    /// downstream impl breaks (rust-systems lens consideration).
    fn language(&self) -> Language;
}
```

The new method has **no default impl** — it is mandatory on every reader. Adding a method to a trait without a default is a breaking change for any external `impl Reader for FooReader` downstream, but graph-specs has zero downstream `Reader` impls today (the only `Reader` impls are `MarkdownReader` and `RustReader` in this workspace). v0.5 ships both impls updated; the breaking change is internally absorbed.

### §3.5 — NDJSON schema v3

`schema_version` bumps to `"3"`. The bump is driven by the new `language` field on every `source`/`spec_source`/`code_source` object — that is a breaking shape change per `specs/ndjson-output.md` §"Schema evolution".

Every existing v2 record shape gains `language` on its source object(s). New record shapes from v0.5 itself are minimal:

```json
{"schema_version":"3","violation":"missing_in_code","concept":"Foo","source":{"kind":"spec","path":"specs/core.md","line":12,"language":"markdown","spec_kind":"markdown_concept"}}
{"schema_version":"3","violation":"missing_in_specs","concept":"Bar","source":{"kind":"code","path":"src/lib.rs","line":3,"language":"rust"}}
{"schema_version":"3","violation":"signature_drift","concept":"Reader","spec_sig":"...","code_sig":"...","spec_source":{"kind":"spec","path":"specs/core.md","line":44,"language":"markdown","spec_kind":"markdown_concept"},"code_source":{"kind":"code","path":"ports/src/lib.rs","line":15,"language":"rust"}}
```

One new variant lands in v0.5 to cover the `union_graphs` conflict path (§3.2):

```json
{"schema_version":"3","violation":"signature_drift_within_side","concept":"OrderService","side":"spec","sources":[{"kind":"spec","path":"specs/php/orders.md","line":10,"language":"markdown","spec_kind":"markdown_concept","sig":"public function place(): void"},{"kind":"spec","path":"src/Orders/OrderService.php","line":42,"language":"php","spec_kind":"inline_attribute","sig":"public function place(Order $o): Receipt"}]}
```

This fires when two spec readers (or two code readers) on the same side disagree about a concept's signature — e.g., the markdown spec says `place(): void` and the PHP `#[Spec(signature: "...")]` attribute says `place(Order $o): Receipt`. The error is **per-side** (intra-side drift), distinct from `signature_drift` (cross-side spec-vs-code drift).

`specs/ndjson-output.md` is updated with the v3 variants and the schema-evolution rule that adding a new `language` enum value (e.g., a future Go adapter) does NOT bump the schema.

**Lockstep with downstream consumers.** qbot-core's `compare-spec-change` parser (Study 002 v4.6 A1, ref `yg/qbot-core#4034`) currently dispatches on `schema_version "1"` and `"2"`. v0.5 ships with a coordinated qbot-core PR that adds the `"3"` arm. Merge order: graph-specs RFC-005/006 do NOT ship until qbot-core can parse v3. Until then, v0.5 callers get v3; v0.4 callers stay on v2.

### §3.6 — `specs/dialect.md` extension

A new section **§"Multi-language fenced blocks"** is added to `specs/dialect.md`:

> Fenced code blocks inside a concept's section carry signature-level
> spec content. The fence language tag dispatches the block to the
> language-specific normalizer:
>
> | Fence tag | Adapter | Normalizer |
> |---|---|---|
> | ```` ```rust ```` | adapter-rust | `adapter-rust::normalize` (v0.2+) |
> | ```` ```php ```` | adapter-php (RFC-005) | `adapter-php::normalize` |
> | ```` ```ts ```` | adapter-typescript (RFC-006) | `adapter-typescript::normalize` |
> | other | ignored | — |
>
> A spec concept may carry fenced blocks in multiple languages
> simultaneously. Each block is matched independently against the
> corresponding language's structural code graph. Drift between
> blocks for different languages is NOT a violation — that is
> intentional cross-language spec content, not drift.

The existing markdown-reader implementation does not need to change for v0.5 (no PHP/TS adapter exists yet to consume PHP/TS fences); the dialect doc declares the future contract so RFC-005/006 land into a known shape.

### §3.7 — Self-dogfood

graph-specs' own `specs/concepts/core.md` and `specs/contexts/*.md` files are updated to:

1. Carry no `language:` annotations in markdown prose — `Language::Markdown` is implied for the spec file itself; `Language::Rust` is implied for fenced ```rust blocks.
2. Be parsed by `MarkdownReader` (with `Language::Markdown` stamped on the `Source`) and the resulting concepts compared against `RustReader`'s output (with `Language::Rust` stamped).
3. The existing dogfood gate (`graph-specs check --specs specs/ --code .` in CI) stays green throughout the v0.5 ship.

The migration is **mechanical**:

- `MarkdownReader` always stamps `Language::Markdown` on every emitted `Source`.
- `RustReader` always stamps `Language::Rust` on every emitted `Source`.
- The composition root passes both readers to the `CheckRequest` per §3.2.
- The diff engine's existing four passes (concept, signature, edge, context) are unchanged — they operate on `Graph`, which is shape-identical to v0.4 except every `Source` now has a `language` field.

A self-dogfood test runs as part of `application::tests::cli` to assert that v0.5's output on graph-specs' own tree matches v0.4's output, modulo the new `language` field on every NDJSON source object.

### §3.8 — Backward compatibility

- **NDJSON v2 consumers** see v3 records as parse errors (the `schema_version` tripwire is the documented behavior per `specs/ndjson-output.md` §"Schema evolution"). The v3 bump is the gate; downstream must add a v3 arm.
- **v0.4 spec trees** continue to pass under v0.5 unchanged. A spec tree with no `specs/contexts/` keeps level-4 a no-op (per RFC-001 §2.4 invariant). No PHP / TS files in the tree → those adapters' `route_by_extension` returns empty, so they are no-ops. The single-language flow (markdown + rust) is the v0.5 default.
- **External `impl Reader for FooReader`** outside this workspace would break on the new mandatory `Reader::language()` method. Inventory: zero such impls exist today (per a `Grep` for `impl Reader for` across cargo registry — graph-specs is not on crates.io yet, RFC-008). Risk is contained.

## §4 — Invariants

1. **Single-language repos are unaffected.** A v0.4-shape repo (markdown specs + Rust code, no PHP/TS) continues to pass v0.5's `graph-specs check` byte-identically, modulo the new `language` field on every NDJSON source.
2. **NDJSON v2 → v3 is a hard break, gated by `schema_version`.** v3 callers MUST emit `"3"`; v2 consumers MUST detect the version mismatch and either upgrade the parser or fail loudly. No silent mis-parse.
3. **The diff engine stays language-agnostic.** `domain::diff` operates on `Graph` and `CheckInput` — it never branches on `Language` or `BuildSystemKind`. Cross-language semantics (RFC-007) layer above the diff, not inside it.
4. **`Reader::language()` is mandatory.** Every concrete `Reader` impl declares its language. The composition root respects `--lang` filtering by querying this method. No reader silently handles "all languages" — that is what the dispatch table is for.
5. **Markdown is the universal spec source.** Every adapter's spec graph CAN come from markdown (`MarkdownReader`); inline attributes / decorators (RFC-005/006) are an additive augmentation. A repo that uses only markdown specs across all its languages is a valid v0.5 (and v0.6, v0.7) configuration.
6. **`OwnedUnit.build_system` is mandatory.** Every owned unit declares its build-system kind so the bounded-context layer (RFC-001 §3.7) interprets the `name` string under the right namespace convention. v0.5 self-dogfood updates `specs/contexts/*.md` to declare `build_system: cargo_crate` on every `Owns` entry.
7. **Per-side intra-graph drift is a violation.** When two spec readers (or two code readers) emit conflicting signatures for the same concept, `SignatureDriftWithinSide` fires. This is what makes the multi-source spec model honest — the union is not a "first-write-wins"; it's an "all-must-agree".
8. **Lockstep with cfdb is unaffected.** RFC-004 changes graph-specs' wire schema, not cfdb's. The cross-fixture pin (`.cfdb/cross-fixture.toml`) is unchanged. cfdb does not consume graph-specs' NDJSON; the cross-dogfood loop checks the inverse direction (graph-specs check on cfdb's tree).
9. **qbot-core consumer lockstep is required.** v0.5 does not ship until qbot-core's `compare-spec-change` parser (`yg/qbot-core#4034`) adds a v3 arm. Documented obligation in §3.5.

## §5 — Architect lenses

(All four return verdicts inline after round 1.)

### §5.1 — Clean architecture (`clean-arch`)

To be filled by the agent-team review.

### §5.2 — Domain-driven design (`ddd-specialist`)

To be filled.

### §5.3 — SOLID + component principles (`solid-architect`)

To be filled.

### §5.4 — Rust systems (`rust-systems`)

To be filled.

## §6 — Non-goals

1. Not shipping any new adapter. RFC-005 (PHP) and RFC-006 (TS) are the adapter RFCs.
2. Not shipping cross-language bounded contexts. RFC-007 lands `ContextPattern::AntiCorruptionLayer` and the cross-language `ContextDecl` semantics.
3. Not shipping inline doc-comment spec sources (PHPDoc `@spec`, TSDoc `@spec`). User decision: markdown stays universal, attributes/decorators are the only inline channel.
4. Not changing the bounded-context spec format. `specs/contexts/*.md` stays markdown for all languages.
5. Not picking a parser backend for PHP / TS. Tree-sitter vs language-native is RFC-005 / RFC-006 territory.
6. Not publishing to crates.io. Deferred to RFC-008.
7. Not breaking the existing `Reader` port shape beyond adding the mandatory `language()` method. The `extract()` signature and `ReaderError` enum are unchanged.
8. Not building the symmetric-absence detector (RFC-009 placeholder, follows from RFC-003 R3-4 limits-doc).

## §7 — Issue decomposition

Each child issue carries the standard `Tests:` template (Unit / Self dogfood / Cross dogfood / Target dogfood). Architects refine prescriptions during round-1 review.

| ID | Slice | Tests prescription |
|---|---|---|
| **R4-1** | Domain types: `Language`, `BuildSystemKind`, `SpecSourceKind` enums; `Source.language` + `Source.spec_kind` fields; `OwnedUnit.build_system` field; `OwnedUnit::cargo_crate(name)` constructor for v0.4 callers. All `#[non_exhaustive]`. | Unit: round-trip serde tests for each enum; constructor smoke test. Self dogfood: 0 violations after migration commit. Cross dogfood: cfdb tree still passes (no schema impact on cfdb). Target dogfood: none — domain-only. |
| **R4-2** | `Reader::language() -> Language` mandatory method on the port trait. `MarkdownReader` returns `Language::Markdown`; `RustReader` returns `Language::Rust`. | Unit: `assert_eq!(MarkdownReader.language(), Language::Markdown)` smoke. Self dogfood: 0 violations. Cross dogfood: none. Target dogfood: none. |
| **R4-3** | Composition root: `CheckRequest` struct with `Vec<Box<dyn Reader>>` per side; `union_graphs` deduplication function; `application::run_check` rewritten to consume `CheckRequest`. | Unit: `union_graphs` test cases (no overlap, exact dup, conflict triggering `SignatureDriftWithinSide`). Self dogfood: graph-specs' own check produces identical violations as v0.4 (modulo the new `language` field on each source). Cross dogfood: cfdb tree passes. Target dogfood: none. |
| **R4-4** | File-extension dispatch: `application/src/dispatch.rs` with `ReaderRegistry` + `route_by_extension`. Wired into the composition root. | Unit: routing table tests for `.md`, `.rs`, unknown extensions. Self dogfood: 0 violations. Cross dogfood: cfdb tree passes (cfdb has only `.rs` and `.md` files; routing handles them). Target dogfood: none. |
| **R4-5** | `--lang` CLI flag plumbed end-to-end. Filters `spec_readers` and `code_readers` to those whose `language()` matches. | Unit: integration test in `application/tests/cli.rs` for each `--lang` value. Self dogfood: `--lang rust` on graph-specs' tree gives identical results to no flag (only Rust readers register today). Cross dogfood: none. Target dogfood: none. |
| **R4-6** | NDJSON schema v3: bump version, add `language` + `spec_kind` fields on every source object, add `signature_drift_within_side` variant, update `specs/ndjson-output.md` as authoritative contract. | Unit: snapshot test on every emitter arm. Self dogfood: graph-specs' v3 NDJSON parses correctly through a v3 fixture parser. Cross dogfood: cfdb consumer still works on its own tree (cfdb does not consume graph-specs NDJSON). Target dogfood: qbot-core PR open against `compare-spec-change` adding v3 arm — proof is a green link. |
| **R4-7** | `specs/dialect.md` §"Multi-language fenced blocks" added. Documents the fence-tag → adapter dispatch contract. | Unit: none — docs only. Self dogfood: 0 violations. Cross dogfood: none. Target dogfood: none — rationale: documentation. |
| **R4-8** | Self-dogfood migration: graph-specs' own `specs/contexts/*.md` get `build_system: cargo_crate` annotations on every `Owns` entry. | Unit: parser test for the new `Owns` syntax. Self dogfood: 0 violations after migration. Cross dogfood: cfdb tree passes (cfdb's `Owns` entries get a parallel migration in cfdb's matching RFC-035 mirror — coordinated). Target dogfood: none. |
| **R4-9** | Downstream coordination: file qbot-core issue for `compare-spec-change` v3 arm. Update Study 002 v4.6 §A1 reference if the existing pin needs a bump. Block v0.5 ship until qbot-core is ready. | Unit: none. Self dogfood: none. Cross dogfood: none. Target dogfood: qbot-core PR # link in v0.5 release notes. |

R4-1 is a prerequisite for everything else (it ships the new domain shape). R4-2 → R4-3 → R4-4 → R4-5 form a chain (port → composition → dispatch → CLI). R4-6 can land in parallel with R4-2..5 once R4-1 is in. R4-7 + R4-8 + R4-9 ship after R4-6.

## §8 — Open questions

| ID | Question | Resolution |
|---|---|---|
| OQ-1 | Should `Language` carry a `Custom(&'static str)` variant for future extensibility (Go, Python, Java) without bumping the schema? | DEFERRED to round-1 review. Tentative: NO — `#[non_exhaustive]` already supports adding variants without breaking exhaustive matches; `Custom` would defeat the type-safety the enum gives. |
| OQ-2 | Should `union_graphs` conflict resolution be `SignatureDriftWithinSide` (current draft) or "first-write-wins with a warning"? | DEFERRED. Tentative: violation, not warning — the multi-source spec model is honest only if intra-side disagreement blocks merge. |
| OQ-3 | Where does `BuildSystemKind` validation live? E.g., a `composer_package` named without a vendor prefix is malformed. | DEFERRED to RFC-005 — PHP-specific validation belongs with the PHP adapter. v0.5 stores the string verbatim. |
| OQ-4 | Should the CLI surface a `--list-languages` flag that prints registered adapters? Useful for debugging polyglot dispatch. | DEFERRED to round-1 review. Tentative: yes, ~10 lines of code, real ergonomic value. |
| OQ-5 | Does `Reader::language()` need to be `&self` or can it be `fn language() -> Language` (associated function)? Associated function is purer but breaks `Box<dyn Reader>` dispatch. | RESOLVED — `&self` (object-safe; required for `Box<dyn Reader>` per §3.2). |

## §9 — Ratification

Awaiting round-1 architect-team verdicts. RFC ratifies when all four lenses return RATIFY (or reject with documented overrides per CLAUDE.md §2.3).

After ratification, §7 becomes the concrete Phase 2 backlog. Each row is filed as a forge issue with body `Refs: docs/rfc/004-multi-language-adapter-contract.md`, worked via `/work-issue-lib`, shipped through the canonical Gitea CI gates. RFC-005 (PHP) and RFC-006 (TS) draft in parallel after this RFC ratifies; their work blocks on R4-1..R4-6 landing first.
