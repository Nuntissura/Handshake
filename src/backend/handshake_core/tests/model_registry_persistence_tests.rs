//! WP-1 MT-014 V2 durable ModelRuntime registry proof.
//!
//! Every test requires real PostgreSQL through the existing Handshake-managed
//! helper. Missing PostgreSQL is a failing proof, never a green skip.

mod knowledge_pg_support;

use std::{
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    llm::{
        boot::build_default_local_client,
        registry::{LocalModelConfig, ProviderKind as LlmProviderKind, ResolvedProvider},
        CompletionRequest, LlmError, ModelTier,
    },
    model_runtime::{
        BaseModelTag, ExplicitModelRuntimeRebind, ModelCapabilities, ModelId, ModelRegistration,
        ModelRegistryPersistenceError, ModelRegistryStore, ModelRuntimeRole, ModelRuntimeSelection,
        ModelRuntimeSelectionPurpose, OperatorId, ProviderKind, RoleBoundModelRegistration,
        RuntimeBinding, MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID, MODEL_RUNTIME_REGISTRY_SCHEMA_ID,
        MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
    },
    process_ledger::{
        acquire_embedded_runtime_instance_lease, resolve_embedded_runtime_host_scope_with_override,
        LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, PostgresProcessLedgerStore,
    },
};
use serde_json::Value;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    Connection, Row,
};
use tokio::sync::Barrier;

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

fn capabilities(binding: RuntimeBinding) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_activation_steering: binding == RuntimeBinding::Candle,
        supports_embedding: true,
        embedding_dimension: Some(768),
        ..Default::default()
    }
}

fn registration(
    artifact_byte: u8,
    binding: RuntimeBinding,
    base_model_tag: &str,
    registered_by: &str,
    artifact_path: &str,
) -> ModelRegistration {
    ModelRegistration {
        model_id: ModelId::new_v7(),
        artifact_path: PathBuf::from(artifact_path),
        sha256: [artifact_byte; 32],
        runtime_binding: binding,
        declared_capabilities: capabilities(binding),
        base_model_tag: BaseModelTag::new(base_model_tag),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new(registered_by),
        provider: ProviderKind::Local,
    }
}

async fn pg_required(test_name: &str) -> knowledge_pg_support::KnowledgePg {
    knowledge_pg_support::knowledge_pg().await.unwrap_or_else(|| {
        panic!(
            "PostgreSQL unavailable for {test_name}: durable model-registry proof requires live Handshake-managed PostgreSQL"
        )
    })
}

async fn wait_for_registry_lock_wait(pool: &sqlx::PgPool, application_name: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let active: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM pg_catalog.pg_stat_activity
                WHERE application_name = $1
                  AND state = 'active'
                  AND wait_event_type = 'Lock'
            )
            "#,
        )
        .bind(application_name)
        .fetch_one(pool)
        .await
        .expect("observe the dedicated registry lock waiter");
        if active {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the registry transaction to block on the controlled row lock"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn unique_advisory_key() -> i64 {
    let value = uuid::Uuid::now_v7();
    i64::from_be_bytes(
        value.as_bytes()[..8]
            .try_into()
            .expect("UUID prefix is exactly eight bytes"),
    )
}

async fn wait_for_transaction_start_gate(
    control: &mut sqlx::PgConnection,
    application_name: &str,
) -> i32 {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let backend_pid = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT pid
            FROM pg_catalog.pg_stat_activity
            WHERE application_name = $1
              AND state = 'active'
              AND wait_event_type = 'Lock'
              AND query LIKE '%pg_advisory_xact_lock%'
            "#,
        )
        .bind(application_name)
        .fetch_optional(&mut *control)
        .await
        .expect("observe cancellation-safe transaction-start gate");
        if let Some(backend_pid) = backend_pid {
            return backend_pid;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for PostgreSQL to accept BEGIN and block at the controlled start gate"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn wait_for_backend_exit(control: &mut sqlx::PgConnection, backend_pid: i32) {
    // The authority guard directly drops a detached raw PgConnection. Allow a
    // bounded server-observation window backed by the test gate's three-second
    // server timeout, but require the exact backend PID to disappear rather
    // than accepting pool replacement alone.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let still_present: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_stat_activity WHERE pid = $1)",
        )
        .bind(backend_pid)
        .fetch_one(&mut *control)
        .await
        .expect("observe cancellation-contaminated backend termination");
        if !still_present {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "cancelled transaction-start backend was not physically closed"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn mt014_persistent_registry_survives_restart_and_reads_back_selection() {
    let pg = pg_required("mt014 persistent registry restart and committed readback").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL model registry authority");
    let original = registration(
        0x11,
        RuntimeBinding::Candle,
        "portable-registry-test",
        "mt014-initial-observer",
        "configured/models/portable-registry-test.gguf",
    );
    let first_store = ModelRegistryStore::new(pool.clone());
    let written = first_store
        .persist_and_read_back(&original)
        .await
        .expect("persist initial selected adapter and read committed row back");

    assert_eq!(written.schema_id, MODEL_RUNTIME_REGISTRY_SCHEMA_ID);
    assert_eq!(
        written.capabilities_schema_id,
        MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID
    );
    assert_eq!(written.artifact_sha256, original.sha256);
    assert_eq!(written.runtime_binding, RuntimeBinding::Candle);
    assert_eq!(written.selection_revision, 1);
    assert_eq!(
        written.selection_created_event_id,
        written.selection_updated_event_id
    );
    assert_eq!(
        written.artifact_locator,
        format!("sha256:{}", hex::encode(original.sha256))
    );
    assert!(!written.artifact_locator.contains("configured/models"));

    let event = sqlx::query(
        r#"
        SELECT event_type, actor_kind, actor_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&written.selection_created_event_id)
    .fetch_one(&pool)
    .await
    .expect("initial selection has a durable EventLedger row");
    assert_eq!(
        event.get::<String, _>("event_type"),
        KernelEventType::ModelRuntimeSelectionRecorded.as_str()
    );
    assert_eq!(event.get::<String, _>("actor_kind"), "system");
    assert_eq!(event.get::<String, _>("actor_id"), "model-runtime-registry");
    let payload: Value = event.get("payload");
    assert_eq!(
        payload["schema_id"],
        MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID
    );
    assert_eq!(payload["action"], "initial_selection");
    assert_eq!(payload["selection_revision"], 1);
    assert_eq!(
        payload["capabilities_schema_id"],
        MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID
    );

    drop(first_store);
    let restarted_store = ModelRegistryStore::new(pool.clone());
    let configured = ModelRuntimeSelection::from(&original);
    let recovered = restarted_store
        .recover_configured_selection_set(std::slice::from_ref(&configured))
        .await
        .expect("recover complete configured selection set after store restart")
        .pop()
        .expect("one configured selection preserves cardinality")
        .expect("durable selection survives store restart");
    assert_eq!(recovered.registry_row_id, written.registry_row_id);
    assert_eq!(recovered.selection(), configured);

    let next_boot_id = ModelId::new_v7();
    let rehydrated = recovered
        .rehydrate_with_current_runtime_model_id(
            next_boot_id,
            PathBuf::from("moved-project/models/portable-registry-test.gguf"),
        )
        .expect("project relocation uses current path and a fresh boot identity");
    assert_eq!(
        rehydrated.artifact_path,
        PathBuf::from("moved-project/models/portable-registry-test.gguf")
    );
    assert_ne!(rehydrated.model_id, written.last_observed_runtime_model_id);
    assert_eq!(
        restarted_store
            .list_recoverable()
            .await
            .expect("enumerate durable registry authority")
            .len(),
        1
    );

    sqlx::query(
        r#"
        UPDATE model_runtime_registry
        SET capabilities_json = capabilities_json || '{"future_unknown":true}'::jsonb
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(original.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("inject unknown capability under unchanged schema id");
    let noncanonical = restarted_store
        .load_by_artifact_sha256(&original.sha256)
        .await
        .expect_err("unknown capability requires a schema migration, never silent ignore");
    assert!(matches!(
        noncanonical,
        ModelRegistryPersistenceError::CorruptRow(_)
    ));
}

#[tokio::test]
async fn mt014_active_defaults_survive_restart_and_failed_cas_preserves_prior_selection() {
    let pg = pg_required("mt014 active default restart persistence and failed CAS").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated active-default authority");
    let completion_a = registration(
        0xa1,
        RuntimeBinding::Candle,
        "completion-a",
        "mt014-active-default-test",
        "models/completion-a.safetensors",
    );
    let completion_b = registration(
        0xa2,
        RuntimeBinding::Candle,
        "completion-b",
        "mt014-active-default-test",
        "models/completion-b.safetensors",
    );
    let embedding = registration(
        0xa3,
        RuntimeBinding::Candle,
        "embedding-default",
        "mt014-active-default-test",
        "models/embedding.safetensors",
    );
    let first_store = ModelRegistryStore::new(pool.clone());
    first_store
        .persist_role_bound_boot_set_and_read_back(&[
            RoleBoundModelRegistration::completion(completion_a.clone()),
            RoleBoundModelRegistration::completion(completion_b.clone()),
            RoleBoundModelRegistration::embedding(embedding.clone()),
        ])
        .await
        .expect("persist role-bound READY set");
    let initialized = first_store
        .ensure_active_defaults(&[
            (
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                completion_a.sha256,
            ),
            (
                ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                embedding.sha256,
            ),
        ])
        .await
        .expect("initialize both PostgreSQL-authoritative purposes");
    assert_eq!(initialized.len(), 2);

    let restarted_store = ModelRegistryStore::new(pool.clone());
    let recovered = restarted_store
        .ensure_active_defaults(&[
            (
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                completion_b.sha256,
            ),
            (
                ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                embedding.sha256,
            ),
        ])
        .await
        .expect("restart recovers existing defaults instead of replacing them");
    let recovered_application = recovered
        .iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default recovered");
    assert_eq!(recovered_application.artifact_sha256, completion_a.sha256);
    assert_eq!(recovered_application.selection_revision, 1);

    let changed = restarted_store
        .select_active_model(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_b.sha256,
            1,
            KernelActor::Operator("native-model-runtime-panel".to_owned()),
            "operator selected completion-b",
        )
        .await
        .expect("audited compare-and-set changes the active completion default");
    assert_eq!(changed.artifact_sha256, completion_b.sha256);
    assert_eq!(changed.selection_revision, 2);
    assert_ne!(
        changed.selection_created_event_id,
        changed.selection_updated_event_id
    );

    let stale_error = restarted_store
        .select_active_model(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_a.sha256,
            1,
            KernelActor::Operator("native-model-runtime-panel".to_owned()),
            "stale retry must fail closed",
        )
        .await
        .expect_err("stale CAS must preserve the committed active default");
    assert!(matches!(
        stale_error,
        ModelRegistryPersistenceError::SelectionRevisionMismatch {
            expected: 1,
            actual: 2
        }
    ));

    sqlx::query(
        r#"
        ALTER TABLE ONLY kernel_event_ledger
        ADD CONSTRAINT mt014_reject_new_active_selection_audit
        CHECK (aggregate_type <> 'model_runtime_active_selection') NOT VALID
        "#,
    )
    .execute(&pool)
    .await
    .expect("install a database-enforced rejection for the next active-selection audit");
    let audit_failure = restarted_store
        .select_active_model(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_a.sha256,
            2,
            KernelActor::Operator("native-model-runtime-panel".to_owned()),
            "audit rejection must roll back the selection",
        )
        .await
        .expect_err("active selection cannot commit without its EventLedger row");
    assert!(
        matches!(audit_failure, ModelRegistryPersistenceError::Audit(_)),
        "audit rejection must remain a typed audit failure: {audit_failure}"
    );
    let active_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_active_selection'",
    )
    .fetch_one(&pool)
    .await
    .expect("count committed active-selection audits after rejected append");
    assert_eq!(
        active_audit_count, 3,
        "two initial purpose events plus one successful application rebind must remain"
    );

    let after_failed_cas = ModelRegistryStore::new(pool.clone())
        .list_active_selections()
        .await
        .expect("fresh store recovers active selections after failed CAS and audit append");
    let application_after = after_failed_cas
        .iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default remains present");
    assert_eq!(application_after.artifact_sha256, completion_b.sha256);
    assert_eq!(application_after.selection_revision, 2);
    let embedding_after = after_failed_cas
        .iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::EmbeddingsDefault)
        .expect("embeddings/default remains present");
    assert_eq!(embedding_after.artifact_sha256, embedding.sha256);
}

/// PART 1 (MT-014 V5) closure proof: the active default-model selection is
/// durable, not process-local. An operator sets a new default (A -> B); after a
/// simulated restart (a brand-new `ModelRegistryStore` against the same
/// PostgreSQL) that re-registers the env-config boot candidate A, the durable
/// selection B is RESTORED rather than reset to the boot candidate.
#[tokio::test]
async fn mt014_active_default_selection_persists_and_restores_after_restart() {
    let pg = pg_required("mt014 active default persists and restores after restart").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated active-default restart authority");
    let completion_a = registration(
        0xb1,
        RuntimeBinding::Candle,
        "completion-a",
        "mt014-restart-restore-test",
        "models/restart-completion-a.safetensors",
    );
    let completion_b = registration(
        0xb2,
        RuntimeBinding::Candle,
        "completion-b",
        "mt014-restart-restore-test",
        "models/restart-completion-b.safetensors",
    );

    // Boot 1: persist two READY completion models and initialize the durable
    // application/default to A (revision 1).
    let boot_one = ModelRegistryStore::new(pool.clone());
    boot_one
        .persist_role_bound_boot_set_and_read_back(&[
            RoleBoundModelRegistration::completion(completion_a.clone()),
            RoleBoundModelRegistration::completion(completion_b.clone()),
        ])
        .await
        .expect("persist role-bound completion set");
    let initialized = boot_one
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_a.sha256,
        )])
        .await
        .expect("initialize durable application/default at boot");
    let initial_app = initialized
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default initialized");
    assert_eq!(initial_app.artifact_sha256, completion_a.sha256);
    assert_eq!(initial_app.selection_revision, 1);

    // Operator selects a NEW default model: audited compare-and-set A -> B.
    let selected = boot_one
        .select_active_model(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_b.sha256,
            1,
            KernelActor::Operator("native-model-runtime-panel".to_owned()),
            "operator selected completion-b as the default completion model",
        )
        .await
        .expect("durably set the new active default");
    assert_eq!(selected.artifact_sha256, completion_b.sha256);
    assert_eq!(selected.selection_revision, 2);

    // Simulate a full restart: a brand-new store instance against the same
    // PostgreSQL, with boot re-offering the env-config candidate A. The durable
    // operator selection (B) must be recovered, never overwritten by boot.
    let boot_two = ModelRegistryStore::new(pool.clone());
    let recovered = boot_two
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            completion_a.sha256,
        )])
        .await
        .expect("restart recovers the durable default instead of reinitializing");
    let recovered_app = recovered
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default recovered after restart");
    assert_eq!(
        recovered_app.artifact_sha256, completion_b.sha256,
        "restart restores the operator-selected default B, not the boot candidate A"
    );
    assert_eq!(
        recovered_app.selection_revision, 2,
        "restored default preserves its committed selection revision"
    );

    // An independent read-only recovery path (a third fresh store) agrees, so the
    // restored value is durable PostgreSQL authority, not a process-local cache.
    let independent = ModelRegistryStore::new(pool.clone())
        .list_active_selections()
        .await
        .expect("independent fresh store recovers the durable default after restart");
    let independent_app = independent
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default present in independent recovery");
    assert_eq!(independent_app.artifact_sha256, completion_b.sha256);
    assert_eq!(independent_app.selection_revision, 2);
}

