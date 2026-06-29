//! MT-006 Atelier main-panel proofs.

use std::sync::{Arc, Mutex};

use egui::accesskit;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::atelier_panel::{
    ckc_folder_row_author_id, ckc_media_album_row_author_id, ckc_media_row_author_id, AtelierPanel,
    ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID, ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
    ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID, ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
    ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID, ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
    ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID, ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
    ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID, ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
    ATELIER_CKC_SHEET_SAVE_AUTHOR_ID, ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
    ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID, ATELIER_CONTENT_CKC_AUTHOR_ID,
    ATELIER_CONTENT_INGEST_AUTHOR_ID, ATELIER_CONTENT_POSEKIT_AUTHOR_ID,
    ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID, ATELIER_INGEST_PASS_AUTHOR_ID,
    ATELIER_INGEST_REJECT_AUTHOR_ID, ATELIER_INGEST_UNSURE_AUTHOR_ID, ATELIER_PANEL_AUTHOR_ID,
    ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID, ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
    ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID, ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
    ATELIER_POSE_RESET_AUTHOR_ID, ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
    ATELIER_POSE_YAW_PLUS_AUTHOR_ID, ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
    ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID, ATELIER_TABLIST_AUTHOR_ID, ATELIER_TAB_CKC_AUTHOR_ID,
    ATELIER_TAB_INGEST_AUTHOR_ID, ATELIER_TAB_POSEKIT_AUTHOR_ID,
};
use handshake_native::atelier_side_panel::{item_author_id, AtelierSidePanel, PANEL_AUTHOR_ID};
use handshake_native::backend_client::{AtelierBatchRow, AtelierItemRow};
use handshake_native::graph::canvas_board::{CanvasEvent, LoomCanvasBoard, ADD_CARD_AUTHOR_ID};
use handshake_native::mcp::{
    dispatch_request, ActionChannel, McpRequest, ScreenshotError, SessionToken,
};
use handshake_native::theme::HsTheme;

const MIRA_DEMO_ROW_AUTHOR_ID: &str = "atelier-ckc-character-018f7848-1111-7000-9000-000000000001";
const ARIA_DEMO_ROW_AUTHOR_ID: &str = "atelier-ckc-character-018f7848-1111-7000-9000-000000000002";
const MIRA_DEMO_ALBUM_ID: &str = "018f7848-1111-7000-9000-00000000a001";
const MIRA_DEMO_MEDIA_ID: &str = "018f7848-1111-7000-9000-00000000b001";
const MIRA_DEMO_FOLDER_REF: &str = "atelier://folder/mira-reference-set";
const MIRA_DEMO_SECOND_ALBUM_ID: &str = "018f7848-1111-7000-9000-00000000a003";
const MIRA_DEMO_SECOND_MEDIA_ID: &str = "018f7848-1111-7000-9000-00000000b003";
const MIRA_DEMO_SECOND_FOLDER_REF: &str = "atelier://folder/mira-expression-set";
const PROBE_ACTIONS: &[accesskit::Action] = &[
    accesskit::Action::Click,
    accesskit::Action::Focus,
    accesskit::Action::SetValue,
    accesskit::Action::ReplaceSelectedText,
    accesskit::Action::ScrollIntoView,
];

fn seeded_side_panel() -> Arc<Mutex<AtelierSidePanel>> {
    Arc::new(Mutex::new(AtelierSidePanel::with_rows(
        vec![AtelierBatchRow {
            batch_id: "batch-1".to_owned(),
            source_label: "Sourcing Run A".to_owned(),
            status: "open".to_owned(),
        }],
        vec![],
        Some((
            "batch-1".to_owned(),
            vec![AtelierItemRow {
                item_id: "item-aaa".to_owned(),
                file_name: "sunset.png".to_owned(),
                source_path: "/intake/sunset.png".to_owned(),
                lane: "accept".to_owned(),
            }],
        )),
    )))
}

fn author_ids(harness: &Harness<'_, AtelierPanel>) -> std::collections::HashSet<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn snapshot_harness(harness: &mut Harness<'_, AtelierPanel>) -> UiTreeSnapshot {
    let mut children = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let author_id = ak.author_id().map(str::to_owned);
        let node_id = ak.id().0;
        let actions = PROBE_ACTIONS
            .iter()
            .filter(|action| ak.data().supports_action(**action))
            .map(|action| format!("{action:?}"))
            .collect();
        children.push(UiTreeNode {
            id: author_id
                .clone()
                .unwrap_or_else(|| format!("node:{node_id}")),
            author_id,
            node_id,
            role: format!("{:?}", ak.role()),
            label: ak.label().map(|value| value.to_string()),
            value: ak.value().map(|value| value.to_string()),
            disabled: ak.is_disabled(),
            actions,
            bounds: None::<UiNodeBounds>,
            children: Vec::new(),
        });
    }
    let widget_count = children.len() + 1;
    UiTreeSnapshot {
        root: UiTreeNode {
            id: "node:atelier-proof-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count,
    }
}

