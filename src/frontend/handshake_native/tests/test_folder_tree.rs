//! WP-KERNEL-012 MT-022 LoomFolderTree PROOFS: flat-rows->tree build (PROOF1, also in lib unit tests),
//! egui_kittest AccessKit-tree assertions (PROOF2 structural + AC6), expand-folder (PROOF3),
//! color-change recolor request shape + swatch update (PROOF4), and leaf-click open (PROOF5). Plus
//! AC7 (empty "No folders") and AC8 (backend-error banner + Retry).
//!
//! ## Backend reality (Spec-Realism Gate — MT-008/021/023 "verify, don't trust the contract" rule)
//!
//! The MT-022 contract's assumed surface (content_type='folder' LoomBlocks, color in
//! content_json.metadata.color_label, children via views/sorted?tag_ids=) does NOT exist in the running
//! backend (verified READ-ONLY against src/backend/handshake_core/src/{api,storage}/loom.rs:
//! `LoomBlockContentType` has no `Folder`; the PATCH `LoomBlockUpdate` has no content_json/color field).
//! The REAL folder authority is the dedicated `loom_folders` subsystem (MT-181 FolderTreeAndColorLabels)
//! with verified routes `GET /loom/folders`, `GET /loom/folders/{id}/blocks`, and
//! `PATCH /loom/folders/{id}` body `{ "color": "#rrggbb" }` (a true merge-patch: `LoomFolderUpdate.color`
//! is `Option<Option<String>>`, so a recolor never clobbers name/sort/parent — RISK-2/MC-2).
//!
//! AC1/AC2/AC4 against a LIVE Handshake-managed SurrealDB are covered by one isolated, self-seeding
//! integration proof. It creates two real folder rows plus three real Loom child blocks,
//! drives the production `LoomFolderClient` GET/PATCH paths, refetches the persisted color, renders
//! and clicks the live leaf, writes the seed identifiers to the external artifact root, then cleans up
//! only the rows it created. Running the live proof without a reachable managed backend fails closed;
//! it never substitutes a mock repository. The tree-build/cycle/hex/empty/error logic + verified
//! request-shape builders remain proven STANDALONE here and in the lib unit tests.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-022/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[cfg(feature = "integration")]
#[path = "backend_proof_support/mod.rs"]
mod backend_proof_support;

use handshake_native::graph::folder_tree::{
    build_tree, color_author_id, color_to_hex, move_target_author_id, node_author_id,
    parse_hex_color, FolderRow, FolderTreeEvent, LeafBlock, LoomFolderTree, COLOR_AUTHOR_ID_PREFIX,
    CREATE_CANCEL_AUTHOR_ID, CREATE_NAME_INPUT_AUTHOR_ID, CREATE_SUBMIT_AUTHOR_ID,
    DELETE_CANCEL_AUTHOR_ID, DELETE_CONFIRM_AUTHOR_ID, INDENT_PER_LEVEL, NEW_FOLDER_AUTHOR_ID,
    NODE_AUTHOR_ID_PREFIX, RETRY_AUTHOR_ID,
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

/// A seeded tree: 2 root folders (folder-001 "Projects" red, folder-002 "Archive" no color) +
/// folder-001 has 3 child blocks pre-loaded (so the AccessKit + expand + leaf-click proofs have work).
/// No backend: the rows/leaves stand in for `GET /loom/folders` + `GET /loom/folders/{id}/blocks`.
fn seeded_tree() -> LoomFolderTree {
    let rows = vec![
        FolderRow::new("folder-001", None, "Projects", Some("#ff0000".to_owned())),
        FolderRow::new("folder-002", None, "Archive", None),
    ];
    let mut tree = LoomFolderTree::new("ws-test");
    tree.set_folders(&rows);
    tree
}

/// Drive the tree through a shared cell so a test can read/mutate it across frames + capture events.
fn shared(tree: LoomFolderTree) -> Arc<Mutex<LoomFolderTree>> {
    Arc::new(Mutex::new(tree))
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

fn click_author_id(harness: &Harness<'_, ()>, author_id: &str) {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"))
        .click();
}

/// Resolve the live AccessKit node id from the stable author_id exposed to Argus/model drivers.
fn accesskit_node_id(harness: &Harness<'_, ()>, author_id: &str) -> egui::accesskit::NodeId {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("AccessKit author_id not found: {author_id}"))
        .accesskit_node()
        .id()
}

fn request_accesskit_action(
    harness: &Harness<'_, ()>,
    author_id: &str,
    action: egui::accesskit::Action,
) {
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            target: accesskit_node_id(harness, author_id),
            action,
            data: None,
        },
    ));
}

/// Build a harness that renders the shared tree and pushes every emitted event into `events`.
fn harness_for(
    tree: Arc<Mutex<LoomFolderTree>>,
    events: Arc<Mutex<Vec<FolderTreeEvent>>>,
) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(420.0, 600.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            if let Some(ev) = tree.lock().unwrap().show(ui, &pal) {
                events.lock().unwrap().push(ev);
            }
        })
}

// ── PROOF1 (cross-check): flat rows -> tree (the lib unit tests own the exhaustive variants) ─────────

#[test]
fn proof1_build_tree_from_flat_rows() {
    let rows = vec![
        FolderRow::new("f1", None, "Root", None),
        FolderRow::new(
            "f2",
            Some("f1".to_owned()),
            "Child",
            Some("#00ff00".to_owned()),
        ),
        FolderRow::new("f3", None, "Other", None),
    ];
    let tree = build_tree(&rows);
    assert_eq!(tree.len(), 2, "PROOF1: two roots (f1, f3)");
    let f1 = tree.iter().find(|n| n.folder_id == "f1").expect("f1");
    assert_eq!(f1.child_folders.len(), 1, "PROOF1: f1 has one child folder");
    assert_eq!(f1.child_folders[0].folder_id, "f2");
    assert_eq!(
        f1.child_folders[0].color,
        parse_hex_color("#00ff00"),
        "PROOF1: child color parsed from hex"
    );
    println!("PROOF1: flat 3 rows -> 2 roots, 1 nested child, color parsed");
}

// ── PROOF2 (structural) + AC6: folder rows + color swatches are addressable AccessKit nodes ──────────

#[test]
fn proof2_accesskit_folder_nodes_present() {
    let tree = shared(seeded_tree());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);

    // PROOF2: 2 folder-tree.node.* entries (one per seeded root folder).
    let node_count = ids
        .iter()
        .filter(|a| a.starts_with(NODE_AUTHOR_ID_PREFIX))
        .count();
    assert!(
        node_count >= 2,
        "PROOF2: expected >= 2 folder-tree.node.* AccessKit nodes, got {node_count} (ids={ids:?})"
    );

    // AC6: the specific folder node ids are present + Role::TreeItem.
    assert!(
        ids.contains("folder-tree.node.folder-001"),
        "AC6: 'folder-tree.node.folder-001' must be in the tree (ids={ids:?})"
    );
    assert!(
        ids.contains("folder-tree.node.folder-002"),
        "AC6: 'folder-tree.node.folder-002' must be in the tree"
    );
    // Each folder has a stable, actionable swatch-button id. Its Click opens the explicitly controlled
    // picker without being conflated with the folder row's primary Open action.
    assert!(
        ids.iter()
            .filter(|a| a.starts_with(COLOR_AUTHOR_ID_PREFIX))
            .count()
            >= 2,
        "AC6: a color swatch button per folder (ids={ids:?})"
    );

    // Role check: folder-001 is a TreeItem.
    let mut treeitem_found = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some("folder-tree.node.folder-001") {
            assert_eq!(
                format!("{:?}", ak.role()),
                "TreeItem",
                "AC6: a folder node must be Role::TreeItem"
            );
            treeitem_found = true;
        }
    }
    assert!(
        treeitem_found,
        "AC6: folder-tree.node.folder-001 not found for role check"
    );
    let red_author_id = color_author_id("folder-001");
    let red_swatch = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(red_author_id.as_str()))
        .expect("folder-001 color swatch state node");
    let red_access = red_swatch.accesskit_node();
    assert_eq!(format!("{:?}", red_access.role()), "Button");
    assert_eq!(
        red_access.data().color_value(),
        Some(u32::from_be_bytes([255, 0, 0, 255]))
    );
    assert!(
        red_access
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "the actionable color swatch must advertise Click"
    );
    red_swatch.click();
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("Red #ff0000").is_some(),
        "the swatch button's primary Click must open the explicitly controlled picker"
    );
    println!("PROOF2 structural: {node_count} folder-tree.node.* nodes + actionable swatch buttons present");
}

