use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};

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
        // MT-014: enforce the provider_ref resolver in the live validation path.
        // A non-empty provider_ref that resolves to neither a canonical provider
        // id nor a known migratable alias is an Unknown dangle and is rejected
        // here (AC-3: "no silent dangle"). Canonical ids pass; the retired
        // `ollama` alias resolves as a deterministic Migration (surfaced + applied
        // by `validate_and_migrate_work_profiles`) and is not an error. Skip when
        // empty so the require_non_empty check above owns the empty case.
        if !route.provider_ref.trim().is_empty()
            && matches!(
                resolve_provider_ref(&route.provider_ref),
                ProviderRefResolution::Unknown(_)
            )
        {
            errors.push(WorkProfileValidationError {
                field: "profiles.role_routes.provider_ref",
                message:
                    "provider_ref must resolve to a canonical provider id or a known migratable alias",
            });
        }
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

// ===========================================================================
// WP-1 MT-014: Work Profiles `provider_ref` resolver (wired into live
// validation).
//
// `WorkProfileRoleRouteV1.provider_ref` was a free String with only a non-empty
// check. MT-014 adds a resolver that validates a route's `provider_ref` against
// the canonical provider id set (`local_runtime`, `openai_compat`, mirroring
// `llm::registry::ProviderKind`) and migrates the retired `ollama` daemon id to
// `local_runtime` deterministically. The migration is SURFACED via an
// FR-EVT-PROFILE Flight Recorder event (mirroring the FR-EVT-PROFILE- receipt
// convention) rather than being silently rewritten in place.
//
// The resolver is ENFORCED by the live validation path (AC-3: "resolves or is
// migrated deterministically, no silent dangle"):
//   * `validate_work_profiles` invokes the resolver per role-route and REJECTS
//     an unresolvable `Unknown` provider_ref with a typed
//     `WorkProfileValidationError` (a canonical id passes; the `ollama` alias is
//     a deterministic migration, not an error).
//   * `validate_and_migrate_work_profiles` is the provider_ref-aware load path:
//     it runs structural validation, then for each `ollama` route emits the
//     surfaced FR-EVT-PROFILE event and APPLIES the migrated `local_runtime`
//     provider_ref into the returned registry (never a silent in-place rewrite).
// ===========================================================================

/// Stable Flight Recorder event key for a surfaced provider_ref migration.
/// Mirrors the FR-EVT-PROFILE- receipt convention
/// ([`WorkProfileReceiptV1::event_ref`] must start with `FR-EVT-PROFILE-`).
pub const PROVIDER_REF_MIGRATION_FR_EVENT: &str = "FR-EVT-PROFILE-PROVIDER-REF-MIGRATED";

/// The canonical provider ids a Work Profile `provider_ref` may resolve to,
/// mirroring `llm::registry::ProviderKind` (`local_runtime` | `openai_compat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalProviderRef {
    LocalRuntime,
    OpenAiCompat,
}

impl CanonicalProviderRef {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalRuntime => "local_runtime",
            Self::OpenAiCompat => "openai_compat",
        }
    }
}

/// The typed resolution of a Work Profile role-route `provider_ref` (MT-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRefResolution {
    /// Already a canonical provider id; no change needed.
    Canonical(CanonicalProviderRef),
    /// A legacy provider id migrated deterministically to a canonical one. The
    /// migration is SURFACED (carries an FR-EVT-PROFILE- event ref) — never a
    /// silent in-place rewrite.
    Migrated {
        from: String,
        to: CanonicalProviderRef,
        event_ref: String,
    },
    /// Not a canonical id and not a known legacy alias — unresolvable. Callers
    /// must surface this as a validation error, never silently coerce it.
    Unknown(String),
}

impl ProviderRefResolution {
    /// The canonical provider id this route resolves to, if any (both the
    /// already-canonical and the migrated cases). `None` for `Unknown`.
    pub fn canonical(&self) -> Option<CanonicalProviderRef> {
        match self {
            Self::Canonical(id) => Some(*id),
            Self::Migrated { to, .. } => Some(*to),
            Self::Unknown(_) => None,
        }
    }

    /// Whether this resolution migrated a legacy id (and therefore must be
    /// surfaced via an FR-EVT-PROFILE event).
    pub fn is_migration(&self) -> bool {
        matches!(self, Self::Migrated { .. })
    }
}

