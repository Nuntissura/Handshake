//! WP-KERNEL-005 MT-156: STEER feedback from visual mismatch -- real
//! PostgreSQL + EventLedger proofs.
//!
//! A visual threshold breach in a validated `VisualDebuggingLoopV1` must be
//! converted into an actionable, durable STEER feedback record (table
//! `atelier_visual_steer_feedback`, migration 0129) -- never a silent failure
//! and never generic prose. Each record emits a
//! `VISUAL_STEER_FEEDBACK_RECORDED` EventLedger event in the same transaction.
//!
//! Gated on `atelier_pg_support::database_url()` (Handshake-managed
//! PostgreSQL; never SQLite).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::visual_steer_feedback::visual_steer_event_family;
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::kernel::visual_debugging_loop::{
    ValidatorSteeringV1, VisualComparisonMode, VisualDebugEvidenceArtifactV1,
    VisualDebuggingLoopV1, VisualDebuggingThresholdConfigV1, VisualDebuggingTriggerKind,
    VisualDebuggingTriggerV1, FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID,
};
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// MT-156: a threshold breach becomes a durable, actionable STEER feedback
/// record (PG row + EventLedger event), idempotent on (loop_id, evidence_id).
#[tokio::test]
async fn mt156_threshold_breach_records_actionable_steer_feedback() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt156_threshold_breach_records_actionable_steer_feedback: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let loop_id = format!("visual-loop-{}", Uuid::new_v4());
    // mismatch 300 bps > threshold 250 bps -> exactly one breach.
    let loop_config = sample_loop(&loop_id, 300);

    let recorded = store
        .record_visual_steer_feedback(&loop_config)
        .await
        .expect("record steer feedback for a threshold breach");
    assert_eq!(recorded.len(), 1, "one breach must yield one record");
    let record = &recorded[0];
    assert_eq!(record.loop_id, loop_id);
    assert_eq!(record.evidence_id, "visual-evidence-1");
    assert_eq!(record.wp_id, "WP-GUI");
    assert_eq!(record.mismatch_basis_points, 300);
    assert_eq!(record.threshold_basis_points, 250);
    assert_eq!(record.target_role, "VALIDATOR");
    assert_eq!(record.receipt_kind, "STEER");
    assert_eq!(record.code_diff_ref, "git://diff/abc123");
    assert_eq!(record.visual_diff_ref, "artifact://visual-diffs/diff-1.png");
    // Actionable, not generic prose: the next action names the breach numbers
    // and the concrete refs to act on.
    assert!(record.next_action.contains("300 bps"));
    assert!(record.next_action.contains("250 bps"));
    assert!(record.next_action.contains("git://diff/abc123"));
    assert!(record
        .next_action
        .contains("artifact://visual-diffs/diff-1.png"));

    // Re-read from PostgreSQL via the list path.
    let reloaded = store
        .list_visual_steer_feedback_for_loop(&loop_id)
        .await
        .expect("list steer feedback for the loop");
    assert_eq!(reloaded.len(), 1, "the breach row must be listed");
    assert_eq!(&reloaded[0], record, "round-trip must preserve every field");

    // The write emitted its EventLedger event in the same transaction.
    let events = store
        .count_events_for_aggregate(
            visual_steer_event_family::VISUAL_STEER_FEEDBACK_RECORDED,
            "atelier_visual_steer_feedback",
            &record.feedback_id,
        )
        .await
        .expect("count VISUAL_STEER_FEEDBACK_RECORDED events");
    assert_eq!(
        events, 1,
        "recording steer feedback must emit exactly one EventLedger event"
    );

    // Idempotent on (loop_id, evidence_id): re-recording the same breach
    // updates the single row (no duplicate) and emits a second event.
    let rerecorded = store
        .record_visual_steer_feedback(&loop_config)
        .await
        .expect("re-record steer feedback");
    assert_eq!(rerecorded.len(), 1);
    let relisted = store
        .list_visual_steer_feedback_for_loop(&loop_id)
        .await
        .expect("re-list steer feedback");
    assert_eq!(relisted.len(), 1, "re-recording must not duplicate the row");
    let events_after_rerecord = store
        .count_events_for_aggregate(
            visual_steer_event_family::VISUAL_STEER_FEEDBACK_RECORDED,
            "atelier_visual_steer_feedback",
            &record.feedback_id,
        )
        .await
        .expect("count events after re-record");
    assert_eq!(
        events_after_rerecord, 2,
        "re-recording must emit a second EventLedger event"
    );
}

