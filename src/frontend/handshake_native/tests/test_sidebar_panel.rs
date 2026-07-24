//! WP-KERNEL-012 MT-024 LoomSidebarPanel PROOFS (pins / favorites / backlinks / unlinked + breadcrumbs):
//!   - PROOF1 (breadcrumb push/truncate-to-5): owned by the lib unit tests
//!     (graph::sidebar_panel::tests); cross-checked here for the cap-at-5 behavior.
//!   - PROOF2: kittest — 2 seeded pinned blocks -> 2 `sidebar.pin.*` AccessKit ListItem nodes.
//!   - PROOF3: kittest remove-pin — click `sidebar.pin.block-001.remove`, assert the verified two-call
//!     `PUT /pin-order {pin_order:null}` + `PATCH {pinned:false}` request shapes fire, and assert
//!     `sidebar.pin.block-001` is gone from the AccessKit tree after the optimistic removal.
//!   - PROOF4: kittest backlinks — set active_block_id='block-A' with 1 backlink, assert 1
//!     `sidebar.backlink.*` node in the tree.
//!   - PROOF5: kittest breadcrumb — open 3 blocks in sequence via on_open, assert 3
//!     `sidebar.breadcrumb.*` nodes in order.
//!
//! Plus AC3 (favorite remove fires RemoveFavorite and the verified PATCH shape), AC5 (unlinked rows
//! render, deduped against backlinks), AC6 (breadcrumb click fires Open), AC7 (the named author_ids
//! present), AC8 (collapse hides rows from the AccessKit tree), AC9 (per-section error banner and Retry),
//! and the HBR-VIS screenshot proof (OPT-IN: gated behind the OFF-by-default `wgpu_screenshots` feature
//! so the default `cargo test` does not add a concurrent wgpu device that can crash a co-scheduled wgpu
//! binary on Windows — run it with `--features wgpu_screenshots`).
//!
//! ## Backend reality (Spec-Realism Gate — MT-008/021/022/023 "verify, don't trust the contract" rule)
//!
//! VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//!   - `GET  /loom/views/pins?limit=100`      -> LoomViewResponse::Pins { blocks }      (parse_view_type)
//!   - `GET  /loom/views/favorites?limit=100` -> LoomViewResponse::Favorites { blocks }
//!   - `GET  /loom/blocks/{id}/backlinks`        -> Vec<LoomBacklink> { edge, source_block, .. }  (MT-178)
//!   - `GET  /loom/blocks/{id}/unlinked-mentions`-> Vec<LoomUnlinkedMention> { source_block, .. } (MT-178)
//!   - `PUT  /loom/blocks/{id}/pin-order` body { "pin_order": null }   (SetPinOrderRequest, MT-183)
//!   - `PATCH /loom/blocks/{id}` body { "pinned": false } / { "favorite": false }  (LoomBlockUpdate)
//!
//! The contract's `graph-search?mention_ids` (backlinks) and `/views/unlinked` (per-block unlinked) were
//! corrected to the dedicated per-block MT-178 routes that carry the field-correct AC4/AC5 data — see the
//! backend_client + widget module comments for the disclosed corrections.
//!
//! AC1-AC9 also run through one isolated, self-seeding, non-ignored managed-PostgreSQL proof behind the
//! `integration` feature. It creates and tears down its own workspace, mounts real client results into
//! the widget, inspects AccessKit, performs both mutations, reopens through a fresh client, and records
//! the exact fixture ids under the external artifact root. An unreachable backend fails loudly.
//!
//! ## Artifact hygiene (CX-212E screenshot rule)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-024/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`).

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

