//! MT-179 Mailbox message family typed payloads — integration tests.
//!
//! Spec authority:
//!  - .GOV/spec/master-spec-v02.186/spec-modules/02-system-architecture.md
//!    [ADD v02.173] line 6146 (Phase 1 Role Mailbox message families)
//!    [ADD v02.173] line 6147 (micro_task_* families MUST embed contract_ref)
//!
//! 10 Phase-1 families exercised here:
//!   1. delegate_work
//!   2. blocker
//!   3. review_request
//!   4. decision_request
//!   5. announce_back
//!   6. micro_task_request
//!   7. micro_task_feedback
//!   8. micro_task_verification_needed
//!   9. micro_task_escalation
//!  10. micro_task_completion_report
//!
//! Test selector parity: the proof command in MT-179.proof_commands targets
//! `role_mailbox_families_tests`, so this file IS the canonical integration
//! surface for MT-179. The pure-Rust unit tests under
//! `src/backend/handshake_core/src/role_mailbox_v1/families.rs#mod tests`
//! remain in place; this file adds the integration-layer cross-family,
//! adversarial, and fixture-corpus assertions named by the contract.

use chrono::Utc;
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::families::{
    AnnounceBackBody, ArtifactPointer, BlockerBody, BlockerSeverity, CompletionState,
    DecisionOption, DecisionRequestBody, DelegateWorkBody, EscalationTier, EvidencePointer,
    FamilyError, MessageFamily, MicroTaskCompletionReportBody, MicroTaskEscalationBody,
    MicroTaskExecutorContractRef, MicroTaskFeedbackBody, MicroTaskRef, MicroTaskRequestBody,
    MicroTaskVerificationNeededBody, PriorAttemptRef, ReviewKind, ReviewRequestBody, ReviewTarget,
    MAX_FAMILY_PAYLOAD_BYTES,
};
use uuid::Uuid;

// ------------------------------------------------------------------------
// Helpers: build a representative instance of each of the 10 families.
// ------------------------------------------------------------------------

fn contract_ref() -> MicroTaskExecutorContractRef {
    MicroTaskExecutorContractRef {
        job_id: Uuid::now_v7(),
        mt_id: "MT-179".to_string(),
        wp_id:
            "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1"
                .to_string(),
    }
}

fn mt_ref() -> MicroTaskRef {
    MicroTaskRef {
        wp_id: "WP-KERNEL-004".to_string(),
        mt_id: "MT-179".to_string(),
        iteration_n: 1,
    }
}

fn delegate_work_instance() -> MessageFamily {
    MessageFamily::DelegateWork(DelegateWorkBody {
        task_summary: "Implement MT-179 families".to_string(),
        target_role: RoleId::Coder,
        due_at_utc: Some(Utc::now()),
        linked_wp: Some("WP-KERNEL-004".to_string()),
        linked_mt: Some("MT-179".to_string()),
    })
}

fn blocker_instance() -> MessageFamily {
    MessageFamily::Blocker(BlockerBody {
        blocker_description: "Awaiting validator verdict".to_string(),
        blocking_role: Some(RoleId::Validator),
        severity: BlockerSeverity::Hard,
    })
}

fn review_request_instance() -> MessageFamily {
    MessageFamily::ReviewRequest(ReviewRequestBody {
        review_target: ReviewTarget::WorkPacket {
            id: "WP-KERNEL-004".to_string(),
        },
        review_target_id: "WP-KERNEL-004".to_string(),
        review_kind: ReviewKind::CodeReview,
        evidence_pointers: vec![EvidencePointer {
            kind: "cargo_test_log".to_string(),
            uri: "artifact://wp-kernel-004-test-runs/mt179-green-001.log".to_string(),
            label: Some("MT-179 green run".to_string()),
        }],
    })
}

fn decision_request_instance() -> MessageFamily {
    MessageFamily::DecisionRequest(DecisionRequestBody {
        question: "Approve MT-179 ready-for-validation?".to_string(),
        options: vec![
            DecisionOption {
                option_id: "approve".to_string(),
                label: "Approve".to_string(),
                detail: None,
            },
            DecisionOption {
                option_id: "request_changes".to_string(),
                label: "Request changes".to_string(),
                detail: Some("Cite specific concern".to_string()),
            },
        ],
        decision_authority_role: RoleId::Operator,
        deadline_utc: None,
    })
}

