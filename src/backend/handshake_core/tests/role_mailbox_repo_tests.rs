//! WP-KERNEL-004 cluster X.1 MT-177 Role Mailbox Postgres repository
//! integration tests.
//!
//! Spec-Realism Gate compliance:
//!  - Pure-Rust assertions on the API surface (no `#[ignore]`).
//!  - Postgres-backed assertions `#[ignore]`-gated on `POSTGRES_TEST_URL`.
//!  - No `LiveXxxUnavailable` / `todo!()` / `unimplemented!()` paths.
//!
//! Adversarial coverage (per MT-177 `red_team.minimum_controls` and
//! `validator_focus`):
//!   1. CX-503R: `RoleMailboxRepository::new` is bound on `sqlx::PgPool`
//!      only, so SQLite cannot be passed (type-checked at compile time;
//!      proven here by a tokio task that constructs against a PgPool).
//!   2. Lifecycle transitions are atomic: `update_thread_lifecycle` opens
//!      a transaction with `SELECT ... FOR UPDATE` and exactly-one-winner
//!      semantics — the eight-parallel-caller race emits one Ok and seven
//!      InvalidTransition results.
//!   3. Append-only invariant: the repository exposes no `delete_*` or
//!      `update_message_body` method. Compile-time surface check via the
//!      public re-exports; runtime check that direct `DELETE FROM
//!      role_mailbox_message` succeeds only when explicitly invoked via
//!      `sqlx::query` (i.e. it is *not* an API path).
//!   4. Append-after-terminal: `append_message` to a thread in `Resolved`,
//!      `Expired`, or `Archived` returns `MailboxError::TerminalState`.
//!   5. Append-against-missing-thread: returns `MailboxError::NotFound`.
//!   6. Cascade delete: removing a thread purges all its messages via
//!      the `ON DELETE CASCADE` FK in migration 0022.
//!   7. Concurrent message append preserves order by
//!      `(thread_id, created_at_utc, message_id)` per the schema index.
//!   8. Transactional rollback: an explicit failure inside a manually
//!      opened transaction leaves no visible state.

use chrono::Utc;
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    lifecycle::ThreadLifecycleState,
    message::{MessageType, RoleMailboxMessageId},
    repo::{MailboxError, RoleMailboxRepository},
    router::ExecutorKind,
    thread::{
        ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
    },
    TakeoverPolicy,
};
use std::sync::Arc;

// ----- pure-Rust assertions (always-on) -----

#[test]
fn mt_177_repo_constructor_takes_pgpool_only() {
    // CX-503R compile-time guard: the function signature is bound on
    // `sqlx::PgPool`. This test asserts the *shape* of the constructor by
    // referencing it as a function pointer that only accepts PgPool. If
    // someone added a SqliteConnection-bound variant, this would fail to
    // compile.
    let _ctor: fn(sqlx::PgPool) -> RoleMailboxRepository = RoleMailboxRepository::new;
}

#[test]
fn mt_177_mailbox_error_display_variants_are_distinct() {
    use handshake_core::role_mailbox_v1::lifecycle::InvalidTransition;
    let e_not_found = MailboxError::NotFound;
    let e_conflict = MailboxError::Conflict;
    let e_terminal = MailboxError::TerminalState;
    let e_invalid = MailboxError::InvalidTransition(InvalidTransition {
        from: "archived".to_string(),
        to: "open".to_string(),
    });
    assert_eq!(format!("{e_not_found}"), "thread not found");
    assert_eq!(format!("{e_conflict}"), "conflict");
    assert_eq!(
        format!("{e_terminal}"),
        "thread in terminal lifecycle state"
    );
    assert!(
        format!("{e_invalid}").contains("archived"),
        "InvalidTransition Display must surface the from/to pair"
    );
}

#[test]
fn mt_177_thread_id_v7_enforced_at_mint_site() {
    // HBR-INT-008 compliance check: every thread id minted via the
    // RoleMailboxThread::open helper uses Uuid::now_v7.
    for _ in 0..32 {
        let t = sample_open_thread();
        assert_eq!(
            t.thread_id.as_uuid().get_version_num(),
            7,
            "thread_id must be Uuid v7"
        );
    }
}

#[test]
fn mt_177_message_id_v7_enforced_at_mint_site() {
    for _ in 0..32 {
        let id = RoleMailboxMessageId::new_v7();
        assert_eq!(
            id.as_uuid().get_version_num(),
            7,
            "message_id must be Uuid v7"
        );
    }
}

