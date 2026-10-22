use super::*;

pub(super) fn backend_listen_addr() -> Result<SocketAddr, io::Error> {
    let value = std::env::var("HANDSHAKE_BACKEND_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:37501".to_owned());
    let addr = value.parse::<SocketAddr>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid HANDSHAKE_BACKEND_LISTEN_ADDR={value:?}: {error}"),
        )
    })?;
    if !addr.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("HANDSHAKE_BACKEND_LISTEN_ADDR must be loopback-only, got {addr}"),
        ));
    }
    Ok(addr)
}

pub(super) fn write_backend_listen_report(addr: SocketAddr) -> Result<(), io::Error> {
    let Some(path) = std::env::var_os("HANDSHAKE_BACKEND_LISTEN_REPORT_FILE").map(PathBuf::from)
    else {
        return Ok(());
    };
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HANDSHAKE_BACKEND_LISTEN_REPORT_FILE must be absolute",
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut report = options.open(&path)?;
    serde_json::to_writer(
        &mut report,
        &serde_json::json!({
            "schema_id": "handshake.backend-listen-report.v1",
            "pid": std::process::id(),
            "listen_addr": addr.to_string(),
        }),
    )
    .map_err(io::Error::other)?;
    report.write_all(b"\n")?;
    report.sync_all()?;
    Ok(())
}

/// Initialize Janitor configuration from environment variables.
///
/// Environment variables:
/// - `JANITOR_DRY_RUN`: Set to "true" to enable dry-run mode (default: false)
/// - `JANITOR_INTERVAL_SECS`: Prune interval in seconds (default: 3600)
/// - `JANITOR_RETENTION_DAYS`: Days to retain AI job results (default: 30)
pub(super) fn init_janitor_config() -> JanitorConfig {
    use crate::storage::{ArtifactKind, RetentionPolicy};

    let dry_run = std::env::var("JANITOR_DRY_RUN")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let interval_secs = std::env::var("JANITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);

    let retention_days = std::env::var("JANITOR_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let policy = RetentionPolicy {
        kind: ArtifactKind::Result,
        window_days: retention_days,
        min_versions: 3,
    };

    tracing::info!(
        target: "handshake_core::janitor",
        dry_run,
        interval_secs,
        retention_days,
        "Janitor config initialized"
    );

    JanitorConfig {
        policies: vec![policy],
        dry_run,
        interval_secs,
        batch_size: 1000,
    }
}

pub(super) async fn init_flight_recorder() -> Result<Arc<DuckDbFlightRecorder>, Box<dyn std::error::Error>> {
    let data_dir = if let Some(configured) = std::env::var_os("HANDSHAKE_DATA_DIR") {
        PathBuf::from(configured)
    } else {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root_dir = manifest_dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or("failed to resolve repo root")?;
        root_dir.join("data")
    };
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }
    // Flight recorder gets its own file
    let fr_db_path = data_dir.join("flight_recorder.db");

    let recorder = DuckDbFlightRecorder::new_on_path(&fr_db_path, 7)?;
    tracing::info!(target: "handshake_core", db_path = %fr_db_path.display(), "flight recorder ready");

    Ok(Arc::new(recorder))
}

pub(super) async fn init_llm_client(
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
    model_registry_authority: Option<ScopedModelRegistryAuthority>,
    runtime_instance: Option<crate::process_ledger::EmbeddedRuntimeInstanceDescriptor>,
) -> Arc<dyn LlmClient> {
    resolve_default_llm_client(
        flight_recorder,
        ledger,
        model_registry_authority,
        runtime_instance,
    )
    .await
}

