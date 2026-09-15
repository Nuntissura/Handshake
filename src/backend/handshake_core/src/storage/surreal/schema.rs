use std::collections::{BTreeMap, BTreeSet};

use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use surrealdb::types::{
    Array as SurrealArray, Datetime, Object as SurrealObject, RecordId, RecordIdKey, SurrealValue,
    Value as SurrealValueData,
};
use tokio::sync::Mutex;

use super::{
    SurrealAdminContext, SurrealStorage, SurrealStorageError, DEFAULT_DATABASE, DEFAULT_NAMESPACE,
};

pub const SCHEMA_VERSION: &str = "wp-kernel-012-surreal-v1";
pub const SCHEMA_REVISION: i64 = 157;
/// Stable v1 lineage identifier retained so existing embedded stores remain readable after the
/// legacy schema-provenance corpus is removed. New source integrity is proven independently by
/// [`DECLARATIVE_SCHEMA_CATALOG_SHA256`] and [`GENERATED_SURREALQL_SHA256`].
pub const SCHEMA_LINEAGE_SHA256: &str =
    "225ed19c0259ef121867ca5da1995813db0c48ee0cbfaded2d871e47b50f7fc1";
// MT-142 re-pin: the predecessor artifact is derived from the current schema.surql
// (see `mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current`), so it
// moves with the knowledge_rich_document_title_anchors block.
// MT-151 re-pin: moves again with loom_blocks.journal_key and storage_graph_anchors.
// MT-152 re-pin: moves again with fems_workspace_write_anchors, then with
// loom_folders.sibling_key / uq_loom_folders_sibling_key (I-152-2 sweep finding; offline
// recomputation, same derivation as the test).
const PREDECESSOR_GENERATED_SURREALQL_SHA256: &str =
    "cd6c96a495e65b24958c4583a3d9935645e25b765ff7403911e08c3c72804a7e";
// MT-142 re-pin: the synthesized predecessor store (derived from the current schema.surql
// with the retired registry field) now carries knowledge_rich_document_title_anchors.
// MT-151 re-pin: it now also carries journal_key and storage_graph_anchors, with table catalog
// ids stripped (run mt142-LIB-20260911T124916Z, HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
// MT-152 re-pin: it now also carries fems_workspace_write_anchors (run mt142-LIB-20260911T162250Z,
// HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
// MT-152 re-pin (I-152-2 sweep finding): it now also carries loom_folders.sibling_key and its
// UNIQUE index (run mt142-LIB-20260911T234013Z, HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
const PREDECESSOR_SCHEMA_INFO_SHA256: &str =
    "3d704adc7ddbf6ec802567aaa614cacedc97f76123b18b66e8ed6ecd1ba81cde";
const PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256: &str =
    "1f8443486cd7101babb56dd6264ffcf08538a1eae24016d2155b19d5eb6370b4";
// MT-142 re-pin: schema.surql gained knowledge_rich_document_title_anchors.
// MT-151 re-pin: schema.surql gained loom_blocks.journal_key and storage_graph_anchors.
// MT-152 re-pin: schema.surql gained fems_workspace_write_anchors, then
// loom_folders.sibling_key / uq_loom_folders_sibling_key (I-152-2 sweep finding).
pub const GENERATED_SURREALQL_SHA256: &str =
    "0b9e32329d735477064393a1406825f94aa55db94f6256e3206f02166ae13dc6";
// MT-142 re-pin: catalog identities gained the knowledge_rich_document_title_anchors objects.
// MT-151 re-pin: catalog identities gained the journal_key field/index and the
// storage_graph_anchors objects.
// MT-152 re-pin: catalog identities gained the fems_workspace_write_anchors objects, then the
// loom_folders sibling_key field/index (I-152-2 sweep finding).
pub const DECLARATIVE_SCHEMA_CATALOG_SHA256: &str =
    "9881bff3f6bd7d02797fb95c88ad51f1ae6f19777f89477e3285cc014c37d014";
// MT-142 re-pin: the seed gained the rich_document_title_anchors registry row (63 rows).
pub const KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256: &str =
    "64d0711c5273c6eb103c3d574b2f7ee98d9d0ebfd46e9c25ad65908b46573b75";
/// Fresh-engine STRUCTURE fingerprint captured with the product-locked SurrealDB 3.2.0
/// engine family after applying the generated schema to an absent RocksDB path.
// MT-142 re-pin: live STRUCTURE fingerprint with knowledge_rich_document_title_anchors applied.
// MT-151 re-pin: live STRUCTURE fingerprint with journal_key and storage_graph_anchors applied
// and engine table catalog ids stripped (see `inspect_schema`); run mt142-LIB-20260911T124916Z,
// `mt139_current_schema_info_pin_matches_fresh_mem_catalog`, and reached identically by the
// in-place MT-151 upgrade (`mt151_exact_mt142_pin_upgrade_materialises_journal_key_and_restarts_current`).
// MT-152 re-pin: live STRUCTURE fingerprint with fems_workspace_write_anchors applied; run
// mt142-LIB-20260911T160117Z, `mt139_current_schema_info_pin_matches_fresh_mem_catalog`, and
// reached identically by the in-place MT-152 upgrade
// (`mt152_exact_mt151_pin_upgrade_adds_fems_write_anchors_and_restarts_current`).
// MT-152 re-pin (I-152-2 sweep finding): loom_folders.sibling_key / uq_loom_folders_sibling_key
// applied; run mt142-LIB-20260911T233822Z, `mt139_current_schema_info_pin_matches_fresh_mem_catalog`
// observed, and reached identically by both in-place MT-152 upgrade proofs.
pub const EXPECTED_SCHEMA_INFO_SHA256: &str =
    "f58198becbec2c5d922d98ae742596ba603566d53c6c8c5b90447aaf1c9c0384";
const EXPECTED_ATELIER_CATALOG_SHA256: &str =
    "25cd85bc8267363891ef9bcece05b2e41b4aa0762e8384f86f4a1563e1d43585";
const PENDING_SCHEMA_INFO_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
/// Second allowlisted lineage (MT-142): every store bootstrapped at schema revision 157 before
/// `knowledge_rich_document_title_anchors` existed. These are the exact pre-MT-142 pins of
/// [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded
/// in place by `upgrade_pre_mt142_current` instead of failing closed.
const PRE_MT142_GENERATED_SURREALQL_SHA256: &str =
    "b4bcdbd16ffbbb3d9543f164f4226d3d952c841e80a7bd9c302b82cb15e3d4f9";
const PRE_MT142_SCHEMA_INFO_SHA256: &str =
    "685bc539ddd8864c773bb8bb599768570faa66a4ff17ee7ab24e10d6e2b2db41";
/// Third allowlisted lineage (MT-151): every store bootstrapped at the MT-142 pin, before
/// `loom_blocks.journal_key` / `uq_loom_blocks_journal_key` and `storage_graph_anchors` existed.
/// These are the exact MT-142 pins of [`GENERATED_SURREALQL_SHA256`] and
/// [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded in place by
/// `upgrade_pre_mt151_current`. Pre-MT-142 stores receive both upgrades in one transaction.
const PRE_MT151_GENERATED_SURREALQL_SHA256: &str =
    "ecfdca9826223629277a218c7f22a3d6aaabf0714274cf0358cc0ab5a8d562a0";
const PRE_MT151_SCHEMA_INFO_SHA256: &str =
    "e117afdb9a7ff9ded218b29a5741b5fbf2541170f0772e475475114fca42a994";
/// Fourth allowlisted lineage (MT-152): every store bootstrapped at the MT-151 pin, before
/// `fems_workspace_write_anchors` existed. These are the exact MT-151 pins of
/// [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded
/// in place by `upgrade_pre_mt152_current`. Pre-MT-142 and pre-MT-151 stores receive the
/// MT-152 statements inside their own upgrade transaction, because the finalize gate pins the
/// current fingerprint.
const PRE_MT152_GENERATED_SURREALQL_SHA256: &str =
    "a8bb72c7fd73c1a2ea0b9563ee2f0534bd9cd435d153975b61bf6794ff9a598a";
const PRE_MT152_SCHEMA_INFO_SHA256: &str =
    "294530f11ca454f1afba332ac9e70e909ab39cac12ce35ad661daff2fd0ff222";
/// MT-152 upgrade statements, applied with the state update in one transaction on top of every
/// allowlisted predecessor lineage. Every DDL statement must stay identical to `schema.surql`
/// (proven by `mt152_upgrade_statements_match_schema`). No backfill: the anchor rows are
/// created lazily by the first FEMS write or workspace delete per workspace.
const MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at ON TABLE fems_workspace_write_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_workspace_write_anchors ON TABLE fems_workspace_write_anchors FIELDS anchor_key UNIQUE;
";
/// MT-152 (I-152-2 sweep finding): the stored `loom_folders.sibling_key` discriminator.
/// `uq_loom_folders_sibling_name` never rejected a duplicate ROOT name because the pinned
/// engine skips uniqueness for any tuple containing NONE
/// (`surrealdb-core-3.2.0/src/idx/index.rs:190-197`); MT-151's audit recorded that index as
/// covering the invariant. Phase one (own transaction, MT-151 `journal_key` pattern): define the
/// field, then `backfill_mt152_folder_sibling_keys` writes every existing row's key and
/// disambiguates pre-existing root duplicates with a stable `#dup<n>` suffix so an operator
/// store upgrades instead of failing at the index build; phase two builds the UNIQUE index in
/// the DDL transaction. Both statements must stay identical to `schema.surql` (proven by
/// `mt152_folder_sibling_key_statements_match_schema`).
const MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS: &str = "\
DEFINE FIELD OVERWRITE sibling_key ON TABLE loom_folders TYPE string ASSERT string::trim($value) != '';
";
const MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS: &str = "\
DEFINE INDEX OVERWRITE uq_loom_folders_sibling_key ON TABLE loom_folders FIELDS sibling_key UNIQUE;
";
/// First MT-151 upgrade phase, committed in its OWN transaction before the DDL transaction:
/// defines the computed `journal_key` and rewrites every existing journal block so the key is
/// materialised and COMMITTED before `uq_loom_blocks_journal_key` is built. The pinned engine
/// builds every `DEFINE INDEX` through its `IndexBuilder` in separate transactions
/// (`surrealdb-core-3.2.0/src/expr/statements/define/index.rs:235`,
/// `kvs/index/builder.rs:864-887`), so a backfill inside the same transaction as the index is
/// invisible to the build and pre-existing journal rows would stay unprotected (observed run
/// `mt142-LIB-20260911T125838Z`). Idempotent: re-running it after a crash is harmless.
const MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS: &str = "\
DEFINE FIELD OVERWRITE journal_key ON TABLE loom_blocks TYPE option<string>
    VALUE IF $this.content_type = 'journal' AND $this.journal_date != NONE {
        type::string($this.workspace_id) + '|' + $this.journal_date
    } ELSE {
        NONE
    };
UPDATE loom_blocks SET updated_at = updated_at WHERE content_type = 'journal' AND journal_date != NONE RETURN NONE;
";
/// Second MT-151 upgrade phase, applied with the state update in one transaction on top of
/// every allowlisted predecessor lineage. Every DDL statement here and in
/// [`MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS`] must stay identical to `schema.surql` (proven
/// by `mt151_upgrade_statements_match_schema`). A store that already holds two journal blocks
/// for one (workspace, date) fails closed at the index build rather than keeping an invariant
/// the index cannot honour.
const MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE INDEX OVERWRITE uq_loom_blocks_journal_key ON TABLE loom_blocks FIELDS journal_key UNIQUE;
DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE storage_graph_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE graph_kind ON TABLE storage_graph_anchors TYPE 'loom_folder_tree' | 'work_packet_dependencies';
DEFINE FIELD OVERWRITE scope_key ON TABLE storage_graph_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE version ON TABLE storage_graph_anchors TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE updated_at ON TABLE storage_graph_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_storage_graph_anchors ON TABLE storage_graph_anchors FIELDS anchor_key UNIQUE;
";
/// DDL and registry row MT-142 adds on top of both allowlisted predecessor lineages. Every DDL
/// line must stay byte-identical to the `knowledge_rich_document_title_anchors` block in
/// `schema.surql` (proven by `mt142_title_anchor_upgrade_statements_match_schema`).
const MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_title_anchors TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE last_rich_document_id ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_title_anchors ON TABLE knowledge_rich_document_title_anchors FIELDS anchor_key UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_rich_document_title_anchors_identity ON TABLE knowledge_rich_document_title_anchors FIELDS workspace_id, title_key UNIQUE;
CREATE ONLY knowledge_schema_registry:rich_document_title_anchors CONTENT {
    family_key: 'rich_document_title_anchors',
    table_name: 'knowledge_rich_document_title_anchors',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-142'
};
";

const SCHEMA: &str = include_str!("schema.surql");
#[cfg(test)]
const PRE_MT109_SCHEMA: &str = include_str!("schema_pre_mt109.surql");
const PRE_MT109_GENERATED_SURREALQL_SHA256: &str =
    "46eac57c4ac3e39acc9d18ac0a43fc62ec01461e8cf3b70b7e2711de2a59da10";
const PRE_MT109_AUTHORITY_INFO_SHA256: &str =
    "8803fbcc07ae64c671d6ecdc206f5a48efdf4d536444ff011d0db66d023672ca";
const PRE_MT109_SCHEMA_INFO_SHA256: &str =
    "bb515db9b0f18c4bc8b7c26cf0773cb9dbe5bb4343ba02bc9ea32218cfddd39d";
#[cfg(test)]
const PRE_MT109_AUTHORITY_SCHEMA: &str = include_str!("schema_pre_mt109_authority.surql");
const KNOWLEDGE_SCHEMA_REGISTRY_SEED: &str = include_str!("knowledge_schema_registry_seed.surql");
const DECLARATIVE_SCHEMA_CATALOG_DOMAIN: &[u8] =
    b"handshake.surreal.declarative-schema-catalog.v1\0";
const PREDECESSOR_KNOWLEDGE_REGISTRY_DOMAIN: &[u8] =
    b"handshake.surreal.predecessor-knowledge-schema-registry.v1\0";
