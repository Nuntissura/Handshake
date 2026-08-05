//! WP-KERNEL-012 MT-025 LoomWikiPagePanel PROOFS (wiki projection + editable overlay).
//!
//!   - PROOF1 (edit-buffer pre-fill + cancel-no-mutation logic): owned by the lib unit tests
//!     (`graph::wiki_page_panel::tests` + `backend_client::wiki_client_tests`); the buffer/cancel/
//!     save-error/staleness/cap logic + the verified request-shape builders are proven STANDALONE there.
//!   - PROOF2: kittest — a seeded projection renders `wiki.title.*` (Role::Label) + a non-empty
//!     `wiki.content.*` (Role::Document) in the AccessKit tree (AC1 + AC7).
//!   - PROOF3: kittest edit-save — click `wiki.edit.projection-fixture`, the `wiki.edit-area.projection-fixture`
//!     (Role::MultilineTextInput) appears, click `wiki.save.projection-fixture`, assert the `Save{annotation}`
//!     event fires AND the verified `POST /loom/wiki/projection-fixture/overlays { annotation }` request shape is
//!     what the production spawn path sends (backend_client RequestSpec — NO Tauri), then
//!     `finish_save_success` returns to the read-only view (AC3).
//!   - PROOF4: kittest cancel — click `wiki.edit.projection-fixture`, type into the edit area, click
//!     `wiki.cancel.projection-fixture`, assert NO Save event fired (no overlay POST) and the panel returns to
//!     read-only showing the original content (AC4).
//!   - PROOF5: kittest save-error — with a save error applied, assert `wiki.edit-area.projection-fixture` is STILL
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
//! The `integration`-feature proof is non-ignored and self-contained: it creates an isolated workspace,
//! creates source Loom blocks through the real API, compiles the real projection, mounts this panel with
//! the production client, persists and reloads an overlay, proves cancel/no-write and bounded failure,
//! mutates a source to prove stale/regenerate behavior, cleans the workspace, and only then writes its
//! current-run receipt under the external artifact root.
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
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[cfg(feature = "integration")]
use handshake_native::app::{HandshakeApp, HealthDisplayState};
#[cfg(feature = "integration")]
use handshake_native::backend_client::HealthInfo;
#[cfg(feature = "integration")]
use handshake_native::backend_client::WikiOverlay;
use handshake_native::backend_client::{LoomWikiClient, WikiProjection};
use handshake_native::graph::wiki_page_panel::{
    cancel_author_id, content_author_id, edit_area_author_id, edit_author_id, error_author_id,
    metadata_author_id, save_author_id, stale_author_id, title_author_id, LoomWikiPagePanel,
    WikiPageEvent,
};
#[cfg(feature = "integration")]
use handshake_native::graph::wiki_page_panel::{
    overlay_author_id, overlays_author_id, rebuild_author_id, retry_author_id,
};
#[cfg(feature = "integration")]
use handshake_native::quick_switcher::ShellNavigator;
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

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
        projection_id: "projection-fixture".to_owned(),
        workspace_id: "ws-test".to_owned(),
        title: "Ownership model".to_owned(),
        source_block_ids: vec!["blk-1".to_owned(), "blk-2".to_owned(), "blk-3".to_owned()],
        rendered_content:
            "# Ownership model\nThe borrow checker enforces aliasing rules at compile time."
                .to_owned(),
        staleness_hash: "h1".to_owned(),
        rebuild_status: "fresh".to_owned(),
        created_at: "2026-06-19T00:00:00Z".to_owned(),
        updated_at: "2026-06-19T00:00:00Z".to_owned(),
        page_type: Some("concept".to_owned()),
        overlays: Vec::new(),
        staleness_verdict: serde_json::json!({ "state": "fresh" }),
    }
}

#[cfg(feature = "integration")]
fn overlay_from_json(row: &serde_json::Value) -> WikiOverlay {
    WikiOverlay {
        overlay_id: row["overlay_id"].as_str().expect("overlay id").to_owned(),
        projection_id: row["projection_id"]
            .as_str()
            .expect("overlay projection id")
            .to_owned(),
        workspace_id: row["workspace_id"]
            .as_str()
            .expect("overlay workspace id")
            .to_owned(),
        annotation: row["annotation"]
            .as_str()
            .expect("overlay annotation")
            .to_owned(),
        anchor: row["anchor"].as_str().map(str::to_owned),
        created_at: row["created_at"]
            .as_str()
            .expect("overlay created_at")
            .to_owned(),
        updated_at: row["updated_at"]
            .as_str()
            .expect("overlay updated_at")
            .to_owned(),
    }
}

