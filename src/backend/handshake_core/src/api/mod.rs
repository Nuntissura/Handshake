use std::sync::Arc;

use axum::{routing::get, Extension, Router};

use crate::AppState;

pub mod atelier;
pub mod bundles;
pub mod canvases;
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
pub mod logs;
pub mod loom;
pub mod model_access;
pub mod model_lane_navigation;
pub mod operator_chat;
pub mod paths;
pub mod role_mailbox;
pub mod source_control;
pub mod user_manual;
pub mod workspaces;

pub fn routes(state: AppState) -> Router {
    let ApiRoutes { router, runtime } = routes_with_runtime(state);
    router.layer(Extension(runtime))
}

pub struct ApiRoutes {
    pub router: Router,
    pub runtime: ApiRouteRuntime,
}

#[derive(Clone)]
pub struct ApiRouteRuntime {
    operator_chat_process_ledger: crate::process_ledger::RetainedLedgerBatcher,
}

#[derive(Debug)]
pub struct ApiRouteRuntimeDrainReport {
    pub operator_chat_process_ledger: crate::process_ledger::LedgerDrainJoinOutcome,
}

impl ApiRouteRuntime {
    pub async fn drain_and_join(&self, timeout: std::time::Duration) -> ApiRouteRuntimeDrainReport {
        ApiRouteRuntimeDrainReport {
            operator_chat_process_ledger: self
                .operator_chat_process_ledger
                .drain_and_join(timeout)
                .await,
        }
    }
}

pub fn routes_with_runtime(state: AppState) -> ApiRoutes {
    let operator_chat_process_ledger = crate::process_ledger::RetainedLedgerBatcher::spawn(
        Arc::new(crate::process_ledger::PostgresProcessLedgerStore::new(
            state.postgres_pool.clone(),
        )),
        Arc::new(crate::process_ledger::NoopOverflowSink),
        crate::process_ledger::LedgerBatcherConfig::default(),
    );
    let router = routes_with_operator_chat_runtime(state, operator_chat_process_ledger.clone());
    ApiRoutes {
        router,
        runtime: ApiRouteRuntime {
            operator_chat_process_ledger,
        },
    }
}

fn routes_with_operator_chat_runtime(
    state: AppState,
    operator_chat_process_ledger: crate::process_ledger::RetainedLedgerBatcher,
) -> Router {
    let workspace_routes = workspaces::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());
    let job_routes = jobs::routes(state.clone());
    let loom_routes = loom::routes(state.clone());
    let flight_recorder_routes = flight_recorder::routes(state.clone());
    let diagnostics_routes = diagnostics::routes(state.clone());
    let model_lane_navigation_routes = model_lane_navigation::routes(state.clone());
    // MT-015: the model-access router owns a dedicated state (the
    // CloudAccessProvider seam), not `AppState`, so it is route-testable without
    // a full AppState. Production wires the OS-keychain-backed service.
    let model_access_routes = model_access::routes(model_access::ModelAccessState::production());
    // MT-012 (F3): wire the LIVE operator chat/launch surface from AppState. The
    // launch service is backed by a real `SwarmCoordinator` + `ModelLaneStore`
    // (the shared PostgreSQL pool), so `POST /operator-chat/launch` performs a real
    // launch (never an inert `503 launch_not_wired`) and `GET
    // /operator-chat/transcript/:run_id` reads captured ModelLaneMessage rows.
    // Cloud/official-CLI lanes are wired from the same production access service
    // the Settings surface writes into. Missing OS-keychain support, absent BYOK
    // keys, or missing CLI executables degrade to unavailable/not-configured,
    // never placeholders. Local lanes launch through the real candle/llama path.
    let operator_chat_catalog = state
        .model_catalog()
        .unwrap_or_else(crate::model_runtime::catalog::ModelCatalog::empty);
    let operator_chat_launch_service =
        crate::swarm_orchestration::production_factory::build_operator_chat_launch_service(
            state.postgres_pool.clone(),
            state.flight_recorder.clone(),
            operator_chat_catalog.clone(),
            operator_chat_process_ledger.clone(),
            operator_chat_cloud_factory(
                state.flight_recorder.clone(),
                operator_chat_process_ledger.ledger(),
            ),
            uuid::Uuid::new_v4(),
        );
    let operator_chat_cloud_registry = operator_chat_cloud_registry();
    let mut operator_chat_state = operator_chat::OperatorChatState::production()
        .with_launch_service(operator_chat_launch_service)
        .with_catalog(operator_chat_catalog)
        .with_cloud_registry(operator_chat_cloud_registry)
        .with_recorder(state.flight_recorder.clone());
    for (provider, status) in operator_chat_cli_bridge_statuses() {
        operator_chat_state = operator_chat_state.with_cli_bridge_provider_status(provider, status);
    }
    let operator_chat_routes = operator_chat::routes(operator_chat_state);
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
    let user_manual_routes = user_manual::routes(state.clone());
    let atelier_routes = atelier::routes(state.clone());
    let source_control_routes = source_control::routes(state.clone());
    let debug_adapter_routes = debug_adapter::routes(state.clone());
    let log_routes = Router::new()
        .route("/logs/tail", get(logs::tail_logs))
        .with_state(state.clone());

    workspace_routes
        .merge(canvas_routes)
        .merge(log_routes)
        .merge(job_routes)
        .merge(loom_routes)
        .merge(diagnostics_routes)
        .merge(model_lane_navigation_routes)
        .merge(model_access_routes)
        .merge(operator_chat_routes)
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
        .merge(user_manual_routes)
        .merge(atelier_routes)
        .merge(source_control_routes)
        .merge(debug_adapter_routes)
}

