//! WP-KERNEL-011 MT-031 — single-installer bundle integrity checker (CX-008-VIS).
//!
//! ## What this is
//!
//! The product-side half of the single-installer guarantee. The MT-031 installer
//! (`build_installer.ps1` + `installer/windows/handshake_native.wxs`) packages the native shell plus
//! every runtime asset it needs into ONE artifact, so a clean Windows profile can install and launch
//! with ZERO external prerequisites (no system WebView2, no separate PostgreSQL install, no CDN
//! download at first run). `check_bundle_integrity` is the runtime self-test that proves that contract
//! held on the installed machine. It walks the asset tree relative to the running executable and fails
//! loudly (HBR-STOP) when a required bundled asset is missing, instead of letting the app start
//! half-initialised and crash later when it reaches for postgres, fonts, or grammars.
//!
//! ## Why exe-relative, and the bundled-postgres path contract
//!
//! The MT-031 contract (RISK-031-01) requires matching `handshake_core::managed_postgres`'s binary
//! discovery. That module (`src/backend/handshake_core/src/managed_postgres.rs`, read-only for this MT)
//! resolves `pg_ctl`/`initdb`/etc. via, in order: an explicit `bin_dir` config, the
//! `HANDSHAKE_MANAGED_PG_BIN` env var (the constant `managed_postgres::MANAGED_PG_BIN_ENV`), the
//! standard `PGBIN` env var, a fixed Windows default install path, then `PATH`. It does **not** today
//! derive a path from `current_exe()`. The single-installer guarantee therefore requires the installer
//! to (a) stage the postgres binaries at a stable exe-relative location and (b) point `managed_postgres`
//! at that location by exporting `HANDSHAKE_MANAGED_PG_BIN` (or `PGBIN`) = `<exe_dir>/bundled/postgres`
//! before the cluster starts. This module defines and verifies that staged location
//! ([`BUNDLED_POSTGRES_SUBDIR`]).
//! Wiring the env export lives in `handshake_core` startup (backend, out of scope for this MT and a
//! `forbidden_path`); the wire-up is recorded as a follow-up in `installer/windows/BUNDLED_DEPS_POLICY.md`.
//! Until then, this checker is the authoritative single source of the bundle layout both halves target.
//!
//! ## Self-check command (HBR-STOP)
//!
//! `handshake-native.exe --self-check` calls [`check_bundle_integrity`] against the installed exe's
//! directory, prints a machine-readable JSON verdict, and exits 0 (all assets present) or 1 (a required
//! asset is missing). It deliberately does NOT start the egui event loop, open a window, or connect to
//! postgres, so it is safe to run in a minimal/headless CI sandbox. See `src/main.rs` arg parsing.

use std::fmt;
use std::path::Path;

/// Subdirectory (relative to the executable) where the installer stages the bundled PostgreSQL
/// binaries. `managed_postgres` must be pointed here via `HANDSHAKE_MANAGED_PG_BIN`/`PGBIN` at startup
/// (see module docs). The anchor binary `pg_ctl` lives directly under this dir.
pub const BUNDLED_POSTGRES_SUBDIR: &str = "bundled/postgres";

/// Subdirectory (relative to the executable) holding the bundled UI fonts. Must contain at least one
/// `.ttf`/`.otf` file (the Inter faces bundled by MT-004), or egui text rendering would fall back to a
/// missing/empty font set on a clean profile.
pub const BUNDLED_FONTS_SUBDIR: &str = "fonts";

/// Subdirectory (relative to the executable) holding bundled syntax grammars (tree-sitter). May be
/// empty on the first installer pass — the integrity check only asserts the directory exists so future
/// grammar additions land in a known, already-bundled location rather than triggering a CDN fetch.
pub const BUNDLED_GRAMMARS_SUBDIR: &str = "grammars";

/// The native shell binary's own file name. Verified present so a corrupt/partial install (exe deleted
/// but launcher shortcut intact) is caught by `--self-check` rather than surfacing as a vague OS error.
pub const NATIVE_BINARY_NAME: &str = "handshake-native.exe";

/// The platform-appropriate `pg_ctl` binary name (the managed-postgres anchor binary).
#[cfg(windows)]
pub const PG_CTL_BINARY_NAME: &str = "pg_ctl.exe";
/// The platform-appropriate `pg_ctl` binary name (the managed-postgres anchor binary).
#[cfg(not(windows))]
pub const PG_CTL_BINARY_NAME: &str = "pg_ctl";