#[tokio::test]
async fn mt014_registry_down_up_recovers_row_from_preserved_audit_chain() {
    let pg = pg_required("mt014 registry down-up audit recovery").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let original = registration(
        0x12,
        RuntimeBinding::Candle,
        "registry-before-recreation",
        "mt014-before-recreation",
        "models/registry-before-recreation.safetensors",
    );
    let original_store = ModelRegistryStore::new(pool.clone());
    let before = original_store
        .persist_and_read_back(&original)
        .await
        .expect("persist registry row and initial audit event before recreation");
    let rebound_target_registration = registration(
        0x12,
        RuntimeBinding::LlamaCpp,
        "registry-rebound-selection",
        "mt014-rebound-operator-observation",
        "models/registry-rebound-selection.gguf",
    );
    let rebound_target = ModelRuntimeSelection::from(&rebound_target_registration);
    let rebound = original_store
        .rebind_selection_for_tests(
            &rebound_target,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("mt014-recovery-operator".to_string()),
                "prove revision-two audit reconstruction after projection recreation",
                1,
            )
            .expect("construct revision-two recovery proof rebind"),
        )
        .await
        .expect("persist revision-two selection and audit event before recreation");
    assert_eq!(rebound.selection_revision, 2);
    assert_eq!(rebound.selection(), rebound_target);
    assert_eq!(rebound.registry_row_id, before.registry_row_id);
    assert_eq!(
        rebound.selection_created_event_id,
        before.selection_created_event_id
    );
    let revision_one_original = registration(
        0x14,
        RuntimeBinding::Candle,
        "registry-revision-one-before-recreation",
        "mt014-revision-one-before-recreation",
        "models/registry-revision-one-before-recreation.safetensors",
    );
    let revision_one_before = original_store
        .persist_and_read_back(&revision_one_original)
        .await
        .expect("persist independent revision-one row before recreation");

    sqlx::raw_sql(include_str!(
        "../migrations/0348_model_runtime_registry.down.sql"
    ))
    .execute(&pool)
    .await
    .expect("remove only the projection tables while retaining EventLedger audit history");
    sqlx::raw_sql(include_str!(
        "../migrations/0348_model_runtime_registry.sql"
    ))
    .execute(&pool)
    .await
    .expect("recreate registry projection tables from the governed migration");

    let current = registration(
        0x12,
        RuntimeBinding::LlamaCpp,
        "registry-after-recreation",
        "mt014-after-recreation",
        "moved/models/registry-after-recreation.gguf",
    );
    assert_ne!(current.model_id, original.model_id);
    let recreated_store = ModelRegistryStore::new(pool.clone());
    let recovered = recreated_store
        .persist_and_read_back(&current)
        .await
        .expect("recover recreated projection from the preserved immutable audit chain");

    assert_eq!(recovered.selection(), ModelRuntimeSelection::from(&current));
    assert_eq!(recovered.selection_revision, rebound.selection_revision);
    assert_eq!(recovered.registry_row_id, rebound.registry_row_id);
    assert_eq!(
        recovered.selection_created_event_id,
        rebound.selection_created_event_id
    );
    assert_eq!(
        recovered.selection_updated_event_id,
        rebound.selection_updated_event_id
    );
    assert_eq!(
        recovered.selection_created_at_utc,
        rebound.selection_created_at_utc
    );
    assert_eq!(
        recovered.selection_updated_at_utc,
        rebound.selection_updated_at_utc
    );
    assert_eq!(recovered.last_observed_runtime_model_id, current.model_id);
    assert_eq!(recovered.base_model_tag, current.base_model_tag);
    assert_eq!(recovered.last_observed_by, current.registered_by);

    let revision_one_current = registration(
        0x14,
        RuntimeBinding::Candle,
        "registry-revision-one-after-recreation",
        "mt014-revision-one-after-recreation",
        "moved/models/registry-revision-one-after-recreation.safetensors",
    );
    let revision_one_recovered = recreated_store
        .persist_and_read_back(&revision_one_current)
        .await
        .expect("recover revision-one row identity and chronology from its initial audit");
    assert_eq!(
        revision_one_recovered.registry_row_id,
        revision_one_before.registry_row_id
    );
    assert_eq!(revision_one_recovered.selection_revision, 1);
    assert_eq!(
        revision_one_recovered.selection_created_at_utc,
        revision_one_before.selection_created_at_utc
    );
    assert_eq!(
        revision_one_recovered.selection_updated_at_utc,
        revision_one_before.selection_updated_at_utc
    );

    let aggregate_id = format!("sha256:{}", hex::encode(current.sha256));
    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.count(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_runtime_registry'
          AND aggregate_id = $1
          AND source_component = 'model_runtime_registry'
        "#,
    )
    .bind(aggregate_id)
    .fetch_one(&pool)
    .await
    .expect("count preserved registry audit chain");
    assert_eq!(audit_count, 2, "recovery must not append a duplicate event");
}

#[tokio::test]
async fn mt014_projection_recreation_rejects_audit_conflict_before_artifact_access() {
    let pg = pg_required("mt014 projection recreation pre-artifact audit gate").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let original = registration(
        0x13,
        RuntimeBinding::Candle,
        "audit-preflight-original",
        "audit-preflight-observer",
        "models/audit-preflight-original.safetensors",
    );
    let store = ModelRegistryStore::new(pool.clone());
    let initial = store
        .persist_and_read_back(&original)
        .await
        .expect("persist initial selection before preflight audit proof");
    let rebound_registration = registration(
        0x13,
        RuntimeBinding::LlamaCpp,
        "audit-preflight-rebound",
        "audit-preflight-rebind-observer",
        "models/audit-preflight-rebound.gguf",
    );
    let rebound = store
        .rebind_selection_for_tests(
            &ModelRuntimeSelection::from(&rebound_registration),
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("mt014-preflight-operator".to_string()),
                "prove projection loss cannot bypass immutable selection before artifact access",
                initial.selection_revision,
            )
            .expect("construct audited revision-two rebind"),
        )
        .await
        .expect("persist revision-two selection before projection recreation");
    assert_eq!(rebound.selection_revision, 2);

    sqlx::raw_sql(include_str!(
        "../migrations/0348_model_runtime_registry.down.sql"
    ))
    .execute(&pool)
    .await
    .expect("drop registry projection while preserving its EventLedger chain");
    sqlx::raw_sql(include_str!(
        "../migrations/0348_model_runtime_registry.sql"
    ))
    .execute(&pool)
    .await
    .expect("recreate empty registry projection");

    let missing_artifact = PathBuf::from(format!(
        "must-not-open-after-projection-loss-{}.safetensors",
        uuid::Uuid::now_v7()
    ));
    assert!(!missing_artifact.exists());
    let resolved = ResolvedProvider {
        provider_id: "local_runtime".to_string(),
        kind: LlmProviderKind::LocalRuntime,
        tier: ModelTier::Local,
        base_url: "local://embedded".to_string(),
        model_id: "mt014-audit-preflight-conflict".to_string(),
        api_key_env: None,
        local_model: Some(LocalModelConfig {
            artifact_path: missing_artifact,
            sha256: original.sha256,
            runtime_binding: RuntimeBinding::Candle,
            display_name: "mt014-audit-preflight-conflict".to_string(),
            embedding_dimension: None,
        }),
        local_embedding_model: None,
    };
    let (ledger, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 4,
            batch_size: 2,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("construct lifecycle ledger for pre-artifact zero-row proof");
    let explicit_scope = format!("mt014-audit-preflight-{}", uuid::Uuid::now_v7());
    let host_scope =
        resolve_embedded_runtime_host_scope_with_override(&pg.schema_url, Some(&explicit_scope))
            .expect("resolve explicit test host scope");
    let lease = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), host_scope)
        .expect("acquire runtime lease for configured boot proof");
    let client = build_default_local_client(
        &resolved,
        Arc::new(NoopRecorder),
        Some(ledger),
        Some(ModelRegistryStore::new(pool.clone())),
        Some(lease.descriptor().clone()),
    )
    .await;
    let error = client
        .completion(CompletionRequest::new(
            uuid::Uuid::now_v7(),
            "must fail on preserved audit before artifact access".to_string(),
            "mt014-audit-preflight-conflict".to_string(),
        ))
        .await
        .expect_err("preserved revision-two selection must reject configured revision-one adapter");
    let reason = error.to_string();
    assert!(
        reason.contains("persistent model registry preflight failed before artifact access")
            && reason.contains("audit-preserved immutable selection"),
        "failure must come from the preserved audit preflight: {reason}"
    );
    assert!(
        !reason.contains("embedded ModelRuntime load failed")
            && !reason.contains("No such file")
            && !reason.contains("os error"),
        "artifact loader must not be reached: {reason}"
    );

    drain
        .drain_available_to(Arc::new(PostgresProcessLedgerStore::new(pool.clone())))
        .await
        .expect("drain lifecycle ledger for zero-row proof");
    let process_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY kernel_process_lifecycle")
            .fetch_one(&pool)
            .await
            .expect("count lifecycle rows after rejected audit preflight");
    let projection_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count recreated registry projection after rejected audit preflight");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count preserved registry audit events");
    assert_eq!(process_count, 0, "preflight conflict emits no START row");
    assert_eq!(
        projection_count, 0,
        "preflight does not rebuild before load"
    );
    assert_eq!(audit_count, 2, "preflight does not alter preserved audit");
}

#[tokio::test]
async fn mt014_authority_rejects_deferred_user_trigger_before_mutation() {
    let pg = pg_required("mt014 committed observation readback").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let trigger_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION "{schema}".mt014_rewrite_registry_observation()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            UPDATE ONLY "{schema}".model_runtime_registry
            SET last_observed_runtime_model_id = '00000000-0000-0000-0000-000000000014'::uuid,
                base_model_tag = 'trigger-rewritten-model',
                last_observed_by = 'trigger-rewritten-observer'
            WHERE registry_row_id = NEW.registry_row_id;
            RETURN NULL;
        END
        $$;

        CREATE CONSTRAINT TRIGGER mt014_rewrite_registry_observation
        AFTER INSERT ON "{schema}".model_runtime_registry
        DEFERRABLE INITIALLY DEFERRED
        FOR EACH ROW
        EXECUTE FUNCTION "{schema}".mt014_rewrite_registry_observation();
        "#,
        schema = pg.schema,
    );
    sqlx::raw_sql(&trigger_sql)
        .execute(&pool)
        .await
        .expect("install hostile deferred committed-observation rewrite trigger");

    let attempted = registration(
        0x13,
        RuntimeBinding::LlamaCpp,
        "expected-observation",
        "mt014-expected-observer",
        "models/expected-observation.gguf",
    );
    let error = ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&attempted)
        .await
        .expect_err("enabled user trigger must fail authority preflight before mutation");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error.to_string().contains("no enabled user trigger"),
        "unexpected hook-free authority error: {error}"
    );

    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after hook rejection");
    assert_eq!(registry_count, 0);
    let aggregate_id = format!("sha256:{}", hex::encode(attempted.sha256));
    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.count(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_runtime_registry'
          AND aggregate_id = $1
          AND source_component = 'model_runtime_registry'
        "#,
    )
    .bind(aggregate_id)
    .fetch_one(&pool)
    .await
    .expect("count audit rows after observation mismatch rollback");
    assert_eq!(
        audit_count, 0,
        "registry row and audit event must roll back atomically"
    );
}

#[tokio::test]
async fn mt014_authority_rejects_event_ledger_user_trigger_before_mutation() {
    let pg = pg_required("mt014 EventLedger hook-free authority").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let trigger_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION "{schema}".mt014_intercept_registry_audit()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            RETURN NEW;
        END
        $$;

        CREATE TRIGGER mt014_intercept_registry_audit
        BEFORE INSERT ON "{schema}".kernel_event_ledger
        FOR EACH ROW
        EXECUTE FUNCTION "{schema}".mt014_intercept_registry_audit();
        "#,
        schema = pg.schema,
    );
    sqlx::raw_sql(&trigger_sql)
        .execute(&pool)
        .await
        .expect("install hostile EventLedger trigger");

    let attempted = registration(
        0x14,
        RuntimeBinding::Candle,
        "event-ledger-hook-probe",
        "mt014-event-ledger-hook-probe",
        "models/event-ledger-hook-probe.safetensors",
    );
    let error = ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&attempted)
        .await
        .expect_err("enabled EventLedger trigger must fail authority preflight before mutation");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error.to_string().contains("no enabled user trigger"),
        "unexpected EventLedger hook-free authority error: {error}"
    );

    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after EventLedger hook rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry audit rows after EventLedger hook rejection");
    assert_eq!(
        registry_count, 0,
        "hook rejection must precede projection mutation"
    );
    assert_eq!(audit_count, 0, "hook rejection must precede audit mutation");
}

#[tokio::test]
async fn mt014_authority_rejects_event_ledger_check_search_path_hook_before_mutation() {
    let pg = pg_required("mt014 EventLedger CHECK search-path authority attack").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let attacker_schema = format!("mt014_attacker_{}", uuid::Uuid::now_v7().simple());
    let hostile_sql = format!(
        r#"
        CREATE SCHEMA "{attacker_schema}";
        CREATE TABLE "{attacker_schema}".model_runtime_registry
            (LIKE "{authority_schema}".model_runtime_registry INCLUDING ALL);

        CREATE OR REPLACE FUNCTION "{authority_schema}".mt014_poison_registry_search_path()
        RETURNS boolean
        LANGUAGE plpgsql
        AS $$
        BEGIN
            PERFORM pg_catalog.set_config(
                'search_path',
                'pg_catalog, "{attacker_schema}", "{authority_schema}", pg_temp',
                true
            );
            RETURN TRUE;
        END
        $$;

        ALTER TABLE "{authority_schema}".kernel_event_ledger
        ADD CONSTRAINT mt014_hostile_event_ledger_search_path_check
        CHECK ("{authority_schema}".mt014_poison_registry_search_path());
        "#,
        authority_schema = pg.schema,
    );
    sqlx::raw_sql(&hostile_sql)
        .execute(&pool)
        .await
        .expect("install hostile EventLedger CHECK search-path hook and shadow projection");

    let attempted = registration(
        0x1a,
        RuntimeBinding::Candle,
        "event-ledger-check-hook",
        "mt014-event-ledger-check-hook",
        "models/event-ledger-check-hook.safetensors",
    );
    let error = ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&attempted)
        .await
        .expect_err("extra EventLedger CHECK must fail exact authority preflight");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("no behavior-bearing extra constraints"),
        "unexpected EventLedger constraint-authority error: {error}"
    );

    let canonical_registry_count: i64 = sqlx::query_scalar(&format!(
        "SELECT pg_catalog.count(*) FROM ONLY \"{}\".model_runtime_registry",
        pg.schema
    ))
    .fetch_one(&pool)
    .await
    .expect("count canonical registry after rejected CHECK hook");
    let canonical_audit_count: i64 = sqlx::query_scalar(&format!(
        "SELECT pg_catalog.count(*) FROM ONLY \"{}\".kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
        pg.schema
    ))
    .fetch_one(&pool)
    .await
    .expect("count canonical audit after rejected CHECK hook");
    let shadow_registry_count: i64 = sqlx::query_scalar(&format!(
        "SELECT pg_catalog.count(*) FROM ONLY \"{attacker_schema}\".model_runtime_registry"
    ))
    .fetch_one(&pool)
    .await
    .expect("count attacker shadow registry after rejected CHECK hook");
    assert_eq!(canonical_registry_count, 0);
    assert_eq!(canonical_audit_count, 0);
    assert_eq!(shadow_registry_count, 0);
}

