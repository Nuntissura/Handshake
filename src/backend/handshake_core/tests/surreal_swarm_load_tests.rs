#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-142 "Harden SurrealDB swarm concurrency and load" - deterministic
//! mixed-workload engine against the real embedded RocksDB store, with an
//! in-memory oracle, exact canonical-state reconciliation after a real
//! shutdown + reopen, and the `hsk.surreal_swarm_load_report@1` emitter.
//!
//! Proves AC-142-3, AC-142-5 (end-to-end retry accounting), AC-142-6
//! (PT-142-3, PT-142-7):
//! * `ci_profile_16_workers_2000_operations_is_correct_and_bounded`
//! * `extended_profile_64_workers_50000_operations` (runs only with
//!   `HANDSHAKE_SWARM_EXTENDED=1`; otherwise prints
//!   `SWARM_EXTENDED=NOT_RUN_UNCONFIGURED` and returns - never `#[ignore]`)
//!
//! Seed: `HANDSHAKE_SWARM_SEED` (printed as `SWARM_SEED=`). Report path is
//! printed as `SWARM_LOAD_REPORT=<path>`; the report directory is
//! `HANDSHAKE_SWARM_LOAD_REPORT_DIR` else
//! `HANDSHAKE_ARTIFACTS_ROOT/handshake-test/swarm-load/`.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeRichDocument, KnowledgeStore, NewKnowledgeEntity,
};
use handshake_core::storage::surreal::swarm_load_report::{
    percentile_report_from_samples, DatasetCardinality, EngineMode, IntegrityVerdict,
    OperationClass, OperationMixEntry, OperationRunStatus, PercentileReport, Rate,
    RemoteProofStatus, SwarmLoadReport, REQUIRED_OPERATION_CLASSES,
    SWARM_LOAD_REPORT_SCHEMA_ID,
};
use handshake_core::storage::surreal::{
    RowFilter, SurrealDatabase, SurrealTestInspector, TableSelector,
};
use handshake_core::storage::{Database, LoomSearchFilters};
use serde_json::json;
use swarm_support::*;
use tokio::time::timeout;

/// Workload shares per operation class (every required class is non-zero).
const OPERATION_MIX: [(OperationClass, f64); 7] = [
    (OperationClass::PointRead, 0.30),
    (OperationClass::RangeOrSearchQuery, 0.10),
    (OperationClass::Create, 0.10),
    (OperationClass::IdempotentUpsert, 0.15),
    (OperationClass::OptimisticVersionedUpdate, 0.20),
    (OperationClass::Delete, 0.05),
    (OperationClass::MultiRecordProjectionOrLedgerTransaction, 0.10),
];

#[derive(Clone, Copy, Debug)]
struct WorkloadConfig {
    profile: &'static str,
    seed: u64,
    workers: u32,
    operations: u64,
    /// Documents seeded before the workload (dataset cardinality floor).
    seed_documents: u64,
    workspaces: u32,
    /// Share of the seeded documents forming the hot set (key skew).
    hot_set_fraction: f64,
    /// Probability that a keyed write targets the hot set.
    contention_ratio: f64,
    per_operation_timeout: Duration,
    per_worker_timeout: Duration,
    whole_test_timeout: Duration,
    /// Dedicated documents raced through shared idempotency keys.
    shared_idempotency_keys: u32,
    shared_entity_keys: u32,
    shared_titles: u32,
}

impl WorkloadConfig {
    fn ci(seed: u64) -> Self {
        Self {
            profile: "ci",
            seed,
            workers: 16,
            operations: 2_000,
            seed_documents: 240,
            workspaces: 4,
            hot_set_fraction: 0.02,
            contention_ratio: 0.25,
            per_operation_timeout: PER_OPERATION_TIMEOUT,
            per_worker_timeout: PER_WORKER_TIMEOUT,
            whole_test_timeout: WHOLE_TEST_TIMEOUT,
            shared_idempotency_keys: 24,
            shared_entity_keys: 32,
            shared_titles: 16,
        }
    }

    fn extended(seed: u64) -> Self {
        Self {
            profile: "extended",
            seed,
            workers: 64,
            operations: 50_000,
            seed_documents: 5_000,
            workspaces: 8,
            hot_set_fraction: 0.01,
            contention_ratio: 0.25,
            per_operation_timeout: PER_OPERATION_TIMEOUT,
            per_worker_timeout: Duration::from_millis(600_000),
            whole_test_timeout: Duration::from_millis(1_800_000),
            shared_idempotency_keys: 256,
            shared_entity_keys: 512,
            shared_titles: 128,
        }
    }

    fn hot_len(&self) -> usize {
        ((self.seed_documents as f64 * self.hot_set_fraction).round() as usize).max(1)
    }
}

fn pick_class(rng: &mut SwarmRng) -> OperationClass {
    let roll = rng.next_f64();
    let mut cumulative = 0.0;
    for (class, share) in OPERATION_MIX {
        cumulative += share;
        if roll < cumulative {
            return class;
        }
    }
    OPERATION_MIX[OPERATION_MIX.len() - 1].0
}

#[derive(Clone, Debug)]
struct DocRef {
    rich_document_id: String,
}

/// Live document pool: `hot` never shrinks (contention never dies), `cold`
/// grows with creates and shrinks with deletes.
struct DocPool {
    hot: Vec<DocRef>,
    cold: Mutex<Vec<DocRef>>,
}

impl DocPool {
    fn pick_hot(&self, rng: &mut SwarmRng) -> DocRef {
        self.hot[rng.below(self.hot.len())].clone()
    }

    fn pick_cold(&self, rng: &mut SwarmRng) -> Option<DocRef> {
        let cold = self.cold.lock().expect("cold pool");
        if cold.is_empty() {
            None
        } else {
            Some(cold[rng.below(cold.len())].clone())
        }
    }

