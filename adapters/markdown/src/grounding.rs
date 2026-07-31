//! The grounding comment's `polarity:` key — RFC-014 §3.2.
//!
//! A **grounding comment** is an HTML comment an upstream tool (cascade /
//! Bosun) carries under a concept heading:
//!
//! ```text
//! ## Member
//! <!-- parent:spec:Unit polarity:forbidden -->
//! ```
//!
//! This module reads exactly one key from it and ignores every other. It
//! performs **no grounding** in the sense the name denotes: *grounding*
//! means ancestorship, which is the `parent:` key's job and is explicitly
//! out of scope (RFC-014 §6). `polarity:` is an independent axis that
//! happens to share the grounding block's syntax, not part of its ancestry
//! payload.
//!
//! Deliberately **not** cascade's full grounding validation — no
//! `GroundingTokens`, no `resolve_parent`, no `resolve_reached_for`.
//! Unrecognised keys are skipped, never rejected.

use domain::Polarity;

/// The key this module reads. Everything else in the comment is skipped.
const KEY: &str = "polarity:";

/// Read the grounding polarity out of one HTML comment's text.
///
/// Anything unreadable resolves to [`Polarity::Declared`] — no comment, no
/// `polarity:` key, or a value this build does not know. **The fallback
/// direction is the point:** a typo leaves the heading's obligation *armed*.
/// A marker can only narrow an obligation somebody deliberately wrote down.
///
/// An unknown value additionally emits a `tracing::warn!` — the tolerant-skip
/// failure mode `- verb:` already uses.
pub fn polarity_from_comment(html: &str) -> Polarity {
    let Some(value) = polarity_token(html) else {
        return Polarity::Declared;
    };
    match value.to_ascii_lowercase().as_str() {
        "declared" => Polarity::Declared,
        "forbidden" => Polarity::Forbidden,
        "illustrative" => Polarity::Illustrative,
        _ => {
            tracing::warn!(
                "unreadable grounding polarity {value:?} — treating the heading as `declared`"
            );
            Polarity::Declared
        }
    }
}

/// Find the `polarity:` key's raw value, **skipping `"…"`-quoted regions**.
///
/// Upstream makes `anchor:"…"` mandatory for every RFC-rooted concept, so a
/// real grounded corpus carries a quoted freeform value in the *same*
/// comment as `polarity:`. A bare substring scan would mis-read a decoy
/// inside that value — entirely plausible on an architecture-methodology
/// corpus, which may carry RFC prose *about* polarity.
fn polarity_token(html: &str) -> Option<&str> {
    let mut in_quote = false;
    for (i, ch) in html.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
        } else if !in_quote && html[i..].starts_with(KEY) {
            return trim_value(&html[i + KEY.len()..]);
        }
    }
    None
}

/// The value token following the key: everything up to the next whitespace,
/// with a comment terminator stripped if it is glued on (`forbidden-->`).
fn trim_value(rest: &str) -> Option<&str> {
    let token = rest.split_whitespace().next()?;
    let token = token.strip_suffix("-->").unwrap_or(token);
    (!token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_three_values() {
        assert_eq!(
            polarity_from_comment("<!-- polarity:declared -->"),
            Polarity::Declared
        );
        assert_eq!(
            polarity_from_comment("<!-- polarity:forbidden -->"),
            Polarity::Forbidden
        );
        assert_eq!(
            polarity_from_comment("<!-- polarity:illustrative -->"),
            Polarity::Illustrative
        );
    }

    #[test]
    fn value_matches_case_insensitively() {
        assert_eq!(
            polarity_from_comment("<!-- polarity:Forbidden -->"),
            Polarity::Forbidden
        );
    }

    #[test]
    fn other_grounding_keys_are_skipped_not_rejected() {
        assert_eq!(
            polarity_from_comment(
                "<!-- parent:spec:Unit anchor:\"the unit of work\" reached_for:x polarity:forbidden -->"
            ),
            Polarity::Forbidden
        );
    }

    #[test]
    fn a_decoy_inside_a_quoted_value_is_not_the_key() {
        // The motivating case: an architecture-methodology corpus carrying
        // RFC prose *about* polarity inside the mandatory `anchor:"…"`.
        assert_eq!(
            polarity_from_comment("<!-- anchor:\"polarity:forbidden is the ban marker\" -->"),
            Polarity::Declared,
            "a quoted decoy must not be read as the key"
        );
        // …and the real key is still found when it follows the decoy.
        assert_eq!(
            polarity_from_comment(
                "<!-- anchor:\"see polarity:illustrative in RFC-9\" polarity:forbidden -->"
            ),
            Polarity::Forbidden
        );
    }

    #[test]
    fn unreadable_input_falls_back_to_declared() {
        // The fallback direction is the point — a typo leaves the
        // obligation armed rather than silently narrowing it.
        assert_eq!(
            polarity_from_comment("<!-- polarity:frobidden -->"),
            Polarity::Declared
        );
        assert_eq!(
            polarity_from_comment("<!-- polarity: -->"),
            Polarity::Declared
        );
        assert_eq!(
            polarity_from_comment("<!-- parent:spec:Unit -->"),
            Polarity::Declared
        );
        assert_eq!(polarity_from_comment(""), Polarity::Declared);
    }

    #[test]
    fn a_glued_comment_terminator_is_not_part_of_the_value() {
        assert_eq!(
            polarity_from_comment("<!--polarity:forbidden-->"),
            Polarity::Forbidden
        );
    }

    #[test]
    fn multibyte_prose_does_not_break_the_scan() {
        // `char_indices` rather than byte indexing — a grounded corpus is
        // prose-heavy and will carry non-ASCII.
        assert_eq!(
            polarity_from_comment("<!-- anchor:\"le concept — révisé\" polarity:illustrative -->"),
            Polarity::Illustrative
        );
    }
}
