//! Leading YAML front-matter detection — `status: draft` and
//! `cohesion: behavioral` (RFC-012 §3.3), plus the front-matter-blanking
//! pass shared by the concept walk ([`crate::section`]) and the heading
//! tree assembler ([`crate::tree`]).

use crate::bullets::{parse_impl_bullet, parse_verb_bullet};

/// Returns `true` when `source` opens with a YAML front-matter block
/// (delimited by lines containing only `---`) that declares
/// `status: draft`.
///
/// A draft spec is **pre-authored ahead of its code**: its concepts,
/// verb anchors, and invariant annotations intentionally have no
/// implementation yet, so every concept-walking reader skips the file
/// and it contributes no nodes, edges, or anchors to the spec graph —
/// no `missing in code` violation can arise from it. Removing or
/// changing the `status:` line re-arms the file: its declarations then
/// resolve against code like any other spec.
///
/// Only the leading front-matter block is consulted — a `status:` line
/// in the prose body has no effect. The value is matched
/// case-insensitively, with or without surrounding quotes, and any
/// trailing `#` comment is ignored. A front-matter block that closes
/// before any `status:` line, or a file with no front-matter at all,
/// is not draft.
pub(crate) fn is_draft(source: &str) -> bool {
    front_matter_value(source, "status").is_some_and(|v| v.eq_ignore_ascii_case("draft"))
}

/// Returns `true` when `source` carries machine-checkable **behavioral
/// substance** (RFC-012 §3.3.1) — at least one `- impl:` / `- verb:` anchor
/// bullet or one `[enforced-by:]` / `[prose-only:]` invariant annotation.
///
/// This is the anti-gaming gate for `cohesion: behavioral`: the marker
/// exempts a context from `ContextWithoutCohesionUnit` only when the context
/// demonstrates behavioral content — never against an empty file. Reuses the
/// canonical bullet grammar ([`parse_impl_bullet`] / [`parse_verb_bullet`])
/// so the substance set cannot drift from what the readers actually parse.
pub(crate) fn has_behavioral_substance(source: &str) -> bool {
    source.lines().any(|line| {
        if line.contains("[enforced-by:") || line.contains("[prose-only:") {
            return true;
        }
        strip_bullet_marker(line)
            .is_some_and(|b| parse_impl_bullet(b).is_some() || parse_verb_bullet(b).is_some())
    })
}

/// Strip a leading markdown list marker (`-` / `*` / `+` followed by
/// whitespace) from `line`, returning the bullet text. `None` when the line
/// is not a list item.
fn strip_bullet_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for marker in ['-', '*', '+'] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            if rest.starts_with([' ', '\t']) {
                return Some(rest.trim_start());
            }
        }
    }
    None
}

/// Returns `true` when `source`'s leading front-matter declares
/// `cohesion: behavioral` (RFC-012 §3.3).
///
/// A behavioral/doctrine context owns no `pub` type by design; this marker
/// lets it satisfy `ContextWithoutCohesionUnit` — **gated** by behavioral
/// substance at the cohesion pass (R12-4), never a bare free pass. Like
/// [`is_draft`], only the leading front-matter is consulted; unlike draft,
/// a behavioral file is **not** skipped — it is a real spec walked normally.
pub(crate) fn is_behavioral_context(source: &str) -> bool {
    front_matter_value(source, "cohesion").is_some_and(|v| v.eq_ignore_ascii_case("behavioral"))
}

/// Read one key's value from the leading YAML front-matter block, if present.
///
/// Returns `None` when there is no leading `---` block (the first non-empty
/// line is not `---`) or the key does not appear before the block closes.
/// The value is stripped of a trailing `#` comment and surrounding quotes —
/// the shared parse for [`is_draft`] / [`is_behavioral_context`].
fn front_matter_value(source: &str, key: &str) -> Option<String> {
    // The opening fence must be the first non-empty line.
    let mut lines = source.lines().skip_while(|l| l.trim().is_empty());
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    let prefix = format!("{key}:");
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            return None;
        }
        if let Some(rest) = trimmed.strip_prefix(prefix.as_str()) {
            let value = rest
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return Some(value.to_owned());
        }
    }
    None
}

/// Replace a leading `---` … `---` front-matter block with blank lines,
/// returning the result (borrowed when there is no leading block).
///
/// `status: draft` files are skipped wholesale, but a `cohesion: behavioral`
/// file (RFC-012 §3.3) is parsed normally — and its `key: value` line
/// immediately above the closing `---` would otherwise be mis-read as a
/// **setext H2 heading**, manufacturing a phantom concept. Blanking the
/// block (rather than stripping it) preserves the line count, so every
/// concept/anchor below keeps its true `path:line`.
pub(crate) fn blank_front_matter(source: &str) -> std::borrow::Cow<'_, str> {
    let lead_ws_len = source.len() - source.trim_start().len();
    let body = &source[lead_ws_len..];
    let Some(first_nl) = body.find('\n') else {
        return std::borrow::Cow::Borrowed(source);
    };
    if body[..first_nl].trim() != "---" {
        return std::borrow::Cow::Borrowed(source);
    }
    let mut cursor = first_nl + 1;
    let block_end = loop {
        let Some(nl) = body[cursor..].find('\n') else {
            // No closing fence — not a well-formed block; leave unchanged.
            return std::borrow::Cow::Borrowed(source);
        };
        let line_end = cursor + nl;
        if body[cursor..line_end].trim() == "---" {
            break lead_ws_len + line_end + 1; // through the closing newline
        }
        cursor = line_end + 1;
    };
    let newlines = source[..block_end].matches('\n').count();
    let mut out = String::with_capacity(newlines + (source.len() - block_end));
    for _ in 0..newlines {
        out.push('\n');
    }
    out.push_str(&source[block_end..]);
    std::borrow::Cow::Owned(out)
}
