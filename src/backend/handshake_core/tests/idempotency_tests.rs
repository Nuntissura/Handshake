use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use handshake_core::{
    process_ledger::idempotency::{
        ApplyOutcome, IdempotencyKey, IdempotencyLedger, SideEffectKind,
    },
    session_checkpoint::{CheckpointStateKind, EventLedgerRow, SessionCheckpoint, StateReplayer},
};
use sqlx::Connection;
use uuid::Uuid;

fn key(session_id: Uuid, event_seq: i64, side_effect_kind: SideEffectKind) -> IdempotencyKey {
    IdempotencyKey {
        session_id,
        event_seq,
        side_effect_kind,
    }
}

fn checkpoint(session_id: Uuid) -> SessionCheckpoint {
    SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        0,
        serde_json::json!({ "counter": 0 }),
        CheckpointStateKind::Periodic,
    )
    .unwrap()
}

fn event(session_id: Uuid, seq: i64) -> EventLedgerRow {
    EventLedgerRow {
        event_id: format!("evt-{seq}"),
        event_sequence: seq,
        session_id,
        event_type: "step".to_string(),
        payload: serde_json::json!({ "by": 1 }),
        created_at: chrono::Utc::now(),
    }
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL test URL");
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect PostgreSQL test URL");
    let schema = format!("mt194_test_{}", Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut conn)
        .await
        .expect("create isolated schema");
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&schema_url)
        .await
        .expect("connect isolated schema");
    sqlx::query(
        r#"
        CREATE TABLE kernel_idempotency_ledger (
            session_id UUID NOT NULL,
            event_seq BIGINT NOT NULL,
            side_effect_kind TEXT NOT NULL,
            applied_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (session_id, event_seq, side_effect_kind)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create idempotency ledger");
    pool
}

#[tokio::test]
async fn mt194_try_apply_twice_applies_once() {
    let ledger = IdempotencyLedger::in_memory();
    let session_id = Uuid::now_v7();
    let applied = Arc::new(AtomicUsize::new(0));
    let first_count = Arc::clone(&applied);
    let second_count = Arc::clone(&applied);
    let idempotency_key = key(session_id, 1, SideEffectKind::MailboxMessagePost);

    let first = ledger
        .try_apply(idempotency_key.clone(), move || async move {
            first_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
    let second = ledger
        .try_apply(idempotency_key, move || async move {
            second_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();

    assert_eq!(first, ApplyOutcome::Applied);
    assert_eq!(second, ApplyOutcome::AlreadyApplied);
    assert_eq!(applied.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mt194_parallel_try_apply_allows_exactly_one_apply() {
    let ledger = Arc::new(IdempotencyLedger::in_memory());
    let applied = Arc::new(AtomicUsize::new(0));
    let idempotency_key = key(Uuid::now_v7(), 7, SideEffectKind::LeaseAcquisition);
    let mut tasks = Vec::new();

    for _ in 0..8 {
        let ledger = Arc::clone(&ledger);
        let applied = Arc::clone(&applied);
        let idempotency_key = idempotency_key.clone();
        tasks.push(tokio::spawn(async move {
            ledger
                .try_apply(idempotency_key, move || async move {
                    applied.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
                .unwrap()
        }));
    }

    let mut outcomes = Vec::new();
    for task in tasks {
        outcomes.push(task.await.unwrap());
    }

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ApplyOutcome::Applied)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ApplyOutcome::AlreadyApplied)
            .count(),
        7
    );
    assert_eq!(applied.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mt194_failed_op_rolls_back_so_retry_can_succeed() {
    let ledger = IdempotencyLedger::in_memory();
    let idempotency_key = key(Uuid::now_v7(), 3, SideEffectKind::PostgresWrite);

    let failed = ledger
        .try_apply(idempotency_key.clone(), || async {
            Err("transient".to_string())
        })
        .await
        .unwrap();
    let retry = ledger
        .try_apply(idempotency_key, || async { Ok(()) })
        .await
        .unwrap();

    assert!(matches!(failed, ApplyOutcome::Failed { .. }));
    assert_eq!(retry, ApplyOutcome::Applied);
}

#[tokio::test]
async fn mt194_postgres_write_idempotency_is_specific_to_table() {
    let ledger = IdempotencyLedger::in_memory();
    let session_id = Uuid::now_v7();
    let applied = Arc::new(AtomicUsize::new(0));
    let primary_table = key(
        session_id,
        11,
        SideEffectKind::postgres_write_table("kernel_events"),
    );
    let archive_table = key(
        session_id,
        11,
        SideEffectKind::postgres_write_table("kernel_events_archive"),
    );

    let first_primary = ledger
        .try_apply(primary_table.clone(), {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let duplicate_primary = ledger
        .try_apply(primary_table, {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let first_archive = ledger
        .try_apply(archive_table.clone(), {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let duplicate_archive = ledger
        .try_apply(archive_table, {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();

    assert_eq!(first_primary, ApplyOutcome::Applied);
    assert_eq!(duplicate_primary, ApplyOutcome::AlreadyApplied);
    assert_eq!(first_archive, ApplyOutcome::Applied);
    assert_eq!(duplicate_archive, ApplyOutcome::AlreadyApplied);
    assert_eq!(applied.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn mt194_file_system_write_idempotency_is_specific_to_target_key() {
    let ledger = IdempotencyLedger::in_memory();
    let session_id = Uuid::now_v7();
    let applied = Arc::new(AtomicUsize::new(0));
    let report_path_key = key(
        session_id,
        12,
        SideEffectKind::file_system_write_target_key("path-sha256:aaaaaaaaaaaaaaaa"),
    );
    let receipt_path_key = key(
        session_id,
        12,
        SideEffectKind::file_system_write_target_key("path-sha256:bbbbbbbbbbbbbbbb"),
    );

    let first_report = ledger
        .try_apply(report_path_key.clone(), {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let duplicate_report = ledger
        .try_apply(report_path_key, {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let first_receipt = ledger
        .try_apply(receipt_path_key.clone(), {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    let duplicate_receipt = ledger
        .try_apply(receipt_path_key, {
            let applied = Arc::clone(&applied);
            move || async move {
                applied.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();

    assert_eq!(first_report, ApplyOutcome::Applied);
    assert_eq!(duplicate_report, ApplyOutcome::AlreadyApplied);
    assert_eq!(first_receipt, ApplyOutcome::Applied);
    assert_eq!(duplicate_receipt, ApplyOutcome::AlreadyApplied);
    assert_eq!(applied.load(Ordering::SeqCst), 2);
}

#[test]
fn mt194_targeted_side_effect_storage_key_is_deterministic_text() {
    let session_id = Uuid::now_v7();
    let primary_table = key(
        session_id,
        13,
        SideEffectKind::postgres_write_table("kernel_events"),
    );
    let archive_table = key(
        session_id,
        13,
        SideEffectKind::postgres_write_table("kernel_events_archive"),
    );
    let tricky_table = key(
        session_id,
        13,
        SideEffectKind::postgres_write_table("kernel_events|table:13:shadow"),
    );
    let report_path_key = key(
        session_id,
        13,
        SideEffectKind::file_system_write_target_key("path-sha256:aaaaaaaaaaaaaaaa"),
    );

    assert_eq!(
        primary_table.side_effect_storage_key(),
        "postgres_write|table:len=13:hex=6b65726e656c5f6576656e7473"
    );
    assert_eq!(
        archive_table.side_effect_storage_key(),
        "postgres_write|table:len=21:hex=6b65726e656c5f6576656e74735f61726368697665"
    );
    assert_eq!(
        tricky_table.side_effect_storage_key(),
        "postgres_write|table:len=29:hex=6b65726e656c5f6576656e74737c7461626c653a31333a736861646f77"
    );
    assert_eq!(
        report_path_key.side_effect_storage_key(),
        "file_system_write|path_key:len=28:hex=706174682d7368613235363a61616161616161616161616161616161"
    );
    assert_ne!(
        primary_table.side_effect_storage_key(),
        archive_table.side_effect_storage_key()
    );
    assert!([
        primary_table.side_effect_storage_key(),
        archive_table.side_effect_storage_key(),
        tricky_table.side_effect_storage_key(),
        report_path_key.side_effect_storage_key(),
    ]
    .iter()
    .all(|value| !value.contains('\0') && !value.contains('\n')));
}

#[tokio::test]
async fn mt194_replay_reuses_ledger_and_does_not_repeat_side_effects() {
    let ledger = IdempotencyLedger::in_memory();
    let session_id = Uuid::now_v7();
    let checkpoint = checkpoint(session_id);
    let events: Vec<_> = (1..=10).map(|seq| event(session_id, seq)).collect();
    let side_effects = Arc::new(AtomicUsize::new(0));

    for _ in 0..2 {
        let plan = StateReplayer::plan(checkpoint.clone(), &events);
        let side_effects = Arc::clone(&side_effects);
        StateReplayer::execute(plan, serde_json::json!({ "counter": 0 }), |state, event| {
            let next = state
                .get("counter")
                .and_then(|value| value.as_i64())
                .unwrap_or(0)
                + 1;
            *state = serde_json::json!({ "counter": next });

            if event.event_sequence % 2 == 0 {
                let outcome = futures::executor::block_on(ledger.try_apply(
                    key(
                        session_id,
                        event.event_sequence,
                        SideEffectKind::FileSystemWrite,
                    ),
                    || {
                        let side_effects = Arc::clone(&side_effects);
                        async move {
                            side_effects.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        }
                    },
                ))
                .unwrap();
                assert!(matches!(
                    outcome,
                    ApplyOutcome::Applied | ApplyOutcome::AlreadyApplied
                ));
            }
            Ok(())
        })
        .unwrap();
    }

    assert_eq!(side_effects.load(Ordering::SeqCst), 5);
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test --test idempotency_tests -- --ignored`"]
async fn mt194_postgres_ledger_distinguishes_side_effect_targets_and_dedupes_duplicates() {
    let pool = postgres_pool().await;
    let ledger = IdempotencyLedger::new(pool.clone());
    let session_id = Uuid::now_v7();
    let applied = Arc::new(AtomicUsize::new(0));
    let primary_table = key(
        session_id,
        1,
        SideEffectKind::postgres_write_table("kernel_events"),
    );
    let archive_table = key(
        session_id,
        1,
        SideEffectKind::postgres_write_table("kernel_events_archive"),
    );
    let report_path_key = key(
        session_id,
        1,
        SideEffectKind::file_system_write_target_key("path-sha256:aaaaaaaaaaaaaaaa"),
    );

    for idempotency_key in [
        primary_table.clone(),
        archive_table.clone(),
        report_path_key.clone(),
    ] {
        let first = ledger
            .try_apply(idempotency_key.clone(), {
                let applied = Arc::clone(&applied);
                move || async move {
                    applied.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await
            .unwrap();
        let duplicate = ledger
            .try_apply(idempotency_key, {
                let applied = Arc::clone(&applied);
                move || async move {
                    applied.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await
            .unwrap();

        assert_eq!(first, ApplyOutcome::Applied);
        assert_eq!(duplicate, ApplyOutcome::AlreadyApplied);
    }

    assert_eq!(applied.load(Ordering::SeqCst), 3);

    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT side_effect_kind
           FROM kernel_idempotency_ledger
           WHERE session_id = $1
           ORDER BY side_effect_kind"#,
    )
    .bind(session_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let stored_keys: Vec<_> = rows.into_iter().map(|row| row.0).collect();
    assert_eq!(stored_keys.len(), 3);
    assert!(stored_keys.contains(&primary_table.side_effect_storage_key()));
    assert!(stored_keys.contains(&archive_table.side_effect_storage_key()));
    assert!(stored_keys.contains(&report_path_key.side_effect_storage_key()));
}