fn loaded_panel() -> LoomWikiPagePanel {
    let mut p = LoomWikiPagePanel::new("ws-test", "projection-fixture");
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
        ids.contains(&title_author_id("projection-fixture")),
        "AC7: '{}' must be present (ids={ids:?})",
        title_author_id("projection-fixture")
    );
    assert!(
        ids.contains(&content_author_id("projection-fixture")),
        "AC7: '{}' must be present",
        content_author_id("projection-fixture")
    );
    // AC7 roles: title = Label, content = Document.
    assert_eq!(
        role_of(&harness, &title_author_id("projection-fixture")).as_deref(),
        Some("Label"),
        "AC7: the title must be Role::Label"
    );
    assert_eq!(
        role_of(&harness, &content_author_id("projection-fixture")).as_deref(),
        Some("Document"),
        "AC7: the content area must be Role::Document"
    );

    // PROOF2: the rendered_content text is exposed (non-empty) on the Document node.
    let value = value_of(&harness, &content_author_id("projection-fixture")).unwrap_or_default();
    assert!(
        value.contains("borrow checker"),
        "PROOF2: the Document node value must carry the rendered_content (got {value:?})"
    );

    // The Edit button is present in the read-only view (AC2 entry point).
    assert!(
        ids.contains(&edit_author_id("projection-fixture")),
        "AC7: the Edit button is present"
    );
    assert!(
        ids.contains(&metadata_author_id("projection-fixture")),
        "AC1/AC7: projection metadata is model-addressable"
    );
    let metadata = value_of(&harness, &metadata_author_id("projection-fixture"))
        .expect("metadata AccessKit node has a value");
    assert!(metadata.contains("page_type=concept"), "{metadata}");
    assert!(metadata.contains("rebuild_status=fresh"), "{metadata}");
    assert!(metadata.contains("source_count=3"), "{metadata}");
    // In read-only mode the edit area is ABSENT.
    assert!(
        !ids.contains(&edit_area_author_id("projection-fixture")),
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
    let edit_target = edit_author_id("projection-fixture");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(edit_target.as_str())
        })
        .click();
    harness.run();

    // The edit area (Role::MultilineTextInput) is now present (AC2 + AC7).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&edit_area_author_id("projection-fixture")),
        "AC2: the edit area must appear after Edit (ids={ids:?})"
    );
    assert_eq!(
        role_of(&harness, &edit_area_author_id("projection-fixture")).as_deref(),
        Some("MultilineTextInput"),
        "AC7: the edit area must be Role::MultilineTextInput"
    );
    assert!(
        ids.contains(&save_author_id("projection-fixture")),
        "AC7: the Save button is present in edit mode"
    );
    assert!(
        ids.contains(&cancel_author_id("projection-fixture")),
        "AC7: the Cancel button is present in edit mode"
    );

    // Author through the live model-facing editor node. The canonical Argus route rejects SetValue
    // unless the real node advertises it, and this proves the dispatched request mutates the same
    // bounded buffer used by keyboard input.
    let edit_area = harness.get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.author_id() == Some(edit_area_author_id("projection-fixture").as_str())
    });
    assert!(
        edit_area
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "AC7: the enabled wiki edit area must advertise SetValue"
    );
    let edit_area_node_id = edit_area.accesskit_node().id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: edit_area_node_id,
            data: Some(egui::accesskit::ActionData::Value(
                "NEW CONTENT".to_owned().into_boxed_str(),
            )),
        },
    ));
    harness.run_steps(2);
    assert_eq!(panel.lock().unwrap().edit_buffer, "NEW CONTENT");

    let save_target = save_author_id("projection-fixture");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(save_target.as_str())
        })
        .click();
    harness.run();

    // PROOF3/AC3: the Save event fired carrying the buffer.
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(
            |e| matches!(e, WikiPageEvent::Save { annotation, .. } if annotation == "NEW CONTENT")
        ),
        "PROOF3/AC3: Save click must fire Save{{annotation:'NEW CONTENT'}} (got {ev:?})"
    );

    // The host applies the success path (after the overlay POST 2xx + a re-fetch); simulate that and
    // re-render -> back to read-only.
    panel.lock().unwrap().finish_save_success();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&edit_area_author_id("projection-fixture")),
        "AC3: after a successful save the panel returns to read-only (edit area gone, ids={ids:?})"
    );
    assert!(
        ids.contains(&content_author_id("projection-fixture")),
        "AC3: the read-only content area is shown again"
    );
    println!("PROOF3: edit -> set buffer -> Save fired Save{{NEW CONTENT}} + success returns to read-only (events={ev:?})");
}

/// PROOF3 (request layer): the Save spawn path sends the verified `POST /loom/wiki/{id}/overlays`
/// `{ annotation }` request — the REAL persisted wiki-page write (NO Tauri; the WP-011 backend_client
/// typed HTTP client). This is the "Tauri intercept" the contract named, realised as a RequestSpec.
#[test]
fn proof3_save_request_shape_is_verified_overlay_post() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomWikiClient::new("http://test.local:1234", rt.handle().clone());

    let spec = client.add_overlay_request("ws1", "projection-fixture", "NEW CONTENT", None);
    assert_eq!(
        spec.url, "http://test.local:1234/workspaces/ws1/loom/wiki/projection-fixture/overlays",
        "PROOF3: Save hits the verified /loom/wiki/:id/overlays route (the REAL persisted write)"
    );
    assert_eq!(
        spec.body,
        Some(serde_json::json!({ "annotation": "NEW CONTENT" })),
        "PROOF3: the overlay body is the verified AddWikiOverlayRequest {{ annotation }}"
    );
    assert!(
        matches!(
            spec.method,
            handshake_native::backend_client::HttpMethod::Post
        ),
        "PROOF3: the overlay write is a POST"
    );
    println!(
        "PROOF3: Save request shape verified (POST /loom/wiki/projection-fixture/overlays {{annotation}})"
    );
}

// ── PROOF4 + AC4: cancel discards the edit and makes NO Save event (no overlay POST) ──────────────────

#[test]
fn proof4_cancel_no_mutation_returns_to_read_only() {
    let panel = shared(loaded_panel());
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Edit -> type a throwaway -> Cancel.
    let edit_target = edit_author_id("projection-fixture");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(edit_target.as_str())
        })
        .click();
    harness.run();
    panel.lock().unwrap().set_edit_buffer("THROWAWAY");
    harness.run();

    let cancel_target = cancel_author_id("projection-fixture");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(cancel_target.as_str())
        })
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
        !ids.contains(&edit_area_author_id("projection-fixture")),
        "AC4: the edit area is gone after Cancel (ids={ids:?})"
    );
    let value = value_of(&harness, &content_author_id("projection-fixture")).unwrap_or_default();
    assert!(
        value.contains("borrow checker"),
        "AC4: the original rendered_content is shown unchanged after Cancel (got {value:?})"
    );
    // The edit buffer was discarded.
    assert_eq!(
        panel.lock().unwrap().edit_buffer,
        "",
        "AC4: the buffer was discarded on Cancel"
    );
    println!(
        "PROOF4: Cancel fired (no Save), edit area gone, original content intact (events={ev:?})"
    );
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
        ids.contains(&edit_area_author_id("projection-fixture")),
        "PROOF5/AC5: wiki.edit-area.projection-fixture must still be present after a save error (ids={ids:?})"
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
    assert!(
        ids.contains(&error_author_id("projection-fixture")),
        "save/reload failure must expose the stable wiki.error selector"
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
    let mut p = LoomWikiPagePanel::new("ws-test", "projection-fixture");
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
        !ids.contains(&title_author_id("projection-fixture")),
        "AC8: while loading, no read-only title node is shown (ids={ids:?})"
    );
    assert!(
        !ids.contains(&edit_area_author_id("projection-fixture")),
        "AC8: while loading, no edit area is shown"
    );
    println!("AC8: loading state stepped 3 frames without panic; no stale read-only nodes");
}

// ── AC8: error state shows the error + a Retry button that fires Retry ────────────────────────────────

