# Changelog

All notable changes to graph-specs-rust will be documented in this file.

## [0.1.0] - 2026-04-21

### 🚀 Features

- *([#1](https://agency.lab:3000/yg/graph-specs-rust/issues/1))* Scaffold hexagonal workspace
- *([#3](https://agency.lab:3000/yg/graph-specs-rust/issues/3))* Concept-level dogfood — first end-to-end equivalence check
- *([#5](https://agency.lab:3000/yg/graph-specs-rust/issues/5))* CI on Gitea Actions — fmt, clippy, pedantic, build, test, dogfood, audit
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Domain types for signature-level equivalence
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Signature extraction + normalisation (opt-in v0.2 semantics)
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* CLI inject-bite tests + self-dogfood rust block
- *([#9](https://agency.lab:3000/yg/graph-specs-rust/issues/9))* Relationship-level equivalence — syn-based declared edges (v0.3)
- *([#14](https://agency.lab:3000/yg/graph-specs-rust/issues/14))* Adopt cfdb for architectural ban rules — unwrap in domain/ports
- *([#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Add --format=ndjson structured output
- *([#22](https://agency.lab:3000/yg/graph-specs-rust/issues/22))* V0.4 domain types + CheckInput envelope + Violation::Context wrapper
- *([#23](https://agency.lab:3000/yg/graph-specs-rust/issues/23))* V0.4 ContextReader port trait
- *(ci)* Cross-dogfood fixture + shared SHA parser ([#32](https://agency.lab:3000/yg/graph-specs-rust/issues/32))
- *(ci)* Wire cross-dogfood CI + sccache setup ([#33](https://agency.lab:3000/yg/graph-specs-rust/issues/33))
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* V0.4 markdown context-file parser + MarkdownReader: ContextReader
- *(ci)* Weekly cross-fixture bump cron — Mon 06:30 UTC ([#34](https://agency.lab:3000/yg/graph-specs-rust/issues/34))
- *(ci)* Weekly closed-loop cross-check cron — Tue 06:30 UTC ([#37](https://agency.lab:3000/yg/graph-specs-rust/issues/37))
- *([#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* V0.4 diff context pass — bounded-context enforcement
- *([#26](https://agency.lab:3000/yg/graph-specs-rust/issues/26))* NDJSON schema v2 + ContextViolation records
- *([#27](https://agency.lab:3000/yg/graph-specs-rust/issues/27))* V0.4 CLI text output for Violation::Context variants
- *([#28](https://agency.lab:3000/yg/graph-specs-rust/issues/28))* V0.4 self-dogfood — declare graph-specs-rust's own contexts
- *([#29](https://agency.lab:3000/yg/graph-specs-rust/issues/29))* V0.4 cfdb ban rules — context-boundary invariants
- *(domain)* Promote schema_version to typed SchemaVersion — dogfood spec drift

### 🐛 Bug Fixes

- *([#5](https://agency.lab:3000/yg/graph-specs-rust/issues/5))* Drop Node-based actions — manual clone + /cache volume
- *([#9](https://agency.lab:3000/yg/graph-specs-rust/issues/9))* Review follow-ups — Self resolution, lifetime strip, proof hygiene
- *(boy-scout [#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* Reduce clones in cross-context edge violation emission
- *(cfdb-lockstep)* Pin cross-fixture to full 40-char SHA
- *(domain)* Use Self in SchemaVersion impl — clippy::use_self

### 🚜 Refactor

- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Diff() consumes Graph — move instead of clone-in-loop
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Hoist rust-reader I/O into read_and_parse helper
- *(boy-scout [#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Split god-files in domain/diff and markdown adapter
- *([#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* Pre-index contexts to eliminate clones-in-loop
- *([#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* Convert pre-index loops to iterator chains

### 📚 Documentation

- Initial README — first spec
- *([#3](https://agency.lab:3000/yg/graph-specs-rust/issues/3))* README maintenance — attribution, status update, specs path fix
- *([#3](https://agency.lab:3000/yg/graph-specs-rust/issues/3))* Dual MIT/Apache-2.0 license
- *([#5](https://agency.lab:3000/yg/graph-specs-rust/issues/5))* CI proofs — success + inject-bite + README badge
- *([#5](https://agency.lab:3000/yg/graph-specs-rust/issues/5))* Capture warm-run CI timing proof (AC 5)
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Capture AC8 hygiene proofs (clippy pedantic, audit, metrics)
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Re-capture AC8 proofs with explicit invocation + json verdict
- Update README for public release — use cases, agent workflow, why markdown
- *([#17](https://agency.lab:3000/yg/graph-specs-rust/issues/17))* Add CLAUDE.md codifying RFC-first methodology + dual control
- RFC-001 — v0.4 bounded-context equivalence (RATIFIED)
- Tests + real infra mandatory; architects prescribe in issues ([#19](https://agency.lab:3000/yg/graph-specs-rust/issues/19))
- *(RFC-002)* Draft cross-dogfood discipline with cfdb
- *(RFC-002)* Revision 1 — mirror amendment matching cfdb RFC-033 revision 1
- *(RFC-002)* Ratify — all four architect lenses RATIFY
- Cross-fixture-bump runbook — mirror of cfdb [#68](https://agency.lab:3000/yg/graph-specs-rust/issues/68) ([#35](https://agency.lab:3000/yg/graph-specs-rust/issues/35))
- Tests: template + SchemaVersion consumption note ([#36](https://agency.lab:3000/yg/graph-specs-rust/issues/36), [#38](https://agency.lab:3000/yg/graph-specs-rust/issues/38))
- *(runbook)* No manual SHA ceremony in SchemaVersion lockstep
- *([#30](https://agency.lab:3000/yg/graph-specs-rust/issues/30))* CHANGELOG.md — v0.4 schema v2 + overlap window policy
- *([#65](https://agency.lab:3000/yg/graph-specs-rust/issues/65))* RFC-003 — OSS readiness + public CI (DRAFT)
- *(RFC-003 r2 [#67](https://agency.lab:3000/yg/graph-specs-rust/issues/67))* Collapse dual-CI to mirror + tiny contributor-CI
- *(RFC-003 r3 [#67](https://agency.lab:3000/yg/graph-specs-rust/issues/67))* Apply round-1 architect verdicts (10 RC items)
- *(RFC-003 r4 [#67](https://agency.lab:3000/yg/graph-specs-rust/issues/67))* RATIFIED — round-2 verdicts (4× RATIFY)
- *(RFC-004)* Multi-language adapter contract (DRAFT round 1)
- *(RFC-004 r2)* Apply round-1 architect verdicts (14 RC items)
- *(RFC-004 r3 [#69](https://agency.lab:3000/yg/graph-specs-rust/issues/69))* RATIFIED — round-2 verdicts (4× RATIFY)

### 🎨 Styling

- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Cargo fmt
- *([#7](https://agency.lab:3000/yg/graph-specs-rust/issues/7))* Doc list indentation + single-char pattern
- *([#9](https://agency.lab:3000/yg/graph-specs-rust/issues/9))* Fix CI pedantic+nursery clippy lints on rust 1.93
- *([#9](https://agency.lab:3000/yg/graph-specs-rust/issues/9))* Rustfmt resolve_self one-liner
- *([#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Cargo fmt
- *([#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Clippy pedantic — add Errors docs, backtick snake_case, reword
- *([#22](https://agency.lab:3000/yg/graph-specs-rust/issues/22))* Trim WHAT-style doc comments per CLAUDE.md §1
- *([#23](https://agency.lab:3000/yg/graph-specs-rust/issues/23))* Trim port docs, convert stub from Ok to Err (§6 rule 3)
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* Cargo fmt
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* Split-brain fix + state simplification from simplify review
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* Cargo fmt on long match arm
- *([#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* Cargo fmt

### 🧪 Testing

- *([#22](https://agency.lab:3000/yg/graph-specs-rust/issues/22))* Prefer assert_eq over match+panic in context wrap test
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* Replace panic! with unreachable! in variant-fallthrough arms

### ⚙️ Miscellaneous Tasks

- Carry forward missed [#1](https://agency.lab:3000/yg/graph-specs-rust/issues/1) fixes (.gitignore + fmt proof)
- *([#3](https://agency.lab:3000/yg/graph-specs-rust/issues/3))* Pedantic lints clean + supplementary quality proofs
- *([#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Recapture audit proof with exit code
- *(boy-scout [#13](https://agency.lab:3000/yg/graph-specs-rust/issues/13))* Add Makefile stubs for ship preflight contract
- *([#22](https://agency.lab:3000/yg/graph-specs-rust/issues/22))* Capture clippy pedantic + dogfood self-check proofs
- *([#22](https://agency.lab:3000/yg/graph-specs-rust/issues/22))* Add graph-specs-check Makefile target
- *([#24](https://agency.lab:3000/yg/graph-specs-rust/issues/24))* Refresh proofs post-dogfood fix
- *([#25](https://agency.lab:3000/yg/graph-specs-rust/issues/25))* Refresh proofs post-refactor
- Lockstep bump to cfdb [#35](https://agency.lab:3000/yg/graph-specs-rust/issues/35) HEAD — :Item.visibility v0.1.1
- Lockstep bump to cfdb [#36](https://agency.lab:3000/yg/graph-specs-rust/issues/36) HEAD — SchemaVersion v0.1.2
- Lockstep bump to cfdb [#83](https://agency.lab:3000/yg/graph-specs-rust/issues/83) HEAD — SchemaVersion v0.1.3
- Lockstep bump to cfdb [#94](https://agency.lab:3000/yg/graph-specs-rust/issues/94) HEAD — SchemaVersion v0.1.4
- Lockstep bump to cfdb [#86](https://agency.lab:3000/yg/graph-specs-rust/issues/86) HEAD — SchemaVersion v0.2.0
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#106](https://agency.lab:3000/yg/graph-specs-rust/issues/106) — SchemaVersion V0_2_0 → V0_2_1
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#42](https://agency.lab:3000/yg/graph-specs-rust/issues/42) — SchemaVersion V0_2_0 → V0_2_2
- *(cfdb-lockstep)* Refresh pin to post-merge cfdb develop bcdb080
- *(cfdb-lockstep)* Bump cross-fixture to cfdb [#107](https://agency.lab:3000/yg/graph-specs-rust/issues/107) (V0_2_2 → V0_2_3)
- *(release-infra)* Add release.yml + git-cliff + Makefile release-prepare
