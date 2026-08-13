//! WP-1 HBR-PRIV negative proof: account-bound resource scope on the real
//! ModelLane and ModelRuntime-registry read/write paths.
//!
//! # What this file is for
//!
//! Every test here is a NEGATIVE test. It exists to prove that a read which
//! used to succeed now fails, against REAL PostgreSQL — no SQLite, no mock, no
//! structs-only fallback. A missing cluster is a failing proof, never a green
//! skip.
//!
//! # Both enforcement layers are proven separately
//!
//! HBR-PRIV-002 says hiding a row in one layer is never sufficient, so
//! enforcement is in two places and each is proven on its own terms:
//!
//!   * **Layer 1 (SQL):** the owner predicate is pushed into the `WHERE`
//!     clause, so a denied row is never transferred out of PostgreSQL. Its
//!     observable signature is therefore *absence* — `NotFound`, not a denial
//!     code. Every layer-1 assertion is paired with a positive control proving
//!     the same row IS readable under its own scope, so "not found" can never
//!     pass vacuously because the row was never written.
//!
//!   * **Layer 2 (post-deserialization):** the stored scope columns are read
//!     back and re-authorized. This is proven by deliberately issuing the SAME
//!     query WITHOUT the owner predicate — i.e. simulating a future edit that
//!     drops layer 1 — and showing the row that comes back is still denied,
//!     with the stable reason code.
//!
//! # Falsifiability
//!
//! `positive_control_*` tests assert the reads DO succeed under the correct
//! scope. If enforcement were over-broad (deny everything) the negatives would
//! still pass while the positives failed, so the pair is what makes the suite
//! meaningful. See `FALSIFIABILITY` notes on the cross-account test.

mod knowledge_pg_support;

use std::path::PathBuf;

use chrono::Utc;
use handshake_core::model_runtime::{
    BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRegistryStore,
    OperatorId as RegistryOperatorId, ProviderKind as RegistryProviderKind,
    RuntimeBinding as RegistryRuntimeBinding,
};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneError, ModelLaneKind, ModelLaneLocusBinding, ModelLaneMessageKind,
    ModelLaneNavigationLookup, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget,
    NewModelLane, NewModelLaneDiagnosticTierStatus, NewModelLaneMessage, NewModelLaneRun,
    RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    stored_resource_scope_from_row, AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
    OwnerAccountId, ResourceAccessContext, ResourceScope, ResourceScopeError, ResourceScopeQuery,
    ScopeDenied, StoredResourceScope, SystemScopeAuthority, WorkspaceScopeRef,
    RESOURCE_SCOPE_SELECT_COLUMNS,
};
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

async fn pg_pool(test_name: &str) -> PgPool {
    let pg = knowledge_pg_support::knowledge_pg().await.unwrap_or_else(|| {
        panic!(
            "PostgreSQL unavailable for {test_name}: HBR-PRIV account-scope proof requires live Handshake-managed PostgreSQL"
        )
    });
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect isolated model-lane scope schema")
}

fn scope_for(owner: OwnerAccountId) -> ResourceScope {
    ResourceScope::new(owner, ActorPrincipalId::mint())
}

fn scope_in_workspace(owner: OwnerAccountId, workspace: &str) -> ResourceScope {
    scope_for(owner).with_workspace(WorkspaceScopeRef::new(workspace).unwrap())
}

/// Store bound to one account for both reads and writes.
fn account_store(pool: &PgPool, scope: &ResourceScope) -> ModelLaneStore {
    ModelLaneStore::new_scoped(pool.clone(), scope.clone())
}

/// Reader bound to one account. Read-only by construction.
fn reader_store(pool: &PgPool, query: ResourceScopeQuery) -> ModelLaneStore {
    ModelLaneStore::new_for_owner(pool.clone(), query)
}