/// Byte-exact SurrealQL projection of the 61 registry tuples independently extracted from the
/// deleted Git migration objects. It is compatibility data only; no deleted file is opened or
/// executed. The current-only 0343 registry row is intentionally absent and added by upgrade.
const PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED: &str = r#"
BEGIN TRANSACTION;
FOR $registry IN [
    ['claim_conflicts', 'knowledge_claim_conflicts', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['claim_spans', 'knowledge_claim_spans', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['claims', 'knowledge_claims', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['code_files', 'knowledge_code_files', 'KnowledgeSource', 'support', '0170_knowledge_code_files.sql', 'WP-KERNEL-009', 'MT-107'],
    ['code_repair_queue', 'knowledge_code_repair_queue', 'KnowledgeSource', 'support', '0230_knowledge_code_repair_queue.sql', 'WP-KERNEL-009', 'MT-108'],
    ['code_scip_imports', 'knowledge_code_scip_imports', 'KnowledgeEdge', 'support', '0171_knowledge_code_scip_imports.sql', 'WP-KERNEL-009', 'MT-105'],
    ['context_bundle_items', 'knowledge_context_bundle_items', 'Support', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['context_bundles', 'knowledge_context_bundles', 'Support', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['crdt_agent_lane_leases', 'knowledge_crdt_agent_lane_leases', 'AgentLaneLease', 'support', '0151_knowledge_crdt_agent_lane_leases.sql', 'WP-KERNEL-009', 'MT-076'],
    ['crdt_ai_edit_proposals', 'knowledge_crdt_ai_edit_proposals', 'AiEditProposal', 'support', '0154_knowledge_crdt_ai_edit_proposals.sql', 'WP-KERNEL-009', 'MT-074'],
    ['crdt_denial_receipts', 'knowledge_crdt_denial_receipts', 'CrdtDenialReceipt', 'support', '0150_knowledge_crdt_denial_receipts.sql', 'WP-KERNEL-009', 'MT-070'],
    ['crdt_graph_proposals', 'knowledge_crdt_graph_proposals', 'GraphMutationProposal', 'support', '0152_knowledge_crdt_graph_proposals.sql', 'WP-KERNEL-009', 'MT-068'],
    ['crdt_promoted_facts', 'knowledge_crdt_promoted_facts', 'KnowledgeClaim', 'authority', '0153_knowledge_crdt_promoted_facts.sql', 'WP-KERNEL-009', 'MT-069'],
    ['crdt_recovery_receipts', 'knowledge_crdt_recovery_receipts', 'CrdtRecoveryReceipt', 'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'WP-KERNEL-009', 'MT-079'],
    ['crdt_swarm_checkpoints', 'knowledge_crdt_swarm_checkpoints', 'SwarmCheckpoint', 'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'WP-KERNEL-009', 'MT-079'],
    ['debug_breakpoints', 'knowledge_debug_breakpoints', 'DebugBreakpoints', 'support', '0331_debug_breakpoints.sql', 'WP-KERNEL-009', 'MT-254'],
    ['document_backlinks', 'knowledge_document_backlinks', 'KnowledgeEdge', 'authority', '0282_knowledge_document_backlinks.sql', 'WP-KERNEL-009', 'MT-155'],
    ['document_embeds', 'knowledge_document_embeds', 'RichDocument', 'authority', '0281_knowledge_document_embeds.sql', 'WP-KERNEL-009', 'MT-152'],
    ['edge_spans', 'knowledge_edge_spans', 'KnowledgeEdge', 'authority', '0136_knowledge_edges.sql', 'WP-KERNEL-009', 'MT-054'],
    ['edges', 'knowledge_edges', 'KnowledgeEdge', 'authority', '0136_knowledge_edges.sql', 'WP-KERNEL-009', 'MT-054'],
    ['editor_code_nodes', 'knowledge_editor_code_nodes', 'EditorCodeNode', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['entities', 'knowledge_entities', 'KnowledgeEntity', 'authority', '0135_knowledge_entities.sql', 'WP-KERNEL-009', 'MT-053'],
    ['entity_spans', 'knowledge_entity_spans', 'KnowledgeEntity', 'authority', '0135_knowledge_entities.sql', 'WP-KERNEL-009', 'MT-053'],
    ['idempotency_keys', 'knowledge_idempotency_keys', 'Support', 'support', '0142_knowledge_idempotency_keys.sql', 'WP-KERNEL-009', 'MT-062'],
    ['index_runs', 'knowledge_index_runs', 'Support', 'authority', '0133_knowledge_index_runs.sql', 'WP-KERNEL-009', 'MT-052'],
    ['ingestion_kind_registry', 'knowledge_ingestion_kind_registry', 'KnowledgeSource', 'projection', '0161_knowledge_ingestion_kind_registry.sql', 'WP-KERNEL-009', 'MT-082'],
    ['ingestion_policy_decisions', 'knowledge_ingestion_policy_decisions', 'KnowledgeSource', 'support', '0160_knowledge_ingestion_policies.sql', 'WP-KERNEL-009', 'MT-081'],
    ['ingestion_receipts', 'knowledge_ingestion_receipts', 'KnowledgeSource', 'authority', '0162_knowledge_ingestion_receipts.sql', 'WP-KERNEL-009', 'MT-085'],
    ['ingestion_repair_queue', 'knowledge_ingestion_repair_queue', 'KnowledgeSource', 'authority', '0164_knowledge_ingestion_repair_queue.sql', 'WP-KERNEL-009', 'MT-094'],
    ['ingestion_root_policies', 'knowledge_ingestion_root_policies', 'KnowledgeSource', 'authority', '0160_knowledge_ingestion_policies.sql', 'WP-KERNEL-009', 'MT-081'],
    ['ingestion_spans', 'knowledge_ingestion_spans', 'KnowledgeSpan', 'authority', '0163_knowledge_ingestion_spans.sql', 'WP-KERNEL-009', 'MT-087'],
    ['memory_bridge_decisions', 'knowledge_memory_bridge_decisions', 'BridgeEdgeJob', 'authority', '0243_knowledge_memory_bridge_edges.sql', 'WP-KERNEL-009', 'MT-124'],
    ['memory_conflict_detection_findings', 'knowledge_memory_conflict_detection_findings', 'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-122'],
    ['memory_conflict_detection_jobs', 'knowledge_memory_conflict_detection_jobs', 'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-122'],
    ['memory_conflict_resolution_jobs', 'knowledge_memory_conflict_resolution_jobs', 'ConflictResolutionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-123'],
    ['memory_facts', 'knowledge_memory_facts', 'MemoryFact', 'authority', '0241_knowledge_memory_facts.sql', 'WP-KERNEL-009', 'MT-114'],
    ['memory_ontology_aliases', 'knowledge_memory_ontology_aliases', 'MemoryOntology', 'authority', '0240_knowledge_memory_ontology.sql', 'WP-KERNEL-009', 'MT-113'],
    ['memory_ontology_terms', 'knowledge_memory_ontology_terms', 'MemoryOntology', 'authority', '0240_knowledge_memory_ontology.sql', 'WP-KERNEL-009', 'MT-113'],
    ['memory_passages', 'knowledge_memory_passages', 'MemoryPassage', 'authority', '0138_knowledge_memory_passages.sql', 'WP-KERNEL-009', 'MT-057'],
    ['parallel_indexing_lease_queue', 'knowledge_parallel_indexing_lease_queue', 'IndexingLease', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-216'],
    ['parallel_swarm_checkpoints', 'knowledge_agent_state_recovery_checkpoints', 'SwarmCheckpoint', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-213'],
    ['parallel_swarm_claims', 'knowledge_agent_worktree_claims', 'SwarmClaim', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-210'],
    ['parallel_swarm_cloud_assistance_receipts', 'knowledge_agent_cloud_assistance_receipts', 'SwarmCloudAssistanceReceipt', 'support', '0314_parallel_swarm_cloud_assistance_receipts.sql', 'WP-KERNEL-009', 'MT-221'],
    ['parallel_swarm_handoffs', 'knowledge_agent_role_mailbox_handoffs', 'SwarmHandoff', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-211'],
    ['parallel_swarm_quiet_background_work', 'knowledge_agent_quiet_background_work', 'SwarmQuietBackgroundWork', 'support', '0313_parallel_swarm_quiet_background_work.sql', 'WP-KERNEL-009', 'MT-219'],
    ['parallel_swarm_recovery_receipts', 'knowledge_agent_recovery_receipts', 'SwarmRecoveryReceipt', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-214'],
    ['passage_evidence', 'knowledge_passage_evidence', 'MemoryPassage', 'authority', '0138_knowledge_memory_passages.sql', 'WP-KERNEL-009', 'MT-057'],
    ['quick_switcher_recents', 'knowledge_quick_switcher_recents', 'QuickSwitcherRecent', 'support', '0322_quick_switcher_recents.sql', 'WP-KERNEL-009', 'MT-256'],
    ['retrieval_traces', 'knowledge_retrieval_traces', 'RetrievalTrace', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['rich_document_drafts', 'knowledge_rich_document_drafts', 'RichDocumentDraftRecovery', 'support', '0328_rich_document_draft_recovery.sql', 'WP-KERNEL-009', 'MT-255'],
    ['rich_document_versions', 'knowledge_rich_document_versions', 'RichDocument', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['rich_documents', 'knowledge_rich_documents', 'RichDocument', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['schema_registry', 'knowledge_schema_registry', 'Support', 'support', '0130_knowledge_schema_namespace.sql', 'WP-KERNEL-009', 'MT-049'],
    ['semantic_catalog_entries', 'knowledge_semantic_catalog_entries', 'Support', 'authority', '0260_knowledge_semantic_catalog.sql', 'WP-KERNEL-009', 'MT-140'],
    ['source_roots', 'knowledge_source_roots', 'KnowledgeSource', 'authority', '0131_knowledge_source_roots.sql', 'WP-KERNEL-009', 'MT-050'],
    ['sources', 'knowledge_sources', 'KnowledgeSource', 'authority', '0132_knowledge_sources.sql', 'WP-KERNEL-009', 'MT-051'],
    ['spans', 'knowledge_spans', 'KnowledgeSpan', 'authority', '0134_knowledge_spans.sql', 'WP-KERNEL-009', 'MT-055'],
    ['wiki_projections', 'knowledge_wiki_projections', 'Projection', 'projection', '0139_knowledge_wiki_projections.sql', 'WP-KERNEL-009', 'MT-058'],
    ['workbench_layout_state', 'knowledge_workbench_layout_states', 'WorkbenchLayoutState', 'support', '0323_workbench_layout_state.sql', 'WP-KERNEL-009', 'MT-246'],
    ['workspace_search_bookmark_state', 'knowledge_workspace_search_bookmark_states', 'WorkspaceSearchBookmarkState', 'support', '0330_workspace_search_bookmark_state.sql', 'WP-KERNEL-009', 'MT-258'],
    ['workspace_settings_state', 'knowledge_workspace_settings_states', 'WorkspaceSettingsState', 'support', '0327_workspace_settings_state.sql', 'WP-KERNEL-009', 'MT-248'],
] {
    CREATE type::record('knowledge_schema_registry', $registry[0]) CONTENT {
        family_key: $registry[0], table_name: $registry[1], record_family: $registry[2],
        authority_class: $registry[3], migration_file: $registry[4],
        wp_id: $registry[5], mt_id: $registry[6]
    };
};
COMMIT TRANSACTION;
"#;
const BOOTSTRAP_STATE_TABLE: &str = "handshake_schema_state";
const BOOTSTRAP_STATE_ID: &str = "handshake_schema_state:primary";
const ATELIER_CATALOG_INFO_CONCURRENCY: usize = 8;
const ATELIER_REQUIRED_SEQUENCES: [&str; 2] =
    ["atelier_pose_context_state_seq", "kernel_event_sequence"];
const DATABASE_STRUCTURE_CATEGORIES: [&str; 12] = [
    "accesses",
    "analyzers",
    "apis",
    "buckets",
    "configs",
    "functions",
    "models",
    "modules",
    "params",
    "sequences",
    "tables",
    "users",
];
// MT-142 re-pin: +1 table (knowledge_rich_document_title_anchors), +7 fields,
// +2 indexes (pk + uq), +1 REFERENCE field, +1 record-id alias assertion.
// MT-151 re-pin: +1 table (storage_graph_anchors: +5 fields, +1 pk index, +1 record-id
// alias assertion) and loom_blocks.journal_key (+1 field, +1 uq index); no REFERENCE field.
// MT-152 re-pin: +1 table (fems_workspace_write_anchors: +4 fields, +1 pk index, +1 record-id
// alias assertion); no REFERENCE field.
const TABLE_DEFINITION_COUNT: usize = 292;
const SOURCE_FIELD_DEFINITION_COUNT: usize = 3191;
const FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT: usize = 238;
const FLEXIBLE_FIELD_DEFINITION_COUNT: usize = 175;
const INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS: [&str; 2] = [
    "DEFINE FIELD OVERWRITE capability_grants ON TABLE atelier_transcript_receipt TYPE any DEFAULT [];",
    "DEFINE FIELD OVERWRITE decisions ON TABLE knowledge_retrieval_traces TYPE any DEFAULT [];",
];
const AUTHORED_FIELD_DEFINITION_COUNT: usize =
    SOURCE_FIELD_DEFINITION_COUNT + FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT;
// SurrealDB 3.2 persists one `field.*` subtype definition per non-Any typed collection nesting
// level. Structured INFO reads the full persisted field catalog, so these engine-generated
// definitions are part of the exact live schema even though they are not authored DEFINE lines.
const ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT: usize = 55;
const FIELD_DEFINITION_COUNT: usize =
    AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT;
const INDEX_DEFINITION_COUNT: usize = 812;
const EVENT_DEFINITION_COUNT: usize = 20;
const VIEW_DEFINITION_COUNT: usize = 2;
const SEQUENCE_DEFINITION_COUNT: usize = 2;
const ACCESS_DEFINITION_COUNT: usize = 1;
const FUNCTION_DEFINITION_COUNT: usize = 10;
const SOURCE_TABLE_COUNT: usize = 289;
const SOURCE_VIEW_COUNT: usize = 2;
const SOURCE_NAMED_INDEX_COUNT: usize = 552;
const SURREAL_PRIMARY_KEY_INDEX_COUNT: usize = 259;
const SURREAL_BOOTSTRAP_STATE_TABLE_COUNT: usize = 1;
const SURREAL_BOOTSTRAP_STATE_INDEX_COUNT: usize = 1;
const REFERENCE_FIELD_COUNT: usize = 406;
const EXPLICIT_REFERENCE_EXISTENCE_ASSERTION_COUNT: usize = 405;
const RECORD_ID_ALIAS_ASSERTION_COUNT: usize = 228;

static BOOTSTRAP_MUTEX: Mutex<()> = Mutex::const_new(());

/// Applies the canonical Atelier schema projection, together with the shared
/// EventLedger table and sequence that every Atelier mutation writes.
///
/// The projection is selected mechanically from the same compiled
/// `schema.surql` consumed by [`bootstrap_schema`]. It is a bounded production
/// bootstrap component, not a hand-maintained test schema. The shared bootstrap
/// mutex covers inspection and mutation, and the DDL is one transaction, so a
/// concurrent caller or failed statement cannot strand a partial projection.
/// Returns `true` only when this call installed the projection.
pub async fn bootstrap_atelier_schema(
    storage: &SurrealStorage,
) -> Result<bool, SurrealStorageError> {
    let _bootstrap_guard = BOOTSTRAP_MUTEX.lock().await;
    let ddl = atelier_schema_ddl();
    let expected = atelier_expected_catalog();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                let present = atelier_table_definitions(&database).await?;
                let expected_atelier_tables = expected
                    .keys()
                    .filter(|table| table.starts_with("atelier_"))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let present_atelier_tables = present
                    .keys()
                    .filter(|table| table.starts_with("atelier_"))
                    .cloned()
                    .collect::<BTreeSet<_>>();

                if present_atelier_tables.is_empty() {
                    database
                        .query(format!(
                            "BEGIN TRANSACTION;\n{ddl}\nCOMMIT TRANSACTION;\n"
                        ))
                        .await?;
                    verify_atelier_catalog(&database, &expected).await?;
                    return Ok(true);
                }

                if present_atelier_tables != expected_atelier_tables {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_ATELIER_SCHEMA_PARTIAL: expected={} present={} first_missing={} first_unexpected={}",
                            expected_atelier_tables.len(),
                            present_atelier_tables.len(),
                            expected_atelier_tables
                                .difference(&present_atelier_tables)
                                .next()
                                .map(String::as_str)
                                .unwrap_or("none"),
                            present_atelier_tables
                                .difference(&expected_atelier_tables)
                                .next()
                                .map(String::as_str)
                                .unwrap_or("none")
                        ),
                    )
                    .await;
                }

                verify_atelier_catalog(&database, &expected).await?;
                Ok(false)
            })
        })
        .await
}

#[derive(Debug, Default, PartialEq, Eq)]
struct AtelierTableDefinition {
    schemafull: bool,
    kind: String,
    is_view: bool,
}

#[derive(Debug, Default)]
struct ExpectedAtelierTable {
    definition: AtelierTableDefinition,
    fields: BTreeSet<String>,
    indexes: BTreeSet<String>,
    events: BTreeSet<String>,
}

fn atelier_expected_catalog() -> BTreeMap<String, ExpectedAtelierTable> {
    let mut catalog: BTreeMap<String, ExpectedAtelierTable> = BTreeMap::new();
    for line in atelier_schema_ddl().lines().map(str::trim_start) {
        if let Some(rest) = line.strip_prefix("DEFINE TABLE OVERWRITE ") {
            let table = rest.split_ascii_whitespace().next().unwrap_or_default();
            if !table.is_empty() {
                let expected = catalog.entry(table.to_owned()).or_default();
                expected.definition = AtelierTableDefinition {
                    schemafull: line.contains(" SCHEMAFULL"),
                    kind: if line.contains(" TYPE NORMAL") {
                        "NORMAL"
                    } else if line.contains(" TYPE RELATION") {
                        "RELATION"
                    } else if line.contains(" TYPE ANY") {
                        "ANY"
                    } else {
                        "NORMAL"
                    }
                    .to_owned(),
                    is_view: line.ends_with(" AS") || line.contains(" AS "),
                };
            }
            continue;
        }

        let (kind, rest) = if let Some(rest) = line.strip_prefix("DEFINE FIELD OVERWRITE ") {
            ("field", rest)
        } else if let Some(rest) = line.strip_prefix("DEFINE INDEX OVERWRITE ") {
            ("index", rest)
        } else if let Some(rest) = line.strip_prefix("DEFINE EVENT OVERWRITE ") {
            ("event", rest)
        } else {
            continue;
        };
        let Some((name, table_tail)) = rest.split_once(" ON TABLE ") else {
            continue;
        };
        let table = table_tail
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches('`');
        let expected = catalog.entry(table.to_owned()).or_default();
        match kind {
            "field" => {
                expected.fields.insert(name.to_owned());
            }
            "index" => {
                expected.indexes.insert(name.to_owned());
            }
            "event" => {
                expected.events.insert(name.to_owned());
            }
            _ => unreachable!(),
        }
    }
    catalog
}

async fn atelier_table_definitions(
    database: &SurrealAdminContext<'_>,
) -> Result<BTreeMap<String, AtelierTableDefinition>, SurrealStorageError> {
    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
    let info: SurrealValueData = response.take(0)?;
    match parse_table_definitions(&info) {
        Ok(definitions) => Ok(definitions),
        Err(reason) => fail_closed(database, reason).await,
    }
}

async fn verify_atelier_catalog(
    database: &SurrealAdminContext<'_>,
    expected: &BTreeMap<String, ExpectedAtelierTable>,
) -> Result<(), SurrealStorageError> {
    let expected_tables = expected.keys().cloned().collect::<BTreeSet<_>>();
    let expected_sequences = ATELIER_REQUIRED_SEQUENCES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    verify_atelier_catalog_fingerprint(
        database,
        &expected_tables,
        &expected_sequences,
        EXPECTED_ATELIER_CATALOG_SHA256,
    )
    .await
}

#[derive(Serialize)]
struct AtelierCatalogInfoEnvelope {
    table_definitions: BTreeMap<String, SurrealValueData>,
    sequence_definitions: BTreeMap<String, SurrealValueData>,
    table_members: BTreeMap<String, SurrealValueData>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    access_definitions: BTreeMap<String, SurrealValueData>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    function_definitions: BTreeMap<String, SurrealValueData>,
}

async fn verify_atelier_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
    expected_fingerprint: &str,
) -> Result<(), SurrealStorageError> {
    if expected_fingerprint.bytes().all(|byte| byte == b'0') {
        return fail_closed(
            database,
            "HANDSHAKE_ATELIER_SCHEMA_CATALOG_FINGERPRINT_UNPINNED".to_owned(),
        )
        .await;
    }
    let observed =
        inspect_atelier_catalog_fingerprint(database, expected_tables, expected_sequences).await?;
    if observed != expected_fingerprint {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_CATALOG_FINGERPRINT_MISMATCH: expected={expected_fingerprint}; observed={observed}"
            ),
        )
        .await;
    }
    Ok(())
}

async fn inspect_atelier_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
) -> Result<String, SurrealStorageError> {
    inspect_catalog_fingerprint(
        database,
        expected_tables,
        expected_sequences,
        CatalogInspectionScope::Atelier,
    )
    .await
}

#[derive(Clone, Copy)]
enum CatalogInspectionScope {
    Atelier,
    ExactDatabase,
}

async fn inspect_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
    scope: CatalogInspectionScope,
) -> Result<String, SurrealStorageError> {
    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
    let database_info: SurrealValueData = response.take(0)?;
    let table_definitions = match parse_named_structures(&database_info, "tables") {
        Ok(definitions) => definitions,
        Err(reason) => return fail_closed(database, reason).await,
    };
    let sequence_definitions = match parse_named_structures(&database_info, "sequences") {
        Ok(definitions) => definitions,
        Err(reason) => return fail_closed(database, reason).await,
    };

    let authority_dependencies = expected_tables.contains("local_accounts");
    let relevant_tables = table_definitions
        .keys()
        .filter(|name| match scope {
            CatalogInspectionScope::Atelier => {
                name.starts_with("atelier_")
                    || *name == "kernel_event_ledger"
                    || (authority_dependencies && expected_tables.contains(*name))
            }
            CatalogInspectionScope::ExactDatabase => true,
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if &relevant_tables != expected_tables {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_TABLE_SET_MISMATCH: expected={expected_tables:?} actual={relevant_tables:?}"
            ),
        )
        .await;
    }

    let relevant_sequences = sequence_definitions
        .keys()
        .filter(|name| match scope {
            CatalogInspectionScope::Atelier => {
                name.starts_with("atelier_") || *name == "kernel_event_sequence"
            }
            CatalogInspectionScope::ExactDatabase => true,
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if &relevant_sequences != expected_sequences {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_SEQUENCE_SET_MISMATCH: expected={expected_sequences:?} actual={relevant_sequences:?}"
            ),
        )
        .await;
    }

    let table_members = stream::iter(expected_tables.iter().cloned().map(|table| async move {
        let mut response = database
            .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
            .await?;
        let info: SurrealValueData = response.take(0)?;
        Ok::<_, SurrealStorageError>((table, canonicalize_info(info)))
    }))
    .buffer_unordered(ATELIER_CATALOG_INFO_CONCURRENCY)
    .try_collect::<Vec<_>>()
    .await?
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    let mut access_definitions = BTreeMap::new();
    let mut function_definitions = BTreeMap::new();
    if authority_dependencies {
        for (category, expected_names, selected) in [
            (
                "accesses",
                authority_catalog_names("access"),
                &mut access_definitions,
            ),
            (
                "functions",
                authority_catalog_names("function"),
                &mut function_definitions,
            ),
        ] {
            let actual = match parse_named_structures(&database_info, category) {
                Ok(definitions) => definitions,
                Err(reason) => return fail_closed(database, reason).await,
            };
            if matches!(scope, CatalogInspectionScope::ExactDatabase)
                && actual.keys().cloned().collect::<BTreeSet<_>>() != expected_names
            {
                return fail_closed(
                    database,
                    format!("HANDSHAKE_LOOM_SCHEMA_AUTHORITY_SET_MISMATCH: {category}"),
                )
                .await;
            }
            for name in expected_names {
                let Some(definition) = actual.get(&name) else {
                    return fail_closed(database, format!("HANDSHAKE_ATELIER_SCHEMA_AUTHORITY_DEPENDENCY_MISSING: {category}/{name}")).await;
                };
                selected.insert(name, canonicalize_info(definition.clone()));
            }
        }
    }
    let envelope = AtelierCatalogInfoEnvelope {
        access_definitions,
        function_definitions,
        table_definitions: expected_tables
            .iter()
            .filter_map(|name| {
                table_definitions
                    .get(name)
                    .cloned()
                    .map(strip_table_catalog_id)
                    .map(|definition| (name.clone(), canonicalize_info(definition)))
            })
            .collect(),
        sequence_definitions: expected_sequences
            .iter()
            .filter_map(|name| {
                sequence_definitions
                    .get(name)
                    .cloned()
                    .map(|definition| (name.clone(), canonicalize_info(definition)))
            })
            .collect(),
        table_members: table_members
            .into_iter()
            .map(|(name, info)| (name, strip_nested_table_catalog_ids(info)))
            .collect(),
    };
    let canonical_json = serde_json::to_string(&envelope)
        .expect("canonical Atelier structured INFO serializes losslessly");
    Ok(sha256_hex(canonical_json.as_bytes()))
}

fn atelier_schema_ddl() -> String {
    let mut ddl = Vec::new();
    let mut include_continuation = false;

    let authority = resource_authority_core_block();
    let bounded_source = SCHEMA.replacen(authority, "", 1);
    for line in bounded_source.lines() {
        let trimmed = line.trim_start();
        let starts_atelier_statement = trimmed.starts_with("DEFINE TABLE OVERWRITE atelier_")
            || trimmed.starts_with("DEFINE SEQUENCE IF NOT EXISTS atelier_")
            || (trimmed.starts_with("DEFINE FIELD OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"))
            || (trimmed.starts_with("DEFINE INDEX OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"))
            || (trimmed.starts_with("DEFINE EVENT OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"));
        let starts_event_ledger_dependency = trimmed
            .starts_with("DEFINE SEQUENCE IF NOT EXISTS kernel_event_sequence ")
            || trimmed.starts_with("DEFINE TABLE OVERWRITE kernel_event_ledger ")
            || ((trimmed.starts_with("DEFINE FIELD OVERWRITE ")
                || trimmed.starts_with("DEFINE INDEX OVERWRITE "))
                && trimmed.contains(" ON TABLE kernel_event_ledger"));

        if include_continuation || starts_atelier_statement || starts_event_ledger_dependency {
            ddl.push(line);
            include_continuation = !trimmed.ends_with(';');
        }
    }

    let mut ddl = ddl.join("\n");
    ddl.push('\n');
    ddl.push_str(authority);
    ddl.push('\n');
    ddl
}

/// Provisions the exact production-schema tables exercised by the focused Loom
/// mutation-receipt tests. Definitions are selected mechanically from the same
/// compiled `schema.surql` as production bootstrap; no test-owned DDL is used.
#[cfg(test)]
pub async fn bootstrap_loom_receipt_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    // FOURTH pin over the Loom receipt-test table set (not the whole schema). Re-pinned by
    // MT-152 (I-152-2): the value below is the catalog WITH MT-151's `loom_blocks.journal_key`
    // field and `uq_loom_blocks_journal_key` index, which MT-151 added inside this table set
    // without moving the pin (previous value 77ab023e..., pinned at e9b81814; run
    // mt142-LIB-20260911T234638Z, HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_FINGERPRINT_MISMATCH
    // observed). The MT-152 loom_folders DDL is outside this set.
    const EXPECTED_CATALOG_SHA256: &str =
        "2d3490115ab484a75ef8896bff1152b95798f3c4437fd45404898172aa87b1c2";
    let ddl = loom_receipt_test_schema_ddl();
    let expected_tables = loom_receipt_test_tables()
        .iter()
        .map(|table| (*table).to_owned())
        .collect::<BTreeSet<_>>();
    let expected_sequences = loom_receipt_test_sequences()
        .iter()
        .map(|sequence| (*sequence).to_owned())
        .collect::<BTreeSet<_>>();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                database
                    .query(format!("BEGIN TRANSACTION;\n{ddl}\nCOMMIT TRANSACTION;\n"))
                    .await?;
                let present_tables = atelier_table_definitions(&database)
                    .await?
                    .into_keys()
                    .collect::<BTreeSet<_>>();
                if present_tables != expected_tables {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_TABLE_MISMATCH: expected={expected_tables:?}; observed={present_tables:?}"
                        ),
                    )
                    .await;
                }
                let observed = inspect_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    CatalogInspectionScope::ExactDatabase,
                )
                .await?;
                if observed != EXPECTED_CATALOG_SHA256 {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_FINGERPRINT_MISMATCH: expected={EXPECTED_CATALOG_SHA256}; observed={observed}"
                        ),
                    )
                    .await;
                }
                Ok(())
            })
        })
        .await
}

#[cfg(test)]
fn loom_receipt_test_tables() -> &'static [&'static str] {
    &[
        "workspaces",
        "loom_blocks",
        "loom_block_search_index",
        "loom_canvas_boards",
        "loom_canvas_placements",
        "loom_canvas_visual_edges",
        "kernel_event_ledger",
        "knowledge_rich_documents",
        "local_accounts",
        "principals",
        "access_spaces",
        "authenticated_sessions",
        "session_exchange_credentials",
        "protected_resources",
        "resource_grants",
        "authorization_audit_events",
    ]
}

#[cfg(test)]
fn loom_receipt_test_sequences() -> &'static [&'static str] {
    &["kernel_event_sequence"]
}

#[cfg(test)]
fn loom_receipt_test_schema_ddl() -> String {
    fn selected_table_statement<'a>(line: &'a str, tables: &[&str]) -> bool {
        let table = if let Some(rest) = line.strip_prefix("DEFINE TABLE OVERWRITE ") {
            rest.split_ascii_whitespace().next()
        } else if line.starts_with("DEFINE FIELD OVERWRITE ")
            || line.starts_with("DEFINE INDEX OVERWRITE ")
            || line.starts_with("DEFINE EVENT OVERWRITE ")
        {
            line.split_once(" ON TABLE ")
                .and_then(|(_, rest)| rest.split_ascii_whitespace().next())
        } else {
            None
        };
        table.is_some_and(|table| tables.contains(&table.trim_end_matches(';')))
    }

    fn selected_sequence_statement(line: &str, sequences: &[&str]) -> bool {
        line.strip_prefix("DEFINE SEQUENCE IF NOT EXISTS ")
            .and_then(|rest| rest.split_ascii_whitespace().next())
            .is_some_and(|sequence| sequences.contains(&sequence.trim_end_matches(';')))
    }

    let mut selected_blocks = vec![resource_authority_core_block()];
    for block in resource_authority_schema_blocks() {
        let first = block.trim_start().lines().next().unwrap_or_default();
        if block != resource_authority_core_block()
            && selected_table_statement(first, loom_receipt_test_tables())
        {
            selected_blocks.push(block);
        }
    }
    let mut bounded_source = SCHEMA.to_owned();
    for block in &selected_blocks {
        bounded_source = bounded_source.replacen(block, "", 1);
    }
    let mut ddl = selected_blocks
        .iter()
        .map(|block| (*block).to_owned())
        .collect::<Vec<_>>();
    let mut include_continuation = false;
    for line in bounded_source.lines() {
        let trimmed = line.trim_start();
        if include_continuation
            || selected_table_statement(trimmed, loom_receipt_test_tables())
            || selected_sequence_statement(trimmed, loom_receipt_test_sequences())
        {
            ddl.push(line.to_owned());
            include_continuation = !trimmed.ends_with(';');
        }
    }
    let mut ddl = ddl.join("\n");
    ddl.push('\n');
    ddl
}

/// Provisions only the authoritative process-ledger schema wave for focused
/// restart/durability proofs. The DDL is sliced from the same compiled
/// `schema.surql` used by production bootstrap, so this test-support path
/// cannot drift into a hand-maintained substitute schema.
#[cfg(feature = "surreal-test-support")]
pub async fn bootstrap_mt137_process_ledger_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    const START: &str = "-- 0021_kernel_process_lifecycle";
    const END: &str = "-- 0022_role_mailbox_threads_messages";
    bootstrap_mt137_test_schema_slice(storage, START, END).await
}

/// Provisions the authoritative EventLedger and aggregate-query schema waves
/// needed by the focused MT-137 Flight Recorder append/reopen/read proof.
#[cfg(feature = "surreal-test-support")]
pub async fn bootstrap_mt137_flight_recorder_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    const START: &str = "-- 0018_kernel_event_ledger";
    const END: &str =
        "-- 0030-0059 Atelier schema projection. Historical legacy server backend backfill DML is";
    bootstrap_mt137_test_schema_slice(storage, START, END).await
}

#[cfg(feature = "surreal-test-support")]
async fn bootstrap_mt137_test_schema_slice(
    storage: &SurrealStorage,
    start: &'static str,
    end: &'static str,
) -> Result<(), SurrealStorageError> {
    let (_, after_start) = SCHEMA
        .split_once(start)
        .expect("compiled Surreal schema contains the focused MT-137 schema start");
    let (ddl, _) = after_start
        .split_once(end)
        .expect("compiled Surreal schema contains the focused MT-137 schema end");
    let ddl = ddl.to_owned();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                database.query(ddl).await?;
                Ok(())
            })
        })
        .await
}