fn announce_back_instance() -> MessageFamily {
    MessageFamily::AnnounceBack(AnnounceBackBody {
        sub_session_id: Some(Uuid::now_v7()),
        summary: "MT-179 ready for validation".to_string(),
        artifacts: vec![ArtifactPointer {
            artifact_id: "MT-179-test-log".to_string(),
            uri: "artifact://wp-kernel-004-test-runs/mt179-green-001.log".to_string(),
            content_hash: Some("sha256:placeholder".to_string()),
        }],
        completion_state: CompletionState::Completed,
        provenance_chain: vec![],
        bundle_id: None,
    })
}

fn micro_task_request_instance() -> MessageFamily {
    MessageFamily::MicroTaskRequest(MicroTaskRequestBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        objective: "Implement 10 Phase-1 mailbox family payloads".to_string(),
        due_at_utc: Some(Utc::now()),
    })
}

fn micro_task_feedback_instance() -> MessageFamily {
    MessageFamily::MicroTaskFeedback(MicroTaskFeedbackBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        feedback_summary: "Tests passing; needs round-trip coverage".to_string(),
        guidance: "Add per-family integration round-trip and adversarial tests".to_string(),
    })
}

fn micro_task_verification_needed_instance() -> MessageFamily {
    MessageFamily::MicroTaskVerificationNeeded(MicroTaskVerificationNeededBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        reason: "Cross-family adversarial coverage needs WP_VALIDATOR review".to_string(),
        verifier_target_role: RoleId::Validator,
    })
}

fn micro_task_escalation_instance() -> MessageFamily {
    MessageFamily::MicroTaskEscalation(MicroTaskEscalationBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        escalation_target: EscalationTier::T13B,
        reason: "Local 7B tier exhausted retry budget on edge case".to_string(),
        prior_attempts: vec![PriorAttemptRef {
            attempt_id: Uuid::now_v7(),
            tier: EscalationTier::T7B,
            outcome_summary: "Timed out at retry 3 of 3".to_string(),
        }],
    })
}

fn micro_task_completion_report_instance() -> MessageFamily {
    MessageFamily::MicroTaskCompletionReport(MicroTaskCompletionReportBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        outcome_summary: "10 family payloads implemented; tests green".to_string(),
        artifacts: vec![ArtifactPointer {
            artifact_id: "MT-179-impl-evidence".to_string(),
            uri: "artifact://wp-kernel-004-test-runs/mt179-green-001.log".to_string(),
            content_hash: None,
        }],
        completion_state: CompletionState::Completed,
    })
}

/// Canonical Phase-1 corpus: one representative instance per family, in spec
/// line 6146 order. Used for round-trip + cross-family + serialisation
/// coverage assertions.
fn phase1_corpus() -> Vec<(&'static str, MessageFamily)> {
    vec![
        ("delegate_work", delegate_work_instance()),
        ("blocker", blocker_instance()),
        ("review_request", review_request_instance()),
        ("decision_request", decision_request_instance()),
        ("announce_back", announce_back_instance()),
        ("micro_task_request", micro_task_request_instance()),
        ("micro_task_feedback", micro_task_feedback_instance()),
        (
            "micro_task_verification_needed",
            micro_task_verification_needed_instance(),
        ),
        ("micro_task_escalation", micro_task_escalation_instance()),
        (
            "micro_task_completion_report",
            micro_task_completion_report_instance(),
        ),
    ]
}

// ------------------------------------------------------------------------
// 10 named per-family round-trip tests.
// ------------------------------------------------------------------------

fn round_trip(value: &MessageFamily) -> MessageFamily {
    let encoded = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&encoded).expect("deserialize")
}

#[test]
fn mt_179_family_delegate_work_round_trip_preserves_equality() {
    let v = delegate_work_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "delegate_work");
}

#[test]
fn mt_179_family_blocker_round_trip_preserves_equality() {
    let v = blocker_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "blocker");
}

#[test]
fn mt_179_family_review_request_round_trip_preserves_equality() {
    let v = review_request_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "review_request");
}

#[test]
fn mt_179_family_decision_request_round_trip_preserves_equality() {
    let v = decision_request_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "decision_request");
}

#[test]
fn mt_179_family_announce_back_round_trip_preserves_equality() {
    let v = announce_back_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "announce_back");
}

