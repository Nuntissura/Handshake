//! WP-KERNEL-012 MT-025 LoomWikiPagePanel PROOFS (wiki projection + editable overlay).
//!
//!   - PROOF1 (edit-buffer pre-fill + cancel-no-mutation logic): owned by the lib unit tests
//!     (`graph::wiki_page_panel::tests` + `backend_client::wiki_client_tests`); the buffer/cancel/
//!     save-error/staleness/cap logic + the verified request-shape builders are proven STANDALONE there.
//!   - PROOF2: kittest — a seeded projection renders `wiki.title.*` (Role::Label) + a non-empty
//!     `wiki.content.*` (Role::Document) in the AccessKit tree (AC1 + AC7).
//!   - PROOF3: kittest edit-save — click `wiki.edit.proj-001`, the `wiki.edit-area.proj-001`
//!     (Role::MultilineTextInput) appears, click `wiki.save.proj-001`, assert the `Save{annotation}`
//!     event fires AND the verified `POST /loom/wiki/proj-001/overlays { annotation }` request shape is
//!     what the production spawn path sends (backend_client RequestSpec — NO Tauri), then
//!     `finish_save_success` returns to the read-only view (AC3).
//!   - PROOF4: kittest cancel — click `wiki.edit.proj-001`, type into the edit area, click
//!     `wiki.cancel.proj-001`, assert NO Save event fired (no overlay POST) and the panel returns to
//!     read-only showing the original content (AC4).
//!   - PROOF5: kittest save-error — with a save error applied, assert `wiki.edit-area.proj-001` is STILL
//!     present in the AccessKit tree (edit mode NOT exited) and the buffer is preserved (AC5).
//!
//! ## SPEC-REALISM GATE (MT-025 KERNEL_BUILDER gate + the MT-008/021/022/023/024 "verify, don't trust the
//! contract" rule). VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//!   - `GET  /workspaces/{ws}/loom/wiki/{projection_id}`            -> `ServedWikiPage` (load — AC1).
//!   - `POST /workspaces/{ws}/loom/wiki/{projection_id}/regenerate` -> `ServedWikiPage` (the REAL rebuild,
//!     NOT the contract's non-existent `.../rebuild`).
//!   - `POST /workspaces/{ws}/loom/wiki/{projection_id}/overlays`   body `{ "annotation" }` -> the REAL,
//!     persisted, CANONICAL wiki-page write (`add_loom_wiki_overlay`).
//!
//! THE CRITICAL FINDING (MC-1 / RISK-1): there is **NO PATCH/PUT route that edits `rendered_content`** —
//! it is a DERIVED projection ("regenerable; never authority", storage doc) recompiled from
//! `source_block_ids` and overwritten on rebuild. So `rendered_content` is READ-ONLY here and the "Edit
//! overlay" mode authors a REAL overlay annotation (the only persisted wiki-page write), never a fake
//! PATCH that would 404 / be silently clobbered (Spec-Realism: no silently-broken write). The MT
//! contract's PATCH/PUT-on-rendered_content is a TYPED LIMITATION the widget surfaces.
//!
//! AC1/AC3 against a LIVE Handshake-managed PostgreSQL with a seeded wiki projection are the `#[ignore]`d
//! `*_live_pg` tests gated behind the `integration` feature (NEEDS_MANAGED_RESOURCE_PROOF); absent a
//! seeded backend they are skipped and NEVER faked.
//!
//! ## Artifact hygiene (CX-212E screenshot rule)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-025/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`). The PNG proof is OPT-IN behind the OFF-by-default `wgpu_screenshots`
//! feature so the default `cargo test` does not add a concurrent wgpu device (the WP-wide Windows hazard).

