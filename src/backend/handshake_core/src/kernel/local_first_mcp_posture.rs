use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_LOCAL_FIRST_MCP_POSTURE_STUB_ID: &str = "WP-1-LocalFirst-Agentic-MCP-Posture-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdapterExecutionPathKind {
    Local,
    McpAdapter,
    CloudAdapter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapabilityGateV1 {
    pub capability_id: String,
    pub granted: bool,
    pub audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterArtifactCacheV1 {
    pub cache_required: bool,
    pub cache_ref: String,
    pub artifact_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterFallbackPolicyV1 {
    pub fallback_to_local: bool,
    pub local_fallback_route_id: String,
    pub deterministic_fallback: bool,
    pub degraded_marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFirstExecutionRouteV1 {
    pub route_id: String,
    pub action_id: String,
    pub path_kind: AdapterExecutionPathKind,
    pub selected_by_default: bool,
    pub adapter_ref: String,
    pub capability_gates: Vec<AdapterCapabilityGateV1>,
    pub cached_artifact: AdapterArtifactCacheV1,
    pub fallback: AdapterFallbackPolicyV1,
    pub flight_recorder_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFirstAgenticMcpPostureV1 {
    pub schema_id: String,
    pub posture_id: String,
    pub folded_stub_ids: Vec<String>,
    pub local_first_default: bool,
    pub mcp_core_dependency_allowed: bool,
    pub routes: Vec<LocalFirstExecutionRouteV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFirstMcpPostureProjectionV1 {
    pub schema_id: String,
    pub posture_id: String,
    pub local_first_default: bool,
    pub core_depends_on_mcp: bool,
    pub route_count: usize,
    pub remote_adapter_route_ids: Vec<String>,
    pub gated_adapter_route_ids: Vec<String>,
    pub cached_artifact_refs: Vec<String>,
    pub fallback_route_ids: Vec<String>,
    pub mutates_remote_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFirstMcpPostureValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_local_first_mcp_posture(
    posture: &LocalFirstAgenticMcpPostureV1,
) -> Result<(), Vec<LocalFirstMcpPostureValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &posture.schema_id);
    require_non_empty(&mut errors, "posture_id", &posture.posture_id);
    require_vec(&mut errors, "folded_stub_ids", &posture.folded_stub_ids);
    require_vec(&mut errors, "routes", &posture.routes);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &posture.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &posture.folded_source_refs,
    );

    if !contains_exact(
        &posture.folded_stub_ids,
        FOLDED_LOCAL_FIRST_MCP_POSTURE_STUB_ID,
    ) {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "folded_stub_ids",
            message: "local-first MCP posture must preserve the folded stub id",
        });
    }
    if !contains_text(
        &posture.folded_source_refs,
        FOLDED_LOCAL_FIRST_MCP_POSTURE_STUB_ID,
    ) {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "folded_source_refs",
            message: "local-first MCP posture must preserve the folded source reference",
        });
    }
    if !posture.local_first_default {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "local_first_default",
            message: "agentic execution must default to local-first routing",
        });
    }
    if posture.mcp_core_dependency_allowed {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "mcp_core_dependency_allowed",
            message: "MCP/cloud paths must stay adapter-only, not core dependencies",
        });
    }

    validate_authority_refs(&mut errors, posture);
    validate_routes(&mut errors, posture);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_local_first_mcp_posture(
    posture: &LocalFirstAgenticMcpPostureV1,
) -> Result<LocalFirstMcpPostureProjectionV1, Vec<LocalFirstMcpPostureValidationError>> {
    validate_local_first_mcp_posture(posture)?;

    let remote_routes: Vec<_> = posture
        .routes
        .iter()
        .filter(|route| route.path_kind != AdapterExecutionPathKind::Local)
        .collect();

    Ok(LocalFirstMcpPostureProjectionV1 {
        schema_id: "hsk.kernel.local_first_mcp_posture_projection@1".to_string(),
        posture_id: posture.posture_id.clone(),
        local_first_default: posture.local_first_default,
        core_depends_on_mcp: posture.mcp_core_dependency_allowed,
        route_count: posture.routes.len(),
        remote_adapter_route_ids: remote_routes
            .iter()
            .map(|route| route.route_id.clone())
            .collect(),
        gated_adapter_route_ids: remote_routes
            .iter()
            .filter(|route| route.capability_gates.iter().all(|gate| gate.granted))
            .map(|route| route.route_id.clone())
            .collect(),
        cached_artifact_refs: remote_routes
            .iter()
            .map(|route| route.cached_artifact.cache_ref.clone())
            .collect(),
        fallback_route_ids: remote_routes
            .iter()
            .filter(|route| route.fallback.fallback_to_local)
            .map(|route| route.route_id.clone())
            .collect(),
        mutates_remote_state: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    posture: &LocalFirstAgenticMcpPostureV1,
) {
    for required_ref in [
        "kernel.work_profiles",
        "kernel.action_catalog",
        "kernel.role_turn_isolation",
        "flight_recorder.agentic_execution",
    ] {
        if !contains_exact(&posture.product_authority_refs, required_ref) {
            errors.push(LocalFirstMcpPostureValidationError {
                field: "product_authority_refs",
                message: "local-first MCP posture must cite work profiles, action catalog, role-turn isolation, and execution recorder authorities",
            });
        }
    }
}

fn validate_routes(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    posture: &LocalFirstAgenticMcpPostureV1,
) {
    let mut route_ids = HashSet::new();
    let local_default_ids: HashSet<&str> = posture
        .routes
        .iter()
        .filter(|route| {
            route.path_kind == AdapterExecutionPathKind::Local && route.selected_by_default
        })
        .map(|route| route.route_id.as_str())
        .collect();

    if local_default_ids.is_empty() {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.selected_by_default",
            message: "at least one local route must be selected by default",
        });
    }

    for route in &posture.routes {
        if !route_ids.insert(route.route_id.as_str()) {
            errors.push(LocalFirstMcpPostureValidationError {
                field: "routes.route_id",
                message: "route ids must be unique",
            });
        }

        require_non_empty(errors, "routes.route_id", &route.route_id);
        require_non_empty(errors, "routes.action_id", &route.action_id);
        require_non_empty(
            errors,
            "routes.flight_recorder_ref",
            &route.flight_recorder_ref,
        );

        if route.path_kind == AdapterExecutionPathKind::Local {
            validate_local_route(errors, route);
        } else {
            validate_remote_adapter_route(errors, route, &local_default_ids);
        }
    }
}

