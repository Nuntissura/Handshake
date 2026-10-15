use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingPolicy {
    LocalFirst,
    CloudReview,
    CloudPlanLocalExecute,
    ParallelDebate,
    ValidatorLane,
    OperatorLane,
}

impl ModelLaneRoutingPolicy {
    pub fn all() -> &'static [Self] {
        &[
            Self::LocalFirst,
            Self::CloudReview,
            Self::CloudPlanLocalExecute,
            Self::ParallelDebate,
            Self::ValidatorLane,
            Self::OperatorLane,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalFirst => "local_first",
            Self::CloudReview => "cloud_review",
            Self::CloudPlanLocalExecute => "cloud_plan_local_execute",
            Self::ParallelDebate => "parallel_debate",
            Self::ValidatorLane => "validator_lane",
            Self::OperatorLane => "operator_lane",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingDispatchTarget {
    LocalModel,
    CloudModel,
    Validator,
    Operator,
    CoordinatorJoin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingStageActivation {
    Always,
    AfterSuccess,
    AfterFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingAuthorityGate {
    None,
    CloudConsent,
    ValidatorAuthority,
    OperatorAuthority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingStageOutcome {
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingStage {
    pub stage_id: String,
    pub target: ModelLaneRoutingDispatchTarget,
    pub activation: ModelLaneRoutingStageActivation,
    pub depends_on: Vec<String>,
    pub authority_gate: ModelLaneRoutingAuthorityGate,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingAuthority {
    pub cloud_consent_receipt_ref: Option<String>,
    pub validator_authority_ref: Option<String>,
    pub operator_authority_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingGraph {
    pub schema_id: String,
    pub policy: ModelLaneRoutingPolicy,
    pub stages: Vec<ModelLaneRoutingStage>,
}

impl ModelLaneRoutingGraph {
    pub const SCHEMA_ID: &'static str = "hsk.model_lane_routing_graph@1";

    pub fn for_policy(policy: ModelLaneRoutingPolicy) -> Self {
        let stages = match policy {
            ModelLaneRoutingPolicy::LocalFirst => vec![
                routing_stage(
                    "local-attempt",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::None,
                ),
                routing_stage(
                    "cloud-escalation",
                    ModelLaneRoutingDispatchTarget::CloudModel,
                    ModelLaneRoutingStageActivation::AfterFailure,
                    &["local-attempt"],
                    ModelLaneRoutingAuthorityGate::CloudConsent,
                ),
            ],
            ModelLaneRoutingPolicy::CloudReview => vec![
                routing_stage(
                    "local-candidate",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::None,
                ),
                routing_stage(
                    "cloud-review",
                    ModelLaneRoutingDispatchTarget::CloudModel,
                    ModelLaneRoutingStageActivation::AfterSuccess,
                    &["local-candidate"],
                    ModelLaneRoutingAuthorityGate::CloudConsent,
                ),
            ],
            ModelLaneRoutingPolicy::CloudPlanLocalExecute => vec![
                routing_stage(
                    "cloud-plan",
                    ModelLaneRoutingDispatchTarget::CloudModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::CloudConsent,
                ),
                routing_stage(
                    "local-execute",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::AfterSuccess,
                    &["cloud-plan"],
                    ModelLaneRoutingAuthorityGate::None,
                ),
            ],
            ModelLaneRoutingPolicy::ParallelDebate => vec![
                routing_stage(
                    "debate-local",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::None,
                ),
                routing_stage(
                    "debate-cloud",
                    ModelLaneRoutingDispatchTarget::CloudModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::CloudConsent,
                ),
                routing_stage(
                    "debate-join",
                    ModelLaneRoutingDispatchTarget::CoordinatorJoin,
                    ModelLaneRoutingStageActivation::AfterSuccess,
                    &["debate-local", "debate-cloud"],
                    ModelLaneRoutingAuthorityGate::None,
                ),
            ],
            ModelLaneRoutingPolicy::ValidatorLane => vec![
                routing_stage(
                    "validation-candidate",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::None,
                ),
                routing_stage(
                    "validator-verdict",
                    ModelLaneRoutingDispatchTarget::Validator,
                    ModelLaneRoutingStageActivation::AfterSuccess,
                    &["validation-candidate"],
                    ModelLaneRoutingAuthorityGate::ValidatorAuthority,
                ),
            ],
            ModelLaneRoutingPolicy::OperatorLane => vec![
                routing_stage(
                    "operator-candidate",
                    ModelLaneRoutingDispatchTarget::LocalModel,
                    ModelLaneRoutingStageActivation::Always,
                    &[],
                    ModelLaneRoutingAuthorityGate::None,
                ),
                routing_stage(
                    "operator-decision",
                    ModelLaneRoutingDispatchTarget::Operator,
                    ModelLaneRoutingStageActivation::AfterSuccess,
                    &["operator-candidate"],
                    ModelLaneRoutingAuthorityGate::OperatorAuthority,
                ),
            ],
        };
        Self {
            schema_id: Self::SCHEMA_ID.to_string(),
            policy,
            stages,
        }
    }

    pub fn validate(&self) -> Result<(), SwarmRoutingError> {
        if self.schema_id != Self::SCHEMA_ID {
            return Err(SwarmRoutingError::InvalidRoutingGraph {
                reason: format!("unsupported schema_id {}", self.schema_id),
            });
        }
        if self.stages.is_empty() {
            return Err(SwarmRoutingError::InvalidRoutingGraph {
                reason: "routing graph has no stages".to_string(),
            });
        }
        let mut seen = BTreeSet::new();
        for stage in &self.stages {
            if stage.stage_id.trim().is_empty() || !seen.insert(stage.stage_id.clone()) {
                return Err(SwarmRoutingError::InvalidRoutingGraph {
                    reason: format!("blank or duplicate stage_id {}", stage.stage_id),
                });
            }
            for dependency in &stage.depends_on {
                if !seen.contains(dependency) {
                    return Err(SwarmRoutingError::InvalidRoutingGraph {
                        reason: format!(
                            "stage {} depends on missing or forward stage {}",
                            stage.stage_id, dependency
                        ),
                    });
                }
            }
        }
        let expected = Self::for_policy(self.policy);
        if self != &expected {
            return Err(SwarmRoutingError::InvalidRoutingGraph {
                reason: format!(
                    "policy {} must use its exact canonical graph; arbitrary policy-labelled DAGs are forbidden",
                    self.policy.as_str()
                ),
            });
        }
        Ok(())
    }

    pub fn require_authority_contract(
        &self,
        authority: &ModelLaneRoutingAuthority,
    ) -> Result<(), SwarmRoutingError> {
        self.validate()?;
        for stage in &self.stages {
            ensure_routing_stage_authority(stage, authority)?;
        }
        Ok(())
    }

    pub fn ready_stages<'a>(
        &'a self,
        completed: &BTreeMap<String, ModelLaneRoutingStageOutcome>,
        authority: &ModelLaneRoutingAuthority,
    ) -> Result<Vec<&'a ModelLaneRoutingStage>, SwarmRoutingError> {
        self.validate()?;
        let mut ready = Vec::new();
        for stage in &self.stages {
            if completed.contains_key(&stage.stage_id) {
                continue;
            }
            if stage
                .depends_on
                .iter()
                .any(|dependency| !completed.contains_key(dependency))
            {
                continue;
            }
            let activated = match stage.activation {
                ModelLaneRoutingStageActivation::Always => true,
                ModelLaneRoutingStageActivation::AfterSuccess => {
                    stage.depends_on.iter().all(|dependency| {
                        completed.get(dependency) == Some(&ModelLaneRoutingStageOutcome::Succeeded)
                    })
                }
                ModelLaneRoutingStageActivation::AfterFailure => {
                    stage.depends_on.iter().any(|dependency| {
                        completed.get(dependency) == Some(&ModelLaneRoutingStageOutcome::Failed)
                    })
                }
            };
            if !activated {
                continue;
            }
            ensure_routing_stage_authority(stage, authority)?;
            ready.push(stage);
        }
        Ok(ready)
    }
}

fn routing_stage(
    stage_id: &str,
    target: ModelLaneRoutingDispatchTarget,
    activation: ModelLaneRoutingStageActivation,
    depends_on: &[&str],
    authority_gate: ModelLaneRoutingAuthorityGate,
) -> ModelLaneRoutingStage {
    ModelLaneRoutingStage {
        stage_id: stage_id.to_string(),
        target,
        activation,
        depends_on: depends_on
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        authority_gate,
    }
}

fn ensure_routing_stage_authority(
    stage: &ModelLaneRoutingStage,
    authority: &ModelLaneRoutingAuthority,
) -> Result<(), SwarmRoutingError> {
    let present = match stage.authority_gate {
        ModelLaneRoutingAuthorityGate::None => true,
        ModelLaneRoutingAuthorityGate::CloudConsent => authority
            .cloud_consent_receipt_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        ModelLaneRoutingAuthorityGate::ValidatorAuthority => authority
            .validator_authority_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        ModelLaneRoutingAuthorityGate::OperatorAuthority => authority
            .operator_authority_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    };
    if present {
        Ok(())
    } else {
        Err(SwarmRoutingError::RoutingAuthorityMissing {
            stage_id: stage.stage_id.clone(),
            authority: format!("{:?}", stage.authority_gate),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SwarmRoutingError {
    #[error("ROUTING_NO_LOCAL_MODEL: a local-tier route was selected but the request carries no explicit local_model")]
    NoLocalModel,
    #[error("ROUTING_NO_CLOUD_MODEL: a cloud-tier route was selected but the request carries no explicit cloud_model")]
    NoCloudModel,
    #[error("ROUTING_NO_CLOUD_PROVIDER: a cloud-tier route was selected but no cloud provider is configured in the policy")]
    NoCloudProvider,
    #[error("ROUTING_ALL_LANES_SUPPRESSED: every admissible lane (local + all configured cloud providers) is breaker-suppressed; cooldown_remaining_ms={cooldown_remaining_ms}")]
    AllLanesSuppressed { cooldown_remaining_ms: u128 },
    #[error("ROUTING_GRAPH_INVALID: {reason}")]
    InvalidRoutingGraph { reason: String },
    #[error("ROUTING_AUTHORITY_MISSING: stage_id={stage_id}; authority={authority}")]
    RoutingAuthorityMissing { stage_id: String, authority: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_model_lane_policies_have_distinct_valid_executable_graphs() {
        let authority = ModelLaneRoutingAuthority {
            cloud_consent_receipt_ref: Some("consent://routing-test".into()),
            validator_authority_ref: Some("validator://routing-test".into()),
            operator_authority_ref: Some("operator://routing-test".into()),
        };
        let mut canonical_graphs = BTreeSet::new();
        for policy in ModelLaneRoutingPolicy::all().iter().copied() {
            let graph = ModelLaneRoutingGraph::for_policy(policy);
            graph.validate().expect("canonical graph validates");
            graph
                .require_authority_contract(&authority)
                .expect("complete authority satisfies canonical graph");
            let ready = graph
                .ready_stages(&BTreeMap::new(), &authority)
                .expect("initial stages are executable");
            assert!(
                !ready.is_empty(),
                "{policy:?} must dispatch an initial stage"
            );
            canonical_graphs.insert(serde_json::to_string(&graph).expect("serialize graph"));
        }
        assert_eq!(
            canonical_graphs.len(),
            ModelLaneRoutingPolicy::all().len(),
            "all six policies must persist distinct executable dispatch graphs"
        );
    }

    #[test]
    fn routing_graphs_fail_closed_at_cloud_validator_and_operator_gates() {
        let empty = ModelLaneRoutingAuthority::default();
        for policy in [
            ModelLaneRoutingPolicy::LocalFirst,
            ModelLaneRoutingPolicy::CloudReview,
            ModelLaneRoutingPolicy::CloudPlanLocalExecute,
            ModelLaneRoutingPolicy::ParallelDebate,
        ] {
            let err = ModelLaneRoutingGraph::for_policy(policy)
                .require_authority_contract(&empty)
                .expect_err("cloud-bearing graph requires cloud consent");
            assert!(matches!(
                err,
                SwarmRoutingError::RoutingAuthorityMissing { .. }
            ));
        }
        for policy in [
            ModelLaneRoutingPolicy::ValidatorLane,
            ModelLaneRoutingPolicy::OperatorLane,
        ] {
            let err = ModelLaneRoutingGraph::for_policy(policy)
                .require_authority_contract(&empty)
                .expect_err("authority lane cannot silently model-fallback");
            assert!(matches!(
                err,
                SwarmRoutingError::RoutingAuthorityMissing { .. }
            ));
        }
    }

    #[test]
    fn local_first_escalates_only_after_failure_and_parallel_debate_joins_both_outputs() {
        let authority = ModelLaneRoutingAuthority {
            cloud_consent_receipt_ref: Some("consent://routing-test".into()),
            ..ModelLaneRoutingAuthority::default()
        };
        let local_first = ModelLaneRoutingGraph::for_policy(ModelLaneRoutingPolicy::LocalFirst);
        let mut completed = BTreeMap::new();
        assert_eq!(
            local_first.ready_stages(&completed, &authority).unwrap()[0].stage_id,
            "local-attempt"
        );
        completed.insert(
            "local-attempt".to_string(),
            ModelLaneRoutingStageOutcome::Failed,
        );
        assert_eq!(
            local_first.ready_stages(&completed, &authority).unwrap()[0].stage_id,
            "cloud-escalation"
        );

        let debate = ModelLaneRoutingGraph::for_policy(ModelLaneRoutingPolicy::ParallelDebate);
        let initial = debate.ready_stages(&BTreeMap::new(), &authority).unwrap();
        assert_eq!(initial.len(), 2, "debate participants dispatch in parallel");
        let mut debate_completed = BTreeMap::new();
        debate_completed.insert(
            "debate-local".to_string(),
            ModelLaneRoutingStageOutcome::Succeeded,
        );
        assert!(debate
            .ready_stages(&debate_completed, &authority)
            .unwrap()
            .is_empty());
        debate_completed.insert(
            "debate-cloud".to_string(),
            ModelLaneRoutingStageOutcome::Succeeded,
        );
        assert_eq!(
            debate.ready_stages(&debate_completed, &authority).unwrap()[0].target,
            ModelLaneRoutingDispatchTarget::CoordinatorJoin
        );
    }
}
