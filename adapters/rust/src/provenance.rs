use std::path::Path;

pub fn find_owned_unit(file_path: &Path, root: &Path) -> Option<String> {
    let mut dir = file_path.parent()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return unit_name_for(dir, root);
        }
        let parent = dir.parent()?;
        if parent == dir {
            return None;
        }
        dir = parent;
    }
}

fn unit_name_for(dir: &Path, root: &Path) -> Option<String> {
    if let Ok(rel) = dir.strip_prefix(root) {
        let s = rel.to_string_lossy();
        if !s.is_empty() {
            return Some(s.replace('\\', "/"));
        }
    }
    dir.file_name().and_then(|n| n.to_str()).map(str::to_owned)
}

pub fn module_path_of(file_path: &Path, root: &Path, unit: Option<&str>) -> Option<String> {
    let unit = unit?;
    let rel = file_path
        .strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let after_unit = rel
        .strip_prefix(unit)
        .unwrap_or(&rel)
        .trim_start_matches('/');
    let after_src = after_unit.strip_prefix("src/").unwrap_or(after_unit);
    let stem = after_src.strip_suffix(".rs").unwrap_or(after_src);
    let mut segments: Vec<&str> = stem.split('/').filter(|s| !s.is_empty()).collect();
    if matches!(segments.last().copied(), Some("lib" | "mod" | "main")) {
        segments.pop();
    }
    if segments.is_empty() {
        Some(unit.to_owned())
    } else {
        Some(format!("{unit}::{}", segments.join("::")))
    }
}