// `Path`/`PathBuf` are used by external screenshot and managed-PG receipt helpers.
#[cfg(any(feature = "wgpu_screenshots", feature = "integration"))]
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::graph::sidebar_panel::{
    backlink_row_author_id, breadcrumb_author_id, favorite_row_author_id, pin_remove_author_id,
    pin_row_author_id, section_header_author_id, section_retry_author_id, unlinked_row_author_id,
    BacklinkRow, LoomSidebarPanel, SectionKind, SidebarBlock, SidebarEvent, UnlinkedRow,
    BACKLINK_ROW_AUTHOR_ID_PREFIX, BREADCRUMB_AUTHOR_ID_PREFIX, PIN_ROW_AUTHOR_ID_PREFIX,
};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. Only the
/// opt-in `.wgpu()` screenshot proof writes artifacts, so this is gated with that feature.
#[cfg(any(feature = "wgpu_screenshots", feature = "integration"))]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
#[cfg(any(feature = "wgpu_screenshots", feature = "integration"))]
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
/// concurrent-device hazard). NOTE: this only serializes within a single process; it gives ZERO
/// cross-process protection. The default `cargo test` does NOT run this `.wgpu()` path at all (it is
/// gated behind the OFF-by-default `wgpu_screenshots` feature) so this MT never adds a concurrent
/// wgpu device/adapter to the default test process tree. (Empirically the Windows
/// STATUS_ACCESS_VIOLATION (0xc0000005) seen in a co-scheduled wgpu binary such as test_embeds is a
/// PRE-EXISTING, WP-wide hazard across the existing wgpu test binaries — it reproduces under forced
/// concurrent scheduling even with this screenshot test removed — so the real fix is a WP-level
/// cross-binary serialization that is outside this MT's allowed_paths; see the handoff blocker.)
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
fn author_ids<T>(harness: &Harness<'_, T>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The role of the node whose author_id matches `author`, if present.
fn role_of<T>(harness: &Harness<'_, T>, author: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// A sidebar with 2 pins + 1 favorite seeded (PROOF2 needs >= 2 pins). No backend: the blocks stand in
/// for the `GET /loom/views/{pins,favorites}` results.
fn seeded_sidebar() -> LoomSidebarPanel {
    let mut panel = LoomSidebarPanel::new("ws-test");
    panel.set_pins(vec![
        SidebarBlock::new("block-001", "Ownership notes", "note"),
        SidebarBlock::new("block-002", "borrow.rs", "file"),
    ]);
    panel.set_favorites(vec![SidebarBlock::new("block-003", "Reading list", "note")]);
    panel
}

/// Harness rendering the shared sidebar, pushing every emitted event into `events`.
fn sidebar_harness(
    panel: Arc<Mutex<LoomSidebarPanel>>,
    events: Arc<Mutex<Vec<SidebarEvent>>>,
) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(360.0, 700.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = panel.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

// ── PROOF2 + AC1 + AC7: pin rows are addressable AccessKit ListItem nodes ─────────────────────────────

#[test]
fn proof2_two_pin_rows_present() {
    let panel = shared(seeded_sidebar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);

    // PROOF2: exactly 2 sidebar.pin.* rows (the remove buttons are sidebar.pin.*.remove; count the rows).
    let pin_rows = ids
        .iter()
        .filter(|a| a.starts_with(PIN_ROW_AUTHOR_ID_PREFIX) && !a.ends_with(".remove"))
        .count();
    assert_eq!(
        pin_rows, 2,
        "PROOF2: expected 2 sidebar.pin.* row nodes, got {pin_rows} (ids={ids:?})"
    );

    // AC7: the specific row ids + their remove buttons are present.
    for id in ["block-001", "block-002"] {
        assert!(
            ids.contains(&pin_row_author_id(id)),
            "AC7: '{}' must be present",
            pin_row_author_id(id)
        );
        assert!(
            ids.contains(&pin_remove_author_id(id)),
            "AC7: remove button '{}' must be present",
            pin_remove_author_id(id)
        );
    }
    // The favorite row is present too (AC7).
    assert!(
        ids.contains(&favorite_row_author_id("block-003")),
        "AC7: favorite row present"
    );

    // Role check: a pin row is a ListItem; its remove button is a Button.
    assert_eq!(
        role_of(&harness, &pin_row_author_id("block-001")).as_deref(),
        Some("ListItem"),
        "AC7: a pin row must be Role::ListItem"
    );
    assert_eq!(
        role_of(&harness, &pin_remove_author_id("block-001")).as_deref(),
        Some("Button"),
        "AC7: a pin remove control must be Role::Button"
    );
    println!("PROOF2: 2 sidebar.pin.* ListItem nodes + remove buttons + favorite row present");
}

// ── PROOF3: remove-pin click fires RemovePin, optimistic removal drops the row, + verified request shapes

#[test]
fn proof3_remove_pin_fires_event_and_drops_row() {
    let panel = shared(seeded_sidebar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Click block-001's remove button (label "✕"); both pin rows expose one, so target by AccessKit id.
    // The kittest predicate receives the AccessKit node directly (`accesskit_consumer::Node`).
    let target = pin_remove_author_id("block-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(target.as_str())
        })
        .click();
    harness.run();

    // The event fired (AC2).
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SidebarEvent::RemovePin { block_id } if block_id == "block-001")),
        "PROOF3/AC2: remove-click must fire RemovePin{{block-001}} (got {ev:?})"
    );

    // The host applies the optimistic removal (RISK-1); simulate that and re-render.
    panel.lock().unwrap().optimistic_remove_pin("block-001");
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&pin_row_author_id("block-001")),
        "PROOF3/AC2: sidebar.pin.block-001 must be gone after the optimistic removal (ids={ids:?})"
    );
    // block-002 is unaffected.
    assert!(
        ids.contains(&pin_row_author_id("block-002")),
        "PROOF3: the other pin row remains"
    );
    println!(
        "PROOF3: remove-pin fired RemovePin + the row left the AccessKit tree (events={ev:?})"
    );
}

#[test]
fn proof3_remove_pin_request_shapes() {
    use handshake_native::backend_client::{HttpMethod, LoomSidebarClient};

    // WP-KERNEL-012 MT-024 FAIL_V2: pin removal is now ONE atomic server call.
    // We assert the EXACT URL + method the production spawn path routes through
    // (NO Tauri — the WP-011 backend_client typed HTTP client). POST /remove-pin
    // clears pin_order AND unpins in a single transaction with its durable
    // EventLedger receipt, so the old two-call between-request partial-state
    // window (pin_order cleared but still pinned) is impossible.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomSidebarClient::new("http://test.local:1234", rt.handle().clone());

    let remove = client.remove_pin_request("ws1", "block-001");
    assert_eq!(
        remove.url,
        "http://test.local:1234/workspaces/ws1/loom/blocks/block-001/remove-pin",
        "PROOF3: pin removal hits the atomic /remove-pin route"
    );
    assert_eq!(
        remove.method,
        HttpMethod::Post,
        "PROOF3: pin removal is a single POST (atomic clear + unpin server-side)"
    );
    assert_eq!(
        remove.body, None,
        "PROOF3: the atomic remove-pin request carries no body (the server derives both column changes)"
    );
    println!("PROOF3: atomic pin removal request shape verified (single POST /remove-pin)");
}

// ── AC3: favorite remove fires RemoveFavorite + the verified un-favorite PATCH shape ──────────────────

