use cascade::{Concept, ParsedSpec};
use domain::{Marker, Polarity};
use ports::ReaderError;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct DialectHeading {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) level: usize,
    pub marker: Marker,
    pub polarity: Polarity,
}

#[derive(Debug)]
pub(crate) struct DialectRead {
    pub ladder: Vec<DialectHeading>,
}

impl DialectRead {
    pub fn concepts(&self) -> impl Iterator<Item = &DialectHeading> {
        self.ladder.iter().filter(|h| (2..=3).contains(&h.level))
    }

    pub fn extent(&self, heading: &DialectHeading) -> (usize, usize) {
        let end = self
            .ladder
            .iter()
            .find(|h| h.line > heading.line)
            .map_or(usize::MAX, |h| h.line);
        (heading.line, end)
    }
}

pub(crate) fn read(path: &Path, source: &str) -> Result<DialectRead, ReaderError> {
    let file = path.display().to_string();
    let parsed = cascade::parse_spec(&file, source);
    refuse(path, &file, &parsed)?;
    Ok(DialectRead {
        ladder: ladder(source, &parsed),
    })
}

fn ladder(source: &str, parsed: &ParsedSpec) -> Vec<DialectHeading> {
    let lines: Vec<&str> = source.lines().collect();
    let whole_file = whole_file_marker(&lines);
    let mut out: Vec<DialectHeading> = parsed
        .sites
        .iter()
        .map(|(name, _, line, level)| {
            let declared = parsed
                .concepts
                .iter()
                .find(|c| c.line == *line && c.name == *name);
            DialectHeading {
                name: name.clone(),
                line: *line,
                level: *level,
                marker: declared
                    .map_or_else(|| unplaced_marker(&lines, *line, whole_file), marker_of),
                polarity: declared.map_or(Polarity::Declared, polarity_of),
            }
        })
        .collect();
    out.extend(
        unexported_rungs(&lines)
            .into_iter()
            .map(|(level, line, name)| DialectHeading {
                name,
                line,
                level,
                marker: Marker::Unmarked,
                polarity: Polarity::Declared,
            }),
    );
    out.sort_by_key(|h| h.line);
    out
}

const DRAFT_BULLET: &str = "- status: draft";
const RETIRED_BULLET: &str = "- status: retired";

fn unplaced_marker(lines: &[&str], heading_line: usize, whole_file: Marker) -> Marker {
    let mut j = heading_line;
    while lines.get(j).is_some_and(|l| l.trim().is_empty()) {
        j += 1;
    }
    let Some(line) = lines.get(j).map(|l| l.trim()) else {
        return whole_file;
    };
    for (bullet, marker) in [
        (RETIRED_BULLET, Marker::Retired),
        (DRAFT_BULLET, Marker::Draft),
    ] {
        if line
            .strip_prefix(bullet)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return marker;
        }
    }
    whole_file
}

fn whole_file_marker(lines: &[&str]) -> Marker {
    let mut lines = lines.iter();
    if lines.next().map(|l| l.trim()) != Some("---") {
        return Marker::Unmarked;
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("status:") {
            return if rest.trim() == "draft" {
                Marker::Draft
            } else {
                Marker::Unmarked
            };
        }
    }
    Marker::Unmarked
}

fn unexported_rungs(lines: &[&str]) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut fenced = false;
    for (i, line) in lines.iter().enumerate() {
        let opener = line.trim_start();
        if opener.starts_with("```") || opener.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let Some(level) = rung_level(line) else {
            continue;
        };
        if (2..=3).contains(&level) {
            continue;
        }
        out.push((level, i + 1, line[level..].trim().to_owned()));
    }
    out
}

fn rung_level(line: &str) -> Option<usize> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    match line[level..].chars().next() {
        None | Some(' ' | '\t') => Some(level),
        Some(_) => None,
    }
}

const fn marker_of(concept: &Concept) -> Marker {
    if concept.retired {
        Marker::Retired
    } else if concept.draft {
        Marker::Draft
    } else {
        Marker::Unmarked
    }
}

const fn polarity_of(concept: &Concept) -> Polarity {
    match concept.polarity {
        cascade::Polarity::Declared => Polarity::Declared,
        cascade::Polarity::Forbidden => Polarity::Forbidden,
        cascade::Polarity::Illustrative => Polarity::Illustrative,
    }
}

