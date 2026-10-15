use handshake_swarm_breaker::breaker::BreakerConfig;

use super::policy::RoutingPolicy;

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
    pub(super) fn breaker_key(self) -> &'static str {
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
    /// rank-5 (load-based escalation): the local tier is OVERLOADED -- concurrency
    /// saturated, in-VM memory pressure / OOM risk, or a latency-SLO breach. The
    /// model itself is healthy, so this escalates the task to cloud to RELIEVE
    /// load WITHOUT tripping the local breaker (overload is transient capacity,
    /// not a lane fault -- penalizing the breaker would wrongly suppress a healthy
    /// local lane). `ForceLocal` still blocks escalation (data-residency).
    Overloaded,
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
/// coordinator turns this into a concrete `SpawnRequest`.
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
