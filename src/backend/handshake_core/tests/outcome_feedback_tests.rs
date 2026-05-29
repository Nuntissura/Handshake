use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::Utc;
use handshake_core::kernel::action_catalog::kernel002_action_catalog;
use handshake_core::memory::outcome_feedback::{
    CapsuleOutcome, FailureClass, MemoryPackItemRef, OUTCOME_ATTACH_ACTION_ID,
    OutcomeAttachSubmitter, OutcomeAttribution, OutcomeError, OutcomeFeedbackLoop, OutcomeReceipt,
    OutcomeScoringTuner, TuningParams,
};
use handshake_core::memory::persistence_postgres::PostgresKernelActionSubmitter;
use handshake_core::storage::{Database, StorageError, StorageResult, postgres::PostgresDatabase};
use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

struct RecordingOutcomeSubmitter {
    attachments: Mutex<Vec<OutcomeAttribution>>,
}

impl RecordingOutcomeSubmitter {
    fn new() -> Self {
        Self {
            attachments: Mutex::new(Vec::new()),
        }
    }
}

impl OutcomeAttachSubmitter for RecordingOutcomeSubmitter {
    fn attach_outcome(
        &self,
        attribution: OutcomeAttribution,
    ) -> Result<OutcomeReceipt, OutcomeError> {
        self.attachments.lock().unwrap().push(attribution.clone());
        Ok(OutcomeReceipt {
            receipt_id: Uuid::now_v7(),
            capsule_id: attribution.capsule_id,
            action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
            recorded_at_utc: Utc::now(),
        })
    }
}

fn pack(ids: &[u128]) -> Vec<MemoryPackItemRef> {
    ids.iter()
        .map(|id| MemoryPackItemRef {
            memory_id: Uuid::from_u128(*id),
            pinned: false,
        })
        .collect()
}

fn seeded_scores(items: &[MemoryPackItemRef], value: f64) -> HashMap<Uuid, f64> {
    items.iter().map(|item| (item.memory_id, value)).collect()
}

#[test]
fn pass_outcome_increases_item_scores_by_exact_pass_boost_when_decay_disabled() {
    let items = pack(&[1, 2, 3]);
    let mut scores = seeded_scores(&items, 0.5);
    let tuning = TuningParams {
        pass_boost: 0.05,
        fail_penalty: 0.10,
        escalation_penalty: 0.15,
        per_item_decay_per_use: 0.0,
        max_abs_change_per_call: 1.0,
    };

    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Pass {
            mt_id: "MT-158".to_string(),
            validator_verdict_id: Uuid::now_v7(),
        },
        &items,
        &tuning,
    );

    for item in &items {
        assert!((scores[&item.memory_id] - 0.55).abs() < f64::EPSILON);
    }
}

#[test]
fn fail_outcome_decreases_item_scores_by_exact_fail_penalty_when_decay_disabled() {
    let items = pack(&[1, 2]);
    let mut scores = seeded_scores(&items, 0.5);
    let tuning = TuningParams {
        pass_boost: 0.05,
        fail_penalty: 0.10,
        escalation_penalty: 0.15,
        per_item_decay_per_use: 0.0,
        max_abs_change_per_call: 1.0,
    };

    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Fail {
            mt_id: "MT-158".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::ValidatorRejected,
        },
        &items,
        &tuning,
    );

    for item in &items {
        assert!((scores[&item.memory_id] - 0.40).abs() < f64::EPSILON);
    }
}

#[test]
fn skipped_outcome_leaves_item_scores_unchanged() {
    let items = pack(&[1]);
    let mut scores = seeded_scores(&items, 0.5);

    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Skipped {
            reason: "not-invoked".to_string(),
        },
        &items,
        &TuningParams::default(),
    );

    assert!((scores[&items[0].memory_id] - 0.5).abs() < f64::EPSILON);
}

#[test]
fn score_change_bound_covers_penalty_and_decay_together() {
    let items = pack(&[1]);
    let mut scores = seeded_scores(&items, 0.5);
    let tuning = TuningParams {
        pass_boost: 0.0,
        fail_penalty: 2.0,
        escalation_penalty: 0.0,
        per_item_decay_per_use: 1.0,
        max_abs_change_per_call: 0.25,
    };

    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Fail {
            mt_id: "MT-158".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::Other,
        },
        &items,
        &tuning,
    );

    assert!(
        (scores[&items[0].memory_id] - 0.25).abs() < f64::EPSILON,
        "total per-call score change must be capped at 0.25"
    );
}

#[test]
fn outcome_attach_action_is_registered_in_kernel_action_catalog() {
    let catalog = kernel002_action_catalog();
    let action = catalog
        .action(OUTCOME_ATTACH_ACTION_ID)
        .expect("outcome attach action must be registered");

    assert_eq!(action.action_id, OUTCOME_ATTACH_ACTION_ID);
    assert_eq!(
        action.expected_write_boxes[0].target_id,
        "memory_capsule_outcome"
    );
}

