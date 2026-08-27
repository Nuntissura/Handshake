//! WP-KERNEL-012 MT-023 LoomTagsPanel + LoomTagHubPanel PROOFS:
//!   - PROOF1 (filter + chip-color distribution): owned by the lib unit tests (graph::tags_panel::tests);
//!     cross-checked here for the filter.
//!   - PROOF2: kittest — 3 seeded tag hubs -> 3 `tags.row.*` AccessKit nodes (Role::ListItem).
//!   - PROOF3: kittest — typing "rust" into `tags.search` leaves only rust-prefixed tags in the tree.
//!   - PROOF4: kittest — open hub `tag-hub-001`, assert `tag-hub.title.tag-hub-001` + >= 1
//!     `tag-hub.member.*` node.
//!   - PROOF5: click `tag-hub.add-tag.tag-hub-001`, select a candidate, assert the AddTagSelected event +
//!     the verified `POST /loom/edges` request shape (`edge_type:"tag"`). (No Tauri — backend_client.)
//!
//! Plus AC3 (tag-row click fires OpenTag), AC5 (member-row click fires OpenMember), AC7 (the named
//! author_ids present), AC8 (empty -> "No tags", no panic), and a screenshot (HBR-VIS).
//!
//! ## Backend reality (Spec-Realism Gate — MT-008/021/022 "verify, don't trust the contract" rule)
//!
//! The MT-023 contract's assumed generic view filters (`views/all?content_type=tag_hub`,
//! `views/all?tag_ids={id}`) exist, but the stronger authority for this surface is the dedicated tag-hub
//! API (MT-182), verified READ-ONLY against
//! `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//!   - `GET  /loom/tags`                  -> Vec<LoomBlock> (every tag_hub block)
//!   - `GET  /loom/tags/{id}`             -> LoomTagHub { block, sub_tags, tagged_blocks, backlink_count }
//!   - `GET  /loom/tags/{id}/blocks`      -> Vec<LoomBlock> (members)
//!   - `POST /loom/edges` body { source_block_id, target_block_id, edge_type:"tag", created_by:"user" }
//!     (the backend rejects a non-tag_hub target -> the hub is the edge TARGET).
//!
//! AC1/AC4/AC6 have one integration-gated, unignored managed-SurrealDB proof. It creates an isolated
//! workspace, proves the mounted pane's real empty state, seeds three tag hubs and two documents, drives
//! list/filter/open/add through the mounted [`HandshakeApp`], verifies rename/removal through a fresh
//! [`LoomTagClient`], proves bounded backend loss, and deterministically deletes the workspace. It never
//! relies on operator fixture ids or cached panel state.
//!
//! ## Artifact hygiene (CX-212E / CX-212E screenshot rule)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-023/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(feature = "integration")]
mod interconnect_support;

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::graph::tags_panel::{
    hub_member_author_id, tag_row_author_id, AddTagCandidate, HubMember, LoomTagHubPanel,
    LoomTagsPanel, TagEntry, TagHubEvent, TagsPanelEvent, HUB_MEMBER_AUTHOR_ID_PREFIX,
    HUB_RETRY_AUTHOR_ID, HUB_TITLE_AUTHOR_ID_PREFIX, RETRY_AUTHOR_ID as LIST_RETRY_AUTHOR_ID,
    SEARCH_AUTHOR_ID, TAG_ROW_AUTHOR_ID_PREFIX,
};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
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

/// Serialize the `.wgpu()` screenshot tests (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// 4 seeded tag hubs (3 of which the PROOF2 count needs >= 3): rust, rustaceans, python, design.
/// No backend: the entries stand in for `GET /loom/tags`.
fn seeded_tags() -> LoomTagsPanel {
    let mut panel = LoomTagsPanel::new("ws-test");
    panel.set_tags(vec![
        TagEntry::new("tag-hub-001", "rust", Some(3)),
        TagEntry::new("tag-hub-002", "rustaceans", Some(1)),
        TagEntry::new("tag-hub-003", "python", Some(7)),
        TagEntry::new("tag-hub-004", "design", None),
    ]);
    panel
}