/// Write one complete run + local lane + message under `scope`, and return the
/// run id. Every identifier is derived from `slug` so two owners can seed
/// structurally identical data that differs only in ownership.
async fn seed_run(store: &ModelLaneStore, slug: &str) -> String {
    let run_id = format!("run-{slug}");
    let lane_id = format!("lane-{slug}");
    store
        .record_run(sample_run(&run_id, slug))
        .await
        .unwrap_or_else(|error| panic!("seed run {run_id}: {error}"));
    store
        .record_lane(sample_lane(&run_id, &lane_id, slug))
        .await
        .unwrap_or_else(|error| panic!("seed lane {lane_id}: {error}"));
    store
        .record_message(sample_message(&run_id, &lane_id, slug))
        .await
        .unwrap_or_else(|error| panic!("seed message for {run_id}: {error}"));
    // The diagnostics projection fails closed unless the HBR-INT-009 three-tier
    // posture is recorded for the run, so seed it here rather than weakening
    // that gate.
    for (tier, state, reason, follow_up) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "EventLedger business-event ledger is the Tier 1 recorder",
            None,
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "internal_diagnostics surface is not available in this worktree",
            Some("usermanual://model-lane-diagnostics#internal-diagnostics-deferred"),
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "external Palmistry watcher is not available in this worktree",
            Some("usermanual://model-lane-diagnostics#palmistry-deferred"),
        ),
    ] {
        store
            .record_diagnostic_tier_status(NewModelLaneDiagnosticTierStatus {
                diagnostic_status_id: format!("diag-{slug}-{}", tier.as_str()),
                behavior_id: "HBR-INT-009".into(),
                run_id: run_id.clone(),
                tier,
                state,
                reason: reason.into(),
                evidence_ref: format!("eventledger://kernel/{slug}"),
                follow_up_ref: follow_up.map(str::to_owned),
                event_ledger_stream_id: format!("mlane-stream-{run_id}"),
                work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
                micro_task_id: "MT-PRIV".into(),
                task_board_id: "task-board://wp-1".into(),
                owner_session: "KERNEL_BUILDER-20260628-220906".into(),
                idempotency_key: format!("idem-diag-{slug}-{}", tier.as_str()),
                diagnostic_payload: json!({
                    "flight_recorder": "WIRED",
                    "internal_diagnostics": "DEFERRED: diagnostics surface MT-008",
                    "palmistry": "DEFERRED: external watcher worktree"
                }),
            })
            .await
            .unwrap_or_else(|error| panic!("seed diagnostic tier for {run_id}: {error}"));
    }
    run_id
}

/// Read a row's stored scope columns with NO owner predicate at all.
///
/// This is the deliberate simulation of layer 1 being absent: it is exactly the
/// query the store would run if someone deleted the `AND owner_account_id = $n`
/// fragment. Feeding the result to `authorize_row` proves layer 2 independently
/// catches what layer 1 would have hidden.
async fn stored_scope_without_predicate(
    pool: &PgPool,
    table: &str,
    key_column: &str,
    key: &str,
) -> handshake_core::swarm_orchestration::resource_scope::StoredResourceScope {
    let sql =
        format!("SELECT {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} WHERE {key_column} = $1");
    let row = sqlx::query(&sql)
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("unpredicated read of {table}.{key_column}={key}: {error}"));
    stored_resource_scope_from_row(&row).expect("decode stored scope columns")
}

fn expect_not_found(result: Result<impl std::fmt::Debug, ModelLaneError>, what: &str) {
    match result {
        Err(ModelLaneError::NotFound(_)) => {}
        Err(other) => panic!("{what}: expected NotFound denial, got {other}"),
        Ok(value) => panic!("{what}: DISCLOSED another account's data: {value:?}"),
    }
}

