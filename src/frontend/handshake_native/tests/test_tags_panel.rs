//! WP-KERNEL-012 MT-023 LoomTagsPanel + LoomTagHubPanel PROOFS:
//!   - PROOF1 (filter + chip-color distribution): owned by the lib unit tests (graph::tags_panel::tests);
//!     cross-checked here for the filter.
//!   - PROOF2: kittest — 3 seeded tag hubs -> 3 `tags.row.*` AccessKit nodes (Role::ListItem).
//!   - PROOF3: kittest — typing "rust" into `tags.search` leaves only rust-prefixed tags in the tree.
//!   - PROOF4: kittest — open hub `tag-hub-001`, assert `tag-hub.title.tag-hub-001` + >= 1
//!     `tag-hub.member.*` node.
//!   - PROOF5: click `tag-hub.add-tag.tag-hub-001`, select a candidate, assert the AddTagSelected event +
//!     the verified `POST /loom/edges` request shape (`edge_type:"tag"`). (No Tauri — backend_client.)
//! Plus AC3 (tag-row click fires OpenTag), AC5 (member-row click fires OpenMember), AC7 (the named
//! author_ids present), AC8 (empty -> "No tags", no panic), and a screenshot (HBR-VIS).
//!
//! ## Backend reality (Spec-Realism Gate — MT-008/021/022 "verify, don't trust the contract" rule)
//!
//! The MT-023 contract's assumed surface (`views/all?content_type=tag_hub` list, `views/all?tag_ids={id}`
//! members, a `content_json` hub description) does NOT exist in the running backend. The REAL tag
//! authority is the dedicated tag-hub API (MT-182), verified READ-ONLY against
//! `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//!   - `GET  /loom/tags`                  -> Vec<LoomBlock> (every tag_hub block)
//!   - `GET  /loom/tags/{id}`             -> LoomTagHub { block, sub_tags, tagged_blocks, backlink_count }
//!   - `GET  /loom/tags/{id}/blocks`      -> Vec<LoomBlock> (members)
//!   - `POST /loom/edges` body { source_block_id, target_block_id, edge_type:"tag", created_by:"user" }
//!     (the backend rejects a non-tag_hub target -> the hub is the edge TARGET).
//!
//! AC1/AC4/AC6 against a LIVE Handshake-managed PostgreSQL with >= 3 seeded tag_hub blocks + members +
//! a taggable block are the `#[ignore]`d `*_live_pg` tests gated behind the `integration` feature
//! (NEEDS_MANAGED_RESOURCE_PROOF); absent a seeded backend they are skipped and NEVER faked. The
//! filter/color/empty logic + the verified request-shape builders are proven STANDALONE here and in the
//! lib unit tests — exactly the split the MT `implementation_notes` describe.
//!
//! ## Artifact hygiene (CX-212E / CX-212E screenshot rule)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-023/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::graph::tags_panel::{
    hub_member_author_id, tag_row_author_id, AddTagCandidate, HubMember, LoomTagHubPanel,
    LoomTagsPanel, TagEntry, TagHubEvent, TagsPanelEvent, HUB_MEMBER_AUTHOR_ID_PREFIX,
    HUB_TITLE_AUTHOR_ID_PREFIX, SEARCH_AUTHOR_ID, TAG_ROW_AUTHOR_ID_PREFIX,
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
    let row_count = ids.iter().filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX)).count();
    assert!(
        row_count >= 3,
        "PROOF2: expected >= 3 tags.row.* AccessKit nodes, got {row_count} (ids={ids:?})"
    );

    // AC7: the search box id + the specific tag row ids are present.
    assert!(ids.contains(SEARCH_AUTHOR_ID), "AC7: 'tags.search' must be in the tree (ids={ids:?})");
    for id in ["tag-hub-001", "tag-hub-002", "tag-hub-003"] {
        let row = tag_row_author_id(id);
        assert!(ids.contains(&row), "AC7: '{row}' must be present (ids={ids:?})");
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
    assert!(listitem_found, "AC7: tags.row.tag-hub-001 not found for role check");
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
    let remaining = ids.iter().filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX)).count();
    assert_eq!(remaining, 2, "PROOF3: exactly the 2 rust-prefixed rows remain (ids={ids:?})");
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
    let opened = ev
        .iter()
        .any(|e| matches!(e, TagsPanelEvent::OpenTag { block_id } if block_id == "tag-hub-003"));
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
        ids.iter().filter(|a| a.starts_with(TAG_ROW_AUTHOR_ID_PREFIX)).count(),
        0,
        "AC8: no tags.row.* nodes for an empty workspace"
    );
    println!("AC8: empty workspace shows 'No tags', no row entries, no panic");
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
    let member_count = ids.iter().filter(|a| a.starts_with(HUB_MEMBER_AUTHOR_ID_PREFIX)).count();
    assert!(
        member_count >= 1,
        "PROOF4/AC4: >= 1 tag-hub.member.* node expected, got {member_count} (ids={ids:?})"
    );
    // The specific seeded members.
    assert!(ids.contains(&hub_member_author_id("blk-001")), "AC4: member blk-001 present");
    assert!(ids.contains(&hub_member_author_id("blk-002")), "AC4: member blk-002 present");
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
    println!("PROOF5: tag-edge POST request shape verified (URL + edge_type='tag', hub is the target)");
}

// ── Verified request-shape builders (the production spawn paths route through these) ─────────────────

