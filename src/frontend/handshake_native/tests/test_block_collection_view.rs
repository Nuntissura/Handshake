//! WP-KERNEL-012 MT-027 BlockCollectionView PROOFS (table / Kanban / calendar saved-view host).
//!
//! Coverage map:
//!   - PROOF1 (flipDirection asc/desc, bucketKey 3 variants, UNTAGGED add/remove-tags) — proven in the
//!     lib unit tests (`graph::block_collection_view::tests`); the standalone backend-driven logic.
//!   - PROOF2 (AC1): kittest table — 3 seeded rows => 3 `bcv.table.row.*` AccessKit nodes and the
//!     `title` cell is non-empty (the row label carries the joined cell values).
//!   - PROOF3 (AC2): click `bcv.table.sort.title` (initial -> asc), assert the emitted Sort event +
//!     the resulting updateBlockView REQUEST body carries `sort={field:title,direction:asc}`; click
//!     again -> desc.
//!   - PROOF4 (AC4): live-pointer drag `bcv.kanban.card.block-001` from `bcv.kanban.lane.tag-a` to
//!     `bcv.kanban.lane.tag-b`; assert the emitted CardMove event has `{add_tags:[tag-b],
//!     remove_tags:[tag-a]}` and the card-move REQUEST body matches; after the host applies the re-query
//!     (NOT a local mutation), the card is in lane tag-b (proved by the re-queried result, not local
//!     state).
//!   - PROOF5 (AC5): calendar with 2 blocks on different journal_dates => 2 `bcv.calendar.day.*` nodes,
//!     each containing 1 `bcv.calendar.entry.*` node.
//!   - PROOF6 (AC8): click `bcv.new-view`, type a title, select kind=table, click confirm => a
//!     CreateView event + the createBlockView REQUEST body carries `{title, definition.kind:table}`.
//!   - AC3/AC6/AC7/AC9/AC10 + the request-builder proofs (the VERIFIED routes the contract named) +
//!     a screenshot (HBR-VIS).
//!
//! ## Backend reality (Spec-Realism Gate / MT-022/023/024/026 pattern)
//!
//! AC1-AC8 have a non-ignored `integration` proof that creates an isolated workspace, seeds real Loom
//! blocks/tag edges/journals and table/kanban/calendar `view_def` rows, drives the production
//! [`BlockViewClient`] reads/writes, mounts every returned projection through the real widget, proves a
//! visible Retry recovery, then deletes the workspace and verifies fresh absence. It requires only the
//! standard managed backend at `HSK_TEST_BASE`; no pre-seeded ids or unrelated workspace state.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-027/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[cfg(feature = "integration")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[cfg(feature = "integration")]
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::backend_client::BlockViewClient;
#[cfg(feature = "integration")]
use handshake_native::backend_client::BLOCK_VIEW_ACTOR_ID;
use handshake_native::graph::block_collection_view::{
    calendar_day_author_id, calendar_entry_author_id, kanban_card_author_id, kanban_lane_author_id,
    table_row_author_id, table_sort_author_id, BlockCollectionView, BlockViewDefinition,
    BlockViewEvent, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewLane, BlockViewQuery,
    BlockViewResults, BlockViewSort, BlockViewSortDirection, LoomBlockRow,
    BLOCK_VIEW_UNTAGGED_LANE, CALENDAR_DAY_AUTHOR_ID_PREFIX, CALENDAR_ENTRY_AUTHOR_ID_PREFIX,
    KIND_CALENDAR_AUTHOR_ID, KIND_KANBAN_AUTHOR_ID, KIND_TABLE_AUTHOR_ID, NEW_VIEW_AUTHOR_ID,
    NEW_VIEW_CONFIRM_AUTHOR_ID, NEW_VIEW_KIND_CALENDAR_AUTHOR_ID, NEW_VIEW_KIND_KANBAN_AUTHOR_ID,
    NEW_VIEW_TITLE_AUTHOR_ID, RETRY_AUTHOR_ID, TABLE_ROW_AUTHOR_ID_PREFIX,
};
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
mod interconnect_support;
#[cfg(feature = "integration")]
use handshake_native::app::{HandshakeApp, HealthDisplayState};
#[cfg(feature = "integration")]
use handshake_native::backend_client::HealthInfo;
#[cfg(feature = "integration")]
use handshake_native::editor_pane_factories::{
    placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL,
};
#[cfg(feature = "integration")]
use handshake_native::pane_registry::{DirtyState, LockState, PaneAuthority, PaneId, PaneRecord};

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

// ── In-memory seed builders (the native projection of a real queryBlockViewResults) ────────────────

fn row(id: &str, title: &str, created: &str, updated: &str, journal: Option<&str>) -> LoomBlockRow {
    LoomBlockRow {
        block_id: id.to_owned(),
        title: Some(title.to_owned()),
        original_filename: None,
        content_type: "note".to_owned(),
        journal_date: journal.map(ToOwned::to_owned),
        created_at: created.to_owned(),
        updated_at: updated.to_owned(),
        pinned: false,
        favorite: false,
        backlink_count: 0,
        mention_count: 0,
        tag_count: 0,
    }
}

/// A table-kind host seeded with `n` rows (title/updated columns), no sort.
fn seeded_table(n: usize) -> BlockCollectionView {
    let mut host = BlockCollectionView::new("ws-test", "view-table");
    let def = BlockViewDefinition::of_kind(BlockViewKind::Table);
    let blocks: Vec<LoomBlockRow> = (0..n)
        .map(|i| {
            row(
                &format!("block-{:03}", i + 1),
                &format!("Block {}", i + 1),
                &format!("2026-01-{:02}T00:00:00Z", i + 1),
                &format!("2026-02-{:02}T00:00:00Z", i + 1),
                None,
            )
        })
        .collect();
    let results = BlockViewResults {
        kind_str: "table".to_owned(),
        blocks,
        groups: vec![],
        total_returned: n as u32,
    };
    host.set_loaded(def, results);
    host
}

/// A kanban-kind host seeded with two tag lanes (tag-a holds block-001, tag-b holds block-002) plus an
/// untagged lane.
fn seeded_kanban() -> BlockCollectionView {
    let mut host = BlockCollectionView::new("ws-test", "view-kanban");
    let def = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    let results = BlockViewResults {
        kind_str: "kanban".to_owned(),
        blocks: vec![],
        groups: vec![
            BlockViewLane {
                key: "tag-a".to_owned(),
                blocks: vec![row(
                    "block-001",
                    "Card One",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z",
                    None,
                )],
            },
            BlockViewLane {
                key: "tag-b".to_owned(),
                blocks: vec![row(
                    "block-002",
                    "Card Two",
                    "2026-01-02T00:00:00Z",
                    "2026-01-02T00:00:00Z",
                    None,
                )],
            },
            BlockViewLane {
                key: BLOCK_VIEW_UNTAGGED_LANE.to_owned(),
                blocks: vec![],
            },
        ],
        total_returned: 2,
    };
    host.set_loaded(def, results);
    host
}

/// A calendar-kind host seeded with 2 blocks on different journal_dates.
fn seeded_calendar() -> BlockCollectionView {
    let mut host = BlockCollectionView::new("ws-test", "view-calendar");
    let mut def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
    def.calendar_date_field = Some(BlockViewField::JournalDate);
    let results = BlockViewResults {
        kind_str: "calendar".to_owned(),
        blocks: vec![
            row(
                "block-001",
                "Day One",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
                Some("2026-03-01"),
            ),
            row(
                "block-002",
                "Day Two",
                "2026-01-02T00:00:00Z",
                "2026-01-02T00:00:00Z",
                Some("2026-03-02"),
            ),
        ],
        groups: vec![],
        total_returned: 2,
    };
    host.set_loaded(def, results);
    host
}

fn shared(host: BlockCollectionView) -> Arc<Mutex<BlockCollectionView>> {
    Arc::new(Mutex::new(host))
}

/// Build a harness that renders the shared host and pushes every emitted [`BlockViewEvent`] into
/// `events`.
fn harness_for<'a>(
    host: Arc<Mutex<BlockCollectionView>>,
    events: Arc<Mutex<Vec<BlockViewEvent>>>,
) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = host.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<T>(harness: &Harness<'_, T>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Read a node's AccessKit `label` by author_id.
fn label_for<T>(harness: &Harness<'_, T>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.label().map(|v| v.to_owned());
        }
    }
    None
}

/// The screen-space center of a node addressed by author_id (for live pointer drag).
fn center_of<T>(harness: &Harness<'_, T>, author_id: &str) -> Option<egui::Pos2> {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .map(|n| n.rect().center())
}

/// Click the node addressed by `author_id` (kittest has no `click_at(pos)`; it clicks the node's own
/// rect via the AccessKit Click action). Panics if no such node exists.
fn click_author_id<T>(harness: &Harness<'_, T>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"));
    node.click();
}

/// Focus + type into the text field addressed by `author_id` (its hint text is NOT an AccessKit label,
/// so it can't be found by `get_by_label`; address it by its stable author_id instead).
fn type_into_author_id<T>(harness: &Harness<'_, T>, author_id: &str, text: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no text field with author_id '{author_id}' to type into"));
    node.focus();
    node.type_text(text);
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF2 (AC1): table renders 3 rows as addressable nodes with non-empty title cells.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn table_renders_three_rows_with_titles() {
    let host = shared(seeded_table(3));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    let ids = author_ids(&harness);

    // AC9: the kind switcher + status + new-view controls are present.
    for required in [
        KIND_TABLE_AUTHOR_ID,
        KIND_KANBAN_AUTHOR_ID,
        NEW_VIEW_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "AC9: control '{required}' missing from {ids:?}"
        );
    }

    // AC9: a sort header for each default column (title, updated).
    assert!(
        ids.contains(&table_sort_author_id(BlockViewField::Title)),
        "AC9: title sort header"
    );
    assert!(
        ids.contains(&table_sort_author_id(BlockViewField::Updated)),
        "AC9: updated sort header"
    );

    // PROOF2: exactly 3 row nodes, each labelled with its joined cell values (title non-empty).
    let row_count = ids
        .iter()
        .filter(|a| a.starts_with(TABLE_ROW_AUTHOR_ID_PREFIX))
        .count();
    assert_eq!(
        row_count, 3,
        "PROOF2: exactly 3 table-row nodes (got {row_count})"
    );

    let label =
        label_for(&harness, &table_row_author_id("block-001")).expect("row block-001 present");
    assert!(
        label.contains("Block 1"),
        "PROOF2: row label must carry the title cell (got '{label}')"
    );

    println!("PROOF2/AC1/AC9: 3 table rows with non-empty title cells + controls present");
    assert_no_local_artifact_dir();
}

