//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 4: shared undo + event-ledger across surfaces (IC-15..IC-18).
//!
//! IC-15/IC-16/IC-18 are MELT-TOGETHER SUBSTRATE proofs that are PROVABLE NOW in-process (no PG): they prove
//! the code editor and the rich-text editor share ONE undo stack — the SAME `Arc<Mutex<InteractionBus>>`
//! undo scope instance — so an undo reverts an edit recorded by either surface, and the per-pane scope policy
//! (POLICY-1 local-first) holds. The LOAD-BEARING anti-RISK-1/anti-RISK-4 control (CTRL-1/CTRL-4): each
//! cross-surface test uses ONE shared bus instance for BOTH surfaces (NOT two independent undo stacks), and
//! IC-18 edits A then B and asserts ONE undo reverts ONLY B (the most-recently-edited surface) leaving A
//! unchanged — inspecting BOTH surfaces after the single undo.
//!
//! IC-17 (event-ledger) binds the Flight Recorder backend and needs a LIVE managed PostgreSQL, so it is
//! `#[ignore]` + `requires_pg`. CTRL-7 VERIFY-OR-BLOCKER (read-only, 2026-06-26): the `GET /events` ledger
//! query endpoint EXISTS (`src/backend/handshake_core/src/api/flight_recorder.rs:74` routes BARE `/events` +
//! `/flight_recorder` -> `list_events`), so IC-17 is requires_pg, NOT BLOCKED. ROUTE SHAPE (corrected): the
//! route is BARE `/events` (NO `/workspaces` prefix), filtered by the `wsid` QUERY param, and returns a bare
//! `Vec<FlightEvent>` (no `events` wrapper). Knowledge-doc saves use the BARE `/knowledge/documents/{id}/save`
//! route ({expected_version,content_json}); the create response wraps the doc under `document.rich_document_id`.
//!
//! Artifact hygiene (CX-212E): no artifact under `src/`.

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use std::sync::{Arc, Mutex};

use handshake_native::code_editor::panel::CodeEditorPanel;
use handshake_native::interop::InteractionBus;
use handshake_native::pane_registry::PaneId;
use handshake_native::rich_editor::interop_adapter::{push_rich_edit_undo, RichSnapshotApplier};

use interconnect_support::{assert_no_local_artifact_dir, mark_status, require_live_backend};

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

