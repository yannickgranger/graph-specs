---
title: RFC-004 — Multi-language adapter contract
status: DRAFT round 1 r2 — round-1 verdicts captured (clean-arch RC×3, ddd-specialist RC×5, solid-architect RC×2 + 3 advisories, rust-systems RC×4); RC fixes applied; awaits round-2 review
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

**Ships in v0.5 (revised in r2 per round-1 verdicts):**

- **Domain additions.** Two enums in `domain`: `CodeLanguage` (`Rust`, `Php`, `TypeScript`, `#[non_exhaustive]`) and `SpecFormat` (`Markdown`, `InlineAttribute`, `#[non_exhaustive]`). `Source` stays an **enum** (variants gain a typed payload field instead of being rewritten as a struct, per rust-systems RC-2): `Source::Spec { path, line, format: SpecFormat }` and `Source::Code { path, line, language: CodeLanguage }`. **`OwnedUnit` is unchanged** in v0.5 — the `BuildSystemKind` discriminant proposed in r1 has been dropped from this RFC entirely (clean-arch RC-1; would have leaked infrastructure into the domain layer that §4 Invariant 3 explicitly forbids the diff engine from branching on).
- **Composition-root extension.** `application::run_check` grows from "two readers" to **two structs**: a compose-time `ReaderSet { spec_readers, code_readers, context_reader }` and a runtime `CheckRequest { spec_root, code_root, lang_hint }` (split per solid-architect RC-1: SRP). The diff engine consumes a `CheckInput { spec_graph, contexts, code_graph }` where each graph is the **union** of all readers' outputs, deduplicated by `(name, source variant, source path, source line)`.
- **File-extension dispatch.** A new `application/src/adapter_routing.rs` (renamed from r1's `dispatch.rs` per clean-arch RC-3) defines `route_by_extension(path) -> Vec<AdapterAssignment>` where `AdapterAssignment` is an enum `{ Spec(usize), Code(usize) }` (replaces r1's `ReaderTarget` + `ReaderSide` split-brain — side is now encoded in the variant name). v0.5 registers `*.md` → markdown spec reader; `*.rs` → rust code reader. RFC-005/006 add their adapters additively.
- **`--lang` CLI hint.** New optional flag `graph-specs check --lang <rust|php|ts>` (takes a `CodeLanguage`) for tree-scoped polyglot checks. **Filters only `code_readers`** (rust-systems RC-4); `spec_readers` always fire because markdown is the universal spec source per Invariant 5. Default behavior unchanged: walks the tree, dispatches by extension, runs every registered adapter.
- **NDJSON schema bumps to `"3"`.** Every record gains a typed payload on its source object: `format` on spec sources (`"markdown"` / `"inline_attribute"`); `language` on code sources (`"rust"` / `"php"` / `"typescript"`). v2 records continue to be produced by v0.4 callers; v3 is produced by v0.5 callers. The tool emits one version per run, never mixing.
- **`Reader` port stays single-method.** The mandatory `language()` method proposed in r1 §3.4 has been **dropped** (combined effect of solid-architect RC-2 + ddd-specialist RC-1). The port trait stays exactly as v0.4: `fn extract(&self, root: &Path) -> Result<Graph, ReaderError>`. Reader-to-language mapping moves to `ReaderRegistry` in `application/src/adapter_routing.rs` — the only place that needs the mapping is the composition root for `--lang` filtering. ISP improved (no fat-interface; single-language readers don't lie); port purity improved (no language enum dependency in `ports`).
- **`specs/dialect.md` extension.** New §"Multi-language fenced blocks" documents which fence languages route to which adapter: ```rust → rust adapter; ```php → php adapter (v0.6); ```ts → ts adapter (v0.6). Markdown stays universal — every adapter's spec source can include markdown concept declarations alongside its code-side extraction.
- **Self-dogfood.** graph-specs' own `specs/concepts/core.md` and `specs/contexts/{equivalence,reading,orchestration}.md` get coordinated updates: `equivalence` Exports the new `CodeLanguage` + `SpecFormat` enums (ddd-specialist RC-4); the dogfood gate stays green throughout.

**Deferred to v0.6 (RFC-005 + RFC-006):**

- The `adapters/php` crate (PHP attribute reader + PHP structural reader behind one composition).
- The `adapters/typescript` crate (TS decorator reader + TS structural reader behind one composition).
- File-type registrations for `*.php`, `*.ts`, `*.tsx` in the dispatch table.

**Deferred to v0.7 (RFC-007):**

- Cross-language bounded contexts (a single `specs/contexts/<name>.md` declaring owned units across Rust + PHP + TS). Lands `ContextPattern::AntiCorruptionLayer` (deferred from RFC-001 §2).
- **`BuildSystemKind` discriminant.** If cross-language bounded contexts need to disambiguate "Cargo crate `foo`" from "Composer package `vendor/foo`" (because the `unit:` strings collide across build systems), RFC-007 reintroduces this concept — but as a port-level annotation or a spec-language-layer suffix (e.g., `unit: cargo:foo`), NOT as a domain enum. Domain stays language-agnostic per the v0.4 invariant.

**Out of scope (explicit non-goals):**

- Inline doc-comment spec sources (PHPDoc `@spec`, TSDoc `@spec`). User decision recorded in session 2026-04-21: markdown stays the universal spec source; PHP/TS get attribute/decorator augmentation but no doc-comment third channel.
- Tree-sitter as the parsing backend (deferred to RFC-005/006 — those RFCs choose between tree-sitter and language-native parsers).
- crates.io publishing (deferred to RFC-008).
- Migration tooling for existing single-language graph-specs consumers — the `--lang` flag plus extension dispatch is the migration story.
- Re-architecting the bounded-context spec format (`specs/contexts/`) to be language-aware. Bounded contexts stay markdown for all languages; cross-language context declarations are RFC-007 territory.

## §3 — Design

### §3.1 — Domain additions (revised in r2)

**Two enums and one variant added to `domain`.** All `#[non_exhaustive]` per the RFC-001 §3.7 invariant.

```rust
/// The runtime / toolchain that owns a code fact. Used on `Source::Code`
/// and (in RFC-007 cross-language contexts) on owned-unit interpretation.
/// Markdown is NOT a member — markdown is a spec format, not a code
/// language (ddd-specialist RC-1 split).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodeLanguage {
    Rust,
    Php,
    TypeScript,
}

/// The authoring format of a spec fact. Used on `Source::Spec`.
/// `Markdown` covers both `specs/concepts/*.md` and `specs/contexts/*.md`
/// (the subdirectory split is a reader-implementation detail, not a
/// domain concept — clean-arch RC-2 + ddd-specialist RC-3 both noted that
/// `specs/concepts/` vs `specs/contexts/` is not a diff-engine concern).
/// `InlineAttribute` covers `#[Spec(...)]` (PHP) and `@Spec(...)` (TS)
/// extracted by RFC-005 / RFC-006 readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpecFormat {
    Markdown,
    InlineAttribute,
}
```

**`Source` stays an enum — variants gain a typed payload, NOT a struct rewrite.** This avoids the 12+ exhaustive-match-site rewrite that rust-systems RC-2 flagged in r1. The migration is field-add only at construction sites:

```rust
// v0.5
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Spec { path: PathBuf, line: usize, format: SpecFormat },
    Code { path: PathBuf, line: usize, language: CodeLanguage },
}
```

Match sites that already destructure with `..` keep working; sites that bind `path` and `line` get one new field to ignore (`format` or `language`) — typically `..`. Match-arm ordering across the 12+ sites listed by rust-systems RC-2 is unchanged. Construction sites get one extra field per `Source::Spec` / `Source::Code` literal — mechanical migration covered in R4-1.

**`OwnedUnit` is unchanged in v0.5.** The r1 proposal to add a `build_system: BuildSystemKind` field is **dropped entirely** (clean-arch RC-1: would have leaked infrastructure semantics into `domain`, contradicting the deliberate "language-agnostic" rationale at `domain/src/context.rs:17`). The 14 `OwnedUnit(...)` construction sites across the workspace stay as-is. `application/src/ndjson.rs:147`'s `owned_unit.0` field access stays valid. If RFC-007's cross-language bounded contexts need to disambiguate same-named units across build systems, they reintroduce the concept at the spec-language layer (`unit: cargo:foo`) or as a port-level annotation — never as a domain enum.

**Why `Source` parameterization works (and r1's struct rewrite was overscoped):** The information added in v0.5 is per-fact provenance — every spec fact has a `SpecFormat`, every code fact has a `CodeLanguage`. Putting these on the variant payload is exactly the right shape. The r1 proposal of a `Source { kind: SourceKind, ..., language: Language, spec_kind: Option<SpecSourceKind> }` struct introduced an `Option<SpecSourceKind>` whose `Some`/`None` semantics were tied to `kind` — an invariant the type system did not enforce. The r2 enum-with-payload makes the invariant structural: `Source::Code` cannot have a `format`; `Source::Spec` cannot have a `language`. The two questions ("code language?" / "spec format?") get one type each, no overlap, no conditional fields.

### §3.2 — `Reader` port unchanged; composition split into `ReaderSet` + `CheckRequest` (revised in r2)

The `Reader` trait (`ports/src/lib.rs:15`) stays **byte-identical to v0.4**:

```rust
pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}
```

The r1 proposal to add a mandatory `language()` method has been **dropped**. Combined effect of solid-architect RC-2 (ISP fat-interface) + ddd-specialist RC-1 (`Language` was a homonym): the language-to-reader mapping moves to the dispatch registry in `application` (§3.3), the only place that needs it. ISP improved (no implementor is forced to lie about being single-language), port purity improved (no enum dependency edge from `ports` to language types).

The composition root splits per solid-architect RC-1 (SRP — runtime inputs vs compose-time configuration are different reasons to change):

```rust
// COMPOSE-TIME — built once at startup or in tests
pub struct ReaderSet {
    pub spec_readers: Vec<Box<dyn Reader>>,
    pub code_readers: Vec<Box<dyn Reader>>,
    pub context_reader: Box<dyn ContextReader>,
    pub registry: ReaderRegistry,  // language tags for --lang filtering (§3.3)
}

// RUNTIME — per-invocation
pub struct CheckRequest {
    pub spec_root: PathBuf,
    pub code_root: PathBuf,
    pub lang_hint: Option<CodeLanguage>,  // --lang flag
}

pub fn run_check(req: CheckRequest, readers: &ReaderSet) -> Result<Vec<Violation>, ReaderError> {
    let active_code_readers = match req.lang_hint {
        Some(lang) => readers.registry.code_readers_for(lang),
        None       => readers.code_readers.iter().enumerate().collect(),
    };
    // spec readers ALWAYS fire — markdown is the universal spec source
    // (Invariant 5; rust-systems RC-4)
    let spec_graph = union_graphs(
        readers.spec_readers.iter()
            .map(|r| r.extract(&req.spec_root))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let spec_contexts = readers.context_reader.extract_contexts(&req.spec_root)?;
    let code_graph = union_graphs(
        active_code_readers.iter()
            .map(|(_, r)| r.extract(&req.code_root))
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(diff(CheckInput::new(spec_graph, spec_contexts), code_graph))
}

/// Deduplicates by (concept-name, source variant, source.path, source.line).
/// Conflicts on the SAME side surface as `SignatureDriftWithinSide` per §3.5.
fn union_graphs(graphs: Vec<Graph>) -> Graph { /* ... */ }
```

The composition root in `application/src/main.rs` builds the `ReaderSet` once at startup. Test code can build it once per test fixture. Default registration:

```rust
// v0.5 default
let readers = ReaderSet {
    spec_readers: vec![Box::new(MarkdownReader)],
    code_readers: vec![Box::new(RustReader)],
    context_reader: Box::new(MarkdownReader),
    registry: ReaderRegistry::v0_5_default(),  // markdown spec, rust code
};

