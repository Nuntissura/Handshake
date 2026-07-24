//! WP-1 MT-012 — F6 (selection audit fires from the UI) + F8 (pane renders
//! fetched ModelLaneMessage transcript rows), proven headlessly through Argus.
//!
//! A recording `OperatorChatBackend` is injected via `HandshakeApp::set_pane_factory`
//! so the pane's real wiring is exercised without a live backend: clicking a model
//! row fires `record_selection` (F6, the previously-dead selection path), and a
//! launch triggers a transcript fetch whose rows the pane RENDERS (F8).

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::operator_chat_pane::{
    model_selection_author_id, session_selection_author_id, transcript_row_author_id_for,
    LaunchCell, ModelsCell, OperatorChatBackend, OperatorChatCloudRow,
    OperatorChatLaunchPaneFactory, OperatorChatLaunchSelection, OperatorChatLaunched,
    OperatorChatModelInventory, OperatorChatModelRow, OperatorChatRoutingAction,
    OperatorChatSessionRow, OperatorChatSubagentRow, RoutingCell, SelectionCell, TranscriptCell,
    TranscriptRow, FOLDER_PICKER_AUTHOR_ID, LAUNCH_AUTHOR_ID, LAUNCH_STATUS_AUTHOR_ID,
    PROMPT_INPUT_AUTHOR_ID, REFRESH_MODELS_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;

/// A recording backend: delivers a fixed inventory + transcript, and records every
/// model selection the pane sends so F6 is provable.
struct RecordingBackend {
    inventory: Mutex<OperatorChatModelInventory>,
    run_id: String,
    transcript: Vec<TranscriptRow>,
    selections: Mutex<Vec<(OperatorChatLaunchSelection, Option<String>)>>,
    launches: Mutex<Vec<(OperatorChatLaunchSelection, String, String, String)>>,
}

impl OperatorChatBackend for RecordingBackend {
    fn fetch_models(&self, cell: ModelsCell) {
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(self.inventory.lock().expect("inventory lock").clone()));
        }
    }

    fn record_selection(
        &self,
        selection: OperatorChatLaunchSelection,
        working_dir: Option<String>,
        cell: SelectionCell,
    ) {
        self.selections
            .lock()
            .expect("selections lock")
            .push((selection, working_dir));
        *cell.lock().expect("selection audit cell") = Some(Ok(()));
    }

    fn launch(
        &self,
        selection: OperatorChatLaunchSelection,
        owner_session_id: &str,
        working_dir: &str,
        prompt: &str,
        cell: LaunchCell,
    ) {
        self.launches.lock().expect("launches lock").push((
            selection,
            owner_session_id.to_string(),
            working_dir.to_string(),
            prompt.to_string(),
        ));
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(OperatorChatLaunched {
                instance_id: "inst-1".to_string(),
                run_id: self.run_id.clone(),
                lane_id: "lane-1".to_string(),
            }));
        }
    }

    fn fetch_transcript(&self, _run_id: &str, cell: TranscriptCell) {
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(self.transcript.clone()));
        }
    }

    fn routing_action(
        &self,
        _action: OperatorChatRoutingAction,
        _request_json: String,
        cell: RoutingCell,
    ) {
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(serde_json::json!({"status": "routed"})));
        }
    }
}

fn ok_app_with(backend: Arc<RecordingBackend>) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_pane_factory(
        PaneType::OperatorChatLaunch,
        Box::new(OperatorChatLaunchPaneFactory::with_backend(
            backend as Arc<dyn OperatorChatBackend>,
        )),
    );
    app
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn live_labels(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().label())
        .collect()
}

fn node_by_author<'a>(
    harness: &'a Harness<'_, HandshakeApp>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| {
            panic!(
                "{author_id} missing from live tree: {:?}",
                live_author_ids(harness)
            )
        })
}

struct RejectingSelectionBackend;

