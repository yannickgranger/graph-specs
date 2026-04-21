//! Graph domain — pure types with no infrastructure dependencies.
//!
//! Models the four-level equivalence from the root README. This crate
//! defines only the types and pure algorithms that operate on them.
//! Infrastructure concerns (reading, parsing, I/O) live in adapter crates.

use std::path::PathBuf;

mod context;
mod diff;
mod tokens;

pub use context::{
    detect_import_cycle, CheckInput, ContextDecl, ContextExport, ContextImport, ContextPattern,
    ContextViolation, OwnedUnit,
};
pub use diff::diff;
pub use tokens::tokenise_target;

/// NDJSON wire-contract version stamped on every record emitted by
/// `graph-specs check --format=ndjson`.
///
/// Promoted from a serialization literal to a domain-owned Published
/// Language type so downstream consumers (e.g. qbot-core's
/// `compare-spec-change` pipeline) gate their parse dispatch against a
/// single typed source rather than re-typing `"1"` / `"2"` magic
/// strings per consumer.
///
/// The current production value — what every new record this build
/// emits carries — is [`SchemaVersion::CURRENT`]. [`SchemaVersion::V1`]
/// is retained for consumers reading v0.3-era fixtures during the
/// overlap window documented at `specs/ndjson-output.md` §Schema
/// evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SchemaVersion {
    V1,
    V2,
}

impl SchemaVersion {
    /// The version stamped on every record this build emits.
    pub const CURRENT: Self = Self::V2;

    /// Wire form — the exact string literal that appears in the
    /// `schema_version` JSON field.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "1",
            Self::V2 => "2",
        }
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A graph of concepts extracted from one side of the equivalence check.
///
/// Either a spec tree or a code tree. Two graphs are equivalent at
/// concept level iff their `nodes` carry the same set of names;
/// equivalent at relationship level iff their `edges` also align (see
/// [`Edge`]).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Graph {
    pub nodes: Vec<ConceptNode>,
    pub edges: Vec<Edge>,
}

impl Graph {
    #[must_use]
    pub const fn new(nodes: Vec<ConceptNode>, edges: Vec<Edge>) -> Self {
        Self { nodes, edges }
    }

    /// Alias for [`Graph::default`] — the v0.3 relationship-level dogfood
    /// wants a code-side RETURNS edge targeting a concept.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
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

/// A declared relationship between two concepts (v0.3).
///
/// Edges are *declared* — derived textually from the spec bullet lines
/// (`- implements: Foo`, `- depends on: Bar`, `- returns: Baz`) or from
/// `syn` AST nodes on the code side (`impl Trait for Type`, struct field
/// types, `pub fn` return types). No name resolution or HIR-level chain
/// following is performed.
///
/// `target` is the tokenised matching key (see [`tokenise_target`]);
/// `raw_target` preserves the original textual form for display in drift
/// messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub source_concept: String,
    pub kind: EdgeKind,
    pub target: String,
    pub raw_target: String,
    pub source: Source,
}

/// The relationship kind of an [`Edge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Spec bullet: `- implements: Foo`. Code: `impl Foo for Type`.
    Implements,
    /// Spec bullet: `- depends on: Foo`. Code: struct field type,
    /// `pub fn` parameter type.
    DependsOn,
    /// Spec bullet: `- returns: Foo`. Code: top-level `pub fn` return type.
    Returns,
}

impl EdgeKind {
    /// Wire-form label used in violation messages and fixture output.
    /// Stable across versions — changing it would break proof files.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::Implements => "IMPLEMENTS",
            Self::DependsOn => "DEPENDS_ON",
            Self::Returns => "RETURNS",
        }
    }
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_label())
    }
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
    /// Spec declares an edge (e.g. `- implements: Foo`) that the code side
    /// does not emit. The spec is claiming a relationship code does not
    /// actually have.
    EdgeMissingInCode {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        spec_source: Source,
    },
    /// Code side emits an edge that the spec does not declare. Only fires
    /// for concepts whose spec section declared at least one bullet edge
    /// (opt-in semantics — a concept with no spec bullets is not inspected
    /// at relationship level).
    EdgeMissingInSpec {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        code_source: Source,
    },
    /// Spec bullet names a target that is not present as a concept in
    /// either graph. The spec is referencing an abstraction that does not
    /// exist in this project.
    EdgeTargetUnknown {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        spec_source: Source,
    },
    /// A v0.4 bounded-context violation. Wraps the three
    /// [`ContextViolation`] variants so consumers that do not opt
    /// into context checking match one arm rather than three.
    Context(ContextViolation),
}
