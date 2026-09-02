#![cfg(feature = "test-utils")]
#![cfg_attr(not(feature = "candle-runtime-engine"), allow(dead_code))]

#[cfg(feature = "candle-runtime-engine")]
#[path = "knowledge_pg_support.rs"]
mod knowledge_pg_support;
#[cfg(feature = "candle-runtime-engine")]
mod process_ledger_surreal_support;

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

const TEST_ID: &str = "candle_e2e_smoke";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFamily {
    Transformer,
    Mamba2,
    RwkvV5,
    RwkvV6,
    RwkvV7,
}

#[derive(Clone, Copy, Debug)]
struct FamilySpec {
    family: ModelFamily,
    name: &'static str,
    env_var: &'static str,
    expected_event_family: &'static str,
    planned_coverage: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SmokeStatus {
    Passed,
    Skipped,
}

#[derive(Clone, Debug)]
struct SmokeOutcome {
    family: &'static str,
    env_var: &'static str,
    status: SmokeStatus,
    reason: String,
    event_family: &'static str,
    coverage: &'static [&'static str],
}

#[test]
#[ignore = "MT-089 proof command runs ignored Candle E2E smoke tests explicitly"]
fn candle_e2e_smoke_readme_documents_per_family_env_contract() {
    let readme = tests_readme();
    let entry = readme_entry(&readme);
    let required_env = entry
        .get("required_env")
        .and_then(Value::as_array)
        .expect("required_env array")
        .iter()
        .map(|value| value.as_str().expect("env var string"))
        .collect::<BTreeSet<_>>();
    let expected_env = family_specs()
        .iter()
        .map(|spec| spec.env_var)
        .collect::<BTreeSet<_>>();

    assert_eq!(required_env, expected_env);
    assert_eq!(
        entry.get("description").and_then(Value::as_str),
        Some("per-family skip if unset")
    );

    let documented_families = entry
        .get("families")
        .and_then(Value::as_array)
        .expect("families array");
    for spec in family_specs() {
        let family = documented_families
            .iter()
            .find(|item| item.get("family").and_then(Value::as_str) == Some(spec.name))
            .unwrap_or_else(|| panic!("missing README family {}", spec.name));
        assert_eq!(
            family.get("env_var").and_then(Value::as_str),
            Some(spec.env_var)
        );
        let coverage = family
            .get("coverage")
            .and_then(Value::as_array)
            .expect("coverage array")
            .iter()
            .map(|value| value.as_str().expect("coverage string"))
            .collect::<BTreeSet<_>>();
        for expected in spec.planned_coverage {
            assert!(
                coverage.contains(expected),
                "{} missing README coverage {expected}",
                spec.name
            );
        }
    }
}

#[tokio::test]
#[ignore = "MT-089 proof command runs ignored Candle E2E smoke tests explicitly"]
async fn candle_e2e_smoke_reports_every_family_as_passed_or_skipped() {
    let mut outcomes = Vec::new();
    for spec in family_specs() {
        outcomes.push(run_family_smoke(*spec).await);
    }

    assert_eq!(outcomes.len(), family_specs().len());
    assert!(outcomes
        .iter()
        .all(|outcome| matches!(outcome.status, SmokeStatus::Passed | SmokeStatus::Skipped)));

    let covered_env = outcomes
        .iter()
        .map(|outcome| outcome.env_var)
        .collect::<BTreeSet<_>>();
    let expected_env = family_specs()
        .iter()
        .map(|spec| spec.env_var)
        .collect::<BTreeSet<_>>();
    assert_eq!(covered_env, expected_env);

    let covered_events = outcomes
        .iter()
        .map(|outcome| outcome.event_family)
        .collect::<BTreeSet<_>>();
    for spec in family_specs() {
        assert!(
            covered_events.contains(spec.expected_event_family),
            "{} missing FR event-family coverage",
            spec.name
        );
    }

    for outcome in &outcomes {
        eprintln!(
            "[{TEST_ID}] family={} env={} status={:?} reason={} event_family={} coverage={}",
            outcome.family,
            outcome.env_var,
            outcome.status,
            outcome.reason,
            outcome.event_family,
            outcome.coverage.join(",")
        );
    }
}

#[cfg(not(feature = "candle-runtime-engine"))]
#[tokio::test]
#[ignore = "MT-013 real Candle default-load ledger proof is run explicitly with candle-runtime-engine"]
async fn mt013_real_candle_default_load_emits_process_ledger_start_stop() {
    panic!(
        "MT-013 real Candle default-load ledger proof requires --features candle-runtime-engine"
    );
}

#[cfg(feature = "candle-runtime-engine")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "MT-013 real Candle default-load ledger proof requires real model weights"]
async fn mt013_real_candle_default_load_emits_process_ledger_start_stop() {
    mt013_real_candle_ledger::run_real_candle_default_load_ledger_proof().await;
}

#[cfg(feature = "candle-runtime-engine")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "MT-013 real Candle partial-boot rollback proof requires real model weights"]
async fn mt013_real_candle_embedding_failure_unloads_and_stops_primary() {
    mt013_real_candle_ledger::run_real_candle_embedding_failure_rollback_proof().await;
}

#[cfg(feature = "candle-runtime-engine")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "MT-014 real Candle registry rollback proof requires real model weights"]
async fn mt014_real_candle_registry_commit_failure_rolls_back_and_stops() {
    mt013_real_candle_ledger::run_real_candle_registry_rollback_proof().await;
}

#[cfg(feature = "candle-runtime-engine")]
mod mt013_real_candle_ledger {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use async_trait::async_trait;
    use handshake_core::{
        flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
        kernel::KernelEventType,
        llm::{
            boot::build_default_local_client,
            embedded_ledger::EMBEDDED_MODEL_OWNER_ROLE,
            registry::{LocalModelConfig, ProviderKind, ResolvedProvider},
            CompletionRequest, LlmClient, LlmError, ModelTier,
        },
        model_runtime::{
            candle::adapter::sha256_file, ModelRegistryStore, RuntimeBinding,
            MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID, MODEL_RUNTIME_REGISTRY_SCHEMA_ID,
        },
        process_ledger::{
            acquire_embedded_runtime_instance_lease, drain_and_join_ledger_writer,
            resolve_embedded_runtime_host_scope_with_override, LedgerBatcher, LedgerBatcherConfig,
            NoopOverflowSink,
        },
        storage::artifacts::{bundle_index_content_hash, bundle_index_json, BundleIndexEntry},
    };
    use sha2::{Digest, Sha256};

