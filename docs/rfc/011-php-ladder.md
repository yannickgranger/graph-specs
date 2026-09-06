# RFC graph-specs-011-php-ladder — the PHP surface, its constructs and its declaration

**Status: council synthesis 2026-09-05. RATIFIED on merge to doxa `develop` by the operator.**
Amendments follow the same path: council synthesis, operator merge.

The identifier `graph-specs-011` is not chosen here. It is claimed:
`graph-specs-012-non-pub-spec-anchor` records under **Numbering** that
"RFC-011 is verbally reserved by RFC-010 (§2/§11.5) for *the PHP
ladder*; this RFC takes **012** to respect that reservation." This
document occupies the reservation and no other number is admissible for
this subject.

Citations below are in the full identifier-and-clause form throughout,
because `keel-dialect#6.5` rules that "A bare number (`RFC-005`) is not
an id and resolves to nothing" and admits no alias into the index or
into any reader. Where the harness or the dialect is named, a bare
section is used, because neither is ever an ancestor of anything here.

## §1 Problem

`graph-specs-004-multi-language-adapter-contract` ratified the
multi-language framework and shipped no adapter. It deferred the PHP
work and, in deferring it, named the successor by a bare number. Two
questions were assigned to that successor and are open to this day:
what counts as a concept-bearing PHP construct, and what stands in
`pub`'s place when the language has no module-level visibility.

Both questions have become sharper since 2026-04-21, because the code
side of the tool changed underneath them.
`graph-specs-010-abstraction-level-equivalence#1` rules that "'graph-specs
↔ code' does not oblige graph-specs to *parse* code; it obliges
graph-specs to *know the code's facts*, which are cfdb's product," and
its §3.3 ships a `CodeFacts` port with a cfdb-query Anti-Corruption
Layer beside the source walk. Its §11.5 addresses PHP directly: PHP is
"*not* 'nearly free' — PHP `:Item` is prop-less (edge-only
containment), so the ACL needs a PHP-specific **edge-traversal** path.
The ladder logic is reused; the fact-extraction path differs."

The consequence is that the PHP code side is not a parser this tool
owns. That half of the question is answered by an ancestor, and this
document transcribes the answer rather than re-deciding it. What
remains genuinely open, and what this document rules, is the surface:
which PHP facts bind a concept heading, and how a repository declares
which of them are on the surface at all.

## §2 Scope

This document rules three things and nothing else.

The concept-bearing PHP construct (§3.1). The declaration that replaces
`pub` (§3.2). The one PHP reader that still reads PHP source, and its
parser backend (§3.3).

Everything the ancestors already rule is cited, never restated as a
decision of this document. The framework —
`CodeLanguage`, `SpecFormat`, the reader set, the routing table, the
wire schema — is
`graph-specs-004-multi-language-adapter-contract`'s and stands as
written. The ladder and the `CodeFacts` port are
`graph-specs-010-abstraction-level-equivalence`'s. The anchor primitive
and the surface principle are
`graph-specs-012-non-pub-spec-anchor`'s. The PHP fact shape is cfdb's,
ruled in `cfdb-041-language-backend-trait` and
`cfdb-045-polyglot-relationship-edges`.

Out of this document's scope, each because an ancestor holds it:
cross-language bounded contexts and the build-system disambiguation of
an owned unit, held by
`graph-specs-004-multi-language-adapter-contract#2` for RFC-007;
inheritance transitivity, held by `cfdb-045-polyglot-relationship-edges#3.3`;
external implements-target resolution and constructor calls, held by
`cfdb-045-polyglot-relationship-edges#6`; the TypeScript surface, which
is a different subject and takes a different identifier.

## §3 Design

### §3.1 The concept-bearing PHP construct

