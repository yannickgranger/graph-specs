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

#[must_use]
pub fn path_under_dir(p: &std::path::Path, name: &str) -> bool {
    p.components().any(|c| c.as_os_str().to_str() == Some(name))
}

#[must_use]
pub fn normalize_context_id(raw: &str) -> String {
    raw.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

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