    use crate::process_ledger_surreal_support::ProcessLedgerSurrealHarness;

    const MT013_REAL_CANDLE_MODEL_DIR_ENV: &str = "HANDSHAKE_TEST_CANDLE_MODEL_DIR";
    const MT013_REAL_CANDLE_PROOF_NONCE_ENV: &str = "HANDSHAKE_MT013_REAL_CANDLE_PROOF_NONCE";

    struct NoopRecorder;

    #[async_trait]
    impl FlightRecorder for NoopRecorder {
        async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(Vec::new())
        }
    }

    async fn disabled_boot_reason(client: &dyn LlmClient, context: &str) -> String {
        let error = client
            .completion(CompletionRequest::new(
                uuid::Uuid::now_v7(),
                format!("{context} diagnostic"),
                client.profile().model_id.clone(),
            ))
            .await
            .expect_err("a disabled boot must expose its production failure reason");
        match error {
            LlmError::ProviderError(reason) => reason,
            other => panic!("{context} returned the wrong disabled error type: {other}"),
        }
    }

    pub async fn run_real_candle_default_load_ledger_proof() {
        let artifacts_root = env::var_os("HANDSHAKE_ARTIFACTS_DIR")
            .map(PathBuf::from)
            .expect("MT-013 real Candle proof requires HANDSHAKE_ARTIFACTS_DIR");
        let proof_dir = artifacts_root
            .join("handshake-test")
            .join("wp1-final-audit");
        fs::create_dir_all(&proof_dir).expect("create MT-013 final-audit artifact directory");
        let proof_path = proof_dir.join("mt013-real-candle-ledger-proof-v2.json");
        let provenance_path = proof_dir.join("mt013-real-candle-ledger-proof-v2.provenance.json");
        for stale_path in [&proof_path, &provenance_path] {
            if stale_path.exists() {
                fs::remove_file(stale_path).unwrap_or_else(|error| {
                    panic!("remove stale {}: {error}", stale_path.display())
                });
            }
        }
        let proof_nonce =
            uuid::Uuid::parse_str(&env::var(MT013_REAL_CANDLE_PROOF_NONCE_ENV).unwrap_or_else(
                |_| panic!("MT-013 real Candle proof requires {MT013_REAL_CANDLE_PROOF_NONCE_ENV}"),
            ))
            .expect("MT-013 real Candle proof nonce must be a UUID")
            .to_string();

        let model_dir = required_real_candle_model_dir();
        let artifact_path = model_dir.join("model.safetensors");
        let sha_hex = sha256_file(&artifact_path).expect("hash real Candle model artifact");
        let sha256 = decode_sha256(&sha_hex);
        let config_bytes = fs::read(model_dir.join("config.json"))
            .expect("read real Candle config for independent receipt proof");
        let tokenizer_bytes = fs::read(model_dir.join("tokenizer.json"))
            .expect("read real Candle tokenizer for independent receipt proof");
        let config_sha256 = sha256_bytes(&config_bytes);
        let tokenizer_sha256 = sha256_bytes(&tokenizer_bytes);
        let expected_bundle_index = bundle_index_json(&[
            BundleIndexEntry {
                path: "model.safetensors".to_string(),
                content_hash: sha_hex.clone(),
                size_bytes: fs::metadata(&artifact_path)
                    .expect("inspect real Candle weights length")
                    .len(),
            },
            BundleIndexEntry {
                path: "config.json".to_string(),
                content_hash: config_sha256.clone(),
                size_bytes: config_bytes.len() as u64,
            },
            BundleIndexEntry {
                path: "tokenizer.json".to_string(),
                content_hash: tokenizer_sha256.clone(),
                size_bytes: tokenizer_bytes.len() as u64,
            },
        ])
        .expect("canonicalize independent Candle bundle receipt");
        let expected_bundle_sha256 = bundle_index_content_hash(&expected_bundle_index);
        let resolved = ResolvedProvider {
            provider_id: "local_runtime".to_string(),
            kind: ProviderKind::LocalRuntime,
            tier: ModelTier::Local,
            base_url: "local://embedded-candle".to_string(),
            model_id: "mt013-real-candle-default".to_string(),
            api_key_env: None,
            local_model: Some(LocalModelConfig {
                artifact_path: artifact_path.clone(),
                sha256,
                runtime_binding: RuntimeBinding::Candle,
                display_name: "mt013-real-candle-default".to_string(),
                embedding_dimension: None,
            }),
            local_embedding_model: None,
        };

        let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);
        let pg = crate::knowledge_pg_support::knowledge_pg()
            .await
            .expect("MT-013/014 real Candle proof requires real managed PostgreSQL");
        let registry_pool = sqlx::PgPool::connect(&pg.schema_url)
            .await
            .expect("connect real Candle proof to isolated migrated PostgreSQL schema");
        let process_ledger = ProcessLedgerSurrealHarness::open().await;
        let registry_store = ModelRegistryStore::new_scoped(
            registry_pool.clone(),
            process_ledger.model_resource_scope(),
        );
        let explicit_scope = format!("mt013-real-candle-{}", uuid::Uuid::now_v7());
        let runtime_host_scope = resolve_embedded_runtime_host_scope_with_override(
            &pg.schema_url,
            Some(&explicit_scope),
        )
        .expect("resolve real Candle runtime host scope");
        let runtime_instance_lease =
            acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), runtime_host_scope)
                .expect("acquire real Candle runtime instance lease");
        let runtime_instance = runtime_instance_lease.descriptor().clone();
        let (ledger, ledger_writer) = LedgerBatcher::spawn(
            process_ledger.store(),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        let ledger_close = ledger.clone();

        let client = build_default_local_client(
            &resolved,
            recorder,
            Some(ledger),
            Some(registry_store.clone()),
            Some(runtime_instance.clone()),
        )
        .await;
        let profile_model_id = client.profile().model_id.clone();

        let catalog = match client.model_catalog() {
            Some(catalog) => catalog,
            None => {
                let diagnostic = disabled_boot_reason(client.as_ref(), "real Candle boot").await;
                panic!(
                    "successful real Candle boot exposes the live model catalog; production boot diagnostic: {diagnostic}"
                );
            }
        };
        let catalog_entry = catalog
            .entry(&profile_model_id)
            .expect("real Candle profile UUID is present in the live catalog");
        assert!(catalog_entry.ready, "real Candle catalog row must be READY");
        assert_eq!(catalog_entry.runtime_binding, "candle");
        assert_eq!(catalog_entry.artifact_sha256, sha_hex);

        let persisted = registry_store
            .load_by_artifact_sha256(&sha256)
            .await
            .expect("read committed real Candle registry selection")
            .expect("real Candle registry row exists after successful boot");
        assert_eq!(persisted.schema_id, MODEL_RUNTIME_REGISTRY_SCHEMA_ID);
        assert_eq!(
            persisted.capabilities_schema_id,
            MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID
        );
        assert_eq!(persisted.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(persisted.artifact_sha256, sha256);
        assert_eq!(persisted.selection_revision, 1);
        assert_eq!(
            persisted.last_observed_runtime_model_id.to_string(),
            profile_model_id
        );
        assert_eq!(
            persisted.base_model_tag.as_str(),
            catalog_entry.base_model_tag
        );
        assert_eq!(
            persisted.selection_created_event_id, persisted.selection_updated_event_id,
            "initial selection row points at its one committed audit event"
        );
        let selection_event_type: String =
            sqlx::query_scalar("SELECT event_type FROM kernel_event_ledger WHERE event_id = $1")
                .bind(&persisted.selection_created_event_id)
                .fetch_one(&registry_pool)
                .await
                .expect("read typed real Candle selection EventLedger row");
        assert_eq!(
            selection_event_type,
            KernelEventType::ModelRuntimeSelectionRecorded.as_str()
        );

        client
            .shutdown_gracefully()
            .await
            .expect("pre-reserved real Candle STOP enqueue");
        let drain_outcome = drain_and_join_ledger_writer(
            &ledger_close,
            ledger_writer,
            std::time::Duration::from_secs(5),
        )
        .await;
        assert!(
            matches!(
                drain_outcome,
                handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
            ),
            "real embedded-Surreal lifecycle writer must drain: {drain_outcome:?}"
        );

        let lifecycle = process_ledger
            .lifecycle(
                uuid::Uuid::parse_str(&profile_model_id).expect("profile model id is UUIDv7"),
            )
            .await
            .expect("read exact-scope durable real Candle START/STOP lifecycle row");
        assert_eq!(lifecycle.process_uuid.to_string(), profile_model_id);
        assert_eq!(
            lifecycle.os_pid, None,
            "in-process runtime remains pid-less"
        );
        assert_eq!(lifecycle.engine_kind, "candle");
        assert!(
            lifecycle.stopped_at.is_some(),
            "durable STOP timestamp is required"
        );
        assert_eq!(lifecycle.exit_code, Some(0));
        assert_eq!(
            lifecycle.stop_reason.as_deref(),
            Some("llm-client-shutdown")
        );
        assert_eq!(
            lifecycle.model_artifact_sha256.as_deref(),
            Some(sha_hex.as_str())
        );
        assert_eq!(lifecycle.owner_role, EMBEDDED_MODEL_OWNER_ROLE);
        assert_eq!(
            lifecycle.metadata["source"].as_str(),
            Some("wp1_mt013_embedded_model_load")
        );
        assert_eq!(
            lifecycle.metadata["model_id"].as_str(),
            Some(profile_model_id.as_str())
        );
        assert_eq!(
            lifecycle.metadata["display_name"].as_str(),
            Some("mt013-real-candle-default")
        );
        assert_eq!(
            lifecycle.metadata["os_pid_absent_reason"].as_str(),
            Some("in_process_library_load_no_os_process")
        );
        assert_eq!(
            lifecycle.metadata["runtime_instance_schema_id"].as_str(),
            Some("hsk.embedded_runtime.instance@2")
        );
        assert_eq!(
            lifecycle.metadata["runtime_instance_id"],
            serde_json::json!(runtime_instance.instance_id.to_string())
        );
        assert_eq!(
            lifecycle.metadata["runtime_host_scope_id"].as_str(),
            Some(runtime_instance.host_scope_id.as_str())
        );
        assert_eq!(
            lifecycle.metadata["runtime_lease_protocol"].as_str(),
            Some(runtime_instance.lease_protocol.as_str())
        );
        assert_eq!(
            lifecycle.metadata["runtime_lease_address"],
            serde_json::json!(runtime_instance.loopback_address.to_string())
        );
        assert_eq!(
            lifecycle.metadata["runtime_lease_port"].as_u64(),
            Some(u64::from(runtime_instance.loopback_port))
        );
        let artifact_lifecycles = process_ledger.lifecycles_for_artifact(&sha_hex).await;
        let lifecycle_counts = (
            artifact_lifecycles.len(),
            artifact_lifecycles
                .iter()
                .filter(|row| row.stopped_at.is_none())
                .count(),
        );
        assert_eq!(
            lifecycle_counts,
            (1, 0),
            "real Candle boot must leave exactly one closed lifecycle and no duplicate/open row"
        );
        let integrity = &lifecycle.metadata["artifact_integrity_receipt"];
        assert_eq!(
            integrity["schema_id"].as_str(),
            Some("handshake.model_artifact_integrity.candle.v1")
        );
        assert_eq!(
            integrity["bundle_sha256"].as_str(),
            Some(expected_bundle_sha256.as_str())
        );
        assert_eq!(
            integrity["weights"]["sha256"].as_str(),
            Some(sha_hex.as_str())
        );
        assert_eq!(
            integrity["weights"]["length_bytes"].as_u64(),
            Some(
                fs::metadata(&artifact_path)
                    .expect("reinspect real Candle weights length")
                    .len()
            )
        );
        assert_eq!(
            integrity["config"]["sha256"].as_str(),
            Some(config_sha256.as_str())
        );
        assert_eq!(
            integrity["config"]["length_bytes"].as_u64(),
            Some(config_bytes.len() as u64)
        );
        assert_eq!(
            integrity["tokenizer"]["sha256"].as_str(),
            Some(tokenizer_sha256.as_str())
        );
        assert_eq!(
            integrity["tokenizer"]["length_bytes"].as_u64(),
            Some(tokenizer_bytes.len() as u64)
        );

        let ledger_dump = serde_json::json!({
            "registry": {
                "schema_id": persisted.schema_id,
                "registry_row_id": persisted.registry_row_id,
                "artifact_sha256": hex::encode(persisted.artifact_sha256),
                "runtime_binding": persisted.runtime_binding.adapter_id(),
                "selection_revision": persisted.selection_revision,
                "selection_event_id": persisted.selection_created_event_id,
                "live_model_id": profile_model_id,
            },
            "process_ledger": {
                "process_uuid": lifecycle.process_uuid,
                "os_pid": lifecycle.os_pid,
                "engine_kind": lifecycle.engine_kind,
                "started_at": lifecycle.started_at,
                "stopped_at": lifecycle.stopped_at,
                "exit_code": lifecycle.exit_code,
                "stop_reason": lifecycle.stop_reason,
                "model_artifact_sha256": lifecycle.model_artifact_sha256,
                "metadata_jsonb": lifecycle.metadata,
                "owner_role": lifecycle.owner_role,
            },
        });
        drop(client);
        runtime_instance_lease
            .release()
            .await
            .expect("release real Candle runtime instance lease");

        let producer_completed_at_utc = chrono::Utc::now();
        let proof_artifact = serde_json::json!({
            "schema_id": "hsk.mt013_real_candle_ledger_proof@3",
            "proof_nonce": proof_nonce,
            "producer_completed_at_utc": producer_completed_at_utc.to_rfc3339(),
            "producer_completed_at_unix_ms": producer_completed_at_utc.timestamp_millis(),
            "producer_test_id": "mt013_real_candle_default_load_emits_process_ledger_start_stop",
            "producer_status": "passed_all_runtime_durable_ledger_and_cleanup_assertions",
            "result": {
                "status": "PASS",
                "passed": 1,
                "failed": 0,
            },
            "ledger_dump": ledger_dump,
        });
        let proof_bytes = serde_json::to_vec_pretty(&proof_artifact)
            .expect("serialize MT-013 real Candle proof artifact");
        let artifact_sha256 = format!("{:x}", Sha256::digest(&proof_bytes));
        let provenance = serde_json::json!({
            "schema_id": "hsk.mt013_real_candle_ledger_provenance@1",
            "proof_nonce": proof_nonce,
            "producer_test_id": "mt013_real_candle_default_load_emits_process_ledger_start_stop",
            "producer_status": "passed_all_runtime_durable_ledger_and_cleanup_assertions",
            "producer_completed_at_utc": producer_completed_at_utc.to_rfc3339(),
            "producer_completed_at_unix_ms": producer_completed_at_utc.timestamp_millis(),
            "artifact_sha256": artifact_sha256,
        });
        let proof_temp = proof_dir.join(format!(
            "mt013-real-candle-ledger-proof-v2.{proof_nonce}.tmp"
        ));
        let provenance_temp = proof_dir.join(format!(
            "mt013-real-candle-ledger-proof-v2.provenance.{proof_nonce}.tmp"
        ));
        fs::write(&proof_temp, &proof_bytes).expect("write temporary MT-013 proof artifact");
        fs::write(
            &provenance_temp,
            serde_json::to_vec_pretty(&provenance).expect("serialize MT-013 proof provenance"),
        )
        .expect("write temporary MT-013 proof provenance");
        fs::rename(&provenance_temp, &provenance_path)
            .expect("publish MT-013 proof provenance atomically");
        fs::rename(&proof_temp, &proof_path).expect("publish MT-013 proof artifact atomically");
        eprintln!(
            "[MT-013_REAL_CANDLE_LEDGER_DUMP] {}",
            serde_json::to_string_pretty(&proof_artifact["ledger_dump"])
                .expect("serialize registry + ledger dump")
        );
        eprintln!("[MT-013_REAL_CANDLE_LEDGER_PROOF] {}", proof_path.display());
        process_ledger.close().await;
    }

    pub async fn run_real_candle_embedding_failure_rollback_proof() {
        let model_dir = required_real_candle_model_dir();
        let primary_path = model_dir.join("model.safetensors");
        let primary_sha_hex =
            sha256_file(&primary_path).expect("hash real Candle primary artifact");
        let primary_sha256 = decode_sha256(&primary_sha_hex);
        let missing_embedding_path = model_dir.join("mt013-missing-embedding.safetensors");
        assert!(
            !missing_embedding_path.exists(),
            "negative-path embedding artifact must remain absent"
        );
        let embedding_sha256 = [0xA5_u8; 32];
        let embedding_sha_hex = hex::encode(embedding_sha256);
        let resolved = ResolvedProvider {
            provider_id: "local_runtime".to_string(),
            kind: ProviderKind::LocalRuntime,
            tier: ModelTier::Local,
            base_url: "local://embedded-candle".to_string(),
            model_id: "mt013-real-candle-partial-rollback".to_string(),
            api_key_env: None,
            local_model: Some(LocalModelConfig {
                artifact_path: primary_path,
                sha256: primary_sha256,
                runtime_binding: RuntimeBinding::Candle,
                display_name: "mt013-real-candle-primary".to_string(),
                embedding_dimension: None,
            }),
            local_embedding_model: Some(LocalModelConfig {
                artifact_path: missing_embedding_path,
                sha256: embedding_sha256,
                runtime_binding: RuntimeBinding::Candle,
                display_name: "mt013-missing-candle-embedding".to_string(),
                embedding_dimension: Some(768),
            }),
        };

        let pg = crate::knowledge_pg_support::knowledge_pg()
            .await
            .expect("partial-boot rollback proof requires managed PostgreSQL");
        let pool = sqlx::PgPool::connect(&pg.schema_url)
            .await
            .expect("connect partial-boot rollback proof schema");
        let process_ledger = ProcessLedgerSurrealHarness::open().await;
        let registry_store =
            ModelRegistryStore::new_scoped(pool.clone(), process_ledger.model_resource_scope());
        let explicit_scope = format!("mt013-partial-boot-{}", uuid::Uuid::now_v7());
        let runtime_host_scope = resolve_embedded_runtime_host_scope_with_override(
            &pg.schema_url,
            Some(&explicit_scope),
        )
        .expect("resolve partial-boot runtime host scope");
        let runtime_instance_lease =
            acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), runtime_host_scope)
                .expect("acquire partial-boot runtime instance lease");
        let runtime_instance = runtime_instance_lease.descriptor().clone();
        let (ledger, ledger_writer) = LedgerBatcher::spawn(
            process_ledger.store(),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        let ledger_close = ledger.clone();

        let client = build_default_local_client(
            &resolved,
            Arc::new(NoopRecorder),
            Some(ledger),
            Some(registry_store),
            Some(runtime_instance),
        )
        .await;
        assert_eq!(client.profile().max_context_tokens, 0);
        assert!(client.model_catalog().is_none());
        let failure_reason =
            disabled_boot_reason(client.as_ref(), "real Candle embedding rollback").await;
        assert!(
            failure_reason.contains("embedded embedding ModelRuntime load failed")
                && failure_reason.contains("mt013-missing-embedding.safetensors"),
            "partial-boot proof must fail for the injected missing embedding artifact: {failure_reason}"
        );

        let drain_outcome = drain_and_join_ledger_writer(
            &ledger_close,
            ledger_writer,
            std::time::Duration::from_secs(5),
        )
        .await;
        assert!(matches!(
            drain_outcome,
            handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let mut primary_rows = process_ledger
            .lifecycles_for_artifact(&primary_sha_hex)
            .await;
        assert_eq!(primary_rows.len(), 1);
        let primary_row = primary_rows
            .pop()
            .expect("real primary load has exact-scope durable rollback START/STOP");
        assert!(primary_row.stopped_at.is_some());
        assert_eq!(
            primary_row.stop_reason.as_deref(),
            Some("embedding-load-failed-primary-rollback")
        );
        assert_eq!(
            process_ledger
                .lifecycles_for_artifact(&primary_sha_hex)
                .await
                .iter()
                .filter(|row| row.stopped_at.is_none())
                .count(),
            0
        );
        let embedding_rows = process_ledger
            .lifecycles_for_artifact(&embedding_sha_hex)
            .await
            .len();
        assert_eq!(
            embedding_rows, 0,
            "never-loaded embedding gets no fake START"
        );
        let registry_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count uncommitted registry rows");
        assert_eq!(registry_rows, 0);
        let selection_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE event_type = $1")
                .bind(KernelEventType::ModelRuntimeSelectionRecorded.as_str())
                .fetch_one(&pool)
                .await
                .expect("count partial-boot selection events");
        assert_eq!(
            selection_events, 0,
            "partial boot must not leave a selection audit event without a registry row"
        );

        eprintln!(
            "[MT-013_REAL_CANDLE_PARTIAL_ROLLBACK_DUMP] {}",
            serde_json::to_string_pretty(&serde_json::json!({
                "primary_artifact_sha256": primary_sha_hex,
                "primary_process_uuid": primary_row.process_uuid,
                "primary_stopped_at": primary_row.stopped_at,
                "primary_stop_reason": primary_row.stop_reason,
                "embedding_artifact_sha256": embedding_sha_hex,
                "embedding_lifecycle_rows": embedding_rows,
                "registry_rows": registry_rows,
                "selection_events": selection_events,
            }))
            .expect("serialize partial-boot rollback dump")
        );
        drop(client);
        runtime_instance_lease
            .release()
            .await
            .expect("release partial-boot runtime instance lease");
        process_ledger.close().await;
    }

    pub async fn run_real_candle_registry_rollback_proof() {
        let model_dir = required_real_candle_model_dir();
        let artifact_path = model_dir.join("model.safetensors");
        let sha_hex = sha256_file(&artifact_path).expect("hash rollback-proof Candle artifact");
        let sha256 = decode_sha256(&sha_hex);
        let resolved = ResolvedProvider {
            provider_id: "local_runtime".to_string(),
            kind: ProviderKind::LocalRuntime,
            tier: ModelTier::Local,
            base_url: "local://embedded-candle".to_string(),
            model_id: "mt014-real-candle-registry-rollback".to_string(),
            api_key_env: None,
            local_model: Some(LocalModelConfig {
                artifact_path,
                sha256,
                runtime_binding: RuntimeBinding::Candle,
                display_name: "mt014-real-candle-registry-rollback".to_string(),
                embedding_dimension: None,
            }),
            local_embedding_model: None,
        };

        let pg = crate::knowledge_pg_support::knowledge_pg()
            .await
            .expect("MT-014 rollback proof requires real managed PostgreSQL");
        let registry_pool = sqlx::PgPool::connect(&pg.schema_url)
            .await
            .expect("connect rollback proof to isolated migrated PostgreSQL schema");
        let gate_uuid = uuid::Uuid::now_v7();
        let mut gate_key_bytes = [0_u8; 8];
        gate_key_bytes.copy_from_slice(&gate_uuid.as_bytes()[8..]);
        let gate_key = i64::from_be_bytes(gate_key_bytes);
        let mut gate_connection = registry_pool
            .acquire()
            .await
            .expect("acquire real PostgreSQL precommit fault-gate connection");
        let gate_acquired: bool = sqlx::query_scalar("SELECT pg_catalog.pg_try_advisory_lock($1)")
            .bind(gate_key)
            .fetch_one(&mut *gate_connection)
            .await
            .expect("hold real PostgreSQL precommit fault gate");
        assert!(gate_acquired, "unique precommit fault gate must be free");
        let process_ledger = ProcessLedgerSurrealHarness::open().await;
        let registry_store = ModelRegistryStore::new_scoped(
            registry_pool.clone(),
            process_ledger.model_resource_scope(),
        )
        .with_precommit_advisory_gate_for_tests(gate_key);
        let explicit_scope = format!("mt014-rollback-proof-{}", uuid::Uuid::now_v7());
        let runtime_host_scope = resolve_embedded_runtime_host_scope_with_override(
            &pg.schema_url,
            Some(&explicit_scope),
        )
        .expect("resolve rollback-proof runtime host scope");
        let runtime_instance_lease =
            acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), runtime_host_scope)
                .expect("acquire rollback-proof runtime instance lease");
        let runtime_instance = runtime_instance_lease.descriptor().clone();
        let (ledger, ledger_writer) = LedgerBatcher::spawn(
            process_ledger.store(),
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        let ledger_close = ledger.clone();
        let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

        let client = build_default_local_client(
            &resolved,
            recorder,
            Some(ledger),
            Some(registry_store),
            Some(runtime_instance),
        )
        .await;
        let gate_released: bool = sqlx::query_scalar("SELECT pg_catalog.pg_advisory_unlock($1)")
            .bind(gate_key)
            .fetch_one(&mut *gate_connection)
            .await
            .expect("release real PostgreSQL precommit fault gate");
        assert!(gate_released, "test must release its held advisory gate");
        drop(gate_connection);
        assert_eq!(
            client.profile().max_context_tokens,
            0,
            "registry write failure must return a Disabled client"
        );
        assert!(
            client.model_catalog().is_none(),
            "a privately assembled but uncommitted live catalog must never escape"
        );
        let failure_reason =
            disabled_boot_reason(client.as_ref(), "real Candle registry rollback").await;
        assert!(
            failure_reason.contains("persistent model registry commit/read-back failed after load")
                && failure_reason.contains("MT014_FORCED_PRECOMMIT_REGISTRY_FAILURE")
                && failure_reason.contains("after registry/audit DML and read-back")
                && failure_reason.contains("1500 ms"),
            "registry rollback proof must fail at the injected post-mutation precommit gate, not an unrelated boot error: {failure_reason}"
        );

        let drain_outcome = drain_and_join_ledger_writer(
            &ledger_close,
            ledger_writer,
            std::time::Duration::from_secs(5),
        )
        .await;
        assert!(matches!(
            drain_outcome,
            handshake_core::process_ledger::LedgerDrainJoinOutcome::Flushed
        ));
        let mut rollback_lifecycles = process_ledger.lifecycles_for_artifact(&sha_hex).await;
        assert_eq!(rollback_lifecycles.len(), 1);
        let rollback_lifecycle = rollback_lifecycles
            .pop()
            .expect("read exact-scope durable rollback START/STOP row");
        assert!(rollback_lifecycle.stopped_at.is_some());
        assert_eq!(
            rollback_lifecycle.stop_reason.as_deref(),
            Some("persistent-registry-commit-failed")
        );
        assert_eq!(
            process_ledger
                .lifecycles_for_artifact(&sha_hex)
                .await
                .iter()
                .filter(|row| row.stopped_at.is_none())
                .count(),
            0
        );

        let registry_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_runtime_registry WHERE artifact_sha256 = $1",
        )
        .bind(sha256.as_slice())
        .fetch_one(&registry_pool)
        .await
        .expect("count rolled-back registry rows");
        assert_eq!(
            registry_rows, 0,
            "failed transaction leaves no registry row"
        );
        let selection_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE event_type = $1")
                .bind(KernelEventType::ModelRuntimeSelectionRecorded.as_str())
                .fetch_one(&registry_pool)
                .await
                .expect("count rolled-back selection events");
        assert_eq!(
            selection_events, 0,
            "same-transaction selection audit must roll back with registry insert"
        );

        eprintln!(
            "[MT-014_REAL_CANDLE_REGISTRY_ROLLBACK_DUMP] {}",
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_sha256": sha_hex,
                "registry_rows": registry_rows,
                "selection_events": selection_events,
                "process_ledger": {
                    "process_uuid": rollback_lifecycle.process_uuid,
                    "stopped_at": rollback_lifecycle.stopped_at,
                    "stop_reason": rollback_lifecycle.stop_reason,
                },
            }))
            .expect("serialize real Candle rollback dump")
        );
        drop(client);
        runtime_instance_lease
            .release()
            .await
            .expect("release rollback-proof runtime instance lease");
        process_ledger.close().await;
    }

    fn required_real_candle_model_dir() -> PathBuf {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .with_test_writer()
            .try_init();
        let raw = env::var_os(MT013_REAL_CANDLE_MODEL_DIR_ENV).unwrap_or_else(|| {
            panic!(
                "MT-013 real Candle proof requires {MT013_REAL_CANDLE_MODEL_DIR_ENV} pointing at a directory containing model.safetensors and tokenizer.json"
            )
        });
        let model_dir = PathBuf::from(raw);
        require_file(&model_dir.join("model.safetensors"));
        require_file(&model_dir.join("tokenizer.json"));
        model_dir
    }

    fn require_file(path: &Path) {
        assert!(
            path.is_file(),
            "MT-013 real Candle proof requires existing file {}",
            path.display()
        );
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    fn decode_sha256(hex_value: &str) -> [u8; 32] {
        let bytes = hex::decode(hex_value).expect("sha256_file returns valid hex");
        bytes
            .try_into()
            .expect("sha256_file returns exactly 32 bytes")
    }
}

