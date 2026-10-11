//! WP-1 MT-015: database-free UserManual coverage for cloud-access configuration.
//!
//! Subscription authentication is owned by the official provider CLIs and
//! optional BYOK secrets are owned by the OS keychain. This proof therefore
//! reads the canonical compiled UserManual corpus directly and creates no
//! durable product or test state.

use std::collections::BTreeSet;

use handshake_core::user_manual::registry::{wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::{seed_corpus, SeedCorpus};
use handshake_core::user_manual::store::UserManualPage;
use handshake_core::user_manual::{
    cloud_model_access_behavior_coverage_matrix, verify_cloud_model_access_behavior_coverage,
    DiagnosticTierPosture, USER_MANUAL_VERSION,
};

fn compiled_pages(corpus: &SeedCorpus) -> Vec<UserManualPage> {
    let observed_at: chrono::DateTime<chrono::Utc> = "1970-01-01T00:00:00Z"
        .parse()
        .expect("fixed compiled-corpus observation timestamp");
    corpus
        .pages
        .iter()
        .map(|page| UserManualPage {
            page_id: format!("compiled-seed:{}", page.slug),
            slug: page.slug.clone(),
            title: page.title.clone(),
            page_kind: page.page_kind.to_owned(),
            audience: page.audience.to_owned(),
            body: page.body_json(),
            content_hash: page.content_hash(),
            manual_version: USER_MANUAL_VERSION.to_owned(),
            source_kind: "compiled_seed".to_owned(),
            spec_anchors: page.spec_anchors.clone(),
            status: "active".to_owned(),
            superseded_by_slug: None,
            ledger_event_id: None,
            created_at: observed_at,
            updated_at: observed_at,
        })
        .collect()
}

#[test]
fn cloud_model_access_behaviors_have_manual_coverage_without_legacy_database() {
    let corpus = seed_corpus();
    let pages = compiled_pages(&corpus);
    let tools = corpus.tools;
    let matrix = cloud_model_access_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();

    for surface in wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::ModelAccess)
    {
        assert!(
            behavior_ids.contains(surface.surface_id),
            "shipped Model Access route {} {} escaped behavior coverage",
            surface.method,
            surface.route
        );
    }
    assert_eq!(behavior_ids.len(), matrix.len());

    for row in &matrix {
        assert_eq!(row.user_manual_slug, "cloud-model-access");
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::NotApplicableWithReason
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::NotApplicableWithReason
        );
        assert!(row.deferred_reason.is_some());
    }

    verify_cloud_model_access_behavior_coverage(&matrix, &pages, &tools).unwrap_or_else(|errors| {
        panic!(
            "cloud-model access behavior coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    let body = pages
        .iter()
        .find(|page| page.slug == "cloud-model-access")
        .expect("cloud-model-access manual page exists")
        .body
        .to_string();
    for required in [
        "model_access_route_tests",
        "put_store_returns_200_and_never_echoes_the_key",
        "delete_byok_key_is_idempotent_and_updates_status",
        "get_providers_reflects_configured_and_excludes_gemini",
        "keychain_unavailable_is_503",
        "cloud_byok_access_config_leak_tests",
        "byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain",
        "test_cloud_models_settings_argus",
        "cloud_models_controls_are_addressable_and_gemini_is_never_offered",
        "cloud_models_key_entry_renders_when_backend_unreachable",
        "typed_byok_key_is_wiped_from_egui_memory_after_close",
        "claude auth status --json",
        "codex login status",
        "CODEX_HOME",
        "Operator credentials are never required",
        "cloud_model_access_manual_coverage_tests",
        "cloud_model_access_behaviors_have_manual_coverage_without_legacy_database",
        "cli_bridge_login_records_the_official_command_without_stealing_focus",
        "cli_login_session_is_pollable_typeable_and_cancellable",
        "unknown_cli_login_session_is_404_on_poll_input_and_cancel",
        "in_app_login_panel_renders_the_provider_prompt_and_routes_the_typed_answer",
        "login_confirmation_never_promises_a_new_terminal_or_focus_change",
        "cli_bridge_login_quiet_tests",
        "in_app_cli_login_creates_no_new_visible_window_and_no_foreground_change",
        "no_backend_spawn_site_creates_a_console_window",
    ] {
        assert!(body.contains(required), "manual missing `{required}`");
    }

    for route in [
        "DELETE /model-access/byok/{provider}/key",
        "GET /model-access/cli-bridge-login/{session}",
        "POST /model-access/cli-bridge-login/{session}/input",
        "POST /model-access/cli-bridge-login/{session}/cancel",
    ] {
        assert!(body.contains(route), "manual missing route `{route}`");
    }
    for stale in [
        "foreground terminal",
        "foreground console",
        "in a new terminal",
        "may take focus",
        "cloud_access_config_tests",
    ] {
        assert!(
            !body.contains(stale),
            "manual retains stale claim `{stale}`"
        );
    }
    for posture in [
        "HBR-INT-009 diagnostic posture",
        "Tier 2 internal_diagnostics: DEFERRED-with-reason",
        "Tier 3 Palmistry: DEFERRED-with-reason",
    ] {
        assert!(body.contains(posture), "manual missing posture `{posture}`");
    }
}