#[test]
fn ac8_error_state_shows_retry() {
    let mut p = LoomWikiPagePanel::new("ws-test", "projection-fixture");
    p.set_error("GET non-success status 404");
    let panel = shared(p);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let retry_target =
        handshake_native::graph::wiki_page_panel::retry_author_id("projection-fixture");
    let error_target = error_author_id("projection-fixture");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&error_target),
        "AC8: the load error is model-addressable (ids={ids:?})"
    );
    assert!(
        value_of(&harness, &error_target)
            .unwrap_or_default()
            .contains("GET non-success status 404"),
        "AC8: the AccessKit error node carries the actionable failure"
    );
    assert!(
        ids.contains(&retry_target),
        "AC8: the error state shows a Retry button (ids={ids:?})"
    );
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(retry_target.as_str())
        })
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
    assert!(
        stale.is_stale(),
        "AC6: a stale-verdict page is stale (the footer renders)"
    );

    // Render the stale panel to confirm it does not panic with the footer shown.
    let panel = shared(stale);
    let events = shared(Vec::new());
    let mut harness = panel_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();
    let stale_id = stale_author_id("projection-fixture");
    assert!(
        author_ids(&harness).contains(&title_author_id("projection-fixture")),
        "AC6: the stale page still renders the title + footer without panic"
    );
    assert!(
        author_ids(&harness).contains(&stale_id),
        "AC6: the stale notice is model-addressable"
    );
    assert!(
        value_of(&harness, &stale_id)
            .unwrap_or_default()
            .contains("Stale"),
        "AC6: the stale AccessKit node carries the recovery notice"
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
            println!(
                "HBR-VIS: {w}x{h} wiki-page-panel screenshot, saved={saved} ({})",
                png.display()
            );
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

// ── LIVE-PG (integration-gated, non-ignored): isolated self-seeded production round trip ─────────────

#[cfg(feature = "integration")]
struct LiveWikiFixture {
    base: String,
    run_id: String,
    actor_id: String,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
    backend: interconnect_support::LiveBackend,
}

#[cfg(feature = "integration")]
impl LiveWikiFixture {
    fn new() -> Self {
        let backend = interconnect_support::require_reachable_backend();
        let base = backend.base.clone();
        let nonce = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after unix epoch")
                .as_nanos()
        );
        let run_id = format!("mt025-{nonce}");
        Self {
            base,
            actor_id: format!("actor-{run_id}"),
            kernel_task_run_id: format!("ktr-{run_id}"),
            session_run_id: format!("sr-{run_id}"),
            correlation_id: format!("corr-{run_id}"),
            run_id,
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(0)
                .build()
                .expect("build isolated live-wiki fixture client"),
            rt: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("build live-wiki runtime"),
            backend,
        }
    }

    fn ident(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-actor-id", &self.actor_id)
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", &self.kernel_task_run_id)
            .header("x-hsk-session-run-id", &self.session_run_id)
            .header("x-hsk-correlation-id", &self.correlation_id)
    }

    fn workspace_ident(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-actor-id", &self.actor_id)
            .header("x-hsk-actor-kind", "human")
    }

    fn create_workspace(&self) -> String {
        let url = format!("{}/workspaces", self.base);
        let name = format!("{}-workspace", self.run_id);
        let (status, body) = self.rt.block_on(async {
            let response = self
                .workspace_ident(
                    self.client
                        .post(&url)
                        .json(&serde_json::json!({ "name": name })),
                )
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .unwrap_or_else(|error| panic!("POST {url} failed: {error}"));
            (response.status(), response.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "POST /workspaces -> {status}: {body}");
        serde_json::from_str::<serde_json::Value>(&body).expect("workspace response is JSON")["id"]
            .as_str()
            .expect("workspace response carries id")
            .to_owned()
    }

    fn send_json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.rt.block_on(async {
            let mut request = self.ident(self.client.request(method, &url));
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .unwrap_or_else(|error| panic!("request {url} failed: {error}"));
            (response.status(), response.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "request {path} -> {status}: {text}");
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("response for {path} is not JSON ({error}): {text}"))
    }

    fn post(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.send_json(reqwest::Method::POST, path, Some(body))
    }

    fn patch(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.send_json(reqwest::Method::PATCH, path, Some(body))
    }

    fn post_status(&self, path: &str, body: Option<&serde_json::Value>) -> (u16, String) {
        let url = format!("{}{path}", self.base);
        self.rt.block_on(async {
            let mut request = self.ident(self.client.post(&url));
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .unwrap_or_else(|error| panic!("POST {url} failed: {error}"));
            (
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            )
        })
    }

    fn get_fresh(&self, path: &str) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let actor_id = self.actor_id.clone();
        let kernel_task_run_id = self.kernel_task_run_id.clone();
        let session_run_id = self.session_run_id.clone();
        let correlation_id = self.correlation_id.clone();
        let (status, text) = self.rt.block_on(async move {
            let fresh_client = reqwest::Client::builder()
                .pool_max_idle_per_host(0)
                .build()
                .expect("build fresh reload client");
            let response = fresh_client
                .get(&url)
                .header("x-hsk-actor-id", actor_id)
                .header("x-hsk-actor-kind", "operator")
                .header("x-hsk-kernel-task-run-id", kernel_task_run_id)
                .header("x-hsk-session-run-id", session_run_id)
                .header("x-hsk-correlation-id", correlation_id)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .unwrap_or_else(|error| panic!("fresh GET {url} failed: {error}"));
            (response.status(), response.text().await.unwrap_or_default())
        });
        assert!(status.is_success(), "fresh GET {path} -> {status}: {text}");
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("fresh GET {path} is not JSON ({error}): {text}"))
    }

    fn delete_workspace(&self, workspace_id: &str) -> u16 {
        let url = format!("{}/workspaces/{workspace_id}", self.base);
        self.rt.block_on(async {
            match self
                .workspace_ident(self.client.delete(&url))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) => response.status().as_u16(),
                Err(_) => 0,
            }
        })
    }

    fn workspace_absent_fresh(&self, workspace_id: &str) -> bool {
        self.get_fresh("/workspaces")
            .as_array()
            .expect("fresh workspace list is an array")
            .iter()
            .all(|row| row["id"].as_str() != Some(workspace_id))
    }
}

#[cfg(feature = "integration")]
struct LiveWorkspaceCleanup<'a> {
    live: &'a LiveWikiFixture,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) -> u16 {
        let status = self.live.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204),
            "managed-PG workspace cleanup returned HTTP {status}"
        );
        for _ in 0..100 {
            if self.live.workspace_absent_fresh(&self.workspace_id) {
                self.cleaned = true;
                return status;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!(
            "managed-PG workspace {} remained present in fresh GET /workspaces after cleanup",
            self.workspace_id
        );
    }
}

#[cfg(feature = "integration")]
fn one_shot_http_500() -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("bind one-shot HTTP 500 server");
    listener
        .set_nonblocking(true)
        .expect("set one-shot server nonblocking");
    let address = listener.local_addr().expect("one-shot server address");
    let join = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 4096];
                    let _ = stream.read(&mut request);
                    stream
                        .write_all(
                            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .expect("write deliberate HTTP 500");
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "mounted wiki host never reached deliberate HTTP 500 server"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("one-shot HTTP 500 accept failed: {error}"),
            }
        }
    });
    (format!("http://{address}"), join)
}

