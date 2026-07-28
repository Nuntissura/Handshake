//! Manifest-driven MT-108 host-surface proof.
//!
//! This test is intentionally runner-only: the bounded external supervisor selects one matrix row,
//! provides its exact source/process identity, and invokes this same production-boundary loop. Ordinary
//! `cargo test` runs skip it, so missing supervisor context can never make the native suite non-hermetic.

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use canonical_argus_driver::{json_has_author_id, live_author_id_selected, CanonicalArgusDriver};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use screenshot_harness::ScreenshotHarness as Harness;

#[derive(Debug, serde::Deserialize)]
struct Matrix {
    schema_id: String,
    rows: Vec<MatrixRow>,
}

#[derive(Debug, serde::Deserialize)]
struct MatrixRow {
    scenario_id: String,
    proof_kind: String,
    route: Option<String>,
    #[serde(default)]
    expected_author_ids: Vec<String>,
    action_author_id: Option<String>,
    action_value: Option<String>,
    action_semantic: Option<String>,
    post_action_target: Option<String>,
}

fn selected_row() -> MatrixRow {
    let matrix: Matrix = serde_json::from_str(include_str!("mt108_argus_matrix.json"))
        .expect("MT-108 Argus matrix JSON parses");
    assert_eq!(matrix.schema_id, "hsk.native_gui.argus_surface_matrix@1");
    let selected = std::env::var("HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID")
        .expect("bounded supervisor selects one MT-108 matrix scenario");
    matrix
        .rows
        .into_iter()
        .find(|row| row.scenario_id == selected)
        .unwrap_or_else(|| panic!("unknown MT-108 matrix scenario {selected:?}"))
}

fn shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build MT-108 host-surface runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    (app, runtime)
}