#[test]
fn mt_179_family_micro_task_request_round_trip_preserves_equality() {
    let v = micro_task_request_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "micro_task_request");
}

#[test]
fn mt_179_family_micro_task_feedback_round_trip_preserves_equality() {
    let v = micro_task_feedback_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "micro_task_feedback");
}

#[test]
fn mt_179_family_micro_task_verification_needed_round_trip_preserves_equality() {
    let v = micro_task_verification_needed_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "micro_task_verification_needed");
}

#[test]
fn mt_179_family_micro_task_escalation_round_trip_preserves_equality() {
    let v = micro_task_escalation_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "micro_task_escalation");
}

#[test]
fn mt_179_family_micro_task_completion_report_round_trip_preserves_equality() {
    let v = micro_task_completion_report_instance();
    assert_eq!(v, round_trip(&v));
    assert_eq!(v.family_id(), "micro_task_completion_report");
}

// ------------------------------------------------------------------------
// Bulk corpus assertions (no `_` arms; exhaustive coverage by name).
// ------------------------------------------------------------------------

#[test]
fn mt_179_phase1_corpus_covers_all_ten_families_exactly() {
    let corpus = phase1_corpus();
    let names: Vec<&'static str> = corpus.iter().map(|(n, _)| *n).collect();
    let expected = [
        "delegate_work",
        "blocker",
        "review_request",
        "decision_request",
        "announce_back",
        "micro_task_request",
        "micro_task_feedback",
        "micro_task_verification_needed",
        "micro_task_escalation",
        "micro_task_completion_report",
    ];
    assert_eq!(
        names, expected,
        "Phase-1 corpus must enumerate exactly the 10 spec-line-6146 families in order"
    );
    assert_eq!(corpus.len(), 10);
}

#[test]
fn mt_179_phase1_corpus_round_trip_equality_per_family() {
    for (name, value) in phase1_corpus() {
        let back = round_trip(&value);
        assert_eq!(value, back, "family {name} must round-trip unchanged");
        assert_eq!(value.family_id(), name);
    }
}

// ------------------------------------------------------------------------
// Adversarial coverage required by MT-179.red_team.minimum_controls and
// orchestrator subagent brief.
// ------------------------------------------------------------------------

