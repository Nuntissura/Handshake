use std::sync::Arc;

use axum::{routing::get, Extension, Router};

use crate::AppState;

#[allow(dead_code)]
fn assert_axum_route_states_are_clone_send_sync_static() {
    fn assert_state<T: Clone + Send + Sync + 'static>() {}
    assert_state::<AppState>();
    assert_state::<operator_chat::OperatorChatState>();
}

pub mod account_scope;
pub mod atelier;
pub mod bundles;
pub mod canvases;
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
pub mod logs;
pub mod loom;
pub mod model_access;
pub mod model_lane_navigation;
pub mod model_runtime_registry;
pub mod operator_chat;
pub mod palmistry;
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
    shared_process_runtime: Option<crate::process_ledger::ProcessReclaimRuntime>,
    operator_chat_process_ledger: Option<crate::process_ledger::RetainedLedgerBatcher>,
    process_reclaim_task: Option<crate::process_ledger::ManagedStalenessReclaimTask>,
    _legacy_runtime_lease: Option<Arc<crate::process_ledger::EmbeddedRuntimeInstanceLease>>,
}

#[derive(Debug)]
pub struct ApiRouteRuntimeDrainReport {
    pub operator_chat_process_ledger: crate::process_ledger::LedgerDrainJoinOutcome,
    pub process_reclaim_quiesced: bool,
}

impl ApiRouteRuntime {
    pub async fn drain_and_join(&self, timeout: std::time::Duration) -> ApiRouteRuntimeDrainReport {
        if let Some(runtime) = &self.shared_process_runtime {
            let report = runtime.shutdown_and_drain(timeout).await;
            return ApiRouteRuntimeDrainReport {
                process_reclaim_quiesced: report.reclaim_task_quiesced,
                operator_chat_process_ledger: report.ledger,
            };
        }
        let process_reclaim_quiesced = match &self.process_reclaim_task {
            Some(task) => task.shutdown_and_join(timeout).await,
            None => true,
        };
        ApiRouteRuntimeDrainReport {
            process_reclaim_quiesced,
            operator_chat_process_ledger: self
                .operator_chat_process_ledger
                .as_ref()
                .map(|ledger| ledger.clone())
                .expect("legacy API runtime must retain its process ledger")
                .drain_and_join(timeout)
                .await,
        }
    }
}

pub fn routes_with_runtime(state: AppState) -> ApiRoutes {
    // One process-wide owner registry is shared by launch and crash recovery;
    // reclaim must dispatch through the same SandboxAdapter authority that
    // created an official-CLI child.
    let cli_sandbox_registry = crate::process_ledger::production_process_sandbox_registry();
    let legacy_runtime_lease = Arc::new(
        crate::process_ledger::acquire_embedded_runtime_instance_lease(
            uuid::Uuid::now_v7(),
            "legacy-api-route-fixture-host",
        )
        .expect("legacy API route runtime must acquire an OS liveness lease"),
    );
    let operator_chat_process_ledger =
        crate::process_ledger::RetainedLedgerBatcher::spawn_with_runtime_owner(
            Arc::new(crate::process_ledger::PostgresProcessLedgerStore::new(
                state.postgres_pool.clone(),
            )),
            Arc::new(crate::process_ledger::NoopOverflowSink),
            crate::process_ledger::LedgerBatcherConfig::default(),
            legacy_runtime_lease.descriptor().process_runtime_owner(),
        );
    let reclaim_store = Arc::new(crate::process_ledger::PostgresProcessLedgerStore::new(
        state.postgres_pool.clone(),
    ));
    let reclaim_killer = Arc::new(crate::process_ledger::ProductionSandboxKill::with_registry(
        state.postgres_pool.clone(),
        Arc::clone(&cli_sandbox_registry),
    ));
    let reclaim = Arc::new(crate::process_ledger::Reclaim::new(
        reclaim_store,
        reclaim_killer,
        Arc::new(operator_chat_process_ledger.ledger()),
    ));
    let stale_source = Arc::new(
        crate::process_ledger::PostgresModelLaneStaleSessionSource::new(
            state.postgres_pool.clone(),
            legacy_runtime_lease.descriptor().clone(),
        ),
    );
    let process_reclaim_task = crate::process_ledger::spawn_managed_staleness_reclaim_task(
        Arc::clone(&reclaim),
        stale_source,
        crate::process_ledger::StalenessReclaimConfig::default(),
    );
    let router = routes_with_operator_chat_runtime(
        state,
        operator_chat_process_ledger.clone(),
        Arc::clone(&reclaim),
        cli_sandbox_registry,
    );
    ApiRoutes {
        router,
        runtime: ApiRouteRuntime {
            shared_process_runtime: None,
            operator_chat_process_ledger: Some(operator_chat_process_ledger),
            process_reclaim_task: Some(process_reclaim_task),
            _legacy_runtime_lease: Some(legacy_runtime_lease),
        },
    }
}