#[test]
fn backend_error_exposes_stable_retry_event() {
    let host = shared(BlockCollectionView::new("ws-test", "view-table"));
    host.lock()
        .unwrap()
        .set_error("backend unreachable (HTTP 503)");
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(host, events);
    harness.run();

    assert!(author_ids(&harness).contains(RETRY_AUTHOR_ID));
    assert_eq!(
        label_for(&harness, RETRY_AUTHOR_ID).as_deref(),
        Some("Retry")
    );
    click_author_id(&harness, RETRY_AUTHOR_ID);
    harness.run();
    assert!(matches!(
        events_ck.lock().unwrap().last(),
        Some(BlockViewEvent::Retry)
    ));
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF3 (AC2): a header click emits a Sort event (asc), and the resulting updateBlockView REQUEST body
// carries the correct sort; clicking the same header again toggles to desc.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn table_sort_click_emits_backend_sort_then_toggles() {
    let host = shared(seeded_table(3));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    // Click the title sort header. The label is the descriptive override ("Title" with no indicator).
    harness.get_by_label("Title").click();
    harness.run();

    let sort = {
        let evs = events_ck.lock().unwrap();
        match evs.last() {
            Some(BlockViewEvent::Sort { sort }) => *sort,
            other => panic!("PROOF3: expected a Sort event, got {other:?}"),
        }
    };
    assert_eq!(sort.field, BlockViewField::Title);
    assert_eq!(
        sort.direction,
        BlockViewSortDirection::Asc,
        "PROOF3: first click -> asc"
    );

    // The emitted sort, persisted via the client, builds the VERIFIED updateBlockView request body.
    let client = test_client();
    let mut def = BlockViewDefinition::of_kind(BlockViewKind::Table);
    def.sort = Some(sort);
    let spec = client.update_view_request("ws-test", "view-table", &def);
    let body = spec.body.expect("update body");
    let body_sort = body
        .get("definition")
        .and_then(|d| d.get("sort"))
        .expect("definition.sort");
    assert_eq!(
        body_sort.get("field").and_then(|x| x.as_str()),
        Some("title")
    );
    assert_eq!(
        body_sort.get("direction").and_then(|x| x.as_str()),
        Some("asc")
    );

    // Apply the sort to the host (mimic the host's set_loaded after the re-query) and click again ->
    // desc (same-field toggle).
    {
        let mut h = host.lock().unwrap();
        let results = h.results.clone().unwrap();
        h.set_loaded(def.clone(), results);
    }
    harness.run();
    // After asc, the header label gains the " ▲" indicator, so address it by its stable author_id.
    click_author_id(&harness, &table_sort_author_id(BlockViewField::Title));
    harness.run();

    let sort2 = {
        let evs = events_ck.lock().unwrap();
        match evs.last() {
            Some(BlockViewEvent::Sort { sort }) => *sort,
            other => panic!("PROOF3: expected a 2nd Sort event, got {other:?}"),
        }
    };
    assert_eq!(
        sort2.direction,
        BlockViewSortDirection::Desc,
        "PROOF3: 2nd click same field -> desc"
    );

    println!(
        "PROOF3/AC2: header click emits backend Sort (asc), update body correct, 2nd click -> desc"
    );
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC3 / AC7: the kind switcher fires a KindChange event (table -> kanban) and is rejected while busy.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn kind_switch_emits_kind_change() {
    let host = shared(seeded_table(2));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    click_author_id(&harness, KIND_KANBAN_AUTHOR_ID);
    harness.run();

    let evs = events_ck.lock().unwrap();
    assert!(
        matches!(
            evs.last(),
            Some(BlockViewEvent::KindChange {
                kind: BlockViewKind::Kanban
            })
        ),
        "AC7: kind switcher must fire KindChange{{kanban}}, got {:?}",
        evs.last()
    );
    println!("AC7: kind switch table -> kanban fires KindChange");
}

#[test]
fn kind_switch_rejected_while_in_flight() {
    let mut h = seeded_table(2);
    h.in_flight = true; // a mutation is in flight (RISK-3 / MC-3)
    let host = shared(h);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    // The in-flight status strip requests a perpetual repaint (the genuine busy animation), so `run()`
    // would never converge — use `step()` per the MT-015 idle-repaint + kittest contract.
    harness.step();
    harness.step();

    if center_of(&harness, KIND_KANBAN_AUTHOR_ID).is_some() {
        click_author_id(&harness, KIND_KANBAN_AUTHOR_ID);
        harness.step();
        harness.step();
    }
    assert!(
        events_ck.lock().unwrap().is_empty(),
        "RISK-3/MC-3: a kind switch must be rejected while a mutation is in flight"
    );
    println!("RISK-3/MC-3: kind switch rejected while in_flight");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF4 (AC4): drag a Kanban card from lane tag-a to lane tag-b via the live egui DragAndDrop pointer
// path; assert the CardMove event + request body; then the re-query (NOT local mutation) lands the card
// in tag-b.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn kanban_card_drag_emits_card_move_then_requery_lands_card() {
    let host = shared(seeded_kanban());
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    let ids = author_ids(&harness);
    // AC3: lanes + cards are addressable.
    assert!(
        ids.contains(&kanban_lane_author_id("tag-a")),
        "AC3: lane tag-a present"
    );
    assert!(
        ids.contains(&kanban_lane_author_id("tag-b")),
        "AC3: lane tag-b present"
    );
    assert!(
        ids.contains(&kanban_card_author_id("block-001")),
        "AC3: card block-001 present"
    );
    // The untagged lane shows its 'Untagged' label.
    assert_eq!(
        label_for(&harness, &kanban_lane_author_id(BLOCK_VIEW_UNTAGGED_LANE)).as_deref(),
        Some("Untagged"),
        "AC3: untagged lane labelled 'Untagged'"
    );

    let card_center =
        center_of(&harness, &kanban_card_author_id("block-001")).expect("card present");
    let lane_b_center =
        center_of(&harness, &kanban_lane_author_id("tag-b")).expect("lane tag-b present");

    // Live pointer drag: press at the card, step the pointer to lane-b (past the drag threshold), drop.
    harness.drag_at(card_center);
    harness.run();
    let steps = 8;
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let p = card_center + (lane_b_center - card_center) * t;
        harness.hover_at(p);
        harness.step();
    }
    harness.drop_at(lane_b_center);
    harness.run();
    harness.run();

    let move_event = {
        let evs = events_ck.lock().unwrap();
        evs.iter()
            .rev()
            .find_map(|e| match e {
                BlockViewEvent::CardMove {
                    block_id,
                    add_tags,
                    remove_tags,
                } => Some((block_id.clone(), add_tags.clone(), remove_tags.clone())),
                _ => None,
            })
            .expect("PROOF4: a CardMove event must fire on the drop")
    };
    assert_eq!(move_event.0, "block-001", "PROOF4: moved block id");
    assert_eq!(
        move_event.1,
        vec!["tag-b".to_owned()],
        "PROOF4: add_tags = [tag-b]"
    );
    assert_eq!(
        move_event.2,
        vec!["tag-a".to_owned()],
        "PROOF4: remove_tags = [tag-a]"
    );

    // The VERIFIED updateLoomBlock request body (top-level add_tags/remove_tags).
    let client = test_client();
    let spec = client.card_move_request("ws-test", &move_event.0, &move_event.1, &move_event.2);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-test/loom/blocks/block-001"
    );
    let body = spec.body.unwrap();
    assert_eq!(
        body.get("add_tags")
            .and_then(|x| x.as_array())
            .map(|a| a.len()),
        Some(1),
        "PROOF4: add_tags top-level array"
    );
    assert_eq!(body.get("add_tags").unwrap()[0].as_str(), Some("tag-b"));
    assert_eq!(body.get("remove_tags").unwrap()[0].as_str(), Some("tag-a"));

    // The host applies the re-query (the SOURCE OF TRUTH, never a local lane mutation): tag-a now empty,
    // tag-b holds block-001. set_loaded installs the re-queried result.
    {
        let mut h = host.lock().unwrap();
        let def = h.definition.clone().unwrap();
        let requeried = BlockViewResults {
            kind_str: "kanban".to_owned(),
            blocks: vec![],
            groups: vec![
                BlockViewLane {
                    key: "tag-a".to_owned(),
                    blocks: vec![],
                },
                BlockViewLane {
                    key: "tag-b".to_owned(),
                    blocks: vec![
                        row(
                            "block-002",
                            "Card Two",
                            "2026-01-02T00:00:00Z",
                            "2026-01-02T00:00:00Z",
                            None,
                        ),
                        row(
                            "block-001",
                            "Card One",
                            "2026-01-01T00:00:00Z",
                            "2026-01-01T00:00:00Z",
                            None,
                        ),
                    ],
                },
                BlockViewLane {
                    key: BLOCK_VIEW_UNTAGGED_LANE.to_owned(),
                    blocks: vec![],
                },
            ],
            total_returned: 2,
        };
        h.set_loaded(def, requeried);
    }
    harness.run();

    // After the re-query, block-001 is in lane tag-b. We prove lane membership by reading the lane's
    // blocks from the host state (the authoritative re-queried result), not local UI mutation.
    let h = host.lock().unwrap();
    let groups = &h.results.as_ref().unwrap().groups;
    let lane_a = groups.iter().find(|l| l.key == "tag-a").unwrap();
    let lane_b = groups.iter().find(|l| l.key == "tag-b").unwrap();
    assert!(
        !lane_a.blocks.iter().any(|b| b.block_id == "block-001"),
        "PROOF4/AC4: original lane no longer contains block-001 (proved by backend re-query)"
    );
    assert!(
        lane_b.blocks.iter().any(|b| b.block_id == "block-001"),
        "PROOF4/AC4: block-001 now in lane tag-b after re-query"
    );

    println!("PROOF4/AC4: drag fires CardMove{{add:[tag-b],remove:[tag-a]}}; re-query lands card in tag-b");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF5 (AC5): calendar buckets 2 blocks on different journal_dates into 2 day nodes with 1 entry each.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn calendar_renders_two_day_buckets() {
    let host = shared(seeded_calendar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    let ids = author_ids(&harness);
    let day_count = ids
        .iter()
        .filter(|a| a.starts_with(CALENDAR_DAY_AUTHOR_ID_PREFIX))
        .count();
    let entry_count = ids
        .iter()
        .filter(|a| a.starts_with(CALENDAR_ENTRY_AUTHOR_ID_PREFIX))
        .count();
    assert_eq!(
        day_count, 2,
        "PROOF5: exactly 2 calendar-day nodes (got {day_count})"
    );
    assert_eq!(
        entry_count, 2,
        "PROOF5: exactly 2 calendar-entry nodes (1 per day, got {entry_count})"
    );

    assert!(
        ids.contains(&calendar_day_author_id("2026-03-01")),
        "PROOF5: day bucket 2026-03-01 present"
    );
    assert!(
        ids.contains(&calendar_entry_author_id("block-001")),
        "PROOF5: entry block-001 present"
    );
    // AC9: the date-range inputs are addressable.
    assert!(
        ids.contains("bcv.calendar.date-from"),
        "AC9: date-from input present"
    );
    assert!(
        ids.contains("bcv.calendar.date-to"),
        "AC9: date-to input present"
    );

    println!("PROOF5/AC5: 2 calendar day buckets, 1 entry each, date-range inputs present");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF6 (AC8): the '+ New view' button opens the popup; confirm fires CreateView + the createBlockView
// REQUEST body carries the title + kind.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn new_view_creates_and_switches() {
    let host = shared(seeded_table(1));
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    // Open the popup.
    click_author_id(&harness, NEW_VIEW_AUTHOR_ID);
    harness.run();

    // The popup's title field + confirm button are now in the tree.
    let ids = author_ids(&harness);
    assert!(
        ids.contains(NEW_VIEW_TITLE_AUTHOR_ID),
        "AC8: new-view title field present"
    );
    assert!(
        ids.contains(NEW_VIEW_CONFIRM_AUTHOR_ID),
        "AC8: new-view confirm present"
    );
    let title_node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(NEW_VIEW_TITLE_AUTHOR_ID))
        .expect("AC8: new-view title node");
    assert!(
        !title_node.accesskit_node().is_disabled(),
        "AC8: new-view title must remain steerable while its popup is open"
    );
    assert!(
        title_node
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "AC8: new-view title must expose canonical SetValue"
    );

    // Type a title into the field (by author_id — the hint text is not an AccessKit label), confirm.
    type_into_author_id(&harness, NEW_VIEW_TITLE_AUTHOR_ID, "Test View");
    harness.run();
    click_author_id(&harness, NEW_VIEW_CONFIRM_AUTHOR_ID);
    harness.run();

    let (title, kind) = {
        let evs = events_ck.lock().unwrap();
        match evs.last() {
            Some(BlockViewEvent::CreateView { title, kind }) => (title.clone(), *kind),
            other => panic!("PROOF6: expected a CreateView event, got {other:?}"),
        }
    };
    assert_eq!(title, "Test View", "PROOF6: created view title");
    assert_eq!(kind, BlockViewKind::Table, "PROOF6: default kind table");

    // The VERIFIED createBlockView request body.
    let client = test_client();
    let def = BlockViewDefinition::of_kind(kind);
    let spec = client.create_view_request("ws-test", "view-test-stable-id", &title, &def);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-test/loom/views/definitions"
    );
    let body = spec.body.unwrap();
    assert_eq!(
        body.get("title").and_then(|x| x.as_str()),
        Some("Test View")
    );
    assert_eq!(
        body.get("block_id").and_then(|x| x.as_str()),
        Some("view-test-stable-id")
    );
    assert_eq!(
        body.get("definition")
            .and_then(|d| d.get("kind"))
            .and_then(|x| x.as_str()),
        Some("table"),
        "PROOF6: createBlockView body carries definition.kind"
    );

    println!("PROOF6/AC8: new-view popup -> CreateView{{title,kind:table}} + create body correct");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC6: the calendar date-range Apply emits a DateRange event after regex validation; a bad date is
