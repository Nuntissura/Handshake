use std::{fs, path::Path};

use handshake_core::sandbox::{bind_specs_from_packet, BindMode, ScopeBinderError};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn allowed_sandbox_glob_produces_readonly_repo_plus_single_readwrite_overlay() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "src/backend/handshake_core/src/sandbox/nested");
    write(
        repo.path(),
        "src/backend/handshake_core/src/sandbox/nested/mod.rs",
    );
    write_packet(
        repo.path(),
        &["src/backend/handshake_core/src/sandbox/**"],
        &[],
    );

    let binds =
        bind_specs_from_packet(&repo.path().join("packet.json"), repo.path()).expect("scope binds");

    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0].mode, BindMode::ReadOnly);
    assert_eq!(slash(&binds[0].guest_path), "/workspace");
    assert_eq!(binds[1].mode, BindMode::ReadWrite);
    assert_eq!(
        binds[1].host_path,
        repo.path()
            .join("src/backend/handshake_core/src/sandbox")
            .canonicalize()
            .expect("sandbox path")
    );
    assert_eq!(
        slash(&binds[1].guest_path),
        "/workspace/src/backend/handshake_core/src/sandbox"
    );
}

#[test]
fn forbidden_path_overlap_is_rejected() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "src/private");
    write(repo.path(), "src/private/secret.txt");
    write_packet(repo.path(), &["src/**"], &["src/private/**"]);

    let error = bind_specs_from_packet(&repo.path().join("packet.json"), repo.path())
        .expect_err("forbidden overlap must fail");

    match error {
        ScopeBinderError::ForbiddenPathInAllowed {
            path,
            allowed_pattern,
            forbidden_pattern,
        } => {
            assert!(path.ends_with("secret.txt") || path.ends_with("private"));
            assert_eq!(allowed_pattern, "src/**");
            assert_eq!(forbidden_pattern, "src/private/**");
        }
        other => panic!("expected ForbiddenPathInAllowed, got {other:?}"),
    }
}

#[test]
fn nested_allowed_roots_merge_to_common_ancestor_when_no_neighbor_would_widen_scope() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "crates/a/src");
    mkdir(repo.path(), "crates/b/src");
    write(repo.path(), "crates/a/src/lib.rs");
    write(repo.path(), "crates/b/src/lib.rs");
    write_packet(repo.path(), &["crates/a/**", "crates/b/**"], &[]);

    let binds =
        bind_specs_from_packet(&repo.path().join("packet.json"), repo.path()).expect("scope binds");

    let writable = binds
        .iter()
        .filter(|bind| bind.mode == BindMode::ReadWrite)
        .collect::<Vec<_>>();
    assert_eq!(writable.len(), 1);
    assert_eq!(
        writable[0].host_path,
        repo.path().join("crates").canonicalize().expect("crates")
    );
    assert_eq!(slash(&writable[0].guest_path), "/workspace/crates");
}

#[test]
fn common_ancestor_merge_does_not_widen_when_neighbor_exists() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "components/a/src");
    mkdir(repo.path(), "components/b/src");
    mkdir(repo.path(), "components/unowned/src");
    write(repo.path(), "components/a/src/lib.rs");
    write(repo.path(), "components/b/src/lib.rs");
    write(repo.path(), "components/unowned/src/lib.rs");
    write_packet(repo.path(), &["components/a/**", "components/b/**"], &[]);

    let binds =
        bind_specs_from_packet(&repo.path().join("packet.json"), repo.path()).expect("scope binds");

    let writable = binds
        .iter()
        .filter(|bind| bind.mode == BindMode::ReadWrite)
        .map(|bind| bind.guest_path.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    assert_eq!(writable.len(), 2);
    assert!(writable.contains(&"/workspace/components/a".to_string()));
    assert!(writable.contains(&"/workspace/components/b".to_string()));
}

#[test]
fn windows_host_paths_map_to_posix_guest_paths() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "app/src-tauri/src");
    write(repo.path(), "app/src-tauri/src/lib.rs");
    write_packet(repo.path(), &["app/src-tauri/src/**"], &[]);

    let binds =
        bind_specs_from_packet(&repo.path().join("packet.json"), repo.path()).expect("scope binds");

    assert_eq!(slash(&binds[1].guest_path), "/workspace/app/src-tauri/src");
    assert!(!slash(&binds[1].guest_path).contains('\\'));
    assert!(!slash(&binds[1].guest_path).contains(':'));
}

#[test]
fn prose_forbidden_entries_do_not_block_unrelated_path_scope() {
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "src/backend/handshake_core/src/sandbox");
    write(repo.path(), "src/backend/handshake_core/src/sandbox/mod.rs");
    write_packet(
        repo.path(),
        &["src/backend/handshake_core/src/sandbox/**"],
        &["No SQLite authority, cache, offline, fallback, compatibility, or test fixture anywhere per CX-503R."],
    );

    let binds =
        bind_specs_from_packet(&repo.path().join("packet.json"), repo.path()).expect("scope binds");

    assert_eq!(binds.len(), 2);
}

#[tokio::test]
#[cfg_attr(not(feature = "wsl2-integration"), ignore)]
async fn work_packet_scope_binder_integration_wsl2_bind_modes() {
    if std::env::var("HANDSHAKE_WSL2_SCOPE_BINDER_INTEGRATION").is_err() {
        eprintln!("skipping live WSL2 scope binder integration; set HANDSHAKE_WSL2_SCOPE_BINDER_INTEGRATION=1 to run");
        return;
    }
    // Live enforcement is covered by the existing WSL2 Podman read-only bind
    // integration test; this MT keeps the expensive scope-binder variant opt-in.
}

fn write_packet(repo: &Path, allowed_paths: &[&str], forbidden_paths: &[&str]) {
    let packet = json!({
        "scope": {
            "allowed_paths": allowed_paths,
            "forbidden_paths": forbidden_paths
        }
    });
    fs::write(
        repo.join("packet.json"),
        serde_json::to_vec_pretty(&packet).expect("packet json"),
    )
    .expect("write packet");
}

fn mkdir(repo: &Path, rel: &str) {
    fs::create_dir_all(repo.join(rel)).expect("mkdir");
}

fn write(repo: &Path, rel: &str) {
    if let Some(parent) = repo.join(rel).parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(repo.join(rel), b"fixture").expect("write file");
}

fn slash(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