#[tokio::test]
async fn mt014_authority_rejects_wrong_event_ledger_aggregate_replay_index() {
    let pg = pg_required("mt014 exact EventLedger aggregate replay index").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    sqlx::raw_sql(
        r#"
        DROP INDEX idx_kernel_event_ledger_aggregate_replay;
        CREATE INDEX idx_kernel_event_ledger_aggregate_replay
            ON kernel_event_ledger (
                aggregate_type ASC,
                aggregate_id DESC,
                event_sequence ASC
            );
        "#,
    )
    .execute(&pool)
    .await
    .expect("replace EventLedger replay index with same-name mixed key direction");
    let error = ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect_err("mixed-direction EventLedger replay index must fail authority preflight");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("aggregate_type, aggregate_id, event_sequence"),
        "unexpected EventLedger replay-index authority error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after replay-index rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry audit rows after replay-index rejection");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn mt014_authority_rejects_drifted_event_ledger_sequence_parameters() {
    let pg = pg_required("mt014 exact EventLedger sequence parameters").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    sqlx::query("ALTER SEQUENCE kernel_event_ledger_event_sequence_seq CACHE 32")
        .execute(&pool)
        .await
        .expect("drift EventLedger sequence cache without changing its identity");
    let error = ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect_err("drifted EventLedger sequence parameters must fail authority preflight");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("exact owned permanent bigint sequence"),
        "unexpected EventLedger sequence authority error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after sequence-parameter rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry audit rows after sequence-parameter rejection");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn mt014_authority_rejects_exhausted_event_ledger_sequence_live_state() {
    let pg = pg_required("mt014 EventLedger sequence live-state operability").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");

    sqlx::query(
        "SELECT pg_catalog.setval('kernel_event_ledger_event_sequence_seq'::pg_catalog.regclass, $1, false)",
    )
    .bind(i64::MAX)
    .execute(&pool)
    .await
    .expect("place EventLedger sequence at bigint max with its final value still callable");
    ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect("max with is_called=false still has one operable sequence value");

    sqlx::query(
        "SELECT pg_catalog.setval('kernel_event_ledger_event_sequence_seq'::pg_catalog.regclass, $1, true)",
    )
    .bind(i64::MAX)
    .execute(&pool)
    .await
    .expect("exhaust the EventLedger sequence without changing catalog parameters");
    let error = ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect_err(
            "catalog-valid but exhausted EventLedger sequence must fail authority preflight",
        );
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error.to_string().contains("is exhausted at bigint max"),
        "unexpected EventLedger sequence exhaustion error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after exhausted-sequence rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry audit rows after exhausted-sequence rejection");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);
}

#[tokio::test]
async fn mt014_authority_rejects_sequence_select_without_nextval_privilege() {
    let pg = pg_required("mt014 EventLedger sequence nextval privilege").await;
    let role = format!(
        "mt014_sequence_select_only_{}",
        uuid::Uuid::now_v7().simple()
    );
    let setup = format!(
        r#"
        CREATE ROLE {role} NOLOGIN;
        GRANT {role} TO CURRENT_USER;
        GRANT USAGE ON SCHEMA {schema} TO {role};
        GRANT SELECT ON TABLE {schema}.model_runtime_registry, {schema}.kernel_event_ledger TO {role};
        GRANT SELECT ON SEQUENCE {schema}.kernel_event_ledger_event_sequence_seq TO {role};
        "#,
        schema = pg.schema,
    );
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&pg.schema_url)
        .await
        .expect("connect max-one sequence ACL proof pool");
    sqlx::raw_sql(&setup)
        .execute(&pool)
        .await
        .expect("create SELECT-only EventLedger sequence role");
    sqlx::query(&format!("SET ROLE {role}"))
        .execute(&pool)
        .await
        .expect("enter SELECT-only EventLedger sequence role");
    let active_role: String = sqlx::query_scalar("SELECT CURRENT_USER::pg_catalog.text")
        .fetch_one(&pool)
        .await
        .expect("read active sequence ACL proof role");
    assert_eq!(active_role, role);

    let error = ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect_err("SELECT without USAGE or UPDATE must not satisfy nextval operability");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("lacks USAGE or UPDATE privilege"),
        "unexpected EventLedger sequence ACL error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after sequence ACL rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count audit rows after sequence ACL rejection");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);

    let cleanup = format!(
        r#"
        DROP OWNED BY {role};
        REVOKE {role} FROM CURRENT_USER;
        DROP ROLE {role};
        "#
    );
    sqlx::raw_sql(&cleanup)
        .execute(&pool)
        .await
        .expect("remove SELECT-only EventLedger sequence role");
}

#[tokio::test]
async fn mt014_projection_timestamps_must_equal_referenced_event_timestamps() {
    let pg = pg_required("mt014 projection timestamp audit binding").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let updated_tamper = registration(
        0x15,
        RuntimeBinding::Candle,
        "updated-time-tamper",
        "mt014-time-auditor",
        "models/updated-time-tamper.safetensors",
    );
    let created_tamper = registration(
        0x16,
        RuntimeBinding::Candle,
        "created-time-tamper",
        "mt014-time-auditor",
        "models/created-time-tamper.safetensors",
    );
    store
        .persist_boot_set_and_read_back(&[updated_tamper.clone(), created_tamper.clone()])
        .await
        .expect("persist two timestamp-tamper authorities");

    sqlx::query(
        "UPDATE ONLY model_runtime_registry SET selection_updated_at_utc = selection_updated_at_utc + INTERVAL '1 second' WHERE artifact_sha256 = $1",
    )
    .bind(updated_tamper.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("tamper only the projection updated timestamp");
    let updated_error = store
        .load_by_artifact_sha256(&updated_tamper.sha256)
        .await
        .expect_err("projection updated timestamp must bind to the latest audit event");
    assert!(
        updated_error.to_string().contains("updated timestamp")
            && updated_error.to_string().contains("EventLedger timestamp"),
        "unexpected updated-timestamp audit error: {updated_error}"
    );

    sqlx::query(
        "UPDATE ONLY model_runtime_registry SET selection_created_at_utc = selection_created_at_utc - INTERVAL '1 second' WHERE artifact_sha256 = $1",
    )
    .bind(created_tamper.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("tamper only the projection created timestamp");
    let created_error = store
        .load_by_artifact_sha256(&created_tamper.sha256)
        .await
        .expect_err("projection created timestamp must bind to the initial audit event");
    assert!(
        created_error.to_string().contains("created timestamp")
            && created_error.to_string().contains("EventLedger timestamp"),
        "unexpected created-timestamp audit error: {created_error}"
    );
}

#[tokio::test]
async fn mt014_oversized_event_ledger_chain_fails_closed_without_mutation() {
    let pg = pg_required("mt014 bounded EventLedger chain validation").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let attempted = registration(
        0x17,
        RuntimeBinding::Candle,
        "oversized-audit-chain",
        "mt014-bounded-auditor",
        "models/oversized-audit-chain.safetensors",
    );
    let persisted = store
        .persist_and_read_back(&attempted)
        .await
        .expect("persist initial bounded-chain authority");

    sqlx::query(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload,
            created_at
        )
        SELECT pg_catalog.concat('mt014-overflow-event-', $1, '-', series.value),
               source.event_version,
               source.kernel_task_run_id,
               source.session_run_id,
               source.aggregate_type,
               source.aggregate_id,
               pg_catalog.concat('mt014-overflow-idempotency-', $1, '-', series.value),
               source.event_type,
               source.actor_kind,
               source.actor_id,
               source.event_id,
               source.correlation_id,
               source.payload_hash,
               source.source_component,
               source.payload,
               source.created_at
        FROM ONLY kernel_event_ledger AS source
        CROSS JOIN pg_catalog.generate_series(1, 4096) AS series(value)
        WHERE source.event_id = $2
        "#,
    )
    .bind(hex::encode(attempted.sha256))
    .bind(&persisted.selection_created_event_id)
    .execute(&pool)
    .await
    .expect("append an actual over-cap audit suffix");

    let error = store
        .load_by_artifact_sha256(&attempted.sha256)
        .await
        .expect_err("over-cap EventLedger chain must fail closed");
    assert!(
        error.to_string().contains("bounded 4096-event limit"),
        "unexpected bounded-chain error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count unchanged registry projection after oversized read rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(&persisted.artifact_locator)
    .fetch_one(&pool)
    .await
    .expect("count preserved oversized audit chain");
    assert_eq!(
        registry_count, 1,
        "read rejection must not mutate projection"
    );
    assert_eq!(audit_count, 4097, "read rejection must not truncate audit");
}

#[tokio::test]
async fn mt014_statement_timeout_is_bounded_and_typed() {
    let pg = pg_required("mt014 typed bounded statement timeout").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let started = Instant::now();
    let error = store
        .prove_statement_timeout_for_tests()
        .await
        .expect_err("authority statement beyond the production deadline must be canceled");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("bounded 2000 ms authority deadline"),
        "unexpected typed statement-timeout error: {error}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "statement timeout must bound the deliberately longer database call"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after timeout rollback");
    assert_eq!(registry_count, 0, "timeout proof must not mutate authority");
}

#[tokio::test]
async fn mt014_transaction_start_is_bounded_before_a_connection_is_available() {
    let pg = pg_required("mt014 bounded transaction start").await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect(&pg.schema_url)
        .await
        .expect("connect one-connection PostgreSQL registry authority");
    let held = pool
        .acquire()
        .await
        .expect("hold the sole registry pool connection");
    let store = ModelRegistryStore::new(pool.clone());
    let started = Instant::now();
    let error = store
        .ensure_authority_available()
        .await
        .expect_err("transaction start must not inherit the pool's longer acquire timeout");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("transaction start exceeded the bounded 2000ms deadline"),
        "unexpected bounded transaction-start error: {error}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "transaction start must fail near the registry's two-second deadline"
    );
    drop(held);
    store
        .ensure_authority_available()
        .await
        .expect("timed-out initialization must leave the authority store recoverable");
}

#[tokio::test]
async fn mt014_cancelled_transaction_start_closes_backend_instead_of_repooling() {
    let pg = pg_required("mt014 cancellation-safe transaction initialization").await;
    let application_name = format!("mt014-cancel-start-{}", uuid::Uuid::now_v7().simple());
    let options = PgConnectOptions::from_str(&pg.schema_url)
        .expect("parse isolated PostgreSQL registry URL")
        .application_name(&application_name);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect_with(options)
        .await
        .expect("connect cancellation-proof one-connection registry pool");
    let mut control = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect independent transaction-start control session");
    let advisory_key = unique_advisory_key();
    sqlx::query("SELECT pg_catalog.pg_advisory_lock($1)")
        .bind(advisory_key)
        .execute(&mut control)
        .await
        .expect("hold transaction-start gate from the control session");

    let store = ModelRegistryStore::new(pool.clone());
    let probe_store = store.clone();
    let probe = tokio::spawn(async move {
        probe_store
            .prove_cancel_safe_transaction_start_for_tests(advisory_key)
            .await
    });
    let cancelled_backend_pid =
        wait_for_transaction_start_gate(&mut control, &application_name).await;
    probe.abort();
    let cancellation = probe
        .await
        .expect_err("aborted transaction-start future must report cancellation");
    assert!(cancellation.is_cancelled());
    wait_for_backend_exit(&mut control, cancelled_backend_pid).await;

    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&mut control)
            .await
            .expect("count registry rows after transaction-start cancellation");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&mut control)
    .await
    .expect("count audit rows after transaction-start cancellation");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);
    let unlocked: bool = sqlx::query_scalar("SELECT pg_catalog.pg_advisory_unlock($1)")
        .bind(advisory_key)
        .fetch_one(&mut control)
        .await
        .expect("release controlled transaction-start gate");
    assert!(unlocked);

    store
        .ensure_authority_available()
        .await
        .expect("same max-one pool must recover after cancellation closes its backend");
    let replacement_backend_pid: i32 = sqlx::query_scalar("SELECT pg_catalog.pg_backend_pid()")
        .fetch_one(&pool)
        .await
        .expect("read replacement backend identity");
    assert_ne!(replacement_backend_pid, cancelled_backend_pid);
    store
        .persist_and_read_back(&registration(
            0x2a,
            RuntimeBinding::Candle,
            "cancel-safe-start",
            "mt014-cancel-safe-start",
            "models/cancel-safe-start.safetensors",
        ))
        .await
        .expect("replacement backend must complete a real registry and audit commit");
}

#[tokio::test]
async fn mt014_transaction_initialization_timeout_closes_blocked_backend() {
    let pg = pg_required("mt014 bounded first transaction server round trip").await;
    let application_name = format!("mt014-timeout-start-{}", uuid::Uuid::now_v7().simple());
    let options = PgConnectOptions::from_str(&pg.schema_url)
        .expect("parse isolated PostgreSQL registry URL")
        .application_name(&application_name);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect_with(options)
        .await
        .expect("connect timeout-proof one-connection registry pool");
    let mut control = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect independent transaction-timeout control session");
    let advisory_key = unique_advisory_key();
    sqlx::query("SELECT pg_catalog.pg_advisory_lock($1)")
        .bind(advisory_key)
        .execute(&mut control)
        .await
        .expect("hold transaction-start timeout gate");

    let store = ModelRegistryStore::new(pool.clone());
    let probe_store = store.clone();
    let started = Instant::now();
    let probe = tokio::spawn(async move {
        probe_store
            .prove_cancel_safe_transaction_start_for_tests(advisory_key)
            .await
    });
    let timed_out_backend_pid =
        wait_for_transaction_start_gate(&mut control, &application_name).await;
    let error = probe
        .await
        .expect("transaction-start timeout probe task must complete")
        .expect_err("blocked first server round trip must hit the client deadline");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("transaction initialization exceeded the bounded 2000ms deadline"),
        "unexpected transaction-initialization timeout: {error}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    wait_for_backend_exit(&mut control, timed_out_backend_pid).await;
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&mut control)
            .await
            .expect("count registry rows after transaction-start timeout");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&mut control)
    .await
    .expect("count audit rows after transaction-start timeout");
    assert_eq!(registry_count, 0);
    assert_eq!(audit_count, 0);
    let unlocked: bool = sqlx::query_scalar("SELECT pg_catalog.pg_advisory_unlock($1)")
        .bind(advisory_key)
        .fetch_one(&mut control)
        .await
        .expect("release transaction-start timeout gate");
    assert!(unlocked);

    store
        .ensure_authority_available()
        .await
        .expect("timeout-closed max-one pool must recover with a clean backend");
    let replacement_backend_pid: i32 = sqlx::query_scalar("SELECT pg_catalog.pg_backend_pid()")
        .fetch_one(&pool)
        .await
        .expect("read post-timeout backend identity");
    assert_ne!(replacement_backend_pid, timed_out_backend_pid);
}