pub(super) fn model_access_state(
    runtime: &ProcessReclaimRuntime,
    scope: &api::account_scope::ProductLocalResourceScope,
) -> Result<(
    api::model_access::ModelAccessState,
    crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig,
    Arc<dyn crate::model_runtime::cloud::ProviderAccessRegistry>,
    Vec<String>,
), Box<dyn std::error::Error>> {
    use crate::model_runtime::cloud::{
        CliBridgeProvider, LiveCliSpawner, ProductionCliBridgeAuthStatusProbe,
    };
    use crate::model_runtime::cloud::official_cli_bridge::CliSubprocessSpawner;
    let spawner = Arc::new(LiveCliSpawner::new(
        Arc::new(runtime.ledger().ledger()), runtime.sandbox_registry(),
    ).with_reclaim(runtime.reclaim(), ReclaimResourceScope::from_exact(scope.exact())?));
    let mut launches = Vec::new();
    let mut launch_configs = Vec::new();
    match std::env::var_os(api::model_access::HANDSHAKE_OFFICIAL_CLI_CONFIG_PATH) {
        Some(path) => {
            let path = PathBuf::from(path);
            let config = match api::model_access::load_official_cli_config_from_path(
                &path, CliBridgeProvider::Codex,
            ) {
                Ok(config) => (CliBridgeProvider::Codex, config),
                Err(api::model_access::OfficialCliConfigError::Contract("provider")) => (
                    CliBridgeProvider::ClaudeCode,
                    api::model_access::load_official_cli_config_from_path(
                        &path, CliBridgeProvider::ClaudeCode,
                    )?,
                ),
                Err(error) => return Err(error.into()),
            };
            let (launch, _) = config.1.clone().into_parts();
            launch_configs.push((config.0.id().to_owned(), config.1));
            spawner.pin_config(&launch).map_err(|_| io::Error::new(
                io::ErrorKind::InvalidInput, "official CLI launch graph rejected",
            ))?;
            launches.push((config.0, launch));
        }
        None => tracing::warn!(
            target: "handshake_core::model_access",
            code = "official_cli_configuration_unavailable",
            "CLI access remains unavailable without the explicit configuration record",
        ),
    }
    let probe = Arc::new(ProductionCliBridgeAuthStatusProbe::from_canonical_launches(
        spawner.clone(), launches,
    ));
    let state = api::model_access::ModelAccessState::production_for_owner(
        scope.exact().clone(), probe.clone(), probe,
    );
    let (mut cloud, registry) = match state.production_cloud_dependencies() {
        Ok(dependencies) => dependencies,
        Err(api::model_access::ModelAccessProductionError::CloudAccess(
            crate::model_runtime::cloud::AccessConfigError::KeychainUnavailable,
        )) => (
            crate::swarm_orchestration::production_factory::CloudLaneFactoryConfig::unconfigured(),
            state.production_provider_registry()?,
        ),
        Err(error) => return Err(error.into()),
    };
    let mut providers = Vec::new();
    for (provider, config) in launch_configs {
        providers.push(provider.clone());
        cloud = cloud.with_official_cli_provider_replay_captured(
            provider, spawner.clone(), config, None,
        );
    }
    Ok((state, cloud, registry, providers))
}

pub(super) async fn bootstrap_health(State(state): State<crate::AccountBootstrapState>) -> Json<HealthResponse> {
    let (status, version) = match state.storage.ping().await {
        Ok(()) => match state.storage.migration_version().await {
            Ok(version) => ("ok", Some(version)),
            Err(_) => ("error", None),
        },
        Err(_) => ("error", None),
    };
    Json(build_health_response(status, version))
}

pub(super) async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let (db_status, migration_version) = match state.storage.ping().await {
        Ok(_) => match state.storage.migration_version().await {
            Ok(version) => ("ok", Some(version)),
            Err(err) => {
                tracing::error!(target: "handshake_core", route = "/health", error = %err, "db migration version check error");
                ("error", None)
            }
        },
        Err(err) => {
            tracing::error!(target: "handshake_core", route = "/health", error = %err, "db check error");
            ("error", None)
        }
    };

    let response = build_health_response(db_status, migration_version);
    tracing::info!(
        target: "handshake_core",
        route = "/health",
        status = response.status,
        db_status = db_status,
        "health check"
    );

    Json(response)
}

fn build_health_response(db_status: &str, migration_version: Option<i64>) -> HealthResponse {
    let overall_status = if db_status == "ok" { "ok" } else { "error" };

    HealthResponse {
        status: overall_status.to_string(),
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
        db_status: db_status.to_string(),
        migration_version,
    }
}

#[cfg(test)]
mod tests {
    use super::build_health_response;

    #[test]
    fn health_response_ok_sets_status_ok() {
        let response = build_health_response("ok", Some(9));
        assert_eq!(response.status, "ok");
        assert_eq!(response.component, "handshake_core");
        assert_eq!(response.db_status, "ok");
        assert_eq!(response.migration_version, Some(9));
    }

    #[test]
    fn health_response_error_maps_to_overall_error() {
        let response = build_health_response("error", None);
        assert_eq!(response.status, "error");
        assert_eq!(response.db_status, "error");
        assert_eq!(response.migration_version, None);
    }
}

