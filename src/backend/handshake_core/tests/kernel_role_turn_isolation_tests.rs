use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_turn_isolation::{
        project_role_turn_isolation, validate_role_turn_isolation, RoleTurnEffectiveMode,
        RoleTurnExecutionMode, RoleTurnIsolationPolicyV1, RoleTurnPassKind, RoleTurnReplayPinV1,
        RoleTurnResetSupport, RoleTurnTraceRecordV1,
    },
};

#[test]
fn kernel_role_turn_isolation_defaults_to_isolated_replayable_turns() {
    let policy = sample_policy();

    validate_role_turn_isolation(&policy).expect("role turn isolation policy validates");

    assert_eq!(
        policy.default_execution_mode,
        RoleTurnExecutionMode::Isolated
    );
    assert!(policy.role_turns.iter().all(|turn| {
        turn.requested_mode == RoleTurnExecutionMode::Isolated
            && turn.role_window_reset
            && turn.context_window_reset
            && turn.inherited_context_refs.is_empty()
    }));
}

#[test]
fn kernel_role_turn_isolation_projection_records_pins_and_effective_modes() {
    let policy = sample_policy();
    let projection = project_role_turn_isolation(&policy).expect("projection builds");

    assert!(projection.isolated_by_default);
    assert_eq!(projection.turn_count, 3);
    assert_eq!(projection.replay_pin_count, 3);
    assert!(projection
        .isolated_turn_ids
        .contains(&"turn-coder-claim".to_string()));
    assert!(projection
        .degraded_turn_ids
        .contains(&"turn-validator-extract".to_string()));
    assert!(projection
        .requested_effective_pairs
        .contains(&"turn-validator-extract:Isolated->DegradedIsolated".to_string()));
    assert!(projection.denied_cross_role_bleed_turn_ids.is_empty());
    assert!(!projection.mutates_runtime_state);
}

#[test]
fn kernel_role_turn_isolation_rejects_nonisolated_bleed_and_missing_replay_pins() {
    let mut policy = sample_policy();
    policy.default_execution_mode = RoleTurnExecutionMode::NonIsolated;
    policy.role_turns[0]
        .inherited_context_refs
        .push("role-context://previous-validator".to_string());
    policy.role_turns[1].replay_pins.clear();
    policy.role_turns[2].effective_mode = RoleTurnEffectiveMode::NonIsolated;
    policy.role_turns[2].role_window_reset = false;
    policy.role_turns[2].context_window_reset = false;

    let errors =
        validate_role_turn_isolation(&policy).expect_err("unsafe role-turn policy must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "default_execution_mode"));
    assert!(errors
        .iter()
        .any(|error| error.field == "role_turns.inherited_context_refs"));
    assert!(errors
        .iter()
        .any(|error| error.field == "role_turns.replay_pins"));
    assert!(errors
        .iter()
        .any(|error| error.field == "role_turns.effective_mode"));
    assert!(errors
        .iter()
        .any(|error| error.field == "role_turns.role_window_reset"));
    assert!(errors
        .iter()
        .any(|error| error.field == "role_turns.context_window_reset"));
}

#[test]
fn kernel_role_turn_isolation_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.role_turn_isolation.project")
        .expect("role-turn isolation projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "role_turn_cross_role_bleed_denied"));
}

fn sample_policy() -> RoleTurnIsolationPolicyV1 {
    RoleTurnIsolationPolicyV1 {
        schema_id: "hsk.kernel.role_turn_isolation@1".to_string(),
        policy_id: "role-turn-isolation-mt037".to_string(),
        folded_stub_ids: vec!["WP-1-Role-Turn-Isolation-v1".to_string()],
        default_execution_mode: RoleTurnExecutionMode::Isolated,
        allow_non_isolated_override: false,
        strict_reset_required_by_default: true,
        role_turns: vec![
            turn(
                "turn-coder-claim",
                "CODER",
                RoleTurnPassKind::Claim,
                RoleTurnResetSupport::StrictReset,
                RoleTurnEffectiveMode::Isolated,
                None,
            ),
            turn(
                "turn-reviewer-glance",
                "VALIDATOR",
                RoleTurnPassKind::Glance,
                RoleTurnResetSupport::StrictReset,
                RoleTurnEffectiveMode::Isolated,
                None,
            ),
            turn(
                "turn-validator-extract",
                "INTEGRATION_VALIDATOR",
                RoleTurnPassKind::Extract,
                RoleTurnResetSupport::DegradedReset,
                RoleTurnEffectiveMode::DegradedIsolated,
                Some("DEGRADED_RESET_PINNED_SPANS_ONLY"),
            ),
        ],
        product_authority_refs: vec![
            "kernel.workflow_transition_registry".to_string(),
            "kernel.role_mailbox_loop_control".to_string(),
            "flight_recorder.role_turn".to_string(),
            "kernel.fems_mt_handoff_memory_context".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Turn-Isolation-v1.contract.json".to_string(),
        ],
    }
}

fn turn(
    turn_id: &str,
    role_id: &str,
    pass_kind: RoleTurnPassKind,
    reset_support: RoleTurnResetSupport,
    effective_mode: RoleTurnEffectiveMode,
    degraded_marker: Option<&str>,
) -> RoleTurnTraceRecordV1 {
    RoleTurnTraceRecordV1 {
        turn_id: turn_id.to_string(),
        role_id: role_id.to_string(),
        pass_kind,
        requested_mode: RoleTurnExecutionMode::Isolated,
        effective_mode,
        reset_support,
        role_window_reset: true,
        context_window_reset: true,
        inherited_context_refs: Vec::new(),
        replay_pins: vec![RoleTurnReplayPinV1 {
            pin_id: format!("pin-{turn_id}"),
            input_ref: format!("input://{turn_id}"),
            selected_span_ref: format!("span://{turn_id}"),
            content_hash: format!("hash-{turn_id}"),
            tie_break_key: format!("tie-break-{turn_id}"),
            degraded_marker: degraded_marker.map(str::to_string),
        }],
        trace_ref: format!("trace://{turn_id}"),
        provenance_ref: format!("provenance://{turn_id}"),
    }
}
