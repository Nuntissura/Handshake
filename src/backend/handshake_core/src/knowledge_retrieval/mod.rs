//! WP-KERNEL-009 RetrievalContextAndRanking (MT-129..MT-144): the explainable
//! retrieval layer of the ProjectKnowledgeIndex.
//!
//! Master Spec anchors: 02-system-architecture.md §2.3.13.11 (RetrievalTrace is
//! "a replayable record of none/direct_load/exact_lookup/graph_traversal/
//! hybrid_rag, including why broader retrieval was used or skipped"), §2.3.14
//! [ADD v02.178] (the cheapest-authoritative rule), and §2.6.6.7.14 (QueryPlan /
//! RetrievalTrace / RetrievalBudgets / SemanticCatalog typed objects).
//!
//! This module is the PRODUCT LOGIC over two committed substrates:
//!   * the knowledge substrate (`storage/knowledge.rs`: sources, spans,
//!     entities, edges, claims, passages, AND the context-bundle /
//!     retrieval-trace tables 0141), and
//!   * the MemoryGraph (`knowledge_memory/**`, `storage/knowledge_memory.rs`:
//!     ontology terms, facts, conflicts, graph projections).
//!
//! It does NOT re-create the durable trace/bundle tables — those already exist
//! (MT-060, migration 0141). Its own storage surface is
//! `storage/knowledge_retrieval.rs` (the SemanticCatalog table 0260 + the
//! QueryPlan/RetrievalTrace persistence helper). Authority is PostgreSQL /
//! EventLedger; everything this module compiles for a consumer (a context
//! bundle, a debug payload, an export manifest) is a PROJECTION.
//!
//! Pipeline shape (a retrieval): a [`planner::RetrievalRequest`] →
//! [`planner::CheapestAuthoritativePathPlanner`] picks the cheapest
//! authoritative [`plan::QueryPlan`] mode (recording the non-hybrid reason);
//! [`schema_filter`] narrows fact candidates; [`graph_planner`] expands a
//! bounded neighborhood; [`passage_fallback`] catches missing/stale/contradicted
//! graphs; [`ranking`] scores candidates deterministically; [`snippet`] cites
//! them with spans/hashes/receipts; [`budget`] bounds the set; [`compiler`]
//! persists the kernel ContextBundle V1 + a replayable [`plan::RetrievalTrace`].
//! The four bridges ([`project_brain`], [`semantic_catalog`], [`ai_ready_export`],
//! [`context_pack_recorder`]) connect the folded-stub concepts into this layer.

pub mod ai_ready_export;
pub mod budget;
pub mod compiler;
pub mod context_pack_recorder;
pub mod executor;
pub mod fixtures;
pub mod graph_planner;
pub mod passage_fallback;
pub mod plan;
pub mod planner;
pub mod project_brain;
pub mod ranking;
pub mod schema_filter;
pub mod semantic_catalog;
pub mod snippet;