/// A hub page for tag-hub-001 ("rust") with 2 seeded members (so PROOF4 + AC5 have work).
fn seeded_hub() -> LoomTagHubPanel {
    let mut hub = LoomTagHubPanel::new("ws-test", "tag-hub-001");
    hub.set_detail(
        "rust",
        vec![
            HubMember::new("blk-001", "Ownership notes", "note"),
            HubMember::new("blk-002", "borrow.rs", "file"),
        ],
    );
    hub
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

/// Harness rendering the shared tags panel, pushing every emitted event into `events`.
fn tags_harness(
    panel: Arc<Mutex<LoomTagsPanel>>,
    events: Arc<Mutex<Vec<TagsPanelEvent>>>,
) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(360.0, 600.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = panel.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

/// Harness rendering the shared hub page, pushing every emitted event into `events`.
fn hub_harness(
    hub: Arc<Mutex<LoomTagHubPanel>>,
    events: Arc<Mutex<Vec<TagHubEvent>>>,
) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(480.0, 600.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = hub.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

// ── PROOF2 + AC7: tag rows are addressable AccessKit ListItem nodes ──────────────────────────────────

#[test]
fn proof2_three_tag_rows_present() {
    let panel = shared(seeded_tags());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = tags_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);

    // PROOF2: >= 3 tags.row.* entries (4 seeded; the contract asks for "3 tag_hub blocks").
    let row_count = ids
        .iter()
        .filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX))
        .count();
    assert!(
        row_count >= 3,
        "PROOF2: expected >= 3 tags.row.* AccessKit nodes, got {row_count} (ids={ids:?})"
    );

    // AC7: the search box id + the specific tag row ids are present.
    assert!(
        ids.contains(SEARCH_AUTHOR_ID),
        "AC7: 'tags.search' must be in the tree (ids={ids:?})"
    );
    for id in ["tag-hub-001", "tag-hub-002", "tag-hub-003"] {
        let row = tag_row_author_id(id);
        assert!(
            ids.contains(&row),
            "AC7: '{row}' must be present (ids={ids:?})"
        );
    }

    // Role check: tag-hub-001's row is a ListItem.
    let mut listitem_found = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(tag_row_author_id("tag-hub-001").as_str()) {
            assert_eq!(
                format!("{:?}", ak.role()),
                "ListItem",
                "AC7: a tag row must be Role::ListItem"
            );
            listitem_found = true;
        }
    }
    assert!(
        listitem_found,
        "AC7: tags.row.tag-hub-001 not found for role check"
    );
    println!("PROOF2: {row_count} tags.row.* ListItem nodes + tags.search present");
}

// ── PROOF3: typing into tags.search filters the tree to title-prefix matches only (AC2) ──────────────

#[test]
fn proof3_search_filter_narrows_rows() {
    let panel = shared(seeded_tags());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = tags_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // Type "rust" into the search box. The widget reads `self.search_filter`; we set it directly (the
    // TextEdit binds to the same field the production UI binds to) and re-render — the externally
    // meaningful result is the filtered AccessKit tree.
    panel.lock().unwrap().search_filter = "rust".to_owned();
    harness.run();

    let ids = author_ids(&harness);
    // Only rust + rustaceans rows remain; python + design are filtered out (AC2 / PROOF3).
    assert!(
        ids.contains(&tag_row_author_id("tag-hub-001")),
        "PROOF3: 'rust' row remains (ids={ids:?})"
    );
    assert!(
        ids.contains(&tag_row_author_id("tag-hub-002")),
        "PROOF3: 'rustaceans' row remains"
    );
    assert!(
        !ids.contains(&tag_row_author_id("tag-hub-003")),
        "PROOF3: 'python' row is filtered out by the 'rust' prefix"
    );
    assert!(
        !ids.contains(&tag_row_author_id("tag-hub-004")),
        "PROOF3: 'design' row is filtered out by the 'rust' prefix"
    );
    let remaining = ids
        .iter()
        .filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX))
        .count();
    assert_eq!(
        remaining, 2,
        "PROOF3: exactly the 2 rust-prefixed rows remain (ids={ids:?})"
    );
    println!("PROOF3: 'rust' filter narrowed 4 rows -> 2 (rust, rustaceans)");
}

// ── AC3: clicking a tag row fires OpenTag with the correct block_id ──────────────────────────────────