#[test]
#[ignore = "MT-108 runner-only matrix row; requires bounded external supervisor identity and GPU capture"]
fn mt108_argus_manifest_surface_route() {
    let row = selected_row();
    assert_eq!(row.proof_kind, "host_route");
    let route = row.route.as_deref().expect("host-route row has a route");
    let (app, _runtime) = shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let live_ctx = harness.ctx.clone();

    let fired = match route {
        "__command_palette__" => {
            harness.state_mut().open_command_palette();
            true
        }
        "__quick_switcher__" => {
            harness.state_mut().open_quick_switcher();
            true
        }
        command => harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&live_ctx, command),
    };
    assert!(fired, "matrix route {route:?} produced an observable open");
    harness.run_steps(4);

    assert_eq!(
        row.post_action_target.as_deref(),
        Some("present"),
        "{}: host action must declare its target-presence postcondition",
        row.scenario_id
    );
    let (tab_target, reactivate_host_tab) = if let Some(action) = row.action_author_id.as_deref() {
        (action.to_owned(), false)
    } else {
        let pane_id = harness
            .state()
            .active_pane()
            .cloned()
            .expect("host route retains an active pane");
        let bar = harness
            .state()
            .tab_bar_states()
            .get(&pane_id)
            .expect("active pane owns a tab bar");
        let tab = bar
            .tabs
            .get(bar.active_index)
            .expect("active pane owns an active tab");
        let target = handshake_native::tab_bar::tab_author_id_for(
            pane_id.as_ref(),
            bar.active_index,
            &tab.pane_type,
        );
        (target, true)
    };

    let mut argus = CanonicalArgusDriver::bind(harness.state(), &row.scenario_id);
    let mounted = argus.inspect(&mut harness);
    for author_id in &row.expected_author_ids {
        assert!(
            json_has_author_id(&mounted, author_id),
            "{}: mounted surface is missing stable author_id {author_id}",
            row.scenario_id
        );
    }
    assert!(
        json_has_author_id(&mounted, &tab_target),
        "{}: canonical inspection is missing action target {tab_target}",
        row.scenario_id
    );
    if reactivate_host_tab {
        assert_eq!(
            row.action_semantic.as_deref(),
            Some("reactivate_host_tab"),
            "{}: dynamic tab action must declare reactivation semantics",
            row.scenario_id
        );
        let away_route = if route == "view.code-editor" {
            "view.rich-note"
        } else {
            "view.code-editor"
        };
        assert!(
            harness
                .state_mut()
                .dispatch_palette_action_for_test_with_ctx(&live_ctx, away_route),
            "{}: failed to navigate away before reactivating its host tab",
            row.scenario_id
        );
        harness.run_steps(4);
        let away_pane_id = harness
            .state()
            .active_pane()
            .cloned()
            .expect("away route retains an active pane");
        let away_bar = harness
            .state()
            .tab_bar_states()
            .get(&away_pane_id)
            .expect("away route owns a tab bar");
        let away_tab = away_bar
            .tabs
            .get(away_bar.active_index)
            .expect("away route owns an active tab");
        let away_target = handshake_native::tab_bar::tab_author_id_for(
            away_pane_id.as_ref(),
            away_bar.active_index,
            &away_tab.pane_type,
        );
        assert_ne!(
            away_target, tab_target,
            "{}: away route did not select a distinct tab",
            row.scenario_id
        );
        assert_eq!(
            live_author_id_selected(&harness, &tab_target),
            Some(false),
            "{}: original host tab stayed selected after navigate-away",
            row.scenario_id
        );
        assert_eq!(
            live_author_id_selected(&harness, &away_target),
            Some(true),
            "{}: navigate-away tab is not selected before reactivation",
            row.scenario_id
        );
        assert!(
            json_has_author_id(&argus.inspect(&mut harness), &tab_target),
            "{}: inactive host tab is not inspectable before reactivation",
            row.scenario_id
        );
    } else {
        let expected_semantic = if row.action_value.is_some() {
            "set_search_query"
        } else {
            "refresh_preserves_control"
        };
        assert_eq!(
            row.action_semantic.as_deref(),
            Some(expected_semantic),
            "{}: explicit action semantic is not declared",
            row.scenario_id
        );
    }
    let observation = match row.action_value.as_deref() {
        Some(value) => argus.set_value_and_reinspect(&mut harness, &tab_target, value),
        None => argus.click_and_reinspect(&mut harness, &tab_target),
    };
    if reactivate_host_tab {
        assert_eq!(
            observation.target_selected_before,
            Some(false),
            "{}: trace did not observe the original tab inactive before click",
            row.scenario_id
        );
        assert_eq!(
            observation.target_selected_after,
            Some(true),
            "{}: trace did not observe the original tab selected after click",
            row.scenario_id
        );
    }
    if let Some(value) = row.action_value.as_deref() {
        assert!(
            json_has_author_id_value(&observation.after, &tab_target, value),
            "{}: fresh inspection did not expose the exact applied value {:?} on {}",
            row.scenario_id,
            value,
            tab_target
        );
    }
    assert!(matches!(
        observation.receipt_status.as_str(),
        "applied" | "indeterminate"
    ));
    assert_eq!(
        json_has_author_id(&observation.after, &tab_target),
        true,
        "{}: fresh inspection does not match the declared post-action target state",
        row.scenario_id
    );
    for author_id in &row.expected_author_ids {
        assert!(
            json_has_author_id(&observation.after, author_id),
            "{}: fresh inspection lost mounted surface author_id {author_id}",
            row.scenario_id
        );
    }
    let render = harness.render();
    if screenshot_harness::screenshot_marker::gpu_screenshot_enabled() {
        render.expect("MT-108 pixel closure requires a material captured frame");
    } else {
        assert!(
            render.is_err(),
            "MT-108 headless mode must emit a typed DEFERRED marker, never a captured frame"
        );
    }
    argus.finish();
}

fn json_has_author_id_value(value: &serde_json::Value, author_id: &str, expected: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            (map.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id)
                && map.get("value").and_then(serde_json::Value::as_str) == Some(expected))
                || map
                    .values()
                    .any(|child| json_has_author_id_value(child, author_id, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_has_author_id_value(child, author_id, expected)),
        _ => false,
    }
}
