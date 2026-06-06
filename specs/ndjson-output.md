# `graph-specs check --format=ndjson` — output schema

Authoritative wire contract for the `ndjson` output format introduced in v0.4.

Consumers (e.g. qbot-core's `compare-spec-delta` and Study 002 Phase A1 pipeline) MUST reference this document, not the source code, as the stable contract.

## Invocation

```bash
graph-specs check --specs <path> --code <path> --format ndjson
```

`--format` accepts `text` (default, human-readable) or `ndjson`. The text format is unchanged from v0.1–v0.3.

## Exit codes

Identical to `--format=text`:

- `0` — zero violations
- `1` — one or more violations, none fatal
- `2` — reader error **or** any `SignatureUnparseable` violation

## Output shape

One line per violation. Each line is a single JSON object terminated by `\n`. A clean tree produces **no output** (not `[]`, not `{}`, not `0 violations.` — empty stdout).

Consumers MUST parse line-by-line. The file is NOT a JSON array.

## Record: top-level fields

Every record carries these two fields at the top level:

| Field | Type | Value |
|---|---|---|
| `schema_version` | string | `"3"` — bumped on breaking schema changes (v0.4 bump: three bounded-context variants; **v3 (RFC-010): three abstraction-ladder `Cohesion` variants**). Report records (`graph-specs report`, §Report records) version independently and remain `"2"`. |
| `violation` | string | snake_case discriminator, one of the values below |

Additional fields are per-variant (see below).

## Source location object

Every violation carries at least one source location. The shape is:

```json
{ "kind": "spec" | "code", "path": "...", "line": <integer> }
```

- `kind: "spec"` — location is inside a markdown spec file
- `kind: "code"` — location is inside a Rust source file
- `path` — the reader-emitted path (typically repo-relative, but the tool does not normalize — consumers SHOULD NOT assume normalization)
- `line` — 1-based line number

## Variants

### `missing_in_code`

Concept declared in specs, absent from code.

```json
{"schema_version":"3","violation":"missing_in_code","concept":"Foo","source":{"kind":"spec","path":"specs/core.md","line":12}}
```

Field `source` is always `kind: "spec"`.

### `missing_in_specs`

Concept declared in code, absent from specs.

```json
{"schema_version":"3","violation":"missing_in_specs","concept":"Bar","source":{"kind":"code","path":"src/lib.rs","line":3}}
```

Field `source` is always `kind: "code"`.

### `signature_drift`

Both sides declare the concept with a signature; signatures disagree after normalization.

```json
{"schema_version":"3","violation":"signature_drift","concept":"Reader","spec_sig":"fn extract(&self)","code_sig":"fn extract(&self, root: &Path)","spec_source":{"kind":"spec","path":"specs/core.md","line":44},"code_source":{"kind":"code","path":"ports/src/lib.rs","line":15}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `spec_sig` | string | normalized signature as the spec's fenced rust block declares |
| `code_sig` | string | normalized signature extracted from the syn AST |
| `spec_source` | source object (kind=spec) | where in the spec |
| `code_source` | source object (kind=code) | where in the code |

### `signature_missing_in_spec`

Code declares a signature; spec has the concept heading but no fenced rust block.

```json
{"schema_version":"3","violation":"signature_missing_in_spec","concept":"Reader","code_sig":"fn extract(&self, root: &Path)","code_source":{"kind":"code","path":"ports/src/lib.rs","line":15}}
```

### `signature_unparseable`

Spec's fenced rust block failed to parse via `syn`. The concept is dropped from signature comparison until the spec is fixed. **This variant triggers exit code 2.**

```json
{"schema_version":"3","violation":"signature_unparseable","concept":"Broken","raw":"fn foo(","error":"expected `)`","source":{"kind":"spec","path":"specs/broken.md","line":9}}
```

### `edge_missing_in_code`

Spec declares a relationship edge (`- implements: Foo`, `- depends on: Bar`, `- returns: Baz`) that the code side does not emit.

```json
{"schema_version":"3","violation":"edge_missing_in_code","concept":"MarkdownReader","edge_kind":"IMPLEMENTS","target":"Reader","spec_source":{"kind":"spec","path":"specs/core.md","line":7}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `edge_kind` | string | one of `"IMPLEMENTS"`, `"DEPENDS_ON"`, `"RETURNS"` (stable wire labels) |
| `target` | string | the concept named in the relationship |
| `spec_source` | source object (kind=spec) | location of the bullet in the spec |

### `edge_missing_in_spec`

Code emits a relationship edge the spec does not declare. Fires only for concepts whose spec section declared at least one edge bullet (opt-in per concept).

```json
{"schema_version":"3","violation":"edge_missing_in_spec","concept":"MarkdownReader","edge_kind":"DEPENDS_ON","target":"Graph","code_source":{"kind":"code","path":"adapters/markdown/src/lib.rs","line":42}}
```

### `edge_target_unknown`

Spec bullet names a target concept that is not present as a concept in either graph.

```json
{"schema_version":"3","violation":"edge_target_unknown","concept":"MarkdownReader","edge_kind":"RETURNS","target":"Frobnicator","spec_source":{"kind":"spec","path":"specs/core.md","line":50}}
```

### `context_membership_unknown` (v2, v0.4)

A `pub` type in code lives in a crate that is not listed under any declared context's `Owns` block.

```json
{"schema_version":"3","violation":"context_membership_unknown","concept":"Orphan","owned_unit":"stray-crate","source":{"kind":"code","path":"stray-crate/src/lib.rs","line":3}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `owned_unit` | string | the path-directory prefix where the orphan concept lives (e.g. `domain`, `adapters/markdown`) |
| `source` | source object (kind=code) | where the orphan is defined |

### `cross_context_edge_unauthorized` (v2, v0.4)

A v0.3 edge targets a concept in another context that is NOT listed in the owning context's `Imports` declarations.

```json
{"schema_version":"3","violation":"cross_context_edge_unauthorized","concept":"MarkdownReader","owning_context":"reading","edge_kind":"DEPENDS_ON","target":"TradingPort","target_context":"trading","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `owning_context` | string | the declared context of the edge's source concept |
| `edge_kind` | string | `"IMPLEMENTS"` / `"DEPENDS_ON"` / `"RETURNS"` |
| `target` | string | the concept the edge points at |
| `target_context` | string | the declared context of the target concept |
| `spec_source` | source object (kind=spec) | location of the `Imports` section that failed to authorize the edge |

### `cross_context_edge_undeclared` (v2, v0.4)

A v0.3 edge crosses a context boundary, IS listed in the importing context's `Imports`, but the target context's spec does not declare the import back as an `Exports` entry (asymmetric declaration).

```json
{"schema_version":"3","violation":"cross_context_edge_undeclared","concept":"MarkdownReader","owning_context":"reading","edge_kind":"IMPLEMENTS","target":"Reader","target_context":"equivalence","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12}}
```

Same field shape as `cross_context_edge_unauthorized`. The difference is the cause: `unauthorized` means "you didn't ask"; `undeclared` means "you asked but they don't publish that."

### `implements_draft_concept` (additive at v0.4; rides the current schema_version)

A `pub` code item whose name matches a concept heading living in a `status: draft` spec file. The draft imposes no code-existence obligation, but implementing it while the spec is still draft leaves the code item with no active owning heading. Distinct from `missing_in_specs` (where no heading exists anywhere) — here a draft heading exists but is not yet ratified. It was added as an **additive** variant (it did not bump the version on its own — see §Schema evolution), and like every record now carries the current `schema_version` (`"3"`).

```json
{"schema_version":"3","violation":"implements_draft_concept","name":"Widget","draft_source":{"kind":"spec","path":"specs/concepts/drafts.md","line":7}}
```

| Field | Type | Meaning |
|---|---|---|
| `name` | string | the concept name (code item name = draft heading name) |
| `draft_source` | source object (kind=spec) | location of the draft heading |

**Remediation:** either promote the draft (flip the `status:` field to ratified, set `code_landing_pr`) or remove the code item until the spec is ratified.

## v3 cohesion variants (RFC-010 — the abstraction ladder)

The `Cohesion` variants check the upward concept→context rung: that a `concepts/` file's `H1` declares a real bounded context, and that each concept is documented under the context the code resolves it to. They are the breaking change that justifies the v2 → v3 bump. Spec-side variants fire with zero code facts; `concept_context_mismatch` is code-fact-gated (needs `specs/contexts/` Owns).

### `context_without_cohesion_unit`

An `H1` context heading with no `H2`/`H3` concept under it — a bounded context that declares no cohesion unit.

```json
{"schema_version":"3","violation":"context_without_cohesion_unit","context":"reading","file":"specs/concepts/reading.md"}
```

| Field | Type | Meaning |
|---|---|---|
| `context` | string | the normalised context identifier from the `H1` |
| `file` | string | the offending `concepts/` file |

### `sub_concept_orphan`

An `H3` sub-concept with no enclosing `H2` concept (a depth skip).

```json
{"schema_version":"3","violation":"sub_concept_orphan","sub_concept":"InnerThing","file":"specs/concepts/reading.md"}
```

| Field | Type | Meaning |
|---|---|---|
| `sub_concept` | string | the orphaned `H3` heading text |
| `file` | string | the offending `concepts/` file |

### `concept_context_mismatch`

A concept's spec-side declared owning context (its `concepts/` H1, with `specs/contexts/` export precedence) disagrees with the context the code resolves it to (the `specs/contexts/` Owns block owning the crate the `pub` type lives in).

```json
{"schema_version":"3","violation":"concept_context_mismatch","concept":"Widget","declared":"reading","code_context":"equivalence","spec_source":{"kind":"spec","path":"specs/concepts/reading.md","line":7}}
```

| Field | Type | Meaning |
|---|---|---|
| `concept` | string | the concept name |
| `declared` | string | the spec-side declared owning context |
| `code_context` | string | the context the code resolves the concept to |
| `spec_source` | source object (kind=spec) | where the concept is documented |

> **Planned additive extension (no further bump, tracked at #136):** source objects on code-bearing records will gain the agnostic provenance triple (`module_path` / `unit` / `context`). Because these are optional additive fields (see §Schema evolution), they will NOT bump `schema_version` again.

## v0.7 anchor variant (RFC-012 — non-`pub` spec anchors)

### `dangling_anchor` (additive at v0.7; rides the current `schema_version` `"3"`)

A concept's `- impl: <qname>` anchor (RFC-012 §3.2) names a code item that does not exist anywhere in the code tree, at any visibility. The equivalence-defect analog of `missing_in_code` for an anchored concept: the anchor redirected the concept's target to `qname`, and `qname` did not resolve. A **top-level** variant (not nested under `Cohesion`) so a consumer that opts out of cohesion checking cannot silently suppress broken-anchor detection. Added as an **additive** variant — it did not bump the version (see §Schema evolution).

```json
{"schema_version":"3","violation":"dangling_anchor","concept":"ValidateIntakeFull","target":"validate_intake","source":{"kind":"spec","path":"specs/concepts/intake_validation.md","line":3}}
```

| Field | Type | Meaning |
|---|---|---|
| `concept` | string | the anchored concept heading |
| `target` | string | the `- impl:` qname that did not resolve |
| `source` | source object (kind=spec) | where the anchor bullet is declared |

> An anchor whose target **does** resolve emits **no** record — the concept is satisfied (no `missing_in_code`). Resolution honours any visibility (`pub`, `pub(crate)`, a `fn`, a `const`), so a concept whose canonical implementation is `pub(crate)` need not manufacture a `pub` type.

## v0.5 forward-compat — `unknown_context_violation`

`ContextViolation` carries `#[non_exhaustive]` in the domain type. If a future v0.5 adds a variant not known to this tool version, the record emits with `"violation":"unknown_context_violation"` and the `concept` field only. Consumers SHOULD treat unknown variants as tripwires — the tool version on the producer side is ahead of the consumer's schema.

## Report records (v0.5)

`graph-specs report --verb-coverage --format=ndjson` emits one JSON object per
line, with `"record"` as the top-level discriminator (distinct from `"violation"`
used by `check`). All report records carry `"schema_version":"2"` — this is an
**additive** extension; existing `violation` records are structurally unchanged.

### `verb_coverage`

One record per `pub fn` found in the code tree.

```json
{"record":"verb_coverage","schema_version":"2","context":"equivalence","pub_fn":{"name":"run_check","source":{"kind":"code","path":"application/src/lib.rs","line":33}},"cited":true}
```

| Field | Type | Meaning |
|---|---|---|
| `context` | string or null | bounded context that owns the fn's crate; `null` if the crate is not declared under any `Owns` block |
| `pub_fn.name` | string | function name |
| `pub_fn.source` | source object | location of the declaration (`kind` always `"code"`) |
| `cited` | bool | `true` if the fn name appears as a concept node in the spec graph |

### `tier_histogram`

One record per (context, tier) pair that has at least one annotation.

```json
{"record":"tier_histogram","schema_version":"2","context":null,"tier":"cypher","count":4}
```

| Field | Type | Meaning |
|---|---|---|
| `context` | string or null | bounded context; `null` for orphaned annotations |
| `tier` | string | one of `"cypher"`, `"tier0"`, `"script_fence"`, `"prose_only"` |
| `count` | integer | number of invariant annotations at this tier within the context |

### `homonym`

One record per name that appears in more than one bounded context.

```json
{"record":"homonym","schema_version":"2","name":"Foo","contexts":[{"context":"ctx_a","sanctioned_by_pattern":"PublishedLanguage","asymmetric":false},{"context":"ctx_b","sanctioned_by_pattern":null,"asymmetric":true}]}
```

| Field | Type | Meaning |
|---|---|---|
| `name` | string | the shared name |
| `contexts` | array | one entry per context where the name appears |
| `contexts[].context` | string | context name |
| `contexts[].sanctioned_by_pattern` | string or null | `"PublishedLanguage"` / `"SharedKernel"` / `"Conformist"` / `"CustomerSupplier"` or `null` when undeclared |
| `contexts[].asymmetric` | bool | `true` when the export and import patterns disagree for this name in this context |

### Sort order

All three record types follow the same stable sort as `--format=text`:
verb-coverage and tier-histogram sort by context name (null last), then by fn
name / tier discriminant within each context. Homonyms sort by name.

## Schema evolution

`schema_version` is a string, not a semver tuple. Consumers compare it against the exact string they were built against.

Compatible (non-breaking) changes — **no version bump**:
- Adding a new variant to the `violation` enum
- Adding a new top-level field with a default/optional meaning
- Widening a string value's permitted set

Breaking changes — **`schema_version` increments** (e.g., `"1"` → `"2"` → `"3"`):
- Removing a field
- Renaming a field or a `violation` discriminator value
- Changing a field's JSON type
- Changing the interpretation of an existing `violation` discriminator

Version history:
- `"1"` — v0.1–v0.3 (concept / signature / edge variants).
- `"2"` — v0.4 added the bounded-context variants (`context_membership_unknown`, `cross_context_edge_*`, `cross_verb_unauthorized`).
- `"3"` — RFC-010 added the abstraction-ladder `Cohesion` variants (`context_without_cohesion_unit`, `sub_concept_orphan`, `concept_context_mismatch`). Consumers dispatch on `"3"`; the qbot-core `compare-spec-change` lockstep arm is tracked at #135. Like `ContextViolation`, `CohesionViolation` is `#[non_exhaustive]` — an unknown future cohesion variant emits `"violation":"unknown_cohesion_violation"` as a tripwire.

## Determinism

Record order reflects the order `domain::diff()` returns violations, which is deterministic for a fixed input tree. Consumers SHOULD NOT rely on a particular order across tool versions.

## Relationship to `--format=text`

The two formats emit the same **set** of violations; they differ only in wire form. Exit codes are identical. When both are needed, run the tool twice; the cost is linear in the input tree.