A PHP fact binds a concept heading exactly when cfdb's PHP producer
emits it as an `:Item` at the concept rung. Today that is one shape.
`cfdb-045-polyglot-relationship-edges#3.2` records what the producer
actually emits: "PHP maps both `class_declaration` and
`interface_declaration` to `:Item{kind:\"trait\"}` and disambiguates with
`php_construct = node.kind()`." Both constructs bind. The discriminator
between them is the `php_construct` property, which the same clause
makes a documented property of the `:Item` descriptor rather than a new
kind — `cfdb-045-polyglot-relationship-edges#4` invariant 2 is "No new
node label, no new `:Item.kind`," and
`cfdb-041-language-backend-trait#4.5` gates any widening of that closed
set behind its own RFC plus a lockstep pull request here.
Amendment 2026-09-07: the concept rung is three shapes.
`cfdb-045-polyglot-relationship-edges#3.2`, amended the same day,
carries `enum_declaration` as the producer's sixth `php_construct`
value, a PHP enum being `:Item{kind:"enum", php_construct:"enum_declaration"}`
— the producer emits two kinds, `"trait"` for a class or an interface
and `"enum"` for an enum, and the quotation above describes the first
alone. An enum binds a concept heading as a class or an interface does:
it is a top-level named type, the shape the Rust side binds as
`pub enum` (`graph-specs-004-multi-language-adapter-contract#1`), and
`"enum"` is already in `cfdb-041-language-backend-trait#4`'s closed
kind set, so no kind is widened. The reader accepts the value before
the producer emits it, in the order `cfdb-045-polyglot-relationship-edges#3.2`
states.

That both a port and its adapter claim a heading is not a defect of
this ruling and is not new. It is the shape the tool already runs on
itself. `graph-specs-004-multi-language-adapter-contract#1` describes
the Rust code side as emitting "concept nodes from top-level `pub
struct` / `pub enum` / `pub trait` / `pub type`" — a trait and the
struct that implements it are two top-level pub items and bind two
headings. graph-specs' own specification tree does exactly this. A rule
that admitted PHP interfaces and refused PHP classes would make the two
languages disagree about what an abstraction is, and would do it to
avoid a cost the Rust side already pays deliberately.

What answers the volume objection is the ladder, not an exclusion.
`graph-specs-010-abstraction-level-equivalence#3.1` maps heading depth
to a typed abstraction level and §11 aligns the rungs with cfdb's
vocabulary, `H2 Concept ↔ :Item`. A port and its adapter are two
concepts at the concept rung and are written as two headings; where an
author wants one, the sub-concept rung and the chain declaration of
`keel-dialect#3.3` express it.

Method facts do not bind a concept heading. cfdb's PHP producer emits
method `:Item`s — `cfdb-045-polyglot-relationship-edges#2` records that
"PHP already emits method items" — and
`graph-specs-010-abstraction-level-equivalence#11` places members at the
fourth rung, below the concept rung. A method is reached, when it must
be reached, through the anchor bullet of
`graph-specs-012-non-pub-spec-anchor#3.2`, which is the ratified way a
heading binds a code item the concept walk does not surface.

### §3.2 What replaces `pub`

Rust's surface test is a property of each item.
`graph-specs-012-non-pub-spec-anchor#3.4` records where that property
lives on both adapters: "the source-walk `syn` AST visits every item
before the `pub` filter drops non-`pub` ones ...; the cfdb keyspace
carries every `:Item` with a `visibility` prop." PHP affords neither
half. The language has no module-level visibility to read, and
`graph-specs-010-abstraction-level-equivalence#11.5` rules that PHP
`:Item` is prop-less. There is no per-item property to filter on and
none can be invented here.

The temptation is to let the surface be whatever the extractor found —
every class under the autoload root. That is refused, and it is refused
by an ancestor rather than by this document's preference.
`graph-specs-012-non-pub-spec-anchor#3.1` rejects the relaxed filter in
terms that apply unchanged: it "widens the *whole* concept surface
(every `pub(crate)` type in the tree becomes a concept the spec must
now document), inverting the gate from opt-out to mandatory and
breaking the 'public API is the spec surface' principle." A PHP rule
that admits every class in the tree inverts the same gate in the same
way.

