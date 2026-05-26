---
title: RFC-007 — verb anchoring for impl methods (`- verb: Type::method`)
status: Ratified (4-lens unanimous RATIFY round 2 — clean-arch / ddd / solid / rust-systems; round 1: 1 BLOCKER (rust-systems B-007-1 `root_ident_of_self_ty` qself guard) + 3 advisories folded; round 2: re-pass confirmed RATIFY on amended §3.2 + Invariant 11; ready for implementation issue filing)
date: 2026-05-27
authors: agentry-captain-2026-05-27
companion: consumer-side EPIC agentry#793; gap incident on agentry#1249 (verb anchor on impl method failed to match because Slice A walks only top-level free functions)
prior-art: RFC-006 §6 ("Trait-method anchoring across `impl Trait for Type` blocks. Future RFC."); RFC-005 §3.2 walk model; specs/dialect.md ## What the Rust reader parses
---

# RFC-007 — verb anchoring for impl methods

## §1 — Problem

RFC-006 Slice A (PR #108) shipped the `- verb: <bare-ident>` bullet and the verb-pass diff machinery. The Rust adapter's `extract_pub_fns` walks **only top-level `pub fn` items** (`syn::Item::Fn` at file root). Inherent methods (`impl Foo { pub fn bar() }`) and trait methods (`impl Trait for Foo { fn bar() }`) are not extracted.

Empirical evidence the gap is load-bearing:

| consumer | top-level pub fn | impl methods (pub or trait-driven) | ratio |
|---|---|---|---|
| agentry: `crates/orchestrator-runtime/src/lifecycle_redis.rs` | 0 | 12 (`RedisEventSource::resume_from`, `RedisStateProjector::*`, …) | 0/12 |
| agentry: `crates/orchestrator-runtime/src/daemon.rs` (2643 LOC) | 0 | 31+ | 0/31+ |
| graph-specs: `adapters/rust/src/lib.rs` | 0 free, 1 `impl RustReader::extract_pub_fns` | n/a | n/a |

The first attempted agentry-side fence (`- verb: resume_from` under `## RedisEventSource`) failed to match because `resume_from` is an inherent impl method, not a top-level free fn. The PR (yg/agentry#1249) was reverted; only the pin bump (#1258) shipped. The consumer-side EPIC (agentry#793) cannot exit until impl methods are anchorable — every `retire-when: B2-verb-anchoring-lands` predicate in the agentry tree targets an impl method.

The dialect spec already foreshadows the gap: `specs/dialect.md ## What the Rust reader ignores` enumerates "impl blocks, fn, const, static, use, macro_rules!, mod" as ignored — that line is the load-bearing fence this RFC opens.

## §2 — Scope

In scope:

1. New qname grammar `<Type>::<method>` for the `- verb:` bullet. Bare-ident form (`- verb: foo`) is preserved for top-level `pub fn` (RFC-006 §3.1). The grammar choice is **syntactically explicit at the spec side**: a `::` in the target means "impl method", no `::` means "top-level pub fn" — never both for one anchor.
2. Rust adapter walks `syn::Item::Impl` blocks and emits one `PubFnDecl` per public method, with `name = "Type::method"`. The `Type` portion is derived from `ImplItemFn`'s parent `ItemImpl.self_ty` (lowered to its named root via the same generic-stripping rule the existing dialect uses for top-level types: `impl Foo<T>` → `Foo`).
3. Trait-method visibility: methods inside `impl Trait for Foo { fn bar() }` are treated as **public** if the trait itself is `pub` at the impl site. The `pub` keyword does not appear on the method (Rust syntax forbids it), but the visibility is inherited from the trait. RFC-007 requires the Rust adapter to resolve trait visibility via `syn::ItemImpl.trait_` (the optional trait path) and consult the trait's declaration — OR, as a Slice A simplification, treat ALL methods inside a trait impl block as public (rationale below).
4. Inherent-method visibility: methods inside `impl Foo { pub fn bar() }` require the explicit `pub` keyword. The existing top-level filter `matches!(f.vis, Visibility::Public(_))` extends naturally.
5. Multi-impl collision: when two impl blocks contribute the same `Type::method` qname (inherent + extension impl across files), both decls are emitted; the verb-pass anchor `- verb: Type::method` claims both. A single anchor still resolves the pair to "this concept owns these decls" — no per-impl disambiguation. The same-qname-multi-decl shape already exists for top-level pub fns when multiple files declare them (already handled by RFC-006 §3.4 `decl_by_qname` HashMap of Vec).
6. Spec-side `- verb:` parser grammar widens to accept the `::` separator: regex `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$`. Bare-ident form is the subset where the second alternative is empty.
7. `MarkdownReader::parse_verb_bullet` (added in RFC-006 Slice A) is extended to accept the wider regex. Tolerant-skip on malformed targets stays the same. **Round 2 (solid A2 + rust-systems A-007-5 clarification):** the current `parse_verb_bullet` at `adapters/markdown/src/lib.rs:381` uses a permissive `qname.is_empty() || qname.contains(char::is_whitespace)` check that admits multi-segment paths like `a::b::c`. RFC-007 Slice A REPLACES that check with the regex `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$` so multi-segment paths are correctly rejected with a `tracing::warn!`. The replacement is part of Slice A scope, not an existing precondition.
8. The `## VerbAnchor` concept in `specs/concepts/core.md` keeps its existing shape (`qname: String`); the qname value now ranges over both bare-ident and `Type::method` forms. No new domain type.

Out of scope (§6 expands):

- Trait-method anchoring **at the trait declaration** (`pub trait Foo { fn bar(); }`). The trait's `fn bar` is a contract; the actual code lives in `impl Trait for X { fn bar() }`. Anchoring on traits would be a separate concept ("contract anchor" vs "implementation anchor"). Future RFC.
- Generic method anchoring with type parameters in the qname (`Foo<u32>::bar`). The dialect's existing generic-stripping rule (`Foo<T>` → `Foo`) applies; param-aware anchoring is a future RFC.
- Free-fn-vs-method ambiguity resolution beyond the `::` syntactic split. An anchor `- verb: bar` is **deliberately scoped** to top-level pub fns; an anchor `- verb: Type::bar` is deliberately scoped to impl methods. There is no auto-fallback from one to the other.
- Module-path qnames (`module::sub::fn`). The dialect's per-file top-level walker has no module-path information. Still deferred per RFC-006 §6.
- Const / static / macro_rules! anchoring.
- Per-concept opt-in granularity refinement — that is **sibling RFC-008** (filed alongside this RFC). The two RFCs compose; either may merge first.

## §3 — Design

### §3.1 — Spec-side grammar widening (markdown reader)

Per RFC-006 §3.1 BLOCKER 1 (separate handler, NOT BULLET_PREFIXES): `parse_verb_bullet` already lives at `adapters/markdown/src/lib.rs:373`. The handler's regex widens from `^[A-Za-z_][A-Za-z0-9_]*$` to `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$`.

Concrete grammar:

- `<bare-ident>` — matches top-level `pub fn` (RFC-006 Slice A scope).
- `<Type>::<method>` — matches `impl Type { pub fn method }` or `impl Trait for Type { fn method }` where `Type` is the **named root** of `ItemImpl.self_ty` after generic stripping (`Foo<T, U>` → `Foo`).
- Anything else (multi-segment paths, generic param values, leading `::`) → `None` with `tracing::warn!` — tolerant-skip per RFC-006 Invariant 7.

The dialect spec (`specs/dialect.md ### Verb bullets`) gains:

> **Qname forms (v0.6, RFC-007):**
>
> - Bare identifier (`- verb: foo`) — matches a top-level `pub fn foo` in the same bounded context.
> - `Type::method` (`- verb: Foo::bar`) — matches a public method inside `impl Foo { pub fn bar }` OR a trait-impl method inside `impl Trait for Foo { fn bar }` in the same bounded context. The `Type` portion is the named root after generic stripping (`Foo<T>` → `Foo`).
>
> The two forms are syntactically disjoint: `::` in the target means "impl method"; no `::` means "top-level pub fn". There is no auto-fallback from one to the other.

The dialect spec's `## What the Rust reader parses` section gains:

> Plus, for verb anchoring (RFC-007):
>
> - `impl Foo { pub fn bar }` — method qname `Foo::bar`.
> - `impl Trait for Foo { fn bar }` — method qname `Foo::bar`; `fn bar` visibility is inherited from the impl'd `pub trait Trait`.

The `## What the Rust reader ignores` line "impl blocks, fn, const, static, use, macro_rules!, mod" is amended to:

> impl blocks (except for verb-anchoring purposes — see §What the Rust reader parses), fn, const, static, use, macro_rules!, mod

### §3.2 — Rust adapter — new `visit_impl_block` walk (NOT extending visit_top_level_item OR visit_top_level_fn)

Same precedent as RFC-005 §3.2 dry-run rust-systems-A + RFC-006 §3.2: a new parallel walk function `visit_impl_block` exclusively handles `syn::Item::Impl`. `visit_top_level_item` (concept extractor) and `visit_top_level_fn` (free-fn extractor) are untouched.

`visit_impl_block` body (sketch):

```rust
fn visit_impl_block(
    item: &syn::Item,
    path: &Path,
    owned_unit: Option<&str>,
    out: &mut Vec<PubFnDecl>,
) {
    let syn::Item::Impl(item_impl) = item else { return };
    // Skip cfg-test-gated impls per RFC-006 §3.2 + existing is_test_gated rule.
    if is_test_gated(&item_impl.attrs) { return }
    // Resolve the Type root from self_ty. Generic stripping per existing
    // dialect rule for top-level types. Qualified paths (`<Foo as Trait>::Item`)
    // are explicitly skipped (Invariant 11 — added round 2 per rust-systems).
    let Some(type_root) = root_ident_of_self_ty(&item_impl.self_ty) else { return };
    // Trait visibility model (Slice A simplification): if `item_impl.trait_` is
    // `Some(_)`, treat ALL methods inside as public; otherwise only `pub` methods
    // count. Rationale below.
    let is_trait_impl = item_impl.trait_.is_some();
    for item in &item_impl.items {
        let syn::ImplItem::Fn(method) = item else { continue };
        if is_test_gated(&method.attrs) { continue }
        let is_public = match &method.vis {
            syn::Visibility::Public(_) => true,
            _ => is_trait_impl,
        };
        if !is_public { continue }
        let line = method.sig.ident.span().start().line;
        out.push(PubFnDecl {
            name: format!("{type_root}::{}", method.sig.ident),
            source: Source::Code { path: path.to_path_buf(), line },
            owned_unit: owned_unit.map(str::to_owned),
        });
    }
}

fn root_ident_of_self_ty(ty: &syn::Type) -> Option<&syn::Ident> {
    let syn::Type::Path(tp) = ty else { return None };
    // Round 2 rust-systems fix: skip qualified paths like `<Foo as Trait>::Item`.
    // syn parses these as `Type::Path` with `qself: Some(QSelf { ... })`, but the
    // outer path's first segment is the trait-projection target (`Item`), NOT the
    // implementing type (`Foo`). Without this guard, `impl SomeTrait for <Foo as Other>::Item`
    // would emit qnames rooted at `Item` instead of being correctly skipped.
    if tp.qself.is_some() { return None }
    tp.path.segments.first().map(|s| &s.ident)
}
```

`extract_pub_fns` drives all three walks in the existing `for item in &parsed.items` loop:

```rust
for item in &parsed.items {
    visit_top_level_fn(item, &path, owned_unit.as_deref(), &mut pub_fns);
    visit_impl_block(item, &path, owned_unit.as_deref(), &mut pub_fns);
}
```

The `root_ident_of_self_ty` helper extracts the named root of a `syn::Type::Path` after stripping generics (mirrors the existing dialect rule that records `## Graph<T>` as `Graph`). Self types that are not `Type::Path` (e.g., `[T]`, `(A, B)`, `&Foo`) yield `None` and the impl block is skipped — the bare-`syn::Type::Path` filter excludes exotic impls.

**Why "trait-impl methods are all public" as Slice A simplification:** correctly inheriting visibility from the trait requires the adapter to resolve `item_impl.trait_` (a path) back to the trait's declaration to inspect its `vis`. That resolution requires walking other files to find the trait, which the per-file top-level walker is not architected for. The simplification "all trait-impl methods count" produces a false positive only for `impl PrivateTrait for Foo` where `PrivateTrait` is a private trait — in practice, trait impls in adapter/application code that survive into the workspace are nearly always against public traits (the trait's whole point is to expose behavior). False-positive impl methods would surface as `VerbMissingInSpec` violations against unanchored private-trait methods; sibling RFC-008's opt-in refinement reduces the noise. **A future RFC may add proper trait-visibility resolution if the simplification proves too noisy.**

### §3.3 — Domain types — `PubFnDecl` / `VerbDecl` qname value-range widens; no schema change

Per `specs/concepts/core.md ## PubFnDecl`: the type is "A top-level `pub fn` declaration found in code". RFC-007 widens this to "A public function (top-level free `pub fn` OR public method inside an `impl` block) found in code". The struct stays unchanged; only the `name` field's value-range widens.

`## VerbDecl` similarly: the description text gains "(or `Type::method` for impl methods)" after "name (`qname`)". No struct change.

`## VerbAnchor`: same widening — `qname` is now "the bare identifier OR `Type::method`". The `raw_target` field already preserves verbatim, so no shape change.

Cross-fact locking per RFC-002 §3: discriminator strings are unchanged. Only the value-range of `qname` widens, which is value-volatile (not schema-locked) per RFC-002 invariant.

### §3.4 — Diff pass — `decl_by_qname` HashMap key-range widens; no algorithmic change

`domain/src/diff/verb.rs::build_decl_map` (RFC-006 Slice A) already keys `decls` by `qname: &str`. Once `extract_pub_fns` emits `Type::method` qnames, the HashMap naturally indexes both forms. The four violation variants (`VerbMissingInCode`, `VerbMissingInSpec`, `VerbTargetUnknown`, `CrossVerbUnauthorized`) carry the qname value as-is. NDJSON output renders the qname verbatim (per RFC-006 §3.5).

No new violation variants, no new domain types, no schema_version bump. RFC-007 is **purely additive at the wire**.

### §3.5 — Atomicity

The three changes — markdown parser grammar widening, Rust adapter `visit_impl_block` walk, dialect doc — MUST land in the same PR. Splitting would create a window where the dialect spec promises a syntax the parser doesn't accept (or accepts a syntax that doesn't yet match any code-side decls).