// ---------------------------------------------------------------------------
// 1. Cross-account isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_accounts_cannot_read_each_others_lane_rows() {
    let pool = pg_pool("cross-account lane isolation").await;

    let alice = OwnerAccountId::mint();
    let bob = OwnerAccountId::mint();
    assert_ne!(alice, bob, "the two owning accounts must be distinct");

    let alice_scope = scope_for(alice);
    let bob_scope = scope_for(bob);
    let alice_store = account_store(&pool, &alice_scope);
    let bob_store = account_store(&pool, &bob_scope);

    let alice_run = seed_run(&alice_store, "alice").await;
    let bob_run = seed_run(&bob_store, "bob").await;

    // -- POSITIVE CONTROL ---------------------------------------------------
    // Without this, every negative below could pass simply because nothing was
    // ever written. FALSIFIABILITY: this is the assertion that fails first if
    // enforcement is over-broad.
    let own = alice_store
        .replay_run(&alice_run)
        .await
        .expect("the owning account must still replay its own run");
    assert_eq!(own.run.run_id, alice_run);
    assert_eq!(own.lanes.len(), 1, "alice's own lane must be visible");
    assert_eq!(own.messages.len(), 1, "alice's own message must be visible");

    // -- LAYER 1: replay_run ------------------------------------------------
    //
    // FALSIFIABILITY: this assertion was verified to be able to fail. Replacing
    // it with the opposite expectation (`.expect("alice must be able to replay
    // bob's run")`) produced a real failure —
    // `panicked ... : NotFound("run_id run-bob")` — before being restored, so
    // the negative is observing enforcement rather than passing vacuously.
    expect_not_found(
        alice_store.replay_run(&bob_run).await,
        "alice replaying bob's run",
    );
    expect_not_found(
        bob_store.replay_run(&alice_run).await,
        "bob replaying alice's run",
    );

    // -- LAYER 1: the eight navigation routes -------------------------------
    expect_not_found(
        alice_store.navigation_by_run(&bob_run).await,
        "navigation_by_run across accounts",
    );
    expect_not_found(
        alice_store.navigation_by_lane("lane-bob").await,
        "navigation_by_lane across accounts",
    );
    expect_not_found(
        alice_store.navigation_by_message("msg-bob").await,
        "navigation_by_message across accounts",
    );
    expect_not_found(
        alice_store.navigation_by_recovery(&bob_run).await,
        "navigation_by_recovery across accounts",
    );
    expect_not_found(
        alice_store
            .navigation_by_diagnostics(&bob_run, None, None, None)
            .await,
        "navigation_by_diagnostics across accounts",
    );
    expect_not_found(
        alice_store.navigation_by_trace("trace-bob", None).await,
        "navigation_by_trace across accounts",
    );
    expect_not_found(
        alice_store
            .navigation_by_artifact_or_context(None, Some("ctx-bob"), None)
            .await,
        "navigation_by_artifact_or_context across accounts",
    );
    expect_not_found(
        alice_store
            .navigation_by_lookup(ModelLaneNavigationLookup {
                run_id: Some(bob_run.clone()),
                ..Default::default()
            })
            .await,
        "navigation_by_lookup across accounts",
    );

    // -- LAYER 1: diagnostics projection ------------------------------------
    // `latest_diagnostics_projection` used to hand back whoever's run was
    // globally newest. Bob seeded last, so before scoping this returned BOB's
    // run to alice.
    let latest = alice_store
        .latest_diagnostics_projection()
        .await
        .expect("alice has a run of her own to project");
    assert_eq!(
        latest.run.run_id, alice_run,
        "latest diagnostics must be the newest run THIS account owns, not the newest on the node"
    );

    // -- LAYER 2: post-deserialization authorization ------------------------
    // Simulate the SQL predicate being dropped. The row comes back; the second
    // layer must still refuse it, with the stable reason code.
    let bobs_run_scope =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &bob_run).await;
    let denied = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice))
        .authorize_row(&bobs_run_scope)
        .expect_err("layer 2 must deny a cross-account row even with no SQL predicate");
    assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_OWNER_MISMATCH");

    let bobs_lane_scope =
        stored_scope_without_predicate(&pool, "model_lanes", "lane_id", "lane-bob").await;
    assert_eq!(
        ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice))
            .authorize_row(&bobs_lane_scope)
            .expect_err("layer 2 must deny a cross-account lane row")
            .reason_code(),
        "RESOURCE_SCOPE_OWNER_MISMATCH"
    );

    let bobs_message_scope =
        stored_scope_without_predicate(&pool, "model_lane_messages", "message_id", "msg-bob").await;
    assert_eq!(
        ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice))
            .authorize_row(&bobs_message_scope)
            .expect_err("layer 2 must deny a cross-account message row")
            .reason_code(),
        "RESOURCE_SCOPE_OWNER_MISMATCH"
    );

    // And the write path really did stamp distinct owners — otherwise every
    // assertion above would be testing nothing.
    assert_eq!(
        bobs_run_scope.owner_account_id,
        Some(bob),
        "bob's run row must be stamped with bob's account"
    );
    let alices_run_scope =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &alice_run).await;
    assert_eq!(alices_run_scope.owner_account_id, Some(alice));
    assert_ne!(
        alices_run_scope.owner_account_id, bobs_run_scope.owner_account_id,
        "the two seeded runs must differ in owner, not just in id"
    );
}

