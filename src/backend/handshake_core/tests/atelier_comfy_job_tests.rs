//! WP-KERNEL-005 MT-126/MT-127/MT-128 live PostgreSQL round-trip proofs for the
//! ComfyUI job lifecycle in the `atelier::comfy` submodule.
//!
//! No mocks: each test connects the real `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the job-queue records with REAL data, and
//! asserts the load-bearing invariants:
//!   * MT-126: a job request enqueues to QUEUED (the queued receipt), round-trips
//!     by id and via list, persists the scrubbed request body, and emits
//!     `JOB_ENQUEUED`.
//!   * MT-127: poll maps each raw lifecycle status to its typed poll state, and
//!     the QUEUED->RUNNING->COMPLETED transitions advance the job.
//!   * MT-128: cancel and timeout preserve partial evidence so no evidence is
//!     lost, and cancel-of-terminal is rejected.
//! Each mutation emits the canonical Atelier EventLedger family so MT-005
//! coverage holds. The shared live database may carry rows from prior runs, so
//! every job/run id is run-scoped via `Uuid::new_v4()` and assertions are
//! per-aggregate (NO global counts).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::comfy::{
    comfy_event_family, ComfyJobPollState, ComfyJobStatus, NewComfyJobRequest,
};
use handshake_core::atelier::AtelierStore;
use serde_json::json;
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

fn new_job(run_id: Uuid) -> NewComfyJobRequest {
    NewComfyJobRequest {
        workflow_run_id: run_id,
        spec_id: None,
        request_json: json!({
            "graph": { "nodes": [{ "id": "1", "class_type": "PoseRig" }] },
            "seed": 42,
            // A credential-bearing key that MUST be scrubbed on enqueue.
            "authorization": "Bearer should-be-redacted",
        }),
    }
}

#[tokio::test]
async fn mt126_comfy_job_enqueues_with_receipt_and_round_trips() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt126_comfy_job_enqueues_with_receipt_and_round_trips: no DATABASE_URL");
        return;
    };
    let store = connected_store(&url).await;
    let run_id = Uuid::new_v4();

    // Enqueue builds + persists a QUEUED job (the queued receipt) and emits the event.
    let job = store
        .enqueue_comfy_job(&new_job(run_id))
        .await
        .expect("enqueue comfy job");
    assert_eq!(job.workflow_run_id, run_id);
    assert_eq!(job.status, ComfyJobStatus::Queued);
    assert!(job.started_at.is_none());
    assert!(job.finished_at.is_none());
    assert!(job.partial_evidence_ref.is_none());
    // Credential material in the request body was scrubbed (LAW-COMFY-INTAKE-005).
    assert_eq!(
        job.request_json["authorization"],
        json!("[REDACTED]"),
        "credential key must be scrubbed on enqueue"
    );
    assert_eq!(job.request_json["seed"], json!(42));

    // Round-trips by id.
    let fetched = store
        .get_comfy_job(job.job_id)
        .await
        .expect("get comfy job")
        .expect("job exists");
    assert_eq!(fetched, job);

    // Round-trips via the QUEUED list filter (find our run; no global count).
    let queued = store
        .list_comfy_jobs(Some(ComfyJobStatus::Queued))
        .await
        .expect("list queued jobs");
    assert!(
        queued.iter().any(|j| j.job_id == job.job_id),
        "enqueued job must appear in the QUEUED list"
    );

    // JOB_ENQUEUED event recorded for THIS job aggregate (run-scoped count).
    let enqueued_events = store
        .count_events_for_aggregate(
            comfy_event_family::JOB_ENQUEUED,
            "atelier_comfy_job",
            &job.job_id.to_string(),
        )
        .await
        .expect("count enqueue events");
    assert_eq!(enqueued_events, 1, "exactly one enqueue event for this job");
}

#[tokio::test]
async fn mt127_comfy_job_polling_maps_status_transitions() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt127_comfy_job_polling_maps_status_transitions: no DATABASE_URL");
        return;
    };
    let store = connected_store(&url).await;
    let run_id = Uuid::new_v4();

    let job = store
        .enqueue_comfy_job(&new_job(run_id))
        .await
        .expect("enqueue comfy job");

    // QUEUED -> poll maps to `queued`, non-terminal.
    let poll = store.poll_comfy_job(job.job_id).await.expect("poll queued");
    assert_eq!(poll.status, ComfyJobStatus::Queued);
    assert_eq!(poll.poll_state, ComfyJobPollState::Queued);
    assert!(!poll.terminal);

    // Advance QUEUED -> RUNNING.
    let running = store
        .mark_comfy_job_running(job.job_id)
        .await
        .expect("mark running");
    assert_eq!(running.status, ComfyJobStatus::Running);
    assert!(running.started_at.is_some());
    let poll = store.poll_comfy_job(job.job_id).await.expect("poll running");
    assert_eq!(poll.poll_state, ComfyJobPollState::Running);
    assert!(!poll.terminal);

    // Re-running a RUNNING job is rejected (transition guard).
    assert!(
        store.mark_comfy_job_running(job.job_id).await.is_err(),
        "running a non-QUEUED job must be rejected"
    );

    // Advance RUNNING -> COMPLETED; poll maps COMPLETED -> `done`, terminal.
    let completed = store
        .mark_comfy_job_completed(job.job_id)
        .await
        .expect("mark completed");
    assert_eq!(completed.status, ComfyJobStatus::Completed);
    assert!(completed.finished_at.is_some());
    let poll = store
        .poll_comfy_job(job.job_id)
        .await
        .expect("poll completed");
    assert_eq!(poll.poll_state, ComfyJobPollState::Done);
    assert!(poll.terminal);

    // The pure status->poll-state mapping is exhaustive and stable.
    assert_eq!(
        ComfyJobPollState::from_status(ComfyJobStatus::Failed),
        ComfyJobPollState::Failed
    );
    assert_eq!(
        ComfyJobPollState::from_status(ComfyJobStatus::Cancelled),
        ComfyJobPollState::Cancelled
    );
    assert_eq!(
        ComfyJobPollState::from_status(ComfyJobStatus::TimedOut),
        ComfyJobPollState::TimedOut
    );

    // Polling a missing job is a not-found error.
    assert!(
        store.poll_comfy_job(Uuid::new_v4()).await.is_err(),
        "polling a missing job must error"
    );

    // Run-scoped event proofs (this job aggregate only).
    let agg = job.job_id.to_string();
    for family in [
        comfy_event_family::JOB_RUNNING,
        comfy_event_family::JOB_COMPLETED,
    ] {
        let count = store
            .count_events_for_aggregate(family, "atelier_comfy_job", &agg)
            .await
            .expect("count transition event");
        assert_eq!(count, 1, "exactly one {family} event for this job");
    }
}