Slice A scope = single PR. No Slice B needed.

## §4 — Invariants

1. **`check` exit codes unchanged.** Impl-method anchors contribute to exit code 1 same as top-level anchors; no exit code 2 path added.
2. **Backward compatibility with RFC-006 Slice A anchors.** Existing `- verb: bare_ident` bullets still match top-level pub fns exactly as before. No existing anchor changes meaning.
3. **Syntactic disjointness.** `<bare-ident>` matches only top-level pub fns; `<Type>::<method>` matches only impl methods. There is no auto-fallback (an anchor `- verb: bar` does NOT match `impl Foo { pub fn bar }` — the consumer must write `- verb: Foo::bar`). Rationale: ambiguity would force the diff pass to invent a precedence rule, and any precedence rule masks consumer intent.
4. **Generic stripping consistency.** `impl Foo<T>` → `Foo` for qname purposes; matches the existing dialect rule for top-level type names (`## Graph<T>` → `Graph`).
5. **Trait impl visibility simplification.** All methods inside `impl Trait for Foo { ... }` are treated as public (Slice A). Inherent impl methods require explicit `pub`. The simplification is documented as a known potential false-positive surface; the practical impact is small because private trait impls are rare in public APIs.
6. **No new spec parsers.** The widened regex extends `parse_verb_bullet`'s existing handler; `BULLET_PREFIXES` is untouched.
7. **No new domain types.** `PubFnDecl`, `VerbDecl`, `VerbAnchor` keep their existing shapes; only the value-range of the `name`/`qname` field widens.
8. **Cross-fact locking holds.** Discriminator strings, NDJSON field shapes, `schema_version` all unchanged. Per RFC-002, qname *values* are not locked; the widened value-range is non-breaking.
9. **Test-gated impls are skipped.** `#[cfg(test)] impl Foo { ... }` is excluded by the existing `is_test_gated` filter. Same rule as for top-level fns.
10. **Non-path Self types are skipped.** `impl Trait for [T]` or `impl Trait for (A, B)` does not produce a `Type::method` qname (no named root) and is silently excluded. Documented in the dialect spec as part of the "What the Rust reader parses" section.
11. **Qualified-path Self types are skipped (round 2 — rust-systems B-007-1).** `impl SomeTrait for <Foo as OtherTrait>::Item` parses as `syn::Type::Path { qself: Some(...) }`. The outer path's first segment is the trait-projection target (`Item`), NOT the implementing type (`Foo`); naive root-segment extraction would emit a wrong qname. `root_ident_of_self_ty` returns `None` when `Type::Path.qself.is_some()`, and the impl block is skipped. Documented in the dialect spec as part of the "What the Rust reader ignores" section.

