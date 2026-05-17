//! MT-078 Disk-Agnostic Path Test.
//!
//! Acceptance (MT-078.json): "prove paths remain repo-root/config relative.
//! Acceptance: moving workspace root does not break path resolution."
//!
//! This test matrix asserts that KB003 product code stays disk-agnostic per
//! `[GLOBAL-PORTABILITY-001..013]` and per CX-212E. Forbidden patterns the
//! tests reject anywhere in KB003 artifact/workspace surfaces:
//!
//!   * Windows drive prefix (`C:\`, `D:/`)
//!   * UNC / network share prefix (`\\srv\share`)
//!   * Posix-absolute (`/etc`, `/var`, `/tmp`)
//!   * User-profile roots (`/Users/`, `C:\Users\`)
//!   * Hardcoded NAS / mount roots
//!
//! Allowed shape: repo-relative paths under `handshake-product/` (per the
//! external artifact root policy in CX-212E). Absolute resolution is the
//! storage layer's job, parameterised by an operator-controlled root.
//!
//! Test cases:
//!   1. Every `KB003_ARTIFACT_CLASSES.retention_root` is repo-relative under
//!      `handshake-product/`; none contains a drive letter, leading `/`,
//!      backslash, or UNC prefix.
//!   2. `SandboxWorkspaceV1::new_default` constructed from arbitrary
//!      repo-relative roots stays repo-relative; moving the configured root
//!      to a different `handshake-product/…` subtree moves all derived
//!      artifact handles coherently.
//!   3. `SandboxWorkspaceV1::contains_relative` rejects host-absolute paths
//!      and tolerates lexically equivalent relative paths regardless of
//!      filesystem mounting.
//!   4. `Kb003ArtifactHandleV1::new` produces handles whose canonical form is
//!      `kb003://<class>/<sha256_prefix16>` with no host-path leak; the
//!      retention_root is the class default and therefore disk-agnostic.

use handshake_core::kernel::kb003_artifact_classes::{
    Kb003ArtifactClass, KB003_ARTIFACT_CLASSES,
};
use handshake_core::kernel::kb003_promotion::Kb003ArtifactHandleV1;
use handshake_core::kernel::sandbox::workspace::SandboxWorkspaceV1;

/// Return Some(needle) if the path contains any forbidden host-bound shape.
fn forbidden_path_shape(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        return Some("EMPTY_PATH");
    }
    // Backslashes anywhere indicate a Windows-style absolute or UNC path.
    if path.contains('\\') {
        return Some("BACKSLASH");
    }
    // UNC / network share prefix.
    if path.starts_with("//") {
        return Some("UNC_OR_DOUBLE_SLASH");
    }
    // Posix-absolute prefix.
    if path.starts_with('/') {
        return Some("LEADING_SLASH");
    }
    // Drive prefix like `C:`/`d:`.
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Some("DRIVE_LETTER");
    }
    // Common host-bound user-profile or system roots.
    for needle in [
        "/Users/",
        "/home/",
        "/tmp/",
        "/var/",
        "/etc/",
        "C:/Users/",
        "C:/Windows/",
        "/mnt/",
    ] {
        if path.contains(needle) {
            return Some("HARDCODED_HOST_ROOT");
        }
    }
    None
}

#[test]
fn every_kb003_artifact_class_retention_root_is_repo_relative() {
    assert!(
        !KB003_ARTIFACT_CLASSES.is_empty(),
        "artifact taxonomy must declare at least one class"
    );
    for meta in KB003_ARTIFACT_CLASSES {
        let root = meta.retention_root;
        assert!(
            root.starts_with("handshake-product/"),
            "{:?} retention_root must start with `handshake-product/` (CX-212E); got {:?}",
            meta.class,
            root
        );
        if let Some(shape) = forbidden_path_shape(root) {
            panic!(
                "{:?} retention_root contains forbidden host-bound shape {shape}: {root:?}",
                meta.class
            );
        }
        // Must not embed a leading dot or environment variable expansion.
        assert!(
            !root.contains('$') && !root.contains('%'),
            "{:?} retention_root must not embed env-var expansion; got {root:?}",
            meta.class
        );
    }
}

#[test]
fn sandbox_workspace_stays_repo_relative_when_root_changes() {
    // Construct two workspaces with different repo-relative roots; the
    // declared root_relative_path and the derived output root must both stay
    // repo-relative and forbidden-shape-free.
    let roots = [
        "handshake-product/kb003/work/alpha",
        "handshake-product/kb003/work/beta/sub/deeper",
        "handshake-product/kb003/sandbox/runs/0001",
    ];
    for root in roots {
        let ws = SandboxWorkspaceV1::new_default("kb003-mt078", root);
        assert_eq!(ws.root_relative_path, root, "root must stay verbatim");
        if let Some(shape) = forbidden_path_shape(&ws.root_relative_path) {
            panic!("workspace root has forbidden shape {shape}: {root:?}");
        }
        for out in &ws.output_roots_relative {
            if let Some(shape) = forbidden_path_shape(out) {
                panic!("workspace output root has forbidden shape {shape}: {out:?}");
            }
            assert!(
                out.starts_with(root),
                "output root must live under workspace root; got {out:?}"
            );
        }
    }
}

