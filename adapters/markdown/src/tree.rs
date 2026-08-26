use crate::grounding::read;
use crate::markdown_utils::{normalize_context_id, path_under_dir};
use domain::{behavioral_exemption_applies, AbstractionLevel, CohesionViolation};
use ports::ReaderError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNode {
    pub level: AbstractionLevel,
    pub text: String,
    pub id: Option<String>,
    pub line: usize,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecTree {
    pub file: PathBuf,
    pub nodes: Vec<HeadingNode>,
    pub behavioral: bool,
    pub has_substance: bool,
}

impl SpecTree {
    #[must_use]
    pub fn context_id(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find(|n| n.level == AbstractionLevel::Context)
            .and_then(|n| n.id.as_deref())
    }

    #[must_use]
    pub fn concept_declarations(&self) -> Vec<(&str, &str)> {
        let Some(ctx) = self.context_id() else {
            return Vec::new();
        };
        self.nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.level,
                    AbstractionLevel::Concept | AbstractionLevel::SubConcept
                )
            })
            .map(|n| (n.text.as_str(), ctx))
            .collect()
    }

    #[must_use]
    pub fn cohesion_violations(&self) -> Vec<CohesionViolation> {
        let context_exempt = behavioral_exemption_applies(self.behavioral, self.has_substance);
        let mut out = Vec::new();
        for (idx, node) in self.nodes.iter().enumerate() {
            match node.level {
                AbstractionLevel::Context => {
                    if !self.has_cohesion_unit(idx) && !context_exempt {
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

#[derive(Default)]
struct Pointers {
    context: Option<usize>,
    concept: Option<usize>,
    sub_concept: Option<usize>,
}

impl Pointers {
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

pub fn assemble_tree(source: &str, file: &Path) -> Result<SpecTree, ReaderError> {
    let dialect = read(file, source)?;
    let mut pointers = Pointers::default();
    let mut nodes: Vec<HeadingNode> = Vec::new();
    for rung in &dialect.ladder {
        if rung.name.is_empty() {
            continue;
        }
        let depth = u8::try_from(rung.level).unwrap_or(u8::MAX);
        let level = AbstractionLevel::from_heading_depth(depth);
        let id = (level == AbstractionLevel::Context).then(|| normalize_context_id(&rung.name));
        let idx = nodes.len();
        let parent = pointers.link(level, idx);
        nodes.push(HeadingNode {
            level,
            text: rung.name.clone(),
            id,
            line: rung.line,
            parent,
        });
    }
    Ok(SpecTree {
        file: file.to_path_buf(),
        nodes,
        behavioral: crate::is_behavioral_context(source),
        has_substance: crate::has_behavioral_substance(source),
    })
}

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
        trees.push(assemble_tree(&source, path)?);
    }
    Ok(trees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::parse_context_file;

    fn tree(source: &str) -> SpecTree {
        assemble_tree(source, Path::new("specs/concepts/equivalence.md"))
            .expect("well-formed dialect")
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
        let t = tree("# equivalence\n\n### Orphaned\n");
        let v = t.cohesion_violations();
        assert!(
            v.contains(&CohesionViolation::SubConceptOrphan {
                sub_concept: "Orphaned".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }),
            "expected SubConceptOrphan, got {v:?}"
        );
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
    fn a_descriptive_h1_is_prose_and_costs_the_file_nothing() {
        let t = assemble_tree(
            "# Core concepts: the equivalence layer\n\n## Graph\n",
            Path::new("specs/concepts/core.md"),
        )
        .expect("well-formed dialect");
        assert_eq!(
            t.nodes.iter().map(|n| n.level).collect::<Vec<_>>(),
            vec![AbstractionLevel::Context, AbstractionLevel::Concept],
            "the H1 is prose; the concept under it still stands"
        );
        assert!(t.cohesion_violations().is_empty());
    }

    #[test]
    fn a_setext_heading_is_no_cohesion_unit() {
        let t = tree("# widgets\n\nWidget\n------\n\nProse.\n");
        assert_eq!(
            t.cohesion_violations(),
            vec![CohesionViolation::ContextWithoutCohesionUnit {
                context: "widgets".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }],
            "the ladder and the concept read are one list; a setext heading is in neither"
        );
    }

    #[test]
    fn an_indented_hash_run_is_no_cohesion_unit() {
        let t = tree("# widgets\n\n  ## Widget\n");
        assert_eq!(
            t.cohesion_violations(),
            vec![CohesionViolation::ContextWithoutCohesionUnit {
                context: "widgets".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }]
        );
    }

    #[test]
    fn the_ladder_names_a_concept_exactly_as_the_one_reader_does() {
        let t = tree("# widgets\n\n## Widget ##\n\n## *Other*\n");
        assert_eq!(
            t.concept_declarations(),
            vec![("Widget ##", "widgets"), ("*Other*", "widgets")],
            "the name the ladder binds is the name the graph node carries"
        );
    }

    #[test]
    fn a_hash_run_inside_a_fence_is_no_rung() {
        let t = tree("# widgets\n\n```markdown\n## Widget\n```\n");
        assert_eq!(
            t.cohesion_violations(),
            vec![CohesionViolation::ContextWithoutCohesionUnit {
                context: "widgets".to_owned(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }]
        );
    }

    #[test]
    fn normalization_matches_on_both_sides() {
        let concepts = tree("# AC verifier\n\n## Foo\n");
        assert_eq!(concepts.context_id(), Some("ac-verifier"));
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
        let t = tree("# equivalence\n\n## Graph\n\n#### member\n");
        assert_eq!(t.nodes[2].level, AbstractionLevel::Member);
        assert_eq!(t.nodes[2].parent, Some(1));
    }

    #[test]
    fn assemble_spec_trees_drops_no_file_over_its_h1() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().expect("tempdir");
        let write = |rel: &str, body: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::File::create(&p)
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
        };
        write("concepts/cfdb-cli.md", "# Spec: cfdb-cli\n\n## Foo\n");
        write("concepts/reading.md", "# reading\n\n## Bar\n");

        let trees = assemble_spec_trees(dir.path()).expect("walk must not abort");
        let ids: Vec<_> = trees.iter().filter_map(SpecTree::context_id).collect();
        assert_eq!(
            ids,
            vec!["spec:-cfdb-cli", "reading"],
            "a prose H1 keeps its file in the walk"
        );
    }

    #[test]
    fn draft_doc_with_only_an_h1_declares_no_cohesion_unit() {
        let t = tree("---\nstatus: draft\n---\n\n# equivalence\n\nJust prose.\n");
        assert_eq!(
            t.cohesion_violations(),
            vec![CohesionViolation::ContextWithoutCohesionUnit {
                context: "equivalence".to_string(),
                file: PathBuf::from("specs/concepts/equivalence.md"),
            }]
        );
    }

    #[test]
    fn a_marked_heading_counts_as_a_cohesion_unit() {
        let t = tree("---\nstatus: draft\n---\n\n# equivalence\n\n## Graph\n");
        assert!(
            t.cohesion_violations().is_empty(),
            "a marked H2 satisfies its context: {:?}",
            t.cohesion_violations()
        );

        let t = tree("# equivalence\n\n## Graph\n\n- status: draft\n");
        assert!(t.cohesion_violations().is_empty());
    }

    #[test]
    fn the_behavioral_exemption_applies_to_draft_docs_on_the_same_terms() {
        let exempt = tree(
            "---\nstatus: draft\ncohesion: behavioral\n---\n\n# secrets\n\n- impl: rotate_key\n",
        );
        assert!(
            exempt.cohesion_violations().is_empty(),
            "behavioral + substance exempts a draft doc: {:?}",
            exempt.cohesion_violations()
        );

        let bare = tree("---\nstatus: draft\ncohesion: behavioral\n---\n\n# secrets\n\nProse.\n");
        assert_eq!(
            bare.cohesion_violations().len(),
            1,
            "behavioral without substance is still a violation, draft or not"
        );
    }

    #[test]
    fn assemble_spec_trees_walks_draft_files() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().expect("tempdir");
        let write = |rel: &str, body: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::File::create(&p)
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
        };
        write(
            "concepts/draft.md",
            "---\nstatus: draft\n---\n\n# reading\n",
        );
        write("concepts/live.md", "# equivalence\n\n## Graph\n");

        let trees = assemble_spec_trees(dir.path()).expect("walk");
        let ids: Vec<_> = trees.iter().filter_map(SpecTree::context_id).collect();
        assert_eq!(
            ids,
            vec!["reading", "equivalence"],
            "draft files are no longer skipped by the ladder walk"
        );
        let violations: Vec<_> = trees
            .iter()
            .flat_map(SpecTree::cohesion_violations)
            .collect();
        assert_eq!(
            violations.len(),
            1,
            "the H1-only draft doc reds; the live doc does not: {violations:?}"
        );
    }

    #[test]
    fn self_dogfood_concept_specs_have_one_context_and_no_cohesion_violations() {
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

    #[test]
    fn assemble_tree_sets_behavioral_and_blanks_front_matter() {
        let t =
            tree("---\ncohesion: behavioral\n---\n\n# secrets\n\n#### Operational invariants\n");
        assert!(t.behavioral, "cohesion: behavioral must set the flag");
        assert!(
            t.nodes.iter().all(|n| n.text != "cohesion: behavioral"),
            "front-matter must not become a heading: {:?}",
            t.nodes
        );
        let ctx = t
            .nodes
            .iter()
            .find(|n| n.level == AbstractionLevel::Context)
            .expect("context node");
        assert_eq!(ctx.id.as_deref(), Some("secrets"));
        assert_eq!(ctx.line, 5);
    }

    #[test]
    fn assemble_tree_without_front_matter_is_not_behavioral() {
        let t = tree("# equivalence\n## Graph\n");
        assert!(!t.behavioral);
    }

    #[test]
    fn behavioral_doctrine_context_with_prose_only_annotation_is_exempt() {
        let t = tree(
            "---\ncohesion: behavioral\n---\n\n# secrets\n\n#### Operational invariants\n\n- INV-x: no type until a future RFC [prose-only: doctrine]\n",
        );
        assert!(t.behavioral && t.has_substance);
        assert!(
            t.cohesion_violations().is_empty(),
            "behavioral+substance must be exempt: {:?}",
            t.cohesion_violations()
        );
    }

    #[test]
    fn behavioral_with_impl_anchor_substance_is_exempt() {
        let t = tree("---\ncohesion: behavioral\n---\n\n# fsm\n\n- impl: merge_rail\n");
        assert!(t.has_substance);
        assert!(t.cohesion_violations().is_empty());
    }

    #[test]
    fn behavioral_with_verb_anchor_substance_is_exempt() {
        let t = tree("---\ncohesion: behavioral\n---\n\n# fsm\n\n- verb: merge_rail\n");
        assert!(t.has_substance);
        assert!(t.cohesion_violations().is_empty());
    }

    #[test]
    fn behavioral_without_substance_still_fires() {
        let t =
            tree("---\ncohesion: behavioral\n---\n\n# empty-doctrine\n\nJust prose, no anchors.\n");
        assert!(t.behavioral && !t.has_substance);
        assert!(
            t.cohesion_violations()
                .iter()
                .any(|v| matches!(v, CohesionViolation::ContextWithoutCohesionUnit { .. })),
            "empty behavioral file must still be a violation"
        );
    }

    #[test]
    fn non_behavioral_type_free_context_still_fires() {
        let t = tree("# lonely\n\n- impl: something\n");
        assert!(!t.behavioral);
        assert!(t
            .cohesion_violations()
            .iter()
            .any(|v| matches!(v, CohesionViolation::ContextWithoutCohesionUnit { .. })));
    }
}