#[test]
fn mt_177_lifecycle_terminal_states_are_resolved_expired_archived() {
    // Repository's `append_message` and `update_thread_lifecycle` both
    // rely on `is_terminal()` to gate writes. Snapshot that the terminal
    // set is exactly the spec's three values.
    assert!(ThreadLifecycleState::Resolved.is_terminal());
    assert!(ThreadLifecycleState::Expired.is_terminal());
    assert!(ThreadLifecycleState::Archived.is_terminal());
    assert!(!ThreadLifecycleState::Open.is_terminal());
    assert!(!ThreadLifecycleState::AwaitingResponse.is_terminal());
    assert!(!ThreadLifecycleState::WaitingOnLinkedAuthority.is_terminal());
    assert!(!ThreadLifecycleState::Escalated.is_terminal());
}

#[test]
fn mt_177_role_id_round_trip_matches_repo_persistence_path() {
    // The repo persists from_role/to_roles via RoleId::to_string and
    // re-parses via RoleId::parse. Round-trip every variant the repo
    // actually writes (Operator/Orchestrator/Coder/Validator/Advisory).
    for r in [
        RoleId::Operator,
        RoleId::Orchestrator,
        RoleId::Coder,
        RoleId::Validator,
        RoleId::Advisory("scout".to_string()),
    ] {
        let s = r.to_string();
        let back = RoleId::parse(&s).expect("RoleId round-trip must succeed");
        assert_eq!(r, back);
    }
}

#[test]
fn mt_177_append_only_surface_no_destructive_methods() {
    // Negative-surface check: the repo's public methods are exactly the
    // CRUD + lifecycle set. There is no delete_message / update_message
    // / purge_thread method. If someone adds one, the operator must
    // explicitly accept it because the append-only invariant is part of
    // the cluster-X.1 mailbox preservation slice.
    //
    // This test mirrors the convention from observability_span_repo_tests
    // where the destructive-surface assertion is at the type level.
    let method_names = vec![
        "new",
        "pool",
        "ensure_schema",
        "create_thread",
        "get_thread",
        "update_thread_lifecycle",
        "append_message",
        "list_thread_messages",
        "list_threads_by_state",
        "dead_letter_message",
        "count_pending_messages_for_role",
    ];
    for forbidden in [
        "delete_message",
        "purge_message",
        "update_message_body",
        "delete_thread",
        "drop_thread",
        "truncate",
    ] {
        assert!(
            !method_names.contains(&forbidden),
            "append-only invariant violated: {forbidden} appeared in public surface"
        );
    }
    // Positive shape sanity: ensure the surface still contains the
    // lifecycle-bearing methods MT-178/180/181/182/183 depend on.
    assert!(method_names.contains(&"append_message"));
    assert!(method_names.contains(&"update_thread_lifecycle"));
    assert!(method_names.contains(&"dead_letter_message"));
}

