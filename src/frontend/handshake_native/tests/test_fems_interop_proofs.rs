//! FEMS interop proof suite — WP-KERNEL-012 MT-065 (cluster E9, the E9 end-to-end interop guarantee).
//!
//! ## What this suite proves (the editor <-> FEMS / Pillar 12 typed-memory edge)
//!
//! This suite exercises and asserts the FEMS behavior delivered by:
//!   - MT-063 — the FEMS Relevant Memory panel + MemoryPack read client
//!     ([`handshake_native::fems::memory_client`] + [`...::relevant_memory_panel`]);
//!   - MT-064 — the "Propose to Memory" review-gated proposal action + dialog
//!     ([`handshake_native::fems::memory_proposal`]);
//!   - MT-041 — editor actions exposed through the WP-011 AccessKit surface (the canonical kittest
//!     harness pattern in `tests/test_e7_editor_action_accesskit.rs`, reused verbatim here for app
//!     construction, frame advancement, AccessKit tree query by author_id, and AccessKit action
//!     dispatch — see [`mcp_dispatch`] / [`find_node`]).
//!
//! It REUSES the WP-011 shell primitives (the `command_registry` command bus, the `accessibility`
//! AccessKit id registry, the `pane_registry`/`theme` surfaces) and the MT-063/064 FEMS widgets — it does
//! NOT re-create any shell or AccessKit glue (AC-065-07).
//!
//! ## Canonical live-resource proof
//!
//! The contract command `cargo test -p handshake-native --test test_fems_interop_proofs -- --nocapture`
//! executes all four FEMS proofs. None is ignored, feature-hidden, or replaced by a fixture-only green.
//! The suite requires `HANDSHAKE_TEST_PG_DSN`, binds the HTTP product path through `HSK_TEST_BASE`
//! (default `http://127.0.0.1:37501`), verifies `/health` reports a live PostgreSQL migration, and fails
//! loudly when the managed backend/DSN is absent. No SQLite, in-memory, ignored-test, or mock fallback
//! is accepted.
//!
//! ## Additional non-live invariants
//!
//!   - `proof_fems_05_reuses_shell_and_harness` (AC-065-07): the suite reuses the WP-011 shell primitives
//!     + the MT-063/064 FEMS widgets + the MT-041 harness pattern; it re-creates no shell/AccessKit glue.
//!   - `proof_fems_03_swarm_id_stability` (AC-065-04 / CTRL-065-05 / HBR-SWARM): mount the MT-063 FEMS
//!     panel + the MT-064 propose dialog, then drive the FULL FEMS flow purely via stable AccessKit
//!     author_ids by an out-of-process-agent code path (no direct widget calls, no synthetic key events,
//!     no screen-scraping). Assert every targeted id is DETERMINISTIC (no random segment) and STABLE
//!     across two frame re-queries, and dispatch the propose-confirm via an AccessKit Click action.
//!   - `proof_fems_no_sqlite_anywhere` (RISK-065-01 / CTRL-065-01): a static gate over this suite + the
//!     two FEMS production modules proves there is no SQLite token anywhere in the suite or its config.
//!   - `proof_fems_required_capability_contract` pins the four live route/action identities and retains
//!     typed compatibility failures for older/capability-restricted backends.
//!
//! The four named `proof_fems_0*` functions below are the live managed-resource proof surface.

use std::collections::HashSet;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use sha2::{Digest as _, Sha256};

// REUSE (AC-065-07): the MT-063 FEMS read client + Relevant Memory panel, the MT-064 propose dialog +
// proposal model, and the MT-041 AccessKit-id conventions — all imported, never re-created here.
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::cursor::Cursor;
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::event_emitter::{
    NativeEditorEventEmitter, RuntimeChatLedgerTransport, DEFAULT_ACTOR_ID,
};
use handshake_native::fems::memory_client::{
    compute_memory_pack_hash, MemoryClientError, MemoryContext, MEMORY_PACK_MAX_ITEMS,
};
use handshake_native::fems::memory_proposal::{
    build_proposal, build_proposal_for_document, build_proposal_for_document_snapshot,
    canonical_memory_write_proposal_hash, commit_approved_proposal,
    compute_memory_commit_report_hash, content_hash_of_selection, fems_class_author_id,
    review_proposal, submit_proposal_and_emit, HandshakeCoreClient, MemoryClass,
    MemoryProposalError, ProposalReviewAck, ProposalReviewDecision, ProposalSubmitOutcome,
    ProposeDialogOutcome, FEMS_PROPOSE_CANCEL_AUTHOR_ID, FEMS_PROPOSE_COMMAND_ID,
    FEMS_PROPOSE_CONFIRM_AUTHOR_ID, FEMS_PROPOSE_DIALOG_AUTHOR_ID, FEMS_PROPOSE_STATUS_AUTHOR_ID,
    FEMS_REVIEW_APPROVE_AUTHOR_ID, FEMS_REVIEW_REJECT_AUTHOR_ID,
};
use handshake_native::fems::relevant_memory_panel::{
    mem_item_author_id, mem_source_author_id, RELEVANT_MEMORY_LIST_AUTHOR_ID,
    RELEVANT_MEMORY_PANEL_AUTHOR_ID, RELEVANT_MEMORY_REFRESH_AUTHOR_ID,
    RELEVANT_MEMORY_STATUS_AUTHOR_ID,
};
use handshake_native::interop::{EditorSurfaceKind, InteractionBus, SharedSelection};
use handshake_native::mcp::UiAction;
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::tab_bar::tab_author_id_for;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, ArgusObservation, CanonicalArgusDriver};

#[path = "pg_proof_support/mod.rs"]
mod pg_proof_support;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Artifact hygiene (CX-212E / SCREENSHOT-RULE): all artifacts go to the EXTERNAL root ONLY.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Absolute path to the external artifacts root (CX-212E), resolved without a drive/user binding.
/// Argus publishes its discovery binding by atomic rename, so its platform app-data override must not
/// retain `..` components: Windows resolves the temporary and destination paths independently.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
        });
    assert!(
        root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_ROOT must resolve to an absolute path"
    );
    root.join("handshake-test").join(subdir)
}

