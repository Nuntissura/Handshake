use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use surrealdb::types::{
    Array as SurrealArray, Object as SurrealObject, SurrealValue, Value as SurrealValueData,
};
use tokio::sync::Mutex;

use super::{
    SurrealAdminContext, SurrealStorage, SurrealStorageError,
};

pub const SCHEMA_VERSION: &str = "wp-kernel-012-surreal-v1";
pub const SCHEMA_REVISION: i64 = 157;
pub const SOURCE_FORWARD_MIGRATION_COUNT: usize = 157;
pub const SOURCE_FORWARD_WAVE_MANIFEST_SHA256: &str =
    "225ed19c0259ef121867ca5da1995813db0c48ee0cbfaded2d871e47b50f7fc1";
pub const GENERATED_SURREALQL_SHA256: &str =
    "fa749e9190b6940f255fa160090e3a0f4fa46e95a31da960226e95047e04a785";
/// Two-stage proof pin. Captured 2026-09-03 from fresh-engine STRUCTURE receipts on the
/// WP-1 embedded-Surreal tree (schema.surql fixed `type::is::array` -> `type::is_array`, so
/// this branch fingerprint necessarily differs from any pre-fix lineage).
///
/// Receipt evidence: eight independent freshly-created embedded RocksDB engines each applied the
/// canonical schema and reported the identical live INFO fingerprint below, alongside structural
/// counts that reconcile against the authored source (270 tables; 3214 fields = 3172 authored
/// DEFINE FIELD lines + 42 engine-generated collection subtypes; 769 indexes = authored DEFINE
/// INDEX lines). Log: ${HANDSHAKE_ARTIFACTS_ROOT}/handshake-test/wp1-mmo-v6/wp1-mmo-v6-kb0902/
/// 2026-09-03T10-51-23-007Z-parent-mt002-suite.log
///
/// Known limitation: reproducibility is proven across engines on one host and one SurrealDB
/// build. It is not yet proven host- or engine-version-independent.
pub const EXPECTED_SCHEMA_INFO_SHA256: &str =
    "ae45afb78e02a7984c741d275311b07154a834c5668cd918c999cf6f415847dc";
const PENDING_SCHEMA_INFO_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

const SCHEMA: &str = include_str!("schema.surql");
const SOURCE_MANIFEST_DOMAIN: &[u8] = b"handshake.surreal.source-wave-manifest.v1\0";
pub(super) const BOOTSTRAP_STATE_TABLE: &str = "handshake_schema_state";
const BOOTSTRAP_STATE_ID: &str = "handshake_schema_state:primary";
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
const TABLE_DEFINITION_COUNT: usize = 270;
const SOURCE_FIELD_DEFINITION_COUNT: usize = 2940;
const FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT: usize = 232;
const FLEXIBLE_FIELD_DEFINITION_COUNT: usize = 170;
const AUTHORED_FIELD_DEFINITION_COUNT: usize =
    SOURCE_FIELD_DEFINITION_COUNT + FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT;
// SurrealDB 3.2 persists one `field.*` subtype definition per non-Any typed collection nesting
// level. Structured INFO reads the full persisted field catalog, so these engine-generated
// definitions are part of the exact live schema even though they are not authored DEFINE lines.
const ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT: usize = 42;
const FIELD_DEFINITION_COUNT: usize =
    AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT;
const INDEX_DEFINITION_COUNT: usize = 769;
const SOURCE_TABLE_COUNT: usize = 267;
const SOURCE_VIEW_COUNT: usize = 2;
const SOURCE_NAMED_INDEX_COUNT: usize = 519;
const SURREAL_PRIMARY_KEY_INDEX_COUNT: usize = 249;
const SURREAL_BOOTSTRAP_STATE_TABLE_COUNT: usize = 1;
const SURREAL_BOOTSTRAP_STATE_INDEX_COUNT: usize = 1;
const REFERENCE_FIELD_COUNT: usize = 388;
const RECORD_ID_ALIAS_ASSERTION_COUNT: usize = 217;

static BOOTSTRAP_LOCKS: Mutex<BTreeMap<(String, String), Arc<Mutex<()>>>> =
    Mutex::const_new(BTreeMap::new());

