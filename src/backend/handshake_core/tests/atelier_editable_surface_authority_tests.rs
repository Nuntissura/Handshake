//! WP-KERNEL-005 MT-149 proof — EditableSurface providers wired to the
//! embedded-store authority records.
//!
//! The production editable-surface providers run the full isolate -> propose ->
//! promote contract against the embedded store: `snapshot` reads
//! the persisted live value (seeded from the real ModelManual), `promote`
//! writes through the single authority write path, the promoted value is
//! RE-READ from the embedded store, and every live-authority write mirrors through
//! the EventLedger. No closures assert test-authored constants.

mod atelier_surreal_support;

use std::sync::Arc;

use handshake_core::atelier::editable_surface_authority::{
    editable_surface_event_family, live_model_manual_surface as embedded_model_manual_surface,
    live_retrieval_policy_surface as embedded_retrieval_policy_surface, policy_parameter_token,
    task_type_token,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::KernelEventType;
use handshake_core::memory::{CapsulePolicyTable, TaskType};
use handshake_core::model_manual::model_manual;
use handshake_core::self_improve::editable_surface::{
    EditableSurfaceProvider, EditableSurfaceSnapshot, SurfaceProposal,
};
use handshake_core::self_improve::iteration::{LoopTarget, PolicyParameterRef};
use handshake_core::storage::Database;
use uuid::Uuid;

async fn connected_store_with_ledger() -> (
    AtelierStore,
    Arc<dyn Database>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness.database.clone(), harness)
}