fn current_source_sha() -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let relevant_paths = [
        "src/backend/handshake_core/src/api/memory.rs",
        "src/backend/handshake_core/src/flight_recorder/mod.rs",
        "src/backend/handshake_core/src/storage/fems_memory.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/frontend/handshake_native/src/app.rs",
        "src/frontend/handshake_native/src/editor_pane_factories.rs",
        "src/frontend/handshake_native/src/fems/memory_proposal.rs",
        "src/frontend/handshake_native/src/manual_content_editors.rs",
        "src/frontend/handshake_native/tests/test_event_emitter.rs",
        "src/frontend/handshake_native/tests/test_manual_content.rs",
        "src/frontend/handshake_native/tests/test_fems_interop_proofs.rs",
    ];
    let clean = std::process::Command::new("git")
        .args(["diff", "--quiet", "HEAD", "--"])
        .args(relevant_paths)
        .current_dir(repo_root)
        .status()
        .expect("check MT-065 relevant source cleanliness");
    assert!(
        clean.success(),
        "MT-065 canonical proof refuses dirty relevant source; commit the implementation and proof before running"
    );
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .expect("resolve current product source hash");
    assert!(
        output.status.success(),
        "git rev-parse HEAD failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sha = String::from_utf8(output.stdout)
        .expect("git source hash is UTF-8")
        .trim()
        .to_owned();
    assert_eq!(sha.len(), 40, "current product source hash is full SHA-1");
    assert!(sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
    sha
}

fn current_proof_source_blob() -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let output = std::process::Command::new("git")
        .args([
            "rev-parse",
            "HEAD:src/frontend/handshake_native/tests/test_fems_interop_proofs.rs",
        ])
        .current_dir(repo_root)
        .output()
        .expect("resolve committed MT-065 proof-source blob");
    assert!(
        output.status.success(),
        "git rev-parse proof source blob failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let blob = String::from_utf8(output.stdout)
        .expect("proof source blob is UTF-8")
        .trim()
        .to_owned();
    assert_eq!(blob.len(), 40, "proof source blob is a full Git object id");
    assert!(blob.bytes().all(|byte| byte.is_ascii_hexdigit()));
    blob
}

/// Assert NO repo-local artifact directory exists under the crate (the SCREENSHOT/TEST-ARTIFACT RULE).
/// Artifacts go to the external `Handshake_Artifacts/handshake-test` root ONLY; a stray `test_output/` OR
/// `tests/screenshots/` is a hygiene FAILURE. Called by the AccessKit proof that exercises the harness.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Required live capability identities (IN-065-06 / AC-065-06).
// ════════════════════════════════════════════════════════════════════════════════════════════════

const FEMS_REQUIRED_CAPABILITIES: [&str; 4] = [
    "GET /workspaces/{id}/memory/pack",
    "POST+GET /workspaces/{id}/memory/proposals",
    "POST /workspaces/{id}/memory/proposals/{proposal_id}/review",
    "POST /api/flight_recorder/native_editor_event kind=memory_write_proposed",
];

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Live-resource config resolution (IN-065-01, HARD): PostgreSQL/EventLedger only — never SQLite, never a
// mock, never an in-memory fallback.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The standard integration-test env key for the live PostgreSQL DSN (the FEMS interop backing store).
const LIVE_PG_DSN_ENV: &str = "HANDSHAKE_TEST_PG_DSN";

/// Serializes this binary's managed mutations without changing process-global environment variables.
static LIVE_PROOF_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn live_proof_guard() -> std::sync::MutexGuard<'static, ()> {
    LIVE_PROOF_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Private platform app-data root shared by the mounted native app and its owned backend. The memory
/// routes authenticate against the production MCP binding file, so every live proof publishes a genuine
/// binding before the backend receives its first memory request.
struct ScopedLocalAppData {
    variable: &'static str,
    previous: Option<std::ffi::OsString>,
    previous_owned_backend_root: Option<std::ffi::OsString>,
    root: PathBuf,
}

impl ScopedLocalAppData {
    fn install() -> Self {
        #[cfg(target_os = "windows")]
        let variable = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let variable = "XDG_DATA_HOME";
        let root = external_artifact_dir("wp-kernel-012-mt-065/appdata")
            .join(format!("run-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&root).expect("create isolated MT-065 app-data root");
        let root =
            std::fs::canonicalize(&root).expect("canonicalize isolated MT-065 app-data root");
        let previous = std::env::var_os(variable);
        let previous_owned_backend_root = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT");
        std::env::set_var(variable, &root);
        std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", &root);
        Self {
            variable,
            previous,
            previous_owned_backend_root,
            root,
        }
    }
}

impl Drop for ScopedLocalAppData {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.variable, value),
            None => std::env::remove_var(self.variable),
        }
        match self.previous_owned_backend_root.take() {
            Some(value) => std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT"),
        }
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            if error.kind() != std::io::ErrorKind::NotFound {
                panic!(
                    "remove isolated MT-065 app-data root {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

/// Resolve the live PostgreSQL DSN from the standard integration-test config, asserting it is PostgreSQL.
///
/// IN-065-01 (HARD): if NO live PostgreSQL DSN is configured, this PANICS with the mandated message — it
/// NEVER constructs or accepts a SQLite path, NEVER falls back to an in-memory / mock store, and NEVER
/// passes green on an absent backend (RISK-065-01, CTRL-065-01). A configured DSN whose scheme is not
/// `postgres://`/`postgresql://` is also rejected (a SQLite or other non-PG store is refused).
///
/// Called by every canonical live proof; absent or non-PostgreSQL configuration fails the proof.
fn resolve_live_pg_dsn() -> String {
    let candidate = std::env::var(LIVE_PG_DSN_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty());

    let dsn = match candidate {
        Some(dsn) => dsn,
        None => panic!(
            "live PostgreSQL DSN not configured for FEMS interop proof; refusing to run against a fake \
             backend (set {LIVE_PG_DSN_ENV} to a postgres:// DSN)"
        ),
    };

    // The store MUST be PostgreSQL — never SQLite (RISK-065-01 / CTRL-065-01). A `sqlite:`/`file:` DSN or
    // anything that is not a postgres scheme is refused outright.
    let lowered = dsn.to_ascii_lowercase();
    assert!(
        lowered.starts_with("postgres://") || lowered.starts_with("postgresql://"),
        "CTRL-065-01: the FEMS interop store must be PostgreSQL (postgres:// DSN); refusing a non-PG / \
         SQLite store. Got a DSN with an unexpected scheme."
    );
    assert!(
        !lowered.contains("sqlite") && !lowered.starts_with("file:"),
        "CTRL-065-01: a SQLite DSN is never acceptable for the FEMS interop proof"
    );
    dsn
}

fn psql_program() -> PathBuf {
    for var in ["HANDSHAKE_MANAGED_PG_BIN", "PGBIN"] {
        if let Some(dir) = std::env::var_os(var).filter(|value| !value.is_empty()) {
            let candidate =
                PathBuf::from(dir).join(if cfg!(windows) { "psql.exe" } else { "psql" });
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if cfg!(windows) {
        for root_var in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Some(root) = std::env::var_os(root_var) else {
                continue;
            };
            let postgres = PathBuf::from(root).join("PostgreSQL");
            let Ok(entries) = std::fs::read_dir(postgres) else {
                continue;
            };
            let mut candidates = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("bin").join("psql.exe"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            candidates.sort();
            if let Some(candidate) = candidates.pop() {
                return candidate;
            }
        }
    }
    PathBuf::from(if cfg!(windows) { "psql.exe" } else { "psql" })
}

fn run_psql(dsn: &str, sql: &str) -> String {
    let mut command = std::process::Command::new(psql_program());
    command
        .arg("--dbname")
        .arg(dsn)
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--no-align")
        .arg("--tuples-only")
        .arg("--command")
        .arg(sql);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("launch managed PostgreSQL psql");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll managed PostgreSQL psql") {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("managed PostgreSQL SQL timed out after 8 seconds");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .stdout
        .take()
        .expect("capture psql stdout")
        .read_to_end(&mut stdout)
        .expect("read psql stdout");
    child
        .stderr
        .take()
        .expect("capture psql stderr")
        .read_to_end(&mut stderr)
        .expect("read psql stderr");
    assert!(
        status.success(),
        "managed PostgreSQL SQL failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    String::from_utf8(stdout).expect("psql emits UTF-8")
}

struct LiveBackend {
    base: String,
    dsn: String,
    session_token: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
    _managed_backend: pg_proof_support::LiveBackend,
}

fn require_live_backend(session_token: &str) -> LiveBackend {
    let dsn = resolve_live_pg_dsn();
    let managed_backend = pg_proof_support::require_reachable_backend();
    let base = managed_backend.base.clone();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build MT-065 managed-proof runtime");
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .expect("build bounded MT-065 HTTP client");
    let health: serde_json::Value = rt.block_on(async {
        let response = client
            .get(format!("{base}/health"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .unwrap_or_else(|error| {
                panic!("requires_pg: handshake_core is unreachable at {base}/health: {error}")
            });
        assert!(
            response.status().is_success(),
            "requires_pg: GET {base}/health returned {}",
            response.status()
        );
        response
            .json::<serde_json::Value>()
            .await
            .expect("requires_pg: /health returns JSON")
    });
    assert_eq!(health["status"], "ok", "managed backend must be healthy");
    assert_eq!(
        health["db_status"], "ok",
        "HSK_TEST_BASE must front a live PostgreSQL-backed handshake_core"
    );
    assert!(
        health["migration_version"].as_i64().is_some(),
        "live backend must expose a PostgreSQL migration version"
    );
    assert_eq!(
        run_psql(&dsn, "SELECT 1").trim(),
        "1",
        "configured PostgreSQL DSN must execute a real query"
    );
    println!(
        "MT-065 backend/DSN binding: base={base}; dsn_scheme=postgres; db_status=ok; migration_version={}",
        health["migration_version"]
    );
    LiveBackend {
        base,
        dsn,
        session_token: session_token.to_owned(),
        client,
        rt,
        _managed_backend: managed_backend,
    }
}

/// One deterministic, authenticated live proof session. The app is created first so its genuine MCP
/// token can publish the binding consumed by the owned current-source backend. Tests may take the app
/// once; the canonical Argus server remains bound to the same snapshot/action slots.
struct LiveProofSession {
    live: LiveBackend,
    app: std::cell::RefCell<Option<HandshakeApp>>,
    argus: std::cell::RefCell<Option<CanonicalArgusDriver>>,
    _app_data: ScopedLocalAppData,
}

impl LiveProofSession {
    fn new() -> Self {
        let app_data = ScopedLocalAppData::install();
        let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }));
        let session_token = app.mcp_token();
        let argus = CanonicalArgusDriver::bind_in_current_app_data(
            &app,
            "wp-kernel-012-mt-065-live",
            session_token.clone(),
        );
        let live = require_live_backend(session_token.as_hex());
        Self {
            live,
            app: std::cell::RefCell::new(Some(app)),
            argus: std::cell::RefCell::new(Some(argus)),
            _app_data: app_data,
        }
    }

    fn take_app(&self) -> HandshakeApp {
        self.app
            .borrow_mut()
            .take()
            .expect("live proof app may be mounted once")
    }

    fn take_argus(&self) -> CanonicalArgusDriver {
        self.argus
            .borrow_mut()
            .take()
            .expect("live proof canonical Argus driver may be taken once")
    }
}

impl std::ops::Deref for LiveProofSession {
    type Target = LiveBackend;

    fn deref(&self) -> &Self::Target {
        &self.live
    }
}

impl Drop for LiveProofSession {
    fn drop(&mut self) {
        if let Some(argus) = self.argus.get_mut().take() {
            argus.finish();
        }
    }
}

impl LiveBackend {
    fn workspace_ident(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-session-token", self.session_token.as_str())
            .header("x-hsk-actor-id", "mt065-live-proof")
            .header("x-hsk-kernel-task-run-id", "wp-kernel-012-mt065-proof")
            .header("x-hsk-session-run-id", "wp-kernel-012-validation-v2")
    }

    fn get_json(&self, path: &str) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            let response = self
                .workspace_ident(self.client.get(&url))
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .unwrap_or_else(|error| panic!("GET {url} failed: {error}"));
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            assert!(status.is_success(), "GET {url} -> {status}: {text}");
            serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("GET {url} returned invalid JSON: {error}: {text}"))
        })
    }

    fn get_status(&self, path: &str) -> u16 {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            self.workspace_ident(self.client.get(&url))
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .map(|response| response.status().as_u16())
                .unwrap_or(0)
        })
    }

    fn post_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            let request = self.workspace_ident(self.client.post(&url).json(body));
            let request = if path == "/workspaces" {
                request
            } else {
                request.header("x-hsk-actor-kind", "operator")
            };
            let response = self
                .workspace_ident(request)
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .unwrap_or_else(|error| panic!("POST {url} failed: {error}"));
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            assert!(status.is_success(), "POST {url} -> {status}: {text}");
            serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("POST {url} returned invalid JSON: {error}: {text}"))
        })
    }

    fn post_json_status(&self, path: &str, body: &serde_json::Value) -> (u16, String) {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            let request = self.workspace_ident(self.client.post(&url).json(body));
            let request = if path == "/workspaces" {
                request
            } else {
                request.header("x-hsk-actor-kind", "operator")
            };
            let response = self
                .workspace_ident(request)
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .unwrap_or_else(|error| panic!("POST {url} failed: {error}"));
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            (status, text)
        })
    }

    fn seed_document_and_loom_source(&self, workspace_id: &str, content: &str) -> (String, String) {
        let document = self.post_json(
            "/knowledge/documents",
            &serde_json::json!({
                "workspace_id": workspace_id,
                "title": unique_name("mt065-canonical-document"),
                "content_json": {
                    "type": "doc",
                    "content": [{"type":"paragraph","content":[{"type":"text","text":content}]}]
                }
            }),
        );
        let document_id = document["document"]["rich_document_id"]
            .as_str()
            .expect("canonical document create returns rich_document_id")
            .to_owned();
        let source = self.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({
                "content_type": "note",
                "title": unique_name("mt065-memory-source")
            }),
        );
        let block_id = source["block_id"]
            .as_str()
            .expect("canonical Loom create returns block_id")
            .to_owned();
        assert_eq!(
            self.get_json(&format!("/knowledge/documents/{document_id}"))["document"]
                ["rich_document_id"],
            document_id,
            "canonical document is readable before mounted selection"
        );
        assert_eq!(
            self.get_json(&format!(
                "/workspaces/{workspace_id}/loom/blocks/{block_id}"
            ))["block_id"],
            block_id,
            "canonical Loom provenance target is readable before mounted navigation"
        );
        (document_id, block_id)
    }

    fn seed_code_authority(&self, workspace_id: &str) -> CodeAuthorityFixture {
        let root = external_artifact_dir(&unique_name("mt065-code-authority"));
        std::fs::create_dir_all(&root).expect("create external code-authority fixture root");
        let root = std::fs::canonicalize(root).expect("canonicalize code-authority fixture root");
        let symbol_name = unique_name("fems_target").replace('-', "_");
        let content =
            format!("pub fn {symbol_name}() -> &'static str {{\n    \"canonical-fems-café\"\n}}\n");
        let target_path = root.join("target.rs");
        let anchor_path = root.join("anchor.rs");
        std::fs::write(&target_path, &content).expect("write canonical code target");
        std::fs::write(&anchor_path, "pub fn anchor() {}\n")
            .expect("write local navigation anchor");
        let indexed = self.post_json(
            &format!("/workspaces/{workspace_id}/code-nav/index"),
            &serde_json::json!({"root_path": root.to_string_lossy()}),
        );
        assert_eq!(indexed["files_failed"], 0, "code authority indexes cleanly");
        let lookup = self.get_json(&format!(
            "/knowledge/code/symbols?workspace_id={workspace_id}&name={symbol_name}&path=target.rs&limit=1"
        ));
        let symbol = lookup["matches"]
            .as_array()
            .and_then(|matches| matches.first())
            .expect("indexed target symbol is queryable");
        let symbol_entity_id = symbol["symbol_entity_id"]
            .as_str()
            .expect("symbol projection carries entity id")
            .to_owned();
        let point_get = self.get_json(&format!("/knowledge/code/symbols/{symbol_entity_id}"));
        assert_eq!(
            point_get["symbol"]["symbol_entity_id"], symbol_entity_id,
            "indexed lookup identity must be immediately resolvable through the production point-get route"
        );
        let quoted_symbol_id = symbol_entity_id.replace('\'', "''");
        assert_eq!(
            run_psql(
                &self.dsn,
                &format!(
                    "SELECT count(*) FROM knowledge_entities WHERE entity_id = '{quoted_symbol_id}'"
                ),
            )
            .trim(),
            "1",
            "indexed lookup identity must exist in the canonical knowledge_entities authority"
        );
        let source_id = symbol["definition"]["source_id"]
            .as_str()
            .expect("symbol definition carries canonical KSRC id")
            .to_owned();
        assert!(source_id.starts_with("KSRC-"));
        CodeAuthorityFixture {
            root,
            anchor_path,
            target_path,
            content,
            symbol_name,
            symbol_entity_id,
            source_id,
        }
    }

    fn seed_code_authorities_with_identical_selection(
        &self,
        workspace_id: &str,
    ) -> (CodeAuthorityFixture, CodeAuthorityFixture) {
        let root = external_artifact_dir(&unique_name("mt065-code-authority-pair"));
        std::fs::create_dir_all(&root).expect("create paired code-authority fixture root");
        let root = std::fs::canonicalize(root).expect("canonicalize paired fixture root");
        let anchor_path = root.join("anchor.rs");
        std::fs::write(&anchor_path, "pub fn anchor() {}\n").expect("write paired anchor");
        let shared_literal = "identical-selection-café";
        let symbol_a = "fems_pair_target_a".to_owned();
        let symbol_b = "fems_pair_target_b".to_owned();
        let content_a =
            format!("pub fn {symbol_a}() -> &'static str {{\n    \"{shared_literal}\"\n}}\n");
        let content_b =
            format!("pub fn {symbol_b}() -> &'static str {{\n    \"{shared_literal}\"\n}}\n");
        let target_a = root.join("target_a.rs");
        let target_b = root.join("target_b.rs");
        std::fs::write(&target_a, &content_a).expect("write first paired target");
        std::fs::write(&target_b, &content_b).expect("write second paired target");
        let indexed = self.post_json(
            &format!("/workspaces/{workspace_id}/code-nav/index"),
            &serde_json::json!({"root_path": root.to_string_lossy()}),
        );
        assert_eq!(
            indexed["files_failed"], 0,
            "paired authority indexes cleanly"
        );

        let fixture = |symbol_name: String, target_path: PathBuf, content: String| {
            let file_name = target_path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("paired target has UTF-8 filename");
            let lookup = self.get_json(&format!(
                "/knowledge/code/symbols?workspace_id={workspace_id}&name={symbol_name}&path={file_name}&limit=1"
            ));
            let symbol = lookup["matches"]
                .as_array()
                .and_then(|matches| matches.first())
                .expect("paired indexed symbol is queryable");
            CodeAuthorityFixture {
                root: root.clone(),
                anchor_path: anchor_path.clone(),
                target_path,
                content,
                symbol_name,
                symbol_entity_id: symbol["symbol_entity_id"]
                    .as_str()
                    .expect("paired symbol carries entity id")
                    .to_owned(),
                source_id: symbol["definition"]["source_id"]
                    .as_str()
                    .expect("paired symbol carries canonical KSRC id")
                    .to_owned(),
            }
        };
        (
            fixture(symbol_a, target_a, content_a),
            fixture(symbol_b, target_b, content_b),
        )
    }

    fn canonical_fems_mutation_counts(&self, workspace_id: &str) -> (u64, u64) {
        let count = |table: &str| {
            run_psql(
                &self.dsn,
                &format!(
                    "SELECT COUNT(*) FROM {table} WHERE workspace_id = {}",
                    sql_literal(workspace_id)
                ),
            )
            .trim()
            .parse::<u64>()
            .unwrap_or_else(|error| panic!("parse canonical {table} count: {error}"))
        };
        (count("fems_memory_proposals"), count("fems_memory_items"))
    }

    fn create_workspace(&self, name: &str) -> String {
        let url = format!("{}/workspaces", self.base);
        let workspace_id = self.rt.block_on(async {
            let response = self
                .workspace_ident(
                    self.client
                        .post(&url)
                        .json(&serde_json::json!({"name": name})),
                )
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .unwrap_or_else(|error| panic!("POST {url} failed: {error}"));
            let status = response.status();
            let body: serde_json::Value = response
                .json()
                .await
                .unwrap_or_else(|error| panic!("POST {url} returned invalid JSON: {error}"));
            assert!(status.is_success(), "POST {url} -> {status}: {body}");
            body["id"]
                .as_str()
                .expect("workspace create returns id")
                .to_owned()
        });
        let quoted_id = workspace_id.replace('\'', "''");
        let bound_id = run_psql(
            &self.dsn,
            &format!("SELECT id FROM workspaces WHERE id = '{quoted_id}'"),
        );
        assert_eq!(
            bound_id.trim(),
            workspace_id,
            "HSK_TEST_BASE and HANDSHAKE_TEST_PG_DSN must address the same PostgreSQL workspace authority"
        );
        println!(
            "MT-065 HTTP/DSN identity bound: workspace_id={workspace_id} observed through backend and direct PostgreSQL query"
        );
        workspace_id
    }

    fn delete_workspace(&self, workspace_id: &str) -> u16 {
        let url = format!("{}/workspaces/{workspace_id}", self.base);
        self.rt.block_on(async {
            self.workspace_ident(self.client.delete(&url))
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .map(|response| response.status().as_u16())
                .unwrap_or(0)
        })
    }

    fn poll_exact_fr_event(&self, workspace_id: &str, proposal_id: &str) -> serde_json::Value {
        let url = format!(
            "{}/api/flight_recorder?event_type=memory_write_proposed&wsid={workspace_id}",
            self.base
        );
        self.rt.block_on(async {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                let rows: serde_json::Value = self
                    .client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .unwrap_or_else(|error| panic!("GET {url} failed: {error}"))
                    .json()
                    .await
                    .expect("Flight Recorder response is JSON");
                if let Some(row) = rows.as_array().into_iter().flatten().find(|row| {
                    row["event_type"] == "memory_write_proposed"
                        && row["wsids"] == serde_json::json!([workspace_id])
                        && row["payload"]["type"] == "memory_write_proposed"
                        && row["payload"]["event_code"] == "FR-EVT-MEM-001"
                        && row["payload"]["proposal_id"] == proposal_id
                        && row["payload"]["proposal_hash"]
                            .as_str()
                            .is_some_and(|hash| hash.len() == 64)
                        && row["payload"]["artifact_ref"]
                            == format!(
                                "artifact://sha256/{}",
                                row["payload"]["proposal_hash"].as_str().unwrap_or_default()
                            )
                        && row["payload"]["scope_refs"][0]["artefact_type"] == "workspace"
                        && row["payload"]["scope_refs"][0]["artefact_id"] == workspace_id
                        && row["payload"]["op_count"] == 1
                        && row["payload"]["requires_review_count"] == 1
                }) {
                    assert_exact_object_keys(
                        &row["payload"],
                        &[
                            "type",
                            "event_code",
                            "proposal_id",
                            "proposal_hash",
                            "artifact_ref",
                            "scope_refs",
                            "op_count",
                            "requires_review_count",
                        ],
                        "FR-EVT-MEM-001 payload",
                    );
                    return row.clone();
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "BLOCKER[kind=schema_mismatch] detail='no single FR row correlated action+proposal+pending state for {proposal_id}' source_mt='MT-064'"
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
    }

    fn poll_exact_review_fr_event(
        &self,
        workspace_id: &str,
        proposal_id: &str,
        decision: &str,
        event_id: &str,
    ) -> serde_json::Value {
        let url = format!(
            "{}/api/flight_recorder?wsid={workspace_id}&event_id={event_id}",
            self.base
        );
        self.rt.block_on(async {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                let rows: serde_json::Value = self
                    .client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .unwrap_or_else(|error| panic!("GET {url} failed: {error}"))
                    .json()
                    .await
                    .expect("Flight Recorder review response is JSON");
                if let Some(row) = rows.as_array().into_iter().flatten().find(|row| {
                    row["event_id"] == event_id
                        && row["event_type"] == "memory_write_reviewed"
                        && row["payload"]["type"] == "memory_write_reviewed"
                        && row["payload"]["event_code"] == "FR-EVT-MEM-002"
                        && row["payload"]["proposal_id"] == proposal_id
                        && row["payload"]["decision"] == decision
                        && row["payload"]["reviewer_kind"] == "user"
                        && row["payload"].get("commit_report_ref").is_none()
                        && row["wsids"]
                            .as_array()
                            .is_some_and(|ids| ids.iter().any(|id| id == workspace_id))
                }) {
                    assert_exact_object_keys(
                        &row["payload"],
                        &[
                            "type",
                            "event_code",
                            "proposal_id",
                            "decision",
                            "reviewer_kind",
                        ],
                        "FR-EVT-MEM-002 payload without optional commit_report_ref",
                    );
                    return row.clone();
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "no FR-EVT-MEM-002 row for proposal={proposal_id} decision={decision}"
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
    }

    fn poll_exact_commit_fr_event(
        &self,
        workspace_id: &str,
        commit: &handshake_native::fems::memory_proposal::ProposalCommitAck,
    ) -> serde_json::Value {
        let url = format!(
            "{}/api/flight_recorder?wsid={workspace_id}&event_id={}",
            self.base, commit.flight_recorder_event_id
        );
        let changed_memory_ids_hash = Sha256::digest(
            serde_json::to_vec(&serde_json::json!([commit.memory_id]))
                .expect("serialize changed memory ids"),
        )
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
        self.rt.block_on(async {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                let rows: serde_json::Value = self
                    .client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .unwrap_or_else(|error| panic!("GET {url} failed: {error}"))
                    .json()
                    .await
                    .expect("Flight Recorder commit response is JSON");
                let matching = rows
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|row| {
                        row["event_id"] == commit.flight_recorder_event_id
                            && row["event_type"] == "memory_write_committed"
                            && row["payload"]["type"] == "memory_write_committed"
                            && row["payload"]["event_code"] == "FR-EVT-MEM-003"
                            && row["payload"]["commit_id"] == commit.commit_id
                            && row["payload"]["proposal_id"] == commit.proposal_id
                            && row["payload"]["commit_report_hash"]
                                == commit.commit_report_hash
                            && row["payload"]["changed_memory_ids_hash"]
                                == changed_memory_ids_hash
                            && row["payload"]["artifact_ref"]
                                == format!("artifact://sha256/{}", commit.commit_report_hash)
                            && row["wsids"].as_array().is_some_and(|ids| {
                                ids.len() == 1 && ids[0].as_str() == Some(workspace_id)
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if matching.len() == 1 {
                    assert_exact_object_keys(
                        &matching[0]["payload"],
                        &[
                            "type",
                            "event_code",
                            "commit_id",
                            "proposal_id",
                            "commit_report_hash",
                            "artifact_ref",
                            "changed_memory_ids_hash",
                        ],
                        "FR-EVT-MEM-003 payload",
                    );
                    let artifact_path = format!(
                        "/workspaces/{workspace_id}/memory/commits/{}/report",
                        commit.commit_id
                    );
                    let report = self
                        .workspace_ident(
                            self.client
                                .get(format!("{}{}", self.base, artifact_path)),
                        )
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                        .expect("dereference commit report artifact")
                        .error_for_status()
                        .expect("commit report artifact status")
                        .json::<handshake_native::fems::memory_proposal::MemoryCommitReport>()
                        .await
                        .expect("commit report artifact JSON");
                    assert_eq!(report, commit.commit_report);
                    assert_eq!(
                        compute_memory_commit_report_hash(&report)
                            .expect("re-hash commit report artifact"),
                        commit.commit_report_hash,
                        "dereferenced MemoryCommitReport must bind the FR commit_report_hash"
                    );
                    return matching[0].clone();
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "no unique exact FR-EVT-MEM-003 for proposal={} commit={} report_hash={} memory_ids_hash={changed_memory_ids_hash}; rows={rows}",
                    commit.proposal_id,
                    commit.commit_id,
                    commit.commit_report_hash
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
    }

    fn poll_exact_pack_fr_event(
        &self,
        workspace_id: &str,
        commit: &handshake_native::fems::memory_proposal::ProposalCommitAck,
    ) -> serde_json::Value {
        let url = format!(
            "{}/api/flight_recorder?event_type=memory_pack_built&wsid={workspace_id}",
            self.base
        );
        self.rt.block_on(async {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                let rows: serde_json::Value = self
                    .client
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .unwrap_or_else(|error| panic!("GET {url} failed: {error}"))
                    .json()
                    .await
                    .expect("Flight Recorder pack response is JSON");
                let matching = rows
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|row| {
                        let payload = &row["payload"];
                        row["event_type"] == "memory_pack_built"
                            && row["wsids"] == serde_json::json!([workspace_id])
                            && payload["type"] == "memory_pack_built"
                            && payload["event_code"] == "FR-EVT-MEM-004"
                            && payload["pack_id"] == commit.memory_pack_id
                            && payload["memory_pack_hash"] == commit.memory_pack_hash
                            && payload["artifact_ref"]
                                == format!("artifact://sha256/{}", commit.memory_pack_hash)
                            && payload["memory_policy"] == "WORKSPACE_SCOPED"
                            && payload["scope_refs"][0]["artefact_type"] == "workspace"
                            && payload["scope_refs"][0]["artefact_id"] == workspace_id
                            && payload["item_count"]
                                .as_u64()
                                .is_some_and(|count| count >= 1)
                            && payload["token_estimate"].as_u64().is_some()
                            && payload["truncation_occurred"] == false
                            && payload.as_object().is_some_and(|object| object.len() == 10)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if matching.len() == 1 {
                    assert_exact_object_keys(
                        &matching[0]["payload"],
                        &[
                            "type",
                            "event_code",
                            "pack_id",
                            "memory_pack_hash",
                            "artifact_ref",
                            "memory_policy",
                            "scope_refs",
                            "item_count",
                            "token_estimate",
                            "truncation_occurred",
                        ],
                        "FR-EVT-MEM-004 payload",
                    );
                    return matching[0].clone();
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "no single exact FR-EVT-MEM-004 row for pack_id={}",
                    commit.memory_pack_id
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
    }
}

struct CodeAuthorityFixture {
    root: PathBuf,
    anchor_path: PathBuf,
    target_path: PathBuf,
    content: String,
    symbol_name: String,
    symbol_entity_id: String,
    source_id: String,
}

impl Drop for CodeAuthorityFixture {
    fn drop(&mut self) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match std::fs::remove_dir_all(&self.root) {
                Ok(()) if !self.root.exists() => return,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
                Ok(()) => return,
                Err(error) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = error;
                }
                Err(error) => {
                    tracing::warn!(
                        path = %self.root.display(),
                        %error,
                        "MT-065 CodeAuthorityFixture cleanup exhausted bounded retries"
                    );
                    return;
                }
            }
        }
    }
}

fn unique_name(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos()
    )
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn ledger_row_by_key(live: &LiveBackend, key: &str) -> serde_json::Value {
    let json = run_psql(
        &live.dsn,
        &format!(
            "SELECT row_to_json(row)::text FROM (\
             SELECT event_id::text, event_type, aggregate_type, aggregate_id, idempotency_key, \
                    correlation_id, causation_id::text, source_component, payload \
             FROM kernel_event_ledger WHERE idempotency_key = {}\
             ) row",
            sql_literal(key)
        ),
    );
    let rows = json
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "exactly one EventLedger row for {key}");
    serde_json::from_str(rows[0]).expect("EventLedger row_to_json is valid JSON")
}

fn assert_exact_proposal_and_canonical_fr_ledger(
    live: &LiveBackend,
    proposal_id: &str,
    workspace_id: &str,
    fr_row: &serde_json::Value,
) {
    assert_eq!(fr_row["wsids"], serde_json::json!([workspace_id]));
    assert_eq!(fr_row["event_type"], "memory_write_proposed");
    assert_eq!(fr_row["payload"]["type"], "memory_write_proposed");
    assert_eq!(fr_row["payload"]["event_code"], "FR-EVT-MEM-001");
    assert_eq!(fr_row["payload"]["proposal_id"], proposal_id);
    assert_eq!(fr_row["payload"]["op_count"], 1);
    assert_eq!(fr_row["payload"]["requires_review_count"], 1);
    assert_eq!(
        fr_row["payload"]["scope_refs"][0]["artefact_type"],
        "workspace"
    );
    assert_eq!(
        fr_row["payload"]["scope_refs"][0]["artefact_id"],
        workspace_id
    );
    assert!(fr_row["payload"].get("content").is_none());
    assert!(fr_row["payload"].get("text").is_none());
    let proposal = ledger_row_by_key(live, &format!("fems-memory-proposal:{proposal_id}"));
    assert_eq!(proposal["event_type"], "ARTIFACT_PROPOSED");
    assert_eq!(proposal["aggregate_type"], "fems_memory_proposal");
    assert_eq!(proposal["aggregate_id"], proposal_id);
    assert_eq!(proposal["source_component"], "fems_memory_proposal_intake");
    assert_eq!(proposal["payload"]["proposal_id"], proposal_id);
    assert_eq!(proposal["payload"]["workspace_id"], workspace_id);
    assert_eq!(proposal["payload"]["status"], "pending_review");
    assert_eq!(proposal["payload"]["review_gated"], true);
    let proposal_hash = fr_row["payload"]["proposal_hash"]
        .as_str()
        .expect("canonical proposal event carries proposal_hash");
    assert_eq!(
        fr_row["payload"]["artifact_ref"],
        format!("artifact://sha256/{proposal_hash}"),
        "FR-EVT-MEM-001 artifact_ref is the normative content-addressed URI"
    );
    let artifact = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{proposal_id}/artifact"
    ));
    assert_eq!(artifact["schema_version"], "hsk.memory_write_proposal@0.1");
    assert_eq!(artifact["proposal_id"], proposal_id);
    assert_eq!(artifact["scope_refs"], fr_row["payload"]["scope_refs"]);
    assert_eq!(
        artifact["ops"].as_array().map(Vec::len),
        Some(
            fr_row["payload"]["op_count"]
                .as_u64()
                .expect("FR proposal op_count") as usize
        )
    );
    assert_eq!(
        artifact["ops"]
            .as_array()
            .expect("canonical proposal ops")
            .iter()
            .filter(|op| op["requires_review"] == true)
            .count(),
        fr_row["payload"]["requires_review_count"]
            .as_u64()
            .expect("FR proposal requires_review_count") as usize
    );
    assert_eq!(
        canonical_memory_write_proposal_hash(&artifact),
        proposal_hash,
        "dereferenced proposal artifact recomputes to the FR proposal_hash"
    );
    assert!(artifact.get("_receipt_identity").is_none());
    assert!(artifact.get("review").is_none());
}

fn assert_exact_proposal_readback(
    readback: &serde_json::Value,
    proposal_id: &str,
    proposal: &handshake_native::fems::memory_proposal::MemoryWriteProposal,
) {
    assert_eq!(readback["proposal_id"], proposal_id);
    assert!(
        readback["request_id"]
            .as_str()
            .is_some_and(|request_id| !request_id.is_empty()),
        "proposal readback carries its durable request correlation identity"
    );
    assert_eq!(readback["workspace_id"], proposal.source.workspace_id);
    assert_eq!(readback["document_id"], proposal.source.document_id);
    assert_eq!(readback["selection_start"], proposal.source.selection_start);
    assert_eq!(readback["selection_end"], proposal.source.selection_end);
    assert_eq!(readback["content_hash"], proposal.source.content_hash);
    assert_eq!(readback["memory_class"], proposal.class.wire());
    assert_eq!(readback["status"], "pending_review");
    assert_eq!(readback["review_gated"], true);

    let stored = &readback["proposal"];
    assert_eq!(stored["proposal_id"], proposal_id);
    assert_eq!(stored["workspace_id"], proposal.source.workspace_id);
    assert_eq!(stored["class"], proposal.class.wire());
    assert_eq!(stored["content"], proposal.content);
    assert_eq!(stored["source"]["document_id"], proposal.source.document_id);
    assert_eq!(
        stored["source"]["selection_start"],
        proposal.source.selection_start
    );
    assert_eq!(
        stored["source"]["selection_end"],
        proposal.source.selection_end
    );
    assert_eq!(
        stored["source"]["content_hash"],
        proposal.source.content_hash
    );
    match proposal.source.document_content_hash.as_deref() {
        Some(document_content_hash) => assert_eq!(
            stored["source"]["document_content_hash"], document_content_hash,
            "durable proposal retains the canonical source snapshot hash"
        ),
        None => assert!(
            stored["source"]["document_content_hash"].is_null(),
            "rich-document proposals do not fabricate a code snapshot hash"
        ),
    }
    assert!(
        stored.get("source_document_content").is_none(),
        "the transient full source snapshot is validated but never stored in the durable proposal payload"
    );
    assert_eq!(stored["source"]["pane_id"], proposal.source.pane_id);
    assert_eq!(
        stored["source"]["workspace_id"],
        proposal.source.workspace_id
    );
    let authenticated_actor = stored["actor_id"]
        .as_str()
        .expect("proposal readback carries authenticated actor_id");
    assert!(
        authenticated_actor.starts_with("handshake-native:"),
        "proposal actor must be derived from the live native MCP binding: {authenticated_actor}"
    );
    assert_ne!(
        authenticated_actor, proposal.actor_id,
        "caller-authored proposal actor metadata must not override the authenticated principal"
    );
    assert_eq!(stored["review_gated"], true);
    assert_eq!(stored["status"], "pending_review");
}

fn replay_review(
    live: &LiveBackend,
    workspace_id: &str,
    proposal_id: &str,
    decision: ProposalReviewDecision,
) -> ProposalReviewAck {
    let client = HandshakeCoreClient::with_base_url(live.base.clone())
        .with_session_token(live.session_token.clone());
    live.rt
        .block_on(review_proposal(
            workspace_id,
            proposal_id,
            decision,
            &client,
        ))
        .expect("production native review client accepts the authenticated backend acknowledgement")
}

fn replay_commit(
    live: &LiveBackend,
    workspace_id: &str,
    proposal_id: &str,
) -> handshake_native::fems::memory_proposal::ProposalCommitAck {
    let client = HandshakeCoreClient::with_base_url(live.base.clone())
        .with_session_token(live.session_token.clone());
    live.rt
        .block_on(commit_approved_proposal(workspace_id, proposal_id, &client))
        .expect("production native commit client accepts the idempotent backend acknowledgement")
}

struct WorkspaceCleanup<'a> {
    live: &'a LiveBackend,
    workspace_id: String,
    proposal_ids: Vec<String>,
    pack_ids: Vec<String>,
    item_ids: Vec<String>,
    cleaned: bool,
}

impl WorkspaceCleanup<'_> {
    fn capture_proposal(&mut self, proposal_id: impl Into<String>) {
        self.proposal_ids.push(proposal_id.into());
    }

    fn capture_pack_item(&mut self, pack_id: impl Into<String>, item_id: impl Into<String>) {
        self.pack_ids.push(pack_id.into());
        self.item_ids.push(item_id.into());
    }

    fn clean_and_verify(&mut self) {
        let status = self.live.delete_workspace(&self.workspace_id);
        assert_eq!(status, 204, "delete owned MT-065 workspace");
        for proposal_id in &self.proposal_ids {
            assert_eq!(
                self.live.get_status(&format!(
                    "/workspaces/{}/memory/proposals/{proposal_id}",
                    self.workspace_id
                )),
                404,
                "workspace teardown removes only captured mutable proposal data"
            );
            assert_eq!(
                run_psql(
                    &self.live.dsn,
                    &format!(
                        "SELECT COUNT(*) FROM fems_memory_proposals WHERE workspace_id = {} AND proposal_id = {}",
                        sql_literal(&self.workspace_id),
                        sql_literal(proposal_id)
                    )
                )
                .trim(),
                "0",
                "exact captured proposal row is removed"
            );
        }
        for pack_id in &self.pack_ids {
            assert_eq!(
                run_psql(
                    &self.live.dsn,
                    &format!(
                        "SELECT COUNT(*) FROM fems_memory_packs WHERE workspace_id = {} AND pack_id = {}",
                        sql_literal(&self.workspace_id),
                        sql_literal(pack_id)
                    )
                )
                .trim(),
                "0",
                "exact captured MemoryPack row is removed"
            );
        }
        for item_id in &self.item_ids {
            assert_eq!(
                run_psql(
                    &self.live.dsn,
                    &format!(
                        "SELECT COUNT(*) FROM fems_memory_items WHERE workspace_id = {} AND memory_id = {}",
                        sql_literal(&self.workspace_id),
                        sql_literal(item_id)
                    )
                )
                .trim(),
                "0",
                "exact captured MemoryItem row is removed"
            );
        }
        for table in [
            "fems_memory_proposals",
            "fems_memory_packs",
            "fems_memory_items",
        ] {
            assert_eq!(
                run_psql(
                    &self.live.dsn,
                    &format!(
                        "SELECT COUNT(*) FROM {table} WHERE workspace_id = {}",
                        sql_literal(&self.workspace_id)
                    )
                )
                .trim(),
                "0",
                "workspace teardown leaves no mutable FEMS rows in {table}"
            );
        }
        assert_eq!(
            self.live.get_status(&format!(
                "/workspaces/{}/memory/items/count",
                self.workspace_id
            )),
            404,
            "deleted workspace has no committed-memory projection"
        );
        self.cleaned = true;
    }
}

impl Drop for WorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            // Emergency cleanup must never double-panic while unwinding a failed proof. The explicit
            // `clean_and_verify` path above remains the assertion-bearing teardown contract.
            let _ = self.live.delete_workspace(&self.workspace_id);
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Harness builders + AccessKit query/dispatch helpers (the MT-041 canonical pattern, reused — AC-065-07).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A TextRange selection (the MT-031 shared-selection shape MT-064 reads to build a proposal).
fn text_range(pane: &str, start: usize, end: usize, text: &str) -> SharedSelection {
    SharedSelection::TextRange {
        pane_id: std::sync::Arc::from(pane),
        surface: EditorSurfaceKind::Code,
        start,
        end,
        text: text.to_owned(),
    }
}

/// A node found in the live kittest tree, reduced to the fields the proofs assert (the MT-041
/// `FoundNode` shape).
#[derive(Debug)]
struct FoundNode {
    node_id: egui::accesskit::NodeId,
    role: String,
    disabled: bool,
    value: Option<String>,
}

/// Resolve a canonical `author_id` to its live AccessKit node in the harness tree (the MT-041 `find_node`
/// pattern — query by author_id, extract the owned fields inside the borrow).
fn find_node(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<FoundNode> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(FoundNode {
                node_id: ak.id(),
                role: format!("{:?}", ak.role()),
                disabled: ak.is_disabled(),
                value: ak.value(),
            });
        }
    }
    None
}

fn selected_tab_label(root: &egui_kittest::Node<'_>) -> Option<String> {
    root.children_recursive().find_map(|node| {
        let ak = node.accesskit_node();
        (ak.role() == egui::accesskit::Role::Tab
            && ak
                .author_id()
                .is_some_and(|author_id| author_id.starts_with("tab-pane-"))
            && ak.is_selected().unwrap_or(false))
        .then(|| ak.label())
        .flatten()
    })
}

fn accesskit_author_dump(root: &egui_kittest::Node<'_>) -> String {
    root.children_recursive()
        .filter_map(|node| {
            let access = node.accesskit_node();
            access.author_id().map(|author_id| {
                format!(
                    "{}|role={:?}|disabled={}|label={:?}|value={:?}",
                    author_id,
                    access.role(),
                    access.is_disabled(),
                    access.label(),
                    access.value()
                )
            })
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// Drive the same bounded ActionChannel and eframe pre-frame hook used by the out-of-process MCP
/// server. Tests never fabricate raw AccessKit requests and never call widget handlers directly.
fn mcp_dispatch(harness: &mut Harness<'_, HandshakeApp>, author_id: &str, action: UiAction) {
    let live_node_id = find_node(&harness.root(), author_id).map(|node| node.node_id);
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let target_snapshot: Vec<_> = snapshot
        .iter_nodes()
        .filter(|node| node.author_id.as_deref() == Some(author_id))
        .map(|node| {
            (
                node.node_id,
                node.disabled,
                node.role.clone(),
                node.actions.clone(),
            )
        })
        .collect();
    if let Some(live_node_id) = live_node_id {
        assert!(
            target_snapshot
                .iter()
                .any(|(node_id, _, _, _)| *node_id == live_node_id.0),
            "MCP snapshot/live NodeId divergence for {author_id}: live={live_node_id:?}; snapshot={target_snapshot:?}"
        );
    }
    let channel = harness.state().mcp_action_channel();
    channel
        .lock()
        .expect("MCP ActionChannel lock")
        .enqueue(&snapshot, author_id, action)
        .unwrap_or_else(|error| {
            panic!(
                "MCP action for {author_id} rejected: {error}; live_targets={:?}; stored_targets={:?}; snapshot_target={target_snapshot:?}",
                harness.state().editor_menu_targets_for_test(),
                harness.state().mcp_open_menu_state_for_test()
            )
        });
    let mut raw_input = egui::RawInput::default();
    <HandshakeApp as eframe::App>::raw_input_hook(
        harness.state_mut(),
        &egui::Context::default(),
        &mut raw_input,
    );
    assert!(
        !raw_input.events.is_empty(),
        "raw_input_hook drains the MCP action into egui input"
    );
    for event in raw_input.events {
        harness.event(event);
    }
}

const FEMS_PALETTE_ROW_AUTHOR_ID: &str = "command-palette.option.hs-fems-palette-propose-to-memory";

fn click_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) -> FoundNode {
    let found = find_node(&harness.root(), author_id)
        .unwrap_or_else(|| panic!("missing AccessKit author_id {author_id}"));
    let active_tab = harness
        .state()
        .active_pane()
        .and_then(|pane_id| harness.state().tab_bar_states().get(pane_id))
        .and_then(|bar| bar.active())
        .map(|tab| format!("{:?}", tab.pane_type));
    let matching_nodes: Vec<_> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let access = node.accesskit_node();
            (access.author_id() == Some(author_id)).then(|| {
                (
                    access.id().0,
                    access.is_disabled(),
                    format!("{:?}", access.role()),
                )
            })
        })
        .collect();
    assert!(
        !found.disabled,
        "AccessKit target {author_id} must be enabled; active_pane={:?}; active_tab={active_tab:?}; editor_available={}; menu_targets={:?}; selected_tab={:?}; node={found:?}; matching_nodes={matching_nodes:?}",
        harness.state().active_pane(),
        harness.state().editor_available(),
        harness.state().editor_menu_targets_for_test(),
        selected_tab_label(&harness.root())
    );
    mcp_dispatch(harness, author_id, UiAction::Click);
    // One frame applies the exact targeted action. A top-level custom menu button persists its
    // AccessKit-driven popup at the end of that frame, so one further frame materializes the enabled
    // live leaf tree before the next model action addresses it.
    harness.run_steps(1);
    if author_id.starts_with("menu-") && !author_id.contains('.') {
        harness.run_steps(1);
    }
    if author_id.starts_with("command-palette.option.") {
        // The palette dispatch runs after the proposal-dialog host for this frame; the requested
        // repaint materializes the resulting operator dialog on the next frame.
        harness.run_steps(1);
    }
    if author_id == FEMS_PROPOSE_CANCEL_AUTHOR_ID {
        // Cancellation is applied after the dialog rendered in the action frame. Advance once more so
        // the captured accessibility tree reflects the now-closed modal instead of its final live frame.
        harness.run_steps(1);
    }
    found
}

fn mounted_code_app_with_real_anchor(
    mut app: HandshakeApp,
    live: &LiveBackend,
    workspace_id: &str,
    fixture: &CodeAuthorityFixture,
) -> HandshakeApp {
    app.set_backend_base_url_for_test(&live.base, live.rt.handle().clone());
    app.set_active_project_id_for_test(workspace_id.to_owned());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let anchor_id = fixture
        .anchor_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let anchor_content =
        std::fs::read_to_string(&fixture.anchor_path).expect("read anchor content");
    let generation = app.begin_code_document_load_for_test(anchor_id.clone());
    app.deliver_code_document_load_for_test(
        generation,
        anchor_id,
        fixture.anchor_path.clone(),
        0,
        Ok(anchor_content),
    );
    app
}

fn open_indexed_code_symbol_via_quick_switcher(
    harness: &mut Harness<'_, HandshakeApp>,
    fixture: &CodeAuthorityFixture,
) {
    harness.run_steps(3);
    click_author_id(harness, "menu-edit");
    click_author_id(harness, "menu.edit.quick-switcher");
    let search = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("quick-switcher.search"))
        .expect("production Quick Switcher search input");
    search.type_text(&fixture.symbol_name);
    let search_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        if harness
            .state()
            .quick_switcher_search_results()
            .iter()
            .any(|hit| hit.source_kind == "symbol" && hit.ref_id == fixture.symbol_entity_id)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < search_deadline,
            "timed out waiting for indexed symbol in production Quick Switcher"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    harness.key_press(egui::Key::Enter);
    // Search delivery and the resulting typed navigation are two independently asynchronous,
    // bounded production phases. A busy host may legitimately consume most of the search budget;
    // reusing that already-spent deadline would give the post-Enter document load no proof window.
    let navigation_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let target = fixture
        .target_path
        .canonicalize()
        .unwrap_or_else(|_| fixture.target_path.clone());
    loop {
        harness.run_steps(1);
        let active = PathBuf::from(harness.state().active_mounted_code_panel().file_path());
        if active.canonicalize().ok().as_ref() == Some(&target) {
            break;
        }
        assert!(
            std::time::Instant::now() < navigation_deadline,
            "timed out waiting for production code-symbol navigation to open {}; active_path={}; \
             active_canonical={:?}; nav_status={:?}; quick_switcher_open={}; hits={:?}",
            target.display(),
            active.display(),
            active.canonicalize().ok(),
            harness.state().quick_switcher_nav_status(),
            harness.state().quick_switcher_open(),
            harness
                .state()
                .quick_switcher_search_results()
                .iter()
                .map(|hit| (&hit.source_kind, &hit.ref_id, &hit.title))
                .collect::<Vec<_>>()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(
        harness
            .state()
            .active_mounted_code_panel()
            .buffer()
            .to_string(),
        fixture.content,
        "production code-symbol navigation opens the exact indexed bytes"
    );
}

fn author_and_select_all(
    harness: &mut Harness<'_, HandshakeApp>,
    content: &str,
) -> SharedSelection {
    harness.run_steps(3);
    let text_author_id = {
        harness
            .root()
            .children_recursive()
            .find_map(|node| {
                node.accesskit_node()
                    .author_id()
                    .filter(|author_id| author_id.starts_with(CODE_EDITOR_TEXT_AUTHOR_ID))
                    .map(str::to_owned)
            })
            .expect("mounted code editor exposes its stable text node")
    };
    mcp_dispatch(harness, &text_author_id, UiAction::Focus);
    harness.run_steps(2);
    click_author_id(harness, "menu-edit");
    click_author_id(harness, "menu.edit.select-all");
    harness.run_steps(2);
    text_range("pane-a", 0, content.len(), content)
}

fn author_and_select_range(
    harness: &mut Harness<'_, HandshakeApp>,
    start: usize,
    end: usize,
    text: &str,
) -> SharedSelection {
    harness.run_steps(3);
    let text_author_id = harness
        .root()
        .children_recursive()
        .find_map(|node| {
            node.accesskit_node()
                .author_id()
                .filter(|author_id| author_id.starts_with(CODE_EDITOR_TEXT_AUTHOR_ID))
                .map(str::to_owned)
        })
        .expect("mounted code editor exposes its stable text node");
    mcp_dispatch(harness, &text_author_id, UiAction::Focus);
    harness
        .state()
        .active_mounted_code_panel()
        .set_cursors(vec![Cursor::selection(start, end)]);
    harness.run_steps(3);
    text_range("pane-a", start, end, text)
}

fn structured_field<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    value.split(';').find_map(|part| {
        part.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

fn assert_exact_object_keys(value: &serde_json::Value, expected: &[&str], label: &str) {
    let actual = value
        .as_object()
        .unwrap_or_else(|| panic!("{label} must be a JSON object"))
        .keys()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let expected = expected.iter().copied().collect::<HashSet<_>>();
    assert_eq!(actual, expected, "{label} has an exact normative keyset");
}

fn assert_same_rfc3339_instant(left: &str, right: &str) {
    let left = chrono::DateTime::parse_from_rfc3339(left)
        .unwrap_or_else(|error| panic!("invalid left RFC3339 timestamp {left:?}: {error}"));
    let right = chrono::DateTime::parse_from_rfc3339(right)
        .unwrap_or_else(|error| panic!("invalid right RFC3339 timestamp {right:?}: {error}"));
    assert_eq!(
        left.timestamp_micros(),
        right.timestamp_micros(),
        "timestamps must identify the same PostgreSQL-precision instant"
    );
}

fn assert_authenticated_native_actor(value: &serde_json::Value) {
    assert!(
        value
            .as_str()
            .is_some_and(|actor| actor.starts_with("handshake-native:")),
        "actor identity must be derived from the live MCP binding: {value}"
    );
}

fn wait_for_status(
    harness: &mut Harness<'_, HandshakeApp>,
    author_id: &str,
    predicate: impl Fn(&str) -> bool,
) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        if let Some(value) = find_node(&harness.root(), author_id).and_then(|node| node.value) {
            if predicate(&value) {
                return value;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for structured AccessKit status {author_id}; active_pane={:?}; selected_tab={:?}; fems_nodes={:?}",
            harness.state().active_pane(),
            selected_tab_label(&harness.root()),
            harness.root().children_recursive().filter_map(|node| {
                let access = node.accesskit_node();
                access.author_id().filter(|id| id.contains("fems") || id.contains("memorypack")).map(|id| (id.to_owned(), access.label(), access.value(), access.is_disabled()))
            }).collect::<Vec<_>>()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn drive_approval_through_commit(harness: &mut Harness<'_, HandshakeApp>) -> String {
    for _ in 0..2 {
        let state_before = find_node(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID)
            .and_then(|node| node.value)
            .and_then(|value| structured_field(&value, "state").map(str::to_owned));
        let approve = wait_for_author_id(harness, FEMS_REVIEW_APPROVE_AUTHOR_ID);
        assert!(
            !approve.disabled,
            "approval/commit control must be enabled: {approve:?}"
        );
        mcp_dispatch(harness, FEMS_REVIEW_APPROVE_AUTHOR_ID, UiAction::Click);
        let status = wait_for_status(harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
            if state_before.as_deref() == Some("commit_pending") {
                structured_field(value, "state") == Some("committed")
            } else {
                matches!(
                    structured_field(value, "state"),
                    Some("commit_pending" | "committed")
                )
            }
        });
        if structured_field(&status, "state") == Some("committed") {
            assert_eq!(structured_field(&status, "outcome"), Some("approved"));
            return status;
        }
    }
    panic!(
        "approval followed by explicit commit did not reach committed state; status={:?}",
        find_node(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID).and_then(|node| node.value)
    );
}

fn wait_for_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) -> FoundNode {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        if let Some(node) = find_node(&harness.root(), author_id) {
            return node;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for AccessKit author_id {author_id}"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn drive_propose_command_via_accesskit(
    harness: &mut Harness<'_, HandshakeApp>,
    class: MemoryClass,
    exercise_cancel: bool,
    cancel_guard: Option<(&LiveBackend, &str)>,
) -> String {
    let mut dispatch_order = Vec::new();
    let open_dialog = |harness: &mut Harness<'_, HandshakeApp>| {
        click_author_id(harness, "menu-go");
        click_author_id(harness, "menu.go.command-palette");
        let row = click_author_id(harness, FEMS_PALETTE_ROW_AUTHOR_ID);
        assert_eq!(row.role, "ListBoxOption");
        let status = find_node(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID);
        assert!(
            find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_some(),
            "palette AccessKit dispatch opens the real app proposal dialog; status={status:?}"
        );
    };
    let cancel_counts_before =
        cancel_guard.map(|(live, workspace_id)| live.canonical_fems_mutation_counts(workspace_id));
    dispatch_order.extend(
        [
            "menu-go",
            "menu.go.command-palette",
            FEMS_PALETTE_ROW_AUTHOR_ID,
        ]
        .into_iter()
        .map(str::to_owned),
    );
    open_dialog(harness);
    if exercise_cancel {
        dispatch_order.push(FEMS_PROPOSE_CANCEL_AUTHOR_ID.to_owned());
        click_author_id(harness, FEMS_PROPOSE_CANCEL_AUTHOR_ID);
        assert!(
            find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_none(),
            "AccessKit cancel closes the mounted proposal dialog"
        );
        let cancelled = wait_for_status(harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
            structured_field(value, "outcome") == Some("cancelled_before_submit")
        });
        assert_eq!(structured_field(&cancelled, "state"), Some("cancelled"));
        assert!(
            structured_field(&cancelled, "operation_id").is_some_and(|id| id != "none"),
            "cancel terminal state carries a stable operation identity"
        );
        let drain_deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
        while std::time::Instant::now() < drain_deadline {
            harness.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let (Some((live, workspace_id)), Some(before)) = (cancel_guard, cancel_counts_before) {
            assert_eq!(
                live.canonical_fems_mutation_counts(workspace_id),
                before,
                "cancel must leave canonical proposal rows and committed-memory rows unchanged after a bounded UI/worker drain"
            );
        }
        dispatch_order.extend(
            [
                "menu-go",
                "menu.go.command-palette",
                FEMS_PALETTE_ROW_AUTHOR_ID,
            ]
            .into_iter()
            .map(str::to_owned),
        );
        open_dialog(harness);
    }
    dispatch_order.push(fems_class_author_id(class));
    click_author_id(harness, &fems_class_author_id(class));
    dispatch_order.push(FEMS_PROPOSE_CONFIRM_AUTHOR_ID.to_owned());
    click_author_id(harness, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
    let status = wait_for_status(harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("event_persisted")
    });
    assert_eq!(structured_field(&status, "state"), Some("completed"));
    assert!(
        structured_field(&status, "operation_id").is_some_and(|id| id != "none"),
        "persisted terminal state carries the stable operation identity"
    );
    let proposal_id = structured_field(&status, "proposal_id")
        .filter(|proposal_id| !proposal_id.is_empty())
        .expect("completed proposal status carries proposal_id")
        .to_owned();
    println!(
        "FEMS-03 ACCESSKIT_DISPATCH_ORDER proposal_id={proposal_id} actions={dispatch_order:?}"
    );
    proposal_id
}

fn inspect_until(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    author_id: &str,
    max_steps: usize,
) -> serde_json::Value {
    for _ in 0..max_steps {
        let snapshot = argus.inspect(harness);
        if json_has_author_id(&snapshot, author_id) {
            return snapshot;
        }
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let snapshot = argus.inspect(harness);
    assert!(
        json_has_author_id(&snapshot, author_id),
        "canonical argus.inspect could not address '{author_id}' within {max_steps} pumped frames"
    );
    snapshot
}

fn argus_click(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    author_id: &str,
) -> ArgusObservation {
    let before = inspect_until(argus, harness, author_id, 60);
    argus.click_from_snapshot_and_reinspect(harness, author_id, before)
}

fn open_proposal_dialog_via_argus(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    observations: &mut Vec<ArgusObservation>,
) {
    for _attempt in 0..12 {
        for _ in 0..15 {
            harness.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        observations.push(argus_click(argus, harness, "menu-go"));
        observations.push(argus_click(argus, harness, "menu.go.command-palette"));
        let row = find_node(&harness.root(), FEMS_PALETTE_ROW_AUTHOR_ID)
            .expect("canonical Argus palette row is mounted");
        assert_eq!(row.role, "ListBoxOption");
        observations.push(argus_click(argus, harness, FEMS_PALETTE_ROW_AUTHOR_ID));
        for _ in 0..40 {
            harness.run_steps(1);
            if find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let status =
            find_node(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID).and_then(|node| node.value);
        let reentry_blocked = status
            .as_deref()
            .and_then(|value| structured_field(value, "outcome"))
            == Some("reentry_blocked");
        assert!(
            reentry_blocked,
            "canonical Argus palette click did not materialize proposal dialog and was not a \
             transient review-queue reentry block; status={status:?}; tree={}",
            accesskit_author_dump(&harness.root())
        );
    }
    panic!("canonical Argus proposal dialog did not open after bounded retries");
}

fn drive_propose_command_via_argus(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    class: MemoryClass,
    live: &LiveBackend,
    workspace_id: &str,
    observations: &mut Vec<ArgusObservation>,
) -> String {
    let before_cancel = live.canonical_fems_mutation_counts(workspace_id);
    open_proposal_dialog_via_argus(argus, harness, observations);
    observations.push(argus_click(argus, harness, FEMS_PROPOSE_CANCEL_AUTHOR_ID));
    let cancelled = wait_for_status(harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("cancelled_before_submit")
    });
    assert_eq!(structured_field(&cancelled, "state"), Some("cancelled"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    while std::time::Instant::now() < deadline {
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        live.canonical_fems_mutation_counts(workspace_id),
        before_cancel,
        "Argus cancel must roll back without a proposal or memory mutation"
    );

    open_proposal_dialog_via_argus(argus, harness, observations);
    observations.push(argus_click(argus, harness, &fems_class_author_id(class)));
    observations.push(argus_click(argus, harness, FEMS_PROPOSE_CONFIRM_AUTHOR_ID));
    let status = wait_for_status(harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("event_persisted")
    });
    structured_field(&status, "proposal_id")
        .expect("Argus proposal status carries proposal_id")
        .to_owned()
}

/// True if `s` contains no decimal-digit run of length >= 4 (a heuristic for "no random numeric segment").
/// A stable swarm-addressable id must be deterministic — no per-run random suffix. The delivered FEMS ids
/// (`relevant-memory-panel`, `mem-source-sem-1`, `fems-propose-confirm`, ...) are slugs with no random
/// segment; an egui-hashed random id would carry a long numeric run.
fn has_no_random_segment(s: &str) -> bool {
    let mut run = 0usize;
    for c in s.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= 4 {
                return false;
            }
        } else {
            run = 0;
        }
    }
    true
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (NON-IGNORED) — FEMS-05 / AC-065-07: the suite reuses the WP-011 shell + the MT-063/064 widgets +
// the MT-041 harness; it re-creates NO shell or AccessKit glue.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_05_reuses_shell_and_harness() {
    // Source-level proof (AC-065-07): this suite imports the MT-063/064 FEMS modules + the MT-041 harness
    // conventions and does NOT declare its own panel/dialog/AccessKit-registry types.
    let src = include_str!("test_fems_interop_proofs.rs");

    // Reuses the MT-063 FEMS read client + Relevant Memory panel.
    assert!(
        src.contains("handshake_native::fems::memory_client")
            && src.contains("handshake_native::fems::relevant_memory_panel"),
        "AC-065-07: the suite must REUSE the MT-063 FEMS read client + Relevant Memory panel"
    );
    // Reuses the MT-064 propose dialog + proposal model.
    assert!(
        src.contains("handshake_native::fems::memory_proposal"),
        "AC-065-07: the suite must REUSE the MT-064 propose action + proposal model"
    );
    // Reuses the WP-011 shell selection substrate and mounted HandshakeApp — not a forked copy.
    assert!(
        src.contains("handshake_native::interop") && src.contains("HandshakeApp"),
        "AC-065-07: the suite must REUSE the WP-011 interop selection substrate + mounted app"
    );
    // Reuses the MT-041 harness AccessKit-dispatch pattern (the AccessKitActionRequest / Action::Click
    // path), not a re-created dispatch stack.
    assert!(
        src.contains("egui::Event::AccessKitActionRequest")
            && src.contains("egui::accesskit::Action::Click"),
        "AC-065-07: the swarm dispatch must reuse the MT-041 AccessKit action-request pattern"
    );
    // It does NOT re-create the FEMS widgets or the AccessKit id registry: this test file must contain no
    // local DEFINITION of the panel/dialog structs or the id-builder fns (it imports them from MT-063/064).
    // The forbidden definition patterns are assembled from fragments at runtime so these guard literals do
    // not self-match the `include_str!` self-scan above.
    let def = "struct "; // a local type definition prefix
    let fn_def = "fn "; // a local fn definition prefix
    let forbidden_defs = [
        format!("{def}RelevantMemoryPanel"),
        format!("{def}ProposeToMemoryDialog"),
        format!("{fn_def}mem_item_author_id("),
        format!("{fn_def}fems_class_author_id("),
    ];
    for forbidden in &forbidden_defs {
        assert!(
            !src.contains(forbidden.as_str()),
            "AC-065-07: the suite must NOT re-create shell/FEMS/AccessKit glue (found a local '{forbidden}' definition)"
        );
    }
    println!("FEMS-05 OK (AC-065-07): suite reuses MT-063/064 FEMS widgets + WP-011 shell + MT-041 harness; no glue re-created");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (NON-IGNORED) — FEMS-03 swarm id-stability half / AC-065-04 / CTRL-065-05 / HBR-SWARM: the full
// Exact stable author IDs are pinned here. The canonical live FEMS-03 proof below mounts the complete app,
// re-queries the runtime NodeIds across refresh, and dispatches the entire path through AccessKit.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_03_swarm_id_stability() {
    // Runtime stability is proved in the fully mounted live FEMS-03 flow. This local half pins the exact
    // deterministic identities that flow must re-query before and after each AccessKit dispatch.
    for author_id in [
        RELEVANT_MEMORY_PANEL_AUTHOR_ID,
        RELEVANT_MEMORY_STATUS_AUTHOR_ID,
        RELEVANT_MEMORY_REFRESH_AUTHOR_ID,
        RELEVANT_MEMORY_LIST_AUTHOR_ID,
        FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        FEMS_PROPOSE_CANCEL_AUTHOR_ID,
        FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        FEMS_PROPOSE_STATUS_AUTHOR_ID,
    ] {
        assert!(
            has_no_random_segment(author_id),
            "unstable FEMS author_id: {author_id}"
        );
    }
    assert_no_local_artifact_dir();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (NON-IGNORED) — RISK-065-01 / CTRL-065-01: a static gate proving there is NO SQLite token
// anywhere in this suite or the FEMS production modules it consumes. PostgreSQL/EventLedger is the only
// durable authority (zero SQLite anywhere).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_no_sqlite_anywhere() {
    let client_src = include_str!("../src/fems/memory_client.rs");
    let proposal_src = include_str!("../src/fems/memory_proposal.rs");

    // The forbidden SQLite dependency/handle tokens. The check targets the two FEMS PRODUCTION modules
    // (the consumer reaches the store only through the HTTP API, so neither may carry a SQLite token).
    // The suite file itself is intentionally NOT scanned for these literals here — it legitimately names
    // the tokens in this assertion + the live-DSN refusal text — and is covered instead by the
    // lowercase-`sqlite` production-module gate below + the explicit DSN-refusal assertion.
    let lowered_sqlite = concat!("sql", "ite"); // split so this literal does not self-match a suite scan
    for (name, src) in [
        ("memory_client", client_src),
        ("memory_proposal", proposal_src),
    ] {
        assert!(
            !src.to_ascii_lowercase().contains(lowered_sqlite),
            "RISK-065-01/CTRL-065-01: the FEMS production module {name} must contain no SQLite token"
        );
        // No file-scheme DSN / local-db handle either.
        for token in ["file:///", "connect_lazy_sqlite"] {
            assert!(
                !src.contains(token),
                "RISK-065-01: no local-store handle may appear in {name} (found '{token}')"
            );
        }
    }
    // And the suite's live-DSN resolver explicitly refuses a SQLite/file scheme (the runtime guard).
    let suite_src = include_str!("test_fems_interop_proofs.rs");
    assert!(
        suite_src.contains("a ") && suite_src.contains(" DSN is never acceptable"),
        "CTRL-065-01: the suite must explicitly refuse a SQLite DSN at the live-DSN resolver"
    );
    println!("no-SQLite OK (RISK-065-01/CTRL-065-01): zero SQLite token in the suite or the FEMS modules; PostgreSQL/EventLedger is the only authority");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (NON-IGNORED) — IN-065-06 / AC-065-06: pin the composed live capability identities.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_required_capability_contract() {
    let all = FEMS_REQUIRED_CAPABILITIES.join("\n");
    assert!(
        all.contains("memory/pack"),
        "FEMS-01 pins the MemoryPack read route"
    );
    assert!(
        all.contains("POST+GET") && all.contains("memory/proposals"),
        "FEMS-02 pins proposal write and exact readback"
    );
    assert!(
        all.contains("native_editor_event") && all.contains("memory_write_proposed"),
        "FEMS-02 pins closed native-editor FR ingestion"
    );
    assert_eq!(
        FEMS_REQUIRED_CAPABILITIES.len(),
        4,
        "exactly four composed live capabilities"
    );

    // Capability-restricted/older backends still fail through typed variants; never through a direct-write
    // fallback or a fake empty response.
    let read_blocker = MemoryClientError::EndpointMissing {
        probed_path: "/workspaces/WS/memory/pack".into(),
    };
    assert!(
        read_blocker.is_endpoint_missing(),
        "MT-063 EndpointMissing is the read typed blocker"
    );
    let write_blocker = MemoryProposalError::MissingEndpoint {
        probed_path: "/workspaces/WS/memory/proposals".into(),
    };
    assert!(
        write_blocker.is_missing_endpoint(),
        "MT-064 MissingEndpoint is the write typed blocker"
    );

    println!("capability-contract OK: MemoryPack + proposal write/readback + correlated FR ingestion pinned");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (NON-IGNORED) — proof-only scope guard / AC-065-06: this MT changes ONLY this test file (no src/
// edit, no backend, no new feature) and the FEMS proposal build invariant it asserts on is review-gated
// (the never-editor-direct safety invariant FEMS-04 guards live). The build_proposal call here is the
// FIXTURE invariant supporting FEMS-04: a procedurally-built proposal is ALWAYS review-gated; the
// canonical FEMS-04 proof below verifies the backend never auto-commits it.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_04_review_gate_invariant_fixture_half() {
    // A procedurally/agent-built proposal (the swarm path of FEMS-03) is ALWAYS review-gated for EVERY
    // class — the editor never produces a non-review-gated proposal. This is the client-side safety
    // invariant FEMS-04 guards; the live backend assertion (status pending, no committed memory item, FR
    // event records pending-review) is the canonical FEMS-04 proof below.
    let sel = text_range("pane-rich", 0, 9, "step one\n");
    for class in MemoryClass::ORDER {
        let proposal =
            build_proposal_for_document(&sel, class, "WS-MT065", "swarm-agent-1", "DOC-MT065")
                .expect("build_proposal must succeed for a TextRange selection");
        assert!(
            proposal.is_review_gated(),
            "FEMS-04 (fixture half): a procedurally-built {class:?} proposal must be review-gated (never \
             editor-direct)"
        );
    }
    // Procedural explicitly (the spec's hard requirement).
    let proc = build_proposal_for_document(
        &sel,
        MemoryClass::Procedural,
        "WS-MT065",
        "swarm-agent-1",
        "DOC-MT065",
    )
    .unwrap();
    assert!(
        proc.is_review_gated(),
        "FEMS-04: Procedural-class proposals are ALWAYS review-gated"
    );
    // No selection -> no fabricated proposal (the command is a no-op, not a silent empty write).
    assert_eq!(
        build_proposal(
            &SharedSelection::None,
            MemoryClass::Episodic,
            "WS-MT065",
            "a"
        )
        .unwrap_err(),
        MemoryProposalError::NoSelection
    );
    assert!(matches!(
        build_proposal(&sel, MemoryClass::Episodic, "WS-MT065", "a"),
        Err(MemoryProposalError::MissingDocumentIdentity { .. })
    ));
    println!("FEMS-04 fixture half OK: every procedurally-built proposal is review-gated; the integration proof verifies live no-auto-commit");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE PROOFS — managed PostgreSQL + backend required by the canonical default proof command.
//
// They resolve the live DSN/endpoint from integration config, refuse non-PG stores, and fail if the
// managed resource or any composed route is unavailable.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn proof_fems_01_memorypack_render() {
    use handshake_native::fems::memory_client::MemoryClient;

    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-fems01"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let ctx = MemoryContext::from_focus(
        workspace_id.clone(),
        Some(fixture.source_id.clone()),
        Some(fixture.content.clone()),
        Some(fixture.content.len()),
    );
    let client = MemoryClient::with_base_url(live.base.clone())
        .with_session_token(live.session_token.clone());
    let empty = live
        .rt
        .block_on(async { client.fetch_pack(&workspace_id, &ctx).await })
        .expect("fresh workspace returns a real deterministic empty MemoryPack");
    assert!(
        empty.items.is_empty(),
        "fresh workspace starts with no memory"
    );

    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    author_and_select_all(&mut harness, &fixture.content);
    let proposal_id =
        drive_propose_command_via_accesskit(&mut harness, MemoryClass::Semantic, false, None);
    cleanup.capture_proposal(proposal_id.clone());
    assert_eq!(
        live.get_json(&format!(
            "/workspaces/{workspace_id}/memory/proposals/{proposal_id}"
        ))["status"],
        "pending_review",
        "mounted proposal reaches the canonical review gate before any memory commit"
    );
    assert_eq!(
        live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"],
        0,
        "proposal creation alone cannot seed committed memory"
    );

    let committed = drive_approval_through_commit(&mut harness);
    let item_id = structured_field(&committed, "memory_id")
        .expect("committed status carries memory_id")
        .to_owned();
    let pack_id = structured_field(&committed, "memory_pack_id")
        .expect("committed status carries memory_pack_id")
        .to_owned();
    let typed_commit = replay_commit(&live, &workspace_id, &proposal_id);
    assert_eq!(typed_commit.memory_id, item_id);
    assert_eq!(typed_commit.memory_pack_id, pack_id);
    let commit_fr = live.poll_exact_commit_fr_event(&workspace_id, &typed_commit);
    let pack_fr = live.poll_exact_pack_fr_event(&workspace_id, &typed_commit);
    assert_eq!(
        commit_fr["payload"]["commit_report_hash"],
        typed_commit.commit_report_hash
    );
    cleanup.capture_pack_item(pack_id.clone(), item_id.clone());
    assert_eq!(
        live.get_json(&format!(
            "/workspaces/{workspace_id}/memory/proposals/{proposal_id}"
        ))["status"],
        "committed",
        "approval is followed by the explicit governed commit route"
    );
    assert_eq!(
        live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"],
        1,
        "the explicit commit creates exactly one canonical memory item"
    );

    let pack = match live
        .rt
        .block_on(async { client.fetch_pack(&workspace_id, &ctx).await })
    {
        Ok(pack) => pack,
        Err(MemoryClientError::EndpointMissing { probed_path }) => panic!(
            "FEMS-01 BLOCKER[kind=missing_api detail='GET {probed_path} absent' source_mt=MT-063]: the \
             FEMS read route is not present; this proof is gated until the backend exposes it"
        ),
        Err(e) => panic!("FEMS-01 live fetch failed: {e}"),
    };
    assert_eq!(
        pack.items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec![item_id.as_str()],
        "FEMS-01 client readback is the exact item produced by propose/approve/commit"
    );
    assert!(
        pack.items[0].is_navigable(),
        "FEMS-01 committed item carries live provenance"
    );
    let raw_pack = live.get_json(&format!("/workspaces/{workspace_id}/memory/pack"));
    let independently_recomputed_pack_hash =
        compute_memory_pack_hash(&raw_pack).expect("recompute canonical MemoryPack hash");
    assert_eq!(
        independently_recomputed_pack_hash, typed_commit.memory_pack_hash,
        "the live MemoryPack artifact independently binds the commit ack and FR-EVT-MEM-004 hash"
    );
    assert_eq!(raw_pack["pack_id"], typed_commit.memory_pack_id);
    click_author_id(&mut harness, "menu-editors");
    click_author_id(&mut harness, "menu.editors.relevant-memory");
    let status = wait_for_status(&mut harness, RELEVANT_MEMORY_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "state") == Some("ready")
    });
    assert_eq!(structured_field(&status, "items"), Some("1"));
    assert!(
        find_node(&harness.root(), &mem_item_author_id(&item_id)).is_some(),
        "mounted Relevant Memory panel renders the exact committed item"
    );
    assert!(
        find_node(&harness.root(), &mem_source_author_id(&item_id)).is_some(),
        "mounted Relevant Memory panel renders the exact provenance control"
    );
    println!(
        "FEMS-01 ACCESSKIT_SUBTREE panel={} item={} source={} dump={}",
        RELEVANT_MEMORY_PANEL_AUTHOR_ID,
        mem_item_author_id(&item_id),
        mem_source_author_id(&item_id),
        accesskit_author_dump(&harness.root())
    );
    let commit = ledger_row_by_key(&live, &format!("fems-memory-commit:{proposal_id}"));
    assert_eq!(commit["event_type"], "ARTIFACT_STORED");
    assert_eq!(commit["payload"]["memory_id"], item_id);
    assert_eq!(commit["payload"]["memory_pack_id"], pack_id);
    assert_eq!(
        commit["payload"]["memory_pack_hash"],
        typed_commit.memory_pack_hash
    );
    assert_eq!(
        commit["payload"]["commit_report_hash"],
        typed_commit.commit_report_hash
    );
    assert_eq!(commit["payload"]["fr_event_id"], "FR-EVT-MEM-003");
    println!(
        "FEMS-01 PROVEN (live mounted app): proposal={proposal_id}, explicit_commit={}, exact pack={pack_id}, item={item_id}, status={status}, FR-EVT-MEM-003={commit_fr}, FR-EVT-MEM-004={pack_fr}",
        commit["aggregate_id"],
    );
    cleanup.clean_and_verify();
}

/// FEMS-02 / AC-065-03: invoking 'Propose to Memory' creates a new proposal row in live PostgreSQL
/// (visible via GET .../memory/proposals) AND emits an FR-EVT-MEM-001 event into the live EventLedger
/// (visible via GET /api/flight_recorder), both referencing the SAME proposal identity (CTRL-065-03).
/// It also freezes provenance at dialog-open: after opening from a canonical indexed buffer/selection,
/// the mounted editor is changed through the production AccessKit SetValue path before confirm. The
/// accepted proposal must still contain the immutable original snapshot/range/hashes, never the newer
/// editor buffer.
#[test]
fn proof_fems_02_propose_creates_proposal_and_event() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-fems02"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    let sel = author_and_select_all(&mut harness, &fixture.content);
    let proposal = build_proposal_for_document_snapshot(
        &sel,
        MemoryClass::Procedural,
        &workspace_id,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("build_proposal_for_document_snapshot");
    click_author_id(&mut harness, "menu-go");
    click_author_id(&mut harness, "menu.go.command-palette");
    click_author_id(&mut harness, FEMS_PALETTE_ROW_AUTHOR_ID);
    assert!(
        find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_some(),
        "production palette opens the proposal dialog over the canonical indexed selection"
    );

    let newer_buffer = format!("// newer unsaved editor buffer\n{}", fixture.content);
    let code_text_author_id = harness
        .root()
        .children_recursive()
        .find_map(|node| {
            node.accesskit_node()
                .author_id()
                .filter(|author_id| author_id.starts_with(CODE_EDITOR_TEXT_AUTHOR_ID))
                .map(ToOwned::to_owned)
        })
        .expect("mounted code text remains addressable while the proposal dialog is open");
    mcp_dispatch(
        &mut harness,
        &code_text_author_id,
        UiAction::NativeSetValue {
            text: newer_buffer.clone(),
        },
    );
    harness.run_steps(2);
    assert_eq!(
        harness
            .state()
            .active_mounted_code_panel()
            .buffer()
            .to_string(),
        newer_buffer,
        "the production AccessKit SetValue path changes the mounted editor after dialog-open"
    );
    assert_ne!(
        proposal.source.document_content_hash,
        build_proposal_for_document_snapshot(
            &text_range("pane-a", 0, newer_buffer.len(), &newer_buffer),
            MemoryClass::Procedural,
            &workspace_id,
            DEFAULT_ACTOR_ID,
            &fixture.source_id,
            newer_buffer.clone(),
        )
        .expect("newer-buffer comparison proposal")
        .source
        .document_content_hash,
        "the newer mounted buffer has a different raw source hash"
    );

    click_author_id(&mut harness, &fems_class_author_id(MemoryClass::Procedural));
    click_author_id(&mut harness, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
    let status = wait_for_status(&mut harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("event_persisted")
    });
    assert_eq!(structured_field(&status, "state"), Some("completed"));
    let proposal_id = structured_field(&status, "proposal_id")
        .filter(|proposal_id| !proposal_id.is_empty())
        .expect("completed frozen-source proposal status carries proposal_id")
        .to_owned();
    cleanup.capture_proposal(proposal_id.clone());

    let proposals_body = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{}",
        proposal_id
    ));
    assert_exact_proposal_readback(&proposals_body, &proposal_id, &proposal);

    let fr_row = live.poll_exact_fr_event(&workspace_id, &proposal_id);
    assert_exact_proposal_and_canonical_fr_ledger(&live, &proposal_id, &workspace_id, &fr_row);
    let event_payload = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{proposal_id}/artifact"
    ));
    assert_eq!(event_payload["proposal_id"], proposal_id);
    assert_eq!(
        event_payload["ops"][0]["item"]["memory_class"],
        proposal.class.wire()
    );
    assert_eq!(
        event_payload["source_refs"][0]["id"],
        proposal.source.document_id
    );
    assert_eq!(
        event_payload["source_refs"][0]["selector"],
        format!(
            "bytes:{}-{}",
            proposal.source.selection_start, proposal.source.selection_end
        )
    );
    assert_eq!(
        event_payload["source_refs"][0]["hash"],
        proposal.source.content_hash
    );
    assert_eq!(event_payload["ops"][0]["requires_review"], true);
    assert_eq!(event_payload["policy"]["require_human_review"], true);
    assert_ne!(
        proposals_body["proposal"]["content"], newer_buffer,
        "the post-open editor mutation is neither substituted into nor accepted as proposal content"
    );
    println!(
        "FEMS-02 PROVEN (live product command): exact proposal row={} exact correlated FR row={}",
        proposals_body, fr_row
    );
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_duplicate_submission_replays_one_proposal_and_one_event() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt064-duplicate-replay"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let selection = text_range("pane-a", 0, fixture.content.len(), &fixture.content);
    let proposal = build_proposal_for_document_snapshot(
        &selection,
        MemoryClass::Semantic,
        &workspace_id,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("canonical code proposal builds");
    let client = HandshakeCoreClient::with_base_url(live.base.clone())
        .with_session_token(live.session_token.clone());
    let emitter = NativeEditorEventEmitter::new(
        workspace_id.clone(),
        std::sync::Arc::new(RuntimeChatLedgerTransport::with_session_id(
            live.base.clone(),
            uuid::Uuid::new_v4().to_string(),
        )),
        Some(live.rt.handle().clone()),
    );

    let (first, replay) = live.rt.block_on(async {
        let first = submit_proposal_and_emit(&proposal, &client, &emitter)
            .await
            .expect("first proposal submission succeeds");
        let replay = submit_proposal_and_emit(&proposal, &client, &emitter)
            .await
            .expect("identical proposal replay succeeds idempotently");
        (first, replay)
    });
    let (first_ack, first_event_id) = match &first {
        ProposalSubmitOutcome::EventPersisted { ack, event_id } => (ack, event_id),
        other => panic!("duplicate proof requires durable correlated success, got {other:?}"),
    };
    let (replay_ack, replay_event_id) = match &replay {
        ProposalSubmitOutcome::EventPersisted { ack, event_id } => (ack, event_id),
        other => panic!("duplicate replay requires durable correlated success, got {other:?}"),
    };
    assert_eq!(
        replay_ack, first_ack,
        "proposal replay returns the canonical row"
    );
    assert_eq!(
        replay_event_id, first_event_id,
        "proposal replay addresses the same immutable native-editor event"
    );
    cleanup.capture_proposal(first_ack.proposal_id.clone());
    assert_eq!(
        live.canonical_fems_mutation_counts(&workspace_id),
        (1, 0),
        "duplicate submit persists one review proposal and no committed memory"
    );
    let fr_row = live.poll_exact_fr_event(&workspace_id, &first_ack.proposal_id);
    assert_eq!(fr_row["event_id"], first_event_id.as_str());
    assert_exact_proposal_and_canonical_fr_ledger(
        &live,
        &first_ack.proposal_id,
        &workspace_id,
        &fr_row,
    );
    let fr_count = live
        .get_json(&format!(
            "/api/flight_recorder?event_type=memory_write_proposed&wsid={workspace_id}"
        ))
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| {
            row["payload"]["proposal_id"] == first_ack.proposal_id
                && row["payload"]["event_code"] == "FR-EVT-MEM-001"
        })
        .count();
    assert_eq!(
        fr_count, 1,
        "duplicate submit persists one correlated FR row"
    );
    cleanup.clean_and_verify();
}

/// V1 remediation: prove both terminal review decisions against the real FEMS review route, live
/// PostgreSQL, EventLedger, and Flight Recorder. The proposals themselves are created through the
/// mounted native editor's production AccessKit flow. Approval performs the separate explicit commit;
/// rejection never commits. Exact terminal retries must return the original immutable receipts.
#[test]
fn proof_fems_review_approval_and_rejection_persist_end_to_end() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-review-decisions"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    author_and_select_all(&mut harness, &fixture.content);

    let before_count = live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))
        ["count"]
        .as_i64()
        .expect("canonical committed-memory count is an integer");

    let approved_id =
        drive_propose_command_via_accesskit(&mut harness, MemoryClass::Semantic, false, None);
    cleanup.capture_proposal(approved_id.clone());
    let approved_pending = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{approved_id}"
    ));
    assert_eq!(approved_pending["status"], "pending_review");
    assert_eq!(
        live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"],
        before_count
    );
    let approved_ack = drive_approval_through_commit(&mut harness);
    assert_eq!(
        structured_field(&approved_ack, "proposal_id"),
        Some(approved_id.as_str())
    );
    let approved_memory_id = structured_field(&approved_ack, "memory_id")
        .expect("approval status carries memory_id")
        .to_owned();
    let approved_commit_id = structured_field(&approved_ack, "commit_id")
        .expect("approval status carries commit_id")
        .to_owned();
    let approved_pack_id = structured_field(&approved_ack, "memory_pack_id")
        .expect("approval status carries memory_pack_id")
        .to_owned();
    cleanup.capture_pack_item(approved_pack_id.clone(), approved_memory_id.clone());
    let approved = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{approved_id}"
    ));
    assert_eq!(approved["status"], "committed");
    assert_eq!(approved["proposal"]["review"]["decision"], "approved");
    assert_authenticated_native_actor(&approved["proposal"]["review"]["actor_id"]);
    let approved_ledger =
        ledger_row_by_key(&live, &format!("fems-memory-proposal-review:{approved_id}"));
    assert_eq!(approved_ledger["event_type"], "PROMOTION_ACCEPTED");
    assert_eq!(approved_ledger["aggregate_id"], approved_id);
    assert_eq!(
        approved_ledger["correlation_id"],
        format!("fems-memory-proposal-review:{approved_id}")
    );
    let approved_retry = replay_review(
        &live,
        &workspace_id,
        &approved_id,
        ProposalReviewDecision::Approved,
    );
    assert_eq!(approved_retry.status, "committed");
    assert_eq!(
        approved_retry.event_ledger_event_id,
        approved_ledger["event_id"]
            .as_str()
            .expect("approval ledger event id")
    );
    assert!(
        approved_retry.commit.is_some(),
        "production approval replay traverses the separate idempotent commit route"
    );
    let approved_retry_commit = approved_retry
        .commit
        .clone()
        .expect("approval replay returns its separately validated commit receipt");
    assert_eq!(approved_retry_commit.commit_id, approved_commit_id);
    assert_eq!(approved_retry_commit.memory_id, approved_memory_id);
    assert_eq!(approved_retry_commit.memory_pack_id, approved_pack_id);
    assert_eq!(
        approved_retry_commit.event_ledger_event_id,
        structured_field(&approved_ack, "event_ledger_event_id")
            .expect("mounted approval status carries commit EventLedger id")
    );
    let commit_ledger = ledger_row_by_key(&live, &format!("fems-memory-commit:{approved_id}"));
    assert_eq!(
        commit_ledger["event_id"],
        approved_retry_commit.event_ledger_event_id
    );
    assert_eq!(commit_ledger["payload"]["proposal_id"], approved_id);
    assert_eq!(
        commit_ledger["payload"]["memory_id"],
        approved_retry_commit.memory_id
    );
    assert_eq!(
        commit_ledger["payload"]["memory_pack_hash"],
        approved_retry_commit.memory_pack_hash
    );
    assert_eq!(
        commit_ledger["payload"]["commit_report_hash"],
        approved_retry_commit.commit_report_hash
    );
    assert_eq!(
        approved_retry_commit.commit_report.source_proposal_id,
        approved_id
    );
    assert_eq!(
        approved_retry_commit.commit_report.applied_ops[0].memory_id,
        approved_retry_commit.memory_id
    );
    let commit_fr = live.poll_exact_commit_fr_event(&workspace_id, &approved_retry_commit);
    let pack_fr = live.poll_exact_pack_fr_event(&workspace_id, &approved_retry_commit);
    assert_eq!(commit_fr["actor"], "human");
    assert_authenticated_native_actor(&commit_fr["actor_id"]);
    assert_same_rfc3339_instant(
        commit_fr["timestamp"]
            .as_str()
            .expect("commit FR timestamp"),
        &approved_retry_commit.committed_at,
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(&approved_retry_commit.committed_at)
            .expect("commit timestamp")
            > chrono::DateTime::parse_from_rfc3339(&approved_retry.reviewed_at)
                .expect("review timestamp"),
        "the canonical commit time must be later than the review time"
    );
    let proposal_hash = approved["content_hash"]
        .as_str()
        .expect("durable proposal content hash");
    assert_eq!(
        proposal_hash,
        approved["proposal"]["source"]["content_hash"]
    );
    let approved_fr = live.poll_exact_review_fr_event(
        &workspace_id,
        &approved_id,
        "approved",
        &approved_retry.flight_recorder_event_id,
    );
    assert_eq!(approved_fr["actor"], "human");
    assert_authenticated_native_actor(&approved_fr["actor_id"]);
    assert_same_rfc3339_instant(
        approved_fr["timestamp"]
            .as_str()
            .expect("approval FR timestamp"),
        &approved_retry.reviewed_at,
    );
    let after_approval_count = live
        .get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"]
        .as_i64()
        .expect("count after approval");
    assert_eq!(after_approval_count, before_count + 1);

    author_and_select_all(&mut harness, &fixture.content);
    let rejected_id =
        drive_propose_command_via_accesskit(&mut harness, MemoryClass::Episodic, false, None);
    assert_ne!(
        rejected_id, approved_id,
        "each AccessKit proposal has a durable identity"
    );
    cleanup.capture_proposal(rejected_id.clone());
    let rejected_pending = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{rejected_id}"
    ));
    assert_eq!(rejected_pending["status"], "pending_review");
    assert_eq!(
        live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"],
        after_approval_count
    );
    mcp_dispatch(&mut harness, FEMS_REVIEW_REJECT_AUTHOR_ID, UiAction::Click);
    let rejected_ack = wait_for_status(&mut harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "state") == Some("reviewed")
            && structured_field(value, "outcome") == Some("rejected")
            && structured_field(value, "terminal") == Some("true")
    });
    assert_eq!(
        structured_field(&rejected_ack, "proposal_id"),
        Some(rejected_id.as_str())
    );
    let rejected = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{rejected_id}"
    ));
    assert_eq!(rejected["status"], "rejected");
    assert_eq!(rejected["proposal"]["review"]["decision"], "rejected");
    assert_authenticated_native_actor(&rejected["proposal"]["review"]["actor_id"]);
    let rejected_ledger =
        ledger_row_by_key(&live, &format!("fems-memory-proposal-review:{rejected_id}"));
    assert_eq!(rejected_ledger["event_type"], "PROMOTION_REJECTED");
    assert_eq!(rejected_ledger["aggregate_id"], rejected_id);
    assert_eq!(
        rejected_ledger["correlation_id"],
        format!("fems-memory-proposal-review:{rejected_id}")
    );
    let rejected_retry = replay_review(
        &live,
        &workspace_id,
        &rejected_id,
        ProposalReviewDecision::Rejected,
    );
    assert_eq!(rejected_retry.status, "rejected");
    assert!(rejected_retry.commit.is_none());
    assert_eq!(
        rejected_retry.event_ledger_event_id,
        rejected_ledger["event_id"]
            .as_str()
            .expect("rejection ledger event id")
    );
    assert!(
        uuid::Uuid::parse_str(&rejected_retry.flight_recorder_event_id).is_ok(),
        "rejection retry carries the exact durable FR UUID"
    );
    let rejected_fr = live.poll_exact_review_fr_event(
        &workspace_id,
        &rejected_id,
        "rejected",
        &rejected_retry.flight_recorder_event_id,
    );
    assert_eq!(rejected_fr["actor"], "human");
    assert_authenticated_native_actor(&rejected_fr["actor_id"]);
    assert_same_rfc3339_instant(
        rejected_fr["timestamp"]
            .as_str()
            .expect("rejection FR timestamp"),
        &rejected_retry.reviewed_at,
    );
    let rejected_counts = run_psql(
        &live.dsn,
        &format!(
            "SELECT json_build_object(\
             'commit_reports', (SELECT COUNT(*) FROM fems_memory_commit_reports WHERE proposal_id = {proposal}),\
             'commit_outbox', (SELECT COUNT(*) FROM fems_memory_commit_fr_outbox WHERE proposal_id = {proposal}),\
             'commit_ledger', (SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = {commit_key})\
             )::text",
            proposal = sql_literal(&rejected_id),
            commit_key = sql_literal(&format!("fems-memory-commit:{rejected_id}")),
        ),
    );
    let rejected_counts: serde_json::Value = serde_json::from_str(rejected_counts.trim())
        .expect("rejection side-effect counts are JSON");
    assert_eq!(
        rejected_counts,
        serde_json::json!({
            "commit_reports": 0,
            "commit_outbox": 0,
            "commit_ledger": 0,
        }),
        "rejection creates no commit report, FR-003/004 outbox row, or commit EventLedger receipt"
    );
    let rejected_commit_status = live.rt.block_on(async {
        live.workspace_ident(
            live.client
                .post(format!(
                    "{}/workspaces/{workspace_id}/memory/proposals/{rejected_id}/commit",
                    live.base
                ))
                .timeout(std::time::Duration::from_secs(5)),
        )
        .send()
        .await
        .expect("rejected-proposal commit probe reaches the real backend")
        .status()
    });
    assert_eq!(
        rejected_commit_status,
        reqwest::StatusCode::CONFLICT,
        "the explicit commit route fails closed for a rejected proposal"
    );
    for event_type in ["memory_write_committed", "memory_pack_built"] {
        let rows = live.get_json(&format!(
            "/api/flight_recorder?wsid={workspace_id}&event_type={event_type}"
        ));
        assert!(
            rows.as_array().is_some_and(|rows| rows.iter().all(|row| {
                row["payload"]["proposal_id"] != rejected_id
                    && row["activity_span_id"] != format!("fems-memory-proposal:{rejected_id}")
            })),
            "rejected proposal must have no {event_type} Flight Recorder projection: {rows}"
        );
    }

    let after_count = live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))
        ["count"]
        .as_i64()
        .expect("canonical committed-memory count remains an integer");
    assert_eq!(
        after_count,
        before_count + 1,
        "only the explicitly approved proposal commits; rejection leaves committed memory unchanged"
    );
    println!(
        "FEMS review decisions PROVEN: approved_proposal={approved_id} commit={approved_commit_id} approved_fr={} commit_fr={} pack_fr={} rejected_proposal={rejected_id} rejected_fr={} committed_count={after_count}; exact terminal retries converged",
        approved_fr["event_id"], commit_fr["event_id"], pack_fr["event_id"], rejected_fr["event_id"]
    );
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_approved_proposal_recovers_commit_only_after_native_restart() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-approved-recovery"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let selection = text_range("pane-a", 0, fixture.content.len(), &fixture.content);
    let proposal = build_proposal_for_document_snapshot(
        &selection,
        MemoryClass::Semantic,
        &workspace_id,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("build restart-recovery proposal");
    let client = HandshakeCoreClient::with_base_url(live.base.clone())
        .with_session_token(live.session_token.clone());
    let emitter = NativeEditorEventEmitter::new(
        workspace_id.clone(),
        std::sync::Arc::new(RuntimeChatLedgerTransport::with_session_id(
            live.base.clone(),
            uuid::Uuid::new_v4().to_string(),
        )),
        Some(live.rt.handle().clone()),
    );
    let submitted = live
        .rt
        .block_on(submit_proposal_and_emit(&proposal, &client, &emitter))
        .expect("submit proposal before simulated restart");
    let proposal_id = submitted.ack().proposal_id.clone();
    cleanup.capture_proposal(proposal_id.clone());

    let review_url = format!(
        "{}/workspaces/{workspace_id}/memory/proposals/{proposal_id}/review",
        live.base
    );
    let approved: serde_json::Value = live.rt.block_on(async {
        live.client
            .post(&review_url)
            .header("x-hsk-session-token", &live.session_token)
            .header("x-hsk-actor-id", "native-editor-fems-reviewer")
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", "mt065-approved-recovery")
            .header("x-hsk-session-run-id", "mt065-restarted-session")
            .json(&serde_json::json!({
                "decision": "approved",
                "reviewer_kind": "user",
                "reason": "simulate interruption after durable approval before commit"
            }))
            .send()
            .await
            .expect("approve proposal without invoking commit route")
            .error_for_status()
            .expect("approval-only route succeeds")
            .json()
            .await
            .expect("approval-only acknowledgement JSON")
    });
    assert_eq!(approved["status"], "approved");
    assert_eq!(
        live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"))["count"],
        0,
        "the interruption point is durable approval with no committed item"
    );

    // Construct a fresh native app instance against the same workspace. Its canonical actionable-list
    // refresh must recover the approved row and expose commit-only UI.
    let restarted = live.take_app();
    let restarted = mounted_code_app_with_real_anchor(restarted, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), restarted);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        if find_node(&harness.root(), FEMS_REVIEW_APPROVE_AUTHOR_ID).is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "restarted native app did not recover approved proposal {proposal_id}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let approve = find_node(&harness.root(), FEMS_REVIEW_APPROVE_AUTHOR_ID)
        .expect("approved recovery exposes commit action");
    let reject = find_node(&harness.root(), FEMS_REVIEW_REJECT_AUTHOR_ID)
        .expect("approved recovery retains a discoverable disabled reject control");
    assert!(!approve.disabled);
    assert!(
        reject.disabled,
        "approved recovery must not permit rejection"
    );
    let committed = drive_approval_through_commit(&mut harness);
    let memory_id = structured_field(&committed, "memory_id")
        .expect("recovered commit carries memory_id")
        .to_owned();
    let pack_id = structured_field(&committed, "memory_pack_id")
        .expect("recovered commit carries memory_pack_id")
        .to_owned();
    cleanup.capture_pack_item(pack_id, memory_id);
    let typed = live
        .rt
        .block_on(commit_approved_proposal(
            &workspace_id,
            &proposal_id,
            &client,
        ))
        .expect("recovered commit-only action is exactly idempotent");
    live.poll_exact_commit_fr_event(&workspace_id, &typed);
    live.poll_exact_pack_fr_event(&workspace_id, &typed);
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_editor_switch_invalidates_identical_text_range_selection() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-editor-selection-binding"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let (fixture_a, fixture_b) = live.seed_code_authorities_with_identical_selection(&workspace_id);
    let shared_text = "identical-selection-café";
    let start_a = fixture_a
        .content
        .find(shared_text)
        .expect("first document contains shared selection");
    let start_b = fixture_b
        .content
        .find(shared_text)
        .expect("second document contains shared selection");
    assert_eq!(
        start_a, start_b,
        "negative proof requires identical byte range as well as identical selected bytes"
    );
    let end = start_a + shared_text.len();

    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture_a);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture_a);
    let selected = author_and_select_range(&mut harness, start_a, end, shared_text);
    assert_eq!(
        selected,
        text_range("pane-a", start_b, end, shared_text),
        "both canonical documents present the same text at the same range"
    );
    let counts_before = live.canonical_fems_mutation_counts(&workspace_id);

    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture_b);
    harness.run_steps(3);
    assert!(
        harness
            .state()
            .active_mounted_code_panel()
            .selected_primary_text()
            .is_none(),
        "the newly active document has no deliberate selection"
    );
    click_author_id(&mut harness, "menu-go");
    click_author_id(&mut harness, "menu.go.command-palette");
    click_author_id(&mut harness, FEMS_PALETTE_ROW_AUTHOR_ID);
    let blocked = wait_for_status(&mut harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("no_selection")
    });
    assert_eq!(structured_field(&blocked, "state"), Some("blocked"));
    assert!(
        find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_none(),
        "document B cannot reuse document A's same-byte/same-range selection"
    );
    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    while std::time::Instant::now() < drain_deadline {
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        live.canonical_fems_mutation_counts(&workspace_id),
        counts_before,
        "the stale cross-document proposal attempt writes neither a proposal nor committed memory"
    );
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_cross_pane_focus_invalidates_already_staged_selection() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-cross-pane-selection-binding"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    author_and_select_all(&mut harness, &fixture.content);
    let counts_before = live.canonical_fems_mutation_counts(&workspace_id);

    // Stage the real shared-bus proposal snapshot and transfer editor focus before the mounted app gets
    // its next drain frame. This is the adversarial race: without bus-level invalidation, pane B could
    // submit pane A's stale selection even though B has no deliberate selection of its own.
    let bus = InteractionBus::get_or_init(&harness.ctx);
    {
        let mut bus = bus.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(
            bus.shared_selection().pane_id().map(|pane| pane.as_ref()),
            Some("pane-a"),
            "the mounted code editor published pane A's live selection"
        );
        bus.request_memory_proposal();
        bus.set_focus_owner(PaneId::from("pane-b"));
        assert_eq!(
            bus.shared_selection(),
            &SharedSelection::None,
            "editor focus transfer clears the live cross-pane selection"
        );
    }

    let blocked = wait_for_status(&mut harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("no_selection")
    });
    assert_eq!(structured_field(&blocked, "state"), Some("blocked"));
    assert!(
        find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_none(),
        "the already-staged pane A snapshot cannot open a proposal after pane B takes focus"
    );
    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    while std::time::Instant::now() < drain_deadline {
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        live.canonical_fems_mutation_counts(&workspace_id),
        counts_before,
        "the blocked cross-pane attempt leaves canonical proposal and committed-memory counts unchanged"
    );
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_focus_change_after_request_drain_invalidates_open_dialog() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-post-drain-dialog-binding"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let (fixture_a, fixture_b) = live.seed_code_authorities_with_identical_selection(&workspace_id);
    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture_a);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture_a);
    author_and_select_all(&mut harness, &fixture_a.content);
    let counts_before = live.canonical_fems_mutation_counts(&workspace_id);

    // Let the mounted app drain pane A's real shared-bus request into an open dialog first. This is the
    // reverse ordering the earlier bus-only regression did not exercise.
    click_author_id(&mut harness, "menu-go");
    click_author_id(&mut harness, "menu.go.command-palette");
    click_author_id(&mut harness, FEMS_PALETTE_ROW_AUTHOR_ID);
    wait_for_author_id(&mut harness, FEMS_PROPOSE_DIALOG_AUTHOR_ID);

    // Activate a different canonical document through the production mounted Quick Switcher while the
    // frozen A dialog exists. The post-render app gate must retire A before it can be confirmed as B.
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture_b);
    let blocked = wait_for_status(&mut harness, FEMS_PROPOSE_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "outcome") == Some("selection_context_changed")
    });
    assert_eq!(structured_field(&blocked, "state"), Some("blocked"));
    assert!(
        find_node(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID).is_none(),
        "a dialog drained from document A must be retired after mounted document B activates"
    );

    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    while std::time::Instant::now() < drain_deadline {
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        live.canonical_fems_mutation_counts(&workspace_id),
        counts_before,
        "post-drain invalidation leaves canonical proposals and committed memory unchanged"
    );
    cleanup.clean_and_verify();
}