// v0.6 (post-RFC-005/006) example:
// spec_readers: vec![MarkdownReader, PhpAttributeReader, TsDecoratorReader]
// code_readers: vec![RustReader, PhpStructuralReader, TsStructuralReader]
// registry: maps each code reader to its CodeLanguage for --lang filtering
```

**Note on `Box<dyn Reader>` vs static dispatch.** v0.5 uses `Box<dyn Reader>` for two reasons: (a) the reader set is determined at runtime by file-extension dispatch, not at compile time; (b) the composition root ergonomics (push-back into a `Vec`) are cleaner than enum-dispatch. Per rust-systems lens: the actual virtual-call count on graph-specs' own tree is `len(spec_readers) + len(code_readers) = 2` per run (readers walk the tree internally), trivially dominated by I/O.

### §3.3 — Adapter routing (renamed from `dispatch` per clean-arch RC-3)

A new module `application/src/adapter_routing.rs` owns the routing table and the language registry:

```rust
/// Encodes both the assigned reader and which side of the diff it
/// contributes to. Side is the variant name — no separate `ReaderSide`
/// enum (clean-arch RC-3: avoids split-brain with `domain::Source`'s
/// Spec/Code variant naming).
pub enum AdapterAssignment {
    Spec(usize),  // index into ReaderSet.spec_readers
    Code(usize),  // index into ReaderSet.code_readers
}

