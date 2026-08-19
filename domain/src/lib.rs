use std::path::PathBuf;

mod abstraction;
mod anchor;
mod cohesion;
mod context;
mod diff;
mod marker;
mod polarity;
mod provenance;
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
pub use marker::{
    CheckOutcome, Marker, PendingRecord, RealizedRecord, RetirementCompleteRecord,
    RetirementIncompleteRecord,
};
pub use polarity::Polarity;
pub use provenance::Provenance;
pub use report::{
    report_verb_coverage, HomonymAppearance, HomonymRecord, InvariantAnnotation, PubFnDecl,
    ReportOutput, TierHistogramRecord, TierKind, VerbCoverageRecord,
};
pub use tokens::tokenise_target;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SchemaVersion {
    V1,
    V2,
    V3,
    V4,
}

impl SchemaVersion {
    pub const CURRENT: Self = Self::V4;

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "1",
            Self::V2 => "2",
            Self::V3 => "3",
            Self::V4 => "4",
        }
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptNode {
    pub name: String,
    pub source: Source,
    pub signature: SignatureState,
    pub module_path: Option<String>,
    pub unit: Option<String>,
    pub context: Option<String>,
    pub marker: Marker,
    pub polarity: Polarity,
}

impl ConceptNode {
    #[must_use]
    pub const fn new(name: String, source: Source, signature: SignatureState) -> Self {
        Self {
            name,
            source,
            signature,
            module_path: None,
            unit: None,
            context: None,
            marker: Marker::Unmarked,
            polarity: Polarity::Declared,
        }
    }

    #[must_use]
    pub const fn with_polarity(mut self, polarity: Polarity) -> Self {
        self.polarity = polarity;
        self
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub source_concept: String,
    pub kind: EdgeKind,
    pub target: String,
    pub raw_target: String,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Implements,
    DependsOn,
    Returns,
}

impl EdgeKind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Spec { path: PathBuf, line: usize },
    Code { path: PathBuf, line: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbDecl {
    pub qname: String,
    pub owned_unit: Option<String>,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbAnchor {
    pub concept: String,
    pub qname: String,
    pub raw_target: String,
    pub source: Source,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Violation {
    MissingInCode {
        name: String,
        spec_source: Source,
    },
    MissingInSpecs {
        name: String,
        code_source: Source,
    },
    ForbiddenConceptReintroduced {
        name: String,
        spec_source: Source,
        code_source: Source,
    },
    SignatureDrift {
        name: String,
        spec_sig: String,
        code_sig: String,
        spec_source: Source,
        code_source: Source,
    },
    SignatureMissingInSpec {
        name: String,
        code_sig: String,
        code_source: Source,
    },
    SignatureUnparseable {
        name: String,
        raw: String,
        error: String,
        source: Source,
    },
    EdgeMissingInCode {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        spec_source: Source,
    },
    EdgeMissingInSpec {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        code_source: Source,
    },
    EdgeTargetUnknown {
        concept: String,
        edge_kind: EdgeKind,
        target: String,
        spec_source: Source,
    },
    Context(ContextViolation),
    VerbMissingInCode {
        concept: String,
        qname: String,
        spec_source: Source,
    },
    VerbMissingInSpec {
        qname: String,
        code_source: Source,
    },
    VerbTargetUnknown {
        concept: String,
        qname: String,
        spec_source: Source,
    },
    Cohesion(CohesionViolation),
    DanglingAnchor {
        concept: String,
        target: String,
        spec_source: Source,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_current_is_v4() {
        assert_eq!(SchemaVersion::CURRENT, SchemaVersion::V4);
        assert_eq!(SchemaVersion::CURRENT.as_str(), "4");
    }

    #[test]
    fn schema_version_wire_strings_are_stable() {
        assert_eq!(SchemaVersion::V1.as_str(), "1");
        assert_eq!(SchemaVersion::V2.as_str(), "2");
        assert_eq!(SchemaVersion::V3.as_str(), "3");
        assert_eq!(SchemaVersion::V4.as_str(), "4");
        assert_eq!(SchemaVersion::V4.to_string(), "4");
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
        assert_eq!(n.name, "Graph");
        assert_eq!(n.signature, SignatureState::Absent);
    }

    #[test]
    fn with_provenance_accepts_partial_facts() {
        let n = ConceptNode::new("X".to_owned(), code_src(), SignatureState::Absent)
            .with_provenance(None, None, Some("equivalence".to_owned()));
        assert_eq!(n.module_path, None);
        assert_eq!(n.context.as_deref(), Some("equivalence"));
    }
}