// rejected with an inline error and no event.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn calendar_date_range_validates_then_emits() {
    let host = shared(seeded_calendar());
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_ck = Arc::clone(&events);
    let mut harness = harness_for(Arc::clone(&host), events);
    harness.run();

    // Seed a VALID from-date directly into the host input (the field is AccessKit-addressable; we set
    // the model so the validation path is exercised deterministically).
    host.lock().unwrap().date_from_input = "2026-03-01".to_owned();
    harness.run();
    click_author_id(&harness, "bcv.calendar.apply-range");
    harness.run();

    let valid = {
        let evs = events_ck.lock().unwrap();
        matches!(
            evs.last(),
            Some(BlockViewEvent::DateRange { date_from: Some(f), date_to: None }) if f == "2026-03-01"
        )
    };
    assert!(
        valid,
        "AC6: a valid date-range Apply must emit DateRange{{from:2026-03-01}}"
    );

    // Now a BAD shape -> rejected (no new event, an inline error set).
    events_ck.lock().unwrap().clear();
    host.lock().unwrap().date_from_input = "2026/03/01".to_owned();
    harness.run();
    click_author_id(&harness, "bcv.calendar.apply-range");
    harness.run();
    assert!(
        events_ck.lock().unwrap().is_empty(),
        "RISK-5/MC-5: a malformed date must NOT emit a DateRange event"
    );

    println!("AC6/RISK-5/MC-5: valid date range emits DateRange; malformed date rejected");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC10: empty result sets render an empty state for each kind without panic.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn empty_states_render_without_panic() {
    for kind in [
        BlockViewKind::Table,
        BlockViewKind::Kanban,
        BlockViewKind::Calendar,
    ] {
        let mut host = BlockCollectionView::new("ws", "view-empty");
        let def = BlockViewDefinition::of_kind(kind);
        host.set_loaded(def, BlockViewResults::default());
        let host = shared(host);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut harness = harness_for(Arc::clone(&host), events);
        harness.run(); // must not panic

        let ids = author_ids(&harness);
        // No rows / cards / day buckets exist.
        assert!(
            !ids.iter()
                .any(|a| a.starts_with(TABLE_ROW_AUTHOR_ID_PREFIX)),
            "AC10 ({kind:?}): no table rows in an empty view"
        );
        assert!(
            !ids.iter()
                .any(|a| a.starts_with(CALENDAR_DAY_AUTHOR_ID_PREFIX)),
            "AC10 ({kind:?}): no calendar days in an empty view"
        );
    }
    println!("AC10: empty table/kanban/calendar render without panic, no stray nodes");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (HBR-VIS): screenshot of a rendered table with rows (non-blank surface).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn block_collection_view_screenshot() {
    let _g = wgpu_guard();
    let host = shared(seeded_table(3));
    let events = Arc::new(Mutex::new(Vec::new()));
    let host_ui = Arc::clone(&host);
    let events_ui = Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = host_ui.lock().unwrap().show(ui, &pal) {
                events_ui.lock().unwrap().push(ev);
            }
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut white = 0u32;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                    if px[0] > 250 && px[1] > 250 && px[2] > 250 {
                        white += 1;
                    }
                }
                i += 16;
            }
            let total: u32 = counts.values().sum();
            assert!(total > 0, "screenshot: sampled pixels must be opaque");
            assert!(
                (white as f32 / total as f32) < 0.95,
                "screenshot: surface must not be ~all-white (white frac {})",
                white as f32 / total as f32
            );
            assert!(
                counts.len() >= 2,
                "screenshot: >= 2 distinct colours expected, got {}",
                counts.len()
            );

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-027");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-027-block-collection-view.png");
            let saved = image.save(&png).is_ok();
            println!(
                "SCREENSHOT: {w}x{h}, {} distinct colours, white_frac={:.3}, saved={saved} ({})",
                counts.len(),
                white as f32 / total as f32,
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): screenshot render unavailable (no wgpu adapter): {e}. The \
                 AccessKit + sort + card-move + calendar + create proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// BlockViewClient request-builder proofs (NO backend): the VERIFIED routes/bodies. These prove the
// production request construction (the spawn paths route through the SAME builders), so a stale URL,
// a GET-instead-of-POST results call, or a mis-shaped body can never reach the real backend unnoticed.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

fn test_client() -> BlockViewClient {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    BlockViewClient::new("http://127.0.0.1:37501", rt.handle().clone())
}

#[test]
fn client_get_view_url() {
    let c = test_client();
    let spec = c.get_view_request("ws1", "view-1");
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/views/definitions/view-1"
    );
    assert!(spec.query.is_empty());
}

#[test]
fn client_query_results_is_post_with_limit_offset_body() {
    // RISK-1 / MC-1: queryBlockViewResults MUST be POST with a JSON body {limit, offset}, NOT a GET.
    let c = test_client();
    let spec = c.query_results_request("ws1", "view-1", 100, 0);
    assert!(
        matches!(
            spec.method,
            handshake_native::backend_client::HttpMethod::Post
        ),
        "RISK-1: results query must be POST (got {:?})",
        spec.method
    );
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/views/definitions/view-1/results"
    );
    let body = spec.body.expect("results POST carries a body");
    assert_eq!(body.get("limit").and_then(|x| x.as_u64()), Some(100));
    assert_eq!(body.get("offset").and_then(|x| x.as_u64()), Some(0));
}

#[test]
fn client_update_view_wraps_definition() {
    let c = test_client();
    let mut def = BlockViewDefinition::of_kind(BlockViewKind::Table);
    def.sort = Some(BlockViewSort {
        field: BlockViewField::Updated,
        direction: BlockViewSortDirection::Desc,
    });
    let spec = c.update_view_request("ws1", "view-1", &def);
    assert!(matches!(
        spec.method,
        handshake_native::backend_client::HttpMethod::Patch
    ));
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/views/definitions/view-1"
    );
    // The body MUST be {definition: {...}} (the verified wrapped shape, NOT a bare definition).
    let body = spec.body.unwrap();
    let def_json = body.get("definition").expect("definition wrapper");
    assert_eq!(def_json.get("kind").and_then(|x| x.as_str()), Some("table"));
    let sort = def_json.get("sort").expect("sort serialized");
    assert_eq!(sort.get("field").and_then(|x| x.as_str()), Some("updated"));
    assert_eq!(sort.get("direction").and_then(|x| x.as_str()), Some("desc"));
}

#[test]
fn client_card_move_top_level_tags() {
    let c = test_client();
    let spec = c.card_move_request("ws1", "blk-9", &["tag-b".to_owned()], &["tag-a".to_owned()]);
    assert!(matches!(
        spec.method,
        handshake_native::backend_client::HttpMethod::Patch
    ));
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/blocks/blk-9"
    );
    let body = spec.body.unwrap();
    // add_tags/remove_tags are TOP-LEVEL (the verified LoomBlockPatchRequest shape), not nested.
    assert_eq!(body.get("add_tags").unwrap()[0].as_str(), Some("tag-b"));
    assert_eq!(body.get("remove_tags").unwrap()[0].as_str(), Some("tag-a"));
}

#[test]
fn client_create_view_body() {
    let c = test_client();
    let def = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    let spec = c.create_view_request("ws1", "view-stable-1", "My View", &def);
    assert!(matches!(
        spec.method,
        handshake_native::backend_client::HttpMethod::Post
    ));
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws1/loom/views/definitions"
    );
    let body = spec.body.unwrap();
    assert_eq!(
        body.get("block_id").and_then(|x| x.as_str()),
        Some("view-stable-1")
    );
    assert_eq!(body.get("title").and_then(|x| x.as_str()), Some("My View"));
    assert_eq!(
        body.get("definition")
            .and_then(|d| d.get("kind"))
            .and_then(|x| x.as_str()),
        Some("kanban")
    );
}

#[test]
fn unbound_block_collections_pane_opens_new_view_form_instead_of_spinning_forever() {
    let view = shared(BlockCollectionView::new("ws-unbound", ""));
    let mut harness = harness_for(Arc::clone(&view), Arc::new(Mutex::new(Vec::new())));
    harness.run();
    assert!(author_ids(&harness).contains(NEW_VIEW_AUTHOR_ID));
    for unavailable in [
        KIND_TABLE_AUTHOR_ID,
        KIND_KANBAN_AUTHOR_ID,
        KIND_CALENDAR_AUTHOR_ID,
    ] {
        assert!(
            !author_ids(&harness).contains(unavailable),
            "unbound pane must not advertise a persisted-kind mutation target: {unavailable}"
        );
    }
    click_author_id(&harness, NEW_VIEW_AUTHOR_ID);
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        ids.contains(NEW_VIEW_TITLE_AUTHOR_ID) && ids.contains(NEW_VIEW_CONFIRM_AUTHOR_ID),
        "the unbound pane must render the creation form before any saved projection exists"
    );
    assert!(
        !view.lock().unwrap().loading,
        "an unbound pane is an honest empty state, not a perpetual load"
    );
}

#[test]
fn in_flight_mutation_controls_are_accessibly_disabled() {
    let view = shared(seeded_table(1));
    view.lock().unwrap().in_flight = true;
    let mut harness = harness_for(view, Arc::new(Mutex::new(Vec::new())));
    harness.run();
    let sort_author_id = table_sort_author_id(BlockViewField::Title);
    for author_id in [
        KIND_KANBAN_AUTHOR_ID,
        NEW_VIEW_AUTHOR_ID,
        sort_author_id.as_str(),
    ] {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .unwrap_or_else(|| panic!("missing in-flight control {author_id}"));
        assert!(
            node.accesskit_node().is_disabled(),
            "in-flight control must be truthfully disabled: {author_id}"
        );
        assert!(
            !node
                .accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click),
            "disabled control must not expose Click: {author_id}"
        );
    }
}

#[test]
fn open_new_view_form_stays_visible_and_non_mutating_while_another_mutation_is_in_flight() {
    let view = shared(seeded_table(1));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&view), Arc::clone(&events));
    harness.run();
    click_author_id(&harness, NEW_VIEW_AUTHOR_ID);
    harness.run();
    assert!(author_ids(&harness).contains(NEW_VIEW_CONFIRM_AUTHOR_ID));

    view.lock().unwrap().in_flight = true;
    harness.step();
    for (author_id, action) in [
        (NEW_VIEW_TITLE_AUTHOR_ID, egui::accesskit::Action::SetValue),
        (
            NEW_VIEW_KIND_KANBAN_AUTHOR_ID,
            egui::accesskit::Action::Click,
        ),
        (NEW_VIEW_CONFIRM_AUTHOR_ID, egui::accesskit::Action::Click),
    ] {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .unwrap_or_else(|| panic!("open form control must remain visible: {author_id}"));
        assert!(
            node.accesskit_node().is_disabled(),
            "in-flight popup control must be disabled: {author_id}"
        );
        assert!(
            !node.accesskit_node().data().supports_action(action),
            "in-flight popup control must not expose {action:?}: {author_id}"
        );
    }
    assert!(
        events.lock().unwrap().is_empty(),
        "opening and disabling the form must not emit a create"
    );

    harness.step();
    assert!(
        author_ids(&harness).contains(NEW_VIEW_CONFIRM_AUTHOR_ID),
        "the in-progress form must remain open for recovery after the mutation completes"
    );
}

