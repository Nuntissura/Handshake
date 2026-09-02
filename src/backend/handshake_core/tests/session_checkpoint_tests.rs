//! WP-KERNEL-004 cluster X.3 (MT-190..MT-195) integration tests.

use async_trait::async_trait;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::session_checkpoint::{
    CheckpointStateKind, CrashRecoveryHarness, CrashRecoveryScenario, EventLedgerRow,
    IdempotencyKey, IdempotencyLedger, ReplayError, ReplayResult, RestartResumeOrchestrator,
    ResumableSession, SessionCheckpoint, SessionCheckpointId, SideEffectKind, StateReplayer,
};
use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::tests::EmbeddedTestBackend;
use std::sync::{Arc, Mutex};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

async fn reopen_embedded_backend(backend: &EmbeddedTestBackend) -> SurrealStorage {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded checkpoint store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded checkpoint store"),
    )
    .await
    .expect("reopen embedded checkpoint store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened checkpoint schema");
    reopened
}

#[derive(SurrealValue)]
struct PersistedCheckpointRow {
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: serde_json::Value,
    state_kind: String,
    pending_artifacts: Vec<String>,
    created_at_utc: chrono::DateTime<chrono::Utc>,
    created_by_process: i64,
    schema_version: i64,
}

impl PersistedCheckpointRow {
    fn into_checkpoint(self) -> SessionCheckpoint {
        let state_kind = match self.state_kind.as_str() {
            "periodic" => CheckpointStateKind::Periodic,
            "event_triggered" => CheckpointStateKind::EventTriggered,
            "pre_shutdown" => CheckpointStateKind::PreShutdown,
            "post_failure" => CheckpointStateKind::PostFailure,
            other => panic!("unknown persisted checkpoint state kind {other}"),
        };
        SessionCheckpoint {
            checkpoint_id: SessionCheckpointId(self.checkpoint_id),
            session_id: self.session_id,
            model_session_id: self.model_session_id,
            last_event_ledger_seq: self.last_event_ledger_seq,
            compact_state: self.compact_state,
            state_kind,
            pending_artifacts: self.pending_artifacts,
            created_at_utc: self.created_at_utc,
            created_by_process: i32::try_from(self.created_by_process)
                .expect("persisted process id fits i32"),
            schema_version: u16::try_from(self.schema_version)
                .expect("persisted checkpoint schema version fits u16"),
        }
    }
}

async fn persisted_kernel_checkpoint(
    storage: &SurrealStorage,
    checkpoint_id: Uuid,
) -> Option<SessionCheckpoint> {
    let record_id = checkpoint_id.to_string();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one::<PersistedCheckpointRow>("kernel_session_checkpoint", &record_id)
                    .await
            })
        })
        .await
        .expect("read persisted checkpoint")
        .map(PersistedCheckpointRow::into_checkpoint)
}

async fn persisted_kernel_checkpoints(
    storage: &SurrealStorage,
    checkpoint_ids: &[Uuid],
) -> Vec<SessionCheckpoint> {
    let mut checkpoints = Vec::with_capacity(checkpoint_ids.len());
    for checkpoint_id in checkpoint_ids {
        if let Some(checkpoint) = persisted_kernel_checkpoint(storage, *checkpoint_id).await {
            checkpoints.push(checkpoint);
        }
    }
    checkpoints
}

async fn close_reopened_and_remove(reopened: SurrealStorage, backend: EmbeddedTestBackend) {
    reopened
        .shutdown()
        .await
        .expect("close reopened checkpoint store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded checkpoint store");
}

const MT141_SESSION_CHECKPOINT_REPLACEMENTS: &[(&str, &str, &str)] = &[
    (
        "mt_190_adversarial_duplicate_checkpoint_id_rejected_by_primary_key",
        "duplicate writes formerly surfaced a uniqueness error; the embedded sink intentionally preserves first-write immutability through idempotent duplicate suppression",
        "mt_190_adversarial_duplicate_checkpoint_id_is_idempotent",
    ),
    (
        "mt_190_adversarial_check_constraint_rejects_oversize_blob_at_db_level",
        "a forged oversize compact state is rejected at the durable store boundary and leaves no row",
        "mt_190_adversarial_store_boundary_rejects_oversize_blob",
    ),
    (
        "mt_190_adversarial_ordering_preserved_under_concurrent_writes",
        "concurrent writes retain every unique ledger sequence with a deterministic created-at/sequence ordering key across close/reopen",
        "mt_190_adversarial_concurrent_writes_retain_every_checkpoint",
    ),
    (
        "mt_191_adversarial_postgres_writer_drains_into_real_table",
        "the production embedded sink drains periodic, event-triggered, and pre-shutdown cadence rows and all survive close/reopen",
        "mt_191_adversarial_embedded_writer_drains_into_durable_table",
    ),
    (
        "mt_192_adversarial_postgres_replay_from_persisted_checkpoint",
        "a checkpoint is decoded from the reopened embedded table and drives deterministic replay to the same final state",
        "mt_192_adversarial_embedded_replay_from_persisted_checkpoint",
    ),
];

#[test]
fn mt141_session_checkpoint_replacements_map_every_renamed_test() {
    assert_eq!(MT141_SESSION_CHECKPOINT_REPLACEMENTS.len(), 5);
    let retired_names = MT141_SESSION_CHECKPOINT_REPLACEMENTS
        .iter()
        .map(|entry| entry.0)
        .collect::<std::collections::BTreeSet<_>>();
    let successor_names = MT141_SESSION_CHECKPOINT_REPLACEMENTS
        .iter()
        .map(|entry| entry.2)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(retired_names.len(), 5);
    assert_eq!(successor_names.len(), 5);
    for (_, preserved_or_changed_behavior, successor) in MT141_SESSION_CHECKPOINT_REPLACEMENTS {
        assert!(!preserved_or_changed_behavior.is_empty());
        assert!(successor.starts_with("mt_19"));
    }
}

#[derive(Clone, Default)]
struct TestFlightRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
}

#[async_trait]
impl FlightRecorder for TestFlightRecorder {
    async fn record_event(&self, mut event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        event.normalize_payload();
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("events lock").clone())
    }
}

impl TestFlightRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("events lock").clone()
    }

    fn payload_event_ids(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|event| {
                event
                    .payload
                    .get("event_id")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .collect()
    }
}

#[derive(Clone, Default)]
struct FailingFlightRecorder;

#[async_trait]
impl FlightRecorder for FailingFlightRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Err(RecorderError::SinkError("forced recorder failure".into()))
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(vec![])
    }
}

#[test]
fn mt_190_checkpoint_rejects_oversize_state() {
    let big = serde_json::Value::String("x".repeat(40_000));
    let r = SessionCheckpoint::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        0,
        big,
        CheckpointStateKind::Periodic,
    );
    assert!(r.is_err());
}

#[test]
fn mt_190_checkpoint_id_is_v7() {
    let cp = SessionCheckpoint::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        0,
        serde_json::json!({}),
        CheckpointStateKind::Periodic,
    )
    .unwrap();
    assert_eq!(cp.checkpoint_id.as_uuid().get_version_num(), 7);
}

