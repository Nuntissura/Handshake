//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 1: CKC media -> note embed (IC-01..IC-05).
//!
//! This suite proves the CKC<->note/canvas/Stage melt-together edge. IC-05 drives the shared in-process
//! InteractionBus; IC-01..04 run by default through the managed product-backend fixture and self-seed
//! assets, typed CKC blocks, native RichDocuments, canvas placements, and backlinks. For IC-01/02/04 the
//! in-process half ALSO proves the embed atom's
//! `content_json` SHAPE round-trips structurally (the hsLink atom the backend persists), so the PG half is
//! the durable save/reload, not the whole proof.
//!
//! The backend authority has dedicated `ckc_moodboard` and `ckc_character` content types. IC-03/04 assert
//! those exact persisted types; a generic note fallback is not accepted.
//!
//! ## Artifact hygiene (CX-212E, HARD)
//! No artifact is ever written under `src/`. The hygiene guard fails the run on a repo-local artifact dir.

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use std::sync::Arc;

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::created_canvas_placement_from_response;
use handshake_native::backend_client::HealthInfo;
use handshake_native::graph::canvas_board::{CanvasPlacementCard, LoomCanvasBoard};
use handshake_native::interop::{
    AtelierItemKind, AtelierRef, DragPayload, EditorSurfaceKind, InteractionBus, SharedSelection,
    CMD_ROUTE_TO_STAGE,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::doc_json::{
    from_json_string, to_content_json_value,
};
use handshake_native::rich_editor::document_model::node::{BlockNode, HsLinkNode};
use handshake_native::rich_editor::document_model::{DocPosition, Selection};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::stage_pane::{StageContent, STAGE_ROUTED_CONTENT_AUTHOR_ID};
use handshake_native::tab_bar::TabState;

use interconnect_support::{
    assert_no_local_artifact_dir, author_node_value, require_live_backend,
    save_rich_document_via_production_manager, ScenarioAttempt,
};

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

/// Drive the shipped rich-editor CKC insertion transaction over a real caret and return the resulting
/// document. This is the same operation the editor's typed drag/drop handler invokes after resolving a
/// `DragPayload`, including the editor undo receipt and caret movement; no direct DocModel mutation.
fn doc_with_ckc_embed(ref_kind: &str, ref_value: &str, label: &str) -> BlockNode {
    let mut state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("embed ")]));
    let inserted = RichEditorWidget::insert_atelier_embed_at_caret(
        &mut state,
        HsLinkNode::new(ref_kind, ref_value, label),
    );
    assert!(
        inserted,
        "CKC embed must insert through the native editor transaction"
    );
    assert_eq!(
        state.undo.len(),
        1,
        "CKC editor insertion records one native undo transaction"
    );
    state.doc
}

/// The created document id from a `POST /knowledge/documents` response. The real handler returns
/// `{ "document": created, ... }` where the id lives at `document.rich_document_id`
/// (verified against src/backend/handshake_core/src/api/knowledge_documents.rs:729-737); this mirrors the
/// proven `created_doc_id` helper in test_parity_rich_editor.rs. Verified fallbacks only.
fn created_doc_id(created: &serde_json::Value) -> String {
    created
        .get("document")
        .and_then(|d| d.get("rich_document_id"))
        .and_then(|v| v.as_str())
        .or_else(|| created.get("rich_document_id").and_then(|v| v.as_str()))
        .or_else(|| created.get("id").and_then(|v| v.as_str()))
        .expect(
            "requires_pg: created document returns a rich_document_id (document.rich_document_id)",
        )
        .to_owned()
}

/// The current `doc_version` of a `POST /knowledge/documents` response, for the optimistic-concurrency
/// `/save` route (`{ expected_version, content_json }`). Defaults to 1 when absent.
fn created_doc_version(created: &serde_json::Value) -> i64 {
    created
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| created.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(1)
}

