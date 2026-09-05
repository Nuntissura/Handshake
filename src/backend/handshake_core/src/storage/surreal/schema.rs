use std::collections::{BTreeMap, BTreeSet};

use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use surrealdb::types::{
    Array as SurrealArray, Object as SurrealObject, SurrealValue, Value as SurrealValueData,
};
use tokio::sync::Mutex;

use super::{
    SurrealAdminContext, SurrealStorage, SurrealStorageError, DEFAULT_DATABASE, DEFAULT_NAMESPACE,
};

pub const SCHEMA_VERSION: &str = "wp-kernel-012-surreal-v1";
pub const SCHEMA_REVISION: i64 = 158;
/// Stable v1 lineage identifier retained so existing embedded stores remain readable after the
/// legacy schema-provenance corpus is removed. New source integrity is proven independently by
/// [`DECLARATIVE_SCHEMA_CATALOG_SHA256`] and [`GENERATED_SURREALQL_SHA256`].
pub const SCHEMA_LINEAGE_SHA256: &str =
    "225ed19c0259ef121867ca5da1995813db0c48ee0cbfaded2d871e47b50f7fc1";
const PREDECESSOR_GENERATED_SURREALQL_SHA256: &str =
    "c21630b082cd8c505199cc54877d12edfbfcc6069e50f77d28e5b36cb5c8fac0";
const PREDECESSOR_SCHEMA_INFO_SHA256: &str =
    "6b4e5a157a3ce6ceaee9ded4d04843fc0387be1fb1fefc69a2203262cd8a1938";
const PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256: &str =
    "1f8443486cd7101babb56dd6264ffcf08538a1eae24016d2155b19d5eb6370b4";
pub const GENERATED_SURREALQL_SHA256: &str =
    "9d3b830c78ffb4ac04b5af0cbdac510b29483decd076acb3e8ea81d194da4c8f";
pub const DECLARATIVE_SCHEMA_CATALOG_SHA256: &str =
    "8d72ae48dd23fab0d4f13a750da8daad46e156a276aca207bec665016163fb27";
pub const KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256: &str =
    "f51ef10d8ebc0c728a075e7a5efe4a19503cd46dea2cfa0f1bfe59332f2e34fa";
/// Fresh-engine STRUCTURE fingerprint captured with the product-locked SurrealDB 3.2.0
/// engine family after applying the generated schema to an absent RocksDB path.
pub const EXPECTED_SCHEMA_INFO_SHA256: &str =
    "047b1decfd076e9237d5aa5d6f8836801cbba38038ec9d84a278ebe5b69a275e";
const EXPECTED_ATELIER_CATALOG_SHA256: &str =
    "f14a7778809d15a07fbdeae930002637af15ea42d8945b6c9636dc2d15ff274c";
const PENDING_SCHEMA_INFO_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

const SCHEMA: &str = include_str!("schema.surql");
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
const TABLE_DEFINITION_COUNT: usize = 292;
const SOURCE_FIELD_DEFINITION_COUNT: usize = 3217;
const FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT: usize = 249;
const FLEXIBLE_FIELD_DEFINITION_COUNT: usize = 180;
const INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS: [&str; 2] = [
    "DEFINE FIELD OVERWRITE capability_grants ON TABLE atelier_transcript_receipt TYPE any DEFAULT [];",
    "DEFINE FIELD OVERWRITE decisions ON TABLE knowledge_retrieval_traces TYPE any DEFAULT [];",
];
const AUTHORED_FIELD_DEFINITION_COUNT: usize =
    SOURCE_FIELD_DEFINITION_COUNT + FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT;
// SurrealDB 3.2 persists one `field.*` subtype definition per non-Any typed collection nesting
// level. Structured INFO reads the full persisted field catalog, so these engine-generated
// definitions are part of the exact live schema even though they are not authored DEFINE lines.
const ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT: usize = 48;
const FIELD_DEFINITION_COUNT: usize =
    AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT;