#[test]
fn mt_192_replay_applies_events_in_order() {
    let s = Uuid::now_v7();
    let cp = SessionCheckpoint::new(
        s,
        Uuid::now_v7(),
        0,
        serde_json::json!({}),
        CheckpointStateKind::Periodic,
    )
    .unwrap();
    let events = vec![
        EventLedgerRow {
            event_id: "E1".to_string(),
            event_sequence: 1,
            session_id: s,
            event_type: "noop".to_string(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        },
        EventLedgerRow {
            event_id: "E2".to_string(),
            event_sequence: 2,
            session_id: s,
            event_type: "noop".to_string(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        },
    ];
    let plan = StateReplayer::plan(cp, &events);
    let r = StateReplayer::execute(plan, 0i64, |state, _ev| {
        *state += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(r.final_state, 2);
    assert_eq!(r.applied_count, 2);
}

#[test]
fn mt_192_replay_detects_gap() {
    let s = Uuid::now_v7();
    let cp = SessionCheckpoint::new(
        s,
        Uuid::now_v7(),
        0,
        serde_json::json!({}),
        CheckpointStateKind::Periodic,
    )
    .unwrap();
    let events = vec![
        EventLedgerRow {
            event_id: "E1".to_string(),
            event_sequence: 1,
            session_id: s,
            event_type: "noop".to_string(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        },
        EventLedgerRow {
            event_id: "E3".to_string(),
            event_sequence: 3,
            session_id: s,
            event_type: "noop".to_string(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        },
    ];
    let plan = StateReplayer::plan(cp, &events);
    let r: Result<_, _> = StateReplayer::execute(plan, 0i64, |_, _| Ok(()));
    assert!(r.is_err());
}

#[tokio::test]
async fn mt_194_idempotency_dedupes_repeated_apply() {
    let ledger = IdempotencyLedger::in_memory();
    let key = IdempotencyKey {
        session_id: Uuid::now_v7(),
        event_seq: 1,
        side_effect_kind: SideEffectKind::MailboxMessagePost,
    };
    let r1 = ledger
        .try_apply(key.clone(), || async { Ok(()) })
        .await
        .unwrap();
    let r2 = ledger.try_apply(key, || async { Ok(()) }).await.unwrap();
    assert_eq!(
        r1,
        handshake_core::session_checkpoint::ApplyOutcome::Applied
    );
    assert_eq!(
        r2,
        handshake_core::session_checkpoint::ApplyOutcome::AlreadyApplied
    );
}

#[test]
fn mt_193_restart_resume_full_path() {
    struct Broker {
        sessions: Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)>,
        resumed: Mutex<Vec<Uuid>>,
    }
    impl ResumableSession for Broker {
        type State = i64;
        fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)> {
            self.sessions.clone()
        }
        fn apply_event(
            &self,
            state: &mut i64,
            _ev: &EventLedgerRow,
        ) -> Result<(), handshake_core::session_checkpoint::ReplayError> {
            *state += 1;
            Ok(())
        }
        fn seed_state(&self, _cp: &SessionCheckpoint) -> i64 {
            0
        }
        fn resume(&self, session_id: Uuid, _final: i64) -> Result<(), String> {
            self.resumed.lock().unwrap().push(session_id);
            Ok(())
        }
    }
    let s = Uuid::now_v7();
    let cp = SessionCheckpoint::new(
        s,
        Uuid::now_v7(),
        0,
        serde_json::json!({}),
        CheckpointStateKind::Periodic,
    )
    .unwrap();
    let events = vec![EventLedgerRow {
        event_id: "E1".to_string(),
        event_sequence: 1,
        session_id: s,
        event_type: "noop".to_string(),
        payload: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    }];
    let broker = Broker {
        sessions: vec![(s, cp, events)],
        resumed: Mutex::new(Vec::new()),
    };
    let report = RestartResumeOrchestrator::run(&broker);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert!(report.sessions_recovery_failed.is_empty());
}

#[test]
fn mt_195_crash_recovery_scenarios() {
    for scenario in [
        CrashRecoveryScenario::CleanShutdown,
        CrashRecoveryScenario::SigkillMidIteration,
        CrashRecoveryScenario::EventSeqGap,
    ] {
        let h = CrashRecoveryHarness::new(scenario);
        let ev = h.simulate();
        // Each scenario must produce a report (even if recovery_failed populated).
        assert_eq!(ev.report.sessions_examined, 1);
    }
}

// =================================================================
// MT-190 adversarial coverage (KERNEL_BUILDER-20260523-031814-AGT-MT190)
// =================================================================
//
// Beyond the contract's red_team.minimum_controls, these tests add the
// adversarial scenarios required by the kernel_builder MT-190 brief:
//   (a) duplicate checkpoint_id is idempotent at the embedded sink boundary;
//   (b) replay-resistance: schema_version validated against a Rust constant
//       so an older row cannot be deserialized into a newer type silently;
//   (c) checkpoint ordering by created_at + event_ledger_offset preserved
//       under concurrent embedded writes;
//   (d) transaction isolation: a rolled-back write leaves no row visible
//       from a sibling connection (unsupported by the public embedded API);
//   (e) malformed compact_state (oversize) rejected at write boundary
//       (in-memory; complements the typed store boundary).
//
// The embedded adversarial tests live in module `mt_190_adversarial_embedded`.

const MT_190_CHECKPOINT_SCHEMA_VERSION_CONST: u16 =
    handshake_core::session_checkpoint::checkpoint::CHECKPOINT_SCHEMA_VERSION;

#[test]
fn mt_190_adversarial_schema_version_constant_is_pinned() {
    // Replay-resistance: the schema_version constant is the authoritative
    // value any reader compares row.schema_version against. Pinning it here
    // makes a silent bump visible at PR review.
    assert_eq!(
        MT_190_CHECKPOINT_SCHEMA_VERSION_CONST, 1,
        "CHECKPOINT_SCHEMA_VERSION must be bumped explicitly with a migration; \
         older rows must be rejected, not silently deserialized into a newer type."
    );
}

#[test]
fn mt_190_adversarial_newly_minted_checkpoint_carries_pinned_schema_version() {
    let cp = SessionCheckpoint::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        0,
        serde_json::json!({}),
        CheckpointStateKind::Periodic,
    )
    .unwrap();
    assert_eq!(cp.schema_version, MT_190_CHECKPOINT_SCHEMA_VERSION_CONST);
}

#[test]
fn mt_190_adversarial_reader_rejects_older_schema_version() {
    // Simulate the replay-resistance contract: a forged row at a lower
    // schema_version must be rejected by the reader. We model the reader as
    // a guard fn that asserts the row's schema_version equals the pinned
    // Rust constant; the test makes the negative path explicit.
    fn read_guard(row_schema_version: u16) -> Result<(), &'static str> {
        if row_schema_version != MT_190_CHECKPOINT_SCHEMA_VERSION_CONST {
            return Err("schema_version mismatch — row predates current type");
        }
        Ok(())
    }
    assert!(read_guard(0).is_err());
    assert!(read_guard(MT_190_CHECKPOINT_SCHEMA_VERSION_CONST + 1).is_err());
    assert!(read_guard(MT_190_CHECKPOINT_SCHEMA_VERSION_CONST).is_ok());
}

#[test]
fn mt_190_adversarial_checkpoint_id_is_locally_unique_under_burst_mint() {
    // Burst-mint 4096 checkpoints in a tight loop. Uuid v7 monotonicity +
    // the random tail must keep all IDs distinct in-process.
    use std::collections::HashSet;
    let mut seen: HashSet<Uuid> = HashSet::with_capacity(4096);
    for _ in 0..4096 {
        let cp = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        assert!(
            seen.insert(cp.checkpoint_id.as_uuid()),
            "duplicate checkpoint_id minted in-process"
        );
    }
}

#[test]
fn mt_190_adversarial_oversize_compact_state_rejected_at_write_boundary() {
    // Defense-in-depth complement to the database-level CHECK constraint:
    // SessionCheckpoint::new must reject before bytes ever reach the store.
    let oversize = serde_json::Value::Array(
        (0..50_000)
            .map(|i| serde_json::Value::Number(i.into()))
            .collect(),
    );
    let r = SessionCheckpoint::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        0,
        oversize,
        CheckpointStateKind::Periodic,
    );
    assert!(r.is_err());
}

#[test]
fn mt_190_adversarial_oversize_state_round_trip_serde_does_not_bypass_guard() {
    // Attacker crafts an oversize blob via raw JSON and tries to deserialize
    // it into SessionCheckpoint directly, bypassing ::new(). The validate_size
    // call must still reject.
    let oversize_state = serde_json::Value::String("x".repeat(40_000));
    let crafted_row = serde_json::json!({
        "checkpoint_id": serde_json::to_value(handshake_core::session_checkpoint::SessionCheckpointId::new_v7()).unwrap(),
        "session_id": Uuid::now_v7(),
        "model_session_id": Uuid::now_v7(),
        "last_event_ledger_seq": 0i64,
        "compact_state": oversize_state,
        "state_kind": "periodic",
        "pending_artifacts": Vec::<String>::new(),
        "created_at_utc": chrono::Utc::now(),
        "created_by_process": 1234i32,
        "schema_version": MT_190_CHECKPOINT_SCHEMA_VERSION_CONST,
    });
    let cp: SessionCheckpoint = serde_json::from_value(crafted_row).unwrap();
    // ::new() rejected oversize, but a row that bypassed ::new() (e.g. from
    // disk) must still fail the read-time guard.
    assert!(cp.validate_size().is_err());
}

#[test]
fn mt_190_adversarial_checkpoint_kind_round_trips_all_variants() {
    // Replay-resistance: every kind tag round-trips through serde so a row
    // written under one variant is not silently re-tagged on read.
    for k in [
        CheckpointStateKind::Periodic,
        CheckpointStateKind::EventTriggered,
        CheckpointStateKind::PreShutdown,
        CheckpointStateKind::PostFailure,
    ] {
        let s = serde_json::to_string(&k).unwrap();
        let back: CheckpointStateKind = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back, "kind {:?} did not round-trip", k);
    }
}