#[tokio::test]
async fn mt014_transaction_start_opens_a_connection_from_a_zero_idle_pool() {
    let pg = pg_required("mt014 zero-idle transaction start").await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .connect_lazy(&pg.schema_url)
        .expect("create a zero-idle lazy PostgreSQL registry pool");
    assert_eq!(
        pool.size(),
        0,
        "lazy proof pool must start with no idle connection"
    );
    ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect("bounded transaction start must open a new connection when capacity exists");
    assert_eq!(
        pool.size(),
        1,
        "successful authority proof must establish the first pooled connection"
    );
}

#[tokio::test]
async fn mt014_non_utc_session_persists_actual_utc_event_and_projection_timestamps() {
    let pg = pg_required("mt014 transaction-local UTC timestamp authority").await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&pg.schema_url)
        .await
        .expect("connect one-connection PostgreSQL registry authority");
    sqlx::query("SET TIME ZONE 'Pacific/Honolulu'")
        .execute(&pool)
        .await
        .expect("install hostile non-UTC session TimeZone");
    let before = Utc::now() - chrono::Duration::seconds(1);
    let persisted = ModelRegistryStore::new(pool.clone())
        .persist_and_read_back(&registration(
            0x29,
            RuntimeBinding::Candle,
            "utc-timestamp-model",
            "mt014-utc-timestamp",
            "models/utc-timestamp.safetensors",
        ))
        .await
        .expect("registry transaction must pin UTC before EventLedger append");
    let after = Utc::now() + chrono::Duration::seconds(1);
    let event_created_at: chrono::DateTime<Utc> = sqlx::query_scalar(
        "SELECT created_at AT TIME ZONE 'UTC' FROM ONLY kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&persisted.selection_created_event_id)
    .fetch_one(&pool)
    .await
    .expect("read canonical EventLedger UTC timestamp");
    assert_eq!(
        event_created_at, persisted.selection_created_at_utc,
        "projection and EventLedger must retain the same UTC instant"
    );
    assert!(
        event_created_at >= before && event_created_at <= after,
        "stored UTC instant {event_created_at} must match wall-clock UTC window {before}..={after}"
    );
    let session_time_zone: String = sqlx::query_scalar("SHOW TimeZone")
        .fetch_one(&pool)
        .await
        .expect("read session TimeZone after registry transaction");
    assert_eq!(
        session_time_zone, "Pacific/Honolulu",
        "registry UTC pin must be transaction-local"
    );
}

#[tokio::test]
async fn mt014_registry_row_enumeration_cap_rejects_4097_rows_before_decode() {
    let pg = pg_required("mt014 registry row enumeration cap").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let seed = registration(
        0x2a,
        RuntimeBinding::Candle,
        "row-cap-seed",
        "mt014-row-cap",
        "models/row-cap-seed.safetensors",
    );
    store
        .persist_and_read_back(&seed)
        .await
        .expect("persist seed row and canonical EventLedger refs");
    sqlx::query(
        r#"
        WITH seed AS (
            SELECT *
            FROM ONLY model_runtime_registry
            WHERE artifact_sha256 = $1
        ),
        generated AS (
            SELECT series.i,
                   pg_catalog.decode(
                       pg_catalog.lpad(pg_catalog.to_hex(series.i), 64, '0'),
                       'hex'
                   ) AS artifact_sha256
            FROM pg_catalog.generate_series(2, 4097) AS series(i)
        )
        INSERT INTO model_runtime_registry (
            schema_id,
            registry_row_id,
            artifact_sha256,
            artifact_locator,
            last_observed_runtime_model_id,
            runtime_binding,
            capabilities_schema_id,
            capabilities_json,
            provider,
            base_model_tag,
            last_observed_by,
            selection_revision,
            selection_created_event_id,
            selection_updated_event_id,
            selection_created_at_utc,
            selection_updated_at_utc,
            last_observed_at_utc
        )
        SELECT seed.schema_id,
               pg_catalog.format(
                   '00000000-0000-7000-8000-%s',
                   pg_catalog.lpad(pg_catalog.to_hex(generated.i), 12, '0')
               )::pg_catalog.uuid,
               generated.artifact_sha256,
               'sha256:' || pg_catalog.encode(generated.artifact_sha256, 'hex'),
               pg_catalog.format(
                   '00000000-0000-7001-8000-%s',
                   pg_catalog.lpad(pg_catalog.to_hex(generated.i), 12, '0')
               )::pg_catalog.uuid,
               seed.runtime_binding,
               seed.capabilities_schema_id,
               seed.capabilities_json,
               seed.provider,
               'row-cap-' || generated.i::pg_catalog.text,
               seed.last_observed_by,
               seed.selection_revision,
               seed.selection_created_event_id,
               seed.selection_updated_event_id,
               seed.selection_created_at_utc,
               seed.selection_updated_at_utc,
               seed.last_observed_at_utc
        FROM seed
        CROSS JOIN generated
        "#,
    )
    .bind(seed.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("insert 4096 additional constraint-valid projection rows");
    let error = store
        .list_recoverable()
        .await
        .expect_err("4097 projection rows must hit the row branch before decode/audit");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error.to_string().contains("bounded 4096-row limit"),
        "unexpected row-cap error: {error}"
    );
    let row_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count projection rows after bounded rejection");
    assert_eq!(row_count, 4097, "bounded read must not mutate projection");
}

#[tokio::test]
async fn mt014_oversized_registry_json_and_event_payloads_reject_before_decode() {
    let pg = pg_required("mt014 registry and EventLedger byte budgets").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let model = registration(
        0x2b,
        RuntimeBinding::Candle,
        "byte-budget-model",
        "mt014-byte-budget",
        "models/byte-budget.safetensors",
    );
    let persisted = store
        .persist_and_read_back(&model)
        .await
        .expect("persist canonical row before byte-bound tampering");
    sqlx::query(
        "UPDATE ONLY model_runtime_registry SET capabilities_json = pg_catalog.jsonb_build_object('oversized', pg_catalog.repeat('x', 70000)) WHERE artifact_sha256 = $1",
    )
    .bind(model.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("install oversized but object-shaped capabilities JSON");
    let registry_error = store
        .load_by_artifact_sha256(&model.sha256)
        .await
        .expect_err("oversized capabilities JSON must reject before serde decode");
    assert!(
        matches!(registry_error, ModelRegistryPersistenceError::CorruptRow(_))
            && registry_error
                .to_string()
                .contains("bounded 65536-byte decode limit"),
        "unexpected registry byte-budget error: {registry_error}"
    );
    sqlx::query(
        "UPDATE ONLY model_runtime_registry SET capabilities_json = $2 WHERE artifact_sha256 = $1",
    )
    .bind(model.sha256.as_slice())
    .bind(
        serde_json::to_value(capabilities(RuntimeBinding::Candle))
            .expect("serialize canonical capabilities"),
    )
    .execute(&pool)
    .await
    .expect("restore canonical capabilities JSON");
    sqlx::query(
        "UPDATE ONLY kernel_event_ledger SET payload = pg_catalog.jsonb_build_object('oversized', pg_catalog.repeat('x', 1048577)) WHERE event_id = $1",
    )
    .bind(&persisted.selection_created_event_id)
    .execute(&pool)
    .await
    .expect("install oversized EventLedger payload");
    let event_error = store
        .load_by_artifact_sha256(&model.sha256)
        .await
        .expect_err("oversized EventLedger payload must reject before JSON transfer/decode");
    assert!(
        matches!(event_error, ModelRegistryPersistenceError::Audit(_))
            && event_error
                .to_string()
                .contains("bounded 1048576-byte decode limit"),
        "unexpected EventLedger byte-budget error: {event_error}"
    );
    let row_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after byte-bound rejections");
    assert_eq!(row_count, 1, "byte-bound reads must not mutate projection");
}

#[tokio::test]
async fn mt014_aggregate_registry_and_event_transfer_byte_caps_are_enforced() {
    let pg = pg_required("mt014 aggregate registry and EventLedger byte budgets").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let seed = registration(
        0x2c,
        RuntimeBinding::Candle,
        "aggregate-byte-seed",
        "mt014-aggregate-byte-budget",
        "models/aggregate-byte-seed.safetensors",
    );
    let persisted = store
        .persist_and_read_back(&seed)
        .await
        .expect("persist canonical seed before aggregate byte-bound probes");
    sqlx::query(
        r#"
        WITH seed AS (
            SELECT *
            FROM ONLY model_runtime_registry
            WHERE artifact_sha256 = $1
        ),
        generated AS (
            SELECT series.i,
                   pg_catalog.decode(
                       pg_catalog.lpad(pg_catalog.to_hex(series.i + 10000), 64, '0'),
                       'hex'
                   ) AS artifact_sha256
            FROM pg_catalog.generate_series(2, 20) AS series(i)
        )
        INSERT INTO model_runtime_registry (
            schema_id,
            registry_row_id,
            artifact_sha256,
            artifact_locator,
            last_observed_runtime_model_id,
            runtime_binding,
            capabilities_schema_id,
            capabilities_json,
            provider,
            base_model_tag,
            last_observed_by,
            selection_revision,
            selection_created_event_id,
            selection_updated_event_id,
            selection_created_at_utc,
            selection_updated_at_utc,
            last_observed_at_utc
        )
        SELECT seed.schema_id,
               pg_catalog.format(
                   '00000000-0000-7002-8000-%s',
                   pg_catalog.lpad(pg_catalog.to_hex(generated.i), 12, '0')
               )::pg_catalog.uuid,
               generated.artifact_sha256,
               'sha256:' || pg_catalog.encode(generated.artifact_sha256, 'hex'),
               pg_catalog.format(
                   '00000000-0000-7003-8000-%s',
                   pg_catalog.lpad(pg_catalog.to_hex(generated.i), 12, '0')
               )::pg_catalog.uuid,
               seed.runtime_binding,
               seed.capabilities_schema_id,
               seed.capabilities_json,
               seed.provider,
               pg_catalog.repeat('r', 900000),
               seed.last_observed_by,
               seed.selection_revision,
               seed.selection_created_event_id,
               seed.selection_updated_event_id,
               seed.selection_created_at_utc,
               seed.selection_updated_at_utc,
               seed.last_observed_at_utc
        FROM seed
        CROSS JOIN generated
        "#,
    )
    .bind(seed.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("insert moderate individual registry rows whose aggregate exceeds 16 MiB");
    let registry_error = store
        .list_recoverable()
        .await
        .expect_err("aggregate registry transfer budget must reject before row decode");
    assert!(
        matches!(
            registry_error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && registry_error
            .to_string()
            .contains("bounded 16777216-byte transfer limit"),
        "unexpected aggregate registry byte-budget error: {registry_error}"
    );

    sqlx::query(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload
        )
        SELECT 'MT014-AGGREGATE-BYTE-' || series.i::pg_catalog.text,
               'kernel_event_v1',
               'KTR-MT014-AGGREGATE-BYTE',
               'SESSION-MT014-AGGREGATE-BYTE',
               'model_runtime_registry',
               $1,
               'mt014-aggregate-byte-' || series.i::pg_catalog.text,
               'MODEL_RUNTIME_SELECTED',
               'system',
               'mt014-aggregate-byte-budget',
               NULL,
               NULL,
               'preflight-rejects-before-hash-validation',
               'model_runtime_registry',
               pg_catalog.jsonb_build_object(
                   'moderate_individual_payload',
                   pg_catalog.repeat('e', 900000)
               )
        FROM pg_catalog.generate_series(2, 20) AS series(i)
        "#,
    )
    .bind(&persisted.artifact_locator)
    .execute(&pool)
    .await
    .expect("insert moderate individual audit rows whose aggregate exceeds 16 MiB");
    let event_error = store
        .load_by_artifact_sha256(&seed.sha256)
        .await
        .expect_err("aggregate EventLedger transfer budget must reject before event decode");
    assert!(
        matches!(event_error, ModelRegistryPersistenceError::Audit(_))
            && event_error
                .to_string()
                .contains("bounded 16777216-byte transfer limit"),
        "unexpected aggregate EventLedger byte-budget error: {event_error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after aggregate byte-bound rejections");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(&persisted.artifact_locator)
    .fetch_one(&pool)
    .await
    .expect("count audit rows after aggregate byte-bound rejection");
    assert_eq!(registry_count, 20);
    assert_eq!(audit_count, 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn mt014_event_payload_fetch_is_pinned_to_byte_preflight_sequences() {
    let pg = pg_required("mt014 EventLedger byte-preflight insert interleaving").await;
    let application_name = format!("mt014_event_bytes_{}", uuid::Uuid::now_v7().simple());
    let separator = if pg.schema_url.contains('?') {
        "&"
    } else {
        "?"
    };
    let store_url = format!(
        "{}{separator}application_name={application_name}",
        pg.schema_url
    );
    let store_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&store_url)
        .await
        .expect("connect one-session bounded EventLedger store pool");
    let control_pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect independent EventLedger interleaving control pool");
    let store = ModelRegistryStore::new(store_pool);
    let model = registration(
        0x2d,
        RuntimeBinding::Candle,
        "event-sequence-pin",
        "mt014-event-sequence-pin",
        "models/event-sequence-pin.safetensors",
    );
    let persisted = store
        .persist_and_read_back(&model)
        .await
        .expect("persist canonical row before EventLedger interleaving");

    let mut blocker = control_pool
        .begin()
        .await
        .expect("begin controlled EventLedger row-lock blocker");
    sqlx::query("SELECT event_id FROM ONLY kernel_event_ledger WHERE event_id = $1 FOR UPDATE")
        .bind(&persisted.selection_created_event_id)
        .fetch_one(&mut *blocker)
        .await
        .expect("hold existing audit row while byte preflight takes its statement snapshot");
    let bounded_load_task = {
        let store = store.clone();
        let aggregate_id = persisted.artifact_locator.clone();
        tokio::spawn(async move {
            store
                .prove_event_byte_preflight_sequence_pin_for_tests(&aggregate_id)
                .await
        })
    };
    wait_for_registry_lock_wait(&control_pool, &application_name).await;

    sqlx::query(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload
        )
        VALUES (
            'MT014-INTERLEAVED-OVERSIZED-EVENT',
            'kernel_event_v1',
            'KTR-MT014-INTERLEAVED-OVERSIZED-EVENT',
            'SESSION-MT014-INTERLEAVED-OVERSIZED-EVENT',
            'model_runtime_registry',
            $1,
            'mt014-interleaved-oversized-event',
            'MODEL_RUNTIME_SELECTED',
            'system',
            'mt014-event-sequence-pin',
            NULL,
            NULL,
            'preflight-sequence-pin-must-exclude-this-row',
            'model_runtime_registry',
            pg_catalog.jsonb_build_object(
                'interleaved_oversized_payload',
                pg_catalog.repeat('x', 1048577)
            )
        )
        "#,
    )
    .bind(&persisted.artifact_locator)
    .execute(&control_pool)
    .await
    .expect("commit oversized same-aggregate event after preflight snapshot");
    blocker
        .rollback()
        .await
        .expect("release existing audit row so bounded preflight can finish");

    let bounded_count = bounded_load_task
        .await
        .expect("interleaved bounded-load task joins")
        .expect(
            "payload fetch must use only the fixed-size sequences returned by its earlier preflight",
        );
    assert_eq!(
        bounded_count, 1,
        "one invocation must fetch only its preflighted original event sequence"
    );
    let next_read_error = store
        .load_by_artifact_sha256(&model.sha256)
        .await
        .expect_err("the next snapshot must byte-check and reject the oversized event");
    assert!(
        matches!(next_read_error, ModelRegistryPersistenceError::Audit(_))
            && next_read_error
                .to_string()
                .contains("bounded 1048576-byte decode limit"),
        "unexpected post-interleaving byte-budget error: {next_read_error}"
    );
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(&persisted.artifact_locator)
    .fetch_one(&control_pool)
    .await
    .expect("count original plus interleaved audit rows");
    assert_eq!(audit_count, 2);
}

#[tokio::test]
async fn mt014_enumeration_rejects_over_budget_revision_total_without_mutation() {
    let pg = pg_required("mt014 bounded enumeration audit total").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect isolated PostgreSQL registry authority");
    let store = ModelRegistryStore::new(pool.clone());
    let first = registration(
        0x18,
        RuntimeBinding::Candle,
        "enumeration-budget-first",
        "mt014-enumeration-budget",
        "models/enumeration-budget-first.safetensors",
    );
    let second = registration(
        0x19,
        RuntimeBinding::Candle,
        "enumeration-budget-second",
        "mt014-enumeration-budget",
        "models/enumeration-budget-second.safetensors",
    );
    store
        .persist_boot_set_and_read_back(&[first.clone(), second.clone()])
        .await
        .expect("persist enumeration budget rows");
    sqlx::query(
        r#"
        UPDATE ONLY model_runtime_registry
        SET selection_revision = CASE artifact_sha256
            WHEN $1 THEN 3000
            WHEN $2 THEN 2000
            ELSE selection_revision
        END
        WHERE artifact_sha256 IN ($1, $2)
        "#,
    )
    .bind(first.sha256.as_slice())
    .bind(second.sha256.as_slice())
    .execute(&pool)
    .await
    .expect("forge an over-budget projection revision total");

    let error = store
        .list_recoverable()
        .await
        .expect_err("enumeration over the total audit budget must fail before query fan-out");
    assert!(
        matches!(
            error,
            ModelRegistryPersistenceError::AuthorityUnavailable(_)
        ) && error
            .to_string()
            .contains("exceeding the bounded 4096-event total"),
        "unexpected enumeration budget error: {error}"
    );
    let registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count projections after bounded enumeration rejection");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count audits after bounded enumeration rejection");
    assert_eq!(
        registry_count, 2,
        "enumeration rejection must not delete rows"
    );
    assert_eq!(audit_count, 2, "enumeration rejection must not alter audit");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt014_concurrent_incompatible_adapter_selection_has_one_winner() {
    let pg = pg_required("mt014 concurrent incompatible adapter selection").await;
    let pool_a = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect first concurrent registry writer");
    let store_a = ModelRegistryStore::new(pool_a.clone());
    let store_b = ModelRegistryStore::new(
        sqlx::PgPool::connect(&pg.schema_url)
            .await
            .expect("connect second concurrent registry writer"),
    );
    let candidate_a = registration(
        0x22,
        RuntimeBinding::Candle,
        "concurrent-candle",
        "writer-a",
        "models/concurrent.safetensors",
    );
    let candidate_b = registration(
        0x22,
        RuntimeBinding::LlamaCpp,
        "concurrent-llama-cpp",
        "writer-b",
        "models/concurrent.gguf",
    );
    let barrier = Arc::new(Barrier::new(3));
    let first = {
        let barrier = barrier.clone();
        let store = store_a.clone();
        let candidate = candidate_a.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            store.persist_and_read_back(&candidate).await
        })
    };
    let second = {
        let barrier = barrier.clone();
        let store = store_b.clone();
        let candidate = candidate_b.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            store.persist_and_read_back(&candidate).await
        })
    };
    barrier.wait().await;
    let first = first.await.expect("first writer task joins");
    let second = second.await.expect("second writer task joins");

    match (&first, &second) {
        (Ok(winner), Err(ModelRegistryPersistenceError::SelectionConflict(_)))
        | (Err(ModelRegistryPersistenceError::SelectionConflict(_)), Ok(winner)) => {
            assert_eq!(winner.selection_revision, 1);
        }
        (left, right) => panic!(
            "serialized incompatible selections require exactly one winner, got left={left:?}, right={right:?}"
        ),
    }
    let rows = store_a
        .list_recoverable()
        .await
        .expect("read serialized durable selection");
    assert_eq!(rows.len(), 1);
    let event_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_runtime_registry'
          AND aggregate_id = $1
        "#,
    )
    .bind(format!("sha256:{}", hex::encode([0x22; 32])))
    .fetch_one(&pool_a)
    .await
    .expect("count committed model registry audit events");
    assert_eq!(event_count, 1, "losing transaction must leave no audit row");
}

