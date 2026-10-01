use std::path::{Path, PathBuf};

use ports::ReaderError;

const DEFAULT_VENDOR_DIR: &str = "vendor";
const CONTEXTS_DIR: &str = "specs/contexts";
const SPECS_DIR: &str = "specs";

pub fn declaring_a_context(workspace_root: &Path) -> Result<Vec<PathBuf>, ReaderError> {
    let vendor = workspace_root.join(vendor_dir(workspace_root)?);
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

fn vendor_dir(workspace_root: &Path) -> Result<String, ReaderError> {
    let manifest_path = workspace_root.join("composer.json");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DEFAULT_VENDOR_DIR.to_owned())
        }
        Err(e) => {
            return Err(ReaderError::IoFailed {
                path: manifest_path,
                cause: e.to_string(),
            })
        }
    };
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest).map_err(|e| ReaderError::ParseFailed {
            path: manifest_path,
            line: 0,
            message: format!(
                "composer.json is not readable JSON, so where the workspace installs its packages has no answer: {e}"
            ),
        })?;
    Ok(configured_vendor_dir(&manifest))
}

fn configured_vendor_dir(manifest: &serde_json::Value) -> String {
    manifest
        .get("config")
        .and_then(|config| config.get("vendor-dir"))
        .and_then(serde_json::Value::as_str)
        .map(|raw| raw.trim().trim_start_matches("./").trim_matches('/'))
        .filter(|raw| !raw.is_empty())
        .unwrap_or(DEFAULT_VENDOR_DIR)
        .to_owned()
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