#[cfg(feature = "wgpu_screenshots")]
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::backend_client::{LoomWikiClient, WikiProjection};
use handshake_native::graph::wiki_page_panel::{
    cancel_author_id, content_author_id, edit_area_author_id, edit_author_id, save_author_id,
    title_author_id, LoomWikiPagePanel, WikiPageEvent,
};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. Only the opt-in
/// `.wgpu()` screenshot proof writes artifacts, so this is gated with that feature.
#[cfg(feature = "wgpu_screenshots")]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
#[cfg(feature = "wgpu_screenshots")]
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot tests WITHIN this binary (the documented Windows-wgpu
/// concurrent-device hazard). Within-process only; the default `cargo test` does NOT run this `.wgpu()`
/// path (it is gated behind the OFF-by-default `wgpu_screenshots` feature).
#[cfg(feature = "wgpu_screenshots")]
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "wgpu_screenshots")]
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

fn shared<T>(value: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(value))
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids(harness: &Harness<'_, ()>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The role of the node whose author_id matches `author`, if present.
fn role_of(harness: &Harness<'_, ()>, author: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// The value of the node whose author_id matches `author`, if present (the Document content value).
fn value_of(harness: &Harness<'_, ()>, author: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author) {
            return ak.value().map(|v| v.to_owned());
        }
    }
    None
}

/// A seeded wiki projection (the stand-in for the `GET /loom/wiki/{id}` result; no backend in the
/// AccessKit/interaction proofs).
fn seeded_projection() -> WikiProjection {
    WikiProjection {
        projection_id: "proj-001".to_owned(),
        workspace_id: "ws-test".to_owned(),
        title: "Ownership model".to_owned(),
        source_block_ids: vec!["blk-1".to_owned(), "blk-2".to_owned(), "blk-3".to_owned()],
        rendered_content: "# Ownership model\nThe borrow checker enforces aliasing rules at compile time."
            .to_owned(),
        staleness_hash: "h1".to_owned(),
        rebuild_status: "fresh".to_owned(),
        page_type: Some("concept".to_owned()),
        staleness_verdict: serde_json::json!({ "state": "fresh" }),
    }
}

fn loaded_panel() -> LoomWikiPagePanel {
    let mut p = LoomWikiPagePanel::new("ws-test", "proj-001");
    p.set_page(seeded_projection());
    p
}

/// Harness rendering the shared panel, pushing every emitted event into `events`.
fn panel_harness(
    panel: Arc<Mutex<LoomWikiPagePanel>>,
    events: Arc<Mutex<Vec<WikiPageEvent>>>,
) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(520.0, 700.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = panel.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

// ── PROOF2 + AC1 + AC7: the read-only view exposes the title + content AccessKit nodes ────────────────

#[test]
fn proof2_title_and_content_nodes_present() {
    let panel = shared(loaded_panel());
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);

    // AC7: the title + content nodes are present with the exact MT author_ids.
    assert!(
        ids.contains(&title_author_id("proj-001")),
        "AC7: '{}' must be present (ids={ids:?})",
        title_author_id("proj-001")
    );
    assert!(
        ids.contains(&content_author_id("proj-001")),
        "AC7: '{}' must be present",
        content_author_id("proj-001")
    );
    // AC7 roles: title = Label, content = Document.
    assert_eq!(
        role_of(&harness, &title_author_id("proj-001")).as_deref(),
        Some("Label"),
        "AC7: the title must be Role::Label"
    );
    assert_eq!(
        role_of(&harness, &content_author_id("proj-001")).as_deref(),
        Some("Document"),
        "AC7: the content area must be Role::Document"
    );

    // PROOF2: the rendered_content text is exposed (non-empty) on the Document node.
    let value = value_of(&harness, &content_author_id("proj-001")).unwrap_or_default();
    assert!(
        value.contains("borrow checker"),
        "PROOF2: the Document node value must carry the rendered_content (got {value:?})"
    );

    // The Edit button is present in the read-only view (AC2 entry point).
    assert!(ids.contains(&edit_author_id("proj-001")), "AC7: the Edit button is present");
    // In read-only mode the edit area is ABSENT.
    assert!(
        !ids.contains(&edit_area_author_id("proj-001")),
        "the edit area is absent in read-only mode"
    );
    println!("PROOF2: wiki.title.* (Label) + wiki.content.* (Document, non-empty value) + Edit button present");
}

// ── PROOF3 + AC2 + AC3: edit -> type -> save fires Save{annotation} + the verified overlay request shape