/// Serializes bootstrap per embedded namespace/database rather than per process.
///
/// Concurrent callers against the SAME namespace/database are still fully serialized, which
/// is the invariant the original process-wide guard existed to protect. Callers against
/// DIFFERENT namespaces/databases no longer queue behind each other: every isolated test
/// store owns a distinct namespace/database, so the process-wide guard made an N-test suite
/// pay N times one full bootstrap (measured 2026-09-03: ~820s each, so a 10-test suite cost
/// 8224s instead of one bootstrap plus overhead).
async fn bootstrap_guard_for(namespace: &str, database: &str) -> Arc<Mutex<()>> {
    let mut locks = BOOTSTRAP_LOCKS.lock().await;
    locks
        .entry((namespace.to_owned(), database.to_owned()))
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

const SOURCE_WAVE_FILES: [(&str, &[u8]); SOURCE_FORWARD_MIGRATION_COUNT] = [
    (
        "0001_init.sql",
        include_bytes!("../../../migrations/0001_init.sql"),
    ),
    (
        "0002_create_ai_core_tables.sql",
        include_bytes!("../../../migrations/0002_create_ai_core_tables.sql"),
    ),
    (
        "0003_add_is_pinned.sql",
        include_bytes!("../../../migrations/0003_add_is_pinned.sql"),
    ),
    (
        "0004_mutation_traceability.sql",
        include_bytes!("../../../migrations/0004_mutation_traceability.sql"),
    ),
    (
        "0005_add_canvas_traceability.sql",
        include_bytes!("../../../migrations/0005_add_canvas_traceability.sql"),
    ),
    (
        "0006_expand_ai_job_model.sql",
        include_bytes!("../../../migrations/0006_expand_ai_job_model.sql"),
    ),
    (
        "0007_workflow_persistence.sql",
        include_bytes!("../../../migrations/0007_workflow_persistence.sql"),
    ),
    (
        "0008_expand_ai_job_model.sql",
        include_bytes!("../../../migrations/0008_expand_ai_job_model.sql"),
    ),
    (
        "0009_add_block_classification.sql",
        include_bytes!("../../../migrations/0009_add_block_classification.sql"),
    ),
    (
        "0010_normalize_ai_job_kind.sql",
        include_bytes!("../../../migrations/0010_normalize_ai_job_kind.sql"),
    ),
    (
        "0011_normalize_micro_task_execution.sql",
        include_bytes!("../../../migrations/0011_normalize_micro_task_execution.sql"),
    ),
    (
        "0012_ai_ready_data_arch.sql",
        include_bytes!("../../../migrations/0012_ai_ready_data_arch.sql"),
    ),
    (
        "0013_loom_mvp.sql",
        include_bytes!("../../../migrations/0013_loom_mvp.sql"),
    ),
    (
        "0014_ai_job_mcp_fields.sql",
        include_bytes!("../../../migrations/0014_ai_job_mcp_fields.sql"),
    ),
    (
        "0015_calendar_storage.sql",
        include_bytes!("../../../migrations/0015_calendar_storage.sql"),
    ),
    (
        "0016_locus_structured_collaboration.sql",
        include_bytes!("../../../migrations/0016_locus_structured_collaboration.sql"),
    ),
    (
        "0017_skill_bank_distillation.sql",
        include_bytes!("../../../migrations/0017_skill_bank_distillation.sql"),
    ),
    (
        "0018_kernel_event_ledger.sql",
        include_bytes!("../../../migrations/0018_kernel_event_ledger.sql"),
    ),
    (
        "0019_kernel_session_queue.sql",
        include_bytes!("../../../migrations/0019_kernel_session_queue.sql"),
    ),
    (
        "0020_kernel_crdt_storage.sql",
        include_bytes!("../../../migrations/0020_kernel_crdt_storage.sql"),
    ),
    (
        "0021_kernel_process_lifecycle.sql",
        include_bytes!("../../../migrations/0021_kernel_process_lifecycle.sql"),
    ),
    (
        "0022_role_mailbox_threads_messages.sql",
        include_bytes!("../../../migrations/0022_role_mailbox_threads_messages.sql"),
    ),
    (
        "0023_micro_task_job_queue.sql",
        include_bytes!("../../../migrations/0023_micro_task_job_queue.sql"),
    ),
    (
        "0024_session_checkpoint.sql",
        include_bytes!("../../../migrations/0024_session_checkpoint.sql"),
    ),
    (
        "0025_observability_spans.sql",
        include_bytes!("../../../migrations/0025_observability_spans.sql"),
    ),
    (
        "0026_mt_scheduler_starvation_watermark.sql",
        include_bytes!("../../../migrations/0026_mt_scheduler_starvation_watermark.sql"),
    ),
    (
        "0027_mt_outcome_distillation_status.sql",
        include_bytes!("../../../migrations/0027_mt_outcome_distillation_status.sql"),
    ),
    (
        "0028_restart_resume_report_wiring.sql",
        include_bytes!("../../../migrations/0028_restart_resume_report_wiring.sql"),
    ),
    (
        "0029_bitemporal_event_ledger_indexes.sql",
        include_bytes!("../../../migrations/0029_bitemporal_event_ledger_indexes.sql"),
    ),
    (
        "0030_atelier_foundation.sql",
        include_bytes!("../../../migrations/0030_atelier_foundation.sql"),
    ),
    (
        "0031_atelier_core_data.sql",
        include_bytes!("../../../migrations/0031_atelier_core_data.sql"),
    ),
    (
        "0032_atelier_pose_diagnostics.sql",
        include_bytes!("../../../migrations/0032_atelier_pose_diagnostics.sql"),
    ),
    (
        "0033_atelier_event_ledger_projection.sql",
        include_bytes!("../../../migrations/0033_atelier_event_ledger_projection.sql"),
    ),
    (
        "0034_atelier_preference_metadata.sql",
        include_bytes!("../../../migrations/0034_atelier_preference_metadata.sql"),
    ),
    (
        "0035_atelier_stealth_uuid_v7_bound_ids.sql",
        include_bytes!("../../../migrations/0035_atelier_stealth_uuid_v7_bound_ids.sql"),
    ),
    (
        "0036_atelier_downloader_capability_grants.sql",
        include_bytes!("../../../migrations/0036_atelier_downloader_capability_grants.sql"),
    ),
    (
        "0037_atelier_sheet_parser_ast.sql",
        include_bytes!("../../../migrations/0037_atelier_sheet_parser_ast.sql"),
    ),
    (
        "0038_atelier_contact_sheet_schema_namespace.sql",
        include_bytes!("../../../migrations/0038_atelier_contact_sheet_schema_namespace.sql"),
    ),
    (
        "0039_atelier_bulk_operation_receipts.sql",
        include_bytes!("../../../migrations/0039_atelier_bulk_operation_receipts.sql"),
    ),
    (
        "0040_atelier_media_artifact_manifest.sql",
        include_bytes!("../../../migrations/0040_atelier_media_artifact_manifest.sql"),
    ),
    (
        "0041_atelier_source_evidence_matrix.sql",
        include_bytes!("../../../migrations/0041_atelier_source_evidence_matrix.sql"),
    ),
    (
        "0042_atelier_source_evidence_matrix_scope.sql",
        include_bytes!("../../../migrations/0042_atelier_source_evidence_matrix_scope.sql"),
    ),
    (
        "0043_atelier_media_review_metadata.sql",
        include_bytes!("../../../migrations/0043_atelier_media_review_metadata.sql"),
    ),
    (
        "0044_atelier_media_derivatives.sql",
        include_bytes!("../../../migrations/0044_atelier_media_derivatives.sql"),
    ),
    (
        "0045_atelier_similarity_rebuild_jobs.sql",
        include_bytes!("../../../migrations/0045_atelier_similarity_rebuild_jobs.sql"),
    ),
    (
        "0046_atelier_ai_tag_suggestions.sql",
        include_bytes!("../../../migrations/0046_atelier_ai_tag_suggestions.sql"),
    ),
    (
        "0047_atelier_media_sidecars.sql",
        include_bytes!("../../../migrations/0047_atelier_media_sidecars.sql"),
    ),
    (
        "0048_atelier_filesystem_health.sql",
        include_bytes!("../../../migrations/0048_atelier_filesystem_health.sql"),
    ),
    (
        "0049_atelier_image_import.sql",
        include_bytes!("../../../migrations/0049_atelier_image_import.sql"),
    ),
    (
        "0050_atelier_media_source_provenance_refs.sql",
        include_bytes!("../../../migrations/0050_atelier_media_source_provenance_refs.sql"),
    ),
    (
        "0051_atelier_intake_batch_resume.sql",
        include_bytes!("../../../migrations/0051_atelier_intake_batch_resume.sql"),
    ),
    (
        "0052_atelier_intake_item_lifecycle.sql",
        include_bytes!("../../../migrations/0052_atelier_intake_item_lifecycle.sql"),
    ),
    (
        "0053_atelier_intake_profile_targets.sql",
        include_bytes!("../../../migrations/0053_atelier_intake_profile_targets.sql"),
    ),
    (
        "0054_atelier_export_intake_links.sql",
        include_bytes!("../../../migrations/0054_atelier_export_intake_links.sql"),
    ),
    (
        "0055_atelier_collection_metadata_application.sql",
        include_bytes!("../../../migrations/0055_atelier_collection_metadata_application.sql"),
    ),
    (
        "0056_atelier_contact_sheet_svg_artifact.sql",
        include_bytes!("../../../migrations/0056_atelier_contact_sheet_svg_artifact.sql"),
    ),
    (
        "0057_atelier_contact_sheet_raster_export_plan.sql",
        include_bytes!("../../../migrations/0057_atelier_contact_sheet_raster_export_plan.sql"),
    ),
    (
        "0058_atelier_character_documents.sql",
        include_bytes!("../../../migrations/0058_atelier_character_documents.sql"),
    ),
    (
        "0059_atelier_story_cards_beats.sql",
        include_bytes!("../../../migrations/0059_atelier_story_cards_beats.sql"),
    ),
    (
        "0061_atelier_character_scripts.sql",
        include_bytes!("../../../migrations/0061_atelier_character_scripts.sql"),
    ),
    (
        "0064_atelier_bracket_links.sql",
        include_bytes!("../../../migrations/0064_atelier_bracket_links.sql"),
    ),
    (
        "0072_atelier_moodboard_schema_layer_model.sql",
        include_bytes!("../../../migrations/0072_atelier_moodboard_schema_layer_model.sql"),
    ),
    (
        "0074_atelier_moodboard_operations_exports.sql",
        include_bytes!("../../../migrations/0074_atelier_moodboard_operations_exports.sql"),
    ),
    (
        "0082_atelier_character_relationships.sql",
        include_bytes!("../../../migrations/0082_atelier_character_relationships.sql"),
    ),
    (
        "0083_atelier_saved_searches.sql",
        include_bytes!("../../../migrations/0083_atelier_saved_searches.sql"),
    ),
    (
        "0084_atelier_web_portfolio_exports.sql",
        include_bytes!("../../../migrations/0084_atelier_web_portfolio_exports.sql"),
    ),
    (
        "0085_atelier_backup_manifests.sql",
        include_bytes!("../../../migrations/0085_atelier_backup_manifests.sql"),
    ),
    (
        "0086_atelier_state_probe_catalog.sql",
        include_bytes!("../../../migrations/0086_atelier_state_probe_catalog.sql"),
    ),
    (
        "0087_atelier_action_receipts.sql",
        include_bytes!("../../../migrations/0087_atelier_action_receipts.sql"),
    ),
    (
        "0089_atelier_reset_orphan_adoption.sql",
        include_bytes!("../../../migrations/0089_atelier_reset_orphan_adoption.sql"),
    ),
    (
        "0090_atelier_pose_sidecars.sql",
        include_bytes!("../../../migrations/0090_atelier_pose_sidecars.sql"),
    ),
    (
        "0092_atelier_pose_context_state.sql",
        include_bytes!("../../../migrations/0092_atelier_pose_context_state.sql"),
    ),
    (
        "0093_atelier_pose_workspace_rig_state.sql",
        include_bytes!("../../../migrations/0093_atelier_pose_workspace_rig_state.sql"),
    ),
    (
        "0100_atelier_identity_crop_artifact.sql",
        include_bytes!("../../../migrations/0100_atelier_identity_crop_artifact.sql"),
    ),
    (
        "0102_atelier_comfy_workflow_receipt.sql",
        include_bytes!("../../../migrations/0102_atelier_comfy_workflow_receipt.sql"),
    ),
    (
        "0103_atelier_comfy_output_registration_failure.sql",
        include_bytes!("../../../migrations/0103_atelier_comfy_output_registration_failure.sql"),
    ),
    (
        "0105_atelier_pose_deferred_feature.sql",
        include_bytes!("../../../migrations/0105_atelier_pose_deferred_feature.sql"),
    ),
    (
        "0106_atelier_comfy_workflow_spec.sql",
        include_bytes!("../../../migrations/0106_atelier_comfy_workflow_spec.sql"),
    ),
    (
        "0107_atelier_comfy_version_metadata.sql",
        include_bytes!("../../../migrations/0107_atelier_comfy_version_metadata.sql"),
    ),
    (
        "0108_atelier_comfy_job_queue.sql",
        include_bytes!("../../../migrations/0108_atelier_comfy_job_queue.sql"),
    ),
    (
        "0109_atelier_comfy_diagnostic_bundle.sql",
        include_bytes!("../../../migrations/0109_atelier_comfy_diagnostic_bundle.sql"),
    ),
    (
        "0111_atelier_diagnostics_validation_matrix.sql",
        include_bytes!("../../../migrations/0111_atelier_diagnostics_validation_matrix.sql"),
    ),
    (
        "0112_atelier_diagnostics_typed_surfaces.sql",
        include_bytes!("../../../migrations/0112_atelier_diagnostics_typed_surfaces.sql"),
    ),
    (
        "0113_atelier_command_log_session_heartbeat.sql",
        include_bytes!("../../../migrations/0113_atelier_command_log_session_heartbeat.sql"),
    ),
    (
        "0114_atelier_model_config_apply.sql",
        include_bytes!("../../../migrations/0114_atelier_model_config_apply.sql"),
    ),
    (
        "0115_atelier_diagnostics_projections.sql",
        include_bytes!("../../../migrations/0115_atelier_diagnostics_projections.sql"),
    ),
    (
        "0116_atelier_dcc_flight_recorder.sql",
        include_bytes!("../../../migrations/0116_atelier_dcc_flight_recorder.sql"),
    ),
    (
        "0117_atelier_editable_surface_authority.sql",
        include_bytes!("../../../migrations/0117_atelier_editable_surface_authority.sql"),
    ),
    (
        "0118_atelier_self_improve_runs.sql",
        include_bytes!("../../../migrations/0118_atelier_self_improve_runs.sql"),
    ),
    (
        "0119_atelier_model_coordination_lease.sql",
        include_bytes!("../../../migrations/0119_atelier_model_coordination_lease.sql"),
    ),
    (
        "0120_kernel_diagnostic_bundle_manifest.sql",
        include_bytes!("../../../migrations/0120_kernel_diagnostic_bundle_manifest.sql"),
    ),
    (
        "0122_atelier_model_manual_merge_drift.sql",
        include_bytes!("../../../migrations/0122_atelier_model_manual_merge_drift.sql"),
    ),
    (
        "0124_kernel_visual_diff_baseline.sql",
        include_bytes!("../../../migrations/0124_kernel_visual_diff_baseline.sql"),
    ),
    (
        "0129_atelier_visual_steer_retention.sql",
        include_bytes!("../../../migrations/0129_atelier_visual_steer_retention.sql"),
    ),
    (
        "0130_knowledge_schema_namespace.sql",
        include_bytes!("../../../migrations/0130_knowledge_schema_namespace.sql"),
    ),
    (
        "0131_knowledge_source_roots.sql",
        include_bytes!("../../../migrations/0131_knowledge_source_roots.sql"),
    ),
    (
        "0132_knowledge_sources.sql",
        include_bytes!("../../../migrations/0132_knowledge_sources.sql"),
    ),
    (
        "0133_knowledge_index_runs.sql",
        include_bytes!("../../../migrations/0133_knowledge_index_runs.sql"),
    ),
    (
        "0134_knowledge_spans.sql",
        include_bytes!("../../../migrations/0134_knowledge_spans.sql"),
    ),
    (
        "0135_knowledge_entities.sql",
        include_bytes!("../../../migrations/0135_knowledge_entities.sql"),
    ),
    (
        "0136_knowledge_edges.sql",
        include_bytes!("../../../migrations/0136_knowledge_edges.sql"),
    ),
    (
        "0137_knowledge_claims.sql",
        include_bytes!("../../../migrations/0137_knowledge_claims.sql"),
    ),
    (
        "0138_knowledge_memory_passages.sql",
        include_bytes!("../../../migrations/0138_knowledge_memory_passages.sql"),
    ),
    (
        "0139_knowledge_wiki_projections.sql",
        include_bytes!("../../../migrations/0139_knowledge_wiki_projections.sql"),
    ),
    (
        "0140_knowledge_rich_documents.sql",
        include_bytes!("../../../migrations/0140_knowledge_rich_documents.sql"),
    ),
    (
        "0141_knowledge_context_bundles.sql",
        include_bytes!("../../../migrations/0141_knowledge_context_bundles.sql"),
    ),
    (
        "0142_knowledge_idempotency_keys.sql",
        include_bytes!("../../../migrations/0142_knowledge_idempotency_keys.sql"),
    ),
    (
        "0150_knowledge_crdt_denial_receipts.sql",
        include_bytes!("../../../migrations/0150_knowledge_crdt_denial_receipts.sql"),
    ),
    (
        "0151_knowledge_crdt_agent_lane_leases.sql",
        include_bytes!("../../../migrations/0151_knowledge_crdt_agent_lane_leases.sql"),
    ),
    (
        "0152_knowledge_crdt_graph_proposals.sql",
        include_bytes!("../../../migrations/0152_knowledge_crdt_graph_proposals.sql"),
    ),
    (
        "0153_knowledge_crdt_promoted_facts.sql",
        include_bytes!("../../../migrations/0153_knowledge_crdt_promoted_facts.sql"),
    ),
    (
        "0154_knowledge_crdt_ai_edit_proposals.sql",
        include_bytes!("../../../migrations/0154_knowledge_crdt_ai_edit_proposals.sql"),
    ),
    (
        "0155_knowledge_crdt_swarm_checkpoints.sql",
        include_bytes!("../../../migrations/0155_knowledge_crdt_swarm_checkpoints.sql"),
    ),
    (
        "0160_knowledge_ingestion_policies.sql",
        include_bytes!("../../../migrations/0160_knowledge_ingestion_policies.sql"),
    ),
    (
        "0161_knowledge_ingestion_kind_registry.sql",
        include_bytes!("../../../migrations/0161_knowledge_ingestion_kind_registry.sql"),
    ),
    (
        "0162_knowledge_ingestion_receipts.sql",
        include_bytes!("../../../migrations/0162_knowledge_ingestion_receipts.sql"),
    ),
    (
        "0163_knowledge_ingestion_spans.sql",
        include_bytes!("../../../migrations/0163_knowledge_ingestion_spans.sql"),
    ),
    (
        "0164_knowledge_ingestion_repair_queue.sql",
        include_bytes!("../../../migrations/0164_knowledge_ingestion_repair_queue.sql"),
    ),
    (
        "0170_knowledge_code_files.sql",
        include_bytes!("../../../migrations/0170_knowledge_code_files.sql"),
    ),
    (
        "0171_knowledge_code_scip_imports.sql",
        include_bytes!("../../../migrations/0171_knowledge_code_scip_imports.sql"),
    ),
    (
        "0230_knowledge_code_repair_queue.sql",
        include_bytes!("../../../migrations/0230_knowledge_code_repair_queue.sql"),
    ),
    (
        "0240_knowledge_memory_ontology.sql",
        include_bytes!("../../../migrations/0240_knowledge_memory_ontology.sql"),
    ),
    (
        "0241_knowledge_memory_facts.sql",
        include_bytes!("../../../migrations/0241_knowledge_memory_facts.sql"),
    ),
    (
        "0242_knowledge_memory_conflict_jobs.sql",
        include_bytes!("../../../migrations/0242_knowledge_memory_conflict_jobs.sql"),
    ),
    (
        "0243_knowledge_memory_bridge_edges.sql",
        include_bytes!("../../../migrations/0243_knowledge_memory_bridge_edges.sql"),
    ),
    (
        "0260_knowledge_semantic_catalog.sql",
        include_bytes!("../../../migrations/0260_knowledge_semantic_catalog.sql"),
    ),
    (
        "0281_knowledge_document_embeds.sql",
        include_bytes!("../../../migrations/0281_knowledge_document_embeds.sql"),
    ),
    (
        "0282_knowledge_document_backlinks.sql",
        include_bytes!("../../../migrations/0282_knowledge_document_backlinks.sql"),
    ),
    (
        "0292_loom_block_knowledge_bridge.sql",
        include_bytes!("../../../migrations/0292_loom_block_knowledge_bridge.sql"),
    ),
    (
        "0294_loom_folders.sql",
        include_bytes!("../../../migrations/0294_loom_folders.sql"),
    ),
    (
        "0295_loom_wiki_overlays.sql",
        include_bytes!("../../../migrations/0295_loom_wiki_overlays.sql"),
    ),
    (
        "0310_user_manual.sql",
        include_bytes!("../../../migrations/0310_user_manual.sql"),
    ),
    (
        "0311_parallel_swarm_state_recovery.sql",
        include_bytes!("../../../migrations/0311_parallel_swarm_state_recovery.sql"),
    ),
    (
        "0313_parallel_swarm_quiet_background_work.sql",
        include_bytes!("../../../migrations/0313_parallel_swarm_quiet_background_work.sql"),
    ),
    (
        "0314_parallel_swarm_cloud_assistance_receipts.sql",
        include_bytes!("../../../migrations/0314_parallel_swarm_cloud_assistance_receipts.sql"),
    ),
    (
        "0322_quick_switcher_recents.sql",
        include_bytes!("../../../migrations/0322_quick_switcher_recents.sql"),
    ),
    (
        "0323_workbench_layout_state.sql",
        include_bytes!("../../../migrations/0323_workbench_layout_state.sql"),
    ),
    (
        "0327_workspace_settings_state.sql",
        include_bytes!("../../../migrations/0327_workspace_settings_state.sql"),
    ),
    (
        "0328_rich_document_draft_recovery.sql",
        include_bytes!("../../../migrations/0328_rich_document_draft_recovery.sql"),
    ),
    (
        "0330_workspace_search_bookmark_state.sql",
        include_bytes!("../../../migrations/0330_workspace_search_bookmark_state.sql"),
    ),
    (
        "0331_debug_breakpoints.sql",
        include_bytes!("../../../migrations/0331_debug_breakpoints.sql"),
    ),
    (
        "0332_media_asset_tiers.sql",
        include_bytes!("../../../migrations/0332_media_asset_tiers.sql"),
    ),
    (
        "0333_loom_ai_suggestions.sql",
        include_bytes!("../../../migrations/0333_loom_ai_suggestions.sql"),
    ),
    (
        "0334_loom_canvas_boards.sql",
        include_bytes!("../../../migrations/0334_loom_canvas_boards.sql"),
    ),
    (
        "0336_loom_search_v2.sql",
        include_bytes!("../../../migrations/0336_loom_search_v2.sql"),
    ),
    (
        "0340_calendar_activity_spans.sql",
        include_bytes!("source_wave/0340_calendar_activity_spans.sql"),
    ),
    (
        "0341_stage_capture_artifacts.sql",
        include_bytes!("source_wave/0341_stage_capture_artifacts.sql"),
    ),
    (
        "0343_knowledge_rich_document_loom_projection.sql",
        include_bytes!("source_wave/0343_knowledge_rich_document_loom_projection.sql"),
    ),
    (
        "0344_atelier_intake_item_loom_projection.sql",
        include_bytes!("source_wave/0344_atelier_intake_item_loom_projection.sql"),
    ),
    (
        "0345_fems_memory_workspace_authority.sql",
        include_bytes!("source_wave/0345_fems_memory_workspace_authority.sql"),
    ),
    (
        "0350_fems_memory_commit_authority.sql",
        include_bytes!("source_wave/0350_fems_memory_commit_authority.sql"),
    ),
    (
        "0351_fems_memory_commit_recovery.sql",
        include_bytes!("source_wave/0351_fems_memory_commit_recovery.sql"),
    ),
    (
        "0352_fems_memory_lifecycle_outbox.sql",
        include_bytes!("source_wave/0352_fems_memory_lifecycle_outbox.sql"),
    ),
    (
        "0353_calendar_lossless_temporal_contract.sql",
        include_bytes!("source_wave/0353_calendar_lossless_temporal_contract.sql"),
    ),
    (
        "0360_preference_records.sql",
        include_bytes!("source_wave/0360_preference_records.sql"),
    ),
    (
        "0361_loom_block_view_fr_outbox.sql",
        include_bytes!("source_wave/0361_loom_block_view_fr_outbox.sql"),
    ),
    (
        "0365_fems_proposal_request_id_canonical_identity.sql",
        include_bytes!("source_wave/0365_fems_proposal_request_id_canonical_identity.sql"),
    ),
];

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

impl SchemaState {
    fn has_current_lineage(&self, namespace: &str, database: &str) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == SCHEMA_REVISION
            && self.target_revision == SCHEMA_REVISION
            && self.namespace == namespace
            && self.database == database
            && self.source_manifest_sha256 == SOURCE_FORWARD_WAVE_MANIFEST_SHA256
            && self.generated_surql_sha256 == GENERATED_SURREALQL_SHA256
    }

    fn is_schema_applied_current(&self, namespace: &str, database: &str) -> bool {
        self.has_current_lineage(namespace, database)
            && self.apply_state == "schema_applied"
            && self.info_fingerprint_sha256 == PENDING_SCHEMA_INFO_SHA256
    }

    fn is_exact_current(&self, namespace: &str, database: &str) -> bool {
        self.has_current_lineage(namespace, database)
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == EXPECTED_SCHEMA_INFO_SHA256
    }
}