#[test]
fn record_outcome_attaches_canonical_action_receipt() {
    let submitter = RecordingOutcomeSubmitter::new();
    let feedback = OutcomeFeedbackLoop::new(&submitter);
    let capsule_id = Uuid::now_v7();
    let items = pack(&[1]);
    let mut scores = seeded_scores(&items, 0.5);

    let receipt = feedback
        .record_outcome(
            capsule_id,
            CapsuleOutcome::Escalation {
                mt_id: "MT-158".to_string(),
                escalation_reason: "validator-mediation".to_string(),
            },
            &mut scores,
            &items,
            &TuningParams::default(),
        )
        .expect("attach outcome");

    assert_eq!(receipt.capsule_id, capsule_id);
    assert_eq!(receipt.action_id, OUTCOME_ATTACH_ACTION_ID);
    let attachments = submitter.attachments.lock().unwrap();
    assert_eq!(attachments.len(), 1);
    assert!(matches!(
        attachments[0].outcome,
        CapsuleOutcome::Escalation { .. }
    ));
    assert!(
        scores[&items[0].memory_id] < 0.5,
        "successful escalation attachment should trigger tuning"
    );
}

#[test]
fn record_outcome_does_not_tune_scores_when_attach_fails() {
    struct RejectingSubmitter;
    impl OutcomeAttachSubmitter for RejectingSubmitter {
        fn attach_outcome(
            &self,
            _attribution: OutcomeAttribution,
        ) -> Result<OutcomeReceipt, OutcomeError> {
            Err(OutcomeError::Rejected {
                code: "reject".to_string(),
                reason: "forced".to_string(),
            })
        }
    }

    let submitter = RejectingSubmitter;
    let feedback = OutcomeFeedbackLoop::new(&submitter);
    let items = pack(&[1]);
    let mut scores = seeded_scores(&items, 0.5);

    let result = feedback.record_outcome(
        Uuid::now_v7(),
        CapsuleOutcome::Fail {
            mt_id: "MT-158".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::Other,
        },
        &mut scores,
        &items,
        &TuningParams::default(),
    );

    assert!(matches!(result, Err(OutcomeError::Rejected { .. })));
    assert!(
        (scores[&items[0].memory_id] - 0.5).abs() < f64::EPSILON,
        "failed attachment must not mutate scores"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires POSTGRES_TEST_URL and an isolated live Postgres schema"]
async fn postgres_outcome_attach_submitter_persists_catalog_event() {
    let (db, pool) = isolated_postgres().await.expect("isolated postgres");
    let submitter = PostgresKernelActionSubmitter::with_db(db);
    let feedback = OutcomeFeedbackLoop::new(&submitter);
    let capsule_id = Uuid::now_v7();
    let items = pack(&[1]);
    let mut scores = seeded_scores(&items, 0.5);

    let receipt = feedback
        .record_outcome(
            capsule_id,
            CapsuleOutcome::Pass {
                mt_id: "MT-158".to_string(),
                validator_verdict_id: Uuid::now_v7(),
            },
            &mut scores,
            &items,
            &TuningParams {
                per_item_decay_per_use: 0.0,
                ..TuningParams::default()
            },
        )
        .expect("record outcome through postgres submitter");

    assert_eq!(receipt.action_id, OUTCOME_ATTACH_ACTION_ID);
    assert!(scores[&items[0].memory_id] > 0.5);

    let row = sqlx::query(
        r#"
        SELECT payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'memory_capsule'
          AND aggregate_id = $1
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(capsule_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("outcome event row");
    let payload: serde_json::Value = row.get("payload");
    let capsule_id_string = capsule_id.to_string();
    assert_eq!(
        payload["catalog_action_id"].as_str(),
        Some(OUTCOME_ATTACH_ACTION_ID)
    );
    assert_eq!(
        payload["request"]["action_id"].as_str(),
        Some(OUTCOME_ATTACH_ACTION_ID)
    );
    assert_eq!(
        payload["write_box_envelope"]["payload"]["attribution"]["capsule_id"].as_str(),
        Some(capsule_id_string.as_str())
    );
    assert_eq!(
        payload["write_box_envelope"]["payload"]["attribution"]["outcome"]["outcome"].as_str(),
        Some("pass")
    );
}

async fn isolated_postgres() -> StorageResult<(Arc<dyn Database>, PgPool)> {
    let url = std::env::var("POSTGRES_TEST_URL")
        .map_err(|_| StorageError::Validation("POSTGRES_TEST_URL not set for postgres tests"))?;
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("mt158_outcome_feedback_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    let pool = PgPool::connect(&schema_url).await?;
    Ok((db.into_arc(), pool))
}