/// Exe-relative path of the bundled managed-postgres anchor binary, built at runtime from
/// [`BUNDLED_POSTGRES_SUBDIR`] + [`PG_CTL_BINARY_NAME`] so the platform-specific binary name
/// (`pg_ctl.exe` on Windows, `pg_ctl` elsewhere) is defined exactly once and stays cross-platform.
/// Returns a forward-slash relative path (matching the rest of [`REQUIRED_ASSETS`]).
pub fn bundled_pg_ctl_rel_path() -> String {
    format!("{BUNDLED_POSTGRES_SUBDIR}/{PG_CTL_BINARY_NAME}")
}

/// A required bundled asset and how to verify it. Kept as data (not ad-hoc `if` blocks) so the same
/// list drives both the runtime self-check and the build-time `bundled_deps_audit` test, and so a
/// reviewer can read the full bundle contract in one place.
#[derive(Debug, Clone, Copy)]
pub struct RequiredAsset {
    /// Path of the asset relative to the executable directory (forward slashes; joined per-OS).
    /// For [`AssetKind::PgCtlBinary`] this is the directory label only; the verified/displayed path
    /// is the cross-platform binary path from [`bundled_pg_ctl_rel_path`] — use [`Self::display_rel_path`].
    pub rel_path: &'static str,
    /// What kind of filesystem check proves this asset is present.
    pub kind: AssetKind,
}

impl RequiredAsset {
    /// The exe-relative path to report for this asset (in `--self-check` JSON and errors). Resolves the
    /// cross-platform managed-postgres binary path for [`AssetKind::PgCtlBinary`]; otherwise `rel_path`.
    pub fn display_rel_path(&self) -> String {
        match self.kind {
            AssetKind::PgCtlBinary => bundled_pg_ctl_rel_path(),
            _ => self.rel_path.to_string(),
        }
    }
}

/// The check that proves a [`RequiredAsset`] is satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// `rel_path` must be an existing regular file.
    File,
    /// `rel_path` must be an existing directory (contents not inspected).
    Dir,
    /// `rel_path` must be an existing directory containing at least one file whose extension matches
    /// any of these (case-insensitive, no leading dot), e.g. fonts.
    DirWithExt(&'static [&'static str]),
    /// The managed-postgres anchor binary. The verified path is built at runtime from
    /// [`bundled_pg_ctl_rel_path`] so the platform-specific binary name ([`PG_CTL_BINARY_NAME`]) is
    /// defined once and not hardcoded in the asset table. `rel_path` is the human-facing label.
    PgCtlBinary,
}

/// The full ordered list of assets the single-installer bundle MUST contain. This is the canonical
/// bundle contract: `build_installer.ps1` stages exactly these, `bundled_deps_audit` asserts them in
/// the staging tree, and [`check_bundle_integrity`] verifies them at runtime relative to the exe.
pub const REQUIRED_ASSETS: &[RequiredAsset] = &[
    RequiredAsset { rel_path: NATIVE_BINARY_NAME, kind: AssetKind::File },
    RequiredAsset {
        // Label only; the verified path is computed cross-platform by `bundled_pg_ctl_rel_path()`.
        rel_path: BUNDLED_POSTGRES_SUBDIR,
        kind: AssetKind::PgCtlBinary,
    },
    RequiredAsset {
        rel_path: BUNDLED_FONTS_SUBDIR,
        kind: AssetKind::DirWithExt(&["ttf", "otf"]),
    },
    RequiredAsset { rel_path: BUNDLED_GRAMMARS_SUBDIR, kind: AssetKind::Dir },
];

/// Why a bundle integrity check failed. Each variant names the exact missing/invalid asset so a
/// no-context operator (or the `--self-check` JSON consumer) knows precisely what the installer dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleIntegrityError {
    /// A required file or directory is absent at its exe-relative path.
    MissingAsset { path: String },
    /// A `DirWithExt` directory exists but contains no file with an accepted extension.
    EmptyAssetDir { path: String, expected_ext: String },
    /// `std::env::current_exe()` could not be resolved (no executable directory to anchor against).
    ExeDirUnresolvable { reason: String },
}

