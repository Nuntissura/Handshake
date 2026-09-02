mod surreal_test_store_support;

use handshake_core::storage::surreal::{RowFilter, SurrealUserManualKnowledgeStore};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use serde_json::{json, Value};
use surreal_test_store_support::EmbeddedSurrealTestScope;

const ENTITY_KEY: &str = "manual/runtime-recovery";

fn exact_scope(workspace_id: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(workspace_id).expect("valid workspace scope"),
    }
}

fn as_resource_scope(scope: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
        .with_session(scope.authenticated_session_id)
        .with_access_space(scope.access_space_id)
        .with_workspace(scope.workspace_id.clone())
}

fn provenance(revision: &str) -> Value {
    json!({
        "source": "user_manual",
        "manual_revision": revision,
        "route": "/usermanual/pages/runtime-recovery"
    })
}

async fn open_fixture() -> (
    EmbeddedSurrealTestScope,
    SurrealUserManualKnowledgeStore,
    ExactResourceScopeAttribution,
) {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated embedded store");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate production storage wrapper");
    let store = SurrealUserManualKnowledgeStore::open(storage)
        .await
        .expect("open UserManual knowledge provider");
    let scope = exact_scope("mt022-knowledge-workspace");
    store
        .ensure_workspace_fixture(scope.workspace_id.as_str())
        .await
        .expect("create scoped workspace fixture");
    (isolated, store, scope)
}

async fn durable_counts(store: &SurrealUserManualKnowledgeStore) -> (u64, u64) {
    let inspector = store.storage().test_inspector();
    let entities = inspector
        .table_selector("knowledge_entities")
        .await
        .expect("select knowledge entity table");
    let events = inspector
        .table_selector("kernel_event_ledger")
        .await
        .expect("select canonical event table");
    (
        inspector
            .row_count(&entities, RowFilter::All)
            .await
            .expect("count knowledge entities"),
        inspector
            .row_count(&events, RowFilter::All)
            .await
            .expect("count canonical events"),
    )
}

#[tokio::test]
async fn exact_scope_retry_and_predecessor_chain_survive_shutdown_and_reopen() {
    let (mut isolated, store, scope) = open_fixture().await;
    let namespace = isolated.namespace().to_owned();
    let database = isolated.database().to_owned();
    assert_eq!(store.storage().namespace(), namespace);
    assert_eq!(store.storage().database(), database);

    let first_a = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery A", provenance("A"))
        .await
        .expect("record first A");
    assert!(first_a.changed);
    assert_eq!(first_a.entity.workspace_id, scope.workspace_id.as_str());
    assert_eq!(first_a.entity.detection_provenance, provenance("A"));

    let first_a_retry = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery A", provenance("A"))
        .await
        .expect("retry first A");
    assert!(!first_a_retry.changed);
    assert_eq!(first_a_retry.entity, first_a.entity);
    assert_eq!(
        first_a_retry.event_ledger_event_id,
        first_a.event_ledger_event_id
    );

    let b = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery B", provenance("B"))
        .await
        .expect("record B after A");
    assert!(b.changed);
    assert_eq!(b.entity.entity_id, first_a.entity.entity_id);
    assert_eq!(b.entity.detection_provenance, provenance("B"));
    assert_ne!(b.event_ledger_event_id, first_a.event_ledger_event_id);

    let second_a = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery A", provenance("A"))
        .await
        .expect("record predecessor-bound second A");
    assert!(second_a.changed);
    assert_eq!(second_a.entity.entity_id, first_a.entity.entity_id);
    assert_eq!(second_a.entity.detection_provenance, provenance("A"));
    assert_ne!(
        second_a.event_ledger_event_id,
        first_a.event_ledger_event_id
    );
    assert_ne!(second_a.event_ledger_event_id, b.event_ledger_event_id);

    drop(store);
    isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close all shared storage clones");
    isolated
        .reopen()
        .await
        .expect("reopen exact allocated scope");
    let reopened_storage = isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace and database");
    assert_eq!(isolated.namespace(), namespace);
    assert_eq!(isolated.database(), database);
    assert_eq!(reopened_storage.namespace(), namespace);
    assert_eq!(reopened_storage.database(), database);
    let reopened = SurrealUserManualKnowledgeStore::open(reopened_storage)
        .await
        .expect("reopen provider over same storage");

    let persisted = reopened
        .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
        .await
        .expect("read persisted entity")
        .expect("entity survived restart");
    assert_eq!(persisted.entity, second_a.entity);
    assert_eq!(persisted.entity.detection_provenance, provenance("A"));
    assert_eq!(
        persisted.event_ledger_event_id,
        second_a.event_ledger_event_id
    );

    let retry_after_reopen = reopened
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery A", provenance("A"))
        .await
        .expect("retry after reopen");
    assert!(!retry_after_reopen.changed);
    assert_eq!(retry_after_reopen.entity, second_a.entity);
    assert_eq!(
        retry_after_reopen.event_ledger_event_id,
        second_a.event_ledger_event_id
    );

    drop(reopened);
    isolated.cleanup().await.expect("clean exact test scope");
}