#[tokio::test]
async fn mt014_display_name_change_preserves_selection_and_revision() {
    let pg = pg_required("mt014 display rename and project relocation").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect rename proof authority");
    let store = ModelRegistryStore::new(pool);
    let first = registration(
        0x33,
        RuntimeBinding::Candle,
        "old-display-name",
        "initial-observer",
        "old-root/models/model.safetensors",
    );
    let original = store
        .persist_and_read_back(&first)
        .await
        .expect("persist original observation");
    let renamed = registration(
        0x33,
        RuntimeBinding::Candle,
        "renamed-display-name",
        "next-boot-observer",
        "new-root/models/model.safetensors",
    );
    let updated = store
        .persist_and_read_back(&renamed)
        .await
        .expect("mutable rename and relocation must not conflict");

    assert_eq!(updated.registry_row_id, original.registry_row_id);
    assert_eq!(updated.selection_revision, original.selection_revision);
    assert_eq!(
        updated.selection_created_event_id,
        original.selection_created_event_id
    );
    assert_eq!(
        updated.selection_updated_event_id,
        original.selection_updated_event_id
    );
    assert_eq!(
        updated.selection_updated_at_utc,
        original.selection_updated_at_utc
    );
    assert_eq!(updated.base_model_tag, renamed.base_model_tag);
    assert_eq!(updated.last_observed_by, renamed.registered_by);
    assert_eq!(updated.last_observed_runtime_model_id, renamed.model_id);
    assert_eq!(updated.artifact_locator, original.artifact_locator);
    assert_eq!(updated.selection(), original.selection());
}

#[tokio::test]
async fn mt014_explicit_rebind_requires_audit_and_compare_and_swap() {
    let pg = pg_required("mt014 explicit audited compare-and-swap rebind").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect explicit rebind proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    let initial_registration = registration(
        0x44,
        RuntimeBinding::Candle,
        "last-successful-display",
        "last-successful-observer",
        "models/rebind-source.safetensors",
    );
    let initial = store
        .persist_and_read_back(&initial_registration)
        .await
        .expect("persist initial immutable selection");

    assert!(matches!(
        ExplicitModelRuntimeRebind::new(KernelActor::Operator("".to_string()), "valid reason", 1),
        Err(ModelRegistryPersistenceError::InvalidRebind(_))
    ));
    assert!(matches!(
        ExplicitModelRuntimeRebind::new(
            KernelActor::System("not-an-operator".to_string()),
            "valid reason",
            1,
        ),
        Err(ModelRegistryPersistenceError::InvalidRebind(_))
    ));
    assert!(matches!(
        ExplicitModelRuntimeRebind::new(KernelActor::Operator("operator-a".to_string()), "   ", 1),
        Err(ModelRegistryPersistenceError::InvalidRebind(_))
    ));

    let target_registration = registration(
        0x44,
        RuntimeBinding::LlamaCpp,
        "must-not-overwrite-observation-label",
        "must-not-overwrite-observer",
        "must-not-overwrite-observation-path.gguf",
    );
    let target = ModelRuntimeSelection::from(&target_registration);
    let rebound = store
        .rebind_selection_for_tests(
            &target,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("operator-a".to_string()),
                "operator selected llama.cpp after unload",
                1,
            )
            .expect("construct explicit rebind evidence"),
        )
        .await
        .expect("compare-and-swap immutable selection with same-transaction audit");
    assert_eq!(rebound.selection_revision, 2);
    assert_eq!(rebound.selection(), target);
    assert_eq!(
        rebound.selection_created_event_id,
        initial.selection_created_event_id
    );
    assert_ne!(
        rebound.selection_updated_event_id,
        initial.selection_updated_event_id
    );
    assert_eq!(
        rebound.last_observed_runtime_model_id, initial.last_observed_runtime_model_id,
        "rebind is not a successful load observation"
    );
    assert_eq!(rebound.base_model_tag, initial.base_model_tag);
    assert_eq!(rebound.last_observed_by, initial.last_observed_by);
    assert_eq!(rebound.last_observed_at_utc, initial.last_observed_at_utc);

    let event = sqlx::query(
        r#"
        SELECT event_type, actor_kind, actor_id, causation_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&rebound.selection_updated_event_id)
    .fetch_one(&pool)
    .await
    .expect("explicit rebind has a committed audit event");
    assert_eq!(
        event.get::<String, _>("event_type"),
        KernelEventType::ModelRuntimeSelectionRebound.as_str()
    );
    assert_eq!(event.get::<String, _>("actor_kind"), "operator");
    assert_eq!(event.get::<String, _>("actor_id"), "operator-a");
    assert_eq!(
        event.get::<Option<String>, _>("causation_id"),
        Some(initial.selection_updated_event_id.clone())
    );
    let payload: Value = event.get("payload");
    assert_eq!(payload["action"], "explicit_rebind");
    assert_eq!(payload["selection_revision"], 2);
    assert_eq!(
        payload["reason"],
        "operator selected llama.cpp after unload"
    );

    let stale = store
        .rebind_selection_for_tests(
            &initial.selection(),
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("operator-b".to_string()),
                "stale panel action",
                1,
            )
            .expect("construct stale compare-and-swap request"),
        )
        .await
        .expect_err("stale revision must not overwrite the committed rebind");
    assert!(matches!(
        stale,
        ModelRegistryPersistenceError::SelectionRevisionMismatch {
            expected: 1,
            actual: 2
        }
    ));
    let implicit = store
        .persist_and_read_back(&initial_registration)
        .await
        .expect_err("ordinary boot cannot silently undo an explicit rebind");
    assert!(matches!(
        implicit,
        ModelRegistryPersistenceError::SelectionConflict(_)
    ));
    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(&rebound.artifact_locator)
    .fetch_one(&pool)
    .await
    .expect("count initial and explicit rebind events");
    assert_eq!(
        event_count, 2,
        "stale and implicit writes leave no audit rows"
    );
}

#[tokio::test]
async fn mt014_registry_rejects_projection_rewound_to_valid_audit_prefix() {
    let pg = pg_required("mt014 valid audit-prefix rewind rejection").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect audit-prefix rewind authority");
    let store = ModelRegistryStore::new(pool.clone());
    let original = registration(
        0x45,
        RuntimeBinding::Candle,
        "audit-prefix-original",
        "audit-prefix-observer",
        "models/audit-prefix-original.safetensors",
    );
    let initial = store
        .persist_and_read_back(&original)
        .await
        .expect("persist revision-one selection");
    let target_registration = registration(
        0x45,
        RuntimeBinding::LlamaCpp,
        "audit-prefix-rebound",
        "audit-prefix-rebind-observer",
        "models/audit-prefix-rebound.gguf",
    );
    let rebound = store
        .rebind_selection_for_tests(
            &ModelRuntimeSelection::from(&target_registration),
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("audit-prefix-operator".to_string()),
                "create revision two before rewinding only the projection",
                1,
            )
            .expect("construct revision-two rebind"),
        )
        .await
        .expect("persist revision-two audit authority");
    assert_eq!(rebound.selection_revision, 2);

    sqlx::query(
        r#"
        UPDATE ONLY model_runtime_registry
        SET runtime_binding = 'candle',
            capabilities_json = $2,
            provider = 'local',
            selection_revision = 1,
            selection_updated_event_id = selection_created_event_id,
            selection_updated_at_utc = selection_created_at_utc
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(original.sha256.as_slice())
    .bind(
        serde_json::to_value(ModelRuntimeSelection::from(&original).declared_capabilities)
            .expect("encode exact revision-one capabilities"),
    )
    .execute(&pool)
    .await
    .expect("rewind projection to an internally valid revision-one prefix");

    let error = store
        .load_by_artifact_sha256(&original.sha256)
        .await
        .expect_err("latest revision-two EventLedger authority must defeat projection rewind");
    assert!(
        matches!(error, ModelRegistryPersistenceError::CorruptRow(_))
            && error
                .to_string()
                .contains("not the latest EventLedger authority"),
        "rewind rejection must identify the ignored audit suffix: {error}"
    );
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(format!("sha256:{}", hex::encode(original.sha256)))
    .fetch_one(&pool)
    .await
    .expect("count preserved audit suffix after projection rewind");
    assert_eq!(audit_count, 2);
    assert_eq!(initial.selection_revision, 1);
}

#[tokio::test]
async fn mt014_primary_and_embedding_registration_is_atomic_on_conflict() {
    let pg = pg_required("mt014 atomic primary plus embedding registration").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect atomic boot-set proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    let existing_embedding = registration(
        0x52,
        RuntimeBinding::Candle,
        "existing-embedding",
        "existing-observer",
        "models/embedding.safetensors",
    );
    let persisted_embedding = store
        .persist_and_read_back(&existing_embedding)
        .await
        .expect("persist existing embedding selection");

    let primary = registration(
        0x51,
        RuntimeBinding::Candle,
        "new-primary",
        "boot-observer",
        "models/primary.safetensors",
    );
    let incompatible_embedding = registration(
        0x52,
        RuntimeBinding::LlamaCpp,
        "conflicting-embedding",
        "boot-observer",
        "models/embedding.gguf",
    );
    let error = store
        .persist_boot_set_and_read_back(&[primary.clone(), incompatible_embedding])
        .await
        .expect_err("one conflict must roll back the complete primary/embedding set");
    assert!(matches!(
        error,
        ModelRegistryPersistenceError::SelectionConflict(_)
    ));
    assert!(
        store
            .load_by_artifact_sha256(&primary.sha256)
            .await
            .expect("read primary artifact after rejected batch")
            .is_none(),
        "primary row must not escape a rejected boot-set transaction"
    );
    let embedding_after = store
        .load_by_artifact_sha256(&existing_embedding.sha256)
        .await
        .expect("read existing embedding after rejected batch")
        .expect("existing embedding selection remains");
    assert_eq!(embedding_after, persisted_embedding);
    assert_eq!(
        store
            .list_recoverable()
            .await
            .expect("enumerate authority after rejected batch")
            .len(),
        1
    );
    let primary_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry' AND aggregate_id = $1",
    )
    .bind(format!("sha256:{}", hex::encode(primary.sha256)))
    .fetch_one(&pool)
    .await
    .expect("count rolled-back primary selection events");
    assert_eq!(primary_event_count, 0);
}

