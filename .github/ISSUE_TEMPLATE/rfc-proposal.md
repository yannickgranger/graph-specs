---
name: RFC proposal
about: Propose a new capability (output format, equivalence level, CLI surface, schema bump)
title: "rfc: "
labels: rfc
---

New capabilities in `graph-specs` are **RFC-first**: the shape of the
solution is negotiated in a short RFC under `docs/rfc/` before code exists.
This issue starts that conversation — sketch it here; a ratified version
becomes `docs/rfc/NNN-<kebab-title>.md`.

## 1. Problem

What user-visible or methodology problem does this solve? Cite the concrete
case (issue, session, or upstream need) that prompts it.

## 2. Scope

Exact deliverables — what ships, and what explicitly does **not**.

## 3. Design (sketch)

Types, CLI surface, wire/NDJSON format, schema additions, exit codes.

## 4. Invariants

What must still hold after the change — the spec/code dogfood stays green,
the NDJSON wire schema stays stable (or bumps deliberately), backward
compatibility.

## 5. Non-goals

What this proposal explicitly does not address.

## 6. Rough decomposition

The vertical slices you envision, one per future issue. (Each slice will
carry a `Tests:` line — see [CONTRIBUTING.md](../../CONTRIBUTING.md).)