    fn pick(&self, rng: &mut SwarmRng, contention_ratio: f64) -> DocRef {
        if rng.chance(contention_ratio) {
            self.pick_hot(rng)
        } else {
            self.pick_cold(rng).unwrap_or_else(|| self.pick_hot(rng))
        }
    }

    fn push_cold(&self, doc: DocRef) {
        self.cold.lock().expect("cold pool").push(doc);
    }

    /// Removes and returns a random cold document (reserved for deletion).
    fn take_cold(&self, rng: &mut SwarmRng) -> Option<DocRef> {
        let mut cold = self.cold.lock().expect("cold pool");
        if cold.is_empty() {
            return None;
        }
        let index = rng.below(cold.len());
        Some(cold.swap_remove(index))
    }

    fn remove(&self, rich_document_id: &str) {
        let mut cold = self.cold.lock().expect("cold pool");
        cold.retain(|doc| doc.rich_document_id != rich_document_id);
    }
}

struct Selectors {
    documents: TableSelector,
    loom_blocks: TableSelector,
    search_index: TableSelector,
}

struct Shared {
    config: WorkloadConfig,
    run_id: String,
    workspaces: Vec<String>,
    pool: DocPool,
    /// Dedicated documents for the shared idempotency keys (never in a pool).
    idempotency_targets: Vec<DocRef>,
    oracle: Mutex<Oracle>,
    gauge: InFlightGauge,
    db: SurrealDatabase,
    doc_api: DocApi,
    inspector: SurrealTestInspector,
    selectors: Selectors,
}

type Cache = HashMap<String, (i64, String)>;

struct Worker {
    shared: Arc<Shared>,
    id: u32,
    rng: SwarmRng,
    cache: Cache,
    metrics: SwarmMetrics,
    sub: u64,
}

