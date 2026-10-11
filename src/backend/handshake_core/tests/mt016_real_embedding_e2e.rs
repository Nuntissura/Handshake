//! MT-016 production-boundary proof entrypoint.
//!
//! The prior target coupled the proof to relational model-registry and process-ledger
//! implementations. MT-016 now requires one injected embedded `SurrealStorage` for
//! model registration, Loom indexing, retrieval evidence, and EventLedger receipts.
//! Parfit owns the remaining boot/composition change, so this target stays explicitly
//! ignored-and-failing until that production constructor is available. A synthetic
//! client here would be a partial-green surrogate, not the required real-model proof.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "blocked on Parfit wiring the production model boot path to SurrealLoomSearchStore"]
async fn mt016_real_candle_embedding_restart_recovers_role_isolation_and_event_ledger() {
    panic!(
        "MT-016 runtime proof blocked: production boot must inject one cloned SurrealStorage into \
         SurrealLoomSearchStore::open, register the dedicated RoleBoundModelRegistration through \
         register_embedding_model, and pass the exact scope/store to Loom reindex and search"
    );
}
