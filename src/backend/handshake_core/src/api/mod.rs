use std::sync::Arc;
use account_scope::ProductLocalResourceScope;

pub struct ApiRoutes {
    pub router: Router,
    pub runtime: ApiRouteRuntime,
}

#[derive(Clone)]
pub struct ApiRouteRuntime {
    shared_process_runtime: Option<crate::process_ledger::ProcessReclaimRuntime>,
    operator_chat_process_ledger: Option<crate::process_ledger::RetainedLedgerBatcher>,
    process_reclaim_task: Option<crate::process_ledger::ManagedStalenessReclaimTask>,
    swarm_event_drain: Option<crate::swarm_orchestration::events::DurableSwarmFrDrain>,
    operator_chat_launch_service: Option<Arc<crate::swarm_orchestration::operator_chat::OperatorChatLaunchService>>,
    coordinator_shutdown: Arc<tokio::sync::OnceCell<Result<(), String>>>,
    _standalone_runtime_lease: Option<Arc<crate::process_ledger::EmbeddedRuntimeInstanceLease>>,
}

#[derive(Debug)]
pub struct ApiRouteRuntimeDrainReport {
    pub operator_chat_process_ledger: Option<crate::process_ledger::LedgerDrainJoinOutcome>,
    pub process_reclaim_quiesced: bool,
    pub swarm_events_flushed: bool,
    pub coordinator_quiesced: bool,
    pub coordinator_error: Option<String>,
    pub lease_released: bool,
    pub lease_retained_reason: Option<String>,
}

impl ApiRouteRuntime {
    pub async fn quiesce_swarm(&self, timeout: std::time::Duration) -> Result<(), String> {
        self.coordinator_shutdown.get_or_init(|| async {
            match &self.operator_chat_launch_service {
                Some(service) => match tokio::time::timeout(timeout, service.coordinator().drain_all()).await {
                    Ok(result) => result.map_err(|error| error.to_string()),
                    Err(_) => Err("coordinator drain deadline elapsed; retirement is unproven".to_owned()),
                },
                None => Ok(()),
            }
        }).await.clone()
    }

    pub async fn drain_and_join(&self, timeout: std::time::Duration) -> ApiRouteRuntimeDrainReport {
        let coordinator_error = self.quiesce_swarm(timeout).await.err();
        if coordinator_error.is_some() {
            return ApiRouteRuntimeDrainReport {
                operator_chat_process_ledger: None,
                process_reclaim_quiesced: false,
                swarm_events_flushed: false,
                coordinator_quiesced: false,
                coordinator_error,
                lease_released: false,
                lease_retained_reason: Some("coordinator retirement is unproven; subsequent drains were not run".to_owned()),
            };
        }
        let swarm_events_flushed = match &self.swarm_event_drain {
            Some(drain) => drain.drain_and_join(timeout).await,
            None => true,
        };
        if let Some(runtime) = &self.shared_process_runtime {
            let report = runtime.shutdown_and_drain(timeout).await;
            return ApiRouteRuntimeDrainReport {
                process_reclaim_quiesced: report.reclaim_task_quiesced,
                operator_chat_process_ledger: Some(report.ledger),
                swarm_events_flushed,
                coordinator_quiesced: true,
                coordinator_error: None,
                lease_released: report.lease_released,
                lease_retained_reason: report.lease_retained_reason,
            };
        }
        let process_reclaim_quiesced = match &self.process_reclaim_task {
            Some(task) => task.shutdown_and_join(timeout).await,
            None => true,
        };
        ApiRouteRuntimeDrainReport {
            process_reclaim_quiesced,
            swarm_events_flushed,
            coordinator_quiesced: true,
            coordinator_error: None,
            lease_released: false,
            lease_retained_reason: Some("shared runtime lease-release proof is unavailable".to_owned()),
            operator_chat_process_ledger: Some(self
                .operator_chat_process_ledger
                .as_ref()
                .map(|ledger| ledger.clone())
                .expect("standalone API runtime must retain its process ledger")
                .drain_and_join(timeout)
                .await),
        }
    }
}

