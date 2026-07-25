//! WP-KERNEL-012 E9 MT-064 remediation (FAIL_V2): CANONICAL Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED "Propose to Memory" dialog, driven over REAL managed PostgreSQL.
//!
//! `validation_v2` remediation step 2 requires: "drive the editor proposal UI through canonical Argus
//! and update canonical UserManual". The isolated `test_memory_proposal.rs` kittest coverage drives the
//! propose dialog WIDGET (AccessKit tree + ScreenshotHarness), and `test_fems_interop_proofs.rs`
//! FEMS-03 drives the palette->dialog->confirm through the real localhost `SwarmMcpServer` transport via
//! `argus.click`. NEITHER exercises the canonical `argus.inspect` JSON-RPC snapshot of the mounted
//! dialog's stable `author_id`s (the inspect -> steer -> re-inspect pattern the sibling surfaces use in
//! `test_folder_tree_argus.rs` / `test_embeds_argus.rs`). This test closes that exact gap:
//!
//!   1. starts REAL managed PostgreSQL + the owned product `handshake_core` backend (pg_proof_support),
//!   2. mounts the production `HandshakeApp` shell pointed at that live backend with a code document +
//!      a live selection, so the "Propose to Memory" affordance is reachable,
//!   3. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` an out-of-process swarm agent uses) to the mounted app,
//!   4. drives the palette -> proposal dialog through canonical `argus.click`,
//!   5. `argus.inspect` proves the mounted dialog's stable `author_id`s are addressable in the live
//!      JSON-RPC snapshot (`fems-propose-dialog`, `fems-class-{episodic|semantic|procedural}`,
//!      `fems-propose-confirm`),
//!   6. `argus.click` the class radio + confirm to SUBMIT a real review-gated proposal,
//!   7. drives the host until the real proposal PERSISTS in live PostgreSQL AND the FR-EVT-MEM-001
//!      (`memory_write_proposed`) event lands in the live Flight Recorder / EventLedger (exact shape:
//!      event_code, proposal_id, proposal_hash, artifact_ref, scope_refs, op_count,
//!      requires_review_count, and NO raw memory content),
//!   8. FRESH `argus.inspect` re-observes the terminal state (the `fems-propose-status` node reports
//!      `outcome=event_persisted` with the durable `proposal_id`), and
//!   9. writes the before/after canonical trees externally + a screenshot marker (headless DEFERRED is
//!      an acceptable typed outcome).
//!
//! Live-resource law: NO SQLite, NO mock, NO in-memory fallback. If the managed backend/PostgreSQL is
//! not configured the shared fixture PANICS (never a silent green). Artifact hygiene (CX-212E): every
//! artifact is written ONLY under the EXTERNAL `Handshake_Artifacts/handshake-test/` root.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, ArgusObservation, CanonicalArgusDriver};

#[path = "pg_proof_support/mod.rs"]
mod pg_proof_support;
use pg_proof_support::{require_live_backend, LiveBackend};

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::fems::memory_proposal::{
    canonical_memory_write_proposal_hash, fems_class_author_id, MemoryClass,
    FEMS_PROPOSE_CONFIRM_AUTHOR_ID, FEMS_PROPOSE_DIALOG_AUTHOR_ID, FEMS_PROPOSE_STATUS_AUTHOR_ID,
};
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::tab_bar::TabState;

// ── top menu / palette author_ids (stable WP-011 shell + MT-031 command surface) ─────────────────────
const MENU_EDIT: &str = "menu-edit";
const MENU_EDIT_SELECT_ALL: &str = "menu.edit.select-all";
const MENU_GO: &str = "menu-go";
const MENU_GO_COMMAND_PALETTE: &str = "menu.go.command-palette";
const FEMS_PALETTE_ROW_AUTHOR_ID: &str = "command-palette.option.hs-fems-palette-propose-to-memory";

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' artifact dir may exist (found {})",
            p.display()
        );
    }
}

