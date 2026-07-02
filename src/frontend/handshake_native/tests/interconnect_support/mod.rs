//! WP-KERNEL-012 MT-046 — shared support for the four `test_interconnect_*.rs` interconnection-proof
//! suites (cluster E8, the melt-together capstone). Lives in a `tests/` SUBDIRECTORY so Cargo does NOT
//! compile it as a standalone test binary (only top-level `tests/*.rs` are test targets); each suite pulls
//! it in with `#[path = "interconnect_support/mod.rs"] mod interconnect_support;`.
//!
//! ## What it provides
//!
//! - [`mark_status`] — manifest write-back into `tests/test_interconnect_manifest.json` (the 18-entry
//!   manifest), under a cross-process advisory lock (the four suites run as separate concurrent binaries).
//!   A passing in-process substrate proof re-affirms `PASS`; a requires_pg proof affirms `REQUIRES_PG`; a
//!   live-PG `--ignored` run upgrades its entry to `PASS`. Non-fatal on any IO error (the proof asserts are
//!   the real gate, mirroring MT-044 `parity_manifest_support`).
//! - [`require_live_backend`] / [`LiveBackend`] — the honest live-PG gate for the requires_pg scenarios:
//!   resolves `HSK_TEST_BASE` (default `http://127.0.0.1:37501`) + `HSK_TEST_WORKSPACE_ID`, verifies
//!   reachability via `/health`, and PANICS with a `requires_pg` message otherwise (no mock smuggling, no
//!   silent skip, NO SQLite). Same shape as MT-044 `pg_proof_support`.
//! - [`assert_no_local_artifact_dir`] — CX-212E hygiene guard (checks `test_output/` AND
//!   `tests/screenshots/`); a tracked artifact under `src/` is a hygiene FAILURE.
//! - [`author_ids`] / [`author_node_value`] — AccessKit tree readers for the in-process substrate proofs.

#![allow(dead_code)] // each suite uses a subset of the helpers; the others are not dead in aggregate.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::backend_client::shared_http_client;

// ── Manifest write-back (RISK + CTRL: the 18-entry manifest must reflect the run) ─────────────────────

/// The deterministic manifest path under the crate root, independent of the test's working directory.
pub fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_interconnect_manifest.json")
}

/// Rewrite the manifest entry whose `scenario_id` matches to `status`, under a cross-process advisory
/// lock. Non-fatal on any IO/lock failure (warns; never panics — the proof asserts are the gate). The
/// substrate proofs call this with `"PASS"`, the requires_pg proofs with `"REQUIRES_PG"`, the gated
/// live-PG `--ignored` proofs upgrade to `"PASS"`, and IC-13 records `"SKIPPED"`.
pub fn mark_status(scenario_id: &str, status: &str) {
    let path = manifest_path();
    let lock_path = path.with_extension("json.lock");

    let _guard = match FileLock::acquire(&lock_path, Duration::from_secs(10)) {
        Some(g) => g,
        None => {
            eprintln!(
                "WARN(interconnect-manifest): could not acquire {lock_path:?} within budget; skipping the \
                 status write-back for {scenario_id} (the proof assertions still gate this scenario)"
            );
            return;
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "WARN(interconnect-manifest): read {path:?} failed: {e}; skipping {scenario_id}"
            );
            return;
        }
    };
    let mut value: serde_json::Value = match serde_json::from_str(&src) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "WARN(interconnect-manifest): parse {path:?} failed: {e}; skipping {scenario_id}"
            );
            return;
        }
    };
    let Some(arr) = value.as_array_mut() else {
        eprintln!(
            "WARN(interconnect-manifest): manifest is not a JSON array; skipping {scenario_id}"
        );
        return;
    };
    let mut updated = false;
    for entry in arr.iter_mut() {
        if entry.get("scenario_id").and_then(|v| v.as_str()) == Some(scenario_id) {
            entry["status"] = serde_json::Value::String(status.to_owned());
            updated = true;
            break;
        }
    }
    if !updated {
        eprintln!("WARN(interconnect-manifest): no manifest entry for scenario_id={scenario_id}");
        return;
    }
    match serde_json::to_string_pretty(&value) {
        Ok(mut out) => {
            out.push('\n');
            if let Err(e) = std::fs::write(&path, out) {
                eprintln!("WARN(interconnect-manifest): write {path:?} failed: {e}");
            }
        }
        Err(e) => eprintln!("WARN(interconnect-manifest): serialize failed: {e}"),
    }
}

