use domain::ContextDecl;
use ports::{ContextReader, ReaderError};
use std::path::{Path, PathBuf};

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