/// Maps file extensions to adapter assignments AND tracks the
/// `CodeLanguage` of each code reader (for --lang filtering).
pub struct ReaderRegistry {
    code_languages: Vec<CodeLanguage>, // parallel index to ReaderSet.code_readers
    // spec readers don't need a language tag — they always fire (Invariant 5)
}

impl ReaderRegistry {
    pub fn code_readers_for<'a>(
        &self,
        lang: CodeLanguage,
    ) -> impl Iterator<Item = (usize, &'a Box<dyn Reader>)> + 'a { /* ... */ }
}

pub fn route_by_extension(path: &Path) -> Vec<AdapterAssignment> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") => vec![AdapterAssignment::Spec(MARKDOWN_SPEC_INDEX)],
        Some("rs") => vec![AdapterAssignment::Code(RUST_CODE_INDEX)],
        // RFC-005 adds: Some("php") => vec![Spec(PHP_ATTR_INDEX), Code(PHP_STRUCT_INDEX)]
        // RFC-006 adds: Some("ts") | Some("tsx") => vec![Spec(TS_DECOR_INDEX), Code(TS_STRUCT_INDEX)]
        _ => vec![], // unknown extension → silently skip
    }
}
```

`*.php` and `*.ts` files dispatch to **two readers** each (one per side), reflecting the §3.2 design where each language ships both a spec-source reader (attribute/decorator) and a code-source reader (structural).

The dispatch table is the OCP modification point per solid-architect A-1: adding a Go adapter is one new arm in `route_by_extension`, one new variant in `CodeLanguage`, one new entry in `ReaderRegistry::code_languages`. No diff-engine arm changes; no other adapter touches.

**Unknown extensions silently skip.** Matches v0.4 behavior (`adapter-rust` ignores `*.toml`; `adapter-markdown` ignores `*.txt`). Documented in `specs/dialect.md` §"What the rust reader ignores".

### §3.4 — `--lang` CLI hint (revised in r2 per rust-systems RC-4)

Optional flag for tree-scoping in polyglot monorepos. **Filters only `code_readers`** — spec readers always fire because markdown is the universal spec source per Invariant 5. Without this rule (the r1 mistake): `--lang rust` would filter out `MarkdownReader` from spec readers, leaving an empty spec graph and producing a false-pass with zero violations.

```bash
graph-specs check --specs specs/ --code . --lang rust   # only Rust code readers fire; ALL spec readers fire
graph-specs check --specs specs/ --code . --lang php    # only PHP code readers fire (post-RFC-005); ALL spec readers fire
graph-specs check --specs specs/ --code .               # default: all registered adapters fire
```

**Implementation algorithm:**

```rust
let active_code_readers = match req.lang_hint {
    None       => readers.code_readers.iter().enumerate().collect::<Vec<_>>(),
    Some(lang) => readers.registry.code_readers_for(lang).collect(),
};
// spec_readers always fire — no filter
```

Adapters do NOT carry a `language()` method on the `Reader` trait (per the §3.2 r2 revision: solid-architect RC-2 + ddd-specialist RC-1). The `ReaderRegistry` (§3.3) holds the language tag for each code reader at compose-time. Looking up "which code readers handle Rust?" is a registry query, not a per-reader method call.

R4-5's negative dogfood test: with graph-specs' all-rust-code + all-markdown-spec tree, `--lang rust` MUST produce the same violation set as no flag. An empty `active_code_readers` after filtering is a bug, not an expected state.

### §3.5 — NDJSON schema v3 (revised in r2)

`schema_version` bumps to `"3"`. The bump is driven by the new typed payload on every source object — that is a breaking shape change per `specs/ndjson-output.md` §"Schema evolution".

Every existing v2 record shape gains a typed payload on its source(s): `format` on spec sources (`"markdown"` / `"inline_attribute"`); `language` on code sources (`"rust"` / `"php"` / `"typescript"`). The two field names disambiguate the two domain enums (`SpecFormat` vs `CodeLanguage`) per ddd-specialist RC-1's split — no shared `language` field whose meaning depends on `kind`.

```json
{"schema_version":"3","violation":"missing_in_code","concept":"Foo","source":{"kind":"spec","path":"specs/core.md","line":12,"format":"markdown"}}
{"schema_version":"3","violation":"missing_in_specs","concept":"Bar","source":{"kind":"code","path":"src/lib.rs","line":3,"language":"rust"}}
{"schema_version":"3","violation":"signature_drift","concept":"Reader","spec_sig":"...","code_sig":"...","spec_source":{"kind":"spec","path":"specs/core.md","line":44,"format":"markdown"},"code_source":{"kind":"code","path":"ports/src/lib.rs","line":15,"language":"rust"}}
```

One new variant lands in v0.5 to cover the `union_graphs` conflict path (§3.2):

```json
{"schema_version":"3","violation":"signature_drift_within_side","concept":"OrderService","side":"spec","sources":[{"kind":"spec","path":"specs/php/orders.md","line":10,"format":"markdown","sig":"public function place(): void"},{"kind":"spec","path":"src/Orders/OrderService.php","line":42,"format":"inline_attribute","sig":"public function place(Order $o): Receipt"}]}
```

This fires when two spec readers (or two code readers) on the same side disagree about a concept's signature — e.g., the markdown spec says `place(): void` and the PHP `#[Spec(signature: "...")]` attribute says `place(Order $o): Receipt`. The error is **per-side** (intra-side drift), distinct from `signature_drift` (cross-side spec-vs-code drift).