impl OperatorChatBackend for RejectingSelectionBackend {
    fn fetch_models(&self, cell: ModelsCell) {
        *cell.lock().expect("models cell") = Some(Ok(OperatorChatModelInventory {
            inventory_source: "selection-rejection-probe".to_owned(),
            sessions: vec![OperatorChatSessionRow {
                session_id: "session-ready".to_owned(),
                parent_session_id: None,
                label: "Ready session".to_owned(),
                status: "available".to_owned(),
            }],
            local: vec![OperatorChatModelRow {
                model_id: "local-model-rejected".to_owned(),
                display_name: "Rejected local model".to_owned(),
                runtime_binding: "candle".to_owned(),
                ready: true,
            }],
            ..Default::default()
        }));
    }

    fn record_selection(
        &self,
        _selection: OperatorChatLaunchSelection,
        _working_dir: Option<String>,
        cell: SelectionCell,
    ) {
        *cell.lock().expect("selection cell") = Some(Err(
            "500 selection_audit_failed: recorder unavailable".to_owned(),
        ));
    }

    fn launch(
        &self,
        _selection: OperatorChatLaunchSelection,
        _owner_session_id: &str,
        _working_dir: &str,
        _prompt: &str,
        _cell: LaunchCell,
    ) {
        panic!("a rejected selection must never become launchable");
    }

    fn fetch_transcript(&self, _run_id: &str, _cell: TranscriptCell) {}

    fn routing_action(
        &self,
        _action: OperatorChatRoutingAction,
        _request_json: String,
        cell: RoutingCell,
    ) {
        *cell.lock().expect("routing cell") = Some(Ok(serde_json::json!({})));
    }
}

#[test]
fn operator_chat_rejects_model_selection_when_audit_persistence_fails() {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_pane_factory(
        PaneType::OperatorChatLaunch,
        Box::new(OperatorChatLaunchPaneFactory::with_backend(Arc::new(
            RejectingSelectionBackend,
        ))),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Operator Chat").click();
    harness.run();
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();

    let model_author = model_selection_author_id("local", None, "local-model-rejected");
    node_by_author(&harness, &model_author).click_accesskit();
    harness.run();
    harness.run();

    harness.get_by_label(
        "Model selection was not accepted because its audit record failed: 500 selection_audit_failed: recorder unavailable. Retry the selection.",
    );
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "an unaudited model selection must not become launchable"
    );
}

