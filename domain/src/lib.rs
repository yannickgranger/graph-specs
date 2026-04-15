//! Graph domain — pure types with no infrastructure dependencies.
//!
//! Models the four-level equivalence from the root README. This crate
//! defines only the types and pure algorithms that operate on them.
//! Infrastructure concerns (reading, parsing, I/O) live in adapter crates.

use std::path::PathBuf;

mod diff;

pub use diff::diff;

/// A graph of concepts extracted from one side of the equivalence check
/// (either a spec tree or a code tree). Two graphs are equivalent at
/// concept level iff their `nodes` carry the same set of names.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Graph {
    pub nodes: Vec<ConceptNode>,
}

/// A single named concept located at a specific source site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptNode {
    pub name: String,
    pub source: Source,
}

/// Where a concept was found — either in a spec file or a code file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Spec { path: PathBuf, line: usize },
    Code { path: PathBuf, line: usize },
}

/// A single equivalence violation between spec and code graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// Concept declared in specs but absent from code.
    MissingInCode { name: String, spec_source: Source },
    /// Concept declared in code but absent from specs.
    MissingInSpecs { name: String, code_source: Source },
}
