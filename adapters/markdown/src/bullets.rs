use domain::{tokenise_target, ConceptAnchor, EdgeKind, Marker, Source, VerbAnchor};
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
pub fn parse_verb_bullet(text: &str) -> Option<VerbAnchor> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix("verb:")?;
    let qname = parse_anchor_qname(rest)?;
    Some(VerbAnchor {
        concept: String::new(),
        qname: qname.to_owned(),
        raw_target: trimmed.to_owned(),
        source: Source::Spec {
            path: std::path::PathBuf::new(),
            line: 0,
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
            path: std::path::PathBuf::new(),
            line: 0,
        },
    })
}

#[must_use]
pub fn parse_status_marker(text: &str) -> Option<Marker> {
    let rest = text.trim().strip_prefix("status:")?;
    marker_from_value(rest.split_whitespace().next()?)
}

#[must_use]
pub const fn marker_from_value(value: &str) -> Option<Marker> {
    if value.eq_ignore_ascii_case("draft") {
        Some(Marker::Draft)
    } else if value.eq_ignore_ascii_case("retired") {
        Some(Marker::Retired)
    } else {
        None
    }
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
