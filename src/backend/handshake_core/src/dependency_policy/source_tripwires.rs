use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::{DependencyPolicyError, RuntimeDependencyAllowlist};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceTripwireViolation {
    pub path: String,
    pub class_id: String,
    pub pattern: String,
    pub surface: String,
}

struct BuiltInPattern {
    class_id: &'static str,
    pattern: &'static str,
}

const BUILT_IN_PATTERNS: &[BuiltInPattern] = &[
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "sqlite://",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "sqlite:",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: ".sqlite",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: ".sqlite3",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "rusqlite",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "sqlx::Sqlite",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "SqlitePool",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "SqlitePoolOptions",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "SqliteConnectOptions",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "SqliteDatabase",
    },
    BuiltInPattern {
        class_id: "sqlite",
        pattern: "sqlite fallback",
    },
    BuiltInPattern {
        class_id: "outside_app",
        pattern: "photoshop.exe",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "ollama serve",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "localhost:11434",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "127.0.0.1:11434",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "npm run dev",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "localhost:5173",
    },
    BuiltInPattern {
        class_id: "outside_server_daemon",
        pattern: "127.0.0.1:5173",
    },
];

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "css", "html", "json", "toml",
];

const SKIP_DIRS: &[&str] = &["node_modules", "dist", "dist-harness", "target", ".git"];

const BUILT_IN_SELF_EXEMPT_PATHS: &[&str] = &[
    "src/backend/handshake_core/src/dependency_policy/manifest_tripwires.rs",
    "src/backend/handshake_core/src/dependency_policy/mod.rs",
    "src/backend/handshake_core/src/dependency_policy/source_tripwires.rs",
];

#[derive(Debug, Error)]
pub enum SourceTripwirePolicyError {
    #[error(transparent)]
    Allowlist(#[from] DependencyPolicyError),
    #[error("failed to read source file for dependency-policy scan at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("forbidden dependency-policy source tripwire violations: {violations:?}")]
    Violations {
        violations: Vec<SourceTripwireViolation>,
    },
}

pub fn assert_source_tripwire_policy(repo_root: &Path) -> Result<(), SourceTripwirePolicyError> {
    let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(repo_root)?;
    assert_no_source_tripwire_violations(repo_root, &allowlist)
}

pub fn assert_source_tripwire_policy_for_files<I, P>(
    repo_root: &Path,
    files: I,
) -> Result<(), SourceTripwirePolicyError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let allowlist = RuntimeDependencyAllowlist::load_from_repo_root(repo_root)?;
    let violations = scan_source_file_paths(repo_root, &allowlist, files)?;
    if violations.is_empty() {
        Ok(())
    } else {
        Err(SourceTripwirePolicyError::Violations { violations })
    }
}

pub fn assert_no_source_tripwire_violations(
    repo_root: &Path,
    allowlist: &RuntimeDependencyAllowlist,
) -> Result<(), SourceTripwirePolicyError> {
    let violations = scan_product_source_roots(repo_root, allowlist)?;
    if violations.is_empty() {
        Ok(())
    } else {
        Err(SourceTripwirePolicyError::Violations { violations })
    }
}

pub fn scan_product_source_roots(
    repo_root: &Path,
    allowlist: &RuntimeDependencyAllowlist,
) -> Result<Vec<SourceTripwireViolation>, SourceTripwirePolicyError> {
    let mut violations = BTreeSet::new();
    let self_exempt_paths = self_exempt_paths(allowlist);
    for root_rel in &allowlist.product_scan_roots {
        let root = repo_root.join(repo_relative_to_path(root_rel));
        for file in walk_source_files(&root)? {
            let rel = repo_relative_path(repo_root, &file);
            if self_exempt_paths.contains(&rel) {
                continue;
            }
            let content =
                std::fs::read_to_string(&file).map_err(|source| SourceTripwirePolicyError::Io {
                    path: rel.clone(),
                    source,
                })?;
            for violation in scan_source_text(&rel, &content, allowlist) {
                if is_allowed_policy_exception(&violation, allowlist) {
                    continue;
                }
                violations.insert(violation);
            }
        }
    }
    Ok(violations.into_iter().collect())
}

pub fn scan_source_file_paths<I, P>(
    repo_root: &Path,
    allowlist: &RuntimeDependencyAllowlist,
    files: I,
) -> Result<Vec<SourceTripwireViolation>, SourceTripwirePolicyError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut violations = BTreeSet::new();
    let self_exempt_paths = self_exempt_paths(allowlist);
    for file in files {
        let file = file.as_ref();
        if !is_source_file(file) {
            continue;
        }
        let rel = repo_relative_path(repo_root, file);
        if self_exempt_paths.contains(&rel) {
            continue;
        }
        let content =
            std::fs::read_to_string(file).map_err(|source| SourceTripwirePolicyError::Io {
                path: rel.clone(),
                source,
            })?;
        for violation in scan_source_text(&rel, &content, allowlist) {
            if is_allowed_policy_exception(&violation, allowlist) {
                continue;
            }
            violations.insert(violation);
        }
    }
    Ok(violations.into_iter().collect())
}