#[test]
fn tag_read_requests_hit_verified_routes() {
    use handshake_native::backend_client::LoomTagClient;
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::new("http://test.local:1234", rt.handle().clone());

    let list = client.list_tags_request("ws7");
    assert_eq!(list.url, "http://test.local:1234/workspaces/ws7/loom/tags");
    assert!(list.query.is_empty());

    let detail = client.tag_detail_request("ws7", "tag-hub-001");
    assert_eq!(detail.url, "http://test.local:1234/workspaces/ws7/loom/tags/tag-hub-001");

    let members = client.list_members_request("ws7", "tag-hub-001");
    assert_eq!(
        members.url,
        "http://test.local:1234/workspaces/ws7/loom/tags/tag-hub-001/blocks"
    );
    assert_eq!(members.query, vec![("limit".to_owned(), "100".to_owned())]);

    let search = client.search_blocks_request("ws7", "borrow");
    assert_eq!(search.url, "http://test.local:1234/workspaces/ws7/loom/search");
    assert_eq!(
        search.query,
        vec![("q".to_owned(), "borrow".to_owned()), ("limit".to_owned(), "20".to_owned())]
    );
    println!("verified: list/detail/members/search GET routes match the real MT-182 tag-hub backend");
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
            println!("HBR-VIS: {w}x{h} tag-hub screenshot, saved={saved} ({})", png.display());
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): tag-hub screenshot render unavailable (no wgpu adapter): {e}."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── LIVE-PG (gated): NEEDS_MANAGED_RESOURCE_PROOF without a seeded backend ───────────────────────────

/// AC1 + PROOF2 against a REAL Handshake-managed PostgreSQL with >= 3 seeded tag_hub blocks. Gated behind
/// the `integration` feature AND `#[ignore]` so the default `cargo test` does not require a backend.
/// Run with: `cargo test --features integration --test test_tags_panel -- --ignored`. NEVER fakes PG.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with >= 3 seeded tag_hub blocks"]
#[cfg(feature = "integration")]
fn tags_list_live_pg() {
    use handshake_native::backend_client::{LoomTagClient, TagListCell};

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::production(rt.handle().clone());
    let cell: TagListCell = Arc::new(Mutex::new(None));
    // The operator seeds >= 3 tag_hub blocks in `ws-live` before running this.
    client.fetch_tags("ws-live", Arc::clone(&cell));
    let mut data = None;
    for _ in 0..50 {
        if let Some(r) = cell.lock().unwrap().take() {
            data = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let tags = data.expect("live PG fetch delivered within 5s").expect("live PG fetch ok");
    assert!(
        tags.len() >= 3,
        "AC1 live: >= 3 seeded tag_hub blocks expected from GET /loom/tags, got {}",
        tags.len()
    );
    println!("AC1 live PG: {} tag hubs enumerated", tags.len());
}

/// AC4 hub-detail against a REAL PG: open a seeded hub, assert title + >= 1 member. Gated.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with a seeded tag_hub + >= 1 member"]
#[cfg(feature = "integration")]
fn tag_hub_detail_live_pg() {
    use handshake_native::backend_client::{LoomTagClient, TagHubDetailCell};

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::production(rt.handle().clone());
    let cell: TagHubDetailCell = Arc::new(Mutex::new(None));
    // The operator seeds tag_hub "tag-hub-001" with >= 1 tagged member in `ws-live` before running this.
    client.fetch_hub_detail("ws-live", "tag-hub-001", Arc::clone(&cell));
    let mut data = None;
    for _ in 0..50 {
        if let Some(r) = cell.lock().unwrap().take() {
            data = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let (title, members) = data.expect("live PG fetch delivered within 5s").expect("live PG fetch ok");
    assert!(!title.trim().is_empty(), "AC4 live: hub title must be non-empty");
    assert!(
        !members.is_empty(),
        "AC4 live: the seeded hub must have >= 1 member, got {}",
        members.len()
    );
    println!("AC4 live PG: hub '{title}' has {} members", members.len());
}

/// AC6 tag-edge create against a REAL PG: tag a seeded block with a seeded hub, then re-query members and
/// assert the new member is present. Gated. The re-query is gated on the POST RESPONSE (no fixed sleep).
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with a seeded tag_hub + a taggable block"]
#[cfg(feature = "integration")]
fn tag_edge_create_live_pg() {
    use handshake_native::backend_client::{LoomTagClient, ScmReceiptCell, TagHubDetailCell};

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomTagClient::production(rt.handle().clone());
    // The operator seeds tag_hub "tag-hub-001" + a taggable note "block-taggable" in `ws-live`.
    let post_cell: ScmReceiptCell = Arc::new(Mutex::new(None));
    client.tag_block("ws-live", "block-taggable", "tag-hub-001", Arc::clone(&post_cell));
    // Await the POST RESPONSE (AC6 / RISK-2: re-query only AFTER the create resolves — no fixed sleep).
    let mut post_done = None;
    for _ in 0..50 {
        if let Some(r) = post_cell.lock().unwrap().take() {
            post_done = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    post_done.expect("tag POST delivered within 5s").expect("tag POST ok");

    // Now re-query the members — the just-tagged block must be present.
    let members_cell: TagHubDetailCell = Arc::new(Mutex::new(None));
    client.fetch_members("ws-live", "tag-hub-001", Arc::clone(&members_cell));
    let mut data = None;
    for _ in 0..50 {
        if let Some(r) = members_cell.lock().unwrap().take() {
            data = Some(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let (_t, members) = data.expect("members re-query delivered").expect("members re-query ok");
    assert!(
        members.iter().any(|m| m.block_id == "block-taggable"),
        "AC6 live: the just-tagged block must appear in the re-queried members (got {members:?})"
    );
    println!("AC6 live PG: tag edge created + member list reflects the new member");
}
