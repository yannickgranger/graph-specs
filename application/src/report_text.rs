use crate::report::context_key;
use crate::text::source_pair;
use domain::{
    ContextPattern, HomonymRecord, ReportOutput, TierHistogramRecord, TierKind, VerbCoverageRecord,
};
use std::io::{self, Write};

pub fn emit_text(out: &mut impl Write, report: &ReportOutput) -> io::Result<()> {
    emit_verb_coverage(out, &report.verb_coverage)?;
    emit_tier_histogram(out, &report.tier_histogram)?;
    emit_homonyms(out, &report.homonyms)?;
    Ok(())
}

fn emit_verb_coverage(out: &mut impl Write, records: &[VerbCoverageRecord]) -> io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    writeln!(out, "=== verb coverage ===")?;

    let mut sorted: Vec<&VerbCoverageRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        context_key(a.context.as_deref())
            .cmp(&context_key(b.context.as_deref()))
            .then_with(|| a.pub_fn.name.cmp(&b.pub_fn.name))
    });

    let mut prev_heading: Option<String> = None;
    for rec in &sorted {
        let heading = rec.context.clone().unwrap_or_else(|| "orphaned".to_owned());
        if prev_heading.as_deref() != Some(heading.as_str()) {
            writeln!(out, "  [context: {heading}]")?;
            prev_heading = Some(heading);
        }
        let (path, line) = source_pair(&rec.pub_fn.source);
        let cited = if rec.cited { "cited" } else { "uncited" };
        writeln!(
            out,
            "    {} ({}:{}) [{cited}]",
            rec.pub_fn.name,
            path.display(),
            line
        )?;
    }
    Ok(())
}

fn emit_tier_histogram(out: &mut impl Write, records: &[TierHistogramRecord]) -> io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    writeln!(out, "=== tier histogram ===")?;

    let mut sorted: Vec<&TierHistogramRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        context_key(a.context.as_deref())
            .cmp(&context_key(b.context.as_deref()))
            .then_with(|| a.tier.cmp(&b.tier))
    });

    let mut prev_heading: Option<String> = None;
    for rec in &sorted {
        let heading = rec.context.clone().unwrap_or_else(|| "orphaned".to_owned());
        if prev_heading.as_deref() != Some(heading.as_str()) {
            writeln!(out, "  [context: {heading}]")?;
            prev_heading = Some(heading);
        }
        writeln!(out, "    {}: {}", tier_label(rec.tier), rec.count)?;
    }
    Ok(())
}

fn emit_homonyms(out: &mut impl Write, records: &[HomonymRecord]) -> io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    writeln!(out, "=== homonyms ===")?;
    let mut sorted: Vec<&HomonymRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for rec in sorted {
        writeln!(out, "  {}", rec.name)?;
        for app in &rec.contexts {
            let marker = match app.sanctioned_by_pattern {
                Some(ContextPattern::PublishedLanguage) => "[PublishedLanguage]",
                Some(ContextPattern::SharedKernel) => "[SharedKernel]",
                _ => "[!]",
            };
            let asym = if app.asymmetric { " [asymmetric]" } else { "" };
            writeln!(out, "    {} {marker}{asym}", app.context_name)?;
        }
    }
    Ok(())
}

const fn tier_label(tier: TierKind) -> &'static str {
    match tier {
        TierKind::Cypher => "Cypher",
        TierKind::Tier0 => "Tier0",
        TierKind::ScriptFence => "ScriptFence",
        TierKind::ProseOnly => "ProseOnly",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::make_report;
    use domain::{HomonymAppearance, HomonymRecord, ReportOutput};

    fn render(report: &ReportOutput) -> String {
        let mut buf = Vec::new();
        emit_text(&mut buf, report).expect("write");
        String::from_utf8(buf).expect("utf8")
    }

    #[test]
    fn emit_text_verb_coverage_section() {
        let out = render(&make_report());
        assert!(
            out.contains("=== verb coverage ==="),
            "missing section header"
        );
        assert!(
            out.contains("[context: equivalence]"),
            "missing context heading"
        );
        assert!(out.contains("run_check"), "missing fn name");
        assert!(out.contains("[cited]"), "missing cited marker");
        assert!(
            out.contains("application/src/lib.rs:33"),
            "missing source location"
        );
    }

    #[test]
    fn emit_text_tier_histogram_section() {
        let out = render(&make_report());
        assert!(
            out.contains("=== tier histogram ==="),
            "missing section header"
        );
        assert!(
            out.contains("[context: orphaned]"),
            "missing orphaned heading for None context"
        );
        assert!(out.contains("Cypher: 3"), "missing tier count");
    }

    #[test]
    fn emit_text_homonym_section() {
        let out = render(&make_report());
        assert!(out.contains("=== homonyms ==="), "missing section header");
        assert!(out.contains("  Foo"), "missing homonym name");
        assert!(
            out.contains("[PublishedLanguage]"),
            "missing PublishedLanguage marker"
        );
        assert!(out.contains("[!]"), "missing split-brain marker");
        assert!(out.contains("[asymmetric]"), "missing asymmetric marker");
    }

    #[test]
    fn emit_text_determinism() {
        let report = ReportOutput {
            verb_coverage: vec![],
            tier_histogram: vec![],
            homonyms: vec![
                HomonymRecord {
                    name: "Zebra".to_owned(),
                    contexts: vec![HomonymAppearance {
                        context_name: "ctx_z".to_owned(),
                        sanctioned_by_pattern: None,
                        asymmetric: false,
                    }],
                },
                HomonymRecord {
                    name: "Apple".to_owned(),
                    contexts: vec![HomonymAppearance {
                        context_name: "ctx_a".to_owned(),
                        sanctioned_by_pattern: None,
                        asymmetric: false,
                    }],
                },
            ],
        };
        let out1 = render(&report);
        let out2 = render(&report);
        assert_eq!(out1, out2, "output must be deterministic");

        let apple_pos = out1.find("  Apple").expect("Apple must appear");
        let zebra_pos = out1.find("  Zebra").expect("Zebra must appear");
        assert!(apple_pos < zebra_pos, "Apple must sort before Zebra");
    }

    #[test]
    fn emit_text_empty_sections_omitted() {
        let report = ReportOutput::default();
        let out = render(&report);
        assert!(!out.contains("=== verb coverage ==="));
        assert!(!out.contains("=== tier histogram ==="));
        assert!(!out.contains("=== homonyms ==="));
    }
}