pub fn scan_source_text(
    path: &str,
    content: &str,
    allowlist: &RuntimeDependencyAllowlist,
) -> Vec<SourceTripwireViolation> {
    let normalized_path = path.replace('\\', "/");
    let path_lower = normalized_path.to_ascii_lowercase();
    let content_lower = content.to_ascii_lowercase();
    let mut violations = BTreeSet::new();

    for class in &allowlist.forbidden_runtime_dependency_classes {
        for pattern in &class.source_scan_patterns {
            record_if_matches(
                &mut violations,
                &normalized_path,
                &path_lower,
                &content_lower,
                &class.id,
                pattern,
            );
        }
    }

    for built_in in BUILT_IN_PATTERNS {
        record_if_matches(
            &mut violations,
            &normalized_path,
            &path_lower,
            &content_lower,
            built_in.class_id,
            built_in.pattern,
        );
    }

    violations.into_iter().collect()
}

fn self_exempt_paths(allowlist: &RuntimeDependencyAllowlist) -> BTreeSet<String> {
    BUILT_IN_SELF_EXEMPT_PATHS
        .iter()
        .map(|path| path.to_string())
        .chain(allowlist.scan_self_exempt_paths.paths.iter().cloned())
        .collect()
}

fn is_allowed_policy_exception(
    violation: &SourceTripwireViolation,
    allowlist: &RuntimeDependencyAllowlist,
) -> bool {
    if violation.class_id == "docker_default"
        && allowlist
            .docker_opt_in_exceptions
            .iter()
            .any(|exception| violation.path.starts_with(&exception.path_prefix))
    {
        return true;
    }

    allowlist
        .source_tripwire_exceptions
        .entries
        .iter()
        .any(|exception| {
            exception.class_id == violation.class_id
                && exception.path == violation.path
                && exception
                    .patterns
                    .iter()
                    .any(|pattern| pattern.eq_ignore_ascii_case(&violation.pattern))
        })
}

fn walk_source_files(root: &Path) -> Result<Vec<PathBuf>, SourceTripwirePolicyError> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|source| SourceTripwirePolicyError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| SourceTripwirePolicyError::Io {
                path: dir.display().to_string(),
                source,
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|source| SourceTripwirePolicyError::Io {
                    path: path.display().to_string(),
                    source,
                })?;
            if file_type.is_dir() {
                if !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| SKIP_DIRS.contains(&name))
                {
                    stack.push(path);
                }
                continue;
            }
            if is_source_file(&path) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            SOURCE_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn repo_relative_to_path(path: &str) -> PathBuf {
    path.split('/').collect()
}

fn repo_relative_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => part.to_str().map(str::to_string),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn record_if_matches(
    violations: &mut BTreeSet<SourceTripwireViolation>,
    path: &str,
    path_lower: &str,
    content_lower: &str,
    class_id: &str,
    pattern: &str,
) {
    let pattern_lower = pattern.to_ascii_lowercase();
    let path_match = path_lower.contains(&pattern_lower);
    let content_match = content_lower.contains(&pattern_lower);
    if !path_match && !content_match {
        return;
    }
    violations.insert(SourceTripwireViolation {
        path: path.to_string(),
        class_id: class_id.to_string(),
        pattern: pattern.to_string(),
        surface: match (path_match, content_match) {
            (true, true) => "path+content",
            (true, false) => "path",
            (false, true) => "content",
            (false, false) => "none",
        }
        .to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_policy::{repo_root_from_manifest_dir, RuntimeDependencyAllowlist};

    fn allowlist() -> RuntimeDependencyAllowlist {
        RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
            .expect("allowlist loads")
    }

    #[test]
    fn catches_sqlite_fixture_shapes_not_present_in_allowlist_literal_patterns() {
        let violations = scan_source_text(
            "src/backend/handshake_core/tests/fixtures/cache.rs",
            r#"let db = "tmp/cache.sqlite3"; struct SqliteDatabase;"#,
            &allowlist(),
        );
        assert!(violations.iter().any(|v| v.class_id == "sqlite"));
    }

    #[test]
    fn catches_external_default_proof_dependencies() {
        let violations = scan_source_text(
            "src/backend/handshake_core/src/proof.rs",
            r#"redis://127.0.0.1:6379 docker compose up https://cdn.jsdelivr.net/pkg.js"#,
            &allowlist(),
        );
        let classes: BTreeSet<&str> = violations.iter().map(|v| v.class_id.as_str()).collect();
        assert!(classes.contains("outside_server_daemon"));
        assert!(classes.contains("docker_default"));
        assert!(classes.contains("cdn_runtime_asset"));
    }

    #[test]
    fn honors_exact_pattern_scoped_source_tripwire_exceptions() {
        let allowlist = allowlist();
        let violations =
            scan_source_file_paths(
                &repo_root_from_manifest_dir(),
                &allowlist,
                [repo_root_from_manifest_dir()
                    .join("src/backend/handshake_core/src/llm/registry.rs")],
            )
            .expect("source tripwire scan runs");
        assert!(
            violations.is_empty(),
            "documented local model-runtime exception should not become a broad source violation: {violations:?}"
        );

        let probe = repo_root_from_manifest_dir()
            .join("src/backend/handshake_core/src/model_runtime/forbidden_probe.rs");
        let violations = scan_source_text(
            &repo_relative_path(&repo_root_from_manifest_dir(), &probe),
            r#"const URL: &str = "http://localhost:11434";"#,
            &allowlist,
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.class_id == "outside_server_daemon"),
            "same source pattern outside the exact exception path must still fail closed: {violations:?}"
        );
    }
}