const TABLE_NAMES: [&str; TABLE_DEFINITION_COUNT] = [
    "access_spaces",
    "adapter_checkpoint",
    "ai_bronze_records",
    "ai_embedding_models",
    "ai_embedding_registry",
    "ai_job_mcp_fields",
    "ai_jobs",
    "ai_silver_records",
    "assets",
    "atelier_action_receipt",
    "atelier_ai_tag_suggestion",
    "atelier_anchor_verification_record",
    "atelier_backup_manifest",
    "atelier_backup_restore_preflight",
    "atelier_bracket_link_projection",
    "atelier_bulk_operation_receipt",
    "atelier_caption_artifact",
    "atelier_character",
    "atelier_character_document",
    "atelier_character_document_version",
    "atelier_character_relationship",
    "atelier_character_relationship_graph_projection",
    "atelier_character_script",
    "atelier_character_tag",
    "atelier_collection",
    "atelier_collection_item",
    "atelier_collection_metadata_application",
    "atelier_comfy_bridge_probe",
    "atelier_comfy_capability_registration",
    "atelier_comfy_capability_reject",
    "atelier_comfy_declared_output",
    "atelier_comfy_diagnostic_bundle",
    "atelier_comfy_fallback_marker",
    "atelier_comfy_intake_output",
    "atelier_comfy_job",
    "atelier_comfy_output_registration_failure",
    "atelier_comfy_version_metadata",
    "atelier_comfy_workflow_receipt",
    "atelier_comfy_workflow_spec",
    "atelier_command_corpus_blocked",
    "atelier_command_corpus_entry",
    "atelier_command_corpus_parity_report",
    "atelier_command_log",
    "atelier_contact_sheet",
    "atelier_contact_sheet_raster_export_plan",
    "atelier_contact_sheet_svg_artifact",
    "atelier_dcc_panel_projection",
    "atelier_dcc_workflow_panel_projection",
    "atelier_diagnostics_error_taxonomy",
    "atelier_diagnostics_prompt_response_matrix",
    "atelier_diagnostics_session",
    "atelier_diagnostics_validation_matrix",
    "atelier_event",
    "atelier_export_intake_link",
    "atelier_export_manifest_entry",
    "atelier_export_request",
    "atelier_export_result",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_fr_workflow_event",
    "atelier_handler_version_matrix",
    "atelier_identity_crop_artifact",
    "atelier_identity_profile",
    "atelier_image_import_request",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_intake_item_loom_projection",
    "atelier_intake_item_rejection_audit",
    "atelier_md_allowlist_policy",
    "atelier_md_auth_context",
    "atelier_md_checkpoint",
    "atelier_md_download_session",
    "atelier_md_item_state",
    "atelier_md_output_root",
    "atelier_md_session_receipt",
    "atelier_media_annotation",
    "atelier_media_asset",
    "atelier_media_asset_tag",
    "atelier_media_derivative",
    "atelier_media_probe_report",
    "atelier_media_review_metadata",
    "atelier_media_sidecar",
    "atelier_media_source_provenance_ref",
    "atelier_model_apply",
    "atelier_model_config",
    "atelier_model_coordination_lease",
    "atelier_model_manual_drift_guard",
    "atelier_model_manual_row_merge",
    "atelier_model_manual_section",
    "atelier_moodboard",
    "atelier_moodboard_export_request",
    "atelier_moodboard_operation_receipt",
    "atelier_orphan_manifest",
    "atelier_orphan_manifest_item",
    "atelier_pose_calibration",
    "atelier_pose_context_state",
    "atelier_pose_deferred_feature",
    "atelier_pose_head_pose",
    "atelier_pose_rig",
    "atelier_pose_sidecar",
    "atelier_pose_workspace_rig_state",
    "atelier_preference",
    "atelier_reset_operation",
    "atelier_retrieval_policy",
    "atelier_saved_search",
    "atelier_screenshot_artifact_storage",
    "atelier_self_improve_sandbox_run",
    "atelier_sheet_parse_snapshot",
    "atelier_sheet_version",
    "atelier_similarity_projection",
    "atelier_similarity_rebuild_job",
    "atelier_source_evidence_record",
    "atelier_sourcing_binding_decision",
    "atelier_sourcing_ingestion_receipt",
    "atelier_sourcing_spec",
    "atelier_spec_drift_finding",
    "atelier_state_probe_catalog_entry",
    "atelier_stealth_capture",
    "atelier_stealth_ref",
    "atelier_stealth_window",
    "atelier_story_beat",
    "atelier_story_card",
    "atelier_synthetic_input_guard",
    "atelier_tag",
    "atelier_tag_rule",
    "atelier_transcript_artifact",
    "atelier_transcript_receipt",
    "atelier_trash_marker",
    "atelier_validator_first_pass_run",
    "atelier_version_mismatch_receipt",
    "atelier_visual_steer_feedback",
    "atelier_web_portfolio_export_request",
    "atelier_web_portfolio_export_result",
    "atelier_work_state_projection",
    "authenticated_sessions",
    "authorization_audit_events",
    "blocks",
    "calendar_activity_spans",
    "calendar_events",
    "calendar_mutation_outbox",
    "calendar_sources",
    "canvas_edges",
    "canvas_nodes",
    "canvases",
    "dependencies",
    "distill_example",
    "distill_job",
    "documents",
    "eval_run",
    "fems_memory_commit_fr_outbox",
    "fems_memory_commit_reports",
    "fems_memory_items",
    "fems_memory_lifecycle_fr_outbox",
    "fems_memory_packs",
    "fems_memory_proposal_request_id_rekey",
    "fems_memory_proposals",
    "fems_workspace_write_anchors",
    "governance_check_runs",
    "handshake_schema_state",
    "kb003_promotion_decisions",
    "kb003_promotion_receipts",
    "kb003_sandbox_policies",
    "kb003_sandbox_runs",
    "kb003_validation_runs",
    "kernel_activity_span",
    "kernel_crdt_snapshots",
    "kernel_crdt_updates",
    "kernel_diagnostic_bundle_manifest",
    "kernel_distillation_candidate",
    "kernel_event_ledger",
    "kernel_idempotency_ledger",
    "kernel_micro_task_job",
    "kernel_model_session_span",
    "kernel_mt_loop_checkpoint",
    "kernel_mt_outcome",
    "kernel_process_lifecycle",
    "kernel_restart_resume_report",
    "kernel_session_checkpoint",
    "kernel_session_queue",
    "kernel_visual_diff_baseline",
    "kernel_visual_diff_request",
    "kernel_visual_diff_result",
    "knowledge_agent_cloud_assistance_receipts",
    "knowledge_agent_quiet_background_work",
    "knowledge_agent_recovery_receipts",
    "knowledge_agent_role_mailbox_handoffs",
    "knowledge_agent_state_recovery_checkpoints",
    "knowledge_agent_worktree_claims",
    "knowledge_claim_conflicts",
    "knowledge_claim_spans",
    "knowledge_claims",
    "knowledge_code_files",
    "knowledge_code_repair_queue",
    "knowledge_code_scip_imports",
    "knowledge_context_bundle_items",
    "knowledge_context_bundles",
    "knowledge_crdt_agent_lane_leases",
    "knowledge_crdt_ai_edit_proposals",
    "knowledge_crdt_denial_receipts",
    "knowledge_crdt_graph_proposals",
    "knowledge_crdt_promoted_facts",
    "knowledge_crdt_recovery_receipts",
    "knowledge_crdt_swarm_checkpoints",
    "knowledge_debug_breakpoints",
    "knowledge_document_backlinks",
    "knowledge_document_embeds",
    "knowledge_edge_spans",
    "knowledge_edges",
    "knowledge_editor_code_nodes",
    "knowledge_entities",
    "knowledge_entity_spans",
    "knowledge_idempotency_keys",
    "knowledge_index_runs",
    "knowledge_ingestion_kind_registry",
    "knowledge_ingestion_policy_decisions",
    "knowledge_ingestion_receipts",
    "knowledge_ingestion_repair_queue",
    "knowledge_ingestion_root_policies",
    "knowledge_ingestion_spans",
    "knowledge_memory_bridge_decisions",
    "knowledge_memory_conflict_detection_findings",
    "knowledge_memory_conflict_detection_jobs",
    "knowledge_memory_conflict_resolution_jobs",
    "knowledge_memory_facts",
    "knowledge_memory_ontology_aliases",
    "knowledge_memory_ontology_terms",
    "knowledge_memory_passages",
    "knowledge_parallel_indexing_lease_queue",
    "knowledge_passage_evidence",
    "knowledge_quick_switcher_recents",
    "knowledge_retrieval_traces",
    "knowledge_rich_document_drafts",
    "knowledge_rich_document_loom_projection_0343_state",
    "knowledge_rich_document_title_anchors",
    "knowledge_rich_document_versions",
    "knowledge_rich_documents",
    "knowledge_schema_registry",
    "knowledge_semantic_catalog_entries",
    "knowledge_source_roots",
    "knowledge_sources",
    "knowledge_spans",
    "knowledge_wiki_projections",
    "knowledge_workbench_layout_states",
    "knowledge_workspace_search_bookmark_states",
    "knowledge_workspace_settings_states",
    "local_accounts",
    "loom_ai_suggestions",
    "loom_block_knowledge_bridge",
    "loom_block_search_index",
    "loom_block_view_fr_outbox",
    "loom_blocks",
    "loom_canvas_boards",
    "loom_canvas_placements",
    "loom_canvas_visual_edges",
    "loom_collection_members",
    "loom_collections",
    "loom_edges",
    "loom_folder_members",
    "loom_folders",
    "loom_wiki_overlays",
    "media_asset_tiers",
    "micro_tasks",
    "model_session_checkpoints",
    "model_session_messages",
    "model_sessions",
    "mt_iterations",
    "preference_change_receipts",
    "preference_records",
    "principals",
    "protected_resources",
    "replay_candidates",
    "resource_grants",
    "role_mailbox_claim_lease",
    "role_mailbox_handoff_bundle",
    "role_mailbox_message",
    "role_mailbox_thread",
    "session_exchange_credentials",
    "skill_log_entry",
    "skill_log_file_ref",
    "stage_capture_artifacts",
    "storage_graph_anchors",
    "user_manual_anchors",
    "user_manual_feature_entries",
    "user_manual_legacy_aliases",
    "user_manual_pages",
    "user_manual_sections",
    "user_manual_tool_entries",
    "user_manual_versions",
    "work_packets",
    "workflow_node_executions",
    "workflow_runs",
    "workspaces",
];

/// Tables whose source `id` column is represented only by the Surreal record ID.
const RECORD_ID_ONLY_TABLES: [&str; 18] = [
    "workspaces",
    "documents",
    "blocks",
    "canvases",
    "canvas_nodes",
    "canvas_edges",
    "ai_jobs",
    "workflow_runs",
    "workflow_node_executions",
    "ai_embedding_registry",
    "calendar_sources",
    "calendar_events",
    "skill_log_entry",
    "skill_log_file_ref",
    "distill_job",
    "adapter_checkpoint",
    "eval_run",
    "preference_change_receipts",
];

/// Referenced targets that retain a domain-facing single-column key alias.
/// Each corresponding field ASSERTs equality with `record::id($this.id)`.
const REFERENCED_BUSINESS_KEY_ALIASES: [(&str, &str); 9] = [
    ("ai_bronze_records", "bronze_id"),
    ("assets", "asset_id"),
    ("loom_blocks", "block_id"),
    ("work_packets", "wp_id"),
    ("kernel_event_ledger", "event_id"),
    ("role_mailbox_thread", "thread_id"),
    ("role_mailbox_claim_lease", "lease_id"),
    ("kernel_micro_task_job", "job_id"),
    ("kernel_model_session_span", "span_id"),
];

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct SchemaState {
    version: String,
    revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
    info_fingerprint_sha256: String,
    apply_state: String,
    target_revision: i64,
}

#[derive(SurrealValue)]
struct BootstrapBindings {
    schema_version: String,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
}

#[derive(SurrealValue)]
struct FinalizeBindings {
    schema_version: String,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
    pending_info_fingerprint_sha256: String,
    info_fingerprint_sha256: String,
}

#[derive(SurrealValue)]
struct PredecessorUpgradeBindings {
    schema_version: String,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    predecessor_generated_surql_sha256: String,
    predecessor_info_fingerprint_sha256: String,
    generated_surql_sha256: String,
    pending_info_fingerprint_sha256: String,
    schema_source: String,
}

impl SchemaState {
    fn has_stable_v1_identity(&self) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == SCHEMA_REVISION
            && self.target_revision == SCHEMA_REVISION
            && self.namespace == DEFAULT_NAMESPACE
            && self.database == DEFAULT_DATABASE
            && self.source_manifest_sha256 == SCHEMA_LINEAGE_SHA256
    }

    fn is_schema_applied_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == GENERATED_SURREALQL_SHA256
            && self.apply_state == "schema_applied"
            && self.info_fingerprint_sha256 == PENDING_SCHEMA_INFO_SHA256
    }

    fn is_exact_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == EXPECTED_SCHEMA_INFO_SHA256
    }

    fn is_exact_pre_mt109_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PRE_MT109_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT109_SCHEMA_INFO_SHA256
    }

    fn is_exact_supported_predecessor(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PREDECESSOR_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PREDECESSOR_SCHEMA_INFO_SHA256
    }

    /// Exact pre-MT-142 current lineage (revision 157 without the title-anchor table).
    fn is_exact_pre_mt142_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PRE_MT142_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT142_SCHEMA_INFO_SHA256
    }

    /// Exact MT-142 current lineage (revision 157 with the title-anchor table, before the
    /// MT-151 journal key and graph anchors).
    fn is_exact_pre_mt151_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PRE_MT151_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT151_SCHEMA_INFO_SHA256
    }

    /// Exact MT-151 current lineage (revision 157 with journal_key and graph anchors, before
    /// the MT-152 FEMS workspace write anchors).
    fn is_exact_pre_mt152_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PRE_MT152_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT152_SCHEMA_INFO_SHA256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaBootstrapOutcome {
    InstalledFresh,
    ReusedExactCurrent,
    ResumedCurrentApply,
    UpgradedSupportedPredecessor,
}

impl SchemaBootstrapOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InstalledFresh => "installed_fresh",
            Self::ReusedExactCurrent => "reused_exact_current",
            Self::ResumedCurrentApply => "resumed_current_apply",
            Self::UpgradedSupportedPredecessor => "upgraded_supported_predecessor",
        }
    }

    const fn reused_existing_schema(self) -> bool {
        !matches!(self, Self::InstalledFresh)
    }
}

/// Receipt derived from the durable state row and live INFO introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaBootstrapReport {
    pub schema_version: String,
    pub namespace: String,
    pub database: String,
    pub declarative_schema_files: usize,
    pub source_manifest_sha256: String,
    pub generated_surql_sha256: String,
    pub info_fingerprint_sha256: String,
    pub tables_defined: usize,
    pub fields_defined: usize,
    pub indexes_defined: usize,
    pub table_names: Vec<String>,
    pub outcome: SchemaBootstrapOutcome,
    /// Compatibility projection. Prefer [`SchemaBootstrapReport::outcome`] when mutation matters.
    pub reused_existing_schema: bool,
}

#[derive(Debug)]
struct ObservedSchema {
    info_fingerprint_sha256: String,
    tables_defined: usize,
    fields_defined: usize,
    indexes_defined: usize,
    table_names: Vec<String>,
}

#[derive(Serialize)]
struct CanonicalInfoEnvelope {
    database: SurrealValueData,
    tables: BTreeMap<String, SurrealValueData>,
}

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct KnowledgeSchemaRegistryMetadata {
    family_key: String,
    table_name: String,
    record_family: String,
    authority_class: String,
    schema_source: String,
    wp_id: String,
    mt_id: String,
}

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct PredecessorKnowledgeSchemaRegistryMetadata {
    family_key: String,
    table_name: String,
    record_family: String,
    authority_class: String,
    retired_source: String,
    wp_id: String,
    mt_id: String,
}

fn expected_knowledge_schema_registry_metadata(
) -> Result<Vec<KnowledgeSchemaRegistryMetadata>, String> {
    let mut rows = Vec::new();
    for line in KNOWLEDGE_SCHEMA_REGISTRY_SEED.lines() {
        let line = line.trim();
        if !line.starts_with("{ family_key:") {
            continue;
        }
        let values = line
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_PARSE_FAILED: {line}"
            ));
        }
        rows.push(KnowledgeSchemaRegistryMetadata {
            family_key: values[0].clone(),
            table_name: values[1].clone(),
            record_family: values[2].clone(),
            authority_class: values[3].clone(),
            schema_source: values[4].clone(),
            wp_id: values[5].clone(),
            mt_id: values[6].clone(),
        });
    }
    rows.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    // MT-142 re-pin: 61 historical rows + 0343 state row + rich_document_title_anchors row.
    if rows.len() != 63 {
        return Err(format!(
            "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_COUNT: expected=63 observed={}",
            rows.len()
        ));
    }
    Ok(rows)
}

fn expected_predecessor_registry_metadata(
) -> Result<Vec<PredecessorKnowledgeSchemaRegistryMetadata>, String> {
    let mut rows = Vec::new();
    let mut family_keys = BTreeSet::new();
    for line in PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED.lines() {
        let line = line.trim();
        if !line.starts_with("['") {
            continue;
        }
        let values = line
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_SEED_PARSE_FAILED: {line}"
            ));
        }
        if !family_keys.insert(values[0].clone()) {
            return Err(format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_FAMILY_DUPLICATE: {}",
                values[0]
            ));
        }
        rows.push(PredecessorKnowledgeSchemaRegistryMetadata {
            family_key: values[0].clone(),
            table_name: values[1].clone(),
            record_family: values[2].clone(),
            authority_class: values[3].clone(),
            retired_source: values[4].clone(),
            wp_id: values[5].clone(),
            mt_id: values[6].clone(),
        });
    }
    rows.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    if rows.len() != 61 {
        return Err(format!(
            "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_SEED_COUNT: expected=61 observed={}",
            rows.len()
        ));
    }
    Ok(rows)
}

async fn read_knowledge_schema_registry_metadata(
    database: &SurrealAdminContext<'_>,
) -> Result<Vec<KnowledgeSchemaRegistryMetadata>, SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT family_key, table_name, record_family, authority_class, schema_source, \
             wp_id, mt_id FROM knowledge_schema_registry ORDER BY family_key ASC;",
        )
        .await?;
    Ok(response.take(0)?)
}

fn compute_predecessor_registry_hash(
    rows: &[PredecessorKnowledgeSchemaRegistryMetadata],
) -> String {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    let mut hasher = Sha256::new();
    hasher.update(PREDECESSOR_KNOWLEDGE_REGISTRY_DOMAIN);
    for row in sorted {
        for field in [
            row.family_key.as_str(),
            row.table_name.as_str(),
            row.record_family.as_str(),
            row.authority_class.as_str(),
            row.retired_source.as_str(),
            row.wp_id.as_str(),
            row.mt_id.as_str(),
        ] {
            hasher.update((field.len() as u32).to_be_bytes());
            hasher.update(field.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

async fn ensure_supported_predecessor_registry(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT family_key, table_name, record_family, authority_class, \
             migration_file AS retired_source, wp_id, mt_id \
             FROM knowledge_schema_registry ORDER BY family_key ASC;",
        )
        .await?;
    let observed: Vec<PredecessorKnowledgeSchemaRegistryMetadata> = response.take(0)?;
    if observed.len() != 61
        || observed.iter().any(|row| row.retired_source.is_empty())
        || compute_predecessor_registry_hash(&observed) != PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_KNOWLEDGE_REGISTRY_DIVERGENT: rows={}; sha256={}",
                observed.len(),
                compute_predecessor_registry_hash(&observed)
            ),
        )
        .await;
    }
    Ok(())
}

async fn ensure_knowledge_schema_registry(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let expected = match expected_knowledge_schema_registry_metadata() {
        Ok(expected) => expected,
        Err(reason) => return fail_closed(database, reason).await,
    };
    let observed = read_knowledge_schema_registry_metadata(database).await?;
    if observed == expected {
        return Ok(());
    }
    if !observed.is_empty() {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_DIVERGENT: expected={expected:?}; observed={observed:?}"
            ),
        )
        .await;
    }
    database.query(KNOWLEDGE_SCHEMA_REGISTRY_SEED).await?;
    let seeded = read_knowledge_schema_registry_metadata(database).await?;
    if seeded != expected {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_VERIFY_FAILED: expected={expected:?}; observed={seeded:?}"
            ),
        )
        .await;
    }
    Ok(())
}

/// Installs the sole declarative Surreal schema or verifies an exact-current schema.
///
/// V1 fails closed for every lower, divergent, or unknown lineage. One exact allowlisted
/// predecessor is upgraded transactionally from its retired registry field to the declarative
/// `schema_source` field; no deleted migration file is read or executed. The sole resumable
/// incomplete state is the exact-current `schema_applied` receipt written after committed DDL or
/// predecessor upgrade. It is finalized only after complete live INFO matches the compiled
/// fingerprint. A process-wide mutex serializes callers; each transaction rechecks durable state
/// before mutation. Exact-current restarts return before executing any `OVERWRITE` statement.
pub async fn bootstrap_schema(
    storage: &SurrealStorage,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let _bootstrap_guard = BOOTSTRAP_MUTEX.lock().await;
    let report = storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                verify_compiled_manifest(&database).await?;
                let existing = read_context_and_state(&database).await?;
                let mut verified_observed = None;
                let outcome = match existing {
                    None => {
                        database
                            .query_bound(
                                SCHEMA,
                                BootstrapBindings {
                                    schema_version: SCHEMA_VERSION.to_owned(),
                                    schema_revision: SCHEMA_REVISION,
                                    namespace: DEFAULT_NAMESPACE.to_owned(),
                                    database: DEFAULT_DATABASE.to_owned(),
                                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                    generated_surql_sha256:
                                        GENERATED_SURREALQL_SHA256.to_owned(),
                                },
                            )
                            .await?;
                        let applied_state = match read_context_and_state(&database).await? {
                            Some(state) if state.is_schema_applied_current() => state,
                            Some(state) => {
                                return fail_closed(
                                    &database,
                                    format!(
                                        "HANDSHAKE_SURREAL_SCHEMA_APPLY_STATE_MISMATCH: {state:?}"
                                    ),
                                )
                                .await;
                            }
                            None => {
                                return fail_closed(
                                    &database,
                                    "HANDSHAKE_SURREAL_SCHEMA_APPLY_STATE_MISSING".to_owned(),
                                )
                                .await;
                            }
                        };
                        ensure_knowledge_schema_registry(&database).await?;
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &applied_state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        verified_observed = Some(observed);
                        SchemaBootstrapOutcome::InstalledFresh
                    }
                    Some(state) if state.is_schema_applied_current() => {
                        ensure_knowledge_schema_registry(&database).await?;
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        verified_observed = Some(observed);
                        SchemaBootstrapOutcome::ResumedCurrentApply
                    }
                    Some(state) if state.is_exact_current() => {
                        ensure_knowledge_schema_registry(&database).await?;
                        SchemaBootstrapOutcome::ReusedExactCurrent
                    }
                    Some(state) if state.is_exact_pre_mt109_current() => {
                        verified_observed = Some(upgrade_pre_mt109_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_supported_predecessor() => {
                        verified_observed =
                            Some(upgrade_supported_predecessor(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt142_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt142_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt151_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt151_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt152_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt152_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) => {
                        return fail_closed(
                            &database,
                            format!(
                                "HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE: observed={state:?}; expected_revision={SCHEMA_REVISION}"
                            ),
                        )
                        .await;
                    }
                };

                let state = match read_context_and_state(&database).await? {
                    Some(state) if state.is_exact_current() => state,
                    Some(state) => {
                        return fail_closed(
                            &database,
                            format!(
                                "HANDSHAKE_SURREAL_SCHEMA_POST_APPLY_STATE_MISMATCH: {state:?}"
                            ),
                        )
                        .await;
                    }
                    None => {
                        return fail_closed(
                            &database,
                            "HANDSHAKE_SURREAL_SCHEMA_POST_APPLY_STATE_MISSING".to_owned(),
                        )
                        .await;
                    }
                };

                match verified_observed {
                    Some(observed) => {
                        report_from_observed(&database, state, observed, outcome)
                            .await
                    }
                    None => observe_schema(&database, state, outcome).await,
                }
            })
        })
        .await?;
    tracing::info!(
        target: "handshake_core",
        schema_bootstrap_outcome = report.outcome.as_str(),
        schema_version = %report.schema_version,
        generated_surql_sha256 = %report.generated_surql_sha256,
        info_fingerprint_sha256 = %report.info_fingerprint_sha256,
        "surreal_schema_bootstrap_complete"
    );
    Ok(report)
}

