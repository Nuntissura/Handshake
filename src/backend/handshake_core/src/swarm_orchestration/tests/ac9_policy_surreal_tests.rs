//! AC9 structural denial checks. Executable-wave acceptance additionally requires
//! canonically resolved promotion and policy-specific authority evidence.

use crate::swarm_orchestration::production_factory::execute_production_routing_wave;
use crate::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterKind, DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest,
    ModelLaneError, ModelLanePromotionOutcome, ModelLaneResult,
};
use crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionContext;
use crate::swarm_orchestration::routing::{
    ModelLaneRoutingDispatchTarget, ModelLaneRoutingGraph, ModelLaneRoutingPolicy,
    ModelLaneRoutingStageLaunchPlan, SwarmRoutingError,
};

use super::ac9_lifecycle_surreal_tests::{artifact_workspace, Ac9Harness};

fn expected_stage_ids(policy: ModelLaneRoutingPolicy) -> &'static [&'static str] {
    match policy {
        ModelLaneRoutingPolicy::LocalFirst => &["local-attempt", "cloud-escalation"],
        ModelLaneRoutingPolicy::CloudReview => &["local-candidate", "cloud-review"],
        ModelLaneRoutingPolicy::CloudPlanLocalExecute => &["cloud-plan", "local-execute"],
        ModelLaneRoutingPolicy::ParallelDebate => &["debate-local", "debate-cloud", "debate-join"],
        ModelLaneRoutingPolicy::ValidatorLane => &["validation-candidate", "validator-verdict"],
        ModelLaneRoutingPolicy::OperatorLane => &["operator-candidate", "operator-decision"],
    }
}

/// Producer-owned inputs retain their real consent, projection, process and memory
/// bindings. These request prototypes confer no routing authority by themselves.
pub(super) struct Ac9StageRequestInputs {
    pub(super) local: DexterityLaunchAdapterRequest,
    pub(super) cloud: Option<DexterityLaunchAdapterRequest>,
    pub(super) validator: Option<DexterityLaunchAdapterRequest>,
    pub(super) operator: Option<DexterityLaunchAdapterRequest>,
}

fn bind_stage_request(
    request: &mut DexterityLaunchAdapterRequest,
    context: &ModelLaneRoutingExecutionContext,
    policy: ModelLaneRoutingPolicy,
    stage_id: &str,
) {
    request.run_id = context.run_id.clone();
    request.trace_id = context.trace_id.clone();
    request.run_span_id = context.run_span_id.clone();
    request.coordinator_session_id = context.coordinator_session_id.clone();
    request.routing_policy = policy.as_str().to_string();
    request.work_packet_id = Some(context.work_packet_id.clone());
    request.micro_task_id = context.micro_task_id.clone();
    request.task_board_id = Some(context.task_board_id.clone());
    request.owner_session = context.owner_session.clone();
    request.locus_binding_ref = context.locus_ref.clone();
    request.lane_id = format!("{}:{}", context.run_id, stage_id);
    request.lane_span_id = format!("{}:{}", context.run_span_id, stage_id);
    request.role = stage_id.to_string();
}

impl Ac9StageRequestInputs {
    /// Registers the explicitly synthetic in-process local participant. Its zero
    /// numeric handle is not an OS PID, so reclamation cannot target a host process.
    pub(super) fn register_in_process_local(
        &mut self,
        context: &ModelLaneRoutingExecutionContext,
        policy: ModelLaneRoutingPolicy,
        ledger: &crate::process_ledger::LedgerBatcher,
    ) -> Result<crate::process_ledger::ProcessOwnershipRecordId, crate::process_ledger::ProcessLedgerError> {
        let stage = ModelLaneRoutingGraph::for_policy(policy).stages.into_iter()
            .find(|stage| stage.target == ModelLaneRoutingDispatchTarget::LocalModel)
            .expect("each of the six AC9 policies has a local model stage");
        bind_stage_request(&mut self.local, context, policy, &stage.stage_id);
        let mut meta = crate::process_ledger::SpawnMeta::new(
            0,
            crate::process_ledger::ProcessEngineKind::LlamaCpp,
            "synthetic-ac9-in-process",
        );
        meta.pid_authoritative = false;
        meta.model_id = self.local.model_id.clone();
        meta.runtime_binding = Some("local".into());
        meta.session_id = Some(self.local.model_session_id.clone());
        meta.parent_session_id = Some(self.local.coordinator_session_id.clone());
        meta.trace_id = Some(self.local.trace_id.clone());
        meta.span_id = Some(self.local.lane_span_id.clone());
        meta.owner_wp = self.local.work_packet_id.clone();
        meta.wp_id = self.local.work_packet_id.clone();
        meta.mt_id = self.local.micro_task_id.clone();
        meta.role_id = Some(self.local.role.clone());
        meta.metadata_blob = serde_json::json!({
            "synthetic": true,
            "network_egress": false,
            "run_id": self.local.run_id,
            "lane_id": self.local.lane_id,
        });
        let record_id = crate::process_ledger::record_spawn(ledger, meta)?;
        self.local.process_ownership_ref = Some(format!(
            "process-ledger://{}", record_id.as_uuid(),
        ));
        Ok(record_id)
    }