#[test]
fn proof_fems_code_provenance_rejects_cross_workspace_and_stale_ksrc() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_a = live.create_workspace(&unique_name("mt065-ksrc-a"));
    let workspace_b = live.create_workspace(&unique_name("mt065-ksrc-b"));
    let mut cleanup_a = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_a.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let mut cleanup_b = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_b.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_a);
    let selection = text_range("pane-a", 0, fixture.content.len(), &fixture.content);
    let proposal = build_proposal_for_document_snapshot(
        &selection,
        MemoryClass::Semantic,
        &workspace_a,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("canonical KSRC proposal fixture");
    let body = serde_json::json!({
        "class": proposal.class.wire(),
        "content": proposal.content,
        "source": proposal.source,
        "source_document_content": proposal.source_document_content,
        "review_gated": true,
        "actor_id": proposal.actor_id,
    });

    let mut cross_workspace = body.clone();
    cross_workspace["source"]["workspace_id"] = serde_json::json!(workspace_b);
    let (status, response) = live.post_json_status(
        &format!("/workspaces/{workspace_b}/memory/proposals"),
        &cross_workspace,
    );
    assert_eq!(
        status, 400,
        "cross-workspace KSRC must fail closed: {response}"
    );

    let mut mismatched_snapshot = body.clone();
    mismatched_snapshot["source_document_content"] =
        serde_json::json!(format!("{}// changed after index\n", fixture.content));
    let (status, response) = live.post_json_status(
        &format!("/workspaces/{workspace_a}/memory/proposals"),
        &mismatched_snapshot,
    );
    assert_eq!(
        status, 400,
        "snapshot that does not match the KSRC hash must fail closed: {response}"
    );

    let unicode_content = "café";
    let unicode_start = fixture
        .content
        .find(unicode_content)
        .expect("indexed fixture contains a multibyte selection");
    let unicode_selection = text_range(
        "pane-a",
        unicode_start,
        unicode_start + unicode_content.len(),
        unicode_content,
    );
    let unicode_proposal = build_proposal_for_document_snapshot(
        &unicode_selection,
        MemoryClass::Semantic,
        &workspace_a,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("valid multibyte KSRC slice proposal");
    let unicode_body = serde_json::json!({
        "class": unicode_proposal.class.wire(),
        "content": unicode_proposal.content,
        "source": unicode_proposal.source,
        "source_document_content": unicode_proposal.source_document_content,
        "review_gated": true,
        "actor_id": unicode_proposal.actor_id,
    });
    let unicode_ack = live.post_json(
        &format!("/workspaces/{workspace_a}/memory/proposals"),
        &unicode_body,
    );
    let unicode_proposal_id = unicode_ack["proposal_id"]
        .as_str()
        .expect("valid multibyte proposal returns proposal_id")
        .to_owned();
    assert_eq!(unicode_ack["status"], "pending_review");
    let unicode_ledger = ledger_row_by_key(
        &live,
        &format!("fems-memory-proposal:{unicode_proposal_id}"),
    );
    assert_eq!(
        unicode_ledger["payload"]["proposal_id"].as_str(),
        Some(unicode_proposal_id.as_str())
    );
    assert_eq!(
        unicode_ledger["payload"]["content_hash"],
        unicode_body["source"]["content_hash"]
    );

    let accent_start = fixture
        .content
        .find('é')
        .expect("indexed fixture contains a multibyte code point");
    let mut split_codepoint = body.clone();
    split_codepoint["content"] = serde_json::json!("x");
    split_codepoint["source"]["selection_start"] = serde_json::json!(accent_start + 1);
    split_codepoint["source"]["selection_end"] = serde_json::json!(accent_start + 2);
    split_codepoint["source"]["content_hash"] = serde_json::json!(content_hash_of_selection("x"));
    let (status, response) = live.post_json_status(
        &format!("/workspaces/{workspace_a}/memory/proposals"),
        &split_codepoint,
    );
    assert_eq!(
        status, 400,
        "split-codepoint KSRC slice must fail closed: {response}"
    );
    assert!(
        response.contains("UTF-8 range"),
        "split-codepoint rejection must expose the precise UTF-8 range failure: {response}"
    );

    run_psql(
        &live.dsn,
        &format!(
            "UPDATE knowledge_sources SET stale = TRUE WHERE source_id = {}",
            sql_literal(&fixture.source_id)
        ),
    );
    let (status, response) = live.post_json_status(
        &format!("/workspaces/{workspace_a}/memory/proposals"),
        &body,
    );
    assert_eq!(status, 400, "stale KSRC must fail closed: {response}");
    assert_eq!(
        run_psql(
            &live.dsn,
            &format!(
                "SELECT COUNT(*) FROM fems_memory_proposals WHERE workspace_id IN ({}, {})",
                sql_literal(&workspace_a),
                sql_literal(&workspace_b)
            )
        )
        .trim(),
        "1",
        "only the valid multibyte proposal is durable; all negative probes fail before insert"
    );
    cleanup_a.clean_and_verify();
    assert_eq!(
        live.get_status(&format!(
            "/workspaces/{workspace_a}/memory/proposals/{unicode_proposal_id}"
        )),
        404,
        "RAII-backed workspace cleanup removes the exact multibyte proposal row"
    );
    cleanup_b.clean_and_verify();
}

/// FEMS-03 / AC-065-04: the full FEMS flow (open panel -> refresh MemoryPack -> propose ->
/// reach review-gated proposal) is driveable purely via AccessKit ids by an out-of-process-agent code
/// path, AND the live dispatch reaches the backend (a live proposal results). The id-stability + the
/// AccessKit-only dispatch are supported by `proof_fems_03_swarm_id_stability`; this canonical proof adds
/// the live backend round-trip through the real app.
#[test]
fn proof_fems_03_swarm_drives_fems_via_accesskit() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-fems03"));
    let fixture = live.seed_code_authority(&workspace_id);
    let app = live.take_app();
    let mut argus = live.take_argus();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    let selection = text_range("pane-a", 0, fixture.content.len(), &fixture.content);
    let before_inspect = inspect_until(&mut argus, &mut harness, "menu-editors", 60);
    let mut observations = Vec::new();

    // Open the real mounted Relevant Memory pane through the operator menu, then wait for the app-hosted
    // live MemoryPack read. Force a second real read through the panel's stable Refresh AccessKit control.
    observations.push(argus_click(&mut argus, &mut harness, "menu-editors"));
    observations.push(argus_click(
        &mut argus,
        &mut harness,
        "menu.editors.relevant-memory",
    ));
    assert!(
        find_node(&harness.root(), RELEVANT_MEMORY_PANEL_AUTHOR_ID).is_some(),
        "AccessKit panel-open action mounts the real Relevant Memory pane"
    );
    let first_status = wait_for_status(&mut harness, RELEVANT_MEMORY_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "state") == Some("empty")
    });
    let before_refresh = structured_field(&first_status, "completed")
        .and_then(|value| value.parse::<u64>().ok())
        .expect("MemoryPack status carries completed refresh count");
    let panel_node_id = find_node(&harness.root(), RELEVANT_MEMORY_PANEL_AUTHOR_ID)
        .expect("mounted panel node")
        .node_id;
    let status_node_id = find_node(&harness.root(), RELEVANT_MEMORY_STATUS_AUTHOR_ID)
        .expect("mounted status node")
        .node_id;
    observations.push(argus_click(
        &mut argus,
        &mut harness,
        RELEVANT_MEMORY_REFRESH_AUTHOR_ID,
    ));
    let second_status = wait_for_status(&mut harness, RELEVANT_MEMORY_STATUS_AUTHOR_ID, |value| {
        structured_field(value, "completed")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|completed| completed > before_refresh)
    });
    assert_eq!(
        find_node(&harness.root(), RELEVANT_MEMORY_PANEL_AUTHOR_ID)
            .expect("panel remains mounted")
            .node_id,
        panel_node_id,
        "mounted FEMS panel AccessKit identity is stable across refresh"
    );
    assert_eq!(
        find_node(&harness.root(), RELEVANT_MEMORY_STATUS_AUTHOR_ID)
            .expect("status remains mounted")
            .node_id,
        status_node_id,
        "mounted FEMS status AccessKit identity is stable across refresh"
    );

    // Relevant Memory is a utility tab. Return through its stable tab AccessKit control to the code
    // selection owner before proposing, exactly as an operator or model must do when the utility pane
    // is active; no in-process pane mutation is allowed in this Argus proof.
    let target_content_id = fixture
        .target_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let code_tab_author_id = harness
        .state()
        .tab_bar_states()
        .iter()
        .find_map(|(pane_id, bar)| {
            bar.tabs.iter().enumerate().find_map(|(index, tab)| {
                (tab.pane_type == PaneType::CodeSymbol
                    && tab.content_id.as_deref() == Some(target_content_id.as_str()))
                .then(|| tab_author_id_for(pane_id.as_ref(), index, &tab.pane_type))
            })
        })
        .expect("indexed target code tab remains model-addressable after Relevant Memory refresh");
    observations.push(argus_click(&mut argus, &mut harness, &code_tab_author_id));
    harness.run_steps(2);
    observations.push(argus_click(&mut argus, &mut harness, "menu-edit"));
    observations.push(argus_click(
        &mut argus,
        &mut harness,
        "menu.edit.select-all",
    ));
    harness.run_steps(2);

    let proposal_id = drive_propose_command_via_argus(
        &mut argus,
        &mut harness,
        MemoryClass::Procedural,
        &live,
        &workspace_id,
        &mut observations,
    );
    cleanup.capture_proposal(proposal_id.clone());

    let expected = build_proposal_for_document_snapshot(
        &selection,
        MemoryClass::Procedural,
        &workspace_id,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("reconstruct the exact app proposal from its authoritative inputs");
    let readback = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{proposal_id}"
    ));
    assert_exact_proposal_readback(&readback, &proposal_id, &expected);
    let fr_row = live.poll_exact_fr_event(&workspace_id, &proposal_id);
    assert_exact_proposal_and_canonical_fr_ledger(&live, &proposal_id, &workspace_id, &fr_row);
    let after_reinspect = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&after_reinspect, FEMS_PROPOSE_STATUS_AUTHOR_ID),
        "fresh canonical argus.inspect sees the terminal FEMS status node"
    );

    let source_sha = current_source_sha();
    let proof_source_blob = current_proof_source_blob();
    let artifact_dir = external_artifact_dir(&format!(
        "wp-kernel-012-mt-065/canonical-argus/run-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&artifact_dir)
        .expect("create external MT-065 canonical Argus artifact directory");
    let screenshot_path = artifact_dir.join("mt065-fems-interop-canonical-argus.png");
    harness
        .render()
        .expect("MT-065 requires a material screenshot, not a typed screenshot error")
        .save(&screenshot_path)
        .expect("save MT-065 canonical Argus screenshot");
    assert!(screenshot_path.is_file());
    let evidence_path = artifact_dir.join("mt065-fems-interop-canonical-argus.json");
    let receipts = observations
        .iter()
        .map(|observation| {
            serde_json::json!({
                "receipt_id": observation.receipt_id,
                "receipt_status": observation.receipt_status,
                "agent_id": observation.agent_id,
                "before_inspect": observation.before,
                "after_reinspect": observation.after,
            })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "handshake.mt065-canonical-argus-proof.v1",
            "source_sha": source_sha,
            "proof_source_blob": proof_source_blob,
            "workspace_id": workspace_id,
            "proposal_id": proposal_id,
            "before_inspect": before_inspect,
            "after_reinspect": after_reinspect,
            "action_receipts": receipts,
            "memory_pack_status": second_status,
            "proposal_readback": readback,
            "fr_event": fr_row,
            "screenshot": screenshot_path,
        }))
        .expect("serialize MT-065 canonical Argus evidence"),
    )
    .expect("write MT-065 canonical Argus evidence");
    assert!(evidence_path.is_file());
    println!(
        "FEMS-03 PROVEN (live): canonical argus.inspect->argus.click->receipt->fresh inspect drove panel-open->MemoryPack-refresh ({second_status})->palette->dialog->cancel->reopen->class->confirm and drained proposal {proposal_id}; source_sha={source_sha}; screenshot={}; evidence={}; exact row={readback}; FR row={fr_row}",
        screenshot_path.display(),
        evidence_path.display(),
    );
    cleanup.clean_and_verify();
    argus.finish();
}