/// Extract the `content_json` doc value from a `GET /knowledge/documents/{id}` load response. The real
/// handler returns `{ "document": document, "tree": ..., "code_nodes": ... }` where the persisted blob is
/// `document.content_json` (verified against knowledge_documents.rs:766-770). Falls back to a top-level
/// `content_json` (the create-body echo) when the load wrapper is absent.
fn loaded_content_json(loaded: &serde_json::Value) -> serde_json::Value {
    loaded
        .get("document")
        .and_then(|d| d.get("content_json"))
        .or_else(|| loaded.get("content_json"))
        .cloned()
        .unwrap_or_else(|| loaded.clone())
}

/// Find the first hsLink atom's `(refKind, refValue)` in a content_json doc value.
fn first_hs_link(content_json: &serde_json::Value) -> Option<(String, String)> {
    fn walk(v: &serde_json::Value) -> Option<(String, String)> {
        if let Some(obj) = v.as_object() {
            if obj.get("type").and_then(|t| t.as_str()) == Some("hsLink") {
                let attrs = obj.get("attrs")?;
                return Some((
                    attrs.get("refKind")?.as_str()?.to_owned(),
                    attrs.get("refValue")?.as_str()?.to_owned(),
                ));
            }
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    if let Some(found) = walk(c) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }
    walk(content_json)
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-05 — SUBSTRATE PROOF (PASS, no PG): select text in the mounted rich editor, drive the production
// EDITORS > Route selection to Stage MenuItem by raw AccessKit action, and observe the mounted Stage pane's
// canonical `stage-routed-content` node. The app owns the bus, command dispatch, pane opening, route drain,
// and Stage state; this proof does not install an alias or manually drain pending bus content.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic05_route_selection_to_stage() {
    let attempt = ScenarioAttempt::begin("IC-05");
    const SELECTED: &str = "the routed melt-together selection";
    const SOURCE_DOCUMENT: &str = "DOC-IC05-MOUNTED";
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("IC-05: build mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    let rich_pane = pane("pane-b");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        rich_pane.clone(),
        PaneType::LoomWikiPage,
        DEFAULT_PROJECT_ID,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&rich_pane)
        .expect("IC-05: mounted rich pane owns a tab bar");
    bar.tabs = vec![TabState::new(PaneType::LoomWikiPage)];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(rich_pane));
    let rich_state = app.mounted_rich_state();
    let stage = app.mounted_stage();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("IC-05: mounted rich editor AccessKit text surface is live")
        .focus();
    harness.run_steps(2);
    // Materialize the operator-authored document and selection after the initial mounted frames, matching
    // the live flow. The host installs its startup document while mounting; routing must consume content
    // authored after that initialization, not a pre-mount fixture that the host legitimately replaces.
    {
        let mut state = rich_state.lock().unwrap();
        state.doc = BlockNode::doc(vec![BlockNode::paragraph(SELECTED)]);
        state.wikilinks.document_id = SOURCE_DOCUMENT.to_owned();
        state.selection = Selection::text(
            DocPosition::new(vec![0, 0], 0),
            DocPosition::new(vec![0, 0], SELECTED.chars().count()),
        );
        assert_eq!(
            state.selected_text().map(|(_, _, _, text)| text),
            Some(SELECTED.to_owned()),
            "IC-05: mounted rich editor owns the exact live selection"
        );
    }
    harness.run_steps(1);
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let (published_selection, published_focus) = {
        let bus = bus.lock().unwrap();
        (bus.shared_selection().clone(), bus.focus_owner().cloned())
    };
    assert!(
        matches!(
            &published_selection,
            SharedSelection::TextRange {
                surface: EditorSurfaceKind::RichText,
                text,
                ..
            } if text == SELECTED
        ),
        "IC-05: the mounted rich adapter publishes the exact live selection before menu dispatch; \
         selection={published_selection:?}, focus={published_focus:?}"
    );

    let editors_menu_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("menu-editors"))
        .map(|node| node.accesskit_node().id())
        .expect("IC-05: mounted EDITORS MenuItem has stable author_id");
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: editors_menu_id,
            data: None,
        },
    ));
    harness.run_steps(2);
    let route_menu = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("menu.editors.route-to-stage"))
        .map(|node| {
            (
                node.accesskit_node().id(),
                node.accesskit_node().is_disabled(),
            )
        })
        .expect("IC-05: mounted Route selection to Stage MenuItem is live");
    assert!(
        !route_menu.1,
        "IC-05: rich selection enables Route to Stage"
    );
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: route_menu.0,
            data: None,
        },
    ));
    harness.run_steps(5);

    assert!(
        bus.lock()
            .unwrap()
            .commands()
            .get(CMD_ROUTE_TO_STAGE)
            .is_some(),
        "IC-05: the mounted menu reused the canonical shared-bus command"
    );
    let routed_value = author_node_value(&harness, STAGE_ROUTED_CONTENT_AUTHOR_ID)
        .expect("IC-05: the stage-routed-content AccessKit node must be present after routing");
    assert!(
        routed_value.contains(SELECTED),
        "IC-05: the production Stage AccessKit value matches the routed selection text \
         (got {routed_value:?})"
    );
    match &stage.lock().unwrap().content {
        StageContent::Selection(text, source) => {
            assert_eq!(text, SELECTED);
            assert_eq!(source, SOURCE_DOCUMENT);
        }
        other => panic!("IC-05: expected a routed Selection, got {other:?}"),
    }

    attempt.pass(serde_json::json!({
        "route": "selection_to_stage",
        "mounted_editors_menu": true,
        "shared_app_bus": true,
        "production_accesskit_author_id": STAGE_ROUTED_CONTENT_AUTHOR_ID,
        "manual_pending_route_drain": false,
    }));
    assert_no_local_artifact_dir();
    println!(
        "IC-05 SUBSTRATE PASS: mounted EDITORS menu routed selection over the app bus into \
         AccessKit `stage-routed-content`; backlink.confirmed=na"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-01 / IC-02 / IC-04 — content_json SHAPE half (PASS, no PG): the CKC embed atom is an inline `hsLink`
// node (refKind=HS_images / video / character) that ROUND-TRIPS the backend content_json. This is the
// structural half the default managed-PG save/reload proofs below builds on — it proves
// the editor authors the SAME hsLink the backend persists, not an invented node that would be dropped on
// save. (These do not flip the manifest status, which stays REQUIRES_PG until the durable round-trip runs.)
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ic01_ic02_ic04_ckc_embed_atom_shape_round_trips() {
    for (ic, ref_kind, ref_value, label) in [
        ("IC-01", "HS_images", "asset-img-1", "sunset.png"),
        ("IC-02", "video", "asset-vid-1", "clip.mp4"),
        ("IC-04", "character", "char-block-1", "Aria"),
    ] {
        let doc = doc_with_ckc_embed(ref_kind, ref_value, label);
        // Round-trip through the SAME DocJson the backend persists/loads.
        let json = handshake_native::rich_editor::document_model::doc_json::to_json_string(&doc)
            .expect("serialize content_json (the persisted blob)");
        let back = from_json_string(&json).expect("reload (the loadRichDocument shape)");
        assert_eq!(
            doc, back,
            "{ic}: the CKC embed doc round-trips through DocJson unchanged"
        );
        // The atom is an hsLink carrying the named ref_kind + the asset/block id (NOT an invented node).
        let v = to_content_json_value(&doc);
        let (rk, rv) = first_hs_link(&v).expect("an hsLink atom is present");
        assert_eq!(
            rk, ref_kind,
            "{ic}: the embed is an hsLink with the named refKind"
        );
        assert_eq!(rv, ref_value, "{ic}: refValue carries the asset/block id");
        let json_str = serde_json::to_string(&v).unwrap();
        assert!(
            json_str.contains("\"hsLink\""),
            "{ic}: the embed serializes as an hsLink node"
        );
        assert!(
            !json_str.contains("atelier_embed") && !json_str.contains("\"embed\""),
            "{ic}: the embed must NOT be an invented node (it would be dropped on save)"
        );
        println!("{ic} SHAPE: CKC embed is an hsLink({ref_kind}, {ref_value}) that round-trips content_json");
    }
    assert_no_local_artifact_dir();
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-01..IC-04 — default managed-PostgreSQL persistence proofs. NEVER mocked, NEVER faked.
// VERIFIED REAL ROUTES (the route-shape drift the review flagged is corrected here):
//   - asset-create = POST /workspaces/{ws}/loom/import (loom.rs:217 import_loom_asset -> create_asset);
//     there is NO bare POST /workspaces/{ws}/assets route (only GET /assets/{id}[/content|/thumbnail|/tiers]).
//   - knowledge docs are merged BARE (no /workspaces prefix): POST /knowledge/documents (workspace_id in
//     body), GET /knowledge/documents/{id}, PUT /knowledge/documents/{id}/save ({expected_version,content_json}).
//   - the create response wraps the doc: { "document": { rich_document_id, doc_version, .. }, .. }.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic01_ckc_image_into_note() {
    let attempt = ScenarioAttempt::begin("IC-01");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    // (1) create an asset row via the REAL managed asset-create route POST /workspaces/{ws}/loom/import
    //     (verified: loom.rs:217 import_loom_asset -> storage.create_asset; there is NO bare POST /assets
    //     route). The import returns { block_id, asset_id, content_hash } (LoomImportResult, loom.rs:1871).
    let asset = be.post_json(
        &format!("/workspaces/{ws}/loom/import"),
        &serde_json::json!({
            "bytes_b64": "aW1hZ2UtYnl0ZXM=", // "image-bytes"
            "original_filename": "sunset.png",
            "mime": "image/png"
        }),
    );
    let asset_id = asset["asset_id"]
        .as_str()
        .expect("requires_pg: POST /loom/import returns an asset_id (LoomImportResult.asset_id)")
        .to_owned();
    // (2) create a note carrying the CKC image embed hsLink (refKind=HS_images, refValue=asset_id).
    let doc = doc_with_ckc_embed("HS_images", &asset_id, "sunset.png");
    let content_json = to_content_json_value(&doc);
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-01 note", "content_json": content_json }),
    );
    let doc_id = created_doc_id(&created);
    let version = created_doc_version(&created);
    // (3) save via the REAL optimistic-concurrency route PUT /knowledge/documents/{doc_id}/save
    //     ({ expected_version, content_json }); (4) reload + assert the hsLink node with the asset id.
    let saved = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&doc) }),
    );
    let reloaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let (rk, rv) =
        first_hs_link(&loaded_content_json(&reloaded)).expect("reloaded doc carries an hsLink");
    assert_eq!(
        rk, "HS_images",
        "IC-01: reloaded embed is the HS_images hsLink"
    );
    assert_eq!(rv, asset_id, "IC-01: reloaded embed points at the asset id");
    // The embedded asset renders: GET /workspaces/{ws}/assets/{asset_id} == 200.
    assert_eq!(
        be.get_status(&format!("/workspaces/{ws}/assets/{asset_id}")),
        200
    );
    let save_event_id = saved["save_receipt_event_id"]
        .as_str()
        .expect("IC-01: durable save returns an EventLedger receipt")
        .to_owned();
    let negative_status = be.get_status(&format!("/workspaces/{ws}/assets/AST-ic01-missing"));
    assert_eq!(
        negative_status, 404,
        "IC-01: missing embed asset fails closed"
    );
    // Idempotent cleanup (DropGuard-style best-effort).
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "document_id": doc_id,
        "asset_id": asset_id,
        "event_ledger_event_id": save_event_id,
        "negative_missing_asset_status": negative_status,
    }));
    println!("IC-01 LIVE-PG PASS: CKC image embedded + reloaded with asset {asset_id}; GET /assets == 200");
}

