# Spec dialect

This document describes the markdown dialect `graph-specs-rust` parses.
It is a meta-spec: it describes the format of spec files, not any concept
of the tool itself. The tool's CLI `--specs` flag is pointed at
`specs/concepts/` (not `specs/`), so this file's own headings are never
walked as concepts.

## Registry boundary

`specs/` is the **spec registry** — the directory tree that holds all
authoritative specifications for this project. `docs/` holds non-spec
content (roadmap, operational notes, rationale) and is never walked by
the tool. Moving a document between `specs/` and `docs/` is a meaningful
action: it brings the document under (or removes it from) the gate.

Within `specs/`, the concept-declaration subdirectory is `specs/concepts/`.
The tool's `--specs` flag should point at that subdirectory when running
the dogfood check. Other files under `specs/` (like this one) are
meta-specs that describe the system; they are not walked by the CLI and
their headings do not become concept nodes.

## What the markdown reader parses

Only **structural** elements contribute to the concept graph.

- Level-2 and level-3 headings (`##` and `###`) — the heading text
  becomes a concept node. Heading text is normalised: inline backticks
  are stripped (CommonMark's plain-text rendering), leading/trailing
  whitespace is trimmed, and generic parameters are removed
  (`## Graph<T>` records the concept as `Graph`).
- Fenced `rust` code blocks inside a concept's section — reserved for
  signature-level extraction in a later issue. Currently parsed but not
  diffed.
- Bullets with recognised prefixes (`- implements: X`, `- depends on: X`,
  `- returns: X`, `- verb: <qname>`) — the first three are
  relationship-level anchors; `- verb:` is a function-ownership anchor
  handled by a separate parser path (see [Verb bullets](#verb-bullets)
  below). **Note:** `verb:` is NOT in `BULLET_PREFIXES` code-side; the
  parser dispatches it via a dedicated handler.

### Verb bullets

A `- verb: <qname>` bullet inside a concept section anchors a public
function to that concept's owning bounded context (v0.5).

**Bullet shape:**

```
- verb: <qname>
```

**Qname forms (v0.6):** two forms are accepted, syntactically disjoint:

- **Bare identifier** — `^[A-Za-z_][A-Za-z0-9_]*$`. Matches a top-level
  `pub fn` declaration at the root of a `.rs` file (e.g. `- verb: diff`
  anchors `pub fn diff(...)`). No auto-fallback to impl methods.
- **`Type::method`** — `^[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*$`.
  Matches a public method inside an impl block: either an inherent impl
  (`impl Foo { pub fn bar }`) or a trait impl (`impl Trait for Foo { fn bar }`).
  Trait-impl methods are considered public even without an explicit `pub`
  keyword. The `::` distinguishes this form from a bare identifier —
  no auto-fallback to top-level fns applies.

Multi-segment paths (`a::b::c`), leading `::`, trailing `::`, and
non-identifier characters are rejected with a `tracing::warn!` log
(tolerant-skip).

**Opt-in semantics:** a concept section with no `- verb:` bullets is
never inspected by the verb pass. The pass activates only when at least
one concept in a bounded context carries a verb anchor. Both
concept-level specs (`specs/concepts/`) and context-level specs
(`specs/contexts/`) may carry verb bullets.

**MissingInSpec activation:** unanchored `Type::method` decls are inspected only when concept `## Type` exists in the decl's bounded context AND carries at least one `- verb:` anchor (per-concept, context-scoped). Unanchored top-level free `pub fn`s are inspected when their bounded context has any opt-in concept (per-context).

## Abstraction ladder

Heading **depth** is load-bearing (RFC-010). A heading's role *is* its
depth, and the tool enforces the mapping rather than inferring intent — a
`pub` type documented at the wrong depth is drift the gate surfaces. The
four rungs (`domain::AbstractionLevel`):

| Depth | Rung | Meaning | Diffed? |
|---|---|---|---|
| `#` H1 | **Context** | the file's bounded-context identifier | cohesion (below) |
| `##` H2 | **Concept** | one `pub` type — the concept-graph unit | yes (concept / signature / edge) |
| `###` H3 | **SubConcept** | a nested `pub` type | yes (as a concept) |
| `####`+ H4 | **Member** | a field / variant / param / invariant | emitted, not diffed |

**H1 normalisation.** A concept file's single `# H1` is its bounded-context
identifier. One rule normalises it — lowercase, with internal whitespace
runs collapsed to a single `-` — and the **same** rule is applied to the
`specs/contexts/<name>.md` H1, so both sides resolve to one identifier
(`# AC verifier` → `ac-verifier`). An H1 that does not normalise to an
identifier (it carries punctuation, e.g. a descriptive title like
`# Spec: foo`) declares **no** bounded context: the ladder pass skips that
file rather than failing (companion-dialect robustness).

**The ladder is a separate pass.** It is assembled by a dedicated
`TreeAssembler` walk, *not* by the flat concept reader described above — so
the H1/H4 rungs participate in cohesion checking even though they never
become concept-graph nodes (see [What the markdown reader ignores](#what-the-markdown-reader-ignores)).

**Cohesion invariant (level 5).** The ladder must be coherent:

- an H1 context with **no** H2/H3 concept under it declares no cohesion
  unit → `context_without_cohesion_unit`;
- an H3 sub-concept with **no** enclosing H2 is a depth skip →
  `sub_concept_orphan`;
- a concept documented under one context whose code the `specs/contexts/`
  Owns block resolves to a *different* context →
  `concept_context_mismatch` (code-fact-gated — needs `specs/contexts/`).

These are the upward concept→context rung, complementing the downward
concept→method rung that `- verb:` anchors check.

## Anchors (RFC-012 — non-`pub` spec anchors)

The default rule is that a `## Concept` heading is backed by a top-level
`pub` type and a `# Context` H1 eventually owns one. Two **opt-in** markers
relax that for legitimate shapes — without weakening it: each names real
code the tool still resolves, so the heading↔surface link stays two-way and
zero-baseline (delete the backing and the gate re-arms). Neither is a
suppression or an allowlist.

### `- impl:` concept anchor

A concept whose canonical implementation is legitimately **not** a
top-level `pub` type (a `pub(crate)` type, a `fn`, or a `const`) carries a
single anchor bullet that redirects its equivalence target to a named code
item:

```
## ValidateIntakeFull
- impl: validate_intake
```

The `<qname>` uses the **same** grammar as `- verb:` (a bare identifier or
`Type::method` — one shared grammar, no second parser). The anchored
concept is satisfied when the item resolves at **any** visibility — so no
caller-less `pub` ZST need be manufactured. If the item does not resolve,
`dangling_anchor` fires (a top-level violation, not nested under
cohesion). `impl:` does not collide with the `implements:` edge bullet.
Resolution is consulted only for the anchor qnames, so the global concept
set is unchanged.

### `cohesion: behavioral` front-matter

A behavioral / doctrine context that owns **no** `pub` type by design (it
is realized as `const` + `fn` + enum variants + fences) declares so in its
leading front-matter — a sibling to `status: draft`:

```
---
cohesion: behavioral
---

# secrets
```

This satisfies `context_without_cohesion_unit` for that file — **but only**
when the context also carries machine-checkable behavioral substance: at
least one `- impl:` / `- verb:` anchor or one `[enforced-by:]` /
`[prose-only:]` invariant annotation. A behavioral marker over an empty
file is **not** a free pass; it stays a violation. The marker never
exempts `sub_concept_orphan` (a depth skip is always a defect), and a
type-free context **without** the marker still fires (default-deny). Unlike
a draft, a behavioral file is parsed normally.

## What the markdown reader ignores

Prose changes never affect the graph. The reader does not see:

- Paragraphs, blockquotes, emphasis, strong, strikethrough
- Level-1 and level-4+ headings — these never become **concept-graph
  nodes**, but they are read by the separate ladder pass (H1 = context,
  H4 = member) for cohesion checking (see [Abstraction ladder](#abstraction-ladder))
- Fenced blocks without a recognised language tag (untagged or `txt` or
  similar)
- Bullets without a recognised prefix
- Ordered lists
- Tables, images, links, raw HTML blocks, HTML comments
- Files outside the directory passed to `--specs`
- Any file whose extension is not `.md`
- Every spec file declaring `status: draft` (see ## Draft specs)

## Draft specs

A spec file may open with a YAML front-matter block — lines delimited by
`---` — that declares its lifecycle status:

```
---
status: draft
---

## SomeConcept
```

When the leading front-matter declares `status: draft`, the markdown
reader **skips the whole file** for the equivalence graph: it contributes
no concepts, no verb anchors, and no invariant annotations. A draft spec
therefore imposes no code-existence obligation — no `missing in code`
violation can arise from it.

Its headings are still read into a separate **draft-concept index** that
carries no obligation. This index is used solely so that a code item
whose name matches a draft heading reports the targeted
`implements draft spec` diagnostic (`Violation::ImplementsDraftConcept`)
instead of the generic `missing in specs` orphan. The distinction
matters: `MissingInSpecs` signals an undocumented item, while
`ImplementsDraftConcept` signals an item racing ahead of a ratified spec.

This exists so a canonical concept can be **authored ahead of its
code** — ratified by review, committed to `specs/concepts/`, and filled
in by later PRs — without blocking CI in the interim. Removing or
changing the `status:` line re-arms the file: from that commit on, every
concept it declares must resolve against code like any other spec.

Only the leading front-matter is consulted. The value matches
case-insensitively, with or without surrounding quotes, and a trailing
`#` comment is ignored. A front-matter block that closes before any
`status:` line, a `status:` line in the prose body, or a file with no
front-matter at all, is not draft.

## What the Rust reader parses

Only **top-level public declarations** contribute to the concept graph.

- `pub struct`, `pub enum`, `pub trait`, `pub type` at the root of each
  `*.rs` file. The identifier is the concept name. The file path and
  start line of the identifier are the source location.
- **Impl-method extraction (v0.6, verb-anchoring only):** public methods
  in impl blocks are extracted as `Type::method` qnames for verb-anchor
  matching. This walk is separate from concept extraction and only feeds
  the verb-anchoring pass (`VerbReader::extract_pub_fns`). Both inherent
  impls (`impl Foo { pub fn bar }`) and trait impls
  (`impl Trait for Foo { fn bar }`) are covered.

## What the Rust reader ignores

The code-side filter rules are:

- Non-`pub` items — for the **concept walk**. A non-`pub` item is still
  *resolvable by an explicit `- impl:` anchor* (RFC-012 §3.4): the anchor
  resolver indexes items at any visibility, but only the qnames an anchor
  references are consulted, so the concept set itself is unchanged.
- Items gated by `#[cfg(test)]` or `#[cfg(feature = "…test…")]`
- Declarations nested inside `pub mod foo { … }` (top-level only)
- `impl` blocks (except for verb-anchoring purposes — see ## What the Rust reader parses), `fn`, `const`, `static`, `use`, `macro_rules!`, `mod`
- Per-crate `tests/`, `benches/`, `examples/` directories
- `target/`, `.git/`, `.claude/`, `.proofs/`, `node_modules/` directories
- Any file whose extension is not `.rs`

## Meta note

This dialect spec is itself written in the dialect it describes: `##` and
`###` headings name the subsections, but because this file lives at
`specs/dialect.md` (not under `specs/concepts/`), those headings are not
parsed as concept declarations. The separation is enforced operationally
by the CLI flag, not structurally by the markdown.