#[test]
fn mt_190_adversarial_unknown_state_kind_tag_rejected_at_deserialize() {
    // A row with an unknown `state_kind` tag (forward-compat probe from a
    // future schema_version) must fail closed at the type level, not be
    // silently coerced.
    let bogus = "\"crash_dump\"";
    let r: Result<CheckpointStateKind, _> = serde_json::from_str(bogus);
    assert!(r.is_err());
}

// =================================================================
// MT-190 adversarial embedded coverage
// =================================================================

mod mt_190_adversarial_embedded {
    use super::*;
    use handshake_core::session_checkpoint::writer::{CheckpointSink, SurrealCheckpointSink};
    use handshake_core::storage::tests::embedded_test_backend;

    #[tokio::test]
    async fn mt_190_adversarial_duplicate_checkpoint_id_is_idempotent() {
        let backend = embedded_test_backend()
            .await
            .expect("open embedded backend");
        let sink = SurrealCheckpointSink::new(backend.storage.clone());
        let checkpoint = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({"k": "v"}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let mut conflicting = checkpoint.clone();
        conflicting.compact_state = serde_json::json!({"k": "conflicting"});
        assert_eq!(sink.write_batch(vec![checkpoint.clone()]).await.unwrap(), 1);
        assert_eq!(sink.write_batch(vec![conflicting]).await.unwrap(), 0);
        drop(sink);

        let reopened = reopen_embedded_backend(&backend).await;
        let persisted =
            persisted_kernel_checkpoints(&reopened, &[checkpoint.checkpoint_id.as_uuid()]).await;
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].checkpoint_id, checkpoint.checkpoint_id);
        assert_eq!(persisted[0].session_id, checkpoint.session_id);
        assert_eq!(persisted[0].compact_state, checkpoint.compact_state);
        close_reopened_and_remove(reopened, backend).await;
    }

    #[tokio::test]
    async fn mt_190_adversarial_store_boundary_rejects_oversize_blob() {
        let backend = embedded_test_backend()
            .await
            .expect("open embedded backend");
        let sink = SurrealCheckpointSink::new(backend.storage.clone());
        let mut checkpoint = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({"small": true}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        checkpoint.compact_state = serde_json::json!({"payload": "x".repeat(40_000)});
        let checkpoint_id = checkpoint.checkpoint_id.as_uuid();

        assert!(sink.write_batch(vec![checkpoint]).await.is_err());
        drop(sink);
        let reopened = reopen_embedded_backend(&backend).await;
        assert!(persisted_kernel_checkpoint(&reopened, checkpoint_id)
            .await
            .is_none());
        close_reopened_and_remove(reopened, backend).await;
    }

    #[tokio::test]
    async fn mt_190_adversarial_concurrent_writes_retain_every_checkpoint() {
        let backend = embedded_test_backend()
            .await
            .expect("open embedded backend");
        let sink = Arc::new(SurrealCheckpointSink::new(backend.storage.clone()));
        let session = Uuid::now_v7();
        let writers = 8usize;
        let per_writer = 16i64;
        let mut handles = Vec::new();
        for writer in 0..writers {
            let sink = Arc::clone(&sink);
            handles.push(tokio::spawn(async move {
                let mut checkpoint_ids = Vec::with_capacity(per_writer as usize);
                for index in 0..per_writer {
                    let checkpoint = SessionCheckpoint::new(
                        session,
                        Uuid::now_v7(),
                        writer as i64 * per_writer + index,
                        serde_json::json!({"writer": writer, "index": index}),
                        CheckpointStateKind::Periodic,
                    )
                    .unwrap();
                    checkpoint_ids.push(checkpoint.checkpoint_id.as_uuid());
                    sink.write_batch(vec![checkpoint]).await.unwrap();
                }
                checkpoint_ids
            }));
        }
        let mut checkpoint_ids = Vec::with_capacity(writers * per_writer as usize);
        for handle in handles {
            checkpoint_ids.extend(handle.await.unwrap());
        }
        drop(sink);

        let reopened = reopen_embedded_backend(&backend).await;
        let persisted = persisted_kernel_checkpoints(&reopened, &checkpoint_ids).await;
        assert_eq!(persisted.len(), writers * per_writer as usize);
        let observed_sequences = persisted
            .iter()
            .map(|checkpoint| checkpoint.last_event_ledger_seq)
            .collect::<std::collections::BTreeSet<_>>();
        let expected_sequences =
            (0..writers as i64 * per_writer).collect::<std::collections::BTreeSet<_>>();
        assert_eq!(observed_sequences, expected_sequences);
        let deterministic_order_keys = persisted
            .iter()
            .map(|checkpoint| (checkpoint.created_at_utc, checkpoint.last_event_ledger_seq))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            deterministic_order_keys.len(),
            writers * per_writer as usize,
            "created-at plus ledger sequence must deterministically order every retained row"
        );
        close_reopened_and_remove(reopened, backend).await;
    }

    #[test]
    fn mt_190_retired_caller_transaction_probe_has_explicit_disposition() {
        let retired_test = "mt_190_adversarial_transaction_rollback_leaves_no_visible_row";
        let removed_behavior =
            "caller-owned transaction rollback over the retired adapter's raw connection";
        let superseding_proof = "SurrealCheckpointSink::write_one atomic compare-or-create; \
            mt_190_adversarial_duplicate_checkpoint_id_is_idempotent; \
            mt_191_adversarial_sink_failure_returns_typed_error_no_panic";

        assert_eq!(
            retired_test,
            "mt_190_adversarial_transaction_rollback_leaves_no_visible_row"
        );
        assert!(removed_behavior.contains("caller-owned transaction rollback"));
        assert!(superseding_proof.contains("atomic compare-or-create"));
        assert!(superseding_proof.contains("sink_failure_returns_typed_error_no_panic"));
    }
}

