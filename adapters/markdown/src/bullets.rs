use domain::{tokenise_target, ConceptAnchor, EdgeKind, Source, VerbAnchor};
use regex::Regex;
use std::sync::LazyLock;

pub const BULLET_PREFIXES: &[(&str, EdgeKind)] = &[
    ("implements:", EdgeKind::Implements),
    ("depends on:", EdgeKind::DependsOn),
    ("returns:", EdgeKind::Returns),
];

static VERB_QNAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$").expect("valid regex")
});

pub fn parse_anchor_qname(rest: &str) -> Option<&str> {
    let qname = rest.trim();
    if qname.is_empty() {
        return None;
    }
    if !VERB_QNAME_RE.is_match(qname) {
        tracing::warn!("anchor bullet has malformed qname — skipping: {qname:?}");
        return None;
    }
    Some(qname)
}

#[must_use]
pub fn malformed_anchor(text: &str) -> Option<(&'static str, String)> {
    let trimmed = text.trim();
    for prefix in ["verb:", "impl:"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if parse_anchor_qname(rest).is_none() {
                return Some((prefix.trim_end_matches(':'), rest.trim().to_owned()));
            }
        }
    }
    None
}

#[must_use]
pub fn parse_verb_bullet(text: &str) -> Option<VerbAnchor> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix("verb:")?;
    let qname = parse_anchor_qname(rest)?;
    Some(VerbAnchor {
        concept: String::new(),
        qname: qname.to_owned(),
        raw_target: trimmed.to_owned(),
        source: Source::Spec {
            format: domain::SpecFormat::Markdown,
            path: std::path::PathBuf::new(),
            line: 0,
            context: None,
        },
    })
}

#[must_use]
pub fn parse_impl_bullet(text: &str) -> Option<ConceptAnchor> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix("impl:")?;
    let qname = parse_anchor_qname(rest)?;
    Some(ConceptAnchor {
        concept: String::new(),
        target: qname.to_owned(),
        source: Source::Spec {
            format: domain::SpecFormat::Markdown,
            path: std::path::PathBuf::new(),
            line: 0,
            context: None,
        },
    })
}

pub fn parse_bullet_edge(text: &str) -> Option<(EdgeKind, String, String)> {
    let trimmed = text.trim();
    for (prefix, kind) in BULLET_PREFIXES {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let raw = rest.trim().to_string();
            if raw.is_empty() {
                return None;
            }
            let token = tokenise_target(&raw);
            return Some((*kind, token, raw));
        }
    }
    None
}