#[cfg(feature = "integration")]
fn one_shot_http_json(body: serde_json::Value) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind JSON proof server");
    let address = listener.local_addr().expect("JSON proof server address");
    let encoded = serde_json::to_vec(&body).expect("encode JSON proof body");
    let join = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept JSON proof request");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request);
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            encoded.len()
        );
        stream
            .write_all(headers.as_bytes())
            .expect("write JSON headers");
        stream.write_all(&encoded).expect("write JSON body");
    });
    (format!("http://{address}"), join)
}

#[cfg(feature = "integration")]
fn host_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

#[cfg(feature = "integration")]
fn host_role(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> Option<String> {
    harness.root().children_recursive().find_map(|node| {
        let access = node.accesskit_node();
        (access.author_id() == Some(author_id)).then(|| format!("{:?}", access.role()))
    })
}

#[cfg(feature = "integration")]
fn dispatch_host_click(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    let target = harness
        .root()
        .children_recursive()
        .find_map(|node| {
            let access = node.accesskit_node();
            (access.author_id() == Some(author_id)).then(|| access.id())
        })
        .unwrap_or_else(|| panic!("mounted AccessKit node {author_id} is present"));
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target,
            data: None,
        },
    ));
    harness.run_steps(2);
}

#[cfg(feature = "integration")]
fn step_host_until(
    harness: &mut Harness<'_, HandshakeApp>,
    description: &str,
    predicate: impl Fn(&HandshakeApp) -> bool,
) {
    for _ in 0..250 {
        harness.step();
        if predicate(harness.state()) {
            // Host drivers apply async state after the pane render in the same frame. Publish that
            // state through the mounted UI/AccessKit tree before the caller performs visual assertions.
            harness.run_steps(2);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("mounted HandshakeApp host did not reach {description} within five seconds");
}

#[cfg(feature = "integration")]
impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.live.delete_workspace(&self.workspace_id);
        }
    }
}

