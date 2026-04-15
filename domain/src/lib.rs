//! Graph domain — pure types with no infrastructure dependencies.
//!
//! Models the four-level equivalence described in the repository README:
//! concept, signature, relationship, bounded context. This scaffold
//! exposes placeholder types; real schema design lands in follow-up issues.

/// Placeholder representation of a spec or code graph.
#[derive(Debug, Default)]
pub struct Graph;

/// Placeholder for equivalence violations between two graphs.
#[derive(Debug)]
pub struct Violation;