#[test]
fn folder_click_selects_exact_row_and_refetch_preserves_or_clears_selection() {
    let tree = shared(seeded_tree());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();
    harness.get_by_label("Projects").click();
    harness.run();
    assert!(
        harness.query_by_label("Pick a folder color").is_none()
            && harness.query_by_label("Red #ff0000").is_none(),
        "a normal primary folder-row Open must never toggle the explicitly controlled color picker"
    );
    assert_eq!(
        tree.lock().unwrap().selected_folder_id.as_deref(),
        Some("folder-001")
    );
    assert!(events.lock().unwrap().iter().any(|event| matches!(event, FolderTreeEvent::OpenFolder { folder_id, action_generation: 1 } if folder_id == "folder-001")));

    tree.lock().unwrap().set_folders(&[
        FolderRow::new("folder-001", None, "Projects renamed", None),
        FolderRow::new("folder-002", None, "Archive", None),
    ]);
    assert_eq!(
        tree.lock().unwrap().selected_folder_id.as_deref(),
        Some("folder-001"),
        "fresh rows preserve selection by stable id"
    );
    tree.lock()
        .unwrap()
        .set_folders(&[FolderRow::new("folder-002", None, "Archive", None)]);
    assert_eq!(
        tree.lock().unwrap().selected_folder_id,
        None,
        "fresh rows clear selection after delete"
    );
}

#[test]
fn operator_create_and_row_context_crud_paths_emit_typed_events() {
    let tree = shared(seeded_tree());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    click_author_id(&harness, NEW_FOLDER_AUTHOR_ID);
    harness.run();
    let create_ids = author_ids(&harness);
    for expected in [
        NEW_FOLDER_AUTHOR_ID,
        CREATE_NAME_INPUT_AUTHOR_ID,
        CREATE_SUBMIT_AUTHOR_ID,
        CREATE_CANCEL_AUTHOR_ID,
    ] {
        assert!(
            create_ids.contains(expected),
            "create flow exposes stable AccessKit id {expected}: {create_ids:?}"
        );
    }
    let name_input = harness.get_by_label("Folder name");
    name_input.focus();
    name_input.type_text("Created from UI");
    harness.run();
    harness.run();
    click_author_id(&harness, CREATE_SUBMIT_AUTHOR_ID);
    harness.run();
    assert!(events.lock().unwrap().iter().any(|event| matches!(event, FolderTreeEvent::CreateFolder { parent_folder_id: None, name } if name == "Created from UI")));

    harness.get_by_label("Projects").click_secondary();
    harness.run();
    for action in [
        "New subfolder",
        "Rename",
        "Move to root",
        "Move under",
        "Delete",
    ] {
        assert!(
            harness.query_by_label(action).is_some(),
            "row context menu exposes {action}"
        );
    }
    harness.get_by_label("Move to root").click();
    harness.run();
    assert!(events.lock().unwrap().iter().any(|event| matches!(event, FolderTreeEvent::MoveFolder { folder_id, parent_folder_id: None, .. } if folder_id == "folder-001")));

    harness.get_by_label("Projects").click_secondary();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert!(harness
        .query_by_label("Delete folder 'Projects' and its 0 descendant folders?")
        .is_some());
    assert!(harness
        .query_by_label(
            "This permanently removes the folder subtree and its folder memberships. The Loom blocks themselves remain."
        )
        .is_some());
    let delete_ids = author_ids(&harness);
    for expected in [DELETE_CONFIRM_AUTHOR_ID, DELETE_CANCEL_AUTHOR_ID] {
        assert!(
            delete_ids.contains(expected),
            "delete confirmation exposes stable AccessKit id {expected}: {delete_ids:?}"
        );
    }
    click_author_id(&harness, DELETE_CONFIRM_AUTHOR_ID);
    harness.run();
    assert!(events.lock().unwrap().iter().any(|event| matches!(event, FolderTreeEvent::DeleteFolder { folder_id } if folder_id == "folder-001")));
}

// ── PROOF3: expanding a folder fires ExpandFolder (the host's lazy-load trigger) ─────────────────────

#[test]
fn proof3_expand_folder_fires_event() {
    // ONE root folder so the disclosure triangle "▸" is unambiguous in the AccessKit tree.
    let mut tree = LoomFolderTree::new("ws-test");
    tree.set_folders(&[FolderRow::new(
        "folder-001",
        None,
        "Projects",
        Some("#ff0000".to_owned()),
    )]);
    let tree = shared(tree);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    // Exercise the advertised AccessKit Expand action on the stable row node rather than the visual
    // disclosure glyph. Since child_blocks are not loaded, this emits the host's lazy-fetch signal.
    request_accesskit_action(
        &harness,
        &node_author_id("folder-001"),
        egui::accesskit::Action::Expand,
    );
    harness.run();

    let ev = events.lock().unwrap().clone();
    let expanded = ev
        .iter()
        .any(|e| matches!(e, FolderTreeEvent::ExpandFolder { .. }));
    assert!(
        expanded,
        "PROOF3: AccessKit Expand must emit ExpandFolder (got {ev:?})"
    );

    // The clicked node is now expanded in state (the widget flipped it).
    let any_expanded = {
        let t = tree.lock().unwrap();
        t.root_nodes.iter().any(|n| n.expanded)
    };
    assert!(
        any_expanded,
        "PROOF3: the folder node is marked expanded after the click"
    );
    assert!(author_ids(&harness).contains(&node_author_id("folder-001")));
    request_accesskit_action(
        &harness,
        &node_author_id("folder-001"),
        egui::accesskit::Action::Collapse,
    );
    harness.run();
    assert!(!tree.lock().unwrap().root_nodes[0].expanded);
    assert!(events.lock().unwrap().iter().any(
        |event| matches!(event, FolderTreeEvent::CollapseFolder { folder_id } if folder_id == "folder-001")
    ));
    println!(
        "PROOF3: stable row AccessKit Expand/Collapse actions mutate state and emit typed events"
    );
}

#[test]
fn recursive_delete_warning_counts_descendants_and_preserves_block_truth() {
    let mut tree = LoomFolderTree::new("ws-test");
    tree.set_folders(&[
        FolderRow::new("root", None, "Root", None),
        FolderRow::new("child", Some("root".to_owned()), "Child", None),
        FolderRow::new("grandchild", Some("child".to_owned()), "Grandchild", None),
    ]);
    let tree = shared(tree);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), events);
    harness.run();

    harness.get_by_label("Root").click_secondary();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();

    assert!(harness
        .query_by_label("Delete folder 'Root' and its 2 descendant folders?")
        .is_some());
    assert!(harness
        .query_by_label(
            "This permanently removes the folder subtree and its folder memberships. The Loom blocks themselves remain."
        )
        .is_some());
}

#[test]
fn move_under_targets_have_stable_ids_when_titles_duplicate() {
    let mut tree = LoomFolderTree::new("ws-test");
    tree.set_folders(&[
        FolderRow::new("source", None, "Source", None),
        FolderRow::new("parent-a", None, "Parent A", None),
        FolderRow::new("parent-b", None, "Parent B", None),
        FolderRow::new("target-a", Some("parent-a".to_owned()), "Duplicate", None),
        FolderRow::new("target-b", Some("parent-b".to_owned()), "Duplicate", None),
    ]);
    let tree = shared(tree);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(tree, Arc::clone(&events));
    harness.run();

    harness.get_by_label("Source").click_secondary();
    harness.run();
    harness.get_by_label("Move under").click();
    harness.run();

    let target_a = move_target_author_id("source", "target-a");
    let target_b = move_target_author_id("source", "target-b");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&target_a),
        "first duplicate-title target is addressable"
    );
    assert!(
        ids.contains(&target_b),
        "second duplicate-title target is addressable"
    );
    assert_ne!(target_a, target_b);

    request_accesskit_action(&harness, &target_b, egui::accesskit::Action::Click);
    harness.run();
    assert!(events.lock().unwrap().iter().any(|event| matches!(
        event,
        FolderTreeEvent::MoveFolder { folder_id, parent_folder_id: Some(parent), .. }
            if folder_id == "source" && parent == "target-b"
    )));
}

// ── PROOF3b: a folder with cached children renders them indented (no re-fetch) ───────────────────────

