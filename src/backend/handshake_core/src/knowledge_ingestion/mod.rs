//! WP-KERNEL-009 SourceIngestionAndEvidence (MT-081..MT-096).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 "Project
//! Knowledge Index and Rich Document Authority" [ADD v02.192]. This module is
//! the ingestion front door of the ProjectKnowledgeIndex: it turns operator
//! project roots, governance artifacts, research notes, PDFs, and media
//! transcripts into durable `KnowledgeSource` rows, ingestion spans,
//! extraction receipts, and repair-queue entries — all PostgreSQL +
//! EventLedger authority, never prose.
//!
//! # Model manual (no-context quick start)
//!
//! Purpose: register project roots (allowlist-enforced), ingest files under
//! them (hash-based, secret-aware, restartable), and leave replayable
//! evidence for every attempt — success, partial, failure, deferral.
//!
//! Core workflow:
//! 1. [`allowlist::RootRegistrationPolicy`] decides whether a repo-relative
//!    path may become a root at all (MT-081). Every evaluation persists a
//!    decision row + EventLedger receipt.
//! 2. [`engine::IngestionEngine::register_root`] creates the root through the
//!    storage layer (`storage::knowledge::KnowledgeStore`).
//! 3. [`engine::IngestionEngine::run_ingestion_pass`] walks the root under a
//!    runtime filesystem anchor (machine-local, never stored): per file it
//!    detects the kind ([`kinds`]), normalizes the path ([`paths`]), hashes
//!    content ([`hashing`]), enforces size/line limits ([`backpressure`]),
//!    runs the secret preflight ([`secrets`]), extracts spans
//!    ([`pdf`]/[`transcripts`]/[`governance`]/[`notes`]/code windows), and
//!    persists source + receipt + spans + repair entries.
//! 4. Failures land in the durable repair queue ([`repair`]);
//!    [`engine::IngestionEngine::retry_repair`] re-runs a budgeted attempt.
//! 5. HTTP surface: `src/api/knowledge_ingestion.rs` (`/knowledge/ingestion/*`).
//!
//! Durable state (all `knowledge_`-prefixed, registered in
//! `knowledge_schema_registry`; migrations 0160-0169):
//! * `knowledge_ingestion_root_policies` + `knowledge_ingestion_policy_decisions` (0160, MT-081)
//! * `knowledge_ingestion_kind_registry` (0161, MT-082, projection of [`kinds`])
//! * `knowledge_ingestion_receipts` (0162, MT-085, one row per extraction attempt)
//! * `knowledge_ingestion_spans` (0163, MT-087..MT-091, citable evidence units)
//! * `knowledge_ingestion_repair_queue` (0164, MT-094, durable retry queue)
//! Sources/roots themselves live in `knowledge_source_roots`/`knowledge_sources`
//! (0131/0132) and are written ONLY through `storage::knowledge::KnowledgeStore`.
//!
//! Safety constraints: no SQLite, no Docker, no external daemon, no ASR — the
//! "media transcript" path parses operator-provided SRT/VTT/JSON artifacts.
//! Failures produce typed receipts and repairable metadata, never silent
//! partial indexing. Raw secret bytes never reach a durable row.
//!
//! Common failure modes + recovery:
//! * Root rejected -> read `knowledge_ingestion_policy_decisions` for the
//!   verdict and matched pattern; fix the policy or pass operator approval.
//! * Receipt `failed`/`partial`/`deferred` -> `error_class` + `error_detail`
//!   say why; repairable classes sit in `knowledge_ingestion_repair_queue`.
//! * Receipt `blocked` + `SECRET_BLOCKED` -> the file carries high-severity
//!   secret material; clean the file, then re-run the pass.
//! * Image-only PDF -> `NO_TEXT_LAYER` with OCR guidance in the detail: run
//!   external OCR, import the produced transcript artifact instead.

pub mod allowlist;
pub mod hashing;
pub mod kinds;
pub mod paths;
pub mod pdf;
pub mod receipts;
pub mod spans;
pub mod transcripts;

use crate::storage::StorageError;
use thiserror::Error;
use uuid::Uuid;

/// Typed error surface for the ingestion module.
///
/// Every variant is machine-distinguishable so API handlers and repair
/// tooling can branch without string matching.
#[derive(Debug, Error)]
pub enum IngestionError {
    /// Root registration was denied by the workspace allowlist policy
    /// (MT-081). Carries the durable decision id for replay.
    #[error("root registration denied ({verdict}): {candidate_path} (decision {decision_id})")]
    PolicyDenied {
        verdict: allowlist::PolicyVerdictKind,
        candidate_path: String,
        matched_pattern: Option<String>,
        decision_id: String,
    },
    /// Input failed typed validation before any durable write.
    #[error("ingestion validation failed: {0}")]
    Validation(String),
    /// Underlying storage layer error (PostgreSQL/EventLedger).
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    /// Kernel EventLedger construction error.
    #[error("kernel event error: {0}")]
    Kernel(String),
    /// Filesystem error while walking/reading a runtime-anchored root.
    #[error("filesystem error at {path}: {detail}")]
    Io { path: String, detail: String },
}

pub type IngestionResult<T> = Result<T, IngestionError>;

/// New prefixed ingestion id (`<PREFIX>-<uuidv7 simple>`), matching the
/// `^<PREFIX>-[0-9a-f]{32}$` CHECK constraints in migrations 0160-0169.
pub(crate) fn new_ingestion_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}
