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
