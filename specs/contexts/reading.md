# reading

Adapter context: concrete readers that parse markdown specs and Rust
source into the equivalence context's graph model. `MarkdownReader`
parses both concept files and context declarations; `RustReader`
parses source trees into concept nodes + declared edges.

Reading is **Conformist** to the `Reader` and `ContextReader` port
contracts — it does not negotiate the shape, it implements what
equivalence publishes.

## Owns

- adapters/markdown
- adapters/rust

## Exports (Published Language)

- MarkdownReader (PublishedLanguage)
- RustReader (PublishedLanguage)

## Imports

- Reader from equivalence (Conformist)
- ContextReader from equivalence (Conformist)
- ReaderError from equivalence (PublishedLanguage)
- Graph from equivalence (PublishedLanguage)
- ConceptNode from equivalence (PublishedLanguage)
- SignatureState from equivalence (PublishedLanguage)
- Source from equivalence (PublishedLanguage)
- Edge from equivalence (PublishedLanguage)
- EdgeKind from equivalence (PublishedLanguage)
- ContextDecl from equivalence (PublishedLanguage)
- ContextExport from equivalence (PublishedLanguage)
- ContextImport from equivalence (PublishedLanguage)
- ContextPattern from equivalence (PublishedLanguage)
- OwnedUnit from equivalence (PublishedLanguage)

## Concepts

`MarkdownReader` and `RustReader` live under `specs/concepts/core.md`.
The `Owns` block claims every concept whose code lives under
`adapters/markdown/src/` or `adapters/rust/src/` for this context.