/// Resolves a Work Profile role-route `provider_ref` against the canonical
/// provider id set, migrating the retired `ollama` daemon id to `local_runtime`
/// (the Ollama-as-primary architecture was retired; the embedded ModelRuntime is
/// the local authority). Deterministic and pure: a migration is surfaced in the
/// returned [`ProviderRefResolution::Migrated`] (carrying an FR-EVT-PROFILE-
/// event ref) rather than rewritten in place, and an unrecognized id resolves to
/// [`ProviderRefResolution::Unknown`] (never silently coerced to a default).
pub fn resolve_provider_ref(provider_ref: &str) -> ProviderRefResolution {
    let trimmed = provider_ref.trim();
    match trimmed {
        "local_runtime" => ProviderRefResolution::Canonical(CanonicalProviderRef::LocalRuntime),
        "openai_compat" => ProviderRefResolution::Canonical(CanonicalProviderRef::OpenAiCompat),
        // Legacy daemon id retired with the Ollama-as-primary architecture.
        "ollama" => ProviderRefResolution::Migrated {
            from: trimmed.to_string(),
            to: CanonicalProviderRef::LocalRuntime,
            event_ref: format!("{PROVIDER_REF_MIGRATION_FR_EVENT}:ollama->local_runtime"),
        },
        other => ProviderRefResolution::Unknown(other.to_string()),
    }
}

/// Emits a surfaced FR-EVT-PROFILE Flight Recorder event recording a
/// deterministic provider_ref migration (MT-014). Only a
/// [`ProviderRefResolution::Migrated`] is recorded; canonical/unknown
/// resolutions are no-ops (return `Ok(())` without emitting). A recorder failure
/// is returned to the caller (never silently swallowed) so the migration audit
/// is honest.
pub async fn record_provider_ref_migration(
    recorder: &dyn FlightRecorder,
    profile_id: &str,
    role_id: &str,
    resolution: &ProviderRefResolution,
) -> Result<(), RecorderError> {
    let ProviderRefResolution::Migrated {
        from,
        to,
        event_ref,
    } = resolution
    else {
        return Ok(());
    };
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({
            "fr_event": PROVIDER_REF_MIGRATION_FR_EVENT,
            "type": "work_profile_provider_ref_migrated",
            "event_ref": event_ref,
            "profile_id": profile_id,
            "role_id": role_id,
            "from": from,
            "to": to.as_str(),
        }),
    );
    recorder.record_event(event).await
}

/// A failure from the provider_ref-aware Work Profile load path (MT-014). Kept
/// typed and distinct so a structural/contract failure is never conflated with
/// an infrastructure failure to surface the migration audit event.
#[derive(Debug)]
pub enum WorkProfileLoadError {
    /// The registry failed structural/contract validation. This includes an
    /// unresolvable `provider_ref` dangle rejected by `validate_work_profiles`.
    Validation(Vec<WorkProfileValidationError>),
    /// A deterministic provider_ref migration could not be SURFACED to the
    /// Flight Recorder. The load fails closed rather than applying an unsurfaced
    /// (silent) rewrite.
    Recorder(RecorderError),
}

/// Validates a Work Profile registry AND resolves every role-route `provider_ref`
/// through the canonical provider contract, returning a registry whose
/// provider_refs are all canonical (MT-014, AC-3: "resolves or is migrated
/// deterministically, no silent dangle").
///
/// This is the live provider_ref-aware profile-load path:
/// * structural/contract validation runs first (`validate_work_profiles`), which
///   already rejects an unresolvable `Unknown` provider_ref;
/// * each retired `ollama` alias is migrated to `local_runtime`, SURFACED via a
///   real FR-EVT-PROFILE Flight Recorder event (`record_provider_ref_migration`)
///   and then APPLIED to the returned registry — never a silent in-place rewrite;
/// * canonical provider_refs pass through unchanged and emit no event.
///
/// A recorder failure fails the load (`WorkProfileLoadError::Recorder`) instead
/// of applying an unsurfaced migration.
pub async fn validate_and_migrate_work_profiles(
    registry: &WorkProfileRegistryV1,
    recorder: &dyn FlightRecorder,
) -> Result<WorkProfileRegistryV1, WorkProfileLoadError> {
    validate_work_profiles(registry).map_err(WorkProfileLoadError::Validation)?;

    let mut migrated = registry.clone();
    for profile in &mut migrated.profiles {
        let profile_id = profile.profile_id.clone();
        for route in &mut profile.role_routes {
            let resolution = resolve_provider_ref(&route.provider_ref);
            match &resolution {
                ProviderRefResolution::Canonical(_) => {}
                ProviderRefResolution::Migrated { to, .. } => {
                    // Surface the migration BEFORE applying it, so an audit event
                    // exists for every applied rewrite.
                    record_provider_ref_migration(
                        recorder,
                        &profile_id,
                        &route.role_id,
                        &resolution,
                    )
                    .await
                    .map_err(WorkProfileLoadError::Recorder)?;
                    route.provider_ref = to.as_str().to_string();
                }
                ProviderRefResolution::Unknown(_) => {
                    // Unreachable in practice: `validate_work_profiles` above
                    // already rejects an Unknown provider_ref. Kept as a typed
                    // fail-closed rather than a silent coerce, in case structural
                    // validation is ever relaxed.
                    return Err(WorkProfileLoadError::Validation(vec![
                        WorkProfileValidationError {
                            field: "profiles.role_routes.provider_ref",
                            message:
                                "provider_ref must resolve to a canonical provider id or a known migratable alias",
                        },
                    ]));
                }
            }
        }
    }
    Ok(migrated)
}