pub fn routes_with_process_reclaim_runtime(
    state: AppState,
    process_runtime: crate::process_ledger::ProcessReclaimRuntime,
) -> ApiRoutes {
    let operator_chat_process_ledger = process_runtime.ledger();
    let router = routes_with_operator_chat_runtime(
        state,
        operator_chat_process_ledger,
        process_runtime.reclaim(),
        process_runtime.sandbox_registry(),
    );
    ApiRoutes {
        router,
        runtime: ApiRouteRuntime {
            shared_process_runtime: Some(process_runtime),
            operator_chat_process_ledger: None,
            process_reclaim_task: None,
            _legacy_runtime_lease: None,
        },
    }
}

fn routes_with_operator_chat_runtime(
    state: AppState,
    operator_chat_process_ledger: crate::process_ledger::RetainedLedgerBatcher,
    reclaim: Arc<crate::process_ledger::Reclaim>,
    cli_sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
) -> Router {
    let workspace_routes = workspaces::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());
    let job_routes = jobs::routes(state.clone());
    let loom_routes = loom::routes(state.clone());
    let flight_recorder_routes = flight_recorder::routes(state.clone());
    let diagnostics_routes = diagnostics::routes(state.clone());
    let model_lane_navigation_routes = model_lane_navigation::routes(state.clone());
    let model_runtime_registry_routes = model_runtime_registry::routes(state.clone());
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
    let operator_chat_cloud_wiring = operator_chat_cloud_wiring(
        state.flight_recorder.clone(),
        operator_chat_process_ledger.ledger(),
        cli_sandbox_registry,
        Arc::clone(&reclaim),
    );
    // MT-015: Settings and the picker share the exact canonical, pinned CLI
    // targets accepted by the launch factory. Neither surface independently
    // rediscovers or executes PATH candidates.
    let cli_auth_probe: Arc<dyn crate::model_runtime::cloud::CliBridgeAuthStatusProbe> =
        Arc::new(operator_chat_cloud_wiring.auth_probe.clone());
    let cli_login_launcher: Arc<dyn crate::model_runtime::cloud::CliBridgeLoginLauncher> =
        Arc::new(operator_chat_cloud_wiring.auth_probe.clone());
    let model_access_routes =
        model_access::routes(model_access::ModelAccessState::production_with_cli_runtime(
            cli_auth_probe.clone(),
            cli_login_launcher,
        ));
    let operator_chat_launch_service =
        crate::swarm_orchestration::production_factory::build_operator_chat_launch_service(
            state.postgres_pool.clone(),
            state.flight_recorder.clone(),
            operator_chat_catalog.clone(),
            operator_chat_process_ledger.clone(),
            Arc::clone(&reclaim),
            operator_chat_cloud_wiring.factory,
            uuid::Uuid::new_v4(),
        );
    let operator_chat_cloud_registry = operator_chat_cloud_registry();
    let operator_chat_state = operator_chat::OperatorChatState::production()
        .with_launch_service(operator_chat_launch_service)
        .with_catalog(operator_chat_catalog)
        .with_cloud_registry(operator_chat_cloud_registry)
        .with_session_registry(state.session_registry.clone())
        .with_recorder(state.flight_recorder.clone())
        .with_cli_bridge_auth_probe(cli_auth_probe)
        .with_cli_bridge_launchable_providers(operator_chat_cloud_wiring.launchable_providers);
    let operator_chat_routes = operator_chat::routes(operator_chat_state);
    let palmistry_routes = palmistry::routes(palmistry::PalmistryLaunchState::new(
        operator_chat_process_ledger.ledger(),
        state.flight_recorder.clone(),
        state.storage.clone(),
        state.postgres_pool.clone(),
        Arc::clone(&reclaim),
    ));
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
    // WP-1 live orchestration debug console SSE. Reads the process-wide shared,
    // NON-AUTHORITATIVE console hub that the coordinator's ConsoleSwarmSink tees
    // WP-1 events into (see swarm_orchestration::production_factory). Streaming
    // this surface never affects durable EventLedger/Flight Recorder authority.
    let console_stream_routes =
        console_stream::routes(crate::console_stream::ConsoleBroadcast::shared());
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
        .merge(model_runtime_registry_routes)
        .merge(model_access_routes)
        .merge(operator_chat_routes)
        .merge(palmistry_routes)
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
        .merge(console_stream_routes)
}