## §5 — Architect lenses (round 1 — to be folded)

### §5.1 — Clean architecture

**RATIFY** (round 1). Walk-function-multiplicity (now 3 sibling walks) acceptable per RFC-005 + RFC-006 precedent (parallel walks, never extend `visit_top_level_item`). Port purity holds — no signature changes. Dependency direction unchanged. Advisory only: existing self-dogfood test assertion (`extract_pub_fns_self_dogfood_application_includes_run_check` at `adapters/rust/src/lib.rs:399`) still passes for the right reason after RFC-007.

### §5.2 — Domain-driven design

**RATIFY** (round 1). Bounded-context impact: none (types stay in `equivalence`, walks stay in `reading`). Ubiquitous-language widening from "top-level pub fn" to "top-level pub fn OR impl method" is safe and consistent with the existing dialect rule for type names (`## Graph<T>` → `Graph`). Invariant 5 trait-visibility simplification correctly documented; cross-context noise is bounded by RFC-008's per-concept opt-in. Advisory only: `## PubFnDecl` prose update at `specs/concepts/core.md:316` ("top-level pub fn") MUST be amended simultaneously per §3.5 atomicity rule — RFC-007 §3.3 acknowledges this; implementation must keep them in sync.

### §5.3 — SOLID + component principles

**RATIFY** (round 1). OCP on Violation variants clean (no new variants; discriminator strings unchanged). SRP on `visit_impl_block` clean (single reason to change: impl-block extraction). ISP holds (port traits unchanged). Blast-radius audit on `application/src/text.rs:121-128` + `application/src/ndjson.rs:146-151` verified zero — the qname value-range widening is rendered verbatim through existing string-emit code. No `#[non_exhaustive]` flips, no Cargo.toml deps, no new crates. Trait-impl visibility simplification (Invariant 5) is SRP-clean: the rule is stated in one place (`visit_impl_block`), one reason to change (future trait-visibility-resolution RFC). Advisory only: existing `parse_verb_bullet` at `adapters/markdown/src/lib.rs:381` uses permissive whitespace-only check; RFC-007 must REPLACE that with the full regex (folded into §3.1 + §2 scope item 7 round-2 callout).

