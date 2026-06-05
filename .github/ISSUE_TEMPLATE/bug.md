---
name: Bug report
about: A wrong behaviour in an existing capability
title: "fix: "
labels: bug
---

## What happened

A clear description of the incorrect behaviour.

## Expected

What you expected `graph-specs` to do instead.

## Reproduction

Minimal steps — ideally a tiny `specs/` + code fixture and the exact command:

```bash
graph-specs check --specs <dir> --code <dir>
```

```
<actual output / exit code>
```

## Environment

- graph-specs version (`graph-specs --version`):
- OS / Rust toolchain:

## Notes

A bug fix lands with a **regression test** that reproduces this first
(red → green) in the same PR — see [CONTRIBUTING.md](../../CONTRIBUTING.md).
