//! Tokenisation of edge targets.
//!
//! A declared edge target — from a spec bullet (`- implements: Foo`) or
//! from a rust `syn` type (`impl Foo for Bar`) — is reduced to a single
//! primary identifier before comparison. Both sides funnel through this
//! function so the two readers cannot drift on tokenisation rules.
//!
//! Rules (applied in order):
//!
//! 1. Trim surrounding whitespace.
//! 2. Strip leading references (`&T`, `&mut T`, and repeated `&`),
//!    including any lifetime that follows (`&'a T`, `&'a mut T`).
//! 3. Drop generic parameters — keep only the primary head
//!    (`Result<Graph, E>` → `Result`).
//! 4. Keep only the last `::` segment (`domain::Graph` → `Graph`).
//! 5. Trim again.
//!
//! The result is the "matching token": two edges match iff their
//! `EdgeKind` agrees and their tokenised targets are byte-equal.

/// Reduce a raw edge target to its matching token.
///
/// Returns an empty string only if the input was empty or whitespace-only.
#[must_use]
pub fn tokenise_target(raw: &str) -> String {
    let mut s = raw.trim();
    loop {
        if let Some(rest) = s.strip_prefix('&') {
            s = rest.trim_start();
            continue;
        }
        if let Some(rest) = strip_lifetime(s) {
            s = rest.trim_start();
            continue;
        }
        break;
    }
    if let Some(rest) = s.strip_prefix("mut ") {
        s = rest.trim_start();
    }
    if let Some(i) = s.find('<') {
        s = &s[..i];
    }
    if let Some(last) = s.rsplit("::").next() {
        s = last;
    }
    s.trim().to_string()
}

/// If `s` starts with a Rust lifetime token (`'ident`), return the remainder
/// after consuming it. Returns `None` otherwise. `'_` and `'static` are
/// handled by the same identifier rule.
fn strip_lifetime(s: &str) -> Option<&str> {
    let rest = s.strip_prefix('\'')?;
    let mut chars = rest.char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let end = chars
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_'))
        .map_or(rest.len(), |(i, _)| i);
    Some(&rest[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ident_passes_through() {
        assert_eq!(tokenise_target("Graph"), "Graph");
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(tokenise_target("  Graph  "), "Graph");
    }

    #[test]
    fn strips_leading_reference() {
        assert_eq!(tokenise_target("&Graph"), "Graph");
    }

    #[test]
    fn strips_mut_reference() {
        assert_eq!(tokenise_target("&mut Graph"), "Graph");
    }

    #[test]
    fn strips_double_reference() {
        assert_eq!(tokenise_target("&&Graph"), "Graph");
    }

    #[test]
    fn strips_module_path() {
        assert_eq!(tokenise_target("domain::Graph"), "Graph");
    }

    #[test]
    fn strips_nested_module_path() {
        assert_eq!(tokenise_target("crate::domain::Graph"), "Graph");
    }

    #[test]
    fn strips_generics_to_primary() {
        assert_eq!(tokenise_target("Result<Graph, ReaderError>"), "Result");
    }

    #[test]
    fn strips_generics_with_nested_path_inside() {
        assert_eq!(tokenise_target("Vec<domain::Graph>"), "Vec");
    }

    #[test]
    fn combined_ref_path_generics() {
        assert_eq!(tokenise_target("&mut domain::Result<T, E>"), "Result");
    }

    #[test]
    fn empty_input_yields_empty() {
        assert_eq!(tokenise_target(""), "");
        assert_eq!(tokenise_target("   "), "");
    }

    #[test]
    fn strips_lifetime_after_reference() {
        assert_eq!(tokenise_target("&'a Graph"), "Graph");
    }

    #[test]
    fn strips_lifetime_with_mut_reference() {
        assert_eq!(tokenise_target("&'a mut Graph"), "Graph");
    }

    #[test]
    fn strips_anonymous_lifetime() {
        assert_eq!(tokenise_target("&'_ Graph"), "Graph");
    }

    #[test]
    fn strips_static_lifetime() {
        assert_eq!(tokenise_target("&'static Graph"), "Graph");
    }

    #[test]
    fn bare_lifetime_tokenises_to_empty() {
        assert_eq!(tokenise_target("'a"), "");
    }
}