fn operator_chat_cloud_registry() -> Arc<dyn crate::model_runtime::cloud::ProviderAccessRegistry> {
    match crate::model_runtime::cloud::CloudModelAccess::production() {
        Ok(access) => Arc::new(access.registry()),
        Err(_) => Arc::new(crate::model_runtime::cloud::InMemoryAccessRegistry::new()),
    }
}

struct OperatorChatCloudWiring {
    factory: crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig,
    auth_probe: crate::model_runtime::cloud::ProductionCliBridgeAuthStatusProbe,
    launchable_providers: std::collections::BTreeSet<String>,
}

fn operator_chat_cloud_wiring(
    recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
    ledger: crate::process_ledger::LedgerBatcher,
    sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    reclaim: Arc<crate::process_ledger::Reclaim>,
) -> OperatorChatCloudWiring {
    let Ok(access) = crate::model_runtime::cloud::CloudModelAccess::production() else {
        return OperatorChatCloudWiring {
            factory:
                crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig::unconfigured(
                ),
            auth_probe: crate::model_runtime::cloud::ProductionCliBridgeAuthStatusProbe::default(),
            launchable_providers: std::collections::BTreeSet::new(),
        };
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

    // MT-019 F1: give the CLI spawner the running app's reclaimer so a child whose
    // STOP cannot be proven is reaped now, through the owner-scoped claim, instead
    // of staying OPEN until some later boot.
    let live_spawner = Arc::new(
        crate::model_runtime::cloud::LiveCliSpawner::new(Arc::new(ledger), sandbox_registry)
            .with_reclaim(reclaim),
    );
    let spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner> = live_spawner.clone();
    let observability = Some(Arc::new(
        crate::model_runtime::cloud::CloudLaneObservability {
            flight_recorder: recorder,
            consent: None,
        },
    ));

    let provider_configs = crate::model_runtime::cloud::CliBridgeProvider::OFFERED
        .into_iter()
        .filter_map(|provider| {
            let config = resolve_official_cli_config_from_path(provider)?;
            crate::model_runtime::cloud::CliSubprocessSpawner::pin_config(
                live_spawner.as_ref(),
                config.launch_config(),
            )
            .ok()?;
            Some((provider, config))
        })
        .collect::<Vec<_>>();
    let auth_probe =
        crate::model_runtime::cloud::ProductionCliBridgeAuthStatusProbe::from_canonical_launches(
            live_spawner,
            provider_configs
                .iter()
                .map(|(provider, config)| (*provider, config.launch_config().clone())),
        );
    let launchable_providers = provider_configs
        .iter()
        .map(|(provider, _)| provider.id().to_string())
        .collect();
    let factory = configure_operator_chat_official_cli_providers(
        cloud,
        spawner,
        observability,
        provider_configs
            .into_iter()
            .map(|(provider, config)| (provider.id().to_string(), config)),
    );
    OperatorChatCloudWiring {
        factory,
        auth_probe,
        launchable_providers,
    }
}

/// Production constructor selector for an Operator Chat official-CLI lane.
///
/// Operator Chat replays CLI JSONL into canonical ModelLane messages and emits
/// the corresponding `FR-EVT-AGENT-*` events itself. Keeping this decision in
/// one callable selector makes the duplicate-suppression mode load-bearing in
/// the integration proof: changing production to the observed constructor also
/// changes the tested runtime and makes the exact event count fail.
pub fn configure_operator_chat_official_cli_providers(
    mut cloud: crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig,
    spawner: Arc<dyn crate::model_runtime::cloud::CliSubprocessSpawner>,
    observability: Option<Arc<crate::model_runtime::cloud::CloudLaneObservability>>,
    providers: impl IntoIterator<
        Item = (
            String,
            crate::model_runtime::cloud::AllowlistedCliBridgeConfig,
        ),
    >,
) -> crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig {
    for (provider_id, config) in providers {
        cloud = cloud.with_official_cli_provider_replay_captured(
            provider_id,
            spawner.clone(),
            config,
            observability.clone(),
        );
    }
    cloud
}

pub fn resolve_official_cli_config_from_path(
    provider: crate::model_runtime::cloud::CliBridgeProvider,
) -> Option<crate::model_runtime::cloud::AllowlistedCliBridgeConfig> {
    let command = provider.login_command();
    executable_paths_on_path(command.program)
        .into_iter()
        .find_map(|executable_path| cli_bridge_config_for_executable(provider, executable_path))
}

fn cli_bridge_config_for_executable(
    provider: crate::model_runtime::cloud::CliBridgeProvider,
    executable_path: std::path::PathBuf,
) -> Option<crate::model_runtime::cloud::AllowlistedCliBridgeConfig> {
    crate::model_runtime::cloud::validate_cli_executable_path(&executable_path).ok()?;
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
                "--model".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ],
        ),
    };
    let config = crate::swarm_orchestration::operator_chat::force_json_stream_output(
        crate::model_runtime::cloud::CliBridgeConfig {
            cli_kind,
            executable_path,
            args_template,
            output_format: crate::model_runtime::cloud::CliOutputFormat::JsonStream,
            env_vars: std::collections::HashMap::new(),
            working_dir: None,
            timeout_seconds: 600,
        },
    );
    let model = match provider {
        crate::model_runtime::cloud::CliBridgeProvider::ClaudeCode => "claude-sonnet-4",
        crate::model_runtime::cloud::CliBridgeProvider::Codex => "gpt-5-codex",
    };
    let allowlist = crate::model_runtime::cloud::CliModelAllowlist::new(vec![model.to_string()])
        .expect("static official CLI model allowlist is non-empty");
    Some(crate::model_runtime::cloud::AllowlistedCliBridgeConfig::new(config, allowlist))
}