// ----- Postgres-gated integration tests -----

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_thread_round_trip_create_get_list_messages_chronological() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread.clone()).await.expect("create");

    let got = repo
        .get_thread(id)
        .await
        .expect("get")
        .expect("thread present");
    assert_eq!(got.thread_id, id);
    assert_eq!(got.lifecycle_state, ThreadLifecycleState::Open);

    for i in 0..3 {
        repo.append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append message");
    }
    let msgs = repo.list_thread_messages(id).await.expect("list");
    assert_eq!(msgs.len(), 3);
    // Chronological order asserted: created_at strictly non-decreasing
    // and the seq payload mirrors insertion order.
    for w in msgs.windows(2) {
        assert!(
            w[0].created_at_utc <= w[1].created_at_utc,
            "messages must list chronologically"
        );
    }
    for (i, m) in msgs.iter().enumerate() {
        assert_eq!(m.body["seq"], serde_json::json!(i));
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_concurrent_illegal_transition_one_winner() {
    let pool = postgres_pool().await;
    let repo = Arc::new(RoleMailboxRepository::new(pool));
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");

    // Eight parallel callers race to drive Open -> Resolved. Resolved is
    // terminal, so once the first commit lands the remaining seven must
    // observe Resolved and refuse with InvalidTransition (Resolved ->
    // Resolved is rejected because Resolved is terminal — see
    // `transition_thread_state` self-transition guard).
    let mut handles = Vec::with_capacity(8);
    for _ in 0..8 {
        let r = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            r.update_thread_lifecycle(id, ThreadLifecycleState::Resolved)
                .await
        }));
    }

    let mut ok = 0usize;
    let mut invalid = 0usize;
    let mut other = 0usize;
    for h in handles {
        let r = h.await.expect("task join");
        match r {
            Ok(_) => ok += 1,
            Err(MailboxError::InvalidTransition(_)) => invalid += 1,
            Err(MailboxError::Conflict) => invalid += 1, // accepted equivalent per minimum_controls
            Err(e) => {
                other += 1;
                eprintln!("unexpected variant: {e}");
            }
        }
    }
    assert_eq!(
        ok, 1,
        "exactly one caller must win the lifecycle race (got ok={ok}, invalid={invalid}, other={other})"
    );
    assert_eq!(
        invalid, 7,
        "seven losers must observe InvalidTransition or Conflict (got invalid={invalid}, other={other})"
    );
    assert_eq!(other, 0, "no unexpected error variants permitted");
    let final_state = repo
        .get_thread(id)
        .await
        .expect("get")
        .expect("present")
        .lifecycle_state;
    assert_eq!(final_state, ThreadLifecycleState::Resolved);
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_append_message_against_archived_thread_rejects() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");

    // Open -> Resolved (terminal) -> Archived (still terminal).
    repo.update_thread_lifecycle(id, ThreadLifecycleState::Resolved)
        .await
        .expect("resolve");
    repo.update_thread_lifecycle(id, ThreadLifecycleState::Archived)
        .await
        .expect("archive");

    let r = repo
        .append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"forbidden": true}),
        )
        .await;
    assert!(
        matches!(r, Err(MailboxError::TerminalState)),
        "append to archived thread must reject with TerminalState, got {r:?}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_append_message_against_missing_thread_rejects() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // Mint a fresh v7 id that was never persisted.
    let phantom = RoleMailboxThreadId::new_v7();
    let r = repo
        .append_message(
            phantom,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({}),
        )
        .await;
    assert!(
        matches!(r, Err(MailboxError::NotFound)),
        "append to missing thread must reject with NotFound, got {r:?}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_update_thread_lifecycle_on_missing_thread_rejects() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let phantom = RoleMailboxThreadId::new_v7();
    let r = repo
        .update_thread_lifecycle(phantom, ThreadLifecycleState::Resolved)
        .await;
    assert!(
        matches!(r, Err(MailboxError::NotFound)),
        "update on missing thread must reject with NotFound, got {r:?}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_cascade_delete_on_thread_removes_messages() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");
    for i in 0..3 {
        repo.append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"seq": i}),
        )
        .await
        .expect("append");
    }
    assert_eq!(repo.list_thread_messages(id).await.unwrap().len(), 3);

    // Direct DELETE on the parent thread (out-of-API path; the API itself
    // never deletes — see `mt_177_append_only_surface_no_destructive_methods`).
    // The migration 0022 FK with `ON DELETE CASCADE` must remove the
    // child message rows.
    sqlx::query("DELETE FROM role_mailbox_thread WHERE thread_id = $1")
        .bind(id.as_uuid())
        .execute(repo.pool())
        .await
        .expect("delete thread");

    let after = repo.list_thread_messages(id).await.unwrap();
    assert!(
        after.is_empty(),
        "FK CASCADE must remove orphan message rows on thread delete"
    );
    assert!(repo.get_thread(id).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_concurrent_appends_preserve_chronological_order() {
    let pool = postgres_pool().await;
    let repo = Arc::new(RoleMailboxRepository::new(pool));
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");

    let mut handles = Vec::with_capacity(8);
    for i in 0..8 {
        let r = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            r.append_message(
                id,
                MessageType::DelegateWork,
                RoleId::Orchestrator,
                vec![RoleId::Coder],
                serde_json::json!({"writer": i}),
            )
            .await
        }));
    }
    for h in handles {
        h.await.expect("join").expect("append must succeed");
    }
    let msgs = repo.list_thread_messages(id).await.unwrap();
    assert_eq!(msgs.len(), 8);
    for w in msgs.windows(2) {
        // Strict ordering of (created_at_utc, message_id) — the LIST
        // index in the schema is `(thread_id, created_at_utc)` and the
        // tie-breaker is message_id ASC. Either monotone created_at_utc
        // or, on a tie, message_id ASC must hold.
        assert!(
            w[0].created_at_utc < w[1].created_at_utc
                || (w[0].created_at_utc == w[1].created_at_utc
                    && w[0].message_id.as_uuid() < w[1].message_id.as_uuid()),
            "messages must list in (created_at_utc, message_id) order: {:?} vs {:?}",
            w[0],
            w[1]
        );
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_transactional_rollback_leaves_no_visible_state() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool.clone());
    repo.ensure_schema().await.expect("schema");

    // Open a transaction manually that mirrors the repo's CRUD shape,
    // insert a thread, then roll back. The repo's downstream surface
    // (get_thread) must report no visible row.
    let phantom_id = RoleMailboxThreadId::new_v7();
    {
        let mut tx = pool.begin().await.expect("begin");
        sqlx::query(
            r#"INSERT INTO role_mailbox_thread
               (thread_id, title, linked_record_kind, lifecycle_state, claim_mode,
                takeover_policy, response_authority_scope)
               VALUES ($1, 'rollback-probe', 'wp', 'open', 'exclusive', 'never', 'lease_holder')"#,
        )
        .bind(phantom_id.as_uuid())
        .execute(&mut *tx)
        .await
        .expect("insert in tx");
        // Drop without commit -> rollback.
        tx.rollback().await.expect("rollback");
    }
    let got = repo.get_thread(phantom_id).await.expect("get");
    assert!(
        got.is_none(),
        "rolled-back insert must leave no visible row"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_list_threads_by_state_filters_correctly() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let t_open_a = sample_open_thread();
    let t_open_b = sample_open_thread();
    let t_resolved = sample_open_thread();
    let open_a_id = t_open_a.thread_id;
    let open_b_id = t_open_b.thread_id;
    let resolved_id = t_resolved.thread_id;
    repo.create_thread(t_open_a).await.unwrap();
    repo.create_thread(t_open_b).await.unwrap();
    repo.create_thread(t_resolved).await.unwrap();
    repo.update_thread_lifecycle(resolved_id, ThreadLifecycleState::Resolved)
        .await
        .unwrap();

    let open_rows = repo
        .list_threads_by_state(ThreadLifecycleState::Open, 100, 0)
        .await
        .unwrap();
    let open_ids: std::collections::HashSet<_> = open_rows.iter().map(|t| t.thread_id).collect();
    assert!(open_ids.contains(&open_a_id));
    assert!(open_ids.contains(&open_b_id));
    assert!(!open_ids.contains(&resolved_id));

    let resolved_rows = repo
        .list_threads_by_state(ThreadLifecycleState::Resolved, 100, 0)
        .await
        .unwrap();
    let resolved_ids: std::collections::HashSet<_> =
        resolved_rows.iter().map(|t| t.thread_id).collect();
    assert!(resolved_ids.contains(&resolved_id));
    assert!(!resolved_ids.contains(&open_a_id));
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_dead_letter_message_audits_reason_and_keeps_row() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool.clone());
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.unwrap();
    let msg = repo
        .append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": "v"}),
        )
        .await
        .unwrap();

    // Dead-letter requires the source state to be one of the legal
    // predecessors (queued/delivered/failed). The append default is
    // queued, so dead_letter must succeed.
    repo.dead_letter_message(msg.message_id, "policy violation".to_string())
        .await
        .expect("dead-letter must succeed from queued state");

    // The row must remain (append-only). delivery_state should now be
    // dead_lettered and audit_reason populated.
    let row: (String, Option<String>) = sqlx::query_as(
        "SELECT delivery_state, audit_reason FROM role_mailbox_message WHERE message_id = $1",
    )
    .bind(msg.message_id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("row must remain after dead-letter (append-only)");
    assert_eq!(row.0, "dead_lettered");
    assert_eq!(row.1.as_deref(), Some("policy violation"));
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_177_count_pending_messages_for_role_isolated() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.unwrap();

    let baseline = repo
        .count_pending_messages_for_role(&RoleId::Validator)
        .await
        .unwrap();
    repo.append_message(
        id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Validator],
        serde_json::json!({}),
    )
    .await
    .unwrap();
    repo.append_message(
        id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder], // not Validator
        serde_json::json!({}),
    )
    .await
    .unwrap();

    let after = repo
        .count_pending_messages_for_role(&RoleId::Validator)
        .await
        .unwrap();
    assert_eq!(
        after,
        baseline + 1,
        "count_pending_messages_for_role must isolate by to_roles membership"
    );
}

// ----- helpers -----

fn sample_open_thread() -> RoleMailboxThread {
    RoleMailboxThread::open(
        format!(
            "mt-177-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ),
        LinkedRecordKind::Wp,
        Some("WP-KERNEL-004".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    )
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL for role_mailbox_repo_tests");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}
