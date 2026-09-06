# RFC graph-specs-017-founding-graph-model — Graph, Edge, EdgeKind

**Status: council synthesis 2026-08-20. RATIFIED on merge to doxa `develop` by the operator.**
Amendments follow the same path: council synthesis, operator merge.

Three concepts of the `equivalence` bounded context of `yg/graph-specs-rust` are ruled here and nowhere else: the graph the two readers produce, the relationship a concept declares about another, and that relationship's kind. Their design authority is this document. Everything else that context holds is ruled by the RFC that designed it, and this document rules none of it.

Two citation forms appear below and the difference is deliberate. A ratified ancestor is named in the full identifier-and-clause form, never abbreviated, because `keel-dialect` §6.5 admits no alias into the index or into any reader and an abbreviated identifier resolves to nothing. A reference to the harness or the dialect names a bare section rather than a clause in the identifier-and-clause form, because neither is ever an ancestor of anything here and a full-form citation of one would read as the ancestry claim the harness's R3 forbids.

Every sentence below descends from one of two sources, named beside it: a ratified clause of the corpus, or the founding text of this model recorded in the source repository's history at the commit named in §2. Where a sentence this model would carry has neither, it is not written as a rule: it is raised at §7. No sentence here is licensed by the existence of a mechanism in any tree, and no ratified sentence is rewritten by this document.

## §1 — Problem

The three oldest concepts of the `equivalence` bounded context — the graph, the edge, and the edge's kind — have no ancestor in the corpus. Every other concept of that context roots in the RFC that designed it; these three predate the RFC series and were carried into it undeclared.

The corpus rules on them without ever defining them. `graph-specs-001-bounded-context-equivalence#3.7` rules what does not go inside the graph and leaves the graph itself unchanged; `graph-specs-001-bounded-context-equivalence#3.8` publishes the graph, the concept node and the edge as the context's Published Language; `graph-specs-004-multi-language-adapter-contract#1` requires the two readers to produce graphs of identical shape; `graph-specs-006-verb-anchoring#3.3` names the graph the Published Language of type-level equivalence and refuses verb ownership entry to it; `graph-specs-006-verb-anchoring#3.1` rules the kind a closed type-to-type relationship, in a clause whose subject is the syntax of a different bullet. Each of those is a ruling made from a different subject, and none is the model's design authority: rooting a concept on one of them would be selection rather than transcription, and a root is exclusive and leaf-only (`keel-dialect` §3.5).

The founding text exists, outside the corpus, in the source repository's history. This RFC brings it in as the three concepts' design authority. It changes no ratified sentence: every clause of `graph-specs-001-bounded-context-equivalence`, `graph-specs-004-multi-language-adapter-contract` and `graph-specs-006-verb-anchoring` named here stands exactly as written and is cited, never rewritten. This RFC is the model's design authority and nothing else's; the harness and the dialect govern admission and form and are never a model's ancestor (the harness's §7, item 1).

## §2 — The founding record

Two commits in `yg/graph-specs-rust` carry the sentences this RFC transcribes. They are named because the corpus holds no earlier address for them; the text itself is in that repository's history, and the corpus history is the provenance of everything below.

- `0641c31` (2026-04-15, "docs: initial README — first spec") — the tool's purpose, the one-graph-two-readers shape, the levels of equivalence, and the exactness non-goal. The first spec is the introduction document, and the tool was built against it.
- `11aad13` (2026-04-15, "feat(#9): relationship-level equivalence — syn-based declared edges (v0.3)") — the introduction of the edge and its kind, the edge collection carried beside the nodes, the single matching token, the per-concept opt-in, and the three kinds with their reading rule on each side.

The shape the first spec states:

```
specs (*.md)  ──▶ markdown reader ──▶ graph(specs) ─┐
                                                    ├──▶ diff ──▶ violations
code  (*.rs)  ──▶ Rust reader     ──▶ graph(code)  ─┘
```

One rule governs this document: a sentence here is a transcription of one of those two commits, or a citation of a ratified clause, or it is not written.

## §3 — The model

### §3.1 — Graph

A graph is everything one reader found on one side of the check: the concepts it read, by name, and the relationships those concepts declare (`0641c31`; the relationships arrive as a collection carried beside the nodes, `11aad13`).

Both sides produce one shape. The two readers are fully independent, and what they hand the diff engine is the same kind of thing whether it was read from specifications or from code (`0641c31`; `graph-specs-004-multi-language-adapter-contract#1`). Which side a fact came from is carried by that fact's own source, at the site the reader found it (`graph-specs-004-multi-language-adapter-contract#3.1.3`).

A graph asserts; it does not measure. Two concepts correspond when their names are equal, in both directions: a name the specifications carry and the code does not is a violation, and so is a name the code carries and the specifications do not (`0641c31`). There is no similarity, no score and no threshold anywhere in this model (`0641c31`).

The graph is the Published Language of the `equivalence` context's type-level equivalence (`graph-specs-006-verb-anchoring#3.3`), published as such to the contexts that read and orchestrate (`graph-specs-001-bounded-context-equivalence#3.8`).