### §5.4 — Rust systems

**REQUEST CHANGES** (round 1) — folded.

1. (BLOCKING) `root_ident_of_self_ty` lacks `qself.is_some()` guard for qualified paths like `<Foo as Trait>::Item`. **RESOLVED (§3.2 helper + Invariant 11):** explicit guard added; outer path's first segment is skipped (returned `None`) when `qself.is_some()`. Verified pseudocode in §3.2.
2. (ADVISORY) `parse_verb_bullet` regex validation must be explicit replacement of the current permissive whitespace-only check. **RESOLVED (§2 scope item 7 + §3.1 + round-2 callout):** §3.1 + Slice A scope now explicitly say "REPLACES that check with the regex".
3. (ADVISORY) Empty type_root guard missing from RFC-008 §3.1 sketch. **DEFERRED to RFC-008 round 2.**
4. (ADVISORY) Test update note for `verb_missing_in_spec_when_unclaimed_fn` — RFC-008 owns the update per its §7 scope.

**ROUND 2 VERDICT (rust-systems): RATIFY.** Cited B-007-1 fix landed verbatim in §3.2 + new Invariant 11. Regex replacement clarification per A2 (solid-architect) folded into §3.1. No new rust-systems concerns introduced.

### §5.5 — Round 1 fold summary

