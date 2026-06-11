//! WP-KERNEL-009 MemoryGraphAndClaims (MT-113..MT-128): the Handshake-native
//! memory ontology / fact / passage / claim-graph layer of the
//! ProjectKnowledgeIndex.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 and the
//! WP-009 contract field `translated_memory_system_spec`. This module is the
//! product-logic layer (projections, conflict detection/resolution jobs, bridge
//! generation, claim authority labels, visual-debug payloads, fixtures) over
//! the PostgreSQL storage surface in `storage/knowledge_memory.rs`.
//!
//! It EXTENDS the committed knowledge substrate (entities, edges, claims,
//! spans, passages) — it REUSES the claim lifecycle, the claim conflict table
//! + EventLedger-backed resolution, and the deterministic edge derivation
//! rather than duplicating them. Authority lives in PostgreSQL/EventLedger;
//! everything this module computes for visual-debug / API consumers is a
//! PROJECTION over those authority rows and is never itself authority
//! (spec 2.3.13.11).

pub mod bridge;
pub mod claim_labels;
pub mod conflict;
pub mod fact;
pub mod fixtures;
pub mod passage;
pub mod projection;
pub mod visual_debug;
