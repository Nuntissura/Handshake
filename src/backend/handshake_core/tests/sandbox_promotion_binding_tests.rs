use handshake_core::{
    kernel::{PromotionDecisionKind, PromotionGate},
    sandbox::{
        build_promotion_request, replay_sandbox_promotion_events, AdapterCapabilities, AdapterId,
        GpuPassthrough, IsolationStrength, IsolationTier, ProcessHandle, SandboxPromotionOutcome,
        SandboxPromotionRejectReason, SandboxValidationEvidence, WindowsNativeJailAdapter,
        SANDBOX_PROMOTION_VALIDATED_EVENT_FAMILY, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
    },
};

#[test]
fn successful_sandbox_promotion_event_carries_process_handle_id() {
    let evidence = evidence(strong_capabilities(), 0);
    let process_id = evidence.process_handle.id;
    let decision =
        PromotionGate::decide_sandbox_validated(build_promotion_request(evidence, "ART-candidate"));

    assert_eq!(
        decision.promotion_decision_kind,
        PromotionDecisionKind::Approved
    );
    assert!(matches!(
        decision.outcome,
        SandboxPromotionOutcome::Accepted
    ));
    let event = decision.event_row.expect("accepted promotion event");
    assert_eq!(event.event_family, SANDBOX_PROMOTION_VALIDATED_EVENT_FAMILY);
    assert_eq!(event.process_handle_id, process_id);
    assert_eq!(event.adapter_id, "wsl2_podman");
    assert_eq!(event.candidate_artifact_id, "ART-candidate");
    assert_eq!(event.validation_stdout_artifact_id, "ART-stdout");
    assert_eq!(event.validation_stderr_artifact_id, "ART-stderr");
}

#[test]
fn weak_sandbox_isolation_rejects_promotion() {
    let weak = AdapterCapabilities {
        filesystem_isolation_strength: IsolationStrength::Weak,
        network_isolation_strength: IsolationStrength::Weak,
        ..strong_capabilities()
    };
    let decision = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(weak, 0),
        "ART-candidate",
    ));

    assert_eq!(
        decision.promotion_decision_kind,
        PromotionDecisionKind::Rejected
    );
    assert!(decision.event_row.is_none());
    match decision.outcome {
        SandboxPromotionOutcome::Rejected {
            reason:
                SandboxPromotionRejectReason::InsufficientSandboxIsolation {
                    required_min,
                    observed_filesystem,
                    observed_network,
                },
        } => {
            assert_eq!(required_min, IsolationStrength::Strong);
            assert_eq!(observed_filesystem, IsolationStrength::Weak);
            assert_eq!(observed_network, IsolationStrength::Weak);
        }
        other => panic!("expected insufficient isolation rejection, got {other:?}"),
    }
}

#[test]
fn windows_native_jail_target_capabilities_do_not_approve_without_runtime_backend() {
    let decision = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(WindowsNativeJailAdapter::target_capability_contract(), 0),
        "ART-candidate",
    ));

    assert_eq!(
        decision.promotion_decision_kind,
        PromotionDecisionKind::Rejected
    );
    assert!(decision.event_row.is_none());
    match decision.outcome {
        SandboxPromotionOutcome::Rejected {
            reason: SandboxPromotionRejectReason::AdapterUnavailable { adapter_id, .. },
        } => {
            assert_eq!(adapter_id, WINDOWS_NATIVE_JAIL_ADAPTER_ID);
        }
        other => panic!("expected AdapterUnavailable rejection, got {other:?}"),
    }
}

#[test]
fn fake_runtime_available_windows_native_jail_evidence_does_not_approve_without_very_strong_runtime_shape(
) {
    let mut capabilities = strong_capabilities();
    capabilities.adapter_id = AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID);
    capabilities.win32_native_fidelity = true;
    capabilities.cross_machine_portable = false;

    let decision = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(capabilities, 0),
        "ART-candidate",
    ));

    assert_eq!(
        decision.promotion_decision_kind,
        PromotionDecisionKind::Rejected
    );
    assert!(decision.event_row.is_none());
    match decision.outcome {
        SandboxPromotionOutcome::Rejected {
            reason:
                SandboxPromotionRejectReason::InsufficientSandboxIsolation {
                    required_min,
                    observed_filesystem,
                    observed_network,
                },
        } => {
            assert_eq!(required_min, IsolationStrength::VeryStrong);
            assert_eq!(observed_filesystem, IsolationStrength::Strong);
            assert_eq!(observed_network, IsolationStrength::Strong);
        }
        other => panic!("expected InsufficientSandboxIsolation rejection, got {other:?}"),
    }
}

#[test]
fn nonzero_validation_exit_code_rejects_without_event_row() {
    let decision = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(strong_capabilities(), 7),
        "ART-candidate",
    ));

    assert_eq!(
        decision.promotion_decision_kind,
        PromotionDecisionKind::Rejected
    );
    assert!(decision.event_row.is_none());
    assert!(matches!(
        decision.outcome,
        SandboxPromotionOutcome::Rejected {
            reason: SandboxPromotionRejectReason::ValidationFailed { exit_code: 7 }
        }
    ));
}

#[test]
fn replay_projection_ignores_process_handle_id_for_equivalent_promotions() {
    let first = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(strong_capabilities(), 0),
        "ART-candidate",
    ))
    .event_row
    .expect("first event");
    let second = PromotionGate::decide_sandbox_validated(build_promotion_request(
        evidence(strong_capabilities(), 0),
        "ART-candidate",
    ))
    .event_row
    .expect("second event");
    assert_ne!(
        first.process_handle_id, second.process_handle_id,
        "fixture must differ only by sandbox process handle id"
    );

    let first_projection = replay_sandbox_promotion_events([first]);
    let second_projection = replay_sandbox_promotion_events([second]);

    assert_eq!(first_projection, second_projection);
    let state = first_projection
        .get("ART-candidate")
        .expect("candidate projection");
    assert_eq!(
        state.promotion_decision_kind,
        PromotionDecisionKind::Approved
    );
    assert_eq!(state.adapter_id, "wsl2_podman");
}

fn evidence(
    adapter_capabilities: AdapterCapabilities,
    validation_exit_code: i32,
) -> SandboxValidationEvidence {
    SandboxValidationEvidence {
        process_handle: ProcessHandle::new(
            adapter_capabilities.adapter_id.clone(),
            Some(5150),
            "sandbox-validation-process",
        ),
        adapter_capabilities,
        validation_exit_code,
        validation_stdout_artifact_id: "ART-stdout".to_string(),
        validation_stderr_artifact_id: "ART-stderr".to_string(),
        sandbox_runtime_ms: 42,
    }
}

fn strong_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new("wsl2_podman"),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: handshake_core::sandbox::ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
    }
}