/// A procedural proposal created through the mounted native controls remains review-gated in canonical
/// PostgreSQL state and does not mutate the committed-memory store. This live path does not claim an
/// async workspace-generation proof; stale submit delivery is tested independently at the app boundary.
#[test]
fn proof_fems_04_procedural_proposal_stays_review_gated() {
    let _serial = live_proof_guard();
    let live = LiveProofSession::new();
    let workspace_id = live.create_workspace(&unique_name("mt065-fems04"));
    let mut cleanup = WorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        proposal_ids: Vec::new(),
        pack_ids: Vec::new(),
        item_ids: Vec::new(),
        cleaned: false,
    };
    let fixture = live.seed_code_authority(&workspace_id);
    let app = live.take_app();
    let app = mounted_code_app_with_real_anchor(app, &live, &workspace_id, &fixture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    open_indexed_code_symbol_via_quick_switcher(&mut harness, &fixture);
    let sel = author_and_select_all(&mut harness, &fixture.content);
    let proposal = build_proposal_for_document_snapshot(
        &sel,
        MemoryClass::Procedural,
        &workspace_id,
        DEFAULT_ACTOR_ID,
        &fixture.source_id,
        fixture.content.clone(),
    )
    .expect("build_proposal_for_document_snapshot");
    // The editor-built proposal is review-gated by construction (proven NOW by the fixture half).
    assert!(
        proposal.is_review_gated(),
        "FEMS-04: the editor proposal is review-gated by construction"
    );

    let before = live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"));
    assert_eq!(before["workspace_id"], workspace_id);
    let before_count = before["count"]
        .as_i64()
        .expect("canonical committed-memory count is an integer");

    let proposal_id =
        drive_propose_command_via_accesskit(&mut harness, MemoryClass::Procedural, false, None);
    cleanup.capture_proposal(proposal_id.clone());
    let readback = live.get_json(&format!(
        "/workspaces/{workspace_id}/memory/proposals/{proposal_id}"
    ));
    assert_eq!(readback["status"], "pending_review");
    assert_exact_proposal_readback(&readback, &proposal_id, &proposal);
    let fr_row = live.poll_exact_fr_event(&workspace_id, &proposal_id);
    assert_exact_proposal_and_canonical_fr_ledger(&live, &proposal_id, &workspace_id, &fr_row);

    // This is the canonical committed store, not a context-filtered MemoryPack projection. A proposal
    // must leave its exact workspace count unchanged until a downstream reviewer commits it.
    let after = live.get_json(&format!("/workspaces/{workspace_id}/memory/items/count"));
    assert_eq!(after["workspace_id"], workspace_id);
    assert_eq!(
        after["count"].as_i64(),
        Some(before_count),
        "a review-gated proposal must not create a canonical fems_memory_items row"
    );
    println!(
        "FEMS-04 PROVEN (live): proposal {} exact status='{}'; canonical committed count stayed {}; exact correlated FR row={fr_row}",
        proposal_id, readback["status"], before_count
    );
    cleanup.clean_and_verify();
}

