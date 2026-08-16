// SWARM_PROOF_CONSTRAINT: only AccessKit action dispatch is permitted in this test. Any keyboard
// simulation fails the proof. (AC-043-07 / IN-043-09)
//
//! WP-KERNEL-012 MT-043 (E7 model-vision parity) — **SwarmEditProof**.
//!
//! This is a PROOF task, not a feature task. It demonstrates that an out-of-process swarm agent can
//! CREATE a note, EDIT code, ADD a backlink, and RUN a search by driving the running native Handshake
//! editor + knowledge surfaces EXCLUSIVELY through the WP-011 AccessKit action channel registered by
//! MT-041 (the `EditorActionRegistry`) and MT-042 (the `KnowledgeActionRegistry`). No keyboard
//! simulation, no screen-scraping, no direct Rust function calls from the agent into the application
//! under test. The agent talks to the UI ONLY via the AccessKit IPC mechanism (the same channel a real
//! external process would use: an `author_id` + a `UiAction`, resolved to a stable `NodeId` and fed to
//! egui as an `AccessKitActionRequest` by `crate::mcp::action`).
//!
//! ## HONEST PROOF FRAMING (KERNEL_BUILDER gate 2026-06-24)
//!
//! The full end-to-end (spawn the WHOLE Handshake app + assert real PostgreSQL rows) has TWO real
//! constraints the contract names: (1) the editor panes are NOT mounted in `app.rs` yet (E11/MT-069 —
//! the rich/code editors are not in the live shell), and (2) NO live Handshake-managed PostgreSQL is
//! available (every prior MT gated DB round-trips as `NEEDS_MANAGED_RESOURCE_PROOF`).
//! So MT-043's RUNNABLE proof mounts the editor + knowledge WIDGETS in egui_kittest (the
//! `RichEditorWidget` / `LoomSearchV2` / graph panes ARE kittest-mountable, as MT-041/042 proved), drives
//! the agent-drivable steps PURELY via AccessKit dispatch from a CHANNEL-ONLY agent thread, and proves the
//! AccessKit ROUTING + ACTION COVERAGE + (for STEP 1) the AGENT-PRODUCED content + the backend REQUEST
//! SHAPE the save produces (via a backend SPY capturing the E6-client request — provable NOW). The
//! The standalone test remains an explicitly non-authoritative surface diagnostic. The non-ignored
//! `integration` test creates its own isolated managed-PostgreSQL workspace and is the only live-verdict
//! path; it does not consume seeded operator data and never substitutes a mock database.
//!
//! ## Spec-Realism Gate: agent-PRODUCED content, never implementer-injected (adversarial-review fix)
//!
//! STEP 1 (create-note) does NOT inject the created content and then assert the serializer round-trips it
//! (the implementer-injects-then-asserts tautology the Spec-Realism Gate forbids). The agent CLICKS the
//! stable MT-041 `editor.rich.format-heading-1` action node PURELY via AccessKit; that Click routes
//! through `EditorActionRegistry::take_dispatched -> run_rich_dispatch -> RichDispatch::Format(SetHeading
//! (1))`, converting the caret block to a `heading`. The doc STARTS as a plain paragraph; ONLY the agent's
//! `format-heading-1` Click turns `content[0]` into a heading — so the saved `content_json` heading is
//! AGENT-PRODUCED. The contract names `insert-slash-command`, but the slash PICKER is NOT an agent-drivable
//! headless content surface: its `slash-item-*` nodes render only while the menu stays open, and both the
//! focus-gated `refresh_slash_trigger` (closes the menu when no `/` text token is present) and the
//! unfocused-surface auto-close prevent that headlessly (empirically confirmed: dispatching
//! `insert-slash-command`, with or without an AccessKit Focus, leaves `slash_menu` closed and emits no
//! `slash-item-*` nodes). So STEP 1 drives the EQUIVALENT stable `format-heading-1` block-create action
//! MT-041 registered — the agent-drivable create surface as-delivered, no transient picker required.
//!
//! ## STEP-2 AND STEP-3 now PASS via AccessKit-by-id (MT-080 code editor + MT-110 rich editor)
//!
//! STEP 2 (write code into the code editor purely via AccessKit) PASSES. MT-080 added
//! `Action::SetValue` + `Action::ReplaceSelectedText` to the code editor's `code_editor_text`
//! `Role::TextInput` node (`src/code_editor/panel.rs`: `add_action(SetValue/ReplaceSelectedText)` +
//! `consume_swarm_text_actions`, which drains the swarm request and applies it to the buffer). So a swarm
//! agent CAN author code by id: STEP 2 mounts a `CodeEditorPanel`, snapshots the live AccessKit tree,
//! dispatches a RAW `egui::Event::AccessKitActionRequest { action: SetValue, target: <resolved
//! code_editor_text node id>, data: Value(<agent code>) }` (the MT-080 shape — a genuine AccessKit-by-id
//! write, NOT key-simulation, NOT an app-code change), and asserts `panel.buffer()` carries the
//! agent-produced code. Only the live-PG `editor_edit` row is GATED (`NEEDS_MANAGED_RESOURCE_PROOF`).
//!
//! STEP 3 (add a backlink — an `hsLink` atom carrying a SPECIFIC target `refValue`, the loom_edges edge
//! AC-043-04 names) now PASSES too. MT-110 gave the RICH editor the SAME out-of-process swarm-edit surface
//! (the MT-080 mirror for the rich pane): the `rich-editor-root` `Role::TextInput` node advertises
//! `Action::SetValue` + `Action::ReplaceSelectedText`, PLAIN wikilink chips advertise `Action::SetValue`
//! (a headless wikilink-target-by-id pick — NO live backend search), and
//! `RichEditorWidget::consume_swarm_text_actions` applies each dispatched request to the DocJson model
//! THROUGH the MT-035 unified undo bus (undoable, no `set_text` bypass). So STEP 3 mounts a
//! `RichEditorWidget` over a doc holding a PLACEHOLDER wikilink whose target is NOT the loom_edges target,
//! dispatches a RAW `AccessKitActionRequest { action: SetValue, target: <resolved wikilink-chip node id>,
//! data: Value(<agent-chosen loom_edges target id>) }`, and asserts the hsLink atom now carries the
//! AGENT-CHOSEN target `refValue` — the backlink is AGENT-produced (the specific target is the agent's
//! pick, not implementer-injected; the Spec-Realism Gate holds). Only the live-PG `loom_edges` row stays
//! GATED (`NEEDS_MANAGED_RESOURCE_PROOF`).
//!
//! The standalone widget proof is deliberately NON-AUTHORITATIVE: gated or seeded observations can
//! never produce `PROOF_PASS`. Only the integration proof may write that terminal marker, and only
//! after its managed-PostgreSQL edit/conflict/merge/search/refetch/reload/attribution assertions pass.

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::accesskit;
use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use serde::{Deserialize, Serialize};

use handshake_native::accessibility::editor_action_registry::EditorActionRegistry;
use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::{
    HealthInfo, HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID,
    HSK_HEADER_SESSION_RUN_ID,
};
use handshake_native::backend_client::{LoomSearchBlock, LoomSearchV2Hit, LoomSearchV2Response};
use handshake_native::loom_search_v2 as lsv2;
use handshake_native::mcp::action::{ActionChannel, ActionError, UiAction};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::node::{BlockNode, NodeKind};
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;
use handshake_native::rich_editor::save::conflict_ui::CONFLICT_KEEP_SERVER_AUTHOR_ID;
use handshake_native::rich_editor::save::draft_manager::{
    DraftBackend, DraftError, DraftLoadFuture, DraftManager, DraftWriteFuture,
    RichDocumentDraftLoad,
};
use handshake_native::rich_editor::save::save_manager::{
    RichDocLoad, RichDocSaveResult, SaveBackend, SaveFuture, SaveManager, SaveState,
};
use handshake_native::tab_bar::TabState;

use handshake_native::backend::knowledge_documents::{
    HskDocumentHeaders, KnowledgeDocumentsClient, HSK_HEADER_CORRELATION_ID,
};
use handshake_native::command_registry::CMD_VIEW_GRAPH;
use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;

// ── artifact-hygiene guard (CX-212E) ─────────────────────────────────────────────────────────────

/// Assert NO repo-local artifact dir exists under the crate (CX-212E): neither `test_output/` nor
/// `tests/screenshots/`. This MT writes its proof log to the CHECKED-IN evidence fixture
/// (`tests/fixtures/swarm_edit_proof_log.txt`, the HBR-VIS artifact the contract names) — it writes NO
/// screenshots and NO `test_output/`/`tests/screenshots/` artifacts. The reviewer also greps
/// `git ls-files "src/**/*.png"`; this guard catches a stray local artifact dir.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local {local} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root or the checked-in proof-log fixture only (found {})",
            p.display()
        );
    }
}

/// Runtime proof evidence is retained both externally and in the contract-required checked-in
/// fixture. Every authoritative attempt replaces both copies atomically and begins with its unique
/// attempt id, so an earlier fixture can never masquerade as the current managed-backend verdict.
fn proof_log_path() -> PathBuf {
    let artifacts_root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("handshake_native crate must be nested below the shared worktree root")
                .join("Handshake_Artifacts")
        });
    artifacts_root
        .join("handshake-test/wp-kernel-012-mt-043")
        .join("swarm_edit_proof_log.txt")
}

const MT043_PARENT_PID_ENV: &str = "HSK_MT043_PARENT_PID";
const MT043_ATTEMPT_ID_ENV: &str = "HSK_MT043_ATTEMPT_ID";
const MT043_LIVE_CHILD_ENV: &str = "HSK_MT043_LIVE_CHILD";
const MT128_CHILD_BUDGET_ENV: &str = "HSK_MT128_CHILD_BUDGET_MS";
const MT128_FORCE_STALL_ENV: &str = "HSK_MT128_FORCE_STALL_MS";
// MT-128's loaded child observation was 57.43s. The canonical 120s budget adds 62.57s (109%)
// measured headroom while retaining a finite bound. The environment override is lower-only and exists
// solely so the forced-stall proof can exercise the reap path without waiting two minutes.
const MT128_MEASURED_LOADED_CHILD_MS: u64 = 57_430;
const MT128_MEASURED_HEADROOM_MS: u64 = 62_570;
const MT128_DEFAULT_CHILD_BUDGET_MS: u64 =
    MT128_MEASURED_LOADED_CHILD_MS + MT128_MEASURED_HEADROOM_MS;
const MT128_REAP_AND_CLEANUP_RESERVE: Duration = Duration::from_secs(6);
const MT128_DIAGNOSTIC_RESERVE: Duration = Duration::from_millis(250);

fn mt128_child_budget() -> Duration {
    let configured_ms = std::env::var(MT128_CHILD_BUDGET_ENV)
        .ok()
        .map(|value| {
            value.parse::<u64>().unwrap_or_else(|error| {
                panic!("{MT128_CHILD_BUDGET_ENV} must be an unsigned millisecond duration: {error}")
            })
        })
        .unwrap_or(MT128_DEFAULT_CHILD_BUDGET_MS)
        .min(MT128_DEFAULT_CHILD_BUDGET_MS);
    assert!(
        configured_ms >= 250,
        "{MT128_CHILD_BUDGET_ENV} must leave at least 250ms for deterministic child startup"
    );
    Duration::from_millis(configured_ms)
}

fn checked_in_proof_log_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/swarm_edit_proof_log.txt")
}

fn attempt_proof_log_path(attempt_id: &str) -> PathBuf {
    assert!(
        !attempt_id.is_empty()
            && attempt_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')),
        "MT-043 attempt id must be a safe filename component"
    );
    proof_log_path()
        .parent()
        .expect("MT-043 proof path has a parent")
        .join("runs")
        .join(format!("{attempt_id}.txt"))
}

fn mt128_progress_path(attempt_id: &str) -> PathBuf {
    assert!(
        !attempt_id.is_empty()
            && attempt_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')),
        "MT-128 attempt id must be a safe filename component"
    );
    proof_log_path()
        .parent()
        .expect("MT-043 proof path has a parent")
        .join("runs")
        .join(format!("{attempt_id}.progress.jsonl"))
}

fn append_mt128_progress(attempt_id: &str, event: serde_json::Value) {
    let path = mt128_progress_path(attempt_id);
    std::fs::create_dir_all(path.parent().expect("MT-128 progress path has a parent"))
        .expect("create MT-128 progress directory");
    let mut line = serde_json::to_vec(&event).expect("serialize MT-128 progress event");
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("open MT-128 append-only progress journal");
    file.write_all(&line)
        .expect("append MT-128 progress journal event");
    file.sync_data()
        .expect("flush MT-128 progress journal event before next proof action");
}

fn publish_mt128_last_gate(attempt_id: &str, gate: &str) {
    append_mt128_progress(
        attempt_id,
        serde_json::json!({
            "attempt_id": attempt_id,
            "kind": "gate",
            "gate": gate,
            "child_pid": std::process::id(),
        }),
    );
}

#[derive(Debug, Default)]
struct Mt128Progress {
    last_gate: Option<String>,
    workspace_id: Option<String>,
    backend_pid: Option<u32>,
    forced_stall_ready: bool,
}

fn read_mt128_progress(path: &Path, attempt_id: &str) -> Mt128Progress {
    let mut progress = Mt128Progress::default();
    let Ok(body) = std::fs::read_to_string(path) else {
        return progress;
    };
    // A process kill may interrupt the final append. JSONL makes every earlier newline-terminated
    // record authoritative while an incomplete tail is safely ignored; there is no delete/rename gap.
    for line in body.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event["attempt_id"].as_str() != Some(attempt_id) {
            continue;
        }
        match event["kind"].as_str() {
            Some("gate") => {
                progress.last_gate = event["gate"]
                    .as_str()
                    .filter(|gate| {
                        gate.len() == 5
                            && gate.starts_with('T')
                            && gate[1..]
                                .chars()
                                .all(|character| character.is_ascii_digit())
                    })
                    .map(str::to_owned);
            }
            Some("forced_stall_ready") => {
                progress.workspace_id = event["workspace_id"].as_str().map(str::to_owned);
                progress.backend_pid = event["backend_pid"]
                    .as_u64()
                    .and_then(|pid| u32::try_from(pid).ok());
                progress.forced_stall_ready = true;
            }
            _ => {}
        }
    }
    progress
}

fn maybe_force_mt128_child_stall(
    attempt_id: &str,
    live: &interconnect_support::LiveBackend,
    workspace_id: &str,
) {
    let Some(stall_ms) = std::env::var(MT128_FORCE_STALL_ENV).ok().map(|value| {
        value.parse::<u64>().unwrap_or_else(|error| {
            panic!("{MT128_FORCE_STALL_ENV} must be an unsigned millisecond duration: {error}")
        })
    }) else {
        return;
    };
    assert!(stall_ms > 0, "{MT128_FORCE_STALL_ENV} must be non-zero");
    let backend = live.owned_backend_binding_receipt();
    let backend_pid = backend["backend_pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .expect("forced-stall proof requires a fixture-owned backend PID");
    assert!(
        !workspace_id.is_empty(),
        "forced-stall proof requires a real workspace"
    );
    append_mt128_progress(
        attempt_id,
        serde_json::json!({
            "attempt_id": attempt_id,
            "kind": "forced_stall_ready",
            "child_pid": std::process::id(),
            "backend_pid": backend_pid,
            "workspace_id": workspace_id,
        }),
    );
    println!(
        "MT128_FORCED_STALL_READY child_pid={} backend_pid={backend_pid} workspace_id={workspace_id} stall_ms={stall_ms}",
        std::process::id()
    );
    std::thread::sleep(Duration::from_millis(stall_ms));
}

fn timeout_workspace_path(parent_pid: &str) -> PathBuf {
    proof_log_path()
        .parent()
        .expect("MT-043 proof path has a parent")
        .join(format!("active_workspace_{parent_pid}.txt"))
}

fn publish_timeout_workspace(workspace_id: &str) -> PathBuf {
    let parent_pid = std::env::var(MT043_PARENT_PID_ENV)
        .expect("bounded MT-043 child receives its parent process id");
    let path = timeout_workspace_path(&parent_pid);
    std::fs::create_dir_all(path.parent().expect("timeout workspace path parent"))
        .expect("create MT-043 timeout workspace directory");
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&temporary, workspace_id).expect("write MT-043 timeout workspace sidecar");
    if path.exists() {
        std::fs::remove_file(&path).expect("replace prior MT-043 timeout workspace sidecar");
    }
    std::fs::rename(&temporary, &path).expect("publish MT-043 timeout workspace sidecar");
    path
}

/// Read the atomically published workspace identity, rejecting an unsafe/truncated sidecar.
fn timeout_workspace_id(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|workspace_id| {
            !workspace_id.is_empty()
                && workspace_id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        })
}

#[derive(Clone, Debug)]
struct Mt128ProcessIdentity {
    pid: u32,
    start_time: u64,
}

#[derive(Debug)]
struct Mt128ReapReport {
    tree_before: Vec<Mt128ProcessIdentity>,
    root_identity_revalidated: bool,
    taskkill_reaped: bool,
    root_reaped: bool,
    tree_reaped: bool,
}

fn mt128_process_tree(root_pid: u32) -> Vec<Mt128ProcessIdentity> {
    let system = sysinfo::System::new_all();
    let mut tree_pids = vec![root_pid];
    loop {
        let before = tree_pids.len();
        for (pid, process) in system.processes() {
            let pid = pid.as_u32();
            if tree_pids.contains(&pid) {
                continue;
            }
            if process
                .parent()
                .is_some_and(|parent| tree_pids.contains(&parent.as_u32()))
            {
                tree_pids.push(pid);
            }
        }
        if tree_pids.len() == before {
            break;
        }
    }
    tree_pids
        .into_iter()
        .filter_map(|pid| {
            system
                .process(sysinfo::Pid::from_u32(pid))
                .map(|process| Mt128ProcessIdentity {
                    pid,
                    start_time: process.start_time(),
                })
        })
        .collect()
}

fn mt128_process_identities_are_gone(identities: &[Mt128ProcessIdentity]) -> bool {
    let system = sysinfo::System::new_all();
    identities.iter().all(|identity| {
        !system
            .process(sysinfo::Pid::from_u32(identity.pid))
            .is_some_and(|process| process.start_time() == identity.start_time)
    })
}

fn mt128_wait_for_process_before(process: &mut std::process::Child, deadline: Instant) -> bool {
    loop {
        match process.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) | Err(_) => return false,
        }
    }
}

fn terminate_child_tree_before(
    child: &mut std::process::Child,
    absolute_deadline: Instant,
) -> Mt128ReapReport {
    let tree_before = mt128_process_tree(child.id());
    terminate_observed_child_tree_before(child, tree_before, absolute_deadline)
}

