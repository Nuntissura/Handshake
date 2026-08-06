//! WP-KERNEL-012 MT-113 — PT-113-3: the mounted canonical Argus proof that a bookmark REMOVE
//! terminalizes with a REALISTIC LONG query. This is the exact regression MT-029 measured and could
//! not close inside its own scope.
//!
//! ## What made this necessary
//!
//! `SearchBookmark::stable_id()` hex-encodes the search query and `bookmark_remove_author_id()` then
//! hex-encodes THAT result — a 4x expansion of the original user bytes. The canonical
//! `handshake.click-completion/v1` contract bounds an acknowledgement's `pending_target` at 256 bytes,
//! so a query of just 23 characters overran it, `serialize_observer_click_state` returned `None`, the
//! bookmark completion observer carried NO value, and the Remove action could never terminalize: a
//! permanently `indeterminate` receipt with no diagnostic emitted anywhere. MT-029 therefore proved the
//! SHORT-query case and routed the long-query case here.
//!
//! ## What this file proves
//!
//! One production-mounted `HandshakeApp`, driven only through the real localhost Argus transport, saves
//! a bookmark from a long query and then REMOVES it. Every canonical action is bound to a fresh
//! action-specific terminal predicate, the run is closed with
//! `finish_require_no_indeterminate()`, and a fresh backend GET independently confirms the removal
//! persisted. The composition-level bound itself (PT-113-1), the byte-identity guarantee, injectivity,
//! resolvability, the typed completion-unavailable marker and the authoring-time `debug_assert!` are
//! proven in the library unit tests (`handshake_native::find_in_files::tests::mt113_*` and
//! `handshake_native::mcp::action::tests::mt113_*`) — pure, no backend, no GPU.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! Nothing is written inside the repo; the managed fixtures publish only under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/` root.

#![allow(clippy::items_after_statements)]

#[cfg(feature = "integration")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[cfg(feature = "integration")]
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

/// The realistic long query at the heart of the regression. Its LEGACY (pre-MT-113) Remove route was
/// far past the 256-byte canonical completion-target budget, which is asserted below so this proof can
/// never silently degrade into a short-query re-run of MT-029.
#[cfg(feature = "integration")]
const LONG_BOOKMARK_QUERY_STEM: &str = "authentication middleware refactor across the workspace";

#[cfg(feature = "integration")]
fn legacy_remove_route_len(bookmark_id: &str) -> usize {
    // The exact pre-MT-113 composition: prefix + lowercase bytewise UTF-8 hex of the whole stable id.
    "find-in-files.bookmark-remove.".len() + bookmark_id.len() * 2
}