pub fn compute_generated_surql_sha256() -> String {
    sha256_hex(SCHEMA.as_bytes())
}

pub fn compute_declarative_schema_catalog_sha256() -> Result<String, String> {
    compiled_schema_catalog_entries().map(|entries| compute_catalog_hash(&entries))
}

pub fn compute_knowledge_schema_registry_seed_sha256() -> String {
    sha256_hex(KNOWLEDGE_SCHEMA_REGISTRY_SEED.as_bytes())
}

fn compiled_schema_catalog_entries() -> Result<Vec<String>, String> {
    let mut entries = BTreeSet::new();
    let mut tables = BTreeSet::new();
    let mut fields = 0usize;
    let mut indexes = 0usize;
    let mut events = 0usize;
    let mut views = 0usize;
    let mut sequences = 0usize;
    let mut accesses = 0usize;
    let mut functions = 0usize;

    for raw_line in SCHEMA.lines() {
        let line = raw_line.trim();
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let identity = match tokens.as_slice() {
            ["DEFINE", "TABLE", "OVERWRITE", name, ..] => {
                let name = name.trim_end_matches(';');
                tables.insert(name.to_owned());
                if line.contains(" TYPE NORMAL AS") {
                    views += 1;
                    insert_catalog_identity(&mut entries, format!("view:{name}"))?;
                }
                Some(format!("table:{name}"))
            }
            ["DEFINE", "FIELD", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                fields += 1;
                Some(format!(
                    "field:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "INDEX", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                indexes += 1;
                Some(format!(
                    "index:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "EVENT", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                events += 1;
                Some(format!(
                    "event:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "SEQUENCE", "OVERWRITE", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "SEQUENCE", "IF", "NOT", "EXISTS", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "ACCESS", "OVERWRITE", name, "ON", "DATABASE", ..]
            | ["DEFINE", "ACCESS", "IF", "NOT", "EXISTS", name, "ON", "DATABASE", ..] => {
                accesses += 1;
                Some(format!("access:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "FUNCTION", "OVERWRITE", signature, ..] => {
                functions += 1;
                Some(format!(
                    "function:{}",
                    declarative_function_name(signature)?
                ))
            }
            ["DEFINE", "ACCESS" | "FUNCTION", ..] => {
                return Err(format!(
                    "unsupported declarative authority definition: {line}"
                ));
            }
            _ => None,
        };
        if let Some(identity) = identity {
            insert_catalog_identity(&mut entries, identity)?;
        }
    }

    let expected_tables = TABLE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    if tables != expected_tables {
        return Err(format!(
            "declarative table inventory differs from TABLE_NAMES: parsed={}; expected={}",
            tables.len(),
            expected_tables.len()
        ));
    }
    let observed_counts = (
        tables.len(),
        fields,
        indexes,
        events,
        views,
        sequences,
        accesses,
        functions,
    );
    let expected_counts = (
        TABLE_DEFINITION_COUNT,
        AUTHORED_FIELD_DEFINITION_COUNT,
        INDEX_DEFINITION_COUNT,
        EVENT_DEFINITION_COUNT,
        VIEW_DEFINITION_COUNT,
        SEQUENCE_DEFINITION_COUNT,
        ACCESS_DEFINITION_COUNT,
        FUNCTION_DEFINITION_COUNT,
    );
    if observed_counts != expected_counts {
        return Err(format!(
            "declarative schema catalog counts differ: observed={observed_counts:?}; expected={expected_counts:?}"
        ));
    }

    Ok(entries.into_iter().collect())
}

fn declarative_function_name(signature: &str) -> Result<&str, String> {
    let (name, _) = signature
        .split_once('(')
        .ok_or_else(|| format!("function declaration lacks argument opener: {signature}"))?;
    if !name.starts_with("fn::") || name.len() == 4 {
        return Err(format!("invalid declarative function name: {signature}"));
    }
    Ok(name)
}

fn resource_authority_schema_blocks() -> Vec<&'static str> {
    let blocks = SCHEMA
        .split("-- MT109_AUTHORITY_BEGIN\n")
        .skip(1)
        .map(|tail| {
            tail.split_once("\n-- MT109_AUTHORITY_END")
                .expect("canonical authority block has an end marker")
                .0
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blocks.len(),
        14,
        "canonical schema must contain all fourteen authority blocks"
    );
    assert!(
        blocks.iter().all(|block| !block.trim().is_empty()),
        "authority blocks must not be empty"
    );
    blocks
}

pub(super) fn resource_authority_schema_statements() -> String {
    resource_authority_schema_blocks().join("\n")
}

fn resource_authority_upgrade_statements() -> String {
    format!(
        "{}\n{}",
        resource_authority_schema_statements(),
        MT109_LOOM_SOURCE_BACKFILL
    )
}

const MT109_LOOM_SOURCE_BACKFILL: &str = r#"
LET $documents = SELECT id, rich_document_id, workspace_id FROM knowledge_rich_documents;
IF array::len($documents) > 0 {
    FOR $document IN $documents {
        LET $blocks = SELECT * FROM loom_blocks
            WHERE id = type::record('loom_blocks', $document.rich_document_id) LIMIT 1;
        IF array::len($blocks) > 0 {
            LET $block = $blocks[0];
            IF $block.workspace_id != $document.workspace_id OR $block.content_type != 'note'
                OR ($block.source_rich_document_id != NONE AND $block.source_rich_document_id != $document.id) {
                THROW 'HSK-MT109-LOOM-SOURCE-IDENTITY';
            };
            UPDATE type::record('loom_blocks', $document.rich_document_id)
                SET source_rich_document_id = $document.id RETURN NONE;
        };
    };
};
"#;

fn resource_authority_core_block() -> &'static str {
    let blocks = resource_authority_schema_blocks()
        .into_iter()
        .filter(|block| {
            block
                .trim_start()
                .starts_with("DEFINE TABLE OVERWRITE local_accounts ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blocks.len(),
        1,
        "exactly one canonical authority core block"
    );
    assert!(blocks[0].contains("DEFINE ACCESS IF NOT EXISTS authenticated_session ON DATABASE"));
    blocks[0]
}

fn authority_catalog_names(kind: &str) -> BTreeSet<String> {
    resource_authority_core_block()
        .lines()
        .filter_map(|line| {
            let tokens = line.split_ascii_whitespace().collect::<Vec<_>>();
            match (kind, tokens.as_slice()) {
                (
                    "access",
                    ["DEFINE", "ACCESS", "IF", "NOT", "EXISTS", name, "ON", "DATABASE", ..],
                ) => Some((*name).to_owned()),
                ("function", ["DEFINE", "FUNCTION", "OVERWRITE", signature, ..]) => Some(
                    declarative_function_name(signature)
                        .expect("canonical function signature")
                        .trim_start_matches("fn::")
                        .to_owned(),
                ),
                _ => None,
            }
        })
        .collect()
}

fn insert_catalog_identity(entries: &mut BTreeSet<String>, identity: String) -> Result<(), String> {
    if entries.insert(identity.clone()) {
        Ok(())
    } else {
        Err(format!("duplicate declarative schema identity: {identity}"))
    }
}

fn compute_catalog_hash(entries: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DECLARATIVE_SCHEMA_CATALOG_DOMAIN);
    let mut sorted = entries.to_vec();
    sorted.sort();
    for identity in sorted {
        hasher.update((identity.len() as u32).to_be_bytes());
        hasher.update(identity.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
fn generated_collection_subtype_field_count(schema: &str) -> usize {
    schema
        .lines()
        .filter(|line| line.starts_with("DEFINE FIELD OVERWRITE "))
        .map(|line| line.matches("array<").count() + line.matches("set<").count())
        .sum()
}

async fn verify_compiled_manifest(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let catalog = match compute_declarative_schema_catalog_sha256() {
        Ok(catalog) => catalog,
        Err(reason) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_DECLARATIVE_CATALOG_INVALID: {reason}"),
            )
            .await;
        }
    };
    let generated = compute_generated_surql_sha256();
    let registry_seed = compute_knowledge_schema_registry_seed_sha256();
    let predecessor_registry = match expected_predecessor_registry_metadata() {
        Ok(rows) => compute_predecessor_registry_hash(&rows),
        Err(reason) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_MANIFEST_INVALID: {reason}"),
            )
            .await;
        }
    };
    if catalog != DECLARATIVE_SCHEMA_CATALOG_SHA256
        || generated != GENERATED_SURREALQL_SHA256
        || registry_seed != KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        || predecessor_registry != PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_COMPILED_MANIFEST_DRIFT: catalog={catalog}; generated={generated}; knowledge_registry_seed={registry_seed}; predecessor_registry={predecessor_registry}"
            ),
        )
        .await;
    }
    Ok(())
}

async fn read_context_and_state(
    database: &SurrealAdminContext<'_>,
) -> Result<Option<SchemaState>, SurrealStorageError> {
    let mut response = database
        .query("RETURN session::ns(); RETURN session::db(); INFO FOR DB STRUCTURE;")
        .await?;
    let namespace: Option<String> = response.take(0)?;
    let namespace = match namespace {
        Some(namespace) => namespace,
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_CONTEXT_NAMESPACE_MISSING".to_owned(),
            )
            .await;
        }
    };
    let selected_database: Option<String> = response.take(1)?;
    let selected_database = match selected_database {
        Some(selected_database) => selected_database,
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_CONTEXT_DATABASE_MISSING".to_owned(),
            )
            .await;
        }
    };
    if namespace != DEFAULT_NAMESPACE || selected_database != DEFAULT_DATABASE {
        return Err(SurrealStorageError::ContextMismatch {
            expected_namespace: DEFAULT_NAMESPACE.to_owned(),
            expected_database: DEFAULT_DATABASE.to_owned(),
            actual_namespace: namespace,
            actual_database: selected_database,
        });
    }

    let database_info: SurrealValueData = response.take(2)?;
    let mut nonempty_categories = Vec::new();
    for category in DATABASE_STRUCTURE_CATEGORIES {
        let count = match array_len(&database_info, category) {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        if count != 0 {
            nonempty_categories.push(format!("{category}={count}"));
        }
    }
    let table_names = match parse_named_array(&database_info, "tables") {
        Ok(names) => names,
        Err(reason) => return fail_closed(database, reason).await,
    };
    if !table_names.iter().any(|name| name == BOOTSTRAP_STATE_TABLE) {
        if nonempty_categories.is_empty() {
            return Ok(None);
        }
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY: missing_state_table; {}",
                nonempty_categories.join(",")
            ),
        )
        .await;
    }

    let mut state_response = database
        .query(format!("SELECT * FROM ONLY {BOOTSTRAP_STATE_ID};"))
        .await?;
    let state: Option<SchemaState> = state_response.take(0)?;
    match state {
        Some(state) => Ok(Some(state)),
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_STATE_ROW_MISSING".to_owned(),
            )
            .await
        }
    }
}

async fn upgrade_supported_predecessor(
    database: &SurrealAdminContext<'_>,
    predecessor_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !predecessor_state.is_exact_supported_predecessor() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    ensure_supported_predecessor_registry(database).await?;
    let predecessor_observed = read_schema_catalog(database).await?;
    if predecessor_observed.info_fingerprint_sha256 != PREDECESSOR_SCHEMA_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH: expected={PREDECESSOR_SCHEMA_INFO_SHA256}; observed={}",
                predecessor_observed.info_fingerprint_sha256
            ),
        )
        .await;
    }

    database
        .query_bound(
            r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $schema_revision
    OR $current.target_revision != $schema_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256
    OR $current.apply_state != 'complete'
{
    THROW 'HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_CHANGED';
};
DEFINE FIELD OVERWRITE schema_source ON TABLE knowledge_schema_registry TYPE string;
UPDATE knowledge_schema_registry SET schema_source = $schema_source;
REMOVE FIELD migration_file ON TABLE knowledge_schema_registry;
CREATE ONLY knowledge_schema_registry:rich_document_loom_projection_0343_state CONTENT {
    family_key: 'rich_document_loom_projection_0343_state',
    table_name: 'knowledge_rich_document_loom_projection_0343_state',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-032'
};
DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_title_anchors TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE last_rich_document_id ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_title_anchors ON TABLE knowledge_rich_document_title_anchors FIELDS anchor_key UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_rich_document_title_anchors_identity ON TABLE knowledge_rich_document_title_anchors FIELDS workspace_id, title_key UNIQUE;
CREATE ONLY knowledge_schema_registry:rich_document_title_anchors CONTENT {
    family_key: 'rich_document_title_anchors',
    table_name: 'knowledge_rich_document_title_anchors',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-142'
};
UPDATE ONLY handshake_schema_state:primary SET
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,
    apply_state = 'schema_applied',
    updated_at = time::now();
COMMIT TRANSACTION;
"#.replace("UPDATE ONLY handshake_schema_state:primary SET", &format!("{}\nUPDATE ONLY handshake_schema_state:primary SET", resource_authority_upgrade_statements())).as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PREDECESSOR_GENERATED_SURREALQL_SHA256
                    .to_owned(),
                predecessor_info_fingerprint_sha256: PREDECESSOR_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-151 phase one for both allowlisted delta lineages: materialises `journal_key` on
/// existing journal rows in its own transaction, guarded by the exact predecessor state so it
/// never runs against any other lineage.
async fn materialise_mt151_journal_key(
    database: &SurrealAdminContext<'_>,
    predecessor_generated_surql_sha256: &str,
    predecessor_info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    let materialise = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $schema_revision\n\
    OR $current.target_revision != $schema_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_MT151_MATERIALISE_STATE_CHANGED';\n\
}};\n\
{MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS}\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            materialise.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: predecessor_generated_surql_sha256.to_owned(),
                predecessor_info_fingerprint_sha256: predecessor_info_fingerprint_sha256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;
    Ok(())
}

#[derive(SurrealValue)]
struct FolderSiblingKeyRow {
    id: RecordId,
    workspace_id: RecordId,
    parent_folder_id: Option<RecordId>,
    name: String,
    /// Selected only because the engine requires every ORDER BY idiom in the projection.
    #[allow(dead_code)]
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct FolderSiblingKeyWrite {
    record: RecordId,
    sibling_key: String,
}

#[derive(SurrealValue)]
struct FolderSiblingKeyWrites {
    writes: Vec<FolderSiblingKeyWrite>,
}

fn record_id_key(record: &RecordId) -> String {
    match &record.key {
        RecordIdKey::String(value) => value.clone(),
        other => format!("{other:?}"),
    }
}

/// MT-152 phase one for every allowlisted predecessor lineage: defines `loom_folders.sibling_key`
/// in its own state-guarded transaction, then backfills every existing folder row. Rows are
/// visited in `(created_at, id)` order; the first holder of a key keeps it and each later
/// duplicate gets `<key>#dup<n>` (the visible `name` is never touched), with one structured
/// warning per collision naming every folder id, so the UNIQUE index built in phase two always
/// succeeds and the collision is visible rather than fatal. Idempotent: re-running after a crash
/// recomputes the same keys.
async fn materialise_mt152_folder_sibling_key(
    database: &SurrealAdminContext<'_>,
    predecessor_generated_surql_sha256: &str,
    predecessor_info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    let define = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $schema_revision\n\
    OR $current.target_revision != $schema_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_MT152_FOLDER_SIBLING_KEY_STATE_CHANGED';\n\
}};\n\
{MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS}\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            define.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: predecessor_generated_surql_sha256.to_owned(),
                predecessor_info_fingerprint_sha256: predecessor_info_fingerprint_sha256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;
    backfill_mt152_folder_sibling_keys(database).await
}

async fn backfill_mt152_folder_sibling_keys(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT id, workspace_id, parent_folder_id, name, created_at FROM loom_folders \
             ORDER BY created_at ASC, id ASC;",
        )
        .await?;
    let rows: Vec<FolderSiblingKeyRow> = response.take(0)?;
    let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut writes = Vec::with_capacity(rows.len());
    for row in rows {
        let workspace_id = record_id_key(&row.workspace_id);
        let parent_folder_id = row.parent_folder_id.as_ref().map(record_id_key);
        let base = super::loom_store::loom_folder_sibling_key(
            &workspace_id,
            parent_folder_id.as_deref(),
            &row.name,
        );
        let folder_id = record_id_key(&row.id);
        let seen = holders.entry(base.clone()).or_default();
        let sibling_key = if seen.is_empty() {
            base
        } else {
            format!("{base}#dup{}", seen.len())
        };
        seen.push(folder_id);
        writes.push(FolderSiblingKeyWrite {
            record: row.id,
            sibling_key,
        });
    }
    for (sibling_key, folder_ids) in holders.iter().filter(|(_, ids)| ids.len() > 1) {
        tracing::warn!(
            sibling_key = %sibling_key,
            folder_ids = ?folder_ids,
            kept = %folder_ids[0],
            "HANDSHAKE_SURREAL_LOOM_FOLDER_SIBLING_NAME_COLLISION: pre-existing folders share one sibling name; later duplicates keep their visible name and carry a '#dup<n>' sibling_key suffix (MT-152)"
        );
    }
    if writes.is_empty() {
        return Ok(());
    }
    database
        .query_bound(
            "BEGIN TRANSACTION; \
             FOR $write IN $writes { UPDATE $write.record SET sibling_key = $write.sibling_key RETURN NONE; }; \
             COMMIT TRANSACTION;",
            FolderSiblingKeyWrites { writes },
        )
        .await?;
    Ok(())
}

/// MT-142: upgrades an exact pre-MT-142 current store in place by adding only the
/// `knowledge_rich_document_title_anchors` table and its registry row inside one transaction
/// guarded by the exact prior state, then finalizes through the same fingerprint gate as every
/// other lineage. Application records are untouched. MT-151: the same transaction also applies
/// the MT-151 statements, because the finalize gate pins the current fingerprint.
async fn upgrade_pre_mt142_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt142_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt151_journal_key(
        database,
        PRE_MT142_GENERATED_SURREALQL_SHA256,
        PRE_MT142_SCHEMA_INFO_SHA256,
    )
    .await?;
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT142_GENERATED_SURREALQL_SHA256,
        PRE_MT142_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = resource_authority_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $schema_revision\n\
    OR $current.target_revision != $schema_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS}\
{MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT142_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT142_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-151: upgrades an exact MT-142 current store in place by adding `loom_blocks.journal_key`
/// with its UNIQUE index (materialising the key on existing journal rows first) and the
/// `storage_graph_anchors` table inside one transaction guarded by the exact prior state, then
/// finalizes through the same fingerprint gate as every other lineage. Every other application
/// record is untouched.
async fn upgrade_pre_mt151_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt151_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt151_journal_key(
        database,
        PRE_MT151_GENERATED_SURREALQL_SHA256,
        PRE_MT151_SCHEMA_INFO_SHA256,
    )
    .await?;
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT151_GENERATED_SURREALQL_SHA256,
        PRE_MT151_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = resource_authority_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $schema_revision\n\
    OR $current.target_revision != $schema_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT151_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT151_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-152: upgrades an exact MT-151 current store in place by adding the
/// `fems_workspace_write_anchors` table inside one transaction guarded by the exact prior
/// state, then finalizes through the same fingerprint gate as every other lineage. Every
/// application record is untouched.
async fn upgrade_pre_mt152_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt152_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT152_GENERATED_SURREALQL_SHA256,
        PRE_MT152_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = resource_authority_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $schema_revision\n\
    OR $current.target_revision != $schema_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT152_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

fn mt109_authority_upgrade_bindings() -> PredecessorUpgradeBindings {
    PredecessorUpgradeBindings {
        schema_version: SCHEMA_VERSION.to_owned(),
        schema_revision: SCHEMA_REVISION,
        namespace: DEFAULT_NAMESPACE.to_owned(),
        database: DEFAULT_DATABASE.to_owned(),
        source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
        predecessor_generated_surql_sha256: PRE_MT109_GENERATED_SURREALQL_SHA256.to_owned(),
        predecessor_info_fingerprint_sha256: PRE_MT109_SCHEMA_INFO_SHA256.to_owned(),
        generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
        pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
        schema_source: "storage/surreal/schema.surql".to_owned(),
    }
}

fn mt109_authority_upgrade_query() -> String {
    let authority_delta = resource_authority_upgrade_statements();
    format!(
        r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $schema_revision
    OR $current.target_revision != $schema_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256
    OR $current.apply_state != 'complete'
{{
    THROW 'HANDSHAKE_SURREAL_PRE_MT109_UPGRADE_STATE_CHANGED';
}};
{authority_delta}
UPDATE ONLY handshake_schema_state:primary SET
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,
    apply_state = 'schema_applied',
    updated_at = time::now();
COMMIT TRANSACTION;
"#
    )
}

async fn upgrade_pre_mt109_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt109_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT109_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    // The old receipt predates its out-of-band authority overlay. Accept only either complete
    // observed catalog, including every access/function/permission; the receipt alone is insufficient.
    let predecessor = read_schema_catalog(database).await?;
    if ![
        PRE_MT109_SCHEMA_INFO_SHA256,
        PRE_MT109_AUTHORITY_INFO_SHA256,
    ]
    .contains(&predecessor.info_fingerprint_sha256.as_str())
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PRE_MT109_CATALOG_MISMATCH: observed={}",
                predecessor.info_fingerprint_sha256
            ),
        )
        .await;
    }
    database
        .query_bound(
            mt109_authority_upgrade_query().as_str(),
            mt109_authority_upgrade_bindings(),
        )
        .await?;
    let applied = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        other => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT109_APPLY_STATE_MISMATCH: {other:?}"),
            )
            .await
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &applied, &observed.info_fingerprint_sha256).await?;
    Ok(observed)
}

async fn finalize_schema_state(
    database: &SurrealAdminContext<'_>,
    applied_state: &SchemaState,
    info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    if !applied_state.is_schema_applied_current() || info_fingerprint_sha256.len() != 64 {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_SCHEMA_FINALIZE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    database
        .query_bound(
            r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $schema_revision
    OR $current.target_revision != $schema_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $pending_info_fingerprint_sha256
    OR $current.apply_state != 'schema_applied'
{
    THROW 'HANDSHAKE_SURREAL_SCHEMA_FINALIZE_STATE_CHANGED';
};
UPDATE ONLY handshake_schema_state:primary SET
    info_fingerprint_sha256 = $info_fingerprint_sha256,
    apply_state = 'complete',
    updated_at = time::now();
COMMIT TRANSACTION;
"#,
            FinalizeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                info_fingerprint_sha256: info_fingerprint_sha256.to_owned(),
            },
        )
        .await?;
    Ok(())
}

async fn observe_schema(
    database: &SurrealAdminContext<'_>,
    state: SchemaState,
    outcome: SchemaBootstrapOutcome,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let observed = inspect_schema(database).await?;
    report_from_observed(database, state, observed, outcome).await
}

async fn report_from_observed(
    database: &SurrealAdminContext<'_>,
    state: SchemaState,
    observed: ObservedSchema,
    outcome: SchemaBootstrapOutcome,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    verify_expected_info_fingerprint(database, &observed).await?;
    if observed.info_fingerprint_sha256 != state.info_fingerprint_sha256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH: expected={}; observed={}",
                state.info_fingerprint_sha256, observed.info_fingerprint_sha256
            ),
        )
        .await;
    }

    Ok(SchemaBootstrapReport {
        schema_version: state.version,
        namespace: state.namespace,
        database: state.database,
        declarative_schema_files: 1,
        source_manifest_sha256: state.source_manifest_sha256,
        generated_surql_sha256: state.generated_surql_sha256,
        info_fingerprint_sha256: state.info_fingerprint_sha256,
        tables_defined: observed.tables_defined,
        fields_defined: observed.fields_defined,
        indexes_defined: observed.indexes_defined,
        table_names: observed.table_names,
        outcome,
        reused_existing_schema: outcome.reused_existing_schema(),
    })
}

