//! WP-KERNEL-005 MT-142: durable ModelSession record proofs.
//!
//! The contract requires durable model sessions carrying id, agent, purpose,
//! metadata, timestamps, state, actor and close reason that SURVIVE RESTART.
//! These tests prove exactly that against real PostgreSQL:
//!   * a session created with agent + purpose is re-read through a FRESH
//!     connection pool (simulated process restart) with every identity field
//!     intact;
//!   * `close_model_session` records the close metadata (terminal state, close
//!     reason, closing actor, closed-at) and that metadata survives another
//!     restart;
//!   * non-terminal close states, empty close reasons/actors and unknown
//!     sessions are rejected.
//!
//! Gated on `atelier_pg_support::database_url()` (Handshake-managed
//! PostgreSQL; never SQLite).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::storage::{
    postgres::PostgresDatabase, Database, ModelSessionState, NewModelSession, StorageError,
};
use uuid::Uuid;

fn new_session(session_id: &str) -> NewModelSession {
    NewModelSession {
        session_id: session_id.to_string(),
        parent_session_id: None,
        spawn_depth: 0,
        state: ModelSessionState::Active,
        model_id: "claude-fable-5".to_string(),
        backend: "acp-broker".to_string(),
        parameter_class: "standard".to_string(),
        role: "CODER".to_string(),
        wp_id: Some("WP-KERNEL-005".to_string()),
        mt_id: Some("MT-142".to_string()),
        work_profile_id: None,
        execution_mode: "STANDARD".to_string(),
        memory_policy: "SESSION_SCOPED".to_string(),
        consent_receipt_id: None,
        capability_grants: vec!["fs.read".to_string()],
        capability_token_ids: None,
        job_id: None,
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        agent: Some("CODER:claude-fable-5".to_string()),
        purpose: Some("MT-142 durable model-session restart proof".to_string()),
    }
}

/// MT-142: identity (agent/purpose) and close metadata (close reason, actor,
/// closed-at) survive a simulated restart (fresh PostgreSQL pool per phase).
#[tokio::test]
async fn mt142_model_session_survives_restart_with_close_metadata() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt142_model_session_survives_restart_with_close_metadata: PostgreSQL unavailable"
        );
        return;
    };

    let session_id = format!("mt142-session-{}", Uuid::new_v4());

    // Phase 1: create the durable session, then drop the pool entirely.
    {
        let db = PostgresDatabase::connect(&url, 2)
            .await
            .expect("connect phase-1 pool");
        let created = db
            .upsert_model_session(new_session(&session_id))
            .await
            .expect("create durable model session");
        assert_eq!(created.session_id, session_id);
        assert_eq!(created.agent.as_deref(), Some("CODER:claude-fable-5"));
        assert_eq!(
            created.purpose.as_deref(),
            Some("MT-142 durable model-session restart proof")
        );
        assert_eq!(created.state, ModelSessionState::Active);
        assert_eq!(created.close_reason, None);
        assert_eq!(created.closed_by_actor, None);
        assert_eq!(created.closed_at, None);
    }

    // Phase 2 (simulated restart): a FRESH pool re-reads the full identity,
    // then closes the session with close metadata.
    let closed_at;
    {
        let db = PostgresDatabase::connect(&url, 2)
            .await
            .expect("connect phase-2 pool (restart)");
        let reloaded = db
            .get_model_session(&session_id)
            .await
            .expect("session must survive the restart");
        assert_eq!(reloaded.agent.as_deref(), Some("CODER:claude-fable-5"));
        assert_eq!(
            reloaded.purpose.as_deref(),
            Some("MT-142 durable model-session restart proof")
        );
        assert_eq!(reloaded.state, ModelSessionState::Active);
        assert_eq!(reloaded.wp_id.as_deref(), Some("WP-KERNEL-005"));
        assert_eq!(reloaded.mt_id.as_deref(), Some("MT-142"));

        let closed = db
            .close_model_session(
                &session_id,
                ModelSessionState::Completed,
                "mt-142 restart-survival proof complete",
                "KB-KERNEL-005-CLOSEOUT",
            )
            .await
            .expect("close the durable session");
        assert_eq!(closed.state, ModelSessionState::Completed);
        assert_eq!(
            closed.close_reason.as_deref(),
            Some("mt-142 restart-survival proof complete")
        );
        assert_eq!(
            closed.closed_by_actor.as_deref(),
            Some("KB-KERNEL-005-CLOSEOUT")
        );
        closed_at = closed.closed_at.expect("closed_at must be recorded");
        assert!(closed.closed_at >= Some(closed.created_at));
    }

    // Phase 3 (second restart): the close metadata is durable.
    {
        let db = PostgresDatabase::connect(&url, 2)
            .await
            .expect("connect phase-3 pool (second restart)");
        let after_restart = db
            .get_model_session(&session_id)
            .await
            .expect("closed session must survive the restart");
        assert_eq!(after_restart.state, ModelSessionState::Completed);
        assert_eq!(
            after_restart.close_reason.as_deref(),
            Some("mt-142 restart-survival proof complete")
        );
        assert_eq!(
            after_restart.closed_by_actor.as_deref(),
            Some("KB-KERNEL-005-CLOSEOUT")
        );
        assert_eq!(after_restart.closed_at, Some(closed_at));
        assert_eq!(after_restart.agent.as_deref(), Some("CODER:claude-fable-5"));
        assert_eq!(
            after_restart.purpose.as_deref(),
            Some("MT-142 durable model-session restart proof")
        );
        assert!(after_restart.updated_at >= after_restart.created_at);
    }
}