Two things the corpus has ruled out of the graph stay out of it, each on its own ground. Bounded-context declarations do not live inside the graph and ride in the check input beside it (`graph-specs-001-bounded-context-equivalence#3.7`). Verb ownership is a categorically distinct aggregate that must not corrupt the graph's bounded context (`graph-specs-006-verb-anchoring#3.3`).

The graph grows by addition. The corpus already holds it open for extension so that a later field is not a break for anything downstream (`graph-specs-001-bounded-context-equivalence#3.7`).

### §3.2 — Edge

An edge is a relationship one concept declares about another, and it belongs to the concept that declares it — on the specification side the concept whose section carries the declaration, on the code side the item the relationship was read from (`0641c31` level 3; `11aad13`).

Beyond the concept it belongs to, an edge carries its kind, the target it names reduced to a single matching token, that target's raw text kept for display in drift messages, and the site where the reader found it. The corpus fixes that shape by giving the verb anchor the same one — a tokenized match key beside a verbatim raw target, carried with the concept and the source site (`graph-specs-006-verb-anchoring#3.3`) — and the founding text names the kind, the token and the site when it introduces the relationship (`11aad13`).

The reduction is what lets two independent readers agree: the target is reduced to a single primary token, and both readers reduce to the same one, so that a declaration written in prose and a fact read out of code arrive together (`11aad13`). The founding text names what is stripped to reach that token — references, mutability, lifetimes, generic arguments and module paths (`11aad13`). Whatever a reader strips, the diff engine never branches on the language of a fact (`graph-specs-004-multi-language-adapter-contract#4` invariant 3).

Because an edge belongs to the concept that declares it, two edges correspond when that concept, the kind and the reduced target all agree. Nothing else is compared and no other evidence is admitted (`11aad13`; `0641c31`).

A relationship the specifications declare and the code does not carry is a violation; a relationship the code carries and the specifications have not declared is a violation (`0641c31` level 3).

A target the specifications name that is no concept of the project is a third finding, distinct from both: the declaration points at nothing this model holds, and it is reported as such rather than as a demand the code failed to meet (`11aad13`). The two sides are not symmetric here, and the asymmetry is the model's: a declaration can name something that does not exist and is then wrong, while a code fact naming something outside the project is simply not a relationship of this model (`11aad13`).

Relationship-level equivalence is opt-in, one concept at a time. A concept whose specification side declares no relationship imposes none, and the check over that concept is exactly what it was before relationships existed (`11aad13`).

Two further questions about a relationship are ruled outside this clause and are cited here rather than restated: whether a concept's own declared state relieves it of the demands its declarations would otherwise make, and whether a concept may bear a demand another concept's declaration makes of it. Both are stated once, at `graph-specs-015-spec-retirement-state#3.4`, and carried by citation everywhere else, this clause included.

Relationships are compared only for concepts both sides carry. A concept missing from one side is reported once, at the concept level, and never again as a fan of missing relationships underneath it (`11aad13`).

A code-side relationship whose target is not a concept of the project is not an edge of this model. The graph is closed over the project's own concepts, so primitives, standard-library types and foreign types leave no edge behind (`11aad13`).

### §3.3 — EdgeKind

The kind of an edge is the kind of relationship it states. The founding text names the class as dependency, composition, or call (`0641c31` level 3); the founding commit fixes three — implementation, dependency and return — and defers call-level and use-level declarations to later work (`11aad13`).

Each kind means one thing on both sides, and that correspondence is the whole of the kind's content: implementation is a type implementing a trait; dependency is a concept named in another concept's fields, in a function's parameters, or inside the generic arguments of what a function returns; return is a concept named as the result of a published function (`11aad13`).

The set is closed at three, and what it kinds is a relationship between two concepts: the corpus rules the kind a closed type-to-type relationship enumeration (`graph-specs-006-verb-anchoring#3.1`), which is why a concept's claim on a function is a different construct and never one of these (`graph-specs-006-verb-anchoring#3.3`). A fourth kind is an amendment to this clause and never a reader's local extension. The three declaration prefixes that name the kinds on the specification side are reserved corpus-wide by the dialect's §5.3; the kinds themselves are this clause's.

Outside the context a kind is carried by one label — `IMPLEMENTS`, `DEPENDS_ON`, `RETURNS` — and those labels are what a consumer of the tool's output binds to. The three are named in the founding text at their introduction (`11aad13`); one of them, the dependency label, is additionally carried inside a ratified record shape (`graph-specs-001-bounded-context-equivalence#3.3`), and the cross-context findings carry a kind by its own type rather than by a label (`graph-specs-001-bounded-context-equivalence#3.2`). How those records may change between versions is the schema contract's rule, not this clause's (`graph-specs-001-bounded-context-equivalence#3.3`, `graph-specs-004-multi-language-adapter-contract#3.5`, `graph-specs-004-multi-language-adapter-contract#4` invariant 2).

## §4 — What holds outside the context