/// Isolated platform app-data root shared by this test process AND the owned backend it spawns, so the
/// native MCP session binding (`{LOCALAPPDATA}/handshake/swarm_mcp_binding.json`) the memory routes'
/// `capture_context` validates is discoverable by both. MUST be installed BEFORE the backend is spawned
/// (the child inherits this env). Restores the prior value and removes the root on drop.
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
        let root = std::env::current_dir()
            .expect("resolve current dir for isolated app-data root")
            .join("../../../../Handshake_Artifacts/handshake-test/wp-kernel-012-mt-064/appdata")
            .join(format!("run-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&root).expect("create isolated app-data root");
        let root = std::fs::canonicalize(&root).expect("canonicalize isolated app-data root");
        let previous = std::env::var_os(variable);
        let previous_owned_backend_root = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT");
        std::env::set_var(variable, &root);
        // pg_proof_support treats this explicit root as the force-owned signal. That prevents this
        // current-source proof from attaching to an arbitrary already-running or stale backend.
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
                    "remove isolated MT-064 app-data root {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

/// Parse a `key=value;key=value` structured AccessKit status string.
fn structured_field<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    value.split(';').find_map(|part| {
        part.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

/// The live-tree AccessKit `value` for `author_id`, or `None` if the node is absent / valueless.
fn tree_value(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    root.children_recursive().find_map(|node| {
        let ak = node.accesskit_node();
        (ak.author_id() == Some(author_id))
            .then(|| ak.value())
            .flatten()
    })
}

fn tree_label(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    root.children_recursive().find_map(|node| {
        let ak = node.accesskit_node();
        (ak.author_id() == Some(author_id))
            .then(|| ak.label())
            .flatten()
    })
}

/// True if the live AccessKit tree currently carries a node with `author_id`.
fn tree_has_author_id(root: &egui_kittest::Node<'_>, author_id: &str) -> bool {
    root.children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(author_id))
}

/// The canonical Argus JSON-RPC snapshot value for `author_id`.
fn json_author_value(value: &serde_json::Value, author_id: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                return object
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
            }
            object
                .values()
                .find_map(|value| json_author_value(value, author_id))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| json_author_value(value, author_id)),
        _ => None,
    }
}

/// Mount the production shell pointed at the live backend with a code document + active pane, matching
/// the proven `mounted_code_app` sequence in `test_fems_interop_proofs.rs`. `app_rt` is a multi-thread
/// runtime the caller MUST keep alive: the dialog's off-frame proposal submit spawns onto it.
fn mount_code_app(
    live: &LiveBackend,
    app_rt: &tokio::runtime::Handle,
    workspace_id: &str,
) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(app_rt.clone());
    app.set_backend_base_url_for_test(&live.base, app_rt.clone());
    app.set_active_project_id_for_test(workspace_id.to_owned());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    app
}

/// Load a code document into pane-a under `content_id`, bound to a real provenance source id, so the
/// proposal built from a selection in this pane carries an EXISTING provenance document_id (the backend
/// fail-closes a proposal whose document_id does not exist in the workspace).
fn load_code_document(app: &mut HandshakeApp, content_id: &str, source_id: &str, content: &str) {
    let generation = app.begin_code_document_load_for_test(content_id);
    app.deliver_code_document_load_for_test(
        generation,
        content_id,
        PathBuf::from(format!("{content_id}.rs")),
        0,
        Ok(content.to_owned()),
    );
    app.bind_code_document_source_for_test(content_id, source_id);
}

struct IndexedCodeFixture {
    root: PathBuf,
    source_id: String,
    content: String,
}