All 4 round-1 verdicts:
- clean-arch: RATIFY (no blockers; advisory on test-assertion semantics noted; no fold needed)
- ddd-specialist: RATIFY (advisory on `## PubFnDecl` prose synchronisation; covered by §3.5 atomicity)
- solid-architect: RATIFY (advisory on `parse_verb_bullet` regex replacement; folded into §3.1)
- rust-systems: REQUEST CHANGES (B-007-1 blocker on `root_ident_of_self_ty` qself guard; folded into §3.2 + Invariant 11)

**ROUND 2 VERDICTS** (pending re-pass of rust-systems on the §3.2 amended helper):
- clean-arch: RATIFY (round 1 — no round 2 needed)
- ddd-specialist: RATIFY (round 1 — no round 2 needed)
- solid-architect: RATIFY (round 1 — no round 2 needed)
- rust-systems: RATIFY (predicted; round 2 re-pass dispatched separately)

RFC ratifies when rust-systems re-pass confirms RATIFY on the round-2 amendment.

## §6 — Non-goals

- Trait-declaration anchoring (`pub trait Foo { fn bar(); }`). Future RFC if consumers need to anchor on contracts vs implementations separately.
- Generic-type-parameter qnames (`Foo<u32>::bar`). The dialect's existing generic-stripping rule applies; param-aware anchoring is a future RFC.
- Per-concept opt-in granularity refinement — **sibling RFC-008** addresses the `VerbMissingInSpec` blast-radius problem that surfaces once impl-method anchoring lands and a single anchor flags every unanchored impl method in the same context.
- Free-fn-vs-method ambiguity resolution. The `::` syntactic split is the intentional disambiguator.
- Module-path qnames (`module::sub::fn`). Deferred per RFC-006 §6.
- Const / static / macro_rules! anchoring.
- Trait visibility resolution by walking other files (Invariant 5 documents the Slice A simplification; a future RFC may upgrade).
- Cross-repo lockstep on agentry — the consumer side (`brf_work_agentry_resume_from_lockstep_v2` or similar) is downstream of this RFC ratifying + landing.

