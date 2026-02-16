use std::path::{Component, Path, PathBuf};

use super::errors::{McpError, McpResult};

fn contains_parent_dir(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn contains_prefix_component(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::Prefix(_)))
}

fn ensure_no_symlinks(root: &Path, target: &Path) -> McpResult<()> {
    let root = root
        .canonicalize()
        .map_err(|e| McpError::SecurityViolation(e.to_string()))?;

    let rel = target.strip_prefix(&root).map_err(|_| {
        McpError::SecurityViolation("path escapes allowed root (strip_prefix failed)".to_string())
    })?;

    let mut current = root;
    for component in rel.components() {
        current.push(component);
        let meta = std::fs::symlink_metadata(&current)
            .map_err(|e| McpError::SecurityViolation(e.to_string()))?;
        if meta.file_type().is_symlink() {
            return Err(McpError::SecurityViolation(format!(
                "symlink rejected: {}",
                current.display()
            )));
        }
    }

    Ok(())
}

pub fn canonicalize_under_roots(path_str: &str, allowed_roots: &[PathBuf]) -> McpResult<PathBuf> {
    if path_str.trim().is_empty() {
        return Err(McpError::SecurityViolation("path is empty".to_string()));
    }

    let requested = PathBuf::from(path_str);
    if contains_prefix_component(&requested) && !requested.is_absolute() {
        return Err(McpError::SecurityViolation(
            "path prefix component rejected".to_string(),
        ));
    }
    if contains_parent_dir(&requested) {
        return Err(McpError::SecurityViolation(
            "path traversal rejected".to_string(),
        ));
    }

    let mut errors: Vec<String> = Vec::new();
    for root in allowed_roots {
        let canonical_root = match root.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("root {}: {e}", root.display()));
                continue;
            }
        };

        let candidate = if requested.is_absolute() {
            requested.clone()
        } else {
            canonical_root.join(&requested)
        };

        let canonical_candidate = match candidate.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("candidate {}: {e}", candidate.display()));
                continue;
            }
        };

        if !canonical_candidate.starts_with(&canonical_root) {
            errors.push(format!(
                "candidate {} escapes root {}",
                canonical_candidate.display(),
                canonical_root.display()
            ));
            continue;
        }

        ensure_no_symlinks(&canonical_root, &canonical_candidate)?;
        return Ok(canonical_candidate);
    }

    Err(McpError::SecurityViolation(format!(
        "path is not permitted under allowed roots ({})",
        errors.join("; ")
    )))
}