/// A minimal cross-process advisory lock: an O_EXCL `.lock` file removed on drop, with bounded spin.
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path, budget: Duration) -> Option<Self> {
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

// ── Artifact hygiene (CX-212E / CX-212F): artifacts go to the EXTERNAL root ONLY ──────────────────────

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — a sibling of the
/// repo worktree. Four `..` reach `<repo>/..` where `Handshake_Artifacts` lives.
pub fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` AND `tests/screenshots/`; a tracked artifact under `src/` is a hygiene FAILURE — this
/// guard fails the run if one appears. Per CX-212E the rule OVERRIDES any repo-local path a contract names.
pub fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist ({}) — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only",
            local.display()
        );
    }
}

// ── AccessKit tree readers (the in-process substrate proofs) ──────────────────────────────────────────

/// Every author_id present in the live AccessKit tree.
pub fn author_ids<S>(harness: &Harness<'_, S>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The `value` of the AccessKit node carrying `author_id`, or `None` when absent.
pub fn author_node_value<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.value().map(|v| v.to_owned());
        }
    }
    None
}

// ── Honest live-PG gate for the requires_pg scenarios (no mock smuggling, no silent skip, NO SQLite) ──

/// The default backend base — the managed handshake_core dev port (matches the WP-011/012 test base).
pub const DEFAULT_BASE: &str = "http://127.0.0.1:37501";

/// A resolved, reachable live backend handle for the PG-binding interconnection scenarios. Construction
/// proves the precondition (env + reachability); the request helpers then hit real routes on a private
/// runtime. Mirrors MT-044 `pg_proof_support::LiveBackend`.
pub struct LiveBackend {
    pub base: String,
    pub workspace_id: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
}

/// Resolve + verify the live backend, or PANIC with a `requires_pg` message (no fake-pass, no skip, no
/// SQLite — the only durable authority is real PostgreSQL behind handshake_core).
pub fn require_live_backend() -> LiveBackend {
    let base = std::env::var("HSK_TEST_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_owned());
    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID").unwrap_or_default();
    assert!(
        !workspace_id.is_empty(),
        "requires_pg: set HSK_TEST_WORKSPACE_ID to a seeded workspace on a live handshake_core + \
         PostgreSQL backend ({base}). This interconnection proof is honestly gated — it never mocks PG."
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build the current-thread runtime for the live-backend interconnection proof");
    let client = shared_http_client();

    let base_for_health = base.clone();
    let client_for_health = client.clone();
    let healthy = rt.block_on(async move {
        match client_for_health
            .get(format!("{base_for_health}/health"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    });
    assert!(
        healthy,
        "requires_pg: handshake_core is not reachable at {base}/health. Start the managed backend + \
         PostgreSQL, then run with --ignored. This proof never fakes PG."
    );

    LiveBackend {
        base,
        workspace_id,
        client,
        rt,
    }
}

impl LiveBackend {
    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .post(&url)
                .json(body)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: POST {url} failed: {e}"));
            (resp.status(), resp.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "POST {path} -> {status}: {text}");
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("POST {path} response not JSON ({e}): {text}"))
    }

    pub fn put_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .put(&url)
                .json(body)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: PUT {url} failed: {e}"));
            (resp.status(), resp.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "PUT {path} -> {status}: {text}");
        if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        }
    }

    pub fn get_json(&self, path: &str) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .unwrap_or_else(|e| panic!("requires_pg: GET {url} failed: {e}"));
            (resp.status(), resp.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "GET {path} -> {status}: {text}");
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("GET {path} response not JSON ({e}): {text}"))
    }

    pub fn get_status(&self, path: &str) -> u16 {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            match self.client.get(&url).send().await {
                Ok(resp) => resp.status().as_u16(),
                Err(_) => 0,
            }
        })
    }

    /// Best-effort DELETE for idempotent cleanup (DropGuards). NEVER panics on a non-success or transport
    /// error — a cleanup failure must not mask the proof's own verdict.
    pub fn delete(&self, path: &str) -> u16 {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            match self.client.delete(&url).send().await {
                Ok(resp) => resp.status().as_u16(),
                Err(_) => 0,
            }
        })
    }
}
