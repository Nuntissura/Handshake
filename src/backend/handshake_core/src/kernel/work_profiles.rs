use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_WORK_PROFILES_STUB_ID: &str = "WP-1-Work-Profiles-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileAutonomyKnobsV1 {
    pub max_auto_actions: u8,
    pub requires_operator_approval_for_promotion: bool,
    pub allow_parallel_agents: bool,
    pub allow_network: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileRoleRouteV1 {
    pub role_id: String,
    pub model_ref: String,
    pub provider_ref: String,
    pub capability_profile_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileV1 {
    pub profile_id: String,
    pub profile_version: u32,
    pub profile_id_is_immutable: bool,
    pub display_name: String,
    pub role_routes: Vec<WorkProfileRoleRouteV1>,
    pub autonomy: WorkProfileAutonomyKnobsV1,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileReceiptV1 {
    pub receipt_ref: String,
    pub profile_id: String,
    pub profile_version: u32,
    pub action_request_id: String,
    pub event_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileActionRequestV1 {
    pub action_request_id: String,
    pub action_id: String,
    pub role_id: String,
    pub selected_profile_id: String,
    pub selected_route_model_ref: String,
    pub receipt_ref: String,
    pub job_metadata_work_profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProfileRegistryV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub folded_stub_ids: Vec<String>,
    pub profile_storage_ref: String,
    pub selected_profile_id: String,
    pub profiles: Vec<WorkProfileV1>,
    pub profile_receipts: Vec<WorkProfileReceiptV1>,
    pub action_requests: Vec<WorkProfileActionRequestV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProfileActionRequestProjectionV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub selected_profile_id: String,
    pub profile_ids_locked: bool,
    pub action_request_count: usize,
    pub role_route_bindings: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub autonomy_max_auto_actions: u8,
    pub mutates_profile_store: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProfileValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_work_profiles(
    registry: &WorkProfileRegistryV1,
) -> Result<(), Vec<WorkProfileValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &registry.schema_id);
    require_non_empty(&mut errors, "registry_id", &registry.registry_id);
    require_non_empty(
        &mut errors,
        "profile_storage_ref",
        &registry.profile_storage_ref,
    );
    require_non_empty(
        &mut errors,
        "selected_profile_id",
        &registry.selected_profile_id,
    );
    require_vec(&mut errors, "folded_stub_ids", &registry.folded_stub_ids);
    require_vec(&mut errors, "profiles", &registry.profiles);
    require_vec(&mut errors, "profile_receipts", &registry.profile_receipts);
    require_vec(&mut errors, "action_requests", &registry.action_requests);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &registry.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &registry.folded_source_refs,
    );

    if !contains_exact(&registry.folded_stub_ids, FOLDED_WORK_PROFILES_STUB_ID) {
        errors.push(WorkProfileValidationError {
            field: "folded_stub_ids",
            message: "work profiles must preserve the folded stub id",
        });
    }
    if !contains_text(&registry.folded_source_refs, FOLDED_WORK_PROFILES_STUB_ID) {
        errors.push(WorkProfileValidationError {
            field: "folded_source_refs",
            message: "work profiles must preserve the folded source reference",
        });
    }

    validate_authority_refs(&mut errors, registry);
    validate_profiles(&mut errors, registry);
    validate_receipts_and_requests(&mut errors, registry);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_work_profile_action_requests(
    registry: &WorkProfileRegistryV1,
) -> Result<WorkProfileActionRequestProjectionV1, Vec<WorkProfileValidationError>> {
    validate_work_profiles(registry)?;

    let selected_profile = registry
        .profiles
        .iter()
        .find(|profile| profile.profile_id == registry.selected_profile_id)
        .expect("validated selected profile exists");
    let routes_by_role: HashMap<&str, &WorkProfileRoleRouteV1> = selected_profile
        .role_routes
        .iter()
        .map(|route| (route.role_id.as_str(), route))
        .collect();

    Ok(WorkProfileActionRequestProjectionV1 {
        schema_id: "hsk.kernel.work_profile_action_request_projection@1".to_string(),
        registry_id: registry.registry_id.clone(),
        selected_profile_id: registry.selected_profile_id.clone(),
        profile_ids_locked: registry
            .profiles
            .iter()
            .all(|profile| profile.profile_id_is_immutable),
        action_request_count: registry.action_requests.len(),
        role_route_bindings: registry
            .action_requests
            .iter()
            .filter_map(|request| {
                routes_by_role.get(request.role_id.as_str()).map(|route| {
                    format!(
                        "{}:{}->{}",
                        request.action_request_id, request.role_id, route.model_ref
                    )
                })
            })
            .collect(),
        receipt_refs: registry
            .action_requests
            .iter()
            .map(|request| request.receipt_ref.clone())
            .collect(),
        autonomy_max_auto_actions: selected_profile.autonomy.max_auto_actions,
        mutates_profile_store: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<WorkProfileValidationError>,
    registry: &WorkProfileRegistryV1,
) {
    for required_ref in [
        "kernel.action_catalog",
        "kernel.role_turn_isolation",
        "flight_recorder.profile_events",
        "kernel.workflow_transition_registry",
    ] {
        if !contains_exact(&registry.product_authority_refs, required_ref) {
            errors.push(WorkProfileValidationError {
                field: "product_authority_refs",
                message: "work profiles must cite action catalog, role-turn isolation, profile events, and workflow transition authorities",
            });
        }
    }
}

fn validate_profiles(
    errors: &mut Vec<WorkProfileValidationError>,
    registry: &WorkProfileRegistryV1,
) {
    let mut profile_ids = HashSet::new();
    let mut selected_profile_seen = false;

    for profile in &registry.profiles {
        if !profile_ids.insert(profile.profile_id.as_str()) {
            errors.push(WorkProfileValidationError {
                field: "profiles.profile_id",
                message: "work profile ids must be unique",
            });
        }
        if profile.profile_id == registry.selected_profile_id {
            selected_profile_seen = true;
        }

        require_non_empty(errors, "profiles.profile_id", &profile.profile_id);
        require_non_empty(errors, "profiles.display_name", &profile.display_name);
        require_non_empty(errors, "profiles.created_at_utc", &profile.created_at_utc);
        require_vec(errors, "profiles.role_routes", &profile.role_routes);

        if profile.profile_version == 0 {
            errors.push(WorkProfileValidationError {
                field: "profiles.profile_version",
                message: "work profile version must be greater than zero",
            });
        }
        if !profile.profile_id_is_immutable {
            errors.push(WorkProfileValidationError {
                field: "profiles.profile_id_is_immutable",
                message: "work profile ids must be immutable once referenced by jobs",
            });
        }

        validate_autonomy(errors, &profile.autonomy);
        validate_routes(errors, profile);
    }

    if !selected_profile_seen {
        errors.push(WorkProfileValidationError {
            field: "selected_profile_id",
            message: "selected work profile must exist in profile storage",
        });
    }
}

fn validate_autonomy(
    errors: &mut Vec<WorkProfileValidationError>,
    autonomy: &WorkProfileAutonomyKnobsV1,
) {
    if autonomy.max_auto_actions > 10 {
        errors.push(WorkProfileValidationError {
            field: "profiles.autonomy.max_auto_actions",
            message: "autonomy max_auto_actions must be bounded to <= 10",
        });
    }
    if !autonomy.requires_operator_approval_for_promotion {
        errors.push(WorkProfileValidationError {
            field: "profiles.autonomy.requires_operator_approval_for_promotion",
            message: "profile autonomy must keep promotion behind operator approval",
        });
    }
}

fn validate_routes(errors: &mut Vec<WorkProfileValidationError>, profile: &WorkProfileV1) {
    let mut route_roles = HashSet::new();
    for route in &profile.role_routes {
        if !route_roles.insert(route.role_id.as_str()) {
            errors.push(WorkProfileValidationError {
                field: "profiles.role_routes.role_id",
                message: "role routes must be unique per profile",
            });
        }
        require_non_empty(errors, "profiles.role_routes.role_id", &route.role_id);
        require_non_empty(errors, "profiles.role_routes.model_ref", &route.model_ref);
        require_non_empty(
            errors,
            "profiles.role_routes.provider_ref",
            &route.provider_ref,
        );
        require_non_empty(
            errors,
            "profiles.role_routes.capability_profile_ref",
            &route.capability_profile_ref,
        );
    }
}

fn validate_receipts_and_requests(
    errors: &mut Vec<WorkProfileValidationError>,
    registry: &WorkProfileRegistryV1,
) {
    let profiles_by_id: HashMap<&str, &WorkProfileV1> = registry
        .profiles
        .iter()
        .map(|profile| (profile.profile_id.as_str(), profile))
        .collect();
    let request_ids: HashSet<&str> = registry
        .action_requests
        .iter()
        .map(|request| request.action_request_id.as_str())
        .collect();
    let receipts_by_ref: HashMap<&str, &WorkProfileReceiptV1> = registry
        .profile_receipts
        .iter()
        .map(|receipt| (receipt.receipt_ref.as_str(), receipt))
        .collect();

    validate_receipts(errors, registry, &profiles_by_id, &request_ids);
    validate_action_requests(errors, registry, &profiles_by_id, &receipts_by_ref);
}

fn validate_receipts(
    errors: &mut Vec<WorkProfileValidationError>,
    registry: &WorkProfileRegistryV1,
    profiles_by_id: &HashMap<&str, &WorkProfileV1>,
    request_ids: &HashSet<&str>,
) {
    let mut receipt_refs = HashSet::new();
    for receipt in &registry.profile_receipts {
        if !receipt_refs.insert(receipt.receipt_ref.as_str()) {
            errors.push(WorkProfileValidationError {
                field: "profile_receipts.receipt_ref",
                message: "profile receipt refs must be unique",
            });
        }
        require_non_empty(errors, "profile_receipts.receipt_ref", &receipt.receipt_ref);
        require_non_empty(errors, "profile_receipts.profile_id", &receipt.profile_id);
        require_non_empty(
            errors,
            "profile_receipts.action_request_id",
            &receipt.action_request_id,
        );
        require_non_empty(errors, "profile_receipts.event_ref", &receipt.event_ref);

        if !receipt.event_ref.starts_with("FR-EVT-PROFILE-") {
            errors.push(WorkProfileValidationError {
                field: "profile_receipts.event_ref",
                message: "work profile receipts must cite FR-EVT-PROFILE events",
            });
        }
        if !request_ids.contains(receipt.action_request_id.as_str()) {
            errors.push(WorkProfileValidationError {
                field: "profile_receipts.action_request_id",
                message: "profile receipt must reference an action request",
            });
        }

        match profiles_by_id.get(receipt.profile_id.as_str()) {
            Some(profile) if profile.profile_version == receipt.profile_version => {}
            Some(_) => errors.push(WorkProfileValidationError {
                field: "profile_receipts.profile_version",
                message: "profile receipt must preserve the selected profile version",
            }),
            None => errors.push(WorkProfileValidationError {
                field: "profile_receipts.profile_id",
                message: "profile receipt must reference a stored profile",
            }),
        }
    }
}

fn validate_action_requests(
    errors: &mut Vec<WorkProfileValidationError>,
    registry: &WorkProfileRegistryV1,
    profiles_by_id: &HashMap<&str, &WorkProfileV1>,
    receipts_by_ref: &HashMap<&str, &WorkProfileReceiptV1>,
) {
    let mut request_ids = HashSet::new();
    for request in &registry.action_requests {
        if !request_ids.insert(request.action_request_id.as_str()) {
            errors.push(WorkProfileValidationError {
                field: "action_requests.action_request_id",
                message: "action request ids must be unique",
            });
        }

        require_non_empty(
            errors,
            "action_requests.action_request_id",
            &request.action_request_id,
        );
        require_non_empty(errors, "action_requests.action_id", &request.action_id);
        require_non_empty(errors, "action_requests.role_id", &request.role_id);
        require_non_empty(
            errors,
            "action_requests.selected_profile_id",
            &request.selected_profile_id,
        );
        require_non_empty(
            errors,
            "action_requests.selected_route_model_ref",
            &request.selected_route_model_ref,
        );
        require_non_empty(errors, "action_requests.receipt_ref", &request.receipt_ref);
        require_non_empty(
            errors,
            "action_requests.job_metadata_work_profile_id",
            &request.job_metadata_work_profile_id,
        );

        if request.selected_profile_id != registry.selected_profile_id {
            errors.push(WorkProfileValidationError {
                field: "action_requests.selected_profile_id",
                message: "action requests must use the selected work profile",
            });
        }
        if request.job_metadata_work_profile_id != request.selected_profile_id {
            errors.push(WorkProfileValidationError {
                field: "action_requests.job_metadata_work_profile_id",
                message: "action request job metadata must record work_profile_id",
            });
        }

        validate_action_request_route(errors, request, profiles_by_id);
        validate_action_request_receipt(errors, request, receipts_by_ref);
    }
}

fn validate_action_request_route(
    errors: &mut Vec<WorkProfileValidationError>,
    request: &WorkProfileActionRequestV1,
    profiles_by_id: &HashMap<&str, &WorkProfileV1>,
) {
    let Some(profile) = profiles_by_id.get(request.selected_profile_id.as_str()) else {
        errors.push(WorkProfileValidationError {
            field: "action_requests.selected_profile_id",
            message: "action request selected profile must exist",
        });
        return;
    };

    let route = profile
        .role_routes
        .iter()
        .find(|route| route.role_id == request.role_id);
    match route {
        Some(route) if route.model_ref == request.selected_route_model_ref => {}
        Some(_) => errors.push(WorkProfileValidationError {
            field: "action_requests.selected_route_model_ref",
            message: "action request model ref must match the selected profile role route",
        }),
        None => errors.push(WorkProfileValidationError {
            field: "action_requests.role_id",
            message: "action request role must have a route in the selected profile",
        }),
    }
}

fn validate_action_request_receipt(
    errors: &mut Vec<WorkProfileValidationError>,
    request: &WorkProfileActionRequestV1,
    receipts_by_ref: &HashMap<&str, &WorkProfileReceiptV1>,
) {
    let Some(receipt) = receipts_by_ref.get(request.receipt_ref.as_str()) else {
        errors.push(WorkProfileValidationError {
            field: "action_requests.receipt_ref",
            message: "action request must be bound to a profile receipt",
        });
        return;
    };

    if receipt.action_request_id != request.action_request_id
        || receipt.profile_id != request.selected_profile_id
    {
        errors.push(WorkProfileValidationError {
            field: "action_requests.receipt_ref",
            message: "profile receipt must match action request and selected profile",
        });
    }
}

fn require_non_empty(
    errors: &mut Vec<WorkProfileValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(WorkProfileValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(errors: &mut Vec<WorkProfileValidationError>, field: &'static str, value: &[T]) {
    if value.is_empty() {
        errors.push(WorkProfileValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
