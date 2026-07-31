//! The containment-provenance record rendered into NDJSON source objects
//! — RFC-010 §3.6 / #136.
//!
//! [`Provenance`] is the emitter-facing form of the agnostic triple that
//! [`crate::ConceptNode`] carries as three loose `Option` fields: the
//! diff snapshots one record per code concept (keyed by concept name)
//! into [`crate::CheckOutcome::provenance`] before the code nodes are
//! consumed, so the NDJSON emitter can render the triple inside
//! code-kind source objects without the `Violation` enum growing a
//! provenance field across every code-bearing arm.
//!
//! `context` is resolved at snapshot time from the `specs/contexts/`
//! Owns declarations — the same resolution the cohesion pass uses — not
//! re-derived in the emitter, which has no access to the declarations
//! (the split-brain #136 names).

/// The language-agnostic containment triple for one code concept, as
/// rendered into NDJSON code-kind source objects.
///
/// Pure data; every field is optional because each is independently
/// unavailable (a PHP `:Item` has no `module_path`; a tree without
/// `specs/contexts/` resolves no `context`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    /// The owning module path, crate-root-collapsed.
    pub module_path: Option<String>,
    /// The owning crate / package, relative to the code root.
    pub unit: Option<String>,
    /// The bounded context whose `specs/contexts/` Owns block owns `unit`.
    pub context: Option<String>,
}