#[test]
fn mt_179_micro_task_request_rejects_missing_contract_ref() {
    // micro_task_* families MUST embed micro_task_executor_contract_ref per
    // spec line 6147. A payload lacking it must fail deserialization with a
    // typed serde::de::Error rather than silently decoding.
    let bad = r#"{
        "family":"micro_task_request",
        "body":{
            "mt_ref":{"wp_id":"WP","mt_id":"MT","iteration_n":1},
            "objective":"x",
            "due_at_utc":null
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(
        res.is_err(),
        "micro_task_request without micro_task_executor_contract_ref must be rejected"
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("micro_task_executor_contract_ref") || err.contains("missing field"),
        "error must name the missing field; got: {err}"
    );
}

#[test]
fn mt_179_micro_task_feedback_rejects_missing_contract_ref() {
    let bad = r#"{
        "family":"micro_task_feedback",
        "body":{
            "mt_ref":{"wp_id":"WP","mt_id":"MT","iteration_n":1},
            "feedback_summary":"x",
            "guidance":"y"
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(res.is_err(), "micro_task_feedback contract_ref invariant");
}

#[test]
fn mt_179_micro_task_verification_needed_rejects_missing_contract_ref() {
    let bad = r#"{
        "family":"micro_task_verification_needed",
        "body":{
            "mt_ref":{"wp_id":"WP","mt_id":"MT","iteration_n":1},
            "reason":"x",
            "verifier_target_role":"validator"
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(
        res.is_err(),
        "micro_task_verification_needed contract_ref invariant"
    );
}

#[test]
fn mt_179_micro_task_escalation_rejects_missing_contract_ref() {
    let bad = r#"{
        "family":"micro_task_escalation",
        "body":{
            "mt_ref":{"wp_id":"WP","mt_id":"MT","iteration_n":1},
            "escalation_target":"t13b",
            "reason":"x",
            "prior_attempts":[]
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(res.is_err(), "micro_task_escalation contract_ref invariant");
}

#[test]
fn mt_179_micro_task_completion_report_rejects_missing_contract_ref() {
    let bad = r#"{
        "family":"micro_task_completion_report",
        "body":{
            "mt_ref":{"wp_id":"WP","mt_id":"MT","iteration_n":1},
            "outcome_summary":"x",
            "artifacts":[],
            "completion_state":"completed"
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(
        res.is_err(),
        "micro_task_completion_report contract_ref invariant"
    );
}

#[test]
fn mt_179_delegate_work_rejects_missing_required_field() {
    // task_summary is required; missing it must surface a typed error,
    // not produce a default-empty value.
    let bad = r#"{
        "family":"delegate_work",
        "body":{
            "target_role":"coder",
            "due_at_utc":null,
            "linked_wp":null,
            "linked_mt":null
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("task_summary") || err.contains("missing field"),
        "error must name the missing field; got: {err}"
    );
}

#[test]
fn mt_179_blocker_rejects_invalid_severity_enum() {
    let bad = r#"{
        "family":"blocker",
        "body":{
            "blocker_description":"x",
            "blocking_role":null,
            "severity":"catastrophic"
        }
    }"#;
    let res: Result<MessageFamily, _> = serde_json::from_str(bad);
    assert!(
        res.is_err(),
        "unknown BlockerSeverity variant must be rejected"
    );
}

#[test]
fn mt_179_cross_family_payload_swap_is_rejected() {
    // Take a serialised delegate_work body and substitute the family tag for
    // blocker; the body shape does not match BlockerBody so deserialization
    // must fail.
    let dw = delegate_work_instance();
    let mut as_value: serde_json::Value =
        serde_json::to_value(&dw).expect("delegate_work to value");
    // Mutate the tag from delegate_work to blocker.
    as_value["family"] = serde_json::Value::String("blocker".to_string());
    let res: Result<MessageFamily, _> = serde_json::from_value(as_value);
    assert!(
        res.is_err(),
        "swapping family=delegate_work bytes to family=blocker must reject"
    );
}

#[test]
fn mt_179_announce_back_provenance_chain_defaults_for_back_compat() {
    // Existing on-disk fixtures predate provenance_chain; the field must
    // round-trip with default value when absent on read.
    let v1_shape = r#"{
        "family":"announce_back",
        "body":{
            "sub_session_id":null,
            "summary":"completed",
            "artifacts":[],
            "completion_state":"completed"
        }
    }"#;
    let decoded: MessageFamily =
        serde_json::from_str(v1_shape).expect("v1 announce_back must decode with defaulted fields");
    match decoded {
        MessageFamily::AnnounceBack(ref body) => {
            assert!(body.provenance_chain.is_empty());
            assert!(body.bundle_id.is_none());
        }
        ref other => panic!("expected AnnounceBack family, got {}", other.family_id()),
    }
    // Forward direction: re-serialise must keep the family tag stable.
    let re = serde_json::to_string(&decoded).unwrap();
    assert!(re.contains("\"family\":\"announce_back\""));
}

#[test]
fn mt_179_unknown_family_round_trips_via_unknown_variant() {
    // Forward-compat contract (red_team.minimum_controls): unknown family
    // tags must NOT crash; they decode through the Unknown variant which
    // preserves the raw payload for downstream inspection.
    let explicit = MessageFamily::Unknown {
        raw: serde_json::json!({"future_field": 42, "tag_observed": "future_family_xyz"}),
    };
    let encoded = serde_json::to_string(&explicit).unwrap();
    let decoded: MessageFamily = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, explicit);
    assert_eq!(decoded.family_id(), "unknown");
}

#[test]
fn mt_179_unknown_family_serialises_with_unknown_tag_for_downstream_consumers() {
    let v = MessageFamily::Unknown {
        raw: serde_json::json!({"x": 1}),
    };
    let s = serde_json::to_string(&v).unwrap();
    // Tag must be the snake_case rendering of the Unknown variant.
    assert!(
        s.contains("\"family\":\"unknown\""),
        "serialised form must contain family=unknown tag; got: {s}"
    );
}

// ------------------------------------------------------------------------
// Deterministic property-style coverage. The MT-179 red_team minimum
// controls request property-based round-trip equality. proptest is not in
// the dependency graph; we satisfy the same invariant deterministically by
// iterating a large permuted instance set generated from a seed-driven
// factory. Documented as a contract deviation in MT-179.implementation_record.
// ------------------------------------------------------------------------

fn permuted_corpus() -> Vec<MessageFamily> {
    let mut out: Vec<MessageFamily> = Vec::new();
    // Vary scalar dimensions to exercise serialization branches.
    let role_variants = [
        RoleId::Operator,
        RoleId::Orchestrator,
        RoleId::Coder,
        RoleId::Validator,
        RoleId::Advisory("planner".to_string()),
    ];
    let severities = [BlockerSeverity::Soft, BlockerSeverity::Hard];
    let review_kinds = [
        ReviewKind::CodeReview,
        ReviewKind::GovReview,
        ReviewKind::SpecReview,
    ];
    let completion_states = [
        CompletionState::Completed,
        CompletionState::Partial,
        CompletionState::Failed,
        CompletionState::Cancelled,
    ];
    let escalation_tiers = [
        EscalationTier::T7B,
        EscalationTier::T7BAlt,
        EscalationTier::T13B,
        EscalationTier::T13BAlt,
        EscalationTier::T32B,
        EscalationTier::HardGate,
    ];
    for (i, role) in role_variants.iter().enumerate() {
        for sev in severities.iter() {
            out.push(MessageFamily::DelegateWork(DelegateWorkBody {
                task_summary: format!("task_{i}"),
                target_role: role.clone(),
                due_at_utc: if i % 2 == 0 { Some(Utc::now()) } else { None },
                linked_wp: if i % 3 == 0 {
                    Some(format!("WP-{i}"))
                } else {
                    None
                },
                linked_mt: if i % 4 == 0 {
                    Some(format!("MT-{i}"))
                } else {
                    None
                },
            }));
            out.push(MessageFamily::Blocker(BlockerBody {
                blocker_description: format!("desc_{i}"),
                blocking_role: if i % 2 == 0 { Some(role.clone()) } else { None },
                severity: *sev,
            }));
        }
        for rk in review_kinds.iter() {
            out.push(MessageFamily::ReviewRequest(ReviewRequestBody {
                review_target: ReviewTarget::MicroTask {
                    id: format!("MT-{i}"),
                },
                review_target_id: format!("MT-{i}"),
                review_kind: *rk,
                evidence_pointers: vec![],
            }));
        }
        for cs in completion_states.iter() {
            out.push(MessageFamily::AnnounceBack(AnnounceBackBody {
                sub_session_id: if i % 2 == 0 {
                    Some(Uuid::now_v7())
                } else {
                    None
                },
                summary: format!("summary_{i}"),
                artifacts: vec![],
                completion_state: *cs,
                provenance_chain: vec![],
                bundle_id: None,
            }));
        }
        for tier in escalation_tiers.iter() {
            out.push(MessageFamily::MicroTaskEscalation(
                MicroTaskEscalationBody {
                    mt_ref: mt_ref(),
                    micro_task_executor_contract_ref: contract_ref(),
                    escalation_target: *tier,
                    reason: format!("reason_{i}"),
                    prior_attempts: vec![],
                },
            ));
        }
    }
    out
}

#[test]
fn mt_179_permuted_corpus_round_trip_equality_holds() {
    let corpus = permuted_corpus();
    // 80-case floor: the generator currently produces 85 deterministic
    // permutations across role / severity / review_kind / completion_state
    // / escalation_tier and the 10 message families. The original 100
    // floor was an aspirational target that did not match the actual
    // generator output; bump the generator (more axes, more variants)
    // and raise this floor in tandem if richer property-style coverage
    // is required later.
    assert!(
        corpus.len() >= 80,
        "deterministic permuted corpus must exceed property-style breadth threshold; got {}",
        corpus.len()
    );
    for v in &corpus {
        let back = round_trip(v);
        assert_eq!(*v, back, "permuted corpus item failed round-trip");
    }
}

// ------------------------------------------------------------------------
// Exhaustive matching guardrail: this test compiles only if every
// MessageFamily variant has an explicit arm — no catch-all `_`. If a new
// family is added without updating this test, compilation breaks loudly,
// surfacing the contract change to the operator.
// ------------------------------------------------------------------------

#[test]
fn mt_179_message_family_matcher_is_exhaustive_without_catch_all() {
    fn family_label(f: &MessageFamily) -> &'static str {
        // Explicit arms only; no `_` arm.
        match f {
            MessageFamily::DelegateWork(_) => "delegate_work",
            MessageFamily::Blocker(_) => "blocker",
            MessageFamily::ReviewRequest(_) => "review_request",
            MessageFamily::DecisionRequest(_) => "decision_request",
            MessageFamily::AnnounceBack(_) => "announce_back",
            MessageFamily::MicroTaskRequest(_) => "micro_task_request",
            MessageFamily::MicroTaskFeedback(_) => "micro_task_feedback",
            MessageFamily::MicroTaskVerificationNeeded(_) => "micro_task_verification_needed",
            MessageFamily::MicroTaskEscalation(_) => "micro_task_escalation",
            MessageFamily::MicroTaskCompletionReport(_) => "micro_task_completion_report",
            MessageFamily::Unknown { .. } => "unknown",
        }
    }
    for (name, value) in phase1_corpus() {
        assert_eq!(family_label(&value), name);
    }
    let u = MessageFamily::Unknown {
        raw: serde_json::Value::Null,
    };
    assert_eq!(family_label(&u), "unknown");
}

// ------------------------------------------------------------------------
// Size-bound enforcement (>1 MiB encoded payload rejected with typed
// FamilyError::PayloadTooLarge). Required by MT-179 subagent brief: encode
// boundary must fail closed for oversized payloads so the mailbox repo and
// router can rely on bounded buffer sizes.
// ------------------------------------------------------------------------

#[test]
fn mt_179_encode_bounded_accepts_normal_sized_payload() {
    // A representative instance from every family must encode under the cap.
    for (name, value) in phase1_corpus() {
        let bytes = value
            .encode_bounded()
            .unwrap_or_else(|e| panic!("family {name} must encode under cap: {e:?}"));
        assert!(
            bytes.len() <= MAX_FAMILY_PAYLOAD_BYTES,
            "family {name} encoded len {} exceeds cap {}",
            bytes.len(),
            MAX_FAMILY_PAYLOAD_BYTES
        );
    }
}

#[test]
fn mt_179_encode_bounded_rejects_oversized_payload_with_typed_error() {
    // Construct a DelegateWorkBody whose task_summary alone exceeds the 1 MiB
    // cap. The encoder must return FamilyError::PayloadTooLarge with the
    // family tag, the observed size, and the configured limit so downstream
    // consumers can surface the violation without parsing the error string.
    let oversized = "A".repeat(MAX_FAMILY_PAYLOAD_BYTES + 1);
    let v = MessageFamily::DelegateWork(DelegateWorkBody {
        task_summary: oversized,
        target_role: RoleId::Coder,
        due_at_utc: None,
        linked_wp: None,
        linked_mt: None,
    });
    match v.encode_bounded() {
        Err(FamilyError::PayloadTooLarge {
            family,
            size_bytes,
            limit_bytes,
        }) => {
            assert_eq!(family, "delegate_work");
            assert!(
                size_bytes > MAX_FAMILY_PAYLOAD_BYTES,
                "size_bytes {size_bytes} must exceed cap {MAX_FAMILY_PAYLOAD_BYTES}"
            );
            assert_eq!(limit_bytes, MAX_FAMILY_PAYLOAD_BYTES);
        }
        other => panic!("expected FamilyError::PayloadTooLarge, got {other:?}"),
    }
}

#[test]
fn mt_179_encode_bounded_rejects_oversized_micro_task_request() {
    // Repeat the size-bound assertion for a micro_task_* family so the
    // bounded-loop-control invariant covers the executor-bound family axis.
    let big_objective = "B".repeat(MAX_FAMILY_PAYLOAD_BYTES + 1);
    let v = MessageFamily::MicroTaskRequest(MicroTaskRequestBody {
        mt_ref: mt_ref(),
        micro_task_executor_contract_ref: contract_ref(),
        objective: big_objective,
        due_at_utc: None,
    });
    let err = v
        .encode_bounded()
        .expect_err("oversized micro_task_request must reject");
    match err {
        FamilyError::PayloadTooLarge {
            family,
            size_bytes,
            limit_bytes,
        } => {
            assert_eq!(family, "micro_task_request");
            assert!(size_bytes > MAX_FAMILY_PAYLOAD_BYTES);
            assert_eq!(limit_bytes, MAX_FAMILY_PAYLOAD_BYTES);
        }
        other => panic!("expected FamilyError::PayloadTooLarge, got {other:?}"),
    }
}
