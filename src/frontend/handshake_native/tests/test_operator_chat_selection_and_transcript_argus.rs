//! Component coverage for local pending-profile launch timing and transcript rendering.
//! A recording backend simulates the internal audit before its launch response.
//! These mocks provide no backend admission, process, device or runtime proof.

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::operator_chat_pane::{
    model_selection_author_id, profile_selection_author_id, session_selection_author_id, transcript_row_author_id_for,
    LaunchCell, ModelsCell, OperatorChatBackend, OperatorChatCloudRow,
    OperatorChatLaunchPaneFactory, OperatorChatLaunchSelection, OperatorChatLaunched,
    OperatorChatModelInventory, OperatorChatModelRow, OperatorChatRoutingAction,
    OperatorChatSessionRow, OperatorChatSubagentRow, RoutingCell, SelectionCell, TranscriptCell,
    TranscriptRow, FOLDER_PICKER_AUTHOR_ID, LAUNCH_AUTHOR_ID, LAUNCH_STATUS_AUTHOR_ID,
    PROMPT_INPUT_AUTHOR_ID, REFRESH_MODELS_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;


// Component-only profile fixture. No real artifact, inference or device proof is asserted.
fn component_profile(model_id: &str) -> handshake_native::backend_client::EngineProfileSnapshotV1 {
    let profile = handshake_dexterity::EngineProfile {
        id: format!("component-{model_id}"),
        revision: 1,
        model: handshake_dexterity::Model {
            id: model_id.to_owned(),
            format: handshake_dexterity::ModelFormat::GgufQ4_0,
            content_sha256: "1".repeat(64),
            compiled_context: None,
        },
        engine: handshake_dexterity::Engine::GenieXLlamaCpp,
        device: handshake_dexterity::Device::Cpu,
        preset: handshake_dexterity::Preset {
            name: "component-fixture".to_owned(),
            context_limit: 2048,
            max_tokens: 128,
            sampling: handshake_dexterity::Sampling::LlamaCpp {
                temperature: 0.7, top_k: 40, top_p: 0.95, min_p: 0.05,
            },
        },
    };
    let resolved = handshake_dexterity::Registry::new(vec![profile.clone()])
        .and_then(|registry| registry.resolve(&profile.id))
        .expect("valid component profile");
    handshake_native::backend_client::EngineProfileSnapshotV1 {
        profile, fingerprint: resolved.fingerprint().to_owned(),
    }
}

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
        if selection.lane_kind == "local" {
            // Component simulation of the backend's internal launch-audit boundary.
            self.selections.lock().expect("selections lock")
                .push((selection.clone(), Some(working_dir.to_owned())));
        }
        let engine_profile = selection.engine_profile.clone();
        self.launches.lock().expect("launches lock").push((
            selection,
            owner_session_id.to_string(),
            working_dir.to_string(),
            prompt.to_string(),
        ));
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(OperatorChatLaunched {
                engine_profile,
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
                parent_session_id: Some("component-root".to_owned()),
                label: "Ready session".to_owned(),
                status: "available".to_owned(),
            }],
            local: vec![OperatorChatModelRow {
                model_id: "local-model-rejected".to_owned(),
                engine_profile: Some(component_profile("local-model-rejected")),
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
        selection: OperatorChatLaunchSelection,
        owner_session_id: &str,
        working_dir: &str,
        prompt: &str,
        cell: LaunchCell,
    ) {
        assert_eq!(selection,
            OperatorChatLaunchSelection::local(component_profile("local-model-rejected")));
        assert_eq!(owner_session_id, "session-ready");
        assert_eq!(working_dir, "D:/work/rejected-repo");
        assert_eq!(prompt, "exercise component audit rejection");
        // This mock models an audit rejection, not a process factory or runtime proof.
        *cell.lock().expect("launch rejection cell") =
            Some(Err("component_selection_audit_failed".to_owned()));
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

    let model_author = profile_selection_author_id(&component_profile("local-model-rejected"));
    node_by_author(&harness, &model_author).click_accesskit();
    harness.run();
    harness.run();

    node_by_author(&harness, &session_selection_author_id("session-ready")).click_accesskit();
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).type_text("D:/work/rejected-repo");
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID)
        .type_text("exercise component audit rejection");
    harness.run();
    node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();

    harness.get_by_label(
        "Launch was not accepted; no audited local selection was applied.",
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
                engine_profile: Some(component_profile("local-model-1")),
                    display_name: "Local Model 1".to_string(),
                    runtime_binding: "candle".to_string(),
                    ready: true,
                },
                OperatorChatModelRow {
                    model_id: "local-model-unavailable".to_string(),
                engine_profile: Some(component_profile("local-model-unavailable")),
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
    let model_author = profile_selection_author_id(&component_profile("local-model-1"));
    let unavailable_model_author =
        profile_selection_author_id(&component_profile("local-model-unavailable"));
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

    // Local choice is pending; the complete ordinary launch owns its audit.
    node_by_author(&harness, &model_author).click_accesskit();
    harness.run();
    harness.run();
    assert!(
        !backend.selections.lock().expect("selections").iter()
            .any(|(selection, _)| selection.lane_kind == "local"),
        "a local profile stays pending before a complete ordinary launch"
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

    assert!(
        backend
            .selections
            .lock()
            .expect("selections")
            .iter()
            .any(|(selection, _)| selection == &OperatorChatLaunchSelection::local(component_profile("local-model-1"))),
        "the complete ordinary local launch carries the exact selection into the component audit"
    );

    let launches = backend.launches.lock().expect("launches");
    assert_eq!(
        launches.as_slice(),
        &[(
            OperatorChatLaunchSelection::local(component_profile("local-model-1")),
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


// Deferred cells model response order only. No backend admission or runtime is asserted.
#[derive(Default)]
struct DeferredLaunchBackend {
    models: Mutex<Vec<ModelsCell>>,
    launches: Mutex<Vec<(OperatorChatLaunchSelection, LaunchCell)>>,
    transcript_calls: Mutex<Vec<String>>,
}

impl OperatorChatBackend for DeferredLaunchBackend {
    fn fetch_models(&self, cell: ModelsCell) {
        self.models.lock().expect("model cells").push(cell);
    }
    fn record_selection(&self, _: OperatorChatLaunchSelection, _: Option<String>, _: SelectionCell) {
        panic!("a local candidate must not audit before its complete ordinary launch");
    }
    fn launch(&self, selection: OperatorChatLaunchSelection, owner: &str, folder: &str,
        prompt: &str, cell: LaunchCell) {
        assert_eq!(owner, "component-owner");
        assert_eq!(folder, "D:/work/deferred-component");
        assert_eq!(prompt, "deferred component request");
        self.launches.lock().expect("launch cells").push((selection, cell));
    }
    fn fetch_transcript(&self, run: &str, cell: TranscriptCell) {
        self.transcript_calls.lock().expect("transcript calls").push(run.to_owned());
        *cell.lock().expect("transcript cell") = Some(Ok(Vec::new()));
    }
    fn routing_action(&self, _: OperatorChatRoutingAction, _: String, cell: RoutingCell) {
        *cell.lock().expect("routing cell") = Some(Err("unused component route".to_owned()));
    }
}

fn deferred_inventory(owner_available: bool) -> OperatorChatModelInventory {
    OperatorChatModelInventory {
        local: vec![OperatorChatModelRow {
            model_id: "deferred-component-model".to_owned(),
            engine_profile: Some(component_profile("deferred-component-model")),
            display_name: "Deferred component profile".to_owned(),
            runtime_binding: "component-only".to_owned(), ready: true,
        }],
        sessions: vec![OperatorChatSessionRow {
            session_id: "component-owner".to_owned(),
            parent_session_id: Some("component-root".to_owned()),
            status: if owner_available { "available" } else { "unavailable" }.to_owned(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn deferred_local_launch_recovery_does_not_accept_stale_or_mismatched_reply_or_consume_b() {
    for case in ["owner-disappeared", "mismatched-profile", "empty-run-id"] {
        let backend = Arc::new(DeferredLaunchBackend::default());
        let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(), db_status: "ok".to_owned(), migration_version: Some(1),
        }));
        app.set_pane_factory(PaneType::OperatorChatLaunch,
            Box::new(OperatorChatLaunchPaneFactory::with_backend(backend.clone())));
        let mut harness = Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
        harness.run();
        harness.get_by_label("MODELS").click(); harness.run();
        harness.get_by_label("Open Operator Chat").click(); harness.run();
        node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit(); harness.run();
        let models = backend.models.lock().expect("model cells").last().expect("refresh").clone();
        *models.lock().expect("models") = Some(Ok(deferred_inventory(true)));
        harness.run(); harness.run();
        let profile = component_profile("deferred-component-model");
        node_by_author(&harness, &profile_selection_author_id(&profile)).click_accesskit(); harness.run();
        node_by_author(&harness, &session_selection_author_id("component-owner")).click_accesskit(); harness.run();
        node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).focus(); harness.run();
        node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).type_text("D:/work/deferred-component"); harness.run();
        node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).focus(); harness.run();
        node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).type_text("deferred component request"); harness.run();
        node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit(); harness.run();
        let a = backend.launches.lock().expect("launch cells")[0].1.clone();
        assert!(backend.transcript_calls.lock().expect("transcript calls").is_empty());
        if case == "owner-disappeared" {
            // Deliver inventory and A together: the native drain reads models first.
            node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit(); harness.run();
            let models = backend.models.lock().expect("model cells").last().expect("refresh").clone();
            *models.lock().expect("models") = Some(Ok(deferred_inventory(false)));
        }
        *a.lock().expect("A") = Some(Ok(OperatorChatLaunched {
            engine_profile: Some(if case == "mismatched-profile" {
                component_profile("different-component-model")
            } else { profile.clone() }),
            instance_id: "component-instance-A".to_owned(),
            run_id: if case == "empty-run-id" { "" } else { "component-run-A" }.to_owned(),
            lane_id: "component-lane-A".to_owned(),
        }));
        harness.run(); harness.run();
        assert!(live_labels(&harness).iter().any(|label|
            label.contains("Unaccepted launch response:") && label.contains("component-instance-A")), "{case}");
        assert!(backend.transcript_calls.lock().expect("transcript calls").is_empty(), "{case}");
        // Restore current inputs, begin B, then publish a duplicate late result into A's own cell.
        node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit(); harness.run();
        let models = backend.models.lock().expect("model cells").last().expect("refresh").clone();
        *models.lock().expect("models") = Some(Ok(deferred_inventory(true)));
        harness.run(); harness.run();
        node_by_author(&harness, &session_selection_author_id("component-owner")).click_accesskit(); harness.run();
        node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit(); harness.run();
        let b = backend.launches.lock().expect("launch cells")[1].1.clone();
        *a.lock().expect("late A") = Some(Ok(OperatorChatLaunched {
            engine_profile: Some(profile.clone()), instance_id: "late-instance-A".to_owned(),
            run_id: "late-run-A".to_owned(), lane_id: "late-lane-A".to_owned(),
        }));
        harness.run(); harness.run();
        assert!(node_by_author(&harness, LAUNCH_AUTHOR_ID).accesskit_node().is_disabled());
        assert!(backend.transcript_calls.lock().expect("transcript calls").is_empty());
        *b.lock().expect("B") = Some(Ok(OperatorChatLaunched {
            engine_profile: Some(profile), instance_id: "component-instance-B".to_owned(),
            run_id: "component-run-B".to_owned(), lane_id: "component-lane-B".to_owned(),
        }));
        harness.run(); harness.run();
        assert_eq!(*backend.transcript_calls.lock().expect("transcript calls"), vec!["component-run-B".to_owned()]);
        assert!(!live_labels(&harness).iter().any(|label| label.contains("late-run-A")));
        // Registration keys survive removal of an earlier recovery entry.
        node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit(); harness.run();
        let c = backend.launches.lock().expect("launch cells")[2].1.clone();
        *c.lock().expect("C") = Some(Ok(OperatorChatLaunched {
            engine_profile: None, instance_id: "component-instance-C".to_owned(),
            run_id: "component-run-C".to_owned(), lane_id: "component-lane-C".to_owned(),
        }));
        harness.run(); harness.run();
        let c_dismiss = "operator-chat.launch-recovery.dismiss.2";
        assert!(live_author_ids(&harness).iter().any(|id| id == c_dismiss));
        node_by_author(&harness, "operator-chat.launch-recovery.dismiss.0").click_accesskit();
        harness.run(); harness.run();
        assert!(!live_author_ids(&harness).iter()
            .any(|id| id == "operator-chat.launch-recovery.dismiss.0"));
        assert!(live_author_ids(&harness).iter().any(|id| id == c_dismiss),
            "dismissing A must not rename C's automation target");
        node_by_author(&harness, c_dismiss).click_accesskit(); harness.run(); harness.run();
        assert!(!live_labels(&harness).iter().any(|label|
            label.contains("Unaccepted launch response:") && label.contains("component-instance-C")));
        assert_eq!(*backend.transcript_calls.lock().expect("transcript calls"),
            vec!["component-run-B".to_owned()]);
    }
}
