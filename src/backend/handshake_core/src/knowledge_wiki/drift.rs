//! WP-KERNEL-009 MT-242 `WikiProjectionDriftAndStaleness`.
//!
//! The native analog of codewiki's commit-vs-`source_files` drift check,
//! expressed over EventLedger versions + content hashes instead of git
//! (LM-PWIKI-006/007): every compiled page carries a [`WikiCompileStamp`]
//! (EventLedger source version + the exact cited-source set with content
//! hashes); the drift checker re-resolves the CURRENT hash of every cited
//! source and flags exactly the pages whose cited sources changed, with a
//! concrete [`WikiStaleReason`] (which source, stamped vs current hash, and
//! the ledger version delta). Unchanged sources never flag pages
//! (no-false-stale; proven by negative test).
//!
//! Serving rides [`WikiDriftChecker::evaluate_page`]: every page-serve path
//! attaches the verdict fail-closed (LM-PWIKI-008) — an evaluation error
//! fails the serve; it never silently serves a page as fresh. Pages without a
//! stamp evaluate to [`WikiStalenessVerdict::Unstamped`] and are forbidden to
//! be treated as fresh.
//!
//! NO wall-clock heuristics: every comparison is content-hash or
//! ledger-version based (LM-PWIKI-006).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    KnowledgeRebuildStatus, KnowledgeStore, KnowledgeWikiProjection,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::Database;

use super::compiler::WikiCompileContext;
use super::{
    entity_content_hash, loom_block_state_content_hash, CitedSource, CitedSourceKind,
    WikiCompileResult, WikiCompileStamp, WikiSourceChange, WikiStaleReason, WikiStalenessVerdict,
};

/// One page's drift result.
#[derive(Clone, Debug, serde::Serialize)]
pub struct WikiPageDrift {
    pub projection_id: String,
    pub title: String,
    pub page_type: Option<String>,
    pub verdict: WikiStalenessVerdict,
}

/// Workspace drift report (the MT-242 "exactly which pages are stale and
/// why" surface).
#[derive(Clone, Debug, serde::Serialize)]
pub struct WikiDriftReport {
    pub workspace_id: String,
    pub current_ledger_version: i64,
    pub pages: Vec<WikiPageDrift>,
    pub stale_pages: usize,
    pub fresh_pages: usize,
    pub unstamped_pages: usize,
    /// EventLedger receipt of this drift run (LM-PWIKI-012); present when the
    /// run was receipt-recorded.
    pub receipt_event_id: Option<String>,
}

/// The MT-242 drift checker.
pub struct WikiDriftChecker {
    db: Arc<PostgresDatabase>,
}

