use std::{fs, path::Path};

use handshake_core::sandbox::{bind_specs_from_packet, BindMode, ScopeBinderError};
use serde_json::json;
use tempfile::tempdir;

#[cfg(feature = "wsl2-integration")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "wsl2-integration")]
use handshake_core::sandbox::{
    AdapterId, Command, ImageRef, NetPolicy, ProcessSpec, ResourceLimits, SandboxAdapter,
    SandboxAdapterError, Signal, TrustClass, Wsl2PodmanAdapter, Wsl2PodmanConfig,
};

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

/// MT-052: OS-level proof that the LAYERED bind the scope binder emits is
/// actually enforced by the kernel inside a real rootless-Podman container.
///
/// `bind_specs_from_packet` produces a ReadOnly `/workspace` mount of the whole
/// repo root with one-or-more ReadWrite overlay children (see
/// `work_packet_scope_binder.rs` ~82-96). The unit tests above prove the
/// *shape* of that Vec<BindSpec>; they prove nothing about whether the OS
/// honours it. This test spawns a container with the binder's ACTUAL output
/// (we do NOT hand-build a single bind) and asserts the real kernel behaviour:
///
///   * a write into a path that is only covered by the ReadOnly `/workspace`
///     mount (a sibling NOT inside any allowed overlay) fails with EROFS, and
///   * a write inside a ReadWrite overlay child SUCCEEDS,
///
/// from the same single spawn — i.e. the read-write-inside-read-only layering
/// is enforced at runtime, not merely described in the bind list.
///
/// Gated behind the `wsl2-integration` feature (default CI skips via the
/// per-test `ignore` + the `#[cfg]` on the body's imports), and additionally
/// skips cleanly if no WSL2/Podman host is registered so it never falsely
/// fails on a machine without the runtime. When a host IS present it performs
/// real asserts — there is no auto-pass path.
#[cfg(feature = "wsl2-integration")]
#[tokio::test]
#[cfg_attr(not(feature = "wsl2-integration"), ignore)]
async fn work_packet_scope_binder_integration_wsl2_bind_modes() {
    // 1. Build a repo fixture with BOTH an allowed (writable) subtree and a
    //    sibling that the packet does NOT allow, so the binder yields a
    //    ReadOnly repo-root + a ReadWrite overlay child.
    let repo = tempdir().expect("repo");
    mkdir(repo.path(), "crates/writable/src");
    write(repo.path(), "crates/writable/src/lib.rs");
    // Read-only sibling: present in the repo (so it is under the RO /workspace
    // mount) but NOT in allowed_paths (so no RW overlay covers it).
    mkdir(repo.path(), "crates/readonly_sibling");
    write(repo.path(), "crates/readonly_sibling/locked.txt");
    write_packet(repo.path(), &["crates/writable/**"], &[]);

    // 2. Get the binder's ACTUAL layered output. Do not hand-build binds.
    let binds = bind_specs_from_packet(&repo.path().join("packet.json"), repo.path())
        .expect("scope binds");

    // Sanity: the layering we are about to prove must actually be present.
    assert!(
        binds
            .iter()
            .any(|bind| bind.mode == BindMode::ReadOnly
                && slash(&bind.guest_path) == "/workspace"),
        "binder must emit a ReadOnly /workspace repo-root mount; got {binds:?}"
    );
    let writable_overlay = binds
        .iter()
        .find(|bind| bind.mode == BindMode::ReadWrite)
        .map(|bind| slash(&bind.guest_path))
        .expect("binder must emit at least one ReadWrite overlay child");
    assert!(
        writable_overlay.starts_with("/workspace/"),
        "writable overlay must be nested inside the read-only repo root; got {writable_overlay}"
    );
    // Guest path of the read-only sibling that must remain non-writable.
    let readonly_guest = "/workspace/crates/readonly_sibling";

    // 3. Spawn one container with the binder's exact Vec<BindSpec>. The repo
    //    fixture lives on the Windows host; the WSL2/Podman adapter maps the
    //    host paths under /mnt and applies the binds in order (RO repo root
    //    first, RW overlay children layered on top) at spawn time.
    let adapter = match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::for_distro("Ubuntu")).await {
        Ok(adapter) => adapter,
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("podman unavailable")
                || reason.contains("not registered")
                || reason.contains("WSL") =>
        {
            eprintln!("skipping live WSL2 scope-binder layered-bind integration: {reason}");
            return;
        }
        Err(error) => panic!("WSL2 scope-binder integration setup failed unexpectedly: {error:?}"),
    };

    let handle = adapter
        .spawn(ProcessSpec {
            id: AdapterId::new("scope-binder-layered"),
            image_or_root: ImageRef::new("docker.io/library/alpine:3.20"),
            cmd: vec!["sleep".to_string(), "60".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: binds.clone(),
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            required_capabilities: BTreeSet::new(),
            trust_class: TrustClass::default(),
            metadata: BTreeMap::new(),
        })
        .await
        .expect("spawn container with binder-produced layered binds");

    // 4a. Write into the ReadWrite overlay child MUST succeed.
    let overlay_write = exec_sh(
        &adapter,
        &handle,
        &format!("echo overlay-write > {writable_overlay}/mt052_probe.txt"),
    )
    .await;
    assert_eq!(
        overlay_write.exit_code, 0,
        "write inside the ReadWrite overlay child ({writable_overlay}) must succeed; \
         the layered RW-inside-RO bind is not being enforced as writable"
    );

    // 4b. Write into the ReadOnly sibling (covered only by the RO /workspace
    //     mount) MUST fail, and specifically with EROFS / read-only-fs.
    let readonly_write = exec_sh(
        &adapter,
        &handle,
        &format!("echo blocked > {readonly_guest}/mt052_probe.txt 2>&1"),
    )
    .await;
    assert_ne!(
        readonly_write.exit_code, 0,
        "write into the read-only repo-root region ({readonly_guest}) must fail; \
         the ReadOnly /workspace mount is not being enforced"
    );
    let readonly_stderr = String::from_utf8_lossy(&readonly_write.stdout).to_lowercase();
    assert!(
        readonly_stderr.contains("read-only")
            || readonly_stderr.contains("erofs")
            || readonly_stderr.contains("read only"),
        "read-only-region write must fail with an EROFS/read-only-filesystem error; got: {readonly_stderr:?}"
    );

    // 5. Anti-regression: the overlay write must NOT have leaked onto the host
    //    read-only sibling path, and the overlay file must be visible back on
    //    the host (proving the RW child is a real bind of the host directory).
    assert!(
        repo.path()
            .join("crates/writable/src/mt052_probe.txt")
            .exists(),
        "overlay write must be a real bind-mount of the host writable subtree"
    );
    assert!(
        !repo
            .path()
            .join("crates/readonly_sibling/mt052_probe.txt")
            .exists(),
        "read-only region write must not have reached the host"
    );

    adapter
        .kill(&handle, Signal::Kill)
        .await
        .expect("cleanup scope-binder container");
}

#[cfg(feature = "wsl2-integration")]
async fn exec_sh(
    adapter: &Wsl2PodmanAdapter,
    handle: &handshake_core::sandbox::ProcessHandle,
    script: &str,
) -> handshake_core::sandbox::ExecResult {
    adapter
        .exec(
            handle,
            Command {
                argv: vec!["sh".to_string(), "-c".to_string(), script.to_string()],
                env_overlay: BTreeMap::new(),
                stdin: Some(bytes::Bytes::new()),
                timeout_ms: Some(15_000),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("exec `{script}` failed: {error:?}"))
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
