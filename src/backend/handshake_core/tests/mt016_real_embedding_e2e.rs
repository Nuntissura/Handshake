//! MT-016 real-model proof: production environment resolution, real Candle
//! completion + dedicated embedding models, PostgreSQL registry/EventLedger
//! authority, and restart-stable role selection.

#[cfg(feature = "candle-runtime-engine")]
#[path = "knowledge_pg_support.rs"]
mod knowledge_pg_support;

#[cfg(not(feature = "candle-runtime-engine"))]
#[tokio::test]
#[ignore = "MT-016 real embedding proof requires candle-runtime-engine and real model resources"]
async fn mt016_real_candle_embedding_restart_recovers_role_isolation_and_event_ledger() {
    panic!("MT-016 real embedding proof requires --features candle-runtime-engine,test-utils");
}

#[cfg(feature = "candle-runtime-engine")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires production local-model env, POSTGRES_TEST_URL, and MT-016 proof env"]
async fn mt016_real_candle_embedding_restart_recovers_role_isolation_and_event_ledger() {
    real_proof::run().await;
}

#[cfg(feature = "candle-runtime-engine")]
mod real_proof {
    use std::{
        env, fs,
        path::PathBuf,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use async_trait::async_trait;
    use handshake_core::{
        flight_recorder::{
            EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType,
            RecorderError,
        },
        kernel::KernelEventType,
        llm::{
            boot::resolve_default_llm_client, CompletionRequest, EmbeddingRequest, LlmClient,
            LlmError,
        },
        model_runtime::{
            candle::adapter::sha256_file, ModelCatalogEntry, ModelRegistryStore, ModelRuntimeRole,
            ModelRuntimeSelectionPurpose, RuntimeBinding,
        },
        process_ledger::{
            acquire_embedded_runtime_instance_lease, drain_and_join_ledger_writer,
            resolve_embedded_runtime_host_scope_with_override, LedgerBatcher, LedgerBatcherConfig,
            LedgerDrainJoinOutcome, NoopOverflowSink, PostgresProcessLedgerStore,
            ProcessLedgerError,
        },
    };
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use sqlx::PgPool;
    use uuid::Uuid;

    const TEST_ID: &str =
        "mt016_real_candle_embedding_restart_recovers_role_isolation_and_event_ledger";
    const PROOF_NONCE_ENV: &str = "HANDSHAKE_MT016_REAL_EMBEDDING_PROOF_NONCE";
    const FROZEN_SOURCE_WORKTREE_SHA256_ENV: &str = "HANDSHAKE_MT016_FROZEN_SOURCE_WORKTREE_SHA256";
    const PROOF_FILE: &str = "mt016-real-candle-embedding-restart-proof-v1.json";
    const PROVENANCE_FILE: &str = "mt016-real-candle-embedding-restart-proof-v1.provenance.json";
    const EMBEDDING_DIMENSION: usize = 768;
    const EMBEDDING_INPUT: &str = "Handshake MT-016 restart-stable dedicated embedding model proof";

    #[derive(Clone, Default)]
    struct CapturingRecorder {
        events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    impl CapturingRecorder {
        fn events(&self) -> Vec<FlightRecorderEvent> {
            self.events
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    #[async_trait]
    impl FlightRecorder for CapturingRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.events
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self.events())
        }
    }

    #[derive(Clone)]
    struct ArtifactFixture {
        path: PathBuf,
        sha256: String,
        display_name: String,
        config_sha256: String,
        config_length: u64,
        tokenizer_sha256: String,
        tokenizer_length: u64,
        weights_length: u64,
        hidden_size: usize,
    }

    struct ProofContext {
        proof_path: PathBuf,
        provenance_path: PathBuf,
        proof_temp: PathBuf,
        provenance_temp: PathBuf,
        proof_nonce: String,
        frozen_source_worktree_sha256: String,
        producer_source_sha256: String,
        primary: ArtifactFixture,
        embedding: ArtifactFixture,
    }

    #[derive(Clone)]
    struct BootCatalog {
        completion: ModelCatalogEntry,
        embedding: ModelCatalogEntry,
    }

    pub async fn run() {
        let context = prepare_proof_context();
        let _postgres_test_url = required_env("POSTGRES_TEST_URL");
        let pg = crate::knowledge_pg_support::knowledge_pg()
            .await
            .expect("MT-016 real-model proof requires real PostgreSQL");
        let pool = PgPool::connect(&pg.schema_url)
            .await
            .expect("connect MT-016 proof to isolated migrated PostgreSQL schema");
        let registry_store = ModelRegistryStore::new(pool.clone());
        let recorder = Arc::new(CapturingRecorder::default());
        let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
        let stable_host_scope = format!("mt016-real-embedding-{}", context.proof_nonce);

        let first_lease = acquire_runtime_lease(&pg.schema_url, &stable_host_scope);
        let first_runtime_instance = first_lease.descriptor().clone();
        let (first_ledger, first_writer) = LedgerBatcher::spawn(
            Arc::new(PostgresProcessLedgerStore::new(pool.clone())),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        let first_ledger_close = first_ledger.clone();
        let first_client = resolve_default_llm_client(
            Arc::clone(&flight_recorder),
            Some(first_ledger),
            Some(registry_store.clone()),
            Some(first_runtime_instance.clone()),
        )
        .await;
        let first_catalog = require_catalog(
            first_client.as_ref(),
            &context.primary,
            &context.embedding,
            "first boot",
        )
        .await;
        let first_embedding = compute_real_embedding(
            first_client.as_ref(),
            &first_catalog,
            "first boot embedding",
        )
        .await;
        require_completion_model_cannot_embed(first_client.as_ref(), &first_catalog, "first boot")
            .await;
        let first_active = require_active_selections(
            &registry_store,
            &context.primary.sha256,
            &context.embedding.sha256,
        )
        .await;
        require_persisted_registry_roles(
            &registry_store,
            &context.primary,
            &context.embedding,
            &first_catalog,
        )
        .await;
        graceful_shutdown_and_drain(
            first_client.as_ref(),
            &first_ledger_close,
            first_writer,
            "first boot",
        )
        .await;
        let first_lifecycle = lifecycle_evidence(
            &pool,
            &first_catalog,
            &context.primary.sha256,
            &context.embedding.sha256,
            "first boot",
        )
        .await;
        drop(first_client);
        first_lease
            .release()
            .await
            .expect("release first MT-016 runtime instance lease");

        let second_lease = acquire_runtime_lease(&pg.schema_url, &stable_host_scope);
        let second_runtime_instance = second_lease.descriptor().clone();
        assert_ne!(
            first_runtime_instance.instance_id, second_runtime_instance.instance_id,
            "restart must mint a new runtime instance identity"
        );
        let (second_ledger, second_writer) = LedgerBatcher::spawn(
            Arc::new(PostgresProcessLedgerStore::new(pool.clone())),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        let second_ledger_close = second_ledger.clone();
        let second_client = resolve_default_llm_client(
            Arc::clone(&flight_recorder),
            Some(second_ledger),
            Some(registry_store.clone()),
            Some(second_runtime_instance.clone()),
        )
        .await;
        let second_catalog = require_catalog(
            second_client.as_ref(),
            &context.primary,
            &context.embedding,
            "restart boot",
        )
        .await;
        assert_ne!(
            first_catalog.completion.model_id, second_catalog.completion.model_id,
            "completion routing UUID must be re-minted after restart"
        );
        assert_ne!(
            first_catalog.embedding.model_id, second_catalog.embedding.model_id,
            "embedding routing UUID must be re-minted after restart"
        );
        assert_eq!(
            first_catalog.completion.artifact_sha256,
            second_catalog.completion.artifact_sha256
        );
        assert_eq!(
            first_catalog.embedding.artifact_sha256,
            second_catalog.embedding.artifact_sha256
        );

        let second_embedding =
            compute_real_embedding(second_client.as_ref(), &second_catalog, "restart embedding")
                .await;
        require_completion_model_cannot_embed(
            second_client.as_ref(),
            &second_catalog,
            "restart boot",
        )
        .await;
        let completion_trace_id = Uuid::now_v7();
        let completion = second_client
            .completion(
                CompletionRequest::new(
                    completion_trace_id,
                    "Handshake MT-016 completion-default recovery proof".to_owned(),
                    second_client.selected_model_id(),
                )
                .with_max_tokens(4),
            )
            .await
            .expect("restart completion default must remain real and routable");
        assert_eq!(
            second_client.selected_model_id(),
            second_catalog.completion.model_id,
            "restart default selection must remain the completion role"
        );
        assert!(completion.latency_ms > 0);

        let second_active = require_active_selections(
            &registry_store,
            &context.primary.sha256,
            &context.embedding.sha256,
        )
        .await;
        assert_eq!(
            first_active, second_active,
            "restart must recover the same PostgreSQL active-purpose authority"
        );
        require_persisted_registry_roles(
            &registry_store,
            &context.primary,
            &context.embedding,
            &second_catalog,
        )
        .await;

        let registry_events =
            registry_event_evidence(&pool, &context.primary.sha256, &context.embedding.sha256)
                .await;
        let active_events = active_selection_event_evidence(&pool).await;
        let flight_events = require_flight_recorder_evidence(
            &recorder,
            &first_catalog,
            &second_catalog,
            completion_trace_id,
        );

        graceful_shutdown_and_drain(
            second_client.as_ref(),
            &second_ledger_close,
            second_writer,
            "restart boot",
        )
        .await;
        let second_lifecycle = lifecycle_evidence(
            &pool,
            &second_catalog,
            &context.primary.sha256,
            &context.embedding.sha256,
            "restart boot",
        )
        .await;
        drop(second_client);
        second_lease
            .release()
            .await
            .expect("release restart MT-016 runtime instance lease");

        let proof_body = json!({
            "schema_id": "hsk.mt016_real_candle_embedding_restart_proof@1",
            "proof_nonce": context.proof_nonce,
            "producer_test_id": TEST_ID,
            "producer_status": "passed_real_model_postgresql_restart_role_and_eventledger_assertions",
            "frozen_source_worktree_sha256": context.frozen_source_worktree_sha256,
            "producer_source_sha256": context.producer_source_sha256,
            "postgres_schema": pg.schema,
            "models": {
                "completion": artifact_evidence(&context.primary),
                "embedding": artifact_evidence(&context.embedding),
            },
            "first_boot": {
                "runtime_instance_id": first_runtime_instance.instance_id,
                "catalog": catalog_evidence(&first_catalog),
                "embedding": first_embedding,
                "active_selections": first_active,
                "process_lifecycle": first_lifecycle,
            },
            "restart_boot": {
                "runtime_instance_id": second_runtime_instance.instance_id,
                "catalog": catalog_evidence(&second_catalog),
                "embedding": second_embedding,
                "completion": {
                    "trace_id": completion_trace_id,
                    "selected_model_id": second_catalog.completion.model_id,
                    "completion_tokens": completion.usage.completion_tokens,
                    "response_sha256": sha256_bytes(completion.text.as_bytes()),
                },
                "active_selections": second_active,
                "process_lifecycle": second_lifecycle,
            },
            "event_ledger": {
                "registry_events": registry_events,
                "active_selection_events": active_events,
            },
            "flight_recorder": flight_events,
            "negative_checks": {
                "completion_model_embedding_rejected_on_both_boots": true,
                "embedding_model_never_default_selectable": true,
                "per_boot_routing_ids_not_reused": true,
                "fallback_not_used_for_local_uuid_role_rejection": true,
                "canonical_artifacts_absent_until_all_assertions_passed": true,
            },
            "result": { "status": "PASS", "passed": 1, "failed": 0 },
        });
        publish_proof(&context, proof_body);
    }

    fn prepare_proof_context() -> ProofContext {
        let proof_nonce = Uuid::parse_str(&required_env(PROOF_NONCE_ENV))
            .expect("MT-016 proof nonce must be a UUID")
            .to_string();
        let frozen_source_worktree_sha256 = required_env(FROZEN_SOURCE_WORKTREE_SHA256_ENV);
        require_canonical_sha256("frozen source/worktree", &frozen_source_worktree_sha256);
        let producer_source_sha256 = sha256_bytes(include_bytes!("mt016_real_embedding_e2e.rs"));
        let artifacts_root = PathBuf::from(required_env("HANDSHAKE_ARTIFACTS_DIR"));
        assert!(
            artifacts_root.is_absolute(),
            "HANDSHAKE_ARTIFACTS_DIR must be absolute"
        );
        let repo_root = discover_repo_root();
        assert!(
            !artifacts_root.starts_with(&repo_root),
            "MT-016 proof artifacts must remain outside repo {}",
            repo_root.display()
        );
        let proof_dir = artifacts_root
            .join("handshake-test")
            .join("wp1-final-audit");
        fs::create_dir_all(&proof_dir).expect("create MT-016 external proof directory");
        let proof_path = proof_dir.join(PROOF_FILE);
        let provenance_path = proof_dir.join(PROVENANCE_FILE);
        let proof_temp = proof_dir.join(format!("{PROOF_FILE}.{proof_nonce}.tmp"));
        let provenance_temp = proof_dir.join(format!("{PROVENANCE_FILE}.{proof_nonce}.tmp"));
        for stale in [&proof_path, &provenance_path, &proof_temp, &provenance_temp] {
            if stale.exists() {
                fs::remove_file(stale)
                    .unwrap_or_else(|error| panic!("remove stale {}: {error}", stale.display()));
            }
            assert!(
                !stale.exists(),
                "stale MT-016 proof artifact must be absent before execution: {}",
                stale.display()
            );
        }

        let primary = required_artifact("HANDSHAKE_LOCAL_MODEL", None);
        let embedding =
            required_artifact("HANDSHAKE_LOCAL_EMBEDDING_MODEL", Some(EMBEDDING_DIMENSION));
        assert_ne!(
            primary.sha256, embedding.sha256,
            "MT-016 requires distinct completion and embedding artifacts"
        );

        ProofContext {
            proof_path,
            provenance_path,
            proof_temp,
            provenance_temp,
            proof_nonce,
            frozen_source_worktree_sha256,
            producer_source_sha256,
            primary,
            embedding,
        }
    }

    fn required_artifact(prefix: &str, required_hidden_size: Option<usize>) -> ArtifactFixture {
        let path = PathBuf::from(required_env(&format!("{prefix}_PATH")));
        assert!(path.is_absolute(), "{prefix}_PATH must be absolute");
        assert!(
            path.is_file(),
            "{} must name a real weights file",
            path.display()
        );
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("model.safetensors"),
            "Candle production proof requires model.safetensors"
        );
        let declared_sha256 = required_env(&format!("{prefix}_SHA256"));
        require_canonical_sha256(prefix, &declared_sha256);
        let actual_sha256 = sha256_file(&path).expect("hash configured Candle weights");
        assert_eq!(
            declared_sha256, actual_sha256,
            "{prefix}_SHA256 must match the exact loaded weights"
        );
        assert_eq!(
            required_env(&format!("{prefix}_BINDING")),
            "candle",
            "MT-016 real proof requires the Candle production binding"
        );
        let display_name = required_env(&format!("{prefix}_NAME"));
        let directory = path.parent().expect("weights have a parent directory");
        let config_path = directory.join("config.json");
        let tokenizer_path = directory.join("tokenizer.json");
        assert!(config_path.is_file(), "missing {}", config_path.display());
        assert!(
            tokenizer_path.is_file(),
            "missing {}",
            tokenizer_path.display()
        );
        let config_bytes = fs::read(&config_path).expect("read configured Candle config");
        let tokenizer_bytes = fs::read(&tokenizer_path).expect("read configured Candle tokenizer");
        let config: Value =
            serde_json::from_slice(&config_bytes).expect("configured Candle config is JSON");
        let _: Value =
            serde_json::from_slice(&tokenizer_bytes).expect("configured Candle tokenizer is JSON");
        assert_eq!(
            config.get("model_type").and_then(Value::as_str),
            Some("llama"),
            "current Candle production loader requires a Llama config"
        );
        let hidden_size = config
            .get("hidden_size")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .expect("Candle Llama config has a positive hidden_size");
        if let Some(required) = required_hidden_size {
            let declared_dimension = required_env(&format!("{prefix}_DIMENSION"))
                .parse::<usize>()
                .expect("embedding dimension env is a positive integer");
            assert_eq!(declared_dimension, required);
            assert_eq!(
                hidden_size, required,
                "actual Candle residual width must equal MT-016's declared embedding dimension"
            );
        }
        ArtifactFixture {
            path: path.clone(),
            sha256: actual_sha256,
            display_name,
            config_sha256: sha256_bytes(&config_bytes),
            config_length: config_bytes.len() as u64,
            tokenizer_sha256: sha256_bytes(&tokenizer_bytes),
            tokenizer_length: tokenizer_bytes.len() as u64,
            weights_length: fs::metadata(path)
                .expect("inspect configured Candle weights")
                .len(),
            hidden_size,
        }
    }

    fn acquire_runtime_lease(
        database_url: &str,
        stable_host_scope: &str,
    ) -> handshake_core::process_ledger::EmbeddedRuntimeInstanceLease {
        let scope = resolve_embedded_runtime_host_scope_with_override(
            database_url,
            Some(stable_host_scope),
        )
        .expect("resolve MT-016 runtime host scope");
        acquire_embedded_runtime_instance_lease(Uuid::now_v7(), scope)
            .expect("acquire MT-016 runtime instance lease")
    }

    async fn require_catalog(
        client: &dyn LlmClient,
        primary: &ArtifactFixture,
        embedding: &ArtifactFixture,
        phase: &str,
    ) -> BootCatalog {
        let catalog = client.model_catalog().unwrap_or_else(|| {
            panic!("{phase} did not expose a production model catalog; boot failed closed")
        });
        let entries = catalog.list();
        assert_eq!(entries.len(), 2, "{phase} must expose exactly two models");
        let completion = entries
            .iter()
            .find(|entry| entry.artifact_sha256 == primary.sha256)
            .cloned()
            .unwrap_or_else(|| panic!("{phase} completion artifact is absent"));
        let embedding_entry = entries
            .iter()
            .find(|entry| entry.artifact_sha256 == embedding.sha256)
            .cloned()
            .unwrap_or_else(|| panic!("{phase} embedding artifact is absent"));
        assert_ne!(completion.model_id, embedding_entry.model_id);
        assert_eq!(completion.runtime_binding, "candle");
        assert_eq!(completion.runtime_role, ModelRuntimeRole::Completion);
        assert!(completion.default_selectable);
        assert!(completion.ready);
        assert_eq!(embedding_entry.runtime_binding, "candle");
        assert_eq!(embedding_entry.runtime_role, ModelRuntimeRole::Embedding);
        assert!(!embedding_entry.default_selectable);
        assert!(embedding_entry.ready);
        assert!(embedding_entry.supports_embedding);
        assert_eq!(
            embedding_entry.embedding_dimension,
            Some(EMBEDDING_DIMENSION)
        );
        assert_eq!(client.profile().model_id, completion.model_id);
        assert_eq!(client.selected_model_id(), completion.model_id);
        let selected_embedding = catalog
            .embedding_model_for_dim(EMBEDDING_DIMENSION)
            .unwrap_or_else(|| panic!("{phase} has no active 768-dimensional embedding model"));
        assert_eq!(selected_embedding.model_id, embedding_entry.model_id);
        assert_eq!(selected_embedding.runtime_role, ModelRuntimeRole::Embedding);
        BootCatalog {
            completion,
            embedding: embedding_entry,
        }
    }

    async fn compute_real_embedding(
        client: &dyn LlmClient,
        catalog: &BootCatalog,
        phase: &str,
    ) -> Value {
        let trace_id = Uuid::now_v7();
        let response = client
            .embedding(EmbeddingRequest::new(
                trace_id,
                EMBEDDING_INPUT.to_owned(),
                catalog.embedding.model_id.clone(),
            ))
            .await
            .unwrap_or_else(|error| panic!("{phase} failed: {error}"));
        assert_eq!(response.model_id, catalog.embedding.model_id);
        assert_eq!(response.dim(), EMBEDDING_DIMENSION);
        assert!(response.vector.iter().all(|value| value.is_finite()));
        assert!(
            response.vector.iter().any(|value| *value != 0.0),
            "{phase} must return computed hidden-state values, not a zero vector"
        );
        let vector_bytes = response
            .vector
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        json!({
            "trace_id": trace_id,
            "model_id": response.model_id,
            "dimension": response.dim(),
            "latency_ms": response.latency_ms,
            "vector_sha256": sha256_bytes(&vector_bytes),
            "all_values_finite": true,
            "nonzero_value_observed": true,
        })
    }

    async fn require_completion_model_cannot_embed(
        client: &dyn LlmClient,
        catalog: &BootCatalog,
        phase: &str,
    ) {
        let error = client
            .embedding(EmbeddingRequest::new(
                Uuid::now_v7(),
                "must not cross from embedding role to completion default".to_owned(),
                catalog.completion.model_id.clone(),
            ))
            .await
            .expect_err("completion-only model must be rejected for dedicated embedding routing");
        assert!(
            matches!(&error, LlmError::EmbeddingUnsupported),
            "{phase} completion-role embedding rejection must remain typed: {error}"
        );
    }

    async fn require_persisted_registry_roles(
        store: &ModelRegistryStore,
        primary: &ArtifactFixture,
        embedding: &ArtifactFixture,
        catalog: &BootCatalog,
    ) {
        let primary_sha = decode_sha256(&primary.sha256);
        let embedding_sha = decode_sha256(&embedding.sha256);
        let persisted_primary = store
            .load_by_artifact_sha256(&primary_sha)
            .await
            .expect("read completion registry row")
            .expect("completion registry row exists");
        let persisted_embedding = store
            .load_by_artifact_sha256(&embedding_sha)
            .await
            .expect("read embedding registry row")
            .expect("embedding registry row exists");
        assert_eq!(persisted_primary.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(persisted_primary.runtime_role, ModelRuntimeRole::Completion);
        assert_eq!(
            persisted_primary.last_observed_runtime_model_id.to_string(),
            catalog.completion.model_id
        );
        assert_eq!(persisted_embedding.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(
            persisted_embedding.runtime_role,
            ModelRuntimeRole::Embedding
        );
        assert_eq!(
            persisted_embedding
                .declared_capabilities
                .embedding_dimension,
            Some(EMBEDDING_DIMENSION)
        );
        assert_eq!(
            persisted_embedding
                .last_observed_runtime_model_id
                .to_string(),
            catalog.embedding.model_id
        );
    }

    async fn require_active_selections(
        store: &ModelRegistryStore,
        primary_sha256: &str,
        embedding_sha256: &str,
    ) -> Value {
        let selections = store
            .list_active_selections()
            .await
            .expect("read PostgreSQL active model selections");
        assert_eq!(selections.len(), 2);
        let application = selections
            .iter()
            .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
            .expect("application/default selection exists");
        let embeddings = selections
            .iter()
            .find(|row| row.purpose == ModelRuntimeSelectionPurpose::EmbeddingsDefault)
            .expect("embeddings/default selection exists");
        assert_eq!(application.runtime_role, ModelRuntimeRole::Completion);
        assert_eq!(hex::encode(application.artifact_sha256), primary_sha256);
        assert_eq!(embeddings.runtime_role, ModelRuntimeRole::Embedding);
        assert_eq!(hex::encode(embeddings.artifact_sha256), embedding_sha256);
        json!({
            "application/default": {
                "runtime_role": application.runtime_role.as_str(),
                "artifact_sha256": primary_sha256,
                "selection_revision": application.selection_revision,
                "selection_event_id": application.selection_updated_event_id,
            },
            "embeddings/default": {
                "runtime_role": embeddings.runtime_role.as_str(),
                "artifact_sha256": embedding_sha256,
                "selection_revision": embeddings.selection_revision,
                "selection_event_id": embeddings.selection_updated_event_id,
            },
        })
    }

    async fn graceful_shutdown_and_drain(
        client: &dyn LlmClient,
        ledger: &LedgerBatcher,
        writer: tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
        phase: &str,
    ) {
        client
            .shutdown_gracefully()
            .await
            .unwrap_or_else(|error| panic!("{phase} graceful shutdown failed: {error}"));
        let outcome = drain_and_join_ledger_writer(ledger, writer, Duration::from_secs(5)).await;
        assert!(
            matches!(outcome, LedgerDrainJoinOutcome::Flushed),
            "{phase} process ledger must drain: {outcome:?}"
        );
    }

    async fn lifecycle_evidence(
        pool: &PgPool,
        catalog: &BootCatalog,
        primary_sha256: &str,
        embedding_sha256: &str,
        phase: &str,
    ) -> Value {
        let primary = lifecycle_row(pool, &catalog.completion.model_id, phase).await;
        let embedding = lifecycle_row(pool, &catalog.embedding.model_id, phase).await;
        assert_eq!(primary["model_artifact_sha256"], primary_sha256);
        assert_eq!(embedding["model_artifact_sha256"], embedding_sha256);
        json!({ "completion": primary, "embedding": embedding })
    }

    async fn lifecycle_row(pool: &PgPool, model_id: &str, phase: &str) -> Value {
        let process_uuid = Uuid::parse_str(model_id).expect("catalog model id is a UUID");
        let row: (
            Uuid,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<i32>,
            Option<String>,
            Option<String>,
            Value,
        ) = sqlx::query_as(
            r#"
            SELECT process_uuid, engine_kind, stopped_at, exit_code, stop_reason,
                   model_artifact_sha256, metadata_jsonb
            FROM ONLY kernel_process_lifecycle
            WHERE process_uuid = $1
            "#,
        )
        .bind(process_uuid)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("read {phase} lifecycle for {model_id}: {error}"));
        assert_eq!(row.0, process_uuid);
        assert_eq!(row.1, "candle");
        assert!(
            row.2.is_some(),
            "{phase} STOP must be durable for {model_id}"
        );
        assert_eq!(row.3, Some(0));
        assert_eq!(row.4.as_deref(), Some("llm-client-shutdown"));
        assert_eq!(
            row.6["os_pid_absent_reason"].as_str(),
            Some("in_process_library_load_no_os_process")
        );
        json!({
            "process_uuid": row.0,
            "engine_kind": row.1,
            "stopped_at": row.2,
            "exit_code": row.3,
            "stop_reason": row.4,
            "model_artifact_sha256": row.5,
            "metadata_jsonb": row.6,
        })
    }

    async fn registry_event_evidence(
        pool: &PgPool,
        primary_sha256: &str,
        embedding_sha256: &str,
    ) -> Value {
        let mut result = serde_json::Map::new();
        for (role, sha256) in [
            (ModelRuntimeRole::Completion, primary_sha256),
            (ModelRuntimeRole::Embedding, embedding_sha256),
        ] {
            let aggregate_id = format!("sha256:{sha256}");
            let rows: Vec<(String, String, Value)> = sqlx::query_as(
                r#"
                SELECT event_id, event_type, payload
                FROM ONLY kernel_event_ledger
                WHERE aggregate_type = 'model_runtime_registry'
                  AND aggregate_id = $1
                  AND source_component = 'model_runtime_registry'
                ORDER BY event_sequence ASC
                "#,
            )
            .bind(&aggregate_id)
            .fetch_all(pool)
            .await
            .expect("read model registry EventLedger evidence");
            assert_eq!(rows.len(), 1, "immutable selection has one initial audit");
            assert_eq!(
                rows[0].1,
                KernelEventType::ModelRuntimeSelectionRecorded.as_str()
            );
            assert_eq!(rows[0].2["runtime_role"].as_str(), Some(role.as_str()));
            result.insert(
                role.as_str().to_owned(),
                json!({
                    "aggregate_id": aggregate_id,
                    "event_id": rows[0].0,
                    "event_type": rows[0].1,
                    "runtime_role": rows[0].2["runtime_role"],
                    "artifact_sha256": sha256,
                }),
            );
        }
        Value::Object(result)
    }

    async fn active_selection_event_evidence(pool: &PgPool) -> Value {
        let rows: Vec<(String, String, Value)> = sqlx::query_as(
            r#"
            SELECT event_id, event_type, payload
            FROM ONLY kernel_event_ledger
            WHERE aggregate_type = 'model_runtime_active_selection'
              AND source_component = 'model_runtime_registry'
            ORDER BY event_sequence ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .expect("read active-selection EventLedger evidence");
        assert_eq!(rows.len(), 2, "both active purposes require one audit row");
        let mut purposes = rows
            .iter()
            .map(|row| {
                assert_eq!(
                    row.1,
                    KernelEventType::ModelRuntimeSelectionRecorded.as_str()
                );
                row.2["purpose"]
                    .as_str()
                    .expect("active-selection event names its purpose")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        purposes.sort();
        assert_eq!(purposes, ["application/default", "embeddings/default"]);
        Value::Array(
            rows.into_iter()
                .map(|row| {
                    json!({
                        "event_id": row.0,
                        "event_type": row.1,
                        "purpose": row.2["purpose"],
                        "runtime_role": row.2["runtime_role"],
                        "target_artifact_sha256": row.2["target_artifact_sha256"],
                    })
                })
                .collect(),
        )
    }

    fn require_flight_recorder_evidence(
        recorder: &CapturingRecorder,
        first_catalog: &BootCatalog,
        restart_catalog: &BootCatalog,
        completion_trace_id: Uuid,
    ) -> Value {
        let events = recorder.events();
        let embeddings = events
            .iter()
            .filter(|event| {
                event.event_type == FlightRecorderEventType::DataEmbeddingComputed
                    && event.payload["dimensions"].as_u64() == Some(EMBEDDING_DIMENSION as u64)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            embeddings.len(),
            2,
            "one real embedding event is required per boot"
        );
        assert!(embeddings.iter().any(|event| {
            event.model_id.as_deref() == Some(first_catalog.embedding.model_id.as_str())
        }));
        assert!(embeddings.iter().any(|event| {
            event.model_id.as_deref() == Some(restart_catalog.embedding.model_id.as_str())
        }));
        let completion = events
            .iter()
            .find(|event| {
                event.event_type == FlightRecorderEventType::LlmInference
                    && event.trace_id == completion_trace_id
                    && event.model_id.as_deref()
                        == Some(restart_catalog.completion.model_id.as_str())
            })
            .expect("restart completion emits model-bound Flight Recorder evidence");
        assert!(events.iter().all(|event| {
            event.event_type != FlightRecorderEventType::LlmInference
                || event.trace_id != completion_trace_id
                || event.model_id.as_deref() != Some(restart_catalog.embedding.model_id.as_str())
        }));
        json!({
            "embedding_event_ids": embeddings.iter().map(|event| event.event_id).collect::<Vec<_>>(),
            "completion_event_id": completion.event_id,
            "completion_trace_id": completion_trace_id,
            "completion_model_id": restart_catalog.completion.model_id,
            "first_embedding_model_id": first_catalog.embedding.model_id,
            "restart_embedding_model_id": restart_catalog.embedding.model_id,
        })
    }

    fn artifact_evidence(artifact: &ArtifactFixture) -> Value {
        json!({
            "path": artifact.path,
            "display_name": artifact.display_name,
            "weights": {
                "sha256": artifact.sha256,
                "length_bytes": artifact.weights_length,
            },
            "config": {
                "sha256": artifact.config_sha256,
                "length_bytes": artifact.config_length,
                "hidden_size": artifact.hidden_size,
            },
            "tokenizer": {
                "sha256": artifact.tokenizer_sha256,
                "length_bytes": artifact.tokenizer_length,
            },
        })
    }

    fn catalog_evidence(catalog: &BootCatalog) -> Value {
        json!({
            "completion": {
                "model_id": catalog.completion.model_id,
                "artifact_sha256": catalog.completion.artifact_sha256,
                "runtime_role": catalog.completion.runtime_role.as_str(),
                "default_selectable": catalog.completion.default_selectable,
                "ready": catalog.completion.ready,
            },
            "embedding": {
                "model_id": catalog.embedding.model_id,
                "artifact_sha256": catalog.embedding.artifact_sha256,
                "runtime_role": catalog.embedding.runtime_role.as_str(),
                "default_selectable": catalog.embedding.default_selectable,
                "supports_embedding": catalog.embedding.supports_embedding,
                "embedding_dimension": catalog.embedding.embedding_dimension,
                "ready": catalog.embedding.ready,
            },
        })
    }

    fn publish_proof(context: &ProofContext, mut proof: Value) {
        assert!(!context.proof_path.exists());
        assert!(!context.provenance_path.exists());
        assert!(!context.proof_temp.exists());
        assert!(!context.provenance_temp.exists());
        let completed_at = chrono::Utc::now();
        proof["producer_completed_at_utc"] = json!(completed_at.to_rfc3339());
        proof["producer_completed_at_unix_ms"] = json!(completed_at.timestamp_millis());
        let proof_bytes =
            serde_json::to_vec_pretty(&proof).expect("serialize MT-016 real proof artifact");
        let proof_sha256 = sha256_bytes(&proof_bytes);
        let provenance = json!({
            "schema_id": "hsk.mt016_real_candle_embedding_restart_provenance@1",
            "proof_nonce": context.proof_nonce,
            "producer_test_id": TEST_ID,
            "producer_status": "passed_real_model_postgresql_restart_role_and_eventledger_assertions",
            "producer_completed_at_utc": completed_at.to_rfc3339(),
            "producer_completed_at_unix_ms": completed_at.timestamp_millis(),
            "frozen_source_worktree_sha256": context.frozen_source_worktree_sha256,
            "producer_source_sha256": context.producer_source_sha256,
            "artifact_sha256": proof_sha256,
            "result": "PASS",
        });
        fs::write(&context.proof_temp, &proof_bytes)
            .expect("write temporary MT-016 real proof artifact");
        fs::write(
            &context.provenance_temp,
            serde_json::to_vec_pretty(&provenance).expect("serialize MT-016 real proof provenance"),
        )
        .expect("write temporary MT-016 real proof provenance");
        assert!(!context.proof_path.exists());
        assert!(!context.provenance_path.exists());
        fs::rename(&context.provenance_temp, &context.provenance_path)
            .expect("publish MT-016 provenance atomically");
        fs::rename(&context.proof_temp, &context.proof_path)
            .expect("publish MT-016 proof artifact atomically");
        assert!(context.proof_path.is_file());
        assert!(context.provenance_path.is_file());
        eprintln!(
            "[MT-016_REAL_EMBEDDING_PROOF] {}",
            context.proof_path.display()
        );
    }

    fn required_env(name: &str) -> String {
        let value = env::var(name)
            .unwrap_or_else(|_| panic!("MT-016 real proof requires {name}"))
            .trim()
            .to_owned();
        assert!(!value.is_empty(), "{name} must not be empty");
        value
    }

    fn require_canonical_sha256(label: &str, value: &str) {
        assert!(
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{label} sha256 must be 64 lowercase hexadecimal characters"
        );
    }

    fn decode_sha256(value: &str) -> [u8; 32] {
        require_canonical_sha256("artifact", value);
        let decoded = hex::decode(value).expect("decode canonical artifact SHA-256");
        decoded
            .try_into()
            .expect("canonical SHA-256 decodes to 32 bytes")
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    fn discover_repo_root() -> PathBuf {
        let mut candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            if candidate.join(".git").exists() {
                return candidate;
            }
            assert!(candidate.pop(), "cannot discover repo root");
        }
    }
}