fn validate_local_route(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    route: &LocalFirstExecutionRouteV1,
) {
    if !route.adapter_ref.is_empty() {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.adapter_ref",
            message: "local routes must not require adapter refs",
        });
    }
    if route.cached_artifact.cache_required {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.cached_artifact.cache_required",
            message: "local routes do not require remote artifact cache policy",
        });
    }
    if !route.fallback.fallback_to_local || !route.fallback.deterministic_fallback {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.fallback",
            message: "local routes must be deterministic local fallbacks for remote adapters",
        });
    }
}

fn validate_remote_adapter_route(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    route: &LocalFirstExecutionRouteV1,
    local_default_ids: &HashSet<&str>,
) {
    require_non_empty(errors, "routes.adapter_ref", &route.adapter_ref);
    require_vec(errors, "routes.capability_gates", &route.capability_gates);
    require_non_empty(
        errors,
        "routes.cached_artifact.cache_ref",
        &route.cached_artifact.cache_ref,
    );
    require_non_empty(
        errors,
        "routes.cached_artifact.artifact_hash",
        &route.cached_artifact.artifact_hash,
    );

    if route.selected_by_default {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.selected_by_default",
            message: "remote adapters must not be selected by default",
        });
    }
    if route.capability_gates.iter().any(|gate| !gate.granted) {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.capability_gates",
            message: "remote adapters require granted capabilities before routing",
        });
    }
    for gate in &route.capability_gates {
        require_non_empty(
            errors,
            "routes.capability_gates.capability_id",
            &gate.capability_id,
        );
        require_non_empty(errors, "routes.capability_gates.audit_ref", &gate.audit_ref);
    }
    if !route.cached_artifact.cache_required {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.cached_artifact.cache_required",
            message: "remote adapter outputs must be cached as artifacts where policy permits",
        });
    }
    if !route.fallback.fallback_to_local || !route.fallback.deterministic_fallback {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.fallback",
            message: "remote adapters require deterministic local fallback behavior",
        });
    }
    if !local_default_ids.contains(route.fallback.local_fallback_route_id.as_str()) {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.fallback.local_fallback_route_id",
            message: "remote adapter fallback must target a local default route",
        });
    }
    if route
        .fallback
        .degraded_marker
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        errors.push(LocalFirstMcpPostureValidationError {
            field: "routes.fallback.degraded_marker",
            message: "remote fallback must record an explicit degraded marker",
        });
    }
}

fn require_non_empty(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(LocalFirstMcpPostureValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<LocalFirstMcpPostureValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(LocalFirstMcpPostureValidationError {
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