**The PHP surface is declared, never derived.** The declaration surface
already exists and is language-agnostic by ratified design: the `Owns`
entries of `specs/contexts/<context>.md`, whose owned unit
`graph-specs-004-multi-language-adapter-contract#2` deliberately kept as
an opaque string — its invariant 6 is "`OwnedUnit` stays
language-agnostic in v0.5. No `build_system` field." For a PHP
repository an owned unit is a namespace prefix, and the surface is
every concept-rung `:Item` whose qualified name begins with a declared
prefix. Nothing outside a declared prefix is on the surface, and
nothing outside it binds or demands a heading.

The namespace prefix is not a choice among several discriminators; it
is the only one the PHP keyspace affords.
`cfdb-045-polyglot-relationship-edges#1` records a real extract of a PHP
fixture as "nodes {Item:5, Module:1, Crate:1} edges {IN_MODULE:5,
IN_CRATE:5}" — there is no `:File` node, so "under this directory" is
not a question the graph can be asked. What the graph does carry is the
qualified name: `cfdb-045-polyglot-relationship-edges#3.4` fixes the PHP
scheme as `\Ns\Class::m`, namespace-qualified where the syntax provides
it. Prefix matching over that name is the discriminator, and it is the
whole of it.

This introduces no new syntax and no new domain concept. An `Owns`
value is already an opaque string; a PHP repository writes a namespace
prefix into it where a Rust repository writes a crate name. The
disambiguation of same-named units across build systems stays where
`graph-specs-004-multi-language-adapter-contract#2` put it, with RFC-007,
and this document does not reach for it.

### §3.3 The one reader that reads PHP, and its backend

`graph-specs-004-multi-language-adapter-contract#2` fixes the adapter
shape as two readers behind one composition, a spec-side attribute
reader and a code-side structural reader. The code-side half is
superseded: under
`graph-specs-010-abstraction-level-equivalence#1` and §3.3 the code side
is the `CodeFacts` port, and PHP reaches it through the cfdb-query
Anti-Corruption Layer by the edge-traversal path §11.5 names. graph-specs
owns no PHP structural parser.

The spec-side half survives, and it survives because nothing else can
carry it. The inline channel is the `#[Spec(...)]` PHP attribute;
`graph-specs-004-multi-language-adapter-contract#6` non-goal 3 forbids the
doc-comment alternative, and its invariant 5 keeps markdown the
universal spec source with the attribute channel additive. cfdb does not
model that payload: `cfdb-041-literal-extraction#1` records that cfdb
"extracts only `:Item` ... and `:CallSite`. String literals are not
modelled," and its §6 puts "Attribute-embedded strings (`#[doc=...]`)"
out of scope. So one PHP reader reads PHP source, it reads attributes
only, and it emits spec-side facts carrying the inline-attribute format
that `graph-specs-004-multi-language-adapter-contract#3.1` already
defines. Where it disagrees with the markdown, invariant 7 of that
document rules: markdown is the canonical upstream and the attribute is
the conformist.

The parser backend, which
`graph-specs-004-multi-language-adapter-contract#6` non-goal 5 assigned
here, is tree-sitter. The corpus already runs a pinned PHP grammar in
the producer that supplies this tool's PHP facts —
`cfdb-045-polyglot-relationship-edges#3.4` names
`tree-sitter-php-0.23.11` and stamps its edges with the resolver value
`"tree-sitter-php"`. Choosing the same grammar keeps one PHP syntax
model in the ecosystem. The reader is a leaf adapter and takes no
dependency on any other adapter, per
`graph-specs-016-parse-once-reading-port#1`.

### §3.4 Routing