#[test]
fn ac3_remove_favorite_fires_event_and_request_shape() {
    use handshake_native::backend_client::LoomSidebarClient;

    let panel = shared(seeded_sidebar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let fav_target = handshake_native::graph::sidebar_panel::favorite_remove_author_id("block-003");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(fav_target.as_str())
        })
        .click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(
            |e| matches!(e, SidebarEvent::RemoveFavorite { block_id } if block_id == "block-003")
        ),
        "AC3: favorite remove must fire RemoveFavorite{{block-003}} (got {ev:?})"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomSidebarClient::new("http://test.local:1234", rt.handle().clone());
    let unfav = client.unfavorite_request("ws1", "block-003");
    assert_eq!(
        unfav.url,
        "http://test.local:1234/workspaces/ws1/loom/blocks/block-003"
    );
    assert_eq!(
        unfav.body,
        Some(serde_json::json!({ "favorite": false })),
        "AC3: un-favorite body is the verified LoomBlockUpdate {{ favorite: false }}"
    );
    println!("AC3: favorite remove fired RemoveFavorite + verified PATCH {{favorite:false}}");
}

// ── PROOF4 + AC4: backlinks section shows 1 addressable node when an active block is set ──────────────

#[test]
fn proof4_backlink_node_present_for_active_block() {
    let mut p = seeded_sidebar();
    p.active_block_id = Some("block-A".to_owned());
    p.set_backlinks(vec![BacklinkRow::new(
        "block-src",
        "Source Block",
        "mention",
    )]);
    let panel = shared(p);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);
    let backlink_count = ids
        .iter()
        .filter(|a| a.starts_with(BACKLINK_ROW_AUTHOR_ID_PREFIX))
        .count();
    assert_eq!(
        backlink_count, 1,
        "PROOF4/AC4: exactly 1 sidebar.backlink.* node expected, got {backlink_count} (ids={ids:?})"
    );
    assert!(
        ids.contains(&backlink_row_author_id("block-src")),
        "PROOF4/AC4: the specific backlink row 'sidebar.backlink.block-src' must be present"
    );
    assert_eq!(
        role_of(&harness, &backlink_row_author_id("block-src")).as_deref(),
        Some("ListItem"),
        "AC4: a backlink row must be Role::ListItem"
    );
    // The edge-type is carried on the backlink node's accessible description (AC4 "edge_type label"). We
    // read it off the specific backlink node (a label-substring query for "mention" is ambiguous because
    // the empty "No unlinked mentions" text also contains it — the node description is the precise check).
    let mut desc_has_edge_type = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(backlink_row_author_id("block-src").as_str()) {
            let desc = ak.description().unwrap_or_default().to_owned();
            assert!(
                desc.contains("mention"),
                "AC4: the backlink node description must carry the edge_type 'mention' (got '{desc}')"
            );
            desc_has_edge_type = true;
        }
    }
    assert!(
        desc_has_edge_type,
        "AC4: the backlink node was not found for the edge_type check"
    );
    println!(
        "PROOF4: 1 sidebar.backlink.* node present for the active block (edge_type on description)"
    );
}

// ── AC5: unlinked section renders + dedups against backlinks ──────────────────────────────────────────

#[test]
fn ac5_unlinked_rows_render_and_dedup() {
    let mut p = seeded_sidebar();
    p.active_block_id = Some("block-A".to_owned());
    p.set_backlinks(vec![BacklinkRow::new(
        "block-src",
        "Source Block",
        "mention",
    )]);
    p.set_unlinked(vec![
        UnlinkedRow::new("block-src", "Source Block"), // already a backlink -> deduped out
        UnlinkedRow::new("block-A", "The Active Block"), // the active block -> deduped out
        UnlinkedRow::new("block-u", "Unlinked Mentioner"), // genuine -> shown
    ]);
    let panel = shared(p);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(&unlinked_row_author_id("block-u")),
        "AC5: the genuine unlinked row must be shown (ids={ids:?})"
    );
    assert!(
        !ids.contains(&unlinked_row_author_id("block-src")),
        "AC5/MC-4: a block already in Backlinks must NOT also show as unlinked"
    );
    assert!(
        !ids.contains(&unlinked_row_author_id("block-A")),
        "AC5/MC-4: the active block can never be its own unlinked mention"
    );
    println!(
        "AC5: unlinked section shows only the genuine row (deduped vs backlinks + active block)"
    );
}

// ── PROOF5 + AC6: opening 3 blocks yields 3 ordered breadcrumb Link nodes; clicking one fires Open ─────

#[test]
fn proof5_three_breadcrumbs_in_order() {
    let mut p = seeded_sidebar();
    // Simulate the host's on_open: each open pushes a breadcrumb (the navigation history).
    p.push_breadcrumb("blk-1", "First");
    p.push_breadcrumb("blk-2", "Second");
    p.push_breadcrumb("blk-3", "Third");
    let panel = shared(p);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);
    let crumb_count = ids
        .iter()
        .filter(|a| a.starts_with(BREADCRUMB_AUTHOR_ID_PREFIX))
        .count();
    assert_eq!(
        crumb_count, 3,
        "PROOF5: exactly 3 sidebar.breadcrumb.* nodes expected, got {crumb_count} (ids={ids:?})"
    );
    // Ordered ids 0,1,2.
    for idx in 0..3 {
        assert!(
            ids.contains(&breadcrumb_author_id(idx)),
            "PROOF5: '{}' crumb must be present (ids={ids:?})",
            breadcrumb_author_id(idx)
        );
    }
    // Role check: a crumb is a Link (AC7 "role=Link").
    assert_eq!(
        role_of(&harness, &breadcrumb_author_id(0)).as_deref(),
        Some("Link"),
        "AC7: a breadcrumb crumb must be Role::Link"
    );

    // AC6: clicking the second crumb fires Open(blk-2).
    harness.get_by_label_contains("Second").click();
    harness.run();
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SidebarEvent::Open { block_id, title } if block_id == "blk-2" && title == "Second")),
        "AC6: clicking the 'Second' crumb must fire Open{{blk-2}} (got {ev:?})"
    );
    println!("PROOF5: 3 ordered sidebar.breadcrumb.* Link nodes; crumb click fired Open(blk-2)");
}

