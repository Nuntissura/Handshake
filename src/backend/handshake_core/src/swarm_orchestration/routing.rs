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

use std::collections::HashMap;
use std::time::Instant;

use super::breaker::{AdmitDecision, BreakerConfig, FailureFingerprint, FailureFingerprintBreaker};
use super::error::SwarmErrorClass;

/// Coarse task class the operator (or an upstream scheduler) tags work with. The
/// classifier maps this — together with the size signals — onto a tier. Kept
/// small and explicit so a no-context model can pick the right value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskClass {
    /// Routine work a small/local model handles well (formatting, extraction,
    /// short rewrites, boilerplate). Defaults LOCAL.
    Routine,
    /// Classification / labelling / short-answer work. Defaults LOCAL.
    Classification,
    /// Hard reasoning / synthesis / long-form generation. Defaults CLOUD.
    HardReasoning,
    /// The caller explicitly demands the cloud tier regardless of size (e.g. a
    /// frontier-only capability). Always CLOUD.
    ForceCloud,
    /// The caller explicitly demands the local tier regardless of size (e.g.
    /// data must not leave the host). Always LOCAL.
    ForceLocal,
}

/// The execution tier a routing decision targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskTier {
    /// Real local engine (candle / llama.cpp) — the proven on-host path.
    Local,
    /// A cloud BYOK provider lane.
    Cloud,
}

/// Which cloud provider a cloud-tier decision targets. The policy chooses
/// between the configured providers and isolates failures per provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloudProvider {
    Anthropic,
    OpenAi,
}

impl CloudProvider {
    fn breaker_key(self) -> &'static str {
        match self {
            CloudProvider::Anthropic => "anthropic",
            CloudProvider::OpenAi => "openai",
        }
    }
}

/// Outcome of a prior LOCAL attempt, fed back into [`RoutingPolicy::route`] so a
/// failed/low-confidence local run escalates to cloud. `None` on the first route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalOutcome {
    /// The local attempt failed (engine error / load failure). Escalate.
    Failed,
    /// The local attempt produced output but the caller deemed confidence below
    /// its acceptance threshold. Escalate.
    LowConfidence,
}

/// Outcome of a prior CLOUD attempt against a specific provider, fed back so the
/// policy can trip that provider's breaker and fall back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudOutcome {
    /// The cloud call against `provider` failed (HTTP / provider error /
    /// not-configured). Record against that provider's breaker.
    Failed(CloudProvider),
    /// The cloud call against `provider` succeeded. Heal that provider's
    /// breaker.
    Succeeded(CloudProvider),
}

/// A routing request: pure data the policy classifies. The model fields are
/// EXPLICIT per request (the operator picks which local artifact and which cloud
/// model a given task should use); the policy never invents model names.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutingRequest {
    pub class: TaskClass,
    /// Estimated input size (e.g. prompt tokens / chars). Long inputs bias CLOUD.
    pub estimated_input_tokens: u32,
    /// Requested max output tokens. Long generations bias CLOUD.
    pub max_output_tokens: u32,
    /// EXPLICIT local model the caller wants for a local-tier decision (artifact
    /// path or model name). `None` means the operator did not provide a local
    /// target, so a local-tier decision is not admissible (the policy must route
    /// cloud or surface an error).
    pub local_model: Option<String>,
    /// EXPLICIT cloud model the caller wants for a cloud-tier decision (e.g.
    /// `claude-sonnet-4`, `gpt-4o`). `None` means no cloud target was provided.
    pub cloud_model: Option<String>,
    /// Outcome of a prior local attempt for THIS task, if any (drives escalation).
    pub local_outcome: Option<LocalOutcome>,
    /// Outcome of a prior cloud attempt for THIS task, if any (drives fallback +
    /// breaker accounting). Applied before the decision so a just-failed
    /// provider is excluded from this same route call.
    pub cloud_outcome: Option<CloudOutcome>,
}

impl RoutingRequest {
    /// A first-attempt request with no prior outcomes.
    pub fn new(class: TaskClass, estimated_input_tokens: u32, max_output_tokens: u32) -> Self {
        Self {
            class,
            estimated_input_tokens,
            max_output_tokens,
            local_model: None,
            cloud_model: None,
            local_outcome: None,
            cloud_outcome: None,
        }
    }

