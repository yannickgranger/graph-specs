use crate::bullets::{parse_impl_bullet, parse_verb_bullet};

pub fn has_behavioral_substance(source: &str) -> bool {
    source.lines().any(|line| {
        if line.contains("[enforced-by:") || line.contains("[prose-only:") {
            return true;
        }
        strip_bullet_marker(line)
            .is_some_and(|b| parse_impl_bullet(b).is_some() || parse_verb_bullet(b).is_some())
    })
}

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

pub fn is_behavioral_context(source: &str) -> bool {
    front_matter_value(source, "cohesion").is_some_and(|v| v.eq_ignore_ascii_case("behavioral"))
}

fn front_matter_value(source: &str, key: &str) -> Option<String> {
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

pub fn blank_front_matter(source: &str) -> std::borrow::Cow<'_, str> {
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
            return std::borrow::Cow::Borrowed(source);
        };
        let line_end = cursor + nl;
        if body[cursor..line_end].trim() == "---" {
            break lead_ws_len + line_end + 1;
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