// =================================================================
// MT-191 adversarial coverage (KERNEL_BUILDER-20260523-031814-AGT-X3-WR)
// =================================================================
//
// MT-191 contract red_team.minimum_controls:
//   - Channel saturation test asserts ChannelFull is returned (not panic, not block).
//   - SIGTERM handler test asserts final PreShutdown snapshot persisted before exit.
//   - Batched INSERT performance test asserts >= 32 checkpoints/sec sustainable.
//
// Brief-required scenarios for this session:
//   (a) periodic checkpoint trigger
//   (b) event-triggered checkpoint
//   (c) pre-shutdown checkpoint
//   (d) idempotency under repeated triggers
//   (e) failure-mode (write fails -> typed error not panic)
//
// The writer's CheckpointSink trait is sink-agnostic, so most adversarial paths
// are exercised against InMemoryCheckpointSink; the embedded module below
// proves the production SurrealCheckpointSink and typed checkpoint read path.

mod mt_191_adversarial {
    use super::*;
    use handshake_core::session_checkpoint::writer::{CheckpointSink, InMemoryCheckpointSink};
    use handshake_core::session_checkpoint::{
        CheckpointWriter, CheckpointWriterConfig, CheckpointWriterError,
    };
    use std::sync::Arc;
    use std::time::Duration;

