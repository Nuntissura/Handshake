//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 1: CKC media -> note embed (IC-01..IC-05).
//!
//! This suite proves the CKC<->note/canvas/Stage melt-together edge. Per the KERNEL_BUILDER honest split
//! (the FINAL MT, same split as MT-044/045): the MELT-TOGETHER SUBSTRATE scenario IC-05 (route a note
//! selection to the Stage pane over the ONE shared InteractionBus) is PROVABLE NOW in-process and is GREEN
//! here; the BACKEND-PERSISTENCE scenarios IC-01/02/03/04 bind handshake_core (assets / knowledge documents
//! / loom blocks / canvas placements / backlinks) and need a LIVE managed PostgreSQL, so they are
//! `#[ignore]` + `requires_pg` (the routes EXIST; there is no managed PG in the headless suite; they are
//! NEVER mocked and NEVER faked PASS). For IC-01/02/04 the in-process half ALSO proves the embed atom's
//! `content_json` SHAPE round-trips structurally (the hsLink atom the backend persists), so the PG half is
//! the durable save/reload, not the whole proof.
//!
//! CTRL-8 (RISK-8) VERIFY-OR-BLOCKER (read-only, 2026-06-26): the `ckc_moodboard` (IC-03) and
//! `ckc_character` (IC-04) content_type values are NOT in the backend `LoomBlockContentType` enum
//! (`src/backend/handshake_core/src/storage/loom.rs` defines only note/file/annotated_file/tag_hub/journal/
//! canvas/view_def). Per CTRL-8 those scenarios surface a TYPED BLOCKER and use a generic `note` content_type
//! fallback, marked PARTIAL in the manifest — NO backend edit (src/backend/** is reuse-only). The CKC embed
//! atom (the note-side hsLink) is unaffected: it is an inline `hsLink` ref_kind, not a block content_type.
//!
//! ## Artifact hygiene (CX-212E, HARD)
//! No artifact is ever written under `src/`. The hygiene guard fails the run on a repo-local artifact dir.

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;

use handshake_native::interop::{
    build_from_selection, route_to_stage, EditorSurfaceKind, InteractionBus, SharedSelection,
    CMD_ROUTE_TO_STAGE,
};
use handshake_native::pane_registry::PaneId;
use handshake_native::rich_editor::document_model::doc_json::{
    from_json_string, to_content_json_value,
};
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::stage_pane::{StageContent, StagePane, STAGE_ROUTED_CONTENT_AUTHOR_ID};
use handshake_native::theme::HsTheme;

use interconnect_support::{
    assert_no_local_artifact_dir, author_node_value, mark_status, require_live_backend,
};

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

