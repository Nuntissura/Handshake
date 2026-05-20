use std::collections::BTreeMap;

use handshake_core::ace::{
    FemsSourceRef, FemsSourceRefKind, MemoryPack, MemoryPackBudgets, MemoryPackDeterminismMode,
    MemoryPackItem, MemoryPolicy,
};
use handshake_core::memory::{
    DegradationTier, MemoryCapsule, RetrievalPolicy, TaskType, RETRIEVAL_SCORING_FORMULA_V0,
};

fn sample_pack(item_order: &[&str]) -> MemoryPack {
    let items = item_order
        .iter()
        .map(|id| sample_item(id))
        .collect::<Vec<_>>();

    MemoryPack {
        schema_version: "memory_pack.v1".to_string(),
        pack_id: "pack-1".to_string(),
        generated_at: "2026-05-19T00:00:00Z".to_string(),
        determinism_mode: MemoryPackDeterminismMode::Strict,
        memory_policy: MemoryPolicy::WorkspaceScoped,
        scope_refs: Vec::new(),
        budgets: MemoryPackBudgets {
            max_tokens: 4096,
            max_items: 10,
            max_items_per_type: BTreeMap::new(),
        },
        items,
        token_estimate: 128,
        memory_pack_hash: String::new(),
        warnings: Vec::new(),
    }
}

fn sample_item(id: &str) -> MemoryPackItem {
    MemoryPackItem {
        memory_id: id.to_string(),
        memory_class: "episodic".to_string(),
        item_type: "note".to_string(),
        summary: format!("summary {id}"),
        content: format!("content {id}"),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: 0.9,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: format!("artifact-{id}"),
            hash: None,
            selector: Some(format!("#{}", id)),
            created_at: None,
            classification: None,
        }],
        last_verified_at: Some("2026-05-19T00:00:00Z".to_string()),
    }
}

fn policy() -> RetrievalPolicy {
    RetrievalPolicy {
        top_k: 5,
        capsule_budget_bytes: 8192,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Tiered,
    }
}

#[test]
fn memory_capsule_new_mints_uuid_v7_and_allows_empty_pack() {
    let capsule = MemoryCapsule::new(
        TaskType::KernelBuilderMtImplementation,
        sample_pack(&[]),
        policy(),
    )
    .unwrap();

    assert_eq!(capsule.id.get_version_num(), 7);
    assert_eq!(capsule.task_type, TaskType::KernelBuilderMtImplementation);
    assert!(capsule.pack.items.is_empty());
    assert!(capsule.audit.entries.is_empty());
    assert_eq!(capsule.source_hash.len(), 64);
}

#[test]
fn memory_capsule_source_hash_is_deterministic_for_equivalent_pack_items() {
    let first = MemoryCapsule::new(
        TaskType::KernelBuilderMtImplementation,
        sample_pack(&["mem-a", "mem-b"]),
        policy(),
    )
    .unwrap();
    let second = MemoryCapsule::new(
        TaskType::KernelBuilderMtImplementation,
        sample_pack(&["mem-b", "mem-a"]),
        policy(),
    )
    .unwrap();

    assert_eq!(first.source_hash, second.source_hash);
}

#[test]
fn memory_capsule_suppress_item_updates_audit_without_mutating_pack() {
    let mut capsule = MemoryCapsule::new(
        TaskType::KernelBuilderMtImplementation,
        sample_pack(&["mem-a"]),
        policy(),
    )
    .unwrap();
    let original_pack = capsule.pack.clone();

    capsule
        .suppress_item("mem-a", "stale source evidence")
        .unwrap();

    assert_eq!(capsule.pack, original_pack);
    let entry = capsule.audit.entry("mem-a").unwrap();
    assert_eq!(entry.item_id, "mem-a");
    assert!(!entry.included);
    assert_eq!(
        entry.suppression_reason.as_deref(),
        Some("stale source evidence")
    );
    assert!(entry.source_uri.contains("artifact-mem-a"));
}

#[test]
fn retrieval_policy_round_trips_through_json() {
    let original = policy();

    let json = serde_json::to_string(&original).unwrap();
    let decoded: RetrievalPolicy = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, original);
}
