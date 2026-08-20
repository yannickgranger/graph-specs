# orchestration

Concept-level entries for the **orchestration** bounded context —
the composition root (the CLI binary and the thin `run_check`
library) that wires readers into the diff engine and formats
violations. Only `ReportFormat` is library-public; all other types
it touches are binary-private and excluded from concept-level
equivalence. Prose is encouraged — it is ignored by the reader.

## ReportFormat

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.4 anchor:"Text format: a three-section human table" -->

Output format for `graph-specs report`. `text` is the human-readable
default; `ndjson` emits one JSON object per report record — see
`specs/ndjson-output.md` §Report records (v0.5) for the schema. Lives
in `application`.