/// Seed text lifted from the real built-in ModelManual so the live
/// authority row starts from the real surface, not a test literal.
fn real_manual_seed_text() -> String {
    let manual = model_manual();
    let group = manual
        .feature_groups
        .iter()
        .find(|group| {
            let lowered = group.id.to_ascii_lowercase();
            !lowered.contains("spec")
                && !lowered.contains("role")
                && !lowered.contains("lora")
                && !lowered.contains("weights")
                && !lowered.contains("tool_description")
        })
        .expect("the real ModelManual must expose a loopable feature group");
    format!(
        "[{} v{}] {}: {}",
        group.id, manual.version, group.title, group.description
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_model_manual_surface_promotes_through_live_embedded_authority() {
    let (store, database, _harness) = connected_store_with_ledger().await;

    // Seed the live authority row from the real ModelManual.
    let section_id = format!("manual.capsule.intro-{}", Uuid::now_v7());
    let seed_text = real_manual_seed_text();
    let seeded = store
        .upsert_model_manual_section(&section_id, &seed_text, "manual-seed")
        .await
        .expect("seed live manual section");
    assert_eq!(seeded.revision, 1);

    // Production provider, wired to the live embedded authority record.
    let provider = embedded_model_manual_surface(store.clone(), "self-improve-loop".to_string());
    let target = LoopTarget::ModelManualCapsuleText {
        manual_section_id: section_id.clone(),
    };

    // snapshot READS the live authority surface (before == after == seed).
    let snapshot = provider.snapshot(&target).expect("snapshot live section");
    match &snapshot {
        EditableSurfaceSnapshot::ModelManual {
            before_text,
            after_text,
            ..
        } => {
            assert_eq!(
                before_text, &seed_text,
                "snapshot must read durable state, not a literal"
            );
            assert_eq!(after_text, &seed_text);
        }
        other => panic!("expected ModelManual snapshot, got {other:?}"),
    }

    // apply_proposal is sandbox-scoped: the live row must NOT move yet.
    let candidate_text = format!("{seed_text}\n\nLoop-tuned recovery guidance.");
    let proposed = provider
        .apply_proposal(
            &snapshot,
            SurfaceProposal::ModelManualText {
                new_text: candidate_text.clone(),
            },
        )
        .expect("apply candidate proposal");
    let mid = store
        .get_model_manual_section(&section_id)
        .await
        .expect("re-read live section after proposal")
        .expect("live section row");
    assert_eq!(
        mid.section_text, seed_text,
        "apply_proposal must not write the live authority surface"
    );
    assert_eq!(mid.revision, 1);

    // promote writes the candidate through the single authority path.
    provider
        .promote(&proposed)
        .expect("promote gated candidate");
    let promoted = store
        .get_model_manual_section(&section_id)
        .await
        .expect("re-read live section after promote")
        .expect("live section row");
    assert_eq!(promoted.section_text, candidate_text);
    assert_eq!(promoted.revision, 2, "promotion must bump the revision");
    assert_eq!(promoted.updated_by, "self-improve-loop");

    // A fresh provider snapshot now reads the promoted live value.
    let after = provider
        .snapshot(&target)
        .expect("snapshot promoted section");
    match &after {
        EditableSurfaceSnapshot::ModelManual { before_text, .. } => {
            assert_eq!(before_text, &candidate_text);
        }
        other => panic!("expected ModelManual snapshot, got {other:?}"),
    }

    // Both live-authority writes mirrored through the EventLedger.
    let events = database
        .list_kernel_events_for_aggregate("atelier_model_manual_section", &section_id)
        .await
        .expect("list manual section EventLedger rows");
    let written: Vec<_> = events
        .iter()
        .filter(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == editable_surface_event_family::MODEL_MANUAL_SECTION_WRITTEN
        })
        .collect();
    assert_eq!(written.len(), 2, "seed + promote must each emit an event");
    assert!(written.iter().any(|event| {
        event.payload["atelier_payload"]["revision"] == serde_json::json!(2)
            && event.payload["atelier_payload"]["updated_by"]
                == serde_json::json!("self-improve-loop")
    }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_model_manual_surface_noop_promote_writes_nothing() {
    let (store, _database, _harness) = connected_store_with_ledger().await;

    let section_id = format!("manual.capsule.noop-{}", Uuid::now_v7());
    store
        .upsert_model_manual_section(&section_id, &real_manual_seed_text(), "manual-seed")
        .await
        .expect("seed live manual section");

    let provider = embedded_model_manual_surface(store.clone(), "self-improve-loop".to_string());
    let target = LoopTarget::ModelManualCapsuleText {
        manual_section_id: section_id.clone(),
    };
    let snapshot = provider.snapshot(&target).expect("snapshot live section");
    provider
        .promote(&snapshot)
        .expect("no-op promote must succeed");

    let reread = store
        .get_model_manual_section(&section_id)
        .await
        .expect("re-read live section")
        .expect("live section row");
    assert_eq!(
        reread.revision, 1,
        "no-op promote must not bump the revision"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_retrieval_policy_surface_defaults_then_promotes_to_embedded_store() {
    let (store, database, _harness) = connected_store_with_ledger().await;

    let task_type = TaskType::SelfImprovementLoopEval;
    let parameter = PolicyParameterRef::TopK;

    let provider =
        embedded_retrieval_policy_surface(store.clone(), "self-improve-loop".to_string());
    let target = LoopTarget::RetrievalPolicyParams {
        task_type,
        parameter,
    };

    // With no live row, snapshot reads the real capsule policy default.
    let default_top_k = u64::from(CapsulePolicyTable::default_policy_for(task_type).top_k);
    let snapshot = provider.snapshot(&target).expect("snapshot policy surface");
    match &snapshot {
        EditableSurfaceSnapshot::RetrievalPolicy { before_value, .. } => {
            assert_eq!(
                *before_value, default_top_k,
                "missing live row must fall back to CapsulePolicyTable defaults"
            );
        }
        other => panic!("expected RetrievalPolicy snapshot, got {other:?}"),
    }

    // Propose + promote a different in-range top_k.
    let candidate = if default_top_k >= 64 {
        32
    } else {
        default_top_k + 2
    };
    let proposed = provider
        .apply_proposal(
            &snapshot,
            SurfaceProposal::RetrievalPolicyValue {
                new_value: candidate,
            },
        )
        .expect("apply candidate policy value");
    provider.promote(&proposed).expect("promote policy value");

    // RE-READ the live authority row from the embedded store.
    let live = store
        .get_retrieval_policy_value(task_type, parameter)
        .await
        .expect("re-read live policy value")
        .expect("promoted policy row must exist");
    assert_eq!(live.value, i64::try_from(candidate).unwrap());
    assert_eq!(live.updated_by, "self-improve-loop");

    // The provider now reads the promoted live value, not the default.
    let after = provider
        .snapshot(&target)
        .expect("snapshot promoted policy");
    match &after {
        EditableSurfaceSnapshot::RetrievalPolicy { before_value, .. } => {
            assert_eq!(*before_value, candidate);
        }
        other => panic!("expected RetrievalPolicy snapshot, got {other:?}"),
    }

    // The live-authority write mirrored through the EventLedger.
    let aggregate_id = format!(
        "{}:{}",
        task_type_token(task_type),
        policy_parameter_token(parameter)
    );
    let events = database
        .list_kernel_events_for_aggregate("atelier_retrieval_policy", &aggregate_id)
        .await
        .expect("list retrieval policy EventLedger rows");
    let event = events
        .iter()
        .filter(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == editable_surface_event_family::RETRIEVAL_POLICY_WRITTEN
        })
        .last()
        .expect("policy promotion must emit canonical EventLedger event");
    // `value` is on the atelier event sanitizer's sensitive-key list, so the
    // ledger mirrors it as the canonical deterministic `value_ref`
    // (sha256 of the JSON-serialized value), never the raw number.
    let expected_value_ref = format!(
        "sha256:{}",
        hex::encode(<sha2::Sha256 as sha2::Digest>::digest(
            serde_json::to_vec(&serde_json::json!(candidate)).expect("serialize candidate"),
        ))
    );
    assert_eq!(
        event.payload["atelier_payload"]["value_ref"],
        serde_json::json!(expected_value_ref),
        "promotion event must mirror the promoted value as its canonical value_ref"
    );
    assert_eq!(
        event.payload["atelier_payload"]["task_type"],
        serde_json::json!("self_improvement_loop_eval")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_policy_surface_clamps_out_of_range_proposals_before_any_durable_write() {
    let (store, _database, _harness) = connected_store_with_ledger().await;

    let task_type = TaskType::OperatorTriage;
    let parameter = PolicyParameterRef::TopK;
    let provider =
        embedded_retrieval_policy_surface(store.clone(), "self-improve-loop".to_string());
    let target = LoopTarget::RetrievalPolicyParams {
        task_type,
        parameter,
    };
    let snapshot = provider.snapshot(&target).expect("snapshot policy surface");
    provider
        .apply_proposal(
            &snapshot,
            SurfaceProposal::RetrievalPolicyValue { new_value: 65 },
        )
        .expect_err("top_k above the durable cap must be rejected");

    // Nothing reached the live authority table.
    let live = store
        .get_retrieval_policy_value(task_type, parameter)
        .await
        .expect("re-read live policy value");
    assert!(
        live.is_none(),
        "rejected proposal must not write the live policy row"
    );
}
