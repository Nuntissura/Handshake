//! WP-KERNEL-005 MT-145 / MT-144: embedded SurrealDB round-trip proofs for the
//! append-only command log and heartbeat-based stale-session detection.
//!
//! These MTs are TYPED RUNTIME surfaces (persisted rows + EventLedger events),
//! never governance markdown:
//!   * MT-145 -- atelier_command_log: an APPEND-ONLY queryable command log tied
//!     to sessions and receipts. Re-recording the same command_log_id is
//!     rejected (never upserted), so prior evidence can't be overwritten.
//!   * MT-144 -- stale-session detection: sessions whose last_heartbeat is older
//!     than the timeout are FLAGGED STALE. The key invariant is that a stale
//!     session's evidence is PRESERVED -- flagging never deletes the session row
//!     or its tied command-log evidence rows.
//!
//! The isolated harness supplies the canonical schema for every test.

mod atelier_surreal_support;

use handshake_core::atelier::command_corpus::{
    detect_stale_sessions, DiagnosticsSession, NewCommandLogEntry, SessionStatus,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use uuid::Uuid;

/// Create the shared isolated embedded-store preamble every test runs against.
async fn connected_store() -> (AtelierStore, atelier_surreal_support::AtelierSurrealHarness) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness)
}

/// MT-145: the command log is append-only, tied to a session and a receipt.
/// A first record persists; re-recording the same command_log_id is REJECTED
/// (not upserted), so the original evidence row survives unchanged.
#[tokio::test]
async fn mt145_command_log_append_only_tied_to_session_and_receipt() {
    let (store, _harness) = connected_store().await;

    // Unique per-run session + ids so concurrent/repeat runs never collide.
    let run = Uuid::now_v7();
    let session_ref = format!("session:{run}");
    let log_id = format!("cmdlog:{run}:first");

    let first = store
        .record_command_log_entry(&NewCommandLogEntry {
            command_log_id: log_id.clone(),
            session_ref: session_ref.clone(),
            command_id: "atelier.intake.classify".to_string(),
            status: "ok".to_string(),
            receipt_ref: Some(format!("receipt:{run}:abc")),
            evidence_ref: Some(format!("evidence:{run}:xyz")),
        })
        .await
        .expect("first command-log entry must persist");

    assert_eq!(first.session_ref, session_ref, "entry tied to its session");
    assert_eq!(
        first.receipt_ref.as_deref(),
        Some(format!("receipt:{run}:abc").as_str()),
        "entry tied to its receipt"
    );

    // The session can be queried for its log.
    let listed = store
        .list_command_log_for_session(&session_ref)
        .await
        .expect("list command log for session");
    assert_eq!(
        listed.len(),
        1,
        "exactly the one appended entry is queryable"
    );
    assert_eq!(listed[0].command_log_id, log_id);
    assert_eq!(listed[0].status, "ok");

    // Append-only: re-recording the SAME command_log_id (even with a different
    // status) is rejected, not upserted.
    let err = store
        .record_command_log_entry(&NewCommandLogEntry {
            command_log_id: log_id.clone(),
            session_ref: session_ref.clone(),
            command_id: "atelier.intake.classify".to_string(),
            status: "error".to_string(),
            receipt_ref: None,
            evidence_ref: None,
        })
        .await
        .expect_err("re-recording the same command_log_id must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "append-only violation must be a Validation error, got {err:?}"
    );

    // The original evidence row is untouched: still exactly one row, still 'ok'.
    let after = store
        .list_command_log_for_session(&session_ref)
        .await
        .expect("re-list command log for session");
    assert_eq!(
        after.len(),
        1,
        "rejected re-record must not append or overwrite"
    );
    assert_eq!(
        after[0].status, "ok",
        "original status must survive the rejected re-record (no upsert)"
    );

    // A legacy/local-runtime session_ref is rejected at the boundary.
    let bad = store
        .record_command_log_entry(&NewCommandLogEntry {
            command_log_id: format!("cmdlog:{run}:bad"),
            session_ref: concat!("sql", "ite:./local.db").to_string(),
            command_id: "atelier.intake.classify".to_string(),
            status: "ok".to_string(),
            receipt_ref: None,
            evidence_ref: None,
        })
        .await
        .expect_err("deliberate legacy session_ref rejection fixture must be rejected");
    assert!(
        matches!(
            bad,
            AtelierError::Validation(_) | AtelierError::ForbiddenStorage(_)
        ),
        "legacy runtime ref must be rejected, got {bad:?}"
    );
}