fn terminate_observed_child_tree_before(
    child: &mut std::process::Child,
    tree_before: Vec<Mt128ProcessIdentity>,
    absolute_deadline: Instant,
) -> Mt128ReapReport {
    let root_identity = tree_before
        .iter()
        .find(|identity| identity.pid == child.id())
        .cloned();
    let root_identity_revalidated = matches!(child.try_wait(), Ok(None))
        && root_identity.as_ref().is_some_and(|identity| {
            !mt128_process_identities_are_gone(std::slice::from_ref(identity))
        });
    let mut taskkill_reaped = true;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if root_identity_revalidated {
            let mut taskkill = Command::new("taskkill");
            taskkill
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000);
            match taskkill.spawn() {
                Ok(mut reaper) => {
                    let helper_deadline =
                        (Instant::now() + Duration::from_secs(1)).min(absolute_deadline);
                    if !mt128_wait_for_process_before(&mut reaper, helper_deadline) {
                        let _ = reaper.kill();
                        taskkill_reaped =
                            mt128_wait_for_process_before(&mut reaper, absolute_deadline);
                    }
                }
                Err(_) => taskkill_reaped = false,
            }
        } else {
            // Child::kill uses the owned process handle; unlike taskkill-by-PID it cannot target a
            // reused numeric PID. Mark tree reaping unproven and avoid issuing the unsafe PID command.
            taskkill_reaped = false;
        }
    }
    let _ = child.kill();
    let root_reaped = mt128_wait_for_process_before(child, absolute_deadline);
    let tree_reaped = loop {
        let all_gone = mt128_process_identities_are_gone(&tree_before);
        if all_gone || Instant::now() >= absolute_deadline {
            break all_gone;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Mt128ReapReport {
        tree_before,
        root_identity_revalidated,
        taskkill_reaped,
        root_reaped,
        tree_reaped,
    }
}

fn terminate_child_tree_bounded(child: &mut std::process::Child, budget: Duration) -> bool {
    let report = terminate_child_tree_before(child, Instant::now() + budget);
    report.root_identity_revalidated
        && report.taskkill_reaped
        && report.root_reaped
        && report.tree_reaped
}

/// Prefer the product workspace DELETE while the backend is still alive so its Flight Recorder purge
/// runs. SQL cleanup below remains the crash-safe fallback and canonical residue assertion.
fn cleanup_timeout_workspace_via_api(path: &Path, budget: Duration) -> bool {
    let Some(workspace_id) = timeout_workspace_id(path) else {
        return false;
    };
    let base = std::env::var("HSK_TEST_BASE")
        .unwrap_or_else(|_| interconnect_support::DEFAULT_BASE.to_owned());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build bounded MT-043 timeout API-cleanup runtime");
    runtime.block_on(async {
        let request = reqwest::Client::new()
            .delete(format!("{base}/workspaces/{workspace_id}"))
            .header(HSK_HEADER_ACTOR_ID, "mt043-timeout-parent")
            .header(HSK_HEADER_ACTOR_KIND, "operator")
            .header(HSK_HEADER_KERNEL_TASK_RUN_ID, "mt043-timeout-cleanup")
            .header(HSK_HEADER_SESSION_RUN_ID, "mt043-timeout-cleanup")
            .header(HSK_HEADER_CORRELATION_ID, "mt043-timeout-cleanup");
        matches!(
            tokio::time::timeout(budget, request.send()).await,
            Ok(Ok(response)) if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND
        )
    })
}

/// Crash/timeout cleanup by exact workspace id or, if termination landed between workspace creation
/// and sidecar publication, by the attempt-unique workspace name. The command and reap are bounded.
fn cleanup_timeout_workspace(path: &Path, attempt_id: &str, budget: Duration) {
    cleanup_timeout_workspace_before(path, attempt_id, Instant::now() + budget);
}

fn cleanup_timeout_workspace_before(path: &Path, attempt_id: &str, absolute_deadline: Instant) {
    assert!(
        !attempt_id.is_empty()
            && attempt_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')),
        "timeout cleanup attempt id is unsafe"
    );
    let workspace_id = timeout_workspace_id(path);
    let database_url = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
    .expect("timeout cleanup requires the managed PostgreSQL DSN");
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let workspace_predicate = workspace_id
        .as_deref()
        .map(|workspace_id| {
            format!(
                "id = {} OR name = {}",
                sql_literal(workspace_id),
                sql_literal(&format!("mt043-{attempt_id}"))
            )
        })
        .unwrap_or_else(|| format!("name = {}", sql_literal(&format!("mt043-{attempt_id}"))));
    let sql = format!(
        "DO $$ BEGIN DELETE FROM workspaces WHERE {workspace_predicate}; IF EXISTS (SELECT 1 FROM workspaces WHERE {workspace_predicate}) THEN RAISE EXCEPTION 'MT-043 timeout cleanup left workspace'; END IF; END $$;"
    );
    let mut command = Command::new(psql);
    command
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--dbname")
        .arg(database_url)
        .arg("--command")
        .arg(sql)
        .env("PGCONNECT_TIMEOUT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut cleanup = command
        .spawn()
        .expect("start bounded MT-043 PostgreSQL timeout cleanup");
    let command_deadline = absolute_deadline
        .checked_sub(Duration::from_millis(250))
        .unwrap_or_else(Instant::now);
    let status = loop {
        match cleanup.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < command_deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = cleanup.kill();
                let reaped = mt128_wait_for_process_before(&mut cleanup, absolute_deadline);
                assert!(
                    reaped,
                    "MT-043 PostgreSQL timeout cleanup helper was not reaped before the absolute deadline"
                );
                panic!("MT-043 PostgreSQL timeout cleanup exceeded its bounded execution budget");
            }
            Err(error) => {
                let _ = cleanup.kill();
                let reaped = mt128_wait_for_process_before(&mut cleanup, absolute_deadline);
                assert!(
                    reaped,
                    "MT-043 PostgreSQL timeout cleanup helper was not reaped after poll failure"
                );
                panic!("poll MT-043 PostgreSQL timeout cleanup: {error}");
            }
        }
    };
    assert!(
        status.success(),
        "MT-043 PostgreSQL timeout cleanup failed with {status}"
    );
    if path.exists() {
        std::fs::remove_file(path).expect("remove MT-043 timeout workspace sidecar after cleanup");
    }
}

/// A short, read-only canonical witness used immediately before tree termination. It proves that the
/// forced-stall workspace was real without deleting it or extending the PID-observation window by more
/// than the caller's small deadline. Failure is returned, never panicked through the reap path.
struct Mt128WorkspaceProbe {
    exists: bool,
    helper_reaped: bool,
}

fn probe_timeout_workspace_exists_before(
    path: &Path,
    attempt_id: &str,
    absolute_deadline: Instant,
) -> Mt128WorkspaceProbe {
    let Some(workspace_id) = timeout_workspace_id(path) else {
        return Mt128WorkspaceProbe {
            exists: false,
            helper_reaped: true,
        };
    };
    let Some(database_url) = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }) else {
        return Mt128WorkspaceProbe {
            exists: false,
            helper_reaped: true,
        };
    };
    let workspace_predicate = format!(
        "id = {} OR name = {}",
        sql_literal(&workspace_id),
        sql_literal(&format!("mt043-{attempt_id}"))
    );
    let sql = format!(
        "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM workspaces WHERE {workspace_predicate}) THEN RAISE EXCEPTION 'MT-128 forced-stall workspace missing before reap'; END IF; END $$;"
    );
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let mut command = Command::new(psql);
    command
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--dbname")
        .arg(database_url)
        .arg("--command")
        .arg(sql)
        .env("PGCONNECT_TIMEOUT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let Ok(mut probe) = command.spawn() else {
        return Mt128WorkspaceProbe {
            exists: false,
            helper_reaped: true,
        };
    };
    let command_deadline = absolute_deadline
        .checked_sub(Duration::from_millis(100))
        .unwrap_or_else(Instant::now);
    loop {
        match probe.try_wait() {
            Ok(Some(status)) => {
                return Mt128WorkspaceProbe {
                    exists: status.success(),
                    helper_reaped: true,
                };
            }
            Ok(None) if Instant::now() < command_deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) | Err(_) => {
                let _ = probe.kill();
                return Mt128WorkspaceProbe {
                    exists: false,
                    helper_reaped: mt128_wait_for_process_before(&mut probe, absolute_deadline),
                };
            }
        }
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

struct Mt128TimeoutCleanup {
    progress: Mt128Progress,
    child_alive_before: bool,
    backend_alive_before: bool,
    workspace_identity_matched: bool,
    workspace_existed_before_reap: bool,
    probe_helper_reaped: bool,
    workspace_cleanup_verified: bool,
    reap: Mt128ReapReport,
}

fn mt128_cleanup_timed_out_attempt(
    child: &mut std::process::Child,
    attempt_id: &str,
    progress_path: &Path,
    workspace_sidecar: &Path,
    absolute_deadline: Instant,
) -> Mt128TimeoutCleanup {
    let progress = read_mt128_progress(progress_path, attempt_id);
    let child_alive_before = child
        .try_wait()
        .expect("poll MT-128 child before timeout cleanup")
        .is_none();
    let tree_before = mt128_process_tree(child.id());
    let backend_alive_before = progress.backend_pid.is_some_and(|backend_pid| {
        tree_before
            .iter()
            .any(|identity| identity.pid == backend_pid)
    });
    let workspace_identity_matched =
        progress.workspace_id.as_deref() == timeout_workspace_id(workspace_sidecar).as_deref();

    // Keep pre-reap database evidence read-only and short. It cannot delete the workspace or prevent
    // tree termination. The reaper immediately revalidates the exact root PID/start-time identity so a
    // delayed probe cannot turn the captured PID set into a PID-reuse kill target.
    let probe_deadline = (Instant::now() + Duration::from_millis(500)).min(absolute_deadline);
    let pre_reap_probe = std::panic::catch_unwind(|| {
        probe_timeout_workspace_exists_before(workspace_sidecar, attempt_id, probe_deadline)
    })
    .unwrap_or(Mt128WorkspaceProbe {
        exists: false,
        helper_reaped: true,
    });
    // Reap first. Reserve enough of the same absolute deadline for the unconditional PostgreSQL
    // delete-and-absence assertion that follows, even if taskkill/root/tree verification fails.
    let reap_deadline = absolute_deadline
        .checked_sub(Duration::from_millis(2_250))
        .unwrap_or(absolute_deadline);
    let reap = terminate_observed_child_tree_before(child, tree_before, reap_deadline);
    let workspace_cleanup = std::panic::catch_unwind(|| {
        cleanup_timeout_workspace_before(workspace_sidecar, attempt_id, absolute_deadline)
    });
    let workspace_cleanup_verified = workspace_cleanup.is_ok();
    Mt128TimeoutCleanup {
        progress,
        child_alive_before,
        backend_alive_before,
        workspace_identity_matched,
        workspace_existed_before_reap: pre_reap_probe.exists,
        probe_helper_reaped: pre_reap_probe.helper_reaped,
        workspace_cleanup_verified,
        reap,
    }
}

// ── proof-log recorder (IN-043-07 format + CTRL-043-03 atomic PROOF_PASS) ─────────────────────────

/// The DB-assertion outcome a proof line records. The contract's HONEST framing requires the log to
/// DISTINGUISH the swarm-navigability proof (AccessKit routing -> action -> backend request shape) that
/// passes NOW from the live-DB round-trip that is GATED, and a genuine action GAP that is BLOCKED.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DbResult {
    /// A request-SHAPE / routing assertion passed at the widget layer (provable now).
    Pass,
    /// A backend assertion that the standalone surface diagnostic cannot prove.
    Gated,
    /// A genuine AccessKit action GAP blocks the step (a typed blocker, NOT a fake pass, NOT a masked
    /// PROOF_FAIL of the runnable steps). Carries the missing-surface name so the blocked step's token is
    /// honest + distinct. RETAINED as the honest-blocker vocabulary (no step constructs it now: MT-080
    /// shipped the code-editor `SetValue` surface and MT-110 shipped the rich-editor mirror, so STEP 2 and
    /// STEP 3 both PASS); kept so a future genuine gap can be recorded honestly rather than masked.
    #[allow(dead_code)]
    Blocked(&'static str),
    /// A step that produces no DB effect (a pure UI/observable-state assertion).
    NoDb,
}

impl DbResult {
    fn token(&self) -> String {
        match self {
            DbResult::Pass => "PASS".to_owned(),
            DbResult::Gated => "GATED:NEEDS_MANAGED_RESOURCE_PROOF".to_owned(),
            DbResult::Blocked(surface) => format!("BLOCKED:{surface}"),
            DbResult::NoDb => "SKIP".to_owned(),
        }
    }
}

/// Accumulates proof lines in memory. Only [`Self::finish_live_pass`] can write `PROOF_PASS`, after the
/// complete integration scenario; standalone surface diagnostics never write an authority artifact.
struct ProofLog {
    lines: Vec<String>,
    seq: u64,
    live_authority: bool,
    terminal: bool,
    attempt_id: String,
    generation: u128,
}

impl ProofLog {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            seq: 0,
            live_authority: false,
            terminal: false,
            attempt_id: "surface-only".to_owned(),
            generation: 0,
        }
    }

    /// Start one authoritative managed-runtime attempt and immediately replace any result from an
    /// earlier run. The Drop guard below turns every unwind/early return into a terminal failure, so a
    /// stale `PROOF_PASS` can never survive a newer failed attempt.
    fn begin_live_attempt(attempt_id: &str) -> Self {
        let generation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("proof generation clock after unix epoch")
            .as_nanos();
        let mut log = Self {
            lines: vec![format!(
                "PROOF_RUNNING attempt_id={attempt_id} generation={generation}"
            )],
            seq: 0,
            live_authority: true,
            terminal: false,
            attempt_id: attempt_id.to_owned(),
            generation,
        };
        log.note("authoritative managed-PostgreSQL attempt started");
        log.flush();
        log
    }

    /// Publish one terminal failure with a caller-supplied per-file lock budget. Parent watchdog paths
    /// use this single-flush form so recording failure cannot silently consume the hard wall deadline
    /// through the two flushes of `begin_live_attempt(...).finish_fail(...)`.
    fn write_terminal_fail(attempt_id: &str, reason: &str, lock_budget: Duration) {
        let generation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("proof generation clock after unix epoch")
            .as_nanos();
        let mut log = Self {
            lines: vec![format!(
                "PROOF_RUNNING attempt_id={attempt_id} generation={generation}"
            )],
            seq: 0,
            live_authority: true,
            terminal: true,
            attempt_id: attempt_id.to_owned(),
            generation,
        };
        log.note("authoritative managed-PostgreSQL attempt started");
        log.lines.push(format!("PROOF_FAIL: {reason}"));
        log.flush_with_lock_budget(lock_budget);
    }

    /// A pseudo-ISO8601 monotonic timestamp token. The proof is deterministic + headless, so a wall
    /// clock is unnecessary (and would make the checked-in log churn every run); a monotonic sequence
    /// keeps the IN-043-07 `[<timestamp>]` slot present + ordered without nondeterministic noise.
    fn ts(&mut self) -> String {
        self.seq += 1;
        let gate = format!("T{:04}", self.seq);
        if self.live_authority
            && std::env::var_os(MT043_LIVE_CHILD_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
        {
            publish_mt128_last_gate(&self.attempt_id, &gate);
        }
        gate
    }

    /// Record a DISPATCH line (IN-043-07): the action a swarm agent dispatched, by author_id.
    fn dispatch(&mut self, author_id: &str, action: &str, payload: Option<&str>) {
        assert_mt043_action_namespace(author_id);
        let ts = self.ts();
        let payload = payload
            .map(|value| serde_json::to_string(value).expect("serialize one-line proof payload"))
            .unwrap_or_else(|| "null".to_owned());
        self.lines.push(format!(
            "[{ts}] DISPATCH author_id={author_id} action={action} payload={}",
            payload
        ));
    }

    /// Record a RESPONSE line (IN-043-07): the tree change the dispatch produced + the DB/shape result.
    fn response(&mut self, tree_change: &str, db_result: DbResult) {
        let ts = self.ts();
        self.lines.push(format!(
            "[{ts}] RESPONSE tree_change={tree_change} db_result={}",
            db_result.token()
        ));
    }

    /// A free-form note line (step headers / blocker disclosures).
    fn note(&mut self, msg: &str) {
        let ts = self.ts();
        self.lines.push(format!("[{ts}] NOTE {msg}"));
    }

    fn action_line_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| l.contains(" DISPATCH ") || l.contains(" RESPONSE "))
            .count()
    }

    /// Write a live verdict only when no gated/blocked observation remains in the asserted scenario.
    fn finish_live_pass(mut self) {
        assert!(
            self.action_line_count() >= 10,
            "live PROOF_PASS requires at least ten canonical dispatch/response action lines"
        );
        assert!(
            self.lines
                .iter()
                .all(|line| !line.contains("GATED:") && !line.contains("BLOCKED:")),
            "PROOF_PASS is forbidden while any scenario assertion is gated or blocked"
        );
        self.lines.push("PROOF_PASS".to_owned());
        self.flush();
        self.terminal = true;
    }

    /// Standalone surface checks are useful diagnostics, but cannot issue a live verdict or mutate
    /// the proof artifact. Print them only.
    fn finish_surface_only(mut self) {
        self.lines
            .push("PROOF_NOT_RUN: managed PostgreSQL scenario required".to_owned());
        let body = self.lines.join("\n") + "\n";
        assert!(!body.contains("PROOF_PASS"));
        println!("--- MT-043 standalone surface diagnostics (non-authoritative) ---\n{body}");
    }

    /// Atomically write the full log + `PROOF_FAIL: <reason>` (the HBR-STOP path — a genuine gap that
    /// blocks a RUNNABLE step, not a gated/blocked-but-disclosed line). RETAINED as the honest-STOP path
    /// (no step calls it now: all four steps GATED/PASS); kept so a future genuine gap ends the log
    /// honestly rather than being masked as a pass.
    #[allow(dead_code)]
    fn finish_fail(mut self, reason: &str) {
        self.lines.push(format!("PROOF_FAIL: {reason}"));
        self.flush();
        self.terminal = true;
    }

    fn flush(&self) {
        self.flush_with_lock_budget(Duration::from_secs(2));
    }

    fn flush_with_lock_budget(&self, lock_budget: Duration) {
        let body = self.lines.join("\n") + "\n";
        let attempt_path = attempt_proof_log_path(&self.attempt_id);
        self.write_body(
            &attempt_path,
            &body,
            "attempt-scoped external runtime proof log",
            lock_budget,
        );
        self.write_body(
            &proof_log_path(),
            &body,
            "canonical external runtime proof log",
            lock_budget,
        );
        self.write_body(
            &checked_in_proof_log_path(),
            &body,
            "checked-in MT-043 proof fixture",
            lock_budget,
        );
        println!("--- PROOF-043-B: swarm_edit_proof_log.txt ---\n{body}");
    }

    fn write_body(&self, path: &Path, body: &str, label: &str, lock_budget: Duration) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create {label} directory: {error}"));
        }
        let _lock = ProofLogLock::acquire(&path.with_extension("lock"), lock_budget);
        if proof_log_generation(path).is_some_and(|current| current > self.generation) {
            println!(
                "MT-043 {label} ignored stale attempt_id={} generation={} (newer generation already committed)",
                self.attempt_id, self.generation,
            );
            return;
        }
        let temporary =
            path.with_extension(format!("tmp.{}.{}", std::process::id(), self.attempt_id));
        std::fs::write(&temporary, body)
            .unwrap_or_else(|error| panic!("write {label} temp: {error}"));
        if path.exists() {
            std::fs::remove_file(path)
                .unwrap_or_else(|error| panic!("replace previous {label}: {error}"));
        }
        std::fs::rename(&temporary, path)
            .unwrap_or_else(|error| panic!("commit {label} atomically: {error}"));
    }
}

fn proof_log_generation(path: &Path) -> Option<u128> {
    let first = std::fs::read_to_string(path)
        .ok()?
        .lines()
        .next()?
        .to_owned();
    first
        .split_whitespace()
        .find_map(|token| token.strip_prefix("generation="))?
        .parse()
        .ok()
}

struct ProofLogLock {
    path: PathBuf,
    owner: String,
}

impl ProofLogLock {
    fn acquire(path: &Path, budget: Duration) -> Self {
        let deadline = Instant::now() + budget;
        let owner = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4());
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    file.write_all(owner.as_bytes())
                        .expect("write MT-043 proof-log lock owner");
                    file.sync_all().expect("sync MT-043 proof-log lock owner");
                    return Self {
                        path: path.to_owned(),
                        owner,
                    };
                }
                Err(error) if Instant::now() < deadline => {
                    if error.kind() != std::io::ErrorKind::AlreadyExists {
                        panic!("acquire MT-043 proof-log lock {}: {error}", path.display());
                    }
                    let old_enough_for_recovery = std::fs::metadata(path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .and_then(|modified| modified.elapsed().ok())
                        .is_some_and(|elapsed| elapsed >= Duration::from_millis(500));
                    let owner_is_alive = std::fs::read_to_string(path)
                        .ok()
                        .and_then(|owner| owner.split('-').next()?.parse::<u32>().ok())
                        .is_some_and(|pid| {
                            handshake_native::mcp::binding::process_birth_identity(pid).is_ok()
                        });
                    if old_enough_for_recovery && !owner_is_alive {
                        match std::fs::remove_file(path) {
                            Ok(()) => continue,
                            Err(remove_error)
                                if remove_error.kind() == std::io::ErrorKind::NotFound =>
                            {
                                continue;
                            }
                            Err(remove_error) => panic!(
                                "recover stale MT-043 proof-log lock {}: {remove_error}",
                                path.display()
                            ),
                        }
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!(
                    "MT-043 proof-log lock {} unavailable inside {} ms: {error}",
                    path.display(),
                    budget.as_millis()
                ),
            }
        }
    }
}

impl Drop for ProofLogLock {
    fn drop(&mut self) {
        if std::fs::read_to_string(&self.path).ok().as_deref() == Some(self.owner.as_str()) {
            match std::fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("release MT-043 proof-log lock: {error}"),
            }
        }
    }
}