impl Worker {
    fn oracle(&self) -> std::sync::MutexGuard<'_, Oracle> {
        self.shared.oracle.lock().expect("oracle")
    }

    fn per_op(&self) -> Duration {
        self.shared.config.per_operation_timeout
    }

    async fn head_version(&mut self, doc: &DocRef) -> Result<Option<(i64, String)>, OpOutcome> {
        if let Some(cached) = self.cache.get(&doc.rich_document_id) {
            return Ok(Some(cached.clone()));
        }
        let read = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document(&doc.rich_document_id),
        )
        .await;
        match read {
            Err(_) => Err(OpOutcome::Timeout),
            Ok(Err(error)) => Err(OpOutcome::from_error(&error)),
            Ok(Ok(None)) => {
                self.oracle().note_missing_read(&doc.rich_document_id);
                self.shared.pool.remove(&doc.rich_document_id);
                Ok(None)
            }
            Ok(Ok(Some(document))) => {
                let head = (document.doc_version, document.content_sha256.clone());
                self.cache
                    .insert(doc.rich_document_id.clone(), head.clone());
                Ok(Some(head))
            }
        }
    }

    fn note_saved(&mut self, document: &KnowledgeRichDocument) {
        self.cache.insert(
            document.rich_document_id.clone(),
            (document.doc_version, document.content_sha256.clone()),
        );
        self.oracle().ack_save(document);
    }

    async fn point_read(&mut self) -> OpOutcome {
        let doc = self
            .shared
            .pool
            .pick(&mut self.rng, self.shared.config.contention_ratio);
        let read = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document(&doc.rich_document_id),
        )
        .await;
        let document = match read {
            Err(_) => return OpOutcome::Timeout,
            Ok(Err(error)) => return OpOutcome::from_error(&error),
            Ok(Ok(None)) => {
                self.oracle().note_missing_read(&doc.rich_document_id);
                self.cache.remove(&doc.rich_document_id);
                self.shared.pool.remove(&doc.rich_document_id);
                return OpOutcome::Ok;
            }
            Ok(Ok(Some(document))) => document,
        };
        // Dirty-read proof: the observed head must have a committed version row.
        let row = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document_version(&document.rich_document_id, document.doc_version),
        )
        .await;
        match row {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(row)) => {
                let matches = row.is_some_and(|row| row.content_sha256 == document.content_sha256);
                self.oracle().note_read(&document, matches);
                self.cache.insert(
                    document.rich_document_id.clone(),
                    (document.doc_version, document.content_sha256.clone()),
                );
                OpOutcome::Ok
            }
        }
    }

    async fn range_or_search(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        self.sub += 1;
        if self.sub % 2 == 0 {
            let listed = timeout(
                self.per_op(),
                self.shared
                    .db
                    .list_knowledge_rich_documents(&workspace, None, None),
            )
            .await;
            match listed {
                Err(_) => OpOutcome::Timeout,
                Ok(Err(error)) => OpOutcome::from_error(&error),
                Ok(Ok(documents)) => {
                    for document in &documents {
                        assert!(
                            document.doc_version >= 1,
                            "range query returned an unversioned document {}",
                            document.rich_document_id
                        );
                    }
                    // Dirty-read sample: the first listed head must be committed.
                    if let Some(document) = documents.first() {
                        let row = timeout(
                            self.per_op(),
                            self.shared.db.get_knowledge_rich_document_version(
                                &document.rich_document_id,
                                document.doc_version,
                            ),
                        )
                        .await;
                        match row {
                            Err(_) => return OpOutcome::Timeout,
                            Ok(Err(error)) => return OpOutcome::from_error(&error),
                            Ok(Ok(row)) => {
                                let matches = row
                                    .is_some_and(|row| row.content_sha256 == document.content_sha256);
                                self.oracle().note_read(document, matches);
                            }
                        }
                    }
                    OpOutcome::Ok
                }
            }
        } else {
            let searched = timeout(
                self.per_op(),
                self.shared.db.search_loom_blocks(
                    &workspace,
                    "swarm",
                    LoomSearchFilters::default(),
                    25,
                    0,
                ),
            )
            .await;
            match searched {
                Err(_) => OpOutcome::Timeout,
                Ok(Err(error)) => OpOutcome::from_error(&error),
                Ok(Ok(_results)) => OpOutcome::Ok,
            }
        }
    }

    async fn create(&mut self, operation: u64) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let title = format!("swarm create w{} op{operation} {}", self.id, self.shared.run_id);
        let created = timeout(
            self.per_op(),
            self.shared.db.create_knowledge_rich_document(new_document(
                &workspace,
                &title,
                &format!("created by worker {} op {operation}", self.id),
            )),
        )
        .await;
        match created {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(document)) => {
                self.oracle().ack_create(&document);
                self.cache.insert(
                    document.rich_document_id.clone(),
                    (document.doc_version, document.content_sha256.clone()),
                );
                self.shared.pool.push_cold(DocRef {
                    rich_document_id: document.rich_document_id,
                });
                OpOutcome::Ok
            }
        }
    }

    /// Three idempotent sub-kinds in rotation: keyed save (private key with an
    /// immediate replay, or a shared key raced across workers), natural-key
    /// entity upsert, natural-key title create.
    async fn idempotent_upsert(&mut self, operation: u64) -> OpOutcome {
        self.sub += 1;
        match self.sub % 3 {
            0 => self.idempotent_save(operation).await,
            1 => self.entity_upsert().await,
            _ => self.title_create().await,
        }
    }

    async fn idempotent_save(&mut self, operation: u64) -> OpOutcome {
        let shared_key = self.rng.chance(0.5);
        let (key, doc, expected_version, payload) = if shared_key {
            let index = self.rng.below(self.shared.idempotency_targets.len());
            let doc = self.shared.idempotency_targets[index].clone();
            (
                format!("{}-shared-{index}", self.shared.run_id),
                doc,
                1i64,
                document_content(&format!("shared idempotent effect {index}")),
            )
        } else {
            let doc = self
                .shared
                .pool
                .pick(&mut self.rng, self.shared.config.contention_ratio);
            let Some((version, _)) = (match self.head_version(&doc).await {
                Ok(head) => head,
                Err(outcome) => return outcome,
            }) else {
                return OpOutcome::NotFound("document vanished before idempotent save".to_owned());
            };
            (
                format!("{}-w{}-op{operation}", self.shared.run_id, self.id),
                doc,
                version,
                document_content(&format!("private idempotent effect w{} op{operation}", self.id)),
            )
        };
        let first = self.idempotent_call(&key, &doc, expected_version, payload.clone()).await;
        if first != OpOutcome::Ok || shared_key {
            return first;
        }
        // Immediate replay of a private key: must converge to the same effect.
        let started = Instant::now();
        let replay = self.idempotent_call(&key, &doc, expected_version, payload).await;
        let latency = started.elapsed();
        self.metrics.record(
            OperationClass::IdempotentUpsert,
            &replay,
            latency,
            self.id,
            operation,
        );
        if replay != OpOutcome::Ok {
            self.oracle().violations.push(Violation {
                class: IntegrityVerdict::DuplicateEffect,
                detail: format!("immediate replay of idempotency key {key} did not converge: {replay:?}"),
            });
        }
        first
    }

    async fn idempotent_call(
        &mut self,
        key: &str,
        doc: &DocRef,
        expected_version: i64,
        payload: serde_json::Value,
    ) -> OpOutcome {
        let saved = timeout(
            self.per_op(),
            self.shared.db.save_knowledge_rich_document_version_idempotent(
                key,
                &doc.rich_document_id,
                expected_version,
                payload,
                None,
                None,
                None,
            ),
        )
        .await;
        match saved {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => {
                let outcome = OpOutcome::from_error(&error);
                if matches!(outcome, OpOutcome::TypedConflict(_)) {
                    self.cache.remove(&doc.rich_document_id);
                }
                if matches!(outcome, OpOutcome::NotFound(_)) {
                    self.oracle().note_missing_read(&doc.rich_document_id);
                    self.shared.pool.remove(&doc.rich_document_id);
                }
                outcome
            }
            Ok(Ok(write)) => {
                self.oracle()
                    .ack_idempotent(key, &write.value, write.replayed);
                self.cache.insert(
                    write.value.rich_document_id.clone(),
                    (write.value.doc_version, write.value.content_sha256.clone()),
                );
                OpOutcome::Ok
            }
        }
    }

    async fn entity_upsert(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let key = format!(
            "swarm-entity-{}",
            self.rng.below(self.shared.config.shared_entity_keys as usize)
        );
        let upserted = timeout(
            self.per_op(),
            self.shared.db.upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace.clone(),
                entity_kind: KnowledgeEntityKind::Concept,
                entity_key: key.clone(),
                display_name: format!("Swarm entity {key}"),
                detection_provenance: json!({ "source": "mt142-swarm", "worker": self.id }),
                primary_source_id: None,
                detected_in_run: None,
                evidence_span_ids: Vec::new(),
            }),
        )
        .await;
        match upserted {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(entity)) => {
                self.oracle().note_natural_key(
                    NaturalKeyKind::Entity,
                    &workspace,
                    &key,
                    &entity.entity_id,
                    false,
                );
                OpOutcome::Ok
            }
        }
    }

    async fn title_create(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let title = format!(
            "Swarm Shared Title {}",
            self.rng.below(self.shared.config.shared_titles as usize)
        );
        let created = timeout(
            self.per_op(),
            self.shared
                .db
                .create_knowledge_rich_document_if_title_absent(new_document(
                    &workspace,
                    &title,
                    &format!("title natural key {title}"),
                )),
        )
        .await;
        match created {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok((document, created))) => {
                let mut oracle = self.oracle();
                if created {
                    oracle.ack_create(&document);
                }
                oracle.note_natural_key(
                    NaturalKeyKind::Title,
                    &workspace,
                    &title,
                    &document.rich_document_id,
                    created,
                );
                OpOutcome::Ok
            }
        }
    }

    /// Compare-and-set save with the expected version taken from the worker's
    /// cache, so contention produces real typed stale outcomes.
    async fn optimistic_update(&mut self, operation: u64) -> OpOutcome {
        let doc = self
            .shared
            .pool
            .pick(&mut self.rng, self.shared.config.contention_ratio);
        let Some((expected_version, _)) = (match self.head_version(&doc).await {
            Ok(head) => head,
            Err(outcome) => return outcome,
        }) else {
            return OpOutcome::NotFound("document vanished before optimistic update".to_owned());
        };
        self.save(&doc, expected_version, &format!("optimistic w{} op{operation}", self.id))
            .await
    }

    async fn save(&mut self, doc: &DocRef, expected_version: i64, text: &str) -> OpOutcome {
        let saved = timeout(
            self.per_op(),
            self.shared.db.save_knowledge_rich_document_version(
                &doc.rich_document_id,
                expected_version,
                document_content(text),
                None,
                None,
                None,
            ),
        )
        .await;
        match saved {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => {
                let outcome = OpOutcome::from_error(&error);
                match &outcome {
                    OpOutcome::TypedConflict(_) | OpOutcome::UntypedEngineConflict(_) => {
                        self.cache.remove(&doc.rich_document_id);
                    }
                    OpOutcome::NotFound(_) => {
                        self.oracle().note_missing_read(&doc.rich_document_id);
                        self.cache.remove(&doc.rich_document_id);
                        self.shared.pool.remove(&doc.rich_document_id);
                    }
                    _ => {}
                }
                outcome
            }
            Ok(Ok(document)) => {
                self.note_saved(&document);
                OpOutcome::Ok
            }
        }
    }

    async fn delete(&mut self, operation: u64) -> OpOutcome {
        let Some(doc) = self.shared.pool.take_cold(&mut self.rng) else {
            // The hot set is never deleted; with no cold document left this
            // delete attempt fails typed (counted under Delete, no violation).
            return OpOutcome::NotFound("no cold document is available to delete".to_owned());
        };
        self.oracle().mark_delete_pending(&doc.rich_document_id);
        self.cache.remove(&doc.rich_document_id);
        let deleted = timeout(
            self.per_op(),
            self.shared
                .doc_api
                .delete_document(&doc.rich_document_id, &format!("w{}-op{operation}", self.id)),
        )
        .await;
        match deleted {
            Err(_) => OpOutcome::Timeout,
            Ok(Ok(ack)) => {
                self.oracle()
                    .ack_delete(&doc.rich_document_id, &ack.receipt_event_id);
                OpOutcome::Ok
            }
            Ok(Err(DeleteFailure::Conflict(text))) => OpOutcome::TypedConflict(text),
            Ok(Err(DeleteFailure::NotFound(text))) => OpOutcome::NotFound(text),
            Ok(Err(DeleteFailure::Other(status, text))) => {
                OpOutcome::Terminal(format!("delete HTTP {status}: {text}"))
            }
            Ok(Err(DeleteFailure::Transport(text))) => {
                OpOutcome::Terminal(format!("delete transport: {text}"))
            }
        }
    }

    /// A create or save whose storage transaction touches the document, its
    /// version row, its loom block and its search-index row; atomicity is
    /// verified by reading every touched table afterwards.
    async fn multi_record_transaction(&mut self, operation: u64) -> OpOutcome {
        self.sub += 1;
        let (outcome, target): (OpOutcome, Option<(String, i64, String)>) = if self.sub % 2 == 0 {
            let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
            let title = format!("swarm multi w{} op{operation} {}", self.id, self.shared.run_id);
            let created = timeout(
                self.per_op(),
                self.shared.db.create_knowledge_rich_document(new_document(
                    &workspace,
                    &title,
                    &format!("multi-record create w{} op{operation}", self.id),
                )),
            )
            .await;
            match created {
                Err(_) => (OpOutcome::Timeout, None),
                Ok(Err(error)) => (OpOutcome::from_error(&error), None),
                Ok(Ok(document)) => {
                    self.oracle().ack_create(&document);
                    self.cache.insert(
                        document.rich_document_id.clone(),
                        (document.doc_version, document.content_sha256.clone()),
                    );
                    self.shared.pool.push_cold(DocRef {
                        rich_document_id: document.rich_document_id.clone(),
                    });
                    (
                        OpOutcome::Ok,
                        Some((
                            document.rich_document_id,
                            document.doc_version,
                            document.content_sha256,
                        )),
                    )
                }
            }
        } else {
            let doc = self
                .shared
                .pool
                .pick(&mut self.rng, self.shared.config.contention_ratio);
            let Some((expected_version, _)) = (match self.head_version(&doc).await {
                Ok(head) => head,
                Err(outcome) => return outcome,
            }) else {
                return OpOutcome::NotFound("document vanished before multi-record save".to_owned());
            };
            let outcome = self
                .save(&doc, expected_version, &format!("multi-record save w{} op{operation}", self.id))
                .await;
            let target = (outcome == OpOutcome::Ok)
                .then(|| self.cache.get(&doc.rich_document_id).cloned())
                .flatten()
                .map(|(version, sha)| (doc.rich_document_id.clone(), version, sha));
            (outcome, target)
        };
        let Some((rich_document_id, doc_version, content_sha256)) = target else {
            return outcome;
        };
        // Atomicity proof: every projected row of the committed transaction.
        let checks = timeout(self.per_op(), async {
            let inspector = &self.shared.inspector;
            let selectors = &self.shared.selectors;
            let doc_row = inspector
                .exists(&selectors.documents, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let loom_row = inspector
                .exists(&selectors.loom_blocks, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let search_row = inspector
                .exists(&selectors.search_index, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let version_row = self
                .shared
                .db
                .get_knowledge_rich_document_version(&rich_document_id, doc_version)
                .await
                .map_err(|error| error.to_string())?
                .is_some_and(|row| row.content_sha256 == content_sha256);
            Ok::<_, String>((doc_row, loom_row, search_row, version_row))
        })
        .await;
        match checks {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::Terminal(format!("multi-record verification failed: {error}")),
            Ok(Ok((doc_row, loom_row, search_row, version_row))) => {
                if !(doc_row && loom_row && search_row && version_row) {
                    self.oracle().violations.push(Violation {
                        class: IntegrityVerdict::PartialCommit,
                        detail: format!(
                            "multi-record transaction on {rich_document_id} v{doc_version} left a partial projection: document={doc_row} loom_block={loom_row} search_index={search_row} version_row={version_row}"
                        ),
                    });
                }
                OpOutcome::Ok
            }
        }
    }

    async fn run(mut self, operations: u64) -> SwarmMetrics {
        for operation in 0..operations {
            let class = pick_class(&mut self.rng);
            let shared = Arc::clone(&self.shared);
            let _in_flight = shared.gauge.enter();
            let started = Instant::now();
            let outcome = match class {
                OperationClass::PointRead => self.point_read().await,
                OperationClass::RangeOrSearchQuery => self.range_or_search().await,
                OperationClass::Create => self.create(operation).await,
                OperationClass::IdempotentUpsert => self.idempotent_upsert(operation).await,
                OperationClass::OptimisticVersionedUpdate => self.optimistic_update(operation).await,
                OperationClass::Delete => self.delete(operation).await,
                OperationClass::MultiRecordProjectionOrLedgerTransaction => {
                    self.multi_record_transaction(operation).await
                }
            };
            self.metrics
                .record(class, &outcome, started.elapsed(), self.id, operation);
            if outcome == OpOutcome::Timeout {
                // A timeout is a failure; stop this worker so the report names
                // the class instead of cascading further timeouts.
                break;
            }
        }
        self.metrics
    }
}

struct ProfileOutcome {
    report: SwarmLoadReport,
    report_path: std::path::PathBuf,
    integrity: IntegrityOutcome,
    metrics: SwarmMetrics,
    worker_timeouts: Vec<String>,
    shutdown_elapsed: Duration,
    seeded_documents: u64,
}

async fn seed_documents(
    db: &SurrealDatabase,
    workspaces: &[String],
    count: u64,
    parallelism: u32,
    prefix: &str,
    per_operation_timeout: Duration,
) -> Vec<(KnowledgeRichDocument, String)> {
    let mut tasks = Vec::new();
    for lane in 0..parallelism as u64 {
        let db = db.clone();
        let workspaces = workspaces.to_vec();
        let prefix = prefix.to_owned();
        tasks.push(tokio::spawn(async move {
            let mut created = Vec::new();
            let mut index = lane;
            while index < count {
                let workspace = workspaces[(index % workspaces.len() as u64) as usize].clone();
                let document = timeout(
                    per_operation_timeout,
                    db.create_knowledge_rich_document(new_document(
                        &workspace,
                        &format!("{prefix} {index}"),
                        &format!("{prefix} base {index}"),
                    )),
                )
                .await
                .unwrap_or_else(|_| panic!("seeding document {index} exceeded its per-operation bound"))
                .unwrap_or_else(|error| panic!("seeding document {index} failed: {error}"));
                created.push((document, workspace));
                index += parallelism as u64;
            }
            created
        }));
    }
    let mut documents = Vec::new();
    for task in tasks {
        documents.extend(task.await.expect("seed lane joined"));
    }
    documents.sort_by(|a, b| a.0.title.cmp(&b.0.title));
    documents
}

async fn run_profile(config: WorkloadConfig) -> ProfileOutcome {
    let diagnostics_owned = install_retry_diagnostics();
    let retry_before = retry_diagnostics_snapshot();
    let run_id = new_run_id(&format!("mt142-{}", config.profile));
    println!(
        "SWARM_SEED={} SWARM_RUN_ID={run_id} SWARM_PROFILE={} workers={} operations={} retry_diagnostics_owned={diagnostics_owned}",
        config.seed, config.profile, config.workers, config.operations
    );

    let store = open_embedded_store()
        .await
        .expect("MT-142 requires the embedded SurrealDB test store");
    let mut workspaces = Vec::with_capacity(config.workspaces as usize);
    for _ in 0..config.workspaces {
        workspaces.push(store.create_workspace().await);
    }
    let inspector = store.storage.test_inspector();
    let baseline = table_counts(&inspector).await;
    let selectors = Selectors {
        documents: inspector
            .table_selector("knowledge_rich_documents")
            .await
            .expect("documents selector"),
        loom_blocks: inspector
            .table_selector("loom_blocks")
            .await
            .expect("loom_blocks selector"),
        search_index: inspector
            .table_selector("loom_block_search_index")
            .await
            .expect("search index selector"),
    };
    let doc_api = DocApi::boot(&store).await;

    // Dataset seeding (parallel lanes, deterministic titles).
    let seeding_started = Instant::now();
    let seeded = seed_documents(
        &store.db,
        &workspaces,
        config.seed_documents,
        config.workers.min(16),
        "swarm seed",
        config.per_operation_timeout,
    )
    .await;
    let idempotency_seeds = seed_documents(
        &store.db,
        &workspaces,
        u64::from(config.shared_idempotency_keys),
        config.workers.min(16),
        "swarm idem",
        config.per_operation_timeout,
    )
    .await;
    println!(
        "SWARM_SEEDED documents={} idempotency_targets={} elapsed_ms={}",
        seeded.len(),
        idempotency_seeds.len(),
        seeding_started.elapsed().as_millis()
    );
    let mut oracle = Oracle::default();
    for (document, _) in seeded.iter().chain(idempotency_seeds.iter()) {
        oracle.ack_create(document);
    }
    let hot_len = config.hot_len().min(seeded.len());
    let refs: Vec<DocRef> = seeded
        .iter()
        .map(|(document, _)| DocRef {
            rich_document_id: document.rich_document_id.clone(),
        })
        .collect();
    let pool = DocPool {
        hot: refs[..hot_len].to_vec(),
        cold: Mutex::new(refs[hot_len..].to_vec()),
    };
    let idempotency_targets: Vec<DocRef> = idempotency_seeds
        .iter()
        .map(|(document, _)| DocRef {
            rich_document_id: document.rich_document_id.clone(),
        })
        .collect();
    let seeded_documents = (seeded.len() + idempotency_seeds.len()) as u64;

    let shared = Arc::new(Shared {
        config,
        run_id: run_id.clone(),
        workspaces: workspaces.clone(),
        pool,
        idempotency_targets,
        oracle: Mutex::new(oracle),
        gauge: InFlightGauge::default(),
        db: store.db.clone(),
        doc_api,
        inspector: inspector.clone(),
        selectors,
    });

    // Workload.
    let per_worker = config.operations / u64::from(config.workers);
    let remainder = config.operations % u64::from(config.workers);
    let workload_started = Instant::now();
    let mut tasks = Vec::with_capacity(config.workers as usize);
    for worker_id in 0..config.workers {
        let operations = per_worker + u64::from(u64::from(worker_id) < remainder);
        let worker = Worker {
            shared: Arc::clone(&shared),
            id: worker_id,
            rng: SwarmRng::derive(config.seed, u64::from(worker_id)),
            cache: HashMap::new(),
            metrics: SwarmMetrics::default(),
            sub: u64::from(worker_id),
        };
        let bound = config.per_worker_timeout;
        tasks.push(tokio::spawn(async move {
            match timeout(bound, worker.run(operations)).await {
                Ok(metrics) => Ok(metrics),
                Err(_) => Err(format!(
                    "worker {worker_id} exceeded its per-worker bound of {} ms",
                    bound.as_millis()
                )),
            }
        }));
    }
    let mut metrics = SwarmMetrics::default();
    let mut worker_timeouts = Vec::new();
    for task in tasks {
        match task.await.expect("worker task joined") {
            Ok(worker_metrics) => metrics.merge(worker_metrics),
            Err(timeout_text) => worker_timeouts.push(timeout_text),
        }
    }
    let workload_elapsed = workload_started.elapsed();
    let retry_after = retry_diagnostics_snapshot().delta_since(&retry_before);
    println!(
        "SWARM_WORKLOAD attempted={} succeeded={} conflicts={} untyped_conflicts={} retries={} retry_exhaustions={} timeouts={} cancellations={} max_in_flight={} elapsed_ms={}",
        metrics.attempted_total(),
        metrics.succeeded_total(),
        metrics.conflicts,
        metrics.untyped_conflicts.len(),
        retry_after.scheduled,
        retry_after.exhausted,
        metrics.timeouts,
        metrics.cancellations,
        shared.gauge.high_water(),
        workload_elapsed.as_millis()
    );

    // Graceful shutdown of the one owning engine, then a real reopen.
    let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("every worker released the shared state"));
    shared.doc_api.shutdown().await;
    let oracle = shared.oracle.into_inner().expect("oracle");
    let max_in_flight = shared.gauge.high_water();
    let shutdown_started = Instant::now();
    let shutdown_wait = store.storage.config().shutdown_wait();
    let shutdown_report = store
        .storage
        .shutdown_with_report()
        .await
        .expect("graceful shutdown of the embedded engine must succeed");
    let shutdown_elapsed = shutdown_started.elapsed();
    println!(
        "SWARM_SHUTDOWN elapsed_ms={} report={shutdown_report:?} bound_ms={}",
        shutdown_elapsed.as_millis(),
        shutdown_wait.as_millis()
    );
    assert!(
        !store.storage.is_accepting_operations(),
        "shutdown must stop admission"
    );
    let reopened = store
        .reopen_database()
        .await
        .expect("reopen the same data_dir after shutdown");
    let reopened_inspector = reopened.storage().test_inspector();
    let reconcile_started = Instant::now();
    let integrity = reconcile(&reopened, &reopened_inspector, &oracle, &baseline).await;
    println!(
        "SWARM_INTEGRITY verdict={:?} violations={} documents_checked={} versions_checked={} elapsed_ms={}",
        integrity.verdict,
        integrity.violations.len(),
        integrity.documents_checked,
        integrity.versions_checked,
        reconcile_started.elapsed().as_millis()
    );
    let store_path = reopened.storage().config().path().to_path_buf();
    reopened
        .storage()
        .shutdown()
        .await
        .expect("shutdown of the reopened engine");
    drop(reopened_inspector);
    drop(reopened);
    store.close_and_remove().await.expect("close and remove the store");

    // Report.
    let attempted_total = metrics.attempted_total().max(1);
    let mut operation_mix = BTreeMap::new();
    let mut attempted_by_operation = BTreeMap::new();
    let mut succeeded_by_operation = BTreeMap::new();
    let mut failed_by_operation_and_class = BTreeMap::new();
    let mut latency_by_operation = BTreeMap::new();
    for (class, share) in OPERATION_MIX {
        let class_metrics = metrics.by_class.get(&class);
        let attempted = class_metrics.map(|m| m.attempted).unwrap_or(0);
        operation_mix.insert(
            class,
            OperationMixEntry {
                share,
                status: if attempted > 0 {
                    OperationRunStatus::Run
                } else {
                    OperationRunStatus::NotRun
                },
            },
        );
        attempted_by_operation.insert(class, attempted);
        succeeded_by_operation.insert(class, class_metrics.map(|m| m.succeeded).unwrap_or(0));
        if let Some(m) = class_metrics {
            if !m.failed.is_empty() {
                failed_by_operation_and_class.insert(class, m.failed.clone());
            }
        }
        let mut samples: Vec<f64> = class_metrics.map(|m| m.latency_ms.clone()).unwrap_or_default();
        latency_by_operation.insert(class, percentile_report_from_samples(&mut samples));
    }
    let report = SwarmLoadReport {
        schema_id: SWARM_LOAD_REPORT_SCHEMA_ID.to_owned(),
        run_id: run_id.clone(),
        source_commit: source_commit(),
        surrealdb_version: SURREALDB_VERSION.to_owned(),
        sdk_version: SURREALDB_VERSION.to_owned(),
        engine_mode: EngineMode::EmbeddedRocksDb,
        workload_seed: config.seed,
        worker_count: config.workers,
        operation_count: metrics.attempted_total(),
        dataset_cardinality: DatasetCardinality {
            records: oracle.docs.len() as u64,
            workspaces: config.workspaces,
        },
        operation_mix,
        contention_ratio: config.contention_ratio,
        attempted_by_operation,
        succeeded_by_operation,
        failed_by_operation_and_class,
        conflict_count: metrics.conflicts,
        conflict_rate: Rate::new(metrics.conflicts, attempted_total).expect("non-zero denominator"),
        retry_count: retry_after.scheduled,
        retry_rate: Rate::new(retry_after.scheduled, attempted_total).expect("non-zero denominator"),
        retry_exhaustion_count: retry_after.exhausted,
        // No product hook exposes keyed-lock wait samples to the test yet;
        // an empty class is NOT_RUN, never a zero-latency PASS.
        lock_wait_ms_p50_p95_p99: PercentileReport::NotRun,
        latency_ms_p50_p95_p99_by_operation: latency_by_operation,
        throughput_operations_per_second: metrics.attempted_total() as f64
            / workload_elapsed.as_secs_f64().max(f64::EPSILON),
        maximum_concurrent_operations: max_in_flight as u32,
        timeout_count: metrics.timeouts + worker_timeouts.len() as u64,
        cancellation_count: metrics.cancellations,
        shutdown_elapsed_ms: shutdown_elapsed.as_millis() as u64,
        reopen_integrity_counts_and_hashes: integrity.counts_and_hashes.clone(),
        integrity_verdict: if !worker_timeouts.is_empty() || metrics.timeouts > 0 {
            IntegrityVerdict::Timeout
        } else if !metrics.retry_exhausted_errors.is_empty() {
            IntegrityVerdict::RetryExhausted
        } else {
            integrity.verdict
        },
        remote_proof_status: RemoteProofStatus::NotRunUnconfigured,
        machine_context: machine_context(&store_path),
    };
    let validation = report.validate();
    let mut value = serde_json::to_value(&report).expect("report serializes");
    value["validation"] = match &validation {
        Ok(()) => json!("ok"),
        Err(problems) => json!(problems),
    };
    value["diagnostics"] = json!({
        "retry_diagnostics_owned_by_this_process": diagnostics_owned,
        "untyped_engine_conflicts": metrics.untyped_conflicts.clone(),
        "retry_exhausted_errors": metrics.retry_exhausted_errors.clone(),
        "unexpected_terminal_errors": metrics.unexpected_terminal.iter().take(50).collect::<Vec<_>>(),
        "timed_out_operations": metrics.timed_out_classes.clone(),
        "worker_timeouts": worker_timeouts.clone(),
        "lock_wait_timeouts": metrics.lock_wait_timeouts.clone(),
        "shutdown_report": {
            "drained": shutdown_report.drained,
            "cancelled": shutdown_report.cancelled,
            "engine_elapsed_ms": shutdown_report.elapsed.as_millis() as u64,
            "shutdown_wait_bound_ms": shutdown_wait.as_millis() as u64,
        },
        "lock_wait_samples_note": "keyed-lock waits happen inside the store (guarded_mutation); no product hook exports per-operation lock_wait samples, so lock_wait_ms_p50_p95_p99 is not_run",
        "integrity_violations": integrity.violations.iter().take(100).map(Violation::render).collect::<Vec<_>>(),
        "seeded_documents": seeded_documents,
        "dirty_read_checks": oracle.dirty_read_checks,
        "acknowledged_writes": oracle.acknowledged_writes,
        "workload_elapsed_ms": workload_elapsed.as_millis() as u64,
    });
    let report_path = write_report_json(&format!("swarm-load-{}-{run_id}.json", config.profile), &value);
    println!("SWARM_LOAD_REPORT={}", report_path.display());
    assert_eq!(
        validation,
        Ok(()),
        "hsk.surreal_swarm_load_report@1 must validate"
    );

    ProfileOutcome {
        report,
        report_path,
        integrity,
        metrics,
        worker_timeouts,
        shutdown_elapsed,
        seeded_documents,
    }
}

fn assert_profile(outcome: &ProfileOutcome, config: &WorkloadConfig) {
    let report = &outcome.report;
    assert!(
        outcome.worker_timeouts.is_empty(),
        "no worker may exceed its bound: {:?}",
        outcome.worker_timeouts
    );
    assert_eq!(
        outcome.metrics.timeouts, 0,
        "no operation may exceed the {} ms per-operation bound; timed out: {:?}",
        config.per_operation_timeout.as_millis(),
        outcome.metrics.timed_out_classes
    );
    assert_eq!(report.cancellation_count, 0, "no operation may be cancelled during the workload");
    assert!(
        report.operation_count >= config.operations,
        "the profile must attempt at least {} operations, attempted {}",
        config.operations,
        report.operation_count
    );
    assert!(
        report.worker_count >= config.workers,
        "the profile must run at least {} workers",
        config.workers
    );
    for class in REQUIRED_OPERATION_CLASSES {
        let attempted = report.attempted_by_operation.get(&class).copied().unwrap_or(0);
        assert!(
            attempted > 0,
            "required operation class {class:?} was never attempted (zero-class green is forbidden)"
        );
        let succeeded = report.succeeded_by_operation.get(&class).copied().unwrap_or(0);
        assert!(
            succeeded > 0,
            "required operation class {class:?} never succeeded ({attempted} attempted)"
        );
    }
    assert!(
        outcome.metrics.untyped_conflicts.is_empty(),
        "every same-record loser must be a typed stale/conflict outcome; raw engine conflicts leaked: {:?}",
        outcome.metrics.untyped_conflicts
    );
    assert!(
        outcome.metrics.unexpected_terminal.is_empty(),
        "unexpected terminal errors during the workload: {:?}",
        outcome.metrics.unexpected_terminal
    );
    assert!(
        outcome.metrics.retry_exhausted_errors.is_empty(),
        "retry exhaustion during the profile: {:?}",
        outcome.metrics.retry_exhausted_errors
    );
    assert!(
        outcome.metrics.lock_wait_timeouts.is_empty(),
        "keyed-lock waits must never exceed the statement timeout: {:?}",
        outcome.metrics.lock_wait_timeouts
    );
    assert_eq!(
        outcome.integrity.verdict,
        IntegrityVerdict::Pass,
        "integrity reconciliation after shutdown + reopen must PASS (zero lost writes, zero duplicate effects, zero partial projections, zero dirty reads); first violations:\n{}",
        outcome.integrity.rendered_violations(25)
    );
    assert_eq!(report.integrity_verdict, IntegrityVerdict::Pass, "report verdict");
    assert!(
        report.maximum_concurrent_operations >= 2,
        "workers must overlap (maximum_concurrent_operations >= 2)"
    );
    assert!(
        report.conflict_count > 0,
        "the contended profile must observe at least one typed conflict (zero contention would be a meaningless green)"
    );
    assert_eq!(report.remote_proof_status, RemoteProofStatus::NotRunUnconfigured);
    assert!(
        outcome.shutdown_elapsed <= handshake_core::storage::surreal::DEFAULT_SHUTDOWN_WAIT,
        "shutdown must complete inside its explicit bound (shutdown_wait)"
    );
    assert_eq!(
        report.integrity_verdict,
        IntegrityVerdict::Pass,
        "report integrity verdict must be pass"
    );
    assert!(outcome.report_path.exists(), "report file must exist");
    assert!(outcome.seeded_documents > 0, "dataset must be seeded");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn ci_profile_16_workers_2000_operations_is_correct_and_bounded() {
    let config = WorkloadConfig::ci(workload_seed());
    let outcome = timeout(config.whole_test_timeout, run_profile(config))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "CI profile exceeded its whole-test bound of {} ms",
                config.whole_test_timeout.as_millis()
            )
        });
    assert_profile(&outcome, &config);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn extended_profile_64_workers_50000_operations() {
    let enabled = std::env::var("HANDSHAKE_SWARM_EXTENDED")
        .map(|value| value.trim() == "1")
        .unwrap_or(false);
    if !enabled {
        println!("SWARM_EXTENDED=NOT_RUN_UNCONFIGURED (set HANDSHAKE_SWARM_EXTENDED=1 to run the >=64 worker / >=50000 operation profile)");
        return;
    }
    let config = WorkloadConfig::extended(workload_seed());
    let rss_before = process_rss_bytes();
    println!("SWARM_RSS_BEFORE={rss_before:?}");
    let outcome = timeout(config.whole_test_timeout, run_profile(config))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "extended profile exceeded its whole-test bound of {} ms",
                config.whole_test_timeout.as_millis()
            )
        });
    let rss_after = process_rss_bytes();
    println!("SWARM_RSS_AFTER={rss_after:?}");
    let fragment = json!({
        "schema_id": REPORT_FRAGMENT_SCHEMA_ID,
        "fragment": "extended_profile_memory",
        "run_id": outcome.report.run_id,
        "rss_bytes_before": rss_before,
        "rss_bytes_after": rss_after,
        "workers": config.workers,
        "operations": outcome.report.operation_count,
        "dataset_records": outcome.report.dataset_cardinality.records,
    });
    let path = write_report_json(
        &format!("swarm-load-extended-{}-memory.json", outcome.report.run_id),
        &fragment,
    );
    println!("SWARM_EXTENDED_MEMORY_REPORT={}", path.display());
    assert!(outcome.report.worker_count >= 64, "extended profile needs >= 64 workers");
    assert!(outcome.report.operation_count >= 50_000, "extended profile needs >= 50000 operations");
    assert!(
        outcome.report.dataset_cardinality.records >= 5_000,
        "extended profile needs dataset cardinality >= 5000"
    );
    assert_profile(&outcome, &config);
}