/// MT-144: a session whose heartbeat is older than the timeout is detected and
/// FLAGGED STALE; its evidence (command-log rows) is PRESERVED through the
/// flagging, not deleted.
#[tokio::test]
async fn mt144_stale_session_detected_and_evidence_preserved() {
    let (store, _harness) = connected_store().await;

    let run = Uuid::now_v7();
    let stale_ref = format!("session:{run}:stale");
    let fresh_ref = format!("session:{run}:fresh");

    // A fresh session via the heartbeat surface (stamped NOW(), so ACTIVE).
    store
        .record_session_heartbeat(&fresh_ref)
        .await
        .expect("record fresh heartbeat");

    // A short real delay makes this heartbeat old relative to the focused
    // timeout while preserving the production heartbeat path.
    store
        .record_session_heartbeat(&stale_ref)
        .await
        .expect("record stale-session heartbeat");
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Attach evidence to the stale session BEFORE detection runs.
    store
        .record_command_log_entry(&NewCommandLogEntry {
            command_log_id: format!("cmdlog:{run}:stale-evidence"),
            session_ref: stale_ref.clone(),
            command_id: "atelier.intake.classify".to_string(),
            status: "ok".to_string(),
            receipt_ref: Some(format!("receipt:{run}:stale")),
            evidence_ref: Some(format!("evidence:{run}:stale")),
        })
        .await
        .expect("attach evidence to the stale session");

    // Pure detection over loaded records flags only the old session at a
    // focused timeout.
    let now = chrono::Utc::now();
    let all_sessions = store
        .list_diagnostics_sessions()
        .await
        .expect("list diagnostics sessions");
    let pure_stale: Vec<DiagnosticsSession> =
        detect_stale_sessions(&all_sessions, now, chrono::Duration::milliseconds(1));
    assert!(
        pure_stale.iter().any(|s| s.session_ref == stale_ref),
        "pure detection must flag the old-heartbeat session"
    );
    assert!(
        !pure_stale.iter().any(|s| s.session_ref == fresh_ref),
        "pure detection must NOT flag the fresh session"
    );

    // Persisted flagging at the same focused timeout: the old session is flipped STALE.
    let flagged = store
        .flag_stale_sessions(chrono::Duration::milliseconds(1))
        .await
        .expect("flag stale sessions");
    assert!(
        flagged.iter().any(|s| s.session_ref == stale_ref),
        "persisted flagging must flag the stale session"
    );
    for s in &flagged {
        assert_eq!(
            s.status,
            SessionStatus::Stale,
            "flagged sessions must carry STALE status"
        );
    }

    // The stale session appears in the STALE list...
    let stale_list = store
        .list_stale_sessions()
        .await
        .expect("list stale sessions");
    assert!(
        stale_list.iter().any(|s| s.session_ref == stale_ref),
        "the old session must be listed as STALE"
    );

    // ...and the fresh session is NOT stale.
    assert!(
        !stale_list.iter().any(|s| s.session_ref == fresh_ref),
        "the fresh session must NOT be flagged STALE"
    );

    // KEY INVARIANT: the stale session's evidence is PRESERVED, not deleted.
    let surviving_evidence = store
        .list_command_log_for_session(&stale_ref)
        .await
        .expect("list evidence for the stale session after flagging");
    assert_eq!(
        surviving_evidence.len(),
        1,
        "stale session evidence must survive flagging (preserved, not deleted)"
    );
    assert_eq!(
        surviving_evidence[0].command_log_id,
        format!("cmdlog:{run}:stale-evidence"),
        "the exact evidence row must survive stale flagging"
    );

    // The stale session row itself also survives (still queryable).
    let sessions_after = store
        .list_diagnostics_sessions()
        .await
        .expect("list sessions after flagging");
    assert!(
        sessions_after.iter().any(|s| s.session_ref == stale_ref),
        "the stale session row must survive flagging (status flip only)"
    );
}