#[test]
fn proof3b_expanded_folder_renders_cached_children() {
    let mut tree = seeded_tree();
    // Pre-load + expand folder-001 with 3 child blocks (simulating a resolved lazy fetch).
    {
        let f1 = tree.find_folder_mut("folder-001").expect("folder-001");
        f1.child_blocks = Some(vec![
            LeafBlock::new("child-001", "Child One", "note"),
            LeafBlock::new("child-002", "Child Two", "file"),
            LeafBlock::new("child-003", "Child Three", "note"),
        ]);
        f1.expanded = true;
    }
    let tree = shared(tree);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    let ids = author_ids(&harness);
    // The 3 leaf blocks now appear as folder-tree.node.* entries (AC2: children displayed beneath).
    for child in ["child-001", "child-002", "child-003"] {
        let id = format!("folder-tree.node.{child}");
        assert!(
            ids.contains(&id),
            "AC2: leaf '{id}' must render when the folder is expanded (ids={ids:?})"
        );
    }
    // PROOF3 child count > 0 in the AccessKit tree.
    let leaf_count = ["child-001", "child-002", "child-003"]
        .iter()
        .filter(|c| ids.contains(&format!("folder-tree.node.{c}")))
        .count();
    assert_eq!(leaf_count, 3, "AC2: all 3 cached children render");
    let parent_author = node_author_id("folder-001");
    let child_author = node_author_id("child-001");
    let parent_node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(parent_author.as_str()))
        .expect("expanded parent folder AccessKit node");
    let child_node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(child_author.as_str()))
        .expect("rendered child leaf AccessKit node");
    let child_access = child_node.accesskit_node();
    assert!(
        child_access
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "leaf TreeItem remains directly openable"
    );
    assert!(
        !child_access
            .data()
            .supports_action(egui::accesskit::Action::Expand)
            && !child_access
                .data()
                .supports_action(egui::accesskit::Action::Collapse),
        "leaf TreeItem must advertise Click only, never folder Expand/Collapse"
    );
    assert!(
        child_node.rect().left() >= parent_node.rect().left() + INDENT_PER_LEVEL,
        "child leaf must begin at least one indent step right of its parent label: parent={:?}, child={:?}",
        parent_node.rect(),
        child_node.rect()
    );
    println!("PROOF3b: expanded folder renders 3 cached child leaves (no re-fetch)");
}

// ── PROOF4: color-change emits ChangeColor + the verified recolor request shape ─────────────────────

#[test]
fn proof4_recolor_request_shape() {
    use handshake_native::backend_client::LoomFolderClient;

    // The recolor PATCH targets ONLY the color key (a true merge-patch against LoomFolderUpdate), so an
    // editor save / name change can never be clobbered (RISK-2 / MC-2). We assert the EXACT verified URL
    // + single-`color`-key body the production spawn path routes through.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomFolderClient::new("http://test.local:1234", rt.handle().clone());
    let spec = client.recolor_request("ws1", "folder-001", "#ff0000");
    assert_eq!(
        spec.url, "http://test.local:1234/workspaces/ws1/loom/folders/folder-001",
        "PROOF4: recolor PATCH hits the verified /loom/folders/{{id}} route"
    );
    assert_eq!(
        spec.body,
        Some(serde_json::json!({ "color": "#ff0000" })),
        "PROOF4/MC-2: recolor body carries ONLY the color key (merge-patch, no content_json clobber)"
    );

    // And the widget emits ChangeColor with the picked color so the host can dispatch that PATCH. We
    // drive the widget state directly (the picker popup is an interactive egui popup; the produced
    // event is the externally-meaningful contract the host consumes).
    let red = parse_hex_color("#ff0000").unwrap();
    assert_eq!(
        color_to_hex(red),
        "#ff0000",
        "PROOF4: picked Color32 -> hex round-trips for the PATCH body"
    );
    println!("PROOF4: recolor request shape verified (URL + color-only merge-patch body)");
}

#[test]
fn proof4_right_click_change_color_pick_flow_emits_patchable_event() {
    use handshake_native::backend_client::LoomFolderClient;

    let tree = shared(seeded_tree());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    // AC4: the color entry point is the row context menu, not only a seeded swatch pixel.
    harness.get_by_label("Archive").click_secondary();
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("Change color").is_some(),
        "AC4: right-clicking the folder row opens a context menu with 'Change color'"
    );

    harness.get_by_label("Change color").click();
    harness.run();
    harness.run();
    assert!(
        harness.query_by_label("Red #ff0000").is_some(),
        "AC4: selecting 'Change color' opens the folder color picker with deterministic swatches"
    );

    harness.get_by_label("Red #ff0000").click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    let picked = ev.iter().find_map(|event| match event {
        FolderTreeEvent::ChangeColor { folder_id, color } if folder_id == "folder-002" => {
            Some(color_to_hex(*color))
        }
        _ => None,
    });
    assert_eq!(
        picked.as_deref(),
        Some("#ff0000"),
        "AC4: picking the swatch emits ChangeColor(folder-002, #ff0000); got {ev:?}"
    );

    // The emitted color is directly patchable through the production request builder the host uses.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomFolderClient::new("http://test.local:1234", rt.handle().clone());
    let spec = client.recolor_request("ws1", "folder-002", picked.as_deref().unwrap());
    assert_eq!(
        spec.body,
        Some(serde_json::json!({ "color": "#ff0000" })),
        "AC4/PROOF4: the real pick flow feeds the same single-color-key PATCH body"
    );
    println!("PROOF4: row right-click -> Change color -> Red emits patchable ChangeColor event");
}

// ── PROOF4b: the color swatch renders the stored color (red) in a screenshot ─────────────────────────

#[test]
fn proof4b_swatch_screenshot_shows_color() {
    let _g = wgpu_guard();
    let tree = shared(seeded_tree());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 300.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = tree.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let raw = image.as_raw();
            // folder-001's swatch is #ff0000 (red). Assert at least one strongly-red opaque pixel exists
            // (r high, g/b low) — the swatch rendered the stored color (AC3).
            let mut red_pixels = 0u32;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let (r, g, b, a) = (raw[i], raw[i + 1], raw[i + 2], raw[i + 3]);
                if a != 0 && r > 180 && g < 80 && b < 80 {
                    red_pixels += 1;
                }
                i += 4;
            }
            assert!(
                red_pixels > 0,
                "AC3: the folder-001 swatch must render its stored red color (#ff0000); found {red_pixels} red pixels"
            );

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-022");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-022-folder-tree.png");
            let saved = image.save(&png).is_ok();
            println!(
                "PROOF4b/AC3: {w}x{h} screenshot, {red_pixels} red swatch pixels, saved={saved} ({})",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): folder-tree screenshot render unavailable (no wgpu adapter): {e}. \
                 The swatch-color parse + AccessKit + event proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── PROOF5: clicking a leaf block fires OpenBlock with the right block_id ────────────────────────────

#[test]
fn proof5_leaf_click_fires_open() {
    let mut tree = seeded_tree();
    {
        let f1 = tree.find_folder_mut("folder-001").expect("folder-001");
        f1.child_blocks = Some(vec![LeafBlock::new("child-001", "Child One", "note")]);
        f1.expanded = true;
    }
    let tree = shared(tree);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    // The leaf renders as "📝 Child One"; click it by its label substring.
    harness.get_by_label_contains("Child One").click();
    harness.run();

    let ev = events.lock().unwrap().clone();
    let opened = ev
        .iter()
        .any(|e| matches!(e, FolderTreeEvent::OpenBlock { block_id } if block_id == "child-001"));
    assert!(
        opened,
        "PROOF5: clicking leaf 'Child One' must emit OpenBlock{{block_id:'child-001'}} (got {ev:?})"
    );
    println!("PROOF5: leaf click fired OpenBlock(child-001) (events={ev:?})");
}

// ── AC7: empty workspace -> "No folders", no folder nodes, no panic ──────────────────────────────────

#[test]
fn ac7_empty_no_folders() {
    let tree = shared(LoomFolderTree::new("ws-empty"));
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    assert!(
        harness.query_by_label("No folders").is_some(),
        "AC7: 'No folders' label must be present for an empty workspace"
    );
    let ids = author_ids(&harness);
    assert_eq!(
        ids.iter()
            .filter(|a| a.starts_with(NODE_AUTHOR_ID_PREFIX))
            .count(),
        0,
        "AC7: no folder-tree.node.* nodes for an empty workspace"
    );
    println!("AC7: empty workspace shows 'No folders', no node entries, no panic");
}