pub fn routes_with_process_reclaim_runtime(
    state: AppState,
    process_runtime: crate::process_ledger::ProcessReclaimRuntime,
    product_scope: ProductLocalResourceScope,
    model_access_state: model_access::ModelAccessState,
) -> ApiRoutes {
    let palmistry_state = palmistry::PalmistryLaunchState::new(
        process_runtime.ledger().ledger(),
        state.flight_recorder.clone(),
        state.surreal.clone(),
        product_scope.exact().clone(),
        process_runtime.reclaim(),
    ).with_capability_registry(state.capability_registry.clone());
    ApiRoutes {
        router: routes(state.clone())
            .merge(model_access::authenticated_routes(model_access_state, state.clone()))
            .merge(palmistry::receipt_routes(palmistry_state))
            .layer(axum::Extension(product_scope))
            .layer(axum::Extension(state.surreal.resource_access_lifecycle())),
        runtime: ApiRouteRuntime {
            shared_process_runtime: Some(process_runtime),
            operator_chat_process_ledger: None,
            process_reclaim_task: None,
            swarm_event_drain: None,
            operator_chat_launch_service: None,
            coordinator_shutdown: Arc::new(tokio::sync::OnceCell::new()),
            _standalone_runtime_lease: None,
        },
    }
}

/// Compose the real scoped launch service and retain its paired durable drain.
pub fn routes_with_scoped_operator_chat(
    state: AppState,
    process_runtime: crate::process_ledger::ProcessReclaimRuntime,
    product_scope: ProductLocalResourceScope,
    model_access_state: model_access::ModelAccessState,
    operator_chat_state: operator_chat::OperatorChatState,
    launch_service: Arc<crate::swarm_orchestration::operator_chat::OperatorChatLaunchService>,
    swarm_event_drain: crate::swarm_orchestration::events::DurableSwarmFrDrain,
) -> ApiRoutes {
    let operator_chat_state = operator_chat_state
        .with_account_bootstrap(crate::AccountBootstrapState::from(&state));
    let operator_routes = operator_chat::scoped_routes(operator_chat_state)
        .layer(axum::Extension(product_scope.clone()))
        .layer(axum::Extension(state.surreal.resource_access_lifecycle()));
    let mut routes = routes_with_process_reclaim_runtime(
        state, process_runtime, product_scope, model_access_state,
    );
    routes.router = routes.router.merge(operator_routes);
    routes.runtime.swarm_event_drain = Some(swarm_event_drain);
    routes.runtime.operator_chat_launch_service = Some(launch_service);
    routes
}

use axum::{routing::get, Router};

use crate::AppState;

pub mod account_scope;
pub mod atelier;
pub mod atelier_ckc_intake_facial;
pub mod atelier_ckc_media;
pub mod atelier_ckc_prompt_feedback;
pub mod atelier_ckc_ops;
pub mod atelier_ckc_sheets;
pub mod authority;
pub mod bundles;
pub mod calendar;
pub mod canvases;
pub mod code_nav_index;
pub mod console_stream;
pub mod debug_adapter;
pub mod diagnostics;
pub mod flight_recorder;
pub mod governance_pack;
pub mod jobs;
pub mod kernel;
pub mod knowledge_code_nav;
pub mod knowledge_crdt;
pub mod knowledge_documents;
pub mod knowledge_ingestion;
pub mod knowledge_memory;
pub mod knowledge_retrieval;
pub mod locus;
pub mod logs;
pub mod loom;
pub mod memory;
pub mod model_access;
pub mod model_lane_navigation;
pub mod model_runtime_registry;
pub mod operator_chat;
pub mod palmistry;
pub mod paths;
pub mod preferences;
pub mod role_mailbox;
pub mod source_control;
pub mod stage;
pub mod user_manual;
pub mod workspace_activation_readiness;
pub mod workspaces;