    /// Builds actual pack bytes and their canonical hash through the public memory
    /// builder. Persistence, scope acceptance and process ownership remain separate.
    pub(super) fn build_memory_pack(
        &mut self,
        retriever: &dyn crate::memory::FemsRetriever,
        context: &ModelLaneRoutingExecutionContext,
    ) -> Result<crate::memory::MemoryCapsule, crate::memory::BuilderError> {
        let policy = crate::memory::CapsulePolicyTable::default();
        let capsule = crate::memory::CapsuleBuilder::new(retriever, &policy).build(
            crate::memory::BuildContext {
                task_type: crate::memory::TaskType::SwarmHarnessSession,
                query: context.run_id.clone(),
                role_id: "ac9-policy-wave".into(),
                session_id: context.coordinator_session_id.clone(),
                override_policy: None,
            },
        )?;
        self.local.memory_pack_ref = capsule.pack.pack_id.clone();
        self.local.memory_pack_hash = capsule.pack.memory_pack_hash.clone();
        for request in [&mut self.cloud, &mut self.validator, &mut self.operator]
            .into_iter().flatten()
        {
            request.memory_pack_ref = capsule.pack.pack_id.clone();
            request.memory_pack_hash = capsule.pack.memory_pack_hash.clone();
        }
        Ok(capsule)
    }
}
pub(super) fn policy_stage_requests(
    context: &ModelLaneRoutingExecutionContext,
    policy: ModelLaneRoutingPolicy,
    inputs: &Ac9StageRequestInputs,
) -> ModelLaneResult<Vec<(ModelLaneRoutingStageLaunchPlan, DexterityLaunchAdapterRequest)>> {
    use ModelLaneRoutingDispatchTarget as Target;
    let registry = DexterityLaunchAdapterRegistry::standard();
    let mut requests = Vec::new();
    for stage in ModelLaneRoutingGraph::for_policy(policy).stages {
        let (prototype, expected_kind, provider) = match stage.target {
            Target::LocalModel => (&inputs.local, DexterityLaunchAdapterKind::LocalModelRuntime, None),
            Target::CloudModel => (
                inputs.cloud.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    format!("AC9 {} requires producer-backed cloud launch inputs", policy.as_str()),
                ))?,
                DexterityLaunchAdapterKind::ByokCloudOpenAi,
                Some(crate::model_runtime::ProviderKind::ByokCloud),
            ),
            Target::Validator => (
                inputs.validator.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    "AC9 validator lane requires producer-backed validator inputs".into(),
                ))?,
                DexterityLaunchAdapterKind::Validator,
                None,
            ),
            Target::Operator => (
                inputs.operator.as_ref().ok_or_else(|| ModelLaneError::InvalidInput(
                    "AC9 operator lane requires producer-backed operator inputs".into(),
                ))?,
                DexterityLaunchAdapterKind::HumanOperator,
                None,
            ),
            Target::CoordinatorJoin => continue,
        };
        if prototype.adapter_kind != expected_kind {
            return Err(ModelLaneError::InvalidInput(format!(
                "AC9 {} requires {} input, received {}",
                stage.stage_id, expected_kind.as_str(), prototype.adapter_kind.as_str(),
            )));
        }
        let mut request = prototype.clone();
        bind_stage_request(&mut request, context, policy, &stage.stage_id);
        let normalized = registry.normalize(request.clone())?;
        let (_, lane) = normalized.to_records()?;
        requests.push((ModelLaneRoutingStageLaunchPlan {
            stage_id: stage.stage_id,
            dispatch_target: stage.target,
            lane_id: Some(lane.lane_id),
            model_id: lane.model_id,
            provider,
        }, request));
    }
    Ok(requests)
}
#[tokio::test]
async fn ac9_production_rejects_unresolved_receipt_with_forged_or_cross_run_authority() {
    let mut harness = Ac9Harness::create(
        "policy-authority-denials",
        artifact_workspace("policy-authority-denials"),
    )
    .await;
    let observations = harness
        .run_with_cleanup(|harness| Box::pin(async move {
            let first = harness
                .produce_gate_evidence()
                .await
                .expect("produce and canonically reread first gate evidence");
            let second = harness
                .produce_gate_evidence()
                .await
                .expect("produce and canonically reread independent gate evidence");
            assert_ne!(first.gate.event_id, second.gate.event_id);
            assert_ne!(first.operator_review_receipt_id, second.operator_review_receipt_id);

            let mut observations = Vec::new();
            for case in [
                "unresolved_gate",
                "cross_run_gate",
                "cross_run_operator_review",
                "missing_operator",
            ] {
                let mut candidate = harness.prepare_unresolved_receipt_candidate(&first).await;
                // This baseline deliberately retains an unresolved promotion receipt.
                // These are combined authority denials, not gate-only causal proof or
                // a claim that a kernel event is itself a promotion-receipt object.
                match case {
                    "unresolved_gate" => {
                        candidate.promotion_gate_ref = format!("unresolved-ac9-gate-{}", uuid::Uuid::now_v7());
                    }
                    "cross_run_gate" => candidate.promotion_gate_ref = second.gate.event_id.clone(),
                    "cross_run_operator_review" => {
                        candidate.operator_authority_ref = Some(second.operator_review_receipt_id.clone());
                    }
                    "missing_operator" => candidate.operator_authority_ref = None,
                    _ => unreachable!("closed authority-denial cases"),
                }

                let before = harness
                    .store()
                    .replay_promotion_decisions(&candidate.run_id)
                    .await
                    .expect("read actual promotion authority before negative");
                let run_id = candidate.run_id.clone();
                let result = harness.store().record_promotion_decision(candidate).await;
                let expected_denial = match &result {
                    Err(ModelLaneError::InvalidInput(message)) if case == "missing_operator" => {
                        message == "ROUTING_AUTHORITY_MISSING: stage_id=operator-decision; authority=OperatorAuthority"
                    }
                    Ok(record) if case != "missing_operator" => {
                        record.outcome == ModelLanePromotionOutcome::Denied
                    }
                    _ => false,
                };
                let accepted = matches!(
                    &result,
                    Ok(record) if record.outcome == ModelLanePromotionOutcome::Approved
                );
                let after = harness
                    .store()
                    .replay_promotion_decisions(&run_id)
                    .await
                    .expect("read actual promotion authority after production transition");
                observations.push((
                    case,
                    before.is_empty(),
                    expected_denial,
                    accepted,
                    after.iter().any(|record| record.outcome == ModelLanePromotionOutcome::Approved),
                    format!("{result:?}"),
                ));
            }
            observations
        }))
        .await;

    for (case, initially_empty, expected_denial, accepted, persisted_approval, result) in observations {
        assert!(initially_empty, "{case}: no prior promotion authority may mask the denial");
        assert!(expected_denial, "{case}: require a typed Denied decision or the exact missing-operator error; unrelated errors are not authority-denial proof: {result}");
        assert!(!accepted, "{case}: production must reject unresolved authority: {result}");
        assert!(
            !persisted_approval,
            "{case}: production must not persist approved authority for unresolved evidence"
        );
    }
}