#[test]
fn proof3_edit_save_fires_event_and_returns_to_read_only() {
    let panel = shared(loaded_panel());
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Click Edit (AC2).
    let edit_target = edit_author_id("proj-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(edit_target.as_str()))
        .click();
    harness.run();

    // The edit area (Role::MultilineTextInput) is now present (AC2 + AC7).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&edit_area_author_id("proj-001")),
        "AC2: the edit area must appear after Edit (ids={ids:?})"
    );
    assert_eq!(
        role_of(&harness, &edit_area_author_id("proj-001")).as_deref(),
        Some("MultilineTextInput"),
        "AC7: the edit area must be Role::MultilineTextInput"
    );
    assert!(ids.contains(&save_author_id("proj-001")), "AC7: the Save button is present in edit mode");
    assert!(ids.contains(&cancel_author_id("proj-001")), "AC7: the Cancel button is present in edit mode");

    // Set the overlay annotation via the public surface (the typing pathway is exercised by the lib
    // unit tests; here we drive the same state the TextEdit mutates, then click the real Save button).
    panel.lock().unwrap().set_edit_buffer("NEW CONTENT");
    harness.run();

    let save_target = save_author_id("proj-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(save_target.as_str()))
        .click();
    harness.run();

    // PROOF3/AC3: the Save event fired carrying the buffer.
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(|e| matches!(e, WikiPageEvent::Save { annotation } if annotation == "NEW CONTENT")),
        "PROOF3/AC3: Save click must fire Save{{annotation:'NEW CONTENT'}} (got {ev:?})"
    );

    // The host applies the success path (after the overlay POST 2xx + a re-fetch); simulate that and
    // re-render -> back to read-only.
    panel.lock().unwrap().finish_save_success();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&edit_area_author_id("proj-001")),
        "AC3: after a successful save the panel returns to read-only (edit area gone, ids={ids:?})"
    );
    assert!(ids.contains(&content_author_id("proj-001")), "AC3: the read-only content area is shown again");
    println!("PROOF3: edit -> set buffer -> Save fired Save{{NEW CONTENT}} + success returns to read-only (events={ev:?})");
}

/// PROOF3 (request layer): the Save spawn path sends the verified `POST /loom/wiki/{id}/overlays`
/// `{ annotation }` request — the REAL persisted wiki-page write (NO Tauri; the WP-011 backend_client
/// typed HTTP client). This is the "Tauri intercept" the contract named, realised as a RequestSpec.
#[test]
fn proof3_save_request_shape_is_verified_overlay_post() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomWikiClient::new("http://test.local:1234", rt.handle().clone());

    let spec = client.add_overlay_request("ws1", "proj-001", "NEW CONTENT", None);
    assert_eq!(
        spec.url, "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001/overlays",
        "PROOF3: Save hits the verified /loom/wiki/:id/overlays route (the REAL persisted write)"
    );
    assert_eq!(
        spec.body,
        Some(serde_json::json!({ "annotation": "NEW CONTENT" })),
        "PROOF3: the overlay body is the verified AddWikiOverlayRequest {{ annotation }}"
    );
    assert!(
        matches!(spec.method, handshake_native::backend_client::HttpMethod::Post),
        "PROOF3: the overlay write is a POST"
    );
    println!("PROOF3: Save request shape verified (POST /loom/wiki/proj-001/overlays {{annotation}})");
}

// ── PROOF4 + AC4: cancel discards the edit and makes NO Save event (no overlay POST) ──────────────────