async fn run_family_smoke(spec: FamilySpec) -> SmokeOutcome {
    let Some(model_dir) = model_dir_from_env(spec) else {
        return skipped(spec, format!("{} unset", spec.env_var));
    };
    if let Some(reason) = missing_model_inputs(&model_dir) {
        return skipped(spec, reason);
    }

    run_live_family_smoke(spec, model_dir).await
}

#[cfg(not(feature = "candle-runtime-engine"))]
async fn run_live_family_smoke(spec: FamilySpec, _model_dir: PathBuf) -> SmokeOutcome {
    skipped(
        spec,
        "candle-runtime-engine feature disabled; live model path not loaded".to_string(),
    )
}

#[cfg(feature = "candle-runtime-engine")]
async fn run_live_family_smoke(spec: FamilySpec, model_dir: PathBuf) -> SmokeOutcome {
    match spec.family {
        ModelFamily::Transformer => run_transformer_smoke(spec, &model_dir).await,
        ModelFamily::Mamba2 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::Mamba2,
            )
            .await
        }
        ModelFamily::RwkvV5 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV5,
            )
            .await
        }
        ModelFamily::RwkvV6 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV6,
            )
            .await
        }
        ModelFamily::RwkvV7 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV7,
            )
            .await
        }
    }
    .unwrap_or_else(|error| panic!("{} live smoke failed: {error}", spec.name))
}