// ---------------------------------------------------------------------------
// 2. Same account, two workspaces (the same-project privacy case)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_account_cannot_read_across_its_own_workspaces() {
    let pool = pg_pool("cross-workspace isolation").await;

    let owner = OwnerAccountId::mint();
    let alpha_scope = scope_in_workspace(owner, "ws-alpha");
    let beta_scope = scope_in_workspace(owner, "ws-beta");

    let alpha_store = account_store(&pool, &alpha_scope);
    let beta_store = account_store(&pool, &beta_scope);

    let alpha_run = seed_run(&alpha_store, "alpha").await;
    let beta_run = seed_run(&beta_store, "beta").await;

    // POSITIVE CONTROL: within its own workspace the read still works.
    alpha_store
        .replay_run(&alpha_run)
        .await
        .expect("the owning workspace must still replay its own run");

    // LAYER 1: same owning account, different workspace -> denied.
    expect_not_found(
        alpha_store.replay_run(&beta_run).await,
        "alpha workspace replaying beta workspace's run",
    );
    expect_not_found(
        beta_store.replay_run(&alpha_run).await,
        "beta workspace replaying alpha workspace's run",
    );
    expect_not_found(
        alpha_store.navigation_by_run(&beta_run).await,
        "cross-workspace navigation",
    );

    // LAYER 2: the reason code distinguishes a workspace denial from an owner
    // denial, so operator-facing diagnostics stay actionable.
    let beta_row =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &beta_run).await;
    let denied = ResourceAccessContext::for_reader(
        ResourceScopeQuery::for_owner(owner)
            .within_workspace(WorkspaceScopeRef::new("ws-alpha").unwrap()),
    )
    .authorize_row(&beta_row)
    .expect_err("a row in another workspace of the same account must be denied");
    assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_WORKSPACE_MISMATCH");
    assert_eq!(
        beta_row.owner_account_id,
        Some(owner),
        "this must be the SAME owning account, or it is only re-proving the cross-account case"
    );
}

// ---------------------------------------------------------------------------
// 3. Pre-0363 style unattributed rows are denied, not grandfathered
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unattributed_legacy_row_is_denied_not_grandfathered() {
    let pool = pg_pool("legacy unattributed row denial").await;

    // A store holding the legacy system authority writes rows with a NULL
    // owner_account_id — structurally identical to a row that existed before
    // migration 0363 added the columns.
    let legacy_store = ModelLaneStore::new_system_authority(
        pool.clone(),
        SystemScopeAuthority::internal_subsystem("TEST_PRE_0363_ROW"),
    );
    let legacy_run = seed_run(&legacy_store, "legacy").await;

    let stored =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &legacy_run).await;
    assert_eq!(
        stored.owner_account_id, None,
        "the fixture must actually be unattributed, or this proves nothing"
    );

    // LAYER 1: an account reader cannot see it.
    let reader = reader_store(&pool, ResourceScopeQuery::for_owner(OwnerAccountId::mint()));
    expect_not_found(
        reader.replay_run(&legacy_run).await,
        "account reader replaying an unattributed legacy run",
    );
    expect_not_found(
        reader.navigation_by_run(&legacy_run).await,
        "account reader navigating an unattributed legacy run",
    );

    // LAYER 2: and it is denied on its own merits, not merely filtered out.
    let denied = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(
        OwnerAccountId::mint(),
    ))
    .authorize_row(&stored)
    .expect_err("a row with no owning account must never be readable by an account");
    assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_UNATTRIBUTED");

    // The explicitly-system store can still read it. That is the documented,
    // named cross-owner path (restart recovery), not an accident.
    legacy_store
        .replay_run(&legacy_run)
        .await
        .expect("an explicit SystemScopeAuthority store may read unattributed rows");
}