#[tokio::test]
async fn mt014_registry_authority_shape_rejects_semantic_drift() {
    let pg = pg_required("mt014 registry semantic shape drift rejection").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect semantic shape proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    store
        .ensure_authority_available()
        .await
        .expect("fresh migration satisfies semantic authority shape");

    sqlx::query("ALTER TABLE model_runtime_registry SET UNLOGGED")
        .execute(&pool)
        .await
        .expect("convert registry authority to a crash-truncated unlogged table");
    let persistence_error = store
        .ensure_authority_available()
        .await
        .expect_err("an UNLOGGED table must not satisfy durable registry authority");
    assert!(
        persistence_error.to_string().contains("permanent logged"),
        "shape error must identify crash-durability drift: {persistence_error}"
    );
    sqlx::query("ALTER TABLE model_runtime_registry SET LOGGED")
        .execute(&pool)
        .await
        .expect("restore permanent logged registry authority");
    store
        .ensure_authority_available()
        .await
        .expect("restored permanent table satisfies semantic authority shape");

    sqlx::query("ALTER TABLE model_runtime_registry ALTER COLUMN base_model_tag DROP NOT NULL")
        .execute(&pool)
        .await
        .expect("introduce wrong nullability while preserving column name and type");
    let nullability_error = store
        .ensure_authority_available()
        .await
        .expect_err("nullable required column must fail semantic preflight");
    assert!(
        nullability_error
            .to_string()
            .contains("built-in-type-match/nullability"),
        "shape error must identify nullability drift: {nullability_error}"
    );
    sqlx::query("ALTER TABLE model_runtime_registry ALTER COLUMN base_model_tag SET NOT NULL")
        .execute(&pool)
        .await
        .expect("restore required nullability for constraint-definition probe");

    sqlx::raw_sql(&format!(
        r#"
        CREATE DOMAIN "{schema}"."text" AS pg_catalog.text;
        ALTER TABLE "{schema}".model_runtime_registry
            ALTER COLUMN base_model_tag TYPE "{schema}"."text"
            USING base_model_tag::pg_catalog.text;
        "#,
        schema = pg.schema,
    ))
    .execute(&pool)
    .await
    .expect("replace built-in text column with a same-named authority-schema domain");
    let domain_error = store
        .ensure_authority_available()
        .await
        .expect_err("homonymous domain must not satisfy the built-in pg_catalog type contract");
    assert!(
        domain_error.to_string().contains("built-in-type-match")
            && domain_error.to_string().contains("base_model_tag"),
        "shape error must identify homonymous-domain drift: {domain_error}"
    );
    sqlx::raw_sql(&format!(
        r#"
        ALTER TABLE "{schema}".model_runtime_registry
            ALTER COLUMN base_model_tag TYPE pg_catalog.text
            USING base_model_tag::pg_catalog.text;
        DROP DOMAIN "{schema}"."text";
        ALTER TABLE "{schema}".model_runtime_registry
            ADD COLUMN mt014_unexpected_column pg_catalog.text;
        "#,
        schema = pg.schema,
    ))
    .execute(&pool)
    .await
    .expect("restore built-in text and add one unexpected visible column");
    let extra_column_error = store
        .ensure_authority_available()
        .await
        .expect_err("extra visible column must fail the exact authority contract");
    assert!(
        extra_column_error.to_string().contains("visible columns")
            && extra_column_error.to_string().contains("expected exactly"),
        "shape error must identify extra-column drift: {extra_column_error}"
    );
    sqlx::query("ALTER TABLE model_runtime_registry DROP COLUMN mt014_unexpected_column")
        .execute(&pool)
        .await
        .expect("remove unexpected column before constraint-definition probes");
    store
        .ensure_authority_available()
        .await
        .expect("built-in exact-column authority recovers after drift is removed");

    sqlx::query(
        r#"
        ALTER TABLE model_runtime_registry
            DROP CONSTRAINT chk_model_runtime_registry_selection_revision,
            ADD CONSTRAINT chk_model_runtime_registry_selection_revision
                CHECK (selection_revision >= 0)
        "#,
    )
    .execute(&pool)
    .await
    .expect("replace required constraint with same-name weaker definition");
    let definition_error = store
        .ensure_authority_available()
        .await
        .expect_err("same-name but weaker constraint must fail semantic preflight");
    assert!(
        definition_error.to_string().contains("semantic definition"),
        "shape error must identify constraint-definition drift: {definition_error}"
    );

    sqlx::query(
        r#"
        ALTER TABLE model_runtime_registry
            DROP CONSTRAINT chk_model_runtime_registry_selection_revision,
            ADD CONSTRAINT chk_model_runtime_registry_selection_revision
                CHECK (selection_revision >= 1),
            DROP CONSTRAINT fk_model_runtime_registry_selection_created_event,
            ADD CONSTRAINT fk_model_runtime_registry_selection_created_event
                FOREIGN KEY (selection_created_event_id)
                REFERENCES kernel_event_ledger (event_id) ON DELETE CASCADE
        "#,
    )
    .execute(&pool)
    .await
    .expect("replace required EventLedger FK with same-name cascading definition");
    let foreign_key_error = store
        .ensure_authority_available()
        .await
        .expect_err("same-name but cascading EventLedger FK must fail semantic preflight");
    assert!(
        foreign_key_error
            .to_string()
            .contains("fk_model_runtime_registry_selection_created_event"),
        "shape error must identify EventLedger FK definition drift: {foreign_key_error}"
    );

    sqlx::query("DROP TABLE model_runtime_registry")
        .execute(&pool)
        .await
        .expect("drop drifted authority before exact-shape replacement probe");
    sqlx::raw_sql(include_str!(
        "../migrations/0348_model_runtime_registry.sql"
    ))
    .execute(&pool)
    .await
    .expect("recreate exact migration-shaped authority with a new relation identity");
    let replacement_error = store
        .ensure_authority_available()
        .await
        .expect_err("cached authority must reject a same-schema exact-shape replacement");
    assert!(
        replacement_error.to_string().contains("changed identity"),
        "replacement error must identify authority identity drift: {replacement_error}"
    );
    ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect("a fresh store may bind the recreated exact-shape authority");

    sqlx::query(
        "CREATE TABLE model_runtime_registry_inherited_child () INHERITS (model_runtime_registry)",
    )
    .execute(&pool)
    .await
    .expect("attach a non-authoritative inherited child to the registry table");
    let inheritance_error = ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect_err("registry authority participating in inheritance must fail closed");
    assert!(
        inheritance_error.to_string().contains("changed identity"),
        "inheritance rejection must identify authority drift: {inheritance_error}"
    );
    sqlx::query("DROP TABLE model_runtime_registry_inherited_child")
        .execute(&pool)
        .await
        .expect("remove inherited registry child after negative proof");
    ModelRegistryStore::new(pool.clone())
        .ensure_authority_available()
        .await
        .expect("registry authority recovers after inherited child is removed");

    let base_url = pg.database_url.clone();
    let schema = format!("mt014_wrong_table_kind_{}", uuid::Uuid::now_v7().simple());
    let mut connection = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for wrong relation-kind probe");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut connection)
        .await
        .expect("create isolated wrong-kind schema");
    sqlx::query(&format!(
        "CREATE TABLE {schema}.kernel_event_ledger (event_id pg_catalog.text)"
    ))
    .execute(&mut connection)
    .await
    .expect("create resolver prerequisite for wrong registry relation-kind probe");
    sqlx::query(&format!(
        "CREATE VIEW {schema}.model_runtime_registry AS SELECT 1::BIGINT AS selection_revision"
    ))
    .execute(&mut connection)
    .await
    .expect("create same-named view instead of authority table");
    drop(connection);
    let separator = if base_url.contains('?') { "&" } else { "?" };
    let wrong_kind_url = format!("{base_url}{separator}options=-csearch_path%3D{schema}");
    let wrong_kind_store = ModelRegistryStore::new(
        sqlx::PgPool::connect(&wrong_kind_url)
            .await
            .expect("connect wrong relation-kind schema"),
    );
    let wrong_kind_error = wrong_kind_store
        .ensure_authority_available()
        .await
        .expect_err("same-named view must not satisfy registry authority");
    assert!(wrong_kind_error
        .to_string()
        .contains("ordinary PostgreSQL table"));
}