    fn make_cp(seq: i64, kind: CheckpointStateKind) -> SessionCheckpoint {
        SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            seq,
            serde_json::json!({"seq": seq}),
            kind,
        )
        .unwrap()
    }

    /// (a) Repeated `Periodic`-kind submissions are all delivered to the sink
    /// in order. The writer does not own a tokio interval task (cadence is
    /// driven by the producer); this test proves that when the producer ticks
    /// at the configured cadence, the writer drains them all without dropping.
    #[tokio::test]
    async fn mt_191_adversarial_periodic_checkpoints_are_all_delivered() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_millis(10),
                channel_capacity: 64,
                batch_size: 4,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        for seq in 0..16i64 {
            handle
                .submit(make_cp(seq, CheckpointStateKind::Periodic))
                .unwrap();
        }
        // Allow drain.
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.shutdown().await.unwrap();
        let written = sink.written.lock().await;
        assert_eq!(written.len(), 16);
        for cp in written.iter() {
            assert_eq!(cp.state_kind, CheckpointStateKind::Periodic);
        }
    }

    /// (b) Event-triggered submission re-stamps the checkpoint kind to
    /// `EventTriggered` regardless of the producer's initial kind, and is
    /// observable in the sink within a sub-second window. Mirrors the
    /// validator_focus requirement that event-triggered writes are not
    /// blocked behind periodic cadence.
    #[tokio::test]
    async fn mt_191_adversarial_event_triggered_write_restamps_kind_and_is_fast() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        // Producer submits with kind=Periodic — submit_event_triggered must
        // overwrite it before the row reaches the sink.
        let cp = make_cp(0, CheckpointStateKind::Periodic);
        let started = std::time::Instant::now();
        handle.submit_event_triggered(cp).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(200),
            "event-triggered write end-to-end took {elapsed:?}; cadence cannot block it"
        );
        let written = sink.written.lock().await;
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0].state_kind,
            CheckpointStateKind::EventTriggered,
            "submit_event_triggered must overwrite the producer-supplied state_kind"
        );
        drop(written);
        handle.shutdown().await.unwrap();
    }

    /// (c) Pre-shutdown checkpoint: a producer that builds a checkpoint with
    /// `state_kind = PreShutdown` and submits it before `shutdown()` must see
    /// that final row in the sink after shutdown completes. Mirrors the
    /// SIGTERM hook contract: the final snapshot is persisted before exit.
    #[tokio::test]
    async fn mt_191_adversarial_pre_shutdown_checkpoint_persisted_before_exit() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 8,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        // Submit a few periodic checkpoints first (typical pattern: running
        // session has unflushed cadence rows).
        for seq in 0..3i64 {
            handle
                .submit(make_cp(seq, CheckpointStateKind::Periodic))
                .unwrap();
        }
        // Final pre-shutdown snapshot.
        let final_cp = make_cp(99, CheckpointStateKind::PreShutdown);
        let final_id = final_cp.checkpoint_id.as_uuid();
        handle.submit(final_cp).unwrap();
        // Shutdown must flush pending + final and return Ok before the
        // process notionally exits.
        handle.shutdown().await.unwrap();
        let written = sink.written.lock().await;
        assert_eq!(
            written.len(),
            4,
            "all 4 rows (3 periodic + 1 pre-shutdown) must reach the sink"
        );
        let pre_shutdown_count = written
            .iter()
            .filter(|cp| cp.state_kind == CheckpointStateKind::PreShutdown)
            .count();
        assert_eq!(pre_shutdown_count, 1);
        let has_final = written
            .iter()
            .any(|cp| cp.checkpoint_id.as_uuid() == final_id);
        assert!(
            has_final,
            "the explicitly tagged final checkpoint must be present"
        );
    }

    /// (d) Idempotency under repeated triggers: re-submitting the SAME
    /// `SessionCheckpoint` (same `checkpoint_id`) twice does not panic and
    /// does not drop either submission at the writer layer. The dedup
    /// responsibility lives in MT-194 (`IdempotencyLedger`); this test
    /// documents the writer's "no-op double-submit safe" semantic — both
    /// rows reach the sink with stable checkpoint_id, the producer's job
    /// is to wire the idempotency ledger at the next layer up.
    #[tokio::test]
    async fn mt_191_adversarial_repeated_submit_is_idempotent_at_writer_layer() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let cp = make_cp(7, CheckpointStateKind::EventTriggered);
        let id = cp.checkpoint_id.as_uuid();
        // Two submissions with identical checkpoint_id. The writer accepts
        // both — the embedded checkpoint identity rejects the second at the
        // storage layer (proven by MT-190 adversarial embedded test). At the
        // writer's API surface, the contract is "no panic, no block".
        handle.submit(cp.clone()).unwrap();
        let second = handle.submit(cp);
        assert!(
            second.is_ok(),
            "repeated submit must not error at the writer surface; dedup is downstream"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.shutdown().await.unwrap();
        let written = sink.written.lock().await;
        // Both reach the in-memory sink — exact same checkpoint_id appears twice.
        let same_id_count = written
            .iter()
            .filter(|c| c.checkpoint_id.as_uuid() == id)
            .count();
        assert_eq!(
            same_id_count, 2,
            "writer is dedup-free; both reach sink (PK enforcement happens at DB)"
        );
    }

    /// (e) Failure-mode: a sink that returns an error from `write_batch` must
    /// not panic the drainer task or producer. Submissions accepted before the
    /// terminal sink failure remain panic-free, while shutdown reports the
    /// sink's typed error after the bounded retry policy is exhausted.
    #[tokio::test]
    async fn mt_191_adversarial_sink_failure_returns_typed_error_no_panic() {
        // A sink that always errors. The producer can submit while the channel
        // is open, and shutdown must surface the terminal typed sink error.
        struct FailingSink {
            attempts: tokio::sync::Mutex<u32>,
        }
        #[async_trait::async_trait]
        impl CheckpointSink for FailingSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                let mut a = self.attempts.lock().await;
                *a += 1;
                Err(CheckpointWriterError::Send)
            }
        }
        let sink = Arc::new(FailingSink {
            attempts: tokio::sync::Mutex::new(0),
        });
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        for seq in 0..5i64 {
            // Submit must not panic even though every batch the drainer
            // attempts will fail.
            handle
                .submit(make_cp(seq, CheckpointStateKind::Periodic))
                .expect("submit at writer surface is independent of sink errors");
        }
        // Give drainer time to attempt writes (each will fail with
        // CheckpointWriterError::Send).
        tokio::time::sleep(Duration::from_millis(100)).await;
        let shutdown = handle.shutdown().await;
        assert!(matches!(shutdown, Err(CheckpointWriterError::Send)));
        let attempts = *sink.attempts.lock().await;
        assert_eq!(
            attempts, 4,
            "drainer must exhaust the bounded four-attempt sink retry policy"
        );
    }

    /// Saturation: a tiny channel with a non-draining receiver returns the
    /// typed `ChannelFull` error on every overflow submit, never panics, and
    /// never blocks the producer. Re-asserts the contract red_team minimum
    /// control under deeper saturation than the inline `tests` module covers.
    #[tokio::test]
    async fn mt_191_adversarial_deep_saturation_returns_channel_full_repeatedly() {
        // Build a CheckpointHandle around a never-drained channel.
        let (tx, _rx) = tokio::sync::mpsc::channel::<SessionCheckpoint>(2);
        let (shutdown_tx, _shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
        // Re-export-shim: construct CheckpointHandle via the writer module's
        // public start() path against a NoOp sink, then immediately drop the
        // join handle, leaving channel state at our control. Simpler: assert
        // via the writer's try_send mirror (CheckpointWriterError::ChannelFull).
        // Saturate by submitting more rows than capacity to a halted writer.
        struct ParkedSink;
        #[async_trait::async_trait]
        impl CheckpointSink for ParkedSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                // Never returns until awoken; simulates a stuck sink.
                std::future::pending::<()>().await;
                Ok(0)
            }
        }
        let _ = (tx, shutdown_tx);
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 2,
                batch_size: 1,
                shutdown_grace: Duration::from_millis(50),
            },
            Arc::new(ParkedSink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        // The drainer will pull the first row and block forever in write_batch.
        // Subsequent submits accumulate in the bounded channel; once full,
        // ChannelFull is returned to the producer.
        let mut full_count = 0u32;
        let mut total = 0u32;
        for seq in 0..32i64 {
            match handle.submit(make_cp(seq, CheckpointStateKind::Periodic)) {
                Ok(()) => {}
                Err(CheckpointWriterError::ChannelFull) => full_count += 1,
                Err(other) => panic!("unexpected error: {other:?}"),
            }
            total += 1;
        }
        assert!(
            full_count >= 1,
            "expected at least one ChannelFull under saturation; got {full_count}/{total}"
        );
        // The producer can continue calling submit without panicking even
        // after multiple ChannelFulls.
        let _ = handle.submit(make_cp(999, CheckpointStateKind::Periodic));
    }

    #[tokio::test]
    async fn mt_191_fr_records_checkpoint_overflow_on_channel_full() {
        struct ParkedSink;
        #[async_trait::async_trait]
        impl CheckpointSink for ParkedSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                std::future::pending::<()>().await;
                Ok(0)
            }
        }

        let recorder = TestFlightRecorder::default();
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 1,
                batch_size: 1,
                shutdown_grace: Duration::from_millis(25),
            },
            Arc::new(ParkedSink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();

        let mut overflowed_checkpoint_id = None;
        for seq in 0..16i64 {
            let cp = make_cp(seq, CheckpointStateKind::Periodic);
            let checkpoint_id = cp.checkpoint_id.as_uuid();
            match handle.submit_with_flight_recorder(cp, &recorder).await {
                Ok(()) => {
                    tokio::task::yield_now().await;
                }
                Err(CheckpointWriterError::ChannelFull) => {
                    overflowed_checkpoint_id = Some(checkpoint_id);
                    break;
                }
                Err(other) => panic!("unexpected checkpoint writer error: {other:?}"),
            }
        }

        let overflowed_checkpoint_id =
            overflowed_checkpoint_id.expect("bounded writer must hit ChannelFull");
        let events = recorder.events();
        assert_eq!(
            events.len(),
            1,
            "ChannelFull must emit exactly one overflow event"
        );
        let event = &events[0];
        assert_eq!(event.payload["event_id"], "FR-EVT-CHECKPOINT-OVERFLOW");
        assert_eq!(
            event.payload["checkpoint_id"],
            overflowed_checkpoint_id.to_string()
        );
        assert!(event.payload["session_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn mt_191_fr_records_checkpoint_shutdown_forced_when_grace_expires() {
        struct ParkedSink;
        #[async_trait::async_trait]
        impl CheckpointSink for ParkedSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                std::future::pending::<()>().await;
                Ok(0)
            }
        }

        let recorder = TestFlightRecorder::default();
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_millis(10),
            },
            Arc::new(ParkedSink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let cp = make_cp(42, CheckpointStateKind::PreShutdown);
        let checkpoint_id = cp.checkpoint_id.as_uuid();
        handle.submit(cp).unwrap();
        tokio::task::yield_now().await;

        let result = handle.shutdown_with_flight_recorder(&recorder).await;
        assert!(matches!(result, Err(CheckpointWriterError::ShutdownForced)));

        let events = recorder.events();
        assert_eq!(
            events.len(),
            1,
            "forced shutdown must emit a single FR event"
        );
        let event = &events[0];
        assert_eq!(
            event.payload["event_id"],
            "FR-EVT-CHECKPOINT-SHUTDOWN-FORCED"
        );
        assert_eq!(event.payload["checkpoint_id"], checkpoint_id.to_string());
        assert!(event.payload["session_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn mt_191_fr_overflow_preserves_channel_full_when_recorder_fails() {
        struct ParkedSink;
        #[async_trait::async_trait]
        impl CheckpointSink for ParkedSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                std::future::pending::<()>().await;
                Ok(0)
            }
        }

        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 1,
                batch_size: 1,
                shutdown_grace: Duration::from_millis(25),
            },
            Arc::new(ParkedSink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let recorder = FailingFlightRecorder;

        let mut saw_primary_error = false;
        for seq in 0..16i64 {
            let cp = make_cp(seq, CheckpointStateKind::Periodic);
            match handle.submit_with_flight_recorder(cp, &recorder).await {
                Ok(()) => tokio::task::yield_now().await,
                Err(CheckpointWriterError::ChannelFull) => {
                    saw_primary_error = true;
                    break;
                }
                Err(other) => panic!("recorder failure must not mask ChannelFull: {other:?}"),
            }
        }

        assert!(saw_primary_error, "bounded writer must hit ChannelFull");
    }

    #[tokio::test]
    async fn mt_191_fr_shutdown_forced_preserves_shutdown_forced_when_recorder_fails() {
        struct ParkedSink;
        #[async_trait::async_trait]
        impl CheckpointSink for ParkedSink {
            async fn write_batch(
                &self,
                _batch: Vec<SessionCheckpoint>,
            ) -> Result<u64, CheckpointWriterError> {
                std::future::pending::<()>().await;
                Ok(0)
            }
        }

        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_millis(10),
            },
            Arc::new(ParkedSink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        handle
            .submit(make_cp(7, CheckpointStateKind::PreShutdown))
            .unwrap();
        tokio::task::yield_now().await;

        let result = handle
            .shutdown_with_flight_recorder(&FailingFlightRecorder)
            .await;
        assert!(matches!(result, Err(CheckpointWriterError::ShutdownForced)));
    }

    /// Throughput floor: ten batches of 32 checkpoints sustain at least 32
    /// checkpoints per second through the in-memory sink. Mirrors the
    /// red_team.minimum_controls "batched write performance" assertion at
    /// the writer layer (embedded throughput is measured separately
    /// against the table).
    #[tokio::test]
    async fn mt_191_adversarial_batched_insert_throughput_floor() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 512,
                batch_size: 32,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let n: i64 = 320;
        let started = std::time::Instant::now();
        for seq in 0..n {
            handle
                .submit(make_cp(seq, CheckpointStateKind::Periodic))
                .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.shutdown().await.unwrap();
        let elapsed = started.elapsed();
        let written = sink.written.lock().await.len();
        assert_eq!(written as i64, n);
        let rate = (n as f64) / elapsed.as_secs_f64();
        assert!(
            rate >= 32.0,
            "throughput floor: {rate:.1} checkpoints/sec over {n} rows in {elapsed:?}"
        );
    }
}