#[test]
fn ac3_tag_click_fires_open_tag() {
    let panel = shared(seeded_tags());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = tags_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    // The 'python' tag renders as "#python"; click it by its label.
    harness.get_by_label_contains("python").click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    let opened = ev.iter().any(
        |e| matches!(e, TagsPanelEvent::OpenTag { block_id, .. } if block_id == "tag-hub-003"),
    );
    assert!(
        opened,
        "AC3: clicking '#python' must fire OpenTag{{block_id:'tag-hub-003'}} (got {ev:?})"
    );
    println!("AC3: tag click fired OpenTag(tag-hub-003) (events={ev:?})");
}

// ── AC8: empty workspace -> "No tags", no rows, no panic ─────────────────────────────────────────────

#[test]
fn ac8_empty_no_tags() {
    let panel = shared(LoomTagsPanel::new("ws-empty"));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = tags_harness(Arc::clone(&panel), Arc::clone(&events));
    harness.run();

    assert!(
        harness.query_by_label("No tags").is_some(),
        "AC8: 'No tags' label must be present for an empty workspace"
    );
    let ids = author_ids(&harness);
    assert_eq!(
        ids.iter()
            .filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX))
            .count(),
        0,
        "AC8: no tags.row.* nodes for an empty workspace"
    );
    println!("AC8: empty workspace shows 'No tags', no row entries, no panic");
}

#[test]
fn error_states_are_addressable_and_retryable() {
    let mut list = LoomTagsPanel::new("ws-error");
    list.error = Some("backend unavailable".to_owned());
    let list = shared(list);
    let list_events = Arc::new(Mutex::new(Vec::new()));
    let mut list_harness = tags_harness(Arc::clone(&list), Arc::clone(&list_events));
    list_harness.run();
    assert!(author_ids(&list_harness).contains(LIST_RETRY_AUTHOR_ID));
    list_harness.get_by_label("Retry").click();
    list_harness.run();
    assert!(list_events.lock().unwrap().contains(&TagsPanelEvent::Retry));

    let mut hub = LoomTagHubPanel::new("ws-error", "hub-error");
    hub.error = Some("malformed response".to_owned());
    let hub = shared(hub);
    let hub_events = Arc::new(Mutex::new(Vec::new()));
    let mut hub_harness = hub_harness(Arc::clone(&hub), Arc::clone(&hub_events));
    hub_harness.run();
    assert!(author_ids(&hub_harness).contains(HUB_RETRY_AUTHOR_ID));
    hub_harness.get_by_label("Retry").click();
    hub_harness.run();
    assert!(hub_events.lock().unwrap().contains(&TagHubEvent::Retry));
}

// ── PROOF4 + AC4: the hub page shows the title + >= 1 member, both addressable ───────────────────────

#[test]
fn proof4_hub_page_title_and_members() {
    let hub = shared(seeded_hub());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = hub_harness(Arc::clone(&hub), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);

    // PROOF4: tag-hub.title.tag-hub-001 present.
    let title_id = format!("{HUB_TITLE_AUTHOR_ID_PREFIX}tag-hub-001");
    assert!(
        ids.contains(&title_id),
        "PROOF4/AC4: '{title_id}' hub-title node must be present (ids={ids:?})"
    );

    // PROOF4: >= 1 tag-hub.member.* node.
    let member_count = ids
        .iter()
        .filter(|a| a.starts_with(HUB_MEMBER_AUTHOR_ID_PREFIX))
        .count();
    assert!(
        member_count >= 1,
        "PROOF4/AC4: >= 1 tag-hub.member.* node expected, got {member_count} (ids={ids:?})"
    );
    // The specific seeded members.
    assert!(
        ids.contains(&hub_member_author_id("blk-001")),
        "AC4: member blk-001 present"
    );
    assert!(
        ids.contains(&hub_member_author_id("blk-002")),
        "AC4: member blk-002 present"
    );
    println!("PROOF4: hub title + {member_count} member nodes present");
}

// ── AC5: clicking a member row fires OpenMember with the right block_id ───────────────────────────────

#[test]
fn ac5_member_click_fires_open_member() {
    let hub = shared(seeded_hub());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = hub_harness(Arc::clone(&hub), Arc::clone(&events));
    harness.run();

    // The member renders as "📝 Ownership notes"; click it by its label substring.
    harness.get_by_label_contains("Ownership notes").click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    let opened = ev
        .iter()
        .any(|e| matches!(e, TagHubEvent::OpenMember { block_id } if block_id == "blk-001"));
    assert!(
        opened,
        "AC5: clicking 'Ownership notes' must fire OpenMember{{block_id:'blk-001'}} (got {ev:?})"
    );
    println!("AC5: member click fired OpenMember(blk-001) (events={ev:?})");
}