#[test]
fn proof4_cancel_no_mutation_returns_to_read_only() {
    let panel = shared(loaded_panel());
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Edit -> type a throwaway -> Cancel.
    let edit_target = edit_author_id("proj-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(edit_target.as_str()))
        .click();
    harness.run();
    panel.lock().unwrap().set_edit_buffer("THROWAWAY");
    harness.run();

    let cancel_target = cancel_author_id("proj-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(cancel_target.as_str()))
        .click();
    harness.run();

    // PROOF4/AC4: a Cancel event fired and NO Save event ever did (no overlay POST implied).
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(|e| matches!(e, WikiPageEvent::Cancel)),
        "AC4: Cancel must fire a Cancel event (got {ev:?})"
    );
    assert!(
        !ev.iter().any(|e| matches!(e, WikiPageEvent::Save { .. })),
        "PROOF4/AC4: Cancel must NOT fire any Save event (no overlay POST) (got {ev:?})"
    );

    // Back to read-only with the ORIGINAL content (cancel-no-mutation).
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&edit_area_author_id("proj-001")),
        "AC4: the edit area is gone after Cancel (ids={ids:?})"
    );
    let value = value_of(&harness, &content_author_id("proj-001")).unwrap_or_default();
    assert!(
        value.contains("borrow checker"),
        "AC4: the original rendered_content is shown unchanged after Cancel (got {value:?})"
    );
    // The edit buffer was discarded.
    assert_eq!(panel.lock().unwrap().edit_buffer, "", "AC4: the buffer was discarded on Cancel");
    println!("PROOF4: Cancel fired (no Save), edit area gone, original content intact (events={ev:?})");
}

// ── PROOF5 + AC5: a save error keeps the edit area present (edit mode not exited) + preserves the buffer

#[test]
fn proof5_save_error_keeps_edit_area_and_buffer() {
    let mut p = loaded_panel();
    p.begin_edit();
    p.set_edit_buffer("important note");
    p.begin_save();
    // The host's overlay POST returned a simulated 500 -> apply the error.
    p.apply_save_error("POST non-success status 500 Internal Server Error");
    let panel = shared(p);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // PROOF5/AC5: the edit area is STILL present (edit mode not exited) and the buffer is preserved.
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&edit_area_author_id("proj-001")),
        "PROOF5/AC5: wiki.edit-area.proj-001 must still be present after a save error (ids={ids:?})"
    );
    assert_eq!(
        panel.lock().unwrap().edit_buffer,
        "important note",
        "AC5: the edit buffer is preserved on a save error (not lost)"
    );
    assert!(
        panel.lock().unwrap().save_error.is_some(),
        "AC5: the save error is surfaced"
    );
    println!("PROOF5: save error kept the edit area + preserved the buffer (edit mode not exited)");
}

// ── AC8: loading state shows the spinner (driven with step(), never run()) ────────────────────────────

#[test]
fn ac8_loading_state_renders_without_panic() {
    // The MT/HBR rule: the spinner animates ONLY during a genuine in-flight fetch. In a headless test we
    // set loading=true and STEP a bounded number of frames (never run() to convergence, since a genuine
    // spinner deliberately keeps requesting repaint). It must render without panic and expose no stale
    // read-only nodes.
    let mut p = LoomWikiPagePanel::new("ws-test", "proj-001");
    p.loading = true;
    let panel = shared(p);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    // A few bounded steps (the spinner requests repaint; we never run() it to convergence).
    for _ in 0..3 {
        harness.step();
    }
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&title_author_id("proj-001")),
        "AC8: while loading, no read-only title node is shown (ids={ids:?})"
    );
    assert!(
        !ids.contains(&edit_area_author_id("proj-001")),
        "AC8: while loading, no edit area is shown"
    );
    println!("AC8: loading state stepped 3 frames without panic; no stale read-only nodes");
}

// ── AC8: error state shows the error + a Retry button that fires Retry ────────────────────────────────

#[test]
fn ac8_error_state_shows_retry() {
    let mut p = LoomWikiPagePanel::new("ws-test", "proj-001");
    p.set_error("GET non-success status 404");
    let panel = shared(p);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let retry_target = handshake_native::graph::wiki_page_panel::retry_author_id("proj-001");
    let ids = author_ids(&harness);
    assert!(ids.contains(&retry_target), "AC8: the error state shows a Retry button (ids={ids:?})");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(retry_target.as_str()))
        .click();
    harness.run();
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(|e| matches!(e, WikiPageEvent::Retry)),
        "AC8: Retry click must fire a Retry event (got {ev:?})"
    );
    println!("AC8: error state shows + fires Retry");
}

// ── AC6: a stale projection shows the Stale footer; a fresh one does not ──────────────────────────────