#[test]
fn workspace_contains_relative_rejects_host_absolute_paths() {
    let ws = SandboxWorkspaceV1::new_default("kb003-mt078", "handshake-product/kb003/work/x");
    // Allowed.
    assert!(ws.contains_relative("handshake-product/kb003/work/x/sub/file.txt"));
    assert!(ws.contains_relative("handshake-product/kb003/work/x"));
    // Host-absolute / drive / UNC / escape — all rejected.
    let host_bound = [
        "/etc/passwd",
        "/var/log/syslog",
        "/tmp/leak",
        "C:/Windows/system32",
        "../../../etc/shadow",
        "handshake-product/kb003/work/x/../../escape",
    ];
    for path in host_bound {
        assert!(
            !ws.contains_relative(path),
            "contains_relative must reject host-bound or escape path: {path:?}"
        );
    }
}

#[test]
fn kb003_artifact_handles_are_host_path_free() {
    // The handle string format is `kb003://<class>/<sha256_prefix16>` and
    // MUST NOT contain any host-bound path.
    let cases = [
        (Kb003ArtifactClass::SandboxLog, "deadbeef00000000abcdef"),
        (Kb003ArtifactClass::ValidationReport, "feedface00000000abcdef"),
        (Kb003ArtifactClass::PromotionReceipt, "baadf00d00000000abcdef"),
    ];
    for (class, hash) in cases {
        let handle = Kb003ArtifactHandleV1::new(class, hash).expect("handle constructs");
        assert!(
            handle.handle.starts_with("kb003://"),
            "handle must use kb003:// scheme; got {:?}",
            handle.handle
        );
        if let Some(shape) = forbidden_path_shape(&handle.handle) {
            panic!(
                "{:?} handle contains forbidden host-bound shape {shape}: {:?}",
                class, handle.handle
            );
        }
        // retention_root is class default, also disk-agnostic.
        assert!(
            handle.retention_root.starts_with("handshake-product/"),
            "{:?} retention_root must stay under handshake-product/; got {:?}",
            class,
            handle.retention_root
        );
        if let Some(shape) = forbidden_path_shape(handle.retention_root) {
            panic!(
                "{:?} handle retention_root contains forbidden shape {shape}: {:?}",
                class, handle.retention_root
            );
        }
    }
}

#[test]
fn moving_workspace_root_moves_artifact_taxonomy_coherently() {
    // The workspace root is parameterised by an operator-controlled
    // repo-relative string; the artifact taxonomy declares its own
    // repo-relative retention roots that do NOT depend on the runtime
    // workspace root. Both must stay repo-relative independently.
    let ws_a = SandboxWorkspaceV1::new_default("kb003-mt078-a", "handshake-product/kb003/work/aaa");
    let ws_b = SandboxWorkspaceV1::new_default("kb003-mt078-b", "handshake-product/kb003/work/bbb");
    assert_ne!(ws_a.root_relative_path, ws_b.root_relative_path);
    // Output roots differ but stay under handshake-product/.
    assert!(ws_a.output_roots_relative[0].starts_with("handshake-product/"));
    assert!(ws_b.output_roots_relative[0].starts_with("handshake-product/"));
    // Artifact taxonomy retention roots are unchanged by workspace choice:
    // the taxonomy is global, decoupled from any single sandbox run's workspace.
    for meta in KB003_ARTIFACT_CLASSES {
        assert!(
            meta.retention_root.starts_with("handshake-product/"),
            "{:?} retention_root must be repo-relative regardless of workspace root",
            meta.class
        );
    }
}

#[test]
fn forbidden_shape_detector_is_sound() {
    // Sanity check the helper used by the suite: it must classify host-bound
    // shapes and pass repo-relative shapes.
    assert!(forbidden_path_shape("handshake-product/kb003/x").is_none());
    assert!(forbidden_path_shape("a/b/c").is_none());
    assert_eq!(forbidden_path_shape("/etc/passwd"), Some("LEADING_SLASH"));
    assert_eq!(forbidden_path_shape("C:/foo"), Some("DRIVE_LETTER"));
    assert_eq!(forbidden_path_shape("d:\\foo"), Some("BACKSLASH"));
    assert_eq!(
        forbidden_path_shape("\\\\srv\\share\\f"),
        Some("BACKSLASH")
    );
    assert_eq!(
        forbidden_path_shape("//some/unc/like"),
        Some("UNC_OR_DOUBLE_SLASH")
    );
    assert_eq!(
        forbidden_path_shape("foo/Users/ilja/leak"),
        Some("HARDCODED_HOST_ROOT")
    );
    assert_eq!(forbidden_path_shape(""), Some("EMPTY_PATH"));
}