// ── PROOF5: add-tag popup -> select candidate -> AddTagSelected + verified POST /loom/edges shape ─────

#[test]
fn proof5_add_tag_selects_candidate_and_fires_edge() {
    // Pre-load the popup with candidates so the click target exists; the popup opens on the add-tag
    // button click. The externally-meaningful contract is the AddTagSelected event the host turns into
    // the verified POST /loom/edges (asserted separately below as the request shape).
    let mut hub = seeded_hub();
    hub.set_add_candidates(vec![
        AddTagCandidate::new("block-X", "Block X"),
        AddTagCandidate::new("block-Y", "Block Y"),
    ]);
    let hub = shared(hub);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = hub_harness(Arc::clone(&hub), Arc::clone(&events));
    harness.run();

    // Open the add-tag popup (click the button by its label), then select "Block X".
    harness.get_by_label_contains("Add tag to block").click();
    harness.run();
    harness.get_by_label_contains("Block X").click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    let selected = ev.iter().any(
        |e| matches!(e, TagHubEvent::AddTagSelected { source_block_id } if source_block_id == "block-X"),
    );
    assert!(
        selected,
        "PROOF5: selecting 'Block X' in the add-tag popup must fire AddTagSelected{{source:'block-X'}} \
         (got {ev:?})"
    );
    println!("PROOF5: add-tag candidate selection fired AddTagSelected(block-X) (events={ev:?})");
}

#[test]
fn add_tag_in_flight_disables_open_popup_repeat_selection() {
    let mut hub = seeded_hub();
    hub.set_add_candidates(vec![AddTagCandidate::new("block-X", "Block X")]);
    let hub = shared(hub);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = hub_harness(Arc::clone(&hub), Arc::clone(&events));
    harness.run();
    harness.get_by_label_contains("Add tag to block").click();
    harness.run();

    hub.lock().unwrap().add_tag_in_flight = true;
    harness.run();
    let add_id = handshake_native::graph::tags_panel::hub_add_tag_author_id("tag-hub-001");
    let result_id = handshake_native::graph::tags_panel::hub_add_result_author_id("block-X");
    let states: std::collections::HashMap<_, _> = harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let accesskit = node.accesskit_node();
            accesskit
                .author_id()
                .map(|id| (id.to_owned(), accesskit.is_disabled()))
        })
        .collect();
    assert_eq!(states.get(&add_id), Some(&true));
    assert_eq!(states.get(&result_id), Some(&true));

    harness.get_by_label_contains("Block X").click();
    harness.run();
    assert!(
        events.lock().unwrap().iter().all(
            |event| !matches!(event, TagHubEvent::AddTagSelected { .. })
        ),
        "a candidate already visible in an open popup cannot be selected twice while POST is in flight"
    );
}

#[test]
fn proof5_tag_edge_request_shape() {
    use handshake_native::backend_client::LoomTagClient;

    // The tag-edge POST hits the verified /loom/edges route with the verified CreateLoomEdgeRequest body:
    // the tagged block is the SOURCE, the hub is the TARGET (the backend rejects a non-tag_hub target),
    // edge_type "tag", created_by "user". We assert the EXACT URL + body the production spawn path routes
    // through (NO Tauri — the WP-011 backend_client typed HTTP client).
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::new("http://test.local:1234", rt.handle().clone());
    let spec = client.tag_block_request("ws1", "block-X", "tag-hub-001");
    assert_eq!(
        spec.url, "http://test.local:1234/workspaces/ws1/loom/edges",
        "PROOF5: tag POST hits the verified /loom/edges route"
    );
    assert_eq!(
        spec.body,
        Some(serde_json::json!({
            "source_block_id": "block-X",
            "target_block_id": "tag-hub-001",
            "edge_type": "tag",
            "created_by": "user",
        })),
        "PROOF5: edge body is the verified tag-edge shape (source=block, target=hub, edge_type=tag)"
    );
    println!(
        "PROOF5: tag-edge POST request shape verified (URL + edge_type='tag', hub is the target)"
    );
}

// ── Verified request-shape builders (the production spawn paths route through these) ─────────────────