**Markdown is the canonical upstream in `SignatureDriftWithinSide`** (ddd-specialist RC-5). When two spec-side readers disagree on a concept's signature, the markdown spec is the source of record; the inline attribute is the downstream conformist. Both versions are reported in the `sources` array for human resolution; neither auto-wins. The ACL classification that mediates cross-language spec translation is RFC-007 territory; v0.5 ships the variant + the canonical-upstream rule, no auto-resolution policy.

`specs/ndjson-output.md` is updated with the v3 variants and the schema-evolution rule that adding a new `CodeLanguage` or `SpecFormat` variant (e.g., a future `Go`) does NOT bump the schema.

**`violation_key` const-fn arm required (rust-systems RC-1).** `domain/src/diff.rs:105`'s `const fn violation_key` will reject the new `SignatureDriftWithinSide` variant non-exhaustively at compile time. R4-6 prescription explicitly includes adding rank `9` for `SignatureDriftWithinSide` (after `Context` rank 8) so the const-fn match stays exhaustive.

**Lockstep with downstream consumers.** qbot-core's `compare-spec-change` parser (Study 002 v4.6 A1, ref `yg/qbot-core#4034`) currently dispatches on `schema_version "1"` and `"2"`. v0.5 ships with a coordinated qbot-core PR that adds the `"3"` arm. Merge order: graph-specs v0.5 does NOT ship until qbot-core can parse v3. Until then, v0.5 callers get v3; v0.4 callers stay on v2.

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

### §3.7 — Self-dogfood (revised in r2)

graph-specs' own `specs/concepts/core.md` and `specs/contexts/*.md` files are updated to:

1. Carry no `format:` / `language:` annotations in markdown prose — `SpecFormat::Markdown` is implied for the spec file itself; `CodeLanguage::Rust` is implied for fenced ```rust blocks (per the §3.6 dialect dispatch).
2. Be parsed by `MarkdownReader` (which constructs `Source::Spec { ..., format: SpecFormat::Markdown }`) and compared against `RustReader`'s output (which constructs `Source::Code { ..., language: CodeLanguage::Rust }`).
3. **`specs/contexts/equivalence.md` Exports gain two new entries** per ddd-specialist RC-4: `CodeLanguage (PublishedLanguage)` and `SpecFormat (PublishedLanguage)`. These are pub types in `domain/`; without the Exports update, the dogfood gate would fire `MissingInSpecs` violations on the new types. (Note: the SignatureDriftWithinSide variant doesn't add a new pub type — it's a new arm of the existing `Violation` enum, which `equivalence.md` already exports as a whole.)
4. **`specs/contexts/reading.md` Imports gain `CodeLanguage` and `SpecFormat`** from `equivalence` (PublishedLanguage) since the markdown reader and rust reader both reference the new enums to construct `Source` values.
5. The existing dogfood gate (`graph-specs check --specs specs/ --code .` in CI) stays green throughout the v0.5 ship.

The migration is **mechanical** but has well-defined call-site touches per rust-systems RC-2:

- `Source::Spec { path, line }` construction sites (per rust-systems grep — markdown reader at `adapters/markdown/src/lib.rs:225,266` + `contexts.rs:98`) gain `format: SpecFormat::Markdown`.
- `Source::Code { path, line }` construction sites (rust reader at `adapters/rust/src/lib.rs:145` + `edges.rs:257-258,284`) gain `language: CodeLanguage::Rust`.
- Match sites (`application/src/ndjson.rs:197-198`, `application/src/text.rs:164`, etc.) that use `..` already work; sites that bind only `path` and `line` need to ignore the new field with `..` if not used.
- `OwnedUnit` is unchanged — zero call-site touches across the 14 sites grep'd by rust-systems RC-3.

A self-dogfood test runs as part of `application::tests::cli` to assert that v0.5's output on graph-specs' own tree matches v0.4's output, modulo the new `format` / `language` fields on every NDJSON source object.

### §3.8 — Backward compatibility (revised in r2)

- **NDJSON v2 consumers** see v3 records as parse errors (the `schema_version` tripwire is the documented behavior per `specs/ndjson-output.md` §"Schema evolution"). The v3 bump is the gate; downstream must add a v3 arm. Per rust-systems analysis, the schema_version tripwire fires before serde sees the new typed payloads.
- **v0.4 spec trees** continue to pass under v0.5 unchanged. A spec tree with no `specs/contexts/` keeps level-4 a no-op (per RFC-001 §2.4 invariant). No PHP / TS files in the tree → those adapters' `route_by_extension` returns empty, so they are no-ops. The single-language flow (markdown + rust) is the v0.5 default.
- **External `impl Reader for FooReader`** is unaffected — the r2 design drops the mandatory `language()` method (§3.2). No port-trait change. Existing `Reader` impls continue to compile with one method only.
- **`OwnedUnit` shape** is unchanged in v0.5. External callers that constructed `OwnedUnit("foo".into())` continue to work — no constructor shim needed.
- **`Source` shape** changes but stays an enum: variants gain a typed payload field. Construction-site migration is field-add only (one new field per `Source::Spec` / `Source::Code` literal). Match-site migration is `..` for sites that don't bind the new field. Per rust-systems RC-2, the 12+ call sites are mechanical.

## §4 — Invariants (revised in r2)

1. **Single-language repos are unaffected.** A v0.4-shape repo (markdown specs + Rust code, no PHP/TS) continues to pass v0.5's `graph-specs check` byte-identically, modulo the new `format` / `language` fields on every NDJSON source.
2. **NDJSON v2 → v3 is a hard break, gated by `schema_version`.** v3 callers MUST emit `"3"`; v2 consumers MUST detect the version mismatch and either upgrade the parser or fail loudly. No silent mis-parse.
3. **The diff engine stays language-agnostic.** `domain::diff` operates on `Graph` and `CheckInput` — it never branches on `CodeLanguage` or `SpecFormat`. Cross-language semantics (RFC-007) layer above the diff, not inside it. Per clean-arch RC-1: this is the invariant that forbids `BuildSystemKind` from living in `domain` — adding it would either be unused (hence pointless) or would force the diff engine to branch on it (violating this invariant).
4. **`Reader` port stays single-method.** No `language()` method on the trait. The `ReaderRegistry` in `application` (§3.3) holds language tags for code readers. Adding a new code-language adapter never modifies the port.
5. **Markdown is the universal spec source.** Every adapter's spec graph CAN come from markdown (`MarkdownReader`); inline attributes / decorators (RFC-005/006) are an additive augmentation. A repo that uses only markdown specs across all its languages is a valid v0.5 (and v0.6, v0.7) configuration. **Corollary (rust-systems RC-4):** `--lang` filters only `code_readers`; spec readers always fire.
6. **`OwnedUnit` stays language-agnostic in v0.5.** No `build_system` field. `OwnedUnit(pub String)` is unchanged. Per clean-arch RC-1 + the pre-existing `domain/src/context.rs:17` rationale comment.
7. **Per-side intra-graph drift is a violation; markdown is the canonical upstream.** When two spec readers (or two code readers) emit conflicting signatures for the same concept, `SignatureDriftWithinSide` fires. The variant reports BOTH sources for human resolution (no auto-resolution policy in v0.5; that lands in RFC-007). On the spec side, the markdown spec is the canonical upstream — the inline attribute is the conformist (ddd-specialist RC-5).
8. **Lockstep with cfdb is unaffected.** RFC-004 changes graph-specs' wire schema, not cfdb's. The cross-fixture pin (`.cfdb/cross-fixture.toml`) is unchanged. cfdb does not consume graph-specs' NDJSON; the cross-dogfood loop checks the inverse direction (graph-specs check on cfdb's tree).
9. **qbot-core consumer lockstep is required.** v0.5 does not ship until qbot-core's `compare-spec-change` parser (`yg/qbot-core#4034`) adds a v3 arm. Documented obligation in §3.5.
10. **`const fn violation_key` stays exhaustive.** Adding the `SignatureDriftWithinSide` variant requires a matching arm with rank 9 (after `Context` rank 8). Per rust-systems RC-1, this is a compile blocker if missed; R4-6 prescription names it.