fn build_panel_harness() -> Harness<'static, AtelierPanel> {
    let panel = AtelierPanel::new(
        seeded_side_panel(),
        Arc::new(Mutex::new(LoomCanvasBoard::new("ws-test", "canvas-1"))),
        Arc::new(Mutex::new(Vec::<CanvasEvent>::new())),
    );
    Harness::builder()
        .with_size(egui::vec2(1280.0, 760.0))
        .build_state(
            |ctx, panel: &mut AtelierPanel| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    panel.show(ui, &HsTheme::Dark.palette());
                });
            },
            panel,
        )
}

fn argus_token() -> SessionToken {
    SessionToken::from_hex("mt009-argus-proof-secret")
}

fn argus_req(method: &str, params: serde_json::Value) -> McpRequest {
    McpRequest {
        id: serde_json::json!(1),
        method: method.to_owned(),
        params,
        session_token: "mt009-argus-proof-secret".to_owned(),
        agent_label: Some("mt009-argus-proof".to_owned()),
    }
}

#[test]
fn atelier_main_panel_exposes_ckc_posekit_ingest_tabs() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    for expected in [
        ATELIER_PANEL_AUTHOR_ID,
        ATELIER_TABLIST_AUTHOR_ID,
        ATELIER_TAB_CKC_AUTHOR_ID,
        ATELIER_TAB_POSEKIT_AUTHOR_ID,
        ATELIER_TAB_INGEST_AUTHOR_ID,
        ATELIER_CONTENT_CKC_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
        PANEL_AUTHOR_ID,
        &item_author_id("item-aaa"),
        ADD_CARD_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(expected),
            "expected stable author_id {expected}; got {ids:?}"
        );
    }
}

#[test]
fn ckc_character_sheet_surface_is_model_addressable() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    for expected in [
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        MIRA_DEMO_ROW_AUTHOR_ID,
        ARIA_DEMO_ROW_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
        ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(expected),
            "CKC character-sheet surface must expose stable author_id {expected}; got {ids:?}"
        );
    }
    for expected in [
        ckc_media_album_row_author_id(MIRA_DEMO_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_FOLDER_REF),
        ckc_media_album_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_SECOND_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_SECOND_FOLDER_REF),
    ] {
        assert!(
            ids.contains(&expected),
            "CKC linked-media surface must expose stable author_id {expected}; got {ids:?}"
        );
    }

    for expected in [
        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Focus),
            "CKC text control {expected} must be steerable by Argus set_value"
        );
    }
    let save = harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_SHEET_SAVE_AUTHOR_ID));
    assert!(
        save.accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC append-version control must be steerable by Argus click"
    );
    let media_save =
        harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID));
    assert!(
        media_save
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC media-notes save control must be steerable by Argus click"
    );
}

#[test]
fn argus_inspects_and_steers_ckc_character_sheet_surface() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();
    let snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
    ] {
        assert!(
            snapshot.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include CKC sheet author_id {expected}"
        );
    }

    let mut channel = ActionChannel::new();
    let inspect = dispatch_request(
        &argus_req("argus.inspect", serde_json::json!({})),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let inspect_json = inspect.to_json();
    assert_eq!(inspect_json["result"]["argus"]["method"], "argus.inspect");
    assert_eq!(inspect_json["result"]["argus"]["headless"], true);
    assert_eq!(inspect_json["result"]["argus"]["non_intrusive"], true);

    let set_value = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
                "value": "name: Argus Proof\nworkflow: CKC sheet edit"
            }),
        ),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let set_value_json = set_value.to_json();
    assert_eq!(set_value_json["result"]["queued"], true);
    assert_eq!(
        set_value_json["result"]["target"],
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID
    );

    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after_set = snapshot_harness(&mut harness);
    let editor = after_set
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .expect("editor visible after Argus set_value");
    assert!(
        editor
            .value
            .as_deref()
            .unwrap_or_default()
            .contains("Argus Proof"),
        "Argus set_value must change the live CKC sheet editor value; got {editor:?}"
    );

    let before_ref = snapshot_harness(&mut harness)
        .find_by_author_id(ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("sheet ref label before save");
    let click_snapshot = snapshot_harness(&mut harness);
    let click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_SHEET_SAVE_AUTHOR_ID }),
        ),
        &argus_token(),
        &click_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let click_json = click.to_json();
    assert_eq!(click_json["result"]["queued"], true);
    assert_eq!(
        click_json["result"]["target"],
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID
    );
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after_ref = snapshot_harness(&mut harness)
        .find_by_author_id(ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("sheet ref label after save");
    assert_ne!(
        before_ref, after_ref,
        "Argus click must rerender a new visible CKC sheet_version_ref"
    );
}