`graph-specs-004-multi-language-adapter-contract#3.3` anticipated that a
PHP file would route to two readers, one per side. Under §3.3 above it
routes to one: the spec-side attribute reader. The code side is not a
reader and is not routed by file extension; it is a port the
composition root satisfies. This is a divergence from a ratified
sentence and is recorded as such rather than left to be discovered: the
sentence stands as written, and the reason it no longer describes the
PHP case is `graph-specs-010-abstraction-level-equivalence#3.3`, which
introduced the port after it.

## §4 Invariants

1. graph-specs owns no PHP structural parser. The PHP code side is the
   `CodeFacts` port and reaches PHP through the cfdb-query
   Anti-Corruption Layer's edge-traversal path
   (`graph-specs-010-abstraction-level-equivalence#1`, #3.3, #11.5).
2. No new node label and no new `:Item.kind` is emitted for PHP.
   `php_construct` is the class-versus-interface discriminator
   (`cfdb-045-polyglot-relationship-edges#4` invariant 2, #3.2), and any
   widening of the kind set is gated by
   `cfdb-041-language-backend-trait#4.5` on its own RFC plus a lockstep
   pull request here.
3. The PHP concept surface is declared, never derived. An item outside
   every declared namespace prefix binds no heading and demands none
   (`graph-specs-012-non-pub-spec-anchor#3.1`).
4. Markdown stays the universal spec source; the PHP attribute channel
   is additive and markdown is its canonical upstream
   (`graph-specs-004-multi-language-adapter-contract#4` invariants 5
   and 7).
5. The diff engine never branches on the language of a fact
   (`graph-specs-004-multi-language-adapter-contract#4` invariant 3;
   `graph-specs-010-abstraction-level-equivalence#4` invariant 4).
6. The PHP graph is closed-world. A class implementing an interface
   outside the workspace yields no relationship edge
   (`cfdb-045-polyglot-relationship-edges#3.2`); a concept whose only
   realization lies outside the workspace is satisfied by an anchor or
   not at all (`graph-specs-012-non-pub-spec-anchor#3.2`).
7. A keyspace carries one producer. Cross-resolver identity rests on
   qualified-name scheme disjointness and holds only within a
   single-producer keyspace
   (`cfdb-045-polyglot-relationship-edges#4`).

## §5 Non-goals

1. Not shipping a PHP structural parser (§3.3, invariant 1).
2. Not shipping doc-comment spec sources
   (`graph-specs-004-multi-language-adapter-contract#6` non-goal 3).
3. Not changing the bounded-context spec format
   (`graph-specs-004-multi-language-adapter-contract#6` non-goal 4).
4. Not disambiguating owned units across build systems; that is
   RFC-007's, per
   `graph-specs-004-multi-language-adapter-contract#2`.
5. Not the TypeScript surface. A different subject takes a different
   identifier.
6. Not inheritance transitivity, external implements-target
   resolution, or constructor calls
   (`cfdb-045-polyglot-relationship-edges#3.3`, #6).

## §6 Open — the operator's, not this document's

1. **The slug.** The reservation in
   `graph-specs-012-non-pub-spec-anchor` names the subject "the PHP
   ladder"; `graph-specs-004-multi-language-adapter-contract#2` calls the
   deferred work the PHP adapter. This document assumes they are one
   subject. If they are two, this document holds the wrong identifier
   and the reservation is still owed a document.
2. **The declaration form of a PHP owned unit.** §3.2 writes a
   namespace prefix into an existing opaque `Owns` string and adds no
   syntax. Whether a prefix needs a marked form to be distinguishable
   from a crate name is a question
   `graph-specs-004-multi-language-adapter-contract#2` routed to RFC-007,
   and this document does not answer it.
3. **What the surface is when a repository declares no contexts.** A
   Rust repository with no `specs/contexts/` keeps the fourth level a
   no-op (`graph-specs-004-multi-language-adapter-contract#3.8`) and
   still has `pub` as its surface test. A PHP repository in the same
   position has no surface test at all under §3.2. Whether that is a
   refusal, an empty surface, or a required declaration is not settled
   by any clause the council found.