async fn verify_expected_info_fingerprint(
    database: &SurrealAdminContext<'_>,
    observed: &ObservedSchema,
) -> Result<(), SurrealStorageError> {
    if EXPECTED_SCHEMA_INFO_SHA256.bytes().all(|byte| byte == b'0') {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_UNPINNED: observed={}",
                observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    if observed.info_fingerprint_sha256 != EXPECTED_SCHEMA_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH: expected={EXPECTED_SCHEMA_INFO_SHA256}; observed={}",
                observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    Ok(())
}

async fn inspect_schema(
    database: &SurrealAdminContext<'_>,
) -> Result<ObservedSchema, SurrealStorageError> {
    let observed = read_schema_catalog(database).await?;
    let mut expected_names = TABLE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected_names.sort();
    if observed.table_names != expected_names {
        return fail_closed(database, format!("HANDSHAKE_SURREAL_SCHEMA_TABLE_SET_MISMATCH: expected={expected_names:?}; observed={:?}", observed.table_names)).await;
    }
    if observed.fields_defined != FIELD_DEFINITION_COUNT
        || observed.indexes_defined != INDEX_DEFINITION_COUNT
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_MISMATCH: tables={}; fields={}; indexes={}",
                observed.tables_defined, observed.fields_defined, observed.indexes_defined
            ),
        )
        .await;
    }
    Ok(observed)
}

async fn read_schema_catalog(
    database: &SurrealAdminContext<'_>,
) -> Result<ObservedSchema, SurrealStorageError> {
    let mut db_info_response = database.query("INFO FOR DB STRUCTURE;").await?;
    let db_info: SurrealValueData = db_info_response.take(0)?;
    for category in DATABASE_STRUCTURE_CATEGORIES {
        if let Err(reason) = array_len(&db_info, category) {
            return fail_closed(database, reason).await;
        }
    }
    let mut table_names = match parse_named_array(&db_info, "tables") {
        Ok(names) => names,
        Err(reason) => return fail_closed(database, reason).await,
    };
    table_names.sort();

    let mut fields_defined = 0usize;
    let mut indexes_defined = 0usize;
    let mut table_info_by_name = BTreeMap::new();
    let table_info_query = table_names
        .iter()
        .map(|table| format!("INFO FOR TABLE `{table}` STRUCTURE;"))
        .collect::<String>();
    let mut table_responses = database.query(table_info_query).await?;
    for (statement_index, table) in table_names.iter().enumerate() {
        let table_info: SurrealValueData = table_responses.take(statement_index)?;
        for category in ["events", "fields", "indexes", "lives", "tables"] {
            if let Err(reason) = array_len(&table_info, category) {
                return fail_closed(database, reason).await;
            }
        }
        fields_defined += match array_len(&table_info, "fields") {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        indexes_defined += match array_len(&table_info, "indexes") {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        table_info_by_name.insert(table.clone(), table_info);
    }

    Ok(ObservedSchema {
        info_fingerprint_sha256: canonical_catalog_fingerprint(db_info, table_info_by_name),
        tables_defined: table_names.len(),
        fields_defined,
        indexes_defined,
        table_names,
    })
}

pub(super) fn info_entry_name(value: &SurrealValueData) -> Option<&str> {
    let SurrealValueData::Object(object) = value else {
        return None;
    };
    let Some(SurrealValueData::String(name)) = object.get("name") else {
        return None;
    };
    Some(name)
}

/// The ONE definition of the live schema fingerprint: `INFO FOR DB STRUCTURE` plus every
/// table's `INFO FOR TABLE ... STRUCTURE`, canonicalised (named catalog entries sorted, index
/// column order kept) with the engine's table catalog ids stripped, serialised and hashed.
/// Bootstrap (`inspect_schema`) and the test inspector both pin
/// [`EXPECTED_SCHEMA_INFO_SHA256`] through this function, so they cannot disagree.
///
/// MT-151: table catalog ids (`tables[].id` in STRUCTURE output) are the engine's allocation
/// counter, not schema. A fresh apply numbers each table by its script position; a store
/// upgraded in place by a delta `DEFINE TABLE` allocates the next free id and every later
/// table keeps its old number, so a fingerprint that kept ids could never be reached by any
/// table-adding lineage (run mt142-LIB-20260911T124424Z: every drifting entry was
/// `tables[].id`, 233 fresh vs 300 upgraded for the added table). The MT-138 Atelier catalog
/// fingerprint already strips them for the same reason (`strip_table_catalog_id`).
pub(super) fn canonical_catalog_fingerprint(
    db_info: SurrealValueData,
    tables: BTreeMap<String, SurrealValueData>,
) -> String {
    let canonical = CanonicalInfoEnvelope {
        database: strip_nested_table_catalog_ids(db_info),
        tables: tables
            .into_iter()
            .map(|(name, info)| (name, strip_nested_table_catalog_ids(info)))
            .collect(),
    };
    let canonical_json =
        serde_json::to_string(&canonical).expect("canonical structured INFO serializes losslessly");
    sha256_hex(canonical_json.as_bytes())
}

pub(super) fn canonicalize_info(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut canonical = SurrealObject::new();
            for (key, value) in object.into_inner() {
                canonical.insert(key, canonicalize_info(value));
            }
            SurrealValueData::Object(canonical)
        }
        SurrealValueData::Array(array) => {
            let mut canonical = array
                .into_vec()
                .into_iter()
                .map(canonicalize_info)
                .collect::<Vec<_>>();
            if canonical
                .iter()
                .all(|entry| info_entry_name(entry).is_some())
            {
                canonical.sort_by(|left, right| info_entry_name(left).cmp(&info_entry_name(right)));
            }
            SurrealValueData::Array(SurrealArray::from(canonical))
        }
        scalar => scalar,
    }
}

fn parse_table_definitions(
    value: &SurrealValueData,
) -> Result<BTreeMap<String, AtelierTableDefinition>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(tables)) = object.get("tables") else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `tables` array".to_owned());
    };

    let mut definitions = BTreeMap::new();
    for entry in tables.iter() {
        let SurrealValueData::Object(table) = entry else {
            return Err(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `tables` entry is not an object".to_owned(),
            );
        };
        let name = info_entry_name(entry)
            .map(|name| name.trim_matches('`').to_owned())
            .ok_or_else(|| {
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `tables` entry missing name".to_owned()
            })?;
        let Some(SurrealValueData::Bool(schemafull)) = table.get("schemafull") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing schemafull"
            ));
        };
        let Some(SurrealValueData::Object(kind)) = table.get("kind") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing kind object"
            ));
        };
        let Some(SurrealValueData::String(kind)) = kind.get("kind") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing kind token"
            ));
        };
        let is_view = table
            .get("view")
            .is_some_and(|view| !matches!(view, SurrealValueData::None | SurrealValueData::Null));
        let definition = AtelierTableDefinition {
            schemafull: *schemafull,
            kind: kind.to_owned(),
            is_view,
        };
        if definitions.insert(name.clone(), definition).is_some() {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: duplicate table `{name}`"
            ));
        }
    }
    Ok(definitions)
}

fn parse_named_structures(
    value: &SurrealValueData,
    key: &str,
) -> Result<BTreeMap<String, SurrealValueData>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    let mut definitions = BTreeMap::new();
    for entry in array.iter() {
        let name = info_entry_name(entry)
            .map(|name| name.trim_matches('`').to_owned())
            .ok_or_else(|| {
                format!("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `{key}` entry missing name")
            })?;
        if definitions.insert(name.clone(), entry.clone()).is_some() {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: duplicate `{key}` entry `{name}`"
            ));
        }
    }
    Ok(definitions)
}

fn strip_table_catalog_id(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut object = object.into_inner();
            object.remove("id");
            SurrealValueData::Object(SurrealObject::from(object))
        }
        value => value,
    }
}

fn strip_nested_table_catalog_ids(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut normalized = SurrealObject::new();
            for (key, value) in object.into_inner() {
                let value = if key == "tables" {
                    match value {
                        SurrealValueData::Array(array) => {
                            SurrealValueData::Array(SurrealArray::from(
                                array
                                    .into_vec()
                                    .into_iter()
                                    .map(strip_table_catalog_id)
                                    .collect::<Vec<_>>(),
                            ))
                        }
                        value => value,
                    }
                } else {
                    value
                };
                normalized.insert(key, canonicalize_info(value));
            }
            SurrealValueData::Object(normalized)
        }
        value => canonicalize_info(value),
    }
}

pub(super) fn parse_named_array(
    value: &SurrealValueData,
    key: &str,
) -> Result<Vec<String>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    array
        .iter()
        .map(|entry| {
            info_entry_name(entry)
                .map(|name| name.trim_matches('`').to_owned())
                .ok_or_else(|| {
                    format!("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `{key}` entry missing name")
                })
        })
        .collect()
}

fn array_len(value: &SurrealValueData, key: &str) -> Result<usize, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    Ok(array.len())
}