1. **A host that declares no relationship is not checked for relationships.** The opt-in of §3.2 is a contract with every consumer tree, not an implementation convenience: a repository whose specifications carry no declaration prefix sees exactly the concept- and signature-level check.
2. **A consumer stands outside the model and reads its output.** Treating the tool as an external consumer would is the architecturally correct shape, and the corpus rules that reaching for its library surface from a caller's own CI bypasses the port layer (`graph-specs-002-cross-dogfood#5.1`); the corpus records the same of the tool's own downstream consumers, which import none of these types (`graph-specs-006-verb-anchoring#3.3`). A kind therefore reaches a consumer as one of the three labels of §3.3, and never as the model's internal spelling of it.
3. **Cross-boundary policy is not this RFC's.** Whether a relationship that crosses a context boundary is sanctioned is ruled by `graph-specs-001-bounded-context-equivalence#3.1`, `graph-specs-001-bounded-context-equivalence#3.2` and `graph-specs-001-bounded-context-equivalence#4` invariants 5 and 6. This RFC rules what a relationship is; that RFC rules which ones are allowed.
4. **Wire compatibility is not this RFC's.** Record shapes, their fields and their version gate are ruled by `graph-specs-001-bounded-context-equivalence#3.3` and `graph-specs-004-multi-language-adapter-contract#3.5`, under the version-explicit rule of `graph-specs-004-multi-language-adapter-contract#4` invariant 2.
5. **The concept node and its payloads are not this RFC's.** The concept node, its signature payload, its containment provenance, its state marker and its polarity are each ruled by the clause its own heading roots in, or chains to. This RFC rules the graph that holds them, the relationship, and the relationship's kind — nothing else of the context.

## §5 — Non-goals

- Inference, reasoning, or draft generation of any kind: the model is read mechanically and compared mechanically (`0641c31`).
- Any measure of closeness. Equivalence is exact or it is a violation (`0641c31`).
- Generating documentation or generating code — both sides are authored separately (`0641c31`).
- The authoring grammar of the specification side. Which prefixes carry a declaration, and how a document is written, is the dialect's §5.3, not this RFC's.
- How the tool is packaged, installed or invoked on any host. That a caller reaching for the library surface from its own CI bypasses the port layer is ruled at `graph-specs-002-cross-dogfood#5.1`; how a binary reaches a host is no clause of this RFC.
- Ruling any other level of equivalence. The bounded-context level and the cohesion level are ruled by the RFCs that designed them; this RFC rules the graph every level reads, the relationship, and the relationship's kind. The signature level has no design authority in the corpus; that gap is recorded at §7.

## §6 — Transcription owed

Three headings of `specs/concepts/equivalence.md` in `yg/graph-specs-rust` gain a root on ratification: the graph on §3.1, the edge on §3.2, the kind on §3.3, each with an anchor taken verbatim from that clause's body. The anchor phrases are not quoted here: a phrase this document repeated in a second clause would stop discriminating between the two, and the declaration citing it would be malformed (the dialect's §3.4). No other heading's root moves, and no ratified clause is renumbered by this RFC.

This RFC is authored in the corpus and enters no repository's mirror: it takes no provenance entry, and a corpus id without a provenance entry is not mirrored — the precedent is `graph-specs-016-parse-once-reading-port`, which carries none. The only change owed in the source repository beyond the three roots is the corpus pin the transcribing pass bumps.

## §7 — Raises

**R1 — the kind's Published Language status has no ancestor in either source, and this document does not settle it.** The context's ratified export list names the graph, the concept node, the edge, the source, the violation, the context declaration and the two ports, and does not name the kind (`graph-specs-001-bounded-context-equivalence#3.8`). The founding text cannot name it either: the bounded-context layer did not exist when the kind was introduced (`11aad13` predates `graph-specs-001-bounded-context-equivalence`). The tree's own context declaration nonetheless exports the kind as Published Language, and the corpus carries the kind inside ratified cross-context payloads (`graph-specs-001-bounded-context-equivalence#3.2`). Two positions, neither adopted here:

> **Name it.** The kind crosses the context boundary on every record that carries one, so it is published in fact; a document that rules the kind and leaves its boundary status unwritten leaves the export declaration ungrounded exactly as the three concepts were.

> **Leave it.** The export list is `graph-specs-001-bounded-context-equivalence#3.8`'s ratified content, and naming a new member of it from another document amends that clause from outside; the boundary declaration is that RFC's surface, and the correct instrument is an amendment to it, not a sentence here.

**R2 — the signature level has no design authority in the corpus.** This council found the gap while bounding what this RFC does not rule, and records it once rather than asserting otherwise. No RFC of the graph-specs subject designs signature-level equivalence: the founding commit already describes the tool as extending "from signature-level (v0.2)" and predates the RFC series (`11aad13`), and `graph-specs-006-verb-anchoring#2` defers function-signature-level equivalence to a future RFC. The level is nonetheless checked on every consumer tree. Two dispositions, neither adopted here:

> **Close it.** The level is in exactly the position these three concepts were in before this document: founded before the corpus existed, ruled by nothing, relied on by every host. The same instrument closes it — a founding record transcribed from the same history.

> **Leave it.** No heading roots on the level itself, so nothing in any tree is ungrounded by its absence, and a document is authored when a concept needs an ancestor rather than to complete a set.

Both raises reach the operator once, with this evidence. Nothing of this council is contested.
