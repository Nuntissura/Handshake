use handshake_core::memory::{
    CapsulePolicyTable, DegradationTier, RetrievalPolicy, TaskType, RETRIEVAL_SCORING_FORMULA_V0,
};

#[test]
fn policy_table_has_defaults_for_every_closed_task_type() {
    let task_types = CapsulePolicyTable::task_types();

    assert_eq!(task_types.len(), 8);
    for task_type in task_types {
        let policy = CapsulePolicyTable::default_policy_for(*task_type);
        assert_eq!(policy.task_type, *task_type);
        assert!(policy.top_k > 0);
        assert!(policy.capsule_budget_bytes > 0);
        assert_eq!(policy.scoring_formula_version, RETRIEVAL_SCORING_FORMULA_V0);
    }
}

#[test]
fn policy_table_defaults_match_mt142_contract_values() {
    assert_policy(
        TaskType::ValidatorHbrTestPacket,
        6,
        32_768,
        DegradationTier::Strict,
    );
    assert_policy(
        TaskType::KernelBuilderMtImplementation,
        12,
        65_536,
        DegradationTier::Tiered,
    );
    assert_policy(
        TaskType::IntegrationValidatorBatchReview,
        16,
        131_072,
        DegradationTier::Tiered,
    );
    assert_policy(TaskType::OperatorTriage, 8, 49_152, DegradationTier::Strict);
    assert_policy(
        TaskType::SwarmHarnessSession,
        4,
        16_384,
        DegradationTier::Aggressive,
    );
    assert_policy(
        TaskType::ProcessLedgerInspection,
        8,
        32_768,
        DegradationTier::Strict,
    );
    assert_policy(
        TaskType::SelfImprovementLoopEval,
        6,
        32_768,
        DegradationTier::Strict,
    );
    assert_policy(
        TaskType::GeneralRetrieval,
        10,
        49_152,
        DegradationTier::Tiered,
    );
}

#[test]
fn policy_table_defaults_round_trip_through_stable_json_names() {
    let policies = CapsulePolicyTable::task_types()
        .iter()
        .map(|task_type| CapsulePolicyTable::default_policy_for(*task_type))
        .collect::<Vec<_>>();

    let json = serde_json::to_string(&policies).unwrap();
    assert!(json.contains("kernel_builder_mt_implementation"));
    assert!(json.contains("retrieval_scoring_formula_v0"));

    let decoded: Vec<RetrievalPolicy> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, policies);
}

#[test]
fn task_type_deserialization_rejects_unknown_variants() {
    let unknown = serde_json::from_str::<TaskType>("\"unknown\"");

    assert!(unknown.is_err());
}

#[test]
fn policy_table_budget_bytes_are_16k_aligned() {
    for task_type in CapsulePolicyTable::task_types() {
        let policy = CapsulePolicyTable::default_policy_for(*task_type);

        assert_eq!(
            policy.capsule_budget_bytes % 16_384,
            0,
            "budget must stay 16 KiB aligned for {:?}",
            task_type
        );
    }
}

fn assert_policy(
    task_type: TaskType,
    top_k: u32,
    capsule_budget_bytes: u64,
    graceful_degradation_tier: DegradationTier,
) {
    let policy = CapsulePolicyTable::default_policy_for(task_type);

    assert_eq!(policy.top_k, top_k);
    assert_eq!(policy.capsule_budget_bytes, capsule_budget_bytes);
    assert_eq!(policy.graceful_degradation_tier, graceful_degradation_tier);
}
