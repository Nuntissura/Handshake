//! MT-006 Atelier main-panel proofs.

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::atelier_panel::{
    AtelierPanel, ATELIER_CONTENT_CKC_AUTHOR_ID, ATELIER_CONTENT_INGEST_AUTHOR_ID,
    ATELIER_CONTENT_POSEKIT_AUTHOR_ID, ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
    ATELIER_INGEST_PASS_AUTHOR_ID, ATELIER_INGEST_REJECT_AUTHOR_ID,
    ATELIER_INGEST_UNSURE_AUTHOR_ID, ATELIER_PANEL_AUTHOR_ID, ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
    ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID, ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
    ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID, ATELIER_POSE_RESET_AUTHOR_ID,
    ATELIER_POSE_YAW_MINUS_AUTHOR_ID, ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
    ATELIER_POSE_YAW_SLIDER_AUTHOR_ID, ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
    ATELIER_TABLIST_AUTHOR_ID, ATELIER_TAB_CKC_AUTHOR_ID, ATELIER_TAB_INGEST_AUTHOR_ID,
    ATELIER_TAB_POSEKIT_AUTHOR_ID,
};
use handshake_native::atelier_side_panel::{item_author_id, AtelierSidePanel, PANEL_AUTHOR_ID};
use handshake_native::backend_client::{AtelierBatchRow, AtelierItemRow};
use handshake_native::graph::canvas_board::{CanvasEvent, LoomCanvasBoard, ADD_CARD_AUTHOR_ID};
use handshake_native::theme::HsTheme;

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
