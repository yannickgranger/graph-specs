//! Bullet-line grammars recognised inside a concept section — v0.3 edge
//! prefixes (`- implements:` / `- depends on:` / `- returns:`) and the
//! v0.5/v0.6 anchor prefixes (`- verb:` / `- impl:`), which share one
//! qname grammar.

use domain::{tokenise_target, ConceptAnchor, EdgeKind, Marker, Source, VerbAnchor};
use regex::Regex;
use std::sync::LazyLock;

pub const BULLET_PREFIXES: &[(&str, EdgeKind)] = &[
    ("implements:", EdgeKind::Implements),
    ("depends on:", EdgeKind::DependsOn),
    ("returns:", EdgeKind::Returns),
];

/// Compiled regex for validating verb-bullet qnames (v0.6).
///
/// Accepts bare identifiers (`foo`) and `Type::method` two-segment qnames.
/// Rejects multi-segment paths, leading or trailing `::`, and non-ident chars.
static VERB_QNAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$").expect("valid regex")
});

/// The **single** qname grammar shared by `- verb:` and `- impl:` bullets.
///
/// Validates `qname` against [`VERB_QNAME_RE`] (bare ident or `Type::method`
/// / `Enum::Variant`). Returns the trimmed qname, or `None` — empty silently,
/// non-empty-but-malformed with a tolerant-skip `tracing::warn!`. Both bullet
/// parsers route here so a grammar change touches one site.
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

/// Parse a `- verb: <qname>` bullet into a [`VerbAnchor`] with placeholder fields.
///
/// The `concept` and `source` fields are left as placeholders; the caller
/// (`finish_bullet`) fills them in once the concept name and file location
/// are available.
///
/// Returns `None` for bullets that do not start with `verb:` or whose qname
/// fails [`parse_anchor_qname`] (the shared grammar).
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

/// Parse a `- impl: <qname>` bullet into a [`ConceptAnchor`] (RFC-012 §3.2)
/// with placeholder `concept` / `source` fields the caller (`finish_bullet`)
/// fills in.
///
/// Returns `None` for bullets that do not start with `impl:` or whose qname
/// fails [`parse_anchor_qname`] — the **same** grammar `- verb:` uses (§4 I7).
/// `impl:` does not prefix-collide with the `implements:` edge bullet:
/// `parse_bullet_edge` is tried first in `finish_bullet` and consumes it.
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

/// Read the `- status:` spec-state marker (RFC-013 §3.1, RFC-015 §3.1).
///
/// **Two** legal values — `draft` and `retired` — and neither transitions to
/// the other: `draft` is deleted at ratification, `retired` is never
/// deleted. A presence flag per value, still not a state machine, because
/// the progress axis is the code. Any other `- status:` bullet is an
/// unrecognised prefix under the existing dialect rule and stays inert text,
/// which is what `None` means here.
///
/// The value is matched ASCII-case-insensitively, mirroring the
/// front-matter test. Anything after it on the same line — e.g. the upstream
/// authoring convention `- status: draft (per <RFC>.md §<clause>)` — is
/// tolerated and ignored; that parenthetical is enforced by the upstream
/// tree's own fences and is never gate-parsed (RFC-013 §6).
///
/// Placement is **not** checked here: the marker only binds when it is the
/// first non-blank content line below its heading, which is the caller's
/// concern ([`crate::section`]).
#[must_use]
pub fn parse_status_marker(text: &str) -> Option<Marker> {
    let rest = text.trim().strip_prefix("status:")?;
    marker_from_value(rest.split_whitespace().next()?)
}

/// Map one already-isolated `status:` value to its marker, or `None` when
/// the word is not a legal value. The single value-recognition site — the
/// bullet and front-matter grammars differ in how they *reach* the word,
/// never in which words count.
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

/// Parse a bullet's accumulated text into an (`EdgeKind`, tokenised, raw)
/// triple, if it matches a recognised prefix. Returns `None` for prose
/// bullets and for recognised prefixes with an empty target.
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