## §5 — Architect lenses

(All four return verdicts inline after round 1.)

### §5.0 — Round-1 verdict summary

| Lens | Round 1 | RC items | Round 2 |
|---|---|---|---|
| Clean architecture | REQUEST CHANGES | RC-1 (BLOCKER) `BuildSystemKind` leaks infra into domain (Dependency Rule); RC-2 `SpecSourceKind` placement; RC-3 `dispatch.rs` naming + `ReaderSide`/`SourceKind` split-brain | pending — all three fixes applied |
| Domain-driven design | REQUEST CHANGES | RC-1 (BLOCKING) split `Language` → `CodeLanguage` + `SpecFormat` (homonym); RC-2 "spec source" prose overload; RC-3 drop `SpecSourceKind` (absorbed into `SpecFormat`); RC-4 R4-1 must update `specs/contexts/{equivalence,reading}.md`; RC-5 declare markdown canonical upstream in `SignatureDriftWithinSide` | pending — all five fixes applied |
| SOLID + components | REQUEST CHANGES | RC-1 (BLOCKING) split `CheckRequest` → `ReaderSet` + `CheckRequest` (SRP); RC-2 (BLOCKING) `Reader::languages() -> &[Language]` (ISP) — superseded by drop entirely per §3.2 r2; A-1 OCP advisory; A-2 CCP advisory; A-3 `SignatureDriftWithinSide` clean | pending — both blocking fixes applied (RC-2 went further than slice: dropped from trait entirely) |
| Rust systems | REQUEST CHANGES | RC-1 `violation_key` const-fn arm for `SignatureDriftWithinSide` (compile blocker); RC-2 12+ `Source` enum→struct call-sites (resolved by keeping `Source` as enum); RC-3 `owned_unit.0` ndjson break (resolved by keeping `OwnedUnit` unchanged); RC-4 `--lang` must NOT filter spec readers | pending — all four fixes applied |

All 14 round-1 RC items addressed in r2. Three convergences eliminated entire migration scope:
- `SpecSourceKind` triple-flagged → dropped (clean-arch RC-2 + ddd RC-3 + solid A-3 converged)
- `BuildSystemKind` in domain triple-flagged → dropped from v0.5 entirely (clean-arch RC-1 mandatory; ddd RC-4 + rust-systems RC-3 evaporate as side effects)
- `Source` enum→struct rewrite → kept as enum with parameterized variants (rust-systems RC-2 12+ sites collapse to field-add only)

### §5.1 — Clean architecture (`clean-arch`) — Round 1

**Verdict: REQUEST CHANGES** (all three items applied in r2).

