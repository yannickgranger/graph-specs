# Changelog

All notable changes to graph-specs-rust will be documented in this file.


## [0.7.0] - 2026-07-31

> **⚠️ Breaking wire change — NDJSON `schema_version` `"3"` → `"4"`.**
>
> - The `implements_draft_concept` violation discriminator is **removed**. A
>   consumer matching on it now holds dead code that will never fire again;
>   the condition it reported is a `marker` record instead.
> - Two new record kinds arrive under a separate top-level `marker` key
>   (`pending`, `realized`). The `violation`-keyed stream is unchanged in
>   shape for every surviving variant.
> - `forbidden_concept_reintroduced` is additive and rides the same version.
>
> Consumers pinned to `"3"` must add a v4 arm before upgrading. See
> `specs/ndjson-output.md` §Schema evolution and issue #135.

### 🚀 Features

- *(rfc-013 [#169](https://github.com/yannickgranger/graph-specs/issues/169))* Slice A — spec-state marker, pending/realized records
- *(rfc-013 [#170](https://github.com/yannickgranger/graph-specs/issues/170))* Slice B — cohesion tightening (matrix row 6)
- *(rfc-014 [#168](https://github.com/yannickgranger/graph-specs/issues/168))* R14-1 — read `polarity:` and honour the three values

### 🐛 Bug Fixes

- *(ci)* Relocate test-only imports into the test module; drop redundant pub(crate) in private modules (clippy pedantic)
- *(rfc-013 [#170](https://github.com/yannickgranger/graph-specs/issues/170))* Unindent a doc list continuation line

### 📚 Documentation

- *(rfc)* RFC-013 — spec state marker: 4-lens unanimous RATIFY
- *(rfc)* RFC-014 — grounding polarity: 4-lens unanimous RATIFY
- *(rfc-014)* Drop issue references — RFCs are not derived from issues
- *(rfc-014)* Name the grounding/rootedness mismatch explicitly

### 🎨 Styling

- Cargo fmt — formatter leg never ran (v3 died at the phantom fence, agentry [#2875](https://github.com/yannickgranger/graph-specs/issues/2875))
- Visibility narrowing per the workspace nursery lane (redundant_pub_crate) + fmt

### ⚙️ Miscellaneous Tasks

- *(cross-fixture)* Lockstep bump cfdb pin → V0_6_0 (cfdb [#498](https://github.com/yannickgranger/graph-specs/issues/498) / PR [#506](https://github.com/yannickgranger/graph-specs/issues/506))
- *(lockstep)* Pin cross-fixture to cfdb [#510](https://github.com/yannickgranger/graph-specs/issues/510) (RFC-053 53-A, V0_7_0)
## [0.6.0] - 2026-06-07

### 🚀 Features

- *(rfc-012)* Non-pub spec anchors — `- impl:` markdown anchors resolve impl-method / non-`pub` items, removing the heading↔pub-type rule's caller-less ZST forcing (#143, #144)
- *(rfc-012)* Source-walk `AnchorResolver` + diff wiring + `Violation::DanglingAnchor` — a spec anchor naming a non-existent target now fails the gate (#146, #148)
- *(rfc-012)* `cohesion: behavioral` exemption + anti-gaming gate — behavioral / doctrine contexts may own no concept without a violation (#147, #149)
- *(rfc-012)* cfdb-query `AnchorResolver` parity (#151) + dialect Anchors section + NDJSON schema (#150)

## [0.1.0] - 2026-04-21

### 🚀 Features

- *([#1](https://github.com/yannickgranger/graph-specs/issues/1))* Scaffold hexagonal workspace
- *([#3](https://github.com/yannickgranger/graph-specs/issues/3))* Concept-level dogfood — first end-to-end equivalence check
- *([#5](https://github.com/yannickgranger/graph-specs/issues/5))* CI on Gitea Actions — fmt, clippy, pedantic, build, test, dogfood, audit
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Domain types for signature-level equivalence
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Signature extraction + normalisation (opt-in v0.2 semantics)
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* CLI inject-bite tests + self-dogfood rust block
- *([#9](https://github.com/yannickgranger/graph-specs/issues/9))* Relationship-level equivalence — syn-based declared edges (v0.3)
- *([#14](https://github.com/yannickgranger/graph-specs/issues/14))* Adopt cfdb for architectural ban rules — unwrap in domain/ports
- *([#13](https://github.com/yannickgranger/graph-specs/issues/13))* Add --format=ndjson structured output
- *([#22](https://github.com/yannickgranger/graph-specs/issues/22))* V0.4 domain types + CheckInput envelope + Violation::Context wrapper
- *([#23](https://github.com/yannickgranger/graph-specs/issues/23))* V0.4 ContextReader port trait
- *(ci)* Cross-dogfood fixture + shared SHA parser ([#32](https://github.com/yannickgranger/graph-specs/issues/32))
- *(ci)* Wire cross-dogfood CI + sccache setup ([#33](https://github.com/yannickgranger/graph-specs/issues/33))
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* V0.4 markdown context-file parser + MarkdownReader: ContextReader
- *(ci)* Weekly cross-fixture bump cron — Mon 06:30 UTC ([#34](https://github.com/yannickgranger/graph-specs/issues/34))
- *(ci)* Weekly closed-loop cross-check cron — Tue 06:30 UTC ([#37](https://github.com/yannickgranger/graph-specs/issues/37))
- *([#25](https://github.com/yannickgranger/graph-specs/issues/25))* V0.4 diff context pass — bounded-context enforcement
- *([#26](https://github.com/yannickgranger/graph-specs/issues/26))* NDJSON schema v2 + ContextViolation records
- *([#27](https://github.com/yannickgranger/graph-specs/issues/27))* V0.4 CLI text output for Violation::Context variants
- *([#28](https://github.com/yannickgranger/graph-specs/issues/28))* V0.4 self-dogfood — declare graph-specs-rust's own contexts
- *([#29](https://github.com/yannickgranger/graph-specs/issues/29))* V0.4 cfdb ban rules — context-boundary invariants
- *(domain)* Promote schema_version to typed SchemaVersion — dogfood spec drift

### 🐛 Bug Fixes

- *([#5](https://github.com/yannickgranger/graph-specs/issues/5))* Drop Node-based actions — manual clone + /cache volume
- *([#9](https://github.com/yannickgranger/graph-specs/issues/9))* Review follow-ups — Self resolution, lifetime strip, proof hygiene
- *(boy-scout [#25](https://github.com/yannickgranger/graph-specs/issues/25))* Reduce clones in cross-context edge violation emission
- *(cfdb-lockstep)* Pin cross-fixture to full 40-char SHA
- *(domain)* Use Self in SchemaVersion impl — clippy::use_self

### 🚜 Refactor

- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Diff() consumes Graph — move instead of clone-in-loop
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Hoist rust-reader I/O into read_and_parse helper
- *(boy-scout [#13](https://github.com/yannickgranger/graph-specs/issues/13))* Split god-files in domain/diff and markdown adapter
- *([#25](https://github.com/yannickgranger/graph-specs/issues/25))* Pre-index contexts to eliminate clones-in-loop
- *([#25](https://github.com/yannickgranger/graph-specs/issues/25))* Convert pre-index loops to iterator chains

### 📚 Documentation

- Initial README — first spec
- *([#3](https://github.com/yannickgranger/graph-specs/issues/3))* README maintenance — attribution, status update, specs path fix
- *([#3](https://github.com/yannickgranger/graph-specs/issues/3))* Dual MIT/Apache-2.0 license
- *([#5](https://github.com/yannickgranger/graph-specs/issues/5))* CI proofs — success + inject-bite + README badge
- *([#5](https://github.com/yannickgranger/graph-specs/issues/5))* Capture warm-run CI timing proof (AC 5)
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Capture AC8 hygiene proofs (clippy pedantic, audit, metrics)
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Re-capture AC8 proofs with explicit invocation + json verdict
- Update README for public release — use cases, agent workflow, why markdown
- *([#17](https://github.com/yannickgranger/graph-specs/issues/17))* Add CLAUDE.md codifying RFC-first methodology + dual control
- RFC-001 — v0.4 bounded-context equivalence (RATIFIED)
- Tests + real infra mandatory; architects prescribe in issues ([#19](https://github.com/yannickgranger/graph-specs/issues/19))
- *(RFC-002)* Draft cross-dogfood discipline with cfdb
- *(RFC-002)* Revision 1 — mirror amendment matching cfdb RFC-033 revision 1
- *(RFC-002)* Ratify — all four architect lenses RATIFY
- Cross-fixture-bump runbook — mirror of cfdb [#68](https://github.com/yannickgranger/graph-specs/issues/68) ([#35](https://github.com/yannickgranger/graph-specs/issues/35))
- Tests: template + SchemaVersion consumption note ([#36](https://github.com/yannickgranger/graph-specs/issues/36), [#38](https://github.com/yannickgranger/graph-specs/issues/38))
- *(runbook)* No manual SHA ceremony in SchemaVersion lockstep
- *([#30](https://github.com/yannickgranger/graph-specs/issues/30))* CHANGELOG.md — v0.4 schema v2 + overlap window policy
- *([#65](https://github.com/yannickgranger/graph-specs/issues/65))* RFC-003 — OSS readiness + public CI (DRAFT)
- *(RFC-003 r2 [#67](https://github.com/yannickgranger/graph-specs/issues/67))* Collapse dual-CI to mirror + tiny contributor-CI
- *(RFC-003 r3 [#67](https://github.com/yannickgranger/graph-specs/issues/67))* Apply round-1 architect verdicts (10 RC items)
- *(RFC-003 r4 [#67](https://github.com/yannickgranger/graph-specs/issues/67))* RATIFIED — round-2 verdicts (4× RATIFY)
- *(RFC-004)* Multi-language adapter contract (DRAFT round 1)
- *(RFC-004 r2)* Apply round-1 architect verdicts (14 RC items)
- *(RFC-004 r3 [#69](https://github.com/yannickgranger/graph-specs/issues/69))* RATIFIED — round-2 verdicts (4× RATIFY)

### 🎨 Styling

- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Cargo fmt
- *([#7](https://github.com/yannickgranger/graph-specs/issues/7))* Doc list indentation + single-char pattern
- *([#9](https://github.com/yannickgranger/graph-specs/issues/9))* Fix CI pedantic+nursery clippy lints on rust 1.93
- *([#9](https://github.com/yannickgranger/graph-specs/issues/9))* Rustfmt resolve_self one-liner
- *([#13](https://github.com/yannickgranger/graph-specs/issues/13))* Cargo fmt
- *([#13](https://github.com/yannickgranger/graph-specs/issues/13))* Clippy pedantic — add Errors docs, backtick snake_case, reword
- *([#22](https://github.com/yannickgranger/graph-specs/issues/22))* Trim WHAT-style doc comments per CLAUDE.md §1
- *([#23](https://github.com/yannickgranger/graph-specs/issues/23))* Trim port docs, convert stub from Ok to Err (§6 rule 3)
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* Cargo fmt
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* Split-brain fix + state simplification from simplify review
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* Cargo fmt on long match arm
- *([#25](https://github.com/yannickgranger/graph-specs/issues/25))* Cargo fmt

### 🧪 Testing

- *([#22](https://github.com/yannickgranger/graph-specs/issues/22))* Prefer assert_eq over match+panic in context wrap test
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* Replace panic! with unreachable! in variant-fallthrough arms

### ⚙️ Miscellaneous Tasks

- Carry forward missed [#1](https://github.com/yannickgranger/graph-specs/issues/1) fixes (.gitignore + fmt proof)
- *([#3](https://github.com/yannickgranger/graph-specs/issues/3))* Pedantic lints clean + supplementary quality proofs
- *([#13](https://github.com/yannickgranger/graph-specs/issues/13))* Recapture audit proof with exit code
- *(boy-scout [#13](https://github.com/yannickgranger/graph-specs/issues/13))* Add Makefile stubs for ship preflight contract
- *([#22](https://github.com/yannickgranger/graph-specs/issues/22))* Capture clippy pedantic + dogfood self-check proofs
- *([#22](https://github.com/yannickgranger/graph-specs/issues/22))* Add graph-specs-check Makefile target
- *([#24](https://github.com/yannickgranger/graph-specs/issues/24))* Refresh proofs post-dogfood fix
- *([#25](https://github.com/yannickgranger/graph-specs/issues/25))* Refresh proofs post-refactor
- Lockstep bump to cfdb [#35](https://github.com/yannickgranger/graph-specs/issues/35) HEAD — :Item.visibility v0.1.1
- Lockstep bump to cfdb [#36](https://github.com/yannickgranger/graph-specs/issues/36) HEAD — SchemaVersion v0.1.2
- Lockstep bump to cfdb [#83](https://github.com/yannickgranger/graph-specs/issues/83) HEAD — SchemaVersion v0.1.3
- Lockstep bump to cfdb [#94](https://github.com/yannickgranger/graph-specs/issues/94) HEAD — SchemaVersion v0.1.4
- Lockstep bump to cfdb [#86](https://github.com/yannickgranger/graph-specs/issues/86) HEAD — SchemaVersion v0.2.0
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#106](https://github.com/yannickgranger/graph-specs/issues/106) — SchemaVersion V0_2_0 → V0_2_1
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#42](https://github.com/yannickgranger/graph-specs/issues/42) — SchemaVersion V0_2_0 → V0_2_2
- *(cfdb-lockstep)* Refresh pin to post-merge cfdb develop bcdb080
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#107](https://github.com/yannickgranger/graph-specs/issues/107) (V0_2_2 → V0_2_3)
- *(release-infra)* Add release.yml + git-cliff + Makefile release-prepare
