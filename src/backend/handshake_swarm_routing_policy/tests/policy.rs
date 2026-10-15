#[cfg(test)]
mod tests {
    use handshake_swarm_routing_policy::*;
    use std::time::{Duration, Instant};

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

    // ---- rank-5: load-based escalation (Overloaded) ----

    #[test]
    fn overload_escalates_routine_to_cloud() {
        let mut p = policy();
        let r = req(TaskClass::Routine).with_local_outcome(LocalOutcome::Overloaded);
        assert_eq!(p.route(&r, Instant::now()).unwrap().tier(), TaskTier::Cloud);
    }

    #[test]
    fn overload_does_not_trip_local_breaker() {
        // Overload is transient capacity, not a lane fault: many overloads must
        // NOT suppress the healthy local lane (unlike repeated Failed). After 10
        // overload escalations, a fresh routine (no outcome) still routes local.
        let mut p = policy();
        let now = Instant::now();
        for _ in 0..10 {
            let r = req(TaskClass::Routine).with_local_outcome(LocalOutcome::Overloaded);
            let _ = p.route(&r, now);
        }
        assert_eq!(
            p.route(&req(TaskClass::Routine), now).unwrap().tier(),
            TaskTier::Local
        );
    }

    #[test]
    fn force_local_blocks_overload_escalation() {
        // Data-residency is non-negotiable: overload must not push a force-local
        // task to cloud (no data egress under load).
        let mut p = policy();
        let r = req(TaskClass::ForceLocal).with_local_outcome(LocalOutcome::Overloaded);
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
        assert!(matches!(err, SwarmRoutingError::AllLanesSuppressed { .. }));
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
            let _ = p.route(&base.clone().with_local_outcome(LocalOutcome::Failed), now);
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
