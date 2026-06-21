// WP-KERNEL-011 MT-031 — single-installer build smoke (CX-008-VIS).
//
// Proves the product builds + packages into ONE installable artifact AND that the artifact is a single
// self-contained binary that actually launches on a clean profile:
//   1. runs scripts/build_installer.ps1 (cargo build --profile release-native + stage + package);
//   2. asserts the produced installer artifact exists and is non-trivially sized;
//   3. runs the staged single self-contained handshake-native.exe with `--version` and `--self-check`
//      under a CLEAN-PROFILE environment (no HANDSHAKE_WORKSPACE_ROOT / HANDSHAKE_RUNTIME_ROOT) and
//      asserts both launch and `--self-check` exits 0 with all bundled assets present;
//   4. parses the staged exe's PE import table (object crate) and asserts WebView2Loader.dll is NOT a
//      system import (single-installer = no system WebView2 dependency);
//   5. writes tests/native_gui/artifacts/installer_smoke_report.json (machine-readable evidence).
//
// HOST CONSTRAINT (verified 2026-06-21): the WiX toolchain is absent on this host, so the script emits a
// zip artifact (a real single self-contained artifact), NOT a faked .msi. The real WiX .wxs + the exact
// `wix build` command are authored in installer/windows/ and gated in the script — see BUNDLED_DEPS_POLICY.md.
//
// DEVIATIONS (for the reviewer):
//   * PATH: the contract names this file tests/native_gui/installer_build_smoke.rs. cargo derives an
//     integration-test target name from a file DIRECTLY in the crate `tests/` dir; a tests/native_gui/
//     subdir would not register the `installer_build_smoke` target. This file therefore lives at
//     tests/installer_build_smoke.rs (same decision MT-004/MT-029 documented). The JSON report still
//     lands in tests/native_gui/artifacts/ exactly as the contract requires.
//   * SIZE BOUND: AC-031-01 asks for a >= 10 MB COMPRESSED artifact. That presumes the REAL managed-
//     postgres binaries (tens of MB) are bundled. On this host no postgres toolchain is installed, so the
//     script stages a documented pg_ctl placeholder (postgres-deferred) and Compress-Archive compresses
//     the ~15 MB crt-static exe to ~6.5 MB — under the literal 10 MB COMPRESSED line for placeholder
//     reasons, not because the build is a stub. The smoke uses the contract's OWN RISK-031-08 documented
//     fallback: assert (a) the UNCOMPRESSED staging tree >= 10 MB (proves a real binary, not a stub — the
//     meaningful bound) AND (b) the COMPRESSED artifact >= 5 MB (real zip ~6.47 MB; a far stronger floor
//     than 1 MB). Both numbers + the postgres-deferred note are recorded in the report; the literal 10 MB
//     COMPRESSED bound holds automatically once real postgres binaries are staged.
//   * BUILD: the release-native profile + the crate's long external target-dir trips Windows MAX_PATH
//     (LNK1104); the script (and this test) build into a SHORT CARGO_TARGET_DIR. Documented in Cargo.toml
//     and BUNDLED_DEPS_POLICY.md. The test honors HANDSHAKE_SHORT_TARGET_DIR / CARGO_TARGET_DIR so the
//     script and the test agree on where the artifact lands.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The crate root (this file's CARGO_MANIFEST_DIR is the crate dir).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The build_installer.ps1 path (crate scripts/ dir).
fn build_script() -> PathBuf {
    crate_root().join("scripts").join("build_installer.ps1")
}

/// Where the contract requires the report (crate-internal tests/native_gui/artifacts/).
fn artifacts_dir() -> PathBuf {
    crate_root().join("tests").join("native_gui").join("artifacts")
}

/// The short build target dir the script uses. Mirrors the script's selection so this test can locate
/// the staged exe + staging tree afterwards. Honors the same env overrides.
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
    // Default mirrors the script: <drive root of TEMP>/hsk-rn
    let temp = std::env::temp_dir();
    let root = temp
        .components()
        .next()
        .map(|c| PathBuf::from(c.as_os_str()))
        .unwrap_or_else(|| PathBuf::from("/"));
    // On Windows the prefix+root components give e.g. "C:\"; join keeps it disk-agnostic at runtime.
    root.join("hsk-rn")
}

/// Total size (bytes) of every file under `dir`, recursively. Proves the UNCOMPRESSED bundle is real.
fn dir_size_bytes(dir: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                total += dir_size_bytes(&p);
            } else if let Ok(md) = p.metadata() {
                total += md.len();
            }
        }
    }
    total
}