/// Receipt derived from the durable state row and live INFO introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaBootstrapReport {
    pub schema_version: String,
    pub namespace: String,
    pub database: String,
    pub source_migration_files: usize,
    pub source_manifest_sha256: String,
    pub generated_surql_sha256: String,
    pub info_fingerprint_sha256: String,
    pub tables_defined: usize,
    pub fields_defined: usize,
    pub indexes_defined: usize,
    pub table_names: Vec<String>,
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

/// Installs the fresh 0001-0029 Surreal schema wave or verifies an exact-current schema.
///
/// This is intentionally a transitional source-wave bridge, not a complete product-schema
/// migration system. V1 fails closed for every lower or divergent lineage. The sole resumable
/// incomplete state is the exact-current `schema_applied` receipt written after committed DDL;
/// it is finalized only after complete live INFO matches the compiled fingerprint. A per
/// namespace/database mutex serializes callers against the same database; the DDL transaction repeats the fresh-state guard before mutation.
/// Exact-current restarts return before executing any `OVERWRITE` statement.
pub async fn bootstrap_schema(
    storage: &SurrealStorage,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let bootstrap_guard_handle = bootstrap_guard_for(storage.namespace(), storage.database()).await;
    let _bootstrap_guard = bootstrap_guard_handle.lock().await;
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                verify_compiled_manifest(&database).await?;
                let existing = read_context_and_state(&database).await?;
                let reused_existing_schema = match existing {
                    None => {
                        database
                            .query_bound(
                                SCHEMA,
                                BootstrapBindings {
                                    schema_version: SCHEMA_VERSION.to_owned(),
                                    schema_revision: SCHEMA_REVISION,
                                    namespace: database.namespace().to_owned(),
                                    database: database.database().to_owned(),
                                    source_manifest_sha256:
                                        SOURCE_FORWARD_WAVE_MANIFEST_SHA256.to_owned(),
                                    generated_surql_sha256:
                                        GENERATED_SURREALQL_SHA256.to_owned(),
                                },
                            )
                            .await?;
                        let applied_state = match read_context_and_state(&database).await? {
                            Some(state)
                            if state.is_schema_applied_current(
                                database.namespace(),
                                database.database(),
                            ) => state,
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
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &applied_state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        false
                    }
                    Some(state)
                            if state.is_schema_applied_current(
                                database.namespace(),
                                database.database(),
                            ) => {
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        true
                    }
                    Some(state) if state.is_exact_current(database.namespace(), database.database()) => true,
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
                    Some(state) if state.is_exact_current(database.namespace(), database.database()) => state,
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

                observe_schema(&database, state, reused_existing_schema).await
            })
        })
        .await
}

