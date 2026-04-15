# graph-specs-rust

A graph-based equivalence checker between markdown specifications and Rust code.

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

The tool validates its own specs from the first commit. The tool's `docs/` directory contains markdown specs for the tool itself; the tool reads those specs and its own source and diffs them. Every PR to this repository passes the same four-level check it imposes on downstream consumers.

## Consumer projects

The tool is project-agnostic. Initial dogfood targets:

- **graph-specs-rust itself** — validates its own specs as the primary regression test
- **qbot-core** — extends or replaces the existing `cfdb` sub-workspace
- **agency-orchestrator** (rewrite) — the primary external consumer, validates specs against code from day zero

## Anti-drift gate

Every feature PR in a consumer project runs the four-level check as a CI gate. A violation at any level blocks the merge. There is no baseline file, no ratchet, no allowlist — violations are fixed in the same PR that introduces them, or the PR does not land.

## Status

Early scaffolding. Specs for the tool's own behavior are being authored now; the Rust reader and markdown reader are not yet implemented.
