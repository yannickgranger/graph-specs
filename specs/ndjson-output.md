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

Computed from violations alone (RFC-013 §4 invariant 3): no marker
record moves the exit code, under either marker value.

## Output shape

One line per record. Each line is a single JSON object terminated by `\n`. A tree with no violations **and no markers** produces no output at all (not `[]`, not `{}`, not a summary line — empty stdout).

Consumers MUST parse line-by-line. The file is NOT a JSON array.

## Record: top-level fields

Every record carries `schema_version` plus exactly one discriminator key:

| Field | Type | Value |
|---|---|---|
| `schema_version` | string | `"5"` — bumped on breaking schema changes (v0.4 bump: three bounded-context variants; v3 (RFC-010): three abstraction-ladder `Cohesion` variants; v4 (RFC-013): retirement of the `implements_draft_concept` discriminator value; **v5 (RFC-004 §3.5, amended 2026-09-07): `format` on every spec source, `language` on every code source, and the `signature_drift_within_side` variant**). Report records (`graph-specs report`, §Report records) version independently and remain `"2"`. |
| `violation` | string | snake_case discriminator on a **finding**, one of the values below |
| `marker` | string | discriminator on a **marker record** — `"pending"` or `"realized"` (RFC-013), `"retirement_incomplete"` or `"retirement_complete"` (RFC-015). See §Marker records |

Exactly one of `violation` / `marker` is present on any given record; the two are never combined. Additional fields are per-variant (see below).

## Source location object

Every violation carries at least one source location. The shape is:

```json
{ "kind": "spec", "path": "...", "line": <integer>, "format": "markdown" | "inline_attribute" }
{ "kind": "code", "path": "...", "line": <integer>, "language": "rust" | "php" | "typescript" }
```

- `kind: "spec"` — location is inside a spec source
- `kind: "code"` — location is inside a code source
- `path` — the reader-emitted path (typically repo-relative, but the tool does not normalize — consumers SHOULD NOT assume normalization)
- `line` — 1-based line number
- `format` — **present on every `kind: "spec"` object** (v5, RFC-004 §3.5): the authoring format of the spec fact. `markdown` covers `specs/concepts/` and `specs/contexts/` alike — the subdirectory split is a reader detail, not a domain concept; `inline_attribute` covers the `#[Spec(...)]` and `@Spec(...)` forms a language reader extracts
- `language` — **present on every `kind: "code"` object** (v5, RFC-004 §3.5): the runtime or toolchain that owns the code fact

The two field names are deliberately different rather than one `language`
field whose meaning depends on `kind`: a spec source has a format and no
language, a code source a language and no format, and neither can carry
the other's field. Markdown is not a `language` value — markdown is a spec
format, not a code language.

Both sets are open. Adding a value to either — a future `go`, say — is
additive and does NOT bump `schema_version` (see §Schema evolution); it is
the *fields* that were the breaking change, not their ranges.

### Provenance triple on `kind: "code"` source objects (additive, RFC-010 §3.6 / #136)

A `kind: "code"` source object MAY additionally carry the agnostic
containment provenance triple the check resolved for the record's concept:

```json
{ "kind": "code", "path": "...", "line": <integer>,
  "module_path": "...", "unit": "...", "context": "..." }
```

- `module_path` — the owning module path, crate-root-collapsed (e.g. `domain::diff`)
- `unit` — the owning crate / package, relative to the code root (e.g. `adapters/markdown`)
- `context` — the bounded context whose `specs/contexts/` Owns block owns `unit`