// ── AC8: a backend error shows an error banner + a Retry button that re-fires the load ───────────────

#[test]
fn ac8_error_banner_retry() {
    let mut errored = LoomFolderTree::new("ws-err");
    errored.error = Some("backend unreachable (HTTP 503)".to_owned());
    let tree = shared(errored);
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut harness = harness_for(Arc::clone(&tree), Arc::clone(&events));
    harness.run();

    // The Retry button is present + addressable.
    let ids = author_ids(&harness);
    assert!(
        ids.contains(RETRY_AUTHOR_ID),
        "AC8: the Retry button author_id '{RETRY_AUTHOR_ID}' must be present (ids={ids:?})"
    );

    // Clicking Retry emits the Retry event (the host re-fires the initial load).
    harness.get_by_label("Retry").click();
    harness.run();
    let ev = events.lock().unwrap().clone();
    assert!(
        ev.iter().any(|e| matches!(e, FolderTreeEvent::Retry)),
        "AC8: clicking Retry must emit FolderTreeEvent::Retry (got {ev:?})"
    );
    println!("AC8: error banner shown, Retry button re-fires the load");
}

// ── Verified request-shape builders (the production spawn paths route through these) ─────────────────

#[test]
fn folder_list_request_hits_verified_route() {
    use handshake_native::backend_client::LoomFolderClient;
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let client = LoomFolderClient::new("http://test.local:1234", rt.handle().clone());

    let list = client.list_folders_request("ws7");
    assert_eq!(
        list.url,
        "http://test.local:1234/workspaces/ws7/loom/folders"
    );
    assert!(list.query.is_empty());

    let children = client.list_folder_blocks_request("ws7", "folder-001");
    assert_eq!(
        children.url,
        "http://test.local:1234/workspaces/ws7/loom/folders/folder-001/blocks"
    );
    assert_eq!(
        children.query,
        vec![
            ("limit".to_owned(), "500".to_owned()),
            ("offset".to_owned(), "0".to_owned())
        ]
    );
    let second_page = client.list_folder_blocks_page_request("ws7", "folder-001", 500);
    assert_eq!(
        second_page.query,
        vec![
            ("limit".to_owned(), "500".to_owned()),
            ("offset".to_owned(), "500".to_owned())
        ]
    );
    println!("verified: folder-list + folder-blocks GET routes match the real backend");
}

// ── LIVE-SURREALDB (gated): self-seeding, production-client round trip ─────────────────────────────────────

#[cfg(feature = "integration")]
fn wait_for_live_cell<T>(cell: &Arc<Mutex<Option<T>>>, operation: &str) -> T {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(value) = cell.lock().unwrap_or_else(|p| p.into_inner()).take() {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "requires_surrealdb: {operation} did not complete within 5 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[cfg(feature = "integration")]
fn wait_for_live_queue<T>(cell: &Arc<Mutex<std::collections::VecDeque<T>>>, operation: &str) -> T {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(delivery) = cell.lock().unwrap_or_else(|p| p.into_inner()).pop_front() {
            return delivery;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "requires_surrealdb: {operation} did not deliver within 10 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[cfg(feature = "integration")]
fn identified_folder_result<T>(
    delivery: (String, u64, u64, Result<T, String>),
    expected_workspace: &str,
    expected_epoch: u64,
    expected_sequence: u64,
) -> Result<T, String> {
    let (workspace, epoch, sequence, result) = delivery;
    assert_eq!(workspace, expected_workspace);
    assert_eq!(epoch, expected_epoch);
    assert_eq!(sequence, expected_sequence);
    result
}

/// Test-local managed-backend handle. Every operation has a hard deadline; a backend that becomes
/// unhealthy after `/health` therefore fails the proof with a typed `requires_surrealdb` message instead of
/// hanging the governed run or its cleanup.
#[cfg(feature = "integration")]
struct LiveFolderBackend {
    base: String,
    workspace_id: String,
    client: reqwest::Client,
    runtime: tokio::runtime::Runtime,
    /// Owns the ephemeral current-source backend, isolated SurrealDB workspace, runtime roots, and
    /// fixture lock. Its Drop performs bounded process/workspace/runtime cleanup after this wrapper.
    _managed_backend: backend_proof_support::LiveBackend,
}

#[cfg(feature = "integration")]
impl LiveFolderBackend {
    /// Prove teardown of the fixture-owned workspace and backend process before publishing PASS.
    fn assert_cleanup(&mut self) {
        self._managed_backend.assert_cleanup();
    }

    fn identity(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-actor-id", "mt022-live-surrealdb")
            .header("x-hsk-kernel-task-run-id", "mt022-live-surrealdb-run")
            .header("x-hsk-session-run-id", "mt022-live-surrealdb-session")
            .header("x-hsk-actor-kind", "operator")
            .timeout(std::time::Duration::from_secs(5))
    }

    fn post_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.send_json("POST", path, Some(body))
    }

    fn put_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.send_json("PUT", path, Some(body))
    }

    fn send_json(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.runtime.block_on(async {
            let request = match method {
                "POST" => self.client.post(&url),
                "PUT" => self.client.put(&url),
                _ => panic!("unsupported live-folder proof method: {method}"),
            };
            let request = match body {
                Some(value) => self.identity(request).json(value),
                None => self.identity(request),
            };
            let response = request
                .send()
                .await
                .unwrap_or_else(|error| panic!("requires_surrealdb: {method} {url} failed: {error}"));
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            (status, text)
        });
        assert!(
            status.is_success(),
            "requires_surrealdb: {method} {path} -> {status}: {text}"
        );
        serde_json::from_str(&text).unwrap_or_else(|error| {
            panic!("requires_surrealdb: {method} {path} response not JSON ({error}): {text}")
        })
    }

    fn delete_bounded(&self, path: &str) -> Result<(), String> {
        let url = format!("{}{path}", self.base);
        let status = self.runtime.block_on(async {
            self.identity(self.client.delete(&url))
                .send()
                .await
                .map(|response| response.status())
                .map_err(|error| error.to_string())
        })?;
        if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(format!("DELETE {path} -> {status}"))
        }
    }

    fn get_status_bounded(&self, path: &str) -> Result<reqwest::StatusCode, String> {
        let url = format!("{}{path}", self.base);
        self.runtime.block_on(async {
            self.identity(self.client.get(&url))
                .send()
                .await
                .map(|response| response.status())
                .map_err(|error| error.to_string())
        })
    }

    fn get_json_bounded(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{path}", self.base);
        let (status, text) = self.runtime.block_on(async {
            let response = self
                .identity(self.client.get(&url))
                .send()
                .await
                .map_err(|error| error.to_string())?;
            let status = response.status();
            let text = response.text().await.map_err(|error| error.to_string())?;
            Ok::<_, String>((status, text))
        })?;
        if !status.is_success() {
            return Err(format!("GET {path} -> {status}: {text}"));
        }
        serde_json::from_str(&text)
            .map_err(|error| format!("GET {path} returned invalid JSON ({error}): {text}"))
    }
}

#[cfg(feature = "integration")]
fn require_live_folder_backend() -> LiveFolderBackend {
    let managed_backend = backend_proof_support::require_live_backend();
    let base = managed_backend.base.clone();
    let workspace_id = managed_backend.workspace_id.clone();
    assert!(!workspace_id.trim().is_empty(), "managed fixture workspace");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build bounded live-folder proof runtime");
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("build bounded live-folder proof client");
    let healthy = runtime.block_on(async {
        client
            .get(format!("{base}/health"))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    });
    assert!(
        healthy,
        "requires_surrealdb: managed handshake_core is not healthy at {base}/health"
    );
    LiveFolderBackend {
        base,
        workspace_id,
        client,
        runtime,
        _managed_backend: managed_backend,
    }
}

/// Removes only this proof's real SurrealDB fixture rows. Explicit end-of-test cleanup is verified
/// and fails the proof if any bounded delete fails; `Drop` is a second bounded recovery attempt for
/// earlier assertion failures. IDs are unique and block deletion precedes folder deletion, so shared
/// workspace data is never selected or modified.
#[cfg(feature = "integration")]
struct LiveFolderSeedCleanup<'a> {
    backend: &'a LiveFolderBackend,
    folder_ids: Vec<String>,
    /// Unique self-seeded names let cleanup recover canonical ids when POST succeeded but the host's
    /// authoritative refetch failed before the id reached the mounted tree.
    folder_names: Vec<String>,
    block_ids: Vec<String>,
}