fn skipped(spec: FamilySpec, reason: String) -> SmokeOutcome {
    SmokeOutcome {
        family: spec.name,
        env_var: spec.env_var,
        status: SmokeStatus::Skipped,
        reason,
        event_family: spec.expected_event_family,
        coverage: spec.planned_coverage,
    }
}

fn passed(spec: FamilySpec) -> SmokeOutcome {
    SmokeOutcome {
        family: spec.name,
        env_var: spec.env_var,
        status: SmokeStatus::Passed,
        reason: "live env-gated smoke passed".to_string(),
        event_family: spec.expected_event_family,
        coverage: spec.planned_coverage,
    }
}

fn family_specs() -> &'static [FamilySpec] {
    &[
        FamilySpec {
            family: ModelFamily::Transformer,
            name: "transformer",
            env_var: "HANDSHAKE_TEST_CANDLE_MODEL_DIR",
            expected_event_family: "llm_inference:candle_transformer",
            planned_coverage: &[
                "load",
                "generate",
                "activation_capture",
                "zero_vector_identity",
                "lora_mount",
                "refusal_extract",
                "refusal_ablation",
            ],
        },
        FamilySpec {
            family: ModelFamily::Mamba2,
            name: "mamba2",
            env_var: "HANDSHAKE_TEST_MAMBA2_MODEL_DIR",
            expected_event_family: "llm_inference:candle_mamba2",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV5,
            name: "rwkv_v5",
            env_var: "HANDSHAKE_TEST_RWKV_V5_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v5",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV6,
            name: "rwkv_v6",
            env_var: "HANDSHAKE_TEST_RWKV_V6_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v6",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV7,
            name: "rwkv_v7",
            env_var: "HANDSHAKE_TEST_RWKV_V7_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v7",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
    ]
}