fn operator_chat_cloud_registry() -> Arc<dyn crate::model_runtime::cloud::ProviderAccessRegistry> {
    match crate::model_runtime::cloud::CloudModelAccess::production() {
        Ok(access) => Arc::new(access.registry()),
        Err(_) => Arc::new(crate::model_runtime::cloud::InMemoryAccessRegistry::new()),
    }
}

fn operator_chat_cli_bridge_statuses() -> Vec<(
    &'static str,
    crate::model_runtime::cloud::ProviderAccessStatus,
)> {
    let cloud_access_available =
        crate::model_runtime::cloud::CloudModelAccess::production().is_ok();
    crate::model_runtime::cloud::CliBridgeProvider::OFFERED
        .into_iter()
        .map(|provider| {
            let status =
                if cloud_access_available && cli_bridge_config_from_path(provider).is_some() {
                    crate::model_runtime::cloud::ProviderAccessStatus::Configured
                } else {
                    crate::model_runtime::cloud::ProviderAccessStatus::Unavailable
                };
            (provider.id(), status)
        })
        .collect()
}

fn operator_chat_cloud_factory(
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    ledger: crate::process_ledger::LedgerBatcher,
) -> crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig {
    let Ok(access) = crate::model_runtime::cloud::CloudModelAccess::production() else {
        return crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig::unconfigured();
    };
    let vault = access.vault();
    let mut cloud =
        crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig::from_vault(
            vault,
            Some(
                crate::model_runtime::cloud::ByokProvider::Anthropic
                    .vault_lane()
                    .to_string(),
            ),
            Some(
                crate::model_runtime::cloud::ByokProvider::OpenAi
                    .vault_lane()
                    .to_string(),
            ),
        );

    let spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner> = Arc::new(
        crate::model_runtime::cloud::LiveCliSpawner::new(Arc::new(ledger)),
    );
    let observability = Some(Arc::new(
        crate::model_runtime::cloud::CloudLaneObservability {
            flight_recorder: recorder,
            consent: None,
        },
    ));

    for provider in crate::model_runtime::cloud::CliBridgeProvider::OFFERED {
        if let Some(config) = cli_bridge_config_from_path(provider) {
            cloud = cloud.with_official_cli_provider_replay_captured(
                provider.id(),
                spawner.clone(),
                config,
                observability.clone(),
            );
        }
    }
    cloud
}

fn cli_bridge_config_from_path(
    provider: crate::model_runtime::cloud::CliBridgeProvider,
) -> Option<crate::model_runtime::cloud::CliBridgeConfig> {
    let command = provider.login_command();
    let executable_path = find_executable_on_path(command.program)?;
    let (cli_kind, args_template) = match provider {
        crate::model_runtime::cloud::CliBridgeProvider::ClaudeCode => (
            crate::model_runtime::cloud::CliKind::ClaudeCode,
            vec!["-p".to_string(), "{prompt}".to_string()],
        ),
        crate::model_runtime::cloud::CliBridgeProvider::Codex => (
            crate::model_runtime::cloud::CliKind::CodexCli,
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "{prompt}".to_string(),
            ],
        ),
    };
    Some(
        crate::swarm_orchestration::operator_chat::force_json_stream_output(
            crate::model_runtime::cloud::CliBridgeConfig {
                cli_kind,
                executable_path,
                args_template,
                output_format: crate::model_runtime::cloud::CliOutputFormat::JsonStream,
                env_vars: std::collections::HashMap::new(),
                working_dir: None,
                timeout_seconds: 600,
            },
        ),
    )
}

fn find_executable_on_path(program: &str) -> Option<std::path::PathBuf> {
    let candidates = executable_candidates(program);
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .flat_map(|dir| candidates.iter().map(move |candidate| dir.join(candidate)))
        .find(|path| path.is_file())
}

fn executable_candidates(program: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        let mut candidates = Vec::new();
        let has_extension = std::path::Path::new(program).extension().is_some();
        if has_extension {
            candidates.push(program.to_string());
        } else {
            let path_ext = std::env::var_os("PATHEXT")
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
            for ext in path_ext.split(';').filter(|ext| !ext.trim().is_empty()) {
                candidates.push(format!("{program}{}", ext.to_ascii_lowercase()));
                candidates.push(format!("{program}{}", ext.to_ascii_uppercase()));
            }
            candidates.push(program.to_string());
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![program.to_string()]
    }
}
