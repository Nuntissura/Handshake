use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_ROLE_TURN_ISOLATION_STUB_ID: &str = "WP-1-Role-Turn-Isolation-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleTurnPassKind {
    Claim,
    Glance,
    Extract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleTurnExecutionMode {
    Isolated,
    NonIsolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleTurnEffectiveMode {
    Isolated,
    DegradedIsolated,
    NonIsolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleTurnResetSupport {
    StrictReset,
    DegradedReset,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTurnReplayPinV1 {
    pub pin_id: String,
    pub input_ref: String,
    pub selected_span_ref: String,
    pub content_hash: String,
    pub tie_break_key: String,
    pub degraded_marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTurnTraceRecordV1 {
    pub turn_id: String,
    pub role_id: String,
    pub pass_kind: RoleTurnPassKind,
    pub requested_mode: RoleTurnExecutionMode,
    pub effective_mode: RoleTurnEffectiveMode,
    pub reset_support: RoleTurnResetSupport,
    pub role_window_reset: bool,
    pub context_window_reset: bool,
    pub inherited_context_refs: Vec<String>,
    pub replay_pins: Vec<RoleTurnReplayPinV1>,
    pub trace_ref: String,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTurnIsolationPolicyV1 {
    pub schema_id: String,
    pub policy_id: String,
    pub folded_stub_ids: Vec<String>,
    pub default_execution_mode: RoleTurnExecutionMode,
    pub allow_non_isolated_override: bool,
    pub strict_reset_required_by_default: bool,
    pub role_turns: Vec<RoleTurnTraceRecordV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleTurnIsolationProjectionV1 {
    pub schema_id: String,
    pub policy_id: String,
    pub isolated_by_default: bool,
    pub turn_count: usize,
    pub replay_pin_count: usize,
    pub isolated_turn_ids: Vec<String>,
    pub degraded_turn_ids: Vec<String>,
    pub denied_cross_role_bleed_turn_ids: Vec<String>,
    pub requested_effective_pairs: Vec<String>,
    pub trace_refs: Vec<String>,
    pub mutates_runtime_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleTurnIsolationValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_turn_isolation(
    policy: &RoleTurnIsolationPolicyV1,
) -> Result<(), Vec<RoleTurnIsolationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &policy.schema_id);
    require_non_empty(&mut errors, "policy_id", &policy.policy_id);
    require_vec(&mut errors, "folded_stub_ids", &policy.folded_stub_ids);
    require_vec(&mut errors, "role_turns", &policy.role_turns);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &policy.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &policy.folded_source_refs,
    );

    if !contains_exact(&policy.folded_stub_ids, FOLDED_ROLE_TURN_ISOLATION_STUB_ID) {
        errors.push(RoleTurnIsolationValidationError {
            field: "folded_stub_ids",
            message: "role-turn isolation must preserve the folded stub id",
        });
    }
    if !contains_text(
        &policy.folded_source_refs,
        FOLDED_ROLE_TURN_ISOLATION_STUB_ID,
    ) {
        errors.push(RoleTurnIsolationValidationError {
            field: "folded_source_refs",
            message: "role-turn isolation must preserve the folded source reference",
        });
    }
    if policy.default_execution_mode != RoleTurnExecutionMode::Isolated {
        errors.push(RoleTurnIsolationValidationError {
            field: "default_execution_mode",
            message: "role turns must default to isolated execution",
        });
    }
    if !policy.strict_reset_required_by_default {
        errors.push(RoleTurnIsolationValidationError {
            field: "strict_reset_required_by_default",
            message: "role turns must request strict role/context reset by default",
        });
    }

    validate_authority_refs(&mut errors, policy);
    validate_turns(&mut errors, policy);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_role_turn_isolation(
    policy: &RoleTurnIsolationPolicyV1,
) -> Result<RoleTurnIsolationProjectionV1, Vec<RoleTurnIsolationValidationError>> {
    validate_role_turn_isolation(policy)?;

    Ok(RoleTurnIsolationProjectionV1 {
        schema_id: "hsk.kernel.role_turn_isolation_projection@1".to_string(),
        policy_id: policy.policy_id.clone(),
        isolated_by_default: policy.default_execution_mode == RoleTurnExecutionMode::Isolated,
        turn_count: policy.role_turns.len(),
        replay_pin_count: policy
            .role_turns
            .iter()
            .map(|turn| turn.replay_pins.len())
            .sum(),
        isolated_turn_ids: policy
            .role_turns
            .iter()
            .filter(|turn| turn.effective_mode == RoleTurnEffectiveMode::Isolated)
            .map(|turn| turn.turn_id.clone())
            .collect(),
        degraded_turn_ids: policy
            .role_turns
            .iter()
            .filter(|turn| turn.effective_mode == RoleTurnEffectiveMode::DegradedIsolated)
            .map(|turn| turn.turn_id.clone())
            .collect(),
        denied_cross_role_bleed_turn_ids: policy
            .role_turns
            .iter()
            .filter(|turn| !turn.inherited_context_refs.is_empty())
            .map(|turn| turn.turn_id.clone())
            .collect(),
        requested_effective_pairs: policy
            .role_turns
            .iter()
            .map(|turn| {
                format!(
                    "{}:{:?}->{:?}",
                    turn.turn_id, turn.requested_mode, turn.effective_mode
                )
            })
            .collect(),
        trace_refs: policy
            .role_turns
            .iter()
            .map(|turn| turn.trace_ref.clone())
            .collect(),
        mutates_runtime_state: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<RoleTurnIsolationValidationError>,
    policy: &RoleTurnIsolationPolicyV1,
) {
    for required_ref in [
        "kernel.workflow_transition_registry",
        "kernel.role_mailbox_loop_control",
        "flight_recorder.role_turn",
        "kernel.fems_mt_handoff_memory_context",
    ] {
        if !contains_exact(&policy.product_authority_refs, required_ref) {
            errors.push(RoleTurnIsolationValidationError {
                field: "product_authority_refs",
                message: "role-turn isolation must cite workflow transition, loop control, Flight Recorder, and handoff memory authorities",
            });
        }
    }
}

fn validate_turns(
    errors: &mut Vec<RoleTurnIsolationValidationError>,
    policy: &RoleTurnIsolationPolicyV1,
) {
    let mut turn_ids = HashSet::new();
    let mut pin_ids = HashSet::new();

    for turn in &policy.role_turns {
        if !turn_ids.insert(turn.turn_id.as_str()) {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.turn_id",
                message: "role turn ids must be unique",
            });
        }

        require_non_empty(errors, "role_turns.turn_id", &turn.turn_id);
        require_non_empty(errors, "role_turns.role_id", &turn.role_id);
        require_non_empty(errors, "role_turns.trace_ref", &turn.trace_ref);
        require_non_empty(errors, "role_turns.provenance_ref", &turn.provenance_ref);
        require_vec(errors, "role_turns.replay_pins", &turn.replay_pins);

        if turn.requested_mode != RoleTurnExecutionMode::Isolated
            && !policy.allow_non_isolated_override
        {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.requested_mode",
                message: "non-isolated role turns require an explicit override",
            });
        }
        if turn.effective_mode == RoleTurnEffectiveMode::NonIsolated {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.effective_mode",
                message: "effective role-turn execution must mechanically prevent non-isolated cross-role bleed",
            });
        }
        if !turn.role_window_reset {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.role_window_reset",
                message: "isolated role turns must reset role window state",
            });
        }
        if !turn.context_window_reset {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.context_window_reset",
                message: "isolated role turns must reset context window state",
            });
        }
        if !turn.inherited_context_refs.is_empty() {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.inherited_context_refs",
                message: "isolated role turns must not inherit cross-role context refs",
            });
        }
        if turn.reset_support == RoleTurnResetSupport::Unsupported {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.reset_support",
                message: "unsupported reset backends cannot satisfy role-turn isolation",
            });
        }
        if turn.effective_mode == RoleTurnEffectiveMode::DegradedIsolated
            && turn.reset_support != RoleTurnResetSupport::DegradedReset
        {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.reset_support",
                message: "degraded isolated turns must record degraded reset support",
            });
        }
        if turn.effective_mode == RoleTurnEffectiveMode::Isolated
            && turn.reset_support != RoleTurnResetSupport::StrictReset
        {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.reset_support",
                message: "strict isolated turns must record strict reset support",
            });
        }

        validate_replay_pins(errors, turn, &mut pin_ids);
    }
}