/// Build a one-paragraph doc carrying a single CKC embed hsLink atom (ref_kind/ref_value as named). This
/// is the SAME inline `hsLink` node the backend persists in `content_json` (NOT an invented embed node).
fn doc_with_ckc_embed(ref_kind: &str, ref_value: &str, label: &str) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("embed ")));
    para.children
        .push(Child::HsLink(HsLinkNode::new(ref_kind, ref_value, label)));
    para.children.push(Child::Text(TextLeaf::new("")));
    BlockNode::doc(vec![para])
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
// IC-05 — SUBSTRATE PROOF (PASS, no PG): route a note SELECTION to the Stage pane over the ONE shared bus.
// The melt-together claim is that the selection travels through the SAME InteractionBus route-to-stage
// command (MT-033) into ONE StagePane, whose AccessKit surface then carries the selected text. The
// contract names the AccessKit author_id `stage.selection.preview`; that id is NOT registered in the crate
// (verified read-only across src/ — the real registered ids are `stage-pane` / `stage-routed-content` from
// MT-033/MT-066). Per the IC-05 impl note we assert the REAL registered id (NOT a stale-label assert) and
// record the contract-name discrepancy as a typed note here.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic05_route_selection_to_stage() {
    const SELECTED: &str = "the routed melt-together selection";
    // The Stage pane the shell drain fills (host-held, the production wiring point).
    let stage = Arc::new(Mutex::new(StagePane::new()));
    let stage_h = Arc::clone(&stage);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 420.0))
        .build_ui(move |ui| {
            // The production per-frame shell drain: pull any staged content off the SHARED bus into the
            // Stage pane, then render the round-trip surface (which emits the stage AccessKit nodes).
            let bus = InteractionBus::get_or_init(ui.ctx());
            InteractionBus::with_try_lock(&bus, |b| {
                if let Some(content) = b.take_pending_stage_content() {
                    stage_h.lock().unwrap().receive_routed_content(content);
                }
            });
            let pal = HsTheme::Dark.palette();
            stage_h.lock().unwrap().show_round_trip(ui, &pal);
        });
    harness.run();

    // Before routing: the routed-content region summarizes "nothing routed".
    let before = author_node_value(&harness, STAGE_ROUTED_CONTENT_AUTHOR_ID).unwrap_or_default();
    assert!(
        before.contains("nothing routed"),
        "IC-05: stage starts empty (got {before:?})"
    );

    // The route originates from a rich-text SELECTION published on the SHARED bus. Build the route payload
    // from that exact SharedSelection (the MT-033/MT-066 builder), then route it over the SAME bus command.
    let selection = SharedSelection::TextRange {
        pane_id: pane("pane-rich"),
        surface: EditorSurfaceKind::RichText,
        start: 0,
        end: SELECTED.len(),
        text: SELECTED.to_owned(),
    };
    let payload = build_from_selection(&selection, "ws-mt046").expect("a routable text selection");

    let bus = InteractionBus::get_or_init(&harness.ctx);
    let dispatched = InteractionBus::with_try_lock(&bus, |b| {
        b.register_route_to_stage_command();
        assert!(
            b.commands().get(CMD_ROUTE_TO_STAGE).is_some(),
            "IC-05: route-to-stage cmd registered"
        );
        // route_to_stage stages the StageContent on the bus + dispatches the EXISTING command (reuse).
        route_to_stage(&harness.ctx, b, &payload)
            .map(|ack| ack.staged)
            .unwrap_or(false)
    })
    .unwrap_or(false);
    assert!(
        dispatched,
        "IC-05: the route-to-stage command dispatched over the shared bus"
    );

    // Drain + render frames so the shell pulls the staged content into the Stage pane and the AccessKit
    // tree refreshes (Harness::run advances a layout-level frame — the established flush mechanism; the
    // contract-named `flush_pending_updates()` does NOT exist in the crate, CTRL-3).
    harness.run();
    harness.run();

    // The Stage pane now carries the routed selection. Assert the REAL registered AccessKit surface value
    // matches the selected text (the contract's `stage.selection.preview` is unregistered — typed note).
    let routed_value = author_node_value(&harness, STAGE_ROUTED_CONTENT_AUTHOR_ID)
        .expect("IC-05: the stage-routed-content AccessKit node must be present after routing");
    assert!(
        routed_value.contains(SELECTED),
        "IC-05: the Stage AccessKit surface label/value matches the routed selection text \
         (got {routed_value:?}; contract-named id `stage.selection.preview` is unregistered — asserting \
         the REAL `stage-routed-content` id instead, not a stale label)"
    );
    // And the Stage pane state holds the routed selection (the in-memory landing).
    match &stage.lock().unwrap().content {
        StageContent::Selection(text, _) => assert!(text.contains(SELECTED)),
        other => panic!("IC-05: expected a routed Selection, got {other:?}"),
    }

    mark_status("IC-05", "PASS");
    assert_no_local_artifact_dir();
    println!(
        "IC-05 SUBSTRATE PASS: route-to-stage over the shared InteractionBus landed the selection in ONE \
         StagePane; AccessKit `stage-routed-content` value carries the selected text. backlink.confirmed=na"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-01 / IC-02 / IC-04 — content_json SHAPE half (PASS, no PG): the CKC embed atom is an inline `hsLink`
// node (refKind=HS_images / video / character) that ROUND-TRIPS the backend content_json. This is the
// structural half the durable PG save/reload (the #[ignore] requires_pg proofs below) builds on — it proves
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
// IC-01..IC-04 — BACKEND-PERSISTENCE half: #[ignore] + requires_pg. The routes EXIST (verified against the
// real handshake_core route table, 2026-06-26); the durable save/reload/placement/backlink round-trips need
// a LIVE managed PostgreSQL. NEVER mocked, NEVER faked.
// VERIFIED REAL ROUTES (the route-shape drift the review flagged is corrected here):
//   - asset-create = POST /workspaces/{ws}/loom/import (loom.rs:217 import_loom_asset -> create_asset);
//     there is NO bare POST /workspaces/{ws}/assets route (only GET /assets/{id}[/content|/thumbnail|/tiers]).
//   - knowledge docs are merged BARE (no /workspaces prefix): POST /knowledge/documents (workspace_id in
//     body), GET /knowledge/documents/{id}, PUT /knowledge/documents/{id}/save ({expected_version,content_json}).
//   - the create response wraps the doc: { "document": { rich_document_id, doc_version, .. }, .. }.
// Run with: cargo test -p handshake-native --test test_interconnect_ckc_to_note -- --ignored
//   (set HSK_TEST_BASE + HSK_TEST_WORKSPACE_ID; the operator seeds the workspace).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "requires_pg: live Handshake-managed PostgreSQL + seeded workspace (HSK_TEST_WORKSPACE_ID). \
            Drag CKC image into note: POST /workspaces/{ws}/loom/import (the managed asset-create path), \
            POST /knowledge/documents, PUT /knowledge/documents/{doc_id}/save, reload returns the hsLink \
            node with the asset id; GET /workspaces/{ws}/assets/{id} == 200. Never mocks PG."]
fn interconnect_ic01_ckc_image_into_note() {
    let be = require_live_backend();
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
    let _ = be.put_json(
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
    // Idempotent cleanup (DropGuard-style best-effort).
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    mark_status("IC-01", "PASS");
    println!("IC-01 LIVE-PG PASS: CKC image embedded + reloaded with asset {asset_id}; GET /assets == 200");
}

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace. Drag CKC video into note (content_type \
            video/mp4) via POST /workspaces/{ws}/loom/import, POST /knowledge/documents, PUT \
            /knowledge/documents/{doc_id}/save: saved doc carries an hsLink node refKind=video. Never mocks PG."]
fn interconnect_ic02_ckc_video_into_note() {
    let be = require_live_backend();
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
    let _ = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&doc) }),
    );
    let reloaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let (rk, _rv) =
        first_hs_link(&loaded_content_json(&reloaded)).expect("reloaded doc carries an hsLink");
    assert_eq!(rk, "video", "IC-02: reloaded embed is the video hsLink");
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    mark_status("IC-02", "PASS");
    println!("IC-02 LIVE-PG PASS: CKC video embedded as an hsLink(video) and reloaded");
}

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace. Drag CKC moodboard card onto canvas. \
            CTRL-8 TYPED BLOCKER: ckc_moodboard is NOT a LoomBlockContentType variant in the backend \
            (verified 2026-06-26) — uses a `note` content_type fallback, scenario is PARTIAL. Never mocks PG."]