// ---------------------------------------------------------------------------
// 4. Derived output cannot widen scope
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_derivative_of_mixed_scope_sources_is_not_readable_under_either_source() {
    let pool = pg_pool("derived scope non-widening").await;

    let owner = OwnerAccountId::mint();
    let source_actor = ActorPrincipalId::mint();
    let source_a = ResourceScope::new(owner, source_actor)
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new("ws-alpha").expect("nonblank workspace"));

    // HBR-PRIV-004: the derivative inherits an access scope no broader than ALL
    // contributing sources. First prove all five exact dimensions survive
    // PostgreSQL when every source has the same scope.
    let same_scope_source = source_a.clone();
    let derived =
        ResourceScope::derive_from_sources([&source_a, &same_scope_source], source_actor)
            .expect("same exact-scope derivation is allowed");
    assert_eq!(derived, source_a, "derivation must preserve all five fields");
    let derived_store = account_store(&pool, &derived);
    let derived_run = seed_run(&derived_store, "derived").await;

    let stored =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &derived_run).await;
    assert_eq!(
        stored,
        StoredResourceScope::from(&source_a),
        "the persisted derivative must retain every exact source-scope dimension"
    );

    let run_count_before_negatives: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_runs")
            .fetch_one(&pool)
            .await
            .expect("count runs before mixed-scope derivation negatives");

    let mut wrong_owner = source_a.clone();
    wrong_owner.owner_account_id = OwnerAccountId::mint();
    assert!(matches!(
        ResourceScope::derive_from_sources([&source_a, &wrong_owner], source_actor)
            .expect_err("owner mismatch must fail before persistence"),
        ResourceScopeError::MixedOwnerDerivation { .. }
    ));

    let mut wrong_actor = source_a.clone();
    wrong_actor.actor_principal_id = ActorPrincipalId::mint();
    assert_eq!(
        ResourceScope::derive_from_sources([&source_a, &wrong_actor], source_actor)
            .expect_err("actor mismatch must fail before persistence"),
        ResourceScopeError::MixedActorPrincipalDerivation
    );
    assert_eq!(
        ResourceScope::derive_from_sources(
            [&source_a, &same_scope_source],
            ActorPrincipalId::mint(),
        )
        .expect_err("actor retarget requires delegation authority this seam does not carry"),
        ResourceScopeError::DerivativeActorRetargetDenied
    );

    let mut wrong_session = source_a.clone();
    wrong_session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mut missing_session = source_a.clone();
    missing_session.authenticated_session = None;
    for conflicting in [&wrong_session, &missing_session] {
        for sources in [[&source_a, conflicting], [conflicting, &source_a]] {
            assert_eq!(
                ResourceScope::derive_from_sources(sources, source_actor)
                    .expect_err("session mismatch must fail in either source order"),
                ResourceScopeError::MixedAuthenticatedSessionDerivation
            );
        }
    }

    let mut wrong_access_space = source_a.clone();
    wrong_access_space.access_space = Some(AccessSpaceRef::mint());
    let mut missing_access_space = source_a.clone();
    missing_access_space.access_space = None;
    for conflicting in [&wrong_access_space, &missing_access_space] {
        for sources in [[&source_a, conflicting], [conflicting, &source_a]] {
            assert_eq!(
                ResourceScope::derive_from_sources(sources, source_actor)
                    .expect_err("AccessSpace mismatch must fail in either source order"),
                ResourceScopeError::MixedAccessSpaceDerivation
            );
        }
    }

    let mut wrong_workspace = source_a.clone();
    wrong_workspace.workspace = Some(
        WorkspaceScopeRef::new("ws-beta").expect("nonblank conflicting workspace"),
    );
    let mut missing_workspace = source_a.clone();
    missing_workspace.workspace = None;
    for conflicting in [&wrong_workspace, &missing_workspace] {
        for sources in [[&source_a, conflicting], [conflicting, &source_a]] {
            assert_eq!(
                ResourceScope::derive_from_sources(sources, source_actor)
                    .expect_err("workspace mismatch must fail in either source order"),
                ResourceScopeError::MixedWorkspaceDerivation
            );
        }
    }

    let run_count_after_negatives: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_runs")
            .fetch_one(&pool)
            .await
            .expect("count runs after mixed-scope derivation negatives");
    assert_eq!(
        run_count_after_negatives, run_count_before_negatives,
        "rejected one-field and missing-field derivations must create no durable row"
    );

    derived_store
        .replay_run(&derived_run)
        .await
        .expect("the complete exact source scope must replay its derivative");
}

