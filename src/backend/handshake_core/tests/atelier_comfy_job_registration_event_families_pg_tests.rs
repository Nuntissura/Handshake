//! WP-KERNEL-005 MT-130: ComfyUI queue/run/result + output-registration
//! recovery event families, proven against live Handshake-managed PostgreSQL.
//!
//! The replay subset of MT-130 (REPLAY_REQUESTED/COMPLETED/FAILED) is proven
//! by `atelier_replay_contract_tests.rs`. This file closes the v2 concern for
//! the remaining contract families in ONE end-to-end flow: a job is enqueued,
//! run, and resolved (JOB_ENQUEUED / JOB_RUNNING / JOB_COMPLETED), and a
//! saved-but-unregistered output is preserved and then recovered
//! (OUTPUT_REGISTRATION_FAILURE_RECORDED / OUTPUT_REGISTRATION_FAILURE_RETRIED).
//! Every assertion RE-READS persisted state from PostgreSQL and counts
//! EventLedger rows run-scoped (per aggregate, never global).
//!
//! Run, e.g.:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test atelier_comfy_job_registration_event_families_pg_tests \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::comfy::{
    comfy_event_family, ComfyBridgeFakeAdapterV1, ComfyJobStatus,
    ComfyOutputRegistrationFailureStatus, MediaKind, NewComfyJobRequest,
    NewComfyOutputRegistrationFailure, RoutingIntent,
};
use handshake_core::atelier::{event_family, AtelierStore};
use serde_json::json;
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// MT-130: queue/run/result and registration success/failure-recovery event
/// families are emitted by the real store and land in the EventLedger.
#[tokio::test]
async fn mt130_job_lifecycle_and_registration_recovery_event_families_land_in_ledger() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt130_job_lifecycle_and_registration_recovery_event_families_land_in_ledger: no PostgreSQL"
        );
        return;
    };
    let store = connected_store(&url).await;

    // Every contract family is folded into the aggregate parity list.
    for family in [
        comfy_event_family::JOB_ENQUEUED,
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
        comfy_event_family::REPLAY_REQUESTED,
        comfy_event_family::REPLAY_COMPLETED,
        comfy_event_family::REPLAY_FAILED,
    ] {
        assert!(
            event_family::ALL.contains(&family),
            "{family} must be registered in the aggregate event family list"
        );
    }

    // --- queue/run/result: QUEUED -> RUNNING -> COMPLETED through live PG ---
    let run_id = Uuid::new_v4();
    let job = store
        .enqueue_comfy_job(&NewComfyJobRequest {
            workflow_run_id: run_id,
            spec_id: None,
            request_json: json!({
                "graph": { "nodes": [{ "id": "1", "class_type": "PoseRig" }] },
                "seed": 130,
            }),
        })
        .await
        .expect("enqueue comfy job");
    assert_eq!(job.status, ComfyJobStatus::Queued);
    store
        .mark_comfy_job_running(job.job_id)
        .await
        .expect("advance job to RUNNING");
    store
        .mark_comfy_job_completed(job.job_id)
        .await
        .expect("resolve job to COMPLETED");
    let reread_job = store
        .get_comfy_job(job.job_id)
        .await
        .expect("re-read job from PostgreSQL")
        .expect("job persisted");
    assert_eq!(reread_job.status, ComfyJobStatus::Completed);
    assert!(reread_job.started_at.is_some(), "RUNNING stamped started_at");
    assert!(
        reread_job.finished_at.is_some(),
        "COMPLETED stamped finished_at"
    );

    let job_aggregate = job.job_id.to_string();
    for family in [
        comfy_event_family::JOB_ENQUEUED,
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
    ] {
        let count = store
            .count_events_for_aggregate(family, "atelier_comfy_job", &job_aggregate)
            .await
            .expect("count job lifecycle event");
        assert_eq!(count, 1, "exactly one {family} event for this job");
    }

    // --- registration failure recovery: preserved, then recovered ---
    let artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let failure = store
        .record_comfy_output_registration_failure(&NewComfyOutputRegistrationFailure {
            workflow_run_id: run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            attempted_registration_id: None,
            source_node_instance_id: "saveimage-late-register".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: artifact_ref.clone(),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: Some(130),
            identity_metadata: None,
            failure_stage: "registration".to_string(),
            failure_reason: "capability registration unavailable after image save".to_string(),
            evidence: json!({ "case": "mt-130-event-family-proof" }),
        })
        .await
        .expect("preserve saved output whose registration failed");
    assert_eq!(failure.status, ComfyOutputRegistrationFailureStatus::Retryable);

    let adapter = ComfyBridgeFakeAdapterV1::default();
    let registration = store
        .register_bridge_capability(&adapter.capability_registration(
            run_id,
            ComfyBridgeFakeAdapterV1::CAPABILITY_PROFILE_ID,
            &format!("artifact://atelier/capability-evidence/{}", Uuid::new_v4()),
        ))
        .await
        .expect("register capability before retry");
    let retry = store
        .retry_comfy_output_registration_failure(
            failure.failure_id,
            Some(registration.registration_id),
        )
        .await
        .expect("recover the preserved output");
    assert_eq!(retry.output.artifact_ref, artifact_ref);

    // RE-READ: the failure row flipped to registered with the resolved link.
    let resolved = store
        .get_comfy_output_registration_failure(failure.failure_id)
        .await
        .expect("re-read failure row from PostgreSQL")
        .expect("failure row persisted");
    assert_eq!(
        resolved.status,
        ComfyOutputRegistrationFailureStatus::Registered
    );
    assert_eq!(
        resolved.resolved_intake_output_id,
        Some(retry.output.intake_output_id)
    );
    assert_eq!(resolved.retry_count, 1);

    let failure_aggregate = failure.failure_id.to_string();
    for family in [
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RECORDED,
        comfy_event_family::OUTPUT_REGISTRATION_FAILURE_RETRIED,
    ] {
        let count = store
            .count_events_for_aggregate(
                family,
                "atelier_comfy_output_registration_failure",
                &failure_aggregate,
            )
            .await
            .expect("count registration recovery event");
        assert_eq!(count, 1, "exactly one {family} event for this failure");
    }
}
