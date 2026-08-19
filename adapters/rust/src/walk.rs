use ports::ReaderError;
use std::io::Read;
use std::path::{Path, PathBuf};
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

const CACHEDIR_TAG_SIGNATURE: &str = "Signature: 8a477f597d28d172789f06886806bc55";

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
