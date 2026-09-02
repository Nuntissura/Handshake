#![cfg(all(feature = "test-utils", feature = "surreal-test-support"))]

mod surreal_test_store_support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType, RecorderError,
};
use handshake_core::storage::surreal::{
    SurrealStorage, SurrealSwarmOutboxError, SurrealSwarmOutboxStore,
};
use handshake_core::swarm_orchestration::events::DurableSwarmFrBridge;
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use surreal_test_store_support::EmbeddedSurrealTestScope;
use surrealdb::types::{RecordId, SurrealValue};
use tokio::sync::Semaphore;
use uuid::Uuid;

const OUTBOX_TABLE: &str = "swarm_terminal_event_outbox";
const EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";

#[derive(Debug, SurrealValue)]
struct OutboxProbe {
    event_id: String,
    event_json: String,
    event_json_sha256: String,
    event_ledger_event_id: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    attempts: i64,
    storage_authority: String,
}

#[derive(Debug, SurrealValue)]
struct EventLedgerProbe {
    event_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    payload_hash: String,
    source_component: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Default)]
struct CollectingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl CollectingRecorder {
    fn event_ids(&self) -> Vec<Uuid> {
        self.events
            .lock()
            .expect("collecting recorder lock")
            .iter()
            .map(|event| event.event_id)
            .collect()
    }
}

#[async_trait]
impl FlightRecorder for CollectingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events
            .lock()
            .expect("collecting recorder lock")
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
        Ok(self
            .events
            .lock()
            .expect("collecting recorder lock")
            .clone())
    }
}

struct BlockingRecorder {
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl BlockingRecorder {
    fn new() -> Self {
        Self {
            entered: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
            events: Mutex::new(Vec::new()),
        }
    }

    async fn wait_until_entered(&self) {
        self.entered
            .acquire()
            .await
            .expect("blocking recorder entry semaphore")
            .forget();
    }

    fn release(&self, permits: usize) {
        self.release.add_permits(permits);
    }

    fn event_ids(&self) -> Vec<Uuid> {
        self.events
            .lock()
            .expect("blocking recorder lock")
            .iter()
            .map(|event| event.event_id)
            .collect()
    }
}

#[async_trait]
impl FlightRecorder for BlockingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.entered.add_permits(1);
        self.release
            .acquire()
            .await
            .map_err(|_| RecorderError::SinkError("release semaphore closed".to_owned()))?
            .forget();
        self.events
            .lock()
            .expect("blocking recorder lock")
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
        Ok(self
            .events
            .lock()
            .expect("blocking recorder lock")
            .clone())
    }
}

fn exact_scope() -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-{}", Uuid::now_v7()))
            .expect("non-empty workspace scope"),
    }
}

fn one_field_mismatches(
    scope: &ExactResourceScopeAttribution,
) -> Vec<(&'static str, ExactResourceScopeAttribution)> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access_space = scope.clone();
    access_space.access_space_id = AccessSpaceRef::mint();
    let mut workspace = scope.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new(format!("workspace-{}", Uuid::now_v7()))
            .expect("non-empty mismatched workspace scope");
    vec![
        ("owner_account_id", owner),
        ("actor_principal_id", actor),
        ("authenticated_session_id", session),
        ("access_space_id", access_space),
        ("workspace_id", workspace),
    ]
}

fn terminal_event(label: &str) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::Agent,
        Uuid::now_v7(),
        json!({"mt": "MT-020", "label": label}),
    )
}

fn non_terminal_event(label: &str) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::Diagnostic,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({"mt": "MT-020", "label": label}),
    )
}

fn event_json(event_id: &str, payload: &str) -> String {
    json!({"event_id": event_id, "payload": payload}).to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn outbox_identity(scope: &ExactResourceScopeAttribution, event_id: &str) -> String {
    sha256_hex(
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id,
            event_id
        )
        .as_bytes(),
    )
}

fn outbox_record_id(scope: &ExactResourceScopeAttribution, event_id: &str) -> String {
    format!("swarm-outbox-{}", outbox_identity(scope, event_id))
}