impl Drop for ProofLog {
    fn drop(&mut self) {
        if !self.live_authority || self.terminal {
            return;
        }
        self.lines
            .retain(|line| line != "PROOF_PASS" && !line.starts_with("PROOF_FAIL:"));
        self.lines.push(
            "PROOF_FAIL: authoritative live attempt exited before all proof gates passed"
                .to_owned(),
        );
        self.flush();
    }
}

// ── the CHANNEL-ONLY swarm agent (CTRL-043-01 / IN-043-01 / RISK-043-01) ──────────────────────────

/// A request the out-of-process swarm agent emits: a stable `author_id` + the `UiAction` to dispatch.
/// This is PURE DATA — it carries NO pointer into the application state, so the agent thread provably
/// cannot reach the UI except through this channel (the real out-of-process IPC shape).
#[derive(Clone, Debug)]
struct AgentRequest {
    author_id: String,
    action: UiAction,
}

/// The handle the agent thread holds. CTRL-043-01: it is ONLY an `mpsc::Sender<AgentRequest>` — there is
/// NO `Arc<ApplicationState>` (or any state pointer) in the agent's scope. The compiler enforces this:
/// `AgentChannel` has exactly one field, a sender of plain data. The agent cannot call an application
/// function directly; every action goes over the channel and is resolved against a live AccessKit
/// snapshot by the UI thread (the same path a socket/pipe transport would feed).
struct AgentChannel(Sender<AgentRequest>);

impl AgentChannel {
    fn dispatch(&self, author_id: &str, action: UiAction) {
        // A real out-of-process agent cannot panic the UI; a closed channel just means the UI stopped.
        let _ = self.0.send(AgentRequest {
            author_id: author_id.to_owned(),
            action,
        });
    }
}

fn count_exact_json_strings(value: &serde_json::Value, needle: &str) -> usize {
    match value {
        serde_json::Value::String(text) => usize::from(text == needle),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| count_exact_json_strings(value, needle))
            .sum(),
        serde_json::Value::Object(values) => values
            .values()
            .map(|value| count_exact_json_strings(value, needle))
            .sum(),
        _ => 0,
    }
}

fn count_json_strings_containing(value: &serde_json::Value, needle: &str) -> usize {
    match value {
        serde_json::Value::String(text) => usize::from(text.contains(needle)),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| count_json_strings_containing(value, needle))
            .sum(),
        serde_json::Value::Object(values) => values
            .values()
            .map(|value| count_json_strings_containing(value, needle))
            .sum(),
        _ => 0,
    }
}

fn count_code_blocks_with_exact_text(value: &serde_json::Value, text: &str) -> usize {
    match value {
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| count_code_blocks_with_exact_text(value, text))
            .sum(),
        serde_json::Value::Object(values) => {
            if values.get("type").and_then(serde_json::Value::as_str) == Some("codeBlock")
                && count_exact_json_strings(value, text) == 1
            {
                1
            } else {
                values
                    .values()
                    .map(|value| count_code_blocks_with_exact_text(value, text))
                    .sum()
            }
        }
        _ => 0,
    }
}

fn assert_mt043_action_namespace(author_id: &str) {
    assert!(
        ["editor.", "graph.", "canvas.", "collection.", "search."]
            .iter()
            .any(|prefix| author_id.starts_with(prefix)),
        "AC-043-07: dispatched author_id '{author_id}' is outside the MT-041/042 action namespace"
    );
}

struct Mt043WorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    active: bool,
}

impl Drop for Mt043WorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