/// A rich snapshot applier that restores a `String` doc from its content_json snapshot (the test-doc shape).
fn string_restore() -> RichSnapshotApplier<String> {
    Arc::new(|s: &mut String, snap| {
        *s = snap.as_str().unwrap_or_default().to_owned();
    })
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-15 — Undo crosses rich editor edit (SUBSTRATE PASS): a rich-text edit (insert EDIT_A) recorded on the
// ONE shared bus undo scope is reverted by one undo so the content no longer contains EDIT_A. The SAME
// `Arc<Mutex<InteractionBus>>` instance holds the scope (CTRL-1). The backend version-revert half is
// requires_pg (the in-process scope proves the undo stack reverts the rich edit; PG proves the durable save
// rolls back).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic15_undo_rich_editor() {
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    let rich_pane = pane("pane-rich");

    // The real rich doc state (a String standing in for the content_json the adapter snapshots).
    let rich_doc = Arc::new(Mutex::new(String::from("base")));
    let before = "base".to_owned();
    let after = "base EDIT_A".to_owned();

    // Record the rich edit on the SHARED bus's unified undo scope via the REAL adapter, then apply the edit.
    {
        let mut b = bus.lock().unwrap();
        push_rich_edit_undo(
            &mut b,
            rich_pane.clone(),
            &rich_doc,
            serde_json::json!(before),
            serde_json::json!(after),
            string_restore(),
            "rich: insert EDIT_A",
        );
        b.set_focus_owner(rich_pane.clone());
    }
    *rich_doc.lock().unwrap() = after.clone();
    assert!(rich_doc.lock().unwrap().contains("EDIT_A"), "IC-15: the edit applied (EDIT_A present)");
    assert_eq!(bus.lock().unwrap().local_undo_count(&rich_pane), 1, "IC-15: one entry on the shared scope");

    // One undo on the SAME shared bus reverts the rich edit so EDIT_A is gone.
    let result = bus.lock().unwrap().undo(&rich_pane).expect("an action to undo");
    assert!(result.ok, "IC-15: the undo applied: {result:?}");
    assert!(
        !rich_doc.lock().unwrap().contains("EDIT_A"),
        "IC-15: after Ctrl+Z the rich content no longer contains EDIT_A (got {:?})",
        *rich_doc.lock().unwrap()
    );

    mark_status("IC-15", "PASS");
    assert_no_local_artifact_dir();
    println!("IC-15 SUBSTRATE PASS: EDIT_A absent after one undo on the shared bus undo scope");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-16 — Undo crosses code editor edit (SUBSTRATE PASS): a code-buffer edit (insert CODE_EDIT) recorded on
// the ONE shared bus undo scope is reverted by one undo so the buffer no longer contains CODE_EDIT. Uses the
// REAL CodeEditorPanel + push_code_edit_undo adapter (the real rope set_text restore). The PG version-revert
// half is requires_pg.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic16_undo_code_editor() {
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    let code_pane = pane("pane-code");
    let code_panel = Arc::new(CodeEditorPanel::new("fn main() {}\n", "rs"));

    // Snapshot before, apply a real edit (insert CODE_EDIT at line 1), record on the SHARED scope.
    let before = code_panel.buffer();
    code_panel.set_text("CODE_EDIT\nfn main() {}\n");
    let after = code_panel.buffer();
    assert!(code_panel.buffer().to_string().contains("CODE_EDIT"), "IC-16: the code edit applied");
    {
        let mut b = bus.lock().unwrap();
        handshake_native::code_editor::interop_adapter::push_code_edit_undo(
            &mut b,
            code_pane.clone(),
            &code_panel,
            before.clone(),
            after.clone(),
            "code: insert CODE_EDIT",
        );
        b.set_focus_owner(code_pane.clone());
    }
    assert_eq!(bus.lock().unwrap().local_undo_count(&code_pane), 1, "IC-16: one entry on the shared scope");

    // One undo on the SAME shared bus reverts the code edit so CODE_EDIT is gone.
    let result = bus.lock().unwrap().undo(&code_pane).expect("an action to undo");
    assert!(result.ok, "IC-16: the undo applied: {result:?}");
    assert!(
        !code_panel.buffer().to_string().contains("CODE_EDIT"),
        "IC-16: after Ctrl+Z the code buffer no longer contains CODE_EDIT (got {:?})",
        code_panel.buffer().to_string()
    );
    assert_eq!(
        code_panel.buffer().to_string(),
        before.to_string(),
        "IC-16: the buffer is restored to its pre-edit state"
    );

    mark_status("IC-16", "PASS");
    assert_no_local_artifact_dir();
    println!("IC-16 SUBSTRATE PASS: CODE_EDIT reverted after one undo on the shared bus undo scope");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-18 — Shared undo stack policy: scoped per-pane vs. global (SUBSTRATE PASS, the load-bearing CTRL-4
// proof). Open two panes (rich note A, code file B). Edit BOTH. Trigger undo ONCE. Assert the undo reverts
// the MOST RECENTLY EDITED surface (B), NOT both: if the code edit was last, the code buffer reverts and the
// note is unchanged. This proves the SAME shared undo stack respects the per-pane scope policy (POLICY-1
// local-first). The test uses ONE `Arc<Mutex<InteractionBus>>` shared by both surfaces and inspects BOTH
// surfaces after one undo (CTRL-4) — NOT two independent undo stacks.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic18_undo_scope_policy() {
    // ONE shared bus instance for BOTH surfaces (CTRL-4 — not two independent stacks).
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    let rich_pane = pane("pane-rich-A");
    let code_pane = pane("pane-code-B");

    // Surface A = rich note. Edit it FIRST.
    let rich_doc = Arc::new(Mutex::new(String::from("noteA-base")));
    {
        let mut b = bus.lock().unwrap();
        push_rich_edit_undo(
            &mut b,
            rich_pane.clone(),
            &rich_doc,
            serde_json::json!("noteA-base"),
            serde_json::json!("noteA-EDITED"),
            string_restore(),
            "rich: edit A",
        );
    }
    *rich_doc.lock().unwrap() = "noteA-EDITED".to_owned();

    // Surface B = code file. Edit it SECOND (the MOST RECENTLY edited surface).
    let code_panel = Arc::new(CodeEditorPanel::new("let b = 0;\n", "rs"));
    let code_before = code_panel.buffer();
    code_panel.set_text("let b = 999;\n");
    let code_after = code_panel.buffer();
    {
        let mut b = bus.lock().unwrap();
        handshake_native::code_editor::interop_adapter::push_code_edit_undo(
            &mut b,
            code_pane.clone(),
            &code_panel,
            code_before.clone(),
            code_after.clone(),
            "code: edit B",
        );
        // The most-recently-edited surface holds focus (the per-pane scope authority — POLICY-1).
        b.set_focus_owner(code_pane.clone());
    }

    // Snapshots BEFORE the single undo (to prove A is untouched after).
    let note_before_undo = rich_doc.lock().unwrap().clone();
    let code_before_undo = code_panel.buffer().to_string();
    assert_eq!(note_before_undo, "noteA-EDITED", "IC-18: note A was edited");
    assert!(code_before_undo.contains("999"), "IC-18: code B was edited (most recent)");

    // ONE undo on the focused (most-recently-edited) pane B.
    let result = bus.lock().unwrap().undo(&code_pane).expect("an undo on the focused pane B");
    assert!(result.ok, "IC-18: the single undo applied: {result:?}");

    // INSPECT BOTH surfaces after the one undo (CTRL-4):
    //  - B (the most recently edited) is reverted.
    let code_after_undo = code_panel.buffer().to_string();
    assert_eq!(
        code_after_undo,
        code_before.to_string(),
        "IC-18: ONE undo reverted the MOST RECENTLY edited surface (code B): got {code_after_undo:?}"
    );
    //  - A (the note) is UNCHANGED (POLICY-1 per-pane scope — the undo did not touch A's ring).
    assert_eq!(
        *rich_doc.lock().unwrap(),
        "noteA-EDITED",
        "IC-18: the OTHER surface (note A) is UNCHANGED after the single undo (per-pane scope policy)"
    );
    // The code ring drained; the note ring still has its entry (proving they are distinct per-pane rings on
    // the ONE shared scope, not a single global stack).
    {
        let b = bus.lock().unwrap();
        assert_eq!(b.local_undo_count(&code_pane), 0, "IC-18: the code pane's ring drained");
        assert_eq!(b.local_undo_count(&rich_pane), 1, "IC-18: the note pane's ring is untouched");
    }

    mark_status("IC-18", "PASS");
    assert_no_local_artifact_dir();
    println!(
        "IC-18 SUBSTRATE PASS: scope correct — ONE undo on the shared stack reverted ONLY the most recently \
         edited surface (code B); note A unchanged (per-pane scope policy)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-17 — Event-ledger records cross-surface actions (requires_pg). CTRL-7: GET /events EXISTS (Flight
// Recorder list_events). A sequence (insert text, save, insert wikilink, save again) produces >=2
// KNOWLEDGE_RICH_DOCUMENT_SAVED events in the ledger, in order; the second event's payload carries the
// wikilink block reference. Run with: cargo test -p handshake-native --test test_interconnect_shared_undo_ledger -- --ignored
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "requires_pg: live PostgreSQL + seeded workspace (HSK_TEST_WORKSPACE_ID). GET /events?wsid={ws} \
            (the BARE Flight Recorder list_events route, no /workspaces prefix) returns >= 2 \
            KNOWLEDGE_RICH_DOCUMENT_SAVED events for the note in order; the second carries the wikilink ref. \
            CTRL-7: the endpoint EXISTS (verified). Never mocks PG."]
fn interconnect_ic17_event_ledger_records() {
    use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;
    use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind, TextLeaf};

    let be = require_live_backend();
    let ws = be.workspace_id.clone();

    // (1) create a note, (2) save it (insert text), (3) add a wikilink, (4) save again.
    let plain = BlockNode::doc(vec![BlockNode::paragraph("event ledger note")]);
    // Knowledge docs are merged BARE (no /workspaces prefix): POST /knowledge/documents (workspace_id in
    // body), PUT /knowledge/documents/{id}/save ({expected_version,content_json}); the create response wraps
    // the doc as { "document": { rich_document_id, doc_version, .. }, .. } (verified knowledge_documents.rs).
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-17 note",
            "content_json": to_content_json_value(&plain) }),
    );
    let doc_id = created
        .get("document")
        .and_then(|d| d.get("rich_document_id"))
        .and_then(|v| v.as_str())
        .or_else(|| created.get("rich_document_id").and_then(|v| v.as_str()))
        .or_else(|| created.get("id").and_then(|v| v.as_str()))
        .expect("requires_pg: created document returns a rich_document_id (document.rich_document_id)")
        .to_owned();
    let mut version = created
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| created.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(1);
    let note_block_id = created
        .get("document")
        .and_then(|d| d.get("block_id").or_else(|| d.get("loom_block_id")))
        .and_then(|v| v.as_str())
        .unwrap_or(&doc_id)
        .to_owned();

    // Save #1 (plain) via the REAL /save route.
    let save1 = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&plain) }),
    );
    // Advance the optimistic-concurrency version from the save response so save #2 is accepted.
    version = save1
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| save1.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(version + 1);
    // Save #2 (with a wikilink ref in the body).
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("now links ")));
    para.children.push(Child::HsLink(HsLinkNode::new("file", "linked-block-1", "linked")));
    let with_link = BlockNode::doc(vec![para]);
    let _ = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&with_link) }),
    );

    // The REAL Flight Recorder query route is BARE GET /events (flight_recorder.rs:74; alias /flight_recorder)
    // filtered by the `wsid` QUERY param (NOT a /workspaces path prefix), returning a bare Vec<FlightEvent>
    // (no `events` wrapper) where each event carries event_type/wsids/payload.
    let events = be.get_json(&format!("/events?wsid={ws}"));
    let arr = events.as_array().cloned()
        .or_else(|| events["events"].as_array().cloned()).unwrap_or_default();
    let saved: Vec<&serde_json::Value> = arr.iter().filter(|e| {
        let kind = e["event_type"].as_str().or_else(|| e["kind"].as_str()).unwrap_or("");
        kind.to_uppercase().contains("KNOWLEDGE_RICH_DOCUMENT_SAVED")
            && (e["source_block_id"].as_str() == Some(note_block_id.as_str())
                || e.to_string().contains(&note_block_id)
                || e.to_string().contains(&doc_id))
    }).collect();
    assert!(
        saved.len() >= 2,
        "IC-17: GET /events returns >= 2 KNOWLEDGE_RICH_DOCUMENT_SAVED events for the note (got {})",
        saved.len()
    );
    // AC: the SECOND save event's payload carries the wikilink block reference (linked-block-1). The two
    // saves were issued in order, so the later matching event is the wikilink save. Order the matches by
    // timestamp when present so the assertion does not depend on the server's return order.
    let mut ordered = saved.clone();
    ordered.sort_by_key(|e| e["timestamp"].as_str().unwrap_or("").to_owned());
    let second = ordered.get(1).expect("IC-17: a second KNOWLEDGE_RICH_DOCUMENT_SAVED event");
    assert!(
        second.to_string().contains("linked-block-1"),
        "IC-17: the second save event's payload carries the wikilink block reference (got {second})"
    );

    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    mark_status("IC-17", "PASS");
    println!("IC-17 LIVE-PG PASS: {} KNOWLEDGE_RICH_DOCUMENT_SAVED events recorded for the note", saved.len());
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge4() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 4)");
}
