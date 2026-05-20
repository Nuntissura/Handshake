//! MT-123 cross-crate integration smoke for the distillation
//! candidate registry. Exhaustive coverage lives in the inline tests
//! in `distillation::candidate_registry::tests`; this file pins the
//! API surface + the PromotionGate behaviour.

use std::path::PathBuf;

use handshake_core::distillation::{
    candidate_registry::{
        CandidateAuditEvent, CandidateRegistry, MountDecision, ReviewStatus,
    },
    peft_pipeline::{DistilledLoraArtifact, PeftHyperparams},
};

fn artifact() -> DistilledLoraArtifact {
    DistilledLoraArtifact {
        lora_dir: PathBuf::from("lora"),
        teacher_model_path: PathBuf::from("teacher"),
        student_base_model_path: PathBuf::from("student"),
        corpus_path: PathBuf::from("corpus.jsonl"),
        corpus_turn_count: 10,
        corpus_quarantined_count: 1,
        corpus_rejected_count: 0,
        hyperparams: PeftHyperparams::default(),
        license_tag: "MIT".to_string(),
        operator_signature: "op".to_string(),
        finished_at_utc: "2026-05-20T04:00:00Z".to_string(),
    }
}

#[test]
fn candidate_registry_promotion_gate_pending_default() {
    let registry = CandidateRegistry::new();
    let event = registry
        .register("lora-1", artifact(), "2026-05-20T04:00:00Z")
        .expect("register");
    assert!(matches!(event, CandidateAuditEvent::Registered { .. }));

    let entry = registry.get("lora-1").unwrap().expect("entry");
    assert_eq!(entry.status, ReviewStatus::Pending);

    // Mount of a Pending candidate refuses by default.
    let decision = registry.mount_status("lora-1").unwrap();
    assert!(matches!(decision, MountDecision::Refuse { .. }));
}

#[test]
fn candidate_registry_promote_flips_to_allow_mount() {
    let registry = CandidateRegistry::new();
    registry
        .register("lora-2", artifact(), "2026-05-20T04:00:00Z")
        .unwrap();
    registry
        .promote("lora-2", "op-ilja", "2026-05-20T04:01:00Z")
        .expect("promote");
    let entry = registry.get("lora-2").unwrap().unwrap();
    assert_eq!(entry.status, ReviewStatus::Promoted);
    let decision = registry.mount_status("lora-2").unwrap();
    assert_eq!(decision, MountDecision::Allow);
}

#[test]
fn candidate_registry_reject_locks_mount_until_override() {
    let registry = CandidateRegistry::new();
    registry
        .register("lora-3", artifact(), "2026-05-20T04:00:00Z")
        .unwrap();
    registry
        .reject("lora-3", "op", "quality dropped", "2026-05-20T04:01:00Z")
        .unwrap();
    let decision = registry.mount_status("lora-3").unwrap();
    assert!(matches!(decision, MountDecision::Refuse { .. }));
    // Override flips to Allow for experimental sessions.
    let decision = registry
        .mount_status_with_override("lora-3", true)
        .unwrap();
    assert_eq!(decision, MountDecision::Allow);
}

#[test]
fn candidate_registry_externally_provided_lora_is_not_gated() {
    let registry = CandidateRegistry::new();
    let decision = registry.mount_status("externally-provided").unwrap();
    assert_eq!(decision, MountDecision::NotACandidate);
}