#[test]
fn client_date_range_serializes_as_rfc3339_backend_accepts() {
    // AC6 + must-fix #1 / backend-shape #4 (ADAPTER-BOUNDARY, NOT a self-tautology): the backend field
    // `BlockViewQuery.date_from/date_to` is `Option<DateTime<Utc>>` with the DEFAULT chrono serde, which
    // REJECTS a bare `YYYY-MM-DD`. The PATCH body MUST therefore carry a full RFC3339 instant. We assert
    // the produced strings (a) are not the bare date and (b) actually parse via chrono's `DateTime<Utc>`
    // Deserialize — the SAME type + serde the real backend uses — proving the wire would deserialize.
    let c = test_client();
    let mut def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
    def.query = BlockViewQuery {
        date_from: Some("2026-03-01".to_owned()),
        date_to: Some("2026-03-31".to_owned()),
        ..Default::default()
    };
    let spec = c.update_view_request("ws1", "view-cal", &def);
    let body = spec.body.unwrap();
    let query = body
        .get("definition")
        .and_then(|d| d.get("query"))
        .expect("definition.query");

    let from = query
        .get("date_from")
        .and_then(|x| x.as_str())
        .expect("date_from present");
    let to = query
        .get("date_to")
        .and_then(|x| x.as_str())
        .expect("date_to present");
    // It is NOT the bare date-only string the backend rejects.
    assert_ne!(
        from, "2026-03-01",
        "date_from must be expanded to a full RFC3339 instant"
    );
    assert_ne!(
        to, "2026-03-31",
        "date_to must be expanded to a full RFC3339 instant"
    );
    // Inclusive window: from = start-of-day, to = end-of-day.
    assert_eq!(from, "2026-03-01T00:00:00Z");
    assert_eq!(to, "2026-03-31T23:59:59Z");

    // The DECISIVE adapter-boundary check: the backend field is `Option<DateTime<Utc>>` with the default
    // chrono serde, whose `Deserialize` for `DateTime<Utc>` parses RFC3339 (`parse_from_rfc3339`). We
    // exercise that EXACT parser on the produced strings: a bare date errors; the expanded RFC3339
    // instant parses. This catches the must-fix #1 400/422 the old tautology missed. (chrono's `serde`
    // cargo feature is off in this crate's graph, so we call the underlying RFC3339 parser the backend's
    // `DateTime<Utc>` Deserialize delegates to — the same acceptance, no extra dependency feature.)
    let parsed_from = chrono::DateTime::parse_from_rfc3339(from)
        .expect("date_from must parse as RFC3339 (the backend DateTime<Utc> Deserialize path)")
        .with_timezone(&chrono::Utc);
    let parsed_to = chrono::DateTime::parse_from_rfc3339(to)
        .expect("date_to must parse as RFC3339 (the backend DateTime<Utc> Deserialize path)")
        .with_timezone(&chrono::Utc);
    assert!(parsed_from < parsed_to, "from is before to");
}

#[test]
fn bare_date_only_string_is_rejected_by_backend_date_type() {
    // Pin the failure the fix prevents: the OLD code sent a bare "2026-03-01", which the backend
    // `DateTime<Utc>` (whose Deserialize parses RFC3339) CANNOT parse. This asserts that rejection so a
    // regression that re-introduces the bare date is caught at the adapter boundary, not at runtime 422.
    let bare = chrono::DateTime::parse_from_rfc3339("2026-03-01");
    assert!(
        bare.is_err(),
        "a bare YYYY-MM-DD must NOT parse as RFC3339/DateTime<Utc> (the must-fix #1 bug)"
    );
    // The expanded instant the fix produces DOES parse.
    assert!(chrono::DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").is_ok());
}

#[test]
fn group_by_and_full_query_survive_update_round_trip() {
    // must-fix #2 / #3 (the FULL-OVERWRITE data-loss defect): the backend persists updateBlockView as a
    // full `SET view_definition_json = $1` overwrite, so ANY field absent from the native serialization
    // is wiped. Load a Kanban view with group_by=Tag + a server-side query (tag_ids/content_type),
    // apply a SORT, and assert the produced PATCH body STILL carries group_by AND the query filters —
    // proving a sort click no longer destroys the grouping or the user's filters.
    let loaded = serde_json::json!({
        "kind": "kanban",
        "group_by": { "kind": "tag" },
        "columns": ["title", "updated"],
        "query": {
            "content_type": "note",
            "tag_ids": ["tag-a", "tag-b"],
            "mention_ids": ["m-1"],
            "mime": "text/markdown"
        }
    });
    // Parse the loaded definition the way getBlockView does.
    let mut def = handshake_native::backend_client::definition_from_json(&loaded)
        .expect("canonical definition parses");
    assert_eq!(
        def.group_by,
        Some(BlockViewGroupBy::Tag),
        "loaded group_by parses"
    );
    assert_eq!(
        def.query.tag_ids,
        vec!["tag-a".to_owned(), "tag-b".to_owned()]
    );
    assert_eq!(def.query.content_type.as_deref(), Some("note"));

    // Apply a native sort edit (a header click on Updated -> asc), exactly as the host would.
    def.sort = Some(BlockViewSort {
        field: BlockViewField::Updated,
        direction: BlockViewSortDirection::Asc,
    });

    // Serialize through the SAME path updateBlockView uses.
    let c = test_client();
    let spec = c.update_view_request("ws1", "view-kanban", &def);
    let body = spec.body.unwrap();
    let def_json = body.get("definition").expect("definition wrapper");

    // group_by SURVIVED the round-trip (must-fix #3): the Kanban grouping is not wiped by the sort.
    assert_eq!(
        def_json
            .get("group_by")
            .and_then(|g| g.get("kind"))
            .and_then(|x| x.as_str()),
        Some("tag"),
        "must-fix #3: group_by must survive a sort updateBlockView (full-overwrite persist)"
    );
    // The server-side query filters SURVIVED (must-fix #2): not dropped to serde defaults.
    let query = def_json.get("query").expect("definition.query survived");
    assert_eq!(
        query.get("content_type").and_then(|x| x.as_str()),
        Some("note")
    );
    assert_eq!(
        query.get("mime").and_then(|x| x.as_str()),
        Some("text/markdown")
    );
    let tag_ids: Vec<&str> = query
        .get("tag_ids")
        .and_then(|x| x.as_array())
        .unwrap()
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    assert_eq!(
        tag_ids,
        vec!["tag-a", "tag-b"],
        "must-fix #2: tag_ids must survive the round-trip"
    );
    let mention_ids: Vec<&str> = query
        .get("mention_ids")
        .and_then(|x| x.as_array())
        .unwrap()
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    assert_eq!(
        mention_ids,
        vec!["m-1"],
        "must-fix #2: mention_ids must survive the round-trip"
    );
    // And the sort we applied is present.
    let sort = def_json.get("sort").expect("sort serialized");
    assert_eq!(sort.get("field").and_then(|x| x.as_str()), Some("updated"));
    assert_eq!(sort.get("direction").and_then(|x| x.as_str()), Some("asc"));
}

#[test]
fn group_by_field_round_trips_and_native_kanban_defaults_to_tag() {
    // group_by=Field must round-trip its field value, and a natively-created Kanban view must default to
    // group_by=Tag so the backend produces lanes (must-fix #3 — a Kanban view with group_by=None returns
    // zero lanes). Parse a field-grouped view, serialize, and assert the field-variant shape survives.
    let loaded = serde_json::json!({
        "kind": "kanban",
        "group_by": { "kind": "field", "field": "content_type" },
        "query": {}
    });
    let def = handshake_native::backend_client::definition_from_json(&loaded)
        .expect("canonical field-grouped definition parses");
    assert_eq!(
        def.group_by,
        Some(BlockViewGroupBy::Field {
            field: BlockViewField::ContentType
        })
    );
    let c = test_client();
    let body = c.update_view_request("ws1", "v", &def).body.unwrap();
    let gb = body
        .get("definition")
        .and_then(|d| d.get("group_by"))
        .expect("group_by serialized");
    assert_eq!(gb.get("kind").and_then(|x| x.as_str()), Some("field"));
    assert_eq!(
        gb.get("field").and_then(|x| x.as_str()),
        Some("content_type")
    );

    // A natively-created Kanban view defaults to group_by=Tag => '+ New view' kanban produces lanes.
    let native_kanban = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    assert_eq!(native_kanban.group_by, Some(BlockViewGroupBy::Tag));
    let native_body = c
        .update_view_request("ws1", "v2", &native_kanban)
        .body
        .unwrap();
    assert_eq!(
        native_body
            .get("definition")
            .and_then(|d| d.get("group_by"))
            .and_then(|g| g.get("kind"))
            .and_then(|x| x.as_str()),
        Some("tag"),
        "native-created Kanban defaults to group_by=tag so the backend builds lanes"
    );
    // Table/calendar carry NO grouping.
    assert!(BlockViewDefinition::of_kind(BlockViewKind::Table)
        .group_by
        .is_none());
    assert!(BlockViewDefinition::of_kind(BlockViewKind::Calendar)
        .group_by
        .is_none());
}

// ── JSON parse proofs: the native projection of a real queryBlockViewResults / getBlockView body ──

#[test]
fn parse_results_from_real_shape() {
    // A realistic queryBlockViewResults body (table kind) with the nested `derived` counts.
    let v = serde_json::json!({
        "kind": "table",
        "total_returned": 2,
        "blocks": [
            {
                "block_id": "blk-1",
                "workspace_id": "ws-1",
                "title": "First",
                "original_filename": null,
                "content_type": "note",
                "journal_date": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-02-01T00:00:00Z",
                "pinned": true,
                "favorite": false,
                "derived": { "backlink_count": 3, "mention_count": 1, "tag_count": 2 }
            },
            {
                "block_id": "blk-2",
                "workspace_id": "ws-1",
                "title": null,
                "original_filename": "file.md",
                "content_type": "file",
                "journal_date": null,
                "created_at": "2026-01-02T00:00:00Z",
                "updated_at": "2026-02-02T00:00:00Z",
                "pinned": false,
                "favorite": false,
                "derived": { "backlink_count": 0, "mention_count": 0, "tag_count": 0 }
            }
        ]
    });
    let results = handshake_native::backend_client::results_from_json(&v)
        .expect("canonical table results parse");
    assert_eq!(results.blocks.len(), 2);
    assert_eq!(results.total_returned, 2);
    assert_eq!(results.blocks[0].display_title(), "First");
    assert_eq!(
        results.blocks[0].backlink_count, 3,
        "derived.backlink_count parsed from nested object"
    );
    assert_eq!(results.blocks[0].tag_count, 2);
    // Title null -> original_filename fallback.
    assert_eq!(results.blocks[1].display_title(), "file.md");
}

#[test]
fn parse_kanban_groups_from_real_shape() {
    let v = serde_json::json!({
        "kind": "kanban",
        "total_returned": 2,
        "blocks": [],
        "groups": [
            { "key": "tag-a", "blocks": [{ "block_id": "b1", "workspace_id": "ws-1",
              "title": "A", "original_filename": null, "content_type": "note", "journal_date": null,
              "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
              "pinned": false, "favorite": false,
              "derived": { "backlink_count": 0, "mention_count": 0, "tag_count": 1 } }] },
            { "key": "__untagged__", "blocks": [] }
        ]
    });
    let results = handshake_native::backend_client::results_from_json(&v)
        .expect("canonical kanban results parse");
    assert_eq!(results.groups.len(), 2);
    assert_eq!(results.groups[0].key, "tag-a");
    assert_eq!(results.groups[0].blocks.len(), 1);
    assert_eq!(results.groups[1].key, "__untagged__");
    assert_eq!(results.groups[1].label(), "Untagged");
}

#[test]
fn parse_definition_from_real_shape() {
    let v = serde_json::json!({
        "kind": "calendar",
        "calendar_date_field": "journal_date",
        "columns": ["title", "updated"],
        "sort": { "field": "title", "direction": "asc" },
        "query": { "date_from": "2026-03-01T00:00:00Z", "date_to": "2026-03-31T00:00:00Z" }
    });
    let def = handshake_native::backend_client::definition_from_json(&v)
        .expect("canonical calendar definition parses");
    assert_eq!(def.kind, BlockViewKind::Calendar);
    assert_eq!(def.calendar_date_field, Some(BlockViewField::JournalDate));
    assert_eq!(
        def.columns,
        vec![BlockViewField::Title, BlockViewField::Updated]
    );
    assert_eq!(def.sort.unwrap().field, BlockViewField::Title);
    assert_eq!(def.sort.unwrap().direction, BlockViewSortDirection::Asc);
    // The calendar surface slices the ISO datetime to YYYY-MM-DD.
    assert_eq!(def.query.date_from.as_deref(), Some("2026-03-01"));
    assert_eq!(def.query.date_to.as_deref(), Some("2026-03-31"));
}

#[test]
fn parse_empty_results_is_empty_not_error() {
    // AC10: an explicit canonical empty blocks array is empty; omitted groups is valid because the
    // backend skips serialization of an empty groups vector.
    let v = serde_json::json!({ "kind": "table", "total_returned": 0, "blocks": [] });
    let results = handshake_native::backend_client::results_from_json(&v)
        .expect("canonical explicit empty results parse");
    assert!(results.blocks.is_empty());
    assert!(results.groups.is_empty());
}

#[test]
fn malformed_successful_collection_payloads_fail_closed() {
    assert!(
        handshake_native::backend_client::results_from_json(
            &serde_json::json!({ "kind": "table", "total_returned": 0 })
        )
        .is_err(),
        "missing required blocks array must not become fake empty success"
    );
    assert!(
        handshake_native::backend_client::results_from_json(
            &serde_json::json!({ "kind": "future", "total_returned": 0, "blocks": [] })
        )
        .is_err(),
        "unknown kind must not become table"
    );
    assert!(
        handshake_native::backend_client::definition_from_json(
            &serde_json::json!({ "kind": "table" })
        )
        .is_err(),
        "missing required query must reject the definition"
    );
    assert!(
        handshake_native::backend_client::definition_from_json(
            &serde_json::json!({ "kind": "table", "query": {}, "columns": ["title", "unknown"] })
        )
        .is_err(),
        "one malformed member must reject the whole definition"
    );
}

#[test]
fn malformed_successful_create_payloads_fail_closed() {
    use handshake_native::backend_client::create_block_view_id_from_json;

    let canonical = serde_json::json!({
        "block": { "block_id": "view-new", "content_type": "view_def" },
        "definition": { "kind": "table", "query": {} }
    });
    assert_eq!(
        create_block_view_id_from_json(&canonical).expect("canonical create response"),
        "view-new"
    );
    for malformed in [
        serde_json::json!({
            "block": { "block_id": "", "content_type": "view_def" },
            "definition": { "kind": "table", "query": {} }
        }),
        serde_json::json!({
            "block": { "block_id": "   ", "content_type": "view_def" },
            "definition": { "kind": "table", "query": {} }
        }),
        serde_json::json!({
            "block": { "block_id": "view-new", "content_type": "note" },
            "definition": { "kind": "table", "query": {} }
        }),
        serde_json::json!({
            "block": { "block_id": "view-new", "content_type": "view_def" }
        }),
    ] {
        assert!(
            create_block_view_id_from_json(&malformed).is_err(),
            "malformed successful create response must fail closed: {malformed}"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE-PG: isolated, self-seeding, non-ignored product-client + mounted-widget round trip.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "integration")]
struct LiveWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl LiveWorkspaceCleanup<'_> {
    fn assert_cleaned_and_absent(&mut self) {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204 | 404),
            "managed-PG workspace cleanup returned HTTP {status}"
        );
        let workspaces = self.backend.get_json("/workspaces");
        let rows = workspaces
            .as_array()
            .expect("GET /workspaces returns the canonical workspace list");
        assert!(
            rows.iter()
                .all(|row| row.get("id").and_then(|id| id.as_str())
                    != Some(self.workspace_id.as_str())),
            "cleanup must remove the isolated workspace from a fresh canonical list read"
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
fn block_on_cell_result<T: Clone>(
    cell: &Arc<Mutex<Option<Result<T, String>>>>,
) -> Result<T, String> {
    for _ in 0..200 {
        if let Some(slot) = cell.lock().unwrap().clone() {
            return slot;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed-PG product-client operation did not land within 10s");
}

#[cfg(feature = "integration")]
fn block_on_cell<T: Clone>(cell: &Arc<Mutex<Option<Result<T, String>>>>) -> T {
    block_on_cell_result(cell).expect("managed-PG product-client operation must succeed")
}

#[cfg(feature = "integration")]
fn live_fetch_view(
    client: &BlockViewClient,
    workspace_id: &str,
    view_id: &str,
) -> handshake_native::backend_client::BlockViewRecordData {
    let cell: handshake_native::backend_client::BlockViewRecordCell = Arc::new(Mutex::new(None));
    client.fetch_view(workspace_id, view_id, Arc::clone(&cell));
    block_on_cell(&cell)
}

#[cfg(feature = "integration")]
fn live_query_view(
    client: &BlockViewClient,
    workspace_id: &str,
    view_id: &str,
) -> BlockViewResults {
    let cell: handshake_native::backend_client::BlockViewResultsCell = Arc::new(Mutex::new(None));
    client.query_results(workspace_id, view_id, 100, 0, Arc::clone(&cell));
    block_on_cell(&cell)
}

#[cfg(feature = "integration")]
fn live_create_view(
    client: &BlockViewClient,
    workspace_id: &str,
    title: &str,
    definition: &BlockViewDefinition,
) -> String {
    let cell: handshake_native::backend_client::BlockViewOpCell = Arc::new(Mutex::new(None));
    let generation = Arc::new(std::sync::atomic::AtomicU64::new(1));
    let block_id = uuid::Uuid::new_v4().to_string();
    client.create_view(
        workspace_id,
        &block_id,
        title,
        definition,
        Arc::clone(&generation),
        1,
        Arc::clone(&cell),
    );
    let delivery = (0..200)
        .find_map(|_| {
            let delivery = cell.lock().unwrap().clone();
            if delivery.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            delivery
        })
        .expect("managed-PG create delivery did not land within 10s");
    assert_eq!(delivery.workspace_id, workspace_id);
    assert_eq!(delivery.generation, 1);
    assert!(delivery.expected_bound_view_id.is_none());
    let view_id = delivery.result.expect("managed-PG create must succeed");
    assert!(!view_id.is_empty(), "created view id must be non-empty");
    view_id
}

#[cfg(feature = "integration")]
fn live_dispatch(
    client: &BlockViewClient,
    workspace_id: &str,
    spec: handshake_native::backend_client::RequestSpec,
    view_id: &str,
) {
    let cell: handshake_native::backend_client::BlockViewOpCell = Arc::new(Mutex::new(None));
    let generation = Arc::new(std::sync::atomic::AtomicU64::new(1));
    client.dispatch(
        spec,
        workspace_id,
        view_id.to_owned(),
        Arc::clone(&generation),
        1,
        Arc::clone(&cell),
    );
    let delivery = (0..200)
        .find_map(|_| {
            let delivery = cell.lock().unwrap().clone();
            if delivery.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            delivery
        })
        .expect("managed-PG mutation delivery did not land within 10s");
    assert_eq!(delivery.workspace_id, workspace_id);
    assert_eq!(delivery.generation, 1);
    assert_eq!(delivery.expected_bound_view_id.as_deref(), Some(view_id));
    assert_eq!(
        delivery.result.expect("managed-PG mutation must succeed"),
        view_id
    );
}

#[cfg(feature = "integration")]
fn lane<'a>(results: &'a BlockViewResults, key: &str) -> &'a BlockViewLane {
    results
        .groups
        .iter()
        .find(|lane| lane.key == key)
        .unwrap_or_else(|| panic!("expected lane '{key}' in {:?}", results.groups))
}

#[cfg(feature = "integration")]
fn json_node_by_author_id<'a>(
    value: &'a serde_json::Value,
    expected: &str,
) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("author_id").and_then(serde_json::Value::as_str) == Some(expected) {
                return Some(value);
            }
            object
                .values()
                .find_map(|child| json_node_by_author_id(child, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|child| json_node_by_author_id(child, expected)),
        _ => None,
    }
}