    pub fn with_local_model(mut self, model: impl Into<String>) -> Self {
        self.local_model = Some(model.into());
        self
    }

    pub fn with_cloud_model(mut self, model: impl Into<String>) -> Self {
        self.cloud_model = Some(model.into());
        self
    }

    pub fn with_local_outcome(mut self, outcome: LocalOutcome) -> Self {
        self.local_outcome = Some(outcome);
        self
    }

    pub fn with_cloud_outcome(mut self, outcome: CloudOutcome) -> Self {
        self.cloud_outcome = Some(outcome);
        self
    }
}

/// The lane a routing decision selects, naming the explicit model. The
/// coordinator turns this into a concrete [`super::ids::SpawnRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Spawn the local engine with the named local model.
    Local { model: String },
    /// Spawn the named cloud provider lane with the named cloud model.
    Cloud {
        provider: CloudProvider,
        model: String,
    },
}

impl RoutingDecision {
    pub fn tier(&self) -> TaskTier {
        match self {
            RoutingDecision::Local { .. } => TaskTier::Local,
            RoutingDecision::Cloud { .. } => TaskTier::Cloud,
        }
    }
}

/// Why a route could not be produced. These are genuine routing conditions
/// (missing explicit model, all lanes suppressed), never placeholders.
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
}

/// Tunable thresholds for the deterministic classifier. Defaults are
/// conservative: short routine/classification work stays local, long or hard
/// work goes cloud. All thresholds are explicit so the routing is reproducible
/// and auditable.
#[derive(Clone, Debug)]
pub struct RoutingPolicyConfig {
    /// Inputs at or above this token count bias toward CLOUD even for routine
    /// classes (a long context is expensive/slow for a small local model).
    pub long_input_tokens: u32,
    /// Requested outputs at or above this token count bias toward CLOUD.
    pub long_output_tokens: u32,
    /// Per-provider breaker configuration (trip threshold + cooldown).
    pub breaker: BreakerConfig,
    /// Ordered cloud-provider preference. The first admissible (non-suppressed)
    /// provider in this list is chosen for a cloud-tier decision; fallback walks
    /// the list. Empty means no cloud lane is configured.
    pub cloud_preference: Vec<CloudProvider>,
}

impl Default for RoutingPolicyConfig {
    fn default() -> Self {
        Self {
            long_input_tokens: 4_096,
            long_output_tokens: 1_024,
            breaker: BreakerConfig::default(),
            cloud_preference: vec![CloudProvider::Anthropic, CloudProvider::OpenAi],
        }
    }
}

/// The pluggable local-vs-cloud routing policy. Holds the deterministic
/// classifier config plus one [`FailureFingerprintBreaker`] per lane key
/// (`local`, `anthropic`, `openai`). Not internally synchronised — the caller
/// owns it behind its own lock so a route decision and the breaker update move
/// together, mirroring how the coordinator owns its spawn breaker.
pub struct RoutingPolicy {
    config: RoutingPolicyConfig,
    breakers: HashMap<&'static str, FailureFingerprintBreaker>,
}

const LOCAL_KEY: &str = "local";

impl RoutingPolicy {
    pub fn new(config: RoutingPolicyConfig) -> Self {
        let mut breakers = HashMap::new();
        breakers.insert(LOCAL_KEY, FailureFingerprintBreaker::new(config.breaker));
        breakers.insert(
            CloudProvider::Anthropic.breaker_key(),
            FailureFingerprintBreaker::new(config.breaker),
        );
        breakers.insert(
            CloudProvider::OpenAi.breaker_key(),
            FailureFingerprintBreaker::new(config.breaker),
        );
        Self { config, breakers }
    }

    pub fn with_default() -> Self {
        Self::new(RoutingPolicyConfig::default())
    }

    /// The deterministic, side-effect-free tier classification for a request,
    /// IGNORING breakers and prior outcomes. Exposed so a caller can preview the
    /// natural tier; [`route`] layers escalation + breaker suppression on top.
    pub fn classify(&self, req: &RoutingRequest) -> TaskTier {
        match req.class {
            TaskClass::ForceCloud => TaskTier::Cloud,
            TaskClass::ForceLocal => TaskTier::Local,
            TaskClass::HardReasoning => TaskTier::Cloud,
            TaskClass::Routine | TaskClass::Classification => {
                // Routine/classification default LOCAL, but a long input or a
                // long requested output promotes to CLOUD.
                if req.estimated_input_tokens >= self.config.long_input_tokens
                    || req.max_output_tokens >= self.config.long_output_tokens
                {
                    TaskTier::Cloud
                } else {
                    TaskTier::Local
                }
            }
        }
    }