impl fmt::Display for BundleIntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleIntegrityError::MissingAsset { path } => {
                write!(f, "required bundled asset missing: {path}")
            }
            BundleIntegrityError::EmptyAssetDir { path, expected_ext } => write!(
                f,
                "bundled asset directory '{path}' exists but contains no {expected_ext} file"
            ),
            BundleIntegrityError::ExeDirUnresolvable { reason } => {
                write!(f, "cannot resolve executable directory: {reason}")
            }
        }
    }
}

impl std::error::Error for BundleIntegrityError {}

/// Verify the single-installer bundle against the **running executable's** directory.
///
/// Resolves the exe directory via `std::env::current_exe()` and checks every [`REQUIRED_ASSETS`] entry.
/// Returns the first failure (fail-fast: HBR-STOP wants a clear "this asset is missing" rather than a
/// list the operator must wade through). On success returns `Ok(())`.
pub fn check_bundle_integrity() -> Result<(), BundleIntegrityError> {
    let exe = std::env::current_exe()
        .map_err(|e| BundleIntegrityError::ExeDirUnresolvable { reason: e.to_string() })?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| BundleIntegrityError::ExeDirUnresolvable {
            reason: format!("executable {} has no parent directory", exe.display()),
        })?;
    check_bundle_integrity_at(exe_dir)
}

/// Verify the bundle against an explicit root directory. Factored out of [`check_bundle_integrity`] so
/// the unit test can point at a temp staging tree (and so `bundled_deps_audit` can reuse the exact same
/// rules against the build's staging dir) without needing a real installed exe.
pub fn check_bundle_integrity_at(root: &Path) -> Result<(), BundleIntegrityError> {
    for asset in REQUIRED_ASSETS {
        verify_asset(root, asset)?;
    }
    Ok(())
}

/// Check a single required asset against `root`.
fn verify_asset(root: &Path, asset: &RequiredAsset) -> Result<(), BundleIntegrityError> {
    let full = root.join(asset.rel_path);
    match asset.kind {
        AssetKind::File => {
            if !full.is_file() {
                return Err(BundleIntegrityError::MissingAsset {
                    path: asset.rel_path.to_string(),
                });
            }
        }
        AssetKind::Dir => {
            if !full.is_dir() {
                return Err(BundleIntegrityError::MissingAsset {
                    path: asset.rel_path.to_string(),
                });
            }
        }
        AssetKind::DirWithExt(exts) => {
            if !full.is_dir() {
                return Err(BundleIntegrityError::MissingAsset {
                    path: asset.rel_path.to_string(),
                });
            }
            if !dir_has_file_with_ext(&full, exts) {
                return Err(BundleIntegrityError::EmptyAssetDir {
                    path: asset.rel_path.to_string(),
                    expected_ext: exts.join("/"),
                });
            }
        }
        AssetKind::PgCtlBinary => {
            // Compute the real path (cross-platform binary name) instead of reading asset.rel_path,
            // which is only the directory label for this kind.
            let rel = bundled_pg_ctl_rel_path();
            if !root.join(&rel).is_file() {
                return Err(BundleIntegrityError::MissingAsset { path: rel });
            }
        }
    }
    Ok(())
}

/// True if `dir` directly contains at least one regular file whose extension (case-insensitive) is in
/// `exts`. Does not recurse — bundled fonts live flat under `fonts/`.
fn dir_has_file_with_ext(dir: &Path, exts: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.iter().any(|want| want.eq_ignore_ascii_case(ext)) {
                return true;
            }
        }
    }
    false
}

/// Render the self-check verdict as a machine-readable JSON line (consumed by `installer_build_smoke`).
/// On success: `{"status":"ok","checked_paths":[...]}`. On failure: `{"status":"missing_asset","path":"..."}`
/// or `{"status":"empty_asset_dir","path":"...","expected_ext":"..."}` /
/// `{"status":"exe_dir_unresolvable","reason":"..."}`. Hand-built (no serde) so this stays usable from
/// the thin bin path without pulling serde derive into the self-check; the smoke test parses it back
/// with `serde_json`.
pub fn self_check_json(result: &Result<(), BundleIntegrityError>) -> String {
    match result {
        Ok(()) => {
            let paths: Vec<String> = REQUIRED_ASSETS
                .iter()
                .map(|a| json_escape(&a.display_rel_path()))
                .collect();
            let joined = paths
                .iter()
                .map(|p| format!("\"{p}\""))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"status\":\"ok\",\"checked_paths\":[{joined}]}}")
        }
        Err(BundleIntegrityError::MissingAsset { path }) => {
            format!("{{\"status\":\"missing_asset\",\"path\":\"{}\"}}", json_escape(path))
        }
        Err(BundleIntegrityError::EmptyAssetDir { path, expected_ext }) => format!(
            "{{\"status\":\"empty_asset_dir\",\"path\":\"{}\",\"expected_ext\":\"{}\"}}",
            json_escape(path),
            json_escape(expected_ext)
        ),
        Err(BundleIntegrityError::ExeDirUnresolvable { reason }) => format!(
            "{{\"status\":\"exe_dir_unresolvable\",\"reason\":\"{}\"}}",
            json_escape(reason)
        ),
    }
}

