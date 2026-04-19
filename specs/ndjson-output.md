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
| `schema_version` | string | `"2"` — bumped on breaking schema changes (v0.4 bump: added three bounded-context variants) |
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
{"schema_version":"2","violation":"missing_in_code","concept":"Foo","source":{"kind":"spec","path":"specs/core.md","line":12}}
```

Field `source` is always `kind: "spec"`.

### `missing_in_specs`

Concept declared in code, absent from specs.

```json
{"schema_version":"2","violation":"missing_in_specs","concept":"Bar","source":{"kind":"code","path":"src/lib.rs","line":3}}
```

Field `source` is always `kind: "code"`.

### `signature_drift`

Both sides declare the concept with a signature; signatures disagree after normalization.

```json
{"schema_version":"2","violation":"signature_drift","concept":"Reader","spec_sig":"fn extract(&self)","code_sig":"fn extract(&self, root: &Path)","spec_source":{"kind":"spec","path":"specs/core.md","line":44},"code_source":{"kind":"code","path":"ports/src/lib.rs","line":15}}
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
{"schema_version":"2","violation":"signature_missing_in_spec","concept":"Reader","code_sig":"fn extract(&self, root: &Path)","code_source":{"kind":"code","path":"ports/src/lib.rs","line":15}}
```

### `signature_unparseable`

Spec's fenced rust block failed to parse via `syn`. The concept is dropped from signature comparison until the spec is fixed. **This variant triggers exit code 2.**

```json
{"schema_version":"2","violation":"signature_unparseable","concept":"Broken","raw":"fn foo(","error":"expected `)`","source":{"kind":"spec","path":"specs/broken.md","line":9}}
```

### `edge_missing_in_code`

Spec declares a relationship edge (`- implements: Foo`, `- depends on: Bar`, `- returns: Baz`) that the code side does not emit.

```json
{"schema_version":"2","violation":"edge_missing_in_code","concept":"MarkdownReader","edge_kind":"IMPLEMENTS","target":"Reader","spec_source":{"kind":"spec","path":"specs/core.md","line":7}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `edge_kind` | string | one of `"IMPLEMENTS"`, `"DEPENDS_ON"`, `"RETURNS"` (stable wire labels) |
| `target` | string | the concept named in the relationship |
| `spec_source` | source object (kind=spec) | location of the bullet in the spec |

### `edge_missing_in_spec`

Code emits a relationship edge the spec does not declare. Fires only for concepts whose spec section declared at least one edge bullet (opt-in per concept).

```json
{"schema_version":"2","violation":"edge_missing_in_spec","concept":"MarkdownReader","edge_kind":"DEPENDS_ON","target":"Graph","code_source":{"kind":"code","path":"adapters/markdown/src/lib.rs","line":42}}
```

### `edge_target_unknown`

Spec bullet names a target concept that is not present as a concept in either graph.

```json
{"schema_version":"2","violation":"edge_target_unknown","concept":"MarkdownReader","edge_kind":"RETURNS","target":"Frobnicator","spec_source":{"kind":"spec","path":"specs/core.md","line":50}}
```

### `context_membership_unknown` (v2, v0.4)

A `pub` type in code lives in a crate that is not listed under any declared context's `Owns` block.

```json
{"schema_version":"2","violation":"context_membership_unknown","concept":"Orphan","owned_unit":"stray-crate","source":{"kind":"code","path":"stray-crate/src/lib.rs","line":3}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `owned_unit` | string | the path-directory prefix where the orphan concept lives (e.g. `domain`, `adapters/markdown`) |
| `source` | source object (kind=code) | where the orphan is defined |

### `cross_context_edge_unauthorized` (v2, v0.4)

A v0.3 edge targets a concept in another context that is NOT listed in the owning context's `Imports` declarations.

```json
{"schema_version":"2","violation":"cross_context_edge_unauthorized","concept":"MarkdownReader","owning_context":"reading","edge_kind":"DEPENDS_ON","target":"TradingPort","target_context":"trading","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12}}
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
{"schema_version":"2","violation":"cross_context_edge_undeclared","concept":"MarkdownReader","owning_context":"reading","edge_kind":"IMPLEMENTS","target":"Reader","target_context":"equivalence","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12}}
```

Same field shape as `cross_context_edge_unauthorized`. The difference is the cause: `unauthorized` means "you didn't ask"; `undeclared` means "you asked but they don't publish that."

## v0.5 forward-compat — `unknown_context_violation`

`ContextViolation` carries `#[non_exhaustive]` in the domain type. If a future v0.5 adds a variant not known to this tool version, the record emits with `"violation":"unknown_context_violation"` and the `concept` field only. Consumers SHOULD treat unknown variants as tripwires — the tool version on the producer side is ahead of the consumer's schema.

## Schema evolution

`schema_version` is a string, not a semver tuple. Consumers compare it against the exact string they were built against.

Compatible (non-breaking) changes — **no version bump**:
- Adding a new variant to the `violation` enum
- Adding a new top-level field with a default/optional meaning
- Widening a string value's permitted set

Breaking changes — **`schema_version` increments** (e.g., `"1"` → `"2"`):
- Removing a field
- Renaming a field or a `violation` discriminator value
- Changing a field's JSON type
- Changing the interpretation of an existing `violation` discriminator

## Determinism

Record order reflects the order `domain::diff()` returns violations, which is deterministic for a fixed input tree. Consumers SHOULD NOT rely on a particular order across tool versions.

## Relationship to `--format=text`

The two formats emit the same **set** of violations; they differ only in wire form. Exit codes are identical. When both are needed, run the tool twice; the cost is linear in the input tree.
