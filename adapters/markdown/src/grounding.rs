use domain::Polarity;

const KEY: &str = "polarity:";

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
        assert_eq!(
            polarity_from_comment("<!-- anchor:\"polarity:forbidden is the ban marker\" -->"),
            Polarity::Declared,
            "a quoted decoy must not be read as the key"
        );
        assert_eq!(
            polarity_from_comment(
                "<!-- anchor:\"see polarity:illustrative in RFC-9\" polarity:forbidden -->"
            ),
            Polarity::Forbidden
        );
    }

    #[test]
    fn unreadable_input_falls_back_to_declared() {
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
        assert_eq!(
            polarity_from_comment("<!-- anchor:\"le concept — révisé\" polarity:illustrative -->"),
            Polarity::Illustrative
        );
    }
}