- **RC-1 (BLOCKER):** `BuildSystemKind` placement in `domain` violated the Dependency Rule — `CargoCrate`/`ComposerPackage`/`NpmPackage` are adapter-tier identifiers, not domain concepts. The lens correctly flagged that §4 Invariant 3 (diff engine never branches on `BuildSystemKind`) plus the `domain/src/context.rs:17` rationale comment ("named deliberately to keep the domain model language-agnostic") meant the type either has no business in `domain` or has no business at all in v0.5. Resolution: dropped `BuildSystemKind` from v0.5 entirely (§6 non-goal 9). RFC-007 reintroduces if needed at the spec-language layer or as a port annotation, never as a domain enum.
- **RC-2:** `SpecSourceKind` placement in `domain` was unproven — the lens noted that if the diff engine doesn't branch on it (it doesn't), it's an adapter-tier provenance tag, not a domain concept. Resolution: dropped `SpecSourceKind` entirely; the legitimate `InlineAttribute` distinction is absorbed into `SpecFormat` (per ddd-specialist RC-3 convergence).
- **RC-3:** `dispatch.rs` / `ReaderTarget` / `ReaderSide` named the mechanism, not the responsibility. `ReaderSide { Spec, Code }` shadowed `Source::Spec`/`Source::Code` (split-brain). Resolution: renamed module to `adapter_routing.rs`; `ReaderTarget`+`ReaderSide` collapsed into a single `AdapterAssignment { Spec(usize), Code(usize) }` enum where the side is encoded in the variant name (kills the split-brain).

**Findings that did NOT require changes:** dependency direction of `CheckRequest` carrying `Vec<Box<dyn Reader>>` is sound (`application` → `ports`); composition root wiring respects the inward-pointing rule; `Reader::language()` would have been port-clean had `Language` stayed in `domain` — but solid-architect RC-2 + ddd-specialist RC-1 converged to drop it entirely; forward-looking RFC-005/006 citations are correctly scoped as informational.

### §5.2 — Domain-driven design (`ddd-specialist`) — Round 1

**Verdict: REQUEST CHANGES** (all five items applied in r2).