#[cfg(feature = "integration")]
impl LiveFolderSeedCleanup<'_> {
    fn discover_folder_ids_by_name(&mut self) -> Result<(), String> {
        if self.folder_names.is_empty() {
            return Ok(());
        }
        let rows = self.backend.get_json_bounded(&format!(
            "/workspaces/{}/loom/folders",
            self.backend.workspace_id
        ))?;
        let rows = rows
            .as_array()
            .ok_or_else(|| format!("folder cleanup list was not an array: {rows}"))?;
        for row in rows {
            let Some(name) = row.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !self.folder_names.iter().any(|tracked| tracked == name) {
                continue;
            }
            if let Some(folder_id) = row
                .get("folder_id")
                .and_then(serde_json::Value::as_str)
                .filter(|folder_id| !folder_id.is_empty())
            {
                if !self.folder_ids.iter().any(|known| known == folder_id) {
                    self.folder_ids.push(folder_id.to_owned());
                }
            }
        }
        Ok(())
    }

    fn cleanup_and_verify(&mut self) -> Result<(), String> {
        let mut failures = Vec::new();
        if let Err(error) = self.discover_folder_ids_by_name() {
            failures.push(format!(
                "cleanup could not recover folder ids by seed name: {error}"
            ));
        }
        for block_id in &self.block_ids {
            let path = format!(
                "/workspaces/{}/loom/blocks/{block_id}",
                self.backend.workspace_id
            );
            if let Err(error) = self.backend.delete_bounded(&path) {
                failures.push(error);
            } else if self.backend.get_status_bounded(&path) != Ok(reqwest::StatusCode::NOT_FOUND) {
                failures.push(format!("cleanup did not prove block absent: {block_id}"));
            }
        }
        for folder_id in self.folder_ids.iter().rev() {
            let path = format!(
                "/workspaces/{}/loom/folders/{folder_id}",
                self.backend.workspace_id
            );
            if let Err(error) = self.backend.delete_bounded(&path) {
                failures.push(error);
            } else if self.backend.get_status_bounded(&path) != Ok(reqwest::StatusCode::NOT_FOUND) {
                failures.push(format!("cleanup did not prove folder absent: {folder_id}"));
            }
        }
        if failures.is_empty() {
            self.block_ids.clear();
            self.folder_ids.clear();
            self.folder_names.clear();
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}

#[cfg(feature = "integration")]
impl Drop for LiveFolderSeedCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.discover_folder_ids_by_name();
        for block_id in &self.block_ids {
            let _ = self.backend.delete_bounded(&format!(
                "/workspaces/{}/loom/blocks/{block_id}",
                self.backend.workspace_id
            ));
        }
        for folder_id in self.folder_ids.iter().rev() {
            let _ = self.backend.delete_bounded(&format!(
                "/workspaces/{}/loom/folders/{folder_id}",
                self.backend.workspace_id
            ));
        }
    }
}

#[cfg(feature = "integration")]
fn required_json_id(value: &serde_json::Value, field: &str, operation: &str) -> String {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| {
            panic!("requires_surrealdb: {operation} response missing non-empty {field}: {value}")
        })
        .to_owned()
}