/// MT-142: close metadata is guarded -- non-terminal close states, empty close
/// reasons/actors and unknown sessions are rejected.
#[tokio::test]
async fn mt142_close_model_session_rejects_invalid_close_requests() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt142_close_model_session_rejects_invalid_close_requests: PostgreSQL unavailable"
        );
        return;
    };
    let db = PostgresDatabase::connect(&url, 2)
        .await
        .expect("connect to PostgreSQL");

    let session_id = format!("mt142-guard-{}", Uuid::new_v4());
    db.upsert_model_session(new_session(&session_id))
        .await
        .expect("create durable model session");

    // (a) non-terminal close state -> Validation.
    let non_terminal = db
        .close_model_session(&session_id, ModelSessionState::Paused, "still working", "op")
        .await;
    assert!(
        matches!(non_terminal, Err(StorageError::Validation(_))),
        "a non-terminal close state must be rejected, got {non_terminal:?}"
    );

    // (b) empty close reason -> Validation.
    let empty_reason = db
        .close_model_session(&session_id, ModelSessionState::Cancelled, "   ", "op")
        .await;
    assert!(
        matches!(empty_reason, Err(StorageError::Validation(_))),
        "an empty close_reason must be rejected, got {empty_reason:?}"
    );

    // (c) empty actor -> Validation.
    let empty_actor = db
        .close_model_session(&session_id, ModelSessionState::Cancelled, "operator stop", "")
        .await;
    assert!(
        matches!(empty_actor, Err(StorageError::Validation(_))),
        "an empty actor must be rejected, got {empty_actor:?}"
    );

    // (d) unknown session -> NotFound.
    let unknown = db
        .close_model_session(
            &format!("mt142-missing-{}", Uuid::new_v4()),
            ModelSessionState::Failed,
            "never existed",
            "op",
        )
        .await;
    assert!(
        matches!(unknown, Err(StorageError::NotFound(_))),
        "closing an unknown session must be NotFound, got {unknown:?}"
    );

    // The guarded session is untouched: still Active, no close metadata.
    let untouched = db
        .get_model_session(&session_id)
        .await
        .expect("guarded session still readable");
    assert_eq!(untouched.state, ModelSessionState::Active);
    assert_eq!(untouched.close_reason, None);
    assert_eq!(untouched.closed_by_actor, None);
    assert_eq!(untouched.closed_at, None);
}
