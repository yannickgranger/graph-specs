pub const VERSION_LINE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rev ",
    env!("GRAPH_SPECS_BUILD_REV"),
    ")"
);

pub fn stated_rev() -> String {
    format!("graph-specs {VERSION_LINE}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_REV: &str = env!("GRAPH_SPECS_BUILD_REV");

    #[test]
    fn the_version_line_is_composed_from_the_crate_version_and_the_build_rev() {
        assert_eq!(
            VERSION_LINE,
            format!("{} (rev {})", env!("CARGO_PKG_VERSION"), BUILD_REV),
            "the line a lane greps is the crate version and the rev the binary was built from, composed in one place"
        );
    }

    #[test]
    fn the_build_rev_is_a_full_sha_or_a_stated_absence() {
        assert!(
            BUILD_REV == "unknown"
                || (BUILD_REV.len() == 40 && BUILD_REV.chars().all(|c| c.is_ascii_hexdigit())),
            "a build that cannot read a rev says so; it never renders an empty fragment a reader would mistake for a version: {BUILD_REV}"
        );
    }

    #[test]
    fn the_stated_rev_names_the_tool_so_a_quoted_line_is_self_describing() {
        let stated = stated_rev();
        assert!(
            stated.starts_with("graph-specs ") && stated.contains(BUILD_REV),
            "a ledger quoting this line must be able to tell which instrument and which build answered: {stated}"
        );
    }
}