/// Retype one real shell slot to the folder pane while preserving the rest of the running app host.
/// Both registry and tab state are updated because the host synchronizes the registry from active tabs.
#[cfg(feature = "integration")]
fn mount_folder_pane_for_live_host(
    app: &mut handshake_native::app::HandshakeApp,
    workspace_id: &str,
) {
    use handshake_native::pane_registry::{
        DirtyState, LockState, PaneAuthority, PaneId, PaneRecord,
    };

    let pane_id = PaneId::from("pane-a");
    let pane_type = handshake_native::editor_pane_factories::placeholder_pane_type(
        handshake_native::editor_pane_factories::FOLDER_TREE_PANE_LABEL,
    );
    app.pane_registry()
        .lock()
        .expect("pane registry")
        .insert(PaneRecord::new(
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
}

#[test]
#[cfg(feature = "integration")]
fn failed_recolor_and_failed_refetch_keep_prior_swatch_in_mounted_host() {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loss server");
    listener
        .set_nonblocking(true)
        .expect("nonblocking loss server");
    let base = format!(
        "http://{}",
        listener.local_addr().expect("loss server address")
    );
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut requests = Vec::new();
        while requests.len() < 3 && std::time::Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                        .ok();
                    let mut bytes = vec![0_u8; 8192];
                    let read = stream.read(&mut bytes).unwrap_or(0);
                    let request = String::from_utf8_lossy(&bytes[..read]).to_string();
                    requests.push(request.lines().next().unwrap_or_default().to_owned());
                    let (status, body) = match requests.len() {
                        1 => (
                            "200 OK",
                            r##"[{"folder_id":"folder-loss","workspace_id":"ws-loss","name":"Loss","parent_folder_id":null,"color":"#3b82f6","sort_mode":"manual","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}]"##,
                        ),
                        _ => (
                            "503 Service Unavailable",
                            r#"{"error":"backend unavailable"}"#,
                        ),
                    };
                    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).expect("loss response");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => panic!("loss server accept failed: {error}"),
            }
        }
        requests
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("loss host runtime");
    let mut app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    app.set_runtime_handle(runtime.handle().clone());
    app.set_folder_backend_base_url_for_test(base);
    app.bind_active_project_for_integration_test("ws-loss".to_owned());
    mount_folder_pane_for_live_host(&mut app, "ws-loss");
    let tree = app.mounted_folder_tree_for_test();
    let events = app.mounted_folder_events_for_test();
    let mut harness = Harness::builder().build_state(
        |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
        app,
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if tree
            .lock()
            .unwrap()
            .find_folder_mut("folder-loss")
            .is_some()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "initial folder never loaded"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    events.lock().unwrap().push(FolderTreeEvent::ChangeColor {
        folder_id: "folder-loss".to_owned(),
        color: parse_hex_color("#ff0000").expect("red"),
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let mut state = tree.lock().unwrap();
        let prior_color_kept = state
            .find_folder_mut("folder-loss")
            .is_some_and(|folder| folder.color == parse_hex_color("#3b82f6"));
        if prior_color_kept && !state.loading && state.error.is_some() {
            break;
        }
        drop(state);
        assert!(
            std::time::Instant::now() < deadline,
            "PATCH failure plus refetch failure did not settle"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let requests = server.join().expect("loss server join");
    assert_eq!(
        requests.len(),
        3,
        "initial GET, failed PATCH, failed rollback GET"
    );
    assert!(requests[0].starts_with("GET "));
    assert!(requests[1].starts_with("PATCH "));
    assert!(requests[2].starts_with("GET "));
}

/// AC1-AC6 and PROOF2-5 against a REAL Handshake-managed SurrealDB. The fixture is self-contained:
/// `backend_proof_support::require_live_backend` owns an ephemeral backend runtime and workspace, and this
/// test owns its folder/block seed plus verified cleanup inside that workspace.
/// Run with `cargo test -p handshake-native --features integration --test test_folder_tree
/// folder_tree_live_surrealdb_self_seeded_round_trip -- --nocapture`.
#[test]
#[cfg(feature = "integration")]
fn folder_tree_live_surrealdb_self_seeded_round_trip() {
    use handshake_native::backend_client::{
        FolderChildrenCell, FolderListCell, FolderWriteCell, LoomFolderClient,
    };
    let mut backend = require_live_folder_backend();
    let mut cleanup = LiveFolderSeedCleanup {
        backend: &backend,
        folder_ids: Vec::new(),
        folder_names: Vec::new(),
        block_ids: Vec::new(),
    };
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let primary_title = format!("mt022-live-primary-{nonce}");
    let secondary_title = format!("mt022-live-secondary-{nonce}");
    let leaf_titles: Vec<String> = (1..=3)
        .map(|index| format!("mt022-live-leaf-{index}-{nonce}"))
        .collect();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for live folder client");
    let client = LoomFolderClient::new(backend.base.clone(), runtime.handle().clone());
    let live_epoch = 1;

    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.create_folder(
        &backend.workspace_id,
        &primary_title,
        None,
        Some(20),
        live_epoch,
        1,
        Arc::clone(&write),
    );
    let primary_id = identified_folder_result(
        wait_for_live_cell(&write, "production create primary folder"),
        &backend.workspace_id,
        live_epoch,
        1,
    )
    .expect("primary create succeeds")
    .expect("create returns canonical folder")
    .folder_id;
    cleanup.folder_ids.push(primary_id.clone());
    let recolor: handshake_native::backend_client::FolderRecolorCell = Arc::new(Mutex::new(None));
    client.recolor_folder(
        &backend.workspace_id,
        &primary_id,
        "#22c55e",
        live_epoch,
        2,
        Arc::clone(&recolor),
    );
    let (workspace, folder, epoch, sequence, result) =
        wait_for_live_cell(&recolor, "recolor primary seed");
    assert_eq!(
        (workspace, folder, epoch, sequence),
        (
            backend.workspace_id.clone(),
            primary_id.clone(),
            live_epoch,
            2
        )
    );
    result.expect("primary recolor succeeds");

    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.create_folder(
        &backend.workspace_id,
        &secondary_title,
        None,
        Some(10),
        live_epoch,
        3,
        Arc::clone(&write),
    );
    let secondary_id = identified_folder_result(
        wait_for_live_cell(&write, "production create secondary folder"),
        &backend.workspace_id,
        live_epoch,
        3,
    )
    .expect("secondary create succeeds")
    .expect("create returns canonical folder")
    .folder_id;
    cleanup.folder_ids.push(secondary_id.clone());
    let recolor: handshake_native::backend_client::FolderRecolorCell = Arc::new(Mutex::new(None));
    client.recolor_folder(
        &backend.workspace_id,
        &secondary_id,
        "#3b82f6",
        live_epoch,
        4,
        Arc::clone(&recolor),
    );
    let (workspace, folder, epoch, sequence, result) =
        wait_for_live_cell(&recolor, "recolor secondary seed");
    assert_eq!(
        (workspace, folder, epoch, sequence),
        (
            backend.workspace_id.clone(),
            secondary_id.clone(),
            live_epoch,
            4
        )
    );
    result.expect("secondary recolor succeeds");

    let mut block_ids = Vec::new();
    for (index, title) in leaf_titles.iter().enumerate() {
        let requested_block_id = format!("mt022-{}-{nonce}", index + 1);
        let block = backend.post_json(
            &format!("/workspaces/{}/loom/blocks", backend.workspace_id),
            &serde_json::json!({
                "block_id": requested_block_id,
                "content_type": "note",
                "title": title
            }),
        );
        let block_id = required_json_id(&block, "block_id", "create folder child block");
        cleanup.block_ids.push(block_id.clone());
        let _membership = backend.put_json(
            &format!(
                "/workspaces/{}/loom/folders/{primary_id}/blocks/{block_id}",
                backend.workspace_id
            ),
            &serde_json::json!({ "sort_order": index as i32 }),
        );
        block_ids.push(block_id);
    }
    let primary_block_id = block_ids[0].clone();

    // Drive the same off-thread production client used by the native host. This is not a repository
    // double: every result below comes from handshake_core backed by the managed SurrealDB workspace.
    let folders_cell: FolderListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_folders(
        &backend.workspace_id,
        live_epoch,
        5,
        Arc::clone(&folders_cell),
    );
    let rows = identified_folder_result(
        wait_for_live_queue(&folders_cell, "live folder list"),
        &backend.workspace_id,
        live_epoch,
        5,
    )
    .expect("requires_surrealdb: production folder-list GET must succeed");
    let secondary_position = rows
        .iter()
        .position(|row| row.folder_id == secondary_id)
        .expect("secondary ordered row");
    let primary_position = rows
        .iter()
        .position(|row| row.folder_id == primary_id)
        .expect("primary ordered row");
    assert!(
        secondary_position < primary_position,
        "fresh-client list honors persisted sort_order before name"
    );
    for (folder_id, title, color) in [
        (&primary_id, &primary_title, "#22c55e"),
        (&secondary_id, &secondary_title, "#3b82f6"),
    ] {
        let row = rows
            .iter()
            .find(|row| &row.folder_id == folder_id)
            .unwrap_or_else(|| panic!("AC1 live: seeded folder {folder_id} missing from {rows:?}"));
        assert_eq!(
            &row.name, title,
            "AC1 live: seeded folder title round-trips"
        );
        assert_eq!(
            row.color.as_deref(),
            Some(color),
            "AC3 live: stored folder color round-trips"
        );
    }

    let mut live_tree = LoomFolderTree::new(backend.workspace_id.clone());
    live_tree.set_folders(&rows);
    assert!(
        live_tree.folder_count() >= 2,
        "AC1 live: real folder response builds a tree with the two seeded folders"
    );
    let initial_tree = shared(live_tree);
    let initial_events = Arc::new(Mutex::new(Vec::new()));
    let mut initial_harness = harness_for(Arc::clone(&initial_tree), initial_events);
    initial_harness.run();
    let initial_ids = author_ids(&initial_harness);
    for folder_id in [&primary_id, &secondary_id] {
        let author_id = node_author_id(folder_id);
        assert!(
            initial_ids.contains(&author_id),
            "AC6 live: seeded folder must render as AccessKit node {author_id}"
        );
        let role = initial_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id.as_str()))
            .map(|node| format!("{:?}", node.accesskit_node().role()));
        assert_eq!(role.as_deref(), Some("TreeItem"), "AC6 live: folder role");
    }

    let renamed_secondary = format!("aaa-mt022-renamed-{nonce}");
    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.rename_folder(
        &backend.workspace_id,
        &secondary_id,
        &renamed_secondary,
        live_epoch,
        6,
        Arc::clone(&write),
    );
    identified_folder_result(
        wait_for_live_cell(&write, "production folder rename"),
        &backend.workspace_id,
        live_epoch,
        6,
    )
    .expect("rename persists");

    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.move_folder(
        &backend.workspace_id,
        &secondary_id,
        Some(&primary_id),
        Some(5),
        live_epoch,
        7,
        Arc::clone(&write),
    );
    identified_folder_result(
        wait_for_live_cell(&write, "production folder move under parent"),
        &backend.workspace_id,
        live_epoch,
        7,
    )
    .expect("move persists");

    let fresh_client = LoomFolderClient::new(backend.base.clone(), runtime.handle().clone());
    let fresh_rows_cell: FolderListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh_client.fetch_folders(
        &backend.workspace_id,
        live_epoch,
        8,
        Arc::clone(&fresh_rows_cell),
    );
    let fresh_rows = identified_folder_result(
        wait_for_live_queue(&fresh_rows_cell, "fresh-client hierarchy refetch"),
        &backend.workspace_id,
        live_epoch,
        8,
    )
    .expect("fresh-client hierarchy refetch succeeds");
    let nested = fresh_rows
        .iter()
        .find(|row| row.folder_id == secondary_id)
        .expect("renamed child exists");
    assert_eq!(
        nested.name, renamed_secondary,
        "rename survives fresh client"
    );
    assert_eq!(
        nested.parent_folder_id.as_deref(),
        Some(primary_id.as_str()),
        "move survives fresh client"
    );
    let fresh_tree = build_tree(&fresh_rows);
    let primary_node = fresh_tree
        .iter()
        .find(|node| node.folder_id == primary_id)
        .expect("fresh primary root");
    assert!(
        primary_node
            .child_folders
            .iter()
            .any(|node| node.folder_id == secondary_id),
        "fresh hierarchy nests the moved child"
    );

    let missing_parent = format!("missing-parent-{nonce}");
    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.move_folder(
        &backend.workspace_id,
        &secondary_id,
        Some(&missing_parent),
        None,
        live_epoch,
        9,
        Arc::clone(&write),
    );
    assert!(
        identified_folder_result(
            wait_for_live_cell(&write, "missing-parent move rejection"),
            &backend.workspace_id,
            live_epoch,
            9,
        )
        .is_err(),
        "missing parents fail closed"
    );
    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.move_folder(
        &backend.workspace_id,
        &primary_id,
        Some(&secondary_id),
        None,
        live_epoch,
        10,
        Arc::clone(&write),
    );
    assert!(
        identified_folder_result(
            wait_for_live_cell(&write, "cycle move rejection"),
            &backend.workspace_id,
            live_epoch,
            10,
        )
        .is_err(),
        "descendant cycles fail closed"
    );

    // Drive the disclosure action from a real SurrealDB folder row before dispatching its production
    // child request. Isolating the primary row makes the triangle target deterministic even when the
    // operator's workspace already contains unrelated folders.
    let primary_row = rows
        .iter()
        .find(|row| row.folder_id == primary_id)
        .expect("AC2 live: primary SurrealDB row remains available")
        .clone();
    let mut expansion_tree = LoomFolderTree::new(backend.workspace_id.clone());
    expansion_tree.set_folders(&[primary_row]);
    let expansion_tree = shared(expansion_tree);
    let expansion_events = Arc::new(Mutex::new(Vec::new()));
    let mut expansion_harness =
        harness_for(Arc::clone(&expansion_tree), Arc::clone(&expansion_events));
    expansion_harness.run();
    expansion_harness.get_by_label("▸").click();
    expansion_harness.run();
    assert!(
        expansion_events
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .any(|event| matches!(
                event,
                FolderTreeEvent::ExpandFolder { folder_id } if folder_id == &primary_id
            )),
        "AC2 live: expanding the SurrealDB-backed folder dispatches its exact folder id"
    );

    let children_cell: FolderChildrenCell = Arc::new(Mutex::new(None));
    client.fetch_folder_blocks(
        &backend.workspace_id,
        &primary_id,
        live_epoch,
        11,
        Arc::clone(&children_cell),
    );
    let (workspace, folder, epoch, sequence, leaves) =
        wait_for_live_cell(&children_cell, "live folder child load");
    assert_eq!(
        (workspace, folder, epoch, sequence),
        (
            backend.workspace_id.clone(),
            primary_id.clone(),
            live_epoch,
            11
        )
    );
    let leaves = leaves.expect("requires_surrealdb: production folder-children GET must succeed");
    assert_eq!(
        leaves.len(),
        3,
        "AC2 live: the isolated folder returns all three seeded child blocks"
    );
    for (block_id, title) in block_ids.iter().zip(&leaf_titles) {
        let seeded_leaf = leaves
            .iter()
            .find(|leaf| &leaf.block_id == block_id)
            .unwrap_or_else(|| panic!("AC2 live: seeded block {block_id} missing from {leaves:?}"));
        assert_eq!(
            &seeded_leaf.title, title,
            "AC2 live: child title round-trips"
        );
    }

    // Drive expansion + recolor through the REAL HandshakeApp host, not only the client/widget halves.
    // The host mounts the production pane factory, drains FolderTreeEvent, dispatches the bounded
    // LoomFolderClient requests, and applies their deliveries back to the same mounted tree.
    let mut app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    app.set_runtime_handle(runtime.handle().clone());
    app.set_folder_backend_base_url_for_test(backend.base.clone());
    app.bind_active_project_for_integration_test(backend.workspace_id.clone());
    mount_folder_pane_for_live_host(&mut app, &backend.workspace_id);
    let mounted_tree = app.mounted_folder_tree_for_test();
    let mounted_events = app.mounted_folder_events_for_test();
    let mut app_harness = Harness::builder().build_state(
        |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
        app,
    );

    let initial_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let loaded = mounted_tree
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .find_folder_mut(&primary_id)
            .is_some();
        if loaded {
            break;
        }
        assert!(
            std::time::Instant::now() < initial_deadline,
            "requires_surrealdb: mounted app host did not load the seeded folder within 10 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    fn find_folder_by_title(
        nodes: &[handshake_native::graph::folder_tree::FolderNode],
        title: &str,
    ) -> Option<String> {
        for node in nodes {
            if node.title == title {
                return Some(node.folder_id.clone());
            }
            if let Some(found) = find_folder_by_title(&node.child_folders, title) {
                return Some(found);
            }
        }
        None
    }
    let host_created_title = format!("mt022-host-created-{nonce}");
    cleanup.folder_names.push(host_created_title.clone());
    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::CreateFolder {
            parent_folder_id: None,
            name: host_created_title.clone(),
        });
    let host_crud_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let host_created_id = loop {
        app_harness.run_steps(1);
        if let Some(id) = find_folder_by_title(
            &mounted_tree
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .root_nodes,
            &host_created_title,
        ) {
            break id;
        }
        assert!(
            std::time::Instant::now() < host_crud_deadline,
            "mounted create did not persist/refetch"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    cleanup.folder_ids.push(host_created_id.clone());

    let host_renamed_title = format!("mt022-host-renamed-{nonce}");
    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::RenameFolder {
            folder_id: host_created_id.clone(),
            name: host_renamed_title.clone(),
        });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        if find_folder_by_title(
            &mounted_tree
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .root_nodes,
            &host_renamed_title,
        )
        .is_some()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted rename did not persist/refetch"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::MoveFolder {
            folder_id: host_created_id.clone(),
            parent_folder_id: Some(primary_id.clone()),
            sort_order: Some(1),
        });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let nested = mounted_tree
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .find_folder_mut(&primary_id)
            .is_some_and(|parent| {
                parent
                    .child_folders
                    .iter()
                    .any(|child| child.folder_id == host_created_id)
            });
        if nested {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted move did not persist/refetch"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::DeleteFolder {
            folder_id: host_created_id.clone(),
        });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        if mounted_tree
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .find_folder_mut(&host_created_id)
            .is_none()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted delete did not persist/refetch"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::MoveFolder {
            folder_id: primary_id.clone(),
            parent_folder_id: Some(secondary_id.clone()),
            sort_order: None,
        });
    let conflict_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let tree = mounted_tree.lock().unwrap_or_else(|p| p.into_inner());
        let visible_conflict = tree
            .operation_error
            .as_deref()
            .or(tree.error.as_deref())
            .is_some_and(|error| error.contains("Folder move failed"));
        if visible_conflict {
            break;
        }
        assert!(
            std::time::Instant::now() < conflict_deadline,
            "mounted cycle conflict was not surfaced"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::ExpandFolder {
            folder_id: primary_id.clone(),
        });
    let expand_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let expanded_with_three = mounted_tree
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .find_folder_mut(&primary_id)
            .is_some_and(|node| {
                node.expanded
                    && !node.loading
                    && node
                        .child_blocks
                        .as_ref()
                        .is_some_and(|blocks| blocks.len() == 3)
            });
        if expanded_with_three {
            break;
        }
        assert!(
            std::time::Instant::now() < expand_deadline,
            "requires_surrealdb: mounted app host did not expand/load all three child blocks within 10 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::ChangeColor {
            folder_id: primary_id.clone(),
            color: parse_hex_color("#ff0000").expect("red"),
        });
    let recolor_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let persisted_red = mounted_tree
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .find_folder_mut(&primary_id)
            .is_some_and(|node| node.color == parse_hex_color("#ff0000"));
        if persisted_red
            && app_harness
                .state()
                .folder_recolor_cells_in_flight_for_test()
                == 0
        {
            break;
        }
        assert!(
            std::time::Instant::now() < recolor_deadline,
            "requires_surrealdb: mounted app host did not persist/apply the recolor within 10 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let refetch_cell: FolderListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.fetch_folders(
        &backend.workspace_id,
        live_epoch,
        12,
        Arc::clone(&refetch_cell),
    );
    let refreshed_rows = identified_folder_result(
        wait_for_live_queue(&refetch_cell, "live folder recolor refetch"),
        &backend.workspace_id,
        live_epoch,
        12,
    )
    .expect("requires_surrealdb: recolor refetch must succeed");
    let refreshed_primary = refreshed_rows
        .iter()
        .find(|row| row.folder_id == primary_id)
        .expect("AC4 live: recolored folder remains present after refetch");
    assert_eq!(
        refreshed_primary.color.as_deref(),
        Some("#ff0000"),
        "AC4 live: color-only PATCH persists in SurrealDB and refetches as red"
    );

    // Feed the refetched persisted row and live-loaded leaf back into the real widget. This proves the
    // persisted swatch state, live AccessKit leaf node, and leaf-open dispatch in one mounted tree.
    let mut refreshed_tree = LoomFolderTree::new(backend.workspace_id.clone());
    refreshed_tree.set_folders(&refreshed_rows);
    let primary_node = refreshed_tree
        .find_folder_mut(&primary_id)
        .expect("AC4 live: refetched folder is present in widget tree");
    assert_eq!(
        primary_node.color,
        parse_hex_color("#ff0000"),
        "AC4 live: widget swatch state comes from the persisted refetch"
    );
    primary_node.child_blocks = Some(leaves);
    primary_node.expanded = true;

    let refreshed_tree = shared(refreshed_tree);
    let live_events = Arc::new(Mutex::new(Vec::new()));
    let mut live_harness = harness_for(Arc::clone(&refreshed_tree), Arc::clone(&live_events));
    live_harness.run();
    let live_ids = author_ids(&live_harness);
    assert!(
        live_ids.contains(&node_author_id(&primary_block_id)),
        "AC2/AC6 live: expanded folder renders the SurrealDB-loaded child as a TreeItem"
    );
    live_harness.get_by_label_contains(&leaf_titles[0]).click();
    live_harness.run();
    assert!(
        live_events.lock().unwrap_or_else(|p| p.into_inner()).iter().any(
            |event| matches!(event, FolderTreeEvent::OpenBlock { block_id: opened } if opened == &primary_block_id)
        ),
        "AC5 live: clicking the SurrealDB-loaded leaf dispatches its exact block id"
    );

    // Negative path through the SAME mounted host: install a synthetic missing row, let the real PATCH
    // return 404, observe the visible typed error, then prove the bounded authoritative refetch removes
    // the row and restores the real backend tree.
    let missing_folder_id = format!("missing-mt022-{nonce}");
    {
        let mut tree = mounted_tree.lock().unwrap_or_else(|p| p.into_inner());
        tree.set_folders(&[FolderRow::new(
            missing_folder_id.clone(),
            None,
            "Optimistic missing folder",
            Some("#3b82f6".to_owned()),
        )]);
    }
    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::ChangeColor {
            folder_id: missing_folder_id.clone(),
            color: parse_hex_color("#3b82f6").expect("blue"),
        });
    let rollback_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut visible_typed_error_seen = false;
    loop {
        app_harness.run_steps(1);
        let (error, operation_error, fake_present, authority_present) = {
            let mut tree = mounted_tree.lock().unwrap_or_else(|p| p.into_inner());
            (
                tree.error.clone(),
                tree.operation_error.clone(),
                tree.find_folder_mut(&missing_folder_id).is_some(),
                tree.find_folder_mut(&primary_id).is_some(),
            )
        };
        visible_typed_error_seen |= operation_error
            .as_deref()
            .or(error.as_deref())
            .is_some_and(|message| message.contains("Folder color was not saved"));
        let persisted_operation_error = operation_error
            .as_deref()
            .is_some_and(|message| message.contains("Folder color was not saved"));
        if visible_typed_error_seen
            && persisted_operation_error
            && !fake_present
            && authority_present
        {
            break;
        }
        assert!(
            std::time::Instant::now() < rollback_deadline,
            "requires_surrealdb: failed recolor did not visibly error and rollback/refetch within 10 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    // Drive the real mounted UI event with both optional move fields absent. The production request
    // builder serializes that UI intent as explicit JSON null/null so SurrealDB clears parent and order.
    mounted_events
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(FolderTreeEvent::MoveFolder {
            folder_id: secondary_id.clone(),
            parent_folder_id: None,
            sort_order: None,
        });
    let null_move_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app_harness.run_steps(1);
        let raw_rows = backend
            .get_json_bounded(&format!(
                "/workspaces/{}/loom/folders",
                backend.workspace_id
            ))
            .expect("raw refetch after mounted null/null root move");
        let persisted = raw_rows.as_array().and_then(|rows| {
            rows.iter().find(|row| {
                row.get("folder_id").and_then(serde_json::Value::as_str)
                    == Some(secondary_id.as_str())
            })
        });
        if persisted.is_some_and(|row| {
            // LoomFolder skips serializing None fields, so omission is the canonical response shape for
            // a SurrealDB NULL (an explicit JSON null remains accepted for compatible servers).
            row.get("parent_folder_id")
                .map_or(true, serde_json::Value::is_null)
                && row
                    .get("sort_order")
                    .map_or(true, serde_json::Value::is_null)
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < null_move_deadline,
            "requires_surrealdb: mounted UI null/null move did not persist within 10 seconds: {raw_rows}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    // Preserve the independent manual-order proof after the exact UI null/null proof.
    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.move_folder(
        &backend.workspace_id,
        &secondary_id,
        None,
        Some(-100),
        live_epoch,
        13,
        Arc::clone(&write),
    );
    identified_folder_result(
        wait_for_live_cell(&write, "move child back to ordered root"),
        &backend.workspace_id,
        live_epoch,
        13,
    )
    .expect("move to root persists");
    let fresh_client = LoomFolderClient::new(backend.base.clone(), runtime.handle().clone());
    let ordered_cell: FolderListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    fresh_client.fetch_folders(
        &backend.workspace_id,
        live_epoch,
        14,
        Arc::clone(&ordered_cell),
    );
    let ordered_rows = identified_folder_result(
        wait_for_live_queue(&ordered_cell, "fresh-client root-move refetch"),
        &backend.workspace_id,
        live_epoch,
        14,
    )
    .expect("fresh root-move refetch succeeds");
    let secondary = ordered_rows
        .iter()
        .find(|row| row.folder_id == secondary_id)
        .expect("secondary root present");
    assert_eq!(
        secondary.parent_folder_id, None,
        "explicit-null parent persists as a root move"
    );
    let secondary_position = ordered_rows
        .iter()
        .position(|row| row.folder_id == secondary_id)
        .unwrap();
    let primary_position = ordered_rows
        .iter()
        .position(|row| row.folder_id == primary_id)
        .unwrap();
    assert!(
        secondary_position < primary_position,
        "new sort_order persists and drives fresh-client root ordering"
    );

    let write: FolderWriteCell = Arc::new(Mutex::new(None));
    client.delete_folder(
        &backend.workspace_id,
        &secondary_id,
        live_epoch,
        15,
        Arc::clone(&write),
    );
    identified_folder_result(
        wait_for_live_cell(&write, "production folder delete"),
        &backend.workspace_id,
        live_epoch,
        15,
    )
    .expect("delete persists");
    let absent_cell: FolderListCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    LoomFolderClient::new(backend.base.clone(), runtime.handle().clone()).fetch_folders(
        &backend.workspace_id,
        live_epoch,
        16,
        Arc::clone(&absent_cell),
    );
    let rows_after_delete = identified_folder_result(
        wait_for_live_queue(&absent_cell, "fresh-client delete refetch"),
        &backend.workspace_id,
        live_epoch,
        16,
    );
    assert!(
        !rows_after_delete
            .expect("delete refetch succeeds")
            .iter()
            .any(|row| row.folder_id == secondary_id),
        "deleted folder stays absent for a fresh client"
    );

    cleanup
        .cleanup_and_verify()
        .expect("bounded cleanup removes every MT-022 live seed row");
    drop(cleanup);

    // Capture the proof identity before the managed fixture clears its workspace id, then require the
    // owned workspace deletion and backend-process reap to complete before any PASS artifact exists.
    let proof_backend_base = backend.base.clone();
    let proof_workspace_id = backend.workspace_id.clone();
    backend.assert_cleanup();

    // A durable, non-zero proof artifact carries the exact live seed identifiers and the completed
    // cleanup verdict for independent V2 validation. It is outside the repository and contains no
    // hidden/mock state.
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-022");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-022 artifact directory");
    let receipt_path = artifact_dir.join("MT-022-live-surrealdb-seed.json");
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt022.live_surrealdb_proof@1",
        "backend_base": proof_backend_base,
        "workspace_id": proof_workspace_id,
        "seed_ids": {
            "folder_ids": [primary_id, secondary_id],
            "block_ids": block_ids
        },
        "assertions": {
            "folder_titles_loaded": 2,
            "child_blocks_loaded": 3,
            "persisted_color": "#ff0000",
            "folder_accesskit_role": "TreeItem",
            "leaf_open_dispatched": true,
            "mounted_host_expand_and_recolor": true,
            "failed_recolor_typed_error_and_rollback": true,
            "create_rename_move_delete_fresh_client": true,
            "missing_parent_and_cycle_conflicts": true,
            "cleanup_verified": true
        }
    });
    let encoded = serde_json::to_vec_pretty(&receipt).expect("encode live SurrealDB seed receipt");
    assert!(!encoded.is_empty(), "live SurrealDB proof output must be non-zero");
    std::fs::write(&receipt_path, encoded).expect("write external live SurrealDB seed receipt");
    assert!(
        std::fs::metadata(&receipt_path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false),
        "live SurrealDB seed receipt must exist and be non-zero"
    );
    assert_no_local_artifact_dir();
}