    /// Fingerprint for a lane key. Reuses the coordinator's fingerprint scheme
    /// so the breaker semantics are identical; the lane key is the detail so
    /// each provider gets an isolated signature under one error class.
    fn lane_fp(key: &str) -> FailureFingerprint {
        FailureFingerprint::compute(SwarmErrorClass::ProviderNotConfigured, key)
    }

    /// Is a lane admissible (breaker closed or half-open probe permitted) at `now`?
    fn lane_admissible(&mut self, key: &'static str, now: Instant) -> AdmitDecision {
        let fp = Self::lane_fp(key);
        self.breakers
            .get_mut(key)
            .expect("breaker for known lane key")
            .admit(&fp, now)
    }

    /// Record a lane failure at `now`, returning whether it tripped the breaker.
    fn record_lane_failure(&mut self, key: &'static str, now: Instant) -> bool {
        let fp = Self::lane_fp(key);
        self.breakers
            .get_mut(key)
            .expect("breaker for known lane key")
            .record_failure(&fp, now)
    }

    /// Record a lane success, healing the breaker.
    fn record_lane_success(&mut self, key: &'static str) {
        let fp = Self::lane_fp(key);
        self.breakers
            .get_mut(key)
            .expect("breaker for known lane key")
            .record_success(&fp);
    }

    /// Apply the prior-attempt outcomes carried on the request to the breakers
    /// BEFORE deciding, so a just-failed provider is excluded from this route.
    fn apply_outcomes(&mut self, req: &RoutingRequest, now: Instant) {
        if let Some(outcome) = req.cloud_outcome {
            match outcome {
                CloudOutcome::Failed(provider) => {
                    self.record_lane_failure(provider.breaker_key(), now);
                }
                CloudOutcome::Succeeded(provider) => {
                    self.record_lane_success(provider.breaker_key());
                }
            }
        }
        // A local failure/low-confidence is recorded against the local breaker
        // (so repeated systemic local failures eventually suppress the local
        // lane and force cloud) AND drives escalation in `route`.
        if let Some(outcome) = req.local_outcome {
            match outcome {
                LocalOutcome::Failed | LocalOutcome::LowConfidence => {
                    self.record_lane_failure(LOCAL_KEY, now);
                }
            }
        }
    }

    /// Produce a routing decision at instant `now`, honoring classification,
    /// escalation (local failure/low-confidence -> cloud), per-provider breaker
    /// suppression, and fallback (suppressed cloud provider -> next provider ->
    /// local).
    ///
    /// `now` is injected so tests are deterministic; production passes
    /// `Instant::now()`.
    pub fn route(
        &mut self,
        req: &RoutingRequest,
        now: Instant,
    ) -> Result<RoutingDecision, SwarmRoutingError> {
        self.apply_outcomes(req, now);

        // Escalation: a prior local failure/low-confidence forces the cloud tier
        // for this task regardless of the natural classification — UNLESS the
        // caller force-pinned local (data-residency), which is non-negotiable.
        let escalate = req.local_outcome.is_some() && req.class != TaskClass::ForceLocal;
        let natural_tier = self.classify(req);
        let target_tier = if escalate { TaskTier::Cloud } else { natural_tier };

        match target_tier {
            TaskTier::Local => self.route_local_then_maybe_cloud(req, now),
            TaskTier::Cloud => self.route_cloud_then_fallback(req, now),
        }
    }

