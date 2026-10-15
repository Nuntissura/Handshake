//! Local-vs-cloud routing policy for the swarm coordinator (MT-206).
//!
//! This is the deterministic classifier the operator wants: given a task
//! descriptor it picks a TIER — [`TaskTier::Local`] for routine / classification
//! / short work, [`TaskTier::Cloud`] for hard / long work — and an EXPLICIT
//! per-request model, then escalates / falls back under failure subject to a
//! per-provider circuit breaker.
//!
//! It is deliberately PLUGGABLE and free of any candle / reqwest / vault
//! specifics: it consumes a [`RoutingRequest`] (pure data) and emits a
//! [`RoutingDecision`] naming the lane the coordinator should spawn through. The
//! caller turns the decision into a concrete [`super::ids::SpawnRequest`]
//! (`with_local_artifact` for a local target, `with_cloud_provider` for a cloud
//! target). The policy never spawns anything itself, so it is trivially
//! unit-testable and swappable.
//!
//! ## Tiers, escalation, and fallback
//!
//! - **Classify**: a deterministic, threshold-based classifier maps the task to
//!   a tier. Short / routine / classification work stays LOCAL (cheap, private,
//!   no per-token cost); long / hard / explicitly-cloud work goes CLOUD.
//! - **Escalate**: when a local attempt fails or reports low confidence, the
//!   policy escalates the SAME task to the cloud tier (the next [`route`] call
//!   with `local_outcome = Some(Escalate*)` returns a cloud decision) — this is
//!   the "small model tries first, hard cases go up" pattern.
//! - **Fallback**: when a cloud provider fails, the policy records the failure
//!   against that provider's circuit breaker. Once a provider's breaker trips,
//!   further routes to that provider are suppressed and the policy falls back to
//!   the other configured cloud provider, or back to local if no cloud provider
//!   is admissible.
//!
//! ## Circuit breaker reuse
//!
//! Per-provider breakers reuse the EXACT [`super::breaker::FailureFingerprintBreaker`]
//! the coordinator uses for spawn suppression — one breaker instance per
//! provider key (`local`, `anthropic`, `openai`). The fingerprint folds the
//! provider into the signature so a systemic provider outage (e.g. repeated 5xx
//! from one vendor) trips only that vendor's breaker, leaving the others — and
//! the local lane — admissible. This mirrors the field-standard
//! Closed -> Open -> Half-Open breaker (Nygard / resilience4j / Polly) keyed per
//! downstream, which is exactly the per-provider isolation the operator asked
//! for.


use serde::{Deserialize, Serialize};

pub use handshake_model_lane_routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingAuthorityGate, ModelLaneRoutingDispatchTarget,
    ModelLaneRoutingGraph, ModelLaneRoutingPolicy, ModelLaneRoutingStage,
    ModelLaneRoutingStageActivation, ModelLaneRoutingStageOutcome, SwarmRoutingError,
};

pub use handshake_swarm_routing_policy::{
    CloudOutcome, CloudProvider, LocalOutcome, RoutingDecision, RoutingPolicy, RoutingPolicyConfig,
    RoutingRequest, TaskClass, TaskTier,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingStageLaunchPlan {
    pub stage_id: String,
    pub dispatch_target: ModelLaneRoutingDispatchTarget,
    pub lane_id: Option<String>,
    pub model_id: Option<String>,
    pub provider: Option<crate::model_runtime::ProviderKind>,
}