// ---------------------------------------------------------------------------
// 5. Denials do not leak metadata
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_cross_account_denial_leaks_no_identifiers_or_row_contents() {
    let pool = pg_pool("denial metadata side channel").await;

    let alice = OwnerAccountId::mint();
    let bob = OwnerAccountId::mint();
    let bob_scope = scope_for(bob);
    let bob_store = account_store(&pool, &bob_scope);
    let bob_run = seed_run(&bob_store, "leaky").await;

    let alice_store = reader_store(&pool, ResourceScopeQuery::for_owner(alice));
    let error = alice_store
        .replay_run(&bob_run)
        .await
        .expect_err("cross-account replay must fail");
    let rendered = error.to_string();

    // The caller asked for a run id it already knows, so echoing that id back is
    // not a disclosure. Everything that identifies the OWNER or the row's
    // contents must be absent.
    for secret in [
        bob.to_string(),
        bob_scope.actor_principal_id.to_string(),
        "lane-leaky".to_owned(),
        "msg-leaky".to_owned(),
        "trace-leaky".to_owned(),
        "ctx-leaky".to_owned(),
    ] {
        assert!(
            !rendered.contains(&secret),
            "denial for a cross-account read leaked `{secret}`: {rendered}"
        );
    }

    // The typed denial reason itself is also identifier-free.
    let stored =
        stored_scope_without_predicate(&pool, "model_lane_runs", "run_id", &bob_run).await;
    let denied: ScopeDenied = ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice))
        .authorize_row(&stored)
        .unwrap_err();
    let denial_text = denied.to_string();
    assert!(!denial_text.contains(&bob.to_string()));
    assert!(!denial_text.contains(&alice.to_string()));
    assert_eq!(denied.reason_code(), "RESOURCE_SCOPE_OWNER_MISMATCH");
}