#[test]
fn interconnect_ic02_ckc_video_into_note() {
    let attempt = ScenarioAttempt::begin("IC-02");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    // Managed asset-create via the REAL POST /workspaces/{ws}/loom/import (no bare POST /assets route).
    let asset = be.post_json(
        &format!("/workspaces/{ws}/loom/import"),
        &serde_json::json!({
            "bytes_b64": "dmlkZW8tYnl0ZXM=", // "video-bytes"
            "original_filename": "clip.mp4",
            "mime": "video/mp4"
        }),
    );
    let asset_id = asset["asset_id"]
        .as_str()
        .expect("requires_pg: POST /loom/import returns an asset_id")
        .to_owned();
    let doc = doc_with_ckc_embed("video", &asset_id, "clip.mp4");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-02 note",
            "content_json": to_content_json_value(&doc) }),
    );
    let doc_id = created_doc_id(&created);
    let version = created_doc_version(&created);
    let saved = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&doc) }),
    );
    let reloaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let (rk, rv) =
        first_hs_link(&loaded_content_json(&reloaded)).expect("reloaded doc carries an hsLink");
    assert_eq!(rk, "video", "IC-02: reloaded embed is the video hsLink");
    assert_eq!(
        rv, asset_id,
        "IC-02: video embed keeps the durable asset id"
    );
    let save_event_id = saved["save_receipt_event_id"]
        .as_str()
        .expect("IC-02: durable save returns an EventLedger receipt")
        .to_owned();
    let negative_status = be.get_status(&format!("/workspaces/{ws}/assets/AST-ic02-missing"));
    assert_eq!(
        negative_status, 404,
        "IC-02: missing video asset fails closed"
    );
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "document_id": doc_id,
        "asset_id": asset_id,
        "event_ledger_event_id": save_event_id,
        "negative_missing_asset_status": negative_status,
    }));
    println!("IC-02 LIVE-PG PASS: CKC video embedded as an hsLink(video) and reloaded");
}

