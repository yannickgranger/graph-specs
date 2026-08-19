use crate::{Provenance, Source, Violation};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Marker {
    #[default]
    Unmarked,
    Draft,
    Retired,
}

impl Marker {
    #[must_use]
    pub const fn is_marked(self) -> bool {
        !matches!(self, Self::Unmarked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRecord {
    pub concept: String,
    pub spec_source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealizedRecord {
    pub concept: String,
    pub spec_source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementIncompleteRecord {
    pub concept: String,
    pub spec_source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementCompleteRecord {
    pub concept: String,
    pub spec_source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckOutcome {
    pub violations: Vec<Violation>,
    pub pending: Vec<PendingRecord>,
    pub realized: Vec<RealizedRecord>,
    pub retirement_incomplete: Vec<RetirementIncompleteRecord>,
    pub retirement_complete: Vec<RetirementCompleteRecord>,
    pub provenance: BTreeMap<String, Provenance>,
}

impl CheckOutcome {
    #[must_use]
    pub fn new(
        violations: Vec<Violation>,
        mut pending: Vec<PendingRecord>,
        mut realized: Vec<RealizedRecord>,
        mut retirement_incomplete: Vec<RetirementIncompleteRecord>,
        mut retirement_complete: Vec<RetirementCompleteRecord>,
    ) -> Self {
        sort_records(&mut pending);
        sort_records(&mut realized);
        sort_records(&mut retirement_incomplete);
        sort_records(&mut retirement_complete);
        Self {
            violations,
            pending,
            realized,
            retirement_incomplete,
            retirement_complete,
            provenance: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            violations: Vec::new(),
            pending: Vec::new(),
            realized: Vec::new(),
            retirement_incomplete: Vec::new(),
            retirement_complete: Vec::new(),
            provenance: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

trait MarkerRecord {
    fn sort_key(&self) -> (&str, &Path, usize);
}

macro_rules! marker_record {
    ($($t:ty),+ $(,)?) => {$(
        impl MarkerRecord for $t {
            fn sort_key(&self) -> (&str, &Path, usize) {
                record_key(&self.concept, &self.spec_source)
            }
        }
    )+};
}

marker_record!(
    PendingRecord,
    RealizedRecord,
    RetirementIncompleteRecord,
    RetirementCompleteRecord,
);

fn sort_records<T: MarkerRecord>(records: &mut [T]) {
    records.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
}

fn record_key<'a>(concept: &'a str, source: &'a Source) -> (&'a str, &'a Path, usize) {
    match source {
        Source::Spec { path, line } | Source::Code { path, line } => {
            (concept, path.as_path(), *line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec_at(path: &str, line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from(path),
            line,
        }
    }

    #[test]
    fn empty_outcome_is_clean() {
        assert!(CheckOutcome::empty().is_clean());
    }

    #[test]
    fn marker_records_never_make_an_outcome_unclean() {
        let outcome = CheckOutcome {
            pending: vec![PendingRecord {
                concept: "Digest".to_owned(),
                spec_source: spec_at("specs/concepts/execution.md", 41),
            }],
            realized: vec![RealizedRecord {
                concept: "InboundAcl".to_owned(),
                spec_source: spec_at("specs/concepts/fleet.md", 120),
            }],
            ..CheckOutcome::empty()
        };
        assert!(outcome.is_clean());
    }

    #[test]
    fn retirement_records_never_make_an_outcome_unclean() {
        let outcome = CheckOutcome {
            retirement_incomplete: vec![RetirementIncompleteRecord {
                concept: "AssertionScope".to_owned(),
                spec_source: spec_at("specs/concepts/brief_contract.md", 56),
            }],
            retirement_complete: vec![RetirementCompleteRecord {
                concept: "PrePushRebaseDecision".to_owned(),
                spec_source: spec_at("specs/concepts/agent_contract.md", 665),
            }],
            ..CheckOutcome::empty()
        };
        assert!(outcome.is_clean());
    }

    #[test]
    fn marker_is_unmarked_by_default_and_both_values_read_as_marked() {
        assert_eq!(Marker::default(), Marker::Unmarked);
        assert!(!Marker::Unmarked.is_marked());
        assert!(Marker::Draft.is_marked());
        assert!(Marker::Retired.is_marked());
    }

    #[test]
    fn pending_sorts_by_concept_then_source() {
        let records = vec![
            PendingRecord {
                concept: "Beta".to_owned(),
                spec_source: spec_at("b.md", 1),
            },
            PendingRecord {
                concept: "Alpha".to_owned(),
                spec_source: spec_at("z.md", 9),
            },
            PendingRecord {
                concept: "Alpha".to_owned(),
                spec_source: spec_at("a.md", 3),
            },
        ];
        let outcome = CheckOutcome::new(Vec::new(), records, Vec::new(), Vec::new(), Vec::new());
        let seen: Vec<_> = outcome
            .pending
            .iter()
            .map(|r| {
                let (concept, path, _) = record_key(&r.concept, &r.spec_source);
                (concept, path)
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                ("Alpha", Path::new("a.md")),
                ("Alpha", Path::new("z.md")),
                ("Beta", Path::new("b.md")),
            ]
        );
    }
}