#[tokio::test]
async fn each_scope_dimension_mismatch_and_mixed_or_incomplete_scope_fail_closed() {
    let (mut isolated, store, scope) = open_fixture().await;
    let created = store
        .upsert_user_manual_page_entity(
            &scope,
            ENTITY_KEY,
            "Runtime recovery",
            provenance("scope-baseline"),
        )
        .await
        .expect("record baseline entity");

    let mut owner_mismatch = scope.clone();
    owner_mismatch.owner_account_id = OwnerAccountId::mint();
    let mut actor_mismatch = scope.clone();
    actor_mismatch.actor_principal_id = ActorPrincipalId::mint();
    let mut session_mismatch = scope.clone();
    session_mismatch.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access_mismatch = scope.clone();
    access_mismatch.access_space_id = AccessSpaceRef::mint();
    let mut workspace_mismatch = scope.clone();
    workspace_mismatch.workspace_id =
        WorkspaceScopeRef::new("mt022-other-workspace").expect("valid alternate workspace");

    for (dimension, denied_scope) in [
        ("owner_account_id", owner_mismatch),
        ("actor_principal_id", actor_mismatch),
        ("authenticated_session_id", session_mismatch),
        ("access_space_id", access_mismatch),
        ("workspace_id", workspace_mismatch),
    ] {
        let denied = store
            .get_user_manual_page_entity_by_identity(&denied_scope, ENTITY_KEY)
            .await
            .unwrap_or_else(|error| {
                panic!("{dimension} mismatch query failed unexpectedly: {error}")
            });
        assert!(denied.is_none(), "{dimension} mismatch leaked the entity");
    }

    let source = as_resource_scope(&scope);
    let mut mixed = source.clone();
    mixed.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mixed_error =
        ResourceScope::derive_from_sources([&source, &mixed], scope.actor_principal_id)
            .expect_err("mixed source attribution must not derive a wider scope");
    assert!(mixed_error.to_string().contains("session"));

    for incomplete in [
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session_id),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session_id)
            .with_access_space(scope.access_space_id),
    ] {
        ExactResourceScopeAttribution::try_from_resource_scope(&incomplete)
            .expect_err("incomplete scope must not become durable attribution");
    }

    let unchanged = store
        .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
        .await
        .expect("read baseline after denials")
        .expect("baseline remains visible to exact scope");
    assert_eq!(unchanged.entity, created.entity);
    assert_eq!(
        unchanged.event_ledger_event_id,
        created.event_ledger_event_id
    );

    drop(store);
    isolated.cleanup().await.expect("clean exact test scope");
}

#[tokio::test]
async fn orphan_receipt_blocks_entity_mutation_without_disclosure() {
    let (mut isolated, store, scope) = open_fixture().await;
    let orphan_event_id = store
        .insert_orphan_receipt_fixture(&scope, ENTITY_KEY, "Runtime recovery", provenance("orphan"))
        .await
        .expect("insert controlled orphan receipt");
    assert!(!orphan_event_id.is_empty());

    for attempt in 0..2 {
        let error = store
            .upsert_user_manual_page_entity(
                &scope,
                ENTITY_KEY,
                "Runtime recovery",
                provenance("orphan"),
            )
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("receipt exists without its entity mutation"),
            "attempt {attempt} returned the wrong fail-closed error: {error}"
        );
        assert!(
            store
                .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
                .await
                .expect("read after denied orphan mutation")
                .is_none(),
            "attempt {attempt} materialized or disclosed an entity"
        );
    }

    drop(store);
    isolated.cleanup().await.expect("clean exact test scope");
}

#[tokio::test]
async fn stale_predecessor_race_rejects_changed_and_identical_targets_without_mutation() {
    let (mut isolated, store, scope) = open_fixture().await;
    let first = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery A", provenance("A"))
        .await
        .expect("record predecessor A");
    let current = store
        .upsert_user_manual_page_entity(&scope, ENTITY_KEY, "Runtime recovery B", provenance("B"))
        .await
        .expect("advance canonical predecessor to B");
    let before = durable_counts(&store).await;

    for (display_name, target) in [
        ("Runtime recovery C", provenance("C")),
        ("Runtime recovery B", provenance("B")),
    ] {
        let error = store
            .upsert_user_manual_page_entity_with_expected_predecessor(
                &scope,
                ENTITY_KEY,
                display_name,
                target,
                Some(&first.event_ledger_event_id),
            )
            .await
            .expect_err("stale predecessor must fail even for an otherwise identical retry");
        assert!(error.to_string().contains("predecessor changed"));
        assert!(!error
            .to_string()
            .contains(first.event_ledger_event_id.as_str()));
        assert_eq!(durable_counts(&store).await, before);
    }

    let persisted = store
        .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
        .await
        .expect("read canonical entity after stale denials")
        .expect("canonical entity remains present");
    assert_eq!(persisted.entity, current.entity);
    assert_eq!(
        persisted.event_ledger_event_id,
        current.event_ledger_event_id
    );

    let exact_retry = store
        .upsert_user_manual_page_entity_with_expected_predecessor(
            &scope,
            ENTITY_KEY,
            "Runtime recovery B",
            provenance("B"),
            Some(&current.event_ledger_event_id),
        )
        .await
        .expect("current predecessor permits identical retry");
    assert!(!exact_retry.changed);
    assert_eq!(
        exact_retry.event_ledger_event_id,
        current.event_ledger_event_id
    );
    assert_eq!(durable_counts(&store).await, before);

    drop(store);
    isolated.cleanup().await.expect("clean exact test scope");
}

