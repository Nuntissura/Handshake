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
    model_selection_author_id, transcript_row_author_id_for, LaunchCell, ModelsCell,
    OperatorChatBackend, OperatorChatCloudRow, OperatorChatLaunchPaneFactory,
    OperatorChatLaunchSelection, OperatorChatLaunched, OperatorChatModelInventory,
    OperatorChatModelRow, OperatorChatSubagentRow, TranscriptCell, TranscriptRow,
    FOLDER_PICKER_AUTHOR_ID, LAUNCH_AUTHOR_ID, LAUNCH_STATUS_AUTHOR_ID, PROMPT_INPUT_AUTHOR_ID,
    REFRESH_MODELS_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;

/// A recording backend: delivers a fixed inventory + transcript, and records every
/// model selection the pane sends so F6 is provable.
struct RecordingBackend {
    inventory: OperatorChatModelInventory,
    run_id: String,
    transcript: Vec<TranscriptRow>,
    selections: Mutex<Vec<(OperatorChatLaunchSelection, Option<String>)>>,
    launches: Mutex<Vec<OperatorChatLaunchSelection>>,
}

impl OperatorChatBackend for RecordingBackend {
    fn fetch_models(&self, cell: ModelsCell) {
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Ok(self.inventory.clone()));
        }
    }

    fn record_selection(&self, selection: &OperatorChatLaunchSelection, working_dir: Option<&str>) {
        self.selections
            .lock()
            .expect("selections lock")
            .push((selection.clone(), working_dir.map(str::to_owned)));
    }

    fn launch(
        &self,
        selection: OperatorChatLaunchSelection,
        _working_dir: &str,
        _prompt: &str,
        cell: LaunchCell,
    ) {
        self.launches.lock().expect("launches lock").push(selection);
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

#[test]
fn operator_chat_model_select_fires_audit_and_launch_renders_fetched_transcript() {
    let backend = Arc::new(RecordingBackend {
        inventory: OperatorChatModelInventory {
            local: vec![OperatorChatModelRow {
                model_id: "local-model-1".to_string(),
                display_name: "Local Model 1".to_string(),
                runtime_binding: "candle".to_string(),
                ready: true,
            }],
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
                status: "configured".to_string(),
            }],
            subagents: vec![OperatorChatSubagentRow {
                role: "subagent_coder".to_string(),
                model_id: "subagent://operator-chat/coder".to_string(),
                label: "Subagent Manager / Coder".to_string(),
                status: "available".to_string(),
            }],
            ..Default::default()
        },
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
    harness.get_by_label("RUN").click();
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
        &[OperatorChatLaunchSelection::local("local-model-1")],
        "launch carries the structured LOCAL lane selection instead of hard-coding CLI"
    );
    drop(launches);

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