#[tokio::test]
async fn mt014_registry_rejects_eventledger_chain_and_immutable_row_tampering() {
    let pg = pg_required("mt014 EventLedger selection-chain tamper rejection").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect selection-chain tamper proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    let registrations = (0x81u8..=0x89)
        .map(|byte| {
            registration(
                byte,
                RuntimeBinding::Candle,
                &format!("audit-chain-{byte:02x}"),
                "audit-chain-observer",
                &format!("models/audit-chain-{byte:02x}.safetensors"),
            )
        })
        .collect::<Vec<_>>();
    let persisted = store
        .persist_boot_set_and_read_back(&registrations)
        .await
        .expect("persist independent rows for each negative audit-chain probe");

    let causation_target = ModelRuntimeSelection {
        artifact_sha256: registrations[7].sha256,
        runtime_binding: RuntimeBinding::LlamaCpp,
        runtime_role: ModelRuntimeRole::Completion,
        declared_capabilities: capabilities(RuntimeBinding::LlamaCpp),
        provider: ProviderKind::Local,
    };
    let causation_rebind = store
        .rebind_selection_for_tests(
            &causation_target,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("audit-chain-operator".to_string()),
                "create a revision-two chain for causation tamper proof",
                1,
            )
            .expect("construct revision-two audit-chain request"),
        )
        .await
        .expect("persist revision-two audit-chain row");
    let cross_aggregate_cause = store
        .persist_and_read_back(&registration(
            0x8a,
            RuntimeBinding::Candle,
            "cross-aggregate-audit-chain-cause",
            "audit-chain-observer",
            "models/cross-aggregate-audit-chain-cause.safetensors",
        ))
        .await
        .expect("persist a cross-aggregate EventLedger causation target");

    let sequence_order_target = ModelRuntimeSelection {
        artifact_sha256: registrations[8].sha256,
        runtime_binding: RuntimeBinding::LlamaCpp,
        runtime_role: ModelRuntimeRole::Completion,
        declared_capabilities: capabilities(RuntimeBinding::LlamaCpp),
        provider: ProviderKind::Local,
    };
    let sequence_order_rebind = store
        .rebind_selection_for_tests(
            &sequence_order_target,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("audit-sequence-operator".to_string()),
                "create revision two before a genuine same-aggregate sequence inversion",
                1,
            )
            .expect("construct revision-two sequence-order request"),
        )
        .await
        .expect("persist revision-two sequence-order row");
    let sequence_order_original = ModelRuntimeSelection::from(&registrations[8]);
    let sequence_order_latest = store
        .rebind_selection_for_tests(
            &sequence_order_original,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("audit-sequence-operator".to_string()),
                "create revision three so earlier event order can be inverted without changing the latest authority",
                2,
            )
            .expect("construct revision-three sequence-order request"),
        )
        .await
        .expect("persist revision-three sequence-order row");
    assert_eq!(sequence_order_latest.selection_revision, 3);

    let initial_sequence: i64 =
        sqlx::query_scalar("SELECT event_sequence FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&persisted[8].selection_created_event_id)
            .fetch_one(&pool)
            .await
            .expect("read initial sequence before controlled inversion");
    let revision_two_sequence: i64 =
        sqlx::query_scalar("SELECT event_sequence FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&sequence_order_rebind.selection_updated_event_id)
            .fetch_one(&pool)
            .await
            .expect("read revision-two sequence before controlled inversion");
    let mut sequence_swap = pool
        .begin()
        .await
        .expect("begin atomic EventLedger sequence inversion");
    sqlx::query("UPDATE kernel_event_ledger SET event_sequence = 0 WHERE event_id = $1")
        .bind(&persisted[8].selection_created_event_id)
        .execute(&mut *sequence_swap)
        .await
        .expect("move initial event sequence to an unused temporary value");
    sqlx::query("UPDATE kernel_event_ledger SET event_sequence = $2 WHERE event_id = $1")
        .bind(&sequence_order_rebind.selection_updated_event_id)
        .bind(initial_sequence)
        .execute(&mut *sequence_swap)
        .await
        .expect("move revision-two event before its own cause");
    sqlx::query("UPDATE kernel_event_ledger SET event_sequence = $2 WHERE event_id = $1")
        .bind(&persisted[8].selection_created_event_id)
        .bind(revision_two_sequence)
        .execute(&mut *sequence_swap)
        .await
        .expect("complete the same-aggregate event-sequence inversion");
    sequence_swap
        .commit()
        .await
        .expect("commit controlled EventLedger sequence inversion");

    sqlx::query(
        r#"
        UPDATE model_runtime_registry
        SET selection_created_event_id = $2,
            selection_updated_event_id = $2
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(registrations[0].sha256.as_slice())
    .bind(&persisted[1].selection_created_event_id)
    .execute(&pool)
    .await
    .expect("repoint both valid FKs to an unrelated valid selection event");

    sqlx::query(
        r#"
        UPDATE model_runtime_registry
        SET selection_updated_at_utc = selection_created_at_utc - INTERVAL '1 second'
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(registrations[1].sha256.as_slice())
    .execute(&pool)
    .await
    .expect("invert immutable selection timestamp ordering");

    sqlx::query(
        r#"
        UPDATE model_runtime_registry
        SET runtime_binding = 'llama_cpp',
            capabilities_json = $2
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(registrations[2].sha256.as_slice())
    .bind(
        serde_json::to_value(capabilities(RuntimeBinding::LlamaCpp))
            .expect("serialize canonical valid llama.cpp capabilities"),
    )
    .execute(&pool)
    .await
    .expect("tamper to a valid immutable selection without an audit event");

    sqlx::query(
        "UPDATE model_runtime_registry SET selection_revision = 2 WHERE artifact_sha256 = $1",
    )
    .bind(registrations[3].sha256.as_slice())
    .execute(&pool)
    .await
    .expect("tamper row revision without updating endpoint refs");

    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(payload, '{action}', '"tampered_action"'::jsonb)
        WHERE event_id = $1
        "#,
    )
    .bind(&persisted[4].selection_created_event_id)
    .execute(&pool)
    .await
    .expect("tamper payload without its canonical hash");

    sqlx::query("UPDATE kernel_event_ledger SET payload_hash = $2 WHERE event_id = $1")
        .bind(&persisted[5].selection_created_event_id)
        .bind("0".repeat(64))
        .execute(&pool)
        .await
        .expect("tamper payload hash without changing payload");

    let mut action_payload: Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&persisted[6].selection_created_event_id)
            .fetch_one(&pool)
            .await
            .expect("load action-tamper payload");
    action_payload["action"] = Value::String("tampered_action_with_matching_hash".to_string());
    let action_payload_hash = NewKernelEvent::builder(
        "hash-only-task",
        "hash-only-session",
        KernelEventType::ModelRuntimeSelectionRecorded,
        KernelActor::System("hash-only".to_string()),
    )
    .payload(action_payload.clone())
    .build()
    .expect("compute canonical hash for semantically invalid action")
    .payload_hash;
    sqlx::query(
        "UPDATE kernel_event_ledger SET payload = $2, payload_hash = $3 WHERE event_id = $1",
    )
    .bind(&persisted[6].selection_created_event_id)
    .bind(action_payload)
    .bind(action_payload_hash)
    .execute(&pool)
    .await
    .expect("tamper action while keeping payload hash internally consistent");

    sqlx::query("UPDATE kernel_event_ledger SET causation_id = $2 WHERE event_id = $1")
        .bind(&causation_rebind.selection_updated_event_id)
        .bind(&cross_aggregate_cause.selection_created_event_id)
        .execute(&pool)
        .await
        .expect("repoint rebind causation to an unrelated aggregate event");

    for (index, expected_fragment) in [
        (0usize, "audit chain"),
        (1, "precedes"),
        (2, "audit chain"),
        (3, "revision"),
        (4, "metadata/hash"),
        (5, "metadata/hash"),
        (6, "action"),
        (7, "absent from the artifact-scoped audit set"),
        (8, "not earlier"),
    ] {
        let error = store
            .load_by_artifact_sha256(&registrations[index].sha256)
            .await
            .expect_err("tamper probe must fail closed during same-snapshot recovery");
        assert!(matches!(
            &error,
            ModelRegistryPersistenceError::CorruptRow(_)
        ));
        assert!(
            error.to_string().contains(expected_fragment),
            "tamper probe {index} should identify `{expected_fragment}`: {error}"
        );
    }
}

#[tokio::test]
async fn mt014_registry_migration_replays_and_locator_constraint_binds_exact_sha() {
    let pg = pg_required("mt014 replay-safe registry migration and exact SHA locator").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect migration replay proof authority");

    for replay in 1..=2 {
        sqlx::raw_sql(include_str!(
            "../migrations/0348_model_runtime_registry.sql"
        ))
        .execute(&pool)
        .await
        .unwrap_or_else(|error| panic!("migration replay {replay} must be idempotent: {error}"));
    }

    let store = ModelRegistryStore::new(pool.clone());
    store
        .ensure_authority_available()
        .await
        .expect("replayed authority keeps the exact semantic shape");
    sqlx::raw_sql(
        r#"
        DROP INDEX idx_model_runtime_registry_selection_updated_at;
        CREATE INDEX idx_model_runtime_registry_selection_updated_at
            ON model_runtime_registry (registry_row_id);
        "#,
    )
    .execute(&pool)
    .await
    .expect("replace required replay-safe index with same-name wrong definition");
    let index_error = store
        .ensure_authority_available()
        .await
        .expect_err("same-name wrong index must fail semantic preflight");
    assert!(
        index_error
            .to_string()
            .contains("idx_model_runtime_registry_selection_updated_at"),
        "preflight must identify wrong index semantics: {index_error}"
    );
    sqlx::raw_sql(
        r#"
        DROP INDEX idx_model_runtime_registry_selection_updated_at;
        CREATE INDEX idx_model_runtime_registry_selection_updated_at
            ON model_runtime_registry (selection_updated_at_utc DESC, registry_row_id ASC);
        "#,
    )
    .execute(&pool)
    .await
    .expect("restore required registry index definition");
    store
        .ensure_authority_available()
        .await
        .expect("restored replay-safe index satisfies semantic preflight");

    sqlx::query(
        "CREATE UNIQUE INDEX mt014_extra_registry_base_model_tag_unique ON model_runtime_registry (base_model_tag)",
    )
    .execute(&pool)
    .await
    .expect("install behavior-bearing standalone unique registry index");
    let extra_unique_error = store
        .ensure_authority_available()
        .await
        .expect_err("an extra live registry index must fail exact authority preflight");
    assert!(
        extra_unique_error
            .to_string()
            .contains("exactly its canonical")
            && extra_unique_error
                .to_string()
                .contains("mt014_extra_registry_base_model_tag_unique"),
        "unexpected extra unique-index authority error: {extra_unique_error}"
    );
    sqlx::query("DROP INDEX mt014_extra_registry_base_model_tag_unique")
        .execute(&pool)
        .await
        .expect("remove standalone unique registry index after negative proof");

    sqlx::raw_sql(
        r#"
        CREATE FUNCTION mt014_hostile_registry_index(pg_catalog.text)
        RETURNS pg_catalog.text
        LANGUAGE plpgsql
        IMMUTABLE
        AS $$
        BEGIN
            PERFORM pg_catalog.set_config('TimeZone', 'Pacific/Honolulu', true);
            RETURN $1;
        END;
        $$;
        CREATE INDEX mt014_extra_registry_expression
            ON model_runtime_registry ((mt014_hostile_registry_index(base_model_tag)));
        "#,
    )
    .execute(&pool)
    .await
    .expect("install executable expression index on the empty registry authority");
    let expression_index_error = store
        .ensure_authority_available()
        .await
        .expect_err("an executable extra registry index must fail before any DML");
    assert!(
        expression_index_error
            .to_string()
            .contains("mt014_extra_registry_expression"),
        "unexpected executable-index authority error: {expression_index_error}"
    );
    let pre_dml_registry_count: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after executable-index rejection");
    let pre_dml_audit_count: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count audit rows after executable-index rejection");
    assert_eq!(pre_dml_registry_count, 0);
    assert_eq!(pre_dml_audit_count, 0);
    sqlx::raw_sql(
        r#"
        DROP INDEX mt014_extra_registry_expression;
        DROP FUNCTION mt014_hostile_registry_index(pg_catalog.text);
        "#,
    )
    .execute(&pool)
    .await
    .expect("remove executable registry index after negative proof");
    store
        .ensure_authority_available()
        .await
        .expect("authority recovers after every extra live index is removed");

    let selected = registration(
        0x71,
        RuntimeBinding::Candle,
        "locator-binding",
        "locator-observer",
        "models/locator-binding.safetensors",
    );
    store
        .persist_and_read_back(&selected)
        .await
        .expect("persist row protected by exact locator binding");

    let wrong_but_well_formed_locator = format!("sha256:{}", hex::encode([0x72; 32]));
    let locator_error = sqlx::query(
        "UPDATE model_runtime_registry SET artifact_locator = $2 WHERE artifact_sha256 = $1",
    )
    .bind(selected.sha256.as_slice())
    .bind(wrong_but_well_formed_locator)
    .execute(&pool)
    .await
    .expect_err("a valid-looking locator for a different hash must violate the DB constraint");
    match locator_error {
        sqlx::Error::Database(database_error) => {
            assert_eq!(database_error.code().as_deref(), Some("23514"));
            assert_eq!(
                database_error.constraint(),
                Some("chk_model_runtime_registry_artifact_locator")
            );
        }
        error => panic!("expected PostgreSQL check violation, got {error}"),
    }

    sqlx::query(
        r#"
        ALTER TABLE model_runtime_registry
            DROP CONSTRAINT chk_model_runtime_registry_artifact_locator,
            ADD CONSTRAINT chk_model_runtime_registry_artifact_locator
                CHECK (artifact_locator ~ '^sha256:[0-9a-f]{64}$')
        "#,
    )
    .execute(&pool)
    .await
    .expect("replace exact locator binding with weaker same-name regex");
    let semantic_error = store
        .ensure_authority_available()
        .await
        .expect_err("same-name locator regex must not pass semantic preflight");
    assert!(
        semantic_error
            .to_string()
            .contains("chk_model_runtime_registry_artifact_locator"),
        "preflight must identify the weakened locator constraint: {semantic_error}"
    );
}

#[tokio::test]
async fn mt014_held_selection_lock_times_out_without_registry_or_audit_mutation() {
    let pg = pg_required("mt014 bounded registry advisory-lock contention").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect bounded-lock proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    let selected = registration(
        0x73,
        RuntimeBinding::Candle,
        "held-lock",
        "lock-contender",
        "models/held-lock.safetensors",
    );

    let mut blocker = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect independent advisory-lock holder");
    let mut blocker_tx = blocker
        .begin()
        .await
        .expect("begin independent advisory-lock transaction");
    let lock_key = format!(
        "handshake.model_runtime_registry.selection.v1:{}",
        hex::encode(selected.sha256)
    );
    sqlx::query(
        "SELECT pg_catalog.pg_advisory_xact_lock(('x' || pg_catalog.substr(pg_catalog.md5($1), 1, 16))::bit(64)::bigint)",
    )
    .bind(lock_key)
    .execute(&mut *blocker_tx)
    .await
    .expect("hold the exact registry selection lock");

    let started = Instant::now();
    let error = store
        .persist_and_read_back(&selected)
        .await
        .expect_err("registry contention must fail within a bounded interval");
    let elapsed = started.elapsed();
    assert!(matches!(
        error,
        ModelRegistryPersistenceError::SelectionLockTimeout {
            artifact_sha256,
            timeout_ms: 2_000,
        } if artifact_sha256 == hex::encode(selected.sha256)
    ));
    assert!(
        elapsed >= Duration::from_millis(1_800) && elapsed < Duration::from_secs(5),
        "typed contention timeout must be bounded near two seconds, observed {elapsed:?}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count registry rows after lock timeout"),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
        )
        .fetch_one(&pool)
        .await
        .expect("count model-registry audit rows after lock timeout"),
        0
    );
    blocker_tx
        .rollback()
        .await
        .expect("release independent advisory lock");
}

#[tokio::test]
async fn mt014_non_advisory_row_lock_times_out_without_registry_or_audit_mutation() {
    let pg = pg_required("mt014 bounded non-advisory registry row lock").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect non-advisory row-lock proof authority");
    let store = ModelRegistryStore::new(pool.clone());
    let original_registration = registration(
        0x74,
        RuntimeBinding::Candle,
        "row-lock-original",
        "row-lock-original-observer",
        "models/row-lock-original.safetensors",
    );
    let original = store
        .persist_and_read_back(&original_registration)
        .await
        .expect("persist row before non-advisory lock probe");
    let audit_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count audit rows before non-advisory lock probe");

    let mut blocker = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect independent raw SQL row-lock holder");
    let mut blocker_tx = blocker
        .begin()
        .await
        .expect("begin independent raw SQL row-lock transaction");
    sqlx::query(
        "SELECT registry_row_id FROM model_runtime_registry WHERE artifact_sha256 = $1 FOR UPDATE",
    )
    .bind(original_registration.sha256.as_slice())
    .fetch_one(&mut *blocker_tx)
    .await
    .expect("hold the registry row without taking the advisory lock");

    let next_observation = registration(
        0x74,
        RuntimeBinding::Candle,
        "row-lock-must-not-commit",
        "row-lock-contender",
        "moved/models/row-lock-original.safetensors",
    );
    let started = Instant::now();
    let error = store
        .persist_and_read_back(&next_observation)
        .await
        .expect_err("raw SQL row contention must fail within the database lock timeout");
    let elapsed = started.elapsed();
    assert!(matches!(
        error,
        ModelRegistryPersistenceError::SelectionLockTimeout {
            artifact_sha256,
            timeout_ms: 1_500,
        } if artifact_sha256 == hex::encode(original_registration.sha256)
    ));
    assert!(
        elapsed >= Duration::from_millis(1_300) && elapsed < Duration::from_secs(5),
        "non-advisory row lock must return the typed timeout near 1.5 seconds, observed {elapsed:?}"
    );

    let row = sqlx::query(
        r#"
        SELECT last_observed_runtime_model_id, base_model_tag, last_observed_by,
               selection_revision, selection_updated_event_id
        FROM model_runtime_registry
        WHERE artifact_sha256 = $1
        "#,
    )
    .bind(original_registration.sha256.as_slice())
    .fetch_one(&pool)
    .await
    .expect("read committed row while the raw writer still holds its no-op lock");
    assert_eq!(
        row.get::<uuid::Uuid, _>("last_observed_runtime_model_id"),
        original.last_observed_runtime_model_id.as_uuid()
    );
    assert_eq!(row.get::<String, _>("base_model_tag"), "row-lock-original");
    assert_eq!(
        row.get::<String, _>("last_observed_by"),
        "row-lock-original-observer"
    );
    assert_eq!(row.get::<i64, _>("selection_revision"), 1);
    assert_eq!(
        row.get::<String, _>("selection_updated_event_id"),
        original.selection_updated_event_id
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
        )
        .fetch_one(&pool)
        .await
        .expect("count audit rows after non-advisory lock timeout"),
        audit_count_before,
        "the timed-out observation must not append a registry audit event"
    );
    blocker_tx
        .rollback()
        .await
        .expect("release independent raw SQL row lock");
}

