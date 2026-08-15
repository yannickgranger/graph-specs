//! Line-offset helpers shared by the concept and context parsers.
//!
//! Factored out so `parse_context_file` can reuse them without pulling in
//! the concept-parser's `SectionState` (which is shaped for H2/H3 +
//! fenced-rust + bullet-edge dispatch, not context-file H1 +
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

/// Normalise a heading's raw text into a bounded-context identifier:
/// lowercase, with internal whitespace runs collapsed to a single `-`.
/// `# AC verifier` → `ac-verifier`.
///
/// The single normalisation rule is applied to **both** sides of the
/// ladder — the `specs/contexts/` H1 ([`crate::contexts`]) and the
/// `specs/concepts/` H1 ([`crate::tree`]) — so a context declared as
/// `# AC verifier` and a concept file headed `# AC verifier` resolve to
/// the same identifier.
#[must_use]
pub fn normalize_context_id(raw: &str) -> String {
    raw.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

/// Returns `true` iff `id` is a well-formed context identifier — one or
/// more `-`-separated segments, each a non-empty run of ASCII lowercase
/// alphanumerics (the shape [`normalize_context_id`] produces for a real
/// context name).
///
/// A normalised H1 that fails this carried punctuation or other
/// non-identifier characters (e.g. `core-concepts:-equivalence` from a
/// descriptive title) — it is prose, not a context name, and the tree
/// assembler rejects it.
#[must_use]
pub fn is_context_identifier(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