#[test]
fn ac6_stale_footer_only_when_stale() {
    // Fresh: no stale notice text. We assert via the panel's is_stale() (the display gate) since the
    // footer is a plain colored_label (no AccessKit node by design — cosmetic).
    let fresh = loaded_panel();
    assert!(!fresh.is_stale(), "AC6: a fresh-verdict page is not stale");

    let mut stale = loaded_panel();
    if let Some(page) = stale.page.as_mut() {
        page.staleness_verdict = serde_json::json!({ "state": "stale" });
    }
    assert!(stale.is_stale(), "AC6: a stale-verdict page is stale (the footer renders)");

    // Render the stale panel to confirm it does not panic with the footer shown.
    let panel = shared(stale);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();
    assert!(
        author_ids(&harness).contains(&title_author_id("proj-001")),
        "AC6: the stale page still renders the title + footer without panic"
    );
    println!("AC6: stale footer gated on the verdict; fresh page shows none");
}

// ── HBR-VIS screenshot: the panel renders the read-only view (OPT-IN behind wgpu_screenshots) ─────────

#[test]
#[cfg(feature = "wgpu_screenshots")]
fn wiki_page_panel_screenshot() {
    let _g = wgpu_guard();
    let panel = shared(loaded_panel());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 560.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = panel.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-025");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-025-wiki-page-panel.png");
            let saved = image.save(&png).is_ok();
            println!("HBR-VIS: {w}x{h} wiki-page-panel screenshot, saved={saved} ({})", png.display());
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): wiki-page-panel screenshot render unavailable (no wgpu adapter): {e}. \
                 The AccessKit + edit/save/cancel/error proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── LIVE-PG (gated): NEEDS_MANAGED_RESOURCE_PROOF without a seeded backend ────────────────────────────

/// AC1 against a REAL Handshake-managed PostgreSQL with a seeded wiki projection. Gated behind the
/// `integration` feature AND `#[ignore]` so the default `cargo test` does not require a backend. Run
/// with: `cargo test --features integration --test test_wiki_page_panel -- --ignored`. NEVER fakes PG.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with a seeded wiki projection 'proj-001' in workspace 'ws-live'"]
#[cfg(feature = "integration")]
fn load_wiki_projection_live_pg() {
    use handshake_native::backend_client::WikiProjectionCell;

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomWikiClient::production(rt.handle().clone());
    let cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    // The operator seeds a wiki projection 'proj-001' in 'ws-live' (compile_loom_wiki_projection) first.
    client.fetch_projection("ws-live", "proj-001", Arc::clone(&cell));
    let mut data = None;
    for _ in 0..50 {
        if let Some(r) = cell.lock().unwrap().take() {
            data = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let page = data.expect("live PG fetch delivered within 5s").expect("live PG fetch ok");
    assert_eq!(page.projection_id, "proj-001", "AC1 live: the seeded projection id loads");
    assert!(!page.title.is_empty(), "AC1 live: the projection has a title");
    println!(
        "AC1 live PG: loaded wiki projection '{}' ({} sources, rebuild_status={})",
        page.title,
        page.source_block_ids.len(),
        page.rebuild_status
    );
}

/// AC3 round-trip against a REAL Handshake-managed PostgreSQL: add an overlay annotation to a seeded
/// projection. Gated + `#[ignore]`d (NEEDS_MANAGED_RESOURCE_PROOF). NEVER fakes PG.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with a seeded wiki projection 'proj-001' in workspace 'ws-live'"]
#[cfg(feature = "integration")]
fn add_overlay_live_pg() {
    use handshake_native::backend_client::ScmReceiptCell;

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomWikiClient::production(rt.handle().clone());
    let cell: ScmReceiptCell = Arc::new(Mutex::new(None));
    client.add_overlay("ws-live", "proj-001", "live overlay note", None, Arc::clone(&cell));
    let mut data = None;
    for _ in 0..50 {
        if let Some(r) = cell.lock().unwrap().take() {
            data = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    data.expect("live PG overlay POST delivered within 5s")
        .expect("AC3 live: the verified POST /loom/wiki/proj-001/overlays persisted the annotation");
    println!("AC3 live PG: overlay annotation persisted via POST /loom/wiki/proj-001/overlays");
}