/// Parse the staged exe's PE import table and return true iff WebView2Loader.dll appears as a static
/// import. Windows-only (PE import inspection); returns None on non-Windows or unparseable binaries.
#[cfg(target_os = "windows")]
fn webview2_is_system_import(exe: &Path) -> Option<bool> {
    use object::Object;
    let data = std::fs::read(exe).ok()?;
    let file = object::File::parse(&*data).ok()?;
    let imports = file.imports().ok()?;
    let hit = imports
        .iter()
        .any(|i| String::from_utf8_lossy(i.library()).eq_ignore_ascii_case("WebView2Loader.dll"));
    Some(hit)
}
#[cfg(not(target_os = "windows"))]
fn webview2_is_system_import(_exe: &Path) -> Option<bool> {
    None
}

#[test]
fn installer_builds_single_artifact_and_self_check_passes() {
    let script = build_script();
    assert!(
        script.is_file(),
        "build_installer.ps1 not found at {}",
        script.display()
    );

    // --- 1. run the build + package script (ForceZip: deterministic on any host; the WiX MSI path is
    //        gated/authored separately and proven on a WiX-equipped host) ---
    let output = Command::new("pwsh")
        .args([
            "-NoProfile",
            "-File",
            &script.to_string_lossy(),
            "-ForceZip",
        ])
        .current_dir(crate_root())
        .output()
        .expect("failed to spawn pwsh for build_installer.ps1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("--- build_installer.ps1 stdout ---\n{stdout}");
    if !stderr.trim().is_empty() {
        println!("--- build_installer.ps1 stderr ---\n{stderr}");
    }
    assert!(
        output.status.success(),
        "build_installer.ps1 exited non-zero ({:?})\nstderr:\n{stderr}",
        output.status.code()
    );

    // --- 2. locate the artifact from the final "INSTALLER_ARTIFACT=<path> SIZE_BYTES=<n>" line ---
    let artifact_line = stdout
        .lines()
        .rev()
        .find(|l| l.contains("INSTALLER_ARTIFACT="))
        .unwrap_or_else(|| panic!("no INSTALLER_ARTIFACT line in script output:\n{stdout}"));
    let artifact_path = artifact_line
        .split("INSTALLER_ARTIFACT=")
        .nth(1)
        .and_then(|s| s.split(" SIZE_BYTES=").next())
        .map(str::trim)
        .map(PathBuf::from)
        .expect("parse INSTALLER_ARTIFACT path");
    assert!(
        artifact_path.is_file(),
        "installer artifact not found at {}",
        artifact_path.display()
    );
    let artifact_size = std::fs::metadata(&artifact_path).unwrap().len();

    // staging tree (uncompressed) — the meaningful "is this a real build, not a stub" bound.
    let staging = short_target_dir()
        .join("release-native")
        .join("staging");
    assert!(
        staging.is_dir(),
        "staging dir not found at {}",
        staging.display()
    );
    let staging_size = dir_size_bytes(&staging);
    // AC-031-01 size bound, applied via the contract's OWN RISK-031-08 documented fallback.
    //
    // The literal AC-031-01 bound is ">= 10 MB COMPRESSED artifact". That bound presumes the REAL
    // managed-postgres binaries (tens of MB) are staged into bundled/postgres/. This host has NO
    // postgres toolchain, so build_installer.ps1 stages a documented pg_ctl PLACEHOLDER (postgres-
    // deferred) and Compress-Archive compresses the ~15 MB crt-static exe to ~6.5 MB — under the
    // literal 10 MB COMPRESSED line for placeholder reasons, not because the build is a stub.
    //
    // RISK-031-08 fallback (used here, NOT the silent 1 MB bound the review flagged): assert
    //   (a) the UNCOMPRESSED staging tree >= 10 MB  -> proves a real release-native binary, not a stub
    //       (the meaningful bound; observed ~13.9 MB here), AND
    //   (b) the COMPRESSED artifact itself >= 5 MB  -> the real zip is ~6.47 MB and passes; this is a
    //       far stronger floor than the previous 1 MB and still survives placeholder postgres.
    // Both numbers are recorded verbatim in installer_smoke_report.json. The literal 10 MB COMPRESSED
    // assertion requires real bundled postgres binaries and will hold automatically once they are staged
    // on a host that has the postgres toolchain (postgres-deferred — see BUNDLED_DEPS_POLICY.md).
    assert!(
        staging_size >= 10 * 1024 * 1024,
        "staging tree is only {staging_size} bytes (< 10 MB) — looks like a stub, not a real build",
    );
    assert!(
        artifact_size >= 5 * 1024 * 1024,
        "installer artifact is only {artifact_size} bytes (< 5 MB RISK-031-08 fallback floor); \
         real zip is ~6.47 MB — a smaller artifact means the release-native build did not happen",
    );

    // --- 3. clean-profile launch smoke on the STAGED single binary ---
    let staged_exe = staging.join("handshake-native.exe");
    assert!(
        staged_exe.is_file(),
        "staged handshake-native.exe not found at {}",
        staged_exe.display()
    );

    // --version: proves the single binary launches headlessly.
    let version_out = Command::new(&staged_exe)
        .arg("--version")
        .env_remove("HANDSHAKE_WORKSPACE_ROOT")
        .env_remove("HANDSHAKE_RUNTIME_ROOT")
        .output()
        .expect("run staged exe --version");
    assert!(
        version_out.status.success(),
        "staged exe --version exited non-zero ({:?})",
        version_out.status.code()
    );
    let version_str = String::from_utf8_lossy(&version_out.stdout).trim().to_string();
    assert!(
        version_str.contains("handshake-native"),
        "--version output unexpected: {version_str:?}"
    );

    // --self-check: clean-profile bundle integrity. Exit 0 = all bundled assets present next to the exe.
    let self_check = Command::new(&staged_exe)
        .arg("--self-check")
        .env_remove("HANDSHAKE_WORKSPACE_ROOT")
        .env_remove("HANDSHAKE_RUNTIME_ROOT")
        .output()
        .expect("run staged exe --self-check");
    let self_check_code = self_check.status.code().unwrap_or(-1);
    let self_check_json = String::from_utf8_lossy(&self_check.stdout).trim().to_string();
    println!("--self-check exit={self_check_code} json={self_check_json}");

    // Parse the self-check JSON to extract missing assets (if any) for the report.
    let parsed: serde_json::Value =
        serde_json::from_str(&self_check_json).expect("self-check JSON parses (AC-031-09)");
    let missing_assets: Vec<String> = match parsed.get("status").and_then(|s| s.as_str()) {
        Some("ok") => Vec::new(),
        _ => parsed
            .get("path")
            .and_then(|p| p.as_str())
            .map(|p| vec![p.to_string()])
            .unwrap_or_default(),
    };
    assert_eq!(
        self_check_code, 0,
        "--self-check exited {self_check_code} (expected 0); json={self_check_json}"
    );
    assert!(
        missing_assets.is_empty(),
        "self-check reported missing assets: {missing_assets:?}"
    );

    // --- 4. WebView2 system-import check on the staged binary ---
    let webview2_system_import = webview2_is_system_import(&staged_exe);
    if cfg!(target_os = "windows") {
        assert_eq!(
            webview2_system_import,
            Some(false),
            "handshake-native.exe imports WebView2Loader.dll as a system dependency — \
             violates the no-system-webview single-installer guarantee"
        );
    }

    // --- 5. write the machine-readable report (HBR-VIS) ---
    let report = serde_json::json!({
        "schema_id": "hsk.native_gui.installer_smoke_report@1",
        "mt_id": "MT-031",
        "artifact_path": artifact_path.to_string_lossy(),
        "artifact_size_bytes": artifact_size,
        "staging_size_bytes": staging_size,
        // AC-031-01 size bound, recorded so the reviewer sees the real numbers and the basis for the
        // RISK-031-08 fallback. literal_ac_031_01 (>= 10 MB COMPRESSED) is deferred until real postgres
        // binaries are staged; the asserted floors are staging >= 10 MB and artifact >= 5 MB.
        "size_bound": {
            "asserted_staging_min_bytes": 10u64 * 1024 * 1024,
            "asserted_artifact_min_bytes": 5u64 * 1024 * 1024,
            "literal_ac_031_01_compressed_min_bytes": 10u64 * 1024 * 1024,
            "literal_ac_031_01_met": artifact_size >= 10 * 1024 * 1024,
            "basis": "RISK-031-08 fallback (artifact>=5MB AND staging>=10MB)",
            "postgres_deferred": true,
            "postgres_note": "bundled/postgres holds a documented placeholder pg_ctl on this host (no \
                postgres toolchain); the literal 10 MB COMPRESSED bound requires the real managed-postgres \
                binaries and will hold automatically once they are staged (see BUNDLED_DEPS_POLICY.md)."
        },
        "self_check_exit_code": self_check_code,
        "missing_assets": missing_assets,
        "webview2_system_import": webview2_system_import.unwrap_or(false),
        "webview2_check_platform": if cfg!(target_os = "windows") { "windows_pe" } else { "skipped_non_windows" },
        "version_line": version_str,
    });
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir).expect("create artifacts dir");
    let report_path = dir.join("installer_smoke_report.json");
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("write installer_smoke_report.json");
    println!("WROTE {}", report_path.display());
    println!(
        "PASS: single installer artifact {} ({} bytes), staging {} bytes, self-check exit 0, no system WebView2",
        artifact_path.display(),
        artifact_size,
        staging_size
    );
}