    /// Local-tier route: if the local lane is admissible, take it; otherwise
    /// (local breaker open) fall back to cloud. ForceLocal never falls back.
    fn route_local_then_maybe_cloud(
        &mut self,
        req: &RoutingRequest,
        now: Instant,
    ) -> Result<RoutingDecision, SwarmRoutingError> {
        let local_decision = match self.lane_admissible(LOCAL_KEY, now) {
            AdmitDecision::Admit => {
                let model = req
                    .local_model
                    .clone()
                    .ok_or(SwarmRoutingError::NoLocalModel)?;
                Some(RoutingDecision::Local { model })
            }
            AdmitDecision::Suppress { .. } => None,
        };

        if let Some(decision) = local_decision {
            return Ok(decision);
        }

        // Local lane suppressed. ForceLocal cannot leave the host: surface the
        // suppression honestly rather than silently routing cloud.
        if req.class == TaskClass::ForceLocal {
            let cooldown = match self.lane_admissible(LOCAL_KEY, now) {
                AdmitDecision::Suppress {
                    cooldown_remaining_ms,
                } => cooldown_remaining_ms,
                AdmitDecision::Admit => 0,
            };
            return Err(SwarmRoutingError::AllLanesSuppressed {
                cooldown_remaining_ms: cooldown,
            });
        }

        // Otherwise fall back to cloud.
        self.route_cloud_then_fallback(req, now)
    }

    /// Cloud-tier route: walk the configured provider preference, choosing the
    /// first admissible provider. If every cloud provider is suppressed, fall
    /// back to local (when admissible + the class is not ForceCloud). If nothing
    /// is admissible, surface AllLanesSuppressed.
    fn route_cloud_then_fallback(
        &mut self,
        req: &RoutingRequest,
        now: Instant,
    ) -> Result<RoutingDecision, SwarmRoutingError> {
        if self.config.cloud_preference.is_empty() {
            // No cloud configured. ForceCloud cannot be satisfied; otherwise try
            // local as the only lane.
            if req.class == TaskClass::ForceCloud {
                return Err(SwarmRoutingError::NoCloudProvider);
            }
            return self.route_local_only(req, now);
        }

        let cloud_model = req
            .cloud_model
            .clone()
            .ok_or(SwarmRoutingError::NoCloudModel)?;

        let mut min_cooldown: Option<u128> = None;
        for provider in self.config.cloud_preference.clone() {
            match self.lane_admissible(provider.breaker_key(), now) {
                AdmitDecision::Admit => {
                    return Ok(RoutingDecision::Cloud {
                        provider,
                        model: cloud_model,
                    });
                }
                AdmitDecision::Suppress {
                    cooldown_remaining_ms,
                } => {
                    min_cooldown = Some(
                        min_cooldown
                            .map(|c| c.min(cooldown_remaining_ms))
                            .unwrap_or(cooldown_remaining_ms),
                    );
                }
            }
        }

        // All cloud providers suppressed. ForceCloud has nowhere to go.
        if req.class == TaskClass::ForceCloud {
            return Err(SwarmRoutingError::AllLanesSuppressed {
                cooldown_remaining_ms: min_cooldown.unwrap_or(0),
            });
        }

        // Fall back to local if admissible.
        match self.lane_admissible(LOCAL_KEY, now) {
            AdmitDecision::Admit => {
                let model = req
                    .local_model
                    .clone()
                    .ok_or(SwarmRoutingError::NoLocalModel)?;
                Ok(RoutingDecision::Local { model })
            }
            AdmitDecision::Suppress {
                cooldown_remaining_ms,
            } => Err(SwarmRoutingError::AllLanesSuppressed {
                cooldown_remaining_ms: min_cooldown
                    .map(|c| c.min(cooldown_remaining_ms))
                    .unwrap_or(cooldown_remaining_ms),
            }),
        }
    }