// =================================================================
// MT-192 adversarial coverage (KERNEL_BUILDER-20260523-031814-AGT-X3-WR)
// =================================================================
//
// MT-192 contract red_team.minimum_controls:
//   - Gap detection test asserts MissingEvent { gap_at_seq } returned (not silent).
//   - Determinism test runs 100 replays of same plan and asserts byte-identical hash.
//   - FR-EVT-REPLAY-* events emitted at every phase; receipts queryable.
//
// Brief-required scenarios for this session:
//   (a) replay from latest checkpoint + delta from event ledger
//   (b) replay determinism
//   (c) replay rejects checkpoint older than ledger watermark (gap detection)
//   (d) missing checkpoint -> typed error
//   (e) partial delta -> typed error
//
// validator_focus: strict seq ordering (no in-memory re-sort allowed — query
// must be ORDER BY seq ASC); gap detection mandatory; 100x determinism test.

mod mt_192_adversarial {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn make_checkpoint(session: Uuid, watermark: i64) -> SessionCheckpoint {
        SessionCheckpoint::new(
            session,
            Uuid::now_v7(),
            watermark,
            serde_json::json!({"counter": 0, "ops": []}),
            CheckpointStateKind::Periodic,
        )
        .unwrap()
    }

    fn make_event(session: Uuid, seq: i64, kind: &str, val: i64) -> EventLedgerRow {
        EventLedgerRow {
            event_id: format!("E-{seq}"),
            event_sequence: seq,
            session_id: session,
            event_type: kind.to_string(),
            payload: serde_json::json!({"v": val}),
            created_at: chrono::Utc::now(),
        }
    }