fn executable_paths_on_path(program: &str) -> Vec<std::path::PathBuf> {
    let candidates = executable_candidates(program);
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .flat_map(|dir| candidates.iter().map(move |candidate| dir.join(candidate)))
        .filter(|path| path.is_file())
        .collect()
}

fn executable_candidates(program: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        match std::path::Path::new(program)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
        {
            Some(extension) if extension == "exe" || extension == "com" => {
                vec![program.to_string()]
            }
            Some(extension) if extension == "cmd" => vec![program.to_string()],
            Some(_) => Vec::new(),
            None => vec![
                format!("{program}.exe"),
                format!("{program}.com"),
                format!("{program}.cmd"),
            ],
        }
    }
    #[cfg(not(windows))]
    {
        vec![program.to_string()]
    }
}

#[cfg(test)]
mod process_reclaim_registry_tests {
    use crate::process_ledger::production_process_sandbox_registry;
    use crate::sandbox::palmistry_watcher::PALMISTRY_WATCHER_ADAPTER_ID;
    use crate::sandbox::{AdapterId, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID};

    #[test]
    fn launch_and_reclaim_registry_contains_each_production_host_process_owner() {
        let registry = production_process_sandbox_registry();
        assert!(registry
            .get(&AdapterId::new(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID))
            .is_some());
        assert!(registry
            .get(&AdapterId::new(PALMISTRY_WATCHER_ADAPTER_ID))
            .is_some());
    }
}

#[cfg(all(test, windows))]
mod windows_executable_discovery_tests {
    use super::{cli_bridge_config_for_executable, executable_candidates};
    use crate::model_runtime::cloud::{CliBridgeProvider, CliKind};

    #[test]
    fn discovery_matches_direct_attached_launch_contract() {
        assert_eq!(
            executable_candidates("codex"),
            vec![
                "codex.exe".to_string(),
                "codex.com".to_string(),
                "codex.cmd".to_string(),
            ]
        );
        assert_eq!(executable_candidates("codex.cmd"), vec!["codex.cmd"]);
        assert!(executable_candidates("codex.bat").is_empty());
    }

    #[test]
    fn codex_npm_shim_is_reported_ready_by_provider_configuration() {
        let root =
            std::env::temp_dir().join(format!("handshake-codex-status-{}", uuid::Uuid::now_v7()));
        let script = root
            .join("node_modules")
            .join("@openai")
            .join("codex")
            .join("bin")
            .join("codex.js");
        std::fs::create_dir_all(script.parent().expect("script parent")).expect("fixture dirs");
        std::fs::write(root.join("node.exe"), b"fixture").expect("fixture node");
        std::fs::write(&script, b"// fixture").expect("fixture script");
        let shim = root.join("codex.cmd");
        std::fs::write(
            &shim,
            b"@echo off\r\n\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
        )
        .expect("fixture shim");

        let (config, allowlist) =
            cli_bridge_config_for_executable(CliBridgeProvider::Codex, shim.clone())
                .expect("validated Codex shim must be available to provider status")
                .into_parts();
        assert_eq!(config.cli_kind, CliKind::CodexCli);
        assert_eq!(config.executable_path, shim);
        assert_eq!(
            config.args_template,
            vec!["exec", "--json", "--model", "{model}", "{prompt}"]
        );
        assert!(allowlist.contains("gpt-5-codex"));
        let _ = std::fs::remove_dir_all(root);
    }
}