async fn fail_closed<T>(
    database: &SurrealAdminContext<'_>,
    reason: String,
) -> Result<T, SurrealStorageError> {
    database
        .query_bound("THROW $reason;", ("reason", reason))
        .await?;
    unreachable!("THROW must fail closed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        surreal::{SurrealDatabase, SurrealStorage, SurrealStorageConfig},
        Database, EntityRef, JobMetrics, LoomFolderSortMode, LoomFolderUpdate, NewLoomFolder,
        OperationType, PlannedOperation, StorageError,
    };
    use surrealdb::{engine::local::Mem, Surreal};

    const MT138_MINIMAL_CATALOG_DDL: &str =
        "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1; \
         DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS NONE; \
         DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE string; \
         DEFINE FIELD OVERWRITE marker ON TABLE atelier_mt138_catalog_probe TYPE string; \
         DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS value UNIQUE; \
         DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
             WHEN $event = 'DELETE' \
             THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $before.marker; }; \
         DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
             SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;";

    #[derive(SurrealValue)]
    struct NativeJsonBindings {
        entity_refs: JsonValue,
        planned_operations: JsonValue,
        metrics: JsonValue,
        job_inputs: JsonValue,
    }

    async fn open_test_storage(
        directory: &tempfile::TempDir,
    ) -> Result<SurrealStorage, SurrealStorageError> {
        SurrealStorage::open(SurrealStorageConfig::with_path(
            directory.path().join("store"),
        )?)
        .await
    }

    fn mt138_minimal_catalog_tables() -> BTreeSet<String> {
        BTreeSet::from([
            "atelier_mt138_catalog_probe".to_owned(),
            "atelier_mt138_catalog_view".to_owned(),
        ])
    }

    fn mt138_minimal_catalog_sequences() -> BTreeSet<String> {
        BTreeSet::from(["atelier_mt138_catalog_seq".to_owned()])
    }

    async fn mt138_minimal_catalog_query(
        storage: &SurrealStorage,
        statement: &'static str,
    ) -> Result<(), SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database.query(statement).await?;
                    Ok(())
                })
            })
            .await
    }

    async fn mt138_minimal_catalog_fingerprint(
        storage: &SurrealStorage,
    ) -> Result<String, SurrealStorageError> {
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    inspect_atelier_catalog_fingerprint(
                        &database,
                        &mt138_minimal_catalog_tables(),
                        &mt138_minimal_catalog_sequences(),
                    )
                    .await
                })
            })
            .await
    }

    async fn mt138_verify_minimal_catalog(
        storage: &SurrealStorage,
        expected_fingerprint: String,
    ) -> Result<(), SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    verify_atelier_catalog_fingerprint(
                        &database,
                        &mt138_minimal_catalog_tables(),
                        &mt138_minimal_catalog_sequences(),
                        &expected_fingerprint,
                    )
                    .await
                })
            })
            .await
    }

    async fn mt138_mem_catalog_fingerprint(
        statement: String,
        expected_tables: &BTreeSet<String>,
        expected_sequences: &BTreeSet<String>,
    ) -> Result<String, SurrealStorageError> {
        let client = Surreal::new::<Mem>(()).await?;
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await?;
        let database = SurrealAdminContext { client: &client };
        database.query(statement).await?;
        inspect_atelier_catalog_fingerprint(&database, expected_tables, expected_sequences).await
    }

    async fn mt138_canonical_mem_fingerprint(
        probe_dependencies: bool,
    ) -> Result<String, SurrealStorageError> {
        let expected_tables = atelier_expected_catalog()
            .into_keys()
            .collect::<BTreeSet<_>>();
        assert_eq!(expected_tables.len(), 134);
        let expected_sequences = ATELIER_REQUIRED_SEQUENCES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        let client = Surreal::new::<Mem>(()).await?;
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await?;
        let database = SurrealAdminContext { client: &client };
        database
            .query(format!(
                "BEGIN TRANSACTION;\n{}\nCOMMIT TRANSACTION;",
                atelier_schema_ddl()
            ))
            .await?;
        let fingerprint =
            inspect_atelier_catalog_fingerprint(&database, &expected_tables, &expected_sequences)
                .await?;
        if probe_dependencies {
            assert_eq!(authority_catalog_names("access").len(), 1);
            assert_eq!(authority_catalog_names("function").len(), 9);
            let mut saved = database
                .query("RETURN (INFO FOR DB).functions.mt109_live_session;")
                .await?;
            let original = saved
                .take::<Option<String>>(0)?
                .expect("canonical authority function must exist");
            assert!(original.starts_with("DEFINE FUNCTION fn::mt109_live_session("));
            let restore = format!(
                "{};",
                original
                    .replacen("DEFINE FUNCTION ", "DEFINE FUNCTION OVERWRITE ", 1)
                    .trim_end_matches(';')
            );
            for (mutation, marker) in [
                (
                    "REMOVE FUNCTION fn::mt109_live_session;",
                    "AUTHORITY_DEPENDENCY_MISSING",
                ),
                (
                    "DEFINE FUNCTION OVERWRITE fn::mt109_live_session() { RETURN true; };",
                    "CATALOG_FINGERPRINT_MISMATCH",
                ),
            ] {
                database.query(mutation).await?;
                let error = verify_atelier_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    &fingerprint,
                )
                .await
                .expect_err("altered authority function must reject");
                assert!(
                    error.to_string().contains(marker),
                    "unexpected rejection: {error}"
                );
                database.query(restore.clone()).await?;
                verify_atelier_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    &fingerprint,
                )
                .await?;
            }
            database
                .query("ALTER ACCESS authenticated_session ON DATABASE DURATION FOR TOKEN 6m;")
                .await?;
            let error = verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await
            .expect_err("altered authority access must reject");
            assert!(error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            database
                .query("ALTER ACCESS authenticated_session ON DATABASE DURATION FOR TOKEN 5m;")
                .await?;
            verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await?;
            database.query("DEFINE TABLE mt138_unrelated SCHEMAFULL; DEFINE FUNCTION fn::mt138_unrelated() { RETURN true; };").await?;
            verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await?;
            database
                .query("REMOVE ACCESS authenticated_session ON DATABASE;")
                .await?;
            let error = verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await
            .expect_err("missing authority access must reject");
            assert!(error.to_string().contains("AUTHORITY_DEPENDENCY_MISSING"));
        }
        Ok(fingerprint)
    }

    #[tokio::test]
    async fn mt138_catalog_fingerprint_is_backend_stable_between_mem_and_rocks() {
        tokio::time::timeout(std::time::Duration::from_secs(300), async {
            let directory = tempfile::tempdir().expect("create MT-138 backend parity directory");
            let rocks = open_test_storage(&directory)
                .await
                .expect("open MT-138 RocksDB parity store");
            mt138_minimal_catalog_query(&rocks, MT138_MINIMAL_CATALOG_DDL)
                .await
                .expect("create minimal RocksDB parity catalog");
            let rocks_fingerprint = mt138_minimal_catalog_fingerprint(&rocks)
                .await
                .expect("fingerprint minimal RocksDB catalog");
            let mem_fingerprint = mt138_mem_catalog_fingerprint(
                MT138_MINIMAL_CATALOG_DDL.to_owned(),
                &mt138_minimal_catalog_tables(),
                &mt138_minimal_catalog_sequences(),
            )
            .await
            .expect("fingerprint identical in-memory catalog");
            assert_eq!(
                rocks_fingerprint, mem_fingerprint,
                "normalized structured INFO must be storage-backend invariant"
            );
            rocks
                .shutdown()
                .await
                .expect("close MT-138 RocksDB parity store");
        })
        .await
        .expect("MT-138 Mem/Rocks fingerprint parity exceeded five minutes");
    }

    #[tokio::test]
    async fn mt138_minimal_real_rocks_catalog_rejects_adversarial_mutations() {
        tokio::time::timeout(std::time::Duration::from_secs(300), async {
            let directory = tempfile::tempdir().expect("create MT-138 minimal catalog directory");
            let storage = open_test_storage(&directory)
                .await
                .expect("open MT-138 minimal catalog store");
            mt138_minimal_catalog_query(&storage, MT138_MINIMAL_CATALOG_DDL)
            .await
            .expect("create exact minimal catalog");
            let expected_fingerprint = mt138_minimal_catalog_fingerprint(&storage)
                .await
                .expect("fingerprint exact minimal catalog");
            mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect("accept exact minimal catalog");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_rogue SCHEMAFULL PERMISSIONS NONE;",
            )
            .await
            .expect("create rogue table");
            let rogue_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject rogue Atelier table");
            assert!(rogue_error.to_string().contains("TABLE_SET_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "REMOVE TABLE atelier_mt138_catalog_rogue;",
            )
            .await
            .expect("remove rogue table");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE attacker_extra ON TABLE atelier_mt138_catalog_probe TYPE string;",
            )
            .await
            .expect("create unexpected field");
            let field_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject unexpected field");
            assert!(field_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "REMOVE FIELD attacker_extra ON TABLE atelier_mt138_catalog_probe;",
            )
            .await
            .expect("remove unexpected field");

            mt138_minimal_catalog_query(
                &storage,
                "ALTER TABLE atelier_mt138_catalog_probe SCHEMALESS;",
            )
            .await
            .expect("make probe schemaless");
            let mode_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject schemaless replacement");
            assert!(mode_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "ALTER TABLE atelier_mt138_catalog_probe SCHEMAFULL;",
            )
            .await
            .expect("restore schemafull mode");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL PERMISSIONS NONE;",
            )
            .await
            .expect("replace view with normal table");
            let view_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject non-view replacement");
            assert!(view_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("restore exact view");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE int;",
            )
            .await
            .expect("change existing field type");
            let field_type_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed field type");
            assert!(field_type_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE string;",
            )
            .await
            .expect("restore field type");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS marker;",
            )
            .await
            .expect("change existing index columns and uniqueness");
            let index_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject changed index definition");
            assert!(index_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS value UNIQUE;",
            )
            .await
            .expect("restore index definition");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS FULL;",
            )
            .await
            .expect("broaden table permissions");
            let permission_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed table permissions");
            assert!(permission_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS NONE;",
            )
            .await
            .expect("restore table permissions");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT marker AS value FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("change view query while retaining view type");
            let view_query_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed view query");
            assert!(view_query_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("restore view query");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
                     WHEN $event = 'CREATE' \
                     THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $after.marker; };",
            )
            .await
            .expect("change event condition and action");
            let event_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject changed event definition");
            assert!(event_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
                     WHEN $event = 'DELETE' \
                     THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $before.marker; };",
            )
            .await
            .expect("restore event definition");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 2 START 1;",
            )
            .await
            .expect("change sequence definition");
            let sequence_definition_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed sequence definition");
            assert!(sequence_definition_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1;",
            )
            .await
            .expect("restore sequence definition");

            mt138_minimal_catalog_query(&storage, "REMOVE SEQUENCE atelier_mt138_catalog_seq;")
                .await
                .expect("remove required sequence");
            let missing_sequence_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject missing required sequence");
            assert!(missing_sequence_error
                .to_string()
                .contains("SEQUENCE_SET_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1;",
            )
            .await
            .expect("restore required sequence");

            mt138_verify_minimal_catalog(&storage, expected_fingerprint)
                .await
                .expect("accept fully restored catalog");
            storage
                .shutdown()
                .await
                .expect("close MT-138 minimal catalog store");
        })
        .await
        .expect("MT-138 minimal real-Rocks catalog proof exceeded five minutes");
    }

    #[tokio::test]
    async fn mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin() {
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            let first = mt138_canonical_mem_fingerprint(true)
                .await
                .expect("generate first fresh canonical Atelier fingerprint");
            let second = mt138_canonical_mem_fingerprint(false)
                .await
                .expect("generate second fresh canonical Atelier fingerprint");
            assert_eq!(
                first, second,
                "fresh canonical stores must normalize identically"
            );
            assert_eq!(
                first, EXPECTED_ATELIER_CATALOG_SHA256,
                "compiled Atelier fingerprint pin must match a fresh canonical catalog"
            );
            eprintln!("EXPECTED_ATELIER_CATALOG_SHA256={first}");
        })
        .await
        .expect("MT-138 canonical fingerprint generation exceeded two minutes");
    }

    /// Canonical STRUCTURE catalog of one live store, keyed like `inspect_schema` hashes it,
    /// so a fingerprint mismatch can be explained entry by entry instead of hash by hash.
    async fn canonical_catalog(
        database: &SurrealAdminContext<'_>,
    ) -> Result<BTreeMap<String, String>, SurrealStorageError> {
        let mut entries = BTreeMap::new();
        let mut db_info_response = database.query("INFO FOR DB STRUCTURE;").await?;
        let db_info: SurrealValueData = db_info_response.take(0)?;
        let mut table_names = parse_named_array(&db_info, "tables")
            .unwrap_or_else(|reason| panic!("invalid DB INFO: {reason}"));
        table_names.sort();
        entries.insert(
            "database".to_owned(),
            serde_json::to_string(&strip_nested_table_catalog_ids(db_info))
                .expect("db info serializes (same stripping as canonical_catalog_fingerprint)"),
        );
        for table in table_names {
            let mut response = database
                .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
                .await?;
            let info: SurrealValueData = response.take(0)?;
            entries.insert(
                format!("table:{table}"),
                serde_json::to_string(&strip_nested_table_catalog_ids(info))
                    .expect("table info serializes"),
            );
        }
        Ok(entries)
    }

    /// Fresh in-memory apply of the current script: the reference every lineage must reach.
    async fn fresh_mem_catalog() -> BTreeMap<String, String> {
        let client = Surreal::new::<Mem>(()).await.expect("open memory store");
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await
            .expect("select memory context");
        let database = SurrealAdminContext { client: &client };
        database
            .query_bound(
                SCHEMA,
                BootstrapBindings {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    schema_revision: SCHEMA_REVISION,
                    namespace: DEFAULT_NAMESPACE.to_owned(),
                    database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                    generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                },
            )
            .await
            .expect("apply current schema in memory");
        canonical_catalog(&database)
            .await
            .expect("inspect fresh memory catalog")
    }

    /// Prints every catalog entry that differs from the fresh reference.
    fn report_catalog_drift(
        label: &str,
        reference: &BTreeMap<String, String>,
        observed: &BTreeMap<String, String>,
    ) {
        for (key, expected) in reference {
            match observed.get(key) {
                Some(actual) if actual == expected => {}
                Some(actual) => {
                    eprintln!("{label} DRIFT {key}\n  fresh:    {expected}\n  observed: {actual}")
                }
                None => eprintln!("{label} MISSING {key}"),
            }
        }
        for key in observed.keys() {
            if !reference.contains_key(key) {
                eprintln!("{label} EXTRA {key}");
            }
        }
    }

    async fn mt109_schema_rows(
        storage: &SurrealStorage,
        with_authority: bool,
    ) -> Result<Vec<SurrealValueData>, SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    let tables = if with_authority {
                        vec![
                            "local_accounts",
                            "principals",
                            "access_spaces",
                            "authenticated_sessions",
                            "session_exchange_credentials",
                            "protected_resources",
                            "resource_grants",
                            "authorization_audit_events",
                            "kernel_event_ledger",
                        ]
                    } else {
                        vec!["kernel_event_ledger"]
                    };
                    let mut rows = Vec::new();
                    for table in tables {
                        let mut result = database
                            .query(format!("SELECT * FROM {table} ORDER BY id;"))
                            .await?;
                        rows.push(result.take::<SurrealValueData>(0)?);
                    }
                    Ok(rows)
                })
            })
            .await
    }

    fn copy_mt109_migration_store(
        source: &std::path::Path,
        destination: &std::path::Path,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                std::fs::create_dir_all(&target)?;
                copy_mt109_migration_store(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn mt109_exact_predecessor_migration_preserves_rows_keys_and_rolls_back() {
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, ResourceAction, ResourceGrantSpec, ResourceKind, SigninParams,
        };
        use surrealdb::opt::auth::Record;
        for with_authority in [false, true] {
            let directory = tempfile::Builder::new()
                .prefix("mt109-migration-")
                .tempdir()
                .expect("migration proof directory");
            let rollback_directory = tempfile::Builder::new()
                .prefix("mt109-migration-rollback-")
                .tempdir()
                .expect("rollback migration proof directory");
            let directory_ref = &directory;
            let rollback_directory_ref = &rollback_directory;
            let storage = open_test_storage(directory_ref)
                .await
                .expect("open migration predecessor");
            let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async move {
                storage.with_admin_operation(|database| Box::pin(async move {
                    database.query_bound(PRE_MT109_SCHEMA, BootstrapBindings {
                        schema_version: SCHEMA_VERSION.to_owned(), schema_revision: SCHEMA_REVISION,
                        namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
                        source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                        generated_surql_sha256: PRE_MT109_GENERATED_SURREALQL_SHA256.to_owned(),
                    }).await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    let observed = read_schema_catalog(&database).await?;
                    assert_eq!(observed.info_fingerprint_sha256, PRE_MT109_SCHEMA_INFO_SHA256);
                    database.query(format!("UPDATE handshake_schema_state:primary SET apply_state = 'complete', info_fingerprint_sha256 = '{PRE_MT109_SCHEMA_INFO_SHA256}';")).await?;
                    if with_authority { database.query(PRE_MT109_AUTHORITY_SCHEMA).await?; }
                    Ok(())
                })).await?;
                let mut old_jwt = None;
                let mut session_id = None;
                if with_authority {
                    let capabilities = vec!["fr.read".to_owned(), "fr.ingest.native_editor".to_owned()];
                    let principal = storage.provision_principal("migration-account", "migration-principal", "human_account", "migration-actor", "Operator", &capabilities, "migration-space", None, std::time::Duration::from_secs(3600)).await?;
                    let resource = storage.register_protected_resource(&principal.identity, ResourceKind::FlightRecorder, "migration-workspace", None, "account_private").await?;
                    storage.grant_resource(&principal.identity.account_id, &principal.identity.access_space_id, ResourceGrantSpec {
                        principal_id: principal.identity.principal_id.clone(), resource_id: resource.resource_id,
                        actions: vec![ResourceAction::Read, ResourceAction::Create], capability_ids: capabilities,
                        expires_at: None, delegation_chain: vec![],
                    }).await?;
                    storage.provision_session_credential(&principal.identity, std::time::Duration::from_secs(3600)).await?;
                    storage.authorize_protected_resource(AuthorizationRequest {
                        session_token: principal.session.token.clone(), channel_binding_hash: None, capability_id: "fr.read".to_owned(),
                        resource_kind: ResourceKind::FlightRecorder, external_resource_id: "migration-workspace".to_owned(), action: ResourceAction::Read,
                    }).await?;
                    session_id = Some(principal.session.session_id.clone());
                    let token_hash = sha256_hex(principal.session.token.as_bytes());
                    old_jwt = Some(storage.with_lease(move |client| Box::pin(async move {
                        let ordinary = client.clone();
                        ordinary.use_ns(DEFAULT_NAMESPACE).use_db(DEFAULT_DATABASE).await?;
                        Ok(ordinary.signin(Record { namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(), access: "authenticated_session".to_owned(), params: SigninParams { token_hash, channel_binding_hash: None } }).await?)
                    })).await?);
                }
                for ordinal in 0..2 {
                    let event = crate::kernel::NewKernelEvent::builder("migration-task", "migration-session", crate::kernel::KernelEventType::ArtifactStored, crate::kernel::KernelActor::System("migration-proof".to_owned()))
                        .aggregate("migration", "migration-aggregate").idempotency_key(format!("migration-{ordinal}"))
                        .source_component("mt109-migration-proof").payload(serde_json::json!({"ordinal": ordinal})).build()?;
                    let (_, write) = super::super::event_ledger::prepare_event(event)?;
                    storage.with_admin_operation(move |database| Box::pin(async move {
                        let authority_fields = if with_authority { ", wsids: $wsids, authority_resource_id: $authority_resource_id, authority_session_id: $authority_session_id, authority_capability_id: $authority_capability_id, authority_action: $authority_action" } else { "" };
                        database.query_bound(format!("CREATE $record CONTENT {{ event_id: $event_id, event_version: $event_version, kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, aggregate_type: $aggregate_type, aggregate_id: $aggregate_id, idempotency_key: $idempotency_key, event_type: $event_type, actor_kind: $actor_kind, actor_id: $actor_id, causation_id: $causation_id, correlation_id: $correlation_id, payload_hash: $payload_hash, source_component: $source_component, payload: $payload, created_at: $created_at{authority_fields} }} RETURN AFTER;"), write).await?;
                        Ok(())
                    })).await?;
                }
                let before = mt109_schema_rows(&storage, with_authority).await?;
                let (before_state, before_catalog) = storage.with_admin_operation(|database| Box::pin(async move {
                    Ok((read_context_and_state(&database).await?.expect("old receipt"), read_schema_catalog(&database).await?.info_fingerprint_sha256))
                })).await?;
                assert_eq!(before_catalog, if with_authority { PRE_MT109_AUTHORITY_INFO_SHA256 } else { PRE_MT109_SCHEMA_INFO_SHA256 });
                let mut malformed = before_state.clone();
                malformed.target_revision -= 1;
                assert!(!malformed.is_exact_pre_mt109_current());
                malformed = before_state.clone();
                malformed.apply_state = "unknown".to_owned();
                assert!(!malformed.is_exact_pre_mt109_current());
                for (field, assignments) in [("forward-revision", "revision = 158, target_revision = 158"), ("lower-revision", "revision = 156, target_revision = 156"), ("old-incomplete", "apply_state = 'schema_applied'")] {
                    storage.with_admin_operation(move |database| Box::pin(async move {
                        database.query(format!("UPDATE handshake_schema_state:primary SET {assignments};")).await?;
                        Ok(())
                    })).await?;
                    let rejected_state = storage.with_admin_operation(|database| Box::pin(async move {
                        Ok(read_context_and_state(&database).await?.expect("changed predecessor receipt"))
                    })).await?;
                    let rejection = bootstrap_schema(&storage).await.expect_err("changed predecessor state must fail closed");
                    assert!(rejection.to_string().contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"), "changed {field} returned the wrong rejection: {rejection}");
                    assert_eq!(mt109_schema_rows(&storage, with_authority).await?, before);
                    storage.with_admin_operation(|database| Box::pin(async move {
                        assert_eq!(read_context_and_state(&database).await?.expect("rejected receipt"), rejected_state);
                        database.query("UPDATE handshake_schema_state:primary SET revision = 157, target_revision = 157, apply_state = 'complete';").await?;
                        Ok(())
                    })).await?;
                }
                storage.with_admin_operation(|database| Box::pin(async move {
                    database.query("DEFINE TABLE mt109_unknown_overlay SCHEMAFULL PERMISSIONS NONE;").await?;
                    Ok(())
                })).await?;
                let rejected_catalog = storage.with_admin_operation(|database| Box::pin(async move {
                    Ok(read_schema_catalog(&database).await?.info_fingerprint_sha256)
                })).await?;
                let rejection = bootstrap_schema(&storage).await.expect_err("unknown overlay must fail before DDL");
                assert!(rejection.to_string().contains("HANDSHAKE_SURREAL_PRE_MT109_CATALOG_MISMATCH"));
                assert_eq!(mt109_schema_rows(&storage, with_authority).await?, before);
                storage.with_admin_operation(|database| Box::pin(async move {
                    assert_eq!(read_schema_catalog(&database).await?.info_fingerprint_sha256, rejected_catalog);
                    database.query("REMOVE TABLE mt109_unknown_overlay;").await?;
                    Ok(())
                })).await?;
                let before_marker = storage.with_admin_operation(|database| Box::pin(async move {
                    let mut rows = database.query("SELECT * FROM ONLY handshake_schema_state:primary;").await?;
                    Ok(rows.take::<SurrealValueData>(0)?)
                })).await?;
                storage.shutdown().await?;
                drop(storage);
                copy_mt109_migration_store(
                    directory_ref.path(),
                    rollback_directory_ref.path(),
                )?;

                let upgraded = open_test_storage(directory_ref).await?;
                let upgrade = bootstrap_schema(&upgraded).await?;
                assert_eq!(upgrade.outcome, SchemaBootstrapOutcome::UpgradedSupportedPredecessor);
                assert_eq!(upgrade.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
                assert_eq!(mt109_schema_rows(&upgraded, with_authority).await?, before);
                upgraded.shutdown().await?;
                drop(upgraded);

                let current = open_test_storage(directory_ref).await?;
                let replay = bootstrap_schema(&current).await?;
                assert_eq!(replay.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
                assert_eq!(mt109_schema_rows(&current, with_authority).await?, before);
                if let Some(jwt) = old_jwt {
                    let expected_session = session_id.expect("old session identity");
                    current.with_lease(move |client| Box::pin(async move {
                        let ordinary = client.clone();
                        ordinary.use_ns(DEFAULT_NAMESPACE).use_db(DEFAULT_DATABASE).await?;
                        ordinary.authenticate(jwt).await?;
                        let mut response = ordinary.query("RETURN record::id($auth.id);").await?.check()?;
                        let observed: Option<String> = response.take(0)?;
                        assert_eq!(observed, Some(expected_session), "old JWT must retain its exact session after migration and reopen");
                        Ok(())
                    })).await?;
                }

                let rollback = open_test_storage(rollback_directory_ref).await?;
                let rollback_errors = rollback.with_admin_operation(|database| Box::pin(async move {
                    let injected = mt109_authority_upgrade_query().replace("UPDATE ONLY handshake_schema_state:primary SET", "THROW 'MT109_INJECTED_AUTHORITY_ROLLBACK'; UPDATE ONLY handshake_schema_state:primary SET");
                    let mut result = database.client.query(injected)
                        .bind(SurrealValue::into_value(mt109_authority_upgrade_bindings())).await?;
                    let mut errors = result.take_errors().into_iter().map(|(index, error)| (index, error.to_string())).collect::<Vec<_>>();
                    errors.sort_by_key(|(index, _)| *index);
                    Ok(errors)
                })).await?;
                let mut rollback_counts = std::collections::BTreeMap::new();
                for (_, error) in &rollback_errors {
                    *rollback_counts.entry(error.as_str()).or_insert(0usize) += 1;
                }
                eprintln!("MT109_ROLLBACK_ERROR_COUNTS {rollback_counts:?}");
                eprintln!(
                    "MT109_ROLLBACK_PRIMARY_ERRORS {:?}",
                    rollback_errors
                        .iter()
                        .filter(|(_, error)| {
                            error != "The query was not executed due to a failed transaction"
                        })
                        .collect::<Vec<_>>()
                );
                // Response index 149 is the injected THROW immediately before the schema-state
                // UPDATE: three transaction/precondition statements, 144 authority statements,
                // and two Loom backfill statements precede it. The remaining tail statements
                // must be cancelled.
                let primary_errors = rollback_errors
                    .iter()
                    .filter(|(_, error)| {
                        error != "The query was not executed due to a failed transaction"
                    })
                    .map(|(index, error)| (*index, error.as_str()))
                    .collect::<Vec<_>>();
                assert_eq!(
                    primary_errors,
                    vec![
                        (149, "An error occurred: MT109_INJECTED_AUTHORITY_ROLLBACK"),
                        (150, "The query was not executed due to a cancelled transaction"),
                        (151, "Cannot COMMIT: the transaction was aborted due to a prior error"),
                    ],
                    "injected rollback did not fail at the exact pre-marker-update statement"
                );
                assert!(rollback_errors.iter().all(|(_, error)| matches!(error.as_str(),
                    "An error occurred: MT109_INJECTED_AUTHORITY_ROLLBACK"
                    | "The query was not executed due to a failed transaction"
                    | "The query was not executed due to a cancelled transaction"
                    | "Cannot COMMIT: the transaction was aborted due to a prior error"
                )), "unrelated transaction failure: {rollback_counts:?}");
                rollback.shutdown().await?;
                drop(rollback);

                let rollback_reopened = open_test_storage(rollback_directory_ref).await?;
                assert_eq!(mt109_schema_rows(&rollback_reopened, with_authority).await?, before);
                rollback_reopened.with_admin_operation(|database| Box::pin(async move {
                    let mut marker = database.query("SELECT * FROM ONLY handshake_schema_state:primary;").await?;
                    assert_eq!(marker.take::<SurrealValueData>(0)?, before_marker, "rollback changed complete marker including timestamps");
                    assert_eq!(read_schema_catalog(&database).await?.info_fingerprint_sha256, before_catalog);
                    assert_eq!(read_context_and_state(&database).await?.expect("rollback receipt"), before_state);
                    Ok(())
                })).await?;
                Ok::<(SurrealStorage, SurrealStorage), Box<dyn std::error::Error>>((
                    current,
                    rollback_reopened,
                ))
            })).await;
            let (current, rollback_reopened) = match body {
                Ok(Ok(handles)) => handles,
                Ok(Err(error)) => {
                    let path = directory.keep();
                    let rollback_path = rollback_directory.keep();
                    panic!(
                        "migration proof failed: {error}; stores preserved at {} and {}",
                        path.display(),
                        rollback_path.display()
                    );
                }
                Err(panic) => {
                    eprintln!(
                        "MT109_MIGRATION_STORES_PRESERVED {} {}",
                        directory.path().display(),
                        rollback_directory.path().display()
                    );
                    let _ = directory.keep();
                    let _ = rollback_directory.keep();
                    std::panic::resume_unwind(panic);
                }
            };
            let path = directory.keep();
            crate::storage::tests::shutdown_and_remove_test_store(current, path.clone())
                .await
                .expect("remove closed migration store through centralized cleanup");
            assert!(!path.exists());
            eprintln!("MT109_MIGRATION_STORE_REMOVED {}", path.display());
            let rollback_path = rollback_directory.keep();
            crate::storage::tests::shutdown_and_remove_test_store(
                rollback_reopened,
                rollback_path.clone(),
            )
            .await
            .expect("remove closed rollback migration store through centralized cleanup");
            assert!(!rollback_path.exists());
            eprintln!(
                "MT109_ROLLBACK_MIGRATION_STORE_REMOVED {}",
                rollback_path.display()
            );
        }
    }

    #[tokio::test]
    async fn mt109_loom_catalog_dependencies_are_complete_and_deterministic() {
        let tables = loom_receipt_test_tables()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>();
        let sequences = loom_receipt_test_sequences()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(tables.len(), 16);
        let mut fingerprints = Vec::new();
        for _ in 0..2 {
            let client = Surreal::new::<Mem>(())
                .await
                .expect("open bounded Loom catalog");
            client
                .use_ns(DEFAULT_NAMESPACE)
                .use_db(DEFAULT_DATABASE)
                .await
                .expect("select bounded context");
            let database = SurrealAdminContext { client: &client };
            database
                .query(loom_receipt_test_schema_ddl())
                .await
                .expect("apply canonical bounded Loom dependencies");
            let fingerprint = inspect_catalog_fingerprint(
                &database,
                &tables,
                &sequences,
                CatalogInspectionScope::ExactDatabase,
            )
            .await
            .expect("inspect complete bounded Loom catalog");
            database
                .query("REMOVE FUNCTION fn::mt109_source_read;")
                .await
                .expect("remove exact source dependency");
            let error = inspect_catalog_fingerprint(
                &database,
                &tables,
                &sequences,
                CatalogInspectionScope::ExactDatabase,
            )
            .await
            .expect_err("missing source dependency must reject");
            assert!(
                error
                    .to_string()
                    .contains("HANDSHAKE_LOOM_SCHEMA_AUTHORITY_SET_MISMATCH"),
                "{error}"
            );
            fingerprints.push(fingerprint);
        }
        assert_eq!(fingerprints[0], fingerprints[1]);
        eprintln!("MT109_LOOM_CATALOG_SHA256={}", fingerprints[0]);
        assert_eq!(
            fingerprints[0],
            "2d3490115ab484a75ef8896bff1152b95798f3c4437fd45404898172aa87b1c2"
        );
    }

    #[tokio::test]
    async fn mt109_authority_catalog_pins_are_deterministic() {
        assert_eq!(
            sha256_hex(PRE_MT109_SCHEMA.as_bytes()),
            PRE_MT109_GENERATED_SURREALQL_SHA256
        );
        let mut current_pins = Vec::new();
        let mut predecessor_pins = Vec::new();
        for current in [false, true] {
            for _ in 0..2 {
                let client = Surreal::new::<Mem>(())
                    .await
                    .expect("open isolated catalog store");
                client
                    .use_ns(DEFAULT_NAMESPACE)
                    .use_db(DEFAULT_DATABASE)
                    .await
                    .expect("select catalog context");
                let database = SurrealAdminContext { client: &client };
                database
                    .query_bound(
                        if current { SCHEMA } else { PRE_MT109_SCHEMA },
                        BootstrapBindings {
                            schema_version: SCHEMA_VERSION.to_owned(),
                            schema_revision: SCHEMA_REVISION,
                            namespace: DEFAULT_NAMESPACE.to_owned(),
                            database: DEFAULT_DATABASE.to_owned(),
                            source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                            generated_surql_sha256: if current {
                                GENERATED_SURREALQL_SHA256
                            } else {
                                PRE_MT109_GENERATED_SURREALQL_SHA256
                            }
                            .to_owned(),
                        },
                    )
                    .await
                    .expect("apply catalog base");
                if !current {
                    database
                        .query(PRE_MT109_AUTHORITY_SCHEMA)
                        .await
                        .expect("apply committed predecessor authority");
                }
                let observed = if current {
                    inspect_schema(&database).await
                } else {
                    read_schema_catalog(&database).await
                }
                .expect("inspect complete authority catalog");
                if current {
                    current_pins.push(observed.info_fingerprint_sha256);
                } else {
                    predecessor_pins.push(observed.info_fingerprint_sha256);
                }
            }
        }
        assert_eq!(
            current_pins[0], current_pins[1],
            "fresh current catalog is nondeterministic"
        );
        assert_eq!(
            predecessor_pins[0], predecessor_pins[1],
            "committed overlay catalog is nondeterministic"
        );
        eprintln!("MT109_CURRENT_AUTHORITY_INFO_SHA256={}", current_pins[0]);
        eprintln!(
            "MT109_PREDECESSOR_AUTHORITY_INFO_SHA256={}",
            predecessor_pins[0]
        );
        assert_eq!(current_pins[0], EXPECTED_SCHEMA_INFO_SHA256);
        assert_eq!(predecessor_pins[0], PRE_MT109_AUTHORITY_INFO_SHA256);
    }

    #[tokio::test]
    async fn mt139_current_schema_info_pin_matches_fresh_mem_catalog() {
        let client = Surreal::new::<Mem>(()).await.expect("open memory store");
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await
            .expect("select memory context");
        let database = SurrealAdminContext { client: &client };
        database
            .query_bound(
                SCHEMA,
                BootstrapBindings {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    schema_revision: SCHEMA_REVISION,
                    namespace: DEFAULT_NAMESPACE.to_owned(),
                    database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                    generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                },
            )
            .await
            .expect("apply current schema in memory");
        let observed = inspect_schema(&database)
            .await
            .expect("inspect current memory schema");
        eprintln!(
            "MT139_CURRENT_SCHEMA_INFO_SHA256={}",
            observed.info_fingerprint_sha256
        );
        assert_eq!(
            observed.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
    }

    #[tokio::test]
    async fn mt138_bounded_bootstrap_transaction_rolls_back_on_failure() {
        let directory = tempfile::tempdir().expect("create MT-138 atomicity directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-138 atomicity store");
        let result: Result<(), SurrealStorageError> = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "BEGIN TRANSACTION; \
                             DEFINE TABLE OVERWRITE atelier_mt138_atomic_probe SCHEMAFULL; \
                             THROW 'MT138_INJECTED_BOOTSTRAP_FAILURE'; \
                             COMMIT TRANSACTION;",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(result.is_err());
        let tables: Vec<String> = storage
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        "RETURN array::sort(object::keys((INFO FOR DB).tables));",
                        (),
                    )
                    .await
                })
            })
            .await
            .expect("inspect MT-138 atomic rollback");
        assert!(!tables
            .iter()
            .any(|table| table == "atelier_mt138_atomic_probe"));
        storage.shutdown().await.expect("close atomicity store");
    }

    #[tokio::test]
    async fn mt138_structured_info_reports_reserved_value_field() {
        let directory = tempfile::tempdir().expect("create MT-138 field-info directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-138 field-info store");
        let (fields, definitions) = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "DEFINE TABLE OVERWRITE atelier_mt138_field_probe SCHEMAFULL PERMISSIONS NONE; \
                             DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_field_probe TYPE string; \
                             DEFINE TABLE OVERWRITE atelier_mt138_view_probe TYPE NORMAL AS \
                                 SELECT `value` FROM atelier_mt138_field_probe PERMISSIONS NONE;",
                        )
                        .await?;
                    let mut response = database
                        .query("INFO FOR TABLE atelier_mt138_field_probe STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid field INFO: {reason}"));
                    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
                    let database_info: SurrealValueData = response.take(0)?;
                    let definitions = parse_table_definitions(&database_info)
                        .unwrap_or_else(|reason| panic!("invalid table INFO: {reason}"));
                    Ok((fields, definitions))
                })
            })
            .await
            .expect("read structured field catalog");

        assert!(
            fields.iter().any(|field| field == "value"),
            "structured INFO omitted the reserved-name field: {fields:?}"
        );
        assert_eq!(
            definitions.get("atelier_mt138_field_probe"),
            Some(&AtelierTableDefinition {
                schemafull: true,
                kind: "NORMAL".to_owned(),
                is_view: false,
            })
        );
        assert_eq!(
            definitions.get("atelier_mt138_view_probe"),
            Some(&AtelierTableDefinition {
                schemafull: false,
                kind: "NORMAL".to_owned(),
                is_view: true,
            })
        );
        storage.shutdown().await.expect("close field-info store");
    }

    async fn index_names(
        storage: &SurrealStorage,
        table: &'static str,
    ) -> Result<Vec<String>, SurrealStorageError> {
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let mut names = parse_named_array(&info, "indexes")
                        .unwrap_or_else(|reason| panic!("invalid index INFO: {reason}"));
                    names.sort();
                    Ok(names)
                })
            })
            .await
    }

    #[test]
    fn declarative_schema_catalog_is_complete_and_content_sensitive() {
        let entries = compiled_schema_catalog_entries().expect("parse declarative schema catalog");
        assert_eq!(
            compute_catalog_hash(&entries),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );
        assert_eq!(compute_generated_surql_sha256(), GENERATED_SURREALQL_SHA256);
        assert_eq!(
            compute_knowledge_schema_registry_seed_sha256(),
            KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("table:"))
                .count(),
            TABLE_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("field:"))
                .count(),
            AUTHORED_FIELD_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("index:"))
                .count(),
            INDEX_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("event:"))
                .count(),
            EVENT_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("view:"))
                .count(),
            VIEW_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("sequence:"))
                .count(),
            SEQUENCE_DEFINITION_COUNT
        );

        for (prefix, expected) in [
            ("access:", ACCESS_DEFINITION_COUNT),
            ("function:", FUNCTION_DEFINITION_COUNT),
        ] {
            assert_eq!(
                entries
                    .iter()
                    .filter(|entry| entry.starts_with(prefix))
                    .count(),
                expected
            );
        }
        assert!(entries.contains(&"access:authenticated_session".to_owned()));
        assert_eq!(
            declarative_function_name("fn::mt109_live_session()"),
            Ok("fn::mt109_live_session")
        );
        assert!(declarative_function_name("fn::missing_opener").is_err());
        assert!(declarative_function_name("fn::()").is_err());
        let mut duplicate = entries.iter().cloned().collect::<BTreeSet<_>>();
        assert!(
            insert_catalog_identity(&mut duplicate, "access:authenticated_session".to_owned())
                .is_err()
        );
        assert!(
            insert_catalog_identity(&mut duplicate, "function:fn::mt109_has_grant".to_owned())
                .is_err()
        );
        assert!(resource_authority_schema_statements()
            .contains("DEFINE ACCESS IF NOT EXISTS authenticated_session"));

        let mut reordered = entries.clone();
        reordered.reverse();
        assert_eq!(
            compute_catalog_hash(&reordered),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );

        let mut altered = entries.clone();
        altered.push("table:attacker_rogue".to_owned());
        assert_ne!(
            compute_catalog_hash(&altered),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );
        assert_ne!(
            sha256_hex(format!("{SCHEMA}\n").as_bytes()),
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(
            sha256_hex(format!("{KNOWLEDGE_SCHEMA_REGISTRY_SEED}\n").as_bytes()),
            KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        );
    }

    #[test]
    fn canonical_info_sorts_named_catalog_entries_but_preserves_index_column_order() {
        let left = serde_json::json!({
            "indexes": [
                { "name": "z", "cols": ["first", "second"] },
                { "name": "a", "cols": ["only"] },
            ]
        });
        let reordered_catalog = serde_json::json!({
            "indexes": [
                { "name": "a", "cols": ["only"] },
                { "name": "z", "cols": ["first", "second"] },
            ]
        });
        let changed_index_order = serde_json::json!({
            "indexes": [
                { "name": "a", "cols": ["only"] },
                { "name": "z", "cols": ["second", "first"] },
            ]
        });

        assert_eq!(
            canonicalize_info(left.clone().into_value()),
            canonicalize_info(reordered_catalog.into_value())
        );
        assert_ne!(
            canonicalize_info(left.into_value()),
            canonicalize_info(changed_index_order.into_value())
        );
    }

    /// MT-142: the upgrade statements duplicate the schema.surql block on purpose (the fresh
    /// script is fresh-only); this pins them byte-for-byte to the declarative authority.
    #[test]
    fn mt142_title_anchor_upgrade_statements_match_schema() {
        let ddl_lines: Vec<&str> = MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS
            .lines()
            .filter(|line| line.starts_with("DEFINE "))
            .collect();
        assert_eq!(ddl_lines.len(), 10);
        for line in ddl_lines {
            assert!(
                SCHEMA.lines().any(|schema_line| schema_line == line),
                "MT-142 upgrade DDL drifted from schema.surql: {line}"
            );
        }
        assert!(KNOWLEDGE_SCHEMA_REGISTRY_SEED
            .contains("family_key: 'rich_document_title_anchors', table_name: 'knowledge_rich_document_title_anchors'"));
    }

    /// MT-151: same pin as `mt142_title_anchor_upgrade_statements_match_schema` for the
    /// The exact MT-152 `fems_workspace_write_anchors` block as it appears in `schema.surql`;
    /// removing it from the current script yields the byte-exact MT-151 pin.
    const MT152_FEMS_WRITE_ANCHORS_BLOCK: &str = concat!(
        "\n-- MT-152 fems_workspace_write_anchors: one write anchor per workspace, UPSERTed (with a\n",
        "-- fresh claim_nonce) by every FEMS transaction that creates a row referencing the workspace\n",
        "-- and by the workspace-delete transaction itself (UPSERT then DELETE, so the key is in its\n",
        "-- write set whether or not the row existed). The pinned engine detects conflicts only on\n",
        "-- keys a transaction writes, so a FEMS insert and a workspace delete that overlap in the\n",
        "-- engine now collide at commit instead of both committing with an orphan (MT-146 D-146-1,\n",
        "-- previously ordered only by FEMS_MUTATION_LOCK). Deliberately NOT a record<workspaces>\n",
        "-- reference: the delete removes it explicitly and no cascade scan is involved. A\n",
        "-- serialization device, not domain data.\n",
        "DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL PERMISSIONS NONE;\n",
        "DEFINE FIELD OVERWRITE anchor_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT $value = record::id($this.id);\n",
        "DEFINE FIELD OVERWRITE workspace_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';\n",
        "DEFINE FIELD OVERWRITE claim_nonce ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';\n",
        "DEFINE FIELD OVERWRITE updated_at ON TABLE fems_workspace_write_anchors TYPE datetime DEFAULT time::now();\n",
        "DEFINE INDEX OVERWRITE pk_fems_workspace_write_anchors ON TABLE fems_workspace_write_anchors FIELDS anchor_key UNIQUE;\n",
    );

    /// The exact MT-152 `loom_folders.sibling_key` field block and index line as they appear in
    /// `schema.surql` (I-152-2 sweep finding); removed with the FEMS block to reach the MT-151 pin.
    const MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK: &str = concat!(
        "-- MT-152 sibling_key: stored discriminator for sibling-name uniqueness covering ROOT folders.\n",
        "-- `uq_loom_folders_sibling_name` cannot: the engine skips uniqueness for any tuple containing\n",
        "-- NONE (surrealdb-core-3.2.0/src/idx/index.rs:190-197, NULL != NULL) and roots carry\n",
        "-- parent_folder_id = NONE. `workspace|parent-or-root|name`, set by every create, rename and\n",
        "-- re-parent write; the in-place upgrade backfills existing rows and disambiguates pre-existing\n",
        "-- root duplicates with a stable '#dup<n>' suffix on this key only (never on the visible name).\n",
        "DEFINE FIELD OVERWRITE sibling_key ON TABLE loom_folders TYPE string ASSERT string::trim($value) != '';\n",
    );
    const MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE: &str =
        "DEFINE INDEX OVERWRITE uq_loom_folders_sibling_key ON TABLE loom_folders FIELDS sibling_key UNIQUE;\n";

    /// The current script minus the MT-152 blocks: the exact MT-151 pin.
    fn mt151_pin_schema() -> String {
        for block in [
            MT152_FEMS_WRITE_ANCHORS_BLOCK,
            MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK,
            MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE,
        ] {
            assert_eq!(
                PRE_MT109_SCHEMA.matches(block).count(),
                1,
                "MT-152 block drifted: {block}"
            );
        }
        let pinned = PRE_MT109_SCHEMA
            .replace(MT152_FEMS_WRITE_ANCHORS_BLOCK, "")
            .replace(MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK, "")
            .replace(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE, "");
        assert_eq!(
            sha256_hex(pinned.as_bytes()),
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            "the pre-MT-152 allowlist must be exactly the current script minus the MT-152 block"
        );
        pinned
    }

    /// MT-152: the upgrade DDL is byte-identical (whitespace-normalised) to the fresh-script
    /// `fems_workspace_write_anchors` block, the table is in the inventory, and the lineage
    /// pins moved.
    #[test]
    fn mt152_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let upgrade = statements(MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS);
        assert_eq!(upgrade.len(), 6);
        assert_eq!(upgrade, statements(MT152_FEMS_WRITE_ANCHORS_BLOCK));
        let schema = statements(PRE_MT109_SCHEMA);
        for statement in &upgrade {
            assert!(
                schema
                    .iter()
                    .any(|schema_statement| schema_statement == statement),
                "MT-152 upgrade DDL drifted from schema.surql: {statement}"
            );
        }
        assert!(TABLE_NAMES.contains(&"fems_workspace_write_anchors"));
        assert!(!MT152_FEMS_WRITE_ANCHORS_BLOCK.contains("REFERENCE"));
        assert_ne!(
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT152_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
        // The MT-152 predecessor is the MT-151 current pin, so the two hops chain.
        assert_ne!(
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            PRE_MT151_GENERATED_SURREALQL_SHA256
        );
        let _ = mt151_pin_schema();
    }

    /// MT-152 (I-152-2): the folder `sibling_key` field and UNIQUE index statements applied by
    /// the upgrade are byte-identical (whitespace-normalised) to `schema.surql`, and the field
    /// precedes the index there (the backfill must be committed before the index builds).
    #[test]
    fn mt152_folder_sibling_key_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let field = statements(MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS);
        let index = statements(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS);
        assert_eq!(field.len(), 1);
        assert_eq!(index.len(), 1);
        assert_eq!(field, statements(MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK));
        assert_eq!(index, statements(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE));
        let schema = statements(SCHEMA);
        let field_at = schema
            .iter()
            .position(|s| s == &field[0])
            .expect("field in schema");
        let index_at = schema
            .iter()
            .position(|s| s == &index[0])
            .expect("index in schema");
        assert!(
            field_at < index_at,
            "sibling_key field must precede its UNIQUE index"
        );
        assert_eq!(
            super::super::loom_store::loom_folder_sibling_key("ws-1", None, " Root "),
            "v1|w4:ws-1|r|n4:Root"
        );
        assert_eq!(
            super::super::loom_store::loom_folder_sibling_key("ws-1", Some("LFD-p"), "Child"),
            "v1|w4:ws-1|p5:LFD-p|n5:Child"
        );
    }

    /// MT-152 (I-152-2): a store at the exact MT-151 pin holding two ROOT folders with one name
    /// (legal there - the composite index never covered roots) and one nested folder is upgraded
    /// in place: every row gains `sibling_key`, the later duplicate carries the `#dup1` suffix on
    /// the key only, its visible name is untouched, and the UNIQUE index then rejects a third
    /// root with that name through the product path with the same typed conflict as a nested
    /// duplicate.
    #[tokio::test]
    async fn mt152_exact_mt151_pin_upgrade_backfills_folder_sibling_keys_and_rejects_root_duplicates(
    ) {
        let mt151_pin_schema = mt151_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-151-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-151-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt151_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT152_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt152_folders CONTENT {{ name: 'folders' }}; \
                             CREATE kernel_event_ledger:mt152_folder_evt CONTENT {{ event_id: 'mt152_folder_evt', \
                             event_version: 'v1', kernel_task_run_id: 'run', session_run_id: 'session', \
                             aggregate_type: 'loom_folder', aggregate_id: 'seed', idempotency_key: 'mt152-folder-seed', \
                             event_type: 'seed', actor_kind: 'HUMAN', actor_id: 'test', payload_hash: 'seed', \
                             source_component: 'test', payload: {{ }} }}; \
                             CREATE loom_folders:mt152_root_a CONTENT {{ folder_id: 'mt152_root_a', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: NONE, name: 'Shared', \
                             event_ledger_event_id: kernel_event_ledger:mt152_folder_evt, \
                             created_at: d'2026-01-01T00:00:00Z' }}; \
                             CREATE loom_folders:mt152_root_b CONTENT {{ folder_id: 'mt152_root_b', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: NONE, name: 'Shared', \
                             event_ledger_event_id: kernel_event_ledger:mt152_folder_evt, \
                             created_at: d'2026-01-02T00:00:00Z' }}; \
                             CREATE loom_folders:mt152_child CONTENT {{ folder_id: 'mt152_child', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: loom_folders:mt152_root_a, \
                             name: 'Shared', event_ledger_event_id: kernel_event_ledger:mt152_folder_evt }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-151-pin store with duplicate root folders");
        storage.shutdown().await.expect("close MT-151-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-151-pin store");
        let upgraded = bootstrap_schema(&reopened)
            .await
            .expect("upgrade exact MT-151-pin store holding duplicate root folders");
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut keys = database
                        .query(
                            "SELECT VALUE sibling_key FROM loom_folders ORDER BY folder_id ASC; \
                             SELECT VALUE name FROM loom_folders ORDER BY folder_id ASC;",
                        )
                        .await?;
                    let sibling_keys: Vec<String> = keys.take(0)?;
                    let names: Vec<String> = keys.take(1)?;
                    assert_eq!(
                        sibling_keys,
                        vec![
                            "v1|w13:mt152_folders|p12:mt152_root_a|n6:Shared".to_owned(),
                            "v1|w13:mt152_folders|r|n6:Shared".to_owned(),
                            "v1|w13:mt152_folders|r|n6:Shared#dup1".to_owned(),
                        ]
                    );
                    assert_eq!(
                        names,
                        vec!["Shared"; 3],
                        "visible names are never rewritten"
                    );
                    Ok(())
                })
            })
            .await
            .expect("verify backfilled sibling keys");
        let db = SurrealDatabase::new(reopened.clone());
        let duplicate_root = db
            .create_loom_folder(
                "mt152_folders",
                NewLoomFolder {
                    folder_id: None,
                    workspace_id: "mt152_folders".to_owned(),
                    parent_folder_id: None,
                    name: "Shared".to_owned(),
                    color: None,
                    sort_mode: LoomFolderSortMode::UpdatedDesc,
                    sort_order: None,
                    project_ref: None,
                },
            )
            .await
            .expect_err("a third root 'Shared' must hit uq_loom_folders_sibling_key");
        assert!(
            matches!(
                duplicate_root,
                StorageError::Conflict("loom_folder_sibling_name")
            ),
            "root duplicate must surface the typed sibling-name conflict, got {duplicate_root}"
        );
        let renamed_into_collision = db
            .update_loom_folder(
                "mt152_folders",
                "mt152_child",
                LoomFolderUpdate {
                    parent_folder_id: Some(None),
                    ..LoomFolderUpdate::default()
                },
            )
            .await
            .expect_err("re-parenting the child to root under the taken name must be rejected");
        assert!(
            matches!(renamed_into_collision, StorageError::Conflict("loom_folder_sibling_name")),
            "re-parent into a root collision must surface the typed conflict, got {renamed_into_collision}"
        );
        let distinct = db
            .create_loom_folder(
                "mt152_folders",
                NewLoomFolder {
                    folder_id: None,
                    workspace_id: "mt152_folders".to_owned(),
                    parent_folder_id: None,
                    name: "Distinct".to_owned(),
                    color: None,
                    sort_mode: LoomFolderSortMode::UpdatedDesc,
                    sort_order: None,
                    project_ref: None,
                },
            )
            .await
            .expect("a distinct root name is still accepted after the upgrade");
        assert_eq!(distinct.name, "Distinct");
        reopened.shutdown().await.expect("close upgraded store");
    }

    /// journal-key and graph-anchor DDL; statements are compared whitespace-normalised because
    /// the fresh script and the upgrade both carry the multi-line `journal_key` VALUE verbatim.
    #[test]
    fn mt151_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let mut upgrade = statements(MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS);
        upgrade.extend(statements(
            MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS,
        ));
        assert_eq!(upgrade.len(), 9);
        let schema = statements(SCHEMA);
        for statement in &upgrade {
            assert!(
                schema
                    .iter()
                    .any(|schema_statement| schema_statement == statement),
                "MT-151 upgrade DDL drifted from schema.surql: {statement}"
            );
        }
        assert!(MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS.contains(
            "UPDATE loom_blocks SET updated_at = updated_at WHERE content_type = 'journal' AND journal_date != NONE RETURN NONE;"
        ));
        assert!(TABLE_NAMES.contains(&"storage_graph_anchors"));
        assert_ne!(
            PRE_MT151_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT151_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
    }

    #[test]
    fn predecessor_registry_hash_rejects_changed_nonempty_retired_source() {
        let expected = expected_predecessor_registry_metadata()
            .expect("exact predecessor registry metadata must be complete");
        assert_eq!(
            compute_predecessor_registry_hash(&expected),
            PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
        );

        let mut tampered = expected;
        tampered[0].retired_source = "attacker-controlled-nonempty.sql".to_owned();
        assert_ne!(
            compute_predecessor_registry_hash(&tampered),
            PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
        );
    }

    #[test]
    fn schema_contract_is_wave_scoped_and_identity_safe() {
        assert_eq!(
            TABLE_DEFINITION_COUNT,
            SOURCE_TABLE_COUNT + SOURCE_VIEW_COUNT + SURREAL_BOOTSTRAP_STATE_TABLE_COUNT
        );
        assert_eq!(
            INDEX_DEFINITION_COUNT,
            SOURCE_NAMED_INDEX_COUNT
                + SURREAL_PRIMARY_KEY_INDEX_COUNT
                + SURREAL_BOOTSTRAP_STATE_INDEX_COUNT
        );
        assert_eq!(
            SCHEMA.matches("DEFINE TABLE OVERWRITE ").count(),
            TABLE_DEFINITION_COUNT
        );
        assert_eq!(
            SCHEMA.matches("DEFINE FIELD OVERWRITE ").count(),
            AUTHORED_FIELD_DEFINITION_COUNT
        );
        assert_eq!(
            SCHEMA.matches(" FLEXIBLE").count(),
            FLEXIBLE_FIELD_DEFINITION_COUNT
        );
        let mut expected_type_any_wildcards = std::collections::BTreeSet::new();
        for definition in SCHEMA.lines().filter(|line| {
            line.starts_with("DEFINE FIELD OVERWRITE ") && line.contains(" FLEXIBLE")
        }) {
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let collection_depth =
                definition.matches("array<").count() + definition.matches("set<").count();
            let wildcard = format!(
                "DEFINE FIELD OVERWRITE {field}{} ON TABLE {table} TYPE any;",
                ".*".repeat(collection_depth + 1)
            );
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected SCHEMAFULL wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for {table}.{field}: {wildcard}"
            );
        }
        for definition in SCHEMA.lines().filter(|line| {
            line.starts_with("DEFINE FIELD OVERWRITE ")
                && (line.contains(" TYPE array;")
                    || line.contains(" TYPE array DEFAULT")
                    || line.contains(" TYPE option<array>;")
                    || line.contains(" TYPE option<array> DEFAULT"))
        }) {
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let wildcard = format!("DEFINE FIELD OVERWRITE {field}.* ON TABLE {table} TYPE any;");
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected untyped-array wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for {table}.{field}: {wildcard}"
            );
        }
        for definition in INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS {
            assert!(
                SCHEMA.lines().any(|line| line == definition),
                "missing intentional top-level TYPE any definition: {definition}"
            );
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let wildcard = format!("DEFINE FIELD OVERWRITE {field}.* ON TABLE {table} TYPE any;");
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected union-field wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for intentional union field {table}.{field}: {wildcard}"
            );
        }
        assert_eq!(
            expected_type_any_wildcards.len(),
            FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT
        );
        let type_any_definitions = SCHEMA
            .lines()
            .filter(|line| line.contains("TYPE any"))
            .collect::<Vec<_>>();
        assert_eq!(
            type_any_definitions.len(),
            FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT
                + INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS.len()
        );
        for definition in type_any_definitions {
            if INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS.contains(&definition) {
                continue;
            }
            assert!(
                expected_type_any_wildcards.remove(definition),
                "unauthorized TYPE any definition: {definition}"
            );
        }
        assert!(
            expected_type_any_wildcards.is_empty(),
            "missing expected TYPE any wildcards: {expected_type_any_wildcards:?}"
        );
        assert_eq!(
            generated_collection_subtype_field_count(SCHEMA),
            ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT
        );
        assert_eq!(
            FIELD_DEFINITION_COUNT,
            AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT
        );
        assert!(!SCHEMA.contains("array<any>"));
        assert!(!SCHEMA.contains("set<any>"));
        assert_eq!(
            SCHEMA.matches("DEFINE INDEX OVERWRITE ").count(),
            INDEX_DEFINITION_COUNT
        );
        assert_eq!(
            SCHEMA.matches("REFERENCE ON DELETE ").count(),
            REFERENCE_FIELD_COUNT
        );
        assert_eq!(
            SCHEMA.matches("record::exists($value)").count(),
            EXPLICIT_REFERENCE_EXISTENCE_ASSERTION_COUNT
        );
        assert_eq!(RECORD_ID_ONLY_TABLES.len(), 18);

        for (table, field) in REFERENCED_BUSINESS_KEY_ALIASES {
            let definition = SCHEMA
                .lines()
                .find(|line| {
                    line.starts_with(&format!(
                        "DEFINE FIELD OVERWRITE {field} ON TABLE {table} TYPE"
                    ))
                })
                .unwrap_or_else(|| panic!("missing business-key alias {table}.{field}"));
            assert!(definition.contains("ASSERT $value = record::id($this.id)"));
        }
        assert_eq!(
            SCHEMA.matches("record::id($this.id)").count(),
            RECORD_ID_ALIAS_ASSERTION_COUNT
        );
        for required_table in [
            "atelier_character",
            "atelier_source_evidence_record",
            "atelier_contact_sheet_raster_export_plan",
            "atelier_story_beat",
        ] {
            assert!(SCHEMA.contains(&format!(
                "DEFINE TABLE OVERWRITE {required_table} SCHEMAFULL PERMISSIONS NONE;"
            )));
        }
        assert!(SCHEMA.contains(
            "$value = type::record('atelier_source_evidence_record', [$this.matrix_id, record::id($value)[1]])"
        ));
        assert!(SCHEMA.contains("cascade_atelier_source_evidence_record"));
        assert!(!SCHEMA.contains("apply_state = 'applying'"));
        assert!(SCHEMA.contains("HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY"));
        for database_category in [
            "accesses",
            "analyzers",
            "apis",
            "buckets",
            "configs",
            "functions",
            "models",
            "modules",
            "params",
            "sequences",
            "tables",
            "users",
        ] {
            assert!(SCHEMA.contains(&format!(
                "array::len($existing_database.{database_category}) != 0"
            )));
        }
        assert!(SCHEMA.contains("generated_surql_sha256"));
        assert!(SCHEMA.contains("BEGIN TRANSACTION;"));
        assert!(SCHEMA.contains("COMMIT TRANSACTION;"));
        // No legacy server backend `jsonb` type token may survive the projection. Checked as a
        // whole identifier token, not a substring: source column NAMES such as
        // `attribution_jsonb` (migration 0311) are transcribed verbatim and are not
        // legacy server backend type syntax.
        let lowered = SCHEMA.to_ascii_lowercase();
        assert!(!lowered.contains("::jsonb"));
        assert!(!lowered
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|token| token == "jsonb"));
    }

    #[tokio::test]
    async fn bootstrap_is_concurrent_restart_safe_and_receipt_is_live() {
        let directory = tempfile::Builder::new()
            .prefix("mt109-bootstrap-")
            .tempdir()
            .expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");

        let mut reopened_storage = None;
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let left = storage.clone();
            let right = storage.clone();
            let (left_report, right_report) =
                tokio::join!(bootstrap_schema(&left), bootstrap_schema(&right),);
            let left_report = left_report.expect("left bootstrap");
            let right_report = right_report.expect("right bootstrap");
            assert_ne!(
                left_report.reused_existing_schema,
                right_report.reused_existing_schema
            );
            for report in [&left_report, &right_report] {
                assert_eq!(report.schema_version, SCHEMA_VERSION);
                assert_eq!(report.source_manifest_sha256, SCHEMA_LINEAGE_SHA256);
                assert_eq!(report.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
                assert_eq!(report.info_fingerprint_sha256.len(), 64);
                assert_eq!(report.tables_defined, TABLE_DEFINITION_COUNT);
                assert_eq!(report.fields_defined, FIELD_DEFINITION_COUNT);
                assert_eq!(report.indexes_defined, INDEX_DEFINITION_COUNT);
                assert_eq!(report.table_names.len(), TABLE_DEFINITION_COUNT);
            }
            let before_restart = index_names(&storage, "kernel_event_ledger")
                .await
                .expect("pre-restart INFO");
            storage.shutdown().await.expect("close first store");

            let reopened = open_test_storage(&directory).await.expect("reopen store");
            reopened_storage = Some(reopened.clone());
            let restarted = bootstrap_schema(&reopened)
                .await
                .expect("exact-current restart");
            assert!(restarted.reused_existing_schema);
            assert_eq!(
                before_restart,
                index_names(&reopened, "kernel_event_ledger")
                    .await
                    .expect("post-restart INFO")
            );
            reopened.shutdown().await.expect("close reopened store");
        }))
        .await;
        let mut cleanup_errors = Vec::new();
        if let Some(reopened) = reopened_storage.take() {
            if let Err(error) = reopened.shutdown().await {
                cleanup_errors.push(format!("reopened store shutdown: {error}"));
            }
            drop(reopened);
        }
        if let Err(error) = storage.shutdown().await {
            cleanup_errors.push(format!("original store shutdown: {error}"));
        }
        drop(storage);
        if !cleanup_errors.is_empty() {
            let path = directory.keep();
            panic!(
                "bootstrap proof cleanup failed; store preserved at {}: {}; body_failed={}",
                path.display(),
                cleanup_errors.join("; "),
                body.is_err()
            );
        }
        let path = directory.path().to_path_buf();
        directory
            .close()
            .expect("remove closed bootstrap proof store");
        assert!(
            !path.exists(),
            "bootstrap proof store survived cleanup: {}",
            path.display()
        );
        eprintln!("MT109_BOOTSTRAP_STORE_REMOVED {}", path.display());
        if let Err(panic) = body {
            std::panic::resume_unwind(panic);
        }
    }

    #[tokio::test]
    async fn mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current() {
        const CURRENT_HEADER: &str =
            "-- This transaction is the sole declarative schema authority. Rust bootstrap\n\
-- code verifies these exact bytes, parses every declared object into the pinned\n\
-- semantic catalog, and compares the applied live-engine catalog fail-closed.";
        const PREDECESSOR_HEADER: &str =
            "-- This transaction is the bounded Surreal-native projection of the source\n\
-- wave enumerated by `SOURCE_WAVE_FILES` in schema.rs (migrations 0001-0129\n\
-- plus the selected 0130-0365 bands). Every table created by a forward\n\
-- migration in that enumeration is defined here; the source enumeration is the\n\
-- only authority for which migrations are in the wave.";

        let predecessor_schema = PRE_MT109_SCHEMA.replace(CURRENT_HEADER, PREDECESSOR_HEADER).replace(
            "DEFINE FIELD OVERWRITE schema_source ON TABLE knowledge_schema_registry TYPE string;",
            "DEFINE FIELD OVERWRITE migration_file ON TABLE knowledge_schema_registry TYPE string;",
        );
        assert_eq!(
            sha256_hex(predecessor_schema.as_bytes()),
            PREDECESSOR_GENERATED_SURREALQL_SHA256,
            "predecessor allowlist must be derived from the exact preceding artifact"
        );
        let directory = tempfile::tempdir().expect("temporary predecessor store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open predecessor store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            predecessor_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PREDECESSOR_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    database
                        .query(PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED)
                        .await?;
                    ensure_supported_predecessor_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PREDECESSOR_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt139_predecessor CONTENT {{ name: 'sentinel' }};"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact predecessor store");
        storage.shutdown().await.expect("close predecessor store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen predecessor store");
        let upgraded = bootstrap_schema(&reopened)
            .await
            .expect("upgrade exact predecessor");
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut sentinel = database
                        .query("RETURN workspaces:mt139_predecessor.name;")
                        .await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    ensure_knowledge_schema_registry(&database).await?;
                    let mut response = database
                        .query("INFO FOR TABLE knowledge_schema_registry STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid registry INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "schema_source"));
                    assert!(!fields.iter().any(|field| field == "migration_file"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded data and registry");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    ensure_knowledge_schema_registry(&database).await?;
                    let mut sentinel = database
                        .query("RETURN workspaces:mt139_predecessor.name;")
                        .await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after upgrade");
        current.shutdown().await.expect("close current store");
    }

    /// MT-151: a store at the exact MT-142 pin (the current script minus the MT-151 blocks,
    /// proven byte-exact against `PRE_MT151_GENERATED_SURREALQL_SHA256`) holding a journal
    /// block is upgraded in place: the journal key is materialised on the existing row, the
    /// UNIQUE index then rejects a second journal for that date, `storage_graph_anchors`
    /// exists, the live fingerprint is the current pin, and the state survives a reopen.
    #[tokio::test]
    async fn mt151_exact_mt142_pin_upgrade_materialises_journal_key_and_restarts_current() {
        const JOURNAL_KEY_BLOCK: &str = concat!(
            "-- MT-151 journal_key: stored discriminator for the journal get-or-create natural key\n",
            "-- (workspace, journal_date). NONE for every non-journal row, and the engine skips NONE\n",
            "-- tuples in UNIQUE indexes (surrealdb-core-3.2.0/src/idx/index.rs:193-197), so only\n",
            "-- journal blocks are constrained. Guards the invariant LOOM_MUTATION_LOCK alone used to\n",
            "-- hold, on every write path (get-or-create, create, update.journal_date).\n",
            "DEFINE FIELD OVERWRITE journal_key ON TABLE loom_blocks TYPE option<string>\n",
            "    VALUE IF $this.content_type = 'journal' AND $this.journal_date != NONE {\n",
            "        type::string($this.workspace_id) + '|' + $this.journal_date\n",
            "    } ELSE {\n",
            "        NONE\n",
            "    };\n",
        );
        const JOURNAL_INDEX_LINE: &str =
            "DEFINE INDEX OVERWRITE uq_loom_blocks_journal_key ON TABLE loom_blocks FIELDS journal_key UNIQUE;\n";
        const GRAPH_ANCHORS_BLOCK: &str = concat!(
            "\n-- MT-151 storage_graph_anchors: one version row per graph whose acyclicity is decided\n",
            "-- Rust-side (the Loom folder tree per workspace, the work-packet dependency graph).\n",
            "-- The deciding operation reads the version before its graph read and compare-and-sets\n",
            "-- it inside the committing transaction (THROW on mismatch, UPSERT to bump), so a\n",
            "-- decision against a stale graph fails closed and two writers that overlap in the\n",
            "-- engine collide on this one key at commit. A serialization device, not domain data.\n",
            "DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS NONE;\n",
            "DEFINE FIELD OVERWRITE anchor_key ON TABLE storage_graph_anchors TYPE string ASSERT $value = record::id($this.id);\n",
            "DEFINE FIELD OVERWRITE graph_kind ON TABLE storage_graph_anchors TYPE 'loom_folder_tree' | 'work_packet_dependencies';\n",
            "DEFINE FIELD OVERWRITE scope_key ON TABLE storage_graph_anchors TYPE string ASSERT string::trim($value) != '';\n",
            "DEFINE FIELD OVERWRITE version ON TABLE storage_graph_anchors TYPE int ASSERT $value >= 1;\n",
            "DEFINE FIELD OVERWRITE updated_at ON TABLE storage_graph_anchors TYPE datetime DEFAULT time::now();\n",
            "DEFINE INDEX OVERWRITE pk_storage_graph_anchors ON TABLE storage_graph_anchors FIELDS anchor_key UNIQUE;\n",
        );
        for block in [JOURNAL_KEY_BLOCK, JOURNAL_INDEX_LINE, GRAPH_ANCHORS_BLOCK] {
            assert_eq!(
                PRE_MT109_SCHEMA.matches(block).count(),
                1,
                "MT-151 block drifted: {block}"
            );
        }
        // The MT-152 block sits on top of the MT-151 pin, so the MT-142 pin is the current
        // script minus both; the pre-MT-151 path now applies MT-151 and MT-152 together.
        let mt142_pin_schema = mt151_pin_schema()
            .replace(JOURNAL_KEY_BLOCK, "")
            .replace(JOURNAL_INDEX_LINE, "")
            .replace(GRAPH_ANCHORS_BLOCK, "");
        assert_eq!(
            sha256_hex(mt142_pin_schema.as_bytes()),
            PRE_MT151_GENERATED_SURREALQL_SHA256,
            "the pre-MT-151 allowlist must be exactly the current script minus the MT-151 and MT-152 blocks"
        );

        let directory = tempfile::tempdir().expect("temporary MT-142-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-142-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt142_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT151_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT151_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt151_pin CONTENT {{ name: 'sentinel' }}; \
                             CREATE loom_blocks:mt151_journal CONTENT {{ block_id: 'mt151_journal', \
                             workspace_id: workspaces:mt151_pin, content_type: 'journal', \
                             title: 'Daily Note 2026-09-11', journal_date: '2026-09-11' }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-142-pin store with a journal block");
        storage.shutdown().await.expect("close MT-142-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-142-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT151_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-142-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut materialised = database
                        .query("RETURN loom_blocks:mt151_journal.journal_key;")
                        .await?;
                    let journal_key: Option<String> = materialised.take(0)?;
                    assert_eq!(
                        journal_key.as_deref(),
                        Some("workspaces:mt151_pin|2026-09-11"),
                        "the upgrade must materialise journal_key on the existing journal row"
                    );
                    let duplicate = database
                        .query(
                            "CREATE loom_blocks:mt151_duplicate CONTENT { block_id: 'mt151_duplicate', \
                             workspace_id: workspaces:mt151_pin, content_type: 'journal', \
                             title: 'Daily Note 2026-09-11', journal_date: '2026-09-11' };",
                        )
                        .await
                        .and_then(|response| Ok(response.check()?));
                    let rendered = duplicate
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_default();
                    assert!(
                        rendered.contains("uq_loom_blocks_journal_key"),
                        "a second journal for the date must lose to the upgraded index: {rendered}"
                    );
                    let mut anchors = database
                        .query("INFO FOR TABLE storage_graph_anchors STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = anchors.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid anchors INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "version"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded journal key, index and anchors table");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    let mut sentinel = database.query("RETURN workspaces:mt151_pin.name;").await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after the MT-151 upgrade");
        current.shutdown().await.expect("close current store");
    }

    /// MT-152: a store at the exact MT-151 pin (the current script minus the MT-152 block,
    /// proven byte-exact against `PRE_MT152_GENERATED_SURREALQL_SHA256`) holding a workspace
    /// and a FEMS pack is upgraded in place: `fems_workspace_write_anchors` exists and accepts
    /// an UPSERT, the live fingerprint is the current pin, every application row survives, and
    /// the state survives a reopen.
    #[tokio::test]
    async fn mt152_exact_mt151_pin_upgrade_adds_fems_write_anchors_and_restarts_current() {
        let mt151_pin_schema = mt151_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-151-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-151-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt151_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT152_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt152_pin CONTENT {{ name: 'sentinel' }}; \
                             CREATE fems_memory_packs:mt152_pack CONTENT {{ pack_id: 'mt152_pack', \
                             workspace_id: workspaces:mt152_pin, scope_key: '', pack: {{ v: 1 }}, \
                             generated_at: time::now() }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-151-pin store with a workspace and a FEMS pack");
        storage.shutdown().await.expect("close MT-151-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-151-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT152_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-151-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut anchors = database
                        .query("INFO FOR TABLE fems_workspace_write_anchors STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = anchors.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid anchors INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "claim_nonce"));
                    database
                        .query(
                            "UPSERT fems_workspace_write_anchors:mt152_pin SET \
                             anchor_key = 'mt152_pin', workspace_key = 'mt152_pin', \
                             claim_nonce = 'nonce-1';",
                        )
                        .await?
                        .check()?;
                    let mut pack = database
                        .query("RETURN fems_memory_packs:mt152_pack.pack_id;")
                        .await?;
                    let pack_id: Option<String> = pack.take(0)?;
                    assert_eq!(pack_id.as_deref(), Some("mt152_pack"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded anchors table and surviving rows");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    let mut sentinel = database.query("RETURN workspaces:mt152_pin.name;").await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after the MT-152 upgrade");
        current.shutdown().await.expect("close current store");
    }

    #[tokio::test]
    async fn bootstrap_resumes_exact_current_schema_applied_state() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            SCHEMA,
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                            },
                        )
                        .await?;
                    let pending = read_context_and_state(&database)
                        .await?
                        .expect("schema transaction must write pending state");
                    assert!(pending.is_schema_applied_current());
                    Ok(())
                })
            })
            .await
            .expect("install schema without finalization");

        let resumed = bootstrap_schema(&storage)
            .await
            .expect("resume exact-current schema_applied state");
        assert!(resumed.reused_existing_schema);
        assert_eq!(resumed.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let finalized = read_context_and_state(&database)
                        .await?
                        .expect("finalized state must exist");
                    assert!(finalized.is_exact_current());
                    Ok(())
                })
            })
            .await
            .expect("post-verify finalized state");
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn fresh_bootstrap_rejects_and_preserves_preexisting_data() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("CREATE preexisting:keep SET marker = 'untouched';")
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed pre-existing record");

        let error = bootstrap_schema(&storage)
            .await
            .expect_err("non-empty database must be rejected");
        assert!(
            error
                .to_string()
                .contains("HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY"),
            "unexpected non-empty database error: {error}"
        );
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database.query("RETURN preexisting:keep.marker;").await?;
                    let marker: Option<String> = response.take(0)?;
                    let marker = marker.expect("pre-existing marker must remain readable");
                    assert_eq!(marker, "untouched");
                    Ok(())
                })
            })
            .await
            .expect("pre-existing record remains intact");
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn bootstrap_rejects_lower_or_divergent_lineage() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "DEFINE TABLE handshake_schema_state SCHEMALESS; \
                             CREATE handshake_schema_state:primary CONTENT {{ \
                               version: '{SCHEMA_VERSION}', revision: 28, \
                               namespace: '{DEFAULT_NAMESPACE}', database: '{DEFAULT_DATABASE}', \
                               source_manifest_sha256: '{SCHEMA_LINEAGE_SHA256}', \
                               generated_surql_sha256: '{GENERATED_SURREALQL_SHA256}', \
                               info_fingerprint_sha256: '0000000000000000000000000000000000000000000000000000000000000000', \
                               apply_state: 'complete', target_revision: 28 \
                             }};"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed lower lineage");

        let error = bootstrap_schema(&storage)
            .await
            .expect_err("lower lineage must fail closed");
        assert!(error
            .to_string()
            .contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"));
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn exact_current_bootstrap_rejects_complete_info_tampering() {
        let tamper_queries = [
            (
                "index definition",
                "DEFINE INDEX OVERWRITE idx_ai_jobs_gc ON TABLE ai_jobs FIELDS created_at, status, is_pinned;",
            ),
            ("sequence removal", "REMOVE SEQUENCE kernel_event_sequence;"),
            (
                "field assertion",
                "DEFINE FIELD OVERWRITE size_bytes ON TABLE assets TYPE int ASSERT $value >= -1;",
            ),
        ];

        for (label, tamper_query) in tamper_queries {
            let directory = tempfile::tempdir().expect("temporary Surreal directory");
            let storage = open_test_storage(&directory)
                .await
                .expect("open fresh store");
            bootstrap_schema(&storage).await.expect("bootstrap schema");
            storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(tamper_query).await?;
                        Ok(())
                    })
                })
                .await
                .unwrap_or_else(|error| panic!("apply {label} tamper: {error}"));

            let error = match bootstrap_schema(&storage).await {
                Ok(_) => panic!("{label} tamper must be rejected"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH"),
                "unexpected {label} verdict: {error}"
            );
            storage.shutdown().await.expect("close store");
        }
    }

    #[tokio::test]
    async fn native_json_fields_round_trip_real_domain_serialization_and_reject_wrong_shapes() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        let metrics = JobMetrics::zero();
        let entity_refs = vec![EntityRef {
            entity_id: "document:serde".to_owned(),
            entity_kind: "document".to_owned(),
        }];
        let planned_operations = vec![PlannedOperation {
            op_type: OperationType::Read,
            target: entity_refs[0].clone(),
            description: Some("read representative document".to_owned()),
        }];
        let metrics_json = serde_json::to_value(&metrics).expect("serialize JobMetrics");
        let entity_refs_json =
            serde_json::to_value(&entity_refs).expect("serialize EntityRef list");
        let planned_operations_json =
            serde_json::to_value(&planned_operations).expect("serialize PlannedOperation list");
        let job_inputs_json = serde_json::json!({ "document_id": "serde" });
        let expected = serde_json::json!({
            "entity_refs": entity_refs_json,
            "planned_operations": planned_operations_json,
            "metrics": metrics_json,
            "job_inputs": job_inputs_json,
        });

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query_bound(
                            "CREATE ai_jobs:json_roundtrip SET \
                               trace_id = '00000000-0000-0000-0000-000000000001', \
                               job_kind = 'manual_prompt', status = 'queued', \
                               protocol_id = 'test', profile_id = 'test', \
                               capability_profile_id = 'test', access_mode = 'read_only', \
                               safety_mode = 'strict', entity_refs = $entity_refs, \
                               planned_operations = $planned_operations, metrics = $metrics, \
                               job_inputs = $job_inputs; \
                             RETURN { \
                               entity_refs: ai_jobs:json_roundtrip.entity_refs, \
                               planned_operations: ai_jobs:json_roundtrip.planned_operations, \
                               metrics: ai_jobs:json_roundtrip.metrics, \
                               job_inputs: ai_jobs:json_roundtrip.job_inputs \
                             };",
                            NativeJsonBindings {
                                entity_refs: expected["entity_refs"].clone(),
                                planned_operations: expected["planned_operations"].clone(),
                                metrics: expected["metrics"].clone(),
                                job_inputs: expected["job_inputs"].clone(),
                            },
                        )
                        .await?;
                    let observed: Option<JsonValue> = response.take(1)?;
                    let observed = observed.expect("native JSON readback must exist");
                    assert_eq!(observed, expected);
                    let restored_metrics: JobMetrics =
                        serde_json::from_value(observed["metrics"].clone())
                            .expect("deserialize JobMetrics readback");
                    assert_eq!(
                        serde_json::to_value(restored_metrics).expect("reserialize JobMetrics"),
                        expected["metrics"]
                    );
                    Ok(())
                })
            })
            .await
            .expect("native JSON bind and readback");

        for (label, wrong_shape) in [
            (
                "metrics string",
                "UPDATE ai_jobs:json_roundtrip SET metrics = 'not-an-object';",
            ),
            (
                "entity refs object",
                "UPDATE ai_jobs:json_roundtrip SET entity_refs = {};",
            ),
            (
                "job inputs array",
                "UPDATE ai_jobs:json_roundtrip SET job_inputs = [];",
            ),
        ] {
            let result = storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(wrong_shape).await?;
                        Ok(())
                    })
                })
                .await;
            assert!(result.is_err(), "{label} must fail SCHEMAFULL validation");
        }
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn record_references_reject_orphans_and_preserve_identity_semantics() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        let orphan = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE documents:orphan SET \
                             workspace_id = workspaces:missing, title = 'orphan';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            orphan.is_err(),
            "required orphan reference must be rejected"
        );

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(
                            "CREATE workspaces:identity SET name = 'Identity'; \
                             CREATE documents:child SET workspace_id = workspaces:identity, title = 'Child'; \
                             CREATE blocks:grandchild SET document_id = documents:child, \
                               kind = 'paragraph', sequence = 0, raw_content = 'raw', \
                               display_content = 'display', derived_content = {}; \
                             RETURN documents:child.workspace_id.name; \
                             DELETE workspaces:identity; \
                             RETURN record::exists(documents:child); \
                             RETURN record::exists(blocks:grandchild);",
                        )
                        .await?;
                    let dereferenced_name: Option<String> = response.take(3)?;
                    let child_remains: Option<bool> = response.take(5)?;
                    let grandchild_remains: Option<bool> = response.take(6)?;
                    let dereferenced_name =
                        dereferenced_name.expect("dereferenced workspace name must exist");
                    let child_remains = child_remains.expect("child existence result must exist");
                    let grandchild_remains =
                        grandchild_remains.expect("grandchild existence result must exist");
                    assert_eq!(dereferenced_name, "Identity");
                    assert!(!child_remains, "cascade must remove the referring record");
                    assert!(
                        !grandchild_remains,
                        "multi-hop cascade must remove the grandchild record"
                    );
                    Ok(())
                })
            })
            .await
            .expect("identity, dereference, and delete behavior");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE work_packets:wp_identity SET \
                             wp_id = 'wp_identity', version = 1, title = 'Identity', \
                             status = 'ready', priority = 1, task_board_status = 'READY', \
                             reporter = 'test', created_at = 'now', updated_at = 'now', \
                             vector_clock = '{}', metadata = '{}';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("matching business-key alias");
        let identity_change = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("UPDATE work_packets:wp_identity SET wp_id = 'different';")
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            identity_change.is_err(),
            "business-key alias must be immutable"
        );
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn optional_unset_and_reject_self_references_enforce_delete_contracts() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(
                            "CREATE workspaces:unset_ws SET name = 'Unset'; \
                             CREATE assets:unset_asset SET asset_id = 'unset_asset', \
                               workspace_id = workspaces:unset_ws, kind = 'file', \
                               mime = 'text/plain', content_hash = 'unset-hash', size_bytes = 1; \
                             CREATE loom_blocks:unset_block SET block_id = 'unset_block', \
                               workspace_id = workspaces:unset_ws, content_type = 'file', \
                               asset_id = assets:unset_asset, derived_json = {}; \
                             DELETE assets:unset_asset; \
                             RETURN record::exists(loom_blocks:unset_block); \
                             RETURN loom_blocks:unset_block.asset_id = NONE;",
                        )
                        .await?;
                    let block_remains: Option<bool> = response.take(4)?;
                    let reference_was_unset: Option<bool> = response.take(5)?;
                    let block_remains = block_remains.expect("block existence result must exist");
                    let reference_was_unset =
                        reference_was_unset.expect("UNSET comparison result must exist");
                    assert!(block_remains);
                    assert!(reference_was_unset);
                    Ok(())
                })
            })
            .await
            .expect("optional reference ON DELETE UNSET");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE adapter_checkpoint:parent SET created_at = 'now', \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'parent'; \
                             CREATE adapter_checkpoint:child SET created_at = 'now', \
                               parent_checkpoint_id = adapter_checkpoint:parent, \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'child';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("valid adapter self-reference");
        let rejected_delete = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database.query("DELETE adapter_checkpoint:parent;").await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            rejected_delete.is_err(),
            "REJECT must protect referenced parent"
        );
        let orphan_self_reference = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE adapter_checkpoint:orphan SET created_at = 'now', \
                               parent_checkpoint_id = adapter_checkpoint:missing, \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'orphan';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            orphan_self_reference.is_err(),
            "self-reference must target an existing adapter"
        );
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn uuid_backed_record_ids_reject_textual_identity_aliases() {
        const THREAD_UUID: &str = "018f0000-0000-7000-8000-000000000001";
        const MESSAGE_UUID: &str = "018f0000-0000-7000-8000-000000000002";
        const OTHER_UUID: &str = "018f0000-0000-7000-8000-000000000003";

        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(format!(
                            "CREATE role_mailbox_thread:u'{THREAD_UUID}' SET \
                               thread_id = u'{THREAD_UUID}', title = 'Typed UUID', \
                               linked_record_kind = 'test', lifecycle_state = 'open', \
                               claim_mode = 'exclusive', takeover_policy = 'reject', \
                               response_authority_scope = 'thread'; \
                             CREATE role_mailbox_message:u'{MESSAGE_UUID}' SET \
                               message_id = u'{MESSAGE_UUID}', \
                               thread_id = role_mailbox_thread:u'{THREAD_UUID}', \
                               message_type = 'request', from_role = 'tester', \
                               delivery_state = 'queued', body = {{ purpose: 'uuid-proof' }}; \
                             RETURN record::id(role_mailbox_thread:u'{THREAD_UUID}');"
                        ))
                        .await?;
                    let observed_id: Option<uuid::Uuid> = response.take(2)?;
                    let observed_id = observed_id.expect("typed UUID record id must exist");
                    assert_eq!(observed_id.to_string(), THREAD_UUID);
                    Ok(())
                })
            })
            .await
            .expect("typed UUID record identity and reference");

        for (label, invalid_query) in [
            (
                "textual reference to UUID-backed target",
                format!(
                    "CREATE role_mailbox_message:u'{OTHER_UUID}' SET \
                       message_id = u'{OTHER_UUID}', \
                       thread_id = role_mailbox_thread:'{THREAD_UUID}', \
                       message_type = 'request', from_role = 'tester', \
                       delivery_state = 'queued', body = {{}};"
                ),
            ),
            (
                "textual record ID with typed UUID alias",
                format!(
                    "CREATE role_mailbox_thread:'{OTHER_UUID}' SET \
                       thread_id = u'{OTHER_UUID}', title = 'Wrong key kind', \
                       linked_record_kind = 'test', lifecycle_state = 'open', \
                       claim_mode = 'exclusive', takeover_policy = 'reject', \
                       response_authority_scope = 'thread';"
                ),
            ),
        ] {
            let result = storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(invalid_query).await?;
                        Ok(())
                    })
                })
                .await;
            assert!(result.is_err(), "{label} must be rejected");
        }
        storage.shutdown().await.expect("close store");
    }
}