Each field is **optional and independently absent** — omitted, never
`null` — because each is independently unavailable: a tree without
`specs/contexts/` resolves no `context`; a `context_membership_unknown`
record by definition carries no `context`; a record keyed by something
that is not a code concept (e.g. `verb_missing_in_spec`'s `qname`)
carries none of the three. `kind: "spec"` source objects never carry
them — provenance is a code fact (RFC-010 §3.3).

These are optional additive fields (see §Schema evolution): they ride
the current `schema_version` and did NOT bump it. A consumer that reads
only `kind` / `path` / `line` is unaffected.

## Variants

### `missing_in_code`

Concept declared in specs, absent from code.

```json
{"schema_version":"5","violation":"missing_in_code","concept":"Foo","source":{"kind":"spec","path":"specs/core.md","line":12,"format":"markdown"}}
```

Field `source` is always `kind: "spec"`.

### `missing_in_specs`

Concept declared in code, absent from specs.

```json
{"schema_version":"5","violation":"missing_in_specs","concept":"Bar","source":{"kind":"code","path":"domain/src/lib.rs","line":3,"language":"rust","module_path":"domain","unit":"domain","context":"equivalence"}}
```

Field `source` is always `kind: "code"`, and carries the provenance
triple when resolved (see §Source location object).

### `signature_drift`

Both sides declare the concept with a signature; signatures disagree after normalization.

```json
{"schema_version":"5","violation":"signature_drift","concept":"Reader","spec_sig":"fn extract(&self)","code_sig":"fn extract(&self, root: &Path)","spec_source":{"kind":"spec","path":"specs/core.md","line":44,"format":"markdown"},"code_source":{"kind":"code","path":"ports/src/lib.rs","line":15,"language":"rust"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `spec_sig` | string | normalized signature as the spec's fenced rust block declares |
| `code_sig` | string | normalized signature extracted from the syn AST |
| `spec_source` | source object (kind=spec) | where in the spec |
| `code_source` | source object (kind=code) | where in the code |

### `signature_drift_within_side` (v5, RFC-004 §3.5)

Two readers **on the same side** give one concept two signatures — intra-side
drift, distinct from `signature_drift`, which is spec-versus-code.

```json
{"schema_version":"5","violation":"signature_drift_within_side","concept":"OrderService","side":"spec","sources":[{"kind":"spec","path":"specs/php/orders.md","line":10,"format":"markdown","sig":"public function place(): void"},{"kind":"spec","path":"src/Orders/OrderService.php","line":42,"format":"inline_attribute","sig":"public function place(Order $o): Receipt"}]}
```

| Extra field | Type | Meaning |
|---|---|---|
| `side` | string | `"spec"` or `"code"` — which side's readers disagree |
| `sources` | array of source objects | each carrying its own `sig`, one per disagreeing reader |

**Markdown is the canonical upstream on the spec side.** Both versions are
reported for human resolution and neither auto-wins; the inline attribute is
the downstream conformist (RFC-004 §4 invariant 7).

**No reader emits this variant yet.** It takes two readers on one side, and
this tool ships one spec reader. The first producer is `PhpAttributeReader`
(graph-specs-011 §3.3), whose unit lands after this one and deletes this
paragraph. Until then the variant is the wire contract's, not the checker's:
the schema describes the record a consumer must be ready to parse, and says
plainly that nothing writes it.

### `signature_missing_in_spec`

Code declares a signature; spec has the concept heading but no fenced rust block.

```json
{"schema_version":"5","violation":"signature_missing_in_spec","concept":"Reader","code_sig":"fn extract(&self, root: &Path)","code_source":{"kind":"code","path":"ports/src/lib.rs","line":15,"language":"rust"}}
```

### `signature_unparseable`

Spec's fenced rust block failed to parse via `syn`. The concept is dropped from signature comparison until the spec is fixed. **This variant triggers exit code 2.**

```json
{"schema_version":"5","violation":"signature_unparseable","concept":"Broken","raw":"fn foo(","error":"expected `)`","source":{"kind":"spec","path":"specs/broken.md","line":9,"format":"markdown"}}
```

### `edge_missing_in_code`

Spec declares a relationship edge (`- implements: Foo`, `- depends on: Bar`, `- returns: Baz`) that the code side does not emit.

```json
{"schema_version":"5","violation":"edge_missing_in_code","concept":"MarkdownReader","edge_kind":"IMPLEMENTS","target":"Reader","spec_source":{"kind":"spec","path":"specs/core.md","line":7,"format":"markdown"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `edge_kind` | string | one of `"IMPLEMENTS"`, `"DEPENDS_ON"`, `"RETURNS"` (stable wire labels) |
| `target` | string | the concept named in the relationship |
| `spec_source` | source object (kind=spec) | location of the bullet in the spec |

### `edge_missing_in_spec`

Code emits a relationship edge the spec does not declare. Fires only for concepts whose spec section declared at least one edge bullet (opt-in per concept).

```json
{"schema_version":"5","violation":"edge_missing_in_spec","concept":"MarkdownReader","edge_kind":"DEPENDS_ON","target":"Graph","code_source":{"kind":"code","path":"adapters/markdown/src/lib.rs","line":42,"language":"rust"}}
```

### `edge_target_unknown`

Spec bullet names a target concept that is not present as a concept in either graph.

```json
{"schema_version":"5","violation":"edge_target_unknown","concept":"MarkdownReader","edge_kind":"RETURNS","target":"Frobnicator","spec_source":{"kind":"spec","path":"specs/core.md","line":50,"format":"markdown"}}
```

### `context_membership_unknown` (v2, v0.4)

A `pub` type in code lives in a crate that is not listed under any declared context's `Owns` block.

```json
{"schema_version":"5","violation":"context_membership_unknown","concept":"Orphan","owned_unit":"stray-crate","source":{"kind":"code","path":"stray-crate/src/lib.rs","line":3,"language":"rust"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `owned_unit` | string | the path-directory prefix where the orphan concept lives (e.g. `domain`, `adapters/markdown`) |
| `source` | source object (kind=code) | where the orphan is defined |

### `cross_context_edge_unauthorized` (v2, v0.4)

A v0.3 edge targets a concept in another context that is NOT listed in the owning context's `Imports` declarations.

```json
{"schema_version":"5","violation":"cross_context_edge_unauthorized","concept":"MarkdownReader","owning_context":"reading","edge_kind":"DEPENDS_ON","target":"TradingPort","target_context":"trading","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12,"format":"markdown"}}
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
{"schema_version":"5","violation":"cross_context_edge_undeclared","concept":"MarkdownReader","owning_context":"reading","edge_kind":"IMPLEMENTS","target":"Reader","target_context":"equivalence","spec_source":{"kind":"spec","path":"specs/contexts/reading.md","line":12,"format":"markdown"}}
```

Same field shape as `cross_context_edge_unauthorized`. The difference is the cause: `unauthorized` means "you didn't ask"; `undeclared` means "you asked but they don't publish that."

### `malformed_anchor_bullet` (v0.8)

A `- verb:` or `- impl:` bullet whose qname the shared anchor grammar cannot read. `specs/dialect.md` admits exactly two forms — a bare identifier (`rename`) and `Type::method` (`Course::rename`) — and a qname outside them was previously **discarded in silence**, so the bullet had no effect and said nothing. keel-harness R1 and `graph-specs-010-abstraction-level-equivalence` §11.6 both forbid that: a reader states what it cannot read.

```json
{"schema_version":"5","violation":"malformed_anchor_bullet","concept":"Course","bullet":"verb","qname":"App\\Catalogue\\Course::rename","spec_source":{"kind":"spec","path":"specs/concepts/catalogue.md","line":5,"format":"markdown"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `bullet` | string | `"verb"` or `"impl"` — which anchor bullet carried it |
| `qname` | string | the qname as written, empty when the bullet named nothing |
| `spec_source` | source object (kind=spec) | the bullet's own site |

A malformed bullet produces **only** this record: it is not additionally a `dangling_anchor` or a `verb_missing_in_code`, because the tool never read a target to look for.

The case the message is written to make legible is a **namespace-qualified PHP name** — `App\Catalogue\Course::rename` — because that is the form a PHP author writes first and it is not one of the two the dialect admits. The admitted form for a PHP method is `Course::rename`, the class's short name.

**Remediation:** rewrite the bullet in one of the two admitted forms.

**Schema evolution.** Additive — a new `violation` discriminator rides the current `schema_version` (see §Schema evolution).

### `edge_unanswerable` (v0.8)

A spec heading declares a `- depends on:` or `- returns:` bullet, and the code input this run read emits **no fact of that kind at all**. The bullet is unanswered, not unmet: charging it to the specs as `edge_missing_in_code` would blame the author for a shortfall in the reader (`graph-specs-010-abstraction-level-equivalence` §11.6).

```json
{"schema_version":"5","violation":"edge_unanswerable","concept":"Course","edge_kind":"DEPENDS_ON","target":"Clock","spec_source":{"kind":"spec","path":"specs/concepts/catalogue.md","line":5,"format":"markdown"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `edge_kind` | string | the relationship the bullet declares |
| `target` | string | the concept the bullet points at |
| `spec_source` | source object (kind=spec) | the bullet's own site |

On a PHP keyspace the answerable set is `IMPLEMENTS` alone: cfdb's PHP producer emits no field-type or return-type edge, so `DEPENDS_ON` and `RETURNS` bullets are unanswerable there. On the source walk every kind is answerable and this record never appears.

**Remediation:** none available in the specs — the fact does not exist in the input. Either check the repository against an input whose producer emits that relationship, or accept the bullet as undecidable for this input.

**Schema evolution.** Additive — a new `violation` discriminator rides the current `schema_version` (see §Schema evolution).

### `surface_admits_nothing` (v0.8)

The declared surface admitted **no** concept-rung item while the keyspace holds `N` of them. Emitted **instead of** the per-heading `missing_in_code` records, which would otherwise report every heading as unrealized and make a declaration that matches nothing indistinguishable from a code tree that was deleted. One line naming `N` and the declared prefixes; the run stops there, because a code side that admitted nothing can say nothing true about equivalence.

```json
{"schema_version":"5","violation":"surface_admits_nothing","concept_rung_items":2,"declared_prefixes":["App\\Enrolment"],"keyspace":"/tmp/coreen.json"}
```

| Extra field | Type | Meaning |
|---|---|---|
| `concept_rung_items` | integer | how many concept-rung items the keyspace holds |
| `declared_prefixes` | array of string | every `Owns` entry across `specs/contexts/`, as declared |
| `keyspace` | string | the keyspace file the run read |

This record carries **no `concept`**: the finding is about the declaration and the keyspace, not about any one heading.

**Remediation:** declare a prefix that owns the tree's namespaces, or check the repository against the input its concepts actually live in.

**Schema evolution.** Additive — a new `violation` discriminator rides the current `schema_version` (see §Schema evolution).

### `cross_edge_off_surface` (v0.8)

A relationship edge the code side carries whose far end is an item no declared prefix owns — it is on the graph, so the producer resolved it in-workspace, but it is outside the declared surface. graph-specs-011-php-ladder#4 invariant 3 rules that such an **item** binds no heading and demands none; it does not rule the **edge** whose far end is that item, and an edge dropped for that reason would be a crossing the tool saw and did not say.

```json
{"schema_version":"5","violation":"cross_edge_off_surface","concept":"Course","owning_context":"catalogue","edge_kind":"IMPLEMENTS","target":"Serializable","code_source":{"kind":"code","path":"App\\Catalogue","line":0,"language":"php","module_path":"App\\Catalogue","unit":"App\\Catalogue"}}
```

| Extra field | Type | Meaning |
|---|---|---|
| `owning_context` | string or absent | the context owning the near end, absent when no declared prefix owns it either |
| `edge_kind` | string | `IMPLEMENTS`, `DEPENDS_ON` or `RETURNS` |
| `target` | string | the far end's concept name |
| `code_source` | source object (kind=code) | the near end's site; for a keyspace fact `path` is a namespace, never a file path |

**Remediation:** declare a prefix that owns the far end, or accept the crossing as a boundary of the declared surface and record it.

**Schema evolution.** Additive — a new `violation` discriminator rides the current `schema_version` (see §Schema evolution).

### `implements_draft_concept` — **RETIRED at v4** (RFC-013 §3.4)

This variant reported a `pub` code item whose name matched a heading in a `status: draft` spec. Under RFC-013 that condition is the normal, expected mid-arc state — the signal that a concept is **realized and ready to ratify** — not a failure. It is now a [`realized` marker record](#marker-records-v4-rfc-013), which does not affect the exit code.

The discriminator value `implements_draft_concept` is **no longer emitted**. Its removal is what makes v4 a breaking change rather than an additive one (see §Schema evolution). Consumers pinned to `"3"` keep working against archived output; consumers moving to `"4"` must drop their `implements_draft_concept` arm and read `marker` records instead.

## Marker records (v4, RFC-013)

Marker records report **spec state**, not failures. They ride the same NDJSON stream as violations under a separate top-level discriminator key — `marker`, never `violation` — so that:

- the existing `violation`-filtered stream is unchanged in shape, and
- the `marker`-filtered stream is exactly the ratification worklist.

The key is `marker` and not `report`, because `report` already names the `graph-specs report` subcommand, whose own emitter owns the `"record"` discriminator (see §Report records). Three discriminator keys, three disjoint meanings.

Records are emitted **after** all violations, `pending` before `realized`, each list sorted by concept name then by spec site.

**Marker records never affect the exit code.** A tree whose only findings are markers exits `0` — under either marker value.

**Schema evolution.** RFC-015's two values are **additive**: no `schema_version` bump, on the same ground as RFC-013's marker suppression and RFC-014's polarity narrowings — they change which headings qualify, not what the `marker` discriminator means. One thing that precedent does not cover is that RFC-015 also exempts *edges*, not only headings; the same class one rung down, ruled additive on that basis rather than by silent extension.

### `pending`

A marked concept heading with no backing code item. Emitted **instead of** `missing_in_code`.

```json
{"schema_version":"5","marker":"pending","concept":"Digest","source":{"kind":"spec","path":"specs/concepts/execution.md","line":41,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `marker` | string | always `"pending"` |
| `concept` | string | the marked concept name |
| `source` | source object (kind=spec) | location of the marked heading |

**Remediation:** none required — this is the transcription worklist. Write the code, and the record becomes `realized`.

### `realized`

A marked concept heading whose backing code item exists — by name match or by `- impl:` anchor resolution. Emitted **in addition to** the fully enforced equivalence checks for that pair: a marker never parks a divergence.

```json
{"schema_version":"5","marker":"realized","concept":"InboundAcl","source":{"kind":"spec","path":"specs/concepts/fleet_supervision.md","line":120,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `marker` | string | always `"realized"` |
| `concept` | string | the marked concept name |
| `source` | source object (kind=spec) | location of the marked heading |

**Remediation:** ratify the heading — delete its `- status: draft` line. Ratification is a human act; the checker never mutates spec text.

### `retirement_incomplete`

A `- status: retired` heading whose backing code item is still present (RFC-015 §3.2 row 7). The retirement was announced and the code has not gone yet. Emitted **in addition to** the fully enforced equivalence checks for that pair, exactly as `realized` is: a marker never parks a divergence.

```json
{"schema_version":"5","marker":"retirement_incomplete","concept":"AssertionScope","source":{"kind":"spec","path":"specs/concepts/brief_contract.md","line":56,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `marker` | string | always `"retirement_incomplete"` |
| `concept` | string | the retired concept name |
| `source` | source object (kind=spec) | location of the retired heading |

**Remediation:** delete the code item. This is the one marker record a clean tree carries none of — unlike `pending`, whose remediation writes code, this one's removes it.

### `retirement_complete`

A `- status: retired` heading with no backing code item (RFC-015 §3.2 row 8). The retirement is done. Emitted **instead of** `missing_in_code`, and the heading imposes nothing through its edge bullets, verb anchors or `- impl:` anchors.

```json
{"schema_version":"5","marker":"retirement_complete","concept":"PrePushRebaseDecision","source":{"kind":"spec","path":"specs/concepts/agent_contract.md","line":665,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `marker` | string | always `"retirement_complete"` |
| `concept` | string | the retired concept name |
| `source` | source object (kind=spec) | location of the retired heading |

**Remediation:** none, ever. The `- status: retired` line is never deleted, so this record is permanent and its list never drains — which is why it is emitted and is still not a cleanliness term.

## v3 cohesion variants (RFC-010 — the abstraction ladder)

The `Cohesion` variants check the upward concept→context rung: that a `concepts/` file's `H1` declares a real bounded context, and that each concept is documented under the context the code resolves it to. They are the breaking change that justifies the v2 → v3 bump. Spec-side variants fire with zero code facts; `concept_context_mismatch` is code-fact-gated (needs `specs/contexts/` Owns).

### `context_without_cohesion_unit`

An `H1` context heading with no `H2`/`H3` concept under it — a bounded context that declares no cohesion unit.

```json
{"schema_version":"5","violation":"context_without_cohesion_unit","context":"reading","file":"specs/concepts/reading.md"}
```

| Field | Type | Meaning |
|---|---|---|
| `context` | string | the normalised context identifier from the `H1` |
| `file` | string | the offending `concepts/` file |

### `sub_concept_orphan`

An `H3` sub-concept with no enclosing `H2` concept (a depth skip).

```json
{"schema_version":"5","violation":"sub_concept_orphan","sub_concept":"InnerThing","file":"specs/concepts/reading.md"}
```

| Field | Type | Meaning |
|---|---|---|
| `sub_concept` | string | the orphaned `H3` heading text |
| `file` | string | the offending `concepts/` file |

### `concept_context_mismatch`

A concept's spec-side declared owning context (its `concepts/` H1, with `specs/contexts/` export precedence) disagrees with the context the code resolves it to (the `specs/contexts/` Owns block owning the crate the `pub` type lives in).

```json
{"schema_version":"5","violation":"concept_context_mismatch","concept":"Widget","declared":"reading","code_context":"equivalence","spec_source":{"kind":"spec","path":"specs/concepts/reading.md","line":7,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `concept` | string | the concept name |
| `declared` | string | the spec-side declared owning context |
| `code_context` | string | the context the code resolves the concept to |
| `spec_source` | source object (kind=spec) | where the concept is documented |

> **Shipped (#136):** code-kind source objects carry the agnostic provenance triple (`module_path` / `unit` / `context`) when resolved — see §Source location object. Optional additive fields (see §Schema evolution); they did NOT bump `schema_version`.

## v0.7 anchor variant (RFC-012 — non-`pub` spec anchors)

### `dangling_anchor` (additive at v0.7; rides the current `schema_version`)

A concept's `- impl: <qname>` anchor (RFC-012 §3.2) names a code item that does not exist anywhere in the code tree, at any visibility. The equivalence-defect analog of `missing_in_code` for an anchored concept: the anchor redirected the concept's target to `qname`, and `qname` did not resolve. A **top-level** variant (not nested under `Cohesion`) so a consumer that opts out of cohesion checking cannot silently suppress broken-anchor detection. Added as an **additive** variant — it did not bump the version (see §Schema evolution).

```json
{"schema_version":"5","violation":"dangling_anchor","concept":"ValidateIntakeFull","target":"validate_intake","source":{"kind":"spec","path":"specs/concepts/intake_validation.md","line":3,"format":"markdown"}}
```

| Field | Type | Meaning |
|---|---|---|
| `concept` | string | the anchored concept heading |
| `target` | string | the `- impl:` qname that did not resolve |
| `source` | source object (kind=spec) | where the anchor bullet is declared |

> An anchor whose target **does** resolve emits **no** record — the concept is satisfied (no `missing_in_code`). Resolution honours any visibility (`pub`, `pub(crate)`, a `fn`, a `const`), so a concept whose canonical implementation is `pub(crate)` need not manufacture a `pub` type.

## v0.8 polarity variant (RFC-014 — grounding polarity)

### `forbidden_concept_reintroduced` (additive; rides the current `schema_version`)

A `pub` code item bearing a name its spec heading **expelled** — the heading carries a grounding comment with `polarity:forbidden` (see `specs/dialect.md` §Grounding polarity). Distinct from every other variant in direction: this is not a gap between spec and code, it is code doing something the spec forbids.

Both sites are carried, so the record names what expelled the name and what reintroduced it.

```json
{"schema_version":"5","violation":"forbidden_concept_reintroduced","concept":"Member","spec_source":{"kind":"spec","path":"specs/concepts/topology.md","line":41,"format":"markdown"},"code_source":{"kind":"code","path":"src/model.rs","line":12,"language":"rust"}}
```

| Field | Type | Meaning |
|---|---|---|
| `concept` | string | the expelled name |
| `spec_source` | source object (kind=spec) | the heading that expels it |
| `code_source` | source object (kind=code) | the item that reintroduced it |

**Remediation:** remove the code item. There is no allowlist and no suppression file — deleting the item clears the finding, re-adding it brings the finding back.

> Behavioural parity with `cascade::Finding::ForbiddenConceptRealized`, under a locally-disambiguated name: `Realized` already means the opposite thing in this bounded context (RFC-013's marker record — "the pending concept landed"). Parity is on inputs and outputs, not on identifier spelling.

**Additive** — no `schema_version` bump. The two narrowings of `missing_in_code` that `forbidden` and `illustrative` introduce emit no record at all; like the marker suppression before them they change which headings qualify, not what the discriminator means.

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

Breaking changes — **`schema_version` increments** (e.g., `"1"` → `"2"` → `"3"` → `"4"`):
- Removing a field
- Renaming a field or a `violation` discriminator value
- **Removing a `violation` discriminator value entirely** (RFC-013 §3.5 — a discriminator that silently stops appearing is a worse failure mode for a pattern-matching consumer than a hard version bump; this closed a pre-existing gap in the taxonomy, which covered renaming but not removal)
- Changing a field's JSON type
- Changing the interpretation of an existing `violation` discriminator

Adding a **new top-level discriminator key** (as v4 did with `marker`) is on its own additive — it was the retirement of `implements_draft_concept` in the same release that forced the v3 → v4 bump.

Version history:
- `"1"` — v0.1–v0.3 (concept / signature / edge variants).
- `"2"` — v0.4 added the bounded-context variants (`context_membership_unknown`, `cross_context_edge_*`, `cross_verb_unauthorized`).
- `"4"` — v0.8 added `cross_edge_off_surface`, `surface_admits_nothing`, `edge_unanswerable` and `malformed_anchor_bullet` additively (no bump; the version is listed here because the variant arrived during `"4"`).
- `"3"` — RFC-010 added the abstraction-ladder `Cohesion` variants (`context_without_cohesion_unit`, `sub_concept_orphan`, `concept_context_mismatch`). Consumers dispatch on `"3"`; the qbot-core `compare-spec-change` lockstep arm is tracked at #135. Like `ContextViolation`, `CohesionViolation` is `#[non_exhaustive]` — an unknown future cohesion variant emits `"violation":"unknown_cohesion_violation"` as a tripwire.

## Determinism

Record order reflects the order `domain::diff()` returns its outcome, which is deterministic for a fixed input tree: all violations first, then `pending`, then `realized`, then `retirement_incomplete`, then `retirement_complete`. Within each marker list, records are sorted by concept name and then by spec site. Consumers SHOULD NOT rely on a particular order across tool versions.

## Relationship to `--format=text`

The two formats emit the same **set** of records — violations and markers alike; they differ only in wire form. Exit codes are identical, and in both cases a function of violations alone. Text additionally prints a summary line (`N violations, M pending, K realized-unratified, R retirement-incomplete, C retirement-complete`) that has no NDJSON counterpart: a counting consumer counts records. When both are needed, run the tool twice; the cost is linear in the input tree.