fn interconnect_ic03_ckc_moodboard_on_canvas() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();
    // CTRL-8: the contract's `ckc_moodboard` content_type is absent from the backend LoomBlockContentType
    // enum, so we create the block with the `note` fallback and mark the scenario PARTIAL (typed blocker).
    let block = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": "IC-03 moodboard (note fallback)", "content_type": "note" }),
    );
    let block_id = block["block_id"]
        .as_str()
        .or_else(|| block["id"].as_str())
        .expect("requires_pg: block id")
        .to_owned();
    // Create a canvas board and place the block on it.
    let board = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": "IC-03 canvas", "content_type": "canvas" }),
    );
    let board_id = board["block_id"]
        .as_str()
        .or_else(|| board["id"].as_str())
        .expect("requires_pg: board id")
        .to_owned();
    let _ = be.post_json(
        &format!("/workspaces/{ws}/loom/canvas-boards/{board_id}/placements"),
        &serde_json::json!({ "block_id": block_id, "x": 100.0, "y": 100.0 }),
    );
    let board_state = be.get_json(&format!("/workspaces/{ws}/loom/canvas-boards/{board_id}"));
    let placements = board_state["placements"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        placements
            .iter()
            .any(|p| p["block_id"].as_str() == Some(block_id.as_str())),
        "IC-03: the placed block appears in the canvas board"
    );
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{block_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{board_id}"));
    // PARTIAL, not PASS: the ckc_moodboard content type is a backend gap (typed blocker).
    mark_status("IC-03", "PARTIAL");
    println!(
        "IC-03 LIVE-PG PARTIAL: moodboard placed on canvas via the `note` fallback (CTRL-8 typed blocker: \
         ckc_moodboard absent from LoomBlockContentType — no backend edit in this MT)"
    );
}

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace. CKC character referenced in note registers a \
            backlink. CTRL-8 TYPED BLOCKER: ckc_character is NOT a LoomBlockContentType variant (verified \
            2026-06-26) — uses a `note` content_type fallback, PARTIAL. Never mocks PG."]
fn interconnect_ic04_ckc_character_wikilink_backlink() {
    let be = require_live_backend();
    let ws = be.workspace_id.clone();
    // CTRL-8: `ckc_character` absent -> `note` fallback (typed blocker).
    let character = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({ "title": "IC-04 character (note fallback)", "content_type": "note" }),
    );
    let character_block_id = character["block_id"]
        .as_str()
        .or_else(|| character["id"].as_str())
        .expect("requires_pg: character block id")
        .to_owned();
    // The note carries a character ref hsLink (ref_value = the character block id); save it.
    let doc = doc_with_ckc_embed("character", &character_block_id, "Aria");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-04 note",
            "content_json": to_content_json_value(&doc) }),
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
    // Explicitly save via the REAL /save route — the save is what registers the backlink server-side.
    let _ = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&doc) }),
    );
    // The backlink the save registers: GET backlinks of the character block contains the note's block id.
    let backlinks = be.get_json(&format!(
        "/workspaces/{ws}/loom/blocks/{character_block_id}/backlinks"
    ));
    let found = backlinks
        .as_array()
        .map(|a| {
            a.iter().any(|b| {
                b["source_block_id"].as_str() == Some(note_block_id.as_str())
                    || b["block_id"].as_str() == Some(note_block_id.as_str())
            })
        })
        .unwrap_or(false);
    assert!(
        found,
        "IC-04: the note's block id appears as a backlink of the character block"
    );
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let _ = be.delete(&format!(
        "/workspaces/{ws}/loom/blocks/{character_block_id}"
    ));
    mark_status("IC-04", "PARTIAL");
    println!(
        "IC-04 LIVE-PG PARTIAL: backlink confirmed character<-note via the `note` fallback (CTRL-8 typed \
         blocker: ckc_character absent from LoomBlockContentType). backlink.confirmed=true"
    );
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge1() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 1)");
}
