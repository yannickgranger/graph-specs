//! Line-offset helpers shared by the concept and context parsers.
//!
//! Factored out during #24 so `parse_context_file` can reuse them without
//! pulling in the concept-parser's `SectionState` (which is shaped for
//! H2/H3 + fenced-rust + bullet-edge dispatch, not context-file H1 +
//! four-section structure).

#[must_use]
pub fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

#[must_use]
pub fn line_of_offset(starts: &[usize], offset: usize) -> usize {
    match starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i.max(1),
    }
}

/// Returns true iff any ancestor component of `p` equals `name`.
///
/// Used by the concept and context readers to skip each other's files
/// when both are pointed at the same spec root (v0.4 `--specs specs/`).
#[must_use]
pub fn path_under_dir(p: &std::path::Path, name: &str) -> bool {
    p.components().any(|c| c.as_os_str().to_str() == Some(name))
}
