//! Integration test for the [`ContextReader`] port contract.
//!
//! The port is a pure contract — no production adapter ships in this
//! slice (markdown implements `ContextReader` in #24). The stub here
//! exists only to exercise the trait's shape: can it be implemented,
//! is it object-safe, does the return type thread a `Vec<ContextDecl>`
//! without allocation surprises?
//!
//! Rationale per RFC-001 round-1 clean-arch lens: every port addition
//! must be exercised by at least one implementer proof-of-life, even
//! when no production adapter lands in the same slice.

use domain::ContextDecl;
use ports::{ContextReader, ReaderError};
use std::path::Path;

/// Test-only no-op implementor. Returns an empty context list regardless
/// of input — proves the trait's call signature and return type compile.
struct NoOpStub;

impl ContextReader for NoOpStub {
    fn extract_contexts(&self, _root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        Ok(Vec::new())
    }
}

#[test]
fn context_reader_contract_is_implementable() {
    let r = NoOpStub;
    let result = r.extract_contexts(Path::new("."));
    let contexts = result.expect("NoOpStub cannot fail");
    assert!(contexts.is_empty());
}

#[test]
fn context_reader_is_object_safe() {
    // Trait-object ergonomics — callers holding the reader behind a
    // Box / Arc must not hit a "not object-safe" compile error.
    let r: Box<dyn ContextReader> = Box::new(NoOpStub);
    let _: Result<Vec<ContextDecl>, ReaderError> = r.extract_contexts(Path::new("."));
}
