use std::path::{Path, PathBuf};

use ports::ReaderError;

const CONTEXTS_DIR: &str = "specs/contexts";
const SPECS_DIR: &str = "specs";

pub fn declaring_a_context(workspace_root: &Path) -> Result<Vec<PathBuf>, ReaderError> {
    let vendor = workspace_root.join(crate::manifest::vendor_dir(workspace_root)?);
    if !vendor.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for namespace in sorted_dirs(&vendor)? {
        for root in sorted_dirs(&namespace)? {
            if !root.join("composer.json").is_file() || !declares_a_context(&root)? {
                continue;
            }
            out.push(root.join(SPECS_DIR));
        }
    }
    Ok(out)
}

fn declares_a_context(root: &Path) -> Result<bool, ReaderError> {
    let contexts = root.join(CONTEXTS_DIR);
    let entries = match std::fs::read_dir(&contexts) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(ReaderError::IoFailed {
                path: contexts,
                cause: e.to_string(),
            })
        }
    };
    for entry in entries {
        let path = entry
            .map_err(|e| ReaderError::IoFailed {
                path: contexts.clone(),
                cause: e.to_string(),
            })?
            .path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn sorted_dirs(dir: &Path) -> Result<Vec<PathBuf>, ReaderError> {
    let read = std::fs::read_dir(dir).map_err(|e| ReaderError::IoFailed {
        path: dir.to_path_buf(),
        cause: e.to_string(),
    })?;
    let mut out = Vec::new();
    for entry in read {
        let path = entry
            .map_err(|e| ReaderError::IoFailed {
                path: dir.to_path_buf(),
                cause: e.to_string(),
            })?
            .path();
        if path.is_dir() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
#[path = "installed_tests.rs"]
mod tests;
