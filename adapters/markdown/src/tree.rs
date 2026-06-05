//! Heading-tree assembler — RFC-010 §3.2 / R10-2.
//!
//! A **separate pass** from the concept reader's `handle_event` /
//! `SectionState` (which is at the cognitive-complexity ceiling): this
//! module runs its own fresh `pulldown-cmark` parser over a spec file and
//! assembles the *abstraction ladder* as a parent-linked tree —
//! `H1 → Context`, `H2 → Concept`, `H3 → SubConcept`, `H4+ → Member`
//! ([`domain::AbstractionLevel`]).
//!
//! The tree makes heading **depth** load-bearing. Two structural cohesion
//! defects fall straight out of it, both spec-side (zero code facts):
//!
//! - an H1 context with no H2/H3 concept under it
//!   → [`CohesionViolation::ContextWithoutCohesionUnit`];
//! - an H3 sub-concept with no enclosing H2
//!   → [`CohesionViolation::SubConceptOrphan`].
//!
//! A descriptive H1 that does not normalise to a context identifier
//! (RFC-010 §3.2) is rejected as a [`ReaderError::ParseFailed`].
//!
//! Wiring these into the `check` diff + NDJSON emitter (so they
//! participate in the non-zero exit path) is R10-3/R10-4; this slice owns
//! the assembler, the normalisation rule (shared with [`crate::contexts`]
//! via [`normalize_context_id`]), and orphan detection.

use crate::markdown_utils::{
    compute_line_starts, is_context_identifier, line_of_offset, normalize_context_id,
    path_under_dir,
};
use domain::{AbstractionLevel, CohesionViolation};
use ports::ReaderError;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};

/// One node of the assembled abstraction ladder — a single heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNode {
    /// The rung this heading sits on, derived from its depth.
    pub level: AbstractionLevel,
    /// The heading text, trimmed (generics are **not** stripped here —
    /// that is the concept reader's `normalize_heading` concern).
    pub text: String,
    /// For a `Context` (H1) node, the normalised context identifier
    /// (`# AC verifier` → `ac-verifier`); `None` for every deeper rung.
    pub id: Option<String>,
    /// 1-based line of the heading in the source file.
    pub line: usize,
    /// Index into [`SpecTree::nodes`] of the enclosing heading one rung up
    /// (Concept→Context, SubConcept→Concept, Member→SubConcept), or `None`
    /// when no such parent exists. A `SubConcept` with `parent == None` is
    /// an orphan.
    pub parent: Option<usize>,
}

/// The assembled heading tree for a single spec file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecTree {
    /// The file the tree was assembled from.
    pub file: PathBuf,
    /// Heading nodes in document order; `parent` indices refer back into
    /// this vector.
    pub nodes: Vec<HeadingNode>,
}

impl SpecTree {
    /// The file's single bounded-context identifier — the `id` of the
    /// first `Context` (H1) node, if any.
    #[must_use]
    pub fn context_id(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find(|n| n.level == AbstractionLevel::Context)
            .and_then(|n| n.id.as_deref())
    }

