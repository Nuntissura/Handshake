//! WP-KERNEL-005 MT-145 / MT-144: real PostgreSQL round-trip proofs for the
//! append-only command log and heartbeat-based stale-session detection.
//!
//! These MTs are TYPED RUNTIME surfaces (Postgres rows + EventLedger events),
//! never governance markdown:
//!   * MT-145 -- atelier_command_log: an APPEND-ONLY queryable command log tied
//!     to sessions and receipts. Re-recording the same command_log_id is
//!     rejected (never upserted), so prior evidence can't be overwritten.
//!   * MT-144 -- stale-session detection: sessions whose last_heartbeat is older
//!     than the timeout are FLAGGED STALE. The key invariant is that a stale
//!     session's evidence is PRESERVED -- flagging never deletes the session row
//!     or its tied command-log evidence rows.
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite).
//!
//! NOTE: migration 0113 is not yet wired into `ensure_schema` (the orchestrator
//! wires it after this MT lands). The shared preamble therefore applies the
//! 0113 migration itself; `CREATE TABLE IF NOT EXISTS` makes this idempotent and
//! safe once the orchestrator has wired it in.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::command_corpus::{
    detect_stale_sessions, DiagnosticsSession, NewCommandLogEntry, SessionStatus,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use uuid::Uuid;

/// Connect, ensure the wired schema, then apply the (not-yet-wired) 0113
/// command-log / session migration. Idempotent.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0113_atelier_command_log_session_heartbeat.sql"
    ))
    .execute(store.pool())
    .await
    .expect("apply 0113 command-log/session migration");
    store
}

/// MT-145: the command log is append-only, tied to a session and a receipt.
/// A first record persists; re-recording the same command_log_id is REJECTED
/// (not upserted), so the original evidence row survives unchanged.
#[tokio::test]
async fn mt145_command_log_append_only_tied_to_session_and_receipt() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt145_command_log_append_only_tied_to_session_and_receipt: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

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
    assert_eq!(listed.len(), 1, "exactly the one appended entry is queryable");
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
            session_ref: "sqlite:./local.db".to_string(),
            command_id: "atelier.intake.classify".to_string(),
            status: "ok".to_string(),
            receipt_ref: None,
            evidence_ref: None,
        })
        .await
        .expect_err("legacy sqlite session_ref must be rejected");
    assert!(
        matches!(bad, AtelierError::Validation(_) | AtelierError::ForbiddenStorage(_)),
        "legacy runtime ref must be rejected, got {bad:?}"
    );
}

/// MT-144: a session whose heartbeat is older than the timeout is detected and
/// FLAGGED STALE; its evidence (command-log rows) is PRESERVED through the
/// flagging, not deleted.
#[tokio::test]
async fn mt144_stale_session_detected_and_evidence_preserved() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt144_stale_session_detected_and_evidence_preserved: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let run = Uuid::now_v7();
    let stale_ref = format!("session:{run}:stale");
    let fresh_ref = format!("session:{run}:fresh");

    // A fresh session via the heartbeat surface (stamped NOW(), so ACTIVE).
    store
        .record_session_heartbeat(&fresh_ref)
        .await
        .expect("record fresh heartbeat");

    // A session whose last heartbeat is well in the past. We seed it directly so
    // the heartbeat timestamp is deterministically old (the heartbeat API always
    // stamps NOW()).
    sqlx::query(
        r#"INSERT INTO atelier_diagnostics_session (session_ref, status, last_heartbeat_utc)
           VALUES ($1, 'ACTIVE', NOW() - INTERVAL '1 hour')"#,
    )
    .bind(&stale_ref)
    .execute(store.pool())
    .await
    .expect("seed an old-heartbeat session");

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
    // 10-minute timeout.
    let now = chrono::Utc::now();
    let all_sessions = store
        .list_diagnostics_sessions()
        .await
        .expect("list diagnostics sessions");
    let pure_stale: Vec<DiagnosticsSession> =
        detect_stale_sessions(&all_sessions, now, chrono::Duration::minutes(10));
    assert!(
        pure_stale.iter().any(|s| s.session_ref == stale_ref),
        "pure detection must flag the old-heartbeat session"
    );
    assert!(
        !pure_stale.iter().any(|s| s.session_ref == fresh_ref),
        "pure detection must NOT flag the fresh session"
    );

    // Persisted flagging at the same timeout: the old session is flipped STALE.
    let flagged = store
        .flag_stale_sessions(chrono::Duration::minutes(10))
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