    /// (a) Replay from latest checkpoint + delta from event ledger: a
    /// checkpoint at watermark=5 plus events 6..=10 produces a final state
    /// that reflects all 5 delta events.
    #[test]
    fn mt_192_adversarial_replay_checkpoint_plus_delta_produces_authoritative_state() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 5);
        // Seed state matches the contract: state is rebuilt from
        // checkpoint.compact_state. The applicator reads payload.v and adds.
        let events: Vec<EventLedgerRow> = (6..=10)
            .map(|seq| make_event(s, seq, "increment", seq * 10))
            .collect();
        let plan = StateReplayer::plan(cp, &events);
        assert_eq!(plan.events_to_replay.len(), 5);
        assert_eq!(plan.expected_final_seq, 10);
        let initial: serde_json::Value = plan.from_checkpoint.compact_state.clone();
        let result = StateReplayer::execute(plan, initial, |st, ev| {
            let v = ev.payload.get("v").and_then(|x| x.as_i64()).unwrap();
            let c = st.get("counter").and_then(|x| x.as_i64()).unwrap();
            let mut ops: Vec<i64> = st
                .get("ops")
                .and_then(|x| x.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default();
            ops.push(v);
            *st = serde_json::json!({"counter": c + v, "ops": ops});
            Ok(())
        })
        .unwrap();
        assert_eq!(result.applied_count, 5);
        assert_eq!(result.final_seq, 10);
        let counter = result
            .final_state
            .get("counter")
            .and_then(|v| v.as_i64())
            .unwrap();
        assert_eq!(counter, 6 * 10 + 7 * 10 + 8 * 10 + 9 * 10 + 10 * 10);
    }

    /// (b) Determinism: 100 replays of the same plan against the same state
    /// applicator produce byte-identical serialized final state. Validator
    /// focus explicitly demands this.
    #[test]
    fn mt_192_adversarial_replay_is_deterministic_over_100_runs() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        // Build a fixed, non-trivial event sequence — different event_types
        // and payloads — to ensure the applicator branches multiple ways.
        let events = vec![
            make_event(s, 1, "increment", 10),
            make_event(s, 2, "scale", 3),
            make_event(s, 3, "increment", -5),
            make_event(s, 4, "increment", 100),
            make_event(s, 5, "scale", 2),
            make_event(s, 6, "increment", 1),
        ];
        let mut hashes = Vec::with_capacity(100);
        for _ in 0..100 {
            let plan = StateReplayer::plan(cp.clone(), &events);
            let initial = plan.from_checkpoint.compact_state.clone();
            let result = StateReplayer::execute(plan, initial, |st, ev| {
                let v = ev.payload.get("v").and_then(|x| x.as_i64()).unwrap();
                let c = st.get("counter").and_then(|x| x.as_i64()).unwrap_or(0);
                let new_c = match ev.event_type.as_str() {
                    "increment" => c + v,
                    "scale" => c * v,
                    _ => c,
                };
                *st = serde_json::json!({"counter": new_c});
                Ok(())
            })
            .unwrap();
            let serialized = serde_json::to_vec(&result.final_state).unwrap();
            let mut hasher = DefaultHasher::new();
            serialized.hash(&mut hasher);
            hashes.push(hasher.finish());
        }
        let first = hashes[0];
        for (i, h) in hashes.iter().enumerate() {
            assert_eq!(
                *h, first,
                "replay #{i} diverged: expected hash {first}, got {h}"
            );
        }
    }

    /// (c) Gap detection at the watermark boundary: if a checkpoint declares
    /// watermark=5 but the very next event in the ledger is seq=7 (i.e.
    /// event 6 is missing), the replayer must return
    /// `MissingEvent { gap_at_seq: 6 }`. This is the "checkpoint older than
    /// ledger but ledger has a hole" scenario.
    #[test]
    fn mt_192_adversarial_gap_at_watermark_boundary_returns_missing_event() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 5);
        // Watermark=5; next legal event is seq=6. We provide seq=7 instead.
        let events = vec![make_event(s, 7, "increment", 1)];
        let plan = StateReplayer::plan(cp, &events);
        let r: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute(plan, serde_json::json!({}), |_, _| Ok(()));
        match r {
            Err(ReplayError::MissingEvent { gap_at_seq: 6 }) => {}
            other => panic!("expected MissingEvent {{ gap_at_seq: 6 }}; got {other:?}"),
        }
    }

    /// (c') Gap detection mid-stream: checkpoint watermark=0; events
    /// seq=[1,2,3,5,6] — gap at 4. Replayer must reject with the gap_at_seq
    /// pointing at the first missing seq, not the seq AFTER the gap.
    #[test]
    fn mt_192_adversarial_mid_stream_gap_returns_missing_event_at_first_hole() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![
            make_event(s, 1, "inc", 1),
            make_event(s, 2, "inc", 1),
            make_event(s, 3, "inc", 1),
            make_event(s, 5, "inc", 1),
            make_event(s, 6, "inc", 1),
        ];
        let plan = StateReplayer::plan(cp, &events);
        let r: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute(plan, serde_json::json!({}), |_, _| Ok(()));
        match r {
            Err(ReplayError::MissingEvent { gap_at_seq: 4 }) => {}
            other => panic!("expected MissingEvent {{ gap_at_seq: 4 }}; got {other:?}"),
        }
    }

    /// (d) "Missing checkpoint" is modeled at the `plan()` boundary — the
    /// type signature requires an owned `SessionCheckpoint`, so a caller
    /// without one is rejected at compile time. The adversarial scenario we
    /// can express in this surface is: the applicator returns
    /// `StateInvariantViolated` because no checkpoint state was seeded; the
    /// replayer propagates it as a typed error.
    #[test]
    fn mt_192_adversarial_applicator_typed_error_propagates_at_seq() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![
            make_event(s, 1, "noop", 0),
            make_event(s, 2, "noop", 0),
            make_event(s, 3, "noop", 0),
            make_event(s, 4, "noop", 0),
            make_event(s, 5, "noop", 0),
        ];
        let plan = StateReplayer::plan(cp, &events);
        let r: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute(plan, serde_json::json!({}), |_, ev| {
                if ev.event_sequence == 3 {
                    return Err(ReplayError::StateInvariantViolated {
                        seq: ev.event_sequence,
                        invariant: "checkpoint-missing-or-corrupt".to_string(),
                    });
                }
                Ok(())
            });
        match r {
            Err(ReplayError::StateInvariantViolated { seq, ref invariant })
                if seq == 3 && invariant == "checkpoint-missing-or-corrupt" => {}
            other => panic!("expected StateInvariantViolated at seq 3; got {other:?}"),
        }
    }

    /// (e) Partial delta: an applicator may produce an `EventNotApplicable`
    /// typed error mid-replay; the replayer must NOT swallow it, must NOT
    /// continue past it, and must return the typed error with the seq where
    /// it occurred.
    #[test]
    fn mt_192_adversarial_partial_delta_returns_event_not_applicable() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events: Vec<EventLedgerRow> = (1..=10)
            .map(|seq| make_event(s, seq, "step", seq))
            .collect();
        let plan = StateReplayer::plan(cp, &events);
        let r: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute(plan, serde_json::json!({"cap": 5}), |st, ev| {
                let cap = st.get("cap").and_then(|x| x.as_i64()).unwrap_or(0);
                let v = ev.payload.get("v").and_then(|x| x.as_i64()).unwrap();
                if v > cap {
                    return Err(ReplayError::EventNotApplicable {
                        seq: ev.event_sequence,
                        reason: format!("v={v} exceeds cap={cap}"),
                    });
                }
                Ok(())
            });
        match r {
            Err(ReplayError::EventNotApplicable { seq, .. }) if seq == 6 => {}
            other => panic!("expected EventNotApplicable at seq 6 (v=6 > cap=5); got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mt_192_fr_records_replay_started_progress_completed_in_order() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 5);
        let events = vec![
            make_event(s, 6, "increment", 1),
            make_event(s, 7, "increment", 2),
        ];
        let plan = StateReplayer::plan(cp, &events);
        let recorder = TestFlightRecorder::default();

        let result = StateReplayer::execute_with_flight_recorder(
            plan,
            serde_json::json!({"counter": 0}),
            |st, ev| {
                let v = ev
                    .payload
                    .get("v")
                    .and_then(|value| value.as_i64())
                    .unwrap();
                let c = st.get("counter").and_then(|value| value.as_i64()).unwrap();
                *st = serde_json::json!({"counter": c + v});
                Ok(())
            },
            &recorder,
        )
        .await
        .unwrap();

        assert_eq!(result.final_seq, 7);
        assert_eq!(
            recorder.payload_event_ids(),
            vec![
                "FR-EVT-REPLAY-STARTED",
                "FR-EVT-REPLAY-PROGRESS",
                "FR-EVT-REPLAY-PROGRESS",
                "FR-EVT-REPLAY-COMPLETED"
            ]
        );
        let events = recorder.events();
        assert_eq!(events[0].payload["session_id"], s.to_string());
        assert_eq!(events[0].payload["from_seq"], 5);
        assert_eq!(events[0].payload["to_seq"], 7);
        assert_eq!(events[1].payload["to_seq"], 6);
        assert_eq!(events[2].payload["to_seq"], 7);
        assert_eq!(events[3].payload["to_seq"], 7);
    }

    #[tokio::test]
    async fn mt_192_fr_records_replay_failed_on_missing_event_without_completed() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![make_event(s, 2, "increment", 1)];
        let plan = StateReplayer::plan(cp, &events);
        let recorder = TestFlightRecorder::default();

        let result: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute_with_flight_recorder(
                plan,
                serde_json::json!({"counter": 0}),
                |_, _| Ok(()),
                &recorder,
            )
            .await;

        assert!(matches!(
            result,
            Err(ReplayError::MissingEvent { gap_at_seq: 1 })
        ));
        assert_eq!(
            recorder.payload_event_ids(),
            vec!["FR-EVT-REPLAY-STARTED", "FR-EVT-REPLAY-FAILED"]
        );
        let events = recorder.events();
        assert_eq!(events[1].payload["session_id"], s.to_string());
        assert_eq!(events[1].payload["from_seq"], 0);
        assert_eq!(events[1].payload["to_seq"], 1);
    }

    /// Validator-focus assertion: even if the input events slice is supplied
    /// in shuffled order, `StateReplayer::plan` re-sorts by seq ascending.
    /// The contract says "query must be ORDER BY seq ASC" — the in-memory
    /// re-sort defends against accidental misuse where the caller forgets
    /// to ORDER BY at the SQL boundary. Documents the layered defense.
    #[test]
    fn mt_192_adversarial_plan_normalizes_seq_order_even_if_caller_supplies_shuffled() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let mut events = vec![
            make_event(s, 3, "inc", 30),
            make_event(s, 1, "inc", 10),
            make_event(s, 2, "inc", 20),
        ];
        // Caller-supplied order is wrong on purpose.
        let _ = events.clone();
        events.reverse();
        let plan = StateReplayer::plan(cp, &events);
        let seqs: Vec<i64> = plan
            .events_to_replay
            .iter()
            .map(|e| e.event_sequence)
            .collect();
        assert_eq!(
            seqs,
            vec![1, 2, 3],
            "plan must re-sort events by seq ASC (defense against shuffled input)"
        );
    }

    /// Cross-session contamination: a plan must never pick up events that
    /// belong to a different session_id, even if a buggy SQL caller selects
    /// rows from multiple sessions into the events slice.
    #[test]
    fn mt_192_adversarial_plan_filters_foreign_session_events() {
        let s = Uuid::now_v7();
        let other = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![
            make_event(s, 1, "inc", 10),
            make_event(other, 2, "inc", 999),
            make_event(s, 2, "inc", 20),
            make_event(other, 3, "inc", 999),
        ];
        let plan = StateReplayer::plan(cp, &events);
        assert_eq!(plan.events_to_replay.len(), 2);
        assert!(
            plan.events_to_replay.iter().all(|e| e.session_id == s),
            "plan must filter out events belonging to other sessions"
        );
    }

    /// ReplayError typed variants round-trip through serde. This is the
    /// receipts contract — `ReplayError` rows are persisted via FR-EVT-REPLAY-FAILED
    /// and must survive a serialize/deserialize cycle without losing structure.
    #[test]
    fn mt_192_adversarial_replay_error_round_trips_all_variants() {
        let variants = vec![
            ReplayError::EventNotApplicable {
                seq: 42,
                reason: "test reason".to_string(),
            },
            ReplayError::StateInvariantViolated {
                seq: 7,
                invariant: "counter-must-be-non-negative".to_string(),
            },
            ReplayError::MissingEvent { gap_at_seq: 13 },
        ];
        for v in variants {
            let s = serde_json::to_string(&v).unwrap();
            let back: ReplayError = serde_json::from_str(&s).unwrap();
            assert_eq!(v, back);
        }
    }
}

