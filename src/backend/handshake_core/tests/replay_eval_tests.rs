use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use handshake_core::{
    ace::{FemsSourceRef, FemsSourceRefKind},
    memory::{
        bitemporal::{AsOfQuery, BitemporalIndex, BitemporalItem, BitemporalStamps},
        replay_eval::{
            BitemporalAccessor, CapsuleRecordReader, ReplayError, ReplayEvaluator, ReplayRequest,
        },
        BuildContext, CapsuleBuilder, CapsulePolicyTable, DegradationTier, FemsError,
        FemsRetriever, RetrievalPolicy, RetrievedItem, TaskType, RETRIEVAL_SCORING_FORMULA_V0,
    },
};
use serde_json::Value;
use uuid::Uuid;

struct FrozenFems {
    items: Vec<RetrievedItem>,
}

impl FemsRetriever for FrozenFems {
    fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        Ok(self.items.iter().take(top_k as usize).cloned().collect())
    }
}

#[derive(Clone)]
struct TestRecordReader {
    record: handshake_core::memory::CapsuleRecord,
}

impl CapsuleRecordReader for TestRecordReader {
    fn read_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<handshake_core::memory::CapsuleRecord, ReplayError> {
        if self.record.capsule_id == capsule_id {
            Ok(self.record.clone())
        } else {
            Err(ReplayError::CapsuleNotFound { capsule_id })
        }
    }
}

struct TestBitemporal {
    items: Vec<BitemporalItem>,
}

impl BitemporalAccessor for TestBitemporal {
    fn items_visible_at(&self, query: &AsOfQuery) -> Result<Vec<BitemporalItem>, ReplayError> {
        Ok(self
            .items
            .iter()
            .filter(|item| item.stamps.visible_at(query))
            .cloned()
            .collect())
    }
}

#[test]
fn replay_of_recorded_capsule_rebuilds_same_source_hash_and_empty_diff() {
    let item = retrieved_item(Uuid::from_u128(1), 0.82);
    let original = build_capsule(vec![item.clone()]);
    let record = handshake_core::memory::CapsuleRecord::from_capsule(
        &original,
        Utc::now(),
        "session-1",
        "KERNEL_BUILDER",
    );
    let query = as_of(120, 120);
    let mut index = BitemporalIndex::new();
    index.insert(bitemporal_item(item, stamps(100, 50, None)));

    let result = replay(
        ReplayRequest {
            capsule_id: record.capsule_id,
            replay_as_of: query,
            expected_source_hash: record.capsule_source_hash.clone(),
        },
        &TestRecordReader {
            record: record.clone(),
        },
        &index,
    )
    .expect("recorded capsule replays");

    assert!(result.replay_succeeded);
    assert_eq!(
        result.replayed_capsule.source_hash,
        record.capsule_source_hash
    );
    assert_eq!(result.replayed_capsule.pack.items.len(), 1);
    assert!(result.replay_difference.is_empty());
}

#[test]
fn replay_after_bitemporal_invalidation_reports_item_absent_now_present_then() {
    let item_id = Uuid::from_u128(2);
    let item = retrieved_item(item_id, 0.91);
    let original = build_capsule(vec![item.clone()]);
    let record = handshake_core::memory::CapsuleRecord::from_capsule(
        &original,
        Utc::now(),
        "session-1",
        "KERNEL_BUILDER",
    );

    let result = replay(
        ReplayRequest {
            capsule_id: record.capsule_id,
            replay_as_of: as_of(120, 160),
            expected_source_hash: record.capsule_source_hash.clone(),
        },
        &TestRecordReader { record },
        &TestBitemporal {
            items: vec![bitemporal_item(item, stamps(100, 50, Some(150)))],
        },
    )
    .expect("invalidated item replay still returns diff");

    assert!(!result.replay_succeeded);
    assert_eq!(
        result.replay_difference.items_absent_now_present_then,
        vec![item_id]
    );
    assert!(result
        .replay_difference
        .items_present_now_absent_then
        .is_empty());
}

#[test]
fn wrong_expected_source_hash_returns_typed_error() {
    let item = retrieved_item(Uuid::from_u128(3), 0.72);
    let original = build_capsule(vec![item.clone()]);
    let record = handshake_core::memory::CapsuleRecord::from_capsule(
        &original,
        Utc::now(),
        "session-1",
        "KERNEL_BUILDER",
    );

    let err = replay(
        ReplayRequest {
            capsule_id: record.capsule_id,
            replay_as_of: as_of(120, 120),
            expected_source_hash: "wrong-source-hash".to_string(),
        },
        &TestRecordReader { record },
        &TestBitemporal {
            items: vec![bitemporal_item(item, stamps(100, 50, None))],
        },
    )
    .expect_err("wrong source hash must be typed error");

    assert!(matches!(err, ReplayError::HashMismatch { .. }));
}

