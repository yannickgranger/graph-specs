//! Shared file-tree walking utilities.
//!
//! The concept-level walk ([`crate::RustBackend`]), the pub-fn walk
//! ([`crate::RustReader::extract_pub_fns`]), and the any-visibility anchor
//! walk ([`crate::RustAnchorResolver`]) all traverse the same directory tree
//! under the same exclusion rules and parse each `*.rs` file the same way.

use ports::ReaderError;
use std::path::PathBuf;
use syn::File;
use walkdir::DirEntry;

const EXCLUDED_DIRS: &[&str] = &[
    "target",
    ".git",
    ".claude",
    ".proofs",
    "tests",
    "benches",
    "examples",
    "node_modules",
];

pub(super) fn is_excluded_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    EXCLUDED_DIRS.iter().any(|ex| name.as_ref() == *ex)
}

/// Read a Rust source file and parse it. Consumes `path` — on error the
/// path is moved into the resulting [`ReaderError`] variant; on success it
/// is handed back alongside the parsed file. This lets the caller avoid
/// cloning the path twice inside its walk loop (one clone per error
/// variant) and keeps the heavy-work of per-file I/O + parsing off the
/// hot path of the walker.
pub(super) fn read_and_parse(path: PathBuf) -> Result<(File, PathBuf), ReaderError> {
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return Err(ReaderError::IoFailed {
                path,
                cause: e.to_string(),
            });
        }
    };
    match syn::parse_file(&source) {
        Ok(f) => Ok((f, path)),
        Err(e) => Err(ReaderError::ParseFailed {
            path,
            line: e.span().start().line,
            message: e.to_string(),
        }),
    }
}
