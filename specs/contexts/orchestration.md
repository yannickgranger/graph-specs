# orchestration

Composition root: the CLI binary and the thin `run_check` library that
wires the markdown + Rust readers into the diff engine and formats
violations as text or NDJSON. No domain logic lives here — every
substantive responsibility is delegated to `equivalence` (the diff
engine, the violation types) or `reading` (the concrete readers).

Orchestration stands in a **Customer-Supplier** relationship to
`reading`: it depends on the concrete `MarkdownReader` / `RustReader`
types to satisfy its composition, but reading is free to evolve its
internal parser state machines as long as the published surface holds.

## Owns

- application

## Exports

(none — `orchestration` is a leaf consumer; it publishes nothing
cross-context. The CLI binary is its own deliverable.)

## Imports

- CheckInput from equivalence (PublishedLanguage)
- Violation from equivalence (PublishedLanguage)
- ContextViolation from equivalence (PublishedLanguage)
- Source from equivalence (PublishedLanguage)
- Reader from equivalence (PublishedLanguage)
- ContextReader from equivalence (PublishedLanguage)
- VerbReader from equivalence (PublishedLanguage)
- ReaderError from equivalence (PublishedLanguage)
- ReportOutput from equivalence (PublishedLanguage)
- PubFnDecl from equivalence (PublishedLanguage)
- InvariantAnnotation from equivalence (PublishedLanguage)
- MarkdownReader from reading (CustomerSupplier)
- RustReader from reading (CustomerSupplier)

## Concepts

`ReportFormat` is the one library-public type owned by `orchestration`
— it must appear in `specs/concepts/core.md`. All other types it
touches (CLI command structs, `Format`) are binary-private and excluded
from concept-level equivalence checking.