// ---------------------------------------------------------------------------
// 6. ModelRuntime registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_accounts_cannot_enumerate_each_others_model_registry_rows() {
    let pool = pg_pool("cross-account model registry isolation").await;

    let alice = OwnerAccountId::mint();
    let bob = OwnerAccountId::mint();

    let alice_store = ModelRegistryStore::new_scoped(pool.clone(), scope_for(alice));
    let bob_store = ModelRegistryStore::new_scoped(pool.clone(), scope_for(bob));

    let alice_row = alice_store
        .persist_and_read_back(&registration(0xA1, "alice-registry", "alice-observer"))
        .await
        .expect("alice registers her own model artifact");
    let bob_row = bob_store
        .persist_and_read_back(&registration(0xB2, "bob-registry", "bob-observer"))
        .await
        .expect("bob registers his own model artifact");
    assert_ne!(alice_row.artifact_sha256, bob_row.artifact_sha256);

    // POSITIVE CONTROL first: each account sees exactly its own row.
    let alice_rows = alice_store
        .list_recoverable()
        .await
        .expect("alice enumerates her registry");
    assert_eq!(
        alice_rows.len(),
        1,
        "alice must see exactly her own registry row, saw {}",
        alice_rows.len()
    );
    assert_eq!(alice_rows[0].artifact_sha256, alice_row.artifact_sha256);

    // NEGATIVE: bob's artifact is absent from alice's enumeration.
    assert!(
        !alice_rows
            .iter()
            .any(|row| row.artifact_sha256 == bob_row.artifact_sha256),
        "alice's registry enumeration disclosed bob's registered artifact"
    );

    let bob_rows = bob_store
        .list_recoverable()
        .await
        .expect("bob enumerates his registry");
    assert_eq!(bob_rows.len(), 1);
    assert_eq!(bob_rows[0].artifact_sha256, bob_row.artifact_sha256);

    // LAYER 2 for the registry table.
    let bob_registry_scope = stored_scope_without_predicate(
        &pool,
        "model_runtime_registry",
        "artifact_locator",
        &bob_row.artifact_locator,
    )
    .await;
    assert_eq!(bob_registry_scope.owner_account_id, Some(bob));
    assert_eq!(
        ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice))
            .authorize_row(&bob_registry_scope)
            .expect_err("layer 2 must deny another account's registry row")
            .reason_code(),
        "RESOURCE_SCOPE_OWNER_MISMATCH"
    );

    // A reader with no rows of its own gets an empty enumeration, not the node's.
    let stranger = ModelRegistryStore::new_for_owner(
        pool.clone(),
        ResourceScopeQuery::for_owner(OwnerAccountId::mint()),
    );
    assert!(
        stranger
            .list_recoverable()
            .await
            .expect("a stranger's enumeration must succeed and be empty")
            .is_empty(),
        "an account with no registry rows must not enumerate the whole table"
    );
    assert!(
        stranger
            .list_active_selections()
            .await
            .expect("a stranger's active-selection read must succeed and be empty")
            .is_empty(),
        "an account with no active selections must not see another account's defaults"
    );
}

// ---------------------------------------------------------------------------
// 7. The named system authority is required for the cross-owner boot scan
// ---------------------------------------------------------------------------

#[tokio::test]
async fn boot_recovery_refuses_to_run_from_an_account_scoped_store() {
    let pool = pg_pool("boot recovery authority gate").await;

    // Restart recovery is intentionally cross-owner. An account-scoped store
    // must not be able to reach it, or "recovery" becomes a disclosure route.
    let account_scoped = ModelLaneStore::new_scoped(pool.clone(), scope_for(OwnerAccountId::mint()));
    match account_scoped.recover_restartable_runs_at_boot().await {
        Err(ModelLaneError::AuthorityDenied(detail)) => {
            assert!(
                detail.contains("SystemScopeAuthority"),
                "the refusal must name the authority it requires: {detail}"
            );
        }
        Err(other) => panic!("expected AuthorityDenied, got {other}"),
        Ok(runs) => panic!(
            "an account-scoped store enumerated {} restartable runs across all owners",
            runs.len()
        ),
    }

    // The explicitly named boot authority is accepted.
    ModelLaneStore::new_system_authority(pool.clone(), SystemScopeAuthority::boot_recovery())
        .recover_restartable_runs_at_boot()
        .await
        .expect("the named boot-recovery authority is the supported cross-owner path");
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn sha256_hex() -> String {
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
}

fn locus_for(slug: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-PRIV".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-session-{slug}"),
        session_id: format!("session-lane-{slug}"),
        model_session_id: format!("model-session-lane-{slug}"),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding_ref: format!("locus://wp1/mt-priv/{slug}"),
    }
}

