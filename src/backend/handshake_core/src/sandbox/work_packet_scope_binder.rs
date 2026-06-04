use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use thiserror::Error;

use super::{BindMode, BindSpec};

const WORKSPACE_GUEST_ROOT: &str = "/workspace";

#[derive(Debug, Error)]
pub enum ScopeBinderError {
    #[error("failed to read packet {}: {source}", path.display())]
    PacketRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse packet {}: {source}", path.display())]
    PacketParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("repo root must exist and be a directory: {}", path.display())]
    RepoRootInvalid { path: PathBuf },
    #[error("scope path `{pattern}` is absolute; packet scope paths must be repo-relative")]
    AbsoluteScopePath { pattern: String },
    #[error("scope path `{pattern}` escapes the repo root")]
    ScopePathEscapesRepo { pattern: String },
    #[error("scope glob `{pattern}` is invalid: {source}")]
    InvalidScopeGlob {
        pattern: String,
        source: globset::Error,
    },
    #[error(
        "allowed scope path {} from `{allowed_pattern}` matches forbidden pattern `{forbidden_pattern}`",
        path.display()
    )]
    ForbiddenPathInAllowed {
        path: PathBuf,
        allowed_pattern: String,
        forbidden_pattern: String,
    },
    #[error("allowed scope bind would make the full repo writable")]
    WritableRepoRoot,
}

#[derive(Debug, Deserialize)]
struct PacketScopeDocument {
    #[serde(default)]
    scope: PacketScope,
}

#[derive(Debug, Default, Deserialize)]
struct PacketScope {
    #[serde(default)]
    allowed_paths: Vec<String>,
    #[serde(default)]
    forbidden_paths: Vec<String>,
}

#[derive(Debug)]
struct ScopeGlob {
    pattern: String,
    matcher: GlobMatcher,
}

pub fn bind_specs_from_packet(
    packet_path: &Path,
    repo_root: &Path,
) -> Result<Vec<BindSpec>, ScopeBinderError> {
    let packet = read_packet_scope(packet_path)?;
    let repo_root = canonical_repo_root(repo_root)?;
    let allowed = collect_allowed_roots(&packet.scope.allowed_paths, &repo_root)?;
    let forbidden = compile_scope_globs(&packet.scope.forbidden_paths)?;
    reject_forbidden_overlap(&allowed.matched_paths, &forbidden, &repo_root)?;

    let writable_roots = safe_merge_roots(allowed.bind_roots, &repo_root)?;
    let mut binds = vec![BindSpec {
        host_path: repo_root.clone(),
        guest_path: PathBuf::from(WORKSPACE_GUEST_ROOT),
        mode: BindMode::ReadOnly,
    }];
    for root in writable_roots {
        if root == repo_root {
            return Err(ScopeBinderError::WritableRepoRoot);
        }
        binds.push(BindSpec {
            guest_path: guest_path_for_host(&root, &repo_root),
            host_path: root,
            mode: BindMode::ReadWrite,
        });
    }
    Ok(binds)
}

