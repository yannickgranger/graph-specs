# equivalence

The domain of the checker itself: the graph model, the diff engine, the
port contracts that readers implement, and the violation vocabulary that
downstream consumers observe. Types in `domain/` and `ports/` belong
here. No IO, no parser state — just values, traits, and the three-pass
(v0.3) / four-pass (v0.4) diff algorithm.

## Owns

- domain
- ports

## Exports (Published Language — what this context publishes)

- Graph (PublishedLanguage)
- ConceptNode (PublishedLanguage)
- SignatureState (PublishedLanguage)
- Source (PublishedLanguage)
- Edge (PublishedLanguage)
- EdgeKind (PublishedLanguage)
- Violation (PublishedLanguage)
- ContextViolation (PublishedLanguage)
- ContextDecl (PublishedLanguage)
- ContextExport (PublishedLanguage)
- ContextImport (PublishedLanguage)
- ContextPattern (PublishedLanguage)
- OwnedUnit (PublishedLanguage)
- CheckInput (PublishedLanguage)
- Reader (PublishedLanguage)
- ContextReader (PublishedLanguage)
- ReaderError (PublishedLanguage)

## Imports

(none — `equivalence` is a supplier context; it publishes but does not consume.)

## Concepts

See `specs/concepts/core.md` for the concept-level entries of the types
listed above. The `Owns` block is the machine-readable statement that
every concept whose code lives under `domain/src/` or `ports/src/` is a
member of this context.