#[test]
fn interconnect_ic03_ckc_moodboard_on_canvas() {
    let attempt = ScenarioAttempt::begin("IC-03");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    let block = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": "IC-03 moodboard", "content_type": "ckc_moodboard" }),
    );
    let block_id = block["block_id"]
        .as_str()
        .or_else(|| block["id"].as_str())
        .expect("requires_pg: block id")
        .to_owned();
    assert_eq!(block["content_type"], "ckc_moodboard");
    // Create a canvas board and place the block on it.
    let board = be.post_json(
        &format!("/workspaces/{ws}/loom/canvas-boards"),
        &serde_json::json!({ "title": "IC-03 canvas" }),
    );
    let board_id = board["block_id"]
        .as_str()
        .or_else(|| board["id"].as_str())
        .expect("requires_pg: board id")
        .to_owned();
    let drag_payload = DragPayload::AtelierRef(AtelierRef::with_loom_block(
        block_id.clone(),
        AtelierItemKind::Moodboard,
        "IC-03 moodboard",
        block_id.clone(),
    ));
    let canvas_payload = drag_payload
        .canvas_drag_payload()
        .expect("IC-03: a moodboard with a canonical Loom block converts through the native canvas drop seam");
    let placement = be.post_json(
        &format!("/workspaces/{ws}/loom/canvas-boards/{board_id}/placements"),
        &serde_json::json!({
            "placed_block_id": canvas_payload.block_id,
            "x": 100.0,
            "y": 100.0,
            "w": 320.0,
            "h": 180.0
        }),
    );
    let board_state = be.get_json(&format!("/workspaces/{ws}/loom/canvas-boards/{board_id}"));
    let placements = board_state["placements"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        placements
            .iter()
            .any(|p| p["placed_block_id"].as_str() == Some(block_id.as_str())),
        "IC-03: the placed block appears in the canvas board"
    );
    let parsed = created_canvas_placement_from_response(&placement)
        .expect("IC-03: native canvas parser accepts the persisted placement response");
    let placement_id = parsed.placement_id.clone();
    let mut native_board = LoomCanvasBoard::new(ws.clone(), board_id.clone());
    native_board.set_board(
        vec![CanvasPlacementCard::new(
            parsed.placement_id,
            parsed.placed_block_id,
            parsed.x as f32,
            parsed.y as f32,
            parsed.w as f32,
            parsed.h as f32,
        )],
        Vec::new(),
        egui::Vec2::ZERO,
        1.0,
    );
    assert_eq!(
        native_board.placements.len(),
        1,
        "IC-03: the same persisted placement is loaded into the shipped native canvas state"
    );
    assert_eq!(
        native_board.placements[0].placed_block_id, block_id,
        "IC-03: native canvas keeps the moodboard as a Loom-block reference"
    );
    let board_event_id = board_state["board"]["event_ledger_event_id"]
        .as_str()
        .expect("IC-03: canvas board exposes its EventLedger receipt")
        .to_owned();
    let negative_status = be.get_status(&format!(
        "/workspaces/{ws}/loom/canvas-boards/CANVAS-ic03-missing"
    ));
    assert_eq!(negative_status, 404, "IC-03: missing board fails closed");
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{block_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{board_id}"));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "moodboard_block_id": block_id,
        "canvas_block_id": board_id,
        "placement_id": placement_id,
        "event_ledger_event_id": board_event_id,
        "negative_missing_board_status": negative_status,
    }));
    println!("IC-03 LIVE-PG PASS: typed CKC moodboard persisted and placed on a canvas");
}

