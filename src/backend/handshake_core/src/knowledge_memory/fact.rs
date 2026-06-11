//! MT-114 MemoryFactSchema (product surface).
//!
//! The MemoryFact storage type and CRUD live in
//! `storage::knowledge_memory` (the PostgreSQL authority surface). This module
//! re-exports the fact vocabulary for product-logic consumers (projections,
//! conflict detection, the backend API) so they import a single
//! `knowledge_memory::fact` path instead of reaching into `storage`.
//!
//! A MemoryFact is a structured subject/predicate/object view backed 1:1 by a
//! `knowledge_claims` row: the claim carries the lifecycle, the REQUIRED
//! evidence spans, the transition guard, and the conflict machinery. The fact
//! adds the S/P/O decomposition, the predicate ontology link, and the
//! fact-level authority label (MT-125). It reuses, it does not duplicate.

pub use crate::storage::knowledge_memory::{
    create_memory_fact, get_memory_fact, get_memory_fact_by_claim, list_memory_facts,
    MemoryClaimAuthorityLabel, MemoryFact, MemoryFactObject, NewMemoryFact,
};
