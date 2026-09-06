use ports::ReaderError;
use std::io::Read;
use std::path::Path;
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

pub fn parse_text(source: &str, path: &std::path::Path) -> Result<File, ReaderError> {
    syn::parse_file(source).map_err(|e| ReaderError::ParseFailed {
        path: path.to_path_buf(),
        line: e.span().start().line,
        message: e.to_string(),
    })
}
