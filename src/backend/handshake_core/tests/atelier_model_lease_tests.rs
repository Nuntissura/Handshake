//! WP-KERNEL-005 MT-143 proof — lease/claim contract for parallel model
//! coordination, enforced against Handshake-managed PostgreSQL.
//!
//! TTL, stale state, and conflict errors are proven against the database
//! clock: leases are persisted through `AtelierStore`, RE-READ from
//! PostgreSQL, expiry becomes observable on re-read without any writer,
//! conflicting exclusive claims fail typed, expired holders are taken over
//! durably, and every mutation mirrors through the EventLedger.

mod atelier_pg_support;

use std::sync::Arc;

use atelier_pg_support::database_url;
use handshake_core::atelier::model_lease::{model_lease_event_family, NewModelLeaseClaim};
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::kernel::role_mailbox_claim_lease::{
    ClaimLeaseState, RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

fn new_claim(thread_id: &str, actor_id: &str, ttl_seconds: i64) -> NewModelLeaseClaim {
    NewModelLeaseClaim {
        thread_id: thread_id.to_string(),
        executor_kind: RoleMailboxExecutorKind::LocalSmallModel,
        actor_id: actor_id.to_string(),
        session_id: format!("session-{actor_id}"),
        claim_mode: RoleMailboxClaimMode::ExclusiveLease,
        ttl_seconds,
        linked_work_packet_id: "WP-KERNEL-005".to_string(),
        linked_micro_task_id: "MT-143".to_string(),
    }
}

async fn assert_lease_event(
    database: &Arc<dyn Database>,
    claim_id: Uuid,
    event_family: &str,
) -> serde_json::Value {
    let events = database
        .list_kernel_events_for_aggregate("atelier_model_lease", &claim_id.to_string())
        .await
        .expect("list model lease EventLedger rows");
    let event = events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family
        })
        .unwrap_or_else(|| panic!("lease {claim_id} must emit {event_family}"));
    event.payload["atelier_payload"].clone()
}

#[tokio::test]
async fn mt143_claim_persists_to_pg_and_rereads_with_db_clock_ttl_view() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt143_claim_persists: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let thread_id = format!("mt143-thread-{}", Uuid::now_v7());
    let claimed = store
        .claim_model_lease(&new_claim(&thread_id, "coder-alpha", 3600))
        .await
        .expect("claim model lease");
    assert_eq!(claimed.stored_state, ClaimLeaseState::Active);
    assert!(!claimed.lease_expired);

    // RE-READ from PostgreSQL: the persisted row, not the claim echo.
    let reread = store
        .get_model_lease(claimed.claim_id)
        .await
        .expect("re-read lease from PostgreSQL");
    assert_eq!(reread.claim_id, claimed.claim_id);
    assert_eq!(reread.thread_id, thread_id);
    assert_eq!(reread.actor_id, "coder-alpha");
    assert_eq!(reread.claim_mode, RoleMailboxClaimMode::ExclusiveLease);
    assert_eq!(reread.ttl_seconds, 3600);
    assert_eq!(reread.effective_state, ClaimLeaseState::Active);
    assert!(!reread.lease_expired, "fresh 1h lease must not be expired");
    assert!(reread.lease_age_seconds >= 0);
    assert!(reread.lease_expires_at_utc > reread.claimed_at_utc);

    let payload = assert_lease_event(
        &database,
        claimed.claim_id,
        model_lease_event_family::MODEL_LEASE_CLAIMED,
    )
    .await;
    assert_eq!(payload["thread_id"], serde_json::json!(thread_id));
    assert_eq!(payload["actor_id"], serde_json::json!("coder-alpha"));
    assert_eq!(payload["ttl_seconds"], serde_json::json!(3600));
}

#[tokio::test]
async fn mt143_ttl_expiry_is_observable_on_reread_without_any_writer() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt143_ttl_expiry: PostgreSQL unavailable");
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let thread_id = format!("mt143-ttl-{}", Uuid::now_v7());
    let claimed = store
        .claim_model_lease(&new_claim(&thread_id, "coder-ttl", 1))
        .await
        .expect("claim 1s lease");
    assert!(!claimed.lease_expired);

    // Let the database clock pass lease_expires_at_utc. No writer runs.
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let stale = store
        .get_model_lease(claimed.claim_id)
        .await
        .expect("re-read stale lease");
    assert!(
        stale.lease_expired,
        "TTL must be enforced by the database clock on re-read"
    );
    assert_eq!(
        stale.stored_state,
        ClaimLeaseState::Active,
        "no writer ran, so the stored row is untouched"
    );
    assert_eq!(
        stale.effective_state,
        ClaimLeaseState::Expired,
        "the effective state must surface the TTL expiry"
    );
    assert!(stale.lease_age_seconds >= 1);
}