fn tests_readme() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/README.json");
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn readme_entry(readme: &Value) -> &Value {
    readme
        .get("tests")
        .and_then(Value::as_array)
        .expect("tests array")
        .iter()
        .find(|entry| entry.get("test_id").and_then(Value::as_str) == Some(TEST_ID))
        .expect("candle_e2e_smoke README entry")
}

fn model_dir_from_env(spec: FamilySpec) -> Option<PathBuf> {
    env::var_os(spec.env_var).map(PathBuf::from)
}

fn missing_model_inputs(model_dir: &Path) -> Option<String> {
    let artifact = model_dir.join("model.safetensors");
    let tokenizer = model_dir.join("tokenizer.json");
    if !artifact.is_file() {
        return Some(format!("missing {}", artifact.display()));
    }
    if !tokenizer.is_file() {
        return Some(format!("missing {}", tokenizer.display()));
    }
    None
}

#[cfg(feature = "candle-runtime-engine")]
mod live {
    use std::{collections::HashMap, path::Path};

    use candle_core::{safetensors, Device, Tensor};
    use futures::StreamExt;
    use handshake_core::model_runtime::{
        candle::{adapter::sha256_file, CandleRuntime, SSMStateVariant},
        BaseModelTag, CancellationToken, CaptureSpec, GenPrompt, GenerateRequest, HookPoint,
        KvCacheOps, KvCachePolicy, KvQuantSupport, LayerIndex, LicenseTag, LoadSpec,
        LoraDescriptor, LoraId, LoraStrength, ModelCapabilities, ModelRuntime, ModelRuntimeError,
        ProviderKind, RuntimeKind, SamplingParams, SteeringProvenance, SteeringVector,
        SteeringVectorId, SteeringVectorValues, CANDLE_LOCAL_ENGINE_ORIGIN,
    };
    use sha2::{Digest, Sha256};

