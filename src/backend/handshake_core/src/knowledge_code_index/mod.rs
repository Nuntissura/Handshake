//! WP-KERNEL-009 CodeIndexingAndNavigation (MT-097..MT-112).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 "Project
//! Knowledge Index and Rich Document Authority" [ADD v02.192]. This module
//! turns the code files that the SourceIngestionAndEvidence group
//! (MT-081..MT-096) already registered as `KnowledgeSource` rows into precise,
//! navigable graph records: typed `KnowledgeEntity` symbols, `ast`-kind
//! `KnowledgeSpan` anchors, and `KnowledgeEdge` call/import/test/doc
//! relationships — all PostgreSQL + EventLedger authority, never prose.
//!
//! # Model manual (no-context quick start)
//!
//! Purpose: index code STRUCTURE (symbols, doc/TODO passages, config keys) and
//! expose precise backend NAVIGATION for agents (symbol lookup, definition,
//! references, callers, importers) without an external LSP server.
//!
//! Core workflow:
//! 1. [`parser::CodeParserAdapter`] (MT-097) detects the language from a path
//!    and parses source into a typed, lifetime-free AST node stream. It is the
//!    single Tree-sitter entry point for the whole group; it reuses the
//!    statically-linked grammars the ai_ready_data chunker already proved.
//! 2. The per-language symbol extractors ([`symbols`], MT-098..MT-100) and the
//!    config/schema, doc/TODO, and test extractors ([`config_schema`],
//!    [`docs_todo`], [`tests_map`], MT-101..MT-103) turn that node stream into
//!    [`symbols::ExtractedSymbol`] records.
//! 3. [`engine::CodeIndexEngine`] (the orchestrator) writes each symbol THROUGH
//!    the storage layer (`storage::knowledge::KnowledgeStore`): an `ast`-kind
//!    span (the citeable evidence unit), then a `symbol`-kind entity anchored
//!    to that span, then `contains` edges to the file. Every write leaves an
//!    EventLedger receipt carrying actor/session/correlation identity
//!    (backend-navigation receipt law).
//! 4. [`relationships::RelationshipBuilder`] (MT-104) adds `references`
//!    (call) and `depends_on` (import) edges with the deterministic
//!    `relationship_id`, required source spans, extractor version, and
//!    confidence.
//! 5. [`staleness`] (MT-107) marks code entities stale when the source hash or
//!    parser version changes; [`engine`] (MT-108) keeps a run useful when one
//!    file fails to parse OR fails to read (partial receipts + a durable
//!    `knowledge_code_repair_queue` entry, 0230, that holds the file for
//!    re-parse — real enqueue, not a flag).
//! 6. HTTP surface: `src/api/knowledge_code_nav.rs` (MT-106,
//!    `/knowledge/code/*`). [`monaco_bridge`] (MT-109) and [`context_bridge`]
//!    (MT-110) project the same graph into Monaco code-lens payloads and bounded
//!    context bundles.
//!
//! Durable state: this module OWNS migrations 0170-0179 (its own support
//! tables: code-file index state for staleness, and the SCIP import ledger)
//! plus the MT-108 code-index repair queue (`knowledge_code_repair_queue`,
//! 0230, in the operator-assigned hardening band).
//! Symbols/spans/edges themselves live in the shared `knowledge_entities`
//! /`knowledge_spans`/`knowledge_edges` tables (0134-0136) and are written ONLY
//! through `storage::knowledge::KnowledgeStore` — this module never issues raw
//! SQL against those authority tables.
//!
//! Safety constraints: no SQLite, no Docker, no external daemon, no external
//! LSP server. MT-105 PARSES a provided SCIP/LSIF artifact into knowledge
//! records; it never spawns an indexer. A file the grammar cannot parse
//! produces a typed receipt and (when repairable) a repair-queue entry, never
//! silent partial indexing.
//!
//! Common failure modes + recovery:
//! * File fails to parse OR fails to read (binary / non-UTF-8 / unreadable) ->
//!   the run continues; the file's source row carries a `failed` parser status +
//!   receipt, and a `knowledge_code_repair_queue` entry (MT-108) holds it with a
//!   typed reason class (PARSE_ERROR / READ_ERROR / PANIC / CONFIG_PARSE_ERROR).
//!   Fix the cause, re-run the pass; a successful re-index resolves the entry.
//! * Stale symbols -> the source hash or parser version changed; MT-107 marks
//!   the file's entities stale and the nav API refuses to serve stale results
//!   silently (it flags them). Re-index to refresh.
//! * SCIP import rejected -> the artifact failed typed validation; the import
//!   ledger row (0179) carries the reason.

pub mod config_schema;
pub mod context_bridge;
pub mod docs_todo;
pub mod engine;
pub mod monaco_bridge;
pub mod parser;
pub mod perf;
pub mod relationships;
pub mod scip;
pub mod staleness;
pub mod symbols;
pub mod tests_map;

use crate::storage::StorageError;
use thiserror::Error;

pub use parser::{CodeLanguage, CodeParseError, CodeParserAdapter};

/// Extractor-family version recorded on every code entity/span/edge. Bumped
/// when extraction behavior changes so MT-107 staleness can detect it.
pub const CODE_EXTRACTOR_VERSION: &str = "code_index_extractor_v1";

/// Typed error surface for the code-index module. Every variant is
/// machine-distinguishable so API handlers and the partial-failure handler can
/// branch without string matching.
#[derive(Debug, Error)]
pub enum CodeIndexError {
    /// Input failed typed validation before any durable write.
    #[error("code index validation failed: {0}")]
    Validation(String),
    /// A Tree-sitter parse could not be performed (grammar init / no tree).
    /// A successful parse that merely CONTAINS syntax errors is NOT this error
    /// (it returns a tree with `root_has_error = true`).
    #[error("{0}")]
    Parse(#[from] CodeParseError),
    /// Underlying storage layer error (PostgreSQL/EventLedger).
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    /// Kernel EventLedger construction error.
    #[error("kernel event error: {0}")]
    Kernel(String),
    /// Filesystem error while reading a runtime-anchored source.
    #[error("filesystem error at {path}: {detail}")]
    Io { path: String, detail: String },
}

pub type CodeIndexResult<T> = Result<T, CodeIndexError>;