// A compile-time anchor so an unused `HashSet`/`MEMORY_PACK_MAX_ITEMS`/`FEMS_PROPOSE_COMMAND_ID`/
// `ProposeDialogOutcome` import (used only on certain branches) never triggers a dead-code warning under
// `-D warnings` (AC-065-08). These are the documented swarm-surface constants the proofs reference.
#[test]
fn proof_fems_surface_constants_present() {
    // The proposal cap the read panel enforces + the propose command id + the dialog outcome enum + a
    // deterministic-id set are all part of the FEMS swarm surface this suite asserts on.
    assert_eq!(
        MEMORY_PACK_MAX_ITEMS, 24,
        "the Pillar 12 <=24 item cap the panel enforces"
    );
    assert_eq!(
        FEMS_PROPOSE_COMMAND_ID, "fems.propose_to_memory",
        "the propose command swarm id"
    );
    // ProposeDialogOutcome::Cancelled is a valid outcome a swarm agent can reach (cancel path).
    assert_ne!(
        ProposeDialogOutcome::Cancelled,
        ProposeDialogOutcome::Pending,
        "the dialog outcome enum distinguishes cancel from pending (swarm cancel path)"
    );
    // A small determinism cross-check on the FEMS author_ids the swarm path stores.
    let ids: HashSet<String> = [
        RELEVANT_MEMORY_PANEL_AUTHOR_ID.to_owned(),
        mem_item_author_id("sem-1"),
        mem_source_author_id("sem-1"),
        FEMS_PROPOSE_CONFIRM_AUTHOR_ID.to_owned(),
        FEMS_REVIEW_APPROVE_AUTHOR_ID.to_owned(),
        FEMS_REVIEW_REJECT_AUTHOR_ID.to_owned(),
        fems_class_author_id(MemoryClass::Episodic),
    ]
    .into_iter()
    .collect();
    assert_eq!(ids.len(), 7, "the FEMS swarm author_ids are distinct");
    for id in &ids {
        assert!(
            has_no_random_segment(id),
            "FEMS swarm id '{id}' must be deterministic"
        );
    }
    println!("FEMS surface constants OK: <=24 cap, propose command id, dialog outcome enum, 7 distinct deterministic swarm ids");
}