    /// The spec-side cohesion violations the tree's shape reveals
    /// (RFC-010 §3.5): every `Context` with no concept under it, and every
    /// orphaned `SubConcept`. Detection only — emission into the `check`
    /// diff is R10-3.
    #[must_use]
    pub fn cohesion_violations(&self) -> Vec<CohesionViolation> {
        let mut out = Vec::new();
        for (idx, node) in self.nodes.iter().enumerate() {
            match node.level {
                AbstractionLevel::Context => {
                    if !self.has_cohesion_unit(idx) {
                        out.push(CohesionViolation::ContextWithoutCohesionUnit {
                            context: node.id.clone().unwrap_or_else(|| node.text.clone()),
                            file: self.file.clone(),
                        });
                    }
                }
                AbstractionLevel::SubConcept if node.parent.is_none() => {
                    out.push(CohesionViolation::SubConceptOrphan {
                        sub_concept: node.text.clone(),
                        file: self.file.clone(),
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// Does the `Context` node at `ctx_idx` have at least one `Concept` or
    /// `SubConcept` under it (directly, or — for an orphan — anywhere in
    /// its span before the next `Context`)?
    fn has_cohesion_unit(&self, ctx_idx: usize) -> bool {
        self.nodes[ctx_idx + 1..]
            .iter()
            .take_while(|n| n.level != AbstractionLevel::Context)
            .any(|n| {
                matches!(
                    n.level,
                    AbstractionLevel::Concept | AbstractionLevel::SubConcept
                )
            })
    }
}

/// Running parent pointers while assembling a single file's tree. Held in
/// a struct so the event loop stays a flat dispatch under the
/// cognitive-complexity ceiling.
#[derive(Default)]
struct Pointers {
    context: Option<usize>,
    concept: Option<usize>,
    sub_concept: Option<usize>,
}

impl Pointers {
    /// The parent index for a freshly-seen heading at `level`, updating the
    /// running pointers so the next heading resolves against this one.
    ///
    /// Driven by `==` comparisons rather than an exhaustive `match`, so the
    /// `#[non_exhaustive]` [`AbstractionLevel`] never forces a dead wildcard
    /// arm in this adapter (RFC-010 §3.1 / dry-run §12-E). The trailing
    /// branch is the `Member` (H4+) rung: it parents onto the nearest
    /// sub-concept, falling back to the concept.
    fn link(&mut self, level: AbstractionLevel, idx: usize) -> Option<usize> {
        if level == AbstractionLevel::Context {
            self.context = Some(idx);
            self.concept = None;
            self.sub_concept = None;
            None
        } else if level == AbstractionLevel::Concept {
            self.concept = Some(idx);
            self.sub_concept = None;
            self.context
        } else if level == AbstractionLevel::SubConcept {
            self.sub_concept = Some(idx);
            self.concept
        } else {
            self.sub_concept.or(self.concept)
        }
    }
}

/// Map a `pulldown-cmark` heading level to its 1-based markdown depth.
const fn heading_depth(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Per-file assembly state.
struct AssemblerState<'a> {
    file: &'a Path,
    line_starts: Vec<usize>,
    nodes: Vec<HeadingNode>,
    pointers: Pointers,
    in_heading: Option<(u8, usize)>,
    heading_buf: String,
    error: Option<ReaderError>,
}

impl<'a> AssemblerState<'a> {
    fn new(source: &str, file: &'a Path) -> Self {
        Self {
            file,
            line_starts: compute_line_starts(source),
            nodes: Vec::new(),
            pointers: Pointers::default(),
            in_heading: None,
            heading_buf: String::new(),
            error: None,
        }
    }

    /// Finalise a heading whose text has been collected: validate an H1's
    /// identifier shape, compute its parent link, and push the node.
    fn push_heading(&mut self, depth: u8, line: usize) {
        let level = AbstractionLevel::from_heading_depth(depth);
        let text = self.heading_buf.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let id = if level == AbstractionLevel::Context {
            let normalized = normalize_context_id(&text);
            if !is_context_identifier(&normalized) {
                self.error = Some(ReaderError::ParseFailed {
                    path: self.file.to_path_buf(),
                    line,
                    message: format!(
                        "H1 `{text}` is a descriptive title, not a context identifier \
                         (normalises to `{normalized}`)"
                    ),
                });
                return;
            }
            Some(normalized)
        } else {
            None
        };
        let idx = self.nodes.len();
        let parent = self.pointers.link(level, idx);
        self.nodes.push(HeadingNode {
            level,
            text,
            id,
            line,
            parent,
        });
    }
}

/// Assemble the abstraction-ladder tree for a single markdown spec source.
///
/// # Errors
///
/// Returns [`ReaderError::ParseFailed`] when an H1 heading does not
/// normalise to a context identifier (RFC-010 §3.2 descriptive-title
/// rejection).
pub fn assemble_tree(source: &str, file: &Path) -> Result<SpecTree, ReaderError> {
    let mut st = AssemblerState::new(source, file);
    for (event, range) in Parser::new(source).into_offset_iter() {
        handle_event(&mut st, &event, range);
        if st.error.is_some() {
            break;
        }
    }
    if let Some(err) = st.error {
        return Err(err);
    }
    Ok(SpecTree {
        file: file.to_path_buf(),
        nodes: st.nodes,
    })
}

fn handle_event(st: &mut AssemblerState, event: &Event, range: std::ops::Range<usize>) {
    match event {
        Event::Start(Tag::Heading { level, .. }) => {
            st.in_heading = Some((
                heading_depth(*level),
                line_of_offset(&st.line_starts, range.start),
            ));
            st.heading_buf.clear();
        }
        Event::End(TagEnd::Heading(_)) => {
            if let Some((depth, line)) = st.in_heading.take() {
                st.push_heading(depth, line);
            }
        }
        Event::Text(s) | Event::Code(s) if st.in_heading.is_some() => {
            st.heading_buf.push_str(s);
        }
        _ => {}
    }
}

/// Walk `root` and assemble a [`SpecTree`] for every non-draft concept
/// spec file.
///
/// Selects the `concepts/` subdir when present, else `root`, mirroring the
/// file-selection rules of [`crate::MarkdownReader::extract`]: `*.md` only,
/// `contexts/` excluded (different dialect), drafts skipped.
///
/// # Errors
///
/// Returns [`ReaderError::WalkFailed`] / [`ReaderError::IoFailed`] on I/O
/// failures, or [`ReaderError::ParseFailed`] when a file's H1 is a
/// descriptive title rather than a context identifier.
pub fn assemble_spec_trees(root: &Path) -> Result<Vec<SpecTree>, ReaderError> {
    let concepts_subdir = root.join("concepts");
    let walk_root: &Path = if concepts_subdir.is_dir() {
        concepts_subdir.as_path()
    } else {
        root
    };

    let mut trees = Vec::new();
    for entry in walkdir::WalkDir::new(walk_root).sort_by_file_name() {
        let entry = entry.map_err(|e| ReaderError::WalkFailed {
            root: root.to_path_buf(),
            cause: e.to_string(),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") || path_under_dir(path, "contexts") {
            continue;
        }
        let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
            path: path.to_path_buf(),
            cause: e.to_string(),
        })?;
        if crate::is_draft(&source) {
            continue;
        }
        trees.push(assemble_tree(&source, path)?);
    }
    Ok(trees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::parse_context_file;

    fn tree(source: &str) -> SpecTree {
        assemble_tree(source, Path::new("specs/concepts/equivalence.md")).expect("well-formed tree")
    }

    #[test]
    fn nested_headings_map_to_each_rung_with_parent_links() {
        let t = tree("# equivalence\n\n## Graph\n\n### Inner\n\n#### field\n");
        let levels: Vec<_> = t.nodes.iter().map(|n| n.level).collect();
        assert_eq!(
            levels,
            vec![
                AbstractionLevel::Context,
                AbstractionLevel::Concept,
                AbstractionLevel::SubConcept,
                AbstractionLevel::Member,
            ]
        );
        // Context → no parent; each deeper rung points one rung up.
        assert_eq!(t.nodes[0].parent, None);
        assert_eq!(t.nodes[1].parent, Some(0));
        assert_eq!(t.nodes[2].parent, Some(1));
        assert_eq!(t.nodes[3].parent, Some(2));
        assert_eq!(t.context_id(), Some("equivalence"));
        assert!(t.cohesion_violations().is_empty());
    }

    #[test]
    fn h1_with_no_concept_is_context_without_cohesion_unit() {
        let t = tree("# equivalence\n\nprose only, no H2.\n");
        assert_eq!(
            t.cohesion_violations(),
            vec![CohesionViolation::ContextWithoutCohesionUnit {
                context: "equivalence".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }]
        );
    }

    #[test]
    fn h1_to_h3_depth_skip_is_a_subconcept_orphan() {
        // H3 with no enclosing H2 under its context.
        let t = tree("# equivalence\n\n### Orphaned\n");
        let v = t.cohesion_violations();
        assert!(
            v.contains(&CohesionViolation::SubConceptOrphan {
                sub_concept: "Orphaned".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }),
            "expected SubConceptOrphan, got {v:?}"
        );
        // The H3 is its context's cohesion unit, so no ContextWithoutCohesionUnit.
        assert!(!v
            .iter()
            .any(|x| matches!(x, CohesionViolation::ContextWithoutCohesionUnit { .. })));
        assert_eq!(t.nodes[1].parent, None, "orphan H3 has no parent");
    }

    #[test]
    fn enclosed_h3_is_not_an_orphan() {
        let t = tree("# equivalence\n\n## Graph\n\n### Inner\n");
        assert!(t.cohesion_violations().is_empty());
        assert_eq!(t.nodes[2].parent, Some(1));
    }

    #[test]
    fn descriptive_h1_is_a_reader_error() {
        let err = assemble_tree(
            "# Core concepts: the equivalence layer\n\n## Graph\n",
            Path::new("specs/concepts/core.md"),
        )
        .expect_err("descriptive H1 must be rejected");
        assert!(
            matches!(err, ReaderError::ParseFailed { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn normalization_matches_on_both_sides() {
        // The one rule (RFC-010 §3.2) applied to the concepts-side H1 …
        let concepts = tree("# AC verifier\n\n## Foo\n");
        assert_eq!(concepts.context_id(), Some("ac-verifier"));
        // … and to the contexts-side H1 yields the same identifier.
        let decl = parse_context_file(
            Path::new("specs/contexts/ac-verifier.md"),
            "# AC verifier\n\n## Owns\n\n- foo\n",
        )
        .expect("well-formed context file");
        assert_eq!(decl.name, "ac-verifier");
        assert_eq!(concepts.context_id(), Some(decl.name.as_str()));
    }

    #[test]
    fn member_falls_back_to_concept_when_no_subconcept() {
        // H4 directly under H2 (no H3) parents onto the concept.
        let t = tree("# equivalence\n\n## Graph\n\n#### member\n");
        assert_eq!(t.nodes[2].level, AbstractionLevel::Member);
        assert_eq!(t.nodes[2].parent, Some(1));
    }

    #[test]
    fn self_dogfood_concept_specs_have_one_context_and_no_cohesion_violations() {
        // Real-infra dogfood: assemble this repo's own `specs/concepts/`
        // and assert the post-R10-1 split is ladder-clean.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let specs = Path::new(manifest)
            .join("../../specs/concepts")
            .canonicalize()
            .expect("specs/concepts exists");
        let trees = assemble_spec_trees(&specs).expect("assemble self specs");
        assert!(!trees.is_empty(), "expected concept spec files");
        for t in &trees {
            let contexts = t
                .nodes
                .iter()
                .filter(|n| n.level == AbstractionLevel::Context)
                .count();
            assert_eq!(
                contexts,
                1,
                "{} must declare exactly one Context H1",
                t.file.display()
            );
            assert!(
                t.cohesion_violations().is_empty(),
                "{} has cohesion violations: {:?}",
                t.file.display(),
                t.cohesion_violations()
            );
        }
    }
}