#[tokio::test]
async fn mt014_pinned_authority_schema_ignores_pooled_search_path_shadow() {
    let pg = pg_required("mt014 pinned registry authority schema").await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&pg.schema_url)
        .await
        .expect("connect one-session search-path proof pool");
    let store = ModelRegistryStore::new(pool.clone());
    store
        .ensure_authority_available()
        .await
        .expect("cache and validate the intended authority schema before shadow injection");

    let shadow_schema = format!("mt014_shadow_{}", uuid::Uuid::now_v7().simple());
    let mut setup = pg.raw_connection().await;
    sqlx::query(&format!("CREATE SCHEMA {shadow_schema}"))
        .execute(&mut setup)
        .await
        .expect("create search-path shadow schema");
    sqlx::query(&format!(
        "CREATE TABLE {shadow_schema}.model_runtime_registry (artifact_sha256 BYTEA NOT NULL)"
    ))
    .execute(&mut setup)
    .await
    .expect("create malicious same-name shadow table");
    sqlx::query(&format!(
        "CREATE TABLE {shadow_schema}.kernel_event_ledger (event_id TEXT NOT NULL)"
    ))
    .execute(&mut setup)
    .await
    .expect("create malicious same-name shadow EventLedger table");
    {
        let mut connection = pool
            .acquire()
            .await
            .expect("acquire the only pooled connection for search-path poisoning");
        sqlx::query("CREATE TEMP TABLE model_runtime_registry (artifact_sha256 BYTEA NOT NULL)")
            .execute(&mut *connection)
            .await
            .expect("create same-name temporary-table shadow on the pooled session");
        sqlx::query(&format!(
            "SET search_path TO {shadow_schema}, {}",
            pg.schema
        ))
        .execute(&mut *connection)
        .await
        .expect("poison pooled connection search path with same-name table first");
        let session_sync: String =
            sqlx::query_scalar("SELECT pg_catalog.set_config('synchronous_commit', 'off', false)")
                .fetch_one(&mut *connection)
                .await
                .expect("poison pooled session with asynchronous commit default");
        assert_eq!(session_sync, "off");
        let (fsync, full_page_writes): (String, String) = sqlx::query_as(
            "SELECT pg_catalog.current_setting('fsync'), pg_catalog.current_setting('full_page_writes')",
        )
        .fetch_one(&mut *connection)
        .await
        .expect("read PostgreSQL crash-durability settings for registry proof");
        assert_eq!(fsync, "on", "registry durability proof requires fsync=on");
        assert_eq!(
            full_page_writes, "on",
            "registry durability proof requires full_page_writes=on"
        );
    }

    let selected = registration(
        0x74,
        RuntimeBinding::Candle,
        "schema-pinning",
        "schema-observer",
        "models/schema-pinning.safetensors",
    );
    let persisted = store
        .persist_and_read_back(&selected)
        .await
        .expect("transaction-local schema pin must bypass the poisoned search path");
    assert_eq!(persisted.artifact_sha256, selected.sha256);

    let intended_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {}.model_runtime_registry",
        pg.schema
    ))
    .fetch_one(&mut setup)
    .await
    .expect("count intended authority rows");
    let shadow_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {shadow_schema}.model_runtime_registry"
    ))
    .fetch_one(&mut setup)
    .await
    .expect("count shadow rows");
    let temporary_shadow_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pg_temp.model_runtime_registry")
            .fetch_one(&pool)
            .await
            .expect("count same-session temporary shadow rows");
    assert_eq!(intended_count, 1);
    assert_eq!(shadow_count, 0, "shadow authority must remain untouched");
    assert_eq!(
        temporary_shadow_count, 0,
        "temporary shadow authority must remain untouched"
    );
    let session_default_after_commit: String =
        sqlx::query_scalar("SELECT pg_catalog.current_setting('synchronous_commit')")
            .fetch_one(&pool)
            .await
            .expect("read pooled session durability default after registry commit");
    assert_eq!(
        session_default_after_commit, "off",
        "registry durability override must be transaction-local"
    );

    // The cached store above must prove it can re-pin the intended OIDs while
    // the poisoned session is still live. A fresh store has no such authority
    // identity and must reject the two complete same-name authority pairs.
    let uninitialized_store = ModelRegistryStore::new(pool.clone());
    let ambiguity = uninitialized_store
        .ensure_authority_available()
        .await
        .expect_err("first use must fail closed when two same-name authorities are visible");
    assert!(
        ambiguity.to_string().contains("ambiguous"),
        "shadow ambiguity must be explicit: {ambiguity}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn mt014_boot_readback_is_captured_before_concurrent_rebind_commit() {
    let pg = pg_required("mt014 transaction-captured boot readback under concurrent rebind").await;
    let application_name = format!("mt014_readback_{}", uuid::Uuid::now_v7().simple());
    let separator = if pg.schema_url.contains('?') {
        "&"
    } else {
        "?"
    };
    let observation_url = format!(
        "{}{separator}application_name={application_name}",
        pg.schema_url
    );
    let observation_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&observation_url)
        .await
        .expect("connect one-session observation pool");
    let observation_store = ModelRegistryStore::new(observation_pool.clone());
    let control_pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect independent rebind and activity-observer pool");
    let rebind_store = ModelRegistryStore::new(control_pool.clone());

    let initial_registration = registration(
        0x75,
        RuntimeBinding::Candle,
        "readback-initial",
        "initial-observer",
        "models/readback-race.safetensors",
    );
    observation_store
        .persist_and_read_back(&initial_registration)
        .await
        .expect("persist initial selection before race proof");

    let mut observation_blocker = control_pool
        .begin()
        .await
        .expect("begin controlled observation row-lock blocker");
    sqlx::query(
        "SELECT registry_row_id FROM ONLY model_runtime_registry WHERE artifact_sha256 = $1 FOR UPDATE",
    )
    .bind(initial_registration.sha256.as_slice())
    .fetch_one(&mut *observation_blocker)
    .await
    .expect("hold row lock while observation owns the artifact advisory lock");

    let observed_registration = registration(
        0x75,
        RuntimeBinding::Candle,
        "readback-observed",
        "next-boot-observer",
        "moved/models/readback-race.safetensors",
    );
    let observation_task = {
        let store = observation_store.clone();
        let registration = observed_registration.clone();
        tokio::spawn(async move { store.persist_and_read_back(&registration).await })
    };
    wait_for_registry_lock_wait(&control_pool, &application_name).await;

    let target_registration = registration(
        0x75,
        RuntimeBinding::LlamaCpp,
        "unused-rebind-label",
        "unused-rebind-observer",
        "models/readback-race.gguf",
    );
    let rebind_task = {
        let store = rebind_store.clone();
        let target = ModelRuntimeSelection::from(&target_registration);
        tokio::spawn(async move {
            store
                .rebind_selection_for_tests(
                    &target,
                    ExplicitModelRuntimeRebind::new(
                        KernelActor::Operator("race-operator".to_string()),
                        "concurrent explicit rebind after successful boot observation",
                        1,
                    )
                    .expect("construct concurrent rebind request"),
                )
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(80)).await;

    let pool_hog = {
        let pool = observation_pool.clone();
        tokio::spawn(async move {
            let connection = pool
                .acquire()
                .await
                .expect("queued pool waiter acquires connection after observation commit");
            tokio::time::sleep(Duration::from_millis(750)).await;
            drop(connection);
        })
    };
    observation_blocker
        .rollback()
        .await
        .expect("release controlled observation row lock");

    let observed = observation_task
        .await
        .expect("observation task joins")
        .expect("boot observation returns its own transaction-captured row");
    let rebound = rebind_task
        .await
        .expect("rebind task joins")
        .expect("concurrent rebind commits after boot observation");
    pool_hog.await.expect("pool-hog task joins");

    assert_eq!(observed.selection_revision, 1);
    assert_eq!(observed.runtime_binding, RuntimeBinding::Candle);
    assert_eq!(
        observed.base_model_tag,
        observed_registration.base_model_tag
    );
    assert_eq!(rebound.selection_revision, 2);
    assert_eq!(rebound.runtime_binding, RuntimeBinding::LlamaCpp);
    assert_eq!(
        rebound.selection_created_at_utc, observed.selection_created_at_utc,
        "explicit rebind preserves the initial selection timestamp"
    );
    assert!(
        rebound.selection_updated_at_utc >= observed.selection_updated_at_utc,
        "revision-two audit time must not precede revision one"
    );
    let durable_after_boot_race = rebind_store
        .load_by_artifact_sha256(&initial_registration.sha256)
        .await
        .expect("load final durable registry selection")
        .expect("durable registry row remains present");
    assert_eq!(durable_after_boot_race.selection_revision, 2);
    assert_eq!(
        durable_after_boot_race.runtime_binding,
        RuntimeBinding::LlamaCpp
    );

    let mut rebind_blocker = control_pool
        .begin()
        .await
        .expect("begin controlled rebind row-lock blocker");
    sqlx::query(
        "SELECT registry_row_id FROM ONLY model_runtime_registry WHERE artifact_sha256 = $1 FOR UPDATE",
    )
    .bind(initial_registration.sha256.as_slice())
    .fetch_one(&mut *rebind_blocker)
    .await
    .expect("hold row lock while first rebind owns the artifact advisory lock");

    let revision_three_target = ModelRuntimeSelection::from(&initial_registration);
    let first_rebind_task = {
        let store = observation_store.clone();
        tokio::spawn(async move {
            store
                .rebind_selection_for_tests(
                    &revision_three_target,
                    ExplicitModelRuntimeRebind::new(
                        KernelActor::Operator("first-race-operator".to_string()),
                        "first rebind receipt must remain revision three",
                        2,
                    )
                    .expect("construct first raced rebind request"),
                )
                .await
        })
    };
    wait_for_registry_lock_wait(&control_pool, &application_name).await;

    let revision_four_target = ModelRuntimeSelection::from(&target_registration);
    let second_rebind_task = {
        let store = rebind_store.clone();
        tokio::spawn(async move {
            store
                .rebind_selection_for_tests(
                    &revision_four_target,
                    ExplicitModelRuntimeRebind::new(
                        KernelActor::Operator("second-race-operator".to_string()),
                        "second rebind commits after revision-three receipt capture",
                        3,
                    )
                    .expect("construct second raced rebind request"),
                )
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(80)).await;

    let second_pool_hog = {
        let pool = observation_pool.clone();
        tokio::spawn(async move {
            let connection = pool
                .acquire()
                .await
                .expect("second queued pool waiter acquires after first rebind commit");
            tokio::time::sleep(Duration::from_millis(750)).await;
            drop(connection);
        })
    };
    rebind_blocker
        .rollback()
        .await
        .expect("release controlled rebind row lock");

    let revision_three_receipt = first_rebind_task
        .await
        .expect("first rebind task joins")
        .expect("first rebind returns its own transaction-captured receipt");
    let revision_four_receipt = second_rebind_task
        .await
        .expect("second rebind task joins")
        .expect("second rebind commits after the first");
    second_pool_hog.await.expect("second pool-hog task joins");
    assert_eq!(revision_three_receipt.selection_revision, 3);
    assert_eq!(
        revision_three_receipt.runtime_binding,
        RuntimeBinding::Candle
    );
    assert_eq!(revision_four_receipt.selection_revision, 4);
    assert_eq!(
        revision_four_receipt.runtime_binding,
        RuntimeBinding::LlamaCpp
    );
    assert_eq!(
        revision_three_receipt.selection_created_at_utc, observed.selection_created_at_utc,
        "revision three preserves the original selection timestamp"
    );
    assert_eq!(
        revision_four_receipt.selection_created_at_utc, observed.selection_created_at_utc,
        "revision four preserves the original selection timestamp"
    );
    assert!(
        revision_three_receipt.selection_updated_at_utc >= rebound.selection_updated_at_utc,
        "revision-three audit time must not precede revision two"
    );
    assert!(
        revision_four_receipt.selection_updated_at_utc
            >= revision_three_receipt.selection_updated_at_utc,
        "revision-four audit time must not precede revision three"
    );

    let final_durable = rebind_store
        .load_by_artifact_sha256(&initial_registration.sha256)
        .await
        .expect("load durable selection after both receipt races")
        .expect("durable registry row remains present after both races");
    assert_eq!(final_durable.selection_revision, 4);
    assert_eq!(final_durable.runtime_binding, RuntimeBinding::LlamaCpp);
}

#[tokio::test]
async fn mt013_configured_production_local_boot_without_ledger_fails_before_artifact_access() {
    let pg = pg_required("mt013 configured local boot missing ProcessOwnershipLedger").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect zero-mutation registry proof authority");
    let registry_store = ModelRegistryStore::new(pool.clone());
    let missing_artifact = PathBuf::from(format!(
        "must-not-be-opened-before-ledger-check-{}.safetensors",
        uuid::Uuid::now_v7()
    ));
    let resolved = ResolvedProvider {
        provider_id: "local_runtime".to_string(),
        kind: LlmProviderKind::LocalRuntime,
        tier: ModelTier::Local,
        base_url: "local://embedded".to_string(),
        model_id: "mt013-missing-process-ledger".to_string(),
        api_key_env: None,
        local_model: Some(LocalModelConfig {
            artifact_path: missing_artifact,
            sha256: [0x61; 32],
            runtime_binding: RuntimeBinding::Candle,
            display_name: "mt013-missing-process-ledger".to_string(),
            embedding_dimension: None,
        }),
        local_embedding_model: None,
    };

    let explicit_scope = format!("mt013-missing-ledger-{}", uuid::Uuid::now_v7());
    let host_scope =
        resolve_embedded_runtime_host_scope_with_override(&pg.schema_url, Some(&explicit_scope))
            .expect("derive embedded-runtime lease host scope from the real PostgreSQL authority");
    let lease = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), host_scope)
        .expect("acquire real loopback owner-instance lease before configured boot");
    let client = build_default_local_client(
        &resolved,
        Arc::new(NoopRecorder),
        None,
        Some(registry_store),
        Some(lease.descriptor().clone()),
    )
    .await;
    let error = client
        .completion(CompletionRequest::new(
            uuid::Uuid::now_v7(),
            "must fail before model artifact access".to_string(),
            "mt013-missing-process-ledger".to_string(),
        ))
        .await
        .expect_err("configured production-local boot without ledger must be disabled");
    assert!(
        error.to_string().contains("ProcessOwnershipLedger"),
        "missing-ledger failure must remain explicit: {error}"
    );
    let registry_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_runtime_registry")
        .fetch_one(&pool)
        .await
        .expect("count registry rows after rejected missing-ledger boot");
    let registry_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry events after rejected missing-ledger boot");
    assert_eq!(registry_count, 0);
    assert_eq!(registry_event_count, 0);
}

#[tokio::test]
async fn mt013_undersized_ledger_reservation_fails_before_artifact_access_without_rows() {
    let pg = pg_required("mt013 undersized ProcessOwnershipLedger pre-artifact reservation").await;
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect undersized-ledger proof authority");
    let registry_store = ModelRegistryStore::new(pool.clone());
    let (ledger, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 1,
            batch_size: 1,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("construct one-slot manual ledger that cannot reserve START plus STOP");
    let missing_artifact = PathBuf::from(format!(
        "must-not-be-opened-before-complete-lifecycle-reservation-{}.safetensors",
        uuid::Uuid::now_v7()
    ));
    assert!(
        !missing_artifact.exists(),
        "precondition: deliberately absent artifact path must remain absent"
    );
    let resolved = ResolvedProvider {
        provider_id: "local_runtime".to_string(),
        kind: LlmProviderKind::LocalRuntime,
        tier: ModelTier::Local,
        base_url: "local://embedded".to_string(),
        model_id: "mt013-undersized-process-ledger".to_string(),
        api_key_env: None,
        local_model: Some(LocalModelConfig {
            artifact_path: missing_artifact,
            sha256: [0x62; 32],
            runtime_binding: RuntimeBinding::Candle,
            display_name: "mt013-undersized-process-ledger".to_string(),
            embedding_dimension: None,
        }),
        local_embedding_model: None,
    };

    let explicit_scope = format!("mt013-undersized-ledger-{}", uuid::Uuid::now_v7());
    let host_scope =
        resolve_embedded_runtime_host_scope_with_override(&pg.schema_url, Some(&explicit_scope))
            .expect("derive embedded-runtime lease host scope from the real PostgreSQL authority");
    let lease = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), host_scope)
        .expect("acquire real loopback owner-instance lease before configured boot");
    let client = build_default_local_client(
        &resolved,
        Arc::new(NoopRecorder),
        Some(ledger),
        Some(registry_store),
        Some(lease.descriptor().clone()),
    )
    .await;
    let error = client
        .completion(CompletionRequest::new(
            uuid::Uuid::now_v7(),
            "must fail at complete lifecycle reservation".to_string(),
            "mt013-undersized-process-ledger".to_string(),
        ))
        .await
        .expect_err("undersized ledger must disable configured local boot");
    let reason = match error {
        LlmError::ProviderError(reason) => reason,
        other => panic!(
            "undersized lifecycle reservation must retain the typed provider-error boundary, got {other}"
        ),
    };
    assert!(
        reason.contains("HSK-LOCAL-DISABLED")
            && reason.contains("PROCESS_LEDGER_ENQUEUE_DROPPED")
            && reason.contains("could not reserve complete START/STOP authority")
            && reason.contains("before artifact access")
            && reason.contains("writer is undersized"),
        "failure must identify the pre-artifact complete-lifecycle reservation gate: {reason}"
    );
    assert!(
        !reason.contains("embedded model load failed")
            && !reason.contains("No such file")
            && !reason.contains("os error"),
        "failure must not be an artifact/load error: {reason}"
    );

    drain
        .drain_available_to(Arc::new(PostgresProcessLedgerStore::new(pool.clone())))
        .await
        .expect("flush any manual-ledger events into the real authority for zero-row proof");
    let process_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kernel_process_lifecycle")
        .fetch_one(&pool)
        .await
        .expect("count process rows after rejected undersized-ledger boot");
    let registry_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_runtime_registry")
        .fetch_one(&pool)
        .await
        .expect("count registry rows after rejected undersized-ledger boot");
    let registry_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_registry'",
    )
    .fetch_one(&pool)
    .await
    .expect("count registry events after rejected undersized-ledger boot");
    assert_eq!(
        process_count, 0,
        "failed reservation emits no START/STOP row"
    );
    assert_eq!(
        registry_count, 0,
        "failed reservation persists no selection"
    );
    assert_eq!(
        registry_event_count, 0,
        "failed reservation emits no registry audit event"
    );
}
