# orchestration

Concept-level entries for the **orchestration** bounded context —
the composition root (the CLI binary and the thin `run_check`
library) that wires readers into the diff engine and formats
violations. Only `ReportFormat` is library-public; all other types
it touches are binary-private and excluded from concept-level
equivalence. Prose is encouraged — it is ignored by the reader.

`run_check` loads the spec roots `--specs` names and, beside them, the
`specs/` of every installed Composer package under `--code` whose root
carries both a `composer.json` and at least one `specs/contexts/*.md`
(graph-specs#302, the reader's half of cfdb#754). The install tree is
`config.vendor-dir` when the `--code` manifest names one and `vendor`
otherwise, read two levels deep in sorted order, by the same predicate
and in the same order `cfdb extract` indexes those packages by — the two
instruments agreeing on the set is what makes a crossing have both of its
ends. The Composer knowledge is the PHP adapter's; a manifest that cannot
be parsed refuses rather than falling back to the default tree, since
where a workspace installs its packages then has no answer.

From an installed package the composition root takes two things and no
others: the concept nodes of its `specs/`, so a class it owns is grounded
by the spec corpus that describes it rather than demanded of the
consumer's, and its [ContextDecl](equivalence.md#contextdecl) values
marked `foreign`, so its owned units and exports reach the declared
surface while its own `## Imports` is never audited here. Its spec-side
edges, verb anchors, concept anchors and cohesion stay out: those are its
own quality, refused in its own repository, and a consumer that read them
would hold a package to a contract it has no standing to enforce. Its
`SpecTree` values are read for the one thing the concept rung needs —
which document declares which concept — and their cohesion findings are
computed over the walked tree alone.

## ReportFormat

<!-- parent:rfc:graph-specs-005-verb-coverage-report#3.4 anchor:"Text format: a three-section human table" -->

Output format for `graph-specs report`. `text` is the human-readable
default; `ndjson` emits one JSON object per report record — see
`specs/ndjson-output.md` §Report records (v0.5) for the schema. Lives
in `application`.