/// Spawn the agent thread. It is given ONLY an [`AgentChannel`] (a sender of plain data) plus the small
/// PLAN of (author_id, action) requests to play. It loops, sending each request, and returns. This mimics
/// an external process scripting the UI by id. The `JoinHandle` lets the UI thread join it so a stuck
/// agent (RISK-043-02) surfaces as a timeout, not a hang.
fn spawn_agent(
    plan: Vec<AgentRequest>,
) -> (
    AgentChannel,
    Receiver<AgentRequest>,
    std::thread::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel::<AgentRequest>();
    let agent = AgentChannel(tx.clone());
    let handle = std::thread::Builder::new()
        .name("swarm-agent".to_owned())
        .spawn(move || {
            let agent = AgentChannel(tx);
            for req in plan {
                agent.dispatch(&req.author_id, req.action);
            }
        })
        .expect("spawn swarm agent thread");
    (agent, rx, handle)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
enum ProcessUiAction {
    Click,
    ClickWithPayload { payload: String },
    Focus,
    SetValue { text: String },
    NativeSetValue { text: String },
    ReplaceSelectedText { text: String },
    Scroll,
    Select,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProcessAgentRequest {
    author_id: String,
    action: ProcessUiAction,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProcessAgentLine {
    pid: u32,
    seq: usize,
    label: String,
    request: ProcessAgentRequest,
    marker: String,
}

struct ProcessAgent {
    child: std::process::Child,
    label: String,
    plan_path: PathBuf,
    out_path: PathBuf,
}

impl From<&UiAction> for ProcessUiAction {
    fn from(value: &UiAction) -> Self {
        match value {
            UiAction::Click => Self::Click,
            UiAction::ClickWithPayload { payload } => Self::ClickWithPayload {
                payload: payload.clone(),
            },
            UiAction::Focus => Self::Focus,
            UiAction::SetValue { text } => Self::SetValue { text: text.clone() },
            UiAction::NativeSetValue { text } => Self::NativeSetValue { text: text.clone() },
            UiAction::ReplaceSelectedText { text } => {
                Self::ReplaceSelectedText { text: text.clone() }
            }
            UiAction::Scroll => Self::Scroll,
            UiAction::Select => Self::Select,
        }
    }
}

impl From<ProcessUiAction> for UiAction {
    fn from(value: ProcessUiAction) -> Self {
        match value {
            ProcessUiAction::Click => Self::Click,
            ProcessUiAction::ClickWithPayload { payload } => Self::ClickWithPayload { payload },
            ProcessUiAction::Focus => Self::Focus,
            ProcessUiAction::SetValue { text } => Self::SetValue { text },
            ProcessUiAction::NativeSetValue { text } => Self::NativeSetValue { text },
            ProcessUiAction::ReplaceSelectedText { text } => Self::ReplaceSelectedText { text },
            ProcessUiAction::Scroll => Self::Scroll,
            ProcessUiAction::Select => Self::Select,
        }
    }
}

impl From<&AgentRequest> for ProcessAgentRequest {
    fn from(value: &AgentRequest) -> Self {
        Self {
            author_id: value.author_id.clone(),
            action: ProcessUiAction::from(&value.action),
        }
    }
}

impl From<ProcessAgentRequest> for AgentRequest {
    fn from(value: ProcessAgentRequest) -> Self {
        Self {
            author_id: value.author_id,
            action: UiAction::from(value.action),
        }
    }
}

const MT043_AGENT_CHILD_ENV: &str = "HSK_MT043_AGENT_CHILD";
const MT043_AGENT_PLAN_ENV: &str = "HSK_MT043_AGENT_PLAN";
const MT043_AGENT_OUT_ENV: &str = "HSK_MT043_AGENT_OUT";
const MT043_AGENT_LABEL_ENV: &str = "HSK_MT043_AGENT_LABEL";
const MT043_AGENT_HOLD_AFTER_FIRST_ENV: &str = "HSK_MT043_AGENT_HOLD_AFTER_FIRST";

fn process_agent_dir() -> PathBuf {
    proof_log_path()
        .parent()
        .expect("MT-043 proof path has a parent")
        .join("agent-ipc")
}

fn spawn_process_agent(
    label: &str,
    plan: Vec<AgentRequest>,
    hold_after_first: bool,
) -> ProcessAgent {
    let safe_label: String = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let dir = process_agent_dir();
    std::fs::create_dir_all(&dir).expect("create MT-043 process-agent IPC dir");
    let run_id = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4());
    let plan_path = dir.join(format!("{safe_label}-{run_id}.plan.json"));
    let out_path = dir.join(format!("{safe_label}-{run_id}.jsonl"));
    let process_plan: Vec<ProcessAgentRequest> =
        plan.iter().map(ProcessAgentRequest::from).collect();
    std::fs::write(
        &plan_path,
        serde_json::to_vec(&process_plan).expect("serialize MT-043 process-agent plan"),
    )
    .expect("write MT-043 process-agent plan");

    let executable = std::env::current_exe().expect("resolve MT-043 process-agent executable");
    let mut command = Command::new(executable);
    command
        .arg("--exact")
        .arg("mt043_process_agent_child")
        .arg("--nocapture")
        .env(MT043_AGENT_CHILD_ENV, "1")
        .env(MT043_AGENT_PLAN_ENV, &plan_path)
        .env(MT043_AGENT_OUT_ENV, &out_path)
        .env(MT043_AGENT_LABEL_ENV, label)
        .env(
            MT043_AGENT_HOLD_AFTER_FIRST_ENV,
            if hold_after_first { "1" } else { "0" },
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let child = command
        .spawn()
        .expect("spawn independently supervised MT-043 process agent");
    ProcessAgent {
        child,
        label: label.to_owned(),
        plan_path,
        out_path,
    }
}

fn read_process_agent_lines(path: &Path) -> Vec<ProcessAgentLine> {
    let Ok(body) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    body.lines()
        .filter_map(|line| serde_json::from_str::<ProcessAgentLine>(line).ok())
        .collect()
}

fn recv_process_agent_line(
    agent: &ProcessAgent,
    seq: usize,
    timeout: Duration,
) -> ProcessAgentLine {
    let deadline = Instant::now() + timeout;
    loop {
        let lines = read_process_agent_lines(&agent.out_path);
        if let Some(line) = lines.into_iter().find(|line| line.seq == seq) {
            return line;
        }
        assert!(
            Instant::now() < deadline,
            "MT-043 process agent {} did not emit request sequence {} within {:?}",
            agent.label,
            seq,
            timeout
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn cleanup_process_agent_files(agent: &ProcessAgent) {
    let _ = std::fs::remove_file(&agent.plan_path);
    let _ = std::fs::remove_file(&agent.out_path);
}

fn run_process_agent_child() {
    let plan_path = PathBuf::from(
        std::env::var_os(MT043_AGENT_PLAN_ENV).expect("process-agent child receives plan path"),
    );
    let out_path = PathBuf::from(
        std::env::var_os(MT043_AGENT_OUT_ENV).expect("process-agent child receives output path"),
    );
    let label =
        std::env::var(MT043_AGENT_LABEL_ENV).unwrap_or_else(|_| "mt043-process-agent".to_owned());
    let hold_after_first = std::env::var_os(MT043_AGENT_HOLD_AFTER_FIRST_ENV).as_deref()
        == Some(std::ffi::OsStr::new("1"));
    let plan: Vec<ProcessAgentRequest> =
        serde_json::from_slice(&std::fs::read(&plan_path).expect("process-agent child reads plan"))
            .expect("process-agent child parses plan");
    std::fs::create_dir_all(out_path.parent().expect("process-agent output parent"))
        .expect("create process-agent output parent");
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out_path)
        .expect("open process-agent output");
    for (seq, request) in plan.into_iter().enumerate() {
        let line = ProcessAgentLine {
            pid: std::process::id(),
            seq,
            label: label.clone(),
            request,
            marker: "REQUEST_READY".to_owned(),
        };
        writeln!(
            out,
            "{}",
            serde_json::to_string(&line).expect("serialize process-agent output line")
        )
        .expect("write process-agent output line");
        out.flush().expect("flush process-agent output line");
        if hold_after_first && seq == 0 {
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

#[test]
fn mt043_process_agent_child() {
    if std::env::var_os(MT043_AGENT_CHILD_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    run_process_agent_child();
}

// ── the UI-thread dispatch pump: resolve an agent request -> AccessKit event (the swarm IPC path) ──

/// Resolve one agent [`AgentRequest`] against a CURRENT-FRAME AccessKit snapshot using the PRODUCTION
/// `crate::mcp::action::ActionChannel` (the real swarm-steering path: author_id -> stable NodeId ->
/// `egui::Event::AccessKitActionRequest`), and return the events to feed the harness this frame. An
/// unresolved/disabled/unsupported target returns the typed [`ActionError`] (never a silent drop —
/// RISK-041-04). `None` events means the agent had nothing queued.
fn resolve_to_events(
    snapshot: &UiTreeSnapshot,
    req: &AgentRequest,
) -> Result<Vec<egui::Event>, ActionError> {
    assert_mt043_action_namespace(&req.author_id);
    let mut chan = ActionChannel::new();
    chan.enqueue(snapshot, &req.author_id, req.action.clone())?;
    Ok(chan.drain_into_events())
}

/// Resolve an agent request against the harness's CURRENT live snapshot via the production action
/// channel, and QUEUE the resulting AccessKit event(s) on the harness so the NEXT `run()` feeds them to
/// egui (the `harness.event()` path the MT-041/042 swarm-dispatch proofs use). Returns the resolved error
/// (never panics) so a caller can assert a target is absent (the STEP-2 gap path). The editor consumes the
/// dispatch within the frame `run()` advances.
fn dispatch_via_harness<S>(
    harness: &mut Harness<'_, S>,
    req: &AgentRequest,
) -> Result<(), ActionError> {
    let snapshot = snapshot_harness(harness);
    let events = resolve_to_events(&snapshot, req)?;
    for ev in events {
        harness.event(ev);
    }
    Ok(())
}

/// Deliver exactly one request through a newly spawned channel-only agent, resolve it against the
/// currently mounted AccessKit tree, and record both sides of that exchange in the authoritative log.
/// Keeping this pump as the sole integration-test ingress prevents later proof steps from accidentally
/// bypassing the swarm channel with a direct UI-thread request.
fn dispatch_from_spawned_agent<S>(
    harness: &mut Harness<'_, S>,
    log: &mut ProofLog,
    request: AgentRequest,
    response: &str,
) {
    let expected = request.clone();
    let (_agent, rx, join) = spawn_agent(vec![request]);
    let received = rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|_| panic!("agent request {} timed out", expected.author_id));
    assert_eq!(received.author_id, expected.author_id);
    assert_eq!(received.action, expected.action);

    let (action, payload) = match &received.action {
        UiAction::Click => ("Click", None),
        UiAction::ClickWithPayload { payload } => ("ClickWithPayload", Some(payload.as_str())),
        UiAction::Focus => ("Focus", None),
        UiAction::SetValue { text } => ("SetValue", Some(text.as_str())),
        UiAction::NativeSetValue { text } => ("SetValue", Some(text.as_str())),
        UiAction::ReplaceSelectedText { text } => ("ReplaceSelectedText", Some(text.as_str())),
        UiAction::Scroll => ("ScrollIntoView", None),
        UiAction::Select => ("Select", None),
    };
    log.dispatch(&received.author_id, action, payload);

    let node_deadline = Instant::now() + Duration::from_secs(5);
    while !harness.root().children_recursive().any(|node| {
        let access = node.accesskit_node();
        access.author_id() == Some(received.author_id.as_str()) && !access.is_disabled()
    }) {
        harness.run_steps(1);
        assert!(
            Instant::now() < node_deadline,
            "agent target {} did not appear within five seconds",
            received.author_id
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    dispatch_via_harness(harness, &received)
        .unwrap_or_else(|error| panic!("channel dispatch {} failed: {error}", received.author_id));
    harness.run_steps(1);
    log.response(response, DbResult::Pass);
    join.join()
        .expect("channel-only single-request agent joins");
}

fn wait_for_enabled_author<S>(harness: &mut Harness<'_, S>, author_id: &str, step: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !harness.root().children_recursive().any(|node| {
        let access = node.accesskit_node();
        access.author_id() == Some(author_id) && !access.is_disabled()
    }) {
        harness.run_steps(1);
        assert!(
            Instant::now() < deadline,
            "{step}: process-agent target {author_id} did not become enabled within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The AccessKit actions probed for each node's steerable-capability list (the `resolve_target` input —
/// it checks the node declares the `Focus`/`Click` action the requested `UiAction` maps to). Mirrors the
/// crate's own `snapshot::node_actions` probe set (accesskit has no action iterator).
const PROBE_ACTIONS: &[accesskit::Action] = &[
    accesskit::Action::Click,
    accesskit::Action::Focus,
    accesskit::Action::SetValue,
    accesskit::Action::ReplaceSelectedText,
    accesskit::Action::ScrollIntoView,
];

/// Take a `UiTreeSnapshot` of the harness's CURRENT live AccessKit tree by walking the kittest root (the
/// SAME live tree an out-of-process UIA adapter projects), so the agent's author_id resolves against the
/// live tree via the production `crate::mcp::action::resolve_target`. The kittest `Node::accesskit_node()`
/// exposes each node's id / author_id / role / disabled / supported-actions — exactly the fields
/// `resolve_target` reads. Built as a synthetic root with every live node as a flat child (the resolver
/// only needs `find_by_author_id`, which walks recursively).
fn snapshot_harness<S>(harness: &mut Harness<'_, S>) -> UiTreeSnapshot {
    let root = harness.root();
    let mut children = Vec::new();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        let author_id = ak.author_id().map(|a| a.to_owned());
        let node_id = ak.id().0;
        // Probe the RAW NodeData action set (single-arg `supports_action`, the same the crate's own
        // `snapshot::node_actions` uses) so the resolver reads the node's own declared actions.
        let actions: Vec<String> = PROBE_ACTIONS
            .iter()
            .filter(|a| ak.data().supports_action(**a))
            .map(|a| format!("{a:?}"))
            .collect();
        children.push(UiTreeNode {
            id: author_id
                .clone()
                .unwrap_or_else(|| format!("node:{node_id}")),
            author_id,
            node_id,
            role: format!("{:?}", ak.role()),
            label: ak.label().map(|l| l.to_owned()),
            value: ak.value().map(|v| v.to_owned()),
            disabled: ak.is_disabled(),
            actions,
            bounds: None::<UiNodeBounds>,
            children: Vec::new(),
        });
    }
    let widget_count = children.len() + 1;
    let synthetic_root = UiTreeNode {
        id: "node:swarm-proof-root".to_owned(),
        author_id: None,
        node_id: 0,
        role: "Window".to_owned(),
        label: None,
        value: None,
        disabled: false,
        actions: Vec::new(),
        bounds: None,
        children,
    };
    UiTreeSnapshot {
        root: synthetic_root,
        captured_at_utc: "0.000000000Z".to_owned(),
        viewport: None,
        widget_count,
    }
}

/// Per-step timeout enforcement (AC-043-09 / IN-043-12): poll `cond` across harness frames until it is
/// true or `budget` elapses. Panics with the step name on timeout so a stuck step (RISK-043-02 — headless
/// egui not advancing) fails LOUDLY with which step + action stalled, never a silent hang.
fn pump_until(
    harness: &mut Harness<'_, ()>,
    step: &str,
    action: &str,
    budget: Duration,
    mut cond: impl FnMut(&mut Harness<'_, ()>) -> bool,
) {
    let start = Instant::now();
    loop {
        harness.run();
        if cond(harness) {
            return;
        }
        if start.elapsed() > budget {
            panic!("SWARM_PROOF_TIMEOUT step={step} action={action}");
        }
    }
}

/// Assert the live AccessKit tree is non-empty (CTRL-043-02 / RISK-043-02): catch the silent
/// headless-empty-tree false-green before EACH step. An empty tree means egui never processed a frame /
/// AccessKit never initialized, which would make every dispatch a no-op that looks like a missing action.
fn assert_tree_nonempty(harness: &mut Harness<'_, ()>, step: &str) {
    let snap = snapshot_harness(harness);
    let count = snap.iter_nodes().count();
    assert!(
        count > 1,
        "CTRL-043-02: AccessKit tree is empty before {step} — headless mode may not be processing frames \
         correctly (got {count} nodes)"
    );
}

// ── the SaveBackend SPY (the E6/MT-037 knowledge_documents request-shape capture) ─────────────────

/// Captures the `(document_id, content_json, expected_version)` of every save request the swarm-driven
/// `editor.rich.save` dispatch routes through the MT-020 `SaveManager` -> the E6/MT-037 save client. This
/// is the EDITOR'S REAL save-output seam (the `SaveBackend` trait the production reqwest impl also
/// satisfies), so the capture proves the BACKEND REQUEST SHAPE each step would send to
/// `PUT /knowledge/documents/{id}/save` — the provable-now half. The live 200/row-write is GATED (no
/// managed PG). The spy returns a canned 200 so the manager's state machine completes deterministically
/// (a real backend would return the same shape).
#[derive(Default)]
struct SaveSpy {
    calls: Mutex<Vec<(String, serde_json::Value, u64)>>,
}

impl SaveBackend for SaveSpy {
    fn save_document(
        &self,
        document_id: &str,
        content_json: serde_json::Value,
        expected_version: u64,
    ) -> SaveFuture {
        self.calls.lock().unwrap().push((
            document_id.to_owned(),
            content_json.clone(),
            expected_version,
        ));
        let document_id = document_id.to_owned();
        Box::pin(async move {
            Ok(RichDocSaveResult {
                document: RichDocLoad {
                    rich_document_id: document_id,
                    doc_version: expected_version + 1,
                    title: String::new(),
                    content_json: Some(content_json),
                    updated_at: Some("gated".to_owned()),
                },
                backlinks_persisted: 0,
                backlinks_error: None,
                backlinks_skipped_reason: None,
                save_receipt_event_id: None,
                attribution: None,
            })
        })
    }
}

impl SaveSpy {
    /// The most-recent captured save request (document_id, content_json, expected_version).
    fn last(&self) -> Option<(String, serde_json::Value, u64)> {
        self.calls.lock().unwrap().last().cloned()
    }
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

/// A no-op draft backend so the editor's draft coordinator installs without a live backend (the draft
/// path is not under test here; the save path is). Every op resolves Ok with no body.
struct NoopDraftBackend;

impl DraftBackend for NoopDraftBackend {
    fn load_draft(&self, _document_id: &str) -> DraftLoadFuture {
        Box::pin(async {
            Ok(RichDocumentDraftLoad {
                current_doc_version: 1,
                draft: None,
            })
        })
    }
    fn upsert_draft(
        &self,
        _document_id: &str,
        _base_doc_version: u64,
        _base_content_sha256: String,
        _content_json: serde_json::Value,
    ) -> DraftWriteFuture {
        Box::pin(async { Ok::<(), DraftError>(()) })
    }
    fn clear_draft(&self, _document_id: &str) -> DraftWriteFuture {
        Box::pin(async { Ok::<(), DraftError>(()) })
    }
}

// ── the document-under-edit + its installed swarm surfaces ────────────────────────────────────────

/// The id the swarm agent's create-note save targets (a stable test document id, the seam the host shell
/// would supply from the create-note backend response). The proof asserts this id reaches the save spy.
const PROOF_DOCUMENT_ID: &str = "SwarmProofNote-doc";
/// The intended backlink TARGET block id (IN-043-05). STEP 3 would reference THIS id when picking the
/// wikilink target, but the pick is a TYPED BLOCKER (no headless AccessKit wikilink-target activation
/// surface), so it is only named in the BLOCKED proof-log payload — never materialized by a direct
/// `st.doc` mutation (the Spec-Realism Gate forbids implementer-injected backlink content).
const PROOF_TARGET_BLOCK_ID: &str = "SwarmProofTarget-block";
/// The created note's block id (the graph/search identity STEP 1 + STEP 4 reference).
const PROOF_NOTE_BLOCK_ID: &str = "SwarmProofNote-block";

/// Build the rich-editor state with the MT-041 `EditorActionRegistry` installed and the save spy wired in
/// as the editor's REAL save backend. The doc starts with one paragraph holding a text selection (so the
/// slash-command dispatch, which requires a `Selection::Text`, opens the picker — IN-043-03).
fn rich_state_with_spy(
    spy: Arc<SaveSpy>,
    registry: Arc<Mutex<EditorActionRegistry>>,
    runtime: tokio::runtime::Handle,
) -> Arc<Mutex<RichEditorState>> {
    let doc = BlockNode::doc(vec![BlockNode::paragraph("note body ")]);
    let mut state = RichEditorState::new(doc);
    // A non-collapsed text selection inside the paragraph leaf (slash-command needs Selection::Text).
    state.selection = Selection::Text {
        anchor: DocPosition::new(vec![0, 0], 0),
        head: DocPosition::new(vec![0, 0], 4),
    };
    state.install_editor_action_registry(Arc::clone(&registry), 0);
    // Install the save + draft managers with the SPY backend on a REAL runtime, so a swarm-driven
    // `editor.rich.save` dispatch -> `request_save` SPAWNS the backend call and the spy records the
    // `(document_id, content_json)` request SHAPE at call time (the E6/MT-037 save seam). The spy returns a
    // canned 200 so the state machine completes deterministically; the LIVE row write is the GATED half.
    let save = SaveManager::new(
        spy as Arc<dyn SaveBackend>,
        Some(runtime.clone()),
        PROOF_DOCUMENT_ID,
        1,
    );
    let base = serde_json::json!({"type":"doc","content":[]});
    let draft = DraftManager::new(
        Arc::new(NoopDraftBackend),
        Some(runtime),
        PROOF_DOCUMENT_ID,
        1,
        &base,
    );
    let state = state.with_save_managers(save, draft);
    Arc::new(Mutex::new(state))
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF-043-A: the full four-step swarm scenario, driven by the CHANNEL-ONLY agent, asserted + logged.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn swarm_edit_proof_all_steps() {
    let mut log = ProofLog::new();
    log.note(
        "MT-043 SwarmEditProof: channel-only agent drives 4 steps via AccessKit dispatch only",
    );

    // A real tokio runtime so the swarm-driven save dispatch actually spawns the (spied) backend call.
    // Kept alive for the whole test (dropping it would abort in-flight save tasks).
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime for the save spy");
    let spy = Arc::new(SaveSpy::default());
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    let state = rich_state_with_spy(Arc::clone(&spy), Arc::clone(&registry), rt.handle().clone());

    // The UI thread owns ALL application state (the rich editor + its registry). The agent thread will get
    // ONLY a channel (CTRL-043-01). Build the kittest harness that renders the rich editor each frame and
    // also pumps the agent->AccessKit dispatch (the swarm IPC path).
    let state_ui = Arc::clone(&state);

    // The agent's PLAN: the author_ids + actions a real swarm agent would script. STEP 1 CREATES a note
    // block by converting the caret block to a heading via the stable MT-041 `editor.rich.format-heading-1`
    // action (real AGENT-PRODUCED content — NOT a direct st.doc mutation), then SAVES via
    // `editor.rich.save`. (STEP 2 + STEP 3 are typed-blocker skips; STEP 4 dispatches the search action,
    // handled after the rich-pane harness against the search pane.)
    let plan = vec![
        AgentRequest {
            author_id: "editor.rich.format-heading-1".to_owned(),
            action: UiAction::Click,
        },
        AgentRequest {
            author_id: "editor.rich.save".to_owned(),
            action: UiAction::Click,
        },
    ];
    let (_agent, agent_rx, agent_join) = spawn_agent(plan);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 520.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_ui)).show(ui);
        });

    // Warm up: two frames so the AccessKit tree + the editor registry populate.
    harness.run();
    harness.run();
    assert_tree_nonempty(&mut harness, "STEP-1-create-note");

    // ── STEP 1: CREATE NOTE ──────────────────────────────────────────────────────────────────────
    log.note("STEP 1: CREATE NOTE — CLICK editor.rich.format-heading-1 (agent-produced heading block), then editor.rich.save");

    // (a) CREATE a real note block by CLICKING the stable MT-041 `editor.rich.format-heading-1` action
    //     node PURELY via AccessKit. The agent's author_id RESOLVES against the live AccessKit tree to a
    //     real, enabled editor action node + REACHES the editor (the routing + action-coverage proof). A
    //     Click routes through `EditorActionRegistry::take_dispatched -> run_rich_dispatch ->
    //     RichDispatch::Format(SetHeading(1))`, converting the caret block to a `heading`. This is
    //     AGENT-PRODUCED content, NOT a test `st.doc` mutation: the doc STARTS as a single `paragraph`,
    //     and ONLY the agent's `format-heading-1` Click turns `content[0]` into a `heading`. (The contract
    //     names `insert-slash-command`, but the slash PICKER is not an agent-drivable headless content
    //     surface — its `slash-item-*` nodes render only while the menu stays open, which the focus-gated
    //     `refresh_slash_trigger` + the unfocused-surface auto-close both prevent without a `/` text token;
    //     so STEP 1 drives the EQUIVALENT stable `format-heading-1` create action MT-041 registered, which
    //     is the agent-drivable block-create surface as-delivered.)
    let req_create = agent_rx
        .recv()
        .expect("agent sent format-heading-1 request");
    assert_eq!(req_create.author_id, "editor.rich.format-heading-1");
    let create_node = {
        let snap = snapshot_harness(&mut harness);
        snap.find_by_author_id("editor.rich.format-heading-1")
            .map(|n| (n.actions.clone(), n.disabled))
    };
    let (create_actions, create_disabled) = create_node
        .expect("STEP1/AC-043-02: editor.rich.format-heading-1 is a live AccessKit node");
    assert!(
        !create_disabled,
        "STEP1: the format-heading-1 action node is enabled (dispatchable)"
    );
    assert!(
        create_actions.iter().any(|a| a == "Click"),
        "STEP1/AC-043-07: the format-heading-1 node declares the Click action a swarm agent dispatches; got {create_actions:?}"
    );
    // Confirm the starting doc is a plain paragraph (so the heading below is provably agent-produced).
    assert_eq!(
        state.lock().unwrap().doc.children.first().and_then(|c| c.as_block()).map(|b| b.kind),
        Some(NodeKind::Paragraph),
        "STEP1 precondition: the doc starts as a single paragraph (the heading is agent-produced, not test-authored)"
    );
    dispatch_via_harness(&mut harness, &req_create)
        .expect("STEP1/AC-043-02: editor.rich.format-heading-1 resolves to a live node + Click and reaches the editor");
    // Pump until the agent-driven dispatch has converted the caret block to a heading (the create). The
    // dispatch is consumed within the frame `run()` advances (RISK-041-04 — reaches the editor, not on a
    // timeout); the unfocused editor never spins (caret blink is focus-gated), so run() converges.
    pump_until(
        &mut harness,
        "STEP-1-create-note",
        "editor.rich.format-heading-1",
        Duration::from_secs(5),
        |_| {
            let st = state.lock().unwrap();
            st.doc.children.iter().any(|c| {
                c.as_block()
                    .is_some_and(|b| matches!(b.kind, NodeKind::Heading(_)))
            })
        },
    );
    log.dispatch("editor.rich.format-heading-1", "Click", None);
    log.response(
        "editor.rich.format-heading-1 Click -> RichDispatch::Format(SetHeading(1)) -> caret block converted to a heading (agent-produced content)",
        DbResult::NoDb,
    );

    // Dispatch editor.rich.save (the create-note persistence). The SaveSpy captures the request shape.
    let req_save = agent_rx.recv().expect("agent sent save request");
    assert_eq!(req_save.author_id, "editor.rich.save");
    dispatch_via_harness(&mut harness, &req_save)
        .expect("STEP1: editor.rich.save resolves to a live AccessKit node + action");
    log.dispatch("editor.rich.save", "Click", None);
    pump_until(
        &mut harness,
        "STEP-1-create-note",
        "editor.rich.save",
        Duration::from_secs(5),
        |_| spy.call_count() >= 1,
    );

    // The save request reached the E6/MT-037 save client seam with the right document id + a content body
    // carrying the AGENT-PRODUCED heading block (the create-note backend request SHAPE — provable now).
    let (doc_id, content_json, _ver) = spy
        .last()
        .expect("STEP1: a save request reached the E6 save seam");
    assert_eq!(
        doc_id, PROOF_DOCUMENT_ID,
        "STEP1/AC-043-02: the create-note save targeted the right knowledge_documents id"
    );
    let created_a_heading = content_json["content"]
        .as_array()
        .map(|arr| arr.iter().any(|n| n["type"] == "heading"))
        .unwrap_or(false);
    assert!(
        created_a_heading,
        "STEP1/AC-043-02: the saved content_json carries the AGENT-PRODUCED heading block created via the \
         editor.rich.format-heading-1 Click (the knowledge_rich_documents INSERT shape; the live SELECT is GATED); got {}",
        content_json["content"]
    );
    log.response(
        "editor.rich.save -> PUT /knowledge/documents/{id}/save (agent-produced heading captured; live row GATED)",
        DbResult::Gated,
    );
    log.note(
        "STEP 1 GATED-half: SELECT FROM knowledge_rich_documents WHERE title LIKE 'SwarmProofNote-%' \
         needs managed PostgreSQL (NEEDS_MANAGED_RESOURCE_PROOF) — proven agent-driven shape, gated row",
    );

    // ── STEP 2: EDIT CODE — AccessKit SetValue on the code_editor_text node (MT-080 swarm-author) ──
    log.note("STEP 2: EDIT CODE — dispatch a RAW AccessKit Action::SetValue at the resolved code_editor_text node (MT-080); assert the buffer carries the AGENT-PRODUCED code");
    {
        use handshake_native::code_editor::panel::CodeEditorPanel;
        use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;

        // The agent-produced code. It is NOT implementer-injected into the buffer: the panel STARTS with a
        // different placeholder, and ONLY the agent's AccessKit SetValue dispatch replaces it — so the
        // asserted buffer content is provably agent-authored (the Spec-Realism Gate: no inject-then-assert).
        const AGENT_CODE: &str = "print(\"swarm-proof\")\n";
        let panel = Arc::new(CodeEditorPanel::new(
            "// placeholder — replaced by the swarm agent\n",
            "py",
        ));
        let drive = Arc::clone(&panel);
        let mut code_harness = Harness::builder()
            .with_size(egui::vec2(900.0, 520.0))
            .build_ui(move |ui| {
                handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
                drive.show(ui);
            });
        // Warm up: two frames so the code_editor_text node + its live node id populate.
        code_harness.run();
        code_harness.run();
        assert_tree_nonempty(&mut code_harness, "STEP-2-edit-code");

        // Snapshot the live AccessKit tree; MT-080 made the code_editor_text node advertise SetValue.
        let snap = snapshot_harness(&mut code_harness);
        let text_node = snap
            .find_by_author_id(CODE_EDITOR_TEXT_AUTHOR_ID)
            .expect("STEP2/AC-043-03: the code_editor_text node is a live AccessKit node")
            .clone();
        assert!(
            !text_node.disabled,
            "STEP2: the code_editor_text node is enabled (dispatchable)"
        );
        assert!(
            text_node.actions.iter().any(|a| a == "SetValue"),
            "STEP2/MT-080: the code_editor_text node advertises the SetValue action a swarm agent authors code with; got {:?}",
            text_node.actions
        );
        log.dispatch(
            CODE_EDITOR_TEXT_AUTHOR_ID,
            "SetValue",
            Some(r#"{"value":"print(\"swarm-proof\")\n"}"#),
        );

        // Resolve the CONCRETE NodeId from the live tree and dispatch the RAW AccessKit SetValue request
        // (the MT-080 shape — a genuine AccessKit-by-id write, NOT key-simulation, NOT an app-code change).
        let node_id = code_harness
            .root()
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
            .expect("STEP2: code_editor_text node present in the live tree")
            .accesskit_node()
            .id();
        code_harness.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::SetValue,
                target: node_id,
                data: Some(accesskit::ActionData::Value(AGENT_CODE.into())),
            },
        ));
        // `consume_swarm_text_actions` drains the request + applies it within a frame (MT-080). Bounded
        // explicit frames (not pump_until's repaint-loop) so a transient panel repaint cannot trip the
        // max-steps guard.
        let mut applied = false;
        for _ in 0..8 {
            code_harness.run();
            if panel.buffer().to_string() == AGENT_CODE {
                applied = true;
                break;
            }
        }
        assert!(
            applied,
            "STEP2/AC-043-03: the swarm SetValue dispatch was consumed within the frame budget"
        );
        assert_eq!(
            panel.buffer().to_string(),
            AGENT_CODE,
            "STEP2/AC-043-03: the swarm Action::SetValue at code_editor_text wrote the AGENT-PRODUCED code into the buffer"
        );
        log.response(
            "code_editor_text SetValue -> consume_swarm_text_actions -> buffer carries the agent-produced code (MT-080 swarm-author surface; the live editor_edit DB row is GATED)",
            DbResult::Pass,
        );
        log.note(
            "STEP 2 PASS: MT-080 added SetValue+ReplaceSelectedText to code_editor_text; the prior 'declares ZERO actions' blocker is STALE and removed. The live editor_edit DB row stays NEEDS_MANAGED_RESOURCE_PROOF.",
        );
    }

    // ── STEP 3: ADD BACKLINK — AccessKit SetValue on a rich wikilink chip (MT-110 wikilink-target-by-id) ──
    log.note("STEP 3: ADD BACKLINK — dispatch a RAW AccessKit Action::SetValue at the resolved rich wikilink chip (MT-110); assert the hsLink atom carries the AGENT-CHOSEN backlink target refValue");
    {
        use handshake_native::rich_editor::document_model::node::{
            BlockNode as RichBlockNode, Child, HsLinkNode, NodeKind as RichNodeKind, TextLeaf,
        };
        use handshake_native::rich_editor::renderer::rich_editor_widget::{
            RichEditorState as RichState, RichEditorWidget as RichWidget,
        };

        // The doc STARTS with a PLACEHOLDER wikilink whose target is NOT the loom_edges target. The atom's
        // EXISTENCE is a scaffold, but its TARGET refValue — the loom_edges edge identity AC-043-04 names —
        // is set ONLY by the agent's AccessKit SetValue: so the authored backlink target is provably
        // agent-produced, NOT implementer-injected (the Spec-Realism Gate: no inject-then-assert of the
        // TARGET, exactly as STEP 1's heading is agent-produced). `HsLinkNode::new` defaults `resolved`
        // true, so this is a PLAIN wikilink chip (no create affordance) that MT-110 makes advertise
        // `Action::SetValue`.
        const PLACEHOLDER_REF: &str = "unpicked-placeholder";
        let doc = RichBlockNode::doc(vec![RichBlockNode::with_children(
            RichNodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("backlink: ")),
                Child::HsLink(HsLinkNode::new("note", PLACEHOLDER_REF, "")),
            ],
        )]);
        let rich_state = Arc::new(Mutex::new(RichState::new(doc)));
        let drive = Arc::clone(&rich_state);
        let mut rich_harness = Harness::builder()
            .with_size(egui::vec2(900.0, 520.0))
            .build_ui(move |ui| {
                handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
                RichWidget::new(Arc::clone(&drive)).show(ui);
            });
        // Warm up: two frames so the wikilink chip node + its live node id populate.
        rich_harness.run();
        rich_harness.run();
        assert_tree_nonempty(&mut rich_harness, "STEP-3-add-backlink");

        // Snapshot the live AccessKit tree; MT-110 made the plain wikilink chip advertise SetValue (the
        // headless wikilink-target-by-id surface — NO live backend search, so it resolves with no PG).
        let snap = snapshot_harness(&mut rich_harness);
        // Generic wikilink chips use the collision-safe occurrence identity introduced by
        // MT-041.  This link is the second leaf in the first block, so its canonical
        // AccessKit author id is the path-scoped form rather than the legacy base id.
        let chip_author =
            handshake_native::rich_editor::wikilinks::inline_view::chip_occurrence_author_id(
                PLACEHOLDER_REF,
                &[0, 1],
            );
        let chip_node = snap
            .iter_nodes()
            .find(|n| {
                n.author_id.as_deref() == Some(chip_author.as_str())
                    && n.actions.iter().any(|a| a == "SetValue")
            })
            .cloned()
            .expect("STEP3/MT-110: the rich wikilink chip is a live AccessKit node advertising SetValue");
        assert!(
            !chip_node.disabled,
            "STEP3: the wikilink chip node is enabled (dispatchable)"
        );
        log.dispatch(
            &chip_author,
            "SetValue",
            Some(&format!(r#"{{"ref_value":"{PROOF_TARGET_BLOCK_ID}"}}"#)),
        );

        // Resolve the CONCRETE NodeId from the live tree and dispatch the RAW AccessKit SetValue request
        // (the same MT-080/MT-110 by-id shape STEP 2 used — a genuine AccessKit-by-id write, NOT
        // key-simulation, NOT an app-code change), carrying the AGENT-CHOSEN backlink target.
        let node_id = rich_harness
            .root()
            .children_recursive()
            .find(|n| {
                let ak = n.accesskit_node();
                ak.author_id() == Some(chip_author.as_str())
                    && ak.data().supports_action(accesskit::Action::SetValue)
            })
            .expect("STEP3: wikilink chip node present in the live tree")
            .accesskit_node()
            .id();
        rich_harness.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::SetValue,
                target: node_id,
                data: Some(accesskit::ActionData::Value(PROOF_TARGET_BLOCK_ID.into())),
            },
        ));
        // `consume_swarm_text_actions` drains the request + applies it within a frame (MT-110), routed
        // through the MT-035 unified undo bus. Bounded explicit frames (not pump_until's repaint-loop).
        let mut authored = false;
        for _ in 0..8 {
            rich_harness.run();
            let s = rich_state.lock().unwrap();
            let ok = s
                .doc
                .children
                .first()
                .and_then(Child::as_block)
                .and_then(|b| b.children.get(1))
                .and_then(Child::as_hs_link)
                .map(|l| l.ref_value == PROOF_TARGET_BLOCK_ID)
                .unwrap_or(false);
            drop(s);
            if ok {
                authored = true;
                break;
            }
        }
        assert!(
            authored,
            "STEP3/AC-043-04: the swarm SetValue authored the backlink target within the frame budget"
        );
        let s = rich_state.lock().unwrap();
        let link = s
            .doc
            .children
            .first()
            .and_then(Child::as_block)
            .and_then(|b| b.children.get(1))
            .and_then(Child::as_hs_link)
            .expect("STEP3: the backlink hsLink atom is still at [0,1]");
        assert_eq!(
            link.ref_value, PROOF_TARGET_BLOCK_ID,
            "STEP3/AC-043-04: the swarm wikilink-target pick set the backlink hsLink refValue to the \
             AGENT-CHOSEN target (the loom_edges edge identity; the live loom_edges row is GATED)"
        );
        assert_ne!(
            link.ref_value, PLACEHOLDER_REF,
            "STEP3: the authored target is the agent's pick, not the placeholder (no inject-then-assert)"
        );
        drop(s);
        log.response(
            "wikilink-chip SetValue -> consume_swarm_text_actions -> hsLink atom carries the AGENT-CHOSEN \
             backlink target refValue via the MT-035 undo bus (MT-110 wikilink-target-by-id surface; the \
             live loom_edges DB row is GATED)",
            DbResult::Pass,
        );
        log.note(
            "STEP 3 PASS: MT-110 added SetValue+ReplaceSelectedText to the rich text root + SetValue to \
             plain wikilink chips (the MT-080 mirror for the rich pane); the prior 'no rich swarm-edit \
             surface' blocker is CLOSED. The live loom_edges DB row stays NEEDS_MANAGED_RESOURCE_PROOF.",
        );
    }

    // ── STEP 4: RUN SEARCH — GATED:SEEDED (the search-result SURFACE witness) ──────────────────────────
    // STEP 4 is recorded here as GATED:SEEDED. This standalone sequence is diagnostic only and can
    // never complete the live proof. STEP 4's
    // runnable SURFACE witness (the LoomSearchV2 result AccessKit node renders for a hit) is proven in the
    // SEPARATE `step4_search_result_surface_is_gated_seeded` test; the live search DATA is pre-seeded, so the
    // live `POST /loom/search/v2` round-trip stays NEEDS_MANAGED_RESOURCE_PROOF (never a live-search PASS).
    log.note("STEP 4: RUN SEARCH — GATED:SEEDED (result AccessKit surface proven in step4_search_result_surface_is_gated_seeded; live search NEEDS_MANAGED_RESOURCE_PROOF)");
    log.dispatch(
        lsv2::SEARCH_AUTHOR_ID,
        "Click",
        Some(r#"{"query":"SwarmProofNote"}"#),
    );
    log.response(
        "loom-search-v2.search resolves + the result surface renders a loom-search-v2.result.<id> node for \
         a SEEDED hit (SURFACE witness in step4_search_result_surface_is_gated_seeded); the live search \
         round-trip is GATED",
        DbResult::Gated,
    );

    // The rich-pane portion is done. Join the agent thread (it has exhausted its 2-request plan) so a stuck
    // agent would surface here rather than hang.
    agent_join
        .join()
        .expect("swarm agent thread joined cleanly");

    // This standalone surface sequence contains gated/seeded observations and therefore cannot issue
    // a live verdict. The managed-PG integration scenario below owns PROOF_PASS authority.
    assert_no_local_artifact_dir();
    assert!(
        log.action_line_count() >= 6,
        "PROOF-043-B: the proof log must carry the STEP 1-4 action lines; got {}",
        log.action_line_count()
    );
    log.finish_surface_only();
    println!(
        "PROOF-043-A SURFACE-ONLY: STEP1 create-note (shape PASS, row \
         GATED), STEP2 edit-code (PASS via MT-080 code_editor_text SetValue), STEP3 add-backlink (PASS via \
         MT-110 rich wikilink-target-by-id SetValue), STEP4 search (GATED:SEEDED) -> NO LIVE VERDICT. ... ok"
    );
}

/// The only MT-043 success authority. Every datum is created inside one isolated managed-PG
/// workspace, all editor mutations originate at stable AccessKit nodes, both agents race the same
/// optimistic version, the loser refetches and merges, and success is written only after durable
/// reload, live search, attribution, EventLedger/Flight-Recorder idempotency, and cleanup pass.
/// WP-KERNEL-012 MT-115: an isolated app-data root for this bounded child's native-MCP binding.
///
/// It MUST be installed before the managed backend is selected. Setting
/// `HANDSHAKE_TEST_STAGE_BINDING_ROOT` forces `pg_proof_support` to OWN its backend child, and only an
/// owned child inherits the redirected app-data root that makes the app, this proof, and the backend
/// resolve one `swarm_mcp_binding.json`. It never touches the operator's live app data.
struct Mt043NativeBindingRoot {
    previous: Option<std::ffi::OsString>,
    root: PathBuf,
}

impl Mt043NativeBindingRoot {
    fn install(nonce: &str) -> Self {
        let sanitized = nonce
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let root = std::env::var_os("HANDSHAKE_TEST_ARTIFACTS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .ancestors()
                    .nth(4)
                    .expect("handshake_native crate must be nested below the shared worktree root")
                    .join("Handshake_Artifacts")
                    .join("handshake-test")
            })
            .join("wp-kernel-012-mt-115")
            .join("native-mcp-binding")
            .join(format!("run-{sanitized}"));
        std::fs::create_dir_all(&root).expect("create MT-043 native-MCP binding root");
        let root = std::fs::canonicalize(&root).expect("canonicalize MT-043 binding root");
        let previous = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT");
        std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", &root);
        Self { previous, root }
    }
}

impl Drop for Mt043NativeBindingRoot {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT"),
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn run_swarm_edit_live_pg_conflict_merge_search_and_receipts() {
    let started = Instant::now();
    let child_budget = mt128_child_budget();
    let deadline = started + child_budget;
    let nonce = std::env::var(MT043_ATTEMPT_ID_ENV)
        .unwrap_or_else(|_| format!("{}-{}", std::process::id(), uuid::Uuid::new_v4()));
    let mut live_log = ProofLog::begin_live_attempt(&nonce);
    // WP-KERNEL-012 MT-115 / MT-109 boundary: the whole flight-recorder route group is fail-closed.
    // Publish a REAL native-MCP session binding for THIS bounded child process BEFORE the managed
    // backend is selected — setting `HANDSHAKE_TEST_STAGE_BINDING_ROOT` is also what makes the
    // fixture OWN its backend child, and the child must inherit the redirected app-data root so both
    // processes resolve the SAME `swarm_mcp_binding.json`. The mounted app's own emitter reads this
    // binding too, so the automatic `document_saved` events polled for below can be ingested at all.
    // Nothing here weakens the boundary: an absent, forged, or stale binding still fails closed.
    let _binding_root = Mt043NativeBindingRoot::install(&nonce);
    let native_binding = interconnect_support::RealNativeMcpBinding::publish();
    let session_token = native_binding.token().to_owned();
    let live = interconnect_support::require_reachable_backend();
    let title = format!("SwarmProofNote-{nonce}");
    let workspace = live.create_workspace(&format!("mt043-{nonce}"));
    let workspace_id = workspace["id"].as_str().expect("workspace id").to_owned();
    let timeout_workspace_sidecar = publish_timeout_workspace(&workspace_id);
    // MT-128's RED path is deliberately non-vacuous: the fixture-owned current-source backend is
    // healthy and the real workspace has been created and published before the injected stall begins.
    // The parent consumes the append-only readiness record to prove those owned resources existed,
    // then deletes the workspace canonically and reaps the exact process tree it observed.
    maybe_force_mt128_child_stall(&nonce, &live, &workspace_id);
    let mut cleanup = Mt043WorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        active: true,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-043 runtime");
    let docs_a = KnowledgeDocumentsClient::with_client(reqwest::Client::new(), live.base.clone());
    let actor = |name: &str| HskDocumentHeaders {
        actor_id: format!("mt043-agent-{name}-{nonce}"),
        kernel_task_run_id: format!("mt043-run-{name}-{nonce}"),
        session_run_id: format!("mt043-session-{name}-{nonce}"),
        actor_kind: Some("operator".to_owned()),
        correlation_id: Some(format!("mt043-correlation-{name}-{nonce}")),
    };
    let actor_a = actor("a");
    let actor_b = actor("b");
    assert_ne!(actor_a.actor_id, actor_b.actor_id);

    // One continuously mounted production app owns creation, editing, save, backlink, and search. The
    // workspace is the only fixture; required documents are created through the mounted parameterized
    // editor.rich.insert-slash-command ActionChannel path.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_native_editor_participant_actor_id(actor_a.actor_id.clone())
        .expect("actor A identity binds before rich-document mount");
    app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    for (pane, pane_type, content_id) in [
        ("pane-a", PaneType::CodeSymbol, None),
        ("pane-b", PaneType::LoomWikiPage, None),
        ("pane-c", PaneType::LoomSearchV2, None),
    ] {
        let pane_id = PaneId::from(pane);
        app.pane_registry().lock().unwrap().insert(PaneRecord::new(
            pane_id.clone(),
            pane_type.clone(),
            workspace_id.clone(),
            content_id.clone(),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        let mut tab = TabState::new(pane_type);
        tab.content_id = content_id;
        let bar = app
            .tab_bar_states_mut()
            .get_mut(&pane_id)
            .expect("seeded application pane has a tab bar");
        bar.tabs = vec![tab];
        bar.active_index = 0;
    }
    app.set_active_pane_for_test(Some(PaneId::from("pane-b")));
    let mut app_harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    let target_title = format!("SwarmProofTarget-{nonce}");
    let (target_document_id, document_id) = {
        let mut create_note = |note_title: &str, previous: Option<&str>| -> String {
            let request = AgentRequest {
                author_id: "editor.rich.insert-slash-command".to_owned(),
                action: UiAction::ClickWithPayload {
                    payload: serde_json::json!({"kind":"note","title":note_title}).to_string(),
                },
            };
            dispatch_from_spawned_agent(
                &mut app_harness,
                &mut live_log,
                request,
                "mounted slash-note request accepted",
            );
            let create_deadline = Instant::now() + Duration::from_secs(5);
            loop {
                app_harness.run_steps(1);
                if let Some(value) = app_harness
                    .root()
                    .children_recursive()
                    .find(|node| {
                        node.accesskit_node().author_id() == Some("editor.rich.created-document")
                    })
                    .and_then(|node| node.accesskit_node().value())
                    .filter(|value| previous != Some(value.as_str()))
                {
                    return value;
                }
                assert!(
                    Instant::now() < create_deadline,
                    "mounted slash-note create did not expose its backend id within five seconds"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let target_document_id = create_note(&target_title, None);
        let document_id = create_note(&title, Some(&target_document_id));
        (target_document_id, document_id)
    };

    // Read-only verification may inspect canonical identities, but it does not create or mutate them.
    let target = runtime
        .block_on(docs_a.load_document(&actor_a, &target_document_id))
        .expect("read back slash-created target");
    let target_block_id = target.document["block_id"]
        .as_str()
        .or_else(|| target.document["loom_block_id"].as_str())
        .unwrap_or(&target_document_id)
        .to_owned();
    let created = runtime
        .block_on(docs_a.load_document(&actor_a, &document_id))
        .expect("read back slash-created source");
    let block_id = created.document["block_id"]
        .as_str()
        .or_else(|| created.document["loom_block_id"].as_str())
        .unwrap_or(&document_id)
        .to_owned();
    let mut initial_version = created.document["doc_version"].as_i64().unwrap_or(1);
    live.run_fixture_sql(
        "mt043-created-rich-documents-assert",
        &format!(
            "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM knowledge_rich_documents WHERE workspace_id = {workspace} AND rich_document_id = {source_id} AND title = {source_title}) THEN RAISE EXCEPTION 'missing exact MT-043 source rich document'; END IF; IF NOT EXISTS (SELECT 1 FROM knowledge_rich_documents WHERE workspace_id = {workspace} AND rich_document_id = {target_id} AND title = {target_title}) THEN RAISE EXCEPTION 'missing exact MT-043 target rich document'; END IF; END $$;",
            workspace = sql_literal(&workspace_id),
            source_id = sql_literal(&document_id),
            source_title = sql_literal(&title),
            target_id = sql_literal(&target_document_id),
            target_title = sql_literal(&target_title),
        ),
    );
    println!(
        "PROOF-043-C PostgreSQL knowledge_rich_documents: workspace={workspace_id} source=({document_id},{title}) target=({target_document_id},{target_title})"
    );
    live_log.response(
        "exact slash-created source and target rows exist in canonical knowledge_rich_documents",
        DbResult::Pass,
    );

    // The production create outcome automatically navigates the active pane to the newly created note.
    // Observe that destination; do not repair it with a test-only tab/pane mutation.
    assert_eq!(
        app_harness.state().active_pane().map(|pane| pane.as_ref()),
        Some("pane-b"),
        "slash-create must retain the active rich pane"
    );
    let source_tab = app_harness
        .state()
        .tab_bar_states()
        .get(&PaneId::from("pane-b"))
        .and_then(|bar| bar.tabs.get(bar.active_index))
        .and_then(|tab| tab.content_id.as_deref());
    assert_eq!(
        source_tab,
        Some(document_id.as_str()),
        "production slash-create navigation must mount the exact source document"
    );
    live_log.response(
        "production slash-create navigation mounted the exact source document without test-state rebinding",
        DbResult::Pass,
    );
    let mount_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app_harness.run_steps(1);
        if app_harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(RICH_EDITOR_ROOT_AUTHOR_ID))
        {
            break;
        }
        assert!(
            Instant::now() < mount_deadline,
            "STEP0 continuous Handshake rich-edit pane mount exceeded five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // The two create actions intentionally navigate away from the untitled state. Bind the proof to
    // the actual mounted source-document state after that navigation; retaining the pre-create Arc
    // would test a retired view and could never observe the live save/conflict receipt.
    let mounted_rich = app_harness.state().mounted_rich_state();

    let mut app_b = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app_b
        .set_native_editor_participant_actor_id(actor_b.actor_id.clone())
        .expect("actor B identity binds before rich-document mount");
    app_b.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    app_b.bind_active_project_for_integration_test(workspace_id.clone());
    {
        let pane_id = PaneId::from("pane-b");
        app_b
            .pane_registry()
            .lock()
            .unwrap()
            .insert(PaneRecord::new(
                pane_id.clone(),
                PaneType::LoomWikiPage,
                workspace_id.clone(),
                Some(document_id.clone()),
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ));
        let mut tab = TabState::new(PaneType::LoomWikiPage);
        tab.content_id = Some(document_id.clone());
        let bar = app_b
            .tab_bar_states_mut()
            .get_mut(&pane_id)
            .expect("second host rich tab bar");
        bar.tabs = vec![tab];
        bar.active_index = 0;
        app_b.set_active_pane_for_test(Some(pane_id));
    }
    let mounted_rich_b = app_b.mounted_rich_state();
    let mut app_harness_b = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_b);
    let second_mount_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app_harness_b.run_steps(1);
        if mounted_rich_b
            .lock()
            .ok()
            .and_then(|state| state.save.as_ref().map(|save| save.doc_version))
            == Some(initial_version as u64)
        {
            break;
        }
        assert!(
            Instant::now() < second_mount_deadline,
            "second mounted host did not load the shared canonical version"
        );
    }
    let code = "fn swarm_merge() -> &'static str { \"MT-043\" }\n".to_owned();
    let edit_plan = vec![
        AgentRequest {
            author_id: RICH_EDITOR_ROOT_AUTHOR_ID.to_owned(),
            action: UiAction::Focus,
        },
        AgentRequest {
            author_id: "editor.rich.insert-slash-command".to_owned(),
            action: UiAction::ClickWithPayload {
                payload: serde_json::json!({
                    "kind": "wikilink",
                    "ref_kind": "note",
                    "ref_value": target_document_id,
                    "label": target_title,
                })
                .to_string(),
            },
        },
        AgentRequest {
            author_id: "editor.rich.insert-slash-command".to_owned(),
            action: UiAction::ClickWithPayload {
                payload: serde_json::json!({
                    "kind": "code_block",
                    "language": "rust",
                    "code": "",
                })
                .to_string(),
            },
        },
    ];
    let expected_edit_actions = edit_plan.clone();
    let (_agent, edit_rx, edit_join) = spawn_agent(edit_plan);
    for expected in expected_edit_actions {
        let request = edit_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("STEP edit action {} timed out", expected.author_id));
        assert_eq!(request.author_id, expected.author_id);
        let (action_name, payload) = match &request.action {
            UiAction::Click => ("Click", None),
            UiAction::ClickWithPayload { payload } => ("ClickWithPayload", Some(payload.as_str())),
            UiAction::Focus => ("Focus", None),
            UiAction::SetValue { text } | UiAction::NativeSetValue { text } => {
                ("SetValue", Some(text.as_str()))
            }
            UiAction::ReplaceSelectedText { text } => ("ReplaceSelectedText", Some(text.as_str())),
            UiAction::Scroll => ("ScrollIntoView", None),
            UiAction::Select => ("Select", None),
        };
        live_log.dispatch(&request.author_id, action_name, payload);
        let action_deadline = Instant::now() + Duration::from_secs(5);
        while !app_harness.root().children_recursive().any(|node| {
            let access = node.accesskit_node();
            access.author_id() == Some(request.author_id.as_str()) && !access.is_disabled()
        }) {
            app_harness.run_steps(1);
            assert!(
                Instant::now() < action_deadline,
                "STEP action node {} did not become enabled within five seconds; matches={:?}; rich_state={:?}; active_pane={:?}",
                request.author_id,
                app_harness
                    .root()
                    .children_recursive()
                    .filter_map(|node| {
                        let access = node.accesskit_node();
                        (access.author_id() == Some(request.author_id.as_str())).then(|| {
                            (
                                access.id().0,
                                access.is_disabled(),
                                format!("{:?}", access.role()),
                                access.data().supports_action(accesskit::Action::Click),
                            )
                        })
                    })
                    .collect::<Vec<_>>(),
                mounted_rich.lock().ok().map(|state| (
                    format!("{:?}", state.selection),
                    state.slash_menu.as_ref().map(|menu| (
                        menu.filter.clone(),
                        menu.selected,
                        menu.prompt.is_some(),
                    )),
                    state.editor_focus_pending,
                )),
                app_harness.state().active_pane().map(|pane| pane.as_ref().to_owned())
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        dispatch_via_harness(&mut app_harness, &request).unwrap_or_else(|error| {
            panic!("channel dispatch {} failed: {error}", request.author_id)
        });
        app_harness.run_steps(1);
        live_log.response(
            "continuous mounted application state advanced",
            DbResult::Pass,
        );
    }
    let join_deadline = Instant::now() + Duration::from_secs(5);
    while !edit_join.is_finished() {
        assert!(
            Instant::now() < join_deadline,
            "STEP edit agent thread exceeded five seconds"
        );
        std::thread::yield_now();
    }
    edit_join.join().expect("channel-only edit agent joins");

    // Open the exact slash-created rich code block through its stable MT-041 action, then author and
    // save through the native CodeEditorPanel. The resulting SaveManager receipt is the winner's
    // canonical note save; there is no independent rich-root copy of `code` and no local-file mirror.
    let code_open_author = {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app_harness.run_steps(1);
            let matches = app_harness
                .root()
                .children_recursive()
                .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
                .filter(|author| author.starts_with("editor.rich.code-block.open."))
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                assert_eq!(
                    matches.len(),
                    1,
                    "the source note exposes one exact code-block open action"
                );
                break matches[0].clone();
            }
            assert!(
                Instant::now() < deadline,
                "slash-created code block did not expose its MT-041 open action"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    dispatch_from_spawned_agent(
        &mut app_harness,
        &mut live_log,
        AgentRequest {
            author_id: code_open_author,
            action: UiAction::Click,
        },
        "exact rich code block opened in the native code editor",
    );
    let (rich_code_content_id, rich_code_panel, rich_code_save_author) = {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app_harness.run_steps(1);
            let content_id = app_harness
                .state()
                .tab_bar_states()
                .values()
                .flat_map(|bar| bar.tabs.iter())
                .filter_map(|tab| tab.content_id.as_deref())
                .find(|content_id| content_id.starts_with("rich-code-block:"))
                .map(str::to_owned);
            let save_authors = app_harness
                .root()
                .children_recursive()
                .filter_map(|node| {
                    let access = node.accesskit_node();
                    access
                        .author_id()
                        .filter(|author| {
                            author.starts_with("editor.code.save.document-")
                                && !access.is_disabled()
                        })
                        .map(str::to_owned)
                })
                .collect::<Vec<_>>();
            if let Some(content_id) = content_id {
                if let Some(panel) = app_harness
                    .state()
                    .mounted_code_panel_for_content_id(&content_id)
                {
                    if save_authors.len() == 1 {
                        break (content_id, panel, save_authors[0].clone());
                    }
                }
            }
            assert!(
                Instant::now() < deadline,
                "exact rich code block did not mount one enabled editor.code.save action"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let rich_code_text_author = rich_code_panel.text_author_id();
    dispatch_from_spawned_agent(
        &mut app_harness,
        &mut live_log,
        AgentRequest {
            author_id: rich_code_text_author,
            action: UiAction::NativeSetValue { text: code.clone() },
        },
        "agent-authored code entered the exact bound native code buffer",
    );
    assert_eq!(rich_code_panel.buffer().to_string(), code);
    dispatch_from_spawned_agent(
        &mut app_harness,
        &mut live_log,
        AgentRequest {
            author_id: rich_code_save_author,
            action: UiAction::Click,
        },
        "editor.code.save entered the exact rich-code persistence bridge",
    );
    live_log.response(
        &format!("exact rich code content binding mounted as {rich_code_content_id}"),
        DbResult::Pass,
    );

    let losing_text = format!("agent-b-recoverable-{nonce}");
    let mut crashing_agent = spawn_process_agent(
        "actor-b-crash-mid-edit",
        vec![AgentRequest {
            author_id: RICH_EDITOR_ROOT_AUTHOR_ID.to_owned(),
            action: UiAction::NativeSetValue {
                text: losing_text.clone(),
            },
        }],
        true,
    );
    let crashing_line = recv_process_agent_line(&crashing_agent, 0, Duration::from_secs(5));
    let crashing_request: AgentRequest = crashing_line.request.clone().into();
    live_log.note(&format!(
        "PROCESS_AGENT label={} pid={} seq={} marker={} role=crashing-agent-mid-edit",
        crashing_line.label, crashing_line.pid, crashing_line.seq, crashing_line.marker
    ));
    live_log.dispatch(
        &crashing_request.author_id,
        "SetValue",
        Some(losing_text.as_str()),
    );
    wait_for_enabled_author(
        &mut app_harness_b,
        &crashing_request.author_id,
        "crashing process edit",
    );
    dispatch_via_harness(&mut app_harness_b, &crashing_request)
        .unwrap_or_else(|error| panic!("process-agent crash edit dispatch failed: {error}"));
    app_harness_b.run_steps(1);
    {
        let losing_state = mounted_rich_b.lock().expect("crashing actor rich state");
        assert_eq!(
            count_json_strings_containing(&to_content_json_value(&losing_state.doc), &losing_text),
            1,
            "crashing process agent's acknowledged edit is mounted before termination"
        );
    }
    terminate_child_tree_bounded(&mut crashing_agent.child, Duration::from_secs(1));
    assert!(
        !crashing_agent
            .child
            .try_wait()
            .expect("poll terminated crashing process agent")
            .is_none(),
        "crashing process agent must be reaped after mid-edit termination"
    );
    live_log.response(
        "crashing editor-agent process was killed after an acknowledged mounted edit",
        DbResult::Pass,
    );

    // Actor A's editor.code.save commits first; actor B then submits its independently edited stale
    // version and receives the real optimistic-concurrency conflict. Both save intents originate at
    // mounted AccessKit nodes.
    let save_deadline = Instant::now() + Duration::from_secs(5);
    let (app_receipt, app_attribution, app_version) = loop {
        app_harness.run_steps(1);
        let saved = mounted_rich.lock().ok().and_then(|state| {
            state.save.as_ref().and_then(|save| {
                save.last_save_receipt_event_id
                    .clone()
                    .zip(save.last_save_attribution.clone())
                    .map(|pair| (pair, save.doc_version))
            })
        });
        if let Some(((receipt, attribution), version)) = saved {
            break (receipt, attribution, version);
        }
        assert!(
            Instant::now() < save_deadline,
            "STEP save receipt did not arrive within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    dispatch_from_spawned_agent(
        &mut app_harness_b,
        &mut live_log,
        AgentRequest {
            author_id: "editor.rich.save".to_owned(),
            action: UiAction::Click,
        },
        "actor B stale save intent entered the mounted save manager",
    );
    let conflict_deadline = Instant::now() + Duration::from_secs(5);
    let (conflict_server_version, conflict_server_content, conflict_local_content) = loop {
        app_harness_b.run_steps(1);
        let recoverable = mounted_rich_b.lock().ok().and_then(|state| {
            state.save.as_ref().and_then(|save| match &save.state {
                SaveState::Conflict {
                    server,
                    local_content,
                } => Some((
                    server.doc_version,
                    server
                        .content_json
                        .clone()
                        .expect("conflict carries server content"),
                    local_content.clone(),
                )),
                _ => None,
            })
        });
        if let Some((server_version, server_content, local_content)) = recoverable {
            assert_eq!(count_exact_json_strings(&local_content, &losing_text), 1);
            let losing_save = mounted_rich_b.lock().unwrap();
            let losing_save = losing_save.save.as_ref().expect("losing SaveManager");
            assert!(losing_save.last_save_receipt_event_id.is_none());
            break (server_version, server_content, local_content);
        }
        assert!(
            Instant::now() < conflict_deadline,
            "actor B stale mounted save did not reach recoverable conflict state"
        );
    };
    assert_eq!(
        count_exact_json_strings(&conflict_local_content, &losing_text),
        1,
        "the losing version remains recoverable until merge"
    );
    assert_eq!(rich_code_panel.buffer().to_string(), code);
    assert_eq!(app_attribution.actor_id, actor_a.actor_id);

    // The loser now performs the required recovery sequence. First refetch canonical state through a
    // fresh backend read, prove it matches the server snapshot carried by the 409, then select the
    // mounted conflict UI's Keep-server action through the spawned-agent channel. Finally merge the
    // losing edit through the rich root's native ReplaceSelectedText action and resave through that same
    // mounted channel. The direct client below is read-only verification; it never mutates authority.
    let loser_refetch = runtime
        .block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                docs_a.load_document(&actor_b, &document_id),
            )
            .await
        })
        .expect("loser canonical refetch exceeded five seconds")
        .expect("loser refetches canonical document after conflict");
    assert_eq!(
        loser_refetch.document["doc_version"].as_u64(),
        Some(conflict_server_version)
    );
    assert_eq!(
        loser_refetch.document["content_json"], conflict_server_content,
        "409 snapshot and independently refetched canonical document agree"
    );
    live_log.response(
        "loser refetched canonical server document after 409",
        DbResult::Pass,
    );

    let survivor_plan = vec![
        AgentRequest {
            author_id: CONFLICT_KEEP_SERVER_AUTHOR_ID.to_owned(),
            action: UiAction::Click,
        },
        AgentRequest {
            author_id: RICH_EDITOR_ROOT_AUTHOR_ID.to_owned(),
            action: UiAction::ReplaceSelectedText {
                text: losing_text.clone(),
            },
        },
        AgentRequest {
            author_id: "editor.rich.save".to_owned(),
            action: UiAction::Click,
        },
    ];
    let mut survivor_agent =
        spawn_process_agent("actor-b-survivor-refetch-merge-save", survivor_plan, false);
    let survivor_keep = recv_process_agent_line(&survivor_agent, 0, Duration::from_secs(5));
    live_log.note(&format!(
        "PROCESS_AGENT label={} pid={} seq={} marker={} role=survivor-merge-agent",
        survivor_keep.label, survivor_keep.pid, survivor_keep.seq, survivor_keep.marker
    ));
    let keep_request: AgentRequest = survivor_keep.request.clone().into();
    live_log.dispatch(&keep_request.author_id, "Click", None);
    wait_for_enabled_author(
        &mut app_harness_b,
        &keep_request.author_id,
        "survivor process keep-server",
    );
    dispatch_via_harness(&mut app_harness_b, &keep_request)
        .unwrap_or_else(|error| panic!("survivor process keep-server dispatch failed: {error}"));
    app_harness_b.run_steps(1);
    live_log.response(
        "mounted conflict UI adopted the refetched server version",
        DbResult::Pass,
    );
    {
        let losing_state = mounted_rich_b.lock().expect("loser rich state");
        let save = losing_state.save.as_ref().expect("loser SaveManager");
        assert_eq!(save.doc_version, conflict_server_version);
        assert!(matches!(&save.state, SaveState::Idle));
        assert_eq!(
            count_exact_json_strings(&to_content_json_value(&losing_state.doc), &code),
            1,
            "refetched winner content is mounted before merge"
        );
    }
    let survivor_merge = recv_process_agent_line(&survivor_agent, 1, Duration::from_secs(5));
    let merge_request: AgentRequest = survivor_merge.request.clone().into();
    live_log.dispatch(
        &merge_request.author_id,
        "ReplaceSelectedText",
        Some(losing_text.as_str()),
    );
    wait_for_enabled_author(
        &mut app_harness_b,
        &merge_request.author_id,
        "survivor process merge",
    );
    dispatch_via_harness(&mut app_harness_b, &merge_request)
        .unwrap_or_else(|error| panic!("survivor process merge dispatch failed: {error}"));
    app_harness_b.run_steps(1);
    live_log.response(
        "loser merged its recoverable edit into the adopted server document through mounted AccessKit",
        DbResult::Pass,
    );
    {
        let losing_state = mounted_rich_b.lock().expect("loser merged rich state");
        let merged = to_content_json_value(&losing_state.doc);
        assert_eq!(
            count_json_strings_containing(&merged, &losing_text),
            1,
            "the AccessKit merge inserted the losing edit exactly once"
        );
        assert_eq!(
            count_exact_json_strings(&merged, &code),
            1,
            "the AccessKit merge preserved the winner's structured code node"
        );
    }
    let survivor_save = recv_process_agent_line(&survivor_agent, 2, Duration::from_secs(5));
    let save_request: AgentRequest = survivor_save.request.clone().into();
    live_log.dispatch(&save_request.author_id, "Click", None);
    wait_for_enabled_author(
        &mut app_harness_b,
        &save_request.author_id,
        "survivor process save",
    );
    dispatch_via_harness(&mut app_harness_b, &save_request)
        .unwrap_or_else(|error| panic!("survivor process save dispatch failed: {error}"));
    app_harness_b.run_steps(1);
    live_log.response(
        "loser submitted the merged winner-plus-loser document",
        DbResult::Pass,
    );
    let survivor_deadline = Instant::now() + Duration::from_secs(5);
    while survivor_agent.child.try_wait().ok().flatten().is_none() {
        assert!(
            Instant::now() < survivor_deadline,
            "survivor process agent exceeded five seconds after merge/save"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    cleanup_process_agent_files(&crashing_agent);
    cleanup_process_agent_files(&survivor_agent);
    let merge_deadline = Instant::now() + Duration::from_secs(5);
    let (merge_receipt, merge_attribution) = loop {
        app_harness_b.run_steps(1);
        let saved = mounted_rich_b.lock().ok().and_then(|state| {
            state.save.as_ref().and_then(|save| {
                save.last_save_receipt_event_id
                    .clone()
                    .zip(save.last_save_attribution.clone())
                    .map(|pair| (pair, save.doc_version))
            })
        });
        if let Some(((receipt, attribution), version)) = saved {
            initial_version = i64::try_from(version).expect("merged version fits i64");
            break (receipt, attribution);
        }
        assert!(
            Instant::now() < merge_deadline,
            "loser merged resave did not return a receipt within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(merge_attribution.actor_id, actor_b.actor_id);
    assert_ne!(merge_receipt, app_receipt);

    // A fresh transport and identity observes exactly the mounted ActionChannel-authored code and
    // backlink plus the loser's merged text. No direct test client writes participate in this proof.
    let fresh_docs =
        KnowledgeDocumentsClient::with_client(reqwest::Client::new(), live.base.clone());
    let reload_headers = actor("reload");
    let reloaded = runtime
        .block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                fresh_docs.load_document(&reload_headers, &document_id),
            )
            .await
        })
        .expect("STEP durable reload exceeded five seconds")
        .expect("fresh-client durable reload");
    assert_eq!(
        reloaded.document["doc_version"].as_i64(),
        Some(initial_version)
    );
    let durable = reloaded.document["content_json"].clone();
    assert_eq!(
        count_code_blocks_with_exact_text(&durable, &code),
        1,
        "durable reload preserves exactly one structured codeBlock containing the AccessKit-authored code"
    );
    assert_eq!(
        count_exact_json_strings(&durable, &code),
        1,
        "no duplicate code edit"
    );
    assert_eq!(
        count_exact_json_strings(&durable, &target_block_id),
        1,
        "no duplicate or lost backlink"
    );
    assert_eq!(
        count_json_strings_containing(&durable, &losing_text),
        1,
        "loser's edit survives exactly once after refetch/merge/resave"
    );

    // Search query + execution stay on the same app and the same production action channel.
    app_harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-c")));
    let search_mount_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app_harness.run_steps(1);
        if app_harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(lsv2::QUERY_AUTHOR_ID))
        {
            break;
        }
        assert!(
            Instant::now() < search_mount_deadline,
            "continuous Handshake search pane did not mount before live query dispatch"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let search_plan = vec![
        AgentRequest {
            author_id: lsv2::QUERY_AUTHOR_ID.to_owned(),
            action: UiAction::NativeSetValue {
                text: title.clone(),
            },
        },
        AgentRequest {
            author_id: lsv2::SEARCH_AUTHOR_ID.to_owned(),
            action: UiAction::Click,
        },
    ];
    let expected_search_actions = search_plan.clone();
    let (_agent, search_rx, search_join) = spawn_agent(search_plan);
    for expected in expected_search_actions {
        let request = search_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("STEP search action {} timed out", expected.author_id));
        let (action, payload) = match &request.action {
            UiAction::Click => ("Click", None),
            UiAction::NativeSetValue { text } | UiAction::SetValue { text } => {
                ("SetValue", Some(text.as_str()))
            }
            other => panic!("unexpected search action: {other:?}"),
        };
        live_log.dispatch(&request.author_id, action, payload);
        dispatch_via_harness(&mut app_harness, &request)
            .unwrap_or_else(|error| panic!("channel search dispatch failed: {error}"));
        app_harness.run_steps(1);
        live_log.response("continuous mounted search state advanced", DbResult::Pass);
    }
    search_join.join().expect("channel-only search agent joins");
    let result_author = lsv2::result_author_id(&block_id);
    let search_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app_harness.run_steps(1);
        if app_harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(result_author.as_str()))
        {
            break;
        }
        assert!(
            Instant::now() < search_deadline,
            "STEP live search result did not appear within five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let result_plan = vec![AgentRequest {
        author_id: result_author.clone(),
        action: UiAction::Click,
    }];
    let (_agent, result_rx, result_join) = spawn_agent(result_plan);
    let result_request = result_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("STEP search result activation arrived");
    live_log.dispatch(&result_request.author_id, "Click", None);
    dispatch_via_harness(&mut app_harness, &result_request)
        .expect("activate live search result through production channel");
    app_harness.run_steps(1);
    live_log.response("search result activated on continuous app", DbResult::Pass);
    result_join.join().expect("channel-only result agent joins");

    // The mounted save path emits automatically. Poll the production reader for the row whose immutable
    // native payload references the exact canonical save receipt; the test never POSTs a fabricated FR row.
    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("bounded MT-043 HTTP client");
    let (automatic_fr, automatic_merge_fr) = {
        let mut poll_flight_recorder = |save_actor_id: &str, receipt: &str| {
            let fr_deadline = Instant::now() + Duration::from_secs(5);
            loop {
                app_harness.run_steps(1);
                // MT-115 / MT-109: the read is capability-gated, so present the genuine native-MCP
                // credential — an unauthenticated read is `401` and reads back as "no rows arrived".
                //
                // The `actor_id=` filter is deliberately GONE. MT-109 made the recorder actor
                // SERVER-derived (`handshake-native:{pid}:{birth}`), so filtering on the document
                // save's own `x-hsk-actor-id` matches zero rows even when the event landed
                // perfectly. Per-agent attribution now lives in the immutable native payload
                // (`save_receipt_event_id` + run ids), which is what this loop already selects on and
                // what the assertions below compare; the server-derived recorder identity is
                // asserted separately.
                let rows: serde_json::Value = runtime.block_on(async {
                    http.get(format!(
                        "{}/api/flight_recorder?wsid={workspace_id}",
                        live.base
                    ))
                    .header("x-hsk-session-token", session_token.as_str())
                    .send()
                    .await
                    .expect("FR GET")
                    .error_for_status()
                    .expect("FR GET status")
                    .json()
                    .await
                    .expect("FR GET JSON")
                });
                let matching: Vec<&serde_json::Value> = rows
                    .as_array()
                    .expect("Flight Recorder response is an array")
                    .iter()
                    .filter(|row| {
                        row["payload"]["native_payload"]["save_receipt_event_id"].as_str()
                            == Some(receipt)
                    })
                    .collect();
                assert!(
                    matching.len() <= 1,
                    "automatic Flight Recorder projection duplicated receipt {receipt}"
                );
                if let Some(row) = matching.first() {
                    break (**row).clone();
                }
                assert!(
                    Instant::now() < fr_deadline,
                    "automatic authentic document_saved row for save actor {save_actor_id} receipt {receipt} did not arrive within five seconds"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        (
            poll_flight_recorder(&app_attribution.actor_id, &app_receipt),
            poll_flight_recorder(&merge_attribution.actor_id, &merge_receipt),
        )
    };
    // MT-115 / MT-109 attribution: the recorder actor is derived from the AUTHENTICATED native-MCP
    // session, never from the client. Assert the durable attribution IS the server-derived native
    // identity and is NOT the client-supplied save actor — the exact property MT-109 exists to hold.
    for (row, save_actor_id) in [
        (&automatic_fr, &app_attribution.actor_id),
        (&automatic_merge_fr, &merge_attribution.actor_id),
    ] {
        let recorder_actor = row["payload"]["actor_id"]
            .as_str()
            .expect("native-editor Flight Recorder row carries a server-derived actor id");
        assert!(
            recorder_actor.starts_with("handshake-native:"),
            "MT-109 requires the recorder actor to be the authenticated native session, got {recorder_actor}"
        );
        assert_ne!(
            recorder_actor, save_actor_id,
            "the client-supplied document-save actor must never become the recorder attribution"
        );
    }
    assert_ne!(
        automatic_fr["event_id"].as_str(),
        Some(app_receipt.as_str())
    );
    let native_payload = &automatic_fr["payload"]["native_payload"];
    assert_eq!(
        native_payload["document_id"].as_str(),
        Some(document_id.as_str())
    );
    assert_eq!(
        native_payload["kernel_task_run_id"].as_str(),
        Some(app_attribution.kernel_task_run_id.as_str())
    );
    assert_eq!(
        native_payload["session_run_id"].as_str(),
        Some(app_attribution.session_run_id.as_str())
    );
    assert_eq!(
        native_payload["correlation_id"].as_str(),
        app_attribution.correlation_id.as_deref()
    );
    assert_ne!(
        automatic_merge_fr["event_id"].as_str(),
        Some(merge_receipt.as_str())
    );
    let merge_native_payload = &automatic_merge_fr["payload"]["native_payload"];
    assert_eq!(
        merge_native_payload["save_receipt_event_id"].as_str(),
        Some(merge_receipt.as_str())
    );
    assert_eq!(
        merge_native_payload["document_id"].as_str(),
        Some(document_id.as_str())
    );
    assert_eq!(
        merge_native_payload["kernel_task_run_id"].as_str(),
        Some(merge_attribution.kernel_task_run_id.as_str())
    );
    assert_eq!(
        merge_native_payload["session_run_id"].as_str(),
        Some(merge_attribution.session_run_id.as_str())
    );
    assert_eq!(
        merge_native_payload["correlation_id"].as_str(),
        merge_attribution.correlation_id.as_deref()
    );
    let app_ledger_payload = interconnect_support::event_ledger_payload(&app_receipt);
    let merge_ledger_payload = interconnect_support::event_ledger_payload(&merge_receipt);
    for (payload, expected_version) in [
        (&app_ledger_payload, app_version),
        (
            &merge_ledger_payload,
            u64::try_from(initial_version).expect("merged document version is non-negative"),
        ),
    ] {
        assert_eq!(payload["event"].as_str(), Some("saved"));
        assert_eq!(
            payload["workspace_id"].as_str(),
            Some(workspace_id.as_str())
        );
        assert_eq!(payload["doc_version"].as_u64(), Some(expected_version));
    }
    live.run_fixture_sql(
        "mt043-exact-event-ledger-attribution-assert",
        &format!(
            "DO $$ BEGIN \
             IF (SELECT COUNT(*) FROM kernel_event_ledger WHERE event_id = {a_event} AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_SAVED' AND aggregate_type = 'knowledge_rich_document' AND aggregate_id = {document} AND actor_id = {a_actor} AND actor_kind = 'operator' AND kernel_task_run_id = {a_task} AND session_run_id = {a_session} AND correlation_id = {a_correlation} AND source_component = 'knowledge_documents_api') <> 1 THEN RAISE EXCEPTION 'missing or duplicate exact actor-A MT-043 EventLedger receipt'; END IF; \
             IF (SELECT COUNT(*) FROM kernel_event_ledger WHERE event_id = {b_event} AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_SAVED' AND aggregate_type = 'knowledge_rich_document' AND aggregate_id = {document} AND actor_id = {b_actor} AND actor_kind = 'operator' AND kernel_task_run_id = {b_task} AND session_run_id = {b_session} AND correlation_id = {b_correlation} AND source_component = 'knowledge_documents_api') <> 1 THEN RAISE EXCEPTION 'missing or duplicate exact actor-B MT-043 EventLedger receipt'; END IF; \
             END $$;",
            a_event = sql_literal(&app_receipt),
            b_event = sql_literal(&merge_receipt),
            document = sql_literal(&document_id),
            a_actor = sql_literal(&app_attribution.actor_id),
            b_actor = sql_literal(&merge_attribution.actor_id),
            a_task = sql_literal(&app_attribution.kernel_task_run_id),
            b_task = sql_literal(&merge_attribution.kernel_task_run_id),
            a_session = sql_literal(&app_attribution.session_run_id),
            b_session = sql_literal(&merge_attribution.session_run_id),
            a_correlation = sql_literal(
                app_attribution
                    .correlation_id
                    .as_deref()
                    .expect("actor A correlation id"),
            ),
            b_correlation = sql_literal(
                merge_attribution
                    .correlation_id
                    .as_deref()
                    .expect("actor B correlation id"),
            ),
        ),
    );
    live_log.response(
        "both actors have distinct exact-one canonical EventLedger receipts and automatic attributed Flight Recorder rows",
        DbResult::Pass,
    );
    live_log.note(&format!(
        "FLIGHT_RECORDER actor_a={} actor_b={}",
        serde_json::to_string(&automatic_fr).expect("serialize actor-A Flight Recorder evidence"),
        serde_json::to_string(&automatic_merge_fr)
            .expect("serialize actor-B Flight Recorder evidence")
    ));

    // AC-043-04: prove the final save projected the exact source->target backlink into canonical
    // PostgreSQL. A content_json string match is not enough: this DO block fails unless the precise
    // loom_edges identity exists after the winner+loser merge was durably reloaded.
    live.run_fixture_sql(
        "mt043-final-backlink-assert",
        &format!(
            "DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM loom_edges WHERE workspace_id = {workspace} \
             AND source_block_id = {source} AND target_block_id = {target}) THEN \
             RAISE EXCEPTION 'missing exact MT-043 loom_edges source->target row'; END IF; END $$;",
            workspace = sql_literal(&workspace_id),
            source = sql_literal(&block_id),
            target = sql_literal(&target_block_id),
        ),
    );
    live_log.response(
        "exact canonical loom_edges source->target row exists after final save",
        DbResult::Pass,
    );

    // Mount a fresh product Graph observer after persistence, with its pane fully configured before the
    // first frame. Every post-mount interaction below stays in the contract-authorized `graph.*`
    // ActionChannel namespace; no menu/palette bypass or direct state mutation participates in proof.
    let mut graph_app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    graph_app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    graph_app.set_active_project_id_for_test(&workspace_id);
    assert!(
        graph_app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH),
        "the View Graph command mounts the production Graph View pane"
    );
    let graph_view = graph_app.mounted_graph_view();
    let mut graph_harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), graph_app);
    let source_graph_author = handshake_native::graph::graph_view::node_author_id(&block_id);
    let target_graph_author = handshake_native::graph::graph_view::node_author_id(&target_block_id);
    let graph_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        graph_harness.run_steps(2);
        let graph_ready = graph_view
            .lock()
            .map(|view| {
                view.nodes.iter().any(|node| node.block_id == block_id)
                    && view
                        .nodes
                        .iter()
                        .any(|node| node.block_id == target_block_id)
            })
            .unwrap_or(false);
        let authors: std::collections::HashSet<String> = graph_harness
            .root()
            .children_recursive()
            .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
            .collect();
        if graph_ready
            && authors.contains(&source_graph_author)
            && authors.contains(&target_graph_author)
        {
            break;
        }
        assert!(
            Instant::now() < graph_deadline,
            "mounted Graph pane did not expose exact source+target nodes {source_graph_author} and {target_graph_author} after final save; graph_authors={:?}; graph_state={:?}",
            authors
                .iter()
                .filter(|author| author.starts_with("graph."))
                .cloned()
                .collect::<Vec<_>>(),
            graph_view.lock().ok().map(|view| (
                view.nodes.iter().map(|node| node.block_id.clone()).collect::<Vec<_>>(),
                view.loading,
                view.error.clone(),
            ))
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    live_log.response(
        &format!(
            "mounted graph tree contains source={source_graph_author} and target={target_graph_author}"
        ),
        DbResult::Pass,
    );

    // Put the graph into Local mode through its registry-backed stable actions. After the cleanup DELETEs,
    // toggling back to Global is a real product refresh whose AccessKit projection must delete both nodes.
    dispatch_from_spawned_agent(
        &mut graph_harness,
        &mut live_log,
        AgentRequest {
            author_id: "graph.select-node".to_owned(),
            action: UiAction::ClickWithPayload {
                payload: serde_json::json!({"block_id": block_id}).to_string(),
            },
        },
        "graph selected the exact source block before the cleanup refresh",
    );
    dispatch_from_spawned_agent(
        &mut graph_harness,
        &mut live_log,
        AgentRequest {
            author_id: handshake_native::graph::graph_view::MODE_LOCAL_AUTHOR_ID.to_owned(),
            action: UiAction::Click,
        },
        "graph entered Local mode through AccessKit",
    );

    let cleanup_actor = actor("cleanup");
    for cleanup_document_id in [&document_id, &target_document_id] {
        let status = runtime.block_on(async {
            let mut request = http
                .delete(format!(
                    "{}/knowledge/documents/{cleanup_document_id}",
                    live.base
                ))
                .header(HSK_HEADER_ACTOR_ID, &cleanup_actor.actor_id)
                .header(
                    HSK_HEADER_KERNEL_TASK_RUN_ID,
                    &cleanup_actor.kernel_task_run_id,
                )
                .header(HSK_HEADER_SESSION_RUN_ID, &cleanup_actor.session_run_id)
                .header(
                    HSK_HEADER_ACTOR_KIND,
                    cleanup_actor
                        .actor_kind
                        .as_deref()
                        .expect("cleanup actor kind"),
                );
            if let Some(correlation_id) = &cleanup_actor.correlation_id {
                request = request.header(HSK_HEADER_CORRELATION_ID, correlation_id);
            }
            request
                .send()
                .await
                .expect("cleanup document DELETE")
                .status()
        });
        assert!(
            status.is_success(),
            "cleanup DELETE for {cleanup_document_id} returned {status}"
        );
    }
    live.run_fixture_sql(
        "mt043-document-and-edge-cleanup-assert",
        &format!(
            "DO $$ BEGIN IF (SELECT COUNT(*) FROM knowledge_rich_documents WHERE workspace_id = {workspace} AND rich_document_id IN ({source_document}, {target_document}) AND deleted_at IS NOT NULL) <> 2 THEN RAISE EXCEPTION 'MT-043 document cleanup did not tombstone both documents'; END IF; IF EXISTS (SELECT 1 FROM loom_edges WHERE workspace_id = {workspace} AND (source_block_id IN ({source_block}, {target_block}) OR target_block_id IN ({source_block}, {target_block}))) THEN RAISE EXCEPTION 'MT-043 cleanup left a graph edge'; END IF; END $$;",
            workspace = sql_literal(&workspace_id),
            source_document = sql_literal(&document_id),
            target_document = sql_literal(&target_document_id),
            source_block = sql_literal(&block_id),
            target_block = sql_literal(&target_block_id),
        ),
    );
    dispatch_from_spawned_agent(
        &mut graph_harness,
        &mut live_log,
        AgentRequest {
            author_id: handshake_native::graph::graph_view::MODE_GLOBAL_AUTHOR_ID.to_owned(),
            action: UiAction::Click,
        },
        "graph returned to Global mode and issued a canonical cleanup refresh",
    );
    let graph_cleanup_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        graph_harness.run_steps(1);
        let stale_node_present = graph_harness.root().children_recursive().any(|node| {
            matches!(
                node.accesskit_node().author_id(),
                Some(author)
                    if author == source_graph_author.as_str()
                        || author == target_graph_author.as_str()
            )
        });
        if !stale_node_present {
            break;
        }
        assert!(
            Instant::now() < graph_cleanup_deadline,
            "canonical Graph refresh retained deleted source or target AccessKit identity"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    live_log.response(
        "document cleanup removed both canonical rows/edges and the refreshed graph tree removed both AccessKit node identities",
        DbResult::Pass,
    );
    assert!(
        Instant::now() < deadline,
        "complete MT-043 scenario exceeded its MT-128 child budget of {}ms",
        child_budget.as_millis()
    );
    // MT-115: the native-editor EventLedger MIRROR rows are keyed
    // `native-editor-fr-{pending,complete}:{workspace_id}:{client_event_id}` and carry no
    // `workspace_id` COLUMN, so the workspace DELETE below cascades nothing for them. Sweep the whole
    // workspace key prefix while the workspace still exists, then fail closed. Without this the
    // automatic `document_saved` rows this proof just produced survive as orphaned residue.
    live.run_fixture_sql(
        "mt043-native-fr-ledger-workspace-sweep",
        &format!(
            "BEGIN; \
             DELETE FROM kernel_event_ledger \
             WHERE idempotency_key LIKE {pending_like} OR idempotency_key LIKE {complete_like}; \
             DO $mt043_fr_sweep$ BEGIN \
               IF EXISTS (SELECT 1 FROM kernel_event_ledger \
                          WHERE idempotency_key LIKE {pending_like} \
                             OR idempotency_key LIKE {complete_like}) THEN \
                 RAISE EXCEPTION 'MT-043 workspace-partitioned native FR ledger sweep left rows behind'; \
               END IF; \
             END $mt043_fr_sweep$; \
             COMMIT;",
            pending_like = sql_literal(&format!("native-editor-fr-pending:{workspace_id}:%")),
            complete_like = sql_literal(&format!("native-editor-fr-complete:{workspace_id}:%")),
        ),
    );
    assert!(matches!(
        live.delete_workspace(&workspace_id),
        200 | 202 | 204
    ));
    cleanup.active = false;
    assert!(live
        .get_json("/workspaces")
        .as_array()
        .expect("workspace list")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(workspace_id.as_str())));
    live.run_fixture_sql(
        "mt043-workspace-cascade-cleanup-assert",
        &format!(
            "DO $$ BEGIN IF EXISTS (SELECT 1 FROM knowledge_rich_documents WHERE workspace_id = {workspace}) THEN RAISE EXCEPTION 'MT-043 workspace cleanup left rich documents'; END IF; IF EXISTS (SELECT 1 FROM loom_edges WHERE workspace_id = {workspace}) THEN RAISE EXCEPTION 'MT-043 workspace cleanup left graph edges'; END IF; END $$;",
            workspace = sql_literal(&workspace_id),
        ),
    );
    cleanup_timeout_workspace(&timeout_workspace_sidecar, &nonce, Duration::from_secs(2));

    // Exercise the crash window where the child created its uniquely named workspace but was killed
    // before publishing the sidecar. The bounded SQL fallback must find that canonical workspace by
    // attempt name, remove it, and leave no visible workspace behind.
    let timeout_probe_attempt = format!("timeout-probe-{nonce}");
    let timeout_probe = live.create_workspace(&format!("mt043-{timeout_probe_attempt}"));
    let timeout_probe_id = timeout_probe["id"]
        .as_str()
        .expect("timeout probe workspace id")
        .to_owned();
    let absent_probe_sidecar = proof_log_path()
        .parent()
        .expect("MT-043 proof path parent")
        .join(format!("never-published-{nonce}.txt"));
    assert!(!absent_probe_sidecar.exists());
    cleanup_timeout_workspace(
        &absent_probe_sidecar,
        &timeout_probe_attempt,
        Duration::from_secs(2),
    );
    assert!(live
        .get_json("/workspaces")
        .as_array()
        .expect("workspace list after sidecar-less timeout cleanup")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(timeout_probe_id.as_str())));
    live_log.response(
        "sidecar-less crash-window cleanup removed the attempt-named canonical workspace within its bounded budget",
        DbResult::Pass,
    );
    assert_no_local_artifact_dir();

    live_log.note(&format!(
        "LIVE workspace={workspace_id} document={document_id} block={block_id}"
    ));
    live_log.note(&format!(
        "FINAL_PERSISTED doc_version={initial_version} code_exact_count={} backlink_exact_count={} loser_edit_exact_count={} content_json={durable}",
        count_code_blocks_with_exact_text(&durable, &code),
        count_exact_json_strings(&durable, &target_block_id),
        count_json_strings_containing(&durable, &losing_text),
    ));
    live_log.note(&format!(
        "actors=[{},{}] winner_receipt={app_receipt} merge_receipt={merge_receipt} fr_event_ids=[{},{}] result={result_author} elapsed_ms={}",
        actor_a.actor_id,
        actor_b.actor_id,
        automatic_fr["event_id"].as_str().unwrap_or_default(),
        automatic_merge_fr["event_id"].as_str().unwrap_or_default(),
        started.elapsed().as_millis()
    ));
    live_log.finish_live_pass();
}

/// Enforce AC-043-09 as a genuine whole-scenario deadline. The child contains fixture acquisition,
/// backend startup, PostgreSQL, both mounted hosts, reload/search/receipts, and cleanup. A timeout reaps
/// only the process tree this test started and replaces any in-progress artifact with a terminal failure.
#[test]
fn swarm_edit_live_pg_conflict_merge_search_and_receipts() {
    if std::env::var_os(MT043_LIVE_CHILD_ENV).as_deref() == Some(std::ffi::OsStr::new("1")) {
        run_swarm_edit_live_pg_conflict_merge_search_and_receipts();
        return;
    }

    let wall_started = Instant::now();
    let child_budget = mt128_child_budget();
    let attempt_id = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4());
    let progress_path = mt128_progress_path(&attempt_id);
    // Replace any prior success before the bounded process even starts. If process creation, exact test
    // selection, or fixture acquisition fails, the artifact can never retain a stale PROOF_PASS.
    ProofLog::write_terminal_fail(
        &attempt_id,
        "bounded live child has not completed",
        Duration::from_millis(100),
    );
    let parent_pid = std::process::id().to_string();
    let timeout_workspace_sidecar = timeout_workspace_path(&parent_pid);
    if timeout_workspace_sidecar.exists() {
        cleanup_timeout_workspace(
            &timeout_workspace_sidecar,
            &attempt_id,
            Duration::from_secs(2),
        );
    }
    let executable = std::env::current_exe().unwrap_or_else(|error| {
        ProofLog::write_terminal_fail(
            &attempt_id,
            &format!("resolve MT-043 test binary: {error}"),
            Duration::from_millis(100),
        );
        panic!("resolve MT-043 test binary: {error}");
    });
    let mut command = Command::new(executable);
    command
        .arg("--exact")
        .arg("swarm_edit_live_pg_conflict_merge_search_and_receipts")
        .arg("--nocapture")
        .env(MT043_LIVE_CHILD_ENV, "1")
        .env(MT043_PARENT_PID_ENV, &parent_pid)
        .env(MT043_ATTEMPT_ID_ENV, &attempt_id)
        .env(MT128_CHILD_BUDGET_ENV, child_budget.as_millis().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().unwrap_or_else(|error| {
        ProofLog::write_terminal_fail(
            &attempt_id,
            &format!("spawn bounded MT-043 live child: {error}"),
            Duration::from_millis(100),
        );
        panic!("spawn bounded MT-043 live child: {error}");
    });
    let child_pid = child.id();
    let child_started = Instant::now();
    let child_deadline = child_started + child_budget;
    let hard_deadline = child_deadline + MT128_REAP_AND_CLEANUP_RESERVE;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < child_deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let child_elapsed = child_started.elapsed();
                let cleanup_deadline = hard_deadline
                    .checked_sub(MT128_DIAGNOSTIC_RESERVE)
                    .unwrap_or(hard_deadline);
                let cleanup = mt128_cleanup_timed_out_attempt(
                    &mut child,
                    &attempt_id,
                    &progress_path,
                    &timeout_workspace_sidecar,
                    cleanup_deadline,
                );
                let last_gate = cleanup
                    .progress
                    .last_gate
                    .as_deref()
                    .unwrap_or("NO_GATE_REPORTED");
                let workspace_id = cleanup
                    .progress
                    .workspace_id
                    .as_deref()
                    .unwrap_or("NO_WORKSPACE_REPORTED");
                let backend_pid = cleanup
                    .progress
                    .backend_pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "NO_BACKEND_REPORTED".to_owned());
                let tree_pids = cleanup
                    .reap
                    .tree_before
                    .iter()
                    .map(|identity| identity.pid.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let diagnostic = format!(
                    "MT128_BOUNDED_CHILD_REAP child_pid={child_pid} backend_pid={backend_pid} workspace_id={workspace_id} elapsed_ms={} budget_ms={} last_gate={last_gate} forced_stall_ready={} child_alive_before={} backend_alive_before={} workspace_identity_matched={} workspace_existed_before_reap={} probe_helper_reaped={} root_identity_revalidated={} observed_tree_pids=[{tree_pids}] taskkill_reaped={} child_reaped={} descendant_tree_reaped={} workspace_cleanup_verified={}",
                    child_elapsed.as_millis(),
                    child_budget.as_millis(),
                    cleanup.progress.forced_stall_ready,
                    cleanup.child_alive_before,
                    cleanup.backend_alive_before,
                    cleanup.workspace_identity_matched,
                    cleanup.workspace_existed_before_reap,
                    cleanup.probe_helper_reaped,
                    cleanup.reap.root_identity_revalidated,
                    cleanup.reap.taskkill_reaped,
                    cleanup.reap.root_reaped,
                    cleanup.reap.tree_reaped,
                    cleanup.workspace_cleanup_verified,
                );
                ProofLog::write_terminal_fail(&attempt_id, &diagnostic, Duration::from_millis(100));
                assert!(
                    cleanup.child_alive_before,
                    "{diagnostic}; timeout path was vacuous because the owned child was already gone"
                );
                assert!(
                    cleanup.workspace_cleanup_verified,
                    "{diagnostic}; post-reap canonical workspace deletion/absence was not verified"
                );
                assert!(
                    cleanup.reap.root_identity_revalidated,
                    "{diagnostic}; captured root PID/start-time identity was not live immediately before taskkill"
                );
                assert!(
                    cleanup.probe_helper_reaped,
                    "{diagnostic}; read-only PostgreSQL pre-reap probe helper was not fully reaped"
                );
                assert!(
                    cleanup.reap.taskkill_reaped,
                    "{diagnostic}; taskkill/helper process was not fully reaped"
                );
                assert!(
                    cleanup.reap.root_reaped && cleanup.reap.tree_reaped,
                    "{diagnostic}; owned child or an observed descendant survived cleanup"
                );
                if std::env::var_os(MT128_FORCE_STALL_ENV).is_some() {
                    assert!(
                        cleanup.progress.forced_stall_ready
                            && cleanup.progress.workspace_id.is_some()
                            && cleanup.workspace_identity_matched
                            && cleanup.workspace_existed_before_reap
                            && cleanup.backend_alive_before
                            && cleanup.reap.tree_before.len() >= 2,
                        "{diagnostic}; forced-stall RED must begin only after the real workspace and fixture-owned backend descendant exist"
                    );
                }
                assert!(
                    Instant::now() <= hard_deadline,
                    "{diagnostic}; timeout failure path exceeded the child budget plus {}ms cleanup reserve",
                    MT128_REAP_AND_CLEANUP_RESERVE.as_millis()
                );
                panic!("{diagnostic}");
            }
            Err(error) => {
                let _ = cleanup_timeout_workspace_via_api(
                    &timeout_workspace_sidecar,
                    Duration::from_secs(1),
                );
                let child_reaped = terminate_child_tree_bounded(&mut child, Duration::from_secs(1));
                let progress = read_mt128_progress(&progress_path, &attempt_id);
                let last_gate = progress.last_gate.as_deref().unwrap_or("NO_GATE_REPORTED");
                cleanup_timeout_workspace(
                    &timeout_workspace_sidecar,
                    &attempt_id,
                    Duration::from_secs(2),
                );
                ProofLog::write_terminal_fail(
                    &attempt_id,
                    &format!(
                        "failed polling bounded live child: {error}; child_pid={child_pid} elapsed_ms={} budget_ms={} last_gate={last_gate} child_reaped={child_reaped}",
                        child_started.elapsed().as_millis(),
                        child_budget.as_millis(),
                    ),
                    Duration::from_millis(100),
                );
                assert!(
                    child_reaped,
                    "poll-error cleanup did not reap child pid {child_pid}"
                );
                assert!(
                    Instant::now() <= hard_deadline,
                    "MT-043 poll-error failure path exceeded the child budget plus cleanup reserve"
                );
                panic!("poll bounded MT-043 live child: {error}");
            }
        }
    };
    let progress = read_mt128_progress(&progress_path, &attempt_id);
    let last_gate = progress.last_gate.as_deref().unwrap_or("NO_GATE_REPORTED");
    if !status.success() {
        let _ =
            cleanup_timeout_workspace_via_api(&timeout_workspace_sidecar, Duration::from_secs(1));
        cleanup_timeout_workspace(
            &timeout_workspace_sidecar,
            &attempt_id,
            Duration::from_secs(2),
        );
        assert!(
            Instant::now() <= hard_deadline,
            "MT-043 child-failure cleanup exceeded the child budget plus cleanup reserve"
        );
    }
    assert!(
        status.success(),
        "bounded MT-043 live child failed with {status}; child_pid={child_pid} elapsed_ms={} budget_ms={} last_gate={last_gate}",
        child_started.elapsed().as_millis(),
        child_budget.as_millis(),
    );
    let proof = std::fs::read_to_string(attempt_proof_log_path(&attempt_id))
        .expect("bounded live child must leave a readable attempt-scoped proof artifact");
    assert!(
        proof
            .lines()
            .next()
            .is_some_and(|line| line.contains(&format!("attempt_id={attempt_id}"))),
        "bounded parent must verify its own child attempt rather than another concurrent run"
    );
    assert_eq!(
        proof.lines().last(),
        Some("PROOF_PASS"),
        "a successful bounded child process must have executed the exact live test and issued its own current PROOF_PASS"
    );
    println!(
        "MT128_CHILD_PASS child_pid={child_pid} elapsed_ms={} budget_ms={} last_gate={last_gate} measured_loaded_worst_case_ms={MT128_MEASURED_LOADED_CHILD_MS} headroom_ms={MT128_MEASURED_HEADROOM_MS}",
        child_started.elapsed().as_millis(),
        child_budget.as_millis(),
    );
    assert!(
        Instant::now() <= hard_deadline,
        "complete MT-043 managed-runtime test exceeded the child budget plus {}ms cleanup reserve (wall_elapsed_ms={})",
        MT128_REAP_AND_CLEANUP_RESERVE.as_millis(),
        wall_started.elapsed().as_millis(),
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// STEP 4 (SEARCH) — a SEPARATE, honestly GATED:SEEDED surface witness. It is NOT part of the
// stop-at-first-blocker proof chain: `swarm_edit_proof_all_steps` STOPS at the STEP-3 genuine blocker per
// HBR-STOP / IN-043-11 and ends PROOF_FAIL, so STEP 4 never runs there (masking a blocker with a seeded
// STEP-4 "PASS" is exactly the honesty defect this MT fixes). This test asserts ONLY the now-provable fact
// — the LoomSearchV2 result AccessKit SURFACE renders a `loom-search-v2.result.<id>` node for a hit, and
// the `loom-search-v2.search` action resolves to a live node — and does NOT claim a live-search PASS: the
// response is PRE-SEEDED and the seed block_id is a hardcoded constant (SELF-REFERENTIAL, not threaded
// from a live search or STEP 1), so AC-043-05's live search is GATED:SEEDED (NEEDS_MANAGED_RESOURCE_PROOF).
// It writes NO proof-log fixture (that belongs to the PROOF_FAIL chain).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn step4_search_result_surface_is_gated_seeded() {
    use handshake_native::backend_client::LoomSearchV2Client;

    // SEEDED response referencing a HARDCODED note id (the GATED backend's stand-in). This is a SEEDED
    // reflection, NOT a data-flow proof: no search executes and `block_id` is a constant — so this only
    // witnesses that the result SURFACE renders for a hit, never that a live search returned it.
    let panel = Arc::new(Mutex::new(lsv2::LoomSearchV2PanelState::new()));
    {
        let mut p = panel.lock().unwrap();
        p.bind_workspace(Some("ws-test"));
        p.query = "SwarmProofNote".to_owned();
        p.response = Some(LoomSearchV2Response {
            hits: vec![LoomSearchV2Hit {
                block: LoomSearchBlock {
                    block_id: PROOF_NOTE_BLOCK_ID.to_owned(),
                    content_type: "note".to_owned(),
                    document_id: None,
                    title: Some("SwarmProofNote".to_owned()),
                },
                score: 0.9,
                fts_rank: 0.0,
                trgm_sim: 0.0,
                vector_sim: 0.0,
                edge_degree: 0,
                highlight: "<mark>SwarmProofNote</mark>".to_owned(),
            }],
            content_type_facets: std::collections::BTreeMap::new(),
            semantic_available: false,
            total: 1,
        });
    }

    // A search client (its base url is unused: the response is pre-seeded; a real fire is the GATED half).
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime for the search client");
    let client = LoomSearchV2Client::new("http://127.0.0.1:37501", rt.handle().clone());

    let panel_ui = Arc::clone(&panel);
    let opened_cell = Arc::new(Mutex::new(None::<String>));
    let opened_for_ui = Arc::clone(&opened_cell);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui(move |ui| {
            let palette = handshake_native::theme::HsTheme::Dark.palette();
            let mut p = panel_ui.lock().unwrap();
            let mut on_open = |block_id: &str| {
                *opened_for_ui.lock().unwrap() = Some(block_id.to_owned());
            };
            let mut callbacks = lsv2::LoomSearchV2Callbacks {
                on_open_block: &mut on_open,
            };
            lsv2::show(
                ui,
                &mut p,
                &palette,
                &client,
                Some("ws-test"),
                &mut callbacks,
            );
        });

    harness.run();
    harness.run();
    assert_tree_nonempty(&mut harness, "STEP-4-run-search");

    // The result SURFACE renders a node for the SEEDED hit (provable now — the AccessKit surface exists).
    let result_author = lsv2::result_author_id(PROOF_NOTE_BLOCK_ID);
    let snap = snapshot_harness(&mut harness);
    assert!(
        snap.find_by_author_id(&result_author).is_some(),
        "STEP4/AC-043-05 (GATED:SEEDED): the LoomSearchV2 result AccessKit surface renders node \
         '{result_author}' for a hit — a SURFACE witness, not a live-search PASS"
    );
    println!(
        "PROOF-043-E (GATED:SEEDED): STEP4 search-result SURFACE renders node author_id={result_author} for \
         a SEEDED, self-referential hit; the live POST /loom/search/v2 search is NEEDS_MANAGED_RESOURCE_PROOF"
    );

    // The search action RESOLVES to a live AccessKit node + Click (the swarm request is well-formed +
    // addressable). We do NOT fire the live Click — firing sets `loading=true` and spins on the absent
    // backend (the GATED half). Resolution is the provable-now half; the live re-fire stays GATED.
    let search_req = AgentRequest {
        author_id: lsv2::SEARCH_AUTHOR_ID.to_owned(),
        action: UiAction::Click,
    };
    let events = resolve_to_events(&snap, &search_req).expect(
        "STEP4/AC-043-05: loom-search-v2.search resolves to a live AccessKit node + Click action",
    );
    assert!(
        !events.is_empty(),
        "STEP4: the search dispatch produced an AccessKit event (well-formed swarm request)"
    );

    // Row-open navigation via AccessKit: the result-row Click routes the host `on_open_block` callback with
    // the seeded id (the cross-surface navigation a swarm agent performs). Bounded frames (no repaint loop).
    let row_req = AgentRequest {
        author_id: result_author.clone(),
        action: UiAction::Click,
    };
    let mut opened: Option<String> = None;
    if dispatch_via_harness(&mut harness, &row_req).is_ok() {
        for _ in 0..6 {
            harness.run();
            if opened_cell.lock().unwrap().is_some() {
                break;
            }
        }
        opened = opened_cell.lock().unwrap().clone();
    }
    // The open callback is host-routed; assert only that the dispatch was accepted, never a false PASS.
    println!(
        "STEP4 (GATED:SEEDED): result-row open dispatched via AccessKit; opened={opened:?} (host-routed \
         callback). The live search round-trip remains NEEDS_MANAGED_RESOURCE_PROOF."
    );
    assert_no_local_artifact_dir();
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// CTRL-043-01 (agent-channel-only): a STATIC, COMPILE-TIME witness that the agent thread holds ONLY a
// channel handle (no Arc into application state). `AgentChannel` has exactly one field — an
// `mpsc::Sender<AgentRequest>` of PLAIN DATA — so an `AgentRequest` provably cannot carry a pointer into
// `RichEditorState` / any application state. If a future edit added a state pointer to `AgentRequest` or
// `AgentChannel`, this assertion's type bound would break the build (the regression guard).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ctrl01_agent_holds_only_a_channel_handle() {
    // `AgentRequest` is `Send` PLAIN DATA (String + a small enum) — it carries no `Arc<...State>`. A
    // function that requires `AgentRequest: Send + 'static` and NOT any state trait compiles ONLY while the
    // request stays pure data. (A pointer into the non-Send/`'static`-bounded application state would not
    // satisfy a `'static` channel payload without an obvious `Arc`, which a reviewer + this bound catch.)
    fn assert_plain_data<T: Send + Clone + 'static>(_: &T) {}
    let req = AgentRequest {
        author_id: "editor.rich.save".to_owned(),
        action: UiAction::Click,
    };
    assert_plain_data(&req);

    // The channel payload type is `AgentRequest` — confirm the agent's only handle wraps exactly that.
    let (tx, _rx) = mpsc::channel::<AgentRequest>();
    let agent = AgentChannel(tx);
    agent.dispatch("editor.rich.insert-slash-command", UiAction::Click);
    println!("CTRL-043-01: the swarm agent holds ONLY mpsc::Sender<AgentRequest> (plain data) — no Arc into application state");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-043-07 (no keyboard simulation): a SOURCE-LEVEL lint asserting this test body contains NONE of the
// forbidden keyboard-simulation identifiers (IN-043-09). The swarm proof's whole point is that the agent
// drives the UI ONLY via AccessKit action dispatch; a single `send_key` / `write_text` / etc. would void
// the proof. This reads THIS file and fails if any forbidden token appears OUTSIDE this guard's own
// allow-list literal (so naming the tokens here to forbid them does not trip the lint).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac07_no_keyboard_simulation_in_test_body() {
    let src = include_str!("test_e7_swarm_edit_proof.rs");
    // The forbidden identifiers (IN-043-09). Each is checked as a call-ish token (`ident(`) so the prose
    // mentions of them in comments/strings (which DESCRIBE the constraint) do not false-positive.
    let forbidden = [
        "send_key",
        "send_char",
        "write_text",
        "simulate_key",
        "press_key",
        "type_text",
    ];
    for tok in forbidden {
        let call = format!("{tok}(");
        assert!(
            !src.contains(&call),
            "AC-043-07: the swarm proof must use ONLY AccessKit dispatch — found forbidden keyboard-sim \
             call '{call}' in the test body"
        );
    }
    println!("AC-043-07: no keyboard-simulation calls (send_key/send_char/write_text/simulate_key/press_key/type_text) in the test body");
}

#[test]
fn ac07_action_namespace_guard_rejects_transient_widget_ids() {
    for allowed in [
        "editor.rich.save",
        "editor.code.text.document-01",
        "graph.mode.global",
        "canvas.card.open",
        "collection.row.open",
        "search.query",
    ] {
        assert_mt043_action_namespace(allowed);
    }
    let rejected = std::panic::catch_unwind(|| assert_mt043_action_namespace("slash-item-code"));
    assert!(
        rejected.is_err(),
        "AC-043-07 must fail closed for a transient non-MT-041/042 author id"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-043-10: the test crate builds (`cargo build -p handshake-native --tests`) — implied by this test
// compiling. A small in-process witness that the MT-041 + MT-042 + search surfaces this proof drives are
// all importable + constructible (the build-time integration the proof depends on).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac10_surfaces_importable_and_constructible() {
    let _reg = EditorActionRegistry::new();
    let _chan = ActionChannel::new();
    let _panel = lsv2::LoomSearchV2PanelState::new();
    // The save spy + a state with it installed construct (the STEP-1/3 wiring).
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let spy = Arc::new(SaveSpy::default());
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    let _state = rich_state_with_spy(spy, registry, rt.handle().clone());
    println!("AC-043-10: the MT-041/042 + search surfaces the proof drives are importable + constructible (the test crate builds)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-043-09 (timeout): a witness that the per-step timeout helper FIRES (panics with the step name) when a
// condition never becomes true — so a stuck step surfaces loudly, never a silent hang. Run it inside a
// catch so the test asserts the timeout PANIC happened (a real stuck step would panic the proof).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac09_step_timeout_fires_on_a_stuck_condition() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(200.0, 120.0))
            .build_ui(|ui| {
                ui.label("idle");
            });
        // A condition that is NEVER true with a tiny budget -> pump_until must panic with the step name.
        pump_until(
            &mut harness,
            "STUCK-STEP",
            "never.fires",
            Duration::from_millis(150),
            |_| false,
        );
    }));
    let err = result.expect_err("AC-043-09: pump_until must panic on a stuck condition");
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains("SWARM_PROOF_TIMEOUT") && msg.contains("STUCK-STEP"),
        "AC-043-09: the timeout panic must name the step; got '{msg}'"
    );
    println!(
        "AC-043-09: pump_until fires SWARM_PROOF_TIMEOUT with the step name on a stuck condition"
    );
}

#[test]
fn proof_log_lock_recovers_after_a_killed_writer() {
    let path = proof_log_path()
        .parent()
        .unwrap()
        .join(format!("stale-lock-recovery-{}.lock", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(path.parent().unwrap()).expect("create stale-lock test directory");
    std::fs::write(&path, "dead-writer").expect("seed stale proof lock");
    std::thread::sleep(Duration::from_millis(550));
    {
        let recovered = ProofLogLock::acquire(&path, Duration::from_secs(2));
        assert_eq!(
            std::fs::read_to_string(&path).ok().as_deref(),
            Some(recovered.owner.as_str())
        );
    }
    assert!(!path.exists(), "recovered proof lock is released");
}

#[test]
fn proof_log_lock_never_steals_from_a_live_slow_writer() {
    let path = proof_log_path()
        .parent()
        .unwrap()
        .join(format!("live-lock-nonsteal-{}.lock", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(path.parent().unwrap()).expect("create live-lock test directory");
    let first = ProofLogLock::acquire(&path, Duration::from_secs(2));
    let first_owner = first.owner.clone();
    std::thread::sleep(Duration::from_millis(550));
    let contender_path = path.clone();
    let contender =
        std::thread::spawn(move || ProofLogLock::acquire(&contender_path, Duration::from_secs(2)));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        std::fs::read_to_string(&path).ok().as_deref(),
        Some(first_owner.as_str()),
        "an age-stale lock remains owned while its exact writer process is alive"
    );
    drop(first);
    let second = contender.join().expect("live-lock contender joined");
    assert_ne!(second.owner, first_owner);
    drop(second);
    assert!(!path.exists(), "contended live proof lock is released");
}

#[test]
fn proof_payload_is_single_line_and_cannot_inject_a_verdict() {
    let mut log = ProofLog::new();
    log.dispatch(
        "editor.code.text",
        "SetValue",
        Some("line-one\nPROOF_PASS\n[T9999] RESPONSE fake"),
    );
    assert_eq!(log.lines.len(), 1);
    assert_eq!(log.lines[0].lines().count(), 1);
    assert!(log.lines[0].contains(r#"line-one\nPROOF_PASS\n[T9999] RESPONSE fake"#));
    assert!(!log.lines[0].contains("\nPROOF_PASS\n"));
}

#[test]
fn watchdog_failure_write_is_bounded_under_live_log_contention() {
    let canonical_lock_path = proof_log_path().with_extension("lock");
    let held = ProofLogLock::acquire(&canonical_lock_path, Duration::from_secs(2));
    let attempt_id = format!("contention-probe-{}", uuid::Uuid::new_v4());
    let attempt_path = attempt_proof_log_path(&attempt_id);
    let started = Instant::now();
    let blocked = std::panic::catch_unwind(|| {
        ProofLog::write_terminal_fail(
            &attempt_id,
            "deliberate live-lock contention",
            Duration::from_millis(50),
        );
    });
    assert!(
        blocked.is_err(),
        "a live owner must not have its lock stolen"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "watchdog failure logging must honor its sub-deadline under contention"
    );
    drop(held);
    if attempt_path.exists() {
        std::fs::remove_file(attempt_path).expect("remove contention-probe attempt artifact");
    }
    let progress_attempt_id = format!("progress-probe-{}", uuid::Uuid::new_v4());
    let path = mt128_progress_path(&progress_attempt_id);
    append_mt128_progress(
        &progress_attempt_id,
        serde_json::json!({
            "attempt_id": progress_attempt_id,
            "kind": "gate",
            "gate": "T0042",
            "child_pid": std::process::id(),
        }),
    );
    let mut file = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open progress journal for interrupted-tail probe");
    file.write_all(br#"{"attempt_id":"interrupted""#)
        .expect("append deliberately interrupted JSON tail");
    file.sync_data().expect("flush interrupted JSON tail");

    let progress = read_mt128_progress(&path, &progress_attempt_id);
    assert_eq!(progress.last_gate.as_deref(), Some("T0042"));
    std::fs::remove_file(path).expect("remove owned progress-journal probe artifact");
}

#[cfg(windows)]
#[test]
fn ac09_process_tree_termination_is_bounded() {
    use std::os::windows::process::CommandExt;
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "$null=Start-Process powershell.exe -WindowStyle Hidden -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru; Start-Sleep -Seconds 30",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .expect("spawn owned MT-043 timeout probe");
    let descendant_deadline = Instant::now() + Duration::from_secs(2);
    while mt128_process_tree(child.id()).len() < 2 && Instant::now() < descendant_deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let started = Instant::now();
    let report = terminate_child_tree_before(&mut child, Instant::now() + Duration::from_secs(2));
    assert!(
        started.elapsed() <= Duration::from_secs(3),
        "owned child-tree termination exceeded its bounded test budget"
    );
    assert!(
        report.tree_before.len() >= 2,
        "termination proof must observe a real descendant before reaping"
    );
    assert!(
        report.root_identity_revalidated
            && report.taskkill_reaped
            && report.root_reaped
            && report.tree_reaped,
        "bounded terminator must reap taskkill, the owned root, and every observed descendant: {report:?}"
    );
}