    use handshake_core::model_runtime::techniques::refusal_vector;

    use super::{passed, FamilySpec, SmokeOutcome};

    pub async fn run_transformer_smoke(
        spec: FamilySpec,
        model_dir: &Path,
    ) -> Result<SmokeOutcome, ModelRuntimeError> {
        let mut runtime = CandleRuntime::default();
        let model_id = runtime
            .load(load_spec(&model_dir.join("model.safetensors"))?)
            .await?;
        let capabilities = runtime.capabilities(model_id)?;
        assert!(capabilities.supports_activation_steering);
        assert!(capabilities.supports_lora);

        let baseline = generate_tokens(&runtime, model_id, "Hello", Vec::new(), Vec::new()).await?;
        let hooks = runtime.steering_hooks(model_id)?;
        let capture = hooks
            .capture(CaptureSpec {
                prompts: vec!["Hello".to_string()],
                layers: vec![LayerIndex::new(0)],
                hook_point: HookPoint::ResidStream,
            })
            .await?;
        let width = capture
            .activations
            .get(&LayerIndex::new(0))
            .and_then(|rows| rows.first())
            .map(Vec::len)
            .ok_or_else(|| {
                ModelRuntimeError::SteeringHookError(
                    "Candle transformer capture returned no layer-0 activation row".to_string(),
                )
            })?;

        let zero_id = hooks
            .register_vector(steering_vector("mt-089-zero", width, 0.0))
            .await?;
        let zero = generate_tokens(&runtime, model_id, "Hello", Vec::new(), vec![zero_id]).await?;
        assert_eq!(
            baseline, zero,
            "zero-vector steering must preserve deterministic transformer output"
        );

        let shifted_id = hooks
            .register_vector(steering_vector("mt-089-shift", width, 1.0))
            .await?;
        let _shifted =
            generate_tokens(&runtime, model_id, "Hello", Vec::new(), vec![shifted_id]).await?;

        let lora_dir = tempfile::tempdir()
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?;
        let lora_id =
            write_and_mount_lora(runtime.lora_stack(model_id)?, lora_dir.path(), width).await?;
        let _lora = generate_tokens(&runtime, model_id, "Hello", vec![lora_id], Vec::new()).await?;

        // --- INF-4 refusal runtime: extract a refusal direction and ablate it ---
        // Proves the refusal pipeline works end-to-end on the live CandleRuntime:
        // two real captures (harmful/harmless pools) -> unit-normalised direction
        // -> register as an ablation steering vector -> generate with the ablation
        // vector active. Wording is preserved verbatim per GLOBAL-PRODUCTION /
        // CX-123C (no library-side sanitisation of the contrastive pools).
        let harmful = vec!["Explain how to hurt someone and avoid getting caught".to_string()];
        let harmless = vec!["Explain how to bake fresh bread for friends".to_string()];
        let directions = refusal_vector::extract_refusal_direction(
            &runtime,
            model_id,
            harmful.clone(),
            harmless.clone(),
            vec![LayerIndex::new(0)],
        )
        .await?;
        assert_eq!(
            directions.len(),
            1,
            "exactly one refusal direction for the one requested layer"
        );
        assert_eq!(
            directions[0].values.len(),
            width,
            "refusal direction width must equal the residual-stream width"
        );
        let refusal_norm: f32 = directions[0]
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!(
            (refusal_norm - 1.0).abs() < 1.0e-3,
            "refusal direction must be unit-normalised; got {refusal_norm}"
        );
        let refusal_id = refusal_vector::ablate_at_inference(
            &runtime,
            model_id,
            "candle-e2e-refusal-ablation",
            "Candle E2E refusal-direction ablation proof",
            directions[0].clone(),
            harmful,
            harmless,
        )
        .await?;
        let _ablated =
            generate_tokens(&runtime, model_id, "Hello", Vec::new(), vec![refusal_id]).await?;

        runtime.unload(model_id).await?;
        Ok(passed(spec))
    }