#[test]
fn ac9_six_policy_graphs_reject_reordering_relabelling_and_removed_gates() {
    let _production_entrypoint_compile_anchor = execute_production_routing_wave;
    let policies = ModelLaneRoutingPolicy::all();
    assert_eq!(policies.len(), 6, "AC9 retains all six policy contracts");

    for (index, policy) in policies.iter().copied().enumerate() {
        let canonical = ModelLaneRoutingGraph::for_policy(policy);
        canonical.validate().expect("canonical policy graph validates");
        assert_eq!(
            canonical
                .stages
                .iter()
                .map(|stage| stage.stage_id.as_str())
                .collect::<Vec<_>>(),
            expected_stage_ids(policy),
            "the expected policy stages are explicit, not derived from the returned graph"
        );

        let mut reordered = canonical.clone();
        reordered.stages.reverse();
        assert!(
            matches!(reordered.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject reordered stages",
            policy.as_str()
        );

        let mut relabelled = canonical.clone();
        relabelled.policy = policies[(index + 1) % policies.len()];
        assert!(
            matches!(relabelled.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "changing the policy label cannot authorize another policy's stages"
        );

        let mut duplicate = canonical.clone();
        duplicate.stages.push(canonical.stages[0].clone());
        assert!(
            matches!(duplicate.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject a duplicated stage",
            policy.as_str()
        );

        let mut missing = canonical.clone();
        missing.stages.pop();
        assert!(
            matches!(missing.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject a missing stage",
            policy.as_str()
        );

        let mut ungated = canonical.clone();
        let gated = ungated
            .stages
            .iter_mut()
            .find(|stage| {
                stage.authority_gate
                    != crate::swarm_orchestration::routing::ModelLaneRoutingAuthorityGate::None
            })
            .expect("each policy has a protected cloud, validator, or operator stage");
        gated.authority_gate =
            crate::swarm_orchestration::routing::ModelLaneRoutingAuthorityGate::None;
        assert!(
            matches!(ungated.validate(), Err(SwarmRoutingError::InvalidRoutingGraph { .. })),
            "{} must reject removal of its authority gate",
            policy.as_str()
        );
    }
}