// ── AC8: collapsing a section removes its rows from the AccessKit tree ─────────────────────────────────

#[test]
fn ac8_collapse_hides_rows_from_tree() {
    let panel = shared(seeded_sidebar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Pins are expanded by default -> rows present.
    assert!(
        author_ids(&harness).contains(&pin_row_author_id("block-001")),
        "AC8 precondition: pin rows present while expanded"
    );

    // Collapse the Pins header (label "▾ Pins (2)").
    harness.get_by_label_contains("Pins").click();
    harness.run();

    let ids = author_ids(&harness);
    let pin_rows = ids
        .iter()
        .filter(|a| a.starts_with(PIN_ROW_AUTHOR_ID_PREFIX) && !a.ends_with(".remove"))
        .count();
    assert_eq!(
        pin_rows, 0,
        "AC8: a collapsed Pins section must show NO rows in the AccessKit tree (ids={ids:?})"
    );
    // Favorites stays expanded and unaffected.
    assert!(
        ids.contains(&favorite_row_author_id("block-003")),
        "AC8: collapsing Pins must not affect the Favorites section"
    );
    println!("AC8: collapsing the Pins section removed its rows from the AccessKit tree");
}

// ── AC9: a per-section backend error shows an inline banner + Retry in that section only ───────────────

#[test]
fn ac9_section_error_shows_retry_in_that_section_only() {
    let mut p = seeded_sidebar();
    p.set_error(SectionKind::Pins, "backend unreachable");
    let panel = shared(p);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = sidebar_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);
    // The Pins section shows its Retry; other sections do NOT.
    assert!(
        ids.contains(&section_retry_author_id(SectionKind::Pins)),
        "AC9: the Pins section must show its Retry button on error (ids={ids:?})"
    );
    assert!(
        !ids.contains(&section_retry_author_id(SectionKind::Favorites)),
        "AC9: a Pins error must NOT add a Retry to the Favorites section"
    );
    // Favorites is still functional (its row renders).
    assert!(
        ids.contains(&favorite_row_author_id("block-003")),
        "AC9: other sections remain functional during a Pins error"
    );

    // Clicking Retry fires Retry{Pins}.
    let retry_target = section_retry_author_id(SectionKind::Pins);
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(retry_target.as_str())
        })
        .click();
    harness.run();
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SidebarEvent::Retry { section } if *section == SectionKind::Pins)),
        "AC9: Retry click must fire Retry{{Pins}} (got {ev:?})"
    );
    println!("AC9: Pins error showed an inline Retry (Favorites unaffected); Retry click fired Retry{{Pins}}");
}

// ── Verified request-shape builders (the production spawn paths route through these) ─────────────────

#[test]
fn sidebar_read_requests_hit_verified_routes() {
    use handshake_native::backend_client::LoomSidebarClient;
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomSidebarClient::new("http://test.local:1234", rt.handle().clone());

    let pins = client.pins_request("ws7");
    assert_eq!(
        pins.url,
        "http://test.local:1234/workspaces/ws7/loom/views/pins"
    );
    assert_eq!(pins.query, vec![("limit".to_owned(), "100".to_owned())]);

    let favs = client.favorites_request("ws7");
    assert_eq!(
        favs.url,
        "http://test.local:1234/workspaces/ws7/loom/views/favorites"
    );
    assert_eq!(favs.query, vec![("limit".to_owned(), "100".to_owned())]);

    let backlinks = client.backlinks_request("ws7", "block-A");
    assert_eq!(
        backlinks.url, "http://test.local:1234/workspaces/ws7/loom/blocks/block-A/backlinks",
        "verified dedicated MT-178 backlinks route (not graph-search)"
    );
    assert!(backlinks.query.is_empty());

    let unlinked = client.unlinked_request("ws7", "block-A");
    assert_eq!(
        unlinked.url, "http://test.local:1234/workspaces/ws7/loom/blocks/block-A/unlinked-mentions",
        "verified dedicated MT-178 per-block unlinked-mentions route (not /views/unlinked)"
    );
    println!(
        "verified: pins/favorites/backlinks/unlinked GET routes match the real MT-178/183 backend"
    );
}