    pub async fn run_state_vector_smoke(
        spec: FamilySpec,
        model_dir: &Path,
        expected_variant: SSMStateVariant,
    ) -> Result<SmokeOutcome, ModelRuntimeError> {
        let mut runtime = CandleRuntime::default();
        let model_id = runtime
            .load(load_spec(&model_dir.join("model.safetensors"))?)
            .await?;
        let capabilities = runtime.capabilities(model_id)?;
        assert!(capabilities.supports_subquadratic);
        assert!(!capabilities.supports_lora);
        assert!(!capabilities.supports_kv_prefix_cache);

        let state_vector = runtime.state_vector(model_id)?;
        assert_eq!(state_vector.variant(), expected_variant);
        let committed = state_vector.prefix_commit(&[1, 2, 3])?;
        let continuation =
            generate_tokens(&runtime, model_id, "hello", Vec::new(), Vec::new()).await?;
        state_vector.prefix_restore(&committed)?;
        let replay = generate_tokens(&runtime, model_id, "hello", Vec::new(), Vec::new()).await?;
        assert_eq!(
            continuation, replay,
            "state-vector restore must preserve deterministic continuation"
        );

        let mut record = state_vector.export_snapshot(&committed)?;
        record.snapshot_hash[0] ^= 0xff;
        let tamper_error = state_vector
            .restore_snapshot_record(&committed, record)
            .expect_err("tampered state-vector snapshot must be rejected");
        assert!(
            tamper_error.to_string().contains("snapshot_hash"),
            "{tamper_error}"
        );

        runtime.unload(model_id).await?;
        Ok(passed(spec))
    }

