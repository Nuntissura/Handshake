//! WP-KERNEL-012 E3 MT-024 remediation (FAIL_V4): canonical Argus inspect / safe-steer / re-observe
//! proof for the MOUNTED pins / favorites / backlinks / unlinked sidebar over REAL PostgreSQL.
//!
//! ## What FAIL_V4 said
//!
//! > "The mounted sidebar changes after the pin-removal click, but the required proof cannot
//! > distinguish a successful persisted removal from a failed request, stale refresh, or target
//! > disappearance. The action receipt is indeterminate, and the post-state itself shows a Pins
//! > error/Retry condition rather than authoritative successful persistence."
//!
//! ## What this test now proves
//!
//!   1. A REAL Handshake-managed PostgreSQL workspace is seeded through production HTTP routes with
//!      two pins, one favorite, one inbound mention edge, and one unlinked mention.
//!   2. The production `HandshakeApp` shell mounts the Sidebar pane and the app's OWN per-frame feed
//!      loads every section from that live workspace (no injected fixture).
//!   3. The CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` / `argus.click`
//!      an out-of-process swarm agent uses) inspects the populated tree: every section row, the
//!      breadcrumb strip, the Remove control, the section headers, AND the durable pin-removal
//!      completion observer are addressable by stable `author_id`.
//!   4. ONE canonical `argus.click` on `sidebar.pin.{id}.remove` produces a TERMINAL, NON-INDETERMINATE
//!      `applied` receipt. The receipt can only reach `applied` when BOTH hold:
//!        * the backend's own single authoritative operation receipt reports a persisted removal
//!          (workspace id, block id, mutation revision, HTTP outcome, EventLedger correlation, and the
//!          final persisted pin-order revision), and
//!        * the AUTHORITATIVE refreshed PostgreSQL pin list no longer contains the block.
//!      Target disappearance alone can never satisfy it: the flexible observer form additionally
//!      requires `Applied` <=> the Remove control is gone and `Failed` <=> the Remove control is still
//!      mounted (the rollback preserved the pin).
//!   5. The success path exposes NO Pins error/Retry state.
//!   6. An INDEPENDENT authoritative re-read (the live product HTTP routes, not the UI) confirms the
//!      pin is gone from `GET /loom/views/pins` and that the receipt's EventLedger event id really
//!      exists under `GET /kernel/events/aggregates/loom_block/{block_id}`.
//!   7. The disclosed canonical action-registration gap is closed: a canonical `argus.click` on
//!      `sidebar.pins.header` collapses the section, the collapse is OBSERVABLE in the freshly
//!      inspected tree (it previously was not — the snapshot renders on a fresh `egui::Context` whose
//!      `Memory` was empty, so a collapsed section always re-inspected as expanded), and the header
//!      returns an action-specific `applied` receipt rather than `indeterminate`.
//!
//! A second test proves the empty + error states inspect through canonical Argus without a backend.
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-024/` root.

use std::path::{Path, PathBuf};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::editor_pane_factories::{placeholder_pane_type, SIDEBAR_PANE_LABEL};
use handshake_native::graph::sidebar_panel::{
    section_header_author_id, section_retry_author_id, SectionKind,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' artifact dir may exist (found {})",
            p.display()
        );
    }
}

fn collect_author_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(id) = map.get("author_id").and_then(|v| v.as_str()) {
                out.push(id.to_owned());
            }
            for v in map.values() {
                collect_author_ids(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_author_ids(v, out);
            }
        }
        _ => {}
    }
}

/// The parsed JSON `value` of the node addressed by `author_id`, or `None` when the node (or a
/// machine-readable value) is absent.
fn node_json_value(tree: &serde_json::Value, author_id: &str) -> Option<serde_json::Value> {
    let raw = json_node_by_author_id(tree, author_id)?
        .get("value")?
        .as_str()?;
    serde_json::from_str(raw).ok()
}

/// A live shell with `pane-a` re-typed to the Sidebar pane so the mounted sidebar factory renders in
/// the split. Used by the no-backend empty/error proof.
fn sidebar_shell() -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    retype_pane_a_to_sidebar(&mut app);
    app
}

fn retype_pane_a_to_sidebar(app: &mut HandshakeApp) {
    let ty: PaneType = placeholder_pane_type(SIDEBAR_PANE_LABEL);
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            ty.clone(),
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    let bars = app.tab_bar_states_mut();
    if let Some(bar) = bars.get_mut(&PaneId::from("pane-a")) {
        bar.tabs = vec![handshake_native::tab_bar::TabState::new(ty)];
        bar.active_index = 0;
    }
}

// ── MT-024 FAIL_V4: canonical Argus over REAL PostgreSQL ────────────────────────────────────────────

#[cfg(feature = "integration")]
mod live {
    use super::*;
    use handshake_native::app::MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID;
    use handshake_native::command_registry::CMD_VIEW_SIDEBAR;
    use handshake_native::graph::sidebar_panel::{
        backlink_row_author_id, breadcrumb_author_id, favorite_row_author_id, pin_remove_author_id,
        pin_row_author_id, unlinked_row_author_id,
    };
    use std::time::{Duration, Instant};

    struct LiveWorkspaceCleanup<'a> {
        backend: &'a interconnect_support::LiveBackend,
        workspace_id: String,
        cleaned: bool,
    }

    impl LiveWorkspaceCleanup<'_> {
        fn assert_cleaned(&mut self) {
            let status = self.backend.delete_workspace(&self.workspace_id);
            assert!(
                matches!(status, 200 | 202 | 204 | 404),
                "managed-PG workspace cleanup returned HTTP {status}"
            );
            self.cleaned = true;
        }
    }

    impl Drop for LiveWorkspaceCleanup<'_> {
        fn drop(&mut self) {
            if !self.cleaned {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.backend.delete_workspace(&self.workspace_id)
                }));
            }
        }
    }

    fn unique_suffix() -> String {
        format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after unix epoch")
                .as_nanos()
        )
    }

    fn drive_until(
        harness: &mut Harness<'_, HandshakeApp>,
        panel: &std::sync::Arc<
            std::sync::Mutex<handshake_native::graph::sidebar_panel::LoomSidebarPanel>,
        >,
        condition: impl Fn(&handshake_native::graph::sidebar_panel::LoomSidebarPanel) -> bool,
        proof: &str,
    ) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            harness.run_steps(2);
            if panel.lock().map(|panel| condition(&panel)).unwrap_or(false) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for '{proof}'");
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn remove_owned_prior_artifact(path: &Path) {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!(
                "remove stale owned proof artifact {}: {error}",
                path.display()
            ),
        }
    }

    #[test]
    fn mt024_mounted_sidebar_canonical_argus_inspect_steer_reobserve() {
        let artifact_dir = external_artifact_dir("wp-kernel-012-mt-024/canonical-argus");
        let tree_path = artifact_dir.join("mt024-mounted-sidebar-argus.json");
        let screenshot_path = artifact_dir.join("mt024-mounted-sidebar.png");
        remove_owned_prior_artifact(&tree_path);
        remove_owned_prior_artifact(&screenshot_path);

        let live = interconnect_support::require_reachable_backend();
        let unique = format!("mt024-argus-{}", unique_suffix());
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

        // ── Seed REAL PostgreSQL through the production Loom routes ───────────────────────────────
        let create_block = |content_type: &str, title: &str, pinned: bool| {
            live.post_json(
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &serde_json::json!({
                    "content_type": content_type,
                    "title": title,
                    "pinned": pinned
                }),
            )["block_id"]
                .as_str()
                .expect("block create returns block_id")
                .to_owned()
        };
        let target_title = format!("MT024 Argus Target {unique}");
        let removed_pin = create_block("note", "MT-024 Argus Pin One", true);
        let retained_pin = create_block("file", &target_title, true);
        let favorite = create_block("note", "MT-024 Argus Favorite", false);
        live.patch_json(
            &format!("/workspaces/{workspace_id}/loom/blocks/{favorite}"),
            &serde_json::json!({ "favorite": true }),
        );
        let backlink_source = create_block("note", "MT-024 Argus Linked Source", false);
        let unlinked_source = create_block(
            "note",
            &format!("Draft text mentions {target_title} without a link"),
            false,
        );
        live.post_json(
            &format!("/workspaces/{workspace_id}/loom/edges"),
            &serde_json::json!({
                "source_block_id": backlink_source,
                "target_block_id": retained_pin,
                "edge_type": "mention",
                "created_by": "user"
            }),
        );

        // ── Mount the production shell against that live workspace ────────────────────────────────
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("mt024 canonical argus runtime");
        let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_string(),
            db_status: "ok".to_string(),
            migration_version: Some(1),
        }));
        app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
        app.set_sidebar_backend_base_url_for_test(live.base.clone());
        assert!(
            app.switch_project(&workspace_id),
            "switch to the seeded managed-PG workspace"
        );
        assert!(
            app.dispatch_palette_action_for_test(CMD_VIEW_SIDEBAR),
            "the View Sidebar command mounts the production sidebar pane"
        );
        let panel = app.mounted_sidebar_panel_for_test();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 960.0))
            .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

        drive_until(
            &mut harness,
            &panel,
            |panel| panel.pins.len() == 2 && panel.favorites.len() == 1,
            "the mounted host loads both seeded pins and the seeded favorite from real PostgreSQL",
        );

        // Bind the active block so Backlinks + Unlinked load from the live workspace too, then reopen
        // the Sidebar surface (opening a Loom block replaces the pane content).
        harness
            .state()
            .bind_sidebar_active_block_for_test(&retained_pin);
        drive_until(
            &mut harness,
            &panel,
            |panel| !panel.backlinks.is_empty() && !panel.unlinked.is_empty(),
            "the mounted host loads live backlinks and unlinked mentions for the active block",
        );
        // A breadcrumb so the crumb strip is addressable (AC6).
        if let Ok(mut guard) = panel.lock() {
            guard.push_breadcrumb(retained_pin.clone(), target_title.clone());
        }
        harness.run_steps(2);

        std::fs::create_dir_all(&artifact_dir).expect("create external MT-024 Argus artifact dir");
        let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-024-sidebar");

        // ── (1) Canonical inspect: the populated live sections are addressable ────────────────────
        let before = argus.inspect(&mut harness);
        for author in [
            pin_row_author_id(&removed_pin),
            pin_row_author_id(&retained_pin),
            favorite_row_author_id(&favorite),
            backlink_row_author_id(&backlink_source),
            unlinked_row_author_id(&unlinked_source),
            breadcrumb_author_id(0),
            pin_remove_author_id(&removed_pin),
            section_header_author_id(SectionKind::Pins),
            MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID.to_owned(),
        ] {
            assert!(
                json_has_author_id(&before, &author),
                "canonical argus.inspect must see mounted live-PG node '{author}'"
            );
        }

        let remove_target = pin_remove_author_id(&removed_pin);
        let declaration = node_json_value(&before, &remove_target)
            .expect("the Remove control publishes its click-completion declaration");
        assert_eq!(declaration["schema"], "handshake.click-completion/v1");
        assert_eq!(declaration["mode"], "observer");
        assert_eq!(declaration["flexible_target"], true);
        assert_eq!(
            declaration["observer_author_id"],
            MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID
        );
        let declared_generation = declaration["generation"]
            .as_u64()
            .expect("the Remove declaration carries a generation");
        let declared_semantic = declaration["semantic_value"]
            .as_str()
            .expect("the Remove declaration carries its semantic value")
            .to_owned();
        let observer_before =
            node_json_value(&before, MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID)
                .expect("the durable pin-removal observer publishes a completion state");
        assert_eq!(observer_before["generation"], declared_generation);
        assert_ne!(observer_before["state"], "pending");

        // ── (2) ONE canonical steer: remove the pin over the real Argus transport ─────────────────
        let observation = argus.click_and_reinspect(&mut harness, &remove_target);
        assert_eq!(
            observation.receipt_status, "applied",
            "the canonical remove-pin receipt must be TERMINAL and NON-INDETERMINATE (FAIL_V4)"
        );
        assert!(
            observation
                .agent_id
                .contains(":client:wp-kernel-012-mt-024-sidebar-agent"),
            "the canonical receipt retains the external caller attribution: {}",
            observation.agent_id
        );

        // ── (3) Terminal predicate: authoritative persistence, not target disappearance ───────────
        let predicate_remove_target = remove_target.clone();
        let predicate_removed_row = pin_row_author_id(&removed_pin);
        let predicate_retained_row = pin_row_author_id(&retained_pin);
        let predicate_workspace = workspace_id.clone();
        let predicate_block = removed_pin.clone();
        let predicate_semantic = declared_semantic.clone();
        let predicate_retry = section_retry_author_id(SectionKind::Pins);
        let terminal = argus.assert_latest_terminal_predicate_with_evidence(
            &mut harness,
            "sidebar.pin.remove.authoritative-persisted-absence-v1",
            serde_json::json!({
                "workspace_id": workspace_id,
                "removed_block_id": removed_pin,
                "retained_block_id": retained_pin,
                "declared_generation": declared_generation,
                "expected_observer_generation": declared_generation + 1,
                "expected_observer_state": "applied",
            }),
            move |tree| {
                let Some(observer) =
                    node_json_value(tree, MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID)
                else {
                    return false;
                };
                let Some(detail) = observer["terminal_detail"]
                    .as_str()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                else {
                    return false;
                };
                let receipt = &detail["operation_receipt"];
                observer["state"] == "applied"
                    && observer["generation"].as_u64() == Some(declared_generation + 1)
                    && observer["pending_target"].as_str()
                        == Some(predicate_remove_target.as_str())
                    && observer["semantic_value"].as_str() == Some(predicate_semantic.as_str())
                    // The backend's OWN authoritative operation receipt.
                    && receipt["schema_id"]
                        == "hsk.wp_kernel_012.mt_024.sidebar_mutation_receipt@1"
                    && receipt["operation"] == "sidebar.remove-pin"
                    && receipt["outcome"] == "persisted"
                    && receipt["workspace_id"].as_str() == Some(predicate_workspace.as_str())
                    && receipt["block_id"].as_str() == Some(predicate_block.as_str())
                    && receipt["http_status"].as_u64() == Some(200)
                    && receipt["persisted_pinned"] == serde_json::Value::Bool(false)
                    && receipt["pin_order_cleared"] == serde_json::Value::Bool(true)
                    && receipt["persisted_pin_order"].is_null()
                    && receipt["mutation_revision"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
                    && receipt["event_ledger_lookup"] == "resolved"
                    && receipt["event_ledger_operation"] == "pin_removed"
                    && receipt["event_ledger_event_id"]
                        .as_str()
                        .is_some_and(|value| value.starts_with("KE-"))
                    // The AUTHORITATIVE refreshed PostgreSQL pin list, not the vanished row.
                    && detail["authoritative_refresh_contains_block"]
                        == serde_json::Value::Bool(false)
                    && detail["authoritative_refreshed_pin_count"].as_u64() == Some(1)
                    && detail["authoritative_refresh_error"].is_null()
                    // Post-state shape: the removed row and its control are gone, the other pin
                    // remains, and there is NO ambiguous Pins error/Retry state (FAIL_V4).
                    && !json_has_author_id(tree, &predicate_removed_row)
                    && !json_has_author_id(tree, &predicate_remove_target)
                    && json_has_author_id(tree, &predicate_retained_row)
                    && !json_has_author_id(tree, &predicate_retry)
            },
        );
        let terminal_observation = argus.latest_terminal_observation();
        let terminal_detail: serde_json::Value = serde_json::from_str(
            node_json_value(&terminal, MT024_SIDEBAR_PIN_REMOVAL_COMPLETION_AUTHOR_ID)
                .expect("terminal observer")["terminal_detail"]
                .as_str()
                .expect("terminal detail"),
        )
        .expect("terminal detail is JSON");
        let ledger_event_id = terminal_detail["operation_receipt"]["event_ledger_event_id"]
            .as_str()
            .expect("the authoritative receipt carries an EventLedger correlation")
            .to_owned();

        // ── (4) INDEPENDENT authoritative re-read (product routes, not the UI) ────────────────────
        let persisted_pins = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/views/pins?limit=100"
        ));
        let persisted_ids: Vec<String> = persisted_pins["blocks"]
            .as_array()
            .expect("pins view returns blocks")
            .iter()
            .filter_map(|block| block["block_id"].as_str().map(ToOwned::to_owned))
            .collect();
        assert!(
            !persisted_ids.contains(&removed_pin),
            "independent authoritative re-read still lists the removed pin: {persisted_ids:?}"
        );
        assert!(
            persisted_ids.contains(&retained_pin),
            "independent authoritative re-read lost the untouched pin: {persisted_ids:?}"
        );
        let ledger = live.get_json(&format!(
            "/kernel/events/aggregates/loom_block/{removed_pin}"
        ));
        let ledger_events = ledger.as_array().expect("EventLedger returns an array");
        assert!(
            ledger_events.iter().any(|event| {
                event["event_id"].as_str() == Some(ledger_event_id.as_str())
                    && event["payload"]["operation"].as_str() == Some("pin_removed")
            }),
            "the receipt's EventLedger correlation {ledger_event_id} is not present in the durable \
             ledger for {removed_pin}"
        );

        // ── (5) Canonical action-registration repair: collapse is steerable AND observable ────────
        let pins_header = section_header_author_id(SectionKind::Pins);
        let header_before = node_json_value(&terminal, &pins_header)
            .expect("the Pins header publishes its same-target click-completion token");
        assert_eq!(header_before["mode"], "same_target");
        assert_eq!(header_before["effect"], "sidebar-section-collapse");
        let header_generation = header_before["generation"]
            .as_u64()
            .expect("the Pins header carries a collapse generation");
        let collapse = argus.click_and_reinspect(&mut harness, &pins_header);
        assert_eq!(
            collapse.receipt_status, "applied",
            "the canonical Pins collapse must return an action-specific receipt (V3 disclosure)"
        );
        let predicate_collapsed_row = pin_row_author_id(&retained_pin);
        let collapsed_tree = argus.assert_latest_terminal_predicate_with_evidence(
            &mut harness,
            "sidebar.pins.header.collapse-hides-rows-v1",
            serde_json::json!({
                "expected_collapse_generation": header_generation + 1,
                "expected_hidden_row": predicate_collapsed_row,
            }),
            move |tree| {
                let Some(header) = node_json_value(tree, &pins_header) else {
                    return false;
                };
                header["state"] == "applied"
                    && header["generation"].as_u64() == Some(header_generation + 1)
                    && !json_has_author_id(tree, &predicate_collapsed_row)
            },
        );
        let mut collapsed_ids = Vec::new();
        collect_author_ids(&collapsed_tree, &mut collapsed_ids);
        assert!(
            !collapsed_ids
                .iter()
                .any(|id| id.starts_with("sidebar.pin.")),
            "AC8: a collapsed Pins section exposes no rows to canonical Argus; got {:?}",
            collapsed_ids
                .iter()
                .filter(|id| id.starts_with("sidebar."))
                .collect::<Vec<_>>()
        );

        // ── (6) Evidence, teardown, and the canonical driver's terminal gate ──────────────────────
        let rendered = harness.render();
        cleanup.assert_cleaned();
        argus.finish_require_no_indeterminate();

        std::fs::write(
            &tree_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_id": "hsk.wp_kernel_012.mt_024.canonical_argus_evidence@1",
                "workspace_id": workspace_id,
                "removed_pin_block_id": removed_pin,
                "retained_pin_block_id": retained_pin,
                "favorite_block_id": favorite,
                "backlink_source_block_id": backlink_source,
                "unlinked_source_block_id": unlinked_source,
                "before": before,
                "immediate_after": observation.after,
                "terminal_after": terminal,
                "collapsed_after": collapsed_tree,
                "pin_removal_receipt": terminal_detail,
                "independent_persisted_pin_ids": persisted_ids,
                "independent_event_ledger_event_id": ledger_event_id,
                "receipt_id": terminal_observation.receipt_id,
                "receipt_status": terminal_observation.receipt_status,
                "agent_id": terminal_observation.agent_id,
                "terminal_predicates": terminal_observation.terminal_predicates,
            }))
            .expect("serialize canonical MT-024 sidebar tree evidence"),
        )
        .expect("write canonical MT-024 sidebar tree evidence externally");
        assert!(tree_path.is_file());

        let screenshot_marker = match rendered {
            Ok(image) => {
                image
                    .save(&screenshot_path)
                    .expect("save mounted sidebar screenshot");
                format!("CAPTURED {}", screenshot_path.display())
            }
            Err(deferred) => format!("DEFERRED (headless): {deferred}"),
        };
        println!(
            "MT-024 canonical Argus mounted sidebar (LIVE PG workspace={workspace_id}): \
             inspect(pins/favorites/backlinks/unlinked/breadcrumb/remove/header/observer) -> \
             click({remove_target}) -> TERMINAL receipt={} with persisted receipt \
             (revision={:?} ledger={ledger_event_id} pin_order_cleared=true) and authoritative \
             refreshed absence; collapse receipt={}; agent={} screenshot={screenshot_marker} \
             tree={}",
            terminal_observation.receipt_status,
            terminal_detail["operation_receipt"]["mutation_revision"],
            collapse.receipt_status,
            terminal_observation.agent_id,
            tree_path.display()
        );
        assert_no_local_artifact_dir();
    }
}

#[test]
fn mt024_mounted_sidebar_empty_and_error_states_canonical_argus() {
    let app = sidebar_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // Empty Pins/Favorites (clear the first-frame errors with empty lists), no active block
    // (Backlinks/Unlinked show a neutral prompt, no rows), and a Backlinks section ERROR to prove the
    // Retry control is addressable (AC9).
    {
        let panel = harness.state().mounted_sidebar_panel_for_test();
        let mut guard = panel.lock().unwrap();
        guard.set_pins(vec![]);
        guard.set_favorites(vec![]);
        guard.active_block_id = Some("block-x".to_owned());
        guard.set_error(SectionKind::Backlinks, "Backend unavailable");
    }
    harness.run_steps(2);

    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-024-sidebar-empty");
    let tree = argus.inspect(&mut harness);

    let mut ids = Vec::new();
    collect_author_ids(&tree, &mut ids);
    assert!(
        !ids.iter().any(|id| id.starts_with("sidebar.pin.")),
        "empty Pins must expose NO pin rows; got {:?}",
        ids.iter()
            .filter(|id| id.starts_with("sidebar."))
            .collect::<Vec<_>>()
    );
    assert!(
        !ids.iter().any(|id| id.starts_with("sidebar.favorite.")),
        "empty Favorites must expose NO favorite rows"
    );
    // AC9: the errored Backlinks section exposes its stable Retry control.
    assert!(
        json_has_author_id(&tree, &section_retry_author_id(SectionKind::Backlinks)),
        "errored Backlinks section must expose its Retry control by stable author_id (AC9)"
    );
    // MT-024 V4: every collapsible section header is addressable and publishes its action-specific
    // same-target completion token, so a canonical steer of any section is observable.
    for section in SectionKind::ALL {
        let header = section_header_author_id(section);
        assert!(
            json_has_author_id(&tree, &header),
            "section header '{header}' must be addressable by stable author_id"
        );
        let token = node_json_value(&tree, &header)
            .unwrap_or_else(|| panic!("section header '{header}' publishes a completion token"));
        assert_eq!(token["schema"], "handshake.click-completion/v1");
        assert_eq!(token["mode"], "same_target");
        assert_eq!(token["effect"], "sidebar-section-collapse");
    }

    println!(
        "MT-024 canonical Argus empty/error sidebar: inspect() returned {} author_ids, \
         0 pin/favorite rows, backlinks Retry addressable (AC9), 4 section headers carry \
         same-target collapse completion tokens",
        ids.len()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