fn sample_run(run_id: &str, slug: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{slug}"),
        run_span_id: format!("span-run-{slug}"),
        coordinator_session_id: format!("coordinator-session-{slug}"),
        routing_policy: "local_only".into(),
        context_bundle_id: format!("ctx-{slug}"),
        lane_ids: vec![format!("lane-{slug}")],
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        artifact_namespace: format!("artifact://model-lane/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-PRIV".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        idempotency_key: format!("idem-run-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#recovery".into()),
        locus_binding: Some(locus_for(slug)),
        memory_pack_ref: format!("memory-pack://fems/{slug}/run"),
        memory_pack_hash: sha256_hex(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: format!("budget://{slug}/local-only"),
        selected_model_id: Some("model://mt-priv/deterministic-fake".into()),
        candidate_model_ids: vec!["model://mt-priv/deterministic-fake".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(run_id: &str, lane_id: &str, slug: &str) -> NewModelLane {
    let session_id = format!("session-lane-{slug}");
    let model_session_id = format!("model-session-lane-{slug}");
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{slug}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        kind: ModelLaneKind::LocalModel,
        role: format!("role-{lane_id}"),
        backend: "local".into(),
        model_id: Some("model://mt-priv/deterministic-fake".into()),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt-priv/tool-read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt-priv".into()),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt-priv/{lane_id}")),
        provider_feature_profile_ref: Some("provider-feature-profile://local_runtime".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/local".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/model_runtime".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt-priv/allow-read".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-28T22:30:00Z".into()),
        lease_expires_at_utc: Some("2026-06-28T22:40:00Z".into()),
        reclaim_after_utc: Some("2026-06-28T22:41:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt-priv/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt-priv/scope-proof".into()),
        terminal_status_mapping_ref: Some("terminal-status://session-broker/local".into()),
        process_ownership_ref: Some(format!("process-ledger://mt-priv/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt-priv/bounded".into()),
        last_runtime_status_ref: Some("runtime-status://mt-priv/ready".into()),
        last_recovery_event_ref: Some("recovery://mt-priv/startable".into()),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#lane-recovery".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-PRIV".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding: Some(ModelLaneLocusBinding {
            session_id,
            model_session_id,
            ..locus_for(slug)
        }),
    }
}

fn sample_message(run_id: &str, lane_id: &str, slug: &str) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: format!("msg-{slug}"),
        run_id: run_id.into(),
        trace_id: format!("trace-{slug}"),
        message_span_id: format!("span-msg-{slug}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("span-{lane_id}")],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-session-{slug}"),
            correlation_id: format!("corr-{slug}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://model-lane/messages/msg-{slug}"),
        payload_sha256: sha256_hex(),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        summary: "local lane proposes a typed patch".into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt-priv/allow-read".into()],
        coordinator_session_id: format!("coordinator-session-{slug}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-PRIV".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260628-220906".into(),
        locus_binding: Some(ModelLaneLocusBinding {
            session_id: format!("session-lane-{slug}"),
            model_session_id: format!("model-session-lane-{slug}"),
            ..locus_for(slug)
        }),
        idempotency_key: format!("idem-message-{slug}"),
        replay_order_key: "00000002/message".into(),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt-priv/msg-{slug}")),
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-schema#message-replay".into()),
        created_at_utc: "2026-06-28T22:31:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "WIRED",
            "internal_diagnostics": "DEFERRED: diagnostics surface MT-008",
            "palmistry": "DEFERRED: external watcher worktree"
        }),
    }
}

fn registration(artifact_byte: u8, base_model_tag: &str, registered_by: &str) -> ModelRegistration {
    ModelRegistration {
        model_id: ModelId::new_v7(),
        artifact_path: PathBuf::from(format!("models/{base_model_tag}.safetensors")),
        sha256: [artifact_byte; 32],
        runtime_binding: RegistryRuntimeBinding::Candle,
        declared_capabilities: ModelCapabilities {
            supports_lora: true,
            supports_activation_steering: true,
            supports_embedding: true,
            embedding_dimension: Some(768),
            ..Default::default()
        },
        base_model_tag: BaseModelTag::new(base_model_tag),
        registered_at_utc: Utc::now(),
        registered_by: RegistryOperatorId::new(registered_by),
        provider: RegistryProviderKind::Local,
    }
}
