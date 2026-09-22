//! WP-KERNEL-012 MT-044 — shared parity-manifest write-back helper for the four `test_parity_*.rs`
//! suites (cluster E8). Lives in a `tests/` SUBDIRECTORY so Cargo does not compile it as a standalone
//! test binary (only top-level `tests/*.rs` are test targets); each parity suite pulls it in with
//! `mod parity_manifest_support;`.
//!
//! ## What it does (RISK-3 / CTRL-3: manifest write-back is mandatory)
//!
//! Each passing proof function calls [`mark_pass`] (frontend E1) or [`mark_requires_surrealdb`] (the gated
//! E2/E3/E4 proofs, when run with a live SurrealDB) with its `feature_id`. The helper rewrites the
//! `status` field of that entry in the RUNTIME manifest copy to the given value, so after a full run a
//! no-context model can read that copy and know each feature's state. The committed
//! `tests/parity_manifest.json` is the read-only baseline input (CX-984-001: proof runs never write
//! tracked repo files); the runtime copy is seeded from it on first write and lives under the external
//! artifact root at [`manifest_path`] (`<root>/wp-kernel-012/mt-044/<run>/parity_manifest.json`, where
//! `<run>` is `HSK_MT044_RUN_ID` when set, else `default`).
//!
//! Concurrency: `cargo test` runs test fns in parallel threads within ONE test binary, and the four
//! parity suites are SEPARATE binaries that may run concurrently. A naive read-modify-write would race.
//! [`mark_pass`] therefore takes a cross-process advisory lock (a sibling `.lock` file created
//! O_EXCL with bounded spin-retry) around the read-modify-write so concurrent writers serialize and no
//! update is lost. A failure to acquire the lock within the bound is a NON-FATAL warning (the proof's
//! assertions are the real gate; the manifest write is a record), never a panic that would fail an
//! otherwise-green proof.
//!
//! This module is only ever compiled into the test binaries (it lives under `tests/`), so it never
//! reaches the product binary — satisfying the contract's "#[cfg(test)] guard on manifest-write
//! helpers" intent at the strongest level (test-only by construction).

#![allow(dead_code)] // each suite uses a subset of the helpers; the others are not dead in aggregate.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Rewrite the manifest entry for `feature_id` to `status: "PASS"`. Called by each green E1 proof.
pub fn mark_pass(feature_id: &str) {
    set_status(feature_id, "PASS");
}

/// Rewrite the manifest entry for `feature_id` to `status: "REQUIRES_SURREALDB"`. The default (non-live) E2/
/// E3/E4 proofs already carry this status in the committed manifest; the gated `*_live` proofs upgrade
/// it to PASS via [`mark_pass`] when run against a managed SurrealDB.
pub fn mark_requires_surrealdb(feature_id: &str) {
    set_status(feature_id, "REQUIRES_SURREALDB");
}

/// The committed, read-only baseline manifest under the crate root. Proof runs never write it.
pub fn baseline_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("parity_manifest.json")
}

/// The mutable runtime manifest copy under the external artifact root (never a tracked repo path).
/// Runs sharing `HSK_MT044_RUN_ID` (or leaving it unset) share one copy, so the four parity binaries
/// accumulate their write-backs into the same file under the cross-process lock.
pub fn manifest_path() -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_TEST_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
                .map(PathBuf::from)
                .map(|root| root.join("handshake-test"))
        })
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
                .join("handshake-test")
        });
    let run = std::env::var("HSK_MT044_RUN_ID")
        .ok()
        .filter(|run| !run.trim().is_empty())
        .unwrap_or_else(|| "default".to_owned());
    assert!(
        run.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "HSK_MT044_RUN_ID must be a safe filename component"
    );
    root.join("wp-kernel-012")
        .join("mt-044")
        .join(run)
        .join("parity_manifest.json")
}

/// Read-modify-write the `status` of the entry whose `feature_id` matches, under a cross-process
/// advisory lock. Non-fatal on any IO/lock failure (warns; never panics — the proof asserts are the
/// gate).
fn set_status(feature_id: &str, status: &str) {
    let path = manifest_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "WARN(parity-manifest): create {parent:?} failed: {e}; skipping {feature_id} write-back"
            );
            return;
        }
    }
    let lock_path = path.with_extension("json.lock");

    let _guard = match FileLock::acquire(&lock_path, Duration::from_secs(10)) {
        Some(g) => g,
        None => {
            eprintln!(
                "WARN(parity-manifest): could not acquire {lock_path:?} within budget; skipping the \
                 status write-back for {feature_id} (the proof assertions still gate this feature)"
            );
            return;
        }
    };

    // Seed from the committed baseline until this run's external copy exists; the baseline is read only.
    let source_path = if path.exists() {
        path.clone()
    } else {
        baseline_manifest_path()
    };
    let src = match std::fs::read_to_string(&source_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "WARN(parity-manifest): read {source_path:?} failed: {e}; skipping {feature_id} write-back"
            );
            return;
        }
    };
    let mut value: serde_json::Value = match serde_json::from_str(&src) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "WARN(parity-manifest): parse {source_path:?} failed: {e}; skipping {feature_id} write-back"
            );
            return;
        }
    };
    let Some(arr) = value.as_array_mut() else {
        eprintln!(
            "WARN(parity-manifest): manifest is not a JSON array; skipping {feature_id} write-back"
        );
        return;
    };
    let mut updated = false;
    for entry in arr.iter_mut() {
        if entry.get("feature_id").and_then(|v| v.as_str()) == Some(feature_id) {
            entry["status"] = serde_json::Value::String(status.to_owned());
            updated = true;
            break;
        }
    }
    if !updated {
        eprintln!(
            "WARN(parity-manifest): no manifest entry for feature_id={feature_id}; nothing written"
        );
        return;
    }
    // Pretty-print with a trailing newline so the file stays human-diffable.
    match serde_json::to_string_pretty(&value) {
        Ok(mut out) => {
            out.push('\n');
            if let Err(e) = std::fs::write(&path, out) {
                eprintln!("WARN(parity-manifest): write {path:?} failed: {e}");
            }
        }
        Err(e) => eprintln!("WARN(parity-manifest): serialize failed: {e}"),
    }
}

/// A minimal cross-process advisory lock: an O_EXCL `.lock` file removed on drop, with bounded spin.
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &std::path::Path, budget: Duration) -> Option<Self> {
        let start = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(_) => {
                    return Some(FileLock {
                        path: path.to_path_buf(),
                    })
                }
                Err(_) if start.elapsed() < budget => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => {
                    // Stale lock recovery: if the lock file is older than the budget, steal it (a prior
                    // run likely crashed before cleanup). Best-effort; a failure just retries the loop.
                    if let Ok(meta) = std::fs::metadata(path) {
                        if let Ok(modified) = meta.modified() {
                            if modified.elapsed().map(|e| e > budget).unwrap_or(false) {
                                let _ = std::fs::remove_file(path);
                                continue;
                            }
                        }
                    }
                    return None;
                }
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