impl WikiDriftChecker {
    pub fn new(db: Arc<PostgresDatabase>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &PostgresDatabase {
        &self.db
    }

    /// Evaluate ONE page's staleness verdict against current authority (the
    /// page-serve path). Fail-closed: any resolution error propagates — the
    /// caller must fail the serve rather than serve without a verdict.
    pub async fn evaluate_page(
        &self,
        page: &KnowledgeWikiProjection,
    ) -> WikiCompileResult<WikiStalenessVerdict> {
        let mut verdicts = self
            .evaluate_pages(&page.workspace_id, std::slice::from_ref(page))
            .await?;
        Ok(verdicts.remove(0))
    }

    /// Evaluate one stored stamp value (any page shape that carries
    /// `compile_stamp`; the API serve paths use this). Missing/malformed
    /// stamps verdict [`WikiStalenessVerdict::Unstamped`] — never fresh.
    pub async fn evaluate_stamp_value(
        &self,
        workspace_id: &str,
        compile_stamp: Option<&serde_json::Value>,
    ) -> WikiCompileResult<WikiStalenessVerdict> {
        let stamp = WikiCompileStamp::from_value(compile_stamp);
        let mut verdicts = self.evaluate_stamps(workspace_id, &[stamp]).await?;
        Ok(verdicts.remove(0))
    }

    /// Evaluate a batch of pages (one round of batched hash resolution).
    /// Returns verdicts aligned with `pages`.
    pub async fn evaluate_pages(
        &self,
        workspace_id: &str,
        pages: &[KnowledgeWikiProjection],
    ) -> WikiCompileResult<Vec<WikiStalenessVerdict>> {
        let stamps: Vec<Option<WikiCompileStamp>> = pages
            .iter()
            .map(|page| WikiCompileStamp::from_value(page.compile_stamp.as_ref()))
            .collect();
        self.evaluate_stamps(workspace_id, &stamps).await
    }

    /// Core drift evaluation over parsed stamps (verdicts aligned with
    /// `stamps`).
    pub async fn evaluate_stamps(
        &self,
        workspace_id: &str,
        stamps: &[Option<WikiCompileStamp>],
    ) -> WikiCompileResult<Vec<WikiStalenessVerdict>> {
        let current_ledger_version = self.db.current_event_ledger_version().await?;

        // ---- collect the cited-source union over all stamps -----------------
        let mut source_ids: HashSet<String> = HashSet::new();
        let mut entity_ids: HashSet<String> = HashSet::new();
        let mut block_ids: HashSet<String> = HashSet::new();
        let mut document_ids: HashSet<String> = HashSet::new();
        for stamp in stamps.iter().flatten() {
            for cited in &stamp.cited_sources {
                match cited.kind {
                    CitedSourceKind::Source => source_ids.insert(cited.id.clone()),
                    CitedSourceKind::Entity => entity_ids.insert(cited.id.clone()),
                    CitedSourceKind::LoomBlock => block_ids.insert(cited.id.clone()),
                    CitedSourceKind::RichDocument => document_ids.insert(cited.id.clone()),
                };
            }
        }

        // ---- batch-resolve CURRENT content hashes ---------------------------
        let source_ids: Vec<String> = source_ids.into_iter().collect();
        let entity_ids: Vec<String> = entity_ids.into_iter().collect();
        let block_ids: Vec<String> = block_ids.into_iter().collect();
        let document_ids: Vec<String> = document_ids.into_iter().collect();

        let source_hashes = self.db.get_wiki_source_hashes(&source_ids).await?;
        let entity_hashes: HashMap<String, String> = self
            .db
            .get_wiki_entity_states(&entity_ids)
            .await?
            .into_iter()
            .map(|(entity, source_hash)| {
                let hash = entity_content_hash(&entity, source_hash.as_deref());
                (entity.entity_id, hash)
            })
            .collect();
        let block_hashes: HashMap<String, String> = self
            .db
            .get_wiki_loom_block_states(workspace_id, &block_ids)
            .await?
            .into_iter()
            .map(|state| {
                let hash = loom_block_state_content_hash(
                    state.title.as_deref(),
                    &state.content_type,
                    state.full_text_index.as_deref(),
                    state.document_id.as_deref(),
                    state.asset_id.as_deref(),
                    state.content_hash.as_deref(),
                );
                (state.block_id, hash)
            })
            .collect();
        let document_hashes = self
            .db
            .get_wiki_rich_document_hashes(&document_ids)
            .await?;

        let current_hash_of = |cited: &CitedSource| -> Option<&String> {
            match cited.kind {
                CitedSourceKind::Source => source_hashes.get(&cited.id),
                CitedSourceKind::Entity => entity_hashes.get(&cited.id),
                CitedSourceKind::LoomBlock => block_hashes.get(&cited.id),
                CitedSourceKind::RichDocument => document_hashes.get(&cited.id),
            }
        };

        // ---- per-page verdicts ----------------------------------------------
        let mut verdicts = Vec::with_capacity(stamps.len());
        for stamp in stamps {
            let Some(stamp) = stamp else {
                // No stamp -> forbidden to call fresh (LM-PWIKI-008).
                verdicts.push(WikiStalenessVerdict::Unstamped);
                continue;
            };
            let mut reasons: Vec<WikiStaleReason> = Vec::new();
            for cited in &stamp.cited_sources {
                match current_hash_of(cited) {
                    None => reasons.push(WikiStaleReason {
                        kind: cited.kind,
                        id: cited.id.clone(),
                        stamped_content_hash: cited.content_hash.clone(),
                        current_content_hash: None,
                        change: WikiSourceChange::SourceDeleted,
                    }),
                    Some(current) if current != &cited.content_hash => {
                        reasons.push(WikiStaleReason {
                            kind: cited.kind,
                            id: cited.id.clone(),
                            stamped_content_hash: cited.content_hash.clone(),
                            current_content_hash: Some(current.clone()),
                            change: WikiSourceChange::SourceChanged,
                        })
                    }
                    Some(_) => {}
                }
            }
            if reasons.is_empty() {
                verdicts.push(WikiStalenessVerdict::Fresh {
                    stamp_ledger_version: stamp.ledger_version,
                    current_ledger_version,
                });
            } else {
                verdicts.push(WikiStalenessVerdict::Stale {
                    stamp_ledger_version: stamp.ledger_version,
                    current_ledger_version,
                    reasons,
                });
            }
        }
        Ok(verdicts)
    }

    /// Run the workspace drift check: evaluate every wiki page (typed project
    /// pages AND untyped Loom topic pages — they share the store and the
    /// verdict contract), persist `rebuild_status = 'stale'` marks for
    /// drifted pages (mark stale, never silently; regeneration is what sets
    /// fresh again), and append the EventLedger staleness-verdict receipt
    /// (LM-PWIKI-012).
    pub async fn check_workspace(
        &self,
        ctx: &WikiCompileContext,
        workspace_id: &str,
        persist: bool,
    ) -> WikiCompileResult<WikiDriftReport> {
        ctx.validate()?;
        let current_ledger_version = self.db.current_event_ledger_version().await?;
        let pages = self
            .db
            .list_knowledge_wiki_pages(workspace_id, None, false, 2_000, 0)
            .await?;
        let verdicts = self.evaluate_pages(workspace_id, &pages).await?;

        let mut report_pages = Vec::with_capacity(pages.len());
        let mut stale_pages = 0usize;
        let mut fresh_pages = 0usize;
        let mut unstamped_pages = 0usize;
        for (page, verdict) in pages.iter().zip(verdicts.into_iter()) {
            match &verdict {
                WikiStalenessVerdict::Fresh { .. } => fresh_pages += 1,
                WikiStalenessVerdict::Stale { .. } => stale_pages += 1,
                WikiStalenessVerdict::Unstamped => unstamped_pages += 1,
            }
            report_pages.push(WikiPageDrift {
                projection_id: page.projection_id.clone(),
                title: page.title.clone(),
                page_type: page.page_type.clone(),
                verdict,
            });
        }

        if persist {
            for drift in &report_pages {
                if matches!(drift.verdict, WikiStalenessVerdict::Stale { .. }) {
                    self.db
                        .set_knowledge_projection_rebuild_status(
                            &drift.projection_id,
                            KnowledgeRebuildStatus::Stale,
                        )
                        .await?;
                }
            }
        }

        // Staleness-verdict receipt: the machine-readable drift outcome in the
        // compile log (LM-PWIKI-012).
        let stale_detail: Vec<Value> = report_pages
            .iter()
            .filter(|d| matches!(d.verdict, WikiStalenessVerdict::Stale { .. }))
            .map(|d| {
                json!({
                    "projection_id": d.projection_id,
                    "title": d.title,
                    "page_type": d.page_type,
                    "verdict": d.verdict,
                })
            })
            .collect();
        let unstamped_detail: Vec<Value> = report_pages
            .iter()
            .filter(|d| matches!(d.verdict, WikiStalenessVerdict::Unstamped))
            .map(|d| json!({"projection_id": d.projection_id, "title": d.title}))
            .collect();
        let mut builder = NewKernelEvent::builder(
            ctx.kernel_task_run_id.clone(),
            ctx.session_run_id.clone(),
            KernelEventType::KnowledgeProjectionRebuilt,
            ctx.actor.clone(),
        )
        .aggregate("knowledge_wiki", workspace_id.to_string())
        .source_component("project_wiki_drift_checker")
        .payload(json!({
            "kind": "wiki_drift_check",
            "workspace_id": workspace_id,
            "current_ledger_version": current_ledger_version,
            "pages_checked": report_pages.len(),
            "stale_pages": stale_pages,
            "fresh_pages": fresh_pages,
            "unstamped_pages": unstamped_pages,
            "stale": stale_detail,
            "unstamped": unstamped_detail,
            "persisted": persist,
        }));
        if let Some(correlation_id) = &ctx.correlation_id {
            builder = builder.correlation_id(correlation_id.clone());
        }
        let receipt = self.db.append_kernel_event(builder.build()?).await?;

        Ok(WikiDriftReport {
            workspace_id: workspace_id.to_string(),
            current_ledger_version,
            pages: report_pages,
            stale_pages,
            fresh_pages,
            unstamped_pages,
            receipt_event_id: Some(receipt.event_id),
        })
    }
}