impl Drop for IndexedCodeFixture {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            if error.kind() != std::io::ErrorKind::NotFound {
                panic!(
                    "remove MT-064 indexed-code fixture {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

/// Seed a REAL canonical KSRC code authority through the production code-nav indexer. The mounted code
/// proposal includes a full document snapshot, so using a rich-document id would correctly fail closed
/// (`source_document_content` is code-only); this fixture makes the mounted provenance and backend
/// authority the same exact indexed file.
fn seed_code_authority(
    base: &str,
    session_token: &str,
    workspace_id: &str,
    rt: &tokio::runtime::Handle,
) -> IndexedCodeFixture {
    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let symbol_name = format!("mt064_argus_target_{run_id}");
    let content = format!(
        "pub fn {symbol_name}() -> &'static str {{\n    \"canonical-memory-selection\"\n}}\n"
    );
    let root = external_artifact_dir(&format!("wp-kernel-012-mt-064/code-authority/run-{run_id}"));
    std::fs::create_dir_all(&root).expect("create external MT-064 code-authority root");
    let root = std::fs::canonicalize(root).expect("canonicalize MT-064 code-authority root");
    std::fs::write(root.join("target.rs"), &content).expect("write MT-064 canonical source file");

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .expect("build bounded code-authority client");
    let url = format!("{base}/workspaces/{workspace_id}/code-nav/index");
    let body = serde_json::json!({"root_path": root.to_string_lossy()});
    let indexed: serde_json::Value = rt.block_on(async {
        let response = client
            .post(&url)
            .header("x-hsk-session-token", session_token)
            .header("x-hsk-actor-id", "native-editor-fems-index")
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", "mt064-argus-index")
            .header("x-hsk-session-run-id", "mt064-argus-session")
            .json(&body)
            .send()
            .await
            .expect("seed code-nav index send");
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        assert!(
            status.is_success(),
            "seed code-nav index -> {status}: {text}"
        );
        serde_json::from_str(&text).expect("seed code-nav index JSON")
    });
    assert_eq!(
        indexed["files_failed"], 0,
        "canonical MT-064 code authority indexes cleanly: {indexed}"
    );
    let lookup = get_json_session(
        base,
        session_token,
        &format!(
            "/knowledge/code/symbols?workspace_id={workspace_id}&name={symbol_name}&path=target.rs&limit=1"
        ),
        rt,
    );
    let source_id = lookup["matches"][0]["definition"]["source_id"]
        .as_str()
        .filter(|source_id| source_id.starts_with("KSRC-"))
        .unwrap_or_else(|| panic!("indexed symbol lacks canonical KSRC source id: {lookup}"))
        .to_owned();
    IndexedCodeFixture {
        root,
        source_id,
        content,
    }
}

/// Canonical `argus.inspect` in a bounded pump loop until `author_id` is addressable in the JSON-RPC
/// snapshot. Returns that snapshot (used as the `before` for the click-from-snapshot transport call).
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
    }
    let snapshot = argus.inspect(harness);
    assert!(
        json_has_author_id(&snapshot, author_id),
        "canonical argus.inspect could not address '{author_id}' within {max_steps} pumped frames"
    );
    snapshot
}

/// Inspect-until-present, then drive ONE canonical `argus.click` against that exact snapshot and
/// re-inspect — the production inspect -> click -> re-observe transport path.
fn argus_click(
    argus: &mut CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    author_id: &str,
) -> ArgusObservation {
    let before = inspect_until(argus, harness, author_id, 60);
    argus.click_from_snapshot_and_reinspect(harness, author_id, before)
}

/// Establish a live editor selection through canonical Edit -> Select All. The mounted code pane is the
/// shell's explicit active pane, so the command routes to that editor without an unsupported Argus focus
/// action and publishes the selection into the WP-011 SharedSelection substrate.
fn establish_selection(argus: &mut CanonicalArgusDriver, harness: &mut Harness<'_, HandshakeApp>) {
    harness.run_steps(3);
    argus_click(argus, harness, MENU_EDIT);
    harness.run_steps(1);
    argus_click(argus, harness, MENU_EDIT_SELECT_ALL);
    harness.run_steps(2);
}

/// Open the "Propose to Memory" dialog through the command palette (in-process SETUP path). The FEMS
/// review queue does a one-shot background refresh on workspace bind; opening a proposal while that
/// refresh is in flight is intentionally reentry-blocked, so this settles the queue (pumps frames with
/// real time for the async list to complete + drain) then opens, with a bounded retry on reentry-block.
fn open_propose_dialog(argus: &mut CanonicalArgusDriver, harness: &mut Harness<'_, HandshakeApp>) {
    for _attempt in 0..12 {
        // Let the one-shot review-queue refresh complete + drain (empty workspace → fast).
        for _ in 0..15 {
            harness.run_steps(1);
            std::thread::sleep(Duration::from_millis(100));
        }
        argus_click(argus, harness, MENU_GO);
        harness.run_steps(2);
        argus_click(argus, harness, MENU_GO_COMMAND_PALETTE);
        harness.run_steps(2);
        argus_click(argus, harness, FEMS_PALETTE_ROW_AUTHOR_ID);
        for _ in 0..40 {
            harness.run_steps(1);
            if tree_has_author_id(&harness.root(), FEMS_PROPOSE_DIALOG_AUTHOR_ID) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let status = tree_value(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID);
        let reentry_blocked = status
            .as_deref()
            .and_then(|value| structured_field(value, "outcome"))
            == Some("reentry_blocked");
        assert!(
            reentry_blocked,
            "propose dialog did not open and the block was not the transient review-queue reentry; \
             fems-propose-status={status:?}"
        );
    }
    panic!(
        "propose dialog did not open after settling the FEMS review queue; last status={:?}",
        tree_value(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID)
    );
}

#[test]
fn mt064_mounted_propose_dialog_canonical_argus_inspect_submit_reobserve() {
    // (0a) Isolated platform app-data root shared by this test process AND the owned backend. Installed
    // BEFORE the backend is spawned so the child inherits it; the native MCP session binding published
    // below (which the memory routes' capture_context validates) is then discoverable by both processes.
    let _appdata = ScopedLocalAppData::install();

    // (0b) REAL managed PostgreSQL + owned product backend. Panics (never silently skips) if the live
    // backend/PG is not configured — the shared fixture enforces the no-SQLite / no-mock law.
    let live = require_live_backend();
    let workspace_id = live.workspace_id.clone();
    assert!(
        !workspace_id.is_empty(),
        "the managed fixture must expose a live workspace id"
    );

    // The dialog's proposal submit runs off the egui frame thread; it needs a real multi-thread runtime
    // handle (the current-thread HTTP fixture runtime cannot drive a spawned task). Keep it alive for the
    // whole test.
    let app_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build the mounted-app submit runtime");

    let mut app = mount_code_app(&live, app_rt.handle(), &workspace_id);

    // (0c) Bind the canonical Argus localhost server with the mounted app's OWN MCP session token, in the
    // shared app-data root, publishing the native binding (token + this process's live pid). This is what
    // authorizes the mounted app's live memory-route calls (proposal submit + the review-queue refresh):
    // without a matching binding, the backend's capture_context rejects the session and the FEMS review
    // reentry guard blocks the proposal dialog from ever opening. Bound over `&app` BEFORE `build_state`'s
    // first frame runs, so the very first workspace-bind review-queue refresh is already authorized (an
    // unauthorized first refresh 401s and arms the reentry-retry guard permanently).
    let session_token = app.mcp_token();
    let mut argus = CanonicalArgusDriver::bind_in_current_app_data(
        &app,
        "wp-kernel-012-mt-064-propose",
        session_token,
    );

    // (0d) Seed a REAL indexed code file, then load the code pane under that exact canonical source id.
    // The mounted dialog submits its full code snapshot; the backend validates it byte-for-byte against
    // this KSRC/code-file authority before persisting the proposal.
    let code_fixture = seed_code_authority(
        &live.base,
        &app.mcp_token().as_hex(),
        &workspace_id,
        app_rt.handle(),
    );
    let provenance_document_id = code_fixture.source_id.clone();
    let content = code_fixture.content.as_str();
    load_code_document(
        &mut app,
        &provenance_document_id,
        &provenance_document_id,
        content,
    );
    // Make pane-a's active tab the CodeSymbol document: proposal provenance resolves from the active
    // CodeSymbol tab's content_id -> KnowledgeSource binding (the production quick-switcher navigation
    // creates this tab; the synthetic mount must set it explicitly).
    {
        let mut tab = TabState::new(PaneType::CodeSymbol);
        tab.content_id = Some(provenance_document_id.clone());
        if let Some(bar) = app.tab_bar_states_mut().get_mut(&PaneId::from("pane-a")) {
            bar.tabs = vec![tab];
            bar.active_index = 0;
        }
    }

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 860.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    // (1) Establish a live editor selection through the production focus + Edit -> Select All path (the
    // host boundary that publishes the code selection into the WP-011 SharedSelection substrate the FEMS
    // command reads), then open the propose dialog through the command palette. Every action in this
    // setup and the dialog submit below goes through canonical localhost argus.inspect/argus.click.
    establish_selection(&mut argus, &mut harness);
    open_propose_dialog(&mut argus, &mut harness);

    let artifact_dir = external_artifact_dir(&format!(
        "wp-kernel-012-mt-064/canonical-argus/run-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-064 Argus artifact dir");

    // (2) CANONICAL INSPECT: the mounted dialog's stable author_ids are addressable in the live JSON-RPC
    // snapshot — the exact evidence FAIL_V2 said was missing for the proposal UI.
    let episodic = fems_class_author_id(MemoryClass::Episodic);
    let semantic = fems_class_author_id(MemoryClass::Semantic);
    let procedural = fems_class_author_id(MemoryClass::Procedural);
    let before = inspect_until(&mut argus, &mut harness, FEMS_PROPOSE_DIALOG_AUTHOR_ID, 20);
    for author in [
        FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        episodic.as_str(),
        semantic.as_str(),
        procedural.as_str(),
        FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted propose-dialog node '{author}' in the live tree"
        );
    }

    // (4) CANONICAL STEER on the mounted dialog over the real localhost transport: select the Procedural
    // class (always review-gated) via argus.click, and observe the receipt + caller attribution.
    let class_steer = argus_click(&mut argus, &mut harness, &procedural);
    assert!(
        matches!(
            class_steer.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical class-select receipt is terminal and non-rejected: {}",
        class_steer.receipt_status
    );
    assert!(
        class_steer
            .agent_id
            .contains(":client:wp-kernel-012-mt-064-propose-agent"),
        "the canonical steer receipt retains the external caller attribution: {}",
        class_steer.agent_id
    );

    // (5) CANONICAL CONFIRM over the same localhost transport. This must drive the mounted host's actual
    // dialog -> off-frame submit -> durable-status path; a direct helper POST would be a false-green UI
    // proof because it could pass while the canonical confirm control is disconnected.
    let confirm_steer = argus_click(&mut argus, &mut harness, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
    assert!(
        matches!(
            confirm_steer.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical confirm receipt is terminal and non-rejected: {}",
        confirm_steer.receipt_status
    );
    assert_eq!(
        confirm_steer.agent_id, class_steer.agent_id,
        "class selection and confirmation retain one canonical external caller"
    );

    // Pump the real mounted host until its structured AccessKit status reports the backend's durable
    // proposal + Flight Recorder identities. The dialog is closed by confirmation; the status surface is
    // the canonical terminal re-observation point documented in UserManual.
    let terminal_deadline = Instant::now() + Duration::from_secs(30);
    let terminal_status = loop {
        harness.run_steps(1);
        if let Some(value) = tree_value(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID) {
            if structured_field(&value, "outcome") == Some("event_persisted") {
                break value;
            }
            assert!(
                !matches!(
                    structured_field(&value, "state"),
                    Some("failed" | "blocked" | "partial")
                ),
                "mounted proposal submit reached a non-success terminal status: {value}; label={:?}",
                tree_label(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID)
            );
        }
        assert!(
            Instant::now() < terminal_deadline,
            "mounted canonical confirm did not reach outcome=event_persisted; last status={:?}",
            tree_value(&harness.root(), FEMS_PROPOSE_STATUS_AUTHOR_ID)
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let proposal_id = structured_field(&terminal_status, "proposal_id")
        .filter(|value| !value.is_empty() && *value != "none")
        .expect("terminal mounted status carries durable proposal_id")
        .to_owned();
    let event_id = structured_field(&terminal_status, "event_id")
        .filter(|value| !value.is_empty() && *value != "none")
        .expect("terminal mounted status carries durable event_id")
        .to_owned();

    // FRESH canonical re-inspection must see the terminal status and its exact IDs. This is the
    // inspect -> steer -> re-observe contract, not an in-process-only assertion.
    let after = argus.inspect(&mut harness);
    let canonical_terminal_status = json_author_value(&after, FEMS_PROPOSE_STATUS_AUTHOR_ID)
        .expect("fresh canonical argus.inspect sees fems-propose-status");
    assert_eq!(
        canonical_terminal_status, terminal_status,
        "canonical terminal status matches the mounted AccessKit authority"
    );
    assert_eq!(
        structured_field(&canonical_terminal_status, "outcome"),
        Some("event_persisted")
    );
    assert_eq!(
        structured_field(&canonical_terminal_status, "proposal_id"),
        Some(proposal_id.as_str())
    );
    assert_eq!(
        structured_field(&canonical_terminal_status, "event_id"),
        Some(event_id.as_str())
    );

    let session_hex = harness.state().mcp_token().as_hex().to_owned();
    let deadline = Instant::now() + Duration::from_secs(30);

    // (6) LIVE PostgreSQL readback: the proposal row is a persisted, review-gated pending_review row.
    // Memory routes are session-gated, so the readback carries the mounted app's session token.
    let readback = get_json_session(
        &live.base,
        &session_hex,
        &format!("/workspaces/{workspace_id}/memory/proposals/{proposal_id}"),
        app_rt.handle(),
    );
    assert_eq!(
        readback["proposal_id"], proposal_id,
        "exact proposal readback"
    );
    assert_eq!(
        readback["status"], "pending_review",
        "the editor creates a review-gated proposal, never a direct commit"
    );
    assert_eq!(readback["review_gated"], true, "review_gated is hard-true");
    assert_eq!(readback["memory_class"], "procedural");
    assert_eq!(
        readback["document_id"], provenance_document_id,
        "the proposal carries the exact seeded provenance document id"
    );

    // (7) LIVE Flight Recorder / EventLedger: the correlated FR-EVT-MEM-001 event lands with the EXACT
    // normative shape (the FAIL_V2 root cause) and carries NO raw memory content.
    let fr_row = poll_fr_event(
        &live.base,
        &session_hex,
        &workspace_id,
        &proposal_id,
        deadline,
        app_rt.handle(),
    );
    assert_eq!(fr_row["event_type"], "memory_write_proposed");
    assert_eq!(
        fr_row["event_id"], event_id,
        "terminal status event_id identifies the exact persisted FR-EVT-MEM-001 row"
    );
    assert_eq!(fr_row["wsids"], serde_json::json!([workspace_id]));
    assert_eq!(fr_row["payload"]["type"], "memory_write_proposed");
    assert_eq!(fr_row["payload"]["event_code"], "FR-EVT-MEM-001");
    assert_eq!(fr_row["payload"]["proposal_id"], proposal_id);
    assert_eq!(fr_row["payload"]["op_count"], 1);
    assert_eq!(fr_row["payload"]["requires_review_count"], 1);
    assert!(
        fr_row["payload"]["proposal_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())),
        "FR-EVT-MEM-001 carries a canonical 64-char proposal_hash: {}",
        fr_row["payload"]["proposal_hash"]
    );
    let proposal_artifact = get_json_session(
        &live.base,
        &session_hex,
        &format!("/workspaces/{workspace_id}/memory/proposals/{proposal_id}/artifact"),
        app_rt.handle(),
    );
    assert_eq!(
        proposal_artifact["schema_version"],
        "hsk.memory_write_proposal@0.1"
    );
    assert_eq!(proposal_artifact["proposal_id"], proposal_id);
    assert!(
        uuid::Uuid::parse_str(
            proposal_artifact["proposal_id"]
                .as_str()
                .expect("canonical artifact proposal_id")
        )
        .is_ok(),
        "MemoryWriteProposal proposal_id is a UUID"
    );
    let ops = proposal_artifact["ops"]
        .as_array()
        .expect("canonical MemoryWriteProposal ops");
    assert_eq!(
        ops.len(),
        fr_row["payload"]["op_count"].as_u64().unwrap() as usize
    );
    assert_eq!(
        ops.iter()
            .filter(|op| op["requires_review"] == true)
            .count(),
        fr_row["payload"]["requires_review_count"].as_u64().unwrap() as usize
    );
    let recomputed_proposal_hash = canonical_memory_write_proposal_hash(&proposal_artifact);
    assert_eq!(
        recomputed_proposal_hash, fr_row["payload"]["proposal_hash"],
        "dereferenced canonical proposal artifact recomputes to FR proposal_hash"
    );
    assert_eq!(
        fr_row["payload"]["artifact_ref"],
        format!("artifact://sha256/{recomputed_proposal_hash}")
    );
    assert_eq!(
        fr_row["payload"]["scope_refs"][0]["artefact_type"],
        "workspace"
    );
    assert_eq!(
        fr_row["payload"]["scope_refs"][0]["artefact_id"],
        workspace_id
    );
    assert!(
        fr_row["payload"].get("content").is_none() && fr_row["payload"].get("text").is_none(),
        "FR-EVT-MEM-001 must NOT carry raw memory content: {}",
        fr_row["payload"]
    );

    // (9) Evidence: before/after canonical trees + live rows + a screenshot marker (headless DEFERRED ok).
    let tree_path = artifact_dir.join("mt064-mounted-propose-dialog-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "before_inspect": before,
            "after_reinspect": after,
            "proposal_id": proposal_id,
            "proposal_readback": readback,
            "proposal_artifact": proposal_artifact,
            "fr_event": fr_row,
            "class_steer_receipt_id": class_steer.receipt_id,
            "class_steer_receipt_status": class_steer.receipt_status,
            "confirm_steer_receipt_id": confirm_steer.receipt_id,
            "confirm_steer_receipt_status": confirm_steer.receipt_status,
            "agent_id": confirm_steer.agent_id,
            "terminal_status": canonical_terminal_status,
        }))
        .expect("serialize canonical MT-064 propose-dialog evidence"),
    )
    .expect("write canonical MT-064 propose-dialog evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt064-mounted-propose-dialog.png");
            image
                .save(&path)
                .expect("save mounted propose-dialog screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-064 canonical Argus mounted propose dialog: inspect(dialog+3 classes+confirm) -> \
         argus.click(procedural) -> argus.click(confirm) -> live PG proposal {proposal_id} persisted + \
         FR-EVT-MEM-001 correlated -> reinspect(terminal status); class_receipt={} confirm_receipt={} \
         agent={} \
         screenshot={} tree={}",
        class_steer.receipt_status,
        confirm_steer.receipt_status,
        confirm_steer.agent_id,
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
    drop(app_rt);
}

/// A session-authenticated GET against a live product route (memory routes are session-gated). Sends the
/// mounted app's MCP session token + the stage headers and asserts a 2xx JSON body.
fn get_json_session(
    base: &str,
    session_token: &str,
    path: &str,
    rt: &tokio::runtime::Handle,
) -> serde_json::Value {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .expect("build bounded MT-064 GET client");
    let url = format!("{base}{path}");
    rt.block_on(async {
        let response = client
            .get(&url)
            .header("x-hsk-session-token", session_token)
            .header("x-hsk-actor-id", "native-editor-fems-argus")
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", "mt064-argus-verify")
            .header("x-hsk-session-run-id", "mt064-argus-session")
            .send()
            .await
            .unwrap_or_else(|error| panic!("GET {path} send failed: {error}"));
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        assert!(status.is_success(), "GET {path} -> {status}: {text}");
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("GET {path} not JSON ({error}): {text}"))
    })
}

/// Bounded poll of the LIVE Flight Recorder for the correlated FR-EVT-MEM-001 (`memory_write_proposed`)
/// row. Product write routes await their recorder append before responding, but a short bounded poll
/// tolerates a projection that becomes visible just after the authority commit.
fn poll_fr_event(
    base: &str,
    session_token: &str,
    workspace_id: &str,
    proposal_id: &str,
    deadline: Instant,
    rt: &tokio::runtime::Handle,
) -> serde_json::Value {
    let path = format!("/api/flight_recorder?event_type=memory_write_proposed&wsid={workspace_id}");
    let poll_deadline = deadline.max(Instant::now() + Duration::from_secs(15));
    loop {
        let rows = get_json_session(base, session_token, &path, rt);
        if let Some(row) = rows.as_array().into_iter().flatten().find(|row| {
            row["payload"]["proposal_id"] == proposal_id
                && row["payload"]["event_code"] == "FR-EVT-MEM-001"
        }) {
            return row.clone();
        }
        assert!(
            Instant::now() < poll_deadline,
            "no FR-EVT-MEM-001 row correlated proposal {proposal_id} within the live poll window"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
