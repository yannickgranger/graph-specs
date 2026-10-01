use std::path::{Path, PathBuf};

use ports::ReaderError;

const DEFAULT_VENDOR_DIR: &str = "vendor";
const AUTOLOAD_BLOCKS: [&str; 2] = ["autoload", "autoload-dev"];
const ROOT_KEYS: [&str; 4] = ["psr-4", "psr-0", "classmap", "files"];

pub fn read(workspace_root: &Path) -> Result<Option<serde_json::Value>, ReaderError> {
    let path = workspace_root.join("composer.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(ReaderError::IoFailed {
                path,
                cause: e.to_string(),
            })
        }
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| ReaderError::ParseFailed {
            path,
            line: 0,
            message: format!(
                "composer.json is not readable JSON, so what the project declares as source has no answer: {e}"
            ),
        })
}

pub fn vendor_dir(workspace_root: &Path) -> Result<String, ReaderError> {
    Ok(read(workspace_root)?
        .as_ref()
        .and_then(|manifest| manifest.get("config"))
        .and_then(|config| config.get("vendor-dir"))
        .and_then(serde_json::Value::as_str)
        .map(|raw| raw.trim().trim_start_matches("./").trim_matches('/'))
        .filter(|raw| !raw.is_empty())
        .unwrap_or(DEFAULT_VENDOR_DIR)
        .to_owned())
}

pub fn declared_roots(workspace_root: &Path) -> Result<Option<Vec<PathBuf>>, ReaderError> {
    let Some(manifest) = read(workspace_root)? else {
        return Ok(None);
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for block in AUTOLOAD_BLOCKS {
        let Some(block) = manifest.get(block) else {
            continue;
        };
        for key in ROOT_KEYS {
            match block.get(key) {
                Some(serde_json::Value::Object(map)) => {
                    for value in map.values() {
                        push_paths(value, &mut out);
                    }
                }
                Some(value) => push_paths(value, &mut out),
                None => {}
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(Some(out))
}

fn push_paths(value: &serde_json::Value, out: &mut Vec<PathBuf>) {
    match value {
        serde_json::Value::String(raw) => {
            if let Some(path) = normalized(raw) {
                out.push(path);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                push_paths(value, out);
            }
        }
        _ => {}
    }
}

fn normalized(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim().trim_start_matches("./").trim_end_matches('/');
    (!trimmed.is_empty() && trimmed != ".").then(|| PathBuf::from(trimmed))
}
