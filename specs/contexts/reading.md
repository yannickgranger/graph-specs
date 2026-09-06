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
- adapters/cfdb-query
- adapters/php
- adapters/signature

## Exports (Published Language)

- MarkdownReader (PublishedLanguage)
- RustReader (PublishedLanguage)
- RustLoader (PublishedLanguage)
- RustBackend (PublishedLanguage)
- CfdbQueryReader (PublishedLanguage)
- PhpEdgeTraversal (PublishedLanguage)
- PhpAttributeReader (PublishedLanguage)
- RustSignatures (PublishedLanguage)
- PhpSignatures (PublishedLanguage)
- RustAnchorResolver (PublishedLanguage)
- CfdbAnchorResolver (PublishedLanguage)

## Imports

- DeclaredSurface from equivalence (PublishedLanguage)
- Reader from equivalence (Conformist)
- SpecReader from equivalence (Conformist)
- CodeReader from equivalence (Conformist)
- VerbAnchorReader from equivalence (Conformist)
- ConceptAnchorReader from equivalence (Conformist)
- AnnotationReader from equivalence (Conformist)
- SpecTreeReader from equivalence (Conformist)
- SpecLoader from equivalence (Conformist)
- CodeLoader from equivalence (Conformist)
- LoadedFile from equivalence (PublishedLanguage)
- SpecFileSet from equivalence (PublishedLanguage)
- CodeFileSet from equivalence (PublishedLanguage)
- CodeFacts from equivalence (Conformist)
- ContextReader from equivalence (Conformist)
- VerbReader from equivalence (Conformist)
- AnchorResolver from equivalence (Conformist)
- LanguageBackend from equivalence (Conformist)
- SignatureNormalizer from equivalence (Conformist)
- Extraction from equivalence (PublishedLanguage)
- ReaderError from equivalence (PublishedLanguage)
- Graph from equivalence (PublishedLanguage)
- ConceptNode from equivalence (PublishedLanguage)
- SignatureState from equivalence (PublishedLanguage)
- Polarity from equivalence (PublishedLanguage)
- Source from equivalence (PublishedLanguage)
- CodeLanguage from equivalence (PublishedLanguage)
- SpecFormat from equivalence (PublishedLanguage)
- Edge from equivalence (PublishedLanguage)
- EdgeKind from equivalence (PublishedLanguage)
- ContextDecl from equivalence (PublishedLanguage)
- ContextExport from equivalence (PublishedLanguage)
- ContextImport from equivalence (PublishedLanguage)
- ContextPattern from equivalence (PublishedLanguage)
- OwnedUnit from equivalence (PublishedLanguage)
- PubFnDecl from equivalence (PublishedLanguage)
- VerbAnchor from equivalence (PublishedLanguage)
- ConceptAnchor from equivalence (PublishedLanguage)
- AnchorTarget from equivalence (PublishedLanguage)
- InvariantAnnotation from equivalence (PublishedLanguage)
- TierKind from equivalence (PublishedLanguage)
- AbstractionLevel from equivalence (PublishedLanguage)
- CohesionViolation from equivalence (PublishedLanguage)
- Violation from equivalence (PublishedLanguage)

## Concepts

`MarkdownReader` and `RustReader` live under `specs/concepts/core.md`.
The `Owns` block claims every concept whose code lives under
`adapters/markdown/src/` or `adapters/rust/src/` for this context.
