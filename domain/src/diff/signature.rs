//! Signature-level (v0.2) comparison on matched concept pairs.

use crate::{ConceptNode, SignatureState, Violation};

/// Compare the signature payloads on a matched (spec, code) concept pair.
/// Consumes both sides — each field is moved into the emitted violation
/// rather than cloned.
pub(super) fn compare_signatures(spec: ConceptNode, code: ConceptNode, out: &mut Vec<Violation>) {
    // Unparseable on either side surfaces first — we can't compare against
    // a broken payload, and the author needs to fix the syntax. Spec side
    // wins the race because broken spec markup is the more common cause.
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
        // No-op cases:
        //   - Both Absent → concept-only match, v0.1 semantics preserved.
        //   - Both Normalized and equal → signature match.
        //   - Absent vs Normalized (either direction) → spec has not opted
        //     into signature-level for this concept. No comparison is
        //     performed. `SignatureMissingInSpec` is reserved for v0.4
        //     strict / bounded-context mode and is not emitted in v0.2.
        _ => {}
    }
}
