# graph-specs-rust

A graph-based equivalence checker between markdown specifications and source code.

## Purpose

Build a single graph from two independent sources — markdown specifications and Rust code — and report every point where they disagree.

This tool is the primary **anti-drift gate** for projects that want specifications and implementation to stay strictly synchronized. If a spec says concept X exists, the code must contain type X. If the code introduces an edge between two contexts, the spec must declare that edge. Violations block merges.

## Non-goals

- **Not an LLM.** No inference, no reasoning, no draft generation. Pure mechanical parse-and-compare.
- **Not a documentation generator.** Specs are authored by humans (and their assistants) as a separate activity.
- **Not a code generator.** Code is authored separately.
- **Not a fuzzy matcher.** Equivalence is either exact or a violation. There is no similarity score.

## Core concept: one graph, two readers

```
specs (*.md)  ──▶ markdown reader ──▶ graph(specs) ─┐
                                                    ├──▶ diff ──▶ violations
code  (*.rs)  ──▶ Rust reader     ──▶ graph(code)  ─┘
```

The two readers are fully independent. They produce graphs in the same shape. The diff engine is the single source of truth about equivalence.

## The four levels of equivalence

Every violation reported by the tool belongs to one of four levels. Each level must hold on every feature PR in a consumer project.

### 1. Concept
Every named concept in the specs exists as a type in the code, and vice versa. No spec-only concepts, no code-only types.

### 2. Signature
Every port trait declared in the specs matches the actual Rust trait signature — method names, parameter types, return types, error types.

### 3. Relationship
Every dependency, composition, or call edge the specs declare must exist in the code's graph, and no edge in code that the specs have not declared.

### 4. Bounded context
Every bounded context named in the specs maps to exactly one declared set of crates. No type crosses a context boundary unless the specs explicitly declare the crossing (ACL, shared kernel, published language, etc.).

## Dogfooding from day zero

The tool validates its own specs from the first commit. The tool's `specs/` directory contains markdown specifications for the tool itself; the tool reads those specs and its own source and diffs them. Every PR to this repository passes the same four-level check it imposes on downstream consumers.

Two self-control layers ship in CI:

| Layer | Tool | Question answered |
|---|---|---|
| Equivalence | `graph-specs check` | Do the specs match the code? |
| Architectural bans | [`cfdb`](https://github.com/yannickgranger/cfdb) violations | Does the code use forbidden patterns? |

The cfdb layer runs pinned Cypher rules under `.cfdb/queries/` — e.g. `arch-ban-unwrap-domain-ports.cypher` forbids `.unwrap()` in non-test items inside the hexagonal core. The cfdb commit is pinned in `.cfdb/cfdb.rev`.

## Anti-drift gate

Every feature PR runs the four-level check as a CI gate. A violation at any level blocks the merge. There is no baseline file, no ratchet, no allowlist — violations are fixed in the same PR that introduces them, or the PR does not land.

## Why markdown specs?

Markdown is the format that **both humans and AI agents read natively**. Unlike code comments, AST annotations, or database schemas, markdown specs are:

- **Readable without tooling** — an architect reviews a spec file in 2 minutes instead of doing a 30-minute deep code dive across source files
- **Writable by agents** — LLM coding assistants produce well-structured markdown without special prompting or output parsing
- **Diffable in PRs** — spec changes show up as clean text diffs alongside code changes, making architectural decisions visible in review
- **Ingestible by any LLM context window** — a spec file is a compact, high-signal summary of what a module owns, perfect for feeding into an agent's context before it starts coding

When an AI coding agent starts a session, it can read `specs/` to understand the architecture in seconds rather than reconstructing it from scattered source files. When it finishes, the CI gate ensures its changes didn't break the architectural contract.

## Use cases

### Standalone CI gate

The simplest integration: add `graph-specs check` to your CI pipeline. Point it at your spec directory and your source directory. If a developer adds a type without speccing it, or changes a trait signature without updating the spec, the build fails.

```bash
graph-specs check --specs specs/my-context/ --code crates/my-context/src/
```

### Paired with a code-facts database (cfdb pattern)

For large codebases with existing technical debt, graph-specs pairs naturally with a **code-facts database** — a tool that extracts structural facts (types, call chains, dependencies) from source code into a queryable store.

The division of labor:

| Tool | Role | When it runs |
|------|------|-------------|
| **cfdb** (code-facts DB) | Detect existing debt — duplicates, bypasses, unfinished refactors | On-demand audits, debt triage |
| **graph-specs** | Prevent new drift — block PRs that violate the spec contract | Every PR, in CI |

cfdb is the **X-ray** (finds the disease). graph-specs is the **vaccine** (prevents reinfection). Use cfdb to clean up, then spec-lock the cleaned context so it stays clean.

This repository uses both tools on itself — see `.cfdb/queries/` for the live ban rules.

### Agent-in-the-middle workflow

In an agentic coding setup where AI assistants write code autonomously:

1. **Before coding** — the agent reads `specs/` to understand what exists, what the contracts are, and what the bounded context owns. This replaces the "discovery" phase that would otherwise require expensive code archaeology.

2. **During coding** — the agent knows that any new type needs a spec entry, and any changed trait signature needs a spec update. The spec is the architectural guardrail.

3. **Before shipping** — the agent runs `graph-specs check` locally. If it drifted, it fixes the spec or the code before pushing. CI is the backstop, not the discovery point.

4. **Across sessions** — specs survive session boundaries. Session N cleans up a split-brain and specs the context. Session N+1 reads the spec, understands what exists, and wires into it instead of creating a parallel implementation. The spec is **durable architectural memory** that doesn't depend on the agent recalling prior work.

### Multi-language projects (planned)

The tool is designed for multiple language adapters behind the same port trait. Rust (via `syn`) ships today. PHP and TypeScript adapters (via tree-sitter) are planned. The spec format is language-agnostic — the same markdown file works regardless of which language the code is written in.

## Status

Concept-level and signature-level checks implemented end-to-end and dogfooded against this repository. Relationship and bounded-context levels are planned.

## Authorship

100% written by **Claude** (Anthropic's AI coding assistant) under the proud supervision of its human lead. Every commit, test, and line of documentation — including this README — originates from a Claude session. The human reviews and ratifies; Claude builds. Issues and pull requests are welcome from anyone.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. This is the Rust-ecosystem convention — it gives downstream users maximum flexibility by letting them pick whichever license fits their project best.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions.

<!-- agentry-smoke-test-2026-04-25 -->