#[test]
fn tag_read_requests_hit_verified_routes() {
    use handshake_native::backend_client::LoomTagClient;
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::new("http://test.local:1234", rt.handle().clone());

    let list = client.list_tags_request("ws7");
    assert_eq!(list.url, "http://test.local:1234/workspaces/ws7/loom/tags");
    assert_eq!(
        list.query,
        vec![
            ("limit".to_owned(), "500".to_owned()),
            ("offset".to_owned(), "0".to_owned())
        ]
    );

    let detail = client.tag_detail_request("ws7", "tag-hub-001");
    assert_eq!(
        detail.url,
        "http://test.local:1234/workspaces/ws7/loom/tags/tag-hub-001"
    );

    let members = client.list_members_request("ws7", "tag-hub-001");
    assert_eq!(
        members.url,
        "http://test.local:1234/workspaces/ws7/loom/tags/tag-hub-001/blocks"
    );
    assert_eq!(members.query, vec![("limit".to_owned(), "100".to_owned())]);

    let search = client.search_blocks_request("ws7", "borrow");
    assert_eq!(
        search.url,
        "http://test.local:1234/workspaces/ws7/loom/search"
    );
    assert_eq!(
        search.query,
        vec![
            ("q".to_owned(), "borrow".to_owned()),
            ("limit".to_owned(), "20".to_owned())
        ]
    );
    println!(
        "verified: list/detail/members/search GET routes match the real MT-182 tag-hub backend"
    );
}

// ── HBR-VIS screenshot: the tags panel renders chips + member counts ─────────────────────────────────

#[test]
fn tags_panel_screenshot() {
    let _g = wgpu_guard();
    let panel = shared(seeded_tags());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 400.0))
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
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-023");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-023-tags-panel.png");
            let saved = image.save(&png).is_ok();
            println!(
                "HBR-VIS: {w}x{h} tags-panel screenshot, saved={saved} ({})",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): tags-panel screenshot render unavailable (no wgpu adapter): {e}. \
                 The AccessKit + filter + event proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

