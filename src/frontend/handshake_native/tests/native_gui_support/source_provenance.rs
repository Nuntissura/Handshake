//! Source and binary identity for worktree and clean-export proof runs.
#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

pub fn configured_source_sha() -> Option<String> {
    std::env::var_os("HANDSHAKE_PROOF_SOURCE_SHA").map(|value| {
        let value = value
            .into_string()
            .expect("HANDSHAKE_PROOF_SOURCE_SHA is UTF-8");
        assert!(
            matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "HANDSHAKE_PROOF_SOURCE_SHA must be a full commit hash"
        );
        value
    })
}

pub fn repo_root() -> PathBuf {
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate is inside product root")
        .canonicalize()
        .expect("product root exists");
    let root = if let Some(root) = std::env::var_os("HANDSHAKE_PROOF_REPO_ROOT") {
        PathBuf::from(root)
            .canonicalize()
            .expect("HANDSHAKE_PROOF_REPO_ROOT exists")
    } else if configured_source_sha().is_some() {
        compiled.clone()
    } else {
        PathBuf::from(git(&compiled, &["rev-parse", "--show-toplevel"]))
            .canonicalize()
            .expect("Git product root exists")
    };
    assert_eq!(
        root, compiled,
        "proof must describe the source tree compiled into this test"
    );
    root
}

fn git(root: &Path, args: &[&str]) -> String {
    assert!(
        configured_source_sha().is_none(),
        "export provenance must not invoke Git"
    );
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run Git provenance");
    assert!(
        output.status.success(),
        "Git provenance failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Git output UTF-8")
        .trim()
        .to_owned()
}

pub fn source_sha() -> String {
    configured_source_sha().unwrap_or_else(|| git(&repo_root(), &["rev-parse", "HEAD"]))
}

pub fn sha256_file(path: &Path) -> String {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer).expect("read proof input");
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    format!("{:x}", digest.finalize())
}

fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, String>) {
    assert!(
        path.canonicalize()
            .expect("canonical source input")
            .starts_with(root),
        "source input escapes product root"
    );
    let metadata = std::fs::symlink_metadata(path)
        .unwrap_or_else(|e| panic!("inspect {}: {e}", path.display()));
    assert!(
        !metadata.file_type().is_symlink(),
        "proof source cannot escape through a symlink: {}",
        path.display()
    );
    if metadata.is_file() {
        let relative = path
            .strip_prefix(root)
            .expect("source remains under product root")
            .to_str()
            .expect("source path UTF-8")
            .replace('\\', "/");
        files.insert(relative, sha256_file(path));
    } else if metadata.is_dir() {
        for entry in std::fs::read_dir(path).expect("read source directory") {
            let entry = entry.expect("source directory entry");
            // Authority junctions and mutable dependency/build trees are never source inputs.
            if matches!(
                entry.file_name().to_str(),
                Some(".git" | ".GOV" | "node_modules" | "target" | "__pycache__")
            ) {
                continue;
            }
            visit(root, &entry.path(), files);
        }
    }
}

pub fn source_files(paths: &[&str]) -> BTreeMap<String, String> {
    let root = repo_root();
    let mut files = BTreeMap::new();
    for path in paths {
        let path = root
            .join(path)
            .canonicalize()
            .unwrap_or_else(|e| panic!("proof input {path}: {e}"));
        assert!(
            path.starts_with(&root),
            "proof input escapes product source"
        );
        visit(&root, &path, &mut files);
    }
    assert!(
        !files.is_empty(),
        "proof source inventory must not be empty"
    );
    files
}

pub fn source_blob(path: &str) -> String {
    if configured_source_sha().is_some() {
        format!("sha256:{}", sha256_file(&repo_root().join(path)))
    } else {
        git(&repo_root(), &["rev-parse", &format!("HEAD:{path}")])
    }
}

pub fn source_tree() -> String {
    if configured_source_sha().is_some() {
        let files = source_files(&["."]);
        format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&files).expect("source manifest JSON"))
        )
    } else {
        git(&repo_root(), &["rev-parse", "HEAD^{tree}"])
    }
}

pub fn export_candidate() -> (String, serde_json::Value) {
    let source_sha = configured_source_sha().expect("explicit export source commit");
    let files = source_files(&["."]);
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&files).expect("source manifest JSON"))
    );
    let identity = format!("{source_sha}-sha256-{hash}");
    (
        identity.clone(),
        serde_json::json!({
            "identity": identity, "head_sha": source_sha, "candidate_sha256": hash,
            "source_identity_kind": "explicit_commit_with_observed_file_sha256_manifest",
            "files": files,
        }),
    )
}

pub fn target_root() -> PathBuf {
    let target = PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR")
            .expect("CARGO_TARGET_DIR must name a scoped proof target"),
    )
    .canonicalize()
    .expect("scoped Cargo target exists");
    validate_target(&target);
    target
}

pub fn validate_target(target: &Path) {
    let artifacts = PathBuf::from(
        std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT").expect("HANDSHAKE_ARTIFACTS_ROOT required"),
    )
    .canonicalize()
    .expect("artifact root exists");
    let parts = target
        .strip_prefix(&artifacts)
        .expect("target must be below artifact root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        parts.len() == 4
            && parts[0].starts_with("WP-")
            && parts[1].starts_with("MT-")
            && !parts[2].is_empty()
            && parts[3] == "target",
        "target must use WP/MT/owner/target: {}",
        target.display()
    );
}

pub fn binary(env_key: &str, stem: &str) -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(env_key) {
        let path = PathBuf::from(value)
            .canonicalize()
            .unwrap_or_else(|e| panic!("{env_key}: {e}"));
        assert!(path.is_file(), "{env_key} must name a binary");
        let target = path
            .parent()
            .and_then(Path::parent)
            .expect("binary target/profile path");
        validate_target(target);
        return Some(path);
    }
    let target = target_root();
    let name = if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    };
    ["debug", "release", "release-native"]
        .into_iter()
        .map(|profile| target.join(profile).join(&name))
        .find(|path| path.is_file())
}

pub fn export_binary_provenance(binary: &Path, crate_paths: &[&str]) -> serde_json::Value {
    let root = repo_root();
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let paths = crate_paths
        .iter()
        .map(|p| {
            let canonical = crate_root
                .join(p)
                .canonicalize()
                .expect("binary source exists");
            canonical
                .strip_prefix(&root)
                .expect("binary source inside root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    let files = source_files(&paths.iter().map(String::as_str).collect::<Vec<_>>());
    let binary = binary.canonicalize().expect("binary exists");
    let metadata = std::fs::metadata(&binary).expect("binary metadata");
    let modified = metadata.modified().expect("binary mtime");
    for path in files.keys() {
        assert!(
            modified
                >= std::fs::metadata(root.join(path))
                    .expect("input metadata")
                    .modified()
                    .expect("input mtime"),
            "binary predates source input {path}"
        );
    }
    serde_json::json!({"canonical_path": binary, "sha256": sha256_file(&binary), "size_bytes": metadata.len(),
        "source_sha": source_sha(), "observed_source_count": files.len(), "observed_inputs": files,
        "not_older_than_all_observed_sources": true, "source_identity_kind": "export_file_sha256_manifest"})
}