fn ledger_record_id(scope: &ExactResourceScopeAttribution, event_id: &str) -> String {
    format!("evt-swarm-terminal-{}", outbox_identity(scope, event_id))
}

async fn read_outbox(
    storage: &SurrealStorage,
    scope: &ExactResourceScopeAttribution,
    event_id: &str,
) -> Option<OutboxProbe> {
    let record_id = outbox_record_id(scope, event_id);
    storage
        .with_data_operation(|database| {
            Box::pin(async move { database.select_one(OUTBOX_TABLE, &record_id).await })
        })
        .await
        .expect("read exact outbox record")
}

async fn read_ledger(
    storage: &SurrealStorage,
    scope: &ExactResourceScopeAttribution,
    event_id: &str,
) -> Option<EventLedgerProbe> {
    let record_id = ledger_record_id(scope, event_id);
    storage
        .with_data_operation(|database| {
            Box::pin(async move { database.select_one(EVENT_LEDGER_TABLE, &record_id).await })
        })
        .await
        .expect("read exact canonical event receipt")
}

fn assert_exact_scope(scope: &ExactResourceScopeAttribution, row: &OutboxProbe) {
    assert_eq!(row.owner_account_id, scope.owner_account_id.to_string());
    assert_eq!(row.actor_principal_id, scope.actor_principal_id.to_string());
    assert_eq!(
        row.authenticated_session_id,
        scope.authenticated_session_id.to_string()
    );
    assert_eq!(row.access_space_id, scope.access_space_id.to_string());
    assert_eq!(row.workspace_id, scope.workspace_id.to_string());
}

fn assert_exact_ledger_scope(scope: &ExactResourceScopeAttribution, row: &EventLedgerProbe) {
    assert_eq!(row.owner_account_id, scope.owner_account_id.to_string());
    assert_eq!(row.actor_principal_id, scope.actor_principal_id.to_string());
    assert_eq!(
        row.authenticated_session_id,
        scope.authenticated_session_id.to_string()
    );
    assert_eq!(row.access_space_id, scope.access_space_id.to_string());
    assert_eq!(row.workspace_id, scope.workspace_id.to_string());
}