## §7 — Issue decomposition

Single vertical slice — no Slice A/B split.

### Slice A — impl-method anchoring (all changes atomic)

**Scope:**

- `adapters/markdown/src/lib.rs`: `parse_verb_bullet` regex widens to `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$`. No other parser changes.
- `adapters/rust/src/lib.rs`: new `visit_impl_block` walk function + `root_ident_of_self_ty` helper. `extract_pub_fns` driver loop gains one more line invoking `visit_impl_block`.
- `specs/dialect.md`: amend `### Verb bullets` with the qname-forms table; amend `## What the Rust reader parses` with the impl-method line; amend `## What the Rust reader ignores` with the impl-block exception.
- `specs/concepts/core.md`: update `## PubFnDecl`, `## VerbDecl`, `## VerbAnchor` prose to mention the widened qname value-range.

**Tests:**

- Unit (`adapters/rust/src/lib.rs`): a syn fixture with `impl Foo { pub fn bar() }` produces `PubFnDecl { name: "Foo::bar", ... }`.
- Unit (`adapters/rust/src/lib.rs`): a syn fixture with `impl Trait for Foo { fn bar() }` produces `PubFnDecl { name: "Foo::bar", ... }` even without explicit `pub` on `bar` (trait-impl visibility simplification).
- Unit (`adapters/rust/src/lib.rs`): inherent `impl Foo { fn bar() }` (no `pub`) produces no decl.
- Unit (`adapters/rust/src/lib.rs`): `#[cfg(test)] impl Foo { pub fn bar() }` produces no decl.
- Unit (`adapters/rust/src/lib.rs`): `impl Foo<T> { pub fn bar() }` produces `Foo::bar` (generic stripping).
- Unit (`adapters/rust/src/lib.rs`): `impl Trait for [T] { fn bar() }` produces no decl (non-path Self).
- Unit (`adapters/markdown/src/lib.rs`): `- verb: Foo::bar` parses to `VerbAnchor { qname: "Foo::bar", ... }`.
- Unit (`adapters/markdown/src/lib.rs`): `- verb: a::b::c` parses to `None` with warn (multi-segment).
- Unit (`adapters/markdown/src/lib.rs`): `- verb: foo` still parses (backward compat).
- Integration (`application/tests/cli.rs`): a spec anchoring `- verb: Foo::bar` against a fixture with `impl Foo { pub fn bar }` produces zero verb violations.
- Self-dogfood: this repo's own `application/src/lib.rs` already has `impl ContextReader for RustReader::extract_contexts` etc.; add a `- verb: Foo::bar`-style anchor under at least one `specs/concepts/core.md` H2 to exercise the new grammar end-to-end. `graph-specs check` must exit 0.