pub fn routes(state: AppState) -> Router {
    let authority_routes = authority::routes(state.clone());
    let navigation_routes = model_lane_navigation::routes(state.clone());
    let model_runtime_routes = model_runtime_registry::routes(
        model_runtime_registry::ModelRuntimeRegistryApiState::new(
            state.surreal.clone(),
            state.llm_client.clone(),
        ),
    );
    let workspace_routes = workspaces::routes(state.clone());
    let preferences_routes = preferences::routes(state.clone());
    let calendar_routes = calendar::routes(state.clone());
    let code_nav_index_routes = code_nav_index::routes(state.clone());
    let console_stream_routes = console_stream::routes(crate::console_stream::ConsoleBroadcast::shared());
    let locus_routes = locus::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());
    let job_routes = jobs::routes(state.clone());
    let loom_routes = loom::routes(state.clone());
    let flight_recorder_routes = flight_recorder::routes(state.clone());
    let diagnostics_routes = diagnostics::routes(state.clone());
    let bundle_routes = bundles::routes(state.clone());
    let governance_pack_routes = governance_pack::routes(state.clone());
    let role_mailbox_routes = role_mailbox::routes(state.clone());
    let kernel_routes = kernel::routes(state.clone());
    let knowledge_code_nav_routes = knowledge_code_nav::routes(state.clone());
    let knowledge_crdt_routes = knowledge_crdt::routes(state.clone());
    let knowledge_documents_routes = knowledge_documents::routes(state.clone());
    let knowledge_ingestion_routes = knowledge_ingestion::routes(state.clone());
    let knowledge_memory_routes = knowledge_memory::routes(state.clone());
    let knowledge_retrieval_routes = knowledge_retrieval::routes(state.clone());
    let memory_routes = memory::routes(state.clone());
    let user_manual_routes = user_manual::routes(state.clone());
    let atelier_routes = atelier::routes(state.clone())
        .merge(atelier_ckc_media::routes(state.clone()))
        .merge(atelier_ckc_prompt_feedback::routes(state.clone()))
        .merge(atelier_ckc_ops::routes(state.clone()))
        .merge(atelier_ckc_sheets::routes(state.clone()));
    let source_control_routes = source_control::routes(state.clone());
    let stage_routes = stage::routes(state.clone());
    let debug_adapter_routes = debug_adapter::routes(state.clone());
    let log_routes = Router::new()
        .route("/logs/tail", get(logs::tail_logs))
        .with_state(state.clone());

    authority_routes
        .merge(navigation_routes)
        .merge(model_runtime_routes)
        .merge(workspace_routes)
        .merge(preferences_routes)
        .merge(calendar_routes)
        .merge(code_nav_index_routes)
        .merge(console_stream_routes)
        .merge(locus_routes)
        .merge(canvas_routes)
        .merge(log_routes)
        .merge(job_routes)
        .merge(loom_routes)
        .merge(diagnostics_routes)
        .merge(flight_recorder_routes)
        .merge(bundle_routes)
        .merge(governance_pack_routes)
        .merge(role_mailbox_routes)
        .merge(kernel_routes)
        .merge(knowledge_code_nav_routes)
        .merge(knowledge_crdt_routes)
        .merge(knowledge_documents_routes)
        .merge(knowledge_ingestion_routes)
        .merge(knowledge_memory_routes)
        .merge(knowledge_retrieval_routes)
        .merge(memory_routes)
        .merge(user_manual_routes)
        .merge(atelier_routes)
        .merge(source_control_routes)
        .merge(stage_routes)
        .merge(debug_adapter_routes)
}

#[cfg(test)]
pub(crate) trait MountedRequestExt {
    fn oneshot(self, request: axum::http::Request<axum::body::Body>)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<axum::response::Response, std::convert::Infallible>> + Send>>;
}

#[cfg(test)]
impl MountedRequestExt for axum::Router {
    fn oneshot(self, request: axum::http::Request<axum::body::Body>)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<axum::response::Response, std::convert::Infallible>> + Send>> {
        async fn invoke<S>(mut service: S, request: axum::http::Request<axum::body::Body>)
            -> Result<S::Response, S::Error>
        where S: axum::ServiceExt<axum::http::Request<axum::body::Body>> {
            std::future::poll_fn(|context| service.poll_ready(context)).await?;
            service.call(request).await
        }
        Box::pin(invoke(self, request))
    }
}