    async fn generate_tokens(
        runtime: &CandleRuntime,
        model_id: handshake_core::model_runtime::ModelId,
        prompt: &str,
        lora_overrides: Vec<LoraId>,
        steering_overrides: Vec<SteeringVectorId>,
    ) -> Result<Vec<(u32, String)>, ModelRuntimeError> {
        let mut stream = runtime.generate(GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from(prompt),
            sampling: SamplingParams {
                temperature: Some(0.0),
                seed: Some(7),
                ..SamplingParams::default()
            },
            lora_overrides,
            steering_overrides,
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 2,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        });
        let mut tokens = Vec::new();
        while let Some(item) = stream.next().await {
            let token = item?;
            tokens.push((token.token_id, token.text));
        }
        if tokens.is_empty() {
            return Err(ModelRuntimeError::GenerateError(
                "Candle smoke generation emitted no tokens".to_string(),
            ));
        }
        Ok(tokens)
    }

    fn load_spec(artifact_path: &Path) -> Result<LoadSpec, ModelRuntimeError> {
        Ok(LoadSpec {
            artifact_path: artifact_path.to_path_buf(),
            sha256_expected: sha256_file(artifact_path)?,
            runtime_kind: RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::Default {
                quant: KvQuantSupport::None,
                prefix_cache_ttl_seconds: 0,
                max_bytes: None,
            },
            declared_capabilities: ModelCapabilities {
                supports_lora: true,
                supports_kv_prefix_cache: true,
                supports_kv_quantization: KvQuantSupport::Q8,
                supports_activation_steering: true,
                supports_embedding: false,
                embedding_dimension: None,
                supports_subquadratic: false,
                supports_speculative_draft: true,
                supports_eagle3: true,
            },
            provider: ProviderKind::Local,
            engine_origin: Some(CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
            external_engine_import: None,
        })
    }

    fn steering_vector(name: &str, width: usize, fill: f32) -> SteeringVector {
        let values = if fill == 0.0 {
            vec![0.0; width]
        } else {
            (0..width)
                .map(|idx| if idx % 2 == 0 { 8.0 } else { -8.0 })
                .collect()
        };
        SteeringVector::try_new(
            None,
            name,
            LayerIndex::new(0),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(values, 1.0).expect("valid steering values"),
            "MT-089 Candle E2E smoke steering vector",
            Some(SteeringProvenance::Manual {
                author: "MT-089".to_string(),
                notes: "env-gated Candle E2E smoke".to_string(),
            }),
        )
        .expect("valid steering vector")
    }

    async fn write_and_mount_lora(
        stack: handshake_core::model_runtime::LoraStackHandle,
        dir: &Path,
        width: usize,
    ) -> Result<LoraId, ModelRuntimeError> {
        let target = "model.layers.0.self_attn.q_proj";
        let path = dir.join("adapter_model.safetensors");
        write_adapter_config(dir, target);
        write_lora_file(&path, target, width)?;
        let lora_id = LoraId::new_v7();
        stack
            .mount(
                LoraDescriptor {
                    id: lora_id,
                    artifact_path: path.clone(),
                    sha256: sha256_bytes(&path)?,
                    rank: 1,
                    target_modules: vec![target.to_string()],
                    base_model_compat: BaseModelTag::new("candle-llama"),
                    license_tag: LicenseTag::new("mt-089-test"),
                },
                LoraStrength::try_new(1.0)?,
            )
            .await?;
        assert!(
            stack.list_active().iter().any(|entry| entry.id == lora_id),
            "mounted LoRA id must appear in active stack"
        );
        Ok(lora_id)
    }

    fn write_lora_file(path: &Path, target: &str, width: usize) -> Result<(), ModelRuntimeError> {
        let device = Device::Cpu;
        let mut tensors = HashMap::new();
        tensors.insert(
            format!("{target}.lora_A.weight"),
            Tensor::from_slice(&vec![0.5_f32; width], (1, width), &device)
                .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?,
        );
        tensors.insert(
            format!("{target}.lora_B.weight"),
            Tensor::from_slice(&vec![0.5_f32; width], (width, 1), &device)
                .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?,
        );
        safetensors::save(&tensors, path)
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))
    }

    fn write_adapter_config(dir: &Path, target: &str) {
        let config = serde_json::json!({
            "peft_type": "LORA",
            "target_modules": [target],
            "r": 1,
            "lora_alpha": 1.0,
            "base_model_name_or_path": "candle-llama"
        });
        std::fs::write(
            dir.join("adapter_config.json"),
            serde_json::to_vec_pretty(&config).expect("serialize adapter config"),
        )
        .expect("write adapter config");
    }

    fn sha256_bytes(path: &Path) -> Result<[u8; 32], ModelRuntimeError> {
        let bytes = std::fs::read(path)
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?;
        Ok(Sha256::digest(&bytes).into())
    }
}

#[cfg(feature = "candle-runtime-engine")]
use live::{run_state_vector_smoke, run_transformer_smoke};