fn read_packet_scope(packet_path: &Path) -> Result<PacketScopeDocument, ScopeBinderError> {
    let bytes = fs::read(packet_path).map_err(|source| ScopeBinderError::PacketRead {
        path: packet_path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| ScopeBinderError::PacketParse {
        path: packet_path.to_path_buf(),
        source,
    })
}

fn canonical_repo_root(repo_root: &Path) -> Result<PathBuf, ScopeBinderError> {
    if !repo_root.is_dir() {
        return Err(ScopeBinderError::RepoRootInvalid {
            path: repo_root.to_path_buf(),
        });
    }
    repo_root
        .canonicalize()
        .map_err(|_| ScopeBinderError::RepoRootInvalid {
            path: repo_root.to_path_buf(),
        })
}

#[derive(Debug, Default)]
struct AllowedRoots {
    bind_roots: Vec<PathBuf>,
    matched_paths: Vec<AllowedPathMatch>,
}

#[derive(Debug)]
struct AllowedPathMatch {
    path: PathBuf,
    allowed_pattern: String,
}

fn collect_allowed_roots(
    patterns: &[String],
    repo_root: &Path,
) -> Result<AllowedRoots, ScopeBinderError> {
    let mut roots = BTreeSet::new();
    let mut matched_paths = BTreeSet::new();

    for raw in patterns {
        let pattern = normalize_scope_pattern(raw);
        if pattern.is_empty() {
            continue;
        }
        validate_relative_pattern(&pattern)?;

        if let Some(prefix) = pattern.strip_suffix("/**") {
            let root = repo_root.join(path_from_normalized(prefix));
            if root.exists() {
                let root = canonicalize_existing(&root);
                roots.insert(root.clone());
                collect_descendants(&root, &pattern, &mut matched_paths);
            }
            continue;
        }

        if !contains_glob_meta(&pattern) {
            let path = repo_root.join(path_from_normalized(&pattern));
            if path.exists() {
                let path = canonicalize_existing(&path);
                roots.insert(path.clone());
                if path.is_dir() {
                    collect_descendants(&path, &pattern, &mut matched_paths);
                } else {
                    matched_paths.insert((path, pattern.clone()));
                }
            }
            continue;
        }

        let matcher = compile_scope_glob(&pattern)?;
        for path in walk_repo(repo_root) {
            let relative = relative_slash(&path, repo_root);
            if matcher.is_match(&relative) {
                let path = canonicalize_existing(&path);
                roots.insert(path.clone());
                if path.is_dir() {
                    collect_descendants(&path, &pattern, &mut matched_paths);
                } else {
                    matched_paths.insert((path, pattern.clone()));
                }
            }
        }
    }

    Ok(AllowedRoots {
        bind_roots: roots.into_iter().collect(),
        matched_paths: matched_paths
            .into_iter()
            .map(|(path, allowed_pattern)| AllowedPathMatch {
                path,
                allowed_pattern,
            })
            .collect(),
    })
}

fn compile_scope_globs(patterns: &[String]) -> Result<Vec<ScopeGlob>, ScopeBinderError> {
    patterns
        .iter()
        .map(|raw| normalize_scope_pattern(raw))
        .filter(|pattern| !pattern.is_empty())
        .map(|pattern| {
            validate_relative_pattern(&pattern)?;
            Ok(ScopeGlob {
                matcher: compile_scope_glob(&pattern)?,
                pattern,
            })
        })
        .collect()
}

fn compile_scope_glob(pattern: &str) -> Result<GlobMatcher, ScopeBinderError> {
    Glob::new(pattern)
        .map_err(|source| ScopeBinderError::InvalidScopeGlob {
            pattern: pattern.to_string(),
            source,
        })
        .map(|glob| glob.compile_matcher())
}

fn reject_forbidden_overlap(
    allowed_paths: &[AllowedPathMatch],
    forbidden: &[ScopeGlob],
    repo_root: &Path,
) -> Result<(), ScopeBinderError> {
    for allowed in allowed_paths {
        let relative = relative_slash(&allowed.path, repo_root);
        for forbidden_glob in forbidden {
            if forbidden_glob.matcher.is_match(&relative) {
                return Err(ScopeBinderError::ForbiddenPathInAllowed {
                    path: allowed.path.clone(),
                    allowed_pattern: allowed.allowed_pattern.clone(),
                    forbidden_pattern: forbidden_glob.pattern.clone(),
                });
            }
        }
    }
    Ok(())
}

fn safe_merge_roots(
    roots: Vec<PathBuf>,
    repo_root: &Path,
) -> Result<Vec<PathBuf>, ScopeBinderError> {
    let mut roots = remove_redundant_children(roots);
    loop {
        let mut by_parent: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        for root in &roots {
            if let Some(parent) = root.parent() {
                by_parent
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(root.clone());
            }
        }

        let mut changed = false;
        for (parent, group) in by_parent {
            if group.len() < 2 || parent == repo_root {
                continue;
            }
            if all_direct_children_covered(&parent, &group)? {
                roots.retain(|root| !group.contains(root));
                roots.push(parent);
                roots = remove_redundant_children(roots);
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    roots.sort();
    Ok(roots)
}

fn all_direct_children_covered(parent: &Path, group: &[PathBuf]) -> Result<bool, ScopeBinderError> {
    for entry in fs::read_dir(parent).map_err(|source| ScopeBinderError::PacketRead {
        path: parent.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| ScopeBinderError::PacketRead {
            path: parent.to_path_buf(),
            source,
        })?;
        let child = canonicalize_existing(&entry.path());
        if !group
            .iter()
            .any(|root| child == *root || child.starts_with(root))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn remove_redundant_children(mut roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots.sort();
    roots.dedup();
    let mut kept: Vec<PathBuf> = Vec::new();
    'root: for root in roots {
        for existing in &kept {
            if root == *existing || root.starts_with(existing) {
                continue 'root;
            }
        }
        kept.push(root);
    }
    kept
}

fn collect_descendants(root: &Path, allowed_pattern: &str, out: &mut BTreeSet<(PathBuf, String)>) {
    out.insert((root.to_path_buf(), allowed_pattern.to_string()));
    if root.is_dir() {
        for path in walk_repo(root) {
            out.insert((canonicalize_existing(&path), allowed_pattern.to_string()));
        }
    }
}

fn walk_repo(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if matches!(name.as_ref(), ".git" | "target" | "node_modules") {
                    continue;
                }
                files.push(path.clone());
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn guest_path_for_host(host_path: &Path, repo_root: &Path) -> PathBuf {
    let relative = relative_slash(host_path, repo_root);
    if relative.is_empty() {
        PathBuf::from(WORKSPACE_GUEST_ROOT)
    } else {
        PathBuf::from(format!("{WORKSPACE_GUEST_ROOT}/{relative}"))
    }
}

fn relative_slash(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn normalize_scope_pattern(value: &str) -> String {
    let mut value = value.trim().replace('\\', "/").trim().to_string();
    for prefix in ["- ", "* "] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.trim().to_string();
            break;
        }
    }
    value
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn validate_relative_pattern(pattern: &str) -> Result<(), ScopeBinderError> {
    let path = Path::new(pattern);
    if path.is_absolute() || pattern.starts_with('/') || pattern.get(1..2) == Some(":") {
        return Err(ScopeBinderError::AbsoluteScopePath {
            pattern: pattern.to_string(),
        });
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(ScopeBinderError::ScopePathEscapesRepo {
            pattern: pattern.to_string(),
        });
    }
    Ok(())
}

fn contains_glob_meta(pattern: &str) -> bool {
    pattern
        .chars()
        .any(|ch| matches!(ch, '*' | '?' | '[' | '{'))
}

fn path_from_normalized(pattern: &str) -> PathBuf {
    pattern.split('/').collect()
}

fn canonicalize_existing(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
