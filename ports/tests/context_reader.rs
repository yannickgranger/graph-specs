use domain::ContextDecl;
use ports::{
    AnnotationReader, ConceptAnchorReader, ContextReader, ReaderError, SpecFileSet, SpecTreeReader,
    VerbAnchorReader,
};
use std::path::PathBuf;

struct ErrStub;

impl ContextReader for ErrStub {
    fn extract_contexts(&self, _: &SpecFileSet) -> Result<Vec<ContextDecl>, ReaderError> {
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
        .extract_contexts(&SpecFileSet::new(Vec::new()))
        .expect_err("ErrStub always fails");
    assert!(matches!(err, ReaderError::IoFailed { .. }));
}

impl VerbAnchorReader for ErrStub {
    fn extract_verb_anchors(
        &self,
        _: &SpecFileSet,
    ) -> Result<Vec<domain::VerbAnchor>, ReaderError> {
        Ok(Vec::new())
    }
}

impl ConceptAnchorReader for ErrStub {
    fn extract_concept_anchors(
        &self,
        _: &SpecFileSet,
    ) -> Result<(Vec<domain::ConceptAnchor>, Vec<domain::Violation>), ReaderError> {
        Ok((Vec::new(), Vec::new()))
    }
}

impl AnnotationReader for ErrStub {
    fn extract_annotations(
        &self,
        _: &SpecFileSet,
    ) -> Result<Vec<domain::InvariantAnnotation>, ReaderError> {
        Ok(Vec::new())
    }
}

impl SpecTreeReader for ErrStub {
    fn extract_spec_trees(&self, _: &SpecFileSet) -> Result<Vec<domain::SpecTree>, ReaderError> {
        Ok(Vec::new())
    }
}

#[test]
fn the_four_spec_capability_ports_are_object_safe() {
    let empty = SpecFileSet::new(Vec::new());
    let a: Box<dyn VerbAnchorReader> = Box::new(ErrStub);
    let b: Box<dyn ConceptAnchorReader> = Box::new(ErrStub);
    let c: Box<dyn AnnotationReader> = Box::new(ErrStub);
    let d: Box<dyn SpecTreeReader> = Box::new(ErrStub);
    assert!(a.extract_verb_anchors(&empty).unwrap().is_empty());
    let (anchors, findings) = b.extract_concept_anchors(&empty).unwrap();
    assert!(anchors.is_empty() && findings.is_empty());
    assert!(c.extract_annotations(&empty).unwrap().is_empty());
    assert!(d.extract_spec_trees(&empty).unwrap().is_empty());
}