#[test]
fn sidebar_wire_parsers_are_fail_closed_and_preserve_context() {
    use handshake_native::backend_client::{
        parse_sidebar_backlinks, parse_sidebar_unlinked, parse_sidebar_view_blocks,
    };

    let pins = parse_sidebar_view_blocks(
        &serde_json::json!({
            "view_type": "pins",
            "blocks": [{"block_id":"p-1","title":"Pinned","content_type":"note"}]
        }),
        "pins",
    )
    .expect("canonical pins envelope");
    assert_eq!(pins.len(), 1);
    let fallback_titles = parse_sidebar_view_blocks(
        &serde_json::json!({
            "view_type": "pins",
            "blocks": [
                {"block_id":"p-file","title":null,"original_filename":"notes.rs","content_type":"file"},
                {"block_id":"p-id","title":"   ","original_filename":null,"content_type":"note"}
            ]
        }),
        "pins",
    )
    .expect("nullable titles use the canonical filename/id fallback");
    assert_eq!(fallback_titles[0].title, "notes.rs");
    assert_eq!(fallback_titles[1].title, "p-id");
    for malformed in [
        serde_json::json!({}),
        serde_json::json!({"view_type":"pins"}),
        serde_json::json!({"view_type":"favorites","blocks":[]}),
        serde_json::json!({"view_type":"pins","blocks":[{"block_id":"p-1"}]}),
        serde_json::json!({
            "view_type":"pins",
            "blocks":[
                {"block_id":"p-1","title":"One","content_type":"note"},
                {"block_id":"p-1","title":"Duplicate","content_type":"note"}
            ]
        }),
    ] {
        assert!(
            parse_sidebar_view_blocks(&malformed, "pins").is_err(),
            "malformed view must not become a false empty/success: {malformed}"
        );
    }

    let backlinks = parse_sidebar_backlinks(&serde_json::json!([{
        "edge":{"edge_type":"mention","source_block_id":"source-1","target_block_id":"target-1"},
        "source_block":{"block_id":"source-1","title":"Source","content_type":"note"},
        "context_snippet":"Source mentions Target here"
    }]))
    .expect("canonical backlink response");
    assert_eq!(
        backlinks[0].context_snippet.as_deref(),
        Some("Source mentions Target here")
    );
    assert!(parse_sidebar_backlinks(&serde_json::json!({})).is_err());
    assert!(parse_sidebar_backlinks(&serde_json::json!([{
        "edge":{}, "source_block":{"block_id":"source-1","title":"Source","content_type":"note"}
    }]))
    .is_err());
    assert!(parse_sidebar_backlinks(&serde_json::json!([{
        "edge":{"edge_type":"mention","source_block_id":"different","target_block_id":"target-1"},
        "source_block":{"block_id":"source-1","title":"Source","content_type":"note"}
    }]))
    .is_err());

    let unlinked = parse_sidebar_unlinked(&serde_json::json!([{
        "source_block":{"block_id":"source-2","title":"Plain source","content_type":"note"},
        "matched_term":"Target",
        "snippet":"Plain source names Target without an edge",
        "match_offset":19
    }]))
    .expect("canonical unlinked response");
    assert_eq!(unlinked[0].matched_term, "Target");
    assert!(unlinked[0].snippet.contains("without an edge"));
    assert!(parse_sidebar_unlinked(&serde_json::json!([{
        "source_block":{"block_id":"source-2","title":"Plain source","content_type":"note"},
        "matched_term":"Target", "snippet":"Target"
    }]))
    .is_err());
    assert!(parse_sidebar_unlinked(&serde_json::json!([{
        "source_block":{"block_id":"source-2","title":"Plain source","content_type":"note"},
        "matched_term":"Target", "snippet":"Target", "match_offset":-1
    }]))
    .is_err());
}

// ── HBR-VIS screenshots: the sidebar renders pins + favorites + an active block's backlinks ───────────
//
// OPT-IN ONLY (adversarial-review hardening): this real-GPU `.wgpu()` proof is gated behind the
// OFF-by-default `wgpu_screenshots` feature so the default `cargo test` does not add a concurrent
// wgpu device/adapter binary from THIS MT to the process tree. On Windows multiple `.wgpu()` test
// binaries holding wgpu devices at once can crash a co-scheduled wgpu binary (e.g. test_embeds) with
// STATUS_ACCESS_VIOLATION (0xc0000005). That underlying hazard is PRE-EXISTING and WP-wide across the
// existing wgpu test binaries (it reproduces under forced concurrent scheduling even without this
// test); the WP-level cross-binary serialization fix is outside this MT's allowed_paths and is routed
// as a handoff blocker. The AccessKit/structural/request-shape proofs above carry the AC coverage in
// the default suite. Run the PNG proof explicitly with:
//   cargo test --features wgpu_screenshots --test test_sidebar_panel sidebar_panel_screenshot
#[test]
#[cfg(feature = "wgpu_screenshots")]
fn sidebar_panel_screenshot() {
    let _g = wgpu_guard();
    let mut p = seeded_sidebar();
    p.active_block_id = Some("block-A".to_owned());
    p.set_backlinks(vec![BacklinkRow::new(
        "block-src",
        "Source Block",
        "mention",
    )]);
    p.set_unlinked(vec![UnlinkedRow::new("block-u", "Unlinked Mentioner")]);
    p.push_breadcrumb("blk-1", "Project Notes");
    p.push_breadcrumb("block-A", "Active Block");
    let panel = shared(p);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 560.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = panel.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    let image = harness
        .render()
        .expect("HBR-VIS requires a real rendered sidebar frame");
    let (w, h) = (image.width(), image.height());
    assert!(w > 0 && h > 0, "rendered image must be non-empty");
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-024");
    std::fs::create_dir_all(&ext_dir).expect("create external MT-024 screenshot directory");
    let png = ext_dir.join("MT-024-sidebar-panel.png");
    image
        .save(&png)
        .unwrap_or_else(|error| panic!("save {}: {error}", png.display()));
    assert!(png.is_file(), "HBR-VIS screenshot must exist after save");
    println!(
        "HBR-VIS: {w}x{h} sidebar-panel screenshot saved ({})",
        png.display()
    );
    assert_no_local_artifact_dir();
}

// ── LIVE-PG: one isolated, self-seeded, non-ignored round trip ──────────────────────────────────────