fn validate_replay_pins(
    errors: &mut Vec<RoleTurnIsolationValidationError>,
    turn: &RoleTurnTraceRecordV1,
    pin_ids: &mut HashSet<String>,
) {
    let degraded_turn = turn.effective_mode == RoleTurnEffectiveMode::DegradedIsolated;
    let mut has_degraded_marker = false;

    for pin in &turn.replay_pins {
        if !pin_ids.insert(pin.pin_id.clone()) {
            errors.push(RoleTurnIsolationValidationError {
                field: "role_turns.replay_pins.pin_id",
                message: "replay pin ids must be unique across role turns",
            });
        }

        require_non_empty(errors, "role_turns.replay_pins.pin_id", &pin.pin_id);
        require_non_empty(errors, "role_turns.replay_pins.input_ref", &pin.input_ref);
        require_non_empty(
            errors,
            "role_turns.replay_pins.selected_span_ref",
            &pin.selected_span_ref,
        );
        require_non_empty(
            errors,
            "role_turns.replay_pins.content_hash",
            &pin.content_hash,
        );
        require_non_empty(
            errors,
            "role_turns.replay_pins.tie_break_key",
            &pin.tie_break_key,
        );

        if pin
            .degraded_marker
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            continue;
        }
        has_degraded_marker = true;
    }

    if degraded_turn && !has_degraded_marker {
        errors.push(RoleTurnIsolationValidationError {
            field: "role_turns.replay_pins.degraded_marker",
            message: "degraded isolated turns must record explicit replay degradation markers",
        });
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleTurnIsolationValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleTurnIsolationValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleTurnIsolationValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleTurnIsolationValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