#[test]
fn argus_inspects_and_steers_ckc_linked_media_without_touching_sheet_notes() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();
    let snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
    ] {
        assert!(
            snapshot.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include CKC linked-media author_id {expected}"
        );
    }
    for expected in [
        ckc_media_album_row_author_id(MIRA_DEMO_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_FOLDER_REF),
        ckc_media_album_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_SECOND_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_SECOND_FOLDER_REF),
    ] {
        assert!(
            snapshot.find_by_author_id(&expected).is_some(),
            "Argus inspect snapshot must include CKC linked-media row {expected}"
        );
    }

    let sheet_before = snapshot
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("sheet editor value before media note edit");
    let first_media_note_before = snapshot
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("first media notes value before selecting second media");

    let mut channel = ActionChannel::new();
    let second_media_author_id = ckc_media_row_author_id(MIRA_DEMO_SECOND_MEDIA_ID);
    let select_second = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": second_media_author_id }),
        ),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(select_second.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let second_selected = snapshot_harness(&mut harness);
    let selected_media_note = second_selected
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("second media notes value after selecting second media");
    assert_ne!(
        first_media_note_before, selected_media_note,
        "Argus click on a second CKC media row must retarget the shared image-note editor"
    );

    let set_note = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
                "value": "approved close-up for album, separate from character sheet notes"
            }),
        ),
        &argus_token(),
        &second_selected,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_note.to_json()["result"]["queued"], true);
    let set_tags = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
                "value": "face, approved, training"
            }),
        ),
        &argus_token(),
        &second_selected,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_tags.to_json()["result"]["queued"], true);

    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after = snapshot_harness(&mut harness);
    let media_note = after
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media notes value after Argus set_value");
    assert!(
        media_note.contains("approved close-up"),
        "Argus set_value must update the CKC image-note editor; got {media_note}"
    );
    let media_tags = after
        .find_by_author_id(ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media tags value after Argus set_value");
    assert!(
        media_tags.contains("training"),
        "Argus set_value must update the CKC media tags editor; got {media_tags}"
    );
    let sheet_after = after
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("sheet editor value after media note edit");
    assert_eq!(
        sheet_before, sheet_after,
        "image notes/tags must stay separate from character sheet notes"
    );
}

#[test]
fn atelier_internal_tabs_switch_visible_content_regions() {
    let mut harness = build_panel_harness();
    harness.run();

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_POSEKIT_AUTHOR_ID))
        .click();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        ids.contains(ATELIER_CONTENT_POSEKIT_AUTHOR_ID),
        "Posekit content region should be visible after clicking the Posekit tab"
    );

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_INGEST_AUTHOR_ID))
        .click();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        ids.contains(ATELIER_CONTENT_INGEST_AUTHOR_ID),
        "Ingest content region should be visible after clicking the Ingest tab"
    );
}

#[test]
fn posekit_and_ingest_controls_are_model_addressable() {
    let mut harness = build_panel_harness();
    harness.run();

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_POSEKIT_AUTHOR_ID))
        .click();
    harness.run();
    for expected in [
        ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
        ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
        ATELIER_POSE_RESET_AUTHOR_ID,
        ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
        ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
        ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click)
                || node
                    .accesskit_node()
                    .data()
                    .supports_action(egui::accesskit::Action::Focus),
            "Posekit control {expected} must be steerable by Argus/MCP"
        );
    }

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_INGEST_AUTHOR_ID))
        .click();
    harness.run();
    for expected in [
        ATELIER_INGEST_PASS_AUTHOR_ID,
        ATELIER_INGEST_REJECT_AUTHOR_ID,
        ATELIER_INGEST_UNSURE_AUTHOR_ID,
        ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click)
                || node
                    .accesskit_node()
                    .data()
                    .supports_action(egui::accesskit::Action::Focus),
            "Ingest control {expected} must be steerable by Argus/MCP"
        );
    }
}