/// PT-113-3 / AC-113-5. Bookmark REMOVE — the control that concretely failed — produces a TERMINAL,
/// non-`indeterminate` canonical Argus receipt for a realistic long query, and the removal is
/// independently visible to a fresh backend GET.
#[test]
#[cfg(feature = "integration")]
fn mt113_mounted_bookmark_remove_terminalizes_with_a_realistic_long_query() {
    use std::sync::{Arc, Mutex};

    use canonical_argus_driver::{
        json_has_author_id, json_node_by_author_id, CanonicalArgusDriver,
    };
    use handshake_native::backend_client::{
        BookmarkStateCell, FindInFilesOperation, FindInFilesStamp, WorkspaceSearchClient,
    };
    use handshake_native::find_in_files::{
        bookmark_remove_author_id, bookmark_restore_author_id, parse_bookmark_state, KindFilter,
        SearchBookmark, BOOKMARK_COMPLETION_AUTHOR_ID, BOOKMARK_STATUS_AUTHOR_ID,
        MAX_COMPLETION_TARGET_AUTHOR_BYTES, QUERY_AUTHOR_ID, SAVE_BOOKMARK_AUTHOR_ID,
    };
    use screenshot_harness::ScreenshotHarness as Harness;

    interconnect_support::assert_no_local_artifact_dir();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build MT-113 proof runtime");
    let mut live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt113-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos()
    );
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    // The fixture only owns the workspace IT creates; this proof creates its own, so it owns the
    // matching teardown. A Drop guard keeps the database free of residue even on an assertion unwind.
    struct OwnedWorkspace<'a> {
        backend: &'a interconnect_support::LiveBackend,
        workspace_id: String,
        cleaned: bool,
    }
    impl OwnedWorkspace<'_> {
        fn clean(&mut self) {
            let status = self.backend.delete_workspace(&self.workspace_id);
            assert!(
                matches!(status, 200 | 202 | 204 | 404),
                "owned workspace cleanup returned HTTP {status}"
            );
            self.cleaned = true;
        }
    }
    impl Drop for OwnedWorkspace<'_> {
        fn drop(&mut self) {
            if !self.cleaned {
                let _ = self.backend.delete_workspace(&self.workspace_id);
            }
        }
    }

    // A REALISTIC operator query, made unique per run so concurrent proofs cannot alias each other.
    let bookmark_query = format!("{LONG_BOOKMARK_QUERY_STEM} {unique}");
    let mounted_bookmark = {
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query: bookmark_query.clone(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-08-05T00:00:00Z".to_owned(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        bookmark
    };
    let remove_author_id = bookmark_remove_author_id(&mounted_bookmark.id);
    let restore_author_id = bookmark_restore_author_id(&mounted_bookmark.id);
    let legacy_len = legacy_remove_route_len(&mounted_bookmark.id);
    assert!(
        legacy_len > MAX_COMPLETION_TARGET_AUTHOR_BYTES,
        "this proof is only meaningful for a query whose LEGACY Remove route overran the {MAX_COMPLETION_TARGET_AUTHOR_BYTES}-byte budget; legacy route would have been {legacy_len} bytes"
    );
    assert!(
        remove_author_id.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES
            && restore_author_id.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
        "MT-113: bounded routes must fit the canonical budget: remove={} restore={}",
        remove_author_id.len(),
        restore_author_id.len()
    );

    let mut owned_workspace = OwnedWorkspace {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };

    let mut production_app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    production_app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    production_app.bind_active_project_for_integration_test(workspace_id.clone());
    assert!(production_app.dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    let mut argus = CanonicalArgusDriver::bind(&production_app, "mt113-author-id-budget");
    let mut ui_harness = Harness::builder()
        .proof_mt_id("MT-113")
        .with_size(egui::vec2(2560.0, 1200.0))
        .wgpu()
        .build_state(
            |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
            production_app,
        );
    ui_harness.run_steps(2);

    let initial_tree = argus.inspect(&mut ui_harness);
    assert!(
        json_has_author_id(&initial_tree, QUERY_AUTHOR_ID)
            && json_has_author_id(&initial_tree, SAVE_BOOKMARK_AUTHOR_ID),
        "the mounted Find-in-Files surface must expose the query field and Bookmark Search control"
    );

    // 1. Type the LONG query through the real canonical transport.
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &bookmark_query);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mt113-long-query-visible",
        serde_json::json!({
            "query_chars": bookmark_query.chars().count(),
            "legacy_remove_route_bytes": legacy_len,
            "bounded_remove_route_bytes": remove_author_id.len(),
        }),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(bookmark_query.as_str())
        },
    );

    // 2. Save the bookmark through the production control; the persisted PUT is real.
    argus.click_expect_applied_and_reinspect(&mut ui_harness, SAVE_BOOKMARK_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let ids = interconnect_support::author_ids(&ui_harness);
        if ids.contains(&remove_author_id) && ids.contains(&restore_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mt113-long-query-bookmark-saved",
        serde_json::json!({
            "bookmark_id": mounted_bookmark.id.clone(),
            "remove_author_id": remove_author_id.clone(),
        }),
        |tree| {
            json_has_author_id(tree, &remove_author_id)
                && json_has_author_id(tree, &restore_author_id)
                && json_node_by_author_id(tree, BOOKMARK_STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| {
                        value
                            .split_once("receipt: ")
                            .map(|(_, tail)| tail.trim())
                            .is_some_and(|receipt| !receipt.is_empty())
                    })
        },
    );

    // The bookmark observer is SHARED by save and remove (one persisted PUT). Dispatching the next
    // observer-backed action while the previous one is still settling makes the pre-dispatch
    // declaration and the post-render frame disagree, which the MCP boundary correctly refuses as
    // target-identity drift. Wait for the observer to actually return to `ready` — this is a real
    // precondition of an observer-backed action, not a relaxation of the acknowledgement contract.
    let mut settled_observer = None;
    let mut previous_row: Option<String> = None;
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let observer =
            interconnect_support::author_node_value(&ui_harness, BOOKMARK_COMPLETION_AUTHOR_ID);
        let row = interconnect_support::author_node_value(&ui_harness, &remove_author_id);
        let observer_settled = observer
            .as_deref()
            .is_some_and(|value| !value.contains("\"state\":\"pending\""));
        let row_stable = row.is_some() && row == previous_row;
        if observer_settled && row_stable {
            settled_observer = observer;
            break;
        }
        previous_row = row;
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let settled_observer = settled_observer.expect(
        "the shared bookmark completion observer must leave `pending` and the Remove row must hold a \
         stable declaration across consecutive frames before the next observer-backed action is \
         dispatched",
    );
    assert!(
        !settled_observer.contains("click-completion-unavailable"),
        "the observer must never publish the MT-113 unavailable marker for a bounded route: {settled_observer}"
    );

    // HBR-VIS: a validator must be able to SEE the long-query saved search mounted with its Remove
    // control before the action, and gone after it. On a headless host `render_proof_frame` records a
    // typed DEFERRED outcome instead of a frame; it never silently returns nothing.
    // CX-212E: the ONE MT-113 artifact folder under the external test root. `interconnect_support`'s
    // helper is MT-046-rooted, so this proof resolves its own subdir instead of nesting under another
    // MT's tree (no sibling artifact folders, nothing inside the repo).
    let frame_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("native crate must live below a worktree root")
        .join("Handshake_Artifacts")
        .join("handshake-test")
        .join("wp-kernel-012-mt-113");
    std::fs::create_dir_all(&frame_dir).expect("create MT-113 frame directory");
    let before_frame = ui_harness
        .render_proof_frame("MT-113 long-query bookmark row mounted before Remove")
        .map(|image| {
            let path = frame_dir.join("MT-113-long-query-bookmark-before-remove.png");
            image.save(&path).expect("save MT-113 before frame");
            (path, image)
        });

    // 3. THE REGRESSION: remove it. Before MT-113 this receipt was permanently `indeterminate`
    //    because the observer's `pending_target` could not be serialized at all.
    argus.click_expect_applied_and_reinspect(&mut ui_harness, &remove_author_id);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if !interconnect_support::author_ids(&ui_harness).contains(&remove_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mt113-long-query-bookmark-removed",
        serde_json::json!({"bookmark_id": mounted_bookmark.id.clone()}),
        |tree| {
            !json_has_author_id(tree, &remove_author_id)
                && !json_has_author_id(tree, &restore_author_id)
        },
    );

    let after_frame = ui_harness
        .render_proof_frame("MT-113 long-query bookmark row gone after Remove")
        .map(|image| {
            let path = frame_dir.join("MT-113-long-query-bookmark-after-remove.png");
            image.save(&path).expect("save MT-113 after frame");
            (path, image)
        });
    if let (Some((before_path, before)), Some((after_path, after))) = (&before_frame, &after_frame)
    {
        // A byte-identical before/after pair would mean the capture never observed the mutation.
        assert_ne!(
            before.as_raw(),
            after.as_raw(),
            "MT-113 before/after frames are byte-identical, so the captured pixels prove nothing: {} vs {}",
            before_path.display(),
            after_path.display()
        );
        println!(
            "MT-113 HBR-VIS frames: {} / {}",
            before_path.display(),
            after_path.display()
        );
    }

    // 4. A fresh, independent backend GET must see the removal — the UI verdict is never the proof.
    let search_client = WorkspaceSearchClient::new(&live.base, runtime.handle().clone());
    let absence_cell: BookmarkStateCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.load_bookmarks(
        &workspace_id,
        FindInFilesStamp {
            workspace_id: workspace_id.clone(),
            operation: FindInFilesOperation::BookmarkLoad,
            epoch: 1,
            sequence: 113,
        },
        Arc::clone(&absence_cell),
    );
    let mut absence_blob = None;
    for _ in 0..400 {
        if let Some(delivery) = absence_cell.lock().unwrap().pop_front() {
            let (blob, _, _) = delivery
                .outcome
                .expect("fresh backend bookmark GET succeeds");
            absence_blob = Some(blob);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let absence_state = parse_bookmark_state(
        &absence_blob.expect("fresh backend GET returns the workspace bookmark state"),
    )
    .expect("strict bookmark payload");
    assert!(
        absence_state
            .iter()
            .all(|saved| saved.id != mounted_bookmark.id),
        "the mounted long-query Remove must persist exact-bookmark absence"
    );

    // 5. Strict closure: NO canonical action in this run may have retained `indeterminate`.
    argus.finish_require_no_indeterminate();
    drop(ui_harness);
    owned_workspace.clean();
    drop(owned_workspace);
    live.assert_cleanup();
}
