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
///
/// `signature` carries the optional signature-level payload (v0.2): the
/// normalized form of the pub item's declaration (for code) or of the
/// fenced `rust` block inside the concept section (for specs). Left as
/// [`SignatureState::Absent`] when the reader has no signature data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptNode {
    pub name: String,
    pub source: Source,
    pub signature: SignatureState,
}

/// The signature-level payload on a [`ConceptNode`].
///
/// - `Absent` — reader did not produce a signature (legacy concept-only mode).
/// - `Normalized(s)` — the reader parsed a `syn::Item` and rendered its
///   normalised token stream as `s`. Two concepts match at signature level
///   iff their `Normalized` strings are byte-equal.
/// - `Unparseable { raw, error }` — a spec-side fenced `rust` block failed
///   to parse. Surfaced separately from drift because the cause is a typo
///   in prose, not a drift between sides.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SignatureState {
    #[default]
    Absent,
    Normalized(String),
    Unparseable {
        raw: String,
        error: String,
    },
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
    /// Both sides declare the concept with a signature, but the signatures
    /// disagree after normalisation.
    SignatureDrift {
        name: String,
        spec_sig: String,
        code_sig: String,
        spec_source: Source,
        code_source: Source,
    },
    /// Code side has a signature for the concept; spec side has the concept
    /// heading but no fenced `rust` block. Soft warning — the spec file is
    /// under-specified, not drifted.
    SignatureMissingInSpec {
        name: String,
        code_sig: String,
        code_source: Source,
    },
    /// A spec fenced `rust` block did not parse via `syn`. The concept is
    /// dropped from signature-level comparison until the spec is fixed.
    SignatureUnparseable {
        name: String,
        raw: String,
        error: String,
        source: Source,
    },
}
