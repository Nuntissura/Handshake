use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    work_profiles::{
        project_work_profile_action_requests, validate_work_profiles, WorkProfileActionRequestV1,
        WorkProfileAutonomyKnobsV1, WorkProfileReceiptV1, WorkProfileRegistryV1,
        WorkProfileRoleRouteV1, WorkProfileV1,
    },
};

#[test]
fn kernel_work_profiles_validate_storage_selection_and_immutable_ids() {
    let registry = sample_registry();

    validate_work_profiles(&registry).expect("work profile registry validates");

    assert_eq!(
        registry.profile_storage_ref,
        "profile-store://kernel/work-profiles"
    );
    assert_eq!(registry.selected_profile_id, "profile-kernel-builder-v1");
    assert!(registry.profiles[0].profile_id_is_immutable);
}

#[test]
fn kernel_work_profiles_project_role_routes_receipts_and_action_request_bindings() {
    let registry = sample_registry();
    let projection = project_work_profile_action_requests(&registry).expect("projection builds");

    assert_eq!(projection.selected_profile_id, "profile-kernel-builder-v1");
    assert!(projection.profile_ids_locked);
    assert_eq!(projection.action_request_count, 2);
    assert!(projection
        .role_route_bindings
        .contains(&"action-claim:CODER->model://local-small-coder".to_string()));
    assert!(projection
        .role_route_bindings
        .contains(&"action-validate:VALIDATOR->model://cloud-validator".to_string()));
    assert!(projection
        .receipt_refs
        .contains(&"receipt://profile/action-claim".to_string()));
    assert!(!projection.mutates_profile_store);
}

#[test]
fn kernel_work_profiles_reject_bad_profile_ids_routes_autonomy_and_receipts() {
    let mut registry = sample_registry();
    registry.profiles[0].profile_id_is_immutable = false;
    registry.profiles[0].autonomy.max_auto_actions = 25;
    registry.profiles[0]
        .autonomy
        .requires_operator_approval_for_promotion = false;
    registry.action_requests[0].job_metadata_work_profile_id = "profile-other".to_string();
    registry.action_requests[1].receipt_ref = "receipt://missing".to_string();
    registry.profile_receipts[0].event_ref = "FR-EVT-WRONG-001".to_string();
    registry.profiles[0]
        .role_routes
        .retain(|route| route.role_id != "VALIDATOR");

    let errors = validate_work_profiles(&registry).expect_err("unsafe registry must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "profiles.profile_id_is_immutable"));
    assert!(errors
        .iter()
        .any(|error| error.field == "profiles.autonomy.max_auto_actions"));
    assert!(errors.iter().any(|error| {
        error.field == "profiles.autonomy.requires_operator_approval_for_promotion"
    }));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_requests.job_metadata_work_profile_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_requests.receipt_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "profile_receipts.event_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_requests.role_id"));
}

#[test]
fn kernel_work_profiles_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.work_profiles.project")
        .expect("work profiles projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "work_profile_receipts_bound"));
}

fn sample_registry() -> WorkProfileRegistryV1 {
    WorkProfileRegistryV1 {
        schema_id: "hsk.kernel.work_profiles@1".to_string(),
        registry_id: "work-profiles-mt038".to_string(),
        folded_stub_ids: vec!["WP-1-Work-Profiles-v1".to_string()],
        profile_storage_ref: "profile-store://kernel/work-profiles".to_string(),
        selected_profile_id: "profile-kernel-builder-v1".to_string(),
        profiles: vec![profile()],
        profile_receipts: vec![
            receipt("receipt://profile/action-claim", "action-claim"),
            receipt("receipt://profile/action-validate", "action-validate"),
        ],
        action_requests: vec![
            action_request("action-claim", "CODER", "model://local-small-coder"),
            action_request("action-validate", "VALIDATOR", "model://cloud-validator"),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.role_turn_isolation".to_string(),
            "flight_recorder.profile_events".to_string(),
            "kernel.workflow_transition_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Work-Profiles-v1.contract.json".to_string(),
        ],
    }
}

fn profile() -> WorkProfileV1 {
    WorkProfileV1 {
        profile_id: "profile-kernel-builder-v1".to_string(),
        profile_version: 1,
        profile_id_is_immutable: true,
        display_name: "Kernel Builder".to_string(),
        role_routes: vec![
            route("CODER", "model://local-small-coder"),
            route("VALIDATOR", "model://cloud-validator"),
        ],
        autonomy: WorkProfileAutonomyKnobsV1 {
            max_auto_actions: 4,
            requires_operator_approval_for_promotion: true,
            allow_parallel_agents: true,
            allow_network: false,
        },
        created_at_utc: "2026-05-14T19:05:00Z".to_string(),
    }
}

fn route(role_id: &str, model_ref: &str) -> WorkProfileRoleRouteV1 {
    WorkProfileRoleRouteV1 {
        role_id: role_id.to_string(),
        model_ref: model_ref.to_string(),
        provider_ref: format!("provider://{role_id}").to_ascii_lowercase(),
        capability_profile_ref: format!("capability://{role_id}").to_ascii_lowercase(),
    }
}

fn receipt(receipt_ref: &str, action_request_id: &str) -> WorkProfileReceiptV1 {
    WorkProfileReceiptV1 {
        receipt_ref: receipt_ref.to_string(),
        profile_id: "profile-kernel-builder-v1".to_string(),
        profile_version: 1,
        action_request_id: action_request_id.to_string(),
        event_ref: format!("FR-EVT-PROFILE-001-{action_request_id}"),
    }
}

fn action_request(
    action_request_id: &str,
    role_id: &str,
    selected_route_model_ref: &str,
) -> WorkProfileActionRequestV1 {
    WorkProfileActionRequestV1 {
        action_request_id: action_request_id.to_string(),
        action_id: "kernel.role_turn_isolation.project".to_string(),
        role_id: role_id.to_string(),
        selected_profile_id: "profile-kernel-builder-v1".to_string(),
        selected_route_model_ref: selected_route_model_ref.to_string(),
        receipt_ref: format!("receipt://profile/{action_request_id}"),
        job_metadata_work_profile_id: "profile-kernel-builder-v1".to_string(),
    }
}
