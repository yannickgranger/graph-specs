use ports::{Extraction, LanguageBackend, ReaderError};
use std::path::Path;

struct StubBackend;

impl LanguageBackend for StubBackend {
    fn detect(&self, code_root: &Path) -> bool {
        code_root.join("stub.marker").exists()
    }
    fn extract(&self, _code_root: &Path) -> Result<Extraction, ReaderError> {
        Ok(Extraction::default())
    }
}

#[test]
fn language_backend_contract_is_implementable() {
    let backend = StubBackend;
    assert!(!backend.detect(Path::new("/tmp")));
    let extraction = backend
        .extract(Path::new("/tmp"))
        .expect("stub extract never fails");
    assert!(extraction.concepts.is_empty());
    assert!(extraction.raw_edges.is_empty());
}