/// Minimal JSON string escaping for the hand-built `--self-check` output (backslash, quote, control
/// chars). Paths can contain backslashes on Windows, so this is required for valid JSON.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Run the self-check against the installed exe and return the JSON verdict plus the process exit code
/// (0 = ok, 1 = any failure). Called by `src/main.rs` for the `--self-check` flag. Kept here (not in
/// main) so the exit-code policy is unit-testable and lives beside the checker it reports on.
pub fn run_self_check() -> (String, i32) {
    let result = check_bundle_integrity();
    let code = if result.is_ok() { 0 } else { 1 };
    (self_check_json(&result), code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A throwaway directory under the OS temp dir. Cleaned up on drop. No external crate: the contract
    /// suggested `tempfile` but it is not a current dep and the bundle layout we build is trivial, so a
    /// hand-rolled unique temp dir keeps the "no new deps" constraint.
    struct TempStaging {
        dir: PathBuf,
    }

    impl TempStaging {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!("hsk-mt031-{tag}-{nanos}"));
            fs::create_dir_all(&dir).expect("create temp staging dir");
            Self { dir }
        }

        /// Build a fully-valid staging tree (all REQUIRED_ASSETS present) under this temp dir.
        fn write_valid_bundle(&self) {
            // handshake-native.exe (content is irrelevant to the integrity check; size > 0).
            fs::write(self.dir.join(NATIVE_BINARY_NAME), b"MZ-fake-exe").unwrap();
            // bundled/postgres/<pg_ctl> (cross-platform binary name)
            let pg = self.dir.join("bundled").join("postgres");
            fs::create_dir_all(&pg).unwrap();
            fs::write(pg.join(PG_CTL_BINARY_NAME), b"fake-pg_ctl").unwrap();
            // fonts/ with one .ttf
            let fonts = self.dir.join(BUNDLED_FONTS_SUBDIR);
            fs::create_dir_all(&fonts).unwrap();
            fs::write(fonts.join("Inter-Regular.ttf"), b"fake-ttf").unwrap();
            // grammars/ (empty dir is acceptable)
            fs::create_dir_all(self.dir.join(BUNDLED_GRAMMARS_SUBDIR)).unwrap();
        }
    }

    impl Drop for TempStaging {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// AC-031-08 (positive): a correctly-staged tree passes the integrity check.
    /// PT-031-04 names this exact test (`check_bundle_integrity_returns_ok`).
    #[test]
    fn check_bundle_integrity_returns_ok() {
        let staging = TempStaging::new("ok");
        staging.write_valid_bundle();
        let result = check_bundle_integrity_at(&staging.dir);
        assert!(
            result.is_ok(),
            "valid staging tree should pass integrity check, got {result:?}"
        );
    }

    /// AC-031-08 (negative): removing the postgres anchor binary makes the check fail with the exact
    /// missing path. This is CTRL-031-05 — proves `--self-check` would exit non-zero on a broken install.
    #[test]
    fn check_bundle_integrity_fails_on_missing_postgres() {
        let staging = TempStaging::new("nopg");
        staging.write_valid_bundle();
        fs::remove_file(staging.dir.join("bundled").join("postgres").join(PG_CTL_BINARY_NAME)).unwrap();
        let result = check_bundle_integrity_at(&staging.dir);
        assert_eq!(
            result,
            Err(BundleIntegrityError::MissingAsset {
                path: bundled_pg_ctl_rel_path()
            }),
            "missing pg_ctl must report the exact (cross-platform) rel_path"
        );
    }

    /// Negative: a fonts/ dir with no font file is an EmptyAssetDir error, not a silent pass.
    #[test]
    fn check_bundle_integrity_fails_on_empty_fonts_dir() {
        let staging = TempStaging::new("nofont");
        staging.write_valid_bundle();
        fs::remove_file(staging.dir.join(BUNDLED_FONTS_SUBDIR).join("Inter-Regular.ttf")).unwrap();
        let result = check_bundle_integrity_at(&staging.dir);
        assert!(
            matches!(result, Err(BundleIntegrityError::EmptyAssetDir { ref path, .. }) if path == BUNDLED_FONTS_SUBDIR),
            "fonts dir with no .ttf/.otf must be EmptyAssetDir, got {result:?}"
        );
    }

    /// Negative: a missing grammars/ directory fails (it is a hard-required bundle slot even if empty).
    #[test]
    fn check_bundle_integrity_fails_on_missing_grammars_dir() {
        let staging = TempStaging::new("nogram");
        staging.write_valid_bundle();
        fs::remove_dir_all(staging.dir.join(BUNDLED_GRAMMARS_SUBDIR)).unwrap();
        let result = check_bundle_integrity_at(&staging.dir);
        assert_eq!(
            result,
            Err(BundleIntegrityError::MissingAsset {
                path: BUNDLED_GRAMMARS_SUBDIR.to_string()
            })
        );
    }

    /// The self-check JSON is valid and reports the right status for ok and failure. Mirrors AC-031-09
    /// (the report files must be serde-parseable) for the inline self-check payload.
    #[test]
    fn self_check_json_shapes_are_valid() {
        let ok = self_check_json(&Ok(()));
        assert!(ok.contains("\"status\":\"ok\""));
        assert!(ok.contains("bundled/postgres/pg_ctl.exe"));
        let parsed: serde_json::Value = serde_json::from_str(&ok).expect("ok json parses");
        assert_eq!(parsed["status"], "ok");

        let miss = self_check_json(&Err(BundleIntegrityError::MissingAsset {
            path: "bundled/postgres/pg_ctl.exe".into(),
        }));
        let parsed: serde_json::Value = serde_json::from_str(&miss).expect("miss json parses");
        assert_eq!(parsed["status"], "missing_asset");
        assert_eq!(parsed["path"], "bundled/postgres/pg_ctl.exe");
    }

    /// The exit-code policy (`Ok` -> 0, any failure -> 1) and the JSON verdict are driven by REAL bundle
    /// integrity results, not a constant. We can't call `run_self_check()` directly here because it
    /// anchors on `current_exe()` (the test-harness binary, not a staged bundle), so we exercise the same
    /// code path one layer down: build a valid staging tree -> `check_bundle_integrity_at` -> map the
    /// result through `self_check_json` and the documented exit-code rule, then break the tree and assert
    /// the failure maps to exit 1 and a missing-asset verdict naming the asset.
    #[test]
    fn self_check_exit_code_tracks_real_bundle_integrity() {
        let staging = TempStaging::new("exitcode");
        staging.write_valid_bundle();

        // Valid bundle -> Ok -> exit 0 -> JSON status "ok".
        let ok_result = check_bundle_integrity_at(&staging.dir);
        assert!(ok_result.is_ok(), "valid bundle should pass: {ok_result:?}");
        let ok_code = if ok_result.is_ok() { 0 } else { 1 };
        assert_eq!(ok_code, 0);
        let ok_json = self_check_json(&ok_result);
        let ok_parsed: serde_json::Value = serde_json::from_str(&ok_json).expect("ok json parses");
        assert_eq!(ok_parsed["status"], "ok");

        // Break the bundle (remove the postgres anchor) -> Err -> exit 1 -> status "missing_asset".
        fs::remove_file(staging.dir.join("bundled").join("postgres").join(PG_CTL_BINARY_NAME)).unwrap();
        let bad_result = check_bundle_integrity_at(&staging.dir);
        assert!(bad_result.is_err(), "broken bundle should fail");
        let bad_code = if bad_result.is_ok() { 0 } else { 1 };
        assert_eq!(bad_code, 1, "a failing integrity check must map to exit 1");
        let bad_json = self_check_json(&bad_result);
        let bad_parsed: serde_json::Value = serde_json::from_str(&bad_json).expect("bad json parses");
        assert_eq!(bad_parsed["status"], "missing_asset");
        assert_eq!(bad_parsed["path"], bundled_pg_ctl_rel_path());
    }
}
