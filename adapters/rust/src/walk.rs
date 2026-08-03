//! Shared file-tree walking utilities.
//!
//! The concept-level walk ([`crate::RustBackend`]), the pub-fn walk
//! ([`crate::RustReader::extract_pub_fns`]), and the any-visibility anchor
//! walk ([`crate::RustAnchorResolver`]) all traverse the same directory tree
//! under the same exclusion rules and parse each `*.rs` file the same way.

use ports::ReaderError;
use std::io::Read;
use std::path::{Path, PathBuf};
use syn::File;
use walkdir::DirEntry;

/// Directories excluded by NAME. These are semantic exclusions — a
/// directory is skipped because of what it means in a Rust workspace, not
/// because of anything observable inside it. `tests/`, `benches/` and
/// `examples/` hold real source that is deliberately out of the
/// concept-equivalence surface; the dotted entries are tool state.
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

/// Header of the Cache Directory Tagging Specification, which Cargo writes
/// into every build directory it creates.
///
/// Matching on this is what makes the build-directory exclusion a test of
/// the PROPERTY ("this directory declares itself a cache") rather than of
/// the SPELLING ("this directory is called `target`"). A workspace built
/// with `--target-dir target-musl` produces a tree full of generated
/// bindings — `pub` items with no author and no spec — and the name-based
/// list alone cannot see it.
const CACHEDIR_TAG_SIGNATURE: &str = "Signature: 8a477f597d28d172789f06886806bc55";

/// True when `dir` carries a `CACHEDIR.TAG` opening with the canonical
/// signature.
///
/// Absence of the tag, an unreadable tag, and a tag with any other content
/// all mean "not a cache directory" — the walk continues into it. The
/// exclusion only ever fires on a positive, self-declared marker, so a
/// source directory can never be silently dropped from the surface by
/// this check.
fn is_cache_dir(dir: &Path) -> bool {
    let Ok(file) = std::fs::File::open(dir.join("CACHEDIR.TAG")) else {
        return false;
    };
    let mut head = Vec::with_capacity(CACHEDIR_TAG_SIGNATURE.len());
    let read = file
        .take(CACHEDIR_TAG_SIGNATURE.len() as u64)
        .read_to_end(&mut head);
    read.is_ok() && head == CACHEDIR_TAG_SIGNATURE.as_bytes()
}

/// Whether the walk should skip `entry` and everything beneath it.
///
/// Two independent rules, either of which excludes: the name list above,
/// and the self-declared cache marker. Non-directories are never excluded
/// here — file-level filtering (extension, visibility, `cfg` gating) is the
/// caller's, and keeping this guard directory-only is what stops a
/// name rule from reaching a source file that merely shares its prefix.
pub fn is_excluded_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if EXCLUDED_DIRS.iter().any(|ex| name.as_ref() == *ex) {
        return true;
    }
    is_cache_dir(entry.path())
}

/// Read a Rust source file and parse it. Consumes `path` — on error the
/// path is moved into the resulting [`ReaderError`] variant; on success it
/// is handed back alongside the parsed file. This lets the caller avoid
/// cloning the path twice inside its walk loop (one clone per error
/// variant) and keeps the heavy-work of per-file I/O + parsing off the
/// hot path of the walker.
pub fn read_and_parse(path: PathBuf) -> Result<(File, PathBuf), ReaderError> {
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
