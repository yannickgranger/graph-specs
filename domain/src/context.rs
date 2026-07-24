//! Bounded-context equivalence types — v0.4 per RFC-001.
//!
//! This module introduces the vocabulary for declaring bounded contexts,
//! their `Owns` / `Exports` / `Imports` surfaces, and the violation
//! variants emitted by the v0.4 diff context pass (landing in issue #25).
//!
//! The types are pure data — no diff algorithm here. The context pass
//! lives alongside the three existing passes in `diff.rs` and consumes
//! [`CheckInput`] as its spec-side argument.

mod check_input;
mod cycle;
mod decl;
mod resolve;
mod violation;

pub use check_input::CheckInput;
pub use cycle::detect_import_cycle;
pub use decl::{ContextDecl, ContextExport, ContextImport, ContextPattern, OwnedUnit};
pub use resolve::{context_for_concept, resolve_declared_context};
pub use violation::ContextViolation;
