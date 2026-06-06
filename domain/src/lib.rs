//! Graph domain — pure types with no infrastructure dependencies.
//!
//! Models the four-level equivalence from the root README. This crate
//! defines only the types and pure algorithms that operate on them.
//! Infrastructure concerns (reading, parsing, I/O) live in adapter crates.

use std::path::PathBuf;

mod abstraction;
mod anchor;
mod cohesion;
mod context;
mod diff;
mod report;
mod tokens;

pub use abstraction::AbstractionLevel;
pub use anchor::{
    anchor_violation, behavioral_exemption_applies, AnchorKind, AnchorTarget, ConceptAnchor,
    ResolvedAnchor,
};
pub use cohesion::CohesionViolation;
pub use context::{
    context_for_concept, detect_import_cycle, resolve_declared_context, CheckInput, ContextDecl,
    ContextExport, ContextImport, ContextPattern, ContextViolation, OwnedUnit,
};
pub use diff::diff;
pub use report::{
    report_verb_coverage, HomonymAppearance, HomonymRecord, InvariantAnnotation, PubFnDecl,
    ReportOutput, TierHistogramRecord, TierKind, VerbCoverageRecord,
};
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
    /// v3 (RFC-010 §3.6) — versions the abstraction-ladder `Cohesion`
    /// record kinds (`context_without_cohesion_unit`, `sub_concept_orphan`,
    /// `concept_context_mismatch`) added by R10-3. Consumers dispatch on
    /// `"3"`; the qbot-core `compare-spec-change` v3 arm lockstep is tracked
    /// separately (OQ-3). Provenance source fields are a planned additive
    /// (non-breaking) extension that will NOT bump the version again.
    V3,
}

impl SchemaVersion {
    /// The version stamped on every record this build emits.
    pub const CURRENT: Self = Self::V3;

    /// Wire form — the exact string literal that appears in the
    /// `schema_version` JSON field.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "1",
            Self::V2 => "2",
            Self::V3 => "3",
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
    /// Language-agnostic containment provenance (RFC-010 §3.3). Populated
    /// by a `CodeFacts`-style adapter (source-walk in R10-3, cfdb-query
    /// ACL in R10-6); `None` on the spec side and wherever no code facts
    /// are available. Deliberately **not** cfdb's Rust-specific prop names
    /// (`module_qpath` / `crate` / `bounded_context`) — PHP `:Item` carries
    /// no such props, so the agnostic triple is translated by the ACL.
    ///
    /// `module_path` — the owning module path, crate-root-collapsed.
    pub module_path: Option<String>,
    /// `unit` — the owning crate / package, relative to the code root.
    pub unit: Option<String>,
    /// `context` — the resolved bounded context (`specs/contexts/` Owns or
    /// the cfdb-query ACL), used by the R10-3 cohesion pass.
    pub context: Option<String>,
}

impl ConceptNode {
    /// Construct a concept node with **no** containment provenance — the
    /// spec-side / no-code-facts case. The three provenance fields default
    /// to `None`; populate them with [`ConceptNode::with_provenance`].
    ///
    /// This is an explicit "provenance unknown" constructor, **not** a
    /// `Default` escape: every site decides provenance consciously (RFC-010
    /// §3.7 — `ConceptNode` does not derive `Default`).
    #[must_use]
    pub const fn new(name: String, source: Source, signature: SignatureState) -> Self {
        Self {
            name,
            source,
            signature,
            module_path: None,
            unit: None,
            context: None,
        }
    }

    /// Builder: attach the language-agnostic containment triple
    /// (`module_path` / `unit` / `context`) derived by a code-facts
    /// adapter. Used by the source-walk adapter (R10-3) and the cfdb-query
    /// ACL (R10-6); the R10-1 round-trip test exercises it directly.
    #[must_use]
    pub fn with_provenance(
        mut self,
        module_path: Option<String>,
        unit: Option<String>,
        context: Option<String>,
    ) -> Self {
        self.module_path = module_path;
        self.unit = unit;
        self.context = context;
        self
    }
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

/// A top-level `pub fn` declaration found in code, with its owning crate
/// and the location of its declaration site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbDecl {
    pub qname: String,
    pub owned_unit: Option<String>,
    pub source: Source,
}

/// Spec-side anchor parsed from a `- verb: <ident>` bullet inside a
/// concept section. `concept` names the owning concept; `qname` is the
/// bare identifier; `raw_target` preserves the verbatim bullet text.
///
/// `concept` and `source` are placeholder-initialised by
/// `parse_verb_bullet` and filled in by the caller (`finish_bullet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbAnchor {
    pub concept: String,
    pub qname: String,
    pub raw_target: String,
    pub source: Source,
}

/// Aggregates both sides of the verb-anchoring contract carried by [`CheckInput`].
///
/// `decls` are `pub fn` declarations from code; `anchors` are `- verb:` bullets
/// from spec files. [`Default`] is derived so `CheckInput`'s `#[derive(Default)]`
/// compiles.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VerbOwnership {
    pub decls: Vec<VerbDecl>,
    pub anchors: Vec<VerbAnchor>,
}