pub fn compute_source_wave_manifest_sha256() -> String {
    compute_manifest_hash(&SOURCE_WAVE_FILES)
}

pub fn compute_generated_surql_sha256() -> String {
    sha256_hex(SCHEMA.as_bytes())
}

fn compute_manifest_hash(files: &[(&str, &[u8])]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_MANIFEST_DOMAIN);
    for (name, content) in files {
        hasher.update((name.len() as u32).to_be_bytes());
        hasher.update(name.as_bytes());
        hasher.update((content.len() as u64).to_be_bytes());
        hasher.update(content);
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
    let source = compute_source_wave_manifest_sha256();
    let generated = compute_generated_surql_sha256();
    if source != SOURCE_FORWARD_WAVE_MANIFEST_SHA256 || generated != GENERATED_SURREALQL_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_COMPILED_MANIFEST_DRIFT: source={source}; generated={generated}"
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
    if namespace != database.namespace() || selected_database != database.database() {
        return Err(SurrealStorageError::ContextMismatch {
            expected_namespace: database.namespace().to_owned(),
            expected_database: database.database().to_owned(),
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

async fn finalize_schema_state(
    database: &SurrealAdminContext<'_>,
    applied_state: &SchemaState,
    info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    if !applied_state.is_schema_applied_current(database.namespace(), database.database())
        || info_fingerprint_sha256.len() != 64
    {
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
                namespace: database.namespace().to_owned(),
                database: database.database().to_owned(),
                source_manifest_sha256: SOURCE_FORWARD_WAVE_MANIFEST_SHA256.to_owned(),
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
    reused_existing_schema: bool,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let observed = inspect_schema(database).await?;
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
        source_migration_files: SOURCE_WAVE_FILES.len(),
        source_manifest_sha256: state.source_manifest_sha256,
        generated_surql_sha256: state.generated_surql_sha256,
        info_fingerprint_sha256: state.info_fingerprint_sha256,
        tables_defined: observed.tables_defined,
        fields_defined: observed.fields_defined,
        indexes_defined: observed.indexes_defined,
        table_names: observed.table_names,
        reused_existing_schema,
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
    for table in &table_names {
        let mut table_response = database
            .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
            .await?;
        let table_info: SurrealValueData = table_response.take(0)?;
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
            info_entry_name(entry).map(str::to_owned).ok_or_else(|| {
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

/// Provisions the production schema for the focused Loom mutation-receipt
/// tests. The compiled `schema.surql` bootstrap is the only DDL source, so the
/// tables those tests exercise are exactly the production definitions.
#[cfg(test)]
pub async fn bootstrap_loom_receipt_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    bootstrap_schema(storage).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{DEFAULT_DATABASE, DEFAULT_NAMESPACE};
    use crate::storage::{
        surreal::{SurrealStorage, SurrealStorageConfig},
        EntityRef, JobMetrics, OperationType, PlannedOperation,
    };

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
    fn source_manifest_is_order_and_content_sensitive() {
        assert_eq!(SOURCE_WAVE_FILES.len(), SCHEMA_REVISION as usize);
        assert_eq!(
            compute_source_wave_manifest_sha256(),
            SOURCE_FORWARD_WAVE_MANIFEST_SHA256
        );
        assert_eq!(compute_generated_surql_sha256(), GENERATED_SURREALQL_SHA256);

        let mut reordered = SOURCE_WAVE_FILES.to_vec();
        reordered.swap(0, 1);
        assert_ne!(
            compute_manifest_hash(&reordered),
            SOURCE_FORWARD_WAVE_MANIFEST_SHA256
        );

        let altered = [("0001_init.sql", b"changed".as_slice())];
        assert_ne!(
            compute_manifest_hash(&altered),
            SOURCE_FORWARD_WAVE_MANIFEST_SHA256
        );
        assert_ne!(
            sha256_hex(format!("{SCHEMA}\n").as_bytes()),
            GENERATED_SURREALQL_SHA256
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
        );
        for definition in type_any_definitions {
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
        // No PostgreSQL `jsonb` type token may survive the projection. Checked as a
        // whole identifier token, not a substring: source column NAMES such as
        // `attribution_jsonb` (migration 0311) are transcribed verbatim and are not
        // PostgreSQL type syntax.
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
            assert_eq!(
                report.source_manifest_sha256,
                SOURCE_FORWARD_WAVE_MANIFEST_SHA256
            );
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
                                namespace: database.namespace().to_owned(),
                                database: database.database().to_owned(),
                                source_manifest_sha256: SOURCE_FORWARD_WAVE_MANIFEST_SHA256
                                    .to_owned(),
                                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                            },
                        )
                        .await?;
                    let pending = read_context_and_state(&database)
                        .await?
                        .expect("schema transaction must write pending state");
                    assert!(pending.is_schema_applied_current(database.namespace(), database.database()));
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
                    assert!(finalized.is_exact_current(database.namespace(), database.database()));
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
                               source_manifest_sha256: '{SOURCE_FORWARD_WAVE_MANIFEST_SHA256}', \
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
