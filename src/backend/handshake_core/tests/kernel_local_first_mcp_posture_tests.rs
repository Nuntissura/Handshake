use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    local_first_mcp_posture::{
        project_local_first_mcp_posture, validate_local_first_mcp_posture, AdapterArtifactCacheV1,
        AdapterCapabilityGateV1, AdapterExecutionPathKind, AdapterFallbackPolicyV1,
        LocalFirstAgenticMcpPostureV1, LocalFirstExecutionRouteV1,
    },
};

#[test]
fn kernel_local_first_mcp_posture_defaults_to_local_execution() {
    let posture = sample_posture();

    validate_local_first_mcp_posture(&posture).expect("posture validates");

    assert!(posture.local_first_default);
    assert!(!posture.mcp_core_dependency_allowed);
    assert!(posture.routes.iter().any(
        |route| route.path_kind == AdapterExecutionPathKind::Local && route.selected_by_default
    ));
}

#[test]
fn kernel_local_first_mcp_posture_projects_gated_cached_remote_adapters() {
    let posture = sample_posture();
    let projection = project_local_first_mcp_posture(&posture).expect("projection builds");

    assert!(projection.local_first_default);
    assert!(!projection.core_depends_on_mcp);
    assert_eq!(projection.route_count, 2);
    assert!(projection
        .remote_adapter_route_ids
        .contains(&"route-mcp-summary".to_string()));
    assert!(projection
        .gated_adapter_route_ids
        .contains(&"route-mcp-summary".to_string()));
    assert!(projection
        .cached_artifact_refs
        .contains(&"artifact-cache://mcp/summary".to_string()));
    assert!(projection
        .fallback_route_ids
        .contains(&"route-mcp-summary".to_string()));
    assert!(!projection.mutates_remote_state);
}

#[test]
fn kernel_local_first_mcp_posture_rejects_remote_dependency_without_controls() {
    let mut posture = sample_posture();
    posture.local_first_default = false;
    posture.mcp_core_dependency_allowed = true;
    posture.routes[1].capability_gates.clear();
    posture.routes[1].cached_artifact.cache_required = false;
    posture.routes[1].fallback.fallback_to_local = false;
    posture.routes[1].fallback.deterministic_fallback = false;
    posture.routes[1].fallback.degraded_marker = None;

    let errors =
        validate_local_first_mcp_posture(&posture).expect_err("unsafe MCP posture must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "local_first_default"));
    assert!(errors
        .iter()
        .any(|error| error.field == "mcp_core_dependency_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "routes.capability_gates"));
    assert!(errors
        .iter()
        .any(|error| error.field == "routes.cached_artifact.cache_required"));
    assert!(errors.iter().any(|error| error.field == "routes.fallback"));
    assert!(errors
        .iter()
        .any(|error| error.field == "routes.fallback.degraded_marker"));
}

#[test]
fn kernel_local_first_mcp_posture_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.local_first_mcp_posture.project")
        .expect("local-first MCP posture projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "local_first_default"));
}

fn sample_posture() -> LocalFirstAgenticMcpPostureV1 {
    LocalFirstAgenticMcpPostureV1 {
        schema_id: "hsk.kernel.local_first_mcp_posture@1".to_string(),
        posture_id: "local-first-mcp-mt039".to_string(),
        folded_stub_ids: vec!["WP-1-LocalFirst-Agentic-MCP-Posture-v1".to_string()],
        local_first_default: true,
        mcp_core_dependency_allowed: false,
        routes: vec![
            route("route-local-default", AdapterExecutionPathKind::Local, true),
            route(
                "route-mcp-summary",
                AdapterExecutionPathKind::McpAdapter,
                false,
            ),
        ],
        product_authority_refs: vec![
            "kernel.work_profiles".to_string(),
            "kernel.action_catalog".to_string(),
            "kernel.role_turn_isolation".to_string(),
            "flight_recorder.agentic_execution".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.contract.json"
                .to_string(),
        ],
    }
}

fn route(
    route_id: &str,
    path_kind: AdapterExecutionPathKind,
    selected_by_default: bool,
) -> LocalFirstExecutionRouteV1 {
    let remote = path_kind != AdapterExecutionPathKind::Local;
    LocalFirstExecutionRouteV1 {
        route_id: route_id.to_string(),
        action_id: "kernel.work_profiles.project".to_string(),
        path_kind,
        selected_by_default,
        adapter_ref: if remote {
            format!("adapter://{route_id}")
        } else {
            String::new()
        },
        capability_gates: if remote {
            vec![AdapterCapabilityGateV1 {
                capability_id: "mcp.remote.summary".to_string(),
                granted: true,
                audit_ref: "capability-audit://mcp.remote.summary".to_string(),
            }]
        } else {
            Vec::new()
        },
        cached_artifact: AdapterArtifactCacheV1 {
            cache_required: remote,
            cache_ref: if remote {
                "artifact-cache://mcp/summary".to_string()
            } else {
                String::new()
            },
            artifact_hash: if remote {
                "hash-mcp-summary".to_string()
            } else {
                String::new()
            },
        },
        fallback: AdapterFallbackPolicyV1 {
            fallback_to_local: true,
            local_fallback_route_id: "route-local-default".to_string(),
            deterministic_fallback: true,
            degraded_marker: if remote {
                Some("REMOTE_UNAVAILABLE_LOCAL_FALLBACK".to_string())
            } else {
                None
            },
        },
        flight_recorder_ref: format!("FR-EVT-AGENTIC-ROUTE-{route_id}"),
    }
}