#[tokio::test]
async fn mt143_conflicting_exclusive_claim_fails_typed_against_pg() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt143_conflict: PostgreSQL unavailable");
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let thread_id = format!("mt143-conflict-{}", Uuid::now_v7());
    let holder = store
        .claim_model_lease(&new_claim(&thread_id, "coder-holder", 3600))
        .await
        .expect("claim holder lease");

    let err = store
        .claim_model_lease(&new_claim(&thread_id, "coder-intruder", 3600))
        .await
        .expect_err("second exclusive claim on an unexpired thread must fail");
    match err {
        AtelierError::Conflict(message) => {
            assert!(
                message.contains("coder-holder"),
                "conflict must cite the current holder: {message}"
            );
        }
        other => panic!("expected typed Conflict, got: {other:?}"),
    }

    // The holder's lease is untouched by the rejected claim.
    let reread = store
        .get_model_lease(holder.claim_id)
        .await
        .expect("re-read holder lease");
    assert_eq!(reread.effective_state, ClaimLeaseState::Active);
}

#[tokio::test]
async fn mt143_expired_holder_is_taken_over_durably_with_ledger_event() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt143_takeover: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let thread_id = format!("mt143-takeover-{}", Uuid::now_v7());
    let stale_holder = store
        .claim_model_lease(&new_claim(&thread_id, "coder-stale", 1))
        .await
        .expect("claim 1s lease");
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let successor = store
        .claim_model_lease(&new_claim(&thread_id, "coder-successor", 3600))
        .await
        .expect("claim over an expired holder must succeed");
    assert_eq!(successor.prior_claim_id, Some(stale_holder.claim_id));
    assert_eq!(successor.effective_state, ClaimLeaseState::Active);

    // The prior holder's terminal state is persisted, not inferred.
    let taken_over = store
        .get_model_lease(stale_holder.claim_id)
        .await
        .expect("re-read taken-over lease");
    assert_eq!(taken_over.stored_state, ClaimLeaseState::TakenOver);
    assert!(taken_over.taken_over_at_utc.is_some());
    assert!(taken_over
        .takeover_reason
        .as_deref()
        .unwrap_or_default()
        .contains("coder-successor"));

    let payload = assert_lease_event(
        &database,
        stale_holder.claim_id,
        model_lease_event_family::MODEL_LEASE_TAKEN_OVER,
    )
    .await;
    assert_eq!(
        payload["taken_over_by_claim_id"],
        serde_json::json!(successor.claim_id)
    );

    let history = store
        .list_model_leases_for_thread(&thread_id)
        .await
        .expect("list thread lease history");
    assert_eq!(history.len(), 2, "both lease rows must persist");
}

#[tokio::test]
async fn mt143_renew_and_release_enforce_holder_and_ttl_on_pg() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt143_renew_release: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    // Renew by the holder extends from the database clock.
    let thread_id = format!("mt143-renew-{}", Uuid::now_v7());
    let lease = store
        .claim_model_lease(&new_claim(&thread_id, "coder-renew", 2))
        .await
        .expect("claim lease");
    let renewed = store
        .renew_model_lease(lease.claim_id, "coder-renew", 3600)
        .await
        .expect("holder renews unexpired lease");
    assert_eq!(renewed.ttl_seconds, 3600);
    assert!(renewed.lease_expires_at_utc > lease.lease_expires_at_utc);
    assert_lease_event(
        &database,
        lease.claim_id,
        model_lease_event_family::MODEL_LEASE_RENEWED,
    )
    .await;

    // A foreign actor cannot renew.
    let foreign = store
        .renew_model_lease(lease.claim_id, "coder-other", 3600)
        .await
        .expect_err("foreign renew must fail");
    assert!(matches!(foreign, AtelierError::Conflict(_)));

    // Release by the holder persists 'released' + event.
    let released = store
        .release_model_lease(lease.claim_id, "coder-renew")
        .await
        .expect("holder releases lease");
    assert_eq!(released.stored_state, ClaimLeaseState::Released);
    assert!(released.released_at_utc.is_some());
    let reread = store
        .get_model_lease(lease.claim_id)
        .await
        .expect("re-read released lease");
    assert_eq!(reread.effective_state, ClaimLeaseState::Released);
    assert_lease_event(
        &database,
        lease.claim_id,
        model_lease_event_family::MODEL_LEASE_RELEASED,
    )
    .await;

    // Renewing an expired lease is a typed conflict.
    let thread_id = format!("mt143-renew-expired-{}", Uuid::now_v7());
    let short = store
        .claim_model_lease(&new_claim(&thread_id, "coder-late", 1))
        .await
        .expect("claim 1s lease");
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let expired = store
        .renew_model_lease(short.claim_id, "coder-late", 3600)
        .await
        .expect_err("renewing an expired lease must fail");
    match expired {
        AtelierError::Conflict(message) => {
            assert!(
                message.contains("expired=true"),
                "conflict must surface the TTL expiry: {message}"
            );
        }
        other => panic!("expected typed Conflict, got: {other:?}"),
    }
}
