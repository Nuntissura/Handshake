use super::capsule::{DegradationTier, RetrievalPolicy, TaskType};

pub const RETRIEVAL_SCORING_FORMULA_V0: &str = "retrieval_scoring_formula_v0";

#[derive(Debug, Clone, Copy, Default)]
pub struct CapsulePolicyTable;

impl CapsulePolicyTable {
    pub fn task_types() -> &'static [TaskType] {
        TaskType::all()
    }

    pub fn policy_for(&self, task_type: TaskType) -> RetrievalPolicy {
        Self::default_policy_for(task_type)
    }

    pub fn default_policy_for(task_type: TaskType) -> RetrievalPolicy {
        let (top_k, capsule_budget_bytes, graceful_degradation_tier) = match task_type {
            TaskType::ValidatorHbrTestPacket => (6, 32_768, DegradationTier::Strict),
            TaskType::KernelBuilderMtImplementation => (12, 65_536, DegradationTier::Tiered),
            TaskType::IntegrationValidatorBatchReview => (16, 131_072, DegradationTier::Tiered),
            TaskType::OperatorTriage => (8, 49_152, DegradationTier::Strict),
            TaskType::SwarmHarnessSession => (4, 16_384, DegradationTier::Aggressive),
            TaskType::ProcessLedgerInspection => (8, 32_768, DegradationTier::Strict),
            TaskType::SelfImprovementLoopEval => (6, 32_768, DegradationTier::Strict),
            TaskType::GeneralRetrieval => (10, 49_152, DegradationTier::Tiered),
        };

        RetrievalPolicy {
            top_k,
            capsule_budget_bytes,
            task_type,
            scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
            graceful_degradation_tier,
        }
    }
}
