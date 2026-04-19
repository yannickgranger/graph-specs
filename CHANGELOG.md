# Changelog

All notable changes to graph-specs-rust are recorded in this file.

The project ships logical capability versions (`v0.1`, `v0.2`, ...) rather
than per-crate semver bumps. Each capability is negotiated in an RFC
under `docs/rfc/` before implementation; the capability number below
indexes into that directory.

## v0.4 — 2026-04-19 — Bounded-context equivalence

Reference: `docs/rfc/001-bounded-context-equivalence.md`.

### Added

- **Fourth equivalence level**: cross-context edge enforcement. The
  diff engine gains a fourth pass that detects three new violation
  variants — `ContextMembershipUnknown`, `CrossContextEdgeUnauthorized`,
  and `CrossContextEdgeUndeclared` — all wrapped under
  `Violation::Context(ContextViolation)`.
- **Spec-side vocabulary** for declaring bounded contexts in
  `specs/contexts/<name>.md`: `Owns`, `Exports`, `Imports` sections,
  with four DDD patterns supported — `SharedKernel`,
  `CustomerSupplier`, `Conformist`, `PublishedLanguage`. Other patterns
  (Anti-Corruption Layer, Separate Ways, Open Host Service) deferred
  to a future RFC.
- **`ContextReader` port** in the `ports` crate; `MarkdownReader` now
  implements both `Reader` and `ContextReader`. The Rust reader remains
  `Reader`-only by design.
- **`CheckInput`** type in domain: `diff(CheckInput { graph, contexts },
  code)` replaces the `diff(spec_graph, code_graph)` shape. The old
  shape is still callable with an empty contexts list — see
  *Deprecations / compatibility* below.
- **Self-dogfood**: graph-specs-rust declares its own three contexts
  under `specs/contexts/{equivalence,reading,orchestration}.md`. CI
  enforces zero cross-context drift on every push.
- **CLI invocation shift**: `graph-specs check --specs specs/` now
  picks up both concept-level (`specs/concepts/`) and context-level
  (`specs/contexts/`) files. The CLI accepts the parent `specs/` dir;
  each reader self-scopes via directory-name convention.
- **Three cfdb ban rules** under `.cfdb/queries/` encoding context
  invariants at the syn-fact level: `arch-context-no-cross-layer-unwrap`,
  `arch-context-no-application-in-domain`, `arch-context-no-syn-in-domain`
  (graph-specs-rust#29).

### Changed — **BREAKING for wire consumers**

- **NDJSON schema bumped `"1"` → `"2"`**. Every record now carries
  `"schema_version":"2"` at the top level. Consumers MUST read
  `schema_version` before deserialising payloads and MUST NOT assume
  the old string.
- **New `violation` discriminator values** added to the enum
  (`context_membership_unknown`, `cross_context_edge_unauthorized`,
  `cross_context_edge_undeclared`, `unknown_context_violation`). Per
  `specs/ndjson-output.md` §Schema evolution, a new variant alone
  would NOT bump the schema — the v1→v2 bump is driven by the record
  shape for these new variants carrying nested `spec_source` +
  `target_context` fields rather than the v1 single-`source` shape.
- **Deterministic sort rank**: `Violation::Context(_)` takes rank 8 in
  `violation_key()`, after the seven v0.1–v0.3 variants. Consumers
  relying on a specific variant ordering SHOULD re-verify.

### Consumer migration — overlap window

Downstream consumers (notably qbot-core's Study 002 v4.2 Phase A1
`compare-spec-delta` pipeline) must version-gate parsing:

```
let version = line.get("schema_version").as_str();
match version {
    "1" => parse_v1(line),
    "2" => parse_v2(line),
    other => Err(UnsupportedSchema(other)),
}
```

**Overlap window policy**: consumers are expected to support both
`"1"` and `"2"` for at least the duration of one major consumer
release cycle after v0.4 ships. Producers (this tool) emit only v2
from v0.4 onward — a consumer that still reads v1-only will fail
loudly on the `schema_version` tripwire, not silently mis-parse. The
`unknown_context_violation` forward-compat variant ensures the tool
can land future context-level changes as non-breaking additions while
v2 still stands.

### Deprecations / compatibility

- `diff(graph, code)` pre-v0.4 callers: the new `diff(CheckInput, code)`
  shape accepts `CheckInput::new(graph, Vec::new())` as the v0.3
  equivalent. An empty contexts list short-circuits the fourth pass —
  no `Violation::Context(_)` ever fires. v0.3 spec trees (no
  `specs/contexts/` subdir) continue to pass unchanged.
- CLI: `graph-specs check --specs specs/concepts/ --code .` still
  works. The concept reader falls back to walking the passed directory
  when no `concepts/` subdir exists inside it.

### Cross-repo coordination

- qbot-core: tracking issue filed as `yg/qbot-core#4034` — parser
  update for `compare-spec-delta` to version-gate dispatch and
  support both v1 and v2 during the overlap window. The existing
  documentation sweep `yg/qbot-core#4025` has been nudged to cite
  RFC-001 / schema v2 at every §10 reference (in addition to the
  existing `yg/graph-specs-rust#13` v1 pin).
- cfdb: lockstep `SchemaVersion` pinning is unaffected — these are
  two different schemas (cfdb fact schema vs graph-specs NDJSON
  schema).

## v0.3 — 2026-04 — Declared relationship equivalence

Reference: `docs/rfc/` (pre-RFC; see issue #9 tracker).

Declared edges: `implements:`, `depends on:`, `returns:` bullets in
spec files produce `Edge` values; the Rust reader mirrors via
syn-level impl/field/return extraction. Three new violation variants
(`EdgeMissingInCode`, `EdgeMissingInSpec`, `EdgeTargetUnknown`).

## v0.2 — Signature-level equivalence

Fenced rust blocks in spec files produce normalised signatures
(`SignatureState::Normalized`), diffed against syn-parsed code
signatures. Two new violation variants (`SignatureDrift`,
`SignatureMissingInSpec`) plus `SignatureUnparseable` for malformed
inputs (exit code 2).

## v0.1 — Concept-level equivalence

MVP: `##`/`###` headings in markdown spec files produce concept
nodes, diffed against `pub` top-level items in Rust sources. Two
violation variants (`MissingInCode`, `MissingInSpecs`). Establishes
the NDJSON wire format at `schema_version:"1"`.