- **RC-1 (BLOCKING):** `Language { Rust, Php, TypeScript, Markdown }` was a homonym across three semantic layers — runtime owning a code file, spec authoring format, fenced-block dispatch tag. RFC-001 §5.1's `crate:` → `unit:` precedent showed that conflating language-specific vocabulary with a language-agnostic concept is a split-brain seed. Resolution: split into `CodeLanguage { Rust, Php, TypeScript }` (code-side question) + `SpecFormat { Markdown, InlineAttribute }` (spec-side question). Each enum answers exactly one question. The fenced-block dispatch (§3.6) maps to `CodeLanguage` since it tags fenced blocks for code-side normalization.
- **RC-2:** "spec source" overloaded the `specs/` directory. Resolution: prose now consistently uses "spec-side reader" (matching the `spec_readers` field name); `SpecFormat` is reserved for the enum scope. Vocabulary clarification added at top of §3.1.
- **RC-3:** `SpecSourceKind` was partially absorbed by `Language` and partially redundant — `MarkdownConcept` / `MarkdownContext` are reader-implementation details (the diff engine doesn't branch on subdirectory). Resolution: dropped entirely; the legitimate `InlineAttribute` distinction lives in `SpecFormat`. `Source::Spec` carries `format: SpecFormat` (no `Option<...>` whose meaning depends on context).
- **RC-4:** R4-1 self-dogfood migration was instance-level only (gaining `build_system: cargo_crate` annotations on `Owns` entries) but missed the type-level Published Language signature update. Resolution: R4-1 now explicitly includes `specs/contexts/equivalence.md` Exports gain `CodeLanguage` + `SpecFormat` (PublishedLanguage); `specs/contexts/reading.md` Imports gain the same. (The instance-level migration that ddd RC-4 originally referenced is moot since `OwnedUnit` is unchanged in r2.)
- **RC-5:** ACL deferral correctly scoped, but `SignatureDriftWithinSide` implicitly required a canonical-upstream rule for spec-side conflicts. Resolution: §4 Invariant 7 + §3.5 now declare markdown as canonical upstream on the spec side; the variant reports both sources; no auto-resolution. Auto-resolution is RFC-007 territory.

### §5.3 — SOLID + component principles (`solid-architect`) — Round 1

**Verdict: REQUEST CHANGES** (both blocking items applied; advisories carried).

- **RC-1 (BLOCKING):** `CheckRequest` bundled compose-time decisions (which readers are registered) with runtime inputs (paths, lang hint) — SRP violation: two reasons to change. Test code or library consumers wanting to vary roots while reusing the reader set were forced to reconstruct the entire struct. Resolution: split into `ReaderSet` (compose-time) + `CheckRequest` (runtime); `run_check(req: CheckRequest, readers: &ReaderSet)` signature.
- **RC-2 (BLOCKING):** Mandatory `Reader::language() -> Language` was a fat-interface concern — single-language assumption baked into every implementor. The lens proposed `languages() -> &[Language]` (slice) as the fix. **r2 went further:** combined with ddd-specialist RC-1's `Language` split, the `language()` method on the trait would have to return either `&[CodeLanguage]` or `&[SpecFormat]` depending on reader side, which is incoherent on a unified `Reader` trait. Resolution: `language()` is dropped entirely from the port; `ReaderRegistry` in `application/src/adapter_routing.rs` holds the language tag for each code reader. Cleaner ISP (no implementor lies); cleaner port purity (no enum dependency edge from `ports` to language types).
- **A-1 (advisory carried):** OCP modification point is the `route_by_extension` match arm in `application/src/adapter_routing.rs`, not the enum variants — adding a Go adapter is one new arm + one new `CodeLanguage` variant + one new registry entry. Documented in §3.3 r2.
- **A-2 (advisory carried):** R4-1 and R4-3 CCP groupings are justified — same change-vector trigger (language-adapter registration / multi-source composition semantics).
- **A-3 (advisory carried):** `SignatureDriftWithinSide` is clean — `SpecFormat` is domain provenance, not reader identity. The `sources: Vec<...>` array carries `SpecFormat` + `path`, never references concrete reader types.

Stability metrics impact (per the lens's table): `domain` Zone-of-Pain D=1.00 unchanged (5→2 new types still concrete; A=0); `ports` D=0.35 stays (no new methods on the trait, since `language()` was dropped); `application` D=0.00 stays (composition root). No SDP/ADP violations introduced.

### §5.4 — Rust systems (`rust-systems`) — Round 1

**Verdict: REQUEST CHANGES** (all four items applied in r2; two evaporated by upstream fixes).

- **RC-1:** `domain/src/diff.rs:105`'s `const fn violation_key` will reject the new `SignatureDriftWithinSide` variant non-exhaustively at compile time. Resolution: R4-6 prescription explicitly includes adding rank `9` for `SignatureDriftWithinSide` (after `Context` rank 8) so the const-fn match stays exhaustive. Added as Invariant 10.
- **RC-2:** `Source` enum→struct migration would have touched 12+ exhaustive match sites across 4 crates. Resolution: `Source` stays an **enum** in r2 — variants gain a typed payload field (`format: SpecFormat` on `Spec`; `language: CodeLanguage` on `Code`). Migration is field-add only at construction sites; match sites with `..` already work; sites binding `path`/`line` ignore the new field with `..`. The 12+ call-site list is preserved verbatim in R4-1's prescription.
- **RC-3:** `application/src/ndjson.rs:147`'s `owned_unit.0` field access would have broken under the r1 `OwnedUnit` shape change. Resolution: `OwnedUnit` is **unchanged in v0.5** (clean-arch RC-1 already required dropping `BuildSystemKind` from domain; without the new field, the named-field rewrite is unnecessary). All 14 `OwnedUnit(...)` construction sites stay valid; `owned_unit.0` stays valid.
- **RC-4:** `--lang rust` would have filtered out `MarkdownReader` (returning `Language::Markdown`) from spec readers, producing an empty spec graph and a false pass. Resolution: §3.4 algorithm now explicitly filters only `code_readers`; spec readers always fire (Invariant 5 corollary). R4-5 negative test added.

**Findings that did NOT require changes:** trait object safety preserved (no generics, no Self in return); `#[non_exhaustive]` on the new enums has no `const fn` use-site impact (unlike RFC-001's `violation_key()` concern, which RC-1 above DOES touch via the new variant arm); zero new workspace crates; zero new `[workspace.dependencies]` entries; `Box<dyn Reader>` virtual-call cost is `len(spec_readers) + len(code_readers) = 2` per run, trivially I/O-dominated; NDJSON serde uses `serde_json::json!` macros (hand-built `Value`), not `#[derive(Serialize)]` — no adjacent-tagging pitfall on the `sources` array; `R4-9` qbot-core lockstep is acceptable as a documented obligation (RFC-002-style cross-fixture pin enforcement is overkill for a one-shot consumer bump).

## §6 — Non-goals (revised in r2)

1. Not shipping any new adapter. RFC-005 (PHP) and RFC-006 (TS) are the adapter RFCs.
2. Not shipping cross-language bounded contexts. RFC-007 lands `ContextPattern::AntiCorruptionLayer` and the cross-language `ContextDecl` semantics. Also reintroduces `BuildSystemKind` if needed (at the spec-language layer or as a port annotation, NOT as a domain enum — clean-arch RC-1).
3. Not shipping inline doc-comment spec sources (PHPDoc `@spec`, TSDoc `@spec`). User decision: markdown stays universal, attributes/decorators are the only inline channel.
4. Not changing the bounded-context spec format. `specs/contexts/*.md` stays markdown for all languages.
5. Not picking a parser backend for PHP / TS. Tree-sitter vs language-native is RFC-005 / RFC-006 territory.
6. Not publishing to crates.io. Deferred to RFC-008.
7. **Not changing the `Reader` port shape at all.** The r1 proposal of a mandatory `language()` method has been dropped. The `extract()` signature and `ReaderError` enum are byte-identical to v0.4.
8. Not building the symmetric-absence detector (RFC-009 placeholder, follows from RFC-003 R3-4 limits-doc).
9. **Not adding `BuildSystemKind` to the domain in v0.5.** Per clean-arch RC-1. If RFC-007 needs build-system disambiguation for cross-language contexts, it lands the concept at the spec-language or port layer.
10. **Not auto-resolving `SignatureDriftWithinSide` conflicts in v0.5.** The variant reports BOTH sources; human resolution. The canonical-upstream rule (markdown wins on the spec side) tells humans which to follow, but the tool does not silently pick one. Auto-resolution policy is RFC-007 territory.

## §7 — Issue decomposition

Each child issue carries the standard `Tests:` template (Unit / Self dogfood / Cross dogfood / Target dogfood). Architects refine prescriptions during round-1 review.

| ID | Slice | Tests prescription |
|---|---|---|
| **R4-1** | Domain types: `CodeLanguage` + `SpecFormat` enums (`#[non_exhaustive]`). `Source` enum gains typed payload per variant (`format: SpecFormat` on `Spec`; `language: CodeLanguage` on `Code`). **Updates `specs/contexts/equivalence.md` Exports** with `CodeLanguage` + `SpecFormat` (PublishedLanguage); **updates `specs/contexts/reading.md` Imports** to consume them (per ddd-specialist RC-4). Migrates the 12+ `Source::Spec`/`Source::Code` construction + match sites listed by rust-systems RC-2 across `adapters/markdown/`, `adapters/rust/`, `application/`, `domain/` (file:line list pasted verbatim from rust-systems verdict). | Unit: round-trip serde tests for both enums; `Source::Spec` and `Source::Code` smoke tests. Self dogfood: 0 violations after migration commit (verifies `equivalence.md` Exports update is correct). Cross dogfood: cfdb tree still passes (no schema impact on cfdb). Target dogfood: none — domain-only. |
| **R4-2** | (DROPPED in r2) — was `Reader::language()`. The trait stays single-method. The language tag moves to `ReaderRegistry` per §3.3 (lands in R4-4). | n/a — slice merged into R4-4. |
| **R4-3** | Composition root split per solid-architect RC-1: `ReaderSet` struct (compose-time: `spec_readers`, `code_readers`, `context_reader`, `registry`) + `CheckRequest` struct (runtime: `spec_root`, `code_root`, `lang_hint`); `union_graphs` deduplication function; `application::run_check(req: CheckRequest, readers: &ReaderSet)` signature. | Unit: `union_graphs` test cases (no overlap, exact dup, conflict triggering `SignatureDriftWithinSide`). Self dogfood: graph-specs' own check produces identical violations as v0.4 (modulo the new `format` / `language` fields on each source). Cross dogfood: cfdb tree passes. Target dogfood: none. |
| **R4-4** | Adapter routing: `application/src/adapter_routing.rs` (renamed from r1's `dispatch.rs` per clean-arch RC-3) with `AdapterAssignment { Spec(usize), Code(usize) }` enum (no `ReaderSide` split-brain), `ReaderRegistry` (holds `code_languages: Vec<CodeLanguage>` for code-reader filtering), `route_by_extension`. Wired into the composition root. | Unit: routing table tests for `.md`, `.rs`, unknown extensions; `ReaderRegistry::code_readers_for(CodeLanguage::Rust)` smoke test. Self dogfood: 0 violations. Cross dogfood: cfdb tree passes. Target dogfood: none. |
| **R4-5** | `--lang` CLI flag plumbed end-to-end. **Filters only `code_readers`** via `ReaderRegistry::code_readers_for(lang)` per rust-systems RC-4. `spec_readers` always fire (Invariant 5). | Unit: integration test in `application/tests/cli.rs` for each `--lang` value. Self dogfood: `--lang rust` on graph-specs' tree gives identical results to no flag. **Negative test (rust-systems RC-4):** `--lang rust` on an all-markdown spec tree returns the same violations as no flag; an empty `active_code_readers` after filtering is a bug. Cross dogfood: none. Target dogfood: none. |
| **R4-6** | NDJSON schema v3: bump version; add `format` field on spec-source objects, `language` field on code-source objects; add `signature_drift_within_side` variant with `sources: Vec<SourceWithSig>` array; **add `SignatureDriftWithinSide` arm to `const fn violation_key` at `domain/src/diff.rs:105` with rank 9** per rust-systems RC-1 (compile blocker if missed); update `specs/ndjson-output.md` as authoritative contract. | Unit: snapshot test on every emitter arm; `violation_key` returns rank 9 for `SignatureDriftWithinSide`. Self dogfood: graph-specs' v3 NDJSON parses correctly through a v3 fixture parser. Cross dogfood: cfdb consumer still works on its own tree (cfdb does not consume graph-specs NDJSON). Target dogfood: qbot-core PR open against `compare-spec-change` adding v3 arm — proof is a green link. |
| **R4-7** | `specs/dialect.md` §"Multi-language fenced blocks" added. Documents the fence-tag → adapter dispatch contract. | Unit: none — docs only. Self dogfood: 0 violations. Cross dogfood: none. Target dogfood: none — rationale: documentation. |
| **R4-8** | (DROPPED in r2) — was `OwnedUnit.build_system` self-dogfood migration. `OwnedUnit` is unchanged in v0.5 per clean-arch RC-1; no migration needed. | n/a — `specs/contexts/*.md` `Owns` entries unchanged. |
| **R4-9** | Downstream coordination: file qbot-core issue for `compare-spec-change` v3 arm. Update Study 002 v4.6 §A1 reference if the existing pin needs a bump. Block v0.5 ship until qbot-core is ready. | Unit: none. Self dogfood: none. Cross dogfood: none. Target dogfood: qbot-core PR # link in v0.5 release notes. |

R4-1 is a prerequisite for everything else (ships the new enums + Source variant payload). R4-3 → R4-4 → R4-5 form a chain (composition → dispatch → CLI). R4-6 can land in parallel with R4-3..5 once R4-1 is in. R4-7 + R4-9 ship after R4-6.

**Slices removed in r2:** R4-2 (folded into R4-4 with `ReaderRegistry`); R4-8 (no `OwnedUnit` migration needed).

## §8 — Open questions

| ID | Question | Resolution |
|---|---|---|
| OQ-1 | Should `CodeLanguage` carry a `Custom(&'static str)` variant? | RESOLVED — NO. `#[non_exhaustive]` supports adding variants without breaking exhaustive matches; `Custom` would defeat the type-safety. |
| OQ-2 | Should `union_graphs` conflict resolution be `SignatureDriftWithinSide` (violation) or "first-write-wins with a warning"? | RESOLVED — violation, with markdown as canonical upstream on the spec side (ddd-specialist RC-5). The variant reports both sources for human resolution; no auto-resolution. |
| ~~OQ-3~~ | ~~Where does `BuildSystemKind` validation live?~~ | DROPPED — `BuildSystemKind` is no longer in v0.5 (clean-arch RC-1). RFC-007 reintroduces if needed. |
| OQ-4 | Should the CLI surface a `--list-languages` flag that prints registered adapters? | RESOLVED — YES, ~10 lines, useful debug ergonomic for polyglot dispatch. Lands in R4-5. |
| ~~OQ-5~~ | ~~Does `Reader::language()` need `&self` or associated function?~~ | DROPPED — `Reader::language()` is no longer on the trait (solid-architect RC-2 + ddd-specialist RC-1). Registry holds the mapping. |

## §9 — Ratification

Round 1: all four lenses returned REQUEST CHANGES (clean-arch RC×3, ddd-specialist RC×5, solid-architect RC×2 + 3 advisories, rust-systems RC×4 — 14 RC items total, 5 blocking). All RC items applied in r2. Round 2 awaits re-invocation against the patched RFC.

After ratification, §7 becomes the concrete Phase 2 backlog. Each row is filed as a forge issue with body `Refs: docs/rfc/004-multi-language-adapter-contract.md`, worked via `/work-issue-lib`, shipped through the canonical Gitea CI gates. RFC-005 (PHP) and RFC-006 (TS) draft in parallel after this RFC ratifies; their work blocks on R4-1, R4-3, R4-4, R4-6 landing first.
