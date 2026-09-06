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
(`# AC verifier` → `ac-verifier`). **The H1 is prose** (keel-dialect
§2.1): its name is never bound to code, and it costs its file nothing.
An H1 that carries punctuation — a descriptive title like `# Spec: foo` —
still normalises, still names the file's context for the contexts
cross-check, and the file is neither dropped from the ladder walk nor
failed.

**The ladder and the concept graph are one read.** Both are projections
of a single `cascade::parse_spec` call per file: the `##`/`###` rungs are
that reader's own site list, name for name and line for line, so a
heading can never be a cohesion unit for the ladder and absent from the
graph. The H1 and H4 rungs, which the reader keeps internally and does
not publish at `cascade.rev`, are located beside it under the reader's
own rule — ATX runs at column zero, outside code fences — and they
participate in cohesion checking without ever becoming concept-graph
nodes (see [What the markdown reader ignores](#what-the-markdown-reader-ignores)).
That residue closes when the reader publishes the whole ladder; until
then no other pass in this repo recognises a heading.

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

**Draft files participate** (RFC-013). Spec-state markers relax a
*concept's* code-existence obligation; they never relax this doc-level
structural check. The `cohesion: behavioral` exemption is the only one,
and it applies to a draft doc on exactly the same terms as any other —
substance still required.

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
  nodes**, but they are ladder rungs (H1 = context, H4 = member) for
  cohesion checking (see [Abstraction ladder](#abstraction-ladder))
- **Everything under a `####` rung.** A callout rung's content is
  gate-invisible (keel-dialect §2.1, §5): a concept's extent runs from
  its heading to the next rung of *any* depth, so a reserved bullet or a
  `rust` block under `#### Distinct from` or `#### Keywords` attaches to
  no concept and yields no edge, anchor or signature. The enclosing
  concept resumes at the next `##`/`###`
- Fenced blocks without a recognised language tag (untagged or `txt` or
  similar)
- Bullets without a recognised prefix
- Ordered lists
- Tables, images, links, raw HTML blocks, and HTML comments — with one
  scoped exception, the grounding comment's `polarity:` key (below)
- Files outside the directory passed to `--specs`
- Any file whose extension is not `.md`

Draft files are **not** on that list: since RFC-013 they are parsed like
any other spec (see [Spec-state markers](#spec-state-markers-rfc-013)).

## Spec-state markers (RFC-013)

A concept heading may be declared **ahead of its code**. The checker
reads that state from the spec text — never from file location — via
one marker with two scopes.

### File scope — `status: draft` front-matter

A spec file may open with a YAML front-matter block, delimited by lines
containing only `---`, that declares its lifecycle status:

```
---
status: draft
---

## SomeConcept
```

Whole-file front matter is a **retired authoring form** (keel-dialect
§4); it is read only until the per-heading sweep converts it, marker for
marker. Its admitted values and its refusals are the one reader's, and
every concept heading the reader places is marked from there. A heading
the reader leaves unplaced takes it on the same terms, from the read
beside the reader described under
[Grounding polarity](#grounding-polarity-rfc-014). While it is read: the file is
parsed, not skipped. Only the leading front-matter is consulted, and
only its `status:` key.

The admitted values are exactly two bare words, `draft` and `live`.
Anything else **refuses the file** — `"draft"`, `'draft' # pre-authored`
and `Draft` are each an unknown status, not an unmarked file: quotes are
not stripped, a trailing `#` comment is not a comment here, and the
comparison is byte-exact. A second `status:` key refuses; a front-matter
block that opens `---` and never closes refuses. A block that closes
before any `status:` line, a `status:` line in the prose body, and a file
with no front-matter at all are each simply not draft.

A per-heading bullet inside such a file is the authoring form and
**wins**: a heading marked `retired` inside a `status: draft` file reads
retired.

The file-scope marker narrows **the obligation on code and nothing else**
(keel-dialect §4). It never gates another channel: the invariant
annotations of a draft file are reported like any other file's.

### Heading scope — the `- status: draft` bullet

A bullet reading `- status: draft` marks **exactly one** heading when it
is the **first non-blank content line** below an `H2` or `H3` concept
heading:

```
## SomeConcept

- status: draft
```

Four properties, each load-bearing:

- **Two legal values, and no transition between them.** `draft` declares
  code owed to *exist*; ratification is *deletion of the line*. `retired`
  declares code owed to be *gone*; it is written while the backing item
  is still present, and it is **never** deleted. There is no
  `- status: ratified`, and neither value rewrites into the other — still
  a presence flag per value, never a state machine, because the progress
  axis is the code. The value is compared **case-exact**: `- status:
  Draft` is not the marker. Any other `- status:` bullet is an
  unrecognised prefix under the ordinary dialect rule and stays inert
  text.
- **No subtree inheritance.** A marker binds only to the heading whose
  block it opens; a marked `H2` does not mark its `H3`s. The reader
  models `H2` and `H3` as flat peers, and inheritance would make
  ratification non-local.
- **Mis-placement fails loud, not silent.** A marker bullet that is not
  the first non-blank content line is inert; the heading reads
  *unmarked*, and `missing in code` fires if its code is absent. The
  failure mode of a malformed marker is a visible violation, never a
  silent suppression.
- **Trailing text is tolerated.** Anything after the value on the same
  line — e.g. the upstream authoring convention
  `- status: draft (per <RFC>.md §<clause>)` — is ignored. That
  parenthetical is enforced by the authoring tree's own fences; the gate
  never parses it.

A marker bullet under an `H1`, in a `specs/contexts/` file, or outside
any concept block is inert — the contexts dialect is untouched.

### What a marker changes

| Heading | Backing code item | Result |
|---|---|---|
| unmarked | absent | `missing in code` |
| unmarked | present | pass |
| `draft` | absent | a `pending` record — **not** a violation |
| `draft` | present | full equivalence enforced, plus a `realized` record |
| `retired` | present | full equivalence enforced, plus a `retirement incomplete` record — **not** a violation |
| `retired` | absent | a `retirement complete` record — **not** a violation |

The marker value picks the pair; the backing item picks the member. No
record kind affects the exit code, under either value. See
`specs/ndjson-output.md` §Marker records for the wire shape.

The two `retired` rows do not mean symmetric things. Row 7 is the window
every correct retirement opens — announced, not yet done — and a clean
tree carries none of them. Row 8 is terminal: the marker line is never
deleted, so that list never drains, which is why it is rendered and is
still not a cleanliness term. A never-draining count inside the clean
state would put the clean state out of reach.

Marking is **concept-scoped**. It never suppresses the doc-level
[cohesion invariant](#abstraction-ladder): a doc that declares no concept
heading at all still reports `context_without_cohesion_unit`, marked or
not — and a marked heading *counts* as its context's cohesion unit.

"Backing item present" has two spellings, and they are one fact: a
name-matched `pub` item, or a resolved `- impl:` anchor. A marked
heading whose anchor does not resolve is `pending`, and its
`dangling anchor` violation is suppressed — an unresolved target *is*
the declared-ahead-of-code state the marker announces.

A pending concept, and a row-8 one, impose nothing through their own
declarations. That is `unobliged`, and it is **stated once** under [what
a heading obliges](#what-a-heading-obliges-and-what-it-describes-rfc-015)
— cited here, never restated, because a rule written out under one
record kind reads as scoped to it.

A marker never parks a divergence: once the concept is realized, drift
under it fires the ordinary violation exactly as it would under an
unmarked heading. Escalation happens on contradiction only — never by
age, count, or branch.

## Grounding polarity (RFC-014)

A **grounding comment** is an HTML comment carried under a concept
heading by an upstream tool (cascade / Bosun):

```
## Member
<!-- parent:spec:Unit polarity:forbidden -->
```

**graph-specs parses none of this itself.** The dialect grammar has one
realization — the cascade reader, pinned at `cascade.rev` and linked as a
crate dependency (keel-dialect §12.1). The markdown adapter consumes that
reader for every §2–§6 construct it publishes: the grounding declaration
and its closed key set, the state marker, the front-matter status, the
`##`/`###` rungs of the ladder and their names. One call per file feeds
the concept graph, the abstraction ladder and the invariant-annotation
channel alike, so no two passes here can disagree about what a heading
is. A second parser of any of it is deleted on sight.

**Two constructs the reader does not publish at `cascade.rev` are read
beside it**, in that one module, and nowhere else in this repo. Both are
measured gaps in the one reader, not choices, and both are deleted the
day the reader publishes them.

- The `#` context rung and the `####` callout rung. The reader keeps
  them internally: its published site list carries `##`/`###` only, and
  its concept list carries a `####` rung in an ungrounded document and
  not in a grounded one. They are located under the reader's own rule —
  ATX runs at column zero, outside code fences — and they open no
  concept; see [Abstraction ladder](#abstraction-ladder).
- The state marker of a `##`/`###` the reader sites but leaves
  **unplaced**. The reader publishes a marker only on a concept, and it
  opens no concept for a `##` with no enclosing rung above it — in a
  file with no `#` context rung, every `##` is such a heading. Dropping
  the marker there would void a well-formed §4 declaration and turn a
  suspended obligation into a red gate, which keel-dialect §1 and §3.2
  forbid, so the marker of an unplaced heading is read here, on the
  reader's own terms: the first non-blank line under the heading, the
  bare bullet `- status: draft` or `- status: retired`, value
  case-exact, the retired bullet outranking a whole-file `status:
  draft`. A placed heading never reaches this path — its marker is the
  reader's.

The key set is **closed** — `parent`, `anchor`, `keywords`,
`reached_for`, `polarity` (keel-dialect §3.2). An unrecognised key is
**malformed**: the read refuses, never skips and never voids the
declaration silently. `parent` is required; graph-specs consumes
`polarity:` and lets the reader own ancestorship, but it no longer
ignores the keys it does not consume — a declaration the reader calls
malformed refuses the file.

**What refuses, and what does not.** The reader emits more finding
classes than keel-dialect §7 gives verdict rows, and this repo maps only
the rows. A refusal is raised for a class §7 maps to **malformed** at a
`##`/`###` heading or at the document — a malformed grounding
declaration, an orphan comment, a malformed document declaration, a
malformed front-matter status — and for the **run-level** row: an
unclosed fence stops the run before any node is judged. Classes §7 gives
no row and no owning concept are **not** this repo's findings and never
abort the run: a vocabulary callout with no enclosing home (vocabulary is
another instrument's channel) and a heading that normalises to an empty
name (its site stands under the reader's own name for it and is diffed
like any other). Nothing is escalated past its row, and no row is
dropped.

**This concept is imported, not defined here.** The values and their
meanings are owned upstream (see [Polarity](concepts/equivalence.md));
this repo authors no `polarity:` markers in its own `specs/`. Reading an
externally-authored wire format under a Conformist contract is a scoped
exception to comment-skipping, not a new local convention — there is no
bullet-form alternative to prefer here, because graph-specs owns neither
the value semantics nor the comment encoding.

### Placement and grammar

The comment sits on the **first non-blank line** below an `H2`/`H3`
concept heading, **below the state marker when one exists** — the two
coexist (keel-dialect §3.1), and a marker no longer costs the heading its
declaration. A comment further down the section, or above its heading,
attaches to no concept: **orphan grounding, malformed**, and the read
refuses.

Three values:

| Value | Meaning |
|---|---|
| `declared` | the ordinary obligation — the concept must exist in code (the default) |
| `forbidden` | the name is expelled — code must **not** bear it |
| `illustrative` | an example — the heading neither compels nor satisfies a code item |

No comment, or a comment carrying no `polarity:` key, reads as
`declared` — the stated default. An **unknown value is malformed** and
the read refuses (keel-dialect §3.2, §11(e)): it is never a silent
default, and never a warning that lets the run continue. The old
fallback-to-`declared` was the register entry (e) this repo owed; a typo
now stops the run rather than quietly arming an obligation nobody wrote.

**Extraction is quote-aware.** Upstream makes `anchor:"…"` mandatory for
every RFC-rooted concept, so a real grounded corpus carries a quoted
freeform value in the *same* comment. A `polarity:` appearing inside that
quoted value is prose, not the key — entirely plausible on an
architecture-methodology corpus, which may carry RFC prose *about*
polarity — and it is not read.

### What each value changes

| polarity | code absent | code present |
|---|---|---|
| `declared` | `missing_in_code` (unchanged) | satisfied (unchanged) |
| `forbidden` | clean | `forbidden_concept_reintroduced` |
| `illustrative` | clean | `missing_in_specs` |

The `illustrative` row is upstream's rule, not an invention: it stops the
marker laundering unspecced public surface past the gate. It is a
**match-attempt gate**, not a post-match dispatch — an illustrative
heading never attempts to bind a code item at all, so the item falls
through to the orphan sweep like any undocumented type.

A non-`declared` heading likewise imposes nothing through its own
declarations — the same `unobliged` rule, [stated
once](#what-a-heading-obliges-and-what-it-describes-rfc-015) and cited
here. Both polarity values are members; so is a marked heading whose item
is absent.

### Precedence over the spec-state marker

`polarity != declared` is evaluated **first, and is terminal**. A marked
heading whose polarity is `forbidden` or `illustrative` emits **no**
marker record:

| | `declared` | `forbidden` | `illustrative` |
|---|---|---|---|
| **unmarked** | rows 1/2 above | table above | table above |
| **marked** | `pending` / `realized` | identical to unmarked — `marked` is inert | identical to unmarked — `marked` is inert |

No cell emits both a marker record and a polarity outcome.

This is principled rather than an arbitrary tiebreak. Marking exists to
relax the code-existence obligation a `declared` heading carries.
`forbidden` and `illustrative` carry no such obligation — absence is clean
by definition for both — so there is nothing for marking to relax. It is
not out-competed by polarity; it is *structurally inert*. Emitting
`realized — ratify` on an expelled name would also be an actively wrong
instruction: a reader would see "close this out" and "actively banned"
on the same heading.

## What a heading obliges, and what it describes (RFC-015)

Its own section deliberately, under neither the marker section nor the
polarity section. Placed under either, one rule drawing on two sources
would read as that axis's rule which the other happens to obey — and
both axes contribute members.

Three named predicates. Each is stated **here** and cited, never
restated, by every carrier.

**`unobliged`** — this heading compels no code item to exist. Members: a
heading marked with either value whose item is absent, `forbidden`, and
`illustrative`. It governs the **source side**: an `unobliged` heading
imposes no code-existence demand through its own declarations — its edge
bullets, its `- verb:` anchors, and its `- impl:` anchors alike, the last
including `dangling anchor`.

**`unpointable`** — this heading offers no legitimate code item to point
at, and its own declared state accounts for that. Members: marked with
either value + absent; `illustrative` + absent; `forbidden` + absent;
`forbidden` + present. It governs the **target side**: no heading bears a
code-existence demand made of it by *another* heading's declarations.

**`unbound`** — this heading describes no code item. Member:
`illustrative`, alone. It governs every check presupposing that the
heading describes that item. **Known under-enforced** — the cohesion pass
still fires `concept context mismatch` on non-`declared` headings.

### Why three names and not one rule with caveats

The member sets nest — `unpointable` ⊂ `unobliged` — while the predicates
do not, and each containment has a witness:

- **`forbidden`** is `unobliged` **and bound**. Hanging the binding
  predicate off "compels no code item" bans
  `forbidden concept reintroduced`, the finding the polarity axis exists
  to produce.
- **`illustrative` with its item present** is `unobliged` **and
  pointable**. The item is a legitimate edge target: adding the field
  clears the finding and introduces nothing, so keying the target side on
  `unobliged` would park a real divergence.

Set inclusion does not license clause subordination. A subordinate clause
quantifies over its main clause's subject, so hanging either predicate
off "compels no code item" asserts it of the whole `unobliged` extension
rather than of the subset — which is why the names are separate and the
subordinate form is not used here.

**The accounting clause in `unpointable` is load-bearing and is not a
restatement of the member list.** An unmarked `declared` heading whose
item is absent is *not* a member: that is the first row of the marker
table, where nothing accounts for the absence, because the absence *is*
the finding. Keying on absence alone would silently move that row.

### The predicates are per-heading; the key is per-name

Two headings may share a name across files, and the edge pass keys on an
edge's target, which is a name rather than a heading. The conversion is
**conservative for `unpointable`: a name is unpointable only if every
heading carrying it is.** A heading in one context illustrating a type
really declared in another is the canonical use of `illustrative`, and a
permissive key would suppress an edge into that name while the declared
heading's item sits there satisfiable.

### One direction only

`edge missing in spec` is untouched by the exemption, on either endpoint
and under every marker value and polarity. Code may not carry a
relationship the specs do not declare.

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
- Any directory carrying a `CACHEDIR.TAG` file that opens with the canonical
  Cache Directory Tagging Specification signature. Cargo writes this marker
  into every build tree it creates, so a workspace built with a
  `--target-dir` other than `target` keeps its generated bindings off the
  concept surface. The name list above cannot see such a tree; this rule
  tests what a directory declares itself to be rather than what it is
  called. A directory whose tag is absent, unreadable, or differently
  signed is ordinary source and is walked.
- Any file whose extension is not `.rs`

## Multi-language fenced blocks (RFC-004)

Fenced code blocks inside a concept's section carry signature-level spec
content. The fence language tag dispatches the block to the
language-specific normalizer:

| Fence tag | Adapter | Normalizer |
|---|---|---|
| ````rust```` | adapter-rust | `adapter-rust::normalize` (v0.2+) |
| ````php```` | adapter-php (RFC-005) | `adapter-php::normalize` |
| ````ts```` | adapter-typescript (RFC-006) | `adapter-typescript::normalize` |
| other | ignored | — |

A spec concept may carry fenced blocks in multiple languages
simultaneously. Each block is matched independently against the
corresponding language's structural code graph. Drift between blocks for
different languages is NOT a violation — that is intentional
cross-language spec content, not drift.

A ```php fence is parsed by the one PHP syntax model the ecosystem pins (tree-sitter-php, graph-specs-011-php-ladder#3.3) and normalized by `adapter-php::normalize` — the declaration's tokens re-printed with single spaces, comments, attributes and body dropped, a byte-equal target like the Rust normalizer's; the markdown reader reaches it through a normalizer port supplied at the composition root, never by depending on the adapter crate (graph-specs-004-multi-language-adapter-contract#3.6, amendment of 2026-09-06). A `php` fence that does not parse, or a section carrying more than one, is `Unparseable`, the fence tag naming the language.

The markdown reader does not change for this section: no PHP or
TypeScript adapter exists yet to consume a PHP or TypeScript fence. The
dialect declares the contract ahead of the adapters so that RFC-005 and
RFC-006 land into a known shape.

## Meta note

This dialect spec is itself written in the dialect it describes: `##` and
`###` headings name the subsections, but because this file lives at
`specs/dialect.md` (not under `specs/concepts/`), those headings are not
parsed as concept declarations. The separation is enforced operationally
by the CLI flag, not structurally by the markdown.
