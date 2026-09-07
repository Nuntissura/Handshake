#![allow(dead_code)]

use std::path::Path;

use handshake_core::kernel::sandbox::{policy::SandboxPolicyV1, run::SandboxRunV1};
use handshake_core::storage::kb003_storage::{Kb003Storage, ValidationRunRowV1};
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealKb003Storage, SurrealStorage, SurrealStorageConfig,
};
use surrealdb::types::Value;

pub async fn open(path: &Path) -> SurrealKb003Storage {
    let config = SurrealStorageConfig::for_data_dir(path).expect("configure isolated KB003 store");
    let storage = SurrealStorage::open(config)
        .await
        .expect("open isolated KB003 embedded store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap KB003 schema");
    SurrealKb003Storage::new(storage)
}

pub fn seed_validation_ancestors(
    store: &mut SurrealKb003Storage,
    run: &SandboxRunV1,
    validation: &ValidationRunRowV1,
) {
    let (policy_id, version) = run
        .policy_version_id
        .rsplit_once('@')
        .expect("versioned policy ID");
    let mut policy = SandboxPolicyV1::default_deny("KB003 fixture ancestors");
    policy.policy_id = policy_id.into();
    policy.policy_version = version.parse().expect("numeric policy version");
    assert_eq!(validation.sandbox_run_id, run.run_id.0);
    store
        .insert_sandbox_policy_version(&policy)
        .expect("persist policy ancestor");
    store
        .insert_sandbox_run(run)
        .expect("persist sandbox ancestor");
    store
        .insert_validation_run(validation)
        .expect("persist validation ancestor");
}

pub async fn reopen(store: SurrealKb003Storage, path: &Path) -> SurrealKb003Storage {
    query(
        &store,
        "REMOVE EVENT IF EXISTS mt141_decision_failure ON kb003_promotion_decisions;",
    )
    .await;
    query(
        &store,
        "REMOVE EVENT IF EXISTS mt141_receipt_failure ON kb003_promotion_receipts;",
    )
    .await;
    store
        .embedded_store()
        .shutdown()
        .await
        .expect("close KB003 store");
    drop(store);
    open(path).await
}

pub async fn refuse_decisions(store: &SurrealKb003Storage, variant: u8) {
    let statement = match variant {
        0 => "DEFINE EVENT mt141_decision_failure ON kb003_promotion_decisions THEN { THROW 'simulated storage deadlock during decision insert'; };",
        1 => "DEFINE EVENT mt141_decision_failure ON kb003_promotion_decisions THEN { THROW 'decision_table_offline'; };",
        2 => "DEFINE EVENT mt141_decision_failure ON kb003_promotion_decisions THEN { THROW 'deadlock detected on tx 991 at line 412'; };",
        3 => "DEFINE EVENT mt141_decision_failure ON kb003_promotion_decisions THEN { THROW 'Deadlock Detected on tx 1004 at line 538'; };",
        _ => panic!("unknown test failure variant"),
    };
    query(store, statement).await;
}

pub async fn refuse_receipts(store: &SurrealKb003Storage) {
    query(store, "DEFINE EVENT mt141_receipt_failure ON kb003_promotion_receipts THEN { THROW 'receipt_table_offline'; };").await;
}

pub async fn assert_authority_not_mutated(store: &SurrealKb003Storage) {
    assert_eq!(
        accepted_receipt_count(store).await,
        0,
        "rejection path persisted a receipt for an ACCEPTED decision"
    );
}

pub async fn accepted_receipt_count(store: &SurrealKb003Storage) -> usize {
    query(
        store,
        "SELECT * FROM kb003_promotion_receipts WHERE decision_id.decision = 'ACCEPTED';",
    )
    .await
    .len()
}
pub async fn query(store: &SurrealKb003Storage, statement: &'static str) -> Vec<Value> {
    store
        .embedded_store()
        .with_data_operation(move |context| {
            Box::pin(async move {
                context
                    .query_values::<Value, _>(
                        statement,
                        std::collections::BTreeMap::<String, Value>::new(),
                    )
                    .await
            })
        })
        .await
        .expect("query isolated KB003 store")
}

pub async fn decision_count(store: &SurrealKb003Storage) -> usize {
    query(store, "SELECT * FROM kb003_promotion_decisions;")
        .await
        .len()
}

pub async fn receipt_count(store: &SurrealKb003Storage) -> usize {
    query(store, "SELECT * FROM kb003_promotion_receipts;")
        .await
        .len()
}

pub async fn close(store: SurrealKb003Storage) {
    store
        .embedded_store()
        .shutdown()
        .await
        .expect("close KB003 store");
}