    /// Local-only route used when no cloud provider is configured.
    fn route_local_only(
        &mut self,
        req: &RoutingRequest,
        now: Instant,
    ) -> Result<RoutingDecision, SwarmRoutingError> {
        match self.lane_admissible(LOCAL_KEY, now) {
            AdmitDecision::Admit => {
                let model = req
                    .local_model
                    .clone()
                    .ok_or(SwarmRoutingError::NoLocalModel)?;
                Ok(RoutingDecision::Local { model })
            }
            AdmitDecision::Suppress {
                cooldown_remaining_ms,
            } => Err(SwarmRoutingError::AllLanesSuppressed {
                cooldown_remaining_ms,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn policy() -> RoutingPolicy {
        RoutingPolicy::with_default()
    }

    fn req(class: TaskClass) -> RoutingRequest {
        RoutingRequest::new(class, 100, 100)
            .with_local_model("tinyllama.safetensors")
            .with_cloud_model("claude-sonnet-4")
    }

    // ---- classifier tiers ----

    #[test]
    fn routine_and_classification_short_work_classify_local() {
        let p = policy();
        assert_eq!(p.classify(&req(TaskClass::Routine)), TaskTier::Local);
        assert_eq!(p.classify(&req(TaskClass::Classification)), TaskTier::Local);
    }

    #[test]
    fn hard_reasoning_classifies_cloud() {
        let p = policy();
        assert_eq!(p.classify(&req(TaskClass::HardReasoning)), TaskTier::Cloud);
    }

    #[test]
    fn force_flags_override_size() {
        let p = policy();
        // ForceCloud on tiny input still cloud; ForceLocal on huge input still local.
        let mut huge = req(TaskClass::ForceLocal);
        huge.estimated_input_tokens = 1_000_000;
        huge.max_output_tokens = 1_000_000;
        assert_eq!(p.classify(&huge), TaskTier::Local);
        assert_eq!(p.classify(&req(TaskClass::ForceCloud)), TaskTier::Cloud);
    }

    #[test]
    fn long_input_or_output_promotes_routine_to_cloud() {
        let p = policy();
        let mut long_in = req(TaskClass::Routine);
        long_in.estimated_input_tokens = 8_192;
        assert_eq!(p.classify(&long_in), TaskTier::Cloud);
        let mut long_out = req(TaskClass::Classification);
        long_out.max_output_tokens = 4_096;
        assert_eq!(p.classify(&long_out), TaskTier::Cloud);
    }

    // ---- per-request model selection ----

    #[test]
    fn route_local_carries_explicit_local_model() {
        let mut p = policy();
        let decision = p.route(&req(TaskClass::Routine), Instant::now()).unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Local {
                model: "tinyllama.safetensors".to_string()
            }
        );
    }

    #[test]
    fn route_cloud_carries_explicit_cloud_model_and_first_preference_provider() {
        let mut p = policy();
        let decision = p
            .route(&req(TaskClass::HardReasoning), Instant::now())
            .unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Cloud {
                provider: CloudProvider::Anthropic,
                model: "claude-sonnet-4".to_string()
            }
        );
    }

    #[test]
    fn local_tier_without_explicit_local_model_is_typed_error() {
        let mut p = policy();
        let mut r = RoutingRequest::new(TaskClass::Routine, 10, 10);
        r.cloud_model = Some("gpt-4o".to_string());
        let err = p.route(&r, Instant::now()).unwrap_err();
        assert_eq!(err, SwarmRoutingError::NoLocalModel);
    }

    #[test]
    fn cloud_tier_without_explicit_cloud_model_is_typed_error() {
        let mut p = policy();
        let r = RoutingRequest::new(TaskClass::HardReasoning, 10, 10)
            .with_local_model("local.safetensors");
        let err = p.route(&r, Instant::now()).unwrap_err();
        assert_eq!(err, SwarmRoutingError::NoCloudModel);
    }

    // ---- escalation ----

    #[test]
    fn local_failure_escalates_routine_to_cloud() {
        let mut p = policy();
        let r = req(TaskClass::Routine).with_local_outcome(LocalOutcome::Failed);
        let decision = p.route(&r, Instant::now()).unwrap();
        assert_eq!(decision.tier(), TaskTier::Cloud);
    }

    #[test]
    fn local_low_confidence_escalates_to_cloud() {
        let mut p = policy();
        let r = req(TaskClass::Classification).with_local_outcome(LocalOutcome::LowConfidence);
        assert_eq!(p.route(&r, Instant::now()).unwrap().tier(), TaskTier::Cloud);
    }

    #[test]
    fn force_local_never_escalates_even_on_failure() {
        let mut p = policy();
        let r = req(TaskClass::ForceLocal).with_local_outcome(LocalOutcome::Failed);
        // ForceLocal stays local (data residency); one failure does not trip the
        // breaker (default threshold 5), so it still routes local.
        assert_eq!(p.route(&r, Instant::now()).unwrap().tier(), TaskTier::Local);
    }

    // ---- breaker trip + fallback ----

    #[test]
    fn anthropic_breaker_trip_falls_back_to_openai() {
        let mut p = policy();
        let now = Instant::now();
        // Trip the anthropic breaker with 5 consecutive failures (default
        // threshold). Each route carries the prior anthropic failure outcome.
        let base = req(TaskClass::HardReasoning);
        for _ in 0..5 {
            let r = base
                .clone()
                .with_cloud_outcome(CloudOutcome::Failed(CloudProvider::Anthropic));
            let _ = p.route(&r, now);
        }
        // Next cloud route now skips suppressed anthropic and selects openai.
        let decision = p.route(&base, now).unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Cloud {
                provider: CloudProvider::OpenAi,
                model: "claude-sonnet-4".to_string()
            }
        );
    }

    #[test]
    fn all_cloud_providers_tripped_falls_back_to_local_for_hard_reasoning() {
        let mut p = policy();
        let now = Instant::now();
        let base = req(TaskClass::HardReasoning);
        for provider in [CloudProvider::Anthropic, CloudProvider::OpenAi] {
            for _ in 0..5 {
                let r = base
                    .clone()
                    .with_cloud_outcome(CloudOutcome::Failed(provider));
                let _ = p.route(&r, now);
            }
        }
        // Both cloud breakers open -> hard reasoning falls back to LOCAL (the
        // local lane is still admissible and an explicit local model exists).
        let decision = p.route(&base, now).unwrap();
        assert_eq!(decision.tier(), TaskTier::Local);
    }

    #[test]
    fn force_cloud_with_all_providers_tripped_surfaces_all_lanes_suppressed() {
        let mut p = policy();
        let now = Instant::now();
        let base = req(TaskClass::ForceCloud);
        for provider in [CloudProvider::Anthropic, CloudProvider::OpenAi] {
            for _ in 0..5 {
                let r = base
                    .clone()
                    .with_cloud_outcome(CloudOutcome::Failed(provider));
                let _ = p.route(&r, now);
            }
        }
        let err = p.route(&base, now).unwrap_err();
        assert!(matches!(
            err,
            SwarmRoutingError::AllLanesSuppressed { .. }
        ));
    }

    #[test]
    fn cloud_success_heals_breaker_and_restores_preference() {
        let mut p = policy();
        let now = Instant::now();
        let base = req(TaskClass::HardReasoning);
        // Trip anthropic.
        for _ in 0..5 {
            let _ = p.route(
                &base
                    .clone()
                    .with_cloud_outcome(CloudOutcome::Failed(CloudProvider::Anthropic)),
                now,
            );
        }
        // Falls back to openai now.
        assert_eq!(
            p.route(&base, now).unwrap(),
            RoutingDecision::Cloud {
                provider: CloudProvider::OpenAi,
                model: "claude-sonnet-4".to_string()
            }
        );
        // After cooldown a half-open probe is admitted; record a success to heal.
        let later = now + Duration::from_secs(31);
        let _ = p.route(
            &base
                .clone()
                .with_cloud_outcome(CloudOutcome::Succeeded(CloudProvider::Anthropic)),
            later,
        );
        // Anthropic preferred again.
        assert_eq!(
            p.route(&base, later).unwrap(),
            RoutingDecision::Cloud {
                provider: CloudProvider::Anthropic,
                model: "claude-sonnet-4".to_string()
            }
        );
    }

    #[test]
    fn local_breaker_trip_routes_routine_to_cloud_fallback() {
        let mut p = policy();
        let now = Instant::now();
        // Five local failures trip the local breaker; subsequent routine work
        // (which would normally be local) falls back to cloud.
        let base = req(TaskClass::Routine);
        for _ in 0..5 {
            let _ = p.route(
                &base.clone().with_local_outcome(LocalOutcome::Failed),
                now,
            );
        }
        // A fresh routine request (no outcome) now finds local suppressed and
        // routes cloud.
        let decision = p.route(&base, now).unwrap();
        assert_eq!(decision.tier(), TaskTier::Cloud);
    }

    #[test]
    fn no_cloud_configured_routes_local_only_and_force_cloud_errors() {
        let cfg = RoutingPolicyConfig {
            cloud_preference: vec![],
            ..RoutingPolicyConfig::default()
        };
        let mut p = RoutingPolicy::new(cfg);
        let now = Instant::now();
        // Hard reasoning with no cloud configured falls back to local.
        assert_eq!(
            p.route(&req(TaskClass::HardReasoning), now).unwrap().tier(),
            TaskTier::Local
        );
        // ForceCloud with no cloud configured is a typed NoCloudProvider error.
        let err = p.route(&req(TaskClass::ForceCloud), now).unwrap_err();
        assert_eq!(err, SwarmRoutingError::NoCloudProvider);
    }
}
