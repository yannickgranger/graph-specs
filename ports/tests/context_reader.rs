//! Compile-time proof that [`ContextReader`] is implementable and
//! object-safe. No production stub ships — #24 lands the markdown impl.

use domain::ContextDecl;
use ports::{ContextReader, ReaderError};
use std::path::{Path, PathBuf};

/// Returns `Err` deliberately — a double that can't fail proves nothing
/// about error propagation (CLAUDE.md §6 rule 3).
struct ErrStub;

impl ContextReader for ErrStub {
    fn extract_contexts(&self, _: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        Err(ReaderError::IoFailed {
            path: PathBuf::from("<compile-proof>"),
            cause: "compile proof — not a real reader".into(),
        })
    }
}

#[test]
fn context_reader_contract_is_implementable_and_object_safe() {
    let r: Box<dyn ContextReader> = Box::new(ErrStub);
    let err = r
        .extract_contexts(Path::new("."))
        .expect_err("ErrStub always fails");
    assert!(matches!(err, ReaderError::IoFailed { .. }));
}