/// MT-156: a loop within threshold records nothing, and an invalid loop is
/// rejected (no silent failure) without persisting anything.
#[tokio::test]
async fn mt156_no_breach_records_nothing_and_invalid_loop_is_rejected() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt156_no_breach_records_nothing_and_invalid_loop_is_rejected: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // Within threshold (200 <= 250): nothing recorded.
    let quiet_loop_id = format!("visual-loop-quiet-{}", Uuid::new_v4());
    let quiet = store
        .record_visual_steer_feedback(&sample_loop(&quiet_loop_id, 200))
        .await
        .expect("a within-threshold loop is accepted");
    assert!(quiet.is_empty(), "no breach must record no feedback");
    let listed = store
        .list_visual_steer_feedback_for_loop(&quiet_loop_id)
        .await
        .expect("list feedback for the quiet loop");
    assert!(listed.is_empty(), "no rows may exist for a quiet loop");

    // Invalid loop (steering disabled while a breach exists): rejected as
    // Validation, and nothing is persisted for that loop.
    let invalid_loop_id = format!("visual-loop-invalid-{}", Uuid::new_v4());
    let mut invalid = sample_loop(&invalid_loop_id, 300);
    invalid.validator_steering.threshold_exceeded_sends_steer = false;
    let err = store
        .record_visual_steer_feedback(&invalid)
        .await
        .expect_err("an invalid loop must be rejected, not silently skipped");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "invalid loop must be a Validation error, got {err:?}"
    );
    let invalid_rows = store
        .list_visual_steer_feedback_for_loop(&invalid_loop_id)
        .await
        .expect("list feedback for the rejected loop");
    assert!(
        invalid_rows.is_empty(),
        "a rejected loop must not persist any feedback"
    );
}

// --- helpers -----------------------------------------------------------------

/// A fully valid MT-046 visual debugging loop with one evidence artifact at
/// the given mismatch level (threshold fixed at 250 bps).
fn sample_loop(loop_id: &str, mismatch_basis_points: u32) -> VisualDebuggingLoopV1 {
    VisualDebuggingLoopV1 {
        schema_id: "hsk.kernel.visual_debugging_loop@1".to_string(),
        loop_id: loop_id.to_string(),
        folded_stub_ids: vec![FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID.to_string()],
        gui_bearing_wp_id: "WP-GUI".to_string(),
        triggers: vec![
            trigger(
                "trigger.post_commit",
                VisualDebuggingTriggerKind::PostCommit,
            ),
            trigger(
                "trigger.post_action",
                VisualDebuggingTriggerKind::PostAction,
            ),
        ],
        threshold_config: VisualDebuggingThresholdConfigV1 {
            threshold_config_ref: "packet://WP-GUI/visual-thresholds".to_string(),
            max_pixel_diff_basis_points: 250,
            max_layout_shift_basis_points: 100,
            structural_mismatch_limit: 0,
        },
        evidence_artifacts: vec![VisualDebugEvidenceArtifactV1 {
            evidence_id: "visual-evidence-1".to_string(),
            wp_id: "WP-GUI".to_string(),
            commit_ref: "git://commit/abc123".to_string(),
            screenshot_ref: "artifact://screenshots/screen-1.png".to_string(),
            baseline_ref: "artifact://baselines/screen-1.png".to_string(),
            visual_diff_artifact_ref: "artifact://visual-diffs/diff-1.png".to_string(),
            comparison_mode: VisualComparisonMode::PixelDiff,
            mismatch_basis_points,
            stored_in_artifact_system: true,
        }],
        validator_steering: ValidatorSteeringV1 {
            enabled: true,
            target_role: "VALIDATOR".to_string(),
            receipt_kind: "STEER".to_string(),
            code_diff_ref: "git://diff/abc123".to_string(),
            visual_diff_ref: "artifact://visual-diffs/diff-1.png".to_string(),
            visual_evidence_required: true,
            threshold_exceeded_sends_steer: true,
        },
        product_authority_refs: vec![
            "kernel.product_screenshot_capture".to_string(),
            "kernel.action_catalog".to_string(),
            "artifact_store.visual_evidence".to_string(),
            "validator.steering".to_string(),
        ],
        folded_source_refs: vec![format!(
            ".GOV/task_packets/stubs/{FOLDED_VISUAL_DEBUGGING_LOOP_STUB_ID}.contract.json"
        )],
    }
}

fn trigger(trigger_id: &str, kind: VisualDebuggingTriggerKind) -> VisualDebuggingTriggerV1 {
    VisualDebuggingTriggerV1 {
        trigger_id: trigger_id.to_string(),
        kind,
        screenshot_request_ref: format!("screenshot-request://{trigger_id}"),
        baseline_ref: "artifact://baselines/screen-1.png".to_string(),
        capture_after_ref: format!("event://{trigger_id}/after"),
    }
}