async fn cleanup(scope: &mut EmbeddedSurrealTestScope) {
    let receipt = scope.cleanup().await.expect("clean isolated Surreal scope");
    assert!(receipt.database_absent);
    assert!(receipt.namespace_absent_after_reopen);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_scope_enqueue_recovers_in_new_store_and_deletes_after_delivery() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let first_id = "event-001";
    let second_id = "event-002";
    let first_json = event_json(first_id, "first");
    let second_json = event_json(second_id, "second");

    let first_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    first_store
        .persist(first_id, first_json.clone(), 2)
        .await
        .expect("enqueue first exact-scope terminal event");
    first_store
        .persist(second_id, second_json.clone(), 2)
        .await
        .expect("enqueue second exact-scope terminal event");

    let outbox = read_outbox(&storage, &scope, first_id)
        .await
        .expect("first outbox row exists");
    let ledger = read_ledger(&storage, &scope, first_id)
        .await
        .expect("canonical receipt exists");
    assert_eq!(outbox.event_id, first_id);
    assert_eq!(outbox.event_json, first_json);
    assert_eq!(outbox.event_json_sha256, sha256_hex(outbox.event_json.as_bytes()));
    assert_eq!(outbox.storage_authority, "embedded_surrealdb");
    assert_eq!(outbox.attempts, 0);
    assert_eq!(
        outbox.event_ledger_event_id,
        RecordId::new(EVENT_LEDGER_TABLE, ledger_record_id(&scope, first_id))
    );
    assert_exact_scope(&scope, &outbox);
    assert_eq!(ledger.event_id, ledger_record_id(&scope, first_id));
    assert_eq!(ledger.aggregate_type, "swarm_terminal_event");
    assert_eq!(ledger.aggregate_id, first_id);
    assert_eq!(
        ledger.idempotency_key,
        format!("swarm-terminal:{}", outbox_identity(&scope, first_id))
    );
    assert_eq!(ledger.payload_hash, outbox.event_json_sha256);
    assert_eq!(ledger.source_component, "swarm_terminal_outbox");
    assert_exact_ledger_scope(&scope, &ledger);

    drop(first_store);
    let reopened_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let pending = reopened_store
        .next_pending()
        .await
        .expect("read pending after new store instance")
        .expect("first event remains pending");
    assert_eq!(pending.event_id, first_id);
    assert_eq!(pending.event_json, first_json);

    reopened_store
        .mark_delivered(first_id)
        .await
        .expect("delete first delivered row");
    assert!(read_outbox(&storage, &scope, first_id).await.is_none());
    assert!(read_ledger(&storage, &scope, first_id).await.is_some());
    assert_eq!(
        reopened_store
            .next_pending()
            .await
            .expect("read second pending row")
            .expect("second event remains pending")
            .event_id,
        second_id
    );
    reopened_store
        .mark_delivered(second_id)
        .await
        .expect("delete second delivered row");
    assert!(reopened_store
        .next_pending()
        .await
        .expect("read empty outbox")
        .is_none());

    drop(reopened_store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_scope_mismatch_and_incomplete_scope_fail_closed_without_mutation() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let event_id = "scope-protected-event";
    let original_json = event_json(event_id, "private");
    let exact_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    exact_store
        .persist(event_id, original_json.clone(), 8)
        .await
        .expect("enqueue protected event");

    for (dimension, mismatch) in one_field_mismatches(&scope) {
        let foreign_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), mismatch);
        assert!(
            foreign_store
                .next_pending()
                .await
                .unwrap_or_else(|error| panic!("{dimension} mismatch read failed: {error}"))
                .is_none(),
            "{dimension} mismatch disclosed a pending event"
        );
        foreign_store
            .record_failure(event_id, "foreign mutation")
            .await
            .unwrap_or_else(|error| panic!("{dimension} mismatch failure update failed: {error}"));
        foreign_store
            .mark_delivered(event_id)
            .await
            .unwrap_or_else(|error| panic!("{dimension} mismatch delete failed: {error}"));

        let pending = exact_store
            .next_pending()
            .await
            .expect("exact reader remains authorized")
            .expect("foreign mutation must not remove exact row");
        assert_eq!(pending.event_id, event_id, "{dimension} mismatch retargeted row");
        assert_eq!(pending.event_json, original_json);
        let row = read_outbox(&storage, &scope, event_id)
            .await
            .expect("exact row survives mismatch");
        assert_eq!(row.attempts, 0, "{dimension} mismatch mutated attempts");
        assert_eq!(row.event_json, original_json);
    }

    let incomplete = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint());
    assert!(matches!(
        SurrealSwarmOutboxStore::new(storage.clone(), incomplete),
        Err(SurrealSwarmOutboxError::IncompleteScope)
    ));
    assert_eq!(
        exact_store
            .next_pending()
            .await
            .expect("exact row remains readable after incomplete constructor denial")
            .expect("incomplete constructor must not mutate")
            .event_id,
        event_id
    );
    assert!(read_ledger(&storage, &scope, event_id).await.is_some());

    exact_store
        .mark_delivered(event_id)
        .await
        .expect("clean protected event");
    drop(exact_store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identical_retry_is_stable_conflict_and_capacity_preserve_order_and_receipts() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let first_id = "ordered-001";
    let second_id = "ordered-002";
    let rejected_id = "ordered-003";
    let first_json = event_json(first_id, "stable");
    let second_json = event_json(second_id, "second");

    store
        .persist(first_id, first_json.clone(), 2)
        .await
        .expect("initial persist");
    store
        .persist(first_id, first_json.clone(), 2)
        .await
        .expect("identical queued retry is idempotent");
    assert!(matches!(
        store
            .persist(first_id, event_json(first_id, "conflict"), 2)
            .await,
        Err(SurrealSwarmOutboxError::IdempotencyConflict)
    ));
    store
        .persist(second_id, second_json, 2)
        .await
        .expect("second capacity slot");
    assert!(matches!(
        store
            .persist(rejected_id, event_json(rejected_id, "overflow"), 2)
            .await,
        Err(SurrealSwarmOutboxError::CapacityExceeded { capacity: 2 })
    ));
    assert!(read_outbox(&storage, &scope, rejected_id).await.is_none());
    assert!(read_ledger(&storage, &scope, rejected_id).await.is_none());

    assert_eq!(
        store
            .next_pending()
            .await
            .expect("read first ordered event")
            .expect("first ordered event exists")
            .event_id,
        first_id
    );
    store
        .mark_delivered(first_id)
        .await
        .expect("deliver first ordered event");
    assert_eq!(
        store
            .next_pending()
            .await
            .expect("read second ordered event")
            .expect("second ordered event exists")
            .event_id,
        second_id
    );
    store
        .mark_delivered(second_id)
        .await
        .expect("deliver second ordered event");

    store
        .persist(first_id, first_json.clone(), 2)
        .await
        .expect("identical committed retry is idempotent");
    assert!(store
        .next_pending()
        .await
        .expect("committed retry does not recreate outbox row")
        .is_none());
    assert!(matches!(
        store
            .persist(first_id, event_json(first_id, "late-conflict"), 2)
            .await,
        Err(SurrealSwarmOutboxError::IdempotencyConflict)
    ));
    let receipt = read_ledger(&storage, &scope, first_id)
        .await
        .expect("original canonical receipt survives retries");
    assert_eq!(receipt.payload_hash, sha256_hex(first_json.as_bytes()));

    drop(store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bridge_terminal_events_are_acknowledged_ordered_and_drained_without_loss() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let recorder = Arc::new(CollectingRecorder::default());
    let recorder_port: Arc<dyn FlightRecorder> = recorder.clone();
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_surreal_outbox(recorder_port, store.clone(), 4);
    let first = terminal_event("first");
    let second = terminal_event("second");

    bridge
        .emit(first.clone())
        .expect("first terminal commit acknowledged");
    bridge
        .emit(second.clone())
        .expect("second terminal commit acknowledged");
    assert!(drain.drain_and_join(Duration::from_secs(5)).await);
    assert_eq!(bridge.dropped_count(), 0);
    assert!(!bridge.terminal_is_fenced());
    assert_eq!(recorder.event_ids(), vec![first.event_id, second.event_id]);
    assert!(store
        .next_pending()
        .await
        .expect("read drained outbox")
        .is_none());
    assert!(read_ledger(&storage, &scope, &first.event_id.to_string())
        .await
        .is_some());
    assert!(read_ledger(&storage, &scope, &second.event_id.to_string())
        .await
        .is_some());

    drop(bridge);
    drop(drain);
    drop(store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_terminal_event_recovers_through_new_store_and_healthy_shutdown_drain() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let first_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let event = terminal_event("recover-after-recorder-outage");
    let (degraded_bridge, degraded_drain) =
        DurableSwarmFrBridge::spawn_outbox_only(first_store.clone(), 4);

    degraded_bridge
        .emit(event.clone())
        .expect("terminal event acknowledged after embedded commit");
    assert!(!degraded_drain
        .drain_and_join(Duration::from_secs(5))
        .await);
    assert_eq!(
        first_store
            .next_pending()
            .await
            .expect("read retained terminal event")
            .expect("failed recorder delivery remains pending")
            .event_id,
        event.event_id.to_string()
    );

    drop(degraded_bridge);
    drop(degraded_drain);
    drop(first_store);
    let recovered_store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let recorder = Arc::new(CollectingRecorder::default());
    let recorder_port: Arc<dyn FlightRecorder> = recorder.clone();
    let (healthy_bridge, healthy_drain) = DurableSwarmFrBridge::spawn_with_surreal_outbox(
        recorder_port,
        recovered_store.clone(),
        4,
    );

    assert!(healthy_drain
        .drain_and_join(Duration::from_secs(5))
        .await);
    assert_eq!(recorder.event_ids(), vec![event.event_id]);
    assert!(recovered_store
        .next_pending()
        .await
        .expect("healthy restart drains retained event")
        .is_none());
    assert!(read_ledger(&storage, &scope, &event.event_id.to_string())
        .await
        .is_some());

    drop(healthy_bridge);
    drop(healthy_drain);
    drop(recovered_store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_storage_rejection_fences_terminal_producer_and_fails_shutdown_ack() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let store = SurrealSwarmOutboxStore::new_exact(storage.clone(), exact_scope());
    storage
        .shutdown()
        .await
        .expect("close storage before persistence fault");
    let recorder: Arc<dyn FlightRecorder> = Arc::new(CollectingRecorder::default());
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_surreal_outbox(recorder, store, 2);

    let first_error = bridge
        .emit(terminal_event("closed-storage"))
        .expect_err("closed embedded storage must reject terminal acknowledgement");
    assert!(first_error.contains("storage") || first_error.contains("closed"));
    assert!(bridge.terminal_is_fenced());
    let fenced_error = bridge
        .emit(terminal_event("fenced-after-failure"))
        .expect_err("later terminal event must be fenced");
    assert!(fenced_error.contains("fenced"));
    assert_eq!(bridge.dropped_count(), 1);
    assert!(!drain.drain_and_join(Duration::from_secs(5)).await);

    drop(bridge);
    drop(drain);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_timeout_is_negative_then_recovers_without_terminal_loss() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let scope = exact_scope();
    let store = SurrealSwarmOutboxStore::new_exact(storage.clone(), scope.clone());
    let recorder = Arc::new(BlockingRecorder::new());
    let recorder_port: Arc<dyn FlightRecorder> = recorder.clone();
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_surreal_outbox(recorder_port, store.clone(), 2);
    let event = terminal_event("blocked-shutdown");

    bridge
        .emit(event.clone())
        .expect("outbox commit precedes blocked recorder delivery");
    recorder.wait_until_entered().await;
    assert!(!drain.drain_and_join(Duration::from_millis(1)).await);
    assert!(read_outbox(&storage, &scope, &event.event_id.to_string())
        .await
        .is_some());

    recorder.release(1);
    assert!(drain.drain_and_join(Duration::from_secs(5)).await);
    assert_eq!(recorder.event_ids(), vec![event.event_id]);
    assert!(store
        .next_pending()
        .await
        .expect("terminal event deleted after recovered drain")
        .is_none());

    drop(bridge);
    drop(drain);
    drop(store);
    drop(storage);
    cleanup(&mut test_scope).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_non_terminal_capacity_reports_drop_and_preserves_accepted_order() {
    let mut test_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated Surreal scope");
    let storage = test_scope
        .activate_storage()
        .await
        .expect("activate production storage");
    let store = SurrealSwarmOutboxStore::new_exact(storage.clone(), exact_scope());
    let recorder = Arc::new(BlockingRecorder::new());
    let recorder_port: Arc<dyn FlightRecorder> = recorder.clone();
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_surreal_outbox(recorder_port, store, 1);
    let first = non_terminal_event("accepted-active");
    let second = non_terminal_event("accepted-queued");
    let rejected = non_terminal_event("rejected-full");

    bridge.emit(first.clone()).expect("first event accepted");
    recorder.wait_until_entered().await;
    bridge.emit(second.clone()).expect("second event queued");
    assert!(bridge.emit(rejected).is_err());
    assert_eq!(bridge.dropped_count(), 1);

    recorder.release(2);
    assert!(!drain.drain_and_join(Duration::from_secs(5)).await);
    assert_eq!(recorder.event_ids(), vec![first.event_id, second.event_id]);

    drop(bridge);
    drop(drain);
    drop(storage);
    cleanup(&mut test_scope).await;
}