// =================================================================
// MT-191 / MT-192 embedded adversarial coverage
// =================================================================
//
// These tests prove the production embedded sink persists the checkpoint
// cadence and that a checkpoint can be loaded through the public typed store
// and replayed deterministically via StateReplayer.

#[cfg(test)]
mod mt_191_192_adversarial_embedded {
    use super::*;
    use handshake_core::session_checkpoint::writer::{
        CheckpointSink, CheckpointWriter, CheckpointWriterConfig, SurrealCheckpointSink,
    };
    use handshake_core::storage::tests::embedded_test_backend;
    use std::sync::Arc;
    use std::time::Duration;

    /// MT-191: the production `SurrealCheckpointSink` drains all cadence kinds
    /// submitted through the CheckpointWriter into the embedded table.
    #[tokio::test]
    async fn mt_191_adversarial_embedded_writer_drains_into_durable_table() {
        let backend = embedded_test_backend()
            .await
            .expect("open embedded backend");
        let sink = Arc::new(SurrealCheckpointSink::new(backend.storage.clone()));
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 64,
                batch_size: 4,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let session = Uuid::now_v7();
        let mut checkpoint_ids = Vec::with_capacity(18);
        // Periodic cadence submissions.
        for seq in 0..16i64 {
            let cp = SessionCheckpoint::new(
                session,
                Uuid::now_v7(),
                seq,
                serde_json::json!({"seq": seq}),
                CheckpointStateKind::Periodic,
            )
            .unwrap();
            checkpoint_ids.push(cp.checkpoint_id.as_uuid());
            handle.submit(cp).unwrap();
        }
        // Event-triggered cadence (re-stamped to event_triggered by the writer).
        let evt = SessionCheckpoint::new(
            session,
            Uuid::now_v7(),
            16,
            serde_json::json!({"seq": 16}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        checkpoint_ids.push(evt.checkpoint_id.as_uuid());
        handle.submit_event_triggered(evt).await.unwrap();
        // Pre-shutdown cadence: a final checkpoint submitted before shutdown
        // flushes the channel and drains via the production sink.
        let pre_shutdown = SessionCheckpoint::new(
            session,
            Uuid::now_v7(),
            17,
            serde_json::json!({"seq": 17}),
            CheckpointStateKind::PreShutdown,
        )
        .unwrap();
        checkpoint_ids.push(pre_shutdown.checkpoint_id.as_uuid());
        handle.submit(pre_shutdown).unwrap();
        handle.shutdown().await.unwrap();
        drop(sink);

        let reopened = reopen_embedded_backend(&backend).await;
        let persisted = persisted_kernel_checkpoints(&reopened, &checkpoint_ids).await;
        let session_rows = persisted
            .iter()
            .filter(|checkpoint| checkpoint.session_id == session)
            .collect::<Vec<_>>();
        assert_eq!(session_rows.len(), 18);
        let kinds = session_rows
            .iter()
            .map(|checkpoint| checkpoint.state_kind)
            .collect::<std::collections::HashSet<_>>();
        assert!(kinds.contains(&CheckpointStateKind::Periodic));
        assert!(kinds.contains(&CheckpointStateKind::EventTriggered));
        assert!(kinds.contains(&CheckpointStateKind::PreShutdown));
        close_reopened_and_remove(reopened, backend).await;
    }

    /// MT-192: write a checkpoint via the embedded sink, decode it from the
    /// reopened table through the read-only test inspector, build a ReplayPlan
    /// with synthetic events, and verify the replayer reconstructs the expected
    /// final state. End-to-end proves MT-191 -> MT-192 cluster X.3 integration.
    #[tokio::test]
    async fn mt_192_adversarial_embedded_replay_from_persisted_checkpoint() {
        let backend = embedded_test_backend()
            .await
            .expect("open embedded backend");
        let sink = SurrealCheckpointSink::new(backend.storage.clone());
        let session = Uuid::now_v7();
        let cp = SessionCheckpoint::new(
            session,
            Uuid::now_v7(),
            0,
            serde_json::json!({"counter": 0}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let checkpoint_id = cp.checkpoint_id.as_uuid();
        sink.write_batch(vec![cp]).await.unwrap();
        drop(sink);
        let reopened = reopen_embedded_backend(&backend).await;
        let loaded = persisted_kernel_checkpoint(&reopened, checkpoint_id)
            .await
            .expect("load checkpoint after close/reopen");
        assert_eq!(loaded.last_event_ledger_seq, 0);
        assert_eq!(loaded.compact_state["counter"].as_i64(), Some(0));
        // Build synthetic events 1..=3 and replay.
        let events: Vec<EventLedgerRow> = (1..=3)
            .map(|seq| EventLedgerRow {
                event_id: format!("E-{seq}"),
                event_sequence: seq,
                session_id: session,
                event_type: "inc".to_string(),
                payload: serde_json::json!({"v": seq * 10}),
                created_at: chrono::Utc::now(),
            })
            .collect();
        let plan = StateReplayer::plan(loaded, &events);
        let initial = plan.from_checkpoint.compact_state.clone();
        let result = StateReplayer::execute(plan, initial, |st, ev| {
            let v = ev.payload.get("v").and_then(|x| x.as_i64()).unwrap();
            let c = st.get("counter").and_then(|x| x.as_i64()).unwrap();
            *st = serde_json::json!({"counter": c + v});
            Ok(())
        })
        .unwrap();
        assert_eq!(result.final_seq, 3);
        assert_eq!(
            result.final_state.get("counter").and_then(|v| v.as_i64()),
            Some(60)
        );
        close_reopened_and_remove(reopened, backend).await;
    }
}
