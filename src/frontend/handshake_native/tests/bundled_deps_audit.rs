// WP-KERNEL-011 MT-031 — bundled-deps audit (static bundle inspection, CX-008-VIS).
//
// After build_installer.ps1 stages the single-installer bundle, this test walks the staging tree and
// asserts EVERY required runtime asset is present inside the bundle (so a clean profile needs nothing
// external), and asserts NO disallowed system-level WebView2Loader.dll ships at the bundle top level or
// under bundled/. It writes tests/native_gui/artifacts/bundle_deps_audit_report.json.
//
// It reuses installer::check_bundle_integrity_at as the canonical required-asset rule set (the same
// rules the runtime --self-check uses), so the audit and the runtime check can never silently diverge.
//
// DEVIATION (path): named tests/native_gui/bundled_deps_audit.rs in the contract; lives at
// tests/bundled_deps_audit.rs for the cargo integration-test target-name reason documented in
// installer_build_smoke.rs and MT-004's test_single_binary.rs. The JSON report still lands in
// tests/native_gui/artifacts/.

use std::path::{Path, PathBuf};
use std::process::Command;

use handshake_native::installer;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn build_script() -> PathBuf {
    crate_root().join("scripts").join("build_installer.ps1")
}

fn artifacts_dir() -> PathBuf {
    crate_root().join("tests").join("native_gui").join("artifacts")
}

/// Mirror build_installer.ps1's short-target-dir selection so the audit finds the staging tree.
fn short_target_dir() -> PathBuf {
    if let Ok(d) = std::env::var("HANDSHAKE_SHORT_TARGET_DIR") {
        if !d.trim().is_empty() {
            return PathBuf::from(d);
        }
    }
    if let Ok(d) = std::env::var("CARGO_TARGET_DIR") {
        if !d.trim().is_empty() && d.len() < 40 {
            return PathBuf::from(d);
        }
    }
    let temp = std::env::temp_dir();
    let root = temp
        .components()
        .next()
        .map(|c| PathBuf::from(c.as_os_str()))
        .unwrap_or_else(|| PathBuf::from("/"));
    root.join("hsk-rn")
}

/// Recursively collect every file path under `dir`, relative to `dir` (forward-slash strings).
fn list_files_rel(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect(dir, dir, &mut out);
    out.sort();
    out
}
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(root, &p, out);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// Ensure the staging tree exists; build it if missing (idempotent — the smoke may have built it).
fn ensure_staged() -> PathBuf {
    let staging = short_target_dir().join("release-native").join("staging");
    if staging.is_dir()
        && staging.join("handshake-native.exe").is_file()
    {
        return staging;
    }
    let script = build_script();
    let output = Command::new("pwsh")
        .args([
            "-NoProfile",
            "-File",
            &script.to_string_lossy(),
            "-ForceZip",
        ])
        .current_dir(crate_root())
        .output()
        .expect("spawn pwsh build_installer.ps1");
    assert!(
        output.status.success(),
        "build_installer.ps1 failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(staging.is_dir(), "staging dir still missing after build");
    staging
}

#[test]
fn bundle_contains_all_required_deps_and_no_system_webview() {
    let staging = ensure_staged();
    let files = list_files_rel(&staging);
    println!("staged files: {files:#?}");

    // --- required-asset audit via the canonical runtime rule set ---
    let integrity = installer::check_bundle_integrity_at(&staging);
    let missing: Vec<String> = match &integrity {
        Ok(()) => Vec::new(),
        Err(installer::BundleIntegrityError::MissingAsset { path }) => vec![path.clone()],
        Err(installer::BundleIntegrityError::EmptyAssetDir { path, expected_ext }) => {
            vec![format!("{path} (no {expected_ext} file)")]
        }
        Err(e) => vec![e.to_string()],
    };
    assert!(
        integrity.is_ok(),
        "bundle missing required assets: {missing:?}\nstaged files: {files:#?}"
    );

    // --- disallowed system WebView2: no WebView2Loader.dll at the top level or under bundled/ ---
    // (A WebView2 DLL is only allowed inside a dedicated bundled/browser_pane/ subdir if that pane is
    // ever bundled — not as a system-level dependency. No browser_pane is bundled today.)
    let disallowed_system_webview = files.iter().any(|f| {
        let lower = f.to_ascii_lowercase();
        lower.ends_with("webview2loader.dll")
            && !lower.starts_with("bundled/browser_pane/")
    });
    assert!(
        !disallowed_system_webview,
        "a system-level WebView2Loader.dll is bundled (only allowed inside bundled/browser_pane/): {files:#?}"
    );

    // --- explicit presence list for the report (the key deps a reviewer wants to see named) ---
    let has = |needle: &str| files.iter().any(|f| f.eq_ignore_ascii_case(needle));
    let pg_ctl_rel = installer::bundled_pg_ctl_rel_path();
    let postgres_binary = files
        .iter()
        .find(|f| f.eq_ignore_ascii_case(&pg_ctl_rel))
        .cloned();
    let font_file = files
        .iter()
        .find(|f| {
            let l = f.to_ascii_lowercase();
            l.starts_with("fonts/") && (l.ends_with(".ttf") || l.ends_with(".otf"))
        })
        .cloned();

    assert!(has("handshake-native.exe"), "native binary missing from bundle");
    assert!(postgres_binary.is_some(), "bundled postgres pg_ctl.exe missing");
    assert!(font_file.is_some(), "no bundled font file");

    // --- write the report (HBR-VIS) ---
    let report = serde_json::json!({
        "schema_id": "hsk.native_gui.bundle_deps_audit_report@1",
        "mt_id": "MT-031",
        "files_found": files,
        "native_binary": "handshake-native.exe",
        "postgres_binary": postgres_binary,
        "font_file": font_file,
        "grammars_dir_present": staging.join("grammars").is_dir(),
        "missing": missing,
        "disallowed_system_webview": disallowed_system_webview,
    });
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir).expect("create artifacts dir");
    let report_path = dir.join("bundle_deps_audit_report.json");
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap())
        .expect("write bundle_deps_audit_report.json");
    println!("WROTE {}", report_path.display());
    println!("PASS: bundle complete, missing=[], disallowed_system_webview=false");
}