impl From<PubFnDecl> for VerbDecl {
    fn from(f: PubFnDecl) -> Self {
        Self {
            qname: f.name,
            owned_unit: f.owned_unit,
            source: f.source,
        }
    }
}

/// A single equivalence violation between spec and code graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Violation {
    /// Concept declared in specs but absent from code.
    MissingInCode { name: String, spec_source: Source },
    /// Concept declared in code but absent from specs.
    MissingInSpecs { name: String, code_source: Source },
    /// A `pub` code item whose name matches a heading living in a
    /// `status: draft` spec. The draft imposes no code-existence
    /// obligation, but implementing it while the spec is still draft
    /// leaves the item with no active owning heading. Distinct from
    /// [`Violation::MissingInSpecs`] (no heading exists anywhere) —
    /// here a heading exists but is draft. `draft_source` points at
    /// the draft heading. (per specs/dialect.md ## Draft specs)
    ImplementsDraftConcept { name: String, draft_source: Source },
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
    /// Spec anchor `- verb: <qname>` found under `concept` but no
    /// `pub fn` named `qname` exists anywhere in the code tree.
    VerbMissingInCode {
        concept: String,
        qname: String,
        spec_source: Source,
    },
    /// A `pub fn` named `qname` exists in a verb-anchored context but
    /// no spec anchor claims it.
    VerbMissingInSpec { qname: String, code_source: Source },
    /// Spec anchor references `qname` but no `pub fn` with that name
    /// belongs to any declared bounded context.
    VerbTargetUnknown {
        concept: String,
        qname: String,
        spec_source: Source,
    },
    /// A v0.6 abstraction-ladder cohesion violation (RFC-010 §3.5). Wraps
    /// the three [`CohesionViolation`] variants so consumers that do not
    /// opt into cohesion checking match one arm rather than three —
    /// distinct from [`Violation::Context`] (RFC-001 cross-context edges).
    Cohesion(CohesionViolation),
    /// A v0.7 spec anchor (`- impl: <qname>`) names a code item that does
    /// not exist anywhere in the code tree (RFC-012 §3.5). The
    /// equivalence-defect analog of [`Violation::MissingInCode`] for an
    /// anchored concept — kept a **top-level** arm (not nested in
    /// [`Violation::Cohesion`]) so a consumer that opts out of cohesion
    /// checking cannot silently suppress broken-anchor detection.
    /// `spec_source` points at the anchor bullet for `path:line`.
    DanglingAnchor {
        concept: String,
        target: String,
        spec_source: Source,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- RFC-010 §3.6 / R10-4 NDJSON schema v3 tripwire ---

    #[test]
    fn schema_version_current_is_v3() {
        // Tripwire: the production wire version is `"3"`. A change here is a
        // breaking NDJSON contract change and MUST be paired with a consumer
        // lockstep (OQ-3) + a `specs/ndjson-output.md` update.
        assert_eq!(SchemaVersion::CURRENT, SchemaVersion::V3);
        assert_eq!(SchemaVersion::CURRENT.as_str(), "3");
    }

    #[test]
    fn schema_version_wire_strings_are_stable() {
        assert_eq!(SchemaVersion::V1.as_str(), "1");
        assert_eq!(SchemaVersion::V2.as_str(), "2");
        assert_eq!(SchemaVersion::V3.as_str(), "3");
        assert_eq!(SchemaVersion::V3.to_string(), "3");
    }

    fn code_src() -> Source {
        Source::Code {
            path: PathBuf::from("domain/src/lib.rs"),
            line: 101,
        }
    }

    #[test]
    fn new_leaves_provenance_unset() {
        let n = ConceptNode::new("ConceptNode".to_owned(), code_src(), SignatureState::Absent);
        assert_eq!(n.name, "ConceptNode");
        assert_eq!(n.module_path, None);
        assert_eq!(n.unit, None);
        assert_eq!(n.context, None);
    }

    #[test]
    fn with_provenance_round_trips_the_agnostic_triple() {
        let n = ConceptNode::new("Graph".to_owned(), code_src(), SignatureState::Absent)
            .with_provenance(
                Some("domain".to_owned()),
                Some("domain".to_owned()),
                Some("equivalence".to_owned()),
            );
        assert_eq!(n.module_path.as_deref(), Some("domain"));
        assert_eq!(n.unit.as_deref(), Some("domain"));
        assert_eq!(n.context.as_deref(), Some("equivalence"));
        // name / source / signature are preserved through the builder.
        assert_eq!(n.name, "Graph");
        assert_eq!(n.signature, SignatureState::Absent);
    }

    #[test]
    fn with_provenance_accepts_partial_facts() {
        // PHP `:Item` may yield context via edge-traversal but no module_path.
        let n = ConceptNode::new("X".to_owned(), code_src(), SignatureState::Absent)
            .with_provenance(None, None, Some("equivalence".to_owned()));
        assert_eq!(n.module_path, None);
        assert_eq!(n.context.as_deref(), Some("equivalence"));
    }
}