#[test]
fn operator_chat_model_select_fires_audit_and_launch_renders_fetched_transcript() {
    let backend = Arc::new(RecordingBackend {
        inventory: Mutex::new(OperatorChatModelInventory {
            inventory_source: "operator_chat_backend".to_string(),
            sessions: vec![
                OperatorChatSessionRow {
                    session_id: "child-a".to_string(),
                    parent_session_id: Some("root-a".to_string()),
                    label: "child-a / CODER / gpt-test".to_string(),
                    status: "available".to_string(),
                },
                OperatorChatSessionRow {
                    session_id: "paused-child".to_string(),
                    parent_session_id: Some("root-a".to_string()),
                    label: "paused-child / CODER / gpt-test".to_string(),
                    status: "unavailable".to_string(),
                },
            ],
            local: vec![
                OperatorChatModelRow {
                    model_id: "local-model-1".to_string(),
                    display_name: "Local Model 1".to_string(),
                    runtime_binding: "candle".to_string(),
                    ready: true,
                },
                OperatorChatModelRow {
                    model_id: "local-model-unavailable".to_string(),
                    display_name: "Unavailable Local Model".to_string(),
                    runtime_binding: "candle".to_string(),
                    ready: false,
                },
            ],
            cloud_byok: vec![OperatorChatCloudRow {
                provider: "anthropic".to_string(),
                model_id: "claude-sonnet-4".to_string(),
                label: "Anthropic (Claude)".to_string(),
                status: "configured".to_string(),
            }],
            cloud_cli_bridge: vec![OperatorChatCloudRow {
                provider: "codex".to_string(),
                model_id: "gpt-5-codex".to_string(),
                label: "GPT / Codex CLI".to_string(),
                status: "logged_in".to_string(),
            }],
            subagents: vec![OperatorChatSubagentRow {
                role: "subagent_coder".to_string(),
                model_id: "subagent://operator-chat/coder".to_string(),
                label: "Subagent Manager / Coder".to_string(),
                status: "available".to_string(),
            }],
            ..Default::default()
        }),
        run_id: "operator-run-42".to_string(),
        transcript: vec![
            TranscriptRow {
                role: "thinking".to_string(),
                text: "captured-thought-abc123".to_string(),
                message_id: Some("msg-thought-1".to_string()),
                ordered_index: Some(1),
            },
            TranscriptRow {
                role: "text".to_string(),
                text: "captured-answer-def456".to_string(),
                message_id: Some("msg-answer-2".to_string()),
                ordered_index: Some(2),
            },
        ],
        selections: Mutex::new(Vec::new()),
        launches: Mutex::new(Vec::new()),
    });

    let mut harness = Harness::builder().build_state(
        |ctx, a: &mut HandshakeApp| a.ui(ctx),
        ok_app_with(backend.clone()),
    );
    harness.run();

    // Open the pane through the RUN menu leaf.
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Operator Chat").click();
    harness.run();
    harness.run();
    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::OperatorChatLaunch)),
        "Run menu opened a native OperatorChatLaunch tab"
    );

    // Refresh models: the recording backend delivers the inventory; the row renders.
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    let model_author = model_selection_author_id("local", None, "local-model-1");
    let unavailable_model_author =
        model_selection_author_id("local", None, "local-model-unavailable");
    let session_author = session_selection_author_id("child-a");
    let unavailable_session_author = session_selection_author_id("paused-child");
    let cloud_author = model_selection_author_id("cloud", Some("anthropic"), "claude-sonnet-4");
    let cli_author = model_selection_author_id("cli", Some("codex"), "gpt-5-codex");
    let subagent_author = model_selection_author_id(
        "subagent",
        Some("subagent_coder"),
        "subagent://operator-chat/coder",
    );
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| *id == model_author),
        "the enumerated model row renders and is addressable"
    );
    assert!(
        node_by_author(&harness, &unavailable_model_author)
            .accesskit_node()
            .is_disabled(),
        "a not-ready local model row is visible but disabled"
    );
    assert!(
        node_by_author(&harness, &unavailable_session_author)
            .accesskit_node()
            .is_disabled(),
        "an inactive governed session is visible but disabled"
    );
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "launch remains disabled until a governed owner session is selected"
    );
    node_by_author(&harness, &session_author).click_accesskit();
    harness.run();
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| *id == cloud_author)
            && live_author_ids(&harness).iter().any(|id| *id == cli_author)
            && live_author_ids(&harness)
                .iter()
                .any(|id| *id == subagent_author),
        "cloud, CLI, and subagent rows render with scoped author ids"
    );

    // F6: cloud, CLI, and subagent rows send structured context, not just a label.
    node_by_author(&harness, &cloud_author).click_accesskit();
    harness.run();
    node_by_author(&harness, &cli_author).click_accesskit();
    harness.run();
    node_by_author(&harness, &subagent_author).click_accesskit();
    harness.run();
    let selections = backend.selections.lock().expect("selections").clone();
    assert!(
        selections.iter().any(|(selection, _)| {
            selection.lane_kind == "cloud"
                && selection.model_id == "claude-sonnet-4"
                && selection.cloud_provider.as_deref() == Some("anthropic")
        }),
        "cloud selection audit carries lane/model/provider context: {selections:?}"
    );
    assert!(
        selections.iter().any(|(selection, _)| {
            selection.lane_kind == "cli"
                && selection.model_id == "gpt-5-codex"
                && selection.cli_provider.as_deref() == Some("codex")
        }),
        "CLI selection audit carries the exact provider context: {selections:?}"
    );
    assert!(
        selections.iter().any(|(selection, _)| {
            selection.lane_kind == "subagent"
                && selection.model_id == "subagent://operator-chat/coder"
                && selection.cloud_provider.is_none()
                && selection.cli_provider.is_none()
        }),
        "subagent selection audit carries the no-OS lane/model context: {selections:?}"
    );

    // F6: click the model row -> the selection-audit path fires.
    node_by_author(&harness, &model_author).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        backend
            .selections
            .lock()
            .expect("selections")
            .iter()
            .any(|(selection, _)| selection == &OperatorChatLaunchSelection::local("local-model-1")),
        "clicking a local model row fires the selection-decision audit (F6)"
    );

    // Type folder + prompt (in-app AccessKit focus only; HBR-QUIET).
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).type_text("D:/work/repo");
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).type_text("audit the repo");
    harness.run();

    // F8: launch -> the pane fetches + RENDERS the captured transcript rows.
    node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    harness.run();

    let launches = backend.launches.lock().expect("launches");
    assert_eq!(
        launches.as_slice(),
        &[(
            OperatorChatLaunchSelection::local("local-model-1"),
            "child-a".to_string(),
            "D:/work/repo".to_string(),
            "audit the repo".to_string(),
        )],
        "launch carries the structured LOCAL lane and selected governed owner session"
    );
    drop(launches);

    // A refreshed backend inventory is canonical: degrading the selected owner
    // clears it and disables launch instead of retaining a stale Option value.
    {
        let mut inventory = backend.inventory.lock().expect("inventory");
        inventory
            .sessions
            .iter_mut()
            .find(|row| row.session_id == "child-a")
            .expect("child-a inventory row")
            .status = "unavailable".to_string();
    }
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "launch disables when refresh invalidates the selected owner session"
    );
    harness.get_by_label(
        "Inventory refresh cleared selection: selected governed session is no longer available. Select an available row before launch.",
    );

    // Restore the owner, select it again, then degrade the selected local model.
    // The model selection is cleared independently and the visible reason names it.
    {
        let mut inventory = backend.inventory.lock().expect("inventory");
        inventory
            .sessions
            .iter_mut()
            .find(|row| row.session_id == "child-a")
            .expect("child-a inventory row")
            .status = "available".to_string();
    }
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "recovering the backend session row does not resurrect the cleared owner selection"
    );
    node_by_author(&harness, &session_author).click_accesskit();
    harness.run();
    assert!(
        !node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "the still-canonical selected model can be paired with the restored owner"
    );
    {
        let mut inventory = backend.inventory.lock().expect("inventory");
        inventory
            .local
            .iter_mut()
            .find(|row| row.model_id == "local-model-1")
            .expect("selected local model row")
            .ready = false;
    }
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "launch disables when refresh invalidates the selected local model"
    );
    harness.get_by_label(
        "Inventory refresh cleared selection: selected model is no longer ready or available. Select an available row before launch.",
    );
    {
        let mut inventory = backend.inventory.lock().expect("inventory");
        inventory
            .local
            .iter_mut()
            .find(|row| row.model_id == "local-model-1")
            .expect("selected local model row")
            .ready = true;
    }
    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID)
            .accesskit_node()
            .is_disabled(),
        "recovering the backend model row does not resurrect the cleared model selection"
    );

    let transcript_authors = live_author_ids(&harness)
        .into_iter()
        .filter(|id| id.starts_with("operator-chat.transcript."))
        .collect::<Vec<_>>();
    assert!(
        transcript_authors
            .iter()
            .any(|id| *id == transcript_row_author_id_for(0, Some("msg-thought-1"))),
        "fetched transcript rows render with backend-message author ids: {transcript_authors:?}"
    );
    let labels = live_labels(&harness);
    assert!(
        labels.iter().any(|l| l.contains("captured-thought-abc123")),
        "the fetched captured thought row renders in the transcript: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains("captured-answer-def456")),
        "the fetched captured answer row renders in the transcript"
    );
    assert!(
        labels
            .iter()
            .any(|l| l.contains("launched run operator-run-42")),
        "launch status renders outside the transcript with its own author id"
    );
    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == LAUNCH_STATUS_AUTHOR_ID),
        "launch status has a distinct AccessKit target"
    );
    assert!(
        !labels
            .iter()
            .any(|l| l.contains("system: launched run operator-run-42")),
        "launch status is not injected as a synthetic transcript row"
    );
}