const INDEX_DEFINITION_COUNT: usize = 831;
const EVENT_DEFINITION_COUNT: usize = 19;
const VIEW_DEFINITION_COUNT: usize = 2;
const SEQUENCE_DEFINITION_COUNT: usize = 2;
const SOURCE_TABLE_COUNT: usize = 289;
const SOURCE_VIEW_COUNT: usize = 2;
const SOURCE_NAMED_INDEX_COUNT: usize = 563;
const SURREAL_PRIMARY_KEY_INDEX_COUNT: usize = 267;
const SURREAL_BOOTSTRAP_STATE_TABLE_COUNT: usize = 1;
const SURREAL_BOOTSTRAP_STATE_INDEX_COUNT: usize = 1;
const REFERENCE_FIELD_COUNT: usize = 419;
const RECORD_ID_ALIAS_ASSERTION_COUNT: usize = 225;

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
    for line in SCHEMA.lines().map(str::trim_start) {
        if let Some(rest) = line.strip_prefix("DEFINE TABLE OVERWRITE ") {
            let table = rest.split_ascii_whitespace().next().unwrap_or_default();
            if table.starts_with("atelier_") || table == "kernel_event_ledger" {
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
        if !table.starts_with("atelier_") && table != "kernel_event_ledger" {
            continue;
        }
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

    let relevant_tables = table_definitions
        .keys()
        .filter(|name| match scope {
            CatalogInspectionScope::Atelier => {
                name.starts_with("atelier_") || *name == "kernel_event_ledger"
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

    let envelope = AtelierCatalogInfoEnvelope {
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

    for line in SCHEMA.lines() {
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
    ddl
}

/// Provisions the exact production-schema tables exercised by the focused Loom
/// mutation-receipt tests. Definitions are selected mechanically from the same
/// compiled `schema.surql` as production bootstrap; no test-owned DDL is used.
#[cfg(test)]
pub async fn bootstrap_loom_receipt_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    const EXPECTED_CATALOG_SHA256: &str =
        "77ab023e8e57bee576b0350cf47e24e2a998805edb854a89f143956f42390993";
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

    let mut ddl = Vec::new();
    let mut include_continuation = false;
    for line in SCHEMA.lines() {
        let trimmed = line.trim_start();
        if include_continuation
            || selected_table_statement(trimmed, loom_receipt_test_tables())
            || selected_sequence_statement(trimmed, loom_receipt_test_sequences())
        {
            ddl.push(line);
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
    "handshake_schema_state",
    "workspaces",
    "documents",
    "blocks",
    "canvases",
    "canvas_nodes",
    "canvas_edges",
    "ai_jobs",
    "workflow_runs",
    "workflow_node_executions",
    "model_sessions",
    "model_session_checkpoints",
    "model_session_messages",
    "ai_embedding_models",
    "ai_embedding_registry",
    "ai_bronze_records",
    "ai_silver_records",
    "assets",
    "loom_blocks",
    "loom_edges",
    "ai_job_mcp_fields",
    "calendar_sources",
    "calendar_events",
    "work_packets",
    "micro_tasks",
    "mt_iterations",
    "governance_check_runs",
    "dependencies",
    "skill_log_entry",
    "skill_log_file_ref",
    "distill_job",
    "distill_example",
    "adapter_checkpoint",
    "eval_run",
    "replay_candidates",
    "kernel_event_ledger",
    "kernel_session_queue",
    "kernel_crdt_updates",
    "kernel_crdt_snapshots",
    "kernel_process_lifecycle",
    "role_mailbox_thread",
    "role_mailbox_message",
    "role_mailbox_claim_lease",
    "role_mailbox_handoff_bundle",
    "kernel_micro_task_job",
    "kernel_mt_loop_checkpoint",
    "kernel_mt_outcome",
    "kernel_distillation_candidate",
    "kernel_session_checkpoint",
    "kernel_restart_resume_report",
    "kernel_idempotency_ledger",
    "kernel_model_session_span",
    "kernel_activity_span",
    "atelier_character",
    "atelier_sheet_version",
    "atelier_media_asset",
    "atelier_event",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_collection",
    "atelier_collection_item",
    "atelier_contact_sheet",
    "atelier_tag",
    "atelier_character_tag",
    "atelier_tag_rule",
    "atelier_similarity_projection",
    "atelier_export_request",
    "atelier_export_result",
    "atelier_export_manifest_entry",
    "atelier_media_annotation",
    "atelier_preference",
    "atelier_pose_rig",
    "atelier_pose_head_pose",
    "atelier_pose_calibration",
    "atelier_identity_profile",
    "atelier_comfy_bridge_probe",
    "atelier_comfy_capability_registration",
    "atelier_comfy_declared_output",
    "atelier_comfy_capability_reject",
    "atelier_comfy_intake_output",
    "atelier_comfy_fallback_marker",
    "atelier_sourcing_spec",
    "atelier_handler_version_matrix",
    "atelier_sourcing_binding_decision",
    "atelier_version_mismatch_receipt",
    "atelier_sourcing_ingestion_receipt",
    "atelier_media_probe_report",
    "atelier_transcript_artifact",
    "atelier_caption_artifact",
    "atelier_transcript_receipt",
    "atelier_md_output_root",
    "atelier_md_allowlist_policy",
    "atelier_md_auth_context",
    "atelier_md_download_session",
    "atelier_md_item_state",
    "atelier_md_checkpoint",
    "atelier_md_session_receipt",
    "atelier_command_corpus_entry",
    "atelier_command_corpus_blocked",
    "atelier_command_corpus_parity_report",
    "atelier_stealth_window",
    "atelier_stealth_ref",
    "atelier_stealth_capture",
    "atelier_sheet_parse_snapshot",
    "atelier_bulk_operation_receipt",
    "atelier_trash_marker",
    "atelier_source_evidence_record",
    "atelier_anchor_verification_record",
    "atelier_media_review_metadata",
    "atelier_media_derivative",
    "atelier_similarity_rebuild_job",
    "atelier_ai_tag_suggestion",
    "atelier_media_sidecar",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_image_import_request",
    "atelier_media_source_provenance_ref",
    "atelier_intake_item_rejection_audit",
    "atelier_export_intake_link",
    "atelier_media_asset_tag",
    "atelier_collection_metadata_application",
    "atelier_contact_sheet_svg_artifact",
    "atelier_contact_sheet_raster_export_plan",
    "atelier_character_document",
    "atelier_character_document_version",
    "atelier_story_card",
    "atelier_story_beat",
    "atelier_character_script",
    "atelier_bracket_link_projection",
    "atelier_moodboard",
    "atelier_moodboard_operation_receipt",
    "atelier_moodboard_export_request",
    "atelier_character_relationship",
    "atelier_character_relationship_graph_projection",
    "atelier_saved_search",
    "atelier_web_portfolio_export_request",
    "atelier_web_portfolio_export_result",
    "atelier_backup_manifest",
    "atelier_backup_restore_preflight",
    "atelier_state_probe_catalog_entry",
    "atelier_action_receipt",
    "atelier_reset_operation",
    "atelier_orphan_manifest",
    "atelier_orphan_manifest_item",
    "atelier_pose_sidecar",
    "atelier_pose_context_state",
    "atelier_pose_workspace_rig_state",
    "atelier_identity_crop_artifact",
    "atelier_comfy_workflow_receipt",
    "atelier_comfy_output_registration_failure",
    "atelier_pose_deferred_feature",
    "atelier_comfy_workflow_spec",
    "atelier_comfy_version_metadata",
    "atelier_comfy_job",
    "atelier_comfy_diagnostic_bundle",
    "atelier_diagnostics_validation_matrix",
    "atelier_diagnostics_error_taxonomy",
    "atelier_diagnostics_prompt_response_matrix",
    "atelier_command_log",
    "atelier_diagnostics_session",
    "atelier_model_config",
    "atelier_model_apply",
    "atelier_synthetic_input_guard",
    "atelier_work_state_projection",
    "atelier_dcc_panel_projection",
    "atelier_screenshot_artifact_storage",
    "atelier_spec_drift_finding",
    "atelier_dcc_workflow_panel_projection",
    "atelier_fr_workflow_event",
    "atelier_model_manual_section",
    "atelier_retrieval_policy",
    "atelier_self_improve_sandbox_run",
    "atelier_validator_first_pass_run",
    "atelier_model_coordination_lease",
    "kernel_diagnostic_bundle_manifest",
    "atelier_model_manual_row_merge",
    "atelier_model_manual_drift_guard",
    "kernel_visual_diff_baseline",
    "kernel_visual_diff_request",
    "kernel_visual_diff_result",
    "atelier_visual_steer_feedback",
    "knowledge_schema_registry",
    "knowledge_source_roots",
    "knowledge_sources",
    "knowledge_index_runs",
    "knowledge_spans",
    "knowledge_entities",
    "knowledge_entity_spans",
    "knowledge_edges",
    "knowledge_edge_spans",
    "knowledge_claims",
    "knowledge_claim_spans",
    "knowledge_claim_conflicts",
    "knowledge_memory_passages",
    "knowledge_passage_evidence",
    "knowledge_wiki_projections",
    "knowledge_rich_documents",
    "knowledge_rich_document_versions",
    "knowledge_editor_code_nodes",
    "knowledge_context_bundles",
    "knowledge_context_bundle_items",
    "knowledge_retrieval_traces",
    "knowledge_idempotency_keys",
    "knowledge_crdt_denial_receipts",
    "knowledge_crdt_agent_lane_leases",
    "knowledge_crdt_graph_proposals",
    "knowledge_crdt_promoted_facts",
    "knowledge_crdt_ai_edit_proposals",
    "knowledge_crdt_swarm_checkpoints",
    "knowledge_crdt_recovery_receipts",
    "knowledge_ingestion_root_policies",
    "knowledge_ingestion_policy_decisions",
    "knowledge_ingestion_kind_registry",
    "knowledge_ingestion_receipts",
    "knowledge_ingestion_spans",
    "knowledge_ingestion_repair_queue",
    "knowledge_code_files",
    "knowledge_code_scip_imports",
    "knowledge_code_repair_queue",
    "knowledge_memory_ontology_terms",
    "knowledge_memory_ontology_aliases",
    "knowledge_memory_facts",
    "knowledge_memory_conflict_detection_jobs",
    "knowledge_memory_conflict_detection_findings",
    "knowledge_memory_conflict_resolution_jobs",
    "knowledge_memory_bridge_decisions",
    "knowledge_semantic_catalog_entries",
    "knowledge_document_embeds",
    "knowledge_document_backlinks",
    "loom_block_knowledge_bridge",
    "loom_folders",
    "loom_folder_members",
    "loom_wiki_overlays",
    "user_manual_pages",
    "user_manual_sections",
    "user_manual_anchors",
    "user_manual_tool_entries",
    "user_manual_feature_entries",
    "user_manual_versions",
    "user_manual_legacy_aliases",
    "knowledge_agent_worktree_claims",
    "knowledge_agent_role_mailbox_handoffs",
    "knowledge_agent_state_recovery_checkpoints",
    "knowledge_agent_recovery_receipts",
    "knowledge_parallel_indexing_lease_queue",
    "knowledge_agent_quiet_background_work",
    "knowledge_agent_cloud_assistance_receipts",
    "knowledge_quick_switcher_recents",
    "knowledge_workbench_layout_states",
    "knowledge_workspace_settings_states",
    "knowledge_rich_document_drafts",
    "knowledge_workspace_search_bookmark_states",
    "knowledge_debug_breakpoints",
    "media_asset_tiers",
    "loom_collections",
    "loom_collection_members",
    "loom_ai_suggestions",
    "loom_canvas_boards",
    "loom_canvas_placements",
    "loom_canvas_visual_edges",
    "loom_block_search_index",
    "calendar_activity_spans",
    "stage_capture_artifacts",
    "knowledge_rich_document_loom_projection_0343_state",
    "atelier_intake_item_loom_projection",
    "fems_memory_packs",
    "fems_memory_proposals",
    "fems_memory_items",
    "fems_memory_commit_reports",
    "fems_memory_commit_fr_outbox",
    "fems_memory_lifecycle_fr_outbox",
    "calendar_mutation_outbox",
    "preference_records",
    "preference_change_receipts",
    "loom_block_view_fr_outbox",
    "fems_memory_proposal_request_id_rekey",
    "kb003_sandbox_policies",
    "kb003_sandbox_runs",
    "kb003_validation_runs",
    "kb003_promotion_decisions",
    "kb003_promotion_receipts",
    "atelier_tag_note",
    "atelier_ckc_search_projection",
    "atelier_sheet_field_value_projection",
    "atelier_sheet_artifact_link",
    "atelier_intake_item_metadata",
    "atelier_prompt_feedback_case",
    "atelier_prompt_feedback_verdict",
    "atelier_prompt_feedback_rule_pack",
    "atelier_prompt_feedback_rewrite",
    "atelier_prompt_feedback_export",
    "atelier_collection_item_unlink_receipt",
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

    fn is_exact_supported_predecessor(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == PREDECESSOR_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PREDECESSOR_SCHEMA_INFO_SHA256
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
    if rows.len() != 62 {
        return Err(format!(
            "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_COUNT: expected=62 observed={}",
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
                    Some(state) if state.is_exact_supported_predecessor() => {
                        verified_observed =
                            Some(upgrade_supported_predecessor(&database, &state).await?);
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
    let observed_counts = (tables.len(), fields, indexes, events, views, sequences);
    let expected_counts = (
        TABLE_DEFINITION_COUNT,
        AUTHORED_FIELD_DEFINITION_COUNT,
        INDEX_DEFINITION_COUNT,
        EVENT_DEFINITION_COUNT,
        VIEW_DEFINITION_COUNT,
        SEQUENCE_DEFINITION_COUNT,
    );
    if observed_counts != expected_counts {
        return Err(format!(
            "declarative schema catalog counts differ: observed={observed_counts:?}; expected={expected_counts:?}"
        ));
    }

    Ok(entries.into_iter().collect())
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

use crate::storage::artifacts::sha256_hex;

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
    let predecessor_observed = inspect_schema(database).await?;
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
UPDATE ONLY handshake_schema_state:primary SET
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,
    apply_state = 'schema_applied',
    updated_at = time::now();
COMMIT TRANSACTION;
"#,
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

    let mut expected_names = TABLE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected_names.sort();
    if table_names != expected_names {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_TABLE_SET_MISMATCH: expected={expected_names:?}; observed={table_names:?}"
            ),
        )
        .await;
    }

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
        table_info_by_name.insert(table.clone(), canonicalize_info(table_info));
    }

    if fields_defined != FIELD_DEFINITION_COUNT || indexes_defined != INDEX_DEFINITION_COUNT {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_MISMATCH: tables={}; fields={fields_defined}; indexes={indexes_defined}",
                table_names.len()
            ),
        )
        .await;
    }

    let canonical = CanonicalInfoEnvelope {
        database: canonicalize_info(db_info),
        tables: table_info_by_name,
    };
    let canonical_json =
        serde_json::to_string(&canonical).expect("canonical structured INFO serializes losslessly");

    Ok(ObservedSchema {
        info_fingerprint_sha256: sha256_hex(canonical_json.as_bytes()),
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
        surreal::{SurrealStorage, SurrealStorageConfig},
        EntityRef, JobMetrics, OperationType, PlannedOperation,
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

    async fn mt138_canonical_mem_fingerprint() -> Result<String, SurrealStorageError> {
        let expected_tables = atelier_expected_catalog()
            .into_keys()
            .collect::<BTreeSet<_>>();
        let expected_sequences = ATELIER_REQUIRED_SEQUENCES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        mt138_mem_catalog_fingerprint(
            format!(
                "BEGIN TRANSACTION;\n{}\nCOMMIT TRANSACTION;",
                atelier_schema_ddl()
            ),
            &expected_tables,
            &expected_sequences,
        )
        .await
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
            let first = mt138_canonical_mem_fingerprint()
                .await
                .expect("generate first fresh canonical Atelier fingerprint");
            let second = mt138_canonical_mem_fingerprint()
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
            REFERENCE_FIELD_COUNT
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
            "record::exists(type::record('atelier_source_evidence_record', [$this.matrix_id, $value]))"
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
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");

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

        let predecessor_schema = SCHEMA.replace(CURRENT_HEADER, PREDECESSOR_HEADER).replace(
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
                             status = 'ready', priority = 1, task_board_status = 'ready', \
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