#[test]
fn interconnect_ic04_ckc_character_wikilink_backlink() {
    let attempt = ScenarioAttempt::begin("IC-04");
    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    let character = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": "IC-04 character", "content_type": "ckc_character" }),
    );
    let character_block_id = character["block_id"]
        .as_str()
        .or_else(|| character["id"].as_str())
        .expect("requires_pg: character block id")
        .to_owned();
    assert_eq!(character["content_type"], "ckc_character");
    // A Loom-block wikilink uses the canonical `note` hsLink ref kind and the
    // exact target block id. CKC remains the target block's content type; a
    // `character` ref kind would encode `character:<id>` as an embed identity
    // rather than the block-level wikilink required by IC-04.
    let doc = doc_with_ckc_embed("note", &character_block_id, "Aria");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-04 note",
            "content_json": to_content_json_value(&BlockNode::doc(vec![BlockNode::paragraph("character reference pending")])) }),
    );
    let doc_id = created_doc_id(&created);
    let version = created_doc_version(&created);
    // The note's source Loom block id (the backlink source). The create response wraps the doc under
    // `document`; the loom block id is the rich_document_id-keyed block (fall back to the doc id).
    let note_block_id = created
        .get("document")
        .and_then(|d| d.get("block_id").or_else(|| d.get("loom_block_id")))
        .and_then(|v| v.as_str())
        .unwrap_or(&doc_id)
        .to_owned();
    assert_eq!(
        note_block_id, doc_id,
        "IC-04: KnowledgeRichDocument uses its canonical same-id Loom projection"
    );
    // Save through the production SaveManager + RichDocSaveBackend mounted by the native editor. The
    // create was intentionally link-free, so only this production save can create the backlink.
    let saved = save_rich_document_via_production_manager(
        &be,
        &doc_id,
        version as u64,
        to_content_json_value(&doc),
    );
    // The backlink the save registers: GET backlinks of the character block contains the note's block id.
    let backlinks = be.get_json(&format!(
        "/workspaces/{ws}/loom/blocks/{character_block_id}/backlinks"
    ));
    let found = backlinks
        .as_array()
        .map(|a| {
            a.iter().any(|b| {
                b.pointer("/edge/source_block_id").and_then(|v| v.as_str())
                    == Some(note_block_id.as_str())
                    && b.pointer("/edge/target_block_id").and_then(|v| v.as_str())
                        == Some(character_block_id.as_str())
                    && b.pointer("/source_block/block_id").and_then(|v| v.as_str())
                        == Some(note_block_id.as_str())
            })
        })
        .unwrap_or(false);
    assert!(
        found,
        "IC-04: the note's block id appears as a backlink of the character block"
    );
    assert_eq!(
        saved.backlinks_persisted, 1,
        "IC-04: production SaveManager persists the character backlink"
    );
    let save_event_id = saved.save_receipt_event_id;
    let negative_status = be.get_status(&format!(
        "/workspaces/{ws}/loom/blocks/CKC-ic04-missing/backlinks"
    ));
    assert_eq!(
        negative_status, 404,
        "IC-04: missing character fails closed"
    );
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let _ = be.delete(&format!(
        "/workspaces/{ws}/loom/blocks/{character_block_id}"
    ));
    be.assert_cleanup();
    attempt.pass(serde_json::json!({
        "workspace_id": ws,
        "character_block_id": character_block_id,
        "document_id": doc_id,
        "note_block_id": note_block_id,
        "event_ledger_event_id": save_event_id,
        "negative_missing_character_status": negative_status,
    }));
    println!("IC-04 LIVE-PG PASS: typed CKC character backlink persisted and reloaded");
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge1() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 1)");
}