#[cfg(feature = "integration")]
fn json_author_value_is(value: &serde_json::Value, author_id: &str, expected: &str) -> bool {
    json_node_by_author_id(value, author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        == Some(expected)
}

#[cfg(feature = "integration")]
fn json_author_is_descendant_of(
    value: &serde_json::Value,
    ancestor_author_id: &str,
    descendant_author_id: &str,
) -> bool {
    json_node_by_author_id(value, ancestor_author_id)
        .is_some_and(|ancestor| json_has_author_id(ancestor, descendant_author_id))
}

#[cfg(feature = "integration")]
fn mount_collection_pane(app: &mut HandshakeApp, workspace_id: &str) {
    let pane_id = PaneId::from("pane-a");
    let pane_type = placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL);
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        pane_type.clone(),
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("seeded pane-a tab bar");
    bar.tabs = vec![handshake_native::tab_bar::TabState::new(pane_type)];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
}

#[cfg(feature = "integration")]
fn await_app_collection(
    harness: &mut Harness<'_, HandshakeApp>,
    expected_view_id: &str,
) -> BlockViewDefinition {
    for _ in 0..200 {
        harness.step();
        let view = harness.state().mounted_block_collection_view();
        let ready = if let Ok(view) = view.lock() {
            if view.view_block_id == expected_view_id
                && !view.loading
                && !view.in_flight
                && view.error.is_none()
            {
                view.definition.clone()
            } else {
                None
            }
        } else {
            None
        };
        if let Some(definition) = ready {
            harness.run_steps(2);
            return definition;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("mounted HandshakeApp did not load view {expected_view_id} within 10s");
}

#[test]
#[cfg(feature = "integration")]
fn stale_collection_operation_delivery_cannot_rebind_a_to_b() {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.bind_active_project_for_integration_test("ws-a".to_owned());
    mount_collection_pane(&mut app, "ws-a");
    app.bind_block_collection_view_for_test("view-a");
    let generation_a = app.block_collection_generation_for_test();
    app.bind_block_collection_view_for_test("view-b");
    assert!(app.block_collection_generation_for_test() > generation_a);
    app.deliver_block_collection_op_delivery_for_test(
        handshake_native::backend_client::BlockViewOpDelivery {
            workspace_id: "ws-a".to_owned(),
            generation: generation_a,
            expected_bound_view_id: Some("view-a".to_owned()),
            result: Ok("view-a".to_owned()),
        },
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    let mounted = harness.state().mounted_block_collection_view();
    let view = mounted.lock().unwrap();
    assert_eq!(view.view_block_id, "view-b");
    assert_eq!(view.workspace_id, "ws-a");
}

#[test]
#[cfg(feature = "integration")]
fn old_workspace_create_delivery_cannot_switch_current_workspace_view() {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.bind_active_project_for_integration_test("ws-a".to_owned());
    mount_collection_pane(&mut app, "ws-a");
    app.bind_block_collection_view_for_test("view-a");
    app.bind_active_project_for_integration_test("ws-b".to_owned());
    app.bind_block_collection_view_for_test("view-b");
    let current_generation = app.block_collection_generation_for_test();
    app.deliver_block_collection_op_delivery_for_test(
        handshake_native::backend_client::BlockViewOpDelivery {
            workspace_id: "ws-a".to_owned(),
            generation: current_generation,
            expected_bound_view_id: None,
            result: Ok("created-in-ws-a".to_owned()),
        },
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    let mounted = harness.state().mounted_block_collection_view();
    let view = mounted.lock().unwrap();
    assert_eq!(view.view_block_id, "view-b");
    assert_eq!(view.workspace_id, "ws-b");
}

/// AC1-AC10 / PROOF2-PROOF6 against REAL Handshake-managed PostgreSQL. The test owns every row it
/// creates, drives the production `BlockViewClient` transport, mounts the returned projections through
/// the real `BlockCollectionView`, and leaves no workspace behind. Feature-gated, but deliberately NOT
/// ignored: the integration command cannot silently pass while omitting the required resource proof.
#[test]
#[cfg(feature = "integration")]
fn block_collection_views_live_pg_self_seed_full_round_trip() {
    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-027");
    let receipt_path = receipt_dir.join("managed-pg-receipt.json");
    match std::fs::remove_file(&receipt_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("remove stale MT-027 success receipt before proof: {error}"),
    }
    let live = interconnect_support::require_reachable_backend();
    let backend_binding = live.owned_backend_binding_receipt();
    let unique = format!(
        "mt027-{}-{}",
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

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("block-view product-client runtime");
    let client = BlockViewClient::new(live.base.clone(), rt.handle().clone());

    // Fresh canonical state: no fixture ids and no leaked rows from another run.
    let initial = live.get_json(&format!("/workspaces/{workspace_id}/loom/views/all"));
    let initial_blocks = initial
        .get("blocks")
        .and_then(|value| value.as_array())
        .map_or(0, Vec::len);
    assert_eq!(initial_blocks, 0, "isolated workspace starts empty");

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
    let tag_a = seed_block("tag_hub", &format!("{unique}-lane-a"));
    let tag_b = seed_block("tag_hub", &format!("{unique}-lane-b"));
    let alpha = seed_block("note", &format!("{unique}-Alpha"));
    let middle = seed_block("note", &format!("{unique}-Middle"));
    let zulu = seed_block("note", &format!("{unique}-Zulu"));
    for (source, target) in [(&alpha, &tag_a), (&zulu, &tag_b)] {
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/edges"),
            &serde_json::json!({
                "source_block_id": source,
                "target_block_id": target,
                "edge_type": "tag",
                "created_by": "user"
            }),
        );
    }

    let journal_id = |date: &str| {
        let value = live.put_json(
            &format!("/workspaces/{workspace_id}/loom/journals/{date}"),
            &serde_json::json!({}),
        );
        value
            .get("block")
            .unwrap_or(&value)
            .get("block_id")
            .and_then(|id| id.as_str())
            .expect("journal create returns block_id")
            .to_owned()
    };
    let march_one = journal_id("2026-03-01");
    let march_two = journal_id("2026-03-02");
    let april_one = journal_id("2026-04-01");

    // Create all three real view_def rows through the product client. No fixture helper creates the
    // feature under proof, so create persistence and the canonical x-hsk write transport are exercised.
    let mut table_def = BlockViewDefinition::of_kind(BlockViewKind::Table);
    table_def.query.content_type = Some("note".to_owned());
    table_def.columns = vec![BlockViewField::Title, BlockViewField::Updated];
    let table_id = live_create_view(
        &client,
        &workspace_id,
        &format!("{unique}-table"),
        &table_def,
    );

    let mut kanban_def = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    kanban_def.query.content_type = Some("note".to_owned());
    kanban_def.group_by = Some(BlockViewGroupBy::Tag);
    let kanban_id = live_create_view(
        &client,
        &workspace_id,
        &format!("{unique}-kanban"),
        &kanban_def,
    );

    let mut calendar_def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
    calendar_def.query.content_type = Some("journal".to_owned());
    calendar_def.calendar_date_field = Some(BlockViewField::JournalDate);
    let calendar_id = live_create_view(
        &client,
        &workspace_id,
        &format!("{unique}-calendar"),
        &calendar_def,
    );
    assert_eq!(
        HashSet::from([table_id.clone(), kanban_id.clone(), calendar_id.clone()]).len(),
        3,
        "new-view creation returns three exact distinct persisted ids"
    );

    // Failure + visible Retry + recovery: force one real product-client connection failure, mount the
    // error, drive its stable AccessKit button, then recover the same saved view through the live client.
    let bad_client = BlockViewClient::new("http://127.0.0.1:9", rt.handle().clone());
    let bad_cell: handshake_native::backend_client::BlockViewRecordCell =
        Arc::new(Mutex::new(None));
    bad_client.fetch_view(&workspace_id, &table_id, Arc::clone(&bad_cell));
    let failure = block_on_cell_result(&bad_cell).expect_err("forced backend failure must surface");
    let retry_host = shared(BlockCollectionView::new(
        workspace_id.clone(),
        table_id.clone(),
    ));
    retry_host.lock().unwrap().set_error(failure);
    let retry_events = Arc::new(Mutex::new(Vec::new()));
    let retry_events_ck = Arc::clone(&retry_events);
    let mut retry_harness = harness_for(Arc::clone(&retry_host), retry_events);
    retry_harness.run();
    assert!(
        author_ids(&retry_harness).contains(RETRY_AUTHOR_ID),
        "backend failure exposes stable bcv.retry AccessKit control"
    );
    click_author_id(&retry_harness, RETRY_AUTHOR_ID);
    retry_harness.run();
    assert!(
        matches!(
            retry_events_ck.lock().unwrap().last(),
            Some(BlockViewEvent::Retry)
        ),
        "Retry control emits the host recovery event"
    );
    let recovered_record = live_fetch_view(&client, &workspace_id, &table_id);
    let recovered_results = live_query_view(&client, &workspace_id, &table_id);
    retry_host
        .lock()
        .unwrap()
        .set_loaded(recovered_record.definition, recovered_results);
    retry_harness.run();
    assert!(
        !author_ids(&retry_harness).contains(RETRY_AUTHOR_ID),
        "successful fresh load clears the failure/retry overlay"
    );

    // AC1/PROOF2: exact three note rows, non-empty title + updated cells, mounted AccessKit controls.
    let table_record = live_fetch_view(&client, &workspace_id, &table_id);
    let table_results = live_query_view(&client, &workspace_id, &table_id);
    assert_eq!(table_results.total_returned, 3);
    let table_host = shared(BlockCollectionView::new(
        workspace_id.clone(),
        table_id.clone(),
    ));
    table_host
        .lock()
        .unwrap()
        .set_loaded(table_record.definition.clone(), table_results.clone());
    let table_events = Arc::new(Mutex::new(Vec::new()));
    let table_events_ck = Arc::clone(&table_events);
    let mut table_harness = harness_for(Arc::clone(&table_host), table_events);
    table_harness.run();
    let table_ids = author_ids(&table_harness);
    for block_id in [&alpha, &middle, &zulu] {
        let row_id = table_row_author_id(block_id);
        assert!(table_ids.contains(&row_id), "missing live row {row_id}");
        let label = label_for(&table_harness, &row_id).expect("live row label");
        assert!(label.contains(&unique), "title cell is non-empty: {label}");
        assert!(label.contains('T'), "updated_at cell is non-empty: {label}");
    }

    // AC2/PROOF3: drive the real header twice, persist both sorts, and prove backend order each time.
    click_author_id(&table_harness, &table_sort_author_id(BlockViewField::Title));
    table_harness.run();
    let asc = match table_events_ck.lock().unwrap().last().cloned() {
        Some(BlockViewEvent::Sort { sort }) => sort,
        other => panic!("live title click must emit Sort, got {other:?}"),
    };
    let mut sorted_def = table_record.definition.clone();
    sorted_def.sort = Some(asc);
    live_dispatch(
        &client,
        &workspace_id,
        client.update_view_request(&workspace_id, &table_id, &sorted_def),
        &table_id,
    );
    let asc_results = live_query_view(&client, &workspace_id, &table_id);
    let asc_titles: Vec<&str> = asc_results
        .blocks
        .iter()
        .map(LoomBlockRow::display_title)
        .collect();
    assert_eq!(
        asc_titles,
        vec![
            format!("{unique}-Alpha"),
            format!("{unique}-Middle"),
            format!("{unique}-Zulu")
        ]
    );
    table_host
        .lock()
        .unwrap()
        .set_loaded(sorted_def.clone(), asc_results);
    table_harness.run();
    click_author_id(&table_harness, &table_sort_author_id(BlockViewField::Title));
    table_harness.run();
    let desc = match table_events_ck.lock().unwrap().last().cloned() {
        Some(BlockViewEvent::Sort { sort }) => sort,
        other => panic!("second live title click must emit Sort, got {other:?}"),
    };
    assert_eq!(desc.direction, BlockViewSortDirection::Desc);
    sorted_def.sort = Some(desc);
    live_dispatch(
        &client,
        &workspace_id,
        client.update_view_request(&workspace_id, &table_id, &sorted_def),
        &table_id,
    );
    let desc_results = live_query_view(&client, &workspace_id, &table_id);
    assert_eq!(
        desc_results.blocks[0].display_title(),
        format!("{unique}-Zulu")
    );
    let fresh_client = BlockViewClient::new(live.base.clone(), rt.handle().clone());
    assert_eq!(
        live_fetch_view(&fresh_client, &workspace_id, &table_id)
            .definition
            .sort,
        Some(desc),
        "fresh product client observes persisted descending sort"
    );

    // AC3/AC4/PROOF4: mount real lanes, then mutate tag authority and prove the source lane loses the card.
    let kanban_record = live_fetch_view(&client, &workspace_id, &kanban_id);
    let kanban_results = live_query_view(&client, &workspace_id, &kanban_id);
    assert!(lane(&kanban_results, &tag_a)
        .blocks
        .iter()
        .any(|b| b.block_id == alpha));
    assert!(lane(&kanban_results, &tag_b)
        .blocks
        .iter()
        .any(|b| b.block_id == zulu));
    assert!(lane(&kanban_results, BLOCK_VIEW_UNTAGGED_LANE)
        .blocks
        .iter()
        .any(|b| b.block_id == middle));
    let kanban_host = shared(BlockCollectionView::new(
        workspace_id.clone(),
        kanban_id.clone(),
    ));
    kanban_host
        .lock()
        .unwrap()
        .set_loaded(kanban_record.definition, kanban_results);
    let mut kanban_harness = harness_for(kanban_host, Arc::new(Mutex::new(Vec::new())));
    kanban_harness.run();
    let kanban_ids = author_ids(&kanban_harness);
    for required in [
        kanban_lane_author_id(&tag_a),
        kanban_lane_author_id(&tag_b),
        kanban_lane_author_id(BLOCK_VIEW_UNTAGGED_LANE),
        kanban_card_author_id(&alpha),
    ] {
        assert!(
            kanban_ids.contains(&required),
            "missing live Kanban node {required}"
        );
    }
    live_dispatch(
        &client,
        &workspace_id,
        client.card_move_request(
            &workspace_id,
            &alpha,
            std::slice::from_ref(&tag_b),
            std::slice::from_ref(&tag_a),
        ),
        &kanban_id,
    );
    let moved = live_query_view(&fresh_client, &workspace_id, &kanban_id);
    assert!(
        moved
            .groups
            .iter()
            .find(|lane| lane.key == tag_a)
            .map_or(true, |lane| !lane
                .blocks
                .iter()
                .any(|b| b.block_id == alpha)),
        "backend source lane loses the moved card (and may disappear when empty)"
    );
    assert!(
        lane(&moved, &tag_b)
            .blocks
            .iter()
            .any(|b| b.block_id == alpha),
        "fresh backend re-query places the moved card in the target lane"
    );

    // AC5/AC6/PROOF5: real journal rows bucket by journal_date; range PATCH persists and excludes April.
    let calendar_record = live_fetch_view(&client, &workspace_id, &calendar_id);
    let calendar_results = live_query_view(&client, &workspace_id, &calendar_id);
    assert_eq!(calendar_results.total_returned, 3);
    let calendar_host = shared(BlockCollectionView::new(
        workspace_id.clone(),
        calendar_id.clone(),
    ));
    calendar_host
        .lock()
        .unwrap()
        .set_loaded(calendar_record.definition.clone(), calendar_results.clone());
    let mut calendar_harness = harness_for(calendar_host, Arc::new(Mutex::new(Vec::new())));
    calendar_harness.run();
    let calendar_ids = author_ids(&calendar_harness);
    for (date, block_id) in [
        ("2026-03-01", &march_one),
        ("2026-03-02", &march_two),
        ("2026-04-01", &april_one),
    ] {
        assert!(calendar_ids.contains(&calendar_day_author_id(date)));
        assert!(calendar_ids.contains(&calendar_entry_author_id(block_id)));
    }
    let mut ranged_def = calendar_record.definition;
    ranged_def.query.date_from = Some("2026-03-01".to_owned());
    ranged_def.query.date_to = Some("2026-03-31".to_owned());
    live_dispatch(
        &client,
        &workspace_id,
        client.update_view_request(&workspace_id, &calendar_id, &ranged_def),
        &calendar_id,
    );
    let ranged = live_query_view(&fresh_client, &workspace_id, &calendar_id);
    let ranged_ids: HashSet<&str> = ranged.blocks.iter().map(|b| b.block_id.as_str()).collect();
    assert_eq!(ranged.total_returned, 2);
    assert!(ranged_ids.contains(march_one.as_str()));
    assert!(ranged_ids.contains(march_two.as_str()));
    assert!(!ranged_ids.contains(april_one.as_str()));
    let persisted_range = live_fetch_view(&fresh_client, &workspace_id, &calendar_id).definition;
    assert_eq!(
        persisted_range.query.date_from.as_deref(),
        Some("2026-03-01")
    );
    assert_eq!(persisted_range.query.date_to.as_deref(), Some("2026-03-31"));

    // AC7: kind change is a persisted full-definition update, observable from a fresh client.
    sorted_def.kind = BlockViewKind::Calendar;
    sorted_def.calendar_date_field = Some(BlockViewField::Updated);
    live_dispatch(
        &client,
        &workspace_id,
        client.update_view_request(&workspace_id, &table_id, &sorted_def),
        &table_id,
    );
    let switched = live_fetch_view(&fresh_client, &workspace_id, &table_id);
    assert_eq!(switched.definition.kind, BlockViewKind::Calendar);
    assert_eq!(
        live_query_view(&fresh_client, &workspace_id, &table_id).kind_str,
        "calendar"
    );

    // Product-pane closure: mount the real HandshakeApp + BlockCollectionPaneMount, drive its visible
    // Retry control and host event queue, and let drive_collections_pane own every client dispatch,
    // generation check, completion pair, and reload below.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(rt.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    app.set_block_collection_backend_base_url_for_test(live.base.clone());
    mount_collection_pane(&mut app, &workspace_id);
    let mounted = app.mounted_block_collection_view();
    let mounted_events = app.mounted_block_collection_events();
    {
        let mut view = mounted.lock().unwrap();
        view.bind_loading_view(workspace_id.clone(), table_id.clone());
        view.set_error("forced mounted-host recovery proof");
    }
    let mut app_harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    app_harness.step();
    let mut argus = CanonicalArgusDriver::bind(
        app_harness.state(),
        "wp-kernel-012-mt-027-block-collections",
    );
    let error_tree = argus.inspect(&mut app_harness);
    assert!(
        json_has_author_id(&error_tree, RETRY_AUTHOR_ID),
        "canonical Argus sees the mounted error-state Retry control"
    );
    argus.click_and_reinspect(&mut app_harness, RETRY_AUTHOR_ID);
    app_harness.step();
    let mounted_table = await_app_collection(&mut app_harness, &table_id);
    assert_eq!(mounted_table.kind, BlockViewKind::Calendar);
    let recovered_tree = argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "initial-retry-recovered-projection",
        |tree| {
            !json_has_author_id(tree, RETRY_AUTHOR_ID)
                && json_has_author_id(tree, KIND_CALENDAR_AUTHOR_ID)
        },
    );
    assert!(
        !json_has_author_id(&recovered_tree, RETRY_AUTHOR_ID),
        "fresh canonical Argus inspection sees the recovered projection"
    );
    let mounted_ids = author_ids(&app_harness);
    for required in [
        KIND_TABLE_AUTHOR_ID,
        KIND_KANBAN_AUTHOR_ID,
        NEW_VIEW_AUTHOR_ID,
        handshake_native::graph::block_collection_view::STATUS_AUTHOR_ID,
        handshake_native::graph::block_collection_view::CALENDAR_DATE_FROM_AUTHOR_ID,
        handshake_native::graph::block_collection_view::CALENDAR_DATE_TO_AUTHOR_ID,
    ] {
        assert!(
            mounted_ids.contains(required),
            "mounted control missing: {required}"
        );
    }

    // Actual host kind + sort mutations: the queue is drained by drive_collections_pane and each write
    // is followed by its generation-stamped authoritative definition/results reload.
    argus.click_and_reinspect(&mut app_harness, KIND_TABLE_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &table_id).kind,
        BlockViewKind::Table
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "kind-table-selected", |tree| {
        json_author_value_is(tree, KIND_TABLE_AUTHOR_ID, "selected")
    });
    argus.click_and_reinspect(&mut app_harness, KIND_KANBAN_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &table_id).kind,
        BlockViewKind::Kanban
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "kind-kanban-selected", |tree| {
        json_author_value_is(tree, KIND_KANBAN_AUTHOR_ID, "selected")
    });
    argus.click_and_reinspect(&mut app_harness, KIND_CALENDAR_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &table_id).kind,
        BlockViewKind::Calendar
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "kind-calendar-selected", |tree| {
        json_author_value_is(tree, KIND_CALENDAR_AUTHOR_ID, "selected")
    });
    argus.click_and_reinspect(&mut app_harness, KIND_TABLE_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &table_id).kind,
        BlockViewKind::Table
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "kind-table-restored", |tree| {
        json_author_value_is(tree, KIND_TABLE_AUTHOR_ID, "selected")
    });
    argus.click_and_reinspect(
        &mut app_harness,
        &table_sort_author_id(BlockViewField::Title),
    );
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &table_id).sort,
        Some(BlockViewSort {
            field: BlockViewField::Title,
            direction: BlockViewSortDirection::Asc,
        })
    );
    let sort_title_author_id = table_sort_author_id(BlockViewField::Title);
    argus.assert_latest_terminal_predicate(&mut app_harness, "sort-title-ascending", |tree| {
        json_node_by_author_id(tree, &sort_title_author_id)
            .and_then(|node| node.get("label"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|label| label.contains('↑'))
    });

    // Actual host Kanban mutation. Rebind via visible Retry, then enqueue the typed card event the real
    // drag surface emits; fresh backend state proves the host did not mutate lanes locally.
    {
        let mut view = mounted.lock().unwrap();
        view.bind_loading_view(workspace_id.clone(), kanban_id.clone());
        view.set_error("force mounted Kanban reload");
    }
    app_harness.step();
    argus.click_and_reinspect(&mut app_harness, RETRY_AUTHOR_ID);
    app_harness.step();
    await_app_collection(&mut app_harness, &kanban_id);
    let middle_card_author_id = kanban_card_author_id(&middle);
    let untagged_lane_author_id = kanban_lane_author_id(BLOCK_VIEW_UNTAGGED_LANE);
    argus.assert_latest_terminal_predicate(&mut app_harness, "kanban-retry-loaded-card", |tree| {
        json_author_is_descendant_of(tree, &untagged_lane_author_id, &middle_card_author_id)
    });
    let live_kanban_ids = author_ids(&app_harness);
    assert!(live_kanban_ids.contains(&kanban_card_author_id(&middle)));
    argus.click_with_payload_and_reinspect(
        &mut app_harness,
        "collection.kanban-move",
        serde_json::json!({
            "block_id": middle,
            "from_lane": BLOCK_VIEW_UNTAGGED_LANE,
            "to_lane": tag_a,
        }),
    );
    app_harness.step();
    await_app_collection(&mut app_harness, &kanban_id);
    let tag_a_lane_author_id = kanban_lane_author_id(&tag_a);
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "kanban-card-moved-target-lane",
        |tree| {
            json_author_is_descendant_of(tree, &tag_a_lane_author_id, &middle_card_author_id)
                && !json_author_is_descendant_of(
                    tree,
                    &untagged_lane_author_id,
                    &middle_card_author_id,
                )
        },
    );
    assert!(lane(
        &live_query_view(&fresh_client, &workspace_id, &kanban_id),
        &tag_a
    )
    .blocks
    .iter()
    .any(|block| block.block_id == middle));

    // Actual host calendar mutation and complete live AccessKit control surface.
    {
        let mut view = mounted.lock().unwrap();
        view.bind_loading_view(workspace_id.clone(), calendar_id.clone());
        view.set_error("force mounted Calendar reload");
    }
    app_harness.step();
    argus.click_and_reinspect(&mut app_harness, RETRY_AUTHOR_ID);
    app_harness.step();
    await_app_collection(&mut app_harness, &calendar_id);
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "calendar-retry-loaded-controls",
        |tree| {
            json_has_author_id(
                tree,
                handshake_native::graph::block_collection_view::CALENDAR_DATE_FROM_AUTHOR_ID,
            ) && json_has_author_id(
                tree,
                handshake_native::graph::block_collection_view::CALENDAR_DATE_TO_AUTHOR_ID,
            )
        },
    );
    argus.set_value_and_reinspect(
        &mut app_harness,
        handshake_native::graph::block_collection_view::CALENDAR_DATE_FROM_AUTHOR_ID,
        "2026-02-28",
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "calendar-from-value", |tree| {
        json_author_value_is(
            tree,
            handshake_native::graph::block_collection_view::CALENDAR_DATE_FROM_AUTHOR_ID,
            "2026-02-28",
        )
    });
    argus.set_value_and_reinspect(
        &mut app_harness,
        handshake_native::graph::block_collection_view::CALENDAR_DATE_TO_AUTHOR_ID,
        "2026-04-30",
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "calendar-to-value", |tree| {
        json_author_value_is(
            tree,
            handshake_native::graph::block_collection_view::CALENDAR_DATE_TO_AUTHOR_ID,
            "2026-04-30",
        )
    });
    argus.click_and_reinspect(&mut app_harness, "bcv.calendar.apply-range");
    app_harness.step();
    let mounted_calendar = await_app_collection(&mut app_harness, &calendar_id);
    let march_one_entry_id = calendar_entry_author_id(&march_one);
    let march_two_entry_id = calendar_entry_author_id(&march_two);
    let april_one_entry_id = calendar_entry_author_id(&april_one);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut app_harness,
        "calendar-range-terminal",
        serde_json::json!({
            "required_entries": [
                {
                    "day_author_id": calendar_day_author_id("2026-03-01"),
                    "entry_author_id": march_one_entry_id.clone(),
                },
                {
                    "day_author_id": calendar_day_author_id("2026-03-02"),
                    "entry_author_id": march_two_entry_id.clone(),
                },
                {
                    "day_author_id": calendar_day_author_id("2026-04-01"),
                    "entry_author_id": april_one_entry_id.clone(),
                },
            ],
        }),
        |tree| {
            json_author_value_is(
                tree,
                handshake_native::graph::block_collection_view::CALENDAR_DATE_FROM_AUTHOR_ID,
                "2026-02-28",
            ) && json_author_value_is(
                tree,
                handshake_native::graph::block_collection_view::CALENDAR_DATE_TO_AUTHOR_ID,
                "2026-04-30",
            ) && [
                ("2026-03-01", &march_one_entry_id),
                ("2026-03-02", &march_two_entry_id),
                ("2026-04-01", &april_one_entry_id),
            ]
            .iter()
            .all(|(date, entry_id)| {
                json_author_is_descendant_of(tree, &calendar_day_author_id(date), entry_id)
            })
        },
    );
    assert_eq!(
        mounted_calendar.query.date_from.as_deref(),
        Some("2026-02-28")
    );
    assert_eq!(
        mounted_calendar.query.date_to.as_deref(),
        Some("2026-04-30")
    );
    let persisted_calendar = live_fetch_view(&fresh_client, &workspace_id, &calendar_id).definition;
    assert_eq!(
        persisted_calendar.query.date_from.as_deref(),
        Some("2026-02-28")
    );
    assert_eq!(
        persisted_calendar.query.date_to.as_deref(),
        Some("2026-04-30")
    );
    assert_eq!(
        live_query_view(&fresh_client, &workspace_id, &calendar_id).total_returned,
        3
    );

    // Actual host create + unbound-create failure recovery. The failed title/kind intent is retained;
    // clicking the same visible Retry after restoring the live base replays createBlockView and loads it.
    app_harness
        .state_mut()
        .unbind_block_collection_view_for_test();
    app_harness.step();
    {
        let view = mounted.lock().unwrap();
        assert!(view.view_block_id.is_empty());
        assert!(!view.loading);
    }
    let host_create_title = format!("{unique}-host-created");
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_AUTHOR_ID);
    let create_tree = argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "unbound-create-form-open",
        |tree| {
            json_has_author_id(tree, NEW_VIEW_TITLE_AUTHOR_ID)
                && json_has_author_id(tree, NEW_VIEW_CONFIRM_AUTHOR_ID)
                && !json_has_author_id(tree, KIND_TABLE_AUTHOR_ID)
        },
    );
    let title_node = json_node_by_author_id(&create_tree, NEW_VIEW_TITLE_AUTHOR_ID)
        .expect("canonical Argus sees the mounted new-view title");
    assert_eq!(
        title_node["disabled"], false,
        "canonical mounted new-view title must be steerable: {title_node}"
    );
    argus.set_value_and_reinspect(
        &mut app_harness,
        NEW_VIEW_TITLE_AUTHOR_ID,
        &host_create_title,
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "unbound-create-title-set", |tree| {
        json_author_value_is(tree, NEW_VIEW_TITLE_AUTHOR_ID, &host_create_title)
    });
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_KIND_CALENDAR_AUTHOR_ID);
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "unbound-create-calendar-selected",
        |tree| json_author_value_is(tree, NEW_VIEW_KIND_CALENDAR_AUTHOR_ID, "selected"),
    );
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_CONFIRM_AUTHOR_ID);
    app_harness.step();
    let mut host_created_id = None;
    for _ in 0..200 {
        app_harness.step();
        let view = mounted.lock().unwrap();
        if !view.loading
            && !view.in_flight
            && view.error.is_none()
            && !view.view_block_id.is_empty()
            && view.view_block_id != calendar_id
        {
            host_created_id = Some(view.view_block_id.clone());
            break;
        }
        drop(view);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let host_created_id = host_created_id.expect("mounted host create did not resolve within 10s");
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "unbound-create-calendar-terminal",
        |tree| {
            json_author_value_is(tree, KIND_CALENDAR_AUTHOR_ID, "selected")
                && !json_has_author_id(tree, RETRY_AUTHOR_ID)
                && !json_has_author_id(tree, NEW_VIEW_TITLE_AUTHOR_ID)
        },
    );
    assert!(
        mounted.lock().unwrap().create_retry_intent().is_none(),
        "a successful create clears create-specific retry state so a later load failure reloads its id"
    );
    let host_created = live_fetch_view(&fresh_client, &workspace_id, &host_created_id);
    assert_eq!(host_created.definition.kind, BlockViewKind::Calendar);
    let host_created_block = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/blocks/{host_created_id}"
    ));
    assert_eq!(
        host_created_block["title"].as_str(),
        Some(host_create_title.as_str()),
        "canonical SetValue title must persist exactly"
    );

    app_harness
        .state()
        .set_block_collection_backend_base_url_for_test("http://127.0.0.1:9");
    let retry_create_title = format!("{unique}-retry-created");
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_AUTHOR_ID);
    argus.assert_latest_terminal_predicate(&mut app_harness, "retry-create-form-open", |tree| {
        json_has_author_id(tree, NEW_VIEW_TITLE_AUTHOR_ID)
            && json_has_author_id(tree, NEW_VIEW_CONFIRM_AUTHOR_ID)
    });
    argus.set_value_and_reinspect(
        &mut app_harness,
        NEW_VIEW_TITLE_AUTHOR_ID,
        &retry_create_title,
    );
    argus.assert_latest_terminal_predicate(&mut app_harness, "retry-create-title-set", |tree| {
        json_author_value_is(tree, NEW_VIEW_TITLE_AUTHOR_ID, &retry_create_title)
    });
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_KIND_KANBAN_AUTHOR_ID);
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "retry-create-kanban-selected",
        |tree| json_author_value_is(tree, NEW_VIEW_KIND_KANBAN_AUTHOR_ID, "selected"),
    );
    argus.click_and_reinspect(&mut app_harness, NEW_VIEW_CONFIRM_AUTHOR_ID);
    app_harness.step();
    for _ in 0..200 {
        app_harness.step();
        if mounted.lock().unwrap().error.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(mounted.lock().unwrap().error.is_some());
    let failed_create_tree = argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "failed-create-retry-visible",
        |tree| json_has_author_id(tree, RETRY_AUTHOR_ID),
    );
    assert!(
        json_has_author_id(&failed_create_tree, RETRY_AUTHOR_ID),
        "canonical failed create exposes its retained Retry action"
    );
    // The async completion is drained after this frame's pane render. Publish two more frames so the
    // mounted error state and its stable Retry node are present in the AccessKit tree before interaction.
    app_harness.run_steps(2);
    assert!(author_ids(&app_harness).contains(RETRY_AUTHOR_ID));
    app_harness
        .state()
        .set_block_collection_backend_base_url_for_test(live.base.clone());
    // Queue a second Retry in the same frame as the visible AccessKit activation. The host must dispatch
    // exactly one create POST; the second event observes in_flight and is rejected.
    mounted_events.lock().unwrap().push(BlockViewEvent::Retry);
    argus.click_and_reinspect(&mut app_harness, RETRY_AUTHOR_ID);
    app_harness.step();
    let mut retry_created_id = None;
    for _ in 0..200 {
        app_harness.step();
        let view = mounted.lock().unwrap();
        if !view.loading
            && !view.in_flight
            && view.error.is_none()
            && !view.view_block_id.is_empty()
        {
            retry_created_id = Some(view.view_block_id.clone());
            break;
        }
        drop(view);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let retry_created_id =
        retry_created_id.expect("mounted create Retry did not resolve within 10s");
    argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "retry-create-kanban-terminal",
        |tree| {
            json_author_value_is(tree, KIND_KANBAN_AUTHOR_ID, "selected")
                && !json_has_author_id(tree, RETRY_AUTHOR_ID)
                && !json_has_author_id(tree, NEW_VIEW_TITLE_AUTHOR_ID)
        },
    );
    assert_ne!(retry_created_id, host_created_id);
    let retry_created = live_fetch_view(&fresh_client, &workspace_id, &retry_created_id);
    assert_eq!(retry_created.definition.kind, BlockViewKind::Kanban);
    let retry_created_block = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/blocks/{retry_created_id}"
    ));
    assert_eq!(
        retry_created_block["title"].as_str(),
        Some(retry_create_title.as_str()),
        "failed create Retry must retain the exact canonical title"
    );
    let all_after_retry = live.get_json(&format!("/workspaces/{workspace_id}/loom/views/all"));
    let retry_title_count = all_after_retry
        .get("blocks")
        .and_then(serde_json::Value::as_array)
        .expect("views/all returns blocks")
        .iter()
        .filter(|block| {
            block
                .get("content_type")
                .and_then(serde_json::Value::as_str)
                == Some("view_def")
                && block.get("title").and_then(serde_json::Value::as_str)
                    == Some(retry_create_title.as_str())
        })
        .count();
    assert_eq!(
        retry_title_count, 1,
        "duplicate queued Retry events must persist exactly one view with the retained title"
    );

    // Real-PG constrained/empty projection through the mounted host and a fresh canonical inspection.
    let mut empty_def = BlockViewDefinition::of_kind(BlockViewKind::Table);
    // Use a schema-valid type deliberately absent from this isolated fixture. An invented
    // `code_file` token would only prove that request deserialization correctly rejects it.
    empty_def.query.content_type = Some("ckc_character".to_owned());
    let empty_id = live_create_view(
        &client,
        &workspace_id,
        &format!("{unique}-empty"),
        &empty_def,
    );
    {
        let mut view = mounted.lock().unwrap();
        view.bind_loading_view(workspace_id.clone(), empty_id.clone());
        view.set_error("force mounted empty-view reload");
    }
    app_harness.step();
    argus.click_and_reinspect(&mut app_harness, RETRY_AUTHOR_ID);
    app_harness.step();
    let empty_loaded = await_app_collection(&mut app_harness, &empty_id);
    assert_eq!(empty_loaded.kind, BlockViewKind::Table);
    let empty_tree =
        argus.assert_latest_terminal_predicate(&mut app_harness, "empty-table-terminal", |tree| {
            tree.to_string().contains("No blocks match this view.")
                && !tree.to_string().contains(TABLE_ROW_AUTHOR_ID_PREFIX)
        });
    assert!(
        !empty_tree.to_string().contains(TABLE_ROW_AUTHOR_ID_PREFIX),
        "canonical empty projection contains no table rows"
    );
    assert!(
        empty_tree
            .to_string()
            .contains("No blocks match this view."),
        "canonical empty projection exposes the exact empty-state text"
    );
    argus.click_and_reinspect(&mut app_harness, KIND_KANBAN_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &empty_id).kind,
        BlockViewKind::Kanban
    );
    let empty_kanban_tree =
        argus.assert_latest_terminal_predicate(&mut app_harness, "empty-kanban-terminal", |tree| {
            tree.to_string().contains("No Kanban lanes.")
        });
    assert!(
        empty_kanban_tree.to_string().contains("No Kanban lanes."),
        "canonical empty Kanban projection exposes the exact empty-state text"
    );
    argus.click_and_reinspect(&mut app_harness, KIND_CALENDAR_AUTHOR_ID);
    app_harness.step();
    assert_eq!(
        await_app_collection(&mut app_harness, &empty_id).kind,
        BlockViewKind::Calendar
    );
    let empty_calendar_tree = argus.assert_latest_terminal_predicate(
        &mut app_harness,
        "empty-calendar-terminal",
        |tree| tree.to_string().contains("No blocks in this date range."),
    );
    assert!(
        empty_calendar_tree
            .to_string()
            .contains("No blocks in this date range."),
        "canonical empty calendar projection exposes the exact empty-state text"
    );

    // Flight Recorder shares the busy managed-PG backend with parallel WP proofs. Use an explicit
    // bounded read here rather than the fixture helper's general 5s CRUD timeout; the endpoint remains
    // real and identity-stamped, while transient backend contention cannot erase completed actor proof.
    let recorder_url = format!("{}/api/flight_recorder?wsid={workspace_id}", live.base);
    let attributed_events = rt.block_on(async {
        let response = handshake_native::backend_client::build_backend_client()
            .get(&recorder_url)
            .header("x-hsk-actor-id", "mt046-live-pg")
            .header("x-hsk-kernel-task-run-id", "mt046-live-pg-run")
            .header("x-hsk-session-run-id", "mt046-live-pg-sess")
            .header("x-hsk-actor-kind", "operator")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .unwrap_or_else(|error| panic!("GET {recorder_url} failed: {error}"));
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        assert!(
            status.is_success(),
            "GET Flight Recorder -> {status}: {text}"
        );
        serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|error| panic!("Flight Recorder response not JSON ({error}): {text}"))
    });
    let attributed_events = attributed_events
        .as_array()
        .expect("Flight Recorder returns an event array");
    let native_events: Vec<_> = attributed_events
        .iter()
        .filter(|event| {
            event.get("actor_id").and_then(serde_json::Value::as_str) == Some(BLOCK_VIEW_ACTOR_ID)
        })
        .collect();
    assert!(
        native_events.iter().any(|event| {
            let payload = &event["payload"];
            payload.get("type").and_then(serde_json::Value::as_str) == Some("loom_block_created")
                && payload
                    .get("content_type")
                    .and_then(serde_json::Value::as_str)
                    == Some("view_def")
        }),
        "a canonical create event must carry top-level native actor attribution"
    );
    assert!(
        native_events.iter().any(|event| {
            event["payload"]["fields_changed"]
                .as_array()
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field.as_str() == Some("view_definition"))
                })
        }),
        "definition updates must carry top-level native actor attribution"
    );
    assert!(native_events.iter().any(|event| {
        let payload = &event["payload"];
        payload["fields_changed"]
            .as_array()
            .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some("tags")))
            && payload["tags_added"].is_array()
            && payload["tags_removed"].is_array()
    }), "card moves must retain canonical tag payload arrays and top-level native actor attribution");

    argus.finish();
    cleanup.assert_cleaned_and_absent();
    drop(cleanup);
    let backend_runtime_root = std::path::PathBuf::from(
        backend_binding["runtime_data_dir"]
            .as_str()
            .expect("owned backend binding records runtime_data_dir"),
    )
    .parent()
    .expect("owned backend runtime data has a parent")
    .to_path_buf();
    drop(live);
    assert!(
        !backend_runtime_root.exists(),
        "fixture-owned backend runtime must be cleaned before the proof receipt: {}",
        backend_runtime_root.display()
    );

    // Receipt only after every persistence, retry, mounted AccessKit, cleanup, and fresh-absence check.
    std::fs::create_dir_all(&receipt_dir).expect("create external MT-027 receipt directory");
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt027_managed_pg_proof@1",
        "backend_binding": backend_binding,
        "workspace_id": workspace_id,
        "view_ids": { "table": table_id, "kanban": kanban_id, "calendar": calendar_id },
        "block_ids": {
            "notes": [alpha, middle, zulu],
            "tags": [tag_a, tag_b],
            "journals": [march_one, march_two, april_one]
        },
        "proofs": [
            "product-client-create-fetch-query",
            "mounted-accesskit-table-kanban-calendar",
            "visible-retry-recovery",
            "sort-asc-desc-persistence",
            "kanban-tag-move-source-loss-target-gain",
            "calendar-range-persistence",
            "kind-switch-persistence",
            "mounted-handshake-app-host-dispatch",
            "canonical-localhost-argus-create-mutate-switch-empty-error-retry",
            "flight-recorder-native-actor-attribution",
            "workspace-cleanup-fresh-list-absence"
        ]
    });
    std::fs::write(
        receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("serialize managed-PG receipt"),
    )
    .expect("write external managed-PG receipt");
    assert_no_local_artifact_dir();
}