#[test]
fn replay_is_deterministic_across_invocations() {
    let items = vec![
        retrieved_item(Uuid::from_u128(4), 0.8),
        retrieved_item(Uuid::from_u128(5), 0.7),
    ];
    let original = build_capsule(items.clone());
    let record = handshake_core::memory::CapsuleRecord::from_capsule(
        &original,
        Utc::now(),
        "session-1",
        "KERNEL_BUILDER",
    );
    let reader = TestRecordReader {
        record: record.clone(),
    };
    let bitemporal = TestBitemporal {
        items: items
            .into_iter()
            .map(|item| bitemporal_item(item, stamps(100, 50, None)))
            .collect(),
    };
    let request = ReplayRequest {
        capsule_id: record.capsule_id,
        replay_as_of: as_of(120, 120),
        expected_source_hash: record.capsule_source_hash,
    };

    let first = replay(request.clone(), &reader, &bitemporal).unwrap();
    let second = replay(request, &reader, &bitemporal).unwrap();

    assert_eq!(
        first.replayed_capsule.source_hash,
        second.replayed_capsule.source_hash
    );
    assert_eq!(
        first.replayed_capsule.pack.items,
        second.replayed_capsule.pack.items
    );
    assert_eq!(first.replay_difference, second.replay_difference);
    assert_eq!(first.replay_succeeded, second.replay_succeeded);
}

#[test]
fn replay_surfaces_score_drift_even_when_replayed_item_hash_matches() {
    let item_id = Uuid::from_u128(6);
    let original_item = retrieved_item(item_id, 0.42);
    let replay_item = retrieved_item(item_id, 0.77);
    let original = build_capsule(vec![original_item]);
    let record = handshake_core::memory::CapsuleRecord::from_capsule(
        &original,
        Utc::now(),
        "session-1",
        "KERNEL_BUILDER",
    );

    let result = replay(
        ReplayRequest {
            capsule_id: record.capsule_id,
            replay_as_of: as_of(120, 120),
            expected_source_hash: record.capsule_source_hash.clone(),
        },
        &TestRecordReader { record },
        &TestBitemporal {
            items: vec![bitemporal_item(replay_item, stamps(100, 50, None))],
        },
    )
    .expect("score drift replay returns diff");

    assert!(!result.replay_succeeded);
    assert_eq!(
        result.replay_difference.items_with_different_scores,
        vec![(item_id, 0.42, 0.77)]
    );
}

fn replay(
    request: ReplayRequest,
    reader: &dyn CapsuleRecordReader,
    bitemporal: &dyn BitemporalAccessor,
) -> Result<handshake_core::memory::replay_eval::ReplayResult, ReplayError> {
    let fems = FrozenFems { items: Vec::new() };
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);
    ReplayEvaluator::replay(request, reader, bitemporal, &builder)
}

fn build_capsule(items: Vec<RetrievedItem>) -> handshake_core::memory::MemoryCapsule {
    let fems = FrozenFems { items };
    let table = CapsulePolicyTable;
    CapsuleBuilder::new(&fems, &table)
        .build(BuildContext {
            task_type: TaskType::SelfImprovementLoopEval,
            query: "replay acceptance query".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-1".to_string(),
            override_policy: Some(policy()),
        })
        .expect("test capsule builds")
}

fn policy() -> RetrievalPolicy {
    RetrievalPolicy {
        top_k: 8,
        capsule_budget_bytes: 65_536,
        task_type: TaskType::SelfImprovementLoopEval,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    }
}

fn retrieved_item(item_id: Uuid, score: f64) -> RetrievedItem {
    let item_id = item_id.to_string();
    RetrievedItem {
        item_id: item_id.clone(),
        memory_class: "episodic".to_string(),
        item_type: "note".to_string(),
        summary: format!("summary {item_id}"),
        content: format!("content {item_id}"),
        structured: Some(Value::String("replay-eval".to_string())),
        trust_level: "trusted".to_string(),
        confidence: 0.88,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: format!("artifact-{item_id}"),
            hash: None,
            selector: Some("#acceptance".to_string()),
            created_at: None,
            classification: None,
        }],
        score,
        score_breakdown: BTreeMap::from([("similarity".to_string(), score)]),
        capsule_bytes: 256,
        token_estimate: 64,
        pinned: false,
    }
}

fn bitemporal_item(item: RetrievedItem, stamps: BitemporalStamps) -> BitemporalItem {
    BitemporalItem {
        item_id: Uuid::parse_str(&item.item_id).expect("test item id is uuid"),
        stamps,
        payload: serde_json::to_value(item).expect("retrieved item payload serializes"),
    }
}

fn stamps(valid_from: i64, recorded_at: i64, invalidated_at: Option<i64>) -> BitemporalStamps {
    BitemporalStamps {
        valid_from: at(valid_from),
        valid_until: None,
        recorded_at: at(recorded_at),
        invalidated_at: invalidated_at.map(at),
    }
}

fn as_of(world: i64, recorded: i64) -> AsOfQuery {
    AsOfQuery {
        as_of_world_time: at(world),
        as_of_recorded_time: at(recorded),
    }
}

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
}