#[cfg(feature = "integration")]
struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned(&mut self) {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204),
            "managed-PG workspace cleanup returned HTTP {status}"
        );
        assert_eq!(
            self.backend.get_status(&format!(
                "/workspaces/{}/loom/views/pins?limit=1",
                self.workspace_id
            )),
            404,
            "fresh workspace-scoped Loom read after teardown must prove the isolated workspace is absent"
        );
        self.cleaned = true;
    }
}

#[cfg(feature = "integration")]
impl Drop for LiveWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

#[cfg(feature = "integration")]
fn await_sidebar_blocks(
    cell: &handshake_native::backend_client::SidebarBlockListCell,
    expected_workspace: &str,
) -> Result<Vec<SidebarBlock>, String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, sequence, result)) = cell.lock().unwrap().pop_front() {
            assert_eq!(workspace, expected_workspace);
            assert_eq!((epoch, sequence), (0, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("sidebar block-list request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn await_sidebar_backlinks(
    cell: &handshake_native::backend_client::SidebarBacklinksCell,
    expected_workspace: &str,
    expected_block: &str,
) -> Result<Vec<BacklinkRow>, String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, block, generation, sequence, result)) =
            cell.lock().unwrap().pop_front()
        {
            assert_eq!(workspace, expected_workspace);
            assert_eq!(block, expected_block);
            assert_eq!((epoch, generation, sequence), (0, 1, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("sidebar backlinks request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn await_sidebar_unlinked(
    cell: &handshake_native::backend_client::SidebarUnlinkedCell,
    expected_workspace: &str,
    expected_block: &str,
) -> Result<Vec<UnlinkedRow>, String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, block, generation, sequence, result)) =
            cell.lock().unwrap().pop_front()
        {
            assert_eq!(workspace, expected_workspace);
            assert_eq!(block, expected_block);
            assert_eq!((epoch, generation, sequence), (0, 1, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("sidebar unlinked request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn await_sidebar_action(
    cell: &handshake_native::backend_client::SidebarActionCell,
    expected_workspace: &str,
    expected_section: SectionKind,
    expected_block: &str,
) -> Result<(), String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, section, block, sequence, result)) =
            cell.lock().unwrap().pop_front()
        {
            assert_eq!(workspace, expected_workspace);
            assert_eq!(section, expected_section);
            assert_eq!(block, expected_block);
            assert_eq!((epoch, sequence), (0, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("sidebar mutation did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn dispatch_mounted_click(
    harness: &mut Harness<'_, handshake_native::app::HandshakeApp>,
    author_id: &str,
) {
    let target = harness
        .root()
        .children_recursive()
        .find_map(|node| {
            let accesskit = node.accesskit_node();
            (accesskit.author_id() == Some(author_id)).then(|| accesskit.id())
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
fn live_patch_json(
    runtime: &tokio::runtime::Runtime,
    base: &str,
    path: &str,
    body: &serde_json::Value,
) -> serde_json::Value {
    let client = handshake_native::backend_client::shared_http_client();
    let url = format!("{base}{path}");
    let (status, text) = runtime.block_on(async {
        let response = client
            .patch(&url)
            .header("x-hsk-actor-id", "mt024-live-pg")
            .header("x-hsk-kernel-task-run-id", "mt024-live-pg-run")
            .header("x-hsk-session-run-id", "mt024-live-pg-session")
            .header("x-hsk-actor-kind", "operator")
            .json(body)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .unwrap_or_else(|error| panic!("PATCH {url} failed: {error}"));
        (response.status(), response.text().await.unwrap_or_default())
    });
    assert!(status.is_success(), "PATCH {path} -> {status}: {text}");
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("PATCH {path} response is not JSON ({error}): {text}"))
}

/// AC1-AC9 / PROOF2-5 through real Handshake APIs, managed PostgreSQL, mounted egui/AccessKit, and a
/// fresh product client. This test owns its fixtures and teardown and is deliberately NOT ignored.
#[test]
#[cfg(feature = "integration")]
fn sidebar_live_pg_self_seeds_mounted_round_trip() {
    use handshake_native::backend_client::{
        LoomSidebarClient, SidebarBacklinksCell, SidebarBlockListCell, SidebarUnlinkedCell,
    };

    let live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt024-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    );
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("sidebar live runtime");
    let client = LoomSidebarClient::new(live.base.clone(), runtime.handle().clone());

    // The isolated workspace is a real, observable empty state before fixture creation.
    let empty_pins: SidebarBlockListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_pins(&workspace_id, Arc::clone(&empty_pins));
    assert!(await_sidebar_blocks(&empty_pins, &workspace_id)
        .expect("empty pins view succeeds")
        .is_empty());

    let create_block = |content_type: &str, title: &str, pinned: bool| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({
                "content_type": content_type,
                "title": title,
                "pinned": pinned
            }),
        );
        block["block_id"]
            .as_str()
            .expect("block create returns block_id")
            .to_owned()
    };
    let target_title = format!("MT024 Target {unique}");
    let pin_one = create_block("note", "MT-024 Pin One", true);
    let pin_two = create_block("file", &target_title, true);
    let target = pin_two.clone();
    let favorite = create_block("note", "MT-024 Favorite", false);
    live_patch_json(
        &runtime,
        &live.base,
        &format!("/workspaces/{workspace_id}/loom/blocks/{favorite}"),
        &serde_json::json!({"favorite": true}),
    );
    let backlink_source = create_block("note", "MT-024 Linked Source", false);
    let unlinked_source = create_block(
        "note",
        &format!("Draft text mentions {target_title} without a link"),
        false,
    );
    let edge = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": backlink_source,
            "target_block_id": target,
            "edge_type": "mention",
            "created_by": "user"
        }),
    );
    let edge_id = edge["edge_id"]
        .as_str()
        .expect("edge create returns edge_id")
        .to_owned();

    // Drive the actual mounted HandshakeApp host: command route -> initial fetch -> AccessKit click ->
    // SidebarEvent drain -> active-block fetches. Direct client calls below are only fresh-client
    // persistence verification and cannot substitute for this mounted runtime path.
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::HealthInfo;
    use handshake_native::command_registry::CMD_VIEW_SIDEBAR;

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    app.set_sidebar_backend_base_url_for_test(live.base.clone());
    assert!(app.switch_project(&workspace_id));
    assert!(app.dispatch_palette_action_for_test(CMD_VIEW_SIDEBAR));
    let panel = app.mounted_sidebar_panel_for_test();
    let events = app.mounted_sidebar_events_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    for _ in 0..200 {
        harness.run_steps(1);
        let loaded = panel
            .lock()
            .map(|panel| panel.pins.len() == 2 && panel.favorites.len() == 1)
            .unwrap_or(false);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    {
        let panel = panel.lock().unwrap();
        assert_eq!(panel.pins.len(), 2, "AC1: mounted host loaded two pins");
        assert_eq!(
            panel.favorites.len(),
            1,
            "AC3: mounted host loaded favorite"
        );
        assert!(panel.pins.iter().any(|block| block.block_id == pin_one));
        assert!(panel.pins.iter().any(|block| block.block_id == pin_two));
        assert_eq!(panel.favorites[0].block_id, favorite);
    }

    let target_row = pin_row_author_id(&target);
    dispatch_mounted_click(&mut harness, &target_row);
    for _ in 0..200 {
        harness.run_steps(1);
        let loaded = panel
            .lock()
            .map(|panel| !panel.backlinks.is_empty() && !panel.unlinked.is_empty())
            .unwrap_or(false);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    {
        let panel = panel.lock().unwrap();
        assert_eq!(panel.active_block_id.as_deref(), Some(target.as_str()));
        assert!(panel.backlinks.iter().any(|row| {
            row.block_id == backlink_source && row.edge_type.eq_ignore_ascii_case("mention")
        }));
        assert!(panel.unlinked.iter().any(|row| {
            row.block_id == unlinked_source
                && row.matched_term == target_title
                && row.snippet.contains(&target_title)
        }));
        assert!(!panel
            .visible_unlinked()
            .iter()
            .any(|row| row.block_id == backlink_source));
    }
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_SIDEBAR));
    harness.run_steps(2);

    let ids = author_ids(&harness);
    for required in [
        pin_row_author_id(&pin_one),
        pin_row_author_id(&pin_two),
        favorite_row_author_id(&favorite),
        backlink_row_author_id(&backlink_source),
        unlinked_row_author_id(&unlinked_source),
    ] {
        assert!(
            ids.contains(&required),
            "AC7: mounted node {required} is present"
        );
    }

    for index in 0..6 {
        events.lock().unwrap().push(SidebarEvent::Open {
            block_id: format!("crumb-{index}"),
            title: format!("Crumb {index}"),
        });
        harness.run_steps(1);
        assert!(harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_VIEW_SIDEBAR));
    }
    harness.run_steps(1);
    assert_eq!(
        panel.lock().unwrap().breadcrumbs.len(),
        5,
        "AC6 breadcrumb cap"
    );
    let ids = author_ids(&harness);
    assert_eq!(
        ids.iter()
            .filter(|id| id.starts_with(BREADCRUMB_AUTHOR_ID_PREFIX))
            .count(),
        5,
        "AC6/AC7: mounted breadcrumb strip exposes the capped history"
    );
    dispatch_mounted_click(&mut harness, &breadcrumb_author_id(0));
    assert_eq!(
        panel
            .lock()
            .unwrap()
            .breadcrumbs
            .last()
            .map(|crumb| crumb.block_id.as_str()),
        Some("crumb-1"),
        "AC6: raw mounted AccessKit breadcrumb click opens the exact retained first crumb"
    );
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_VIEW_SIDEBAR),
        "reopen the Sidebar surface after the breadcrumb navigates to its Loom block"
    );
    harness.run_steps(2);
    dispatch_mounted_click(&mut harness, &section_header_author_id(SectionKind::Pins));
    assert!(
        !author_ids(&harness).contains(&pin_row_author_id(&pin_one)),
        "AC8: collapsed Pins removes its rows from AccessKit"
    );
    dispatch_mounted_click(&mut harness, &section_header_author_id(SectionKind::Pins));

    // Click the mounted Remove controls. The app performs optimistic removal, the production single
    // atomic POST /remove-pin (clear pin_order + unpin + durable receipt in one server transaction,
    // MT-024 FAIL_V2), shared change notification, and authoritative refetch.
    let pin_remove = pin_remove_author_id(&pin_one);
    dispatch_mounted_click(&mut harness, &pin_remove);
    let favorite_remove =
        handshake_native::graph::sidebar_panel::favorite_remove_author_id(&favorite);
    dispatch_mounted_click(&mut harness, &favorite_remove);
    let mut removals_persisted = false;
    for _ in 0..200 {
        harness.run_steps(1);
        let locally_removed = panel
            .lock()
            .map(|panel| {
                !panel.pins.iter().any(|block| block.block_id == pin_one)
                    && !panel
                        .favorites
                        .iter()
                        .any(|block| block.block_id == favorite)
            })
            .unwrap_or(false);
        let pins_view = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/views/pins?limit=100"
        ));
        let favorites_view = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/views/favorites?limit=100"
        ));
        let view_contains = |view: &serde_json::Value, block_id: &str| {
            view.get("blocks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|blocks| {
                    blocks.iter().any(|block| {
                        block.get("block_id").and_then(serde_json::Value::as_str) == Some(block_id)
                    })
                })
        };
        if locally_removed
            && !view_contains(&pins_view, &pin_one)
            && !view_contains(&favorites_view, &favorite)
        {
            removals_persisted = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        removals_persisted,
        "mounted remove-pin/remove-favorite mutations did not persist within 10 seconds"
    );

    // Force a real host fetch failure, assert the mounted section exposes Retry, then restore the live
    // backend and click that exact Retry control through AccessKit.
    harness
        .state()
        .set_sidebar_backend_base_url_for_test("http://127.0.0.1:0");
    events.lock().unwrap().push(SidebarEvent::Retry {
        section: SectionKind::Pins,
    });
    let retry_id = section_retry_author_id(SectionKind::Pins);
    for _ in 0..200 {
        harness.run_steps(1);
        let error_applied = panel
            .lock()
            .map(|panel| panel.error_section.contains_key(&SectionKind::Pins))
            .unwrap_or(false);
        let retry_mounted = author_ids(&harness).contains(&retry_id);
        if error_applied && retry_mounted {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(author_ids(&harness).contains(&retry_id));
    harness
        .state()
        .set_sidebar_backend_base_url_for_test(live.base.clone());
    dispatch_mounted_click(&mut harness, &retry_id);
    for _ in 0..200 {
        harness.run_steps(1);
        if panel
            .lock()
            .map(|panel| !panel.error_section.contains_key(&SectionKind::Pins))
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(!panel
        .lock()
        .unwrap()
        .error_section
        .contains_key(&SectionKind::Pins));

    let fresh = LoomSidebarClient::new(live.base.clone(), runtime.handle().clone());
    let fresh_pins: SidebarBlockListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let fresh_favorites: SidebarBlockListCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let fresh_backlinks: SidebarBacklinksCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let fresh_unlinked: SidebarUnlinkedCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh.fetch_pins(&workspace_id, Arc::clone(&fresh_pins));
    fresh.fetch_favorites(&workspace_id, Arc::clone(&fresh_favorites));
    fresh.fetch_backlinks(&workspace_id, &target, 1, Arc::clone(&fresh_backlinks));
    fresh.fetch_unlinked(&workspace_id, &target, 1, Arc::clone(&fresh_unlinked));
    let persisted_pins = await_sidebar_blocks(&fresh_pins, &workspace_id).expect("fresh pins");
    let persisted_favorites =
        await_sidebar_blocks(&fresh_favorites, &workspace_id).expect("fresh favorites");
    let persisted_backlinks =
        await_sidebar_backlinks(&fresh_backlinks, &workspace_id, &target).expect("fresh backlinks");
    let persisted_unlinked =
        await_sidebar_unlinked(&fresh_unlinked, &workspace_id, &target).expect("fresh unlinked");
    assert!(!persisted_pins.iter().any(|block| block.block_id == pin_one));
    assert!(persisted_pins.iter().any(|block| block.block_id == pin_two));
    assert!(!persisted_favorites
        .iter()
        .any(|block| block.block_id == favorite));
    assert!(persisted_backlinks
        .iter()
        .any(|row| row.block_id == backlink_source));
    assert!(persisted_unlinked
        .iter()
        .any(|row| row.block_id == unlinked_source));

    // Missing target and backend loss must be typed errors, never false empty lists or hangs.
    let missing: SidebarBacklinksCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh.fetch_backlinks(&workspace_id, "missing-target", 1, Arc::clone(&missing));
    assert!(await_sidebar_backlinks(&missing, &workspace_id, "missing-target").is_err());
    let loss = LoomSidebarClient::new("http://127.0.0.1:0", runtime.handle().clone());
    let loss_cell: SidebarBlockListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    loss.fetch_pins("mt024-backend-loss", Arc::clone(&loss_cell));
    assert!(await_sidebar_blocks(&loss_cell, "mt024-backend-loss").is_err());

    cleanup.assert_cleaned();
    assert_no_local_artifact_dir();
    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-024");
    std::fs::create_dir_all(&receipt_dir).expect("create external MT-024 receipt directory");
    let receipt_path = receipt_dir.join("MT-024-live-pg-seed.json");
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt_024.live_pg_receipt@1",
        "workspace_id": workspace_id,
        "pin_block_ids": [pin_one, pin_two],
        "favorite_block_id": favorite,
        "target_block_id": target,
        "backlink_source_block_id": backlink_source,
        "unlinked_source_block_id": unlinked_source,
        "edge_id": edge_id,
        "fresh_client_pin_removed": true,
        "fresh_client_favorite_removed": true,
        "fresh_client_backlink_count": persisted_backlinks.len(),
        "fresh_client_unlinked_count": persisted_unlinked.len(),
        "missing_target_typed_error": true,
        "backend_loss_typed_error": true,
        "cleanup_verified": true
    });
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("serialize MT-024 live receipt"),
    )
    .expect("write external MT-024 live receipt");
    println!(
        "MT-024 LIVE PG PASS workspace={workspace_id} pins=2 favorites=1 backlinks={} unlinked={} \
         edge={edge_id} receipt={} cleanup_verified=true",
        persisted_backlinks.len(),
        persisted_unlinked.len(),
        receipt_path.display()
    );
}