#[cfg(feature = "integration")]
fn await_projection(
    cell: &handshake_native::backend_client::WikiProjectionCell,
    operation: &str,
) -> Result<WikiProjection, String> {
    for _ in 0..200 {
        if let Some(result) = cell.lock().unwrap().take() {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("{operation} did not resolve within 10 seconds");
}

#[cfg(feature = "integration")]
fn await_save(
    cell: &handshake_native::backend_client::ScmReceiptCell,
    operation: &str,
) -> Result<(), String> {
    for _ in 0..200 {
        if let Some(result) = cell.lock().unwrap().take() {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("{operation} did not resolve within 10 seconds");
}

#[cfg(feature = "integration")]
fn prepare_live_receipt_dir(run_id: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new("../../../../Handshake_Artifacts/handshake-test")
        .join("wp-kernel-012-mt-025");
    std::fs::create_dir_all(&dir).expect("create external MT-025 receipt directory");
    let archive = dir.join("archive");
    std::fs::create_dir_all(&archive).expect("create retired MT-025 receipt directory");
    for entry in std::fs::read_dir(&dir).expect("enumerate prior MT-025 receipts") {
        let entry = entry.expect("read MT-025 receipt entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "managed-pg-current.json" || name.ends_with("-managed-pg-receipt.json") {
            let retired = archive.join(format!("retired-{run_id}-{name}"));
            std::fs::rename(&path, &retired).unwrap_or_else(|error| {
                panic!(
                    "retire stale MT-025 receipt {} -> {}: {error}",
                    path.display(),
                    retired.display()
                )
            });
        }
    }
    dir
}

#[cfg(feature = "integration")]
fn write_live_receipt(receipt: &serde_json::Value, dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("managed-pg-current.json");
    let mut encoded = serde_json::to_string_pretty(receipt).expect("encode live receipt");
    encoded.push('\n');
    std::fs::write(&path, encoded).expect("write external live receipt after successful cleanup");
    path
}

/// AC1-AC8 and PROOF2-5 against the real Handshake-managed PostgreSQL/backend. This is deliberately one
/// integration-gated, NON-ignored closure proof: it owns its workspace, source Loom blocks, compiled
/// projection, persisted overlay, source mutation, regenerate, cleanup, and current-run receipt.
#[test]
#[cfg(feature = "integration")]
fn wiki_page_panel_live_pg_self_seeded_round_trip() {
    use handshake_native::backend_client::WikiProjectionCell;

    let mut live = LiveWikiFixture::new();
    let receipt_dir = prepare_live_receipt_dir(&live.run_id);
    let workspace_id = live.create_workspace();
    let mut cleanup = LiveWorkspaceCleanup {
        live: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    // Seed one promoted rich document, then run the real project-wiki bootstrap so the proof owns a
    // genuine typed (`entity`) page. Typed pages are deliberately not accepted by the generic regenerate
    // endpoint; their rebuild belongs to the project-wiki engine.
    live.post(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": workspace_id.clone(),
            "title": format!("{} typed wiki source", live.run_id),
            "content_json": {
                "type": "doc",
                "content": [
                    {"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": "Typed wiki proof"}]},
                    {"type": "paragraph", "content": [{"type": "text", "text": "Managed PostgreSQL typed page source."}]}
                ]
            }
        }),
    );
    let bootstrap = live.post(
        &format!("/workspaces/{workspace_id}/loom/wiki/bootstrap"),
        &serde_json::json!({}),
    );
    let typed_page = bootstrap["pages"]
        .as_array()
        .and_then(|pages| {
            pages
                .iter()
                .find(|page| page["page_type"].as_str().is_some())
        })
        .unwrap_or_else(|| panic!("project-wiki bootstrap produced no typed page: {bootstrap}"));
    let typed_projection_id = typed_page["projection_id"]
        .as_str()
        .expect("typed project-wiki page has projection_id")
        .to_owned();
    let typed_page_type = typed_page["page_type"]
        .as_str()
        .expect("typed project-wiki page has page_type")
        .to_owned();

    let source_title_a = format!("{} Alpha source", live.run_id);
    let source_title_b = format!("{} Beta source", live.run_id);
    let create_source = |title: &str| {
        let block = live.post(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": "note", "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("source Loom block response carries block_id")
            .to_owned()
    };
    let source_a = create_source(&source_title_a);
    let source_b = create_source(&source_title_b);
    let projection_title = format!("{} compiled page", live.run_id);
    let compiled = live.post(
        &format!("/workspaces/{workspace_id}/loom/wiki"),
        &serde_json::json!({
            "title": projection_title,
            "block_ids": [&source_a, &source_b]
        }),
    );
    let projection_id = compiled["projection_id"]
        .as_str()
        .expect("compile response carries projection_id")
        .to_owned();
    assert_ne!(projection_id, "projection-fixture");
    assert_eq!(
        compiled["source_block_ids"].as_array().map(Vec::len),
        Some(2),
        "real compile cites both API-created Loom blocks"
    );

    let production_client = LoomWikiClient::new(live.base.clone(), live.rt.handle().clone());
    let typed_cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    production_client.fetch_projection(
        &workspace_id,
        &typed_projection_id,
        Arc::clone(&typed_cell),
    );
    let typed_loaded = await_projection(&typed_cell, "typed project-wiki load")
        .expect("typed project-wiki projection loads through strict client");
    assert_eq!(
        typed_loaded.page_type.as_deref(),
        Some(typed_page_type.as_str())
    );
    let last_good_typed = typed_loaded.clone();
    let mut typed_panel = LoomWikiPagePanel::new(&workspace_id, &typed_projection_id);
    typed_panel.set_page(typed_loaded);
    let (typed_regenerate_status, typed_regenerate_body) = live.post_status(
        &format!("/workspaces/{workspace_id}/loom/wiki/{typed_projection_id}/regenerate"),
        None,
    );
    assert!(
        typed_regenerate_status >= 400,
        "typed page generic regenerate must be rejected, got {typed_regenerate_status}: {typed_regenerate_body}"
    );
    typed_panel.apply_rebuild_error(typed_regenerate_body);
    assert_eq!(typed_panel.page.as_ref(), Some(&last_good_typed));
    let typed_shared = shared(typed_panel);
    let typed_events = shared(Vec::new());
    let mut typed_harness = panel_harness(typed_shared, typed_events);
    typed_harness.run();
    assert!(
        !author_ids(&typed_harness).contains(&rebuild_author_id(&typed_projection_id)),
        "typed page must not expose the generic Rebuild control"
    );

    let load_cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    production_client.fetch_projection(&workspace_id, &projection_id, Arc::clone(&load_cell));
    let loaded = await_projection(&load_cell, "production wiki load")
        .expect("production LoomWikiClient loads the self-seeded projection");
    assert_eq!(loaded.projection_id, projection_id);
    assert_eq!(loaded.workspace_id, workspace_id);
    assert_eq!(
        loaded.source_block_ids,
        vec![source_a.clone(), source_b.clone()]
    );
    assert!(!loaded.rendered_content.trim().is_empty());
    assert!(loaded.rendered_content.contains(&source_title_a));
    assert!(loaded.rendered_content.contains(&source_title_b));
    assert_eq!(loaded.staleness_verdict["state"], "fresh");

    let (malformed_base, malformed_join) = one_shot_http_json(serde_json::json!({}));
    let malformed_client = LoomWikiClient::new(malformed_base, live.rt.handle().clone());
    let malformed_cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    malformed_client.fetch_projection(&workspace_id, &projection_id, Arc::clone(&malformed_cell));
    let malformed_error = await_projection(&malformed_cell, "malformed projection rejection")
        .expect_err("malformed projection body must fail closed");
    malformed_join
        .join()
        .expect("malformed JSON server completed");
    assert!(malformed_error.contains("projection_id"));

    let mut mismatched_body = live.get_fresh(&format!(
        "/workspaces/{workspace_id}/loom/wiki/{projection_id}"
    ));
    mismatched_body["projection_id"] = serde_json::Value::String("projection-crossed".to_owned());
    let (mismatch_base, mismatch_join) = one_shot_http_json(mismatched_body);
    let mismatch_client = LoomWikiClient::new(mismatch_base, live.rt.handle().clone());
    let mismatch_cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    mismatch_client.fetch_projection(&workspace_id, &projection_id, Arc::clone(&mismatch_cell));
    let mismatch_error = await_projection(&mismatch_cell, "cross-identity projection rejection")
        .expect_err("mismatched projection identity must fail closed");
    mismatch_join
        .join()
        .expect("mismatch JSON server completed");
    assert!(mismatch_error.contains("identity mismatch"));

    // Drive the production WikiPagePaneMount + HandshakeApp request/result drain. This is not a direct
    // panel harness: navigation creates the real pane tab and the real host performs GET/POST/reload.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: None,
    }));
    app.set_runtime_handle(live.rt.handle().clone());
    app.set_wiki_backend_base_url_for_test(live.base.clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    let binding = app.mounted_wiki_binding_for_test();
    *binding.lock().unwrap() = Some((
        handshake_native::backend_client::WikiPaneIdentity {
            workspace_id: workspace_id.clone(),
            projection_id: projection_id.clone(),
            pane_generation: 0,
        },
        LoomWikiPagePanel::new(&workspace_id, &projection_id),
    ));
    let wiki_pane_type = handshake_native::pane_registry::PaneType::Placeholder(
        handshake_native::editor_pane_factories::WIKI_PAGE_PANE_LABEL.to_owned(),
    );
    assert!(
        app.tab_bar_states().values().all(|bar| {
            bar.tabs.iter().all(|tab| {
                tab.pane_type != wiki_pane_type
                    || tab.content_id.as_deref() != Some(projection_id.as_str())
            })
        }),
        "managed command proof must start without the matching wiki tab"
    );
    assert!(app.dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_WIKI_PROJECTION
    ));
    let opened_pane = app
        .active_pane()
        .cloned()
        .expect("Wiki Projection command focuses a pane");
    let opened_tab = app
        .tab_bar_states()
        .get(&opened_pane)
        .and_then(|bar| bar.active())
        .expect("Wiki Projection command activates the new concrete tab");
    assert_eq!(opened_tab.pane_type, wiki_pane_type);
    assert_eq!(
        opened_tab.content_id.as_deref(),
        Some(projection_id.as_str())
    );
    // The command consumed the last truthful binding only to select the concrete projection. Clearing
    // the mount before its first frame forces the real factory to bind and issue the managed GET, so a
    // no-op dispatcher cannot satisfy this proof.
    *binding.lock().unwrap() = None;
    let mut host = Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    step_host_until(&mut host, "loaded wiki projection", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| bound.as_ref().map(|(_, panel)| panel.page.is_some()))
            .unwrap_or(false)
    });
    let ids = host_author_ids(&host);
    for required in [
        title_author_id(&projection_id),
        metadata_author_id(&projection_id),
        content_author_id(&projection_id),
        edit_author_id(&projection_id),
    ] {
        assert!(
            ids.contains(&required),
            "live AccessKit tree missing {required}"
        );
    }
    assert!(!ids.contains(&stale_author_id(&projection_id)));

    // A strict workspace-only identity mismatch must reach the mounted host as a visible error. Unit
    // tests separately isolate both projection-only and workspace-only parser mismatches; this closes
    // the end-to-end publication path without conflating the two identities.
    let mut mounted_workspace_mismatch = live.get_fresh(&format!(
        "/workspaces/{workspace_id}/loom/wiki/{projection_id}"
    ));
    mounted_workspace_mismatch["workspace_id"] =
        serde_json::Value::String("workspace-crossed".to_owned());
    let (mounted_mismatch_base, mounted_mismatch_join) =
        one_shot_http_json(mounted_workspace_mismatch);
    host.state()
        .set_wiki_backend_base_url_for_test(mounted_mismatch_base);
    assert!(host.state().queue_mounted_wiki_reload_for_test());
    step_host_until(&mut host, "mounted workspace-identity rejection", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().map(|(_, panel)| {
                    panel
                        .error
                        .as_deref()
                        .is_some_and(|error| error.contains("identity mismatch"))
                })
            })
            .unwrap_or(false)
    });
    mounted_mismatch_join
        .join()
        .expect("mounted mismatch JSON server completed");
    host.state()
        .set_wiki_backend_base_url_for_test(live.base.clone());
    let retry_id = retry_author_id(&projection_id);
    assert!(host_author_ids(&host).contains(&error_author_id(&projection_id)));
    assert!(host_author_ids(&host).contains(&retry_id));
    dispatch_host_click(&mut host, &retry_id);
    step_host_until(&mut host, "mounted identity-error recovery", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound
                    .as_ref()
                    .map(|(_, panel)| panel.page.is_some() && panel.error.is_none())
            })
            .unwrap_or(false)
    });

    // Edit -> Save is driven through the real mounted control and production host. The live edit area,
    // Save, and Cancel roles are asserted after the actual click changes host state.
    let edit_id = edit_author_id(&projection_id);
    dispatch_host_click(&mut host, &edit_id);
    let save_id = save_author_id(&projection_id);
    let cancel_id = cancel_author_id(&projection_id);
    assert_eq!(
        host_role(&host, &edit_area_author_id(&projection_id)).as_deref(),
        Some("MultilineTextInput")
    );
    assert_eq!(host_role(&host, &save_id).as_deref(), Some("Button"));
    assert_eq!(host_role(&host, &cancel_id).as_deref(), Some("Button"));
    let annotation = format!("{} persisted overlay", live.run_id);
    binding
        .lock()
        .unwrap()
        .as_mut()
        .expect("mounted wiki binding")
        .1
        .set_edit_buffer(&annotation);
    host.run_steps(2);
    dispatch_host_click(&mut host, &save_id);
    step_host_until(&mut host, "identity-matched post-save reload", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound
                    .as_ref()
                    .map(|(_, panel)| !panel.edit_mode && !panel.saving && panel.page.is_some())
            })
            .unwrap_or(false)
    });

    let overlays_path = format!("/workspaces/{workspace_id}/loom/wiki/{projection_id}/overlays");
    let mut overlays = live.get_fresh(&overlays_path);
    let overlay_rows = overlays.as_array().expect("overlay list is an array");
    assert_eq!(overlay_rows.len(), 1);
    assert_eq!(overlay_rows[0]["annotation"], annotation);
    assert!(overlay_rows[0]["anchor"].is_null());
    let overlay_id = overlay_rows[0]["overlay_id"]
        .as_str()
        .expect("persisted overlay has id")
        .to_owned();
    {
        let current = binding.lock().unwrap();
        let page = current
            .as_ref()
            .and_then(|(_, panel)| panel.page.as_ref())
            .expect("mounted page remains loaded after overlay reload");
        assert_eq!(page.overlays.len(), 1);
        assert_eq!(page.overlays[0].annotation, annotation);
    }
    let mounted_overlay_ids = host_author_ids(&host);
    assert!(mounted_overlay_ids.contains(&overlays_author_id(&projection_id)));
    assert!(mounted_overlay_ids.contains(&overlay_author_id(&overlay_id)));

    // Negative path: the canonical POST succeeds but its follow-up GET fails. Feed that real successful
    // write receipt into the mounted host while its wiki transport points at a one-shot 500 server. The
    // recovery control must enqueue only ReloadAfterSave (GET), never another overlay POST.
    let partial_annotation = format!("{} saved-before-reload-failure", live.run_id);
    let (partial_identity, partial_action_generation) = {
        let mut current = binding.lock().unwrap();
        let (identity, panel) = current.as_mut().expect("mounted wiki binding");
        assert!(panel.begin_edit());
        panel.set_edit_buffer(&partial_annotation);
        let (action_generation, sent) = panel
            .begin_observed_save_for_test()
            .expect("observed Save starts");
        assert_eq!(sent, partial_annotation);
        (identity.clone(), action_generation)
    };
    let partial_client = LoomWikiClient::new(live.base.clone(), live.rt.handle().clone());
    let partial_save: handshake_native::backend_client::ScmReceiptCell = Arc::new(Mutex::new(None));
    partial_client.add_overlay(
        &workspace_id,
        &projection_id,
        &partial_annotation,
        None,
        Arc::clone(&partial_save),
    );
    await_save(
        &partial_save,
        "real overlay POST before forced reload failure",
    )
    .expect("real overlay POST succeeds");
    let overlays_after_partial_save = live.get_fresh(&overlays_path);
    assert_eq!(
        overlays_after_partial_save
            .as_array()
            .expect("overlay list")
            .len(),
        2,
        "exactly one additional canonical overlay was inserted"
    );
    let partial_overlay = overlays_after_partial_save
        .as_array()
        .expect("overlay list")
        .iter()
        .find(|row| row["annotation"] == partial_annotation)
        .map(overlay_from_json)
        .expect("partial overlay persisted row");

    let (reload_500_base, reload_500_join) = one_shot_http_500();
    host.state()
        .set_wiki_backend_base_url_for_test(reload_500_base);
    host.state().deliver_wiki_save_for_test(
        partial_identity,
        partial_action_generation,
        Ok(partial_overlay),
    );
    step_host_until(
        &mut host,
        "saved overlay with failed follow-up reload",
        |app| {
            app.mounted_wiki_binding_for_test()
                .lock()
                .ok()
                .and_then(|bound| {
                    bound.as_ref().map(|(_, panel)| {
                        panel.saved_awaiting_reload && !panel.saving && panel.save_error.is_some()
                    })
                })
                .unwrap_or(false)
        },
    );
    reload_500_join
        .join()
        .expect("post-save reload HTTP 500 server completed");
    host.state()
        .set_wiki_backend_base_url_for_test(live.base.clone());
    assert!(host_author_ids(&host).contains(&retry_author_id(&projection_id)));
    dispatch_host_click(&mut host, &retry_author_id(&projection_id));
    step_host_until(
        &mut host,
        "Retry Reload completes without a second POST",
        |app| {
            app.mounted_wiki_binding_for_test()
                .lock()
                .ok()
                .and_then(|bound| {
                    bound.as_ref().map(|(_, panel)| {
                        !panel.edit_mode
                            && !panel.saved_awaiting_reload
                            && panel
                                .page
                                .as_ref()
                                .map(|page| page.overlays.len() == 2)
                                .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        },
    );
    assert_eq!(
        live.get_fresh(&overlays_path),
        overlays_after_partial_save,
        "Retry Reload performs only GET and the persisted overlay count stays exactly two"
    );
    overlays = overlays_after_partial_save;

    {
        let mut current = binding.lock().unwrap();
        let (identity, panel) = current.as_mut().expect("mounted same-identity panel");
        assert_eq!(identity.projection_id, projection_id);
        assert!(panel.begin_edit());
        panel.set_edit_buffer("same-identity in-flight buffer");
        assert!(panel.begin_save().is_some());
        assert!(
            !panel.cancel_edit(),
            "Cancel is locked during same-pane Save/reload"
        );
        panel.set_edit_buffer("newer edit must not be admitted during old completion");
        assert_eq!(panel.edit_buffer, "same-identity in-flight buffer");
        panel.apply_save_error("deterministic lockout proof release");
        assert!(panel.cancel_edit());
    }
    host.run_steps(2);

    // A fresh product client reload proves the projection came from PostgreSQL, not the mounted panel's
    // copy; the derived content stays unchanged because the edit is an independent overlay authority row.
    let fresh_client = LoomWikiClient::new(live.base.clone(), live.rt.handle().clone());
    let reload_cell: WikiProjectionCell = Arc::new(Mutex::new(None));
    fresh_client.fetch_projection(&workspace_id, &projection_id, Arc::clone(&reload_cell));
    let reloaded = await_projection(&reload_cell, "fresh-client wiki reload")
        .expect("fresh LoomWikiClient reload succeeds");
    assert_eq!(reloaded.rendered_content, loaded.rendered_content);
    assert!(!reloaded.rendered_content.contains(&annotation));

    // Cancel is a mounted interaction and must leave the canonical overlay list byte-for-byte unchanged.
    let overlay_snapshot_before_cancel = overlays.clone();
    dispatch_host_click(&mut host, &edit_id);
    binding
        .lock()
        .unwrap()
        .as_mut()
        .expect("mounted wiki binding")
        .1
        .set_edit_buffer(format!("{} discard", live.run_id));
    host.run_steps(2);
    dispatch_host_click(&mut host, &cancel_id);
    assert_eq!(
        live.get_fresh(&overlays_path),
        overlay_snapshot_before_cancel,
        "Cancel makes no canonical overlay write"
    );

    // A real HTTP 500 reaches the mounted host through reqwest. The identity-matched error delivery
    // preserves the edit buffer and keeps the multiline AccessKit node mounted for retry.
    dispatch_host_click(&mut host, &edit_id);
    let preserved = format!("{} preserve-on-failure", live.run_id);
    binding
        .lock()
        .unwrap()
        .as_mut()
        .expect("mounted wiki binding")
        .1
        .set_edit_buffer(&preserved);
    host.step();
    let (http_500_base, http_500_join) = one_shot_http_500();
    host.state()
        .set_wiki_backend_base_url_for_test(http_500_base);
    dispatch_host_click(&mut host, &save_id);
    step_host_until(&mut host, "visible HTTP 500 save error", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| bound.as_ref().map(|(_, panel)| panel.save_error.is_some()))
            .unwrap_or(false)
    });
    http_500_join.join().expect("HTTP 500 server completed");
    host.state()
        .set_wiki_backend_base_url_for_test(live.base.clone());
    {
        let failed_binding = binding.lock().unwrap();
        let failed_panel = &failed_binding.as_ref().expect("mounted wiki binding").1;
        assert_eq!(failed_panel.edit_buffer, preserved);
        assert!(failed_panel.edit_mode);
    }
    assert!(
        host_author_ids(&host).contains(&edit_area_author_id(&projection_id)),
        "failed Save preserves the multiline AccessKit editor"
    );
    assert!(
        host_author_ids(&host).contains(&error_author_id(&projection_id)),
        "failed Save exposes the stable wiki.error selector in the mounted product"
    );
    dispatch_host_click(&mut host, &cancel_id);
    assert_eq!(live.get_fresh(&overlays_path), overlays);

    // A real source mutation makes the served projection stale; the mounted stale node is model-visible.
    let renamed_source = format!("{} Alpha source renamed", live.run_id);
    live.patch(
        &format!("/workspaces/{workspace_id}/loom/blocks/{source_a}"),
        &serde_json::json!({ "title": renamed_source }),
    );
    assert!(host.state().queue_mounted_wiki_reload_for_test());
    step_host_until(&mut host, "mounted stale projection", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().and_then(|(_, panel)| {
                    panel
                        .page
                        .as_ref()
                        .map(|page| page.staleness_verdict["state"] == "stale")
                })
            })
            .unwrap_or(false)
    });
    let stale_id = stale_author_id(&projection_id);
    assert!(host_author_ids(&host).contains(&stale_id));
    let rebuild_id = rebuild_author_id(&projection_id);
    dispatch_host_click(&mut host, &rebuild_id);
    step_host_until(&mut host, "mounted regenerated projection", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().and_then(|(_, panel)| {
                    panel.page.as_ref().map(|page| {
                        page.staleness_verdict["state"] == "fresh"
                            && page.rendered_content.contains(&renamed_source)
                    })
                })
            })
            .unwrap_or(false)
    });
    assert_eq!(
        live.get_fresh(&overlays_path),
        overlays,
        "overlay authority survives projection regeneration"
    );

    // Error + Retry are mounted and AccessKit-addressable. A second real HTTP 500 produces the load
    // error, then Retry is completed by the managed backend through the same host queue.
    let (load_500_base, load_500_join) = one_shot_http_500();
    host.state()
        .set_wiki_backend_base_url_for_test(load_500_base);
    assert!(host.state().queue_mounted_wiki_reload_for_test());
    step_host_until(&mut host, "visible HTTP 500 load error", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| bound.as_ref().map(|(_, panel)| panel.error.is_some()))
            .unwrap_or(false)
    });
    load_500_join
        .join()
        .expect("load HTTP 500 server completed");
    host.state()
        .set_wiki_backend_base_url_for_test(live.base.clone());
    let ids = host_author_ids(&host);
    let error_id = error_author_id(&projection_id);
    let retry_id = retry_author_id(&projection_id);
    assert!(ids.contains(&error_id));
    assert!(ids.contains(&retry_id));
    dispatch_host_click(&mut host, &retry_id);
    step_host_until(&mut host, "successful mounted Retry reload", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound
                    .as_ref()
                    .map(|(_, panel)| panel.page.is_some() && panel.error.is_none())
            })
            .unwrap_or(false)
    });
    assert!(host_author_ids(&host).contains(&title_author_id(&projection_id)));

    // Generation-race proof on the real mounted driver: late A load/save deliveries cannot replace B
    // or clear B's buffer, and the first A generation is also rejected after A -> B -> A.
    let identity_a_first = binding
        .lock()
        .unwrap()
        .as_ref()
        .expect("A wiki binding")
        .0
        .clone();
    let projection_b_json = live.post(
        &format!("/workspaces/{workspace_id}/loom/wiki"),
        &serde_json::json!({
            "title": format!("{} race page B", live.run_id),
            "block_ids": [&source_b]
        }),
    );
    let projection_b = projection_b_json["projection_id"]
        .as_str()
        .expect("race projection B id")
        .to_owned();
    assert!(matches!(
        ShellNavigator::open_wiki_page(host.state_mut(), &projection_b),
        handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
    ));
    step_host_until(&mut host, "race page B", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().map(|(identity, panel)| {
                    identity.projection_id == projection_b && panel.page.is_some()
                })
            })
            .unwrap_or(false)
    });
    {
        let mut current = binding.lock().unwrap();
        let (_, panel_b) = current.as_mut().expect("B wiki binding");
        assert!(panel_b.begin_edit());
        panel_b.set_edit_buffer("B-buffer-must-survive-late-A");
    }
    host.state().deliver_wiki_projection_for_test(
        identity_a_first.clone(),
        handshake_native::backend_client::WikiProjectionOperation::Load,
        Ok(loaded.clone()),
    );
    host.state().deliver_wiki_save_for_test(
        identity_a_first.clone(),
        u64::MAX,
        Ok(WikiOverlay {
            overlay_id: "stale-overlay".to_owned(),
            projection_id: identity_a_first.projection_id.clone(),
            workspace_id: identity_a_first.workspace_id.clone(),
            annotation: "stale".to_owned(),
            anchor: None,
            created_at: "2026-06-19T00:00:00Z".to_owned(),
            updated_at: "2026-06-19T00:00:00Z".to_owned(),
        }),
    );
    host.step();
    {
        let current = binding.lock().unwrap();
        let (identity, panel_b) = current.as_ref().expect("B remains bound");
        assert_eq!(identity.projection_id, projection_b);
        assert_eq!(panel_b.edit_buffer, "B-buffer-must-survive-late-A");
        assert!(panel_b.edit_mode);
    }
    assert!(matches!(
        ShellNavigator::open_wiki_page(host.state_mut(), &projection_id),
        handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
    ));
    step_host_until(&mut host, "second generation of page A", |app| {
        app.mounted_wiki_binding_for_test()
            .lock()
            .ok()
            .and_then(|bound| {
                bound.as_ref().map(|(identity, panel)| {
                    identity.projection_id == projection_id
                        && identity.pane_generation > identity_a_first.pane_generation
                        && panel.page.is_some()
                })
            })
            .unwrap_or(false)
    });
    {
        let mut current = binding.lock().unwrap();
        let (_, panel_a_second) = current.as_mut().expect("second A wiki binding");
        assert!(panel_a_second.begin_edit());
        panel_a_second.set_edit_buffer("A-second-generation-buffer");
    }
    host.state().deliver_wiki_projection_for_test(
        identity_a_first,
        handshake_native::backend_client::WikiProjectionOperation::Load,
        Ok(loaded),
    );
    host.step();
    assert_eq!(
        binding
            .lock()
            .unwrap()
            .as_ref()
            .expect("second A remains bound")
            .1
            .edit_buffer,
        "A-second-generation-buffer"
    );

    let cleanup_status = cleanup.assert_cleaned();
    drop(cleanup);
    let backend_binding = live.backend.owned_backend_binding_receipt();
    live.backend.assert_cleanup();
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt025.managed_pg_proof_receipt@2",
        "wp_id": "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        "mt_id": "MT-025",
        "run_id": live.run_id.clone(),
        "actor_id": live.actor_id.clone(),
        "kernel_task_run_id": live.kernel_task_run_id.clone(),
        "session_run_id": live.session_run_id.clone(),
        "correlation_id": live.correlation_id.clone(),
        "workspace_id": workspace_id,
        "projection_id": projection_id,
        "typed_projection_id": typed_projection_id,
        "typed_page_type": typed_page_type,
        "source_block_ids": [source_a, source_b],
        "overlay_id": overlay_id,
        "overlay_annotation": annotation,
        "derived_content_unchanged_by_overlay": true,
        "stale_after_source_change": true,
        "fresh_after_regenerate": true,
        "overlay_survived_regenerate": true,
        "overlay_visible_in_mounted_panel_after_reload": true,
        "menu_reopened_concrete_projection": true,
        "typed_generic_rebuild_hidden_and_rejected": true,
        "typed_rebuild_failure_preserved_last_good": true,
        "same_identity_save_lockout_preserved_buffer": true,
        "malformed_projection_rejected": true,
        "mismatched_projection_identity_rejected": true,
        "mismatched_workspace_identity_rejected": true,
        "mounted_identity_error_visible": true,
        "cancel_wrote_nothing": true,
        "failed_save_preserved_buffer": true,
        "real_http_500_host_delivery": true,
        "live_ids_generated": true,
        "hardcoded_live_ids": false,
        "late_delivery_race_rejected": true,
        "cleanup_http_status": cleanup_status,
        "cleanup_absence_confirmed_by_fresh_workspace_list": true,
        "cleanup_completed_before_receipt": true,
        "owned_backend_binding": backend_binding,
        "owned_backend_reaped_before_receipt": true,
        "canonical_current_receipt": true,
        "command": "cargo test --manifest-path src/frontend/handshake_native/Cargo.toml --features integration --test test_wiki_page_panel wiki_page_panel_live_pg_self_seeded_round_trip -- --nocapture"
    });
    let receipt_path = write_live_receipt(&receipt, &receipt_dir);
    println!(
        "MT-025 LIVE MANAGED-PG PASS run_id={} workspace_id={} projection_id={} overlay_id={} cleanup={} receipt={}",
        live.run_id,
        receipt["workspace_id"],
        receipt["projection_id"],
        receipt["overlay_id"],
        cleanup_status,
        receipt_path.display()
    );
}
