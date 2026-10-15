use std::collections::HashMap;
use std::time::Instant;

use handshake_model_lane_routing::SwarmRoutingError;
use handshake_swarm_breaker::breaker::{
    AdmitDecision, FailureFingerprint, FailureFingerprintBreaker,
};
use handshake_swarm_breaker::SwarmErrorClass;

use super::types::{
    CloudOutcome, CloudProvider, LocalOutcome, RoutingDecision, RoutingPolicyConfig, RoutingRequest,
    TaskClass, TaskTier,
};

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
    /// natural tier; [`Self::route`] layers escalation + breaker suppression on top.
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
                // Overload escalates (via `route`'s is_some() check) but does NOT
                // record a lane failure: the local lane is healthy, just saturated.
                // Tripping its breaker on transient load would wrongly suppress it.
                LocalOutcome::Overloaded => {}
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
        let target_tier = if escalate {
            TaskTier::Cloud
        } else {
            natural_tier
        };

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