fn refuse(path: &Path, file: &str, parsed: &ParsedSpec) -> Result<(), ReaderError> {
    if let Some((_, line)) = parsed.unclosed_fences.first() {
        return Err(refusal(path, *line, "unclosed code fence"));
    }
    if let Some((name, reason, _, line)) = parsed.malformed_headings.first() {
        return Err(refusal(path, *line, &format!("`{name}`: {reason}")));
    }
    if let Some((_, line)) = parsed.orphans.first() {
        return Err(refusal(
            path,
            *line,
            "grounding declaration attaches to no concept",
        ));
    }
    if let Some((_, reason)) = parsed.malformed_frontmatter.first() {
        return Err(refusal(path, 1, reason));
    }
    if let Some((_, reason)) = parsed.malformed.iter().find(|(site, _)| site == file) {
        return Err(refusal(path, 1, reason));
    }
    Ok(())
}

fn refusal(path: &Path, line: usize, message: &str) -> ReaderError {
    ReaderError::ParseFailed {
        path: path.to_path_buf(),
        line,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_str(md: &str) -> Result<DialectRead, ReaderError> {
        read(Path::new("specs/concepts/reading.md"), md)
    }

    fn headings(md: &str) -> Vec<(String, Marker, Polarity)> {
        read_str(md)
            .expect("well-formed dialect")
            .concepts()
            .map(|h| (h.name.clone(), h.marker, h.polarity))
            .collect()
    }

    fn ladder_of(md: &str) -> Vec<(usize, String)> {
        read_str(md)
            .expect("well-formed dialect")
            .ladder
            .into_iter()
            .map(|h| (h.level, h.name))
            .collect()
    }

    const ROOT: &str = "parent:rfc:keel-dialect#3.2 anchor:\"Closed set\"";

    #[test]
    fn the_three_polarity_values_read_through() {
        for (value, expected) in [
            ("declared", Polarity::Declared),
            ("forbidden", Polarity::Forbidden),
            ("illustrative", Polarity::Illustrative),
        ] {
            let md =
                format!("# reading\n\n## Unit\n\n<!-- {ROOT} polarity:{value} -->\n\nProse.\n");
            assert_eq!(headings(&md)[0].2, expected, "polarity:{value}");
        }
    }

    #[test]
    fn an_unknown_polarity_value_is_malformed() {
        let md = format!("# reading\n\n## Unit\n\n<!-- {ROOT} polarity:Forbidden -->\n");
        let err = read_str(&md).expect_err("unknown polarity refuses");
        assert!(
            matches!(&err, ReaderError::ParseFailed { message, .. } if message.contains("unknown polarity")),
            "got {err:?}"
        );
    }

    #[test]
    fn an_unrecognised_key_is_malformed() {
        let md = format!("# reading\n\n## Unit\n\n<!-- {ROOT} obligation:declared -->\n");
        let err = read_str(&md).expect_err("unknown key refuses");
        assert!(
            matches!(&err, ReaderError::ParseFailed { message, .. } if message.contains("unknown key")),
            "got {err:?}"
        );
    }

    #[test]
    fn the_full_key_set_is_admitted() {
        let md = format!(
            "# reading\n\n## Unit\n\n<!-- {ROOT} keywords:\"a, b\" reached_for:\"c\" polarity:forbidden -->\n"
        );
        assert_eq!(
            headings(&md),
            vec![("Unit".to_owned(), Marker::Unmarked, Polarity::Forbidden)]
        );
    }

    #[test]
    fn a_state_marker_and_a_grounding_comment_coexist() {
        let md = format!(
            "# reading\n\n## Unit\n\n- status: draft (per keel-dialect#4)\n\n<!-- {ROOT} polarity:forbidden -->\n"
        );
        assert_eq!(
            headings(&md),
            vec![("Unit".to_owned(), Marker::Draft, Polarity::Forbidden)],
            "the comment below the marker still grounds the heading"
        );
    }

    #[test]
    fn a_marker_value_is_case_exact() {
        let exact = "# reading\n\n## Unit\n\n- status: retired\n";
        assert_eq!(headings(exact)[0].1, Marker::Retired);
        let shouted = "# reading\n\n## Unit\n\n- status: Retired\n";
        assert_eq!(headings(shouted)[0].1, Marker::Unmarked);
    }

    #[test]
    fn a_state_marker_holds_on_a_heading_the_reader_leaves_unplaced() {
        for (md, expected) in [
            ("## Digest\n\n- status: draft\n", Marker::Draft),
            ("## Digest\n\n- status: retired\n", Marker::Retired),
            (
                "---\nstatus: draft\n---\n\n## Digest\n\nProse.\n",
                Marker::Draft,
            ),
            ("## Digest\n\nProse.\n", Marker::Unmarked),
            ("## Digest\n\n- status: Draft\n", Marker::Unmarked),
        ] {
            assert_eq!(
                headings(md)[0].1,
                expected,
                "no `#` rung suspends no §4 declaration: {md:?}"
            );
        }
    }

    #[test]
    fn an_unplaced_marker_reads_the_same_as_a_placed_one() {
        for body in [
            "## Digest\n\n- status: draft\n",
            "## Digest\n\n- status: retired\n",
            "## Digest\n\nProse.\n",
        ] {
            let placed = format!("# core\n\n{body}");
            assert_eq!(
                headings(&placed)[0].1,
                headings(body)[0].1,
                "the `#` rung moves no marker: {body:?}"
            );
        }
    }

    #[test]
    fn a_marker_holds_on_an_unplaced_heading_of_a_grounded_document() {
        let md = format!(
            "## Unit\n\n<!-- {ROOT} -->\n\nProse.\n\n## Digest\n\n- status: draft\n\nProse.\n"
        );
        assert_eq!(
            headings(&md)
                .into_iter()
                .map(|h| (h.0, h.1))
                .collect::<Vec<_>>(),
            vec![
                ("Unit".to_owned(), Marker::Unmarked),
                ("Digest".to_owned(), Marker::Draft)
            ]
        );
    }

    #[test]
    fn a_retired_bullet_outranks_a_whole_file_draft_on_an_unplaced_heading() {
        let md = "---\nstatus: draft\n---\n\n## Digest\n\n- status: retired\n";
        assert_eq!(headings(md)[0].1, Marker::Retired);
    }

    #[test]
    fn an_ungrounded_heading_still_reports_as_a_site() {
        let md = "# reading\n\n## Unit\n\nProse.\n\n### Inner\n";
        let names: Vec<_> = headings(md).into_iter().map(|h| h.0).collect();
        assert_eq!(names, vec!["Unit".to_owned(), "Inner".to_owned()]);
    }

    #[test]
    fn a_setext_heading_opens_no_rung() {
        assert_eq!(
            ladder_of("# reading\n\nUnit\n----\n\nProse.\n"),
            vec![(1, "reading".to_owned())],
            "the one reader is ATX-only; the ladder carries no setext rung"
        );
    }

    #[test]
    fn an_indented_hash_run_opens_no_rung() {
        assert_eq!(
            ladder_of("# reading\n\n  ## Unit\n"),
            vec![(1, "reading".to_owned())]
        );
    }

    #[test]
    fn a_closing_hash_run_and_emphasis_stay_in_the_name() {
        assert_eq!(
            ladder_of("# reading\n\n## Unit ##\n\n## *Other*\n"),
            vec![
                (1, "reading".to_owned()),
                (2, "Unit ##".to_owned()),
                (2, "*Other*".to_owned())
            ],
            "the name is the one reader's, verbatim"
        );
    }

    #[test]
    fn a_hash_run_inside_a_fence_opens_no_rung() {
        assert_eq!(
            ladder_of("# reading\n\n```markdown\n## Unit\n#### Rung\n```\n"),
            vec![(1, "reading".to_owned())]
        );
    }

    #[test]
    fn the_context_and_callout_rungs_stand_beside_the_concepts() {
        assert_eq!(
            ladder_of("# reading\n\n## Unit\n\n#### Distinct from\n\n### Inner\n"),
            vec![
                (1, "reading".to_owned()),
                (2, "Unit".to_owned()),
                (4, "Distinct from".to_owned()),
                (3, "Inner".to_owned())
            ]
        );
    }

    #[test]
    fn a_callout_rung_closes_the_concepts_extent() {
        let read =
            read_str("# reading\n\n## Unit\n\n#### Distinct from\n\n- depends on: Nowhere\n")
                .expect("well-formed dialect");
        let unit = read.concepts().next().expect("one concept").clone();
        let (start, end) = read.extent(&unit);
        assert_eq!((start, end), (3, 5), "the extent stops at the `####` rung");
    }

    #[test]
    fn an_unclosed_fence_refuses_the_run() {
        let err = read_str("# reading\n\n## Unit\n\n```rust\npub struct Unit;\n")
            .expect_err("an unclosed fence is a run-level finding");
        assert!(
            matches!(&err, ReaderError::ParseFailed { message, .. } if message.contains("unclosed code fence")),
            "got {err:?}"
        );
    }

    #[test]
    fn a_homeless_vocabulary_callout_carries_no_verdict_row_and_does_not_refuse() {
        let read = read_str("#### Also reached for\n\n- widget\n")
            .expect("no §7 row maps this to a refusal");
        assert_eq!(
            read.concepts().count(),
            0,
            "it opens no concept either — it is simply not this reader's finding"
        );
    }

    #[test]
    fn an_empty_concept_name_carries_no_verdict_row_and_does_not_refuse() {
        let read = read_str("# reading\n\n## \n\nProse.\n").expect("no §7 row maps this");
        assert_eq!(
            read.concepts().map(|h| h.name.clone()).collect::<Vec<_>>(),
            vec!["##".to_owned()],
            "the site stands under the one reader's own name for it"
        );
    }
}
