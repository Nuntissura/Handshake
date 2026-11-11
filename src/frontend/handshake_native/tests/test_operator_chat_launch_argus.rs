//! WP-1 MT-012 — native Operator Chat offline component proof (Argus).
//!
//! Opens the pane from RUN, checks its stable AccessKit controls, and enters a
//! worktree and prompt through in-app focus. With the offline factory mounted,
//! Launch stays disabled and Refresh Models exposes the unavailable backend.

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::operator_chat_pane::{
    OperatorChatLaunchPaneFactory, ERROR_AUTHOR_ID, FOLDER_PICKER_AUTHOR_ID, LAUNCH_AUTHOR_ID,
    LAUNCH_STATUS_AUTHOR_ID, MODEL_PICKER_AUTHOR_ID, PROMPT_INPUT_AUTHOR_ID,
    REFRESH_MODELS_AUTHOR_ID, SURFACE_AUTHOR_ID, TRANSCRIPT_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;

fn offline_app() -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_pane_factory(
        PaneType::OperatorChatLaunch,
        Box::new(OperatorChatLaunchPaneFactory::offline()),
    );
    app
}

fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), offline_app())
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
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

fn assert_unique_operator_chat_author_ids(harness: &Harness<'_, HandshakeApp>) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for author_id in live_author_ids(harness)
        .into_iter()
        .filter(|id| id.starts_with("operator-chat."))
    {
        *counts.entry(author_id).or_insert(0) += 1;
    }
    let duplicates = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<Vec<_>>();
    assert!(
        duplicates.is_empty(),
        "operator-chat AccessKit author IDs must be unique: {duplicates:?}"
    );
}

fn assert_no_launch_output(harness: &Harness<'_, HandshakeApp>) {
    let ids = live_author_ids(harness);
    assert!(
        !ids.iter().any(|id| id == LAUNCH_STATUS_AUTHOR_ID),
        "offline pane has no launched session status"
    );
    assert!(
        !ids.iter().any(|id| id.starts_with("operator-chat.transcript.")),
        "offline transcript has no returned rows"
    );
}

#[test]
fn operator_chat_offline_argus_opens_and_reports_unavailable_inventory() {
    let mut harness = shell_harness();
    harness.run();

    // Open the pane through the RUN menu leaf.
    node_by_author(&harness, "menu-run").click();
    harness.run();
    node_by_author(&harness, "menu.models.operator-chat").click();
    harness.run();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::OperatorChatLaunch)),
        "Run menu opened a native OperatorChatLaunch tab"
    );

    // Every stable control is addressable by author_id.
    for expected in [
        SURFACE_AUTHOR_ID,
        MODEL_PICKER_AUTHOR_ID,
        FOLDER_PICKER_AUTHOR_ID,
        PROMPT_INPUT_AUTHOR_ID,
        LAUNCH_AUTHOR_ID,
        REFRESH_MODELS_AUTHOR_ID,
        TRANSCRIPT_AUTHOR_ID,
    ] {
        assert!(
            live_author_ids(&harness).iter().any(|id| id == expected),
            "{expected} present in live AccessKit tree"
        );
    }
    assert_unique_operator_chat_author_ids(&harness);

    // Enter the worktree and prompt through the focused AccessKit fields.
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).type_text("D:/work/repo");
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).type_text("audit the repo");
    harness.run();

    assert_eq!(
        node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).value().as_deref(),
        Some("D:/work/repo")
    );
    let prompt = node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).accesskit_node();
    assert!(prompt.supports_text_ranges(), "prompt exposes its entered text");
    assert_eq!(prompt.document_range().text(), "audit the repo");

    let launch = node_by_author(&harness, LAUNCH_AUTHOR_ID);
    assert!(
        launch.accesskit_node().is_disabled(),
        "Launch requires a governed model and owner"
    );
    launch.click_accesskit();
    harness.run();
    harness.run();
    assert_no_launch_output(&harness);
    assert!(
        !live_author_ids(&harness).iter().any(|id| id == ERROR_AUTHOR_ID),
        "a disabled Launch request does not reach the offline backend"
    );

    node_by_author(&harness, REFRESH_MODELS_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();
    assert_eq!(
        node_by_author(&harness, ERROR_AUTHOR_ID).accesskit_node().label().as_deref(),
        Some("backend not wired (offline pane)")
    );
    assert!(
        node_by_author(&harness, MODEL_PICKER_AUTHOR_ID)
            .value()
            .unwrap_or_default()
            .is_empty(),
        "offline refresh leaves the selected model empty"
    );
    assert!(
        !live_author_ids(&harness).iter().any(|id| {
            id.starts_with("operator-chat.model.")
                || id.starts_with("operator-chat.profile.")
                || id.starts_with("operator-chat.session.")
        }),
        "offline refresh exposes no governed model or owner choices"
    );
    assert!(
        node_by_author(&harness, LAUNCH_AUTHOR_ID).accesskit_node().is_disabled(),
        "Launch stays disabled after the offline refresh"
    );
    assert_no_launch_output(&harness);
    assert_unique_operator_chat_author_ids(&harness);
}
