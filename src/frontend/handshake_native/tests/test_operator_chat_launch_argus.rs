//! WP-1 MT-012 — native Operator Chat / Launch pane live UI proof (Argus).
//!
//! Drives the pane headlessly through the RUN menu, targets every control by its
//! stable AccessKit author_id, types into the folder + prompt inputs (in-app
//! AccessKit focus only; no OS-window foreground call — HBR-QUIET), and drives the
//! launch button. The `with_health` app registers the OFFLINE pane factory, so a
//! launch honestly reports "backend not wired" instead of fabricating a session.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::operator_chat_pane::{
    ERROR_AUTHOR_ID, FOLDER_PICKER_AUTHOR_ID, LAUNCH_AUTHOR_ID, MODEL_PICKER_AUTHOR_ID,
    PROMPT_INPUT_AUTHOR_ID, REFRESH_MODELS_AUTHOR_ID, SURFACE_AUTHOR_ID, TRANSCRIPT_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app())
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

#[test]
fn operator_chat_launch_argus_opens_picks_types_and_launches() {
    let mut harness = shell_harness();
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

    // Type a model, folder, and prompt (in-app AccessKit focus only).
    node_by_author(&harness, MODEL_PICKER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, MODEL_PICKER_AUTHOR_ID).type_text("claude-sonnet-4");
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, FOLDER_PICKER_AUTHOR_ID).type_text("D:/work/repo");
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).focus();
    harness.run();
    node_by_author(&harness, PROMPT_INPUT_AUTHOR_ID).type_text("audit the repo");
    harness.run();

    // Drive launch. The with_health app registers the OFFLINE pane, so an honest
    // "backend not wired" error appears (never a fabricated session).
    node_by_author(&harness, LAUNCH_AUTHOR_ID).click_accesskit();
    harness.run();
    harness.run();

    assert!(
        live_author_ids(&harness)
            .iter()
            .any(|id| id == ERROR_AUTHOR_ID),
        "an offline launch surfaces the operator-chat error control"
    );
    assert_unique_operator_chat_author_ids(&harness);
}