#[tokio::test]
async fn mt128_cancel_and_timeout_preserve_partial_evidence() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt128_cancel_and_timeout_preserve_partial_evidence: no DATABASE_URL");
        return;
    };
    let store = connected_store(&url).await;

    // --- Cancel from RUNNING preserves partial evidence. ---
    let cancel_run = Uuid::new_v4();
    let cancel_job = store
        .enqueue_comfy_job(&new_job(cancel_run))
        .await
        .expect("enqueue cancel job");
    store
        .mark_comfy_job_running(cancel_job.job_id)
        .await
        .expect("mark running");

    let partial_ref = format!("artifact://atelier/comfy/partial/{}", Uuid::new_v4());
    let cancelled = store
        .cancel_comfy_job(
            cancel_job.job_id,
            Some("operator cancelled mid-run"),
            Some(&partial_ref),
        )
        .await
        .expect("cancel running job");
    assert_eq!(cancelled.status, ComfyJobStatus::Cancelled);
    assert!(cancelled.finished_at.is_some());
    assert_eq!(
        cancelled.partial_evidence_ref.as_deref(),
        Some(partial_ref.as_str()),
        "partial evidence must be preserved on cancel"
    );
    assert_eq!(
        cancelled.error_reason.as_deref(),
        Some("operator cancelled mid-run")
    );

    // Reject cancel-of-terminal: cancelling an already-cancelled job errors.
    let err = store
        .cancel_comfy_job(cancel_job.job_id, Some("again"), None)
        .await
        .expect_err("cancel of terminal job must be rejected");
    assert!(
        err.to_string().contains("terminal"),
        "unexpected cancel-of-terminal error: {err}"
    );

    // A cancel-preserved partial-evidence event was recorded for this job.
    let preserved = store
        .count_events_for_aggregate(
            comfy_event_family::JOB_PARTIAL_EVIDENCE_PRESERVED,
            "atelier_comfy_job",
            &cancel_job.job_id.to_string(),
        )
        .await
        .expect("count preserved events");
    assert_eq!(preserved, 1, "one partial-evidence event on cancel");
    let cancel_events = store
        .count_events_for_aggregate(
            comfy_event_family::JOB_CANCELLED,
            "atelier_comfy_job",
            &cancel_job.job_id.to_string(),
        )
        .await
        .expect("count cancel events");
    assert_eq!(cancel_events, 1, "one cancel event");

    // --- Timeout from QUEUED preserves partial evidence. ---
    let timeout_run = Uuid::new_v4();
    let timeout_job = store
        .enqueue_comfy_job(&new_job(timeout_run))
        .await
        .expect("enqueue timeout job");
    let timeout_partial = format!("artifact://atelier/comfy/partial/{}", Uuid::new_v4());
    let timed_out = store
        .timeout_comfy_job(
            timeout_job.job_id,
            chrono::Utc::now(),
            Some(&timeout_partial),
        )
        .await
        .expect("timeout job");
    assert_eq!(timed_out.status, ComfyJobStatus::TimedOut);
    assert_eq!(
        timed_out.partial_evidence_ref.as_deref(),
        Some(timeout_partial.as_str()),
        "partial evidence must be preserved on timeout"
    );
    assert!(timed_out.error_reason.is_some());

    // Reject timeout-of-terminal.
    assert!(
        store
            .timeout_comfy_job(timeout_job.job_id, chrono::Utc::now(), None)
            .await
            .is_err(),
        "timeout of terminal job must be rejected"
    );

    // A legacy/.GOV/machine-local partial-evidence ref is rejected.
    let bad_run = Uuid::new_v4();
    let bad_job = store
        .enqueue_comfy_job(&new_job(bad_run))
        .await
        .expect("enqueue bad-evidence job");
    let err = store
        .cancel_comfy_job(
            bad_job.job_id,
            Some("bad ref"),
            Some("artifact://atelier/.GOV/leak.png"),
        )
        .await
        .expect_err("legacy partial-evidence ref must be rejected");
    assert!(
        err.to_string().contains("Handshake-native portable ref"),
        "unexpected bad-ref error: {err}"
    );
    // The rejected job is still QUEUED (no state change on validation failure).
    let still_queued = store
        .get_comfy_job(bad_job.job_id)
        .await
        .expect("get bad job")
        .expect("job exists");
    assert_eq!(still_queued.status, ComfyJobStatus::Queued);
}