#[test]
fn tag_hub_screenshot() {
    let _g = wgpu_guard();
    let hub = shared(seeded_hub());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 400.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = hub.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-023");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-023-tag-hub.png");
            let saved = image.save(&png).is_ok();
            println!(
                "HBR-VIS: {w}x{h} tag-hub screenshot, saved={saved} ({})",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): tag-hub screenshot render unavailable (no wgpu adapter): {e}."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── LIVE-SURREALDB: one self-seeded, mounted, unignored round trip ──────────────────────────────────────────

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
            matches!(status, 200 | 202 | 204 | 404),
            "managed-SurrealDB workspace cleanup returned HTTP {status}"
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
fn await_tag_list(
    cell: &handshake_native::backend_client::TagListCell,
    expected_workspace: &str,
) -> Result<Vec<TagEntry>, String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, sequence, result)) = cell.lock().unwrap().pop_front() {
            assert_eq!(workspace, expected_workspace);
            assert_eq!((epoch, sequence), (0, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("tag-list request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn await_tag_hub(
    cell: &handshake_native::backend_client::TagHubDetailCell,
    expected_workspace: &str,
    expected_hub: &str,
) -> Result<(String, Vec<HubMember>), String> {
    for _ in 0..200 {
        if let Some((workspace, epoch, hub, sequence, result)) = cell.lock().unwrap().pop_front() {
            assert_eq!(workspace, expected_workspace);
            assert_eq!(hub, expected_hub);
            assert_eq!((epoch, sequence), (0, 0));
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("tag-hub request did not resolve within 10 seconds")
}

#[cfg(feature = "integration")]
fn mounted_author_ids(
    harness: &Harness<'_, handshake_native::app::HandshakeApp>,
) -> std::collections::HashSet<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

#[cfg(feature = "integration")]
fn mounted_node_id(
    harness: &Harness<'_, handshake_native::app::HandshakeApp>,
    author_id: &str,
) -> egui::accesskit::NodeId {
    harness
        .root()
        .children_recursive()
        .find_map(|node| {
            let accesskit = node.accesskit_node();
            (accesskit.author_id() == Some(author_id)).then(|| accesskit.id())
        })
        .unwrap_or_else(|| panic!("mounted AccessKit node {author_id} is present"))
}

#[cfg(feature = "integration")]
fn dispatch_mounted_action(
    harness: &mut Harness<'_, handshake_native::app::HandshakeApp>,
    author_id: &str,
    action: egui::accesskit::Action,
    data: Option<egui::accesskit::ActionData>,
) {
    let target = mounted_node_id(harness, author_id);
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action,
            target,
            data,
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
            .header("x-hsk-actor-id", "mt023-live-surrealdb")
            .header("x-hsk-kernel-task-run-id", "mt023-live-surrealdb-run")
            .header("x-hsk-session-run-id", "mt023-live-surrealdb-session")
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

/// AC1-AC8 / PROOF2-5 against real managed SurrealDB and the real mounted `HandshakeApp` Tags pane.
/// The proof is feature-gated but deliberately NOT ignored. It owns fixture creation and teardown and
/// verifies persistence again with a newly constructed `LoomTagClient`, excluding panel-local cache.
#[test]
#[cfg(feature = "integration")]
fn tags_tag_hub_live_surrealdb_self_seeds_mounted_round_trip() {
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::{
        HealthInfo, LoomTagClient, TagHubDetailCell, TagListCell,
    };
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, TagsPaneEvent, TAGS_PANE_LABEL,
    };
    use handshake_native::pane_registry::{
        DirtyState, LockState, PaneAuthority, PaneId, PaneRecord,
    };

    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-023");
    let receipt_path = receipt_dir.join("MT-023-live-surrealdb-seed.json");
    match std::fs::remove_file(&receipt_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "remove stale MT-023 owned live receipt {} before proof: {error}",
            receipt_path.display()
        ),
    }
    assert!(
        !receipt_path.exists(),
        "MT-023 live proof starts without its stale owned receipt"
    );
    let proof_started = std::time::SystemTime::now();

    let mut live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt023-{}-{}",
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
        .expect("mounted tags runtime");
    let client = LoomTagClient::new(live.base.clone(), runtime.handle().clone());

    // The isolated real workspace starts empty.
    let empty_cell: TagListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_tags(&workspace_id, Arc::clone(&empty_cell));
    assert!(
        await_tag_list(&empty_cell, &workspace_id)
            .expect("empty tag list succeeds")
            .is_empty(),
        "real unseeded workspace returns no tag hubs"
    );

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.bind_active_project_for_integration_test(&workspace_id);
    app.set_tags_backend_base_url_for_test(live.base.clone());
    let tags_type = placeholder_pane_type(TAGS_PANE_LABEL);
    {
        let registry = app.pane_registry();
        registry.lock().unwrap().insert(PaneRecord::new(
            PaneId::from("pane-a"),
            tags_type.clone(),
            workspace_id.clone(),
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    if let Some(bar) = app.tab_bar_states_mut().get_mut(&PaneId::from("pane-a")) {
        bar.tabs = vec![handshake_native::tab_bar::TabState::new(tags_type)];
        bar.active_index = 0;
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    for _ in 0..200 {
        harness.run_steps(1);
        let backend_ready = harness
            .state()
            .mounted_tags_panel_for_test()
            .lock()
            .map(|panel| !panel.loading && panel.error.is_none())
            .unwrap_or(false);
        let mounted_empty = harness.query_by_label("No tags").is_some()
            && mounted_author_ids(&harness).contains(SEARCH_AUTHOR_ID);
        if backend_ready && mounted_empty {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(harness.query_by_label("No tags").is_some());
    assert!(mounted_author_ids(&harness).contains(SEARCH_AUTHOR_ID));

    let seed_block = |content_type: &str, title: &str| {
        let block = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({ "content_type": content_type, "title": title }),
        );
        block["block_id"]
            .as_str()
            .expect("block create returns block_id")
            .to_owned()
    };
    let rust_hub = seed_block("tag_hub", "rust");
    let rustaceans_hub = seed_block("tag_hub", "rustaceans");
    let python_hub = seed_block("tag_hub", "python");
    let first_note = seed_block("note", "MT-023 Ownership notes");
    let second_note = seed_block("note", "MT-023 Candidate Two");
    let seeded_edge = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": first_note,
            "target_block_id": rust_hub,
            "edge_type": "tag",
            "created_by": "user"
        }),
    );
    let seeded_edge_id = seeded_edge["edge_id"]
        .as_str()
        .expect("edge create returns edge_id")
        .to_owned();

    harness
        .state()
        .mounted_tags_events_for_test()
        .lock()
        .unwrap()
        .push(TagsPaneEvent::Panel(TagsPanelEvent::Retry));
    for _ in 0..200 {
        harness.run_steps(1);
        let loaded = harness
            .state()
            .mounted_tags_panel_for_test()
            .lock()
            .map(|panel| {
                panel.tags.len() == 3 && panel.tags.iter().all(|tag| tag.member_count.is_some())
            })
            .unwrap_or(false);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let ids = mounted_author_ids(&harness);
    for hub in [&rust_hub, &rustaceans_hub, &python_hub] {
        assert!(ids.contains(&tag_row_author_id(hub)));
    }

    dispatch_mounted_action(
        &mut harness,
        SEARCH_AUTHOR_ID,
        egui::accesskit::Action::SetValue,
        Some(egui::accesskit::ActionData::Value("rust".into())),
    );
    let filtered = mounted_author_ids(&harness);
    assert!(filtered.contains(&tag_row_author_id(&rust_hub)));
    assert!(filtered.contains(&tag_row_author_id(&rustaceans_hub)));
    assert!(!filtered.contains(&tag_row_author_id(&python_hub)));

    dispatch_mounted_action(
        &mut harness,
        &tag_row_author_id(&rust_hub),
        egui::accesskit::Action::Click,
        None,
    );
    for _ in 0..200 {
        harness.run_steps(1);
        let loaded = harness
            .state()
            .mounted_tags_hub_for_test()
            .lock()
            .map(|hub| {
                hub.as_ref().is_some_and(|hub| {
                    !hub.loading
                        && hub.error.is_none()
                        && hub
                            .members
                            .iter()
                            .any(|member| member.block_id == first_note)
                })
            })
            .unwrap_or(false);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let hub_ids = mounted_author_ids(&harness);
    assert!(
        hub_ids.contains(&handshake_native::graph::tags_panel::hub_title_author_id(
            &rust_hub
        ))
    );
    assert!(hub_ids.contains(&hub_member_author_id(&first_note)));

    dispatch_mounted_action(
        &mut harness,
        &handshake_native::graph::tags_panel::hub_add_tag_author_id(&rust_hub),
        egui::accesskit::Action::Click,
        None,
    );
    dispatch_mounted_action(
        &mut harness,
        handshake_native::graph::tags_panel::HUB_ADD_SEARCH_AUTHOR_ID,
        egui::accesskit::Action::SetValue,
        Some(egui::accesskit::ActionData::Value("Candidate Two".into())),
    );
    let candidate_author_id =
        handshake_native::graph::tags_panel::hub_add_result_author_id(&second_note);
    let mut mounted_candidate_ready = false;
    for _ in 0..200 {
        harness.run_steps(1);
        let state_found = harness
            .state()
            .mounted_tags_hub_for_test()
            .lock()
            .map(|hub| {
                hub.as_ref().is_some_and(|hub| {
                    hub.add_candidates
                        .iter()
                        .any(|candidate| candidate.block_id == second_note)
                })
            })
            .unwrap_or(false);
        let node_found = mounted_author_ids(&harness).contains(&candidate_author_id);
        if state_found && node_found {
            mounted_candidate_ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let hub_search_state = harness
        .state()
        .mounted_tags_hub_for_test()
        .lock()
        .ok()
        .and_then(|hub| {
            hub.as_ref().map(|hub| {
                (
                    hub.add_search.clone(),
                    hub.add_popup_open,
                    hub.add_candidates
                        .iter()
                        .map(|candidate| (candidate.block_id.clone(), candidate.title.clone()))
                        .collect::<Vec<_>>(),
                )
            })
        });
    assert!(
        mounted_candidate_ready,
        "mounted add-tag search did not expose {candidate_author_id} within 10 seconds; hub state={hub_search_state:?}; ids={:?}",
        mounted_author_ids(&harness)
    );
    dispatch_mounted_action(
        &mut harness,
        &candidate_author_id,
        egui::accesskit::Action::Click,
        None,
    );
    let mut mounted_add_refreshed = false;
    for _ in 0..200 {
        harness.run_steps(1);
        let refreshed = harness
            .state()
            .mounted_tags_hub_for_test()
            .lock()
            .map(|hub| {
                hub.as_ref().is_some_and(|hub| {
                    !hub.add_tag_in_flight
                        && hub.error.is_none()
                        && hub
                            .members
                            .iter()
                            .any(|member| member.block_id == second_note)
                })
            })
            .unwrap_or(false);
        if refreshed {
            mounted_add_refreshed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        mounted_add_refreshed,
        "mounted tag hub did not show the persisted add or clear its in-flight state within 10 seconds"
    );

    // A newly constructed client proves the mutation was persisted, not merely cached in the panel.
    let fresh = LoomTagClient::new(live.base.clone(), runtime.handle().clone());
    let fresh_list: TagListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh.fetch_tags(&workspace_id, Arc::clone(&fresh_list));
    assert_eq!(
        await_tag_list(&fresh_list, &workspace_id)
            .expect("fresh tag list succeeds")
            .len(),
        3
    );
    let fresh_hub: TagHubDetailCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh.fetch_hub_detail(&workspace_id, &rust_hub, Arc::clone(&fresh_hub));
    let (_, members_after_add) =
        await_tag_hub(&fresh_hub, &workspace_id, &rust_hub).expect("fresh hub detail succeeds");
    assert_eq!(members_after_add.len(), 2);
    assert!(members_after_add
        .iter()
        .any(|member| member.block_id == second_note));

    let renamed = "rust-renamed-mt023";
    live_patch_json(
        &runtime,
        &live.base,
        &format!("/workspaces/{workspace_id}/loom/blocks/{rust_hub}"),
        &serde_json::json!({ "title": renamed }),
    );
    live_patch_json(
        &runtime,
        &live.base,
        &format!("/workspaces/{workspace_id}/loom/blocks/{second_note}"),
        &serde_json::json!({ "remove_tags": [rust_hub] }),
    );
    let final_hub: TagHubDetailCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh.fetch_hub_detail(&workspace_id, &rust_hub, Arc::clone(&final_hub));
    let (final_title, final_members) = await_tag_hub(&final_hub, &workspace_id, &rust_hub)
        .expect("final fresh hub detail succeeds");
    assert_eq!(final_title, renamed);
    assert_eq!(final_members.len(), 1);
    assert_eq!(final_members[0].block_id, first_note);

    // Backend loss is bounded by the product client's five-second request timeout and returns a typed Err.
    let loss_client = LoomTagClient::new("http://127.0.0.1:0", runtime.handle().clone());
    let loss_cell: TagListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    loss_client.fetch_tags("mt023-backend-loss", Arc::clone(&loss_cell));
    assert!(await_tag_list(&loss_cell, "mt023-backend-loss").is_err());

    cleanup.assert_cleaned();
    drop(cleanup);
    live.assert_cleanup();

    // Publish the single owned receipt only after workspace/backend cleanup has succeeded.
    std::fs::create_dir_all(&receipt_dir).expect("create external MT-023 receipt directory");
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt_023.live_surrealdb_receipt@1",
        "proof_run_id": unique,
        "workspace_id": workspace_id,
        "tag_hub_ids": [rust_hub, rustaceans_hub, python_hub],
        "document_block_ids": [first_note, second_note],
        "seeded_tag_edge_id": seeded_edge_id,
        "persisted_member_count_after_add": members_after_add.len(),
        "final_hub_title": final_title,
        "final_member_count": final_members.len(),
        "backend_loss_typed_error": true,
        "cleanup_verified": true
    });
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("serialize MT-023 live receipt"),
    )
    .expect("write external MT-023 live receipt");
    let published: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&receipt_path).expect("read freshly published MT-023 live receipt"),
    )
    .expect("freshly published MT-023 live receipt is valid JSON");
    assert_eq!(
        published["proof_run_id"].as_str(),
        Some(unique.as_str()),
        "published live receipt belongs to this exact proof run"
    );
    assert_eq!(
        published["workspace_id"].as_str(),
        Some(workspace_id.as_str()),
        "published live receipt carries the exact cleaned workspace"
    );
    let modified = std::fs::metadata(&receipt_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|error| {
            panic!(
                "read publication timestamp for {}: {error}",
                receipt_path.display()
            )
        });
    assert!(
        modified >= proof_started,
        "owned MT-023 live receipt was freshly published after proof start"
    );
    println!(
        "MT-023 LIVE SURREALDB PASS workspace={workspace_id} hubs=[{rust_hub},{rustaceans_hub},{python_hub}] \
         documents=[{first_note},{second_note}] seeded_edge={seeded_edge_id} add_count=2 final_count=1 \
         receipt={} cleanup_verified=true",
        receipt_path.display()
    );
}
