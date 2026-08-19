use crate::{ConceptNode, SignatureState, Violation};

pub(super) fn compare_signatures(spec: ConceptNode, code: ConceptNode, out: &mut Vec<Violation>) {
    if matches!(spec.signature, SignatureState::Unparseable { .. }) {
        if let SignatureState::Unparseable { raw, error } = spec.signature {
            out.push(Violation::SignatureUnparseable {
                name: spec.name,
                raw,
                error,
                source: spec.source,
            });
        }
        return;
    }
    if matches!(code.signature, SignatureState::Unparseable { .. }) {
        if let SignatureState::Unparseable { raw, error } = code.signature {
            out.push(Violation::SignatureUnparseable {
                name: code.name,
                raw,
                error,
                source: code.source,
            });
        }
        return;
    }

    match (spec.signature, code.signature) {
        (SignatureState::Normalized(spec_sig), SignatureState::Normalized(code_sig))
            if spec_sig != code_sig =>
        {
            out.push(Violation::SignatureDrift {
                name: spec.name,
                spec_sig,
                code_sig,
                spec_source: spec.source,
                code_source: code.source,
            });
        }
        _ => {}
    }
}