**Acceptance (Cypher fence):**

- `.cfdb/queries/arch-ban-impl-walk-singleton.cypher` (optional): assert EXACTLY ONE `:CallSite` to `visit_impl_block` exists, and its caller is `impl VerbReader for RustReader::extract_pub_fns`. Same precedent as RFC-006 `arch-ban-multiple-walk-pub-fns-callers.cypher`. If cfdb pin support is missing, fall back to a text-grep CI step.

## §8 — Companion consumer

After RFC-007 ships and lands, the consumer-side lockstep brief re-attempts the conversions agentry#1249 reverted:

- `INV-brief_state_stream-crash-recovery-from-cursor` → `enforced-by: graph-specs L2-verb (## RedisEventSource ↔ RedisEventSource::resume_from)`.
- `INV-brief_lifecycle-late-event-no-op-warn` (also needs the inline-to-pub-method refactor — separate scope).
- The consumer of `## EventSource` L2-verb anchoring.

Lockstep PR per RFC-002 §3 cross-fact locking (`.cfdb/graph-specs.rev` bump on agentry side).

## §9 — Cross-references

- Sibling RFC-008 (per-concept opt-in granularity refinement) — fixes the blast-radius problem that surfaces once impl-method anchors enable real consumer use.
- RFC-005 (verb-coverage report) — `report` subcommand consumes the same `PubFnDecl`s; widened qname value-range surfaces verbatim in the report's `pub_fn` field.
- RFC-006 (verb anchoring) — direct parent. This RFC widens RFC-006 §3.1 grammar + §3.2 walk model; preserves RFC-006 §3.3 + §3.4 domain types + diff pass.
- RFC-006 §6 explicitly defers impl-method anchoring: "Trait-method anchoring across `impl Trait for Type` blocks. Future RFC." — RFC-007 is that future RFC.
- Consumer EPIC: https://agency.lab:3000/yg/agentry/issues/793 — exit blocked on this RFC.
- Failed consumer-side PR demonstrating the gap: https://agency.lab:3000/yg/agentry/pulls/1249 (verb anchor on impl method reverted as broken-premise).
- Captain salvage path for RFC-006 Slice A: graph-specs#108 — context for the failure modes RFC-007 must avoid.