#[tokio::test]
async fn ambiguous_or_mismatched_citations_fail_closed_without_mutation_or_leakage() {
    let (mut isolated, store, scope) = open_fixture().await;
    let primary = store
        .upsert_user_manual_page_entity(
            &scope,
            ENTITY_KEY,
            "Runtime recovery",
            provenance("primary"),
        )
        .await
        .expect("record primary citation target");
    let other_key = "manual/runtime-diagnostics";
    let secondary = store
        .upsert_user_manual_page_entity(
            &scope,
            other_key,
            "Runtime diagnostics",
            provenance("secondary"),
        )
        .await
        .expect("record independent citation target");

    let resolved = store
        .resolve_user_manual_page_entity_citation(
            &scope,
            &[ENTITY_KEY],
            &[&primary.event_ledger_event_id],
        )
        .await
        .expect("one exact entity and canonical receipt resolve");
    assert_eq!(resolved.entity, primary.entity);
    let before = durable_counts(&store).await;

    let errors = [
        store
            .resolve_user_manual_page_entity_citation(
                &scope,
                &[ENTITY_KEY, other_key],
                &[&primary.event_ledger_event_id],
            )
            .await
            .expect_err("multiple entity candidates must be ambiguous"),
        store
            .resolve_user_manual_page_entity_citation(
                &scope,
                &[ENTITY_KEY],
                &[
                    &primary.event_ledger_event_id,
                    &secondary.event_ledger_event_id,
                ],
            )
            .await
            .expect_err("multiple receipt candidates must be ambiguous"),
        store
            .resolve_user_manual_page_entity_citation(
                &scope,
                &[ENTITY_KEY],
                &[&secondary.event_ledger_event_id],
            )
            .await
            .expect_err("foreign canonical receipt must not bind to the primary entity"),
        store
            .resolve_user_manual_page_entity_citation(&scope, &[], &[])
            .await
            .expect_err("incomplete citation must fail closed"),
    ];
    for error in errors {
        let message = error.to_string();
        assert!(
            message.contains("ambiguous")
                || message.contains("requires one")
                || message.contains("does not match")
        );
        assert!(!message.contains(ENTITY_KEY));
        assert!(!message.contains(other_key));
        assert!(!message.contains(primary.event_ledger_event_id.as_str()));
        assert!(!message.contains(secondary.event_ledger_event_id.as_str()));
        assert_eq!(durable_counts(&store).await, before);
    }

    let canonical = store
        .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
        .await
        .expect("read primary after citation denials")
        .expect("primary remains present");
    assert_eq!(canonical.entity, primary.entity);
    assert_eq!(
        canonical.event_ledger_event_id,
        primary.event_ledger_event_id
    );

    drop(store);
    isolated.cleanup().await.expect("clean exact test scope");
}

#[tokio::test]
async fn mismatched_canonical_receipt_fails_closed_after_legitimate_entity_creation() {
    let (mut isolated, store, scope) = open_fixture().await;
    let created = store
        .upsert_user_manual_page_entity(
            &scope,
            ENTITY_KEY,
            "Runtime recovery",
            provenance("canonical-receipt"),
        )
        .await
        .expect("create legitimate entity and canonical receipt atomically");
    let before = durable_counts(&store).await;
    let mismatched_event_id = store
        .mismatch_canonical_receipt_fixture(&scope, ENTITY_KEY, "Tampered receipt display name")
        .await
        .expect("apply bounded negative receipt corruption");
    assert_eq!(mismatched_event_id, created.event_ledger_event_id);
    assert_eq!(durable_counts(&store).await, before);

    for error in [
        store
            .get_user_manual_page_entity_by_identity(&scope, ENTITY_KEY)
            .await
            .expect_err("mismatched canonical receipt must block direct read"),
        store
            .resolve_user_manual_page_entity_citation(
                &scope,
                &[ENTITY_KEY],
                &[&created.event_ledger_event_id],
            )
            .await
            .expect_err("mismatched canonical receipt must block citation resolution"),
    ] {
        let message = error.to_string();
        assert!(message.contains("inconsistent receipt evidence"));
        assert!(!message.contains(ENTITY_KEY));
        assert!(!message.contains(created.event_ledger_event_id.as_str()));
        assert_eq!(durable_counts(&store).await, before);
    }

    drop(store);
    isolated.cleanup().await.expect("clean exact test scope");
}
