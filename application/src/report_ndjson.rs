use crate::ndjson::source::report_source_to_json;
use crate::report::context_key;
use domain::{
    ContextPattern, HomonymRecord, ReportOutput, TierHistogramRecord, TierKind, VerbCoverageRecord,
};
use serde_json::{json, Value};
use std::io::{self, Write};

pub fn emit_ndjson(out: &mut impl Write, report: &ReportOutput) -> io::Result<()> {
    emit_verb_coverage_records(out, &report.verb_coverage)?;
    emit_tier_histogram_records(out, &report.tier_histogram)?;
    emit_homonym_records(out, &report.homonyms)?;
    Ok(())
}

fn emit_verb_coverage_records(
    out: &mut impl Write,
    records: &[VerbCoverageRecord],
) -> io::Result<()> {
    let mut sorted: Vec<&VerbCoverageRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        context_key(a.context.as_deref())
            .cmp(&context_key(b.context.as_deref()))
            .then_with(|| a.pub_fn.name.cmp(&b.pub_fn.name))
    });
    for rec in sorted {
        let record = json!({
            "record": "verb_coverage",
            "schema_version": "2",
            "context": rec.context,
            "pub_fn": {
                "name": rec.pub_fn.name,
                "source": report_source_to_json(&rec.pub_fn.source),
            },
            "cited": rec.cited,
        });
        write_record(out, &record)?;
    }
    Ok(())
}

fn emit_tier_histogram_records(
    out: &mut impl Write,
    records: &[TierHistogramRecord],
) -> io::Result<()> {
    let mut sorted: Vec<&TierHistogramRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        context_key(a.context.as_deref())
            .cmp(&context_key(b.context.as_deref()))
            .then_with(|| a.tier.cmp(&b.tier))
    });
    for rec in sorted {
        let record = json!({
            "record": "tier_histogram",
            "schema_version": "2",
            "context": rec.context,
            "tier": tier_wire(rec.tier),
            "count": rec.count,
        });
        write_record(out, &record)?;
    }
    Ok(())
}

fn emit_homonym_records(out: &mut impl Write, records: &[HomonymRecord]) -> io::Result<()> {
    let mut sorted: Vec<&HomonymRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for rec in sorted {
        let contexts: Vec<Value> = rec
            .contexts
            .iter()
            .map(|app| {
                json!({
                    "context": app.context_name,
                    "sanctioned_by_pattern": app.sanctioned_by_pattern.map(ContextPattern::as_label),
                    "asymmetric": app.asymmetric,
                })
            })
            .collect();
        let record = json!({
            "record": "homonym",
            "schema_version": "2",
            "name": rec.name,
            "contexts": contexts,
        });
        write_record(out, &record)?;
    }
    Ok(())
}

fn write_record(out: &mut impl Write, record: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *out, record)?;
    out.write_all(b"\n")
}

const fn tier_wire(tier: TierKind) -> &'static str {
    match tier {
        TierKind::Cypher => "cypher",
        TierKind::Tier0 => "tier0",
        TierKind::ScriptFence => "script_fence",
        TierKind::ProseOnly => "prose_only",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::make_report;
    use domain::{HomonymAppearance, HomonymRecord, ReportOutput};
    use serde_json::Value;

    fn parse_lines(buf: &[u8]) -> Vec<Value> {
        let s = std::str::from_utf8(buf).expect("utf8");
        s.lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).expect("valid json line"))
            .collect()
    }

    fn render(report: &ReportOutput) -> Vec<u8> {
        let mut buf = Vec::new();
        emit_ndjson(&mut buf, report).expect("write");
        buf
    }

    #[test]
    fn emit_ndjson_all_lines_valid_json() {
        let buf = render(&make_report());
        let records = parse_lines(&buf);
        assert!(!records.is_empty(), "expected at least one record");
    }

    #[test]
    fn emit_ndjson_verb_coverage_record() {
        let buf = render(&make_report());
        let records = parse_lines(&buf);
        let vc = records
            .iter()
            .find(|r| r["record"] == "verb_coverage")
            .unwrap();
        assert_eq!(vc["schema_version"], "2");
        assert_eq!(vc["context"], "equivalence");
        assert_eq!(vc["pub_fn"]["name"], "run_check");
        assert_eq!(vc["pub_fn"]["source"]["kind"], "code");
        assert_eq!(vc["cited"], true);
    }

    #[test]
    fn emit_ndjson_tier_histogram_record() {
        let buf = render(&make_report());
        let records = parse_lines(&buf);
        let th = records
            .iter()
            .find(|r| r["record"] == "tier_histogram")
            .unwrap();
        assert_eq!(th["schema_version"], "2");
        assert_eq!(th["context"], Value::Null);
        assert_eq!(th["tier"], "cypher");
        assert_eq!(th["count"], 3);
    }

    #[test]
    fn emit_ndjson_homonym_record() {
        let buf = render(&make_report());
        let records = parse_lines(&buf);
        let hom = records.iter().find(|r| r["record"] == "homonym").unwrap();
        assert_eq!(hom["schema_version"], "2");
        assert_eq!(hom["name"], "Foo");
        let ctxs = hom["contexts"].as_array().unwrap();
        assert_eq!(ctxs.len(), 2);
        let ctx_a = ctxs.iter().find(|c| c["context"] == "ctx_a").unwrap();
        assert_eq!(ctx_a["sanctioned_by_pattern"], "PublishedLanguage");
        assert_eq!(ctx_a["asymmetric"], false);
        let ctx_b = ctxs.iter().find(|c| c["context"] == "ctx_b").unwrap();
        assert_eq!(ctx_b["sanctioned_by_pattern"], Value::Null);
        assert_eq!(ctx_b["asymmetric"], true);
    }

    #[test]
    fn emit_ndjson_determinism() {
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

        let lines = parse_lines(&out1);
        let names: Vec<&str> = lines
            .iter()
            .filter(|r| r["record"] == "homonym")
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["Apple", "Zebra"],
            "homonyms must be sorted alpha"
        );
    }
}
