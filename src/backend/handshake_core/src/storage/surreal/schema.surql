-- Handshake embedded SurrealDB schema.
--
-- This transaction is the bounded Surreal-native projection of the source
-- wave enumerated by `SOURCE_WAVE_FILES` in schema.rs (migrations 0001-0129
-- plus the selected 0130-0365 bands). Every table created by a forward
-- migration in that enumeration is defined here; the source enumeration is the
-- only authority for which migrations are in the wave.
-- The Rust bootstrap returns before this file is executed when
-- the durable state and complete structured INFO fingerprint are exact-current.
-- Two source constructs have no SurrealDB counterpart and are projected, not
-- copied. (1) A PostgreSQL PARTIAL index (`CREATE INDEX ... WHERE <pred>`)
-- becomes a plain index whose leading columns include the predicate columns, so
-- the same lookups stay indexed; where the source partial index was UNIQUE, the
-- projection is NOT unique (a partial uniqueness rule cannot be expressed as a
-- whole-table unique index without forbidding rows the source allows) and the
-- index is renamed from `uq_` to `idx_` so the name does not claim a constraint
-- the index no longer enforces. Those uniqueness rules are enforced in the
-- storage-layer write transaction. (2) PostgreSQL trigger functions and CHECKs
-- that read other rows are cross-row invariants; they stay in the storage layer
-- and the affected tables carry the record links plus the single-row field
-- contract here.
-- V1 is deliberately fresh-only: the guard below rejects every pre-existing
-- database object (including implicit tables created by records) in the same
-- transaction that owns all schema mutation. The durable sequence is
-- intentionally `IF NOT EXISTS`; overwriting it could reuse ledger values.

BEGIN TRANSACTION;

LET $existing_database = INFO FOR DB STRUCTURE;
IF array::len($existing_database.accesses) != 0
    OR array::len($existing_database.analyzers) != 0
    OR array::len($existing_database.apis) != 0
    OR array::len($existing_database.buckets) != 0
    OR array::len($existing_database.configs) != 0
    OR array::len($existing_database.functions) != 0
    OR array::len($existing_database.models) != 0
    OR array::len($existing_database.modules) != 0
    OR array::len($existing_database.params) != 0
    OR array::len($existing_database.sequences) != 0
    OR array::len($existing_database.tables) != 0
    OR array::len($existing_database.users) != 0
{
    THROW {
        code: 'HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY',
        observed: $existing_database,
    };
};

DEFINE TABLE OVERWRITE handshake_schema_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE version ON TABLE handshake_schema_state TYPE string;
DEFINE FIELD OVERWRITE revision ON TABLE handshake_schema_state TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE namespace ON TABLE handshake_schema_state TYPE string;
DEFINE FIELD OVERWRITE database ON TABLE handshake_schema_state TYPE string;
DEFINE FIELD OVERWRITE source_manifest_sha256 ON TABLE handshake_schema_state TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE generated_surql_sha256 ON TABLE handshake_schema_state TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE info_fingerprint_sha256 ON TABLE handshake_schema_state TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE apply_state ON TABLE handshake_schema_state TYPE 'schema_applied' | 'complete';
DEFINE FIELD OVERWRITE target_revision ON TABLE handshake_schema_state TYPE int ASSERT $value = $this.revision;
DEFINE FIELD OVERWRITE started_at ON TABLE handshake_schema_state TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE completed_at ON TABLE handshake_schema_state TYPE datetime ASSERT $value >= $this.started_at;
DEFINE FIELD OVERWRITE updated_at ON TABLE handshake_schema_state TYPE datetime VALUE time::now();
DEFINE INDEX OVERWRITE idx_handshake_schema_state_version ON TABLE handshake_schema_state FIELDS version UNIQUE;

-- 0001_init.sql
DEFINE TABLE OVERWRITE workspaces SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE name ON TABLE workspaces TYPE string;
DEFINE FIELD OVERWRITE last_job_id ON TABLE workspaces TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE workspaces TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE workspaces TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE workspaces TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE workspaces TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE workspaces TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE workspaces TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE workspaces
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'workspaces.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE documents SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE documents TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title ON TABLE documents TYPE string;
DEFINE FIELD OVERWRITE last_job_id ON TABLE documents TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE documents TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE documents TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE documents TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE documents TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE documents TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE documents TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE documents
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'documents.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE blocks SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE document_id ON TABLE blocks TYPE record<documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE blocks TYPE string;
DEFINE FIELD OVERWRITE sequence ON TABLE blocks TYPE int;
DEFINE FIELD OVERWRITE raw_content ON TABLE blocks TYPE string;
DEFINE FIELD OVERWRITE display_content ON TABLE blocks TYPE string;
DEFINE FIELD OVERWRITE derived_content ON TABLE blocks TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE derived_content.* ON TABLE blocks TYPE any;
DEFINE FIELD OVERWRITE last_job_id ON TABLE blocks TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE blocks TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE blocks TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE blocks TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE blocks TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE sensitivity ON TABLE blocks TYPE option<string>;
DEFINE FIELD OVERWRITE exportable ON TABLE blocks TYPE option<bool> DEFAULT true;
DEFINE FIELD OVERWRITE created_at ON TABLE blocks TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE blocks TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE blocks
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'blocks.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE canvases SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE canvases TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title ON TABLE canvases TYPE string;
DEFINE FIELD OVERWRITE last_job_id ON TABLE canvases TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE canvases TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE canvases TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE canvases TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE canvases TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE canvases TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE canvases TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE canvases
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'canvases.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE canvas_nodes SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE canvas_id ON TABLE canvas_nodes TYPE record<canvases> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE canvas_nodes TYPE string;
DEFINE FIELD OVERWRITE position_x ON TABLE canvas_nodes TYPE float;
DEFINE FIELD OVERWRITE position_y ON TABLE canvas_nodes TYPE float;
DEFINE FIELD OVERWRITE data ON TABLE canvas_nodes TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE data.* ON TABLE canvas_nodes TYPE any;
DEFINE FIELD OVERWRITE last_job_id ON TABLE canvas_nodes TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE canvas_nodes TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE canvas_nodes TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE canvas_nodes TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE canvas_nodes TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE canvas_nodes TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE canvas_nodes TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE canvas_nodes
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'canvas_nodes.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE canvas_edges SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE canvas_id ON TABLE canvas_edges TYPE record<canvases> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE from_node_id ON TABLE canvas_edges TYPE record<canvas_nodes> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE to_node_id ON TABLE canvas_edges TYPE record<canvas_nodes> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE canvas_edges TYPE string;
DEFINE FIELD OVERWRITE last_job_id ON TABLE canvas_edges TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE canvas_edges TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE canvas_edges TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE canvas_edges TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE canvas_edges TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE canvas_edges TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE canvas_edges TYPE datetime DEFAULT time::now();
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE canvas_edges
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'canvas_edges.last_job_id is required when last_actor_kind is AI'; };

-- 0002-0011 AI jobs, workflow persistence, and historical normalization.
DEFINE TABLE OVERWRITE ai_jobs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE trace_id ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE ai_jobs TYPE option<string>;
DEFINE FIELD OVERWRITE job_kind ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE status_reason ON TABLE ai_jobs TYPE string DEFAULT 'queued';
DEFINE FIELD OVERWRITE error_message ON TABLE ai_jobs TYPE option<string>;
DEFINE FIELD OVERWRITE protocol_id ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE profile_id ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE capability_profile_id ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE access_mode ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE safety_mode ON TABLE ai_jobs TYPE string;
DEFINE FIELD OVERWRITE entity_refs ON TABLE ai_jobs TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE entity_refs.*.* ON TABLE ai_jobs TYPE any;
DEFINE FIELD OVERWRITE planned_operations ON TABLE ai_jobs TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE planned_operations.*.* ON TABLE ai_jobs TYPE any;
DEFINE FIELD OVERWRITE metrics ON TABLE ai_jobs TYPE object FLEXIBLE DEFAULT { duration_ms: 0, total_tokens: 0, input_tokens: 0, output_tokens: 0, tokens_planner: 0, tokens_executor: 0, entities_read: 0, entities_written: 0, validators_run_count: 0 };
DEFINE FIELD OVERWRITE metrics.* ON TABLE ai_jobs TYPE any;
DEFINE FIELD OVERWRITE job_inputs ON TABLE ai_jobs TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE job_inputs.* ON TABLE ai_jobs TYPE any;
DEFINE FIELD OVERWRITE job_outputs ON TABLE ai_jobs TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE job_outputs.* ON TABLE ai_jobs TYPE any;
DEFINE FIELD OVERWRITE is_pinned ON TABLE ai_jobs TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE created_at ON TABLE ai_jobs TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE ai_jobs TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE idx_ai_jobs_gc ON TABLE ai_jobs FIELDS status, created_at, is_pinned;

DEFINE TABLE OVERWRITE workflow_runs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE workflow_runs TYPE record<ai_jobs> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE status ON TABLE workflow_runs TYPE string;
DEFINE FIELD OVERWRITE last_heartbeat ON TABLE workflow_runs TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at ON TABLE workflow_runs TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE workflow_runs TYPE datetime DEFAULT time::now();

DEFINE TABLE OVERWRITE workflow_node_executions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE workflow_node_executions TYPE record<workflow_runs> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE node_id ON TABLE workflow_node_executions TYPE string;
DEFINE FIELD OVERWRITE node_type ON TABLE workflow_node_executions TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE workflow_node_executions TYPE string;
DEFINE FIELD OVERWRITE sequence ON TABLE workflow_node_executions TYPE int;
DEFINE FIELD OVERWRITE input_payload ON TABLE workflow_node_executions TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE input_payload.* ON TABLE workflow_node_executions TYPE any;
DEFINE FIELD OVERWRITE output_payload ON TABLE workflow_node_executions TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE output_payload.* ON TABLE workflow_node_executions TYPE any;
DEFINE FIELD OVERWRITE error_message ON TABLE workflow_node_executions TYPE option<string>;
DEFINE FIELD OVERWRITE started_at ON TABLE workflow_node_executions TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE finished_at ON TABLE workflow_node_executions TYPE option<datetime>;
DEFINE FIELD OVERWRITE created_at ON TABLE workflow_node_executions TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE workflow_node_executions TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE idx_wne_run_sequence ON TABLE workflow_node_executions FIELDS workflow_run_id, sequence;
DEFINE INDEX OVERWRITE idx_wne_run_node ON TABLE workflow_node_executions FIELDS workflow_run_id, node_id;
DEFINE INDEX OVERWRITE idx_wne_status ON TABLE workflow_node_executions FIELDS status;

UPDATE ai_jobs SET job_kind = 'terminal_exec' WHERE job_kind = 'term_exec';
UPDATE ai_jobs SET job_kind = 'doc_summarize' WHERE job_kind = 'doc_test';
UPDATE ai_jobs SET job_kind = 'workflow_run', protocol_id = 'hsk.governance_pack.export.v0'
    WHERE job_kind = 'governance_pack_export';
UPDATE ai_jobs SET job_kind = 'micro_task_execution', protocol_id = 'micro_task_executor_v1'
    WHERE job_kind = 'workflow_run' AND profile_id = 'micro_task_executor_v1';
UPDATE ai_jobs SET protocol_id = 'micro_task_executor_v1'
    WHERE profile_id = 'micro_task_executor_v1' AND protocol_id != 'micro_task_executor_v1';

-- 0012_ai_ready_data_arch.sql
DEFINE TABLE OVERWRITE ai_embedding_models SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE model_id ON TABLE ai_embedding_models TYPE string;
DEFINE FIELD OVERWRITE model_version ON TABLE ai_embedding_models TYPE string;
DEFINE FIELD OVERWRITE dimensions ON TABLE ai_embedding_models TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE max_input_tokens ON TABLE ai_embedding_models TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE content_types_json ON TABLE ai_embedding_models TYPE array<string>;
DEFINE FIELD OVERWRITE status ON TABLE ai_embedding_models TYPE 'active' | 'deprecated' | 'retired';
DEFINE FIELD OVERWRITE introduced_at ON TABLE ai_embedding_models TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE compatible_with_json ON TABLE ai_embedding_models TYPE array<string>;
DEFINE INDEX OVERWRITE pk_ai_embedding_models ON TABLE ai_embedding_models FIELDS model_id, model_version UNIQUE;

DEFINE TABLE OVERWRITE ai_embedding_registry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE current_default_model_id ON TABLE ai_embedding_registry TYPE string;
DEFINE FIELD OVERWRITE current_default_model_version ON TABLE ai_embedding_registry TYPE string;
DEFINE FIELD OVERWRITE updated_at ON TABLE ai_embedding_registry TYPE datetime DEFAULT time::now();

DEFINE TABLE OVERWRITE ai_bronze_records SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE bronze_id ON TABLE ai_bronze_records TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE ai_bronze_records TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE content_hash ON TABLE ai_bronze_records TYPE string;
DEFINE FIELD OVERWRITE content_type ON TABLE ai_bronze_records TYPE string;
DEFINE FIELD OVERWRITE content_encoding ON TABLE ai_bronze_records TYPE string;
DEFINE FIELD OVERWRITE size_bytes ON TABLE ai_bronze_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE original_filename ON TABLE ai_bronze_records TYPE option<string>;
DEFINE FIELD OVERWRITE artifact_path ON TABLE ai_bronze_records TYPE string;
DEFINE FIELD OVERWRITE ingested_at ON TABLE ai_bronze_records TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE ingestion_source_type ON TABLE ai_bronze_records TYPE 'user' | 'connector' | 'system';
DEFINE FIELD OVERWRITE ingestion_source_id ON TABLE ai_bronze_records TYPE option<string>;
DEFINE FIELD OVERWRITE ingestion_method ON TABLE ai_bronze_records TYPE string;
DEFINE FIELD OVERWRITE external_source_json ON TABLE ai_bronze_records TYPE option<string>;
DEFINE FIELD OVERWRITE is_deleted ON TABLE ai_bronze_records TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE deleted_at ON TABLE ai_bronze_records TYPE option<datetime>;
DEFINE FIELD OVERWRITE retention_policy ON TABLE ai_bronze_records TYPE string DEFAULT 'default';
DEFINE INDEX OVERWRITE pk_ai_bronze_records ON TABLE ai_bronze_records FIELDS bronze_id UNIQUE;
DEFINE INDEX OVERWRITE idx_ai_bronze_workspace ON TABLE ai_bronze_records FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_ai_bronze_content_hash ON TABLE ai_bronze_records FIELDS content_hash;

DEFINE TABLE OVERWRITE ai_silver_records SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE silver_id ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE ai_silver_records TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE bronze_ref ON TABLE ai_silver_records TYPE record<ai_bronze_records> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE chunk_index ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE total_chunks ON TABLE ai_silver_records TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE token_count ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE content_hash ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE byte_start ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE byte_end ON TABLE ai_silver_records TYPE int ASSERT $value >= $this.byte_start;
DEFINE FIELD OVERWRITE line_start ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE line_end ON TABLE ai_silver_records TYPE int ASSERT $value >= $this.line_start;
DEFINE FIELD OVERWRITE chunk_artifact_path ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE embedding_artifact_path ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE embedding_model_id ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE embedding_model_version ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE embedding_dimensions ON TABLE ai_silver_records TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE embedding_compute_latency_ms ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE chunking_strategy ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE chunking_version ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE processing_pipeline_version ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE processed_at ON TABLE ai_silver_records TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE processing_duration_ms ON TABLE ai_silver_records TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE metadata_json ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE validation_status ON TABLE ai_silver_records TYPE 'passed' | 'failed' | 'warning' | 'pending';
DEFINE FIELD OVERWRITE validation_failed_checks_json ON TABLE ai_silver_records TYPE array<string>;
DEFINE FIELD OVERWRITE validated_at ON TABLE ai_silver_records TYPE datetime;
DEFINE FIELD OVERWRITE validator_version ON TABLE ai_silver_records TYPE string;
DEFINE FIELD OVERWRITE is_current ON TABLE ai_silver_records TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE superseded_by ON TABLE ai_silver_records TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE ai_silver_records TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_ai_silver_records ON TABLE ai_silver_records FIELDS silver_id UNIQUE;
DEFINE INDEX OVERWRITE idx_ai_silver_workspace ON TABLE ai_silver_records FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_ai_silver_bronze_ref ON TABLE ai_silver_records FIELDS bronze_ref;
DEFINE INDEX OVERWRITE idx_ai_silver_model ON TABLE ai_silver_records FIELDS embedding_model_id, embedding_model_version;

-- 0013_loom_mvp.sql
DEFINE TABLE OVERWRITE assets SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_id ON TABLE assets TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE assets TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE assets TYPE string;
DEFINE FIELD OVERWRITE mime ON TABLE assets TYPE string;
DEFINE FIELD OVERWRITE original_filename ON TABLE assets TYPE option<string>;
DEFINE FIELD OVERWRITE content_hash ON TABLE assets TYPE string;
DEFINE FIELD OVERWRITE size_bytes ON TABLE assets TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE width ON TABLE assets TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE height ON TABLE assets TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE last_job_id ON TABLE assets TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE assets TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE assets TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE assets TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE assets TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE assets TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE classification ON TABLE assets TYPE 'low' | 'medium' | 'high' | 'unknown' DEFAULT 'low';
DEFINE FIELD OVERWRITE exportable ON TABLE assets TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE is_proxy_of ON TABLE assets TYPE option<string>;
DEFINE FIELD OVERWRITE proxy_asset_id ON TABLE assets TYPE option<string>;
DEFINE INDEX OVERWRITE pk_assets ON TABLE assets FIELDS asset_id UNIQUE;
DEFINE INDEX OVERWRITE idx_assets_workspace ON TABLE assets FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_assets_workspace_content_hash ON TABLE assets FIELDS workspace_id, content_hash UNIQUE;
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE assets
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'assets.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_blocks TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_blocks TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
-- 0334_loom_canvas_boards.sql widens this allow-list with the canvas-board kind.
DEFINE FIELD OVERWRITE content_type ON TABLE loom_blocks TYPE 'note' | 'file' | 'annotated_file' | 'tag_hub' | 'journal' | 'canvas';
DEFINE FIELD OVERWRITE document_id ON TABLE loom_blocks TYPE option<record<documents>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE asset_id ON TABLE loom_blocks TYPE option<record<assets>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE title ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE original_filename ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE content_hash ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE pinned ON TABLE loom_blocks TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE journal_date ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE last_job_id ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE loom_blocks TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE loom_blocks TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE loom_blocks TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE loom_blocks TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_blocks TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE imported_at ON TABLE loom_blocks TYPE option<datetime>;
DEFINE FIELD OVERWRITE backlink_count ON TABLE loom_blocks TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE mention_count ON TABLE loom_blocks TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE tag_count ON TABLE loom_blocks TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE derived_json ON TABLE loom_blocks TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE derived_json.* ON TABLE loom_blocks TYPE any;
DEFINE FIELD OVERWRITE preview_status ON TABLE loom_blocks TYPE 'none' | 'pending' | 'generated' | 'failed' DEFAULT 'none';
DEFINE FIELD OVERWRITE thumbnail_asset_id ON TABLE loom_blocks TYPE option<record<assets>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE proxy_asset_id ON TABLE loom_blocks TYPE option<record<assets>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE INDEX OVERWRITE pk_loom_blocks ON TABLE loom_blocks FIELDS block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_blocks_workspace_updated_at ON TABLE loom_blocks FIELDS workspace_id, updated_at;
DEFINE INDEX OVERWRITE idx_loom_blocks_workspace_pinned ON TABLE loom_blocks FIELDS workspace_id, pinned;
DEFINE INDEX OVERWRITE idx_loom_blocks_workspace_content_hash ON TABLE loom_blocks FIELDS workspace_id, content_hash;
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE loom_blocks
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'loom_blocks.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE edge_id ON TABLE loom_edges TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_edges TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_block_id ON TABLE loom_edges TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE target_block_id ON TABLE loom_edges TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE edge_type ON TABLE loom_edges TYPE 'mention' | 'tag' | 'sub_tag' | 'parent' | 'ai_suggested';
DEFINE FIELD OVERWRITE created_by ON TABLE loom_edges TYPE 'user' | 'ai';
DEFINE FIELD OVERWRITE last_job_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE loom_edges TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE loom_edges TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE loom_edges TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE crdt_site_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE source_document_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE source_text_block_id ON TABLE loom_edges TYPE option<string>;
DEFINE FIELD OVERWRITE offset_start ON TABLE loom_edges TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE offset_end ON TABLE loom_edges TYPE option<int> ASSERT $value = NONE OR $value >= ($this.offset_start ?? 0);
DEFINE INDEX OVERWRITE pk_loom_edges ON TABLE loom_edges FIELDS edge_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_edges_workspace_source ON TABLE loom_edges FIELDS workspace_id, source_block_id;
DEFINE INDEX OVERWRITE idx_loom_edges_workspace_target ON TABLE loom_edges FIELDS workspace_id, target_block_id;
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE loom_edges
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'loom_edges.last_job_id is required when last_actor_kind is AI'; };

-- 0014_ai_job_mcp_fields.sql
DEFINE TABLE OVERWRITE ai_job_mcp_fields SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE ai_job_mcp_fields TYPE record<ai_jobs> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE mcp_server_id ON TABLE ai_job_mcp_fields TYPE option<string>;
DEFINE FIELD OVERWRITE mcp_call_id ON TABLE ai_job_mcp_fields TYPE option<string>;
DEFINE FIELD OVERWRITE mcp_progress_token ON TABLE ai_job_mcp_fields TYPE option<string>;
DEFINE INDEX OVERWRITE pk_ai_job_mcp_fields ON TABLE ai_job_mcp_fields FIELDS job_id UNIQUE;
DEFINE INDEX OVERWRITE idx_ai_job_mcp_fields_progress_token ON TABLE ai_job_mcp_fields FIELDS mcp_progress_token UNIQUE;

-- 0015_calendar_storage.sql
DEFINE TABLE OVERWRITE calendar_sources SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE calendar_sources TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE display_name ON TABLE calendar_sources TYPE string;
DEFINE FIELD OVERWRITE provider_type ON TABLE calendar_sources TYPE string;
DEFINE FIELD OVERWRITE write_policy ON TABLE calendar_sources TYPE string;
DEFINE FIELD OVERWRITE default_tzid ON TABLE calendar_sources TYPE string DEFAULT 'UTC';
DEFINE FIELD OVERWRITE auto_export ON TABLE calendar_sources TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE credentials_ref ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE provider_calendar_id ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE capability_profile_id ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE config_json ON TABLE calendar_sources TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE config_json.* ON TABLE calendar_sources TYPE any;
DEFINE FIELD OVERWRITE sync_state ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE sync_token ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE last_sync_ts ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_full_sync_ts ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_ok_at ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_pull_at ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_push_at ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_error_at ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE last_error_code ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE last_error ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE backoff_until ON TABLE calendar_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE consecutive_failures ON TABLE calendar_sources TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE last_remote_watermark ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE last_local_applied_rev ON TABLE calendar_sources TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE last_job_id ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE calendar_sources TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE calendar_sources TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE calendar_sources TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE calendar_sources TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE calendar_sources TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE idx_calendar_sources_workspace_provider ON TABLE calendar_sources FIELDS workspace_id, provider_type;
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE calendar_sources
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'calendar_sources.last_job_id is required when last_actor_kind is AI'; };

DEFINE TABLE OVERWRITE calendar_events SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE calendar_events TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE calendar_events TYPE record<calendar_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE external_id ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE external_etag ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE title ON TABLE calendar_events TYPE string;
DEFINE FIELD OVERWRITE description ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE location ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE start_ts_utc ON TABLE calendar_events TYPE datetime;
DEFINE FIELD OVERWRITE end_ts_utc ON TABLE calendar_events TYPE datetime ASSERT $value >= $this.start_ts_utc;
DEFINE FIELD OVERWRITE start_local ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE end_local ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE tzid ON TABLE calendar_events TYPE string DEFAULT 'UTC';
DEFINE FIELD OVERWRITE all_day ON TABLE calendar_events TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE was_floating ON TABLE calendar_events TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE status ON TABLE calendar_events TYPE string DEFAULT 'confirmed';
DEFINE FIELD OVERWRITE visibility ON TABLE calendar_events TYPE string DEFAULT 'private';
DEFINE FIELD OVERWRITE export_mode ON TABLE calendar_events TYPE string DEFAULT 'full_export';
DEFINE FIELD OVERWRITE rrule ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE rdate_json ON TABLE calendar_events TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE exdate_json ON TABLE calendar_events TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE is_recurring ON TABLE calendar_events TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE series_id ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE instance_key ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE is_override ON TABLE calendar_events TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE source_last_seen_at ON TABLE calendar_events TYPE option<datetime>;
DEFINE FIELD OVERWRITE created_by ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE attendees_json ON TABLE calendar_events TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE attendees_json.*.* ON TABLE calendar_events TYPE any;
DEFINE FIELD OVERWRITE links_json ON TABLE calendar_events TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE links_json.*.* ON TABLE calendar_events TYPE any;
DEFINE FIELD OVERWRITE provider_payload_json ON TABLE calendar_events TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE provider_payload_json.* ON TABLE calendar_events TYPE any;
DEFINE FIELD OVERWRITE last_job_id ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE last_workflow_id ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE last_actor_id ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE calendar_events TYPE string DEFAULT '00000000-0000-0000-0000-000000000000';
DEFINE FIELD OVERWRITE last_actor_kind ON TABLE calendar_events TYPE string DEFAULT 'SYSTEM';
DEFINE FIELD OVERWRITE created_at ON TABLE calendar_events TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE calendar_events TYPE datetime DEFAULT time::now();
-- 0353_calendar_lossless_temporal_contract.sql adds the date-only all-day
-- intent columns plus the explicit DST-normalization evidence. SurrealDB has no
-- date-only type, so `start_date` / `end_date_exclusive` keep the exact
-- date-only text the migration is written to preserve; coercing them to
-- datetime would be the silent normalization the migration exists to prevent.
DEFINE FIELD OVERWRITE start_date ON TABLE calendar_events TYPE option<string>;
DEFINE FIELD OVERWRITE end_date_exclusive ON TABLE calendar_events TYPE option<string> ASSERT $value = NONE OR $this.start_date = NONE OR $value > $this.start_date;
DEFINE FIELD OVERWRITE normalization_note ON TABLE calendar_events TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE normalization_note.* ON TABLE calendar_events TYPE any;
DEFINE FIELD OVERWRITE temporal_contract_version ON TABLE calendar_events TYPE option<string> DEFAULT 'calendar-v02.201' ASSERT $value = NONE OR $value = 'calendar-v02.201';
DEFINE INDEX OVERWRITE idx_calendar_events_workspace_window ON TABLE calendar_events FIELDS workspace_id, start_ts_utc, end_ts_utc;
DEFINE INDEX OVERWRITE idx_calendar_events_workspace_all_day_dates ON TABLE calendar_events FIELDS workspace_id, start_date, end_date_exclusive;
DEFINE INDEX OVERWRITE idx_calendar_events_source_external ON TABLE calendar_events FIELDS source_id, external_id UNIQUE;
DEFINE INDEX OVERWRITE idx_calendar_events_source_instance ON TABLE calendar_events FIELDS source_id, instance_key;
DEFINE EVENT OVERWRITE enforce_actor_job_consistency ON TABLE calendar_events
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.last_actor_kind = 'AI' AND $after.last_job_id = NONE
    THEN { THROW 'calendar_events.last_job_id is required when last_actor_kind is AI'; };

-- 0016_locus_structured_collaboration.sql
DEFINE TABLE OVERWRITE work_packets SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE wp_id ON TABLE work_packets TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE version ON TABLE work_packets TYPE int;
DEFINE FIELD OVERWRITE title ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE description ON TABLE work_packets TYPE option<string>;
DEFINE FIELD OVERWRITE status ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE priority ON TABLE work_packets TYPE int;
DEFINE FIELD OVERWRITE phase ON TABLE work_packets TYPE option<string>;
DEFINE FIELD OVERWRITE routing ON TABLE work_packets TYPE option<string>;
DEFINE FIELD OVERWRITE task_packet_path ON TABLE work_packets TYPE option<string>;
DEFINE FIELD OVERWRITE task_board_status ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE assignee ON TABLE work_packets TYPE option<string>;
DEFINE FIELD OVERWRITE reporter ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE updated_at ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE vector_clock ON TABLE work_packets TYPE string;
DEFINE FIELD OVERWRITE metadata ON TABLE work_packets TYPE string;
DEFINE INDEX OVERWRITE pk_work_packets ON TABLE work_packets FIELDS wp_id UNIQUE;
DEFINE INDEX OVERWRITE idx_wp_status ON TABLE work_packets FIELDS status;
DEFINE INDEX OVERWRITE idx_wp_priority ON TABLE work_packets FIELDS priority;
DEFINE INDEX OVERWRITE idx_wp_task_board_status ON TABLE work_packets FIELDS task_board_status;

DEFINE TABLE OVERWRITE micro_tasks SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE mt_id ON TABLE micro_tasks TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE micro_tasks TYPE record<work_packets> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE name ON TABLE micro_tasks TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE micro_tasks TYPE string;
DEFINE FIELD OVERWRITE current_iteration ON TABLE micro_tasks TYPE option<int>;
DEFINE FIELD OVERWRITE escalation_level ON TABLE micro_tasks TYPE option<int>;
DEFINE FIELD OVERWRITE metadata ON TABLE micro_tasks TYPE string;
DEFINE INDEX OVERWRITE pk_micro_tasks ON TABLE micro_tasks FIELDS mt_id UNIQUE;

-- 0017_skill_bank_distillation.sql
DEFINE TABLE OVERWRITE skill_log_entry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE version ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE session_id ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE session_turn_index ON TABLE skill_log_entry TYPE int;
DEFINE FIELD OVERWRITE task_id ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE user_id_hash ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE workspace_id ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE task_type ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE task_subtype ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE task_language ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE task_tags ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE task_request_summary ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE actor_role ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE model_name ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE model_family ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE model_revision ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE provider ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE tokenizer_id ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE tokenizer_family ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE context_window_tokens ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE precision ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE inference_params ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE context_refs_json ON TABLE skill_log_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE context_refs_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE snapshots_input_json ON TABLE skill_log_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE snapshots_input_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE snapshots_output_raw_json ON TABLE skill_log_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE snapshots_output_raw_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE snapshots_output_final_json ON TABLE skill_log_entry TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE snapshots_output_final_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE quality_tag ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE thumb ON TABLE skill_log_entry TYPE string;
DEFINE FIELD OVERWRITE quality_score ON TABLE skill_log_entry TYPE option<float>;
DEFINE FIELD OVERWRITE quality_source ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE quality_labels ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE auto_eval_json ON TABLE skill_log_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE auto_eval_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE user_edit_stats_json ON TABLE skill_log_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE user_edit_stats_json.* ON TABLE skill_log_entry TYPE any;
DEFINE FIELD OVERWRITE data_trust_score ON TABLE skill_log_entry TYPE option<float> ASSERT $value = NONE OR ($value >= 0 AND $value <= 1);
DEFINE FIELD OVERWRITE auto_style_score ON TABLE skill_log_entry TYPE option<float>;
DEFINE FIELD OVERWRITE auto_reasoning_score ON TABLE skill_log_entry TYPE option<float>;
DEFINE FIELD OVERWRITE auto_factuality_score ON TABLE skill_log_entry TYPE option<float>;
DEFINE FIELD OVERWRITE latency_ms ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE prompt_tokens ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE completion_tokens ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE total_tokens ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE truncation_occurred ON TABLE skill_log_entry TYPE option<bool>;
DEFINE FIELD OVERWRITE cache_hit ON TABLE skill_log_entry TYPE option<bool>;
DEFINE FIELD OVERWRITE output_char_len ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE output_line_count ON TABLE skill_log_entry TYPE option<int>;
DEFINE FIELD OVERWRITE handshake_version ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE orchestrator_build ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE git_commit ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE os ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE hardware_profile ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE config_profile ON TABLE skill_log_entry TYPE option<string>;
DEFINE FIELD OVERWRITE contains_secrets ON TABLE skill_log_entry TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE pii_present ON TABLE skill_log_entry TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE can_export_off_device ON TABLE skill_log_entry TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE redaction_applied ON TABLE skill_log_entry TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE reward_features_json ON TABLE skill_log_entry TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE reward_features_json.* ON TABLE skill_log_entry TYPE any;
DEFINE INDEX OVERWRITE idx_skill_log_entry_session ON TABLE skill_log_entry FIELDS session_id, session_turn_index;
DEFINE INDEX OVERWRITE idx_skill_log_entry_quality ON TABLE skill_log_entry FIELDS quality_tag, thumb;
DEFINE INDEX OVERWRITE idx_skill_log_entry_privacy ON TABLE skill_log_entry FIELDS contains_secrets, pii_present, can_export_off_device;
DEFINE INDEX OVERWRITE idx_skill_log_entry_created_at ON TABLE skill_log_entry FIELDS created_at;
DEFINE INDEX OVERWRITE idx_skill_log_entry_task_type ON TABLE skill_log_entry FIELDS task_type, task_language;

DEFINE TABLE OVERWRITE skill_log_file_ref SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE log_entry_id ON TABLE skill_log_file_ref TYPE record<skill_log_entry> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE path ON TABLE skill_log_file_ref TYPE string;
DEFINE FIELD OVERWRITE hash ON TABLE skill_log_file_ref TYPE option<string>;
DEFINE INDEX OVERWRITE idx_skill_log_file_ref_log ON TABLE skill_log_file_ref FIELDS log_entry_id;

DEFINE TABLE OVERWRITE distill_job SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE created_at ON TABLE distill_job TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE distill_job TYPE string;
DEFINE FIELD OVERWRITE description ON TABLE distill_job TYPE option<string>;
DEFINE FIELD OVERWRITE config_json ON TABLE distill_job TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE config_json.* ON TABLE distill_job TYPE any;
DEFINE INDEX OVERWRITE idx_distill_job_status ON TABLE distill_job FIELDS status, created_at;

DEFINE TABLE OVERWRITE distill_example SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE distill_example TYPE record<distill_job> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE log_entry_id ON TABLE distill_example TYPE record<skill_log_entry> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE role ON TABLE distill_example TYPE string;
DEFINE FIELD OVERWRITE is_replay ON TABLE distill_example TYPE bool;
DEFINE FIELD OVERWRITE sample_weight ON TABLE distill_example TYPE float;
DEFINE INDEX OVERWRITE pk_distill_example ON TABLE distill_example FIELDS job_id, log_entry_id, role UNIQUE;
DEFINE INDEX OVERWRITE idx_distill_example_job ON TABLE distill_example FIELDS job_id, is_replay;

DEFINE TABLE OVERWRITE adapter_checkpoint SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE created_at ON TABLE adapter_checkpoint TYPE string;
DEFINE FIELD OVERWRITE parent_checkpoint_id ON TABLE adapter_checkpoint TYPE option<record<adapter_checkpoint>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE base_model_name ON TABLE adapter_checkpoint TYPE string;
DEFINE FIELD OVERWRITE adapter_type ON TABLE adapter_checkpoint TYPE string;
DEFINE FIELD OVERWRITE rank_r ON TABLE adapter_checkpoint TYPE int;
DEFINE FIELD OVERWRITE alpha ON TABLE adapter_checkpoint TYPE int;
DEFINE FIELD OVERWRITE learning_rate ON TABLE adapter_checkpoint TYPE float;
DEFINE FIELD OVERWRITE precision ON TABLE adapter_checkpoint TYPE string;
DEFINE FIELD OVERWRITE path ON TABLE adapter_checkpoint TYPE string;
DEFINE FIELD OVERWRITE ewc_state_json ON TABLE adapter_checkpoint TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE ewc_state_json.* ON TABLE adapter_checkpoint TYPE any;
DEFINE FIELD OVERWRITE eval_summary_json ON TABLE adapter_checkpoint TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE eval_summary_json.* ON TABLE adapter_checkpoint TYPE any;
DEFINE FIELD OVERWRITE data_signature ON TABLE adapter_checkpoint TYPE option<string>;
DEFINE FIELD OVERWRITE job_ids_json ON TABLE adapter_checkpoint TYPE option<array<string>>;
DEFINE FIELD OVERWRITE is_approved ON TABLE adapter_checkpoint TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE is_current ON TABLE adapter_checkpoint TYPE bool DEFAULT false;
DEFINE INDEX OVERWRITE idx_adapter_checkpoint_current ON TABLE adapter_checkpoint FIELDS is_current;
DEFINE INDEX OVERWRITE idx_adapter_checkpoint_parent ON TABLE adapter_checkpoint FIELDS parent_checkpoint_id;

DEFINE TABLE OVERWRITE eval_run SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE eval_run TYPE option<record<distill_job>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE eval_run TYPE option<record<adapter_checkpoint>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE eval_run TYPE string;
DEFINE FIELD OVERWRITE suite_name ON TABLE eval_run TYPE string;
DEFINE FIELD OVERWRITE metrics_json ON TABLE eval_run TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE metrics_json.* ON TABLE eval_run TYPE any;

DEFINE TABLE OVERWRITE replay_candidates TYPE NORMAL AS
    SELECT * FROM skill_log_entry
    WHERE quality_tag = 'good' AND contains_secrets = false AND pii_present = false
    PERMISSIONS NONE;

-- 0018_kernel_event_ledger.sql
DEFINE SEQUENCE IF NOT EXISTS kernel_event_sequence BATCH 1 START 1;

DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE event_sequence ON TABLE kernel_event_ledger TYPE int DEFAULT sequence::nextval('kernel_event_sequence') ASSERT $value > 0;
DEFINE FIELD OVERWRITE event_version ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE kernel_task_run_id ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE session_run_id ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE aggregate_type ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE aggregate_id ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE event_type ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE actor_kind ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE causation_id ON TABLE kernel_event_ledger TYPE option<string>;
DEFINE FIELD OVERWRITE correlation_id ON TABLE kernel_event_ledger TYPE option<string>;
DEFINE FIELD OVERWRITE payload_hash ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE source_component ON TABLE kernel_event_ledger TYPE string;
DEFINE FIELD OVERWRITE payload ON TABLE kernel_event_ledger TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE payload.* ON TABLE kernel_event_ledger TYPE any;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE kernel_event_ledger TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE kernel_event_ledger TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE kernel_event_ledger TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE kernel_event_ledger TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE kernel_event_ledger TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE kernel_event_ledger TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_event_ledger ON TABLE kernel_event_ledger FIELDS event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_sequence ON TABLE kernel_event_ledger FIELDS event_sequence UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_idempotency ON TABLE kernel_event_ledger FIELDS idempotency_key UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_task ON TABLE kernel_event_ledger FIELDS kernel_task_run_id;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_session ON TABLE kernel_event_ledger FIELDS session_run_id;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_aggregate_replay ON TABLE kernel_event_ledger FIELDS aggregate_type, aggregate_id, event_sequence;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_event_type ON TABLE kernel_event_ledger FIELDS event_type;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_payload_hash ON TABLE kernel_event_ledger FIELDS payload_hash;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_correlation ON TABLE kernel_event_ledger FIELDS correlation_id;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_causation ON TABLE kernel_event_ledger FIELDS causation_id;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_created_at ON TABLE kernel_event_ledger FIELDS created_at;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_resource_scope ON TABLE kernel_event_ledger FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_sequence;

-- 0019_kernel_session_queue.sql
DEFINE TABLE OVERWRITE kernel_session_queue SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE session_run_id ON TABLE kernel_session_queue TYPE string;
DEFINE FIELD OVERWRITE kernel_task_run_id ON TABLE kernel_session_queue TYPE string;
DEFINE FIELD OVERWRITE adapter_id ON TABLE kernel_session_queue TYPE string;
DEFINE FIELD OVERWRITE state ON TABLE kernel_session_queue TYPE string;
DEFINE FIELD OVERWRITE claimed_by ON TABLE kernel_session_queue TYPE option<string>;
DEFINE FIELD OVERWRITE lease_expires_at ON TABLE kernel_session_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE attempt_count ON TABLE kernel_session_queue TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE available_at ON TABLE kernel_session_queue TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at ON TABLE kernel_session_queue TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE kernel_session_queue TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_session_queue ON TABLE kernel_session_queue FIELDS session_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_session_queue_task ON TABLE kernel_session_queue FIELDS kernel_task_run_id;
DEFINE INDEX OVERWRITE idx_kernel_session_queue_state_available ON TABLE kernel_session_queue FIELDS state, available_at;
DEFINE INDEX OVERWRITE idx_kernel_session_queue_lease ON TABLE kernel_session_queue FIELDS lease_expires_at;

-- 0020_kernel_crdt_storage.sql
DEFINE TABLE OVERWRITE kernel_crdt_updates SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE schema_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE kernel_crdt_updates TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE kernel_crdt_updates TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE kernel_crdt_updates TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE kernel_crdt_updates TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE document_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE update_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE update_seq ON TABLE kernel_crdt_updates TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE update_sha256 ON TABLE kernel_crdt_updates TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE update_bytes_ref ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE update_bytes ON TABLE kernel_crdt_updates TYPE bytes;
DEFINE FIELD OVERWRITE actor_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE actor_kind ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE session_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE trace_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE state_vector_before ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE state_vector_after ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE replay_metadata_json ON TABLE kernel_crdt_updates TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE replay_metadata_json.* ON TABLE kernel_crdt_updates TYPE any;
DEFINE FIELD OVERWRITE event_ledger_stream_id ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE kernel_crdt_updates TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE storage_authority ON TABLE kernel_crdt_updates TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE kernel_crdt_updates TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_crdt_updates ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_updates_seq ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_seq UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_updates_event ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_stream_id, event_ledger_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_updates_bytes_ref ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, update_bytes_ref UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_updates_replay ON TABLE kernel_crdt_updates FIELDS workspace_id, document_id, crdt_document_id, update_seq;
DEFINE INDEX OVERWRITE idx_kernel_crdt_updates_state_vector_after ON TABLE kernel_crdt_updates FIELDS state_vector_after;
DEFINE INDEX OVERWRITE uq_kernel_crdt_updates_scope_ref ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, update_bytes_ref UNIQUE;
DEFINE INDEX OVERWRITE uq_kernel_crdt_updates_scope_seq ON TABLE kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_seq UNIQUE;

DEFINE TABLE OVERWRITE kernel_crdt_snapshots SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE schema_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE snapshot_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE kernel_crdt_snapshots TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE kernel_crdt_snapshots TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE kernel_crdt_snapshots TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE kernel_crdt_snapshots TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE document_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE covered_update_seq ON TABLE kernel_crdt_snapshots TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE state_vector ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE snapshot_sha256 ON TABLE kernel_crdt_snapshots TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE snapshot_bytes_ref ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE snapshot_bytes ON TABLE kernel_crdt_snapshots TYPE bytes;
DEFINE FIELD OVERWRITE actor_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE actor_kind ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE event_ledger_stream_id ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE kernel_crdt_snapshots TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_evidence_update_ids ON TABLE kernel_crdt_snapshots TYPE array<string>;
DEFINE FIELD OVERWRITE storage_authority ON TABLE kernel_crdt_snapshots TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE kernel_crdt_snapshots TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_crdt_snapshots ON TABLE kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, snapshot_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_snapshots_event ON TABLE kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_stream_id, event_ledger_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_snapshots_bytes_ref ON TABLE kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, snapshot_bytes_ref UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_snapshots_latest ON TABLE kernel_crdt_snapshots FIELDS workspace_id, document_id, crdt_document_id, covered_update_seq;
DEFINE INDEX OVERWRITE idx_kernel_crdt_snapshots_state_vector ON TABLE kernel_crdt_snapshots FIELDS state_vector;
DEFINE INDEX OVERWRITE uq_kernel_crdt_snapshots_scope_ref ON TABLE kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, snapshot_bytes_ref UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_crdt_snapshots_scope_seq ON TABLE kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq;

-- 0021_kernel_process_lifecycle.sql
DEFINE TABLE OVERWRITE kernel_process_lifecycle SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE process_uuid ON TABLE kernel_process_lifecycle TYPE uuid;
DEFINE FIELD OVERWRITE process_id ON TABLE kernel_process_lifecycle TYPE uuid VALUE $this.process_uuid;
DEFINE FIELD OVERWRITE os_pid ON TABLE kernel_process_lifecycle TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE parent_session_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE parent_process_id ON TABLE kernel_process_lifecycle TYPE option<uuid>;
DEFINE FIELD OVERWRITE sandbox_adapter_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE adapter_id ON TABLE kernel_process_lifecycle TYPE option<string> VALUE $this.sandbox_adapter_id;
DEFINE FIELD OVERWRITE sandbox_internal_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE engine_kind ON TABLE kernel_process_lifecycle TYPE 'llamacpp' | 'llama_cpp' | 'candle' | 'abliteration_tool' | 'sandbox_container' | 'mechanical_job' | 'asr_worker' | 'comfyui_worker' | 'plugin_process' | 'helper_subprocess' | 'external_compat' | 'webview2_cdp' | 'official_cli_bridge';
DEFINE FIELD OVERWRITE started_at ON TABLE kernel_process_lifecycle TYPE datetime;
DEFINE FIELD OVERWRITE spawned_at_utc ON TABLE kernel_process_lifecycle TYPE datetime VALUE $this.started_at;
DEFINE FIELD OVERWRITE stopped_at ON TABLE kernel_process_lifecycle TYPE option<datetime>;
DEFINE FIELD OVERWRITE stopped_at_utc ON TABLE kernel_process_lifecycle TYPE option<datetime> VALUE $this.stopped_at;
DEFINE FIELD OVERWRITE exit_code ON TABLE kernel_process_lifecycle TYPE option<int>;
DEFINE FIELD OVERWRITE model_artifact_sha256 ON TABLE kernel_process_lifecycle TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE FIELD OVERWRITE work_profile_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE owner_role ON TABLE kernel_process_lifecycle TYPE string;
DEFINE FIELD OVERWRITE owner_wp ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE role_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE wp_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE mt_id ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE stop_reason ON TABLE kernel_process_lifecycle TYPE option<string>;
DEFINE FIELD OVERWRITE sandbox_capabilities_snapshot ON TABLE kernel_process_lifecycle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE sandbox_capabilities_snapshot.* ON TABLE kernel_process_lifecycle TYPE any;
DEFINE FIELD OVERWRITE metadata ON TABLE kernel_process_lifecycle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE metadata.* ON TABLE kernel_process_lifecycle TYPE any;
DEFINE INDEX OVERWRITE pk_kernel_process_lifecycle ON TABLE kernel_process_lifecycle FIELDS process_uuid UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_process_lifecycle_parent_session_started ON TABLE kernel_process_lifecycle FIELDS parent_session_id, started_at;
DEFINE INDEX OVERWRITE idx_kernel_process_lifecycle_engine_started ON TABLE kernel_process_lifecycle FIELDS engine_kind, started_at;
DEFINE INDEX OVERWRITE idx_kernel_process_lifecycle_os_pid ON TABLE kernel_process_lifecycle FIELDS os_pid;
DEFINE INDEX OVERWRITE idx_kernel_process_lifecycle_adapter_spawned ON TABLE kernel_process_lifecycle FIELDS adapter_id, spawned_at_utc;
DEFINE INDEX OVERWRITE idx_kernel_process_lifecycle_wp_spawned ON TABLE kernel_process_lifecycle FIELDS wp_id, spawned_at_utc;

-- 0022_role_mailbox_threads_messages.sql
DEFINE TABLE OVERWRITE role_mailbox_thread SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE thread_id ON TABLE role_mailbox_thread TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE title ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE linked_record_kind ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE linked_record_id ON TABLE role_mailbox_thread TYPE option<string>;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE executor_kind_allowlist ON TABLE role_mailbox_thread TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE claim_mode ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE lease_duration_secs ON TABLE role_mailbox_thread TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE takeover_policy ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE response_authority_scope ON TABLE role_mailbox_thread TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE role_mailbox_thread TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE role_mailbox_thread TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE role_mailbox_thread TYPE option<datetime>;
DEFINE FIELD OVERWRITE archived_at_utc ON TABLE role_mailbox_thread TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_role_mailbox_thread ON TABLE role_mailbox_thread FIELDS thread_id UNIQUE;
DEFINE INDEX OVERWRITE idx_role_mailbox_thread_state_expires ON TABLE role_mailbox_thread FIELDS lifecycle_state, expires_at_utc;
DEFINE INDEX OVERWRITE idx_role_mailbox_thread_linked ON TABLE role_mailbox_thread FIELDS linked_record_kind, linked_record_id;
DEFINE INDEX OVERWRITE idx_role_mailbox_thread_updated ON TABLE role_mailbox_thread FIELDS updated_at_utc;

DEFINE TABLE OVERWRITE role_mailbox_message SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE message_id ON TABLE role_mailbox_message TYPE uuid;
DEFINE FIELD OVERWRITE thread_id ON TABLE role_mailbox_message TYPE record<role_mailbox_thread> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE message_type ON TABLE role_mailbox_message TYPE string;
DEFINE FIELD OVERWRITE from_role ON TABLE role_mailbox_message TYPE string;
DEFINE FIELD OVERWRITE to_roles ON TABLE role_mailbox_message TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE expected_response ON TABLE role_mailbox_message TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE expected_response.* ON TABLE role_mailbox_message TYPE any;
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE role_mailbox_message TYPE option<datetime>;
DEFINE FIELD OVERWRITE delivery_state ON TABLE role_mailbox_message TYPE string;
DEFINE FIELD OVERWRITE body ON TABLE role_mailbox_message TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE body.* ON TABLE role_mailbox_message TYPE any;
DEFINE FIELD OVERWRITE parent_message_id ON TABLE role_mailbox_message TYPE option<uuid>;
DEFINE FIELD OVERWRITE audit_reason ON TABLE role_mailbox_message TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE role_mailbox_message TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_role_mailbox_message ON TABLE role_mailbox_message FIELDS message_id UNIQUE;
DEFINE INDEX OVERWRITE idx_role_mailbox_message_thread_created ON TABLE role_mailbox_message FIELDS thread_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_role_mailbox_message_delivery_state ON TABLE role_mailbox_message FIELDS delivery_state;

DEFINE TABLE OVERWRITE role_mailbox_claim_lease SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE lease_id ON TABLE role_mailbox_claim_lease TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE thread_id ON TABLE role_mailbox_claim_lease TYPE record<role_mailbox_thread> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE holder_executor_kind ON TABLE role_mailbox_claim_lease TYPE string;
DEFINE FIELD OVERWRITE holder_role_id ON TABLE role_mailbox_claim_lease TYPE string;
DEFINE FIELD OVERWRITE holder_session_id ON TABLE role_mailbox_claim_lease TYPE uuid;
DEFINE FIELD OVERWRITE acquired_at_utc ON TABLE role_mailbox_claim_lease TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE role_mailbox_claim_lease TYPE datetime;
DEFINE FIELD OVERWRITE released_at_utc ON TABLE role_mailbox_claim_lease TYPE option<datetime>;
DEFINE FIELD OVERWRITE takeover_of ON TABLE role_mailbox_claim_lease TYPE option<record<role_mailbox_claim_lease>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE takeover_reason ON TABLE role_mailbox_claim_lease TYPE option<string>;
-- SurrealDB has no partial UNIQUE B-tree index. This stored discriminator is
-- an `active:` discriminator for unreleased leases and a lease-specific
-- `released:` discriminator otherwise. This avoids relying on backend-specific
-- NULL/NONE handling in unique indexes.
DEFINE FIELD OVERWRITE active_thread_key ON TABLE role_mailbox_claim_lease TYPE string
    VALUE IF $this.released_at_utc = NONE {
        'active:' + <string>$this.thread_id
    } ELSE {
        'released:' + <string>$this.lease_id
    };
DEFINE INDEX OVERWRITE pk_role_mailbox_claim_lease ON TABLE role_mailbox_claim_lease FIELDS lease_id UNIQUE;
DEFINE INDEX OVERWRITE idx_role_mailbox_claim_lease_active ON TABLE role_mailbox_claim_lease FIELDS active_thread_key UNIQUE;
DEFINE INDEX OVERWRITE idx_role_mailbox_claim_lease_holder ON TABLE role_mailbox_claim_lease FIELDS holder_session_id;

DEFINE TABLE OVERWRITE role_mailbox_handoff_bundle SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE bundle_id ON TABLE role_mailbox_handoff_bundle TYPE uuid;
DEFINE FIELD OVERWRITE source_thread_id ON TABLE role_mailbox_handoff_bundle TYPE record<role_mailbox_thread> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_message_id ON TABLE role_mailbox_handoff_bundle TYPE uuid;
DEFINE FIELD OVERWRITE target_role ON TABLE role_mailbox_handoff_bundle TYPE string;
DEFINE FIELD OVERWRITE target_executor_kind ON TABLE role_mailbox_handoff_bundle TYPE string;
DEFINE FIELD OVERWRITE context_summary ON TABLE role_mailbox_handoff_bundle TYPE string;
DEFINE FIELD OVERWRITE linked_artifacts ON TABLE role_mailbox_handoff_bundle TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE linked_artifacts.*.* ON TABLE role_mailbox_handoff_bundle TYPE any;
DEFINE FIELD OVERWRITE transcript_pointer ON TABLE role_mailbox_handoff_bundle TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE transcript_pointer.* ON TABLE role_mailbox_handoff_bundle TYPE any;
DEFINE FIELD OVERWRITE capability_grants ON TABLE role_mailbox_handoff_bundle TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE capability_grants.*.* ON TABLE role_mailbox_handoff_bundle TYPE any;
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE role_mailbox_handoff_bundle TYPE option<datetime>;
DEFINE FIELD OVERWRITE content_hash ON TABLE role_mailbox_handoff_bundle TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE role_mailbox_handoff_bundle TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_by_session ON TABLE role_mailbox_handoff_bundle TYPE uuid;
DEFINE INDEX OVERWRITE pk_role_mailbox_handoff_bundle ON TABLE role_mailbox_handoff_bundle FIELDS bundle_id UNIQUE;
DEFINE INDEX OVERWRITE idx_role_mailbox_handoff_bundle_thread ON TABLE role_mailbox_handoff_bundle FIELDS source_thread_id;

-- 0023, 0026, and 0027 micro-task queue/outcome schema.
DEFINE TABLE OVERWRITE kernel_micro_task_job SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE kernel_micro_task_job TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE wp_id ON TABLE kernel_micro_task_job TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE kernel_micro_task_job TYPE string;
DEFINE FIELD OVERWRITE mt_contract_path ON TABLE kernel_micro_task_job TYPE string;
DEFINE FIELD OVERWRITE iteration_n ON TABLE kernel_micro_task_job TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE max_iterations ON TABLE kernel_micro_task_job TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE escalation_tier ON TABLE kernel_micro_task_job TYPE string;
DEFINE FIELD OVERWRITE escalation_history ON TABLE kernel_micro_task_job TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE escalation_history.*.* ON TABLE kernel_micro_task_job TYPE any;
DEFINE FIELD OVERWRITE task_tags ON TABLE kernel_micro_task_job TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE lora_id ON TABLE kernel_micro_task_job TYPE option<string>;
DEFINE FIELD OVERWRITE mailbox_thread_id ON TABLE kernel_micro_task_job TYPE option<uuid>;
DEFINE FIELD OVERWRITE state ON TABLE kernel_micro_task_job TYPE string;
DEFINE FIELD OVERWRITE claimed_by_session ON TABLE kernel_micro_task_job TYPE option<uuid>;
DEFINE FIELD OVERWRITE claimed_at_utc ON TABLE kernel_micro_task_job TYPE option<datetime>;
DEFINE FIELD OVERWRITE transition_reason ON TABLE kernel_micro_task_job TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_micro_task_job TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE kernel_micro_task_job TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE starvation_watermark_at_utc ON TABLE kernel_micro_task_job TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_kernel_micro_task_job ON TABLE kernel_micro_task_job FIELDS job_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_micro_task_job_state_tier_created ON TABLE kernel_micro_task_job FIELDS state, escalation_tier, created_at_utc;
DEFINE INDEX OVERWRITE idx_kernel_micro_task_job_wp_mt ON TABLE kernel_micro_task_job FIELDS wp_id, mt_id;
DEFINE INDEX OVERWRITE idx_kernel_micro_task_job_mailbox_thread ON TABLE kernel_micro_task_job FIELDS mailbox_thread_id;
DEFINE INDEX OVERWRITE idx_kernel_micro_task_job_wp_claimed_at ON TABLE kernel_micro_task_job FIELDS wp_id, claimed_at_utc;

DEFINE TABLE OVERWRITE kernel_mt_loop_checkpoint SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE kernel_mt_loop_checkpoint TYPE uuid;
DEFINE FIELD OVERWRITE job_id ON TABLE kernel_mt_loop_checkpoint TYPE record<kernel_micro_task_job> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE iteration_n ON TABLE kernel_mt_loop_checkpoint TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE state_at_checkpoint ON TABLE kernel_mt_loop_checkpoint TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE state_at_checkpoint.* ON TABLE kernel_mt_loop_checkpoint TYPE any;
DEFINE FIELD OVERWRITE retry_budget_remaining ON TABLE kernel_mt_loop_checkpoint TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE verifier_feedback_history ON TABLE kernel_mt_loop_checkpoint TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE verifier_feedback_history.*.* ON TABLE kernel_mt_loop_checkpoint TYPE any;
DEFINE FIELD OVERWRITE compact_summary ON TABLE kernel_mt_loop_checkpoint TYPE string ASSERT bytes::len(<bytes>$value) <= 2048;
DEFINE FIELD OVERWRITE evidence_pointers ON TABLE kernel_mt_loop_checkpoint TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE evidence_pointers.*.* ON TABLE kernel_mt_loop_checkpoint TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_mt_loop_checkpoint TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_by_session ON TABLE kernel_mt_loop_checkpoint TYPE uuid;
DEFINE INDEX OVERWRITE pk_kernel_mt_loop_checkpoint ON TABLE kernel_mt_loop_checkpoint FIELDS checkpoint_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_mt_loop_checkpoint_job ON TABLE kernel_mt_loop_checkpoint FIELDS job_id, created_at_utc;

DEFINE TABLE OVERWRITE kernel_mt_outcome SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE outcome_id ON TABLE kernel_mt_outcome TYPE uuid;
DEFINE FIELD OVERWRITE job_id ON TABLE kernel_mt_outcome TYPE record<kernel_micro_task_job> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE iteration_n ON TABLE kernel_mt_outcome TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE outcome_kind ON TABLE kernel_mt_outcome TYPE string;
DEFINE FIELD OVERWRITE completion_signal ON TABLE kernel_mt_outcome TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE completion_signal.* ON TABLE kernel_mt_outcome TYPE any;
DEFINE FIELD OVERWRITE run_ledger_ref ON TABLE kernel_mt_outcome TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE run_ledger_ref.* ON TABLE kernel_mt_outcome TYPE any;
DEFINE FIELD OVERWRITE evidence_pointers ON TABLE kernel_mt_outcome TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE distillation_candidate_id ON TABLE kernel_mt_outcome TYPE option<uuid>;
DEFINE FIELD OVERWRITE recorded_at_utc ON TABLE kernel_mt_outcome TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE recorded_by_session ON TABLE kernel_mt_outcome TYPE uuid;
DEFINE INDEX OVERWRITE pk_kernel_mt_outcome ON TABLE kernel_mt_outcome FIELDS outcome_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_mt_outcome_job ON TABLE kernel_mt_outcome FIELDS job_id, recorded_at_utc;
DEFINE INDEX OVERWRITE idx_kernel_mt_outcome_unique_job_iteration ON TABLE kernel_mt_outcome FIELDS job_id, iteration_n UNIQUE;

DEFINE TABLE OVERWRITE kernel_distillation_candidate SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE candidate_id ON TABLE kernel_distillation_candidate TYPE uuid;
DEFINE FIELD OVERWRITE job_id ON TABLE kernel_distillation_candidate TYPE record<kernel_micro_task_job> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE summary ON TABLE kernel_distillation_candidate TYPE string;
DEFINE FIELD OVERWRITE prompt_response_trace ON TABLE kernel_distillation_candidate TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE prompt_response_trace.*.* ON TABLE kernel_distillation_candidate TYPE any;
DEFINE FIELD OVERWRITE tier_at_completion ON TABLE kernel_distillation_candidate TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_distillation_candidate TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE status ON TABLE kernel_distillation_candidate TYPE string DEFAULT 'LOGGING_ONLY_PHASE_1';
DEFINE INDEX OVERWRITE pk_kernel_distillation_candidate ON TABLE kernel_distillation_candidate FIELDS candidate_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_distillation_candidate_job ON TABLE kernel_distillation_candidate FIELDS job_id;
DEFINE INDEX OVERWRITE idx_kernel_distillation_candidate_status_tier ON TABLE kernel_distillation_candidate FIELDS status, tier_at_completion;

-- 0024 and 0028 session checkpoint/restart schema.
DEFINE TABLE OVERWRITE kernel_session_checkpoint SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE kernel_session_checkpoint TYPE uuid;
DEFINE FIELD OVERWRITE session_id ON TABLE kernel_session_checkpoint TYPE uuid;
DEFINE FIELD OVERWRITE model_session_id ON TABLE kernel_session_checkpoint TYPE uuid;
DEFINE FIELD OVERWRITE last_event_ledger_seq ON TABLE kernel_session_checkpoint TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE compact_state ON TABLE kernel_session_checkpoint TYPE object FLEXIBLE ASSERT bytes::len(<bytes><string>$value) <= 32768;
DEFINE FIELD OVERWRITE compact_state.* ON TABLE kernel_session_checkpoint TYPE any;
DEFINE FIELD OVERWRITE state_kind ON TABLE kernel_session_checkpoint TYPE string;
DEFINE FIELD OVERWRITE pending_artifacts ON TABLE kernel_session_checkpoint TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_session_checkpoint TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_by_process ON TABLE kernel_session_checkpoint TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE schema_version ON TABLE kernel_session_checkpoint TYPE int DEFAULT 1 ASSERT $value > 0;
DEFINE INDEX OVERWRITE pk_kernel_session_checkpoint ON TABLE kernel_session_checkpoint FIELDS checkpoint_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_session_checkpoint_session ON TABLE kernel_session_checkpoint FIELDS session_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_kernel_session_checkpoint_model_session ON TABLE kernel_session_checkpoint FIELDS model_session_id;

DEFINE TABLE OVERWRITE kernel_restart_resume_report SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE report_id ON TABLE kernel_restart_resume_report TYPE uuid;
DEFINE FIELD OVERWRITE sessions_examined ON TABLE kernel_restart_resume_report TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE sessions_resumed ON TABLE kernel_restart_resume_report TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE sessions_resumed.*.* ON TABLE kernel_restart_resume_report TYPE any;
DEFINE FIELD OVERWRITE sessions_recovery_failed ON TABLE kernel_restart_resume_report TYPE array<array<string | object>> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE sessions_recovery_failed.*.*.* ON TABLE kernel_restart_resume_report TYPE any;
DEFINE FIELD OVERWRITE total_replay_events ON TABLE kernel_restart_resume_report TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE total_duration_ms ON TABLE kernel_restart_resume_report TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE started_at_utc ON TABLE kernel_restart_resume_report TYPE datetime;
DEFINE FIELD OVERWRITE completed_at_utc ON TABLE kernel_restart_resume_report TYPE datetime ASSERT $value >= $this.started_at_utc;
DEFINE FIELD OVERWRITE orphan_reclaims ON TABLE kernel_restart_resume_report TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE orphan_reclaims.*.* ON TABLE kernel_restart_resume_report TYPE any;
DEFINE FIELD OVERWRITE operator_decision_requests ON TABLE kernel_restart_resume_report TYPE array<object> FLEXIBLE DEFAULT [];
DEFINE FIELD OVERWRITE operator_decision_requests.*.* ON TABLE kernel_restart_resume_report TYPE any;
DEFINE FIELD OVERWRITE fr_events_emitted ON TABLE kernel_restart_resume_report TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE schema_version ON TABLE kernel_restart_resume_report TYPE int DEFAULT 2 ASSERT $value > 0;
DEFINE INDEX OVERWRITE pk_kernel_restart_resume_report ON TABLE kernel_restart_resume_report FIELDS report_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_restart_resume_report_started ON TABLE kernel_restart_resume_report FIELDS started_at_utc;

DEFINE TABLE OVERWRITE kernel_idempotency_ledger SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE session_id ON TABLE kernel_idempotency_ledger TYPE uuid;
DEFINE FIELD OVERWRITE event_seq ON TABLE kernel_idempotency_ledger TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE side_effect_kind ON TABLE kernel_idempotency_ledger TYPE string;
DEFINE FIELD OVERWRITE applied_at_utc ON TABLE kernel_idempotency_ledger TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_idempotency_ledger ON TABLE kernel_idempotency_ledger FIELDS session_id, event_seq, side_effect_kind UNIQUE;

-- 0024-0025 observability spans and trigger equivalents.
DEFINE TABLE OVERWRITE kernel_model_session_span SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE span_id ON TABLE kernel_model_session_span TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE model_session_id ON TABLE kernel_model_session_span TYPE uuid;
DEFINE FIELD OVERWRITE session_id ON TABLE kernel_model_session_span TYPE uuid;
DEFINE FIELD OVERWRITE started_at_utc ON TABLE kernel_model_session_span TYPE datetime;
DEFINE FIELD OVERWRITE ended_at_utc ON TABLE kernel_model_session_span TYPE option<datetime> ASSERT $value = NONE OR $value >= $this.started_at_utc;
DEFINE FIELD OVERWRITE status ON TABLE kernel_model_session_span TYPE 'active' | 'completed' | 'failed';
DEFINE FIELD OVERWRITE attributes ON TABLE kernel_model_session_span TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE attributes.* ON TABLE kernel_model_session_span TYPE any;
DEFINE FIELD OVERWRITE last_event_ledger_seq ON TABLE kernel_model_session_span TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE INDEX OVERWRITE pk_kernel_model_session_span ON TABLE kernel_model_session_span FIELDS span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_model_session_span_model_session ON TABLE kernel_model_session_span FIELDS model_session_id, started_at_utc;
DEFINE EVENT OVERWRITE enforce_session_span_immutability ON TABLE kernel_model_session_span
    WHEN $event = 'UPDATE'
      AND (
        $before.attributes != $after.attributes OR
        $before.started_at_utc != $after.started_at_utc OR
        $before.span_id != $after.span_id OR
        $before.model_session_id != $after.model_session_id OR
        $before.session_id != $after.session_id OR
        ($before.last_event_ledger_seq != NONE AND $after.last_event_ledger_seq != NONE
          AND $after.last_event_ledger_seq < $before.last_event_ledger_seq)
      )
    THEN { THROW 'kernel_model_session_span immutable identity/attributes or monotonic watermark violated'; };

DEFINE TABLE OVERWRITE kernel_activity_span SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE span_id ON TABLE kernel_activity_span TYPE uuid;
DEFINE FIELD OVERWRITE parent_span_id ON TABLE kernel_activity_span TYPE record<kernel_model_session_span> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE activity_kind ON TABLE kernel_activity_span TYPE string;
DEFINE FIELD OVERWRITE started_at_utc ON TABLE kernel_activity_span TYPE datetime;
DEFINE FIELD OVERWRITE ended_at_utc ON TABLE kernel_activity_span TYPE option<datetime> ASSERT $value = NONE OR $value >= $this.started_at_utc;
DEFINE FIELD OVERWRITE status ON TABLE kernel_activity_span TYPE 'active' | 'completed' | 'failed';
DEFINE FIELD OVERWRITE attributes ON TABLE kernel_activity_span TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE attributes.* ON TABLE kernel_activity_span TYPE any;
DEFINE FIELD OVERWRITE related_event_ledger_seqs ON TABLE kernel_activity_span TYPE array<int> DEFAULT [];
DEFINE INDEX OVERWRITE pk_kernel_activity_span ON TABLE kernel_activity_span FIELDS span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_activity_span_parent ON TABLE kernel_activity_span FIELDS parent_span_id, started_at_utc;
DEFINE INDEX OVERWRITE idx_kernel_activity_span_kind ON TABLE kernel_activity_span FIELDS activity_kind;
DEFINE EVENT OVERWRITE enforce_activity_span_immutability ON TABLE kernel_activity_span
    WHEN $event = 'UPDATE'
      AND (
        $before.attributes != $after.attributes OR
        $before.started_at_utc != $after.started_at_utc OR
        $before.span_id != $after.span_id OR
        $before.parent_span_id != $after.parent_span_id OR
        $before.activity_kind != $after.activity_kind
      )
    THEN { THROW 'kernel_activity_span immutable identity/attributes violated'; };

-- 0029 bitemporal EventLedger indexes. SurrealDB does not expose partial
-- expression indexes; leading with the exact discriminator fields provides an
-- equivalent selective composite index over native nested object paths.
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_memory_bitemporal_world ON TABLE kernel_event_ledger
    FIELDS aggregate_type, payload.item.stamps.valid_from, payload.item.stamps.valid_until, aggregate_id, event_sequence;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_memory_bitemporal_recorded ON TABLE kernel_event_ledger
    FIELDS aggregate_type, payload.item.stamps.recorded_at, payload.item.stamps.invalidated_at, aggregate_id, event_sequence;
DEFINE INDEX OVERWRITE idx_kernel_event_ledger_memory_bitemporal_manifest ON TABLE kernel_event_ledger
    FIELDS aggregate_type, aggregate_id, event_sequence, payload.item_id;


-- 0030-0059 Atelier schema projection. Historical PostgreSQL backfill DML is
-- intentionally omitted: this bootstrap targets an empty SurrealDB database. PostgreSQL
-- partial-index predicates and ASC/DESC decorations have no SurrealDB schema equivalent;
-- the same lookup columns and uniqueness semantics are retained.

DEFINE TABLE OVERWRITE atelier_character SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE internal_id ON TABLE atelier_character TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE public_id ON TABLE atelier_character TYPE string;
DEFINE FIELD OVERWRITE display_name ON TABLE atelier_character TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_character TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_character TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_character ON TABLE atelier_character FIELDS internal_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_character_1 ON TABLE atelier_character FIELDS public_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_sheet_version SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE version_id ON TABLE atelier_sheet_version TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_sheet_version TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE parent_version_id ON TABLE atelier_sheet_version TYPE option<record<atelier_sheet_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_sheet_version TYPE int;
DEFINE FIELD OVERWRITE raw_text ON TABLE atelier_sheet_version TYPE string;
DEFINE FIELD OVERWRITE author ON TABLE atelier_sheet_version TYPE string;
DEFINE FIELD OVERWRITE tool ON TABLE atelier_sheet_version TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_sheet_version TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_sheet_version ON TABLE atelier_sheet_version FIELDS version_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_sheet_version_1 ON TABLE atelier_sheet_version FIELDS character_internal_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_asset SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_asset TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_media_asset TYPE string;
DEFINE FIELD OVERWRITE mime ON TABLE atelier_media_asset TYPE string;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_media_asset TYPE int;
DEFINE FIELD OVERWRITE source_provenance ON TABLE atelier_media_asset TYPE option<string>;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_media_asset TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_asset TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE retention_class ON TABLE atelier_media_asset TYPE string DEFAULT 'atelier.media.original.retained';
DEFINE FIELD OVERWRITE artifact_manifest ON TABLE atelier_media_asset TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE artifact_manifest.* ON TABLE atelier_media_asset TYPE any;
DEFINE INDEX OVERWRITE pk_atelier_media_asset ON TABLE atelier_media_asset FIELDS asset_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_media_asset_1 ON TABLE atelier_media_asset FIELDS content_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_event SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE event_id ON TABLE atelier_event TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE event_family ON TABLE atelier_event TYPE string;
DEFINE FIELD OVERWRITE aggregate_type ON TABLE atelier_event TYPE string;
DEFINE FIELD OVERWRITE aggregate_id ON TABLE atelier_event TYPE string;
DEFINE FIELD OVERWRITE payload ON TABLE atelier_event TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE payload.* ON TABLE atelier_event TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_event TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE kernel_event_id ON TABLE atelier_event TYPE option<string>;
DEFINE FIELD OVERWRITE kernel_event_sequence ON TABLE atelier_event TYPE option<int>;
DEFINE INDEX OVERWRITE pk_atelier_event ON TABLE atelier_event FIELDS event_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_intake_batch SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE batch_id ON TABLE atelier_intake_batch TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE idempotency_key ON TABLE atelier_intake_batch TYPE string;
DEFINE FIELD OVERWRITE source_label ON TABLE atelier_intake_batch TYPE string;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_intake_batch TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE status ON TABLE atelier_intake_batch TYPE 'open' | 'in_progress' | 'closed' DEFAULT 'open';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_intake_batch TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_intake_batch TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_intake_batch TYPE string ASSERT (string::trim($value) != '');
DEFINE FIELD OVERWRITE mode ON TABLE atelier_intake_batch TYPE 'manual' | 'folder_scan' | 'sourcing_run' DEFAULT 'manual';
DEFINE FIELD OVERWRITE resume_cursor ON TABLE atelier_intake_batch TYPE option<string> ASSERT ($value = NONE OR string::trim($value) != '');
DEFINE FIELD OVERWRITE resumed_at_utc ON TABLE atelier_intake_batch TYPE option<datetime>;
DEFINE FIELD OVERWRITE profile_mode ON TABLE atelier_intake_batch TYPE 'loose_profile' | 'character_linked' DEFAULT 'loose_profile';
DEFINE FIELD OVERWRITE target_character_id ON TABLE atelier_intake_batch TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE target_sheet_version_id ON TABLE atelier_intake_batch TYPE option<record<atelier_sheet_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE target_collection_id ON TABLE atelier_intake_batch TYPE option<record<atelier_collection>> ASSERT ($value = NONE OR record::exists($value)) AND (($this.profile_mode = 'loose_profile' AND $this.target_character_id = NONE AND $this.target_sheet_version_id = NONE AND $value = NONE AND $this.character_internal_id = NONE) OR ($this.profile_mode = 'character_linked' AND $this.target_character_id != NONE)) REFERENCE ON DELETE UNSET;
DEFINE INDEX OVERWRITE pk_atelier_intake_batch ON TABLE atelier_intake_batch FIELDS batch_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_intake_batch_1 ON TABLE atelier_intake_batch FIELDS idempotency_key UNIQUE;

DEFINE TABLE OVERWRITE atelier_intake_item SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_intake_item TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE batch_id ON TABLE atelier_intake_item TYPE record<atelier_intake_batch> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_path ON TABLE atelier_intake_item TYPE string;
DEFINE FIELD OVERWRITE file_name ON TABLE atelier_intake_item TYPE string;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_intake_item TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_intake_item TYPE option<string>;
DEFINE FIELD OVERWRITE lane ON TABLE atelier_intake_item TYPE 'pending' | 'accepted' | 'rejected' | 'deferred' | 'skipped' | 'failed' DEFAULT 'pending';
DEFINE FIELD OVERWRITE lane_reason ON TABLE atelier_intake_item TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_intake_item TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_intake_item TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_intake_item ON TABLE atelier_intake_item FIELDS item_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_intake_item_1 ON TABLE atelier_intake_item FIELDS batch_id, source_path UNIQUE;

DEFINE TABLE OVERWRITE atelier_collection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE collection_id ON TABLE atelier_collection TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE name ON TABLE atelier_collection TYPE string;
DEFINE FIELD OVERWRITE notes ON TABLE atelier_collection TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_collection TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_collection TYPE any;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_collection TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE sheet_version_id ON TABLE atelier_collection TYPE option<record<atelier_sheet_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_collection TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_collection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_collection ON TABLE atelier_collection FIELDS collection_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_collection_1 ON TABLE atelier_collection FIELDS name UNIQUE;

DEFINE TABLE OVERWRITE atelier_collection_item SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE collection_id ON TABLE atelier_collection_item TYPE record<atelier_collection> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_collection_item TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE sort_order ON TABLE atelier_collection_item TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE added_at_utc ON TABLE atelier_collection_item TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_collection_item ON TABLE atelier_collection_item FIELDS collection_id, asset_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_contact_sheet SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE sheet_id ON TABLE atelier_contact_sheet TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE name ON TABLE atelier_contact_sheet TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE source_type ON TABLE atelier_contact_sheet TYPE string DEFAULT 'manual';
DEFINE FIELD OVERWRITE source_id ON TABLE atelier_contact_sheet TYPE option<string>;
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_contact_sheet TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_contact_sheet TYPE any;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_contact_sheet TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE sheet_version_id ON TABLE atelier_contact_sheet TYPE option<record<atelier_sheet_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE manifest ON TABLE atelier_contact_sheet TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE manifest.* ON TABLE atelier_contact_sheet TYPE any;
DEFINE FIELD OVERWRITE image_count ON TABLE atelier_contact_sheet TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_contact_sheet TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_contact_sheet ON TABLE atelier_contact_sheet FIELDS sheet_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_tag SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE tag_id ON TABLE atelier_tag TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE text ON TABLE atelier_tag TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_tag TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_tag ON TABLE atelier_tag FIELDS tag_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_tag_1 ON TABLE atelier_tag FIELDS text UNIQUE;

DEFINE TABLE OVERWRITE atelier_character_tag SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_character_tag TYPE record<atelier_character> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE tag_id ON TABLE atelier_character_tag TYPE record<atelier_tag> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE tag_type ON TABLE atelier_character_tag TYPE 'manual' | 'derived';
DEFINE INDEX OVERWRITE pk_atelier_character_tag ON TABLE atelier_character_tag FIELDS character_internal_id, tag_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_tag_rule SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rule_id ON TABLE atelier_tag_rule TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE source_field_id ON TABLE atelier_tag_rule TYPE string;
DEFINE FIELD OVERWRITE match_type ON TABLE atelier_tag_rule TYPE 'equals' | 'contains' | 'regex';
DEFINE FIELD OVERWRITE pattern ON TABLE atelier_tag_rule TYPE string;
DEFINE FIELD OVERWRITE emit_tag ON TABLE atelier_tag_rule TYPE string;
DEFINE FIELD OVERWRITE enabled ON TABLE atelier_tag_rule TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_tag_rule TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_tag_rule TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_tag_rule ON TABLE atelier_tag_rule FIELDS rule_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_similarity_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_internal_id ON TABLE atelier_similarity_projection TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE dhash_hex ON TABLE atelier_similarity_projection TYPE option<string>;
DEFINE FIELD OVERWRITE palette_json ON TABLE atelier_similarity_projection TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE palette_json.* ON TABLE atelier_similarity_projection TYPE any;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_similarity_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_similarity_projection ON TABLE atelier_similarity_projection FIELDS asset_internal_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_export_request SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE export_id ON TABLE atelier_export_request TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_export_request TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE sheet_version_id ON TABLE atelier_export_request TYPE record<atelier_sheet_version> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE format ON TABLE atelier_export_request TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE atelier_export_request TYPE string DEFAULT 'pending';
DEFINE FIELD OVERWRITE label ON TABLE atelier_export_request TYPE option<string>;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_export_request TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_export_request TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_export_request ON TABLE atelier_export_request FIELDS export_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_export_result SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE result_id ON TABLE atelier_export_result TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE export_id ON TABLE atelier_export_result TYPE record<atelier_export_request> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_export_result TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_export_result TYPE string;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_export_result TYPE int;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_export_result TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_export_result ON TABLE atelier_export_result FIELDS result_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_export_result_1 ON TABLE atelier_export_result FIELDS export_id, content_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_export_manifest_entry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entry_id ON TABLE atelier_export_manifest_entry TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE export_id ON TABLE atelier_export_manifest_entry TYPE record<atelier_export_request> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_export_manifest_entry TYPE int;
DEFINE FIELD OVERWRITE kind ON TABLE atelier_export_manifest_entry TYPE string;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_export_manifest_entry TYPE string;
DEFINE FIELD OVERWRITE pack_path ON TABLE atelier_export_manifest_entry TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_export_manifest_entry TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_export_manifest_entry ON TABLE atelier_export_manifest_entry FIELDS entry_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_export_manifest_entry_1 ON TABLE atelier_export_manifest_entry FIELDS export_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_annotation SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE annotation_id ON TABLE atelier_media_annotation TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_annotation TYPE record<atelier_media_asset> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE atelier_media_annotation TYPE string;
DEFINE FIELD OVERWRITE label ON TABLE atelier_media_annotation TYPE option<string>;
DEFINE FIELD OVERWRITE note ON TABLE atelier_media_annotation TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE geometry ON TABLE atelier_media_annotation TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE geometry.* ON TABLE atelier_media_annotation TYPE any;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_media_annotation TYPE int;
DEFINE FIELD OVERWRITE author ON TABLE atelier_media_annotation TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_annotation TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_media_annotation TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_annotation ON TABLE atelier_media_annotation FIELDS annotation_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_media_annotation_1 ON TABLE atelier_media_annotation FIELDS asset_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_preference SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE preference_id ON TABLE atelier_preference TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE scope_kind ON TABLE atelier_preference TYPE 'global' | 'character';
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_preference TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) AND (($this.scope_kind = 'global' AND $value = NONE) OR ($this.scope_kind = 'character' AND $value != NONE)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE key ON TABLE atelier_preference TYPE string;
DEFINE FIELD OVERWRITE value_type ON TABLE atelier_preference TYPE 'string' | 'bool' | 'integer' | 'float' | 'json' | 'path';
DEFINE FIELD OVERWRITE value ON TABLE atelier_preference TYPE string;
DEFINE FIELD OVERWRITE redacted ON TABLE atelier_preference TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_preference TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_preference TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE namespace ON TABLE atelier_preference TYPE string;
DEFINE FIELD OVERWRITE name ON TABLE atelier_preference TYPE string;
DEFINE FIELD OVERWRITE default_value ON TABLE atelier_preference TYPE option<string>;
DEFINE FIELD OVERWRITE source ON TABLE atelier_preference TYPE string DEFAULT 'operator';
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_preference TYPE option<string>;
DEFINE FIELD OVERWRITE revision ON TABLE atelier_preference TYPE int DEFAULT 1;
DEFINE FIELD OVERWRITE redaction_class ON TABLE atelier_preference TYPE string DEFAULT 'public';
DEFINE INDEX OVERWRITE pk_atelier_preference ON TABLE atelier_preference FIELDS preference_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_preference_1 ON TABLE atelier_preference FIELDS scope_kind, character_internal_id, key UNIQUE;

DEFINE TABLE OVERWRITE atelier_pose_rig SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rig_id ON TABLE atelier_pose_rig TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_pose_rig TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_asset_id ON TABLE atelier_pose_rig TYPE option<record<atelier_media_asset>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_pose_rig TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_pose_rig TYPE string;
DEFINE FIELD OVERWRITE canvas_width ON TABLE atelier_pose_rig TYPE int;
DEFINE FIELD OVERWRITE canvas_height ON TABLE atelier_pose_rig TYPE int;
DEFINE FIELD OVERWRITE detector_provider ON TABLE atelier_pose_rig TYPE string;
DEFINE FIELD OVERWRITE detector_status ON TABLE atelier_pose_rig TYPE 'detected' | 'fallback' | 'failed';
DEFINE FIELD OVERWRITE keypoints_json ON TABLE atelier_pose_rig TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE keypoints_json.* ON TABLE atelier_pose_rig TYPE any;
DEFINE FIELD OVERWRITE sidecar_ref ON TABLE atelier_pose_rig TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_rig TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_rig ON TABLE atelier_pose_rig FIELDS rig_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_pose_rig_1 ON TABLE atelier_pose_rig FIELDS character_internal_id, source_ref, content_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_pose_head_pose SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rig_id ON TABLE atelier_pose_head_pose TYPE record<atelier_pose_rig> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE yaw_deg ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE pitch_deg ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE roll_deg ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE quat_x ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE quat_y ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE quat_z ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE quat_w ON TABLE atelier_pose_head_pose TYPE float;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_head_pose TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_head_pose ON TABLE atelier_pose_head_pose FIELDS rig_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_pose_calibration SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rig_id ON TABLE atelier_pose_calibration TYPE record<atelier_pose_rig> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE state ON TABLE atelier_pose_calibration TYPE 'unresolved' | 'resolved';
DEFINE FIELD OVERWRITE block_reason ON TABLE atelier_pose_calibration TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_calibration TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_pose_calibration TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_calibration ON TABLE atelier_pose_calibration FIELDS rig_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_identity_profile SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE profile_id ON TABLE atelier_identity_profile TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_identity_profile TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_identity_profile TYPE int;
DEFINE FIELD OVERWRITE kind ON TABLE atelier_identity_profile TYPE 'face' | 'reference';
DEFINE FIELD OVERWRITE reference_asset_id ON TABLE atelier_identity_profile TYPE option<record<atelier_media_asset>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE reference_ref ON TABLE atelier_identity_profile TYPE string;
DEFINE FIELD OVERWRITE provenance ON TABLE atelier_identity_profile TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_identity_profile TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_identity_profile ON TABLE atelier_identity_profile FIELDS profile_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_identity_profile_1 ON TABLE atelier_identity_profile FIELDS character_internal_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_bridge_probe SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE probe_id ON TABLE atelier_comfy_bridge_probe TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_bridge_probe TYPE uuid;
DEFINE FIELD OVERWRITE node_class_id ON TABLE atelier_comfy_bridge_probe TYPE string;
DEFINE FIELD OVERWRITE detected ON TABLE atelier_comfy_bridge_probe TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE bridge_protocol_version ON TABLE atelier_comfy_bridge_probe TYPE option<string>;
DEFINE FIELD OVERWRITE node_instance_ids ON TABLE atelier_comfy_bridge_probe TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE node_instance_ids.* ON TABLE atelier_comfy_bridge_probe TYPE any;
DEFINE FIELD OVERWRITE probe_outcome ON TABLE atelier_comfy_bridge_probe TYPE 'bridge_present' | 'bridge_absent' | 'bridge_incompatible';
DEFINE FIELD OVERWRITE fallback_reason ON TABLE atelier_comfy_bridge_probe TYPE option<string> ASSERT ($this.probe_outcome = 'bridge_present' OR $value != NONE);
DEFINE FIELD OVERWRITE probed_at_utc ON TABLE atelier_comfy_bridge_probe TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_bridge_probe ON TABLE atelier_comfy_bridge_probe FIELDS probe_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_bridge_probe_1 ON TABLE atelier_comfy_bridge_probe FIELDS workflow_run_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_capability_registration SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE registration_id ON TABLE atelier_comfy_capability_registration TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_capability_registration TYPE uuid;
DEFINE FIELD OVERWRITE node_class_id ON TABLE atelier_comfy_capability_registration TYPE string;
DEFINE FIELD OVERWRITE bridge_protocol_version ON TABLE atelier_comfy_capability_registration TYPE string;
DEFINE FIELD OVERWRITE capability_grant_ref ON TABLE atelier_comfy_capability_registration TYPE string;
DEFINE FIELD OVERWRITE consent_decision_ref ON TABLE atelier_comfy_capability_registration TYPE option<string>;
DEFINE FIELD OVERWRITE registered_at_utc ON TABLE atelier_comfy_capability_registration TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_capability_registration ON TABLE atelier_comfy_capability_registration FIELDS registration_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_capability_registration_1 ON TABLE atelier_comfy_capability_registration FIELDS workflow_run_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_declared_output SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE declared_output_id ON TABLE atelier_comfy_declared_output TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE registration_id ON TABLE atelier_comfy_declared_output TYPE record<atelier_comfy_capability_registration> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_comfy_declared_output TYPE int;
DEFINE FIELD OVERWRITE output_slot ON TABLE atelier_comfy_declared_output TYPE string;
DEFINE FIELD OVERWRITE media_kind ON TABLE atelier_comfy_declared_output TYPE 'image' | 'mask' | 'latent_preview' | 'video' | 'sidecar_json';
DEFINE FIELD OVERWRITE expected_mime ON TABLE atelier_comfy_declared_output TYPE string;
DEFINE FIELD OVERWRITE routing_intent ON TABLE atelier_comfy_declared_output TYPE 'artifact' | 'sidecar' | 'transient';
DEFINE INDEX OVERWRITE pk_atelier_comfy_declared_output ON TABLE atelier_comfy_declared_output FIELDS declared_output_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_declared_output_1 ON TABLE atelier_comfy_declared_output FIELDS registration_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_capability_reject SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE reject_id ON TABLE atelier_comfy_capability_reject TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE registration_id ON TABLE atelier_comfy_capability_reject TYPE record<atelier_comfy_capability_registration> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_comfy_capability_reject TYPE int;
DEFINE FIELD OVERWRITE output_slot ON TABLE atelier_comfy_capability_reject TYPE string;
DEFINE FIELD OVERWRITE reason ON TABLE atelier_comfy_capability_reject TYPE string;
DEFINE INDEX OVERWRITE pk_atelier_comfy_capability_reject ON TABLE atelier_comfy_capability_reject FIELDS reject_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_capability_reject_1 ON TABLE atelier_comfy_capability_reject FIELDS registration_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_intake_output SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE intake_output_id ON TABLE atelier_comfy_intake_output TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_intake_output TYPE uuid;
DEFINE FIELD OVERWRITE node_execution_id ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE registration_id ON TABLE atelier_comfy_intake_output TYPE option<record<atelier_comfy_capability_registration>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE source_node_instance_id ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE source_output_slot ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE media_kind ON TABLE atelier_comfy_intake_output TYPE 'image' | 'mask' | 'latent_preview' | 'video' | 'sidecar_json';
DEFINE FIELD OVERWRITE mime ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE artifact_manifest_ref ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_comfy_intake_output TYPE string;
DEFINE FIELD OVERWRITE routing_intent ON TABLE atelier_comfy_intake_output TYPE 'artifact' | 'sidecar' | 'transient';
DEFINE FIELD OVERWRITE parent_artifact_ref ON TABLE atelier_comfy_intake_output TYPE option<string>;
DEFINE FIELD OVERWRITE prompt_json_ref ON TABLE atelier_comfy_intake_output TYPE option<string>;
DEFINE FIELD OVERWRITE graph_hash ON TABLE atelier_comfy_intake_output TYPE option<string>;
DEFINE FIELD OVERWRITE seed ON TABLE atelier_comfy_intake_output TYPE option<int>;
DEFINE FIELD OVERWRITE materialized_at_utc ON TABLE atelier_comfy_intake_output TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_intake_output ON TABLE atelier_comfy_intake_output FIELDS intake_output_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_intake_output_1 ON TABLE atelier_comfy_intake_output FIELDS workflow_run_id, content_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_comfy_fallback_marker SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_fallback_marker TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE fallback_reason ON TABLE atelier_comfy_fallback_marker TYPE string;
DEFINE FIELD OVERWRITE engaged_at_utc ON TABLE atelier_comfy_fallback_marker TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_fallback_marker ON TABLE atelier_comfy_fallback_marker FIELDS workflow_run_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_sourcing_spec SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE record_id ON TABLE atelier_sourcing_spec TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE sourcing_spec_id ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE schema_version ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE source_kind ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE handler_family ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE handler_version_pin ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE params_json ON TABLE atelier_sourcing_spec TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE params_json.* ON TABLE atelier_sourcing_spec TYPE any;
DEFINE FIELD OVERWRITE required_capabilities ON TABLE atelier_sourcing_spec TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE required_capabilities.* ON TABLE atelier_sourcing_spec TYPE any;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE atelier_sourcing_spec TYPE option<string>;
DEFINE FIELD OVERWRITE spec_hash ON TABLE atelier_sourcing_spec TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_sourcing_spec TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_sourcing_spec ON TABLE atelier_sourcing_spec FIELDS record_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_sourcing_spec_1 ON TABLE atelier_sourcing_spec FIELDS spec_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_handler_version_matrix SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entry_id ON TABLE atelier_handler_version_matrix TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE handler_family ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE handler_version ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE schema_version_min ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE schema_version_max ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE side_effect ON TABLE atelier_handler_version_matrix TYPE 'READ' | 'WRITE' | 'EXECUTE';
DEFINE FIELD OVERWRITE idempotency ON TABLE atelier_handler_version_matrix TYPE 'IDEMPOTENT' | 'IDEMPOTENT_WITH_KEY' | 'NON_IDEMPOTENT';
DEFINE FIELD OVERWRITE required_capabilities ON TABLE atelier_handler_version_matrix TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE required_capabilities.* ON TABLE atelier_handler_version_matrix TYPE any;
DEFINE FIELD OVERWRITE determinism ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE atelier_handler_version_matrix TYPE 'ACTIVE' | 'DEPRECATED' | 'SUNSET';
DEFINE FIELD OVERWRITE job_profile_ref ON TABLE atelier_handler_version_matrix TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_handler_version_matrix TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_handler_version_matrix ON TABLE atelier_handler_version_matrix FIELDS entry_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_handler_version_matrix_1 ON TABLE atelier_handler_version_matrix FIELDS handler_family, handler_version UNIQUE;

DEFINE TABLE OVERWRITE atelier_sourcing_binding_decision SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE decision_id ON TABLE atelier_sourcing_binding_decision TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE sourcing_spec_id ON TABLE atelier_sourcing_binding_decision TYPE string;
DEFINE FIELD OVERWRITE spec_hash ON TABLE atelier_sourcing_binding_decision TYPE string;
DEFINE FIELD OVERWRITE handler_family ON TABLE atelier_sourcing_binding_decision TYPE string;
DEFINE FIELD OVERWRITE handler_version_pin ON TABLE atelier_sourcing_binding_decision TYPE string;
DEFINE FIELD OVERWRITE bound ON TABLE atelier_sourcing_binding_decision TYPE bool;
DEFINE FIELD OVERWRITE resolved_handler_version ON TABLE atelier_sourcing_binding_decision TYPE option<string>;
DEFINE FIELD OVERWRITE matched_entry_id ON TABLE atelier_sourcing_binding_decision TYPE option<record<atelier_handler_version_matrix>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE matrix_snapshot_id ON TABLE atelier_sourcing_binding_decision TYPE uuid;
DEFINE FIELD OVERWRITE capability_satisfied ON TABLE atelier_sourcing_binding_decision TYPE bool;
DEFINE FIELD OVERWRITE resolution_reason ON TABLE atelier_sourcing_binding_decision TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_sourcing_binding_decision TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_sourcing_binding_decision ON TABLE atelier_sourcing_binding_decision FIELDS decision_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_version_mismatch_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_version_mismatch_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE decision_id ON TABLE atelier_version_mismatch_receipt TYPE record<atelier_sourcing_binding_decision> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE sourcing_spec_id ON TABLE atelier_version_mismatch_receipt TYPE string;
DEFINE FIELD OVERWRITE spec_hash ON TABLE atelier_version_mismatch_receipt TYPE string;
DEFINE FIELD OVERWRITE requested_pin ON TABLE atelier_version_mismatch_receipt TYPE string;
DEFINE FIELD OVERWRITE evaluated_versions ON TABLE atelier_version_mismatch_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE evaluated_versions.* ON TABLE atelier_version_mismatch_receipt TYPE any;
DEFINE FIELD OVERWRITE matrix_snapshot_id ON TABLE atelier_version_mismatch_receipt TYPE uuid;
DEFINE FIELD OVERWRITE reason ON TABLE atelier_version_mismatch_receipt TYPE 'no_matching_version' | 'schema_unsupported' | 'sunset' | 'capability_denied';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_version_mismatch_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_version_mismatch_receipt ON TABLE atelier_version_mismatch_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_version_mismatch_receipt_1 ON TABLE atelier_version_mismatch_receipt FIELDS decision_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_sourcing_ingestion_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_sourcing_ingestion_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE decision_id ON TABLE atelier_sourcing_ingestion_receipt TYPE record<atelier_sourcing_binding_decision> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE ingestion_key ON TABLE atelier_sourcing_ingestion_receipt TYPE string;
DEFINE FIELD OVERWRITE handler_family ON TABLE atelier_sourcing_ingestion_receipt TYPE string;
DEFINE FIELD OVERWRITE handler_version ON TABLE atelier_sourcing_ingestion_receipt TYPE string;
DEFINE FIELD OVERWRITE spec_hash ON TABLE atelier_sourcing_ingestion_receipt TYPE string;
DEFINE FIELD OVERWRITE artifact_manifest_refs ON TABLE atelier_sourcing_ingestion_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE artifact_manifest_refs.* ON TABLE atelier_sourcing_ingestion_receipt TYPE any;
DEFINE FIELD OVERWRITE outcome ON TABLE atelier_sourcing_ingestion_receipt TYPE 'fresh' | 'deduped';
DEFINE FIELD OVERWRITE completed_count ON TABLE atelier_sourcing_ingestion_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE pending_count ON TABLE atelier_sourcing_ingestion_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_sourcing_ingestion_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_sourcing_ingestion_receipt ON TABLE atelier_sourcing_ingestion_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_sourcing_ingestion_receipt_1 ON TABLE atelier_sourcing_ingestion_receipt FIELDS ingestion_key UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_probe_report SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE probe_report_id ON TABLE atelier_media_probe_report TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE media_source_id ON TABLE atelier_media_probe_report TYPE string;
DEFINE FIELD OVERWRITE source_media_hash ON TABLE atelier_media_probe_report TYPE string;
DEFINE FIELD OVERWRITE container ON TABLE atelier_media_probe_report TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE duration_ms ON TABLE atelier_media_probe_report TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE streams ON TABLE atelier_media_probe_report TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE streams.* ON TABLE atelier_media_probe_report TYPE any;
DEFINE FIELD OVERWRITE ffprobe_tool_version ON TABLE atelier_media_probe_report TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_media_probe_report TYPE string;
DEFINE FIELD OVERWRITE probed_at ON TABLE atelier_media_probe_report TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_probe_report TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_probe_report ON TABLE atelier_media_probe_report FIELDS probe_report_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_media_probe_report_1 ON TABLE atelier_media_probe_report FIELDS source_media_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_transcript_artifact SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE transcript_id ON TABLE atelier_transcript_artifact TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE media_source_id ON TABLE atelier_transcript_artifact TYPE string;
DEFINE FIELD OVERWRITE source_media_hash ON TABLE atelier_transcript_artifact TYPE record<atelier_media_probe_report> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE language ON TABLE atelier_transcript_artifact TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE model ON TABLE atelier_transcript_artifact TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE model.* ON TABLE atelier_transcript_artifact TYPE any;
DEFINE FIELD OVERWRITE selection_path ON TABLE atelier_transcript_artifact TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE segments ON TABLE atelier_transcript_artifact TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE segments.* ON TABLE atelier_transcript_artifact TYPE any;
DEFINE FIELD OVERWRITE timing_anchors ON TABLE atelier_transcript_artifact TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE timing_anchors.* ON TABLE atelier_transcript_artifact TYPE any;
DEFINE FIELD OVERWRITE format_version ON TABLE atelier_transcript_artifact TYPE string DEFAULT 'TranscriptArtifactV1';
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_transcript_artifact TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_transcript_artifact TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_transcript_artifact ON TABLE atelier_transcript_artifact FIELDS transcript_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_transcript_artifact_1 ON TABLE atelier_transcript_artifact FIELDS artifact_ref UNIQUE;

DEFINE TABLE OVERWRITE atelier_caption_artifact SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE caption_artifact_id ON TABLE atelier_caption_artifact TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE transcript_id ON TABLE atelier_caption_artifact TYPE record<atelier_transcript_artifact> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE media_source_id ON TABLE atelier_caption_artifact TYPE string;
DEFINE FIELD OVERWRITE source_media_hash ON TABLE atelier_caption_artifact TYPE string;
DEFINE FIELD OVERWRITE format ON TABLE atelier_caption_artifact TYPE 'srt' | 'vtt' | 'ass';
DEFINE FIELD OVERWRITE language ON TABLE atelier_caption_artifact TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE max_line_chars ON TABLE atelier_caption_artifact TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE max_lines_per_cue ON TABLE atelier_caption_artifact TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE min_cue_ms ON TABLE atelier_caption_artifact TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE max_cue_ms ON TABLE atelier_caption_artifact TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE cue_count ON TABLE atelier_caption_artifact TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE derived_from_timing_anchors ON TABLE atelier_caption_artifact TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_caption_artifact TYPE string;
DEFINE FIELD OVERWRITE muxed_media_artifact_id ON TABLE atelier_caption_artifact TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_caption_artifact TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_caption_artifact ON TABLE atelier_caption_artifact FIELDS caption_artifact_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_caption_artifact_1 ON TABLE atelier_caption_artifact FIELDS artifact_ref UNIQUE;

DEFINE TABLE OVERWRITE atelier_transcript_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_transcript_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE kind ON TABLE atelier_transcript_receipt TYPE 'media_probe' | 'transcribe' | 'caption_render';
DEFINE FIELD OVERWRITE job_id ON TABLE atelier_transcript_receipt TYPE string;
DEFINE FIELD OVERWRITE feature_id ON TABLE atelier_transcript_receipt TYPE string DEFAULT 'FEAT-ASR';
DEFINE FIELD OVERWRITE source_media_hash ON TABLE atelier_transcript_receipt TYPE string;
DEFINE FIELD OVERWRITE input_artifact_ids ON TABLE atelier_transcript_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE input_artifact_ids.* ON TABLE atelier_transcript_receipt TYPE any;
DEFINE FIELD OVERWRITE output_artifact_id ON TABLE atelier_transcript_receipt TYPE option<string>;
DEFINE FIELD OVERWRITE capability_grants ON TABLE atelier_transcript_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE capability_grants.* ON TABLE atelier_transcript_receipt TYPE any;
DEFINE FIELD OVERWRITE tool_versions ON TABLE atelier_transcript_receipt TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE tool_versions.* ON TABLE atelier_transcript_receipt TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_transcript_receipt TYPE 'completed' | 'failed';
DEFINE FIELD OVERWRITE error_class ON TABLE atelier_transcript_receipt TYPE option<string>;
DEFINE FIELD OVERWRITE partial_artifact_id ON TABLE atelier_transcript_receipt TYPE option<string>;
DEFINE FIELD OVERWRITE emitted_at ON TABLE atelier_transcript_receipt TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_transcript_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_transcript_receipt ON TABLE atelier_transcript_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_transcript_receipt_1 ON TABLE atelier_transcript_receipt FIELDS job_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_output_root SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE root_id ON TABLE atelier_md_output_root TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE configured_root ON TABLE atelier_md_output_root TYPE string;
DEFINE FIELD OVERWRITE materialization_mode ON TABLE atelier_md_output_root TYPE 'copy' | 'hardlink' | 'symlink' DEFAULT 'hardlink';
DEFINE FIELD OVERWRITE per_mode_subdirs ON TABLE atelier_md_output_root TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE per_mode_subdirs.* ON TABLE atelier_md_output_root TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_output_root TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_md_output_root TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_output_root ON TABLE atelier_md_output_root FIELDS root_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_output_root_1 ON TABLE atelier_md_output_root FIELDS configured_root UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_allowlist_policy SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE allowlist_policy_id ON TABLE atelier_md_allowlist_policy TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE name ON TABLE atelier_md_allowlist_policy TYPE string;
DEFINE FIELD OVERWRITE allowed_domains ON TABLE atelier_md_allowlist_policy TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE allowed_domains.* ON TABLE atelier_md_allowlist_policy TYPE any;
DEFINE FIELD OVERWRITE explicit_url_lists ON TABLE atelier_md_allowlist_policy TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE explicit_url_lists.* ON TABLE atelier_md_allowlist_policy TYPE any;
DEFINE FIELD OVERWRITE default_decision ON TABLE atelier_md_allowlist_policy TYPE 'deny' | 'allow' DEFAULT 'deny';
DEFINE FIELD OVERWRITE rate_limit ON TABLE atelier_md_allowlist_policy TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE rate_limit.* ON TABLE atelier_md_allowlist_policy TYPE any;
DEFINE FIELD OVERWRITE max_pages ON TABLE atelier_md_allowlist_policy TYPE int DEFAULT 1500 ASSERT ($value >= 1 AND $value <= 5000);
DEFINE FIELD OVERWRITE robots_posture ON TABLE atelier_md_allowlist_policy TYPE string DEFAULT 'respect';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_allowlist_policy TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_md_allowlist_policy TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_allowlist_policy ON TABLE atelier_md_allowlist_policy FIELDS allowlist_policy_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_allowlist_policy_1 ON TABLE atelier_md_allowlist_policy FIELDS name UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_auth_context SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE auth_context_ref ON TABLE atelier_md_auth_context TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE label ON TABLE atelier_md_auth_context TYPE string;
DEFINE FIELD OVERWRITE auth_mode ON TABLE atelier_md_auth_context TYPE 'none' | 'session' | 'cookie_jar' | 'header' DEFAULT 'none';
DEFINE FIELD OVERWRITE session_ref ON TABLE atelier_md_auth_context TYPE option<string>;
DEFINE FIELD OVERWRITE cookie_jar_artifact_ref ON TABLE atelier_md_auth_context TYPE option<string>;
DEFINE FIELD OVERWRITE header_secret_refs ON TABLE atelier_md_auth_context TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE header_secret_refs.* ON TABLE atelier_md_auth_context TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_auth_context TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_md_auth_context TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_auth_context ON TABLE atelier_md_auth_context FIELDS auth_context_ref UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_auth_context_1 ON TABLE atelier_md_auth_context FIELDS label UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_download_session SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_md_download_session TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE parent_job_id ON TABLE atelier_md_download_session TYPE string;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE atelier_md_download_session TYPE string;
DEFINE FIELD OVERWRITE source_kind ON TABLE atelier_md_download_session TYPE 'youtube' | 'instagram' | 'forumcrawler' | 'videodownloader';
DEFINE FIELD OVERWRITE auth_context_ref ON TABLE atelier_md_download_session TYPE option<record<atelier_md_auth_context>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE allowlist_policy_id ON TABLE atelier_md_download_session TYPE record<atelier_md_allowlist_policy> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE output_root_id ON TABLE atelier_md_download_session TYPE record<atelier_md_output_root> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE stage ON TABLE atelier_md_download_session TYPE 'resolving' | 'enqueued' | 'fetching' | 'probing' | 'merging' | 'materializing' | 'finalized' | 'paused' | 'failed' | 'cancelled' DEFAULT 'resolving';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_download_session TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_md_download_session TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE protocol_id ON TABLE atelier_md_download_session TYPE string DEFAULT 'hsk.media_downloader.batch.v0';
DEFINE FIELD OVERWRITE capability_profile_id ON TABLE atelier_md_download_session TYPE string DEFAULT 'MediaDownloader';
DEFINE FIELD OVERWRITE capability_grant_ref ON TABLE atelier_md_download_session TYPE string DEFAULT 'capgrant://media_downloader/MediaDownloader/migrated-session';
DEFINE INDEX OVERWRITE pk_atelier_md_download_session ON TABLE atelier_md_download_session FIELDS session_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_download_session_1 ON TABLE atelier_md_download_session FIELDS idempotency_key UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_item_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_md_item_state TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_md_item_state TYPE record<atelier_md_download_session> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE normalized_url ON TABLE atelier_md_item_state TYPE string;
DEFINE FIELD OVERWRITE stable_source_id ON TABLE atelier_md_item_state TYPE option<string>;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_md_item_state TYPE option<string>;
DEFINE FIELD OVERWRITE stage ON TABLE atelier_md_item_state TYPE 'enqueued' | 'fetching' | 'probing' | 'merging' | 'materializing' | 'finalized' | 'skipped' | 'failed' DEFAULT 'enqueued';
DEFINE FIELD OVERWRITE bytes_downloaded ON TABLE atelier_md_item_state TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE bytes_total ON TABLE atelier_md_item_state TYPE option<int>;
DEFINE FIELD OVERWRITE part_path_ref ON TABLE atelier_md_item_state TYPE option<string>;
DEFINE FIELD OVERWRITE attempt_count ON TABLE atelier_md_item_state TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE last_error_code ON TABLE atelier_md_item_state TYPE option<string>;
DEFINE FIELD OVERWRITE resume_token ON TABLE atelier_md_item_state TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_item_state TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_md_item_state TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_item_state ON TABLE atelier_md_item_state FIELDS item_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_item_state_1 ON TABLE atelier_md_item_state FIELDS session_id, normalized_url UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_checkpoint SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE atelier_md_checkpoint TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_md_checkpoint TYPE record<atelier_md_download_session> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_md_checkpoint TYPE option<record<atelier_md_item_state>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE stage ON TABLE atelier_md_checkpoint TYPE string;
DEFINE FIELD OVERWRITE bytes_downloaded ON TABLE atelier_md_checkpoint TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE bytes_total ON TABLE atelier_md_checkpoint TYPE option<int>;
DEFINE FIELD OVERWRITE resume_token ON TABLE atelier_md_checkpoint TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_checkpoint TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_checkpoint ON TABLE atelier_md_checkpoint FIELDS checkpoint_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_md_session_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_md_session_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_md_session_receipt TYPE record<atelier_md_download_session> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE parent_job_id ON TABLE atelier_md_session_receipt TYPE string;
DEFINE FIELD OVERWRITE source_kind ON TABLE atelier_md_session_receipt TYPE 'youtube' | 'instagram' | 'forumcrawler' | 'videodownloader';
DEFINE FIELD OVERWRITE auth_context_ref ON TABLE atelier_md_session_receipt TYPE option<record<atelier_md_auth_context>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE allowlist_policy_id ON TABLE atelier_md_session_receipt TYPE record<atelier_md_allowlist_policy> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE output_root_id ON TABLE atelier_md_session_receipt TYPE record<atelier_md_output_root> ASSERT (record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE item_count ON TABLE atelier_md_session_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE succeeded ON TABLE atelier_md_session_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE failed ON TABLE atelier_md_session_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE skipped_deduped ON TABLE atelier_md_session_receipt TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE materialized_paths ON TABLE atelier_md_session_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE materialized_paths.* ON TABLE atelier_md_session_receipt TYPE any;
DEFINE FIELD OVERWRITE manifest_artifact_ref ON TABLE atelier_md_session_receipt TYPE option<string>;
DEFINE FIELD OVERWRITE started_at_utc ON TABLE atelier_md_session_receipt TYPE option<datetime>;
DEFINE FIELD OVERWRITE ended_at_utc ON TABLE atelier_md_session_receipt TYPE option<datetime>;
DEFINE FIELD OVERWRITE terminal_stage ON TABLE atelier_md_session_receipt TYPE 'finalized' | 'failed' | 'cancelled';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_md_session_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_md_session_receipt ON TABLE atelier_md_session_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_md_session_receipt_1 ON TABLE atelier_md_session_receipt FIELDS session_id, terminal_stage UNIQUE;

DEFINE TABLE OVERWRITE atelier_command_corpus_entry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entry_id ON TABLE atelier_command_corpus_entry TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE action_id ON TABLE atelier_command_corpus_entry TYPE string;
DEFINE FIELD OVERWRITE corpus_source ON TABLE atelier_command_corpus_entry TYPE 'preload' | 'ipc_handler' | 'both';
DEFINE FIELD OVERWRITE owner ON TABLE atelier_command_corpus_entry TYPE string;
DEFINE FIELD OVERWRITE actor_eligibility ON TABLE atelier_command_corpus_entry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE actor_eligibility.* ON TABLE atelier_command_corpus_entry TYPE any;
DEFINE FIELD OVERWRITE params_schema_ref ON TABLE atelier_command_corpus_entry TYPE string;
DEFINE FIELD OVERWRITE input_schema_version ON TABLE atelier_command_corpus_entry TYPE int DEFAULT 1;
DEFINE FIELD OVERWRITE capabilities ON TABLE atelier_command_corpus_entry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE capabilities.* ON TABLE atelier_command_corpus_entry TYPE any;
DEFINE FIELD OVERWRITE execution_class ON TABLE atelier_command_corpus_entry TYPE 'pure_projection' | 'write_box' | 'workflow_job' | 'ai_job';
DEFINE FIELD OVERWRITE receipt_shape ON TABLE atelier_command_corpus_entry TYPE string;
DEFINE FIELD OVERWRITE errors ON TABLE atelier_command_corpus_entry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE errors.* ON TABLE atelier_command_corpus_entry TYPE any;
DEFINE FIELD OVERWRITE foreground_flag ON TABLE atelier_command_corpus_entry TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE manual_anchor ON TABLE atelier_command_corpus_entry TYPE string;
DEFINE FIELD OVERWRITE evidence_class ON TABLE atelier_command_corpus_entry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE evidence_class.* ON TABLE atelier_command_corpus_entry TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_command_corpus_entry TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_command_corpus_entry TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_command_corpus_entry ON TABLE atelier_command_corpus_entry FIELDS entry_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_command_corpus_entry_1 ON TABLE atelier_command_corpus_entry FIELDS action_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_command_corpus_blocked SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE blocked_id ON TABLE atelier_command_corpus_blocked TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE action_id ON TABLE atelier_command_corpus_blocked TYPE string;
DEFINE FIELD OVERWRITE blocked_reason ON TABLE atelier_command_corpus_blocked TYPE 'no_manual_anchor' | 'ungoverned_execution' | 'no_capability_gate' | 'foreground_unbounded' | 'no_typed_receipt' | 'no_event_evidence';
DEFINE FIELD OVERWRITE discovered_in ON TABLE atelier_command_corpus_blocked TYPE 'preload' | 'ipc_handler' | 'both';
DEFINE FIELD OVERWRITE recovery_instruction ON TABLE atelier_command_corpus_blocked TYPE string;
DEFINE FIELD OVERWRITE first_seen_utc ON TABLE atelier_command_corpus_blocked TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE last_seen_utc ON TABLE atelier_command_corpus_blocked TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_command_corpus_blocked ON TABLE atelier_command_corpus_blocked FIELDS blocked_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_command_corpus_blocked_1 ON TABLE atelier_command_corpus_blocked FIELDS action_id, blocked_reason UNIQUE;

DEFINE TABLE OVERWRITE atelier_command_corpus_parity_report SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE report_id ON TABLE atelier_command_corpus_parity_report TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE total_corpus ON TABLE atelier_command_corpus_parity_report TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE covered_count ON TABLE atelier_command_corpus_parity_report TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE blocked_count ON TABLE atelier_command_corpus_parity_report TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE orphaned_manual_count ON TABLE atelier_command_corpus_parity_report TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE defects ON TABLE atelier_command_corpus_parity_report TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE defects.* ON TABLE atelier_command_corpus_parity_report TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_command_corpus_parity_report TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_command_corpus_parity_report ON TABLE atelier_command_corpus_parity_report FIELDS report_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_stealth_window SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE window_ref_id ON TABLE atelier_stealth_window TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE owner_actor ON TABLE atelier_stealth_window TYPE string;
DEFINE FIELD OVERWRITE title ON TABLE atelier_stealth_window TYPE string;
DEFINE FIELD OVERWRITE visibility ON TABLE atelier_stealth_window TYPE 'off_screen_only' | 'diagnostic_embed' | 'foreground_exception_bound';
DEFINE FIELD OVERWRITE quiet_json ON TABLE atelier_stealth_window TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE quiet_json.* ON TABLE atelier_stealth_window TYPE any;
DEFINE FIELD OVERWRITE layout_json ON TABLE atelier_stealth_window TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE layout_json.* ON TABLE atelier_stealth_window TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_stealth_window TYPE 'open' | 'closed' DEFAULT 'open';
DEFINE FIELD OVERWRITE revision ON TABLE atelier_stealth_window TYPE int DEFAULT 1;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_stealth_window TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_stealth_window TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_stealth_window ON TABLE atelier_stealth_window FIELDS window_ref_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_stealth_window_1 ON TABLE atelier_stealth_window FIELDS owner_actor, title UNIQUE;

DEFINE TABLE OVERWRITE atelier_stealth_ref SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE ref_id ON TABLE atelier_stealth_ref TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE window_ref_id ON TABLE atelier_stealth_ref TYPE record<atelier_stealth_window> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_stealth_ref TYPE int;
DEFINE FIELD OVERWRITE ref_kind ON TABLE atelier_stealth_ref TYPE 'artifact' | 'spec_anchor' | 'transcript' | 'job_receipt' | 'ledger_event' | 'screenshot' | 'diagnostic';
DEFINE FIELD OVERWRITE resolver ON TABLE atelier_stealth_ref TYPE string;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE atelier_stealth_ref TYPE string;
DEFINE FIELD OVERWRITE redaction_state ON TABLE atelier_stealth_ref TYPE bool;
DEFINE FIELD OVERWRITE pinned_at_utc ON TABLE atelier_stealth_ref TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_stealth_ref ON TABLE atelier_stealth_ref FIELDS ref_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_stealth_ref_1 ON TABLE atelier_stealth_ref FIELDS window_ref_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_stealth_capture SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE capture_id ON TABLE atelier_stealth_capture TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE window_ref_id ON TABLE atelier_stealth_capture TYPE record<atelier_stealth_window> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE artifact_manifest_id ON TABLE atelier_stealth_capture TYPE string;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE atelier_stealth_capture TYPE string;
DEFINE FIELD OVERWRITE captured_at_utc ON TABLE atelier_stealth_capture TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_stealth_capture ON TABLE atelier_stealth_capture FIELDS capture_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_stealth_capture_1 ON TABLE atelier_stealth_capture FIELDS window_ref_id, artifact_manifest_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_sheet_parse_snapshot SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE parse_id ON TABLE atelier_sheet_parse_snapshot TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE version_id ON TABLE atelier_sheet_parse_snapshot TYPE record<atelier_sheet_version> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE template_id ON TABLE atelier_sheet_parse_snapshot TYPE string;
DEFINE FIELD OVERWRITE source_path ON TABLE atelier_sheet_parse_snapshot TYPE option<string>;
DEFINE FIELD OVERWRITE template_version ON TABLE atelier_sheet_parse_snapshot TYPE option<string>;
DEFINE FIELD OVERWRITE template_hash ON TABLE atelier_sheet_parse_snapshot TYPE string;
DEFINE FIELD OVERWRITE ast ON TABLE atelier_sheet_parse_snapshot TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE ast.* ON TABLE atelier_sheet_parse_snapshot TYPE any;
DEFINE FIELD OVERWRITE unmapped_lines ON TABLE atelier_sheet_parse_snapshot TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE unmapped_lines.* ON TABLE atelier_sheet_parse_snapshot TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_sheet_parse_snapshot TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_sheet_parse_snapshot ON TABLE atelier_sheet_parse_snapshot FIELDS parse_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_sheet_parse_snapshot_1 ON TABLE atelier_sheet_parse_snapshot FIELDS version_id, template_id, template_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_bulk_operation_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_bulk_operation_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE operation ON TABLE atelier_bulk_operation_receipt TYPE string;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_bulk_operation_receipt TYPE string;
DEFINE FIELD OVERWRITE target_count ON TABLE atelier_bulk_operation_receipt TYPE int;
DEFINE FIELD OVERWRITE mutation_count ON TABLE atelier_bulk_operation_receipt TYPE int;
DEFINE FIELD OVERWRITE status ON TABLE atelier_bulk_operation_receipt TYPE string DEFAULT 'applied';
DEFINE FIELD OVERWRITE payload ON TABLE atelier_bulk_operation_receipt TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE payload.* ON TABLE atelier_bulk_operation_receipt TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_bulk_operation_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_bulk_operation_receipt ON TABLE atelier_bulk_operation_receipt FIELDS receipt_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_trash_marker SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE marker_id ON TABLE atelier_trash_marker TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE target_type ON TABLE atelier_trash_marker TYPE 'media_asset' | 'sheet_version';
DEFINE FIELD OVERWRITE target_id ON TABLE atelier_trash_marker TYPE uuid;
DEFINE FIELD OVERWRITE reason ON TABLE atelier_trash_marker TYPE string;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_trash_marker TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_trash_marker TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_trash_marker ON TABLE atelier_trash_marker FIELDS marker_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_trash_marker_1 ON TABLE atelier_trash_marker FIELDS target_type, target_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_source_evidence_record SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE source_id ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE matrix_id ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE source_label ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE product_area ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE maturity_status ON TABLE atelier_source_evidence_record TYPE 'DONE' | 'REVIEW' | 'BLOCKED';
DEFINE FIELD OVERWRITE implementation_status ON TABLE atelier_source_evidence_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE evidence_refs ON TABLE atelier_source_evidence_record TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE evidence_refs.* ON TABLE atelier_source_evidence_record TYPE any;
DEFINE FIELD OVERWRITE proof_refs ON TABLE atelier_source_evidence_record TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE proof_refs.* ON TABLE atelier_source_evidence_record TYPE any;
DEFINE FIELD OVERWRITE gap_reason ON TABLE atelier_source_evidence_record TYPE option<string>;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_source_evidence_record TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_source_evidence_record ON TABLE atelier_source_evidence_record FIELDS matrix_id, source_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_anchor_verification_record SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_id ON TABLE atelier_anchor_verification_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE matrix_id ON TABLE atelier_anchor_verification_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE source_id ON TABLE atelier_anchor_verification_record TYPE record<atelier_source_evidence_record> ASSERT (record::exists($value)) AND (string::trim($value) = $value AND $value != '' AND record::exists(type::record('atelier_source_evidence_record', [$this.matrix_id, $value]))) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE anchor_label ON TABLE atelier_anchor_verification_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE expected_product_path ON TABLE atelier_anchor_verification_record TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE verification_status ON TABLE atelier_anchor_verification_record TYPE 'VERIFIED' | 'BLOCKED_MISSING_ANCHOR';
DEFINE FIELD OVERWRITE verified_product_paths ON TABLE atelier_anchor_verification_record TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE verified_product_paths.* ON TABLE atelier_anchor_verification_record TYPE any;
DEFINE FIELD OVERWRITE blocking_reason ON TABLE atelier_anchor_verification_record TYPE option<string>;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_anchor_verification_record TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_anchor_verification_record ON TABLE atelier_anchor_verification_record FIELDS matrix_id, anchor_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_review_metadata SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_review_metadata TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE favorite ON TABLE atelier_media_review_metadata TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE rating ON TABLE atelier_media_review_metadata TYPE int DEFAULT 0 ASSERT ($value >= 0 AND $value <= 5);
DEFINE FIELD OVERWRITE frontpage ON TABLE atelier_media_review_metadata TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE carousel ON TABLE atelier_media_review_metadata TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE notes ON TABLE atelier_media_review_metadata TYPE option<string>;
DEFINE FIELD OVERWRITE review_status ON TABLE atelier_media_review_metadata TYPE 'unreviewed' | 'review' | 'approved' | 'rejected' | 'deferred' DEFAULT 'unreviewed';
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_media_review_metadata TYPE string;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_media_review_metadata TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_review_metadata ON TABLE atelier_media_review_metadata FIELDS asset_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_derivative SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE derivative_id ON TABLE atelier_media_derivative TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_derivative TYPE record<atelier_media_asset> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE derivative_kind ON TABLE atelier_media_derivative TYPE 'thumbnail' | 'proxy' | 'photo_studio_skeleton';
DEFINE FIELD OVERWRITE target_width ON TABLE atelier_media_derivative TYPE int ASSERT ($value > 0 AND $value <= 16384);
DEFINE FIELD OVERWRITE target_height ON TABLE atelier_media_derivative TYPE int ASSERT ($value > 0 AND $value <= 16384);
DEFINE FIELD OVERWRITE format ON TABLE atelier_media_derivative TYPE 'png' | 'jpeg';
DEFINE FIELD OVERWRITE status ON TABLE atelier_media_derivative TYPE 'pending' | 'generating' | 'generated' | 'retryable_error' | 'failed' DEFAULT 'pending';
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_media_derivative TYPE option<string>;
DEFINE FIELD OVERWRITE artifact_manifest_ref ON TABLE atelier_media_derivative TYPE option<string>;
DEFINE FIELD OVERWRITE mime ON TABLE atelier_media_derivative TYPE option<string>;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_media_derivative TYPE option<int> ASSERT ($value = NONE OR $value > 0);
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_media_derivative TYPE string;
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_media_derivative TYPE string;
DEFINE FIELD OVERWRITE attempt_count ON TABLE atelier_media_derivative TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE retry_count ON TABLE atelier_media_derivative TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE last_error_code ON TABLE atelier_media_derivative TYPE option<string>;
DEFINE FIELD OVERWRITE last_error_ref ON TABLE atelier_media_derivative TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_derivative TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_media_derivative TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_derivative ON TABLE atelier_media_derivative FIELDS derivative_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_media_derivative_1 ON TABLE atelier_media_derivative FIELDS asset_id, derivative_kind, target_width, target_height, format UNIQUE;

DEFINE TABLE OVERWRITE atelier_similarity_rebuild_job SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE atelier_similarity_rebuild_job TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE asset_internal_id ON TABLE atelier_similarity_rebuild_job TYPE record<atelier_media_asset> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE status ON TABLE atelier_similarity_rebuild_job TYPE 'running' | 'completed' | 'failed';
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_similarity_rebuild_job TYPE string;
DEFINE FIELD OVERWRITE processed_count ON TABLE atelier_similarity_rebuild_job TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE failed_count ON TABLE atelier_similarity_rebuild_job TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE dhash_hex ON TABLE atelier_similarity_rebuild_job TYPE option<string>;
DEFINE FIELD OVERWRITE palette_json ON TABLE atelier_similarity_rebuild_job TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE palette_json.* ON TABLE atelier_similarity_rebuild_job TYPE any;
DEFINE FIELD OVERWRITE error_ref ON TABLE atelier_similarity_rebuild_job TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_similarity_rebuild_job TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_similarity_rebuild_job TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_similarity_rebuild_job ON TABLE atelier_similarity_rebuild_job FIELDS job_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_ai_tag_suggestion SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE suggestion_id ON TABLE atelier_ai_tag_suggestion TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_ai_tag_suggestion TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_ai_tag_suggestion TYPE option<record<atelier_media_asset>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE tag_text ON TABLE atelier_ai_tag_suggestion TYPE string;
DEFINE FIELD OVERWRITE confidence ON TABLE atelier_ai_tag_suggestion TYPE option<float> ASSERT ($value = NONE OR ($value >= 0.0 AND $value <= 1.0));
DEFINE FIELD OVERWRITE model_receipt_ref ON TABLE atelier_ai_tag_suggestion TYPE string;
DEFINE FIELD OVERWRITE tool_receipt_ref ON TABLE atelier_ai_tag_suggestion TYPE string;
DEFINE FIELD OVERWRITE suggested_by ON TABLE atelier_ai_tag_suggestion TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE atelier_ai_tag_suggestion TYPE 'proposed' | 'accepted' | 'rejected' | 'applied' DEFAULT 'proposed';
DEFINE FIELD OVERWRITE decided_by ON TABLE atelier_ai_tag_suggestion TYPE option<string>;
DEFINE FIELD OVERWRITE decision_reason ON TABLE atelier_ai_tag_suggestion TYPE option<string>;
DEFINE FIELD OVERWRITE applied_tag_id ON TABLE atelier_ai_tag_suggestion TYPE option<record<atelier_tag>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_ai_tag_suggestion TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_ai_tag_suggestion TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_ai_tag_suggestion ON TABLE atelier_ai_tag_suggestion FIELDS suggestion_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_sidecar SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE sidecar_id ON TABLE atelier_media_sidecar TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE parent_asset_id ON TABLE atelier_media_sidecar TYPE record<atelier_media_asset> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE sidecar_asset_id ON TABLE atelier_media_sidecar TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value != $this.parent_asset_id) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE relation_kind ON TABLE atelier_media_sidecar TYPE 'openpose_json' | 'workflow_json';
DEFINE FIELD OVERWRITE hidden_from_gallery ON TABLE atelier_media_sidecar TYPE bool DEFAULT true ASSERT ($value = true);
DEFINE FIELD OVERWRITE searchable_by_relation ON TABLE atelier_media_sidecar TYPE bool DEFAULT true ASSERT ($value = true);
DEFINE FIELD OVERWRITE created_by ON TABLE atelier_media_sidecar TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_sidecar TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_media_sidecar TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_sidecar ON TABLE atelier_media_sidecar FIELDS sidecar_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_media_sidecar_1 ON TABLE atelier_media_sidecar FIELDS parent_asset_id, sidecar_asset_id, relation_kind UNIQUE;

DEFINE TABLE OVERWRITE atelier_filesystem_health_check SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE check_id ON TABLE atelier_filesystem_health_check TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_filesystem_health_check TYPE string;
DEFINE FIELD OVERWRITE scope_label ON TABLE atelier_filesystem_health_check TYPE option<string>;
DEFINE FIELD OVERWRITE summary ON TABLE atelier_filesystem_health_check TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE summary.* ON TABLE atelier_filesystem_health_check TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_filesystem_health_check TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_filesystem_health_check ON TABLE atelier_filesystem_health_check FIELDS check_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_filesystem_health_finding SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE finding_id ON TABLE atelier_filesystem_health_finding TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE check_id ON TABLE atelier_filesystem_health_finding TYPE record<atelier_filesystem_health_check> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE finding_kind ON TABLE atelier_filesystem_health_finding TYPE 'missing_original' | 'missing_thumbnail' | 'inbox_pending' | 'untracked_original' | 'sidecar_visibility_anomaly';
DEFINE FIELD OVERWRITE target_type ON TABLE atelier_filesystem_health_finding TYPE string;
DEFINE FIELD OVERWRITE target_id ON TABLE atelier_filesystem_health_finding TYPE string;
DEFINE FIELD OVERWRITE details ON TABLE atelier_filesystem_health_finding TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE details.* ON TABLE atelier_filesystem_health_finding TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_filesystem_health_finding TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_filesystem_health_finding ON TABLE atelier_filesystem_health_finding FIELDS finding_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_image_import_request SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE import_id ON TABLE atelier_image_import_request TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE idempotency_key ON TABLE atelier_image_import_request TYPE string;
DEFINE FIELD OVERWRITE source_kind ON TABLE atelier_image_import_request TYPE 'clipboard' | 'url';
DEFINE FIELD OVERWRITE status ON TABLE atelier_image_import_request TYPE 'materialized' | 'queued';
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_image_import_request TYPE string;
DEFINE FIELD OVERWRITE normalized_url ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE source_url_hash ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE source_host ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE source_label ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE expected_mime ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE capability_profile_id ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE capability_grant_ref ON TABLE atelier_image_import_request TYPE option<string> ASSERT (($this.source_kind = 'clipboard' AND $this.asset_id != NONE AND $this.artifact_ref != NONE) OR ($this.source_kind = 'url' AND $this.normalized_url != NONE AND $this.source_url_hash != NONE AND $this.source_host != NONE AND $this.capability_profile_id != NONE AND $value != NONE));
DEFINE FIELD OVERWRITE required_capabilities ON TABLE atelier_image_import_request TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE required_capabilities.* ON TABLE atelier_image_import_request TYPE any;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_image_import_request TYPE option<record<atelier_media_asset>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_image_import_request TYPE option<string>;
DEFINE FIELD OVERWRITE source_provenance ON TABLE atelier_image_import_request TYPE string;
DEFINE FIELD OVERWRITE preflight ON TABLE atelier_image_import_request TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE preflight.* ON TABLE atelier_image_import_request TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_image_import_request TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_image_import_request TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_image_import_request ON TABLE atelier_image_import_request FIELDS import_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_image_import_request_1 ON TABLE atelier_image_import_request FIELDS idempotency_key UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_source_provenance_ref SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_source_provenance_ref TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_url_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string>;
DEFINE FIELD OVERWRITE source_path_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string>;
DEFINE FIELD OVERWRITE source_note_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string>;
DEFINE FIELD OVERWRITE contact_sheet_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string>;
DEFINE FIELD OVERWRITE task_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string>;
DEFINE FIELD OVERWRITE run_ref ON TABLE atelier_media_source_provenance_ref TYPE option<string> ASSERT ($this.source_url_ref != NONE OR $this.source_path_ref != NONE OR $this.source_note_ref != NONE OR $this.contact_sheet_ref != NONE OR $this.task_ref != NONE OR $value != NONE);
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_media_source_provenance_ref TYPE string;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_media_source_provenance_ref TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_source_provenance_ref ON TABLE atelier_media_source_provenance_ref FIELDS asset_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_intake_item_rejection_audit SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE audit_id ON TABLE atelier_intake_item_rejection_audit TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_intake_item_rejection_audit TYPE record<atelier_intake_item> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE batch_id ON TABLE atelier_intake_item_rejection_audit TYPE record<atelier_intake_batch> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE lane ON TABLE atelier_intake_item_rejection_audit TYPE 'rejected' | 'skipped' | 'failed';
DEFINE FIELD OVERWRITE reason ON TABLE atelier_intake_item_rejection_audit TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE source_path_ref ON TABLE atelier_intake_item_rejection_audit TYPE string ASSERT (string::trim($value) = $value AND $value != '');
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_intake_item_rejection_audit TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_intake_item_rejection_audit ON TABLE atelier_intake_item_rejection_audit FIELDS audit_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_intake_item_rejection_audit_1 ON TABLE atelier_intake_item_rejection_audit FIELDS item_id, lane, reason UNIQUE;

DEFINE TABLE OVERWRITE atelier_export_intake_link SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE link_id ON TABLE atelier_export_intake_link TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE export_id ON TABLE atelier_export_intake_link TYPE record<atelier_export_request> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE batch_id ON TABLE atelier_export_intake_link TYPE record<atelier_intake_batch> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_export_intake_link TYPE record<atelier_intake_item> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE target_character_id ON TABLE atelier_export_intake_link TYPE option<record<atelier_character>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE target_sheet_version_id ON TABLE atelier_export_intake_link TYPE option<record<atelier_sheet_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE target_collection_id ON TABLE atelier_export_intake_link TYPE option<record<atelier_collection>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE version_agnostic ON TABLE atelier_export_intake_link TYPE bool;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_export_intake_link TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_export_intake_link ON TABLE atelier_export_intake_link FIELDS link_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_export_intake_link_1 ON TABLE atelier_export_intake_link FIELDS export_id, batch_id, item_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_media_asset_tag SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_asset_tag TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE tag_id ON TABLE atelier_media_asset_tag TYPE record<atelier_tag> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source ON TABLE atelier_media_asset_tag TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_media_asset_tag TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_media_asset_tag ON TABLE atelier_media_asset_tag FIELDS asset_id, tag_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_collection_metadata_application SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE application_id ON TABLE atelier_collection_metadata_application TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE collection_id ON TABLE atelier_collection_metadata_application TYPE record<atelier_collection> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_collection_metadata_application TYPE string;
DEFINE FIELD OVERWRITE applied_tags_json ON TABLE atelier_collection_metadata_application TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE applied_tags_json.* ON TABLE atelier_collection_metadata_application TYPE any;
DEFINE FIELD OVERWRITE removed_tags_json ON TABLE atelier_collection_metadata_application TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE removed_tags_json.* ON TABLE atelier_collection_metadata_application TYPE any;
DEFINE FIELD OVERWRITE affected_asset_count ON TABLE atelier_collection_metadata_application TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_collection_metadata_application TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_collection_metadata_application ON TABLE atelier_collection_metadata_application FIELDS application_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_contact_sheet_svg_artifact SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE svg_artifact_id ON TABLE atelier_contact_sheet_svg_artifact TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE sheet_id ON TABLE atelier_contact_sheet_svg_artifact TYPE record<atelier_contact_sheet> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE manifest_hash ON TABLE atelier_contact_sheet_svg_artifact TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_contact_sheet_svg_artifact TYPE string;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_contact_sheet_svg_artifact TYPE string;
DEFINE FIELD OVERWRITE svg_text ON TABLE atelier_contact_sheet_svg_artifact TYPE string;
DEFINE FIELD OVERWRITE image_count ON TABLE atelier_contact_sheet_svg_artifact TYPE int;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_contact_sheet_svg_artifact TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_contact_sheet_svg_artifact ON TABLE atelier_contact_sheet_svg_artifact FIELDS svg_artifact_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_contact_sheet_svg_artifact_1 ON TABLE atelier_contact_sheet_svg_artifact FIELDS artifact_ref UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_contact_sheet_svg_artifact_2 ON TABLE atelier_contact_sheet_svg_artifact FIELDS sheet_id, manifest_hash UNIQUE;

DEFINE TABLE OVERWRITE atelier_contact_sheet_raster_export_plan SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE plan_id ON TABLE atelier_contact_sheet_raster_export_plan TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE sheet_id ON TABLE atelier_contact_sheet_raster_export_plan TYPE record<atelier_contact_sheet> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE format ON TABLE atelier_contact_sheet_raster_export_plan TYPE 'png' | 'jpg';
DEFINE FIELD OVERWRITE status ON TABLE atelier_contact_sheet_raster_export_plan TYPE 'planned' DEFAULT 'planned';
DEFINE FIELD OVERWRITE reason ON TABLE atelier_contact_sheet_raster_export_plan TYPE string;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_contact_sheet_raster_export_plan TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_contact_sheet_raster_export_plan TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_contact_sheet_raster_export_plan ON TABLE atelier_contact_sheet_raster_export_plan FIELDS plan_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_contact_sheet_raster_export_plan_1 ON TABLE atelier_contact_sheet_raster_export_plan FIELDS sheet_id, format UNIQUE;

DEFINE TABLE OVERWRITE atelier_character_document SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE document_id ON TABLE atelier_character_document TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_character_document TYPE record<atelier_character> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE doc_type ON TABLE atelier_character_document TYPE 'note' | 'story' | 'moodboard';
DEFINE FIELD OVERWRITE title ON TABLE atelier_character_document TYPE string;
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_character_document TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_character_document TYPE any;
DEFINE FIELD OVERWRITE current_version_id ON TABLE atelier_character_document TYPE option<uuid>;
DEFINE FIELD OVERWRITE current_version_seq ON TABLE atelier_character_document TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_character_document TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_character_document TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_character_document ON TABLE atelier_character_document FIELDS document_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_character_document_version SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE version_id ON TABLE atelier_character_document_version TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE document_id ON TABLE atelier_character_document_version TYPE record<atelier_character_document> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE version_seq ON TABLE atelier_character_document_version TYPE int;
DEFINE FIELD OVERWRITE title ON TABLE atelier_character_document_version TYPE string;
DEFINE FIELD OVERWRITE body_raw_text ON TABLE atelier_character_document_version TYPE string;
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_character_document_version TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_character_document_version TYPE any;
DEFINE FIELD OVERWRITE author ON TABLE atelier_character_document_version TYPE string;
DEFINE FIELD OVERWRITE parent_version_id ON TABLE atelier_character_document_version TYPE option<record<atelier_character_document_version>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_character_document_version TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_character_document_version ON TABLE atelier_character_document_version FIELDS version_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_character_document_version_1 ON TABLE atelier_character_document_version FIELDS document_id, version_seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_story_card SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE card_id ON TABLE atelier_story_card TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE story_document_id ON TABLE atelier_story_card TYPE record<atelier_character_document> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_story_card TYPE int;
DEFINE FIELD OVERWRITE title ON TABLE atelier_story_card TYPE string;
DEFINE FIELD OVERWRITE body_raw_text ON TABLE atelier_story_card TYPE string;
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_story_card TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_story_card TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_story_card TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_story_card TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_story_card ON TABLE atelier_story_card FIELDS card_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_story_card_1 ON TABLE atelier_story_card FIELDS story_document_id, seq UNIQUE;

DEFINE TABLE OVERWRITE atelier_story_beat SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE beat_id ON TABLE atelier_story_beat TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE story_document_id ON TABLE atelier_story_beat TYPE record<atelier_character_document> ASSERT (record::exists($value)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE card_id ON TABLE atelier_story_beat TYPE option<record<atelier_story_card>> ASSERT ($value = NONE OR record::exists($value)) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE seq ON TABLE atelier_story_beat TYPE int;
DEFINE FIELD OVERWRITE beat_text ON TABLE atelier_story_beat TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_story_beat TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_story_beat TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_story_beat ON TABLE atelier_story_beat FIELDS beat_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_story_beat_1 ON TABLE atelier_story_beat FIELDS story_document_id, seq UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_event_aggregate ON TABLE atelier_event FIELDS aggregate_type, aggregate_id;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_status ON TABLE atelier_intake_batch FIELDS status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_character ON TABLE atelier_intake_batch FIELDS character_internal_id;
DEFINE INDEX OVERWRITE idx_atelier_intake_item_batch_lane ON TABLE atelier_intake_item FIELDS batch_id, lane;
DEFINE INDEX OVERWRITE idx_atelier_intake_item_hash ON TABLE atelier_intake_item FIELDS content_hash;
DEFINE INDEX OVERWRITE idx_atelier_collection_item_order ON TABLE atelier_collection_item FIELDS collection_id, sort_order, added_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_contact_sheet_source ON TABLE atelier_contact_sheet FIELDS source_type, source_id;
DEFINE INDEX OVERWRITE idx_atelier_contact_sheet_created ON TABLE atelier_contact_sheet FIELDS created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_character_tag_tag ON TABLE atelier_character_tag FIELDS tag_id;
DEFINE INDEX OVERWRITE idx_atelier_similarity_dhash ON TABLE atelier_similarity_projection FIELDS dhash_hex;
DEFINE INDEX OVERWRITE idx_atelier_export_request_character ON TABLE atelier_export_request FIELDS character_internal_id;
DEFINE INDEX OVERWRITE idx_atelier_export_manifest_entry_export ON TABLE atelier_export_manifest_entry FIELDS export_id;
DEFINE INDEX OVERWRITE idx_atelier_media_annotation_asset ON TABLE atelier_media_annotation FIELDS asset_id, seq;
DEFINE INDEX OVERWRITE idx_atelier_preference_scope ON TABLE atelier_preference FIELDS scope_kind, character_internal_id;
DEFINE INDEX OVERWRITE idx_atelier_pose_rig_character ON TABLE atelier_pose_rig FIELDS character_internal_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_pose_rig_source_asset ON TABLE atelier_pose_rig FIELDS source_asset_id;
DEFINE INDEX OVERWRITE idx_atelier_pose_rig_hash ON TABLE atelier_pose_rig FIELDS content_hash;
DEFINE INDEX OVERWRITE idx_atelier_identity_profile_character ON TABLE atelier_identity_profile FIELDS character_internal_id, kind, seq;
DEFINE INDEX OVERWRITE idx_atelier_comfy_probe_outcome ON TABLE atelier_comfy_bridge_probe FIELDS probe_outcome;
DEFINE INDEX OVERWRITE idx_atelier_comfy_declared_output_reg ON TABLE atelier_comfy_declared_output FIELDS registration_id;
DEFINE INDEX OVERWRITE idx_atelier_comfy_capability_reject_reg ON TABLE atelier_comfy_capability_reject FIELDS registration_id;
DEFINE INDEX OVERWRITE idx_atelier_comfy_intake_output_run ON TABLE atelier_comfy_intake_output FIELDS workflow_run_id, materialized_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_comfy_intake_output_reg ON TABLE atelier_comfy_intake_output FIELDS registration_id;
DEFINE INDEX OVERWRITE idx_atelier_sourcing_spec_id ON TABLE atelier_sourcing_spec FIELDS sourcing_spec_id;
DEFINE INDEX OVERWRITE idx_atelier_sourcing_spec_family ON TABLE atelier_sourcing_spec FIELDS handler_family;
DEFINE INDEX OVERWRITE idx_atelier_handler_version_matrix_family ON TABLE atelier_handler_version_matrix FIELDS handler_family, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_binding_decision_spec_hash ON TABLE atelier_sourcing_binding_decision FIELDS spec_hash;
DEFINE INDEX OVERWRITE idx_atelier_binding_decision_matched_entry ON TABLE atelier_sourcing_binding_decision FIELDS matched_entry_id;
DEFINE INDEX OVERWRITE idx_atelier_ingestion_receipt_decision ON TABLE atelier_sourcing_ingestion_receipt FIELDS decision_id;
DEFINE INDEX OVERWRITE idx_atelier_media_probe_source ON TABLE atelier_media_probe_report FIELDS media_source_id;
DEFINE INDEX OVERWRITE idx_atelier_transcript_hash ON TABLE atelier_transcript_artifact FIELDS source_media_hash;
DEFINE INDEX OVERWRITE idx_atelier_caption_transcript ON TABLE atelier_caption_artifact FIELDS transcript_id;
DEFINE INDEX OVERWRITE idx_atelier_caption_hash ON TABLE atelier_caption_artifact FIELDS source_media_hash;
DEFINE INDEX OVERWRITE idx_atelier_transcript_receipt_source ON TABLE atelier_transcript_receipt FIELDS source_media_hash, emitted_at;
DEFINE INDEX OVERWRITE idx_atelier_transcript_receipt_kind ON TABLE atelier_transcript_receipt FIELDS kind, status;
DEFINE INDEX OVERWRITE idx_atelier_md_session_stage ON TABLE atelier_md_download_session FIELDS stage, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_md_session_job ON TABLE atelier_md_download_session FIELDS parent_job_id;
DEFINE INDEX OVERWRITE idx_atelier_md_item_session_stage ON TABLE atelier_md_item_state FIELDS session_id, stage;
DEFINE INDEX OVERWRITE idx_atelier_md_item_stable_id ON TABLE atelier_md_item_state FIELDS stable_source_id;
DEFINE INDEX OVERWRITE idx_atelier_md_item_content_hash ON TABLE atelier_md_item_state FIELDS content_hash;
DEFINE INDEX OVERWRITE idx_atelier_md_checkpoint_session ON TABLE atelier_md_checkpoint FIELDS session_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_md_checkpoint_item ON TABLE atelier_md_checkpoint FIELDS item_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_md_receipt_session ON TABLE atelier_md_session_receipt FIELDS session_id;
DEFINE INDEX OVERWRITE idx_atelier_command_corpus_entry_owner ON TABLE atelier_command_corpus_entry FIELDS owner, action_id;
DEFINE INDEX OVERWRITE idx_atelier_command_corpus_entry_manual_anchor ON TABLE atelier_command_corpus_entry FIELDS manual_anchor;
DEFINE INDEX OVERWRITE idx_atelier_command_corpus_blocked_action ON TABLE atelier_command_corpus_blocked FIELDS action_id;
DEFINE INDEX OVERWRITE idx_atelier_command_corpus_blocked_last_seen ON TABLE atelier_command_corpus_blocked FIELDS last_seen_utc, action_id;
DEFINE INDEX OVERWRITE idx_atelier_command_corpus_parity_report_created ON TABLE atelier_command_corpus_parity_report FIELDS created_at_utc, report_id;
DEFINE INDEX OVERWRITE idx_atelier_stealth_window_owner ON TABLE atelier_stealth_window FIELDS owner_actor, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_stealth_window_owner_status ON TABLE atelier_stealth_window FIELDS owner_actor, status;
DEFINE INDEX OVERWRITE idx_atelier_stealth_ref_window_seq ON TABLE atelier_stealth_ref FIELDS window_ref_id, seq;
DEFINE INDEX OVERWRITE idx_atelier_stealth_capture_window ON TABLE atelier_stealth_capture FIELDS window_ref_id, captured_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_event_kernel_event ON TABLE atelier_event FIELDS kernel_event_id;
DEFINE INDEX OVERWRITE idx_atelier_preference_namespace ON TABLE atelier_preference FIELDS namespace, name;
DEFINE INDEX OVERWRITE idx_atelier_preference_scope_revision ON TABLE atelier_preference FIELDS scope_kind, character_internal_id, revision;
DEFINE INDEX OVERWRITE idx_atelier_md_session_protocol_profile ON TABLE atelier_md_download_session FIELDS protocol_id, capability_profile_id;
DEFINE INDEX OVERWRITE idx_atelier_sheet_parse_snapshot_version ON TABLE atelier_sheet_parse_snapshot FIELDS version_id;
DEFINE INDEX OVERWRITE idx_atelier_sheet_parse_snapshot_template ON TABLE atelier_sheet_parse_snapshot FIELDS template_id, template_hash;
DEFINE INDEX OVERWRITE idx_atelier_bulk_operation_receipt_operation ON TABLE atelier_bulk_operation_receipt FIELDS operation, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_trash_marker_target ON TABLE atelier_trash_marker FIELDS target_type, target_id;
DEFINE INDEX OVERWRITE idx_atelier_media_asset_retention ON TABLE atelier_media_asset FIELDS retention_class;
DEFINE INDEX OVERWRITE idx_atelier_source_evidence_matrix ON TABLE atelier_source_evidence_record FIELDS matrix_id, maturity_status;
DEFINE INDEX OVERWRITE idx_atelier_anchor_verification_matrix ON TABLE atelier_anchor_verification_record FIELDS matrix_id, verification_status;
DEFINE INDEX OVERWRITE idx_atelier_anchor_verification_source ON TABLE atelier_anchor_verification_record FIELDS source_id;
DEFINE INDEX OVERWRITE idx_atelier_media_review_metadata_status ON TABLE atelier_media_review_metadata FIELDS review_status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_media_review_metadata_flags ON TABLE atelier_media_review_metadata FIELDS favorite, frontpage, carousel;
DEFINE INDEX OVERWRITE idx_atelier_media_derivative_asset ON TABLE atelier_media_derivative FIELDS asset_id, derivative_kind, target_width, target_height;
DEFINE INDEX OVERWRITE idx_atelier_media_derivative_status ON TABLE atelier_media_derivative FIELDS status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_similarity_rebuild_job_asset ON TABLE atelier_similarity_rebuild_job FIELDS asset_internal_id, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_similarity_rebuild_job_status ON TABLE atelier_similarity_rebuild_job FIELDS status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_ai_tag_suggestion_character ON TABLE atelier_ai_tag_suggestion FIELDS character_internal_id, status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_ai_tag_suggestion_asset ON TABLE atelier_ai_tag_suggestion FIELDS asset_id, status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_ai_tag_suggestion_status ON TABLE atelier_ai_tag_suggestion FIELDS status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_media_sidecar_parent_relation ON TABLE atelier_media_sidecar FIELDS parent_asset_id, relation_kind, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_media_sidecar_sidecar ON TABLE atelier_media_sidecar FIELDS sidecar_asset_id;
DEFINE INDEX OVERWRITE idx_atelier_media_sidecar_gallery_hidden ON TABLE atelier_media_sidecar FIELDS sidecar_asset_id;
DEFINE INDEX OVERWRITE idx_atelier_filesystem_health_finding_check ON TABLE atelier_filesystem_health_finding FIELDS check_id, finding_kind;
DEFINE INDEX OVERWRITE idx_atelier_filesystem_health_finding_target ON TABLE atelier_filesystem_health_finding FIELDS target_type, target_id;
DEFINE INDEX OVERWRITE idx_atelier_image_import_source ON TABLE atelier_image_import_request FIELDS source_kind, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_image_import_url_hash ON TABLE atelier_image_import_request FIELDS source_url_hash;
DEFINE INDEX OVERWRITE idx_atelier_image_import_asset ON TABLE atelier_image_import_request FIELDS asset_id;
DEFINE INDEX OVERWRITE idx_atelier_media_source_provenance_url ON TABLE atelier_media_source_provenance_ref FIELDS source_url_ref;
DEFINE INDEX OVERWRITE idx_atelier_media_source_provenance_path ON TABLE atelier_media_source_provenance_ref FIELDS source_path_ref;
DEFINE INDEX OVERWRITE idx_atelier_media_source_provenance_contact_sheet ON TABLE atelier_media_source_provenance_ref FIELDS contact_sheet_ref;
DEFINE INDEX OVERWRITE idx_atelier_media_source_provenance_task ON TABLE atelier_media_source_provenance_ref FIELDS task_ref;
DEFINE INDEX OVERWRITE idx_atelier_media_source_provenance_run ON TABLE atelier_media_source_provenance_ref FIELDS run_ref;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_mode_status ON TABLE atelier_intake_batch FIELDS mode, status, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_source_ref ON TABLE atelier_intake_batch FIELDS source_ref;
DEFINE INDEX OVERWRITE idx_atelier_intake_rejection_audit_batch ON TABLE atelier_intake_item_rejection_audit FIELDS batch_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_intake_rejection_audit_item ON TABLE atelier_intake_item_rejection_audit FIELDS item_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_profile_mode ON TABLE atelier_intake_batch FIELDS profile_mode, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_target_character ON TABLE atelier_intake_batch FIELDS target_character_id;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_target_sheet ON TABLE atelier_intake_batch FIELDS target_sheet_version_id;
DEFINE INDEX OVERWRITE idx_atelier_intake_batch_target_collection ON TABLE atelier_intake_batch FIELDS target_collection_id;
DEFINE INDEX OVERWRITE idx_atelier_export_intake_link_export ON TABLE atelier_export_intake_link FIELDS export_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_export_intake_link_batch ON TABLE atelier_export_intake_link FIELDS batch_id;
DEFINE INDEX OVERWRITE idx_atelier_export_intake_link_item ON TABLE atelier_export_intake_link FIELDS item_id;
DEFINE INDEX OVERWRITE idx_atelier_export_intake_link_targets ON TABLE atelier_export_intake_link FIELDS target_character_id, target_sheet_version_id, target_collection_id;
DEFINE INDEX OVERWRITE idx_atelier_media_asset_tag_tag ON TABLE atelier_media_asset_tag FIELDS tag_id, asset_id;
DEFINE INDEX OVERWRITE idx_atelier_collection_metadata_application_collection ON TABLE atelier_collection_metadata_application FIELDS collection_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_contact_sheet_svg_artifact_sheet ON TABLE atelier_contact_sheet_svg_artifact FIELDS sheet_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_contact_sheet_raster_export_plan_sheet ON TABLE atelier_contact_sheet_raster_export_plan FIELDS sheet_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_character_document_character ON TABLE atelier_character_document FIELDS character_internal_id, doc_type, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_character_document_version_document ON TABLE atelier_character_document_version FIELDS document_id, version_seq;
DEFINE INDEX OVERWRITE idx_atelier_story_card_story ON TABLE atelier_story_card FIELDS story_document_id, seq;
DEFINE INDEX OVERWRITE idx_atelier_story_beat_story ON TABLE atelier_story_beat FIELDS story_document_id, seq;
DEFINE INDEX OVERWRITE idx_atelier_story_beat_card ON TABLE atelier_story_beat FIELDS card_id, seq;

DEFINE EVENT OVERWRITE cascade_atelier_source_evidence_record ON TABLE atelier_source_evidence_record
    WHEN $event = 'DELETE'
    THEN { DELETE atelier_anchor_verification_record WHERE matrix_id = $before.matrix_id AND source_id = $before.source_id; };

-- 0130-0142 ProjectKnowledgeIndex source-forward projection (WP-KERNEL-009
-- PostgresEventLedgerCore band). PostgreSQL-only artifacts are intentionally
-- omitted and recorded here rather than approximated: identifier-format regex
-- CHECKs, multi-branch cross-column shape CHECKs, and deferred CONSTRAINT
-- TRIGGERs (edge/claim/passage "must carry >= 1 evidence row" commit guards)
-- have no SurrealDB schema equivalent. Partial UNIQUE indexes are preserved
-- through stored discriminator fields, exactly as `role_mailbox_claim_lease`
-- does; partial NON-unique index predicates are dropped and the same lookup
-- columns retained. Every table below is transcribed from the migration file
-- named in its section comment.

-- 0061_atelier_character_scripts.sql
DEFINE TABLE OVERWRITE atelier_character_script SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE script_id ON TABLE atelier_character_script TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_character_script TYPE record<atelier_character> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE script_name ON TABLE atelier_character_script TYPE string;
DEFINE FIELD OVERWRITE script_body_raw_text ON TABLE atelier_character_script TYPE string;
DEFINE FIELD OVERWRITE provenance_refs_json ON TABLE atelier_character_script TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE provenance_refs_json.* ON TABLE atelier_character_script TYPE any;
DEFINE FIELD OVERWRITE usage_refs_json ON TABLE atelier_character_script TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE usage_refs_json.* ON TABLE atelier_character_script TYPE any;
DEFINE FIELD OVERWRITE authority_mode ON TABLE atelier_character_script TYPE 'data_only' DEFAULT 'data_only';
DEFINE FIELD OVERWRITE hidden_executable_authority ON TABLE atelier_character_script TYPE bool DEFAULT false ASSERT $value = false;
DEFINE FIELD OVERWRITE created_by ON TABLE atelier_character_script TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_character_script TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_character_script TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_character_script ON TABLE atelier_character_script FIELDS script_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_character_script_character ON TABLE atelier_character_script FIELDS character_internal_id, updated_at_utc, script_id;

-- 0064_atelier_bracket_links.sql
DEFINE TABLE OVERWRITE atelier_bracket_link_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE link_id ON TABLE atelier_bracket_link_projection TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE source_document_id ON TABLE atelier_bracket_link_projection TYPE record<atelier_character_document> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_version_id ON TABLE atelier_bracket_link_projection TYPE record<atelier_character_document_version> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_doc_type ON TABLE atelier_bracket_link_projection TYPE 'note' | 'story' | 'moodboard';
DEFINE FIELD OVERWRITE seq ON TABLE atelier_bracket_link_projection TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE raw_marker ON TABLE atelier_bracket_link_projection TYPE string ASSERT string::trim($value) = $value AND string::starts_with($value, '[[') AND string::ends_with($value, ']]');
DEFINE FIELD OVERWRITE target_kind ON TABLE atelier_bracket_link_projection TYPE 'character' | 'document' | 'story' | 'moodboard' | 'image';
DEFINE FIELD OVERWRITE target_id ON TABLE atelier_bracket_link_projection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE target_label ON TABLE atelier_bracket_link_projection TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_bracket_link_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_bracket_link_projection ON TABLE atelier_bracket_link_projection FIELDS link_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_bracket_link_projection_1 ON TABLE atelier_bracket_link_projection FIELDS source_document_id, seq UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_bracket_link_source ON TABLE atelier_bracket_link_projection FIELDS source_document_id, seq;
DEFINE INDEX OVERWRITE idx_atelier_bracket_link_target ON TABLE atelier_bracket_link_projection FIELDS target_kind, target_id, source_document_id, seq;

-- 0072_atelier_moodboard_schema_layer_model.sql. The PostgreSQL BEFORE-INSERT
-- guard trigger (document type + version ownership) is projected as the
-- document/version record links plus the SCHEMAFULL field contract; the
-- cross-row plpgsql assertions are enforced in the storage layer.
DEFINE TABLE OVERWRITE atelier_moodboard SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE snapshot_id ON TABLE atelier_moodboard TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE document_id ON TABLE atelier_moodboard TYPE record<atelier_character_document> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_version_id ON TABLE atelier_moodboard TYPE record<atelier_character_document_version> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE schema_id ON TABLE atelier_moodboard TYPE 'hsk.atelier.moodboard@1';
DEFINE FIELD OVERWRITE schema_version ON TABLE atelier_moodboard TYPE int ASSERT $value = 1;
DEFINE FIELD OVERWRITE raw_json_text ON TABLE atelier_moodboard TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE moodboard_json ON TABLE atelier_moodboard TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE moodboard_json.* ON TABLE atelier_moodboard TYPE any;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE atelier_moodboard TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE author ON TABLE atelier_moodboard TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_moodboard TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_moodboard ON TABLE atelier_moodboard FIELDS snapshot_id UNIQUE;
DEFINE INDEX OVERWRITE ux_atelier_moodboard_document_version_hash ON TABLE atelier_moodboard FIELDS document_id, document_version_id, content_sha256 UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_moodboard_document_latest ON TABLE atelier_moodboard FIELDS document_id, created_at_utc, snapshot_id;

-- 0074_atelier_moodboard_operations_exports.sql. The PostgreSQL guard triggers
-- (snapshot/document/version agreement, payload-hash recomputation, receipt
-- field mirroring, `.GOV` rejection) are storage-layer invariants; the durable
-- shape below is the transcribed table contract.
DEFINE TABLE OVERWRITE atelier_moodboard_operation_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE operation_id ON TABLE atelier_moodboard_operation_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE snapshot_id ON TABLE atelier_moodboard_operation_receipt TYPE record<atelier_moodboard> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_id ON TABLE atelier_moodboard_operation_receipt TYPE record<atelier_character_document> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_version_id ON TABLE atelier_moodboard_operation_receipt TYPE record<atelier_character_document_version> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE operation_kind ON TABLE atelier_moodboard_operation_receipt TYPE 'layer_reordered' | 'element_moved' | 'element_updated' | 'layer_visibility_changed' | 'style_updated' | 'history_annotated';
DEFINE FIELD OVERWRITE operation_payload ON TABLE atelier_moodboard_operation_receipt TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE operation_payload.* ON TABLE atelier_moodboard_operation_receipt TYPE any;
DEFINE FIELD OVERWRITE operation_payload_sha256 ON TABLE atelier_moodboard_operation_receipt TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE receipt_json ON TABLE atelier_moodboard_operation_receipt TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE receipt_json.* ON TABLE atelier_moodboard_operation_receipt TYPE any;
DEFINE FIELD OVERWRITE actor ON TABLE atelier_moodboard_operation_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_moodboard_operation_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_moodboard_operation_receipt ON TABLE atelier_moodboard_operation_receipt FIELDS operation_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_moodboard_operation_receipt_snapshot ON TABLE atelier_moodboard_operation_receipt FIELDS snapshot_id, created_at_utc, operation_id;

DEFINE TABLE OVERWRITE atelier_moodboard_export_request SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE export_id ON TABLE atelier_moodboard_export_request TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE snapshot_id ON TABLE atelier_moodboard_export_request TYPE record<atelier_moodboard> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_id ON TABLE atelier_moodboard_export_request TYPE record<atelier_character_document> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_version_id ON TABLE atelier_moodboard_export_request TYPE record<atelier_character_document_version> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE format ON TABLE atelier_moodboard_export_request TYPE 'png' | 'pdf';
DEFINE FIELD OVERWRITE status ON TABLE atelier_moodboard_export_request TYPE 'planned' DEFAULT 'planned';
DEFINE FIELD OVERWRITE label ON TABLE atelier_moodboard_export_request TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE manifest_json ON TABLE atelier_moodboard_export_request TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE manifest_json.* ON TABLE atelier_moodboard_export_request TYPE any;
DEFINE FIELD OVERWRITE receipt_json ON TABLE atelier_moodboard_export_request TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE receipt_json.* ON TABLE atelier_moodboard_export_request TYPE any;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_moodboard_export_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_moodboard_export_request TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_moodboard_export_request ON TABLE atelier_moodboard_export_request FIELDS export_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_moodboard_export_request_1 ON TABLE atelier_moodboard_export_request FIELDS snapshot_id, format UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_moodboard_export_request_snapshot ON TABLE atelier_moodboard_export_request FIELDS snapshot_id, created_at_utc, format;

-- 0082_atelier_character_relationships.sql
DEFINE TABLE OVERWRITE atelier_character_relationship SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE relationship_id ON TABLE atelier_character_relationship TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE source_character_id ON TABLE atelier_character_relationship TYPE record<atelier_character> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE target_character_id ON TABLE atelier_character_relationship TYPE record<atelier_character> ASSERT record::exists($value) AND $value != $this.source_character_id REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE relationship_kind ON TABLE atelier_character_relationship TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE label ON TABLE atelier_character_relationship TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE notes ON TABLE atelier_character_relationship TYPE string DEFAULT '' ASSERT string::trim($value) = $value;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_character_relationship TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_character_relationship TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_character_relationship ON TABLE atelier_character_relationship FIELDS relationship_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_character_relationship_edge ON TABLE atelier_character_relationship FIELDS source_character_id, target_character_id, relationship_kind UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_character_relationship_source ON TABLE atelier_character_relationship FIELDS source_character_id, updated_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_character_relationship_target ON TABLE atelier_character_relationship FIELDS target_character_id, updated_at_utc;

DEFINE TABLE OVERWRITE atelier_character_relationship_graph_projection TYPE NORMAL AS
    SELECT relationship_id AS edge_id, source_character_id, target_character_id, relationship_kind, label, notes, updated_at_utc
    FROM atelier_character_relationship
    PERMISSIONS NONE;

-- 0083_atelier_saved_searches.sql. The read-only retrieval projection VIEW is a
-- multi-CTE PostgreSQL query over media/tag/review/collection/palette tables
-- with lateral JSON array element expansion; it has no bounded SurrealQL form
-- and is executed by the storage layer, so only the durable filter table is
-- transcribed here.
DEFINE TABLE OVERWRITE atelier_saved_search SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE saved_search_id ON TABLE atelier_saved_search TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE name ON TABLE atelier_saved_search TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE include_tags_json ON TABLE atelier_saved_search TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE include_tags_json.* ON TABLE atelier_saved_search TYPE any;
DEFINE FIELD OVERWRITE exclude_tags_json ON TABLE atelier_saved_search TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE exclude_tags_json.* ON TABLE atelier_saved_search TYPE any;
DEFINE FIELD OVERWRITE min_rating ON TABLE atelier_saved_search TYPE option<int> ASSERT $value = NONE OR ($value >= 0 AND $value <= 5);
DEFINE FIELD OVERWRITE favorite ON TABLE atelier_saved_search TYPE option<bool>;
DEFINE FIELD OVERWRITE color_hex ON TABLE atelier_saved_search TYPE option<string> ASSERT $value = NONE OR (string::starts_with($value, '#') AND string::len($value) = 7);
DEFINE FIELD OVERWRITE scope_kind ON TABLE atelier_saved_search TYPE 'all_media' | 'collection' DEFAULT 'all_media';
DEFINE FIELD OVERWRITE scope_id ON TABLE atelier_saved_search TYPE option<uuid>;
DEFINE FIELD OVERWRITE view_mode ON TABLE atelier_saved_search TYPE 'NSFW' | 'SFW' DEFAULT 'NSFW';
DEFINE FIELD OVERWRITE created_by ON TABLE atelier_saved_search TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_saved_search TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_saved_search TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_saved_search ON TABLE atelier_saved_search FIELDS saved_search_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_saved_search_name ON TABLE atelier_saved_search FIELDS name UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_saved_search_updated ON TABLE atelier_saved_search FIELDS updated_at_utc;

-- 0084_atelier_web_portfolio_exports.sql
DEFINE TABLE OVERWRITE atelier_web_portfolio_export_request SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE portfolio_export_id ON TABLE atelier_web_portfolio_export_request TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE source_collection_id ON TABLE atelier_web_portfolio_export_request TYPE record<atelier_collection> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE slug ON TABLE atelier_web_portfolio_export_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE title ON TABLE atelier_web_portfolio_export_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_web_portfolio_export_request TYPE 'pending' | 'rendered' DEFAULT 'pending';
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_web_portfolio_export_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_web_portfolio_export_request TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_web_portfolio_export_request TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_web_portfolio_export_request ON TABLE atelier_web_portfolio_export_request FIELDS portfolio_export_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_web_portfolio_export_request_slug ON TABLE atelier_web_portfolio_export_request FIELDS slug UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_web_portfolio_export_request_collection ON TABLE atelier_web_portfolio_export_request FIELDS source_collection_id, created_at_utc;

DEFINE TABLE OVERWRITE atelier_web_portfolio_export_result SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE result_id ON TABLE atelier_web_portfolio_export_result TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE portfolio_export_id ON TABLE atelier_web_portfolio_export_result TYPE record<atelier_web_portfolio_export_request> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_web_portfolio_export_result TYPE string ASSERT string::trim($value) != '' AND string::starts_with($value, 'artifact://');
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_web_portfolio_export_result TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_web_portfolio_export_result TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE manifest_json ON TABLE atelier_web_portfolio_export_result TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE manifest_json.* ON TABLE atelier_web_portfolio_export_result TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_web_portfolio_export_result TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_web_portfolio_export_result ON TABLE atelier_web_portfolio_export_result FIELDS result_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_web_portfolio_export_result_1 ON TABLE atelier_web_portfolio_export_result FIELDS portfolio_export_id, content_hash UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_web_portfolio_export_result_request ON TABLE atelier_web_portfolio_export_result FIELDS portfolio_export_id, created_at_utc;

-- 0085_atelier_backup_manifests.sql
DEFINE TABLE OVERWRITE atelier_backup_manifest SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE backup_id ON TABLE atelier_backup_manifest TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE app_version ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE spec_version ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE schema_version ON TABLE atelier_backup_manifest TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '' AND string::starts_with($value, 'artifact://');
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_backup_manifest TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE manifest_hash ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE manifest_json ON TABLE atelier_backup_manifest TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE manifest_json.* ON TABLE atelier_backup_manifest TYPE any;
DEFINE FIELD OVERWRITE created_by ON TABLE atelier_backup_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_backup_manifest TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_backup_manifest ON TABLE atelier_backup_manifest FIELDS backup_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_backup_manifest_hash ON TABLE atelier_backup_manifest FIELDS manifest_hash UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_backup_manifest_created ON TABLE atelier_backup_manifest FIELDS created_at_utc;

DEFINE TABLE OVERWRITE atelier_backup_restore_preflight SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE preflight_id ON TABLE atelier_backup_restore_preflight TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE backup_id ON TABLE atelier_backup_restore_preflight TYPE record<atelier_backup_manifest> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE current_app_version ON TABLE atelier_backup_restore_preflight TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE current_spec_version ON TABLE atelier_backup_restore_preflight TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE current_schema_version ON TABLE atelier_backup_restore_preflight TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE status ON TABLE atelier_backup_restore_preflight TYPE 'accepted' | 'refused';
DEFINE FIELD OVERWRITE refusal_reason ON TABLE atelier_backup_restore_preflight TYPE option<string> ASSERT ($this.status = 'accepted' AND $value = NONE) OR ($this.status = 'refused' AND $value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_backup_restore_preflight TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_backup_restore_preflight TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_backup_restore_preflight ON TABLE atelier_backup_restore_preflight FIELDS preflight_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_backup_restore_preflight_backup ON TABLE atelier_backup_restore_preflight FIELDS backup_id, created_at_utc;

-- 0086_atelier_state_probe_catalog.sql
DEFINE TABLE OVERWRITE atelier_state_probe_catalog_entry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE catalog_id ON TABLE atelier_state_probe_catalog_entry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE probe_id ON TABLE atelier_state_probe_catalog_entry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE surface ON TABLE atelier_state_probe_catalog_entry TYPE 'character' | 'media' | 'intake' | 'collection' | 'docs' | 'moodboard' | 'pose' | 'comfyui_job' | 'session' | 'errors';
DEFINE FIELD OVERWRITE probe_label ON TABLE atelier_state_probe_catalog_entry TYPE string;
DEFINE FIELD OVERWRITE read_model ON TABLE atelier_state_probe_catalog_entry TYPE 'postgres_event_ledger_projection';
DEFINE FIELD OVERWRITE inspection_phase ON TABLE atelier_state_probe_catalog_entry TYPE 'pre_visual_inspection';
DEFINE FIELD OVERWRITE required_before_visual_inspection ON TABLE atelier_state_probe_catalog_entry TYPE bool ASSERT $value = true;
DEFINE FIELD OVERWRITE status ON TABLE atelier_state_probe_catalog_entry TYPE 'ready';
DEFINE FIELD OVERWRITE probe_fields ON TABLE atelier_state_probe_catalog_entry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE probe_fields.* ON TABLE atelier_state_probe_catalog_entry TYPE any;
DEFINE FIELD OVERWRITE evidence_refs ON TABLE atelier_state_probe_catalog_entry TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE evidence_refs.* ON TABLE atelier_state_probe_catalog_entry TYPE any;
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_state_probe_catalog_entry TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_state_probe_catalog_entry ON TABLE atelier_state_probe_catalog_entry FIELDS catalog_id, probe_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_state_probe_catalog_surface ON TABLE atelier_state_probe_catalog_entry FIELDS catalog_id, surface UNIQUE;

-- 0087_atelier_action_receipts.sql
DEFINE TABLE OVERWRITE atelier_action_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_action_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE action_id ON TABLE atelier_action_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE params_sha256 ON TABLE atelier_action_receipt TYPE string ASSERT string::starts_with($value, 'sha256:') AND string::len($value) = 71;
DEFINE FIELD OVERWRITE actor_kind ON TABLE atelier_action_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE atelier_action_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_action_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE started_at_utc ON TABLE atelier_action_receipt TYPE datetime;
DEFINE FIELD OVERWRITE completed_at_utc ON TABLE atelier_action_receipt TYPE datetime ASSERT $value >= $this.started_at_utc;
DEFINE FIELD OVERWRITE status ON TABLE atelier_action_receipt TYPE 'succeeded' | 'failed' | 'rejected';
DEFINE FIELD OVERWRITE target_refs ON TABLE atelier_action_receipt TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE target_refs.* ON TABLE atelier_action_receipt TYPE any;
DEFINE FIELD OVERWRITE evidence_refs ON TABLE atelier_action_receipt TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE evidence_refs.* ON TABLE atelier_action_receipt TYPE any;
DEFINE FIELD OVERWRITE result_refs ON TABLE atelier_action_receipt TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE result_refs.* ON TABLE atelier_action_receipt TYPE any;
DEFINE FIELD OVERWRITE error_class ON TABLE atelier_action_receipt TYPE option<string> ASSERT $this.status = 'succeeded' OR ($value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE recovery_hint ON TABLE atelier_action_receipt TYPE option<string> ASSERT $this.status = 'succeeded' OR ($value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_action_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_action_receipt ON TABLE atelier_action_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_action_receipt_action ON TABLE atelier_action_receipt FIELDS action_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_action_receipt_session ON TABLE atelier_action_receipt FIELDS session_id, created_at_utc;

-- 0089_atelier_reset_orphan_adoption.sql
DEFINE TABLE OVERWRITE atelier_reset_operation SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE reset_id ON TABLE atelier_reset_operation TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE mode ON TABLE atelier_reset_operation TYPE 'preferences_only' | 'full_preserve_original_media';
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_reset_operation TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE reason ON TABLE atelier_reset_operation TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE preferences_deleted_count ON TABLE atelier_reset_operation TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE original_media_preserved_count ON TABLE atelier_reset_operation TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE orphan_manifest_id ON TABLE atelier_reset_operation TYPE option<uuid>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_reset_operation TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_reset_operation ON TABLE atelier_reset_operation FIELDS reset_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_reset_operation_created ON TABLE atelier_reset_operation FIELDS created_at_utc;

DEFINE TABLE OVERWRITE atelier_orphan_manifest SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE manifest_id ON TABLE atelier_orphan_manifest TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE reset_id ON TABLE atelier_orphan_manifest TYPE record<atelier_reset_operation> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE manifest_json ON TABLE atelier_orphan_manifest TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE manifest_json.* ON TABLE atelier_orphan_manifest TYPE any;
DEFINE FIELD OVERWRITE item_count ON TABLE atelier_orphan_manifest TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_orphan_manifest TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_orphan_manifest ON TABLE atelier_orphan_manifest FIELDS manifest_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_orphan_manifest_reset ON TABLE atelier_orphan_manifest FIELDS reset_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_orphan_manifest_item SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE manifest_item_id ON TABLE atelier_orphan_manifest_item TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE manifest_id ON TABLE atelier_orphan_manifest_item TYPE record<atelier_orphan_manifest> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_orphan_manifest_item TYPE record<atelier_media_asset> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_orphan_manifest_item TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_orphan_manifest_item TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE mime ON TABLE atelier_orphan_manifest_item TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_orphan_manifest_item TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE retention_class ON TABLE atelier_orphan_manifest_item TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE adoption_status ON TABLE atelier_orphan_manifest_item TYPE 'orphaned' | 'adopted' DEFAULT 'orphaned';
DEFINE FIELD OVERWRITE adopted_batch_id ON TABLE atelier_orphan_manifest_item TYPE option<record<atelier_intake_batch>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE adopted_item_id ON TABLE atelier_orphan_manifest_item TYPE option<record<atelier_intake_item>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE adopted_by ON TABLE atelier_orphan_manifest_item TYPE option<string> ASSERT ($this.adoption_status = 'orphaned' AND $value = NONE) OR ($this.adoption_status = 'adopted' AND $value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE adopted_at_utc ON TABLE atelier_orphan_manifest_item TYPE option<datetime> ASSERT ($this.adoption_status = 'orphaned' AND $value = NONE) OR ($this.adoption_status = 'adopted' AND $value != NONE);
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_orphan_manifest_item TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_orphan_manifest_item ON TABLE atelier_orphan_manifest_item FIELDS manifest_item_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_orphan_manifest_item_asset ON TABLE atelier_orphan_manifest_item FIELDS manifest_id, asset_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_orphan_manifest_item_manifest_status ON TABLE atelier_orphan_manifest_item FIELDS manifest_id, adoption_status;
DEFINE INDEX OVERWRITE idx_atelier_orphan_manifest_item_asset ON TABLE atelier_orphan_manifest_item FIELDS asset_id;

-- 0090_atelier_pose_sidecars.sql
DEFINE TABLE OVERWRITE atelier_pose_sidecar SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE sidecar_id ON TABLE atelier_pose_sidecar TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE rig_id ON TABLE atelier_pose_sidecar TYPE record<atelier_pose_rig> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_asset_id ON TABLE atelier_pose_sidecar TYPE option<record<atelier_media_asset>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_pose_sidecar TYPE string DEFAULT '' ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE kind ON TABLE atelier_pose_sidecar TYPE 'openpose_json' | 'openpose_png' | 'conditioning_png';
DEFINE FIELD OVERWRITE role ON TABLE atelier_pose_sidecar TYPE string ASSERT $value = $this.kind;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_pose_sidecar TYPE string ASSERT string::starts_with($value, 'artifact://.handshake/artifacts/') AND string::ends_with($value, '/payload');
DEFINE FIELD OVERWRITE manifest_ref ON TABLE atelier_pose_sidecar TYPE string ASSERT string::starts_with($value, 'artifact://.handshake/artifacts/') AND string::ends_with($value, '/artifact.json');
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_pose_sidecar TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_pose_sidecar TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE mime ON TABLE atelier_pose_sidecar TYPE string ASSERT ($this.kind = 'openpose_json' AND $value = 'application/json') OR ($this.kind != 'openpose_json' AND $value = 'image/png');
DEFINE FIELD OVERWRITE width ON TABLE atelier_pose_sidecar TYPE int DEFAULT 0 ASSERT $value > 0;
DEFINE FIELD OVERWRITE height ON TABLE atelier_pose_sidecar TYPE int DEFAULT 0 ASSERT $value > 0;
DEFINE FIELD OVERWRITE status ON TABLE atelier_pose_sidecar TYPE 'rendered' | 'failed' DEFAULT 'rendered';
DEFINE FIELD OVERWRITE error_message ON TABLE atelier_pose_sidecar TYPE option<string> ASSERT ($this.status = 'rendered' AND $value = NONE) OR ($this.status = 'failed' AND $value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_sidecar TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_sidecar ON TABLE atelier_pose_sidecar FIELDS sidecar_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_pose_sidecar_1 ON TABLE atelier_pose_sidecar FIELDS rig_id, kind UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_pose_sidecar_rig ON TABLE atelier_pose_sidecar FIELDS rig_id, created_at_utc;

-- 0092_atelier_pose_context_state.sql. The source `state_seq BIGSERIAL UNIQUE`
-- monotonic counter is projected onto a Handshake-managed Surreal sequence, the
-- same pattern the 0018 kernel event ledger uses.
DEFINE SEQUENCE IF NOT EXISTS atelier_pose_context_state_seq BATCH 1 START 1;

DEFINE TABLE OVERWRITE atelier_pose_context_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE context_id ON TABLE atelier_pose_context_state TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE state_seq ON TABLE atelier_pose_context_state TYPE int DEFAULT sequence::nextval('atelier_pose_context_state_seq') ASSERT $value > 0;
DEFINE FIELD OVERWRITE workspace_ref ON TABLE atelier_pose_context_state TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE kind ON TABLE atelier_pose_context_state TYPE 'blank' | 'single_image' | 'character_linked' | 'collection_linked';
DEFINE FIELD OVERWRITE source_asset_id ON TABLE atelier_pose_context_state TYPE option<record<atelier_media_asset>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_pose_context_state TYPE option<record<atelier_character>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE collection_id ON TABLE atelier_pose_context_state TYPE option<record<atelier_collection>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE selected_rig_id ON TABLE atelier_pose_context_state TYPE option<record<atelier_pose_rig>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_pose_context_state TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_context_state TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_context_state ON TABLE atelier_pose_context_state FIELDS context_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_pose_context_state_seq ON TABLE atelier_pose_context_state FIELDS state_seq UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_pose_context_state_workspace_seq ON TABLE atelier_pose_context_state FIELDS workspace_ref, state_seq;
DEFINE INDEX OVERWRITE idx_atelier_pose_context_state_links ON TABLE atelier_pose_context_state FIELDS character_internal_id, collection_id, selected_rig_id;

-- 0093_atelier_pose_workspace_rig_state.sql. The trailing ALTER re-adds
-- `session_ref` and `open`, which the CREATE above already declares, so the
-- forward end state is the single field set below. The two PostgreSQL partial
-- unique indexes (`WHERE active AND open`, `WHERE open`) have no partial-index
-- form here and are projected as the storage-layer single-active-rig invariant.
DEFINE TABLE OVERWRITE atelier_pose_workspace_rig_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_ref ON TABLE atelier_pose_workspace_rig_state TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE session_ref ON TABLE atelier_pose_workspace_rig_state TYPE string DEFAULT 'pose-session://default' ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE rig_id ON TABLE atelier_pose_workspace_rig_state TYPE record<atelier_pose_rig> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE open ON TABLE atelier_pose_workspace_rig_state TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE sort_order ON TABLE atelier_pose_workspace_rig_state TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE active ON TABLE atelier_pose_workspace_rig_state TYPE bool DEFAULT false ASSERT $this.open OR $value = false;
DEFINE FIELD OVERWRITE dirty_calibration ON TABLE atelier_pose_workspace_rig_state TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE panel_state ON TABLE atelier_pose_workspace_rig_state TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE panel_state.* ON TABLE atelier_pose_workspace_rig_state TYPE any;
DEFINE FIELD OVERWRITE requested_by ON TABLE atelier_pose_workspace_rig_state TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_workspace_rig_state TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_pose_workspace_rig_state TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_workspace_rig_state ON TABLE atelier_pose_workspace_rig_state FIELDS workspace_ref, session_ref, rig_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_pose_workspace_rig_state_order ON TABLE atelier_pose_workspace_rig_state FIELDS workspace_ref, session_ref, open, sort_order, rig_id;

-- 0100_atelier_identity_crop_artifact.sql
DEFINE TABLE OVERWRITE atelier_identity_crop_artifact SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE crop_id ON TABLE atelier_identity_crop_artifact TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE profile_id ON TABLE atelier_identity_crop_artifact TYPE record<atelier_identity_profile> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE profile_version ON TABLE atelier_identity_crop_artifact TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE character_internal_id ON TABLE atelier_identity_crop_artifact TYPE record<atelier_character> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_identity_crop_artifact TYPE string;
DEFINE FIELD OVERWRITE crop_box ON TABLE atelier_identity_crop_artifact TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE crop_box.* ON TABLE atelier_identity_crop_artifact TYPE any;
DEFINE FIELD OVERWRITE landmarks ON TABLE atelier_identity_crop_artifact TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE landmarks.* ON TABLE atelier_identity_crop_artifact TYPE any;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_identity_crop_artifact TYPE string;
DEFINE FIELD OVERWRITE manifest_ref ON TABLE atelier_identity_crop_artifact TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_identity_crop_artifact TYPE string;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_identity_crop_artifact TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE mime ON TABLE atelier_identity_crop_artifact TYPE 'image/png';
DEFINE FIELD OVERWRITE width ON TABLE atelier_identity_crop_artifact TYPE int ASSERT $value = 512;
DEFINE FIELD OVERWRITE height ON TABLE atelier_identity_crop_artifact TYPE int ASSERT $value = 512;
DEFINE FIELD OVERWRITE manifest ON TABLE atelier_identity_crop_artifact TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE manifest.* ON TABLE atelier_identity_crop_artifact TYPE any;
DEFINE FIELD OVERWRITE created_by ON TABLE atelier_identity_crop_artifact TYPE string;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_identity_crop_artifact TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_identity_crop_artifact ON TABLE atelier_identity_crop_artifact FIELDS crop_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_identity_crop_artifact_profile_version_hash ON TABLE atelier_identity_crop_artifact FIELDS profile_id, profile_version, content_hash UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_identity_crop_artifact_profile ON TABLE atelier_identity_crop_artifact FIELDS profile_id, profile_version, created_at_utc;

-- 0102_atelier_comfy_workflow_receipt.sql
DEFINE TABLE OVERWRITE atelier_comfy_workflow_receipt SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE atelier_comfy_workflow_receipt TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE system_id ON TABLE atelier_comfy_workflow_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_workflow_receipt TYPE uuid;
DEFINE FIELD OVERWRITE workflow_spec_ref ON TABLE atelier_comfy_workflow_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workflow_json_ref ON TABLE atelier_comfy_workflow_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE prompt_ref ON TABLE atelier_comfy_workflow_receipt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE all_refs ON TABLE atelier_comfy_workflow_receipt TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE all_refs.* ON TABLE atelier_comfy_workflow_receipt TYPE any;
DEFINE FIELD OVERWRITE outputs ON TABLE atelier_comfy_workflow_receipt TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE outputs.* ON TABLE atelier_comfy_workflow_receipt TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_comfy_workflow_receipt TYPE 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
DEFINE FIELD OVERWRITE error_ref ON TABLE atelier_comfy_workflow_receipt TYPE option<string> ASSERT ($this.status = 'failed' AND $value != NONE AND string::trim($value) != '') OR ($this.status != 'failed' AND ($value = NONE OR string::trim($value) != ''));
DEFINE FIELD OVERWRITE evidence ON TABLE atelier_comfy_workflow_receipt TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE evidence.* ON TABLE atelier_comfy_workflow_receipt TYPE any;
DEFINE FIELD OVERWRITE receipt_json ON TABLE atelier_comfy_workflow_receipt TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE receipt_json.* ON TABLE atelier_comfy_workflow_receipt TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_comfy_workflow_receipt TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_comfy_workflow_receipt TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_workflow_receipt ON TABLE atelier_comfy_workflow_receipt FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_workflow_receipt_run ON TABLE atelier_comfy_workflow_receipt FIELDS workflow_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_comfy_workflow_receipt_status ON TABLE atelier_comfy_workflow_receipt FIELDS status, updated_at_utc;

-- 0103_atelier_comfy_output_registration_failure.sql
DEFINE TABLE OVERWRITE atelier_comfy_output_registration_failure SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE failure_id ON TABLE atelier_comfy_output_registration_failure TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_output_registration_failure TYPE uuid;
DEFINE FIELD OVERWRITE node_execution_id ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE attempted_registration_id ON TABLE atelier_comfy_output_registration_failure TYPE option<uuid>;
DEFINE FIELD OVERWRITE source_node_instance_id ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE source_output_slot ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE media_kind ON TABLE atelier_comfy_output_registration_failure TYPE 'image' | 'mask' | 'latent_preview' | 'video' | 'sidecar_json';
DEFINE FIELD OVERWRITE mime ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE artifact_manifest_ref ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE routing_intent ON TABLE atelier_comfy_output_registration_failure TYPE 'artifact' | 'sidecar' | 'transient';
DEFINE FIELD OVERWRITE parent_artifact_ref ON TABLE atelier_comfy_output_registration_failure TYPE option<string> ASSERT ($value = NONE AND $this.routing_intent != 'sidecar') OR ($value != NONE AND string::trim($value) != '');
DEFINE FIELD OVERWRITE prompt_json_ref ON TABLE atelier_comfy_output_registration_failure TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE graph_hash ON TABLE atelier_comfy_output_registration_failure TYPE option<string>;
DEFINE FIELD OVERWRITE seed ON TABLE atelier_comfy_output_registration_failure TYPE option<int>;
DEFINE FIELD OVERWRITE workflow_input_metadata ON TABLE atelier_comfy_output_registration_failure TYPE object FLEXIBLE DEFAULT { identity: {} };
DEFINE FIELD OVERWRITE workflow_input_metadata.* ON TABLE atelier_comfy_output_registration_failure TYPE any;
DEFINE FIELD OVERWRITE failure_stage ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE failure_reason ON TABLE atelier_comfy_output_registration_failure TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE evidence ON TABLE atelier_comfy_output_registration_failure TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE evidence.* ON TABLE atelier_comfy_output_registration_failure TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_comfy_output_registration_failure TYPE 'retryable' | 'registered' DEFAULT 'retryable';
DEFINE FIELD OVERWRITE retry_count ON TABLE atelier_comfy_output_registration_failure TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE resolved_intake_output_id ON TABLE atelier_comfy_output_registration_failure TYPE option<record<atelier_comfy_intake_output>> ASSERT ($value = NONE AND $this.status = 'retryable') OR (record::exists($value) AND $this.status = 'registered') REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_comfy_output_registration_failure TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_comfy_output_registration_failure TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_output_registration_failure ON TABLE atelier_comfy_output_registration_failure FIELDS failure_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_output_registration_failure_dedup ON TABLE atelier_comfy_output_registration_failure FIELDS workflow_run_id, content_hash, failure_stage UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_comfy_output_registration_failure_run_status ON TABLE atelier_comfy_output_registration_failure FIELDS workflow_run_id, status, updated_at_utc;

-- 0105_atelier_pose_deferred_feature.sql
DEFINE TABLE OVERWRITE atelier_pose_deferred_feature SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE feature_id ON TABLE atelier_pose_deferred_feature TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE feature_kind ON TABLE atelier_pose_deferred_feature TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_pose_deferred_feature TYPE 'PLANNED' | 'DEFERRED' | 'BLOCKED';
DEFINE FIELD OVERWRITE feature_label ON TABLE atelier_pose_deferred_feature TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE deferral_reason ON TABLE atelier_pose_deferred_feature TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE carry_forward ON TABLE atelier_pose_deferred_feature TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_pose_deferred_feature TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_pose_deferred_feature TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_pose_deferred_feature ON TABLE atelier_pose_deferred_feature FIELDS feature_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_pose_deferred_feature_kind ON TABLE atelier_pose_deferred_feature FIELDS feature_kind, status, created_at_utc;

-- 0106_atelier_comfy_workflow_spec.sql
DEFINE TABLE OVERWRITE atelier_comfy_workflow_spec SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE spec_id ON TABLE atelier_comfy_workflow_spec TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_kind ON TABLE atelier_comfy_workflow_spec TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE spec_version ON TABLE atelier_comfy_workflow_spec TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE spec_hash ON TABLE atelier_comfy_workflow_spec TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE handler_id ON TABLE atelier_comfy_workflow_spec TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE compatibility_pin ON TABLE atelier_comfy_workflow_spec TYPE option<string>;
DEFINE FIELD OVERWRITE spec_json ON TABLE atelier_comfy_workflow_spec TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE spec_json.* ON TABLE atelier_comfy_workflow_spec TYPE any;
DEFINE FIELD OVERWRITE source_ref ON TABLE atelier_comfy_workflow_spec TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_comfy_workflow_spec TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_comfy_workflow_spec TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_workflow_spec ON TABLE atelier_comfy_workflow_spec FIELDS spec_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_workflow_spec_kind_version ON TABLE atelier_comfy_workflow_spec FIELDS workflow_kind, spec_version UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_workflow_spec_hash ON TABLE atelier_comfy_workflow_spec FIELDS spec_hash UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_comfy_workflow_spec_kind ON TABLE atelier_comfy_workflow_spec FIELDS workflow_kind, created_at_utc;

-- 0107_atelier_comfy_version_metadata.sql
DEFINE TABLE OVERWRITE atelier_comfy_version_metadata SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE version_metadata_id ON TABLE atelier_comfy_version_metadata TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_version_metadata TYPE uuid;
DEFINE FIELD OVERWRITE spec_id ON TABLE atelier_comfy_version_metadata TYPE option<record<atelier_comfy_workflow_spec>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE pose_model_asset_version ON TABLE atelier_comfy_version_metadata TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE image_tool_version ON TABLE atelier_comfy_version_metadata TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE comfy_model_version ON TABLE atelier_comfy_version_metadata TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE preflight_evidence ON TABLE atelier_comfy_version_metadata TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE preflight_evidence.* ON TABLE atelier_comfy_version_metadata TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_comfy_version_metadata TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_comfy_version_metadata TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_version_metadata ON TABLE atelier_comfy_version_metadata FIELDS version_metadata_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_version_metadata_run ON TABLE atelier_comfy_version_metadata FIELDS workflow_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_comfy_version_metadata_spec ON TABLE atelier_comfy_version_metadata FIELDS spec_id;

-- 0108_atelier_comfy_job_queue.sql
DEFINE TABLE OVERWRITE atelier_comfy_job SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE atelier_comfy_job TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_job TYPE uuid;
DEFINE FIELD OVERWRITE spec_id ON TABLE atelier_comfy_job TYPE option<record<atelier_comfy_workflow_spec>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE request_json ON TABLE atelier_comfy_job TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE request_json.* ON TABLE atelier_comfy_job TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_comfy_job TYPE 'QUEUED' | 'RUNNING' | 'COMPLETED' | 'FAILED' | 'CANCELLED' | 'TIMED_OUT' DEFAULT 'QUEUED';
DEFINE FIELD OVERWRITE partial_evidence_ref ON TABLE atelier_comfy_job TYPE option<string>;
DEFINE FIELD OVERWRITE error_reason ON TABLE atelier_comfy_job TYPE option<string>;
DEFINE FIELD OVERWRITE queued_at ON TABLE atelier_comfy_job TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE started_at ON TABLE atelier_comfy_job TYPE option<datetime>;
DEFINE FIELD OVERWRITE finished_at ON TABLE atelier_comfy_job TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_atelier_comfy_job ON TABLE atelier_comfy_job FIELDS job_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_job_run ON TABLE atelier_comfy_job FIELDS workflow_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_comfy_job_status ON TABLE atelier_comfy_job FIELDS status, queued_at;
DEFINE INDEX OVERWRITE idx_atelier_comfy_job_spec ON TABLE atelier_comfy_job FIELDS spec_id;

-- 0109_atelier_comfy_diagnostic_bundle.sql
DEFINE TABLE OVERWRITE atelier_comfy_diagnostic_bundle SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE bundle_id ON TABLE atelier_comfy_diagnostic_bundle TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE workflow_run_id ON TABLE atelier_comfy_diagnostic_bundle TYPE uuid;
DEFINE FIELD OVERWRITE request_json ON TABLE atelier_comfy_diagnostic_bundle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE request_json.* ON TABLE atelier_comfy_diagnostic_bundle TYPE any;
DEFINE FIELD OVERWRITE refs_json ON TABLE atelier_comfy_diagnostic_bundle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE refs_json.* ON TABLE atelier_comfy_diagnostic_bundle TYPE any;
DEFINE FIELD OVERWRITE versions_json ON TABLE atelier_comfy_diagnostic_bundle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE versions_json.* ON TABLE atelier_comfy_diagnostic_bundle TYPE any;
DEFINE FIELD OVERWRITE logs_ref ON TABLE atelier_comfy_diagnostic_bundle TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE artifacts_json ON TABLE atelier_comfy_diagnostic_bundle TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE artifacts_json.* ON TABLE atelier_comfy_diagnostic_bundle TYPE any;
DEFINE FIELD OVERWRITE error_taxonomy ON TABLE atelier_comfy_diagnostic_bundle TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_comfy_diagnostic_bundle TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_comfy_diagnostic_bundle ON TABLE atelier_comfy_diagnostic_bundle FIELDS bundle_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_comfy_diagnostic_bundle_run ON TABLE atelier_comfy_diagnostic_bundle FIELDS workflow_run_id UNIQUE;

-- 0111_atelier_diagnostics_validation_matrix.sql
DEFINE TABLE OVERWRITE atelier_diagnostics_validation_matrix SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE row_id ON TABLE atelier_diagnostics_validation_matrix TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE matrix_kind ON TABLE atelier_diagnostics_validation_matrix TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE surface ON TABLE atelier_diagnostics_validation_matrix TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE check_id ON TABLE atelier_diagnostics_validation_matrix TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE requirement ON TABLE atelier_diagnostics_validation_matrix TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_diagnostics_validation_matrix TYPE 'REQUIRED' | 'COVERED' | 'DEFERRED';
DEFINE FIELD OVERWRITE evidence_ref ON TABLE atelier_diagnostics_validation_matrix TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_diagnostics_validation_matrix TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_diagnostics_validation_matrix ON TABLE atelier_diagnostics_validation_matrix FIELDS row_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_diagnostics_validation_matrix_kind ON TABLE atelier_diagnostics_validation_matrix FIELDS matrix_kind, surface, created_at_utc;

-- 0112_atelier_diagnostics_typed_surfaces.sql
DEFINE TABLE OVERWRITE atelier_diagnostics_error_taxonomy SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE class ON TABLE atelier_diagnostics_error_taxonomy TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE description ON TABLE atelier_diagnostics_error_taxonomy TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE recovery_hint ON TABLE atelier_diagnostics_error_taxonomy TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_diagnostics_error_taxonomy TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_diagnostics_error_taxonomy ON TABLE atelier_diagnostics_error_taxonomy FIELDS class UNIQUE;

DEFINE TABLE OVERWRITE atelier_diagnostics_prompt_response_matrix SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entry_id ON TABLE atelier_diagnostics_prompt_response_matrix TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE prompt_text ON TABLE atelier_diagnostics_prompt_response_matrix TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE expected_response_shape ON TABLE atelier_diagnostics_prompt_response_matrix TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE expected_response_shape.* ON TABLE atelier_diagnostics_prompt_response_matrix TYPE any;
DEFINE FIELD OVERWRITE scoring_schema ON TABLE atelier_diagnostics_prompt_response_matrix TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE scoring_schema.* ON TABLE atelier_diagnostics_prompt_response_matrix TYPE any;
DEFINE FIELD OVERWRITE status ON TABLE atelier_diagnostics_prompt_response_matrix TYPE 'DEFERRED' | 'ACTIVE' DEFAULT 'DEFERRED';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_diagnostics_prompt_response_matrix TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_diagnostics_prompt_response_matrix ON TABLE atelier_diagnostics_prompt_response_matrix FIELDS entry_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_diagnostics_prompt_response_matrix_status ON TABLE atelier_diagnostics_prompt_response_matrix FIELDS status, entry_id;

-- 0113_atelier_command_log_session_heartbeat.sql
DEFINE TABLE OVERWRITE atelier_command_log SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE command_log_id ON TABLE atelier_command_log TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE session_ref ON TABLE atelier_command_log TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE command_id ON TABLE atelier_command_log TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_command_log TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE receipt_ref ON TABLE atelier_command_log TYPE option<string>;
DEFINE FIELD OVERWRITE evidence_ref ON TABLE atelier_command_log TYPE option<string>;
DEFINE FIELD OVERWRITE recorded_at_utc ON TABLE atelier_command_log TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_command_log ON TABLE atelier_command_log FIELDS command_log_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_command_log_session ON TABLE atelier_command_log FIELDS session_ref, recorded_at_utc;

DEFINE TABLE OVERWRITE atelier_diagnostics_session SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE session_ref ON TABLE atelier_diagnostics_session TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_diagnostics_session TYPE 'ACTIVE' | 'STALE' DEFAULT 'ACTIVE';
DEFINE FIELD OVERWRITE last_heartbeat_utc ON TABLE atelier_diagnostics_session TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_diagnostics_session TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_diagnostics_session ON TABLE atelier_diagnostics_session FIELDS session_ref UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_diagnostics_session_heartbeat ON TABLE atelier_diagnostics_session FIELDS last_heartbeat_utc;

-- 0114_atelier_model_config_apply.sql
DEFINE TABLE OVERWRITE atelier_model_config SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE config_id ON TABLE atelier_model_config TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE base_url ON TABLE atelier_model_config TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE model ON TABLE atelier_model_config TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE api_key_ref ON TABLE atelier_model_config TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE system_prompt ON TABLE atelier_model_config TYPE string;
DEFINE FIELD OVERWRITE timeout_ms ON TABLE atelier_model_config TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_config TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_config ON TABLE atelier_model_config FIELDS config_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_model_apply SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE apply_id ON TABLE atelier_model_apply TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE suggestion_ref ON TABLE atelier_model_apply TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE state ON TABLE atelier_model_apply TYPE 'DRAFT' | 'PREVIEW' | 'VALIDATED' | 'APPLIED' | 'REJECTED' | 'ROLLED_BACK';
DEFINE FIELD OVERWRITE evidence_ref ON TABLE atelier_model_apply TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_apply TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_model_apply TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_apply ON TABLE atelier_model_apply FIELDS apply_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_model_apply_state ON TABLE atelier_model_apply FIELDS state, apply_id;

DEFINE TABLE OVERWRITE atelier_synthetic_input_guard SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE guard_id ON TABLE atelier_synthetic_input_guard TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE op ON TABLE atelier_synthetic_input_guard TYPE 'INJECT_KEY' | 'INJECT_MOUSE' | 'CLICK_ELEMENT' | 'TYPE_TEXT';
DEFINE FIELD OVERWRITE target_ref ON TABLE atelier_synthetic_input_guard TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authorized ON TABLE atelier_synthetic_input_guard TYPE bool;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_synthetic_input_guard TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_synthetic_input_guard ON TABLE atelier_synthetic_input_guard FIELDS guard_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_synthetic_input_guard_op ON TABLE atelier_synthetic_input_guard FIELDS op, guard_id;

-- 0115_atelier_diagnostics_projections.sql. The `atelier_screenshot_artifact_storage`
-- retention/redaction columns added by 0129 (retention_class, exportable,
-- redaction_applied) are folded into the field set below, matching how earlier
-- additive ALTER-only migrations are projected.
DEFINE TABLE OVERWRITE atelier_work_state_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE projection_id ON TABLE atelier_work_state_projection TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE active_mt ON TABLE atelier_work_state_projection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE owner ON TABLE atelier_work_state_projection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_work_state_projection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE blocker ON TABLE atelier_work_state_projection TYPE option<string>;
DEFINE FIELD OVERWRITE receipts_ref ON TABLE atelier_work_state_projection TYPE option<string>;
DEFINE FIELD OVERWRITE next_action ON TABLE atelier_work_state_projection TYPE option<string>;
DEFINE FIELD OVERWRITE evidence_ref ON TABLE atelier_work_state_projection TYPE option<string>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_work_state_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_work_state_projection ON TABLE atelier_work_state_projection FIELDS projection_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_work_state_projection_active_mt ON TABLE atelier_work_state_projection FIELDS active_mt, created_at_utc;

DEFINE TABLE OVERWRITE atelier_dcc_panel_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE panel_id ON TABLE atelier_dcc_panel_projection TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE panel_kind ON TABLE atelier_dcc_panel_projection TYPE 'SESSION' | 'LEASE' | 'COMMAND_LOG' | 'RECOVERY';
DEFINE FIELD OVERWRITE state_json ON TABLE atelier_dcc_panel_projection TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE state_json.* ON TABLE atelier_dcc_panel_projection TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_dcc_panel_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_dcc_panel_projection ON TABLE atelier_dcc_panel_projection FIELDS panel_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_dcc_panel_projection_kind ON TABLE atelier_dcc_panel_projection FIELDS panel_kind, created_at_utc;

DEFINE TABLE OVERWRITE atelier_screenshot_artifact_storage SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE storage_id ON TABLE atelier_screenshot_artifact_storage TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE capture_id ON TABLE atelier_screenshot_artifact_storage TYPE uuid;
DEFINE FIELD OVERWRITE artifact_manifest_id ON TABLE atelier_screenshot_artifact_storage TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE content_sha256 ON TABLE atelier_screenshot_artifact_storage TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE mime ON TABLE atelier_screenshot_artifact_storage TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE width_px ON TABLE atelier_screenshot_artifact_storage TYPE option<int>;
DEFINE FIELD OVERWRITE height_px ON TABLE atelier_screenshot_artifact_storage TYPE option<int>;
DEFINE FIELD OVERWRITE byte_len ON TABLE atelier_screenshot_artifact_storage TYPE option<int>;
DEFINE FIELD OVERWRITE label ON TABLE atelier_screenshot_artifact_storage TYPE option<string>;
DEFINE FIELD OVERWRITE retention_ttl_days ON TABLE atelier_screenshot_artifact_storage TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE pinned ON TABLE atelier_screenshot_artifact_storage TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_screenshot_artifact_storage TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE retention_class ON TABLE atelier_screenshot_artifact_storage TYPE string DEFAULT 'visual-validation';
DEFINE FIELD OVERWRITE exportable ON TABLE atelier_screenshot_artifact_storage TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE redaction_applied ON TABLE atelier_screenshot_artifact_storage TYPE bool DEFAULT false;
DEFINE INDEX OVERWRITE pk_atelier_screenshot_artifact_storage ON TABLE atelier_screenshot_artifact_storage FIELDS storage_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_screenshot_artifact_storage_capture ON TABLE atelier_screenshot_artifact_storage FIELDS capture_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_spec_drift_finding SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE finding_id ON TABLE atelier_spec_drift_finding TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE doc_ref ON TABLE atelier_spec_drift_finding TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE spec_ref ON TABLE atelier_spec_drift_finding TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE drift_kind ON TABLE atelier_spec_drift_finding TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE detail ON TABLE atelier_spec_drift_finding TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_spec_drift_finding TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_spec_drift_finding ON TABLE atelier_spec_drift_finding FIELDS finding_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_spec_drift_finding_kind ON TABLE atelier_spec_drift_finding FIELDS drift_kind, created_at_utc;

-- 0116_atelier_dcc_flight_recorder.sql
DEFINE TABLE OVERWRITE atelier_dcc_workflow_panel_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE panel_id ON TABLE atelier_dcc_workflow_panel_projection TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE panel_kind ON TABLE atelier_dcc_workflow_panel_projection TYPE 'APPROVALS' | 'VISUAL_CAPTURE';
DEFINE FIELD OVERWRITE state_json ON TABLE atelier_dcc_workflow_panel_projection TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE state_json.* ON TABLE atelier_dcc_workflow_panel_projection TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_dcc_workflow_panel_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_dcc_workflow_panel_projection ON TABLE atelier_dcc_workflow_panel_projection FIELDS panel_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_dcc_workflow_panel_projection_kind ON TABLE atelier_dcc_workflow_panel_projection FIELDS panel_kind, created_at_utc;

DEFINE TABLE OVERWRITE atelier_fr_workflow_event SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE record_id ON TABLE atelier_fr_workflow_event TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE event_kind ON TABLE atelier_fr_workflow_event TYPE 'FR-EVT-TOOL-CALL' | 'FR-EVT-TOOL-PROPOSAL' | 'FR-EVT-TOOL-APPLY-DECISION' | 'FR-EVT-VISUAL-CAPTURE' | 'FR-EVT-VISUAL-VALIDATION' | 'FR-EVT-VISUAL-RECOVERY' | 'FR-EVT-BUILD-GUARD' | 'FR-EVT-PACKAGE-GUARD' | 'FR-EVT-STALE-DOC-DETECTED';
DEFINE FIELD OVERWRITE mt_owner ON TABLE atelier_fr_workflow_event TYPE 'MT-191' | 'MT-192' | 'MT-193' ASSERT ($this.event_kind INSIDE ['FR-EVT-TOOL-CALL', 'FR-EVT-TOOL-PROPOSAL', 'FR-EVT-TOOL-APPLY-DECISION'] AND $value = 'MT-191') OR ($this.event_kind INSIDE ['FR-EVT-VISUAL-CAPTURE', 'FR-EVT-VISUAL-VALIDATION', 'FR-EVT-VISUAL-RECOVERY'] AND $value = 'MT-192') OR ($this.event_kind INSIDE ['FR-EVT-BUILD-GUARD', 'FR-EVT-PACKAGE-GUARD', 'FR-EVT-STALE-DOC-DETECTED'] AND $value = 'MT-193');
DEFINE FIELD OVERWRITE session_ref ON TABLE atelier_fr_workflow_event TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE payload ON TABLE atelier_fr_workflow_event TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE payload.* ON TABLE atelier_fr_workflow_event TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_fr_workflow_event TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_fr_workflow_event ON TABLE atelier_fr_workflow_event FIELDS record_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_fr_workflow_event_kind ON TABLE atelier_fr_workflow_event FIELDS event_kind, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_fr_workflow_event_owner ON TABLE atelier_fr_workflow_event FIELDS mt_owner, created_at_utc;

-- 0117_atelier_editable_surface_authority.sql
DEFINE TABLE OVERWRITE atelier_model_manual_section SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE section_id ON TABLE atelier_model_manual_section TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE section_text ON TABLE atelier_model_manual_section TYPE string ASSERT string::trim($value) != '' AND string::len($value) <= 1048576;
DEFINE FIELD OVERWRITE revision ON TABLE atelier_model_manual_section TYPE int DEFAULT 1 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_model_manual_section TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_model_manual_section TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_manual_section TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_manual_section ON TABLE atelier_model_manual_section FIELDS section_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_retrieval_policy SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE task_type ON TABLE atelier_retrieval_policy TYPE 'validator_hbr_test_packet' | 'kernel_builder_mt_implementation' | 'integration_validator_batch_review' | 'operator_triage' | 'swarm_harness_session' | 'process_ledger_inspection' | 'self_improvement_loop_eval' | 'general_retrieval';
DEFINE FIELD OVERWRITE parameter ON TABLE atelier_retrieval_policy TYPE 'top_k' | 'capsule_budget_bytes';
DEFINE FIELD OVERWRITE value ON TABLE atelier_retrieval_policy TYPE int ASSERT $value > 0 AND ($this.parameter != 'top_k' OR $value <= 64) AND ($this.parameter != 'capsule_budget_bytes' OR $value <= 1048576);
DEFINE FIELD OVERWRITE updated_by ON TABLE atelier_retrieval_policy TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at_utc ON TABLE atelier_retrieval_policy TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_retrieval_policy TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_retrieval_policy ON TABLE atelier_retrieval_policy FIELDS task_type, parameter UNIQUE;

-- 0118_atelier_self_improve_runs.sql
DEFINE TABLE OVERWRITE atelier_self_improve_sandbox_run SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE sandbox_run_id ON TABLE atelier_self_improve_sandbox_run TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE surface_kind ON TABLE atelier_self_improve_sandbox_run TYPE 'model_manual' | 'retrieval_policy';
DEFINE FIELD OVERWRITE snapshot_sha256 ON TABLE atelier_self_improve_sandbox_run TYPE string ASSERT string::starts_with($value, 'sha256:') AND string::len($value) = 71;
DEFINE FIELD OVERWRITE workspace_ref ON TABLE atelier_self_improve_sandbox_run TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE atelier_self_improve_sandbox_run TYPE 'provisioned' | 'failed';
DEFINE FIELD OVERWRITE started_at_utc ON TABLE atelier_self_improve_sandbox_run TYPE datetime;
DEFINE FIELD OVERWRITE completed_at_utc ON TABLE atelier_self_improve_sandbox_run TYPE datetime ASSERT $value >= $this.started_at_utc;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_self_improve_sandbox_run TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_self_improve_sandbox_run ON TABLE atelier_self_improve_sandbox_run FIELDS sandbox_run_id UNIQUE;

DEFINE TABLE OVERWRITE atelier_validator_first_pass_run SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE first_pass_run_id ON TABLE atelier_validator_first_pass_run TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE sandbox_run_id ON TABLE atelier_validator_first_pass_run TYPE option<record<atelier_self_improve_sandbox_run>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE corpus_item_id ON TABLE atelier_validator_first_pass_run TYPE uuid;
DEFINE FIELD OVERWRITE hbr_rule_id ON TABLE atelier_validator_first_pass_run TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE packet_under_test ON TABLE atelier_validator_first_pass_run TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE transition ON TABLE atelier_validator_first_pass_run TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE verdict ON TABLE atelier_validator_first_pass_run TYPE 'pass' | 'fail' | 'skip';
DEFINE FIELD OVERWRITE failing_rule_count ON TABLE atelier_validator_first_pass_run TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE latency_ms ON TABLE atelier_validator_first_pass_run TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE capsule_bytes ON TABLE atelier_validator_first_pass_run TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE gate_event_id ON TABLE atelier_validator_first_pass_run TYPE option<uuid>;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_validator_first_pass_run TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_validator_first_pass_run ON TABLE atelier_validator_first_pass_run FIELDS first_pass_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_vfp_run_sandbox ON TABLE atelier_validator_first_pass_run FIELDS sandbox_run_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_atelier_vfp_run_corpus_item ON TABLE atelier_validator_first_pass_run FIELDS corpus_item_id;

-- 0119_atelier_model_coordination_lease.sql. The PostgreSQL partial index
-- (`WHERE lease_state = 'active'`) has no partial-index form here; the
-- single-active-claimant rule is enforced in the claim transaction.
DEFINE TABLE OVERWRITE atelier_model_coordination_lease SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE claim_id ON TABLE atelier_model_coordination_lease TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE thread_id ON TABLE atelier_model_coordination_lease TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE executor_kind ON TABLE atelier_model_coordination_lease TYPE 'local_small_model' | 'local_large_model' | 'cloud_model' | 'reviewer' | 'validator' | 'operator' | 'workflow_automation';
DEFINE FIELD OVERWRITE actor_id ON TABLE atelier_model_coordination_lease TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE session_id ON TABLE atelier_model_coordination_lease TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_mode ON TABLE atelier_model_coordination_lease TYPE 'exclusive_lease' | 'shared_observer' | 'broadcast_request' | 'handoff_reservation';
DEFINE FIELD OVERWRITE lease_state ON TABLE atelier_model_coordination_lease TYPE 'active' | 'released' | 'expired' | 'taken_over' DEFAULT 'active';
DEFINE FIELD OVERWRITE claimed_at_utc ON TABLE atelier_model_coordination_lease TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE ttl_seconds ON TABLE atelier_model_coordination_lease TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE lease_expires_at_utc ON TABLE atelier_model_coordination_lease TYPE datetime ASSERT $value > $this.claimed_at_utc;
DEFINE FIELD OVERWRITE released_at_utc ON TABLE atelier_model_coordination_lease TYPE option<datetime> ASSERT $value = NONE OR $this.lease_state = 'released';
DEFINE FIELD OVERWRITE taken_over_at_utc ON TABLE atelier_model_coordination_lease TYPE option<datetime> ASSERT $value = NONE OR $this.lease_state = 'taken_over';
DEFINE FIELD OVERWRITE takeover_reason ON TABLE atelier_model_coordination_lease TYPE option<string>;
DEFINE FIELD OVERWRITE prior_claim_id ON TABLE atelier_model_coordination_lease TYPE option<record<atelier_model_coordination_lease>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE linked_work_packet_id ON TABLE atelier_model_coordination_lease TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE linked_micro_task_id ON TABLE atelier_model_coordination_lease TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_coordination_lease TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_coordination_lease ON TABLE atelier_model_coordination_lease FIELDS claim_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_model_lease_thread_active ON TABLE atelier_model_coordination_lease FIELDS thread_id, lease_state;
DEFINE INDEX OVERWRITE idx_atelier_model_lease_actor ON TABLE atelier_model_coordination_lease FIELDS actor_id, created_at_utc;

-- 0120_kernel_diagnostic_bundle_manifest.sql
DEFINE TABLE OVERWRITE kernel_diagnostic_bundle_manifest SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE manifest_id ON TABLE kernel_diagnostic_bundle_manifest TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE schema_id ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE subject_kind ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE subject_ref ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE failure_summary ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE error_taxonomy ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE severity ON TABLE kernel_diagnostic_bundle_manifest TYPE 'fatal' | 'error' | 'warning' | 'info' | 'hint';
DEFINE FIELD OVERWRITE created_by ON TABLE kernel_diagnostic_bundle_manifest TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE sections_json ON TABLE kernel_diagnostic_bundle_manifest TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE sections_json.* ON TABLE kernel_diagnostic_bundle_manifest TYPE any;
DEFINE FIELD OVERWRITE reproduction_json ON TABLE kernel_diagnostic_bundle_manifest TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE reproduction_json.* ON TABLE kernel_diagnostic_bundle_manifest TYPE any;
DEFINE FIELD OVERWRITE isolation_json ON TABLE kernel_diagnostic_bundle_manifest TYPE array DEFAULT [] ASSERT array::len($value) > 0;
DEFINE FIELD OVERWRITE isolation_json.* ON TABLE kernel_diagnostic_bundle_manifest TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_diagnostic_bundle_manifest TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_diagnostic_bundle_manifest ON TABLE kernel_diagnostic_bundle_manifest FIELDS manifest_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_diag_bundle_manifest_subject ON TABLE kernel_diagnostic_bundle_manifest FIELDS subject_kind, subject_ref, created_at_utc;

-- 0122_atelier_model_manual_merge_drift.sql
DEFINE TABLE OVERWRITE atelier_model_manual_row_merge SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE run_id ON TABLE atelier_model_manual_row_merge TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE source_kind ON TABLE atelier_model_manual_row_merge TYPE 'core_data' | 'pose_comfy' | 'diagnostics_owned';
DEFINE FIELD OVERWRITE manual_version ON TABLE atelier_model_manual_row_merge TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE merged_row_count ON TABLE atelier_model_manual_row_merge TYPE int ASSERT $value >= 0 AND $value = array::len($this.merged_rows);
DEFINE FIELD OVERWRITE blocker_count ON TABLE atelier_model_manual_row_merge TYPE int ASSERT $value >= 0 AND $value = array::len($this.blockers) AND ($value + $this.merged_row_count) > 0;
DEFINE FIELD OVERWRITE merged_rows ON TABLE atelier_model_manual_row_merge TYPE array;
DEFINE FIELD OVERWRITE merged_rows.* ON TABLE atelier_model_manual_row_merge TYPE any;
DEFINE FIELD OVERWRITE blockers ON TABLE atelier_model_manual_row_merge TYPE array;
DEFINE FIELD OVERWRITE blockers.* ON TABLE atelier_model_manual_row_merge TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_manual_row_merge TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_manual_row_merge ON TABLE atelier_model_manual_row_merge FIELDS run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_mm_row_merge_kind ON TABLE atelier_model_manual_row_merge FIELDS source_kind, created_at_utc;

DEFINE TABLE OVERWRITE atelier_model_manual_drift_guard SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE run_id ON TABLE atelier_model_manual_drift_guard TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE guard_scope ON TABLE atelier_model_manual_drift_guard TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE manual_version ON TABLE atelier_model_manual_drift_guard TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE wired_surface_sha256 ON TABLE atelier_model_manual_drift_guard TYPE string ASSERT string::starts_with($value, 'sha256:') AND string::len($value) = 71;
DEFINE FIELD OVERWRITE wired_surface_changed ON TABLE atelier_model_manual_drift_guard TYPE bool;
DEFINE FIELD OVERWRITE finding_count ON TABLE atelier_model_manual_drift_guard TYPE int ASSERT $value >= 0 AND $value = array::len($this.findings);
DEFINE FIELD OVERWRITE findings ON TABLE atelier_model_manual_drift_guard TYPE array;
DEFINE FIELD OVERWRITE findings.* ON TABLE atelier_model_manual_drift_guard TYPE any;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_model_manual_drift_guard TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_model_manual_drift_guard ON TABLE atelier_model_manual_drift_guard FIELDS run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_mm_drift_guard_scope ON TABLE atelier_model_manual_drift_guard FIELDS guard_scope, created_at_utc;

-- 0124_kernel_visual_diff_baseline.sql
DEFINE TABLE OVERWRITE kernel_visual_diff_baseline SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE baseline_id ON TABLE kernel_visual_diff_baseline TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE surface_id ON TABLE kernel_visual_diff_baseline TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE baseline_ref ON TABLE kernel_visual_diff_baseline TYPE string ASSERT string::starts_with($value, 'artifact://baselines/');
DEFINE FIELD OVERWRITE content_sha256 ON TABLE kernel_visual_diff_baseline TYPE string ASSERT string::starts_with($value, 'sha256:') AND string::len($value) = 71;
DEFINE FIELD OVERWRITE captured_by ON TABLE kernel_visual_diff_baseline TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE captured_at_utc ON TABLE kernel_visual_diff_baseline TYPE datetime;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_visual_diff_baseline TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_visual_diff_baseline ON TABLE kernel_visual_diff_baseline FIELDS baseline_id UNIQUE;
DEFINE INDEX OVERWRITE uq_kernel_vd_baseline_surface_ref ON TABLE kernel_visual_diff_baseline FIELDS surface_id, baseline_ref UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_vd_baseline_surface_latest ON TABLE kernel_visual_diff_baseline FIELDS surface_id, captured_at_utc, baseline_id;

DEFINE TABLE OVERWRITE kernel_visual_diff_request SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE request_id ON TABLE kernel_visual_diff_request TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE surface_id ON TABLE kernel_visual_diff_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE baseline_id ON TABLE kernel_visual_diff_request TYPE option<record<kernel_visual_diff_baseline>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE previous_screenshot_ref ON TABLE kernel_visual_diff_request TYPE option<string> ASSERT ($value = NONE AND $this.baseline_id != NONE) OR (string::starts_with($value, 'artifact://screenshots/') AND $this.baseline_id = NONE);
DEFINE FIELD OVERWRITE candidate_screenshot_ref ON TABLE kernel_visual_diff_request TYPE string ASSERT string::starts_with($value, 'artifact://screenshots/');
DEFINE FIELD OVERWRITE comparison_mode ON TABLE kernel_visual_diff_request TYPE 'pixel_diff' | 'structural_dom' | 'manual';
DEFINE FIELD OVERWRITE threshold_config_ref ON TABLE kernel_visual_diff_request TYPE string ASSERT string::starts_with($value, 'packet://');
DEFINE FIELD OVERWRITE max_pixel_diff_basis_points ON TABLE kernel_visual_diff_request TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE max_layout_shift_basis_points ON TABLE kernel_visual_diff_request TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE structural_mismatch_limit ON TABLE kernel_visual_diff_request TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE metadata_json ON TABLE kernel_visual_diff_request TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE metadata_json.* ON TABLE kernel_visual_diff_request TYPE any;
DEFINE FIELD OVERWRITE requested_by ON TABLE kernel_visual_diff_request TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_visual_diff_request TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_visual_diff_request ON TABLE kernel_visual_diff_request FIELDS request_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_vd_request_surface ON TABLE kernel_visual_diff_request FIELDS surface_id, created_at_utc;

DEFINE TABLE OVERWRITE kernel_visual_diff_result SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE result_id ON TABLE kernel_visual_diff_result TYPE uuid ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE request_id ON TABLE kernel_visual_diff_result TYPE record<kernel_visual_diff_request> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE comparison_mode ON TABLE kernel_visual_diff_result TYPE 'pixel_diff' | 'structural_dom' | 'manual';
DEFINE FIELD OVERWRITE units_compared ON TABLE kernel_visual_diff_result TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE units_differing ON TABLE kernel_visual_diff_result TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE mismatch_basis_points ON TABLE kernel_visual_diff_result TYPE int ASSERT $value >= 0 AND $value <= 10000;
DEFINE FIELD OVERWRITE threshold_exceeded ON TABLE kernel_visual_diff_result TYPE bool;
DEFINE FIELD OVERWRITE outcome ON TABLE kernel_visual_diff_result TYPE 'pass' | 'fail' | 'manual_review_required' ASSERT ($this.comparison_mode = 'manual' AND $value = 'manual_review_required') OR ($this.comparison_mode != 'manual' AND $value != 'manual_review_required');
DEFINE FIELD OVERWRITE computed_at_utc ON TABLE kernel_visual_diff_result TYPE datetime;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE kernel_visual_diff_result TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_kernel_visual_diff_result ON TABLE kernel_visual_diff_result FIELDS result_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kernel_vd_result_request ON TABLE kernel_visual_diff_result FIELDS request_id, created_at_utc;

-- 0129_atelier_visual_steer_retention.sql. The MT-158 retention/redaction
-- columns this migration adds to `atelier_screenshot_artifact_storage` are
-- folded into that table's definition in the 0115 section above.
DEFINE TABLE OVERWRITE atelier_visual_steer_feedback SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE feedback_id ON TABLE atelier_visual_steer_feedback TYPE string ASSERT ($value = record::id($this.id)) AND string::trim($value) != '';
DEFINE FIELD OVERWRITE loop_id ON TABLE atelier_visual_steer_feedback TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE evidence_id ON TABLE atelier_visual_steer_feedback TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE wp_id ON TABLE atelier_visual_steer_feedback TYPE string;
DEFINE FIELD OVERWRITE mismatch_basis_points ON TABLE atelier_visual_steer_feedback TYPE int ASSERT $value > $this.threshold_basis_points;
DEFINE FIELD OVERWRITE threshold_basis_points ON TABLE atelier_visual_steer_feedback TYPE int;
DEFINE FIELD OVERWRITE target_role ON TABLE atelier_visual_steer_feedback TYPE string;
DEFINE FIELD OVERWRITE receipt_kind ON TABLE atelier_visual_steer_feedback TYPE 'STEER';
DEFINE FIELD OVERWRITE code_diff_ref ON TABLE atelier_visual_steer_feedback TYPE string;
DEFINE FIELD OVERWRITE visual_diff_ref ON TABLE atelier_visual_steer_feedback TYPE string;
DEFINE FIELD OVERWRITE next_action ON TABLE atelier_visual_steer_feedback TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_visual_steer_feedback TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_visual_steer_feedback ON TABLE atelier_visual_steer_feedback FIELDS feedback_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_visual_steer_loop_evidence ON TABLE atelier_visual_steer_feedback FIELDS loop_id, evidence_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_visual_steer_feedback_loop ON TABLE atelier_visual_steer_feedback FIELDS loop_id, created_at_utc;

-- 0130_knowledge_schema_namespace.sql
DEFINE TABLE OVERWRITE knowledge_schema_registry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE family_key ON TABLE knowledge_schema_registry TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE table_name ON TABLE knowledge_schema_registry TYPE string;
DEFINE FIELD OVERWRITE record_family ON TABLE knowledge_schema_registry TYPE string;
DEFINE FIELD OVERWRITE authority_class ON TABLE knowledge_schema_registry TYPE 'authority' | 'projection' | 'support';
DEFINE FIELD OVERWRITE migration_file ON TABLE knowledge_schema_registry TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_schema_registry TYPE string DEFAULT 'WP-KERNEL-009';
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_schema_registry TYPE string;
DEFINE FIELD OVERWRITE registered_at ON TABLE knowledge_schema_registry TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_schema_registry ON TABLE knowledge_schema_registry FIELDS family_key UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_schema_registry_table ON TABLE knowledge_schema_registry FIELDS table_name UNIQUE;

-- 0131_knowledge_source_roots.sql
DEFINE TABLE OVERWRITE knowledge_source_roots SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE root_id ON TABLE knowledge_source_roots TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_source_roots TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE display_name ON TABLE knowledge_source_roots TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE root_kind ON TABLE knowledge_source_roots TYPE 'project_repo' | 'governance' | 'artifacts' | 'media_library' | 'external_import' | 'operator_folder';
DEFINE FIELD OVERWRITE repo_relative_path ON TABLE knowledge_source_roots TYPE string;
DEFINE FIELD OVERWRITE path_normalization ON TABLE knowledge_source_roots TYPE 'repo_relative_posix_v1' DEFAULT 'repo_relative_posix_v1';
DEFINE FIELD OVERWRITE allowlist_policy ON TABLE knowledge_source_roots TYPE object FLEXIBLE DEFAULT { include: ['**/*'], exclude: [] };
DEFINE FIELD OVERWRITE allowlist_policy.* ON TABLE knowledge_source_roots TYPE any;
DEFINE FIELD OVERWRITE indexing_eligibility ON TABLE knowledge_source_roots TYPE 'eligible' | 'paused' | 'excluded' DEFAULT 'eligible';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_source_roots TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_source_roots TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_source_roots ON TABLE knowledge_source_roots FIELDS root_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_source_roots_workspace_path ON TABLE knowledge_source_roots FIELDS workspace_id, repo_relative_path UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_source_roots_workspace ON TABLE knowledge_source_roots FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_knowledge_source_roots_eligibility ON TABLE knowledge_source_roots FIELDS indexing_eligibility;

-- 0132_knowledge_sources.sql
DEFINE TABLE OVERWRITE knowledge_sources SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_sources TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_sources TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE root_id ON TABLE knowledge_sources TYPE option<record<knowledge_source_roots>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_kind ON TABLE knowledge_sources TYPE 'file' | 'asset' | 'rich_document' | 'loom_block' | 'external_import' | 'operator_artifact';
DEFINE FIELD OVERWRITE relative_path ON TABLE knowledge_sources TYPE option<string>;
DEFINE FIELD OVERWRITE asset_id ON TABLE knowledge_sources TYPE option<record<assets>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE loom_block_id ON TABLE knowledge_sources TYPE option<record<loom_blocks>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_id ON TABLE knowledge_sources TYPE option<record<documents>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE content_hash ON TABLE knowledge_sources TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE size_bytes ON TABLE knowledge_sources TYPE option<int>;
DEFINE FIELD OVERWRITE provenance ON TABLE knowledge_sources TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE provenance.* ON TABLE knowledge_sources TYPE any;
DEFINE FIELD OVERWRITE permission_scope ON TABLE knowledge_sources TYPE 'workspace' | 'operator_private' | 'shared' DEFAULT 'workspace';
DEFINE FIELD OVERWRITE redaction_state ON TABLE knowledge_sources TYPE 'none' | 'partial' | 'redacted' DEFAULT 'none';
DEFINE FIELD OVERWRITE parser_status ON TABLE knowledge_sources TYPE 'pending' | 'parsed' | 'failed' | 'skipped' DEFAULT 'pending';
DEFINE FIELD OVERWRITE extraction_status ON TABLE knowledge_sources TYPE 'pending' | 'extracted' | 'failed' | 'skipped' DEFAULT 'pending';
DEFINE FIELD OVERWRITE stale ON TABLE knowledge_sources TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE last_index_receipt_event_id ON TABLE knowledge_sources TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE source_modified_at ON TABLE knowledge_sources TYPE option<datetime>;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_sources TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_sources TYPE datetime DEFAULT time::now();
-- Stored discriminator for the source partial UNIQUE index
-- `uq_knowledge_sources_root_path ... WHERE relative_path IS NOT NULL`.
DEFINE FIELD OVERWRITE root_path_key ON TABLE knowledge_sources TYPE string
    VALUE IF $this.relative_path != NONE {
        'path:' + type::string($this.root_id) + '|' + $this.relative_path
    } ELSE {
        'unset:' + $this.source_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_sources ON TABLE knowledge_sources FIELDS source_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_sources_root_path ON TABLE knowledge_sources FIELDS root_path_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_sources_workspace ON TABLE knowledge_sources FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_knowledge_sources_root ON TABLE knowledge_sources FIELDS root_id;
DEFINE INDEX OVERWRITE idx_knowledge_sources_stale ON TABLE knowledge_sources FIELDS stale;
DEFINE INDEX OVERWRITE idx_knowledge_sources_content_hash ON TABLE knowledge_sources FIELDS content_hash;

-- 0133_knowledge_index_runs.sql
DEFINE TABLE OVERWRITE knowledge_index_runs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE index_run_id ON TABLE knowledge_index_runs TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_index_runs TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE root_id ON TABLE knowledge_index_runs TYPE option<record<knowledge_source_roots>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE run_state ON TABLE knowledge_index_runs TYPE 'started' | 'completed' | 'failed' | 'cancelled' DEFAULT 'started';
DEFINE FIELD OVERWRITE scope ON TABLE knowledge_index_runs TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE scope.* ON TABLE knowledge_index_runs TYPE any;
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_index_runs TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_index_runs TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE worktree_id ON TABLE knowledge_index_runs TYPE option<string>;
DEFINE FIELD OVERWRITE restart_checkpoint ON TABLE knowledge_index_runs TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE restart_checkpoint.* ON TABLE knowledge_index_runs TYPE any;
DEFINE FIELD OVERWRITE sources_seen ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE sources_indexed ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE spans_extracted ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE entities_detected ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE edges_written ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE claims_written ON TABLE knowledge_index_runs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE error_capture ON TABLE knowledge_index_runs TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE error_capture.* ON TABLE knowledge_index_runs TYPE any;
DEFINE FIELD OVERWRITE start_receipt_event_id ON TABLE knowledge_index_runs TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE finish_receipt_event_id ON TABLE knowledge_index_runs TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE started_at ON TABLE knowledge_index_runs TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE finished_at ON TABLE knowledge_index_runs TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_knowledge_index_runs ON TABLE knowledge_index_runs FIELDS index_run_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_index_runs_workspace ON TABLE knowledge_index_runs FIELDS workspace_id, started_at;
DEFINE INDEX OVERWRITE idx_knowledge_index_runs_state ON TABLE knowledge_index_runs FIELDS run_state;
DEFINE INDEX OVERWRITE idx_knowledge_index_runs_root ON TABLE knowledge_index_runs FIELDS root_id;

-- 0134_knowledge_spans.sql
DEFINE TABLE OVERWRITE knowledge_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_spans TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_spans TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_kind ON TABLE knowledge_spans TYPE 'byte' | 'text' | 'ast' | 'media_time' | 'page' | 'cell' | 'rich_doc';
DEFINE FIELD OVERWRITE range_start ON TABLE knowledge_spans TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE range_end ON TABLE knowledge_spans TYPE int ASSERT $value >= $this.range_start;
DEFINE FIELD OVERWRITE line_start ON TABLE knowledge_spans TYPE option<int>;
DEFINE FIELD OVERWRITE line_end ON TABLE knowledge_spans TYPE option<int>;
DEFINE FIELD OVERWRITE section_path ON TABLE knowledge_spans TYPE option<string>;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE knowledge_spans TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE parser_version ON TABLE knowledge_spans TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE extraction_receipt_event_id ON TABLE knowledge_spans TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE index_run_id ON TABLE knowledge_spans TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE display_snippet ON TABLE knowledge_spans TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_spans ON TABLE knowledge_spans FIELDS span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_spans_source ON TABLE knowledge_spans FIELDS source_id, range_start;
DEFINE INDEX OVERWRITE idx_knowledge_spans_run ON TABLE knowledge_spans FIELDS index_run_id;
DEFINE INDEX OVERWRITE idx_knowledge_spans_content ON TABLE knowledge_spans FIELDS content_sha256;

-- 0135_knowledge_entities.sql
DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entity_id ON TABLE knowledge_entities TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_entities TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE entity_kind ON TABLE knowledge_entities TYPE 'symbol' | 'concept' | 'file' | 'folder' | 'project' | 'person' | 'role' | 'task' | 'api' | 'schema' | 'command' | 'media' | 'manual_entry' | 'product_primitive' | 'spec_topic' | 'work_packet' | 'micro_task' | 'taskboard_row' | 'rich_document' | 'loom_block' | 'user_manual_page';
DEFINE FIELD OVERWRITE entity_key ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE display_name ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE detection_provenance ON TABLE knowledge_entities TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE detection_provenance.* ON TABLE knowledge_entities TYPE any;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE knowledge_entities TYPE 'active' | 'retired' DEFAULT 'active';
DEFINE FIELD OVERWRITE primary_source_id ON TABLE knowledge_entities TYPE option<record<knowledge_sources>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE first_detected_in_run ON TABLE knowledge_entities TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE last_detected_in_run ON TABLE knowledge_entities TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_entities TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_entities TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_entities ON TABLE knowledge_entities FIELDS entity_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_entities_identity ON TABLE knowledge_entities FIELDS workspace_id, entity_kind, entity_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_entities_workspace_kind ON TABLE knowledge_entities FIELDS workspace_id, entity_kind;
DEFINE INDEX OVERWRITE idx_knowledge_entities_primary_source ON TABLE knowledge_entities FIELDS primary_source_id;

DEFINE TABLE OVERWRITE knowledge_entity_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entity_id ON TABLE knowledge_entity_spans TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_entity_spans TYPE record<knowledge_spans> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE detected_in_run ON TABLE knowledge_entity_spans TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_entity_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_entity_spans ON TABLE knowledge_entity_spans FIELDS entity_id, span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_entity_spans_span ON TABLE knowledge_entity_spans FIELDS span_id;

-- 0136_knowledge_edges.sql
DEFINE TABLE OVERWRITE knowledge_edges SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE edge_id ON TABLE knowledge_edges TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_edges TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE relationship_id ON TABLE knowledge_edges TYPE string;
DEFINE FIELD OVERWRITE edge_type ON TABLE knowledge_edges TYPE 'defines' | 'references' | 'contains' | 'depends_on' | 'implements' | 'documents' | 'validates' | 'derived_from' | 'mentions' | 'links_to' | 'supersedes' | 'relates_to';
DEFINE FIELD OVERWRITE source_entity_id ON TABLE knowledge_edges TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE target_entity_id ON TABLE knowledge_edges TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE extractor_version ON TABLE knowledge_edges TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE knowledge_edges TYPE 'proposed' | 'active' | 'conflicted' | 'retired' DEFAULT 'active';
DEFINE FIELD OVERWRITE confidence ON TABLE knowledge_edges TYPE float ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD OVERWRITE conflict_marker ON TABLE knowledge_edges TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE conflict_marker.* ON TABLE knowledge_edges TYPE any;
DEFINE FIELD OVERWRITE created_in_run ON TABLE knowledge_edges TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE last_seen_in_run ON TABLE knowledge_edges TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_edges TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_edges TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_edges ON TABLE knowledge_edges FIELDS edge_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_edges_relationship ON TABLE knowledge_edges FIELDS workspace_id, relationship_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_edges_source_entity ON TABLE knowledge_edges FIELDS source_entity_id;
DEFINE INDEX OVERWRITE idx_knowledge_edges_target_entity ON TABLE knowledge_edges FIELDS target_entity_id;
DEFINE INDEX OVERWRITE idx_knowledge_edges_type ON TABLE knowledge_edges FIELDS workspace_id, edge_type;

DEFINE TABLE OVERWRITE knowledge_edge_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE edge_id ON TABLE knowledge_edge_spans TYPE record<knowledge_edges> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_edge_spans TYPE record<knowledge_spans> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE recorded_in_run ON TABLE knowledge_edge_spans TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_edge_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_edge_spans ON TABLE knowledge_edge_spans FIELDS edge_id, span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_edge_spans_span ON TABLE knowledge_edge_spans FIELDS span_id;

-- 0137_knowledge_claims.sql
DEFINE TABLE OVERWRITE knowledge_claims SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_claims TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_claims TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE claim_kind ON TABLE knowledge_claims TYPE 'source_fact' | 'product_behavior' | 'task_state' | 'operator_workflow';
DEFINE FIELD OVERWRITE claim_text ON TABLE knowledge_claims TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE subject_entity_id ON TABLE knowledge_claims TYPE option<record<knowledge_entities>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE knowledge_claims TYPE 'proposed' | 'accepted' | 'conflicted' | 'retired' DEFAULT 'proposed';
DEFINE FIELD OVERWRITE temporal_qualifier ON TABLE knowledge_claims TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE temporal_qualifier.* ON TABLE knowledge_claims TYPE any;
DEFINE FIELD OVERWRITE granularity_qualifier ON TABLE knowledge_claims TYPE option<string>;
DEFINE FIELD OVERWRITE confidence ON TABLE knowledge_claims TYPE float DEFAULT 0.5 ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD OVERWRITE retirement_reason ON TABLE knowledge_claims TYPE option<'rejected' | 'superseded' | 'stale' | 'operator_retired'>;
DEFINE FIELD OVERWRITE superseded_by_claim_id ON TABLE knowledge_claims TYPE option<record<knowledge_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE proposed_in_run ON TABLE knowledge_claims TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE resolution_receipt_event_id ON TABLE knowledge_claims TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_claims TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_claims TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_claims ON TABLE knowledge_claims FIELDS claim_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_claims_workspace_state ON TABLE knowledge_claims FIELDS workspace_id, lifecycle_state;
DEFINE INDEX OVERWRITE idx_knowledge_claims_subject ON TABLE knowledge_claims FIELDS subject_entity_id;

DEFINE TABLE OVERWRITE knowledge_claim_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_claim_spans TYPE record<knowledge_claims> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_claim_spans TYPE record<knowledge_spans> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_claim_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_claim_spans ON TABLE knowledge_claim_spans FIELDS claim_id, span_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_claim_spans_span ON TABLE knowledge_claim_spans FIELDS span_id;

DEFINE TABLE OVERWRITE knowledge_claim_conflicts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE conflict_id ON TABLE knowledge_claim_conflicts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_claim_conflicts TYPE record<knowledge_claims> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE conflicting_claim_id ON TABLE knowledge_claim_conflicts TYPE record<knowledge_claims> ASSERT record::exists($value) AND $value != $this.claim_id REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE detected_in_run ON TABLE knowledge_claim_conflicts TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE conflict_reason ON TABLE knowledge_claim_conflicts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE resolution_receipt_event_id ON TABLE knowledge_claim_conflicts TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE detected_at ON TABLE knowledge_claim_conflicts TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE resolved_at ON TABLE knowledge_claim_conflicts TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_knowledge_claim_conflicts ON TABLE knowledge_claim_conflicts FIELDS conflict_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_claim_conflicts_pair ON TABLE knowledge_claim_conflicts FIELDS claim_id, conflicting_claim_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_claim_conflicts_claim ON TABLE knowledge_claim_conflicts FIELDS claim_id;
DEFINE INDEX OVERWRITE idx_knowledge_claim_conflicts_conflicting ON TABLE knowledge_claim_conflicts FIELDS conflicting_claim_id;

-- 0138_knowledge_memory_passages.sql
DEFINE TABLE OVERWRITE knowledge_memory_passages SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE passage_id ON TABLE knowledge_memory_passages TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_passages TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE passage_text ON TABLE knowledge_memory_passages TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE token_count ON TABLE knowledge_memory_passages TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE ocr_transcript_metadata ON TABLE knowledge_memory_passages TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE ocr_transcript_metadata.* ON TABLE knowledge_memory_passages TYPE any;
DEFINE FIELD OVERWRITE extraction_confidence ON TABLE knowledge_memory_passages TYPE float DEFAULT 1.0 ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD OVERWRITE ranking_features ON TABLE knowledge_memory_passages TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE ranking_features.* ON TABLE knowledge_memory_passages TYPE any;
DEFINE FIELD OVERWRITE retrieval_mode ON TABLE knowledge_memory_passages TYPE 'none' | 'direct_load' | 'exact_lookup' | 'graph_traversal' | 'hybrid_rag' DEFAULT 'hybrid_rag';
DEFINE FIELD OVERWRITE freshness_at ON TABLE knowledge_memory_passages TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE compaction_policy ON TABLE knowledge_memory_passages TYPE 'keep' | 'compactable' | 'expired' DEFAULT 'keep';
DEFINE FIELD OVERWRITE failure_receipt_event_id ON TABLE knowledge_memory_passages TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE derived_in_run ON TABLE knowledge_memory_passages TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_passages TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_memory_passages TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_passages ON TABLE knowledge_memory_passages FIELDS passage_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_memory_passages_workspace ON TABLE knowledge_memory_passages FIELDS workspace_id, compaction_policy;
DEFINE INDEX OVERWRITE idx_knowledge_memory_passages_freshness ON TABLE knowledge_memory_passages FIELDS freshness_at;

DEFINE TABLE OVERWRITE knowledge_passage_evidence SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE passage_id ON TABLE knowledge_passage_evidence TYPE record<knowledge_memory_passages> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE ref_kind ON TABLE knowledge_passage_evidence TYPE 'source' | 'claim' | 'span';
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_passage_evidence TYPE option<record<knowledge_sources>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_passage_evidence TYPE option<record<knowledge_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_passage_evidence TYPE option<record<knowledge_spans>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE ordinal ON TABLE knowledge_passage_evidence TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_passage_evidence TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_passage_evidence ON TABLE knowledge_passage_evidence FIELDS passage_id, ref_kind, ordinal UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_passage_evidence_source ON TABLE knowledge_passage_evidence FIELDS source_id;
DEFINE INDEX OVERWRITE idx_knowledge_passage_evidence_claim ON TABLE knowledge_passage_evidence FIELDS claim_id;

-- 0139_knowledge_wiki_projections.sql
DEFINE TABLE OVERWRITE knowledge_wiki_projections SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE projection_id ON TABLE knowledge_wiki_projections TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_wiki_projections TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE projection_kind ON TABLE knowledge_wiki_projections TYPE 'wiki_page' | 'loom_view' | 'graph_view' | 'manual_page' | 'operator_summary';
DEFINE FIELD OVERWRITE title ON TABLE knowledge_wiki_projections TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE source_records ON TABLE knowledge_wiki_projections TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE source_records.* ON TABLE knowledge_wiki_projections TYPE any;
DEFINE FIELD OVERWRITE rendered_content ON TABLE knowledge_wiki_projections TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE rebuild_status ON TABLE knowledge_wiki_projections TYPE 'fresh' | 'stale' | 'rebuilding' | 'failed' DEFAULT 'stale';
DEFINE FIELD OVERWRITE staleness_hash ON TABLE knowledge_wiki_projections TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE rebuild_receipt_event_id ON TABLE knowledge_wiki_projections TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE last_rebuilt_at ON TABLE knowledge_wiki_projections TYPE option<datetime>;
-- 0300/0301 (MT-241..243) typed compiled-page columns, carried by the
-- KnowledgeWikiProjection row type: typed page kind, compile stamp/recipe,
-- and outbound wikilinks.
DEFINE FIELD OVERWRITE page_type ON TABLE knowledge_wiki_projections TYPE option<'module' | 'concept' | 'flow' | 'entity' | 'decision' | 'index'>;
DEFINE FIELD OVERWRITE compile_stamp ON TABLE knowledge_wiki_projections TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE compile_stamp.* ON TABLE knowledge_wiki_projections TYPE any;
DEFINE FIELD OVERWRITE compile_recipe ON TABLE knowledge_wiki_projections TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE compile_recipe.* ON TABLE knowledge_wiki_projections TYPE any;
DEFINE FIELD OVERWRITE page_links ON TABLE knowledge_wiki_projections TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE page_links.* ON TABLE knowledge_wiki_projections TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_wiki_projections TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_wiki_projections TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_wiki_projections ON TABLE knowledge_wiki_projections FIELDS projection_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_wiki_projections_identity ON TABLE knowledge_wiki_projections FIELDS workspace_id, projection_kind, title UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_wiki_projections_status ON TABLE knowledge_wiki_projections FIELDS workspace_id, rebuild_status;

-- 0140_knowledge_rich_documents.sql
DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_rich_documents TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_documents TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_id ON TABLE knowledge_rich_documents TYPE option<record<documents>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE title ON TABLE knowledge_rich_documents TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE schema_version ON TABLE knowledge_rich_documents TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE doc_version ON TABLE knowledge_rich_documents TYPE int DEFAULT 1 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE content_json ON TABLE knowledge_rich_documents TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE content_json.* ON TABLE knowledge_rich_documents TYPE any;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE knowledge_rich_documents TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE crdt_snapshot_id ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE promotion_receipt_event_id ON TABLE knowledge_rich_documents TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE projection_refs ON TABLE knowledge_rich_documents TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE projection_refs.* ON TABLE knowledge_rich_documents TYPE any;
-- 0283+ (MT-145 RichDocumentIdentityModel) identity columns plus the
-- soft-delete tombstone the live-document queries filter on. The
-- owner_actor_kind/owner_actor_id all-or-nothing rule is enforced by the
-- storage layer, mirroring the removed backend's Rust-side validation.
DEFINE FIELD OVERWRITE project_ref ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE folder_ref ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE authority_label ON TABLE knowledge_rich_documents TYPE 'draft' | 'promoted' | 'archived' DEFAULT 'promoted';
DEFINE FIELD OVERWRITE owner_actor_kind ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE owner_actor_id ON TABLE knowledge_rich_documents TYPE option<string>;
DEFINE FIELD OVERWRITE deleted_at ON TABLE knowledge_rich_documents TYPE option<datetime>;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_documents TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_documents TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_documents ON TABLE knowledge_rich_documents FIELDS rich_document_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_rich_documents_workspace ON TABLE knowledge_rich_documents FIELDS workspace_id;

DEFINE TABLE OVERWRITE knowledge_rich_document_versions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_rich_document_versions TYPE record<knowledge_rich_documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE doc_version ON TABLE knowledge_rich_document_versions TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE schema_version ON TABLE knowledge_rich_document_versions TYPE string;
DEFINE FIELD OVERWRITE content_json ON TABLE knowledge_rich_document_versions TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE content_json.* ON TABLE knowledge_rich_document_versions TYPE any;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE knowledge_rich_document_versions TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE crdt_snapshot_id ON TABLE knowledge_rich_document_versions TYPE option<string>;
DEFINE FIELD OVERWRITE promotion_receipt_event_id ON TABLE knowledge_rich_document_versions TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_versions TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_versions ON TABLE knowledge_rich_document_versions FIELDS rich_document_id, doc_version UNIQUE;

DEFINE TABLE OVERWRITE knowledge_editor_code_nodes SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE code_node_id ON TABLE knowledge_editor_code_nodes TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_editor_code_nodes TYPE record<knowledge_rich_documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE node_path ON TABLE knowledge_editor_code_nodes TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE language_id ON TABLE knowledge_editor_code_nodes TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE code_text ON TABLE knowledge_editor_code_nodes TYPE string;
DEFINE FIELD OVERWRITE round_trip_sha256 ON TABLE knowledge_editor_code_nodes TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE worker_requirements ON TABLE knowledge_editor_code_nodes TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE worker_requirements.* ON TABLE knowledge_editor_code_nodes TYPE any;
DEFINE FIELD OVERWRITE source_mapping ON TABLE knowledge_editor_code_nodes TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE source_mapping.* ON TABLE knowledge_editor_code_nodes TYPE any;
DEFINE FIELD OVERWRITE lint_diagnostics ON TABLE knowledge_editor_code_nodes TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE lint_diagnostics.* ON TABLE knowledge_editor_code_nodes TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_editor_code_nodes TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_editor_code_nodes TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_editor_code_nodes ON TABLE knowledge_editor_code_nodes FIELDS code_node_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_editor_code_nodes_path ON TABLE knowledge_editor_code_nodes FIELDS rich_document_id, node_path UNIQUE;

-- 0141_knowledge_context_bundles.sql
DEFINE TABLE OVERWRITE knowledge_context_bundles SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE bundle_id ON TABLE knowledge_context_bundles TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_context_bundles TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kernel_task_run_id ON TABLE knowledge_context_bundles TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE session_run_id ON TABLE knowledge_context_bundles TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE allowed_context ON TABLE knowledge_context_bundles TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE allowed_context.* ON TABLE knowledge_context_bundles TYPE any;
DEFINE FIELD OVERWRITE context_hash ON TABLE knowledge_context_bundles TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE query_text ON TABLE knowledge_context_bundles TYPE option<string>;
DEFINE FIELD OVERWRITE token_budget ON TABLE knowledge_context_bundles TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE tokens_used ON TABLE knowledge_context_bundles TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE build_receipt_event_id ON TABLE knowledge_context_bundles TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_context_bundles TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_context_bundles ON TABLE knowledge_context_bundles FIELDS bundle_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_context_bundles_workspace ON TABLE knowledge_context_bundles FIELDS workspace_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_context_bundles_session ON TABLE knowledge_context_bundles FIELDS session_run_id;

DEFINE TABLE OVERWRITE knowledge_context_bundle_items SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE bundle_id ON TABLE knowledge_context_bundle_items TYPE record<knowledge_context_bundles> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE item_ordinal ON TABLE knowledge_context_bundle_items TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE ref_kind ON TABLE knowledge_context_bundle_items TYPE 'source' | 'span' | 'claim' | 'passage' | 'entity';
DEFINE FIELD OVERWRITE ref_id ON TABLE knowledge_context_bundle_items TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE retrieval_decision ON TABLE knowledge_context_bundle_items TYPE 'included' | 'excluded_budget' | 'excluded_relevance' | 'excluded_redacted';
DEFINE FIELD OVERWRITE relevance_score ON TABLE knowledge_context_bundle_items TYPE option<float> ASSERT $value = NONE OR ($value >= 0.0 AND $value <= 1.0);
DEFINE FIELD OVERWRITE token_count ON TABLE knowledge_context_bundle_items TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE citation ON TABLE knowledge_context_bundle_items TYPE option<string>;
-- MT-186 citation-support columns carried by the KnowledgeContextBundleItem
-- row type: an unsupported item must state why, a supported item must not.
DEFINE FIELD OVERWRITE supported ON TABLE knowledge_context_bundle_items TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE unsupported_reason ON TABLE knowledge_context_bundle_items TYPE option<string> ASSERT ($this.supported = false) = ($value != NONE);
DEFINE INDEX OVERWRITE pk_knowledge_context_bundle_items ON TABLE knowledge_context_bundle_items FIELDS bundle_id, item_ordinal UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_context_bundle_items_ref ON TABLE knowledge_context_bundle_items FIELDS ref_kind, ref_id;

DEFINE TABLE OVERWRITE knowledge_retrieval_traces SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE trace_id ON TABLE knowledge_retrieval_traces TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_retrieval_traces TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE retrieval_mode ON TABLE knowledge_retrieval_traces TYPE 'none' | 'direct_load' | 'exact_lookup' | 'graph_traversal' | 'hybrid_rag';
DEFINE FIELD OVERWRITE mode_reason ON TABLE knowledge_retrieval_traces TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE query_text ON TABLE knowledge_retrieval_traces TYPE option<string>;
DEFINE FIELD OVERWRITE bundle_id ON TABLE knowledge_retrieval_traces TYPE option<record<knowledge_context_bundles>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE decisions ON TABLE knowledge_retrieval_traces TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE decisions.* ON TABLE knowledge_retrieval_traces TYPE any;
DEFINE FIELD OVERWRITE trace_receipt_event_id ON TABLE knowledge_retrieval_traces TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_retrieval_traces TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_retrieval_traces ON TABLE knowledge_retrieval_traces FIELDS trace_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_retrieval_traces_workspace ON TABLE knowledge_retrieval_traces FIELDS workspace_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_retrieval_traces_bundle ON TABLE knowledge_retrieval_traces FIELDS bundle_id;

-- 0142_knowledge_idempotency_keys.sql
DEFINE TABLE OVERWRITE knowledge_idempotency_keys SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE knowledge_idempotency_keys TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_idempotency_keys TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE operation_kind ON TABLE knowledge_idempotency_keys TYPE 'index_run_start' | 'source_upsert' | 'span_write' | 'entity_write' | 'edge_write' | 'claim_write' | 'passage_write' | 'projection_write' | 'rich_document_save' | 'bundle_build';
DEFINE FIELD OVERWRITE request_hash ON TABLE knowledge_idempotency_keys TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE result_ref_kind ON TABLE knowledge_idempotency_keys TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE result_ref_id ON TABLE knowledge_idempotency_keys TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_idempotency_keys TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_idempotency_keys ON TABLE knowledge_idempotency_keys FIELDS idempotency_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_idempotency_keys_workspace ON TABLE knowledge_idempotency_keys FIELDS workspace_id, operation_kind;

-- 0150-0155 WP-KERNEL-009 CRDTAndConcurrencyCore band. `workspace_id` is a
-- plain token in this band (the source migrations carry no workspace foreign
-- key), so it stays a string here.

-- 0150_knowledge_crdt_denial_receipts.sql
DEFINE TABLE OVERWRITE knowledge_crdt_denial_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT $value = record::id($this.id);
-- 0333_loom_ai_suggestions.sql re-states this allow-list as the twelve-value
-- superset that admits the AI-Loom promotion denial kind.
DEFINE FIELD OVERWRITE receipt_kind ON TABLE knowledge_crdt_denial_receipts TYPE 'stale_draft_save' | 'concurrent_draft_fork' | 'ahead_of_head_save' | 'update_content_mismatch' | 'sequence_slot_race' | 'lease_write_denied' | 'index_run_slot_rejected' | 'graph_promotion_denied' | 'ai_edit_promotion_denied' | 'ai_edit_applied_mismatch' | 'ai_edit_applied_update_missing' | 'loom_ai_promotion_denied';
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE document_id ON TABLE knowledge_crdt_denial_receipts TYPE option<string>;
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE knowledge_crdt_denial_receipts TYPE option<string>;
DEFINE FIELD OVERWRITE scope_ref ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_crdt_denial_receipts TYPE 'operator' | 'local_model' | 'cloud_model' | 'validator' | 'system';
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE correlation_id ON TABLE knowledge_crdt_denial_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE denial_payload ON TABLE knowledge_crdt_denial_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE denial_payload.* ON TABLE knowledge_crdt_denial_receipts TYPE any;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_crdt_denial_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE knowledge_crdt_denial_receipts TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_crdt_denial_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_denial_receipts ON TABLE knowledge_crdt_denial_receipts FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_crdt_denial_receipts_idempotency ON TABLE knowledge_crdt_denial_receipts FIELDS idempotency_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_denial_receipts_document ON TABLE knowledge_crdt_denial_receipts FIELDS crdt_document_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_denial_receipts_scope ON TABLE knowledge_crdt_denial_receipts FIELDS scope_ref, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_denial_receipts_actor ON TABLE knowledge_crdt_denial_receipts FIELDS actor_id, created_at;

-- 0151_knowledge_crdt_agent_lane_leases.sql
DEFINE TABLE OVERWRITE knowledge_crdt_agent_lane_leases SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE lease_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE document_id ON TABLE knowledge_crdt_agent_lane_leases TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE knowledge_crdt_agent_lane_leases TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_crdt_agent_lane_leases TYPE 'operator' | 'local_model' | 'cloud_model' | 'validator' | 'system';
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE correlation_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE scope_kind ON TABLE knowledge_crdt_agent_lane_leases TYPE 'workspace' | 'document' | 'source_root' | 'index_run';
DEFINE FIELD OVERWRITE scope_id ON TABLE knowledge_crdt_agent_lane_leases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claimed_at_utc ON TABLE knowledge_crdt_agent_lane_leases TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE knowledge_crdt_agent_lane_leases TYPE datetime ASSERT $value > $this.claimed_at_utc;
DEFINE FIELD OVERWRITE renewal_count ON TABLE knowledge_crdt_agent_lane_leases TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE released_at_utc ON TABLE knowledge_crdt_agent_lane_leases TYPE option<datetime>;
DEFINE FIELD OVERWRITE expired_at_utc ON TABLE knowledge_crdt_agent_lane_leases TYPE option<datetime>;
DEFINE FIELD OVERWRITE takeover_of ON TABLE knowledge_crdt_agent_lane_leases TYPE option<record<knowledge_crdt_agent_lane_leases>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE last_transition_event_id ON TABLE knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
-- Stored discriminator for the source partial UNIQUE index
-- `uq_knowledge_crdt_lease_active_scope ... WHERE released_at_utc IS NULL`.
DEFINE FIELD OVERWRITE active_scope_key ON TABLE knowledge_crdt_agent_lane_leases TYPE string
    VALUE IF $this.released_at_utc = NONE {
        'active:' + $this.owner_account_id + ':' + $this.actor_principal_id + ':' + $this.authenticated_session_id + ':' + $this.access_space_id + ':' + $this.workspace_id + ':' + type::string($this.scope_kind) + ':' + $this.scope_id
    } ELSE {
        'released:' + $this.owner_account_id + ':' + $this.lease_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_crdt_agent_lane_leases ON TABLE knowledge_crdt_agent_lane_leases FIELDS lease_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_crdt_lease_active_scope ON TABLE knowledge_crdt_agent_lane_leases FIELDS active_scope_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_lease_session ON TABLE knowledge_crdt_agent_lane_leases FIELDS session_id;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_lease_expiry ON TABLE knowledge_crdt_agent_lane_leases FIELDS expires_at_utc;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_lease_event ON TABLE knowledge_crdt_agent_lane_leases FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, last_transition_event_id;
DEFINE INDEX OVERWRITE uq_knowledge_crdt_lease_scope_identity ON TABLE knowledge_crdt_agent_lane_leases FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, lease_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_lease_scope_document ON TABLE knowledge_crdt_agent_lane_leases FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, scope_kind, scope_id, expires_at_utc;

-- 0152_knowledge_crdt_graph_proposals.sql
DEFINE TABLE OVERWRITE knowledge_crdt_graph_proposals SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE knowledge_crdt_graph_proposals TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_crdt_graph_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE mutation_kind ON TABLE knowledge_crdt_graph_proposals TYPE 'add_entity' | 'retire_entity' | 'add_edge' | 'retire_edge' | 'add_claim' | 'retire_claim';
DEFINE FIELD OVERWRITE mutation_payload ON TABLE knowledge_crdt_graph_proposals TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE mutation_payload.* ON TABLE knowledge_crdt_graph_proposals TYPE any;
DEFINE FIELD OVERWRITE source_span_refs ON TABLE knowledge_crdt_graph_proposals TYPE array<string> ASSERT array::len($value) >= 1;
DEFINE FIELD OVERWRITE confidence ON TABLE knowledge_crdt_graph_proposals TYPE float ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_crdt_graph_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_crdt_graph_proposals TYPE 'operator' | 'local_model' | 'cloud_model' | 'validator' | 'system';
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_crdt_graph_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE correlation_id ON TABLE knowledge_crdt_graph_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE lease_id ON TABLE knowledge_crdt_graph_proposals TYPE option<record<knowledge_crdt_agent_lane_leases>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE review_state ON TABLE knowledge_crdt_graph_proposals TYPE 'proposed' | 'approved' | 'rejected' | 'promoted' DEFAULT 'proposed';
DEFINE FIELD OVERWRITE decided_by ON TABLE knowledge_crdt_graph_proposals TYPE option<string>;
DEFINE FIELD OVERWRITE decided_at_utc ON TABLE knowledge_crdt_graph_proposals TYPE option<datetime>;
DEFINE FIELD OVERWRITE decision_reason ON TABLE knowledge_crdt_graph_proposals TYPE option<string>;
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_crdt_graph_proposals TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE decided_event_id ON TABLE knowledge_crdt_graph_proposals TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_crdt_graph_proposals TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_graph_proposals ON TABLE knowledge_crdt_graph_proposals FIELDS proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_graph_proposals_workspace_state ON TABLE knowledge_crdt_graph_proposals FIELDS workspace_id, review_state, created_at_utc;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_graph_proposals_actor ON TABLE knowledge_crdt_graph_proposals FIELDS actor_id, created_at_utc;

-- 0153_knowledge_crdt_promoted_facts.sql
DEFINE TABLE OVERWRITE knowledge_crdt_promoted_facts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE fact_id ON TABLE knowledge_crdt_promoted_facts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE proposal_id ON TABLE knowledge_crdt_promoted_facts TYPE record<knowledge_crdt_graph_proposals> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_crdt_promoted_facts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE mutation_kind ON TABLE knowledge_crdt_promoted_facts TYPE 'add_entity' | 'retire_entity' | 'add_edge' | 'retire_edge' | 'add_claim' | 'retire_claim';
DEFINE FIELD OVERWRITE fact_payload ON TABLE knowledge_crdt_promoted_facts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE fact_payload.* ON TABLE knowledge_crdt_promoted_facts TYPE any;
DEFINE FIELD OVERWRITE source_span_refs ON TABLE knowledge_crdt_promoted_facts TYPE array<string> ASSERT array::len($value) >= 1;
DEFINE FIELD OVERWRITE confidence ON TABLE knowledge_crdt_promoted_facts TYPE float ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD OVERWRITE proposed_by ON TABLE knowledge_crdt_promoted_facts TYPE string;
DEFINE FIELD OVERWRITE promoted_by ON TABLE knowledge_crdt_promoted_facts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE promotion_requested_event_id ON TABLE knowledge_crdt_promoted_facts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_accepted_event_id ON TABLE knowledge_crdt_promoted_facts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promoted_at_utc ON TABLE knowledge_crdt_promoted_facts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_promoted_facts ON TABLE knowledge_crdt_promoted_facts FIELDS fact_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_crdt_promoted_facts_proposal ON TABLE knowledge_crdt_promoted_facts FIELDS proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_promoted_facts_workspace ON TABLE knowledge_crdt_promoted_facts FIELDS workspace_id, promoted_at_utc;

-- 0154_knowledge_crdt_ai_edit_proposals.sql
DEFINE TABLE OVERWRITE knowledge_crdt_ai_edit_proposals SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE document_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE crdt_document_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE base_update_seq ON TABLE knowledge_crdt_ai_edit_proposals TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE base_state_vector ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::starts_with($value, 'hsk-sv1:');
DEFINE FIELD OVERWRITE proposed_diff ON TABLE knowledge_crdt_ai_edit_proposals TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE proposed_diff.* ON TABLE knowledge_crdt_ai_edit_proposals TYPE any;
DEFINE FIELD OVERWRITE diff_sha256 ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE source_span_citations ON TABLE knowledge_crdt_ai_edit_proposals TYPE array;
DEFINE FIELD OVERWRITE source_span_citations.* ON TABLE knowledge_crdt_ai_edit_proposals TYPE any;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_crdt_ai_edit_proposals TYPE 'local_model' | 'cloud_model';
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE correlation_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE lease_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<record<knowledge_crdt_agent_lane_leases>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE review_state ON TABLE knowledge_crdt_ai_edit_proposals TYPE 'proposed' | 'approved' | 'rejected' | 'promoted' DEFAULT 'proposed';
DEFINE FIELD OVERWRITE decided_by ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string>;
DEFINE FIELD OVERWRITE decided_at_utc ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<datetime>;
DEFINE FIELD OVERWRITE decision_reason ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string>;
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE decided_event_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_requested_event_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_accepted_event_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE last_transition_event_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE applied_update_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string>;
DEFINE FIELD OVERWRITE applied_update_sha256 ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_crdt_ai_edit_proposals TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_ai_edit_proposals ON TABLE knowledge_crdt_ai_edit_proposals FIELDS proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_ai_edit_proposals_document ON TABLE knowledge_crdt_ai_edit_proposals FIELDS crdt_document_id, review_state, created_at_utc;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_ai_edit_proposals_actor ON TABLE knowledge_crdt_ai_edit_proposals FIELDS actor_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_ai_edit_proposal_event ON TABLE knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, last_transition_event_id;
DEFINE INDEX OVERWRITE uq_knowledge_crdt_ai_edit_proposal_scope_identity ON TABLE knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_ai_edit_proposal_scope_document ON TABLE knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, review_state, created_at_utc;

-- 0155_knowledge_crdt_swarm_checkpoints.sql
DEFINE TABLE OVERWRITE knowledge_crdt_swarm_checkpoints SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE lease_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE record<knowledge_crdt_agent_lane_leases> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE scope_ref ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE resume_pointer ON TABLE knowledge_crdt_swarm_checkpoints TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE resume_pointer.* ON TABLE knowledge_crdt_swarm_checkpoints TYPE any;
DEFINE FIELD OVERWRITE checkpoint_payload ON TABLE knowledge_crdt_swarm_checkpoints TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE checkpoint_payload.* ON TABLE knowledge_crdt_swarm_checkpoints TYPE any;
DEFINE FIELD OVERWRITE payload_sha256 ON TABLE knowledge_crdt_swarm_checkpoints TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_crdt_swarm_checkpoints TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_crdt_swarm_checkpoints TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_swarm_checkpoints ON TABLE knowledge_crdt_swarm_checkpoints FIELDS checkpoint_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_swarm_checkpoints_session ON TABLE knowledge_crdt_swarm_checkpoints FIELDS session_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_swarm_checkpoints_lane ON TABLE knowledge_crdt_swarm_checkpoints FIELDS lane_id, created_at_utc;

DEFINE TABLE OVERWRITE knowledge_crdt_recovery_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_crdt_recovery_receipts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE knowledge_crdt_recovery_receipts TYPE record<knowledge_crdt_swarm_checkpoints> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE prior_session_id ON TABLE knowledge_crdt_recovery_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE new_session_id ON TABLE knowledge_crdt_recovery_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE new_actor_id ON TABLE knowledge_crdt_recovery_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE new_lease_id ON TABLE knowledge_crdt_recovery_receipts TYPE record<knowledge_crdt_agent_lane_leases> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE lease_lineage ON TABLE knowledge_crdt_recovery_receipts TYPE array;
DEFINE FIELD OVERWRITE lease_lineage.* ON TABLE knowledge_crdt_recovery_receipts TYPE any;
DEFINE FIELD OVERWRITE resume_pointer ON TABLE knowledge_crdt_recovery_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE resume_pointer.* ON TABLE knowledge_crdt_recovery_receipts TYPE any;
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_crdt_recovery_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_crdt_recovery_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_crdt_recovery_receipts ON TABLE knowledge_crdt_recovery_receipts FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_recovery_receipts_checkpoint ON TABLE knowledge_crdt_recovery_receipts FIELDS checkpoint_id;
DEFINE INDEX OVERWRITE idx_knowledge_crdt_recovery_receipts_new_session ON TABLE knowledge_crdt_recovery_receipts FIELDS new_session_id;

-- 0160_knowledge_ingestion_policies.sql
DEFINE TABLE OVERWRITE knowledge_ingestion_root_policies SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE policy_id ON TABLE knowledge_ingestion_root_policies TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_ingestion_root_policies TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE policy_version ON TABLE knowledge_ingestion_root_policies TYPE int DEFAULT 1 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE allow_patterns ON TABLE knowledge_ingestion_root_policies TYPE array<string> DEFAULT ['**'];
DEFINE FIELD OVERWRITE deny_patterns ON TABLE knowledge_ingestion_root_policies TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE require_operator_approval ON TABLE knowledge_ingestion_root_policies TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE active ON TABLE knowledge_ingestion_root_policies TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_ingestion_root_policies TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_ingestion_root_policies TYPE datetime DEFAULT time::now();
-- Stored discriminator for the source partial UNIQUE index
-- `uq_knowledge_ingestion_root_policies_active ... WHERE active`.
DEFINE FIELD OVERWRITE active_workspace_key ON TABLE knowledge_ingestion_root_policies TYPE string
    VALUE IF $this.active {
        'active:' + type::string($this.workspace_id)
    } ELSE {
        'inactive:' + $this.policy_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_root_policies ON TABLE knowledge_ingestion_root_policies FIELDS policy_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_ingestion_root_policies_active ON TABLE knowledge_ingestion_root_policies FIELDS active_workspace_key UNIQUE;

DEFINE TABLE OVERWRITE knowledge_ingestion_policy_decisions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE decision_id ON TABLE knowledge_ingestion_policy_decisions TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_ingestion_policy_decisions TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE policy_id ON TABLE knowledge_ingestion_policy_decisions TYPE option<record<knowledge_ingestion_root_policies>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE candidate_path ON TABLE knowledge_ingestion_policy_decisions TYPE string;
DEFINE FIELD OVERWRITE root_kind ON TABLE knowledge_ingestion_policy_decisions TYPE 'project_repo' | 'governance' | 'artifacts' | 'media_library' | 'external_import' | 'operator_folder';
DEFINE FIELD OVERWRITE verdict ON TABLE knowledge_ingestion_policy_decisions TYPE 'allowed' | 'denied_pattern' | 'denied_not_allowlisted' | 'denied_requires_approval';
DEFINE FIELD OVERWRITE matched_pattern ON TABLE knowledge_ingestion_policy_decisions TYPE option<string>;
DEFINE FIELD OVERWRITE operator_approved ON TABLE knowledge_ingestion_policy_decisions TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_ingestion_policy_decisions TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_ingestion_policy_decisions TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE receipt_event_id ON TABLE knowledge_ingestion_policy_decisions TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE decided_at ON TABLE knowledge_ingestion_policy_decisions TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_policy_decisions ON TABLE knowledge_ingestion_policy_decisions FIELDS decision_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_policy_decisions_workspace ON TABLE knowledge_ingestion_policy_decisions FIELDS workspace_id, decided_at;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_policy_decisions_verdict ON TABLE knowledge_ingestion_policy_decisions FIELDS verdict;

-- 0161_knowledge_ingestion_kind_registry.sql
DEFINE TABLE OVERWRITE knowledge_ingestion_kind_registry SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE kind_key ON TABLE knowledge_ingestion_kind_registry TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE display_name ON TABLE knowledge_ingestion_kind_registry TYPE string;
DEFINE FIELD OVERWRITE capabilities ON TABLE knowledge_ingestion_kind_registry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE capabilities.* ON TABLE knowledge_ingestion_kind_registry TYPE any;
DEFINE FIELD OVERWRITE extensions ON TABLE knowledge_ingestion_kind_registry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE extensions.* ON TABLE knowledge_ingestion_kind_registry TYPE any;
DEFINE FIELD OVERWRITE mime_types ON TABLE knowledge_ingestion_kind_registry TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE mime_types.* ON TABLE knowledge_ingestion_kind_registry TYPE any;
DEFINE FIELD OVERWRITE registry_version ON TABLE knowledge_ingestion_kind_registry TYPE string;
DEFINE FIELD OVERWRITE projected_at ON TABLE knowledge_ingestion_kind_registry TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_kind_registry ON TABLE knowledge_ingestion_kind_registry FIELDS kind_key UNIQUE;

-- 0162_knowledge_ingestion_receipts.sql
DEFINE TABLE OVERWRITE knowledge_ingestion_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_ingestion_receipts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_ingestion_receipts TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_ingestion_receipts TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE ingestion_run_token ON TABLE knowledge_ingestion_receipts TYPE option<string>;
DEFINE FIELD OVERWRITE extractor_id ON TABLE knowledge_ingestion_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE extractor_version ON TABLE knowledge_ingestion_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE knowledge_ingestion_receipts TYPE 'success' | 'partial' | 'failed' | 'deferred' | 'skipped' | 'blocked';
DEFINE FIELD OVERWRITE error_class ON TABLE knowledge_ingestion_receipts TYPE option<'UNSUPPORTED_FORMAT' | 'NO_TEXT_LAYER' | 'OCR_NEEDED' | 'ENCRYPTED' | 'MALFORMED_CUE' | 'PARSE_ERROR' | 'OVERSIZE' | 'SECRET_BLOCKED' | 'IO_ERROR' | 'INTERNAL'>;
DEFINE FIELD OVERWRITE error_detail ON TABLE knowledge_ingestion_receipts TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE error_detail.* ON TABLE knowledge_ingestion_receipts TYPE any;
DEFINE FIELD OVERWRITE spans_produced ON TABLE knowledge_ingestion_receipts TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE spans_failed ON TABLE knowledge_ingestion_receipts TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE redaction_count ON TABLE knowledge_ingestion_receipts TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE content_hash ON TABLE knowledge_ingestion_receipts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE duration_ms ON TABLE knowledge_ingestion_receipts TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE receipt_event_id ON TABLE knowledge_ingestion_receipts TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_ingestion_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_receipts ON TABLE knowledge_ingestion_receipts FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_receipts_source ON TABLE knowledge_ingestion_receipts FIELDS source_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_receipts_workspace ON TABLE knowledge_ingestion_receipts FIELDS workspace_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_receipts_status ON TABLE knowledge_ingestion_receipts FIELDS status;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_receipts_run_token ON TABLE knowledge_ingestion_receipts FIELDS ingestion_run_token;

-- 0163_knowledge_ingestion_spans.sql
DEFINE TABLE OVERWRITE knowledge_ingestion_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE span_id ON TABLE knowledge_ingestion_spans TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_ingestion_spans TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_ingestion_spans TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_ingestion_spans TYPE record<knowledge_ingestion_receipts> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE span_index ON TABLE knowledge_ingestion_spans TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE anchor_kind ON TABLE knowledge_ingestion_spans TYPE 'byte_range' | 'line_range' | 'pdf_page' | 'media_time' | 'json_pointer' | 'heading_path';
DEFINE FIELD OVERWRITE anchor ON TABLE knowledge_ingestion_spans TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE anchor.* ON TABLE knowledge_ingestion_spans TYPE any;
DEFINE FIELD OVERWRITE byte_start ON TABLE knowledge_ingestion_spans TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE byte_end ON TABLE knowledge_ingestion_spans TYPE option<int> ASSERT $value = NONE OR $value >= 0;
DEFINE FIELD OVERWRITE content ON TABLE knowledge_ingestion_spans TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE knowledge_ingestion_spans TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE redaction_state ON TABLE knowledge_ingestion_spans TYPE 'none' | 'redacted' DEFAULT 'none';
DEFINE FIELD OVERWRITE link_candidates ON TABLE knowledge_ingestion_spans TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE link_candidates.* ON TABLE knowledge_ingestion_spans TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_ingestion_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_spans ON TABLE knowledge_ingestion_spans FIELDS span_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_ingestion_spans_receipt_index ON TABLE knowledge_ingestion_spans FIELDS receipt_id, span_index UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_spans_source ON TABLE knowledge_ingestion_spans FIELDS source_id, span_index;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_spans_workspace ON TABLE knowledge_ingestion_spans FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_spans_anchor_kind ON TABLE knowledge_ingestion_spans FIELDS anchor_kind;

-- 0164_knowledge_ingestion_repair_queue.sql
DEFINE TABLE OVERWRITE knowledge_ingestion_repair_queue SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE repair_id ON TABLE knowledge_ingestion_repair_queue TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_ingestion_repair_queue TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_ingestion_repair_queue TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_ingestion_repair_queue TYPE option<record<knowledge_ingestion_receipts>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE reason_class ON TABLE knowledge_ingestion_repair_queue TYPE 'UNSUPPORTED_FORMAT' | 'NO_TEXT_LAYER' | 'OCR_NEEDED' | 'ENCRYPTED' | 'MALFORMED_CUE' | 'PARSE_ERROR' | 'OVERSIZE' | 'SECRET_BLOCKED' | 'IO_ERROR' | 'INTERNAL' | 'MISSING_ASSET' | 'DENIED_PATH' | 'STALE_HASH';
DEFINE FIELD OVERWRITE reason_detail ON TABLE knowledge_ingestion_repair_queue TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE reason_detail.* ON TABLE knowledge_ingestion_repair_queue TYPE any;
DEFINE FIELD OVERWRITE state ON TABLE knowledge_ingestion_repair_queue TYPE 'queued' | 'retrying' | 'resolved' | 'dead_letter' DEFAULT 'queued';
DEFINE FIELD OVERWRITE attempts ON TABLE knowledge_ingestion_repair_queue TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE max_attempts ON TABLE knowledge_ingestion_repair_queue TYPE int DEFAULT 3 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE last_attempt_at ON TABLE knowledge_ingestion_repair_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE resolved_receipt_id ON TABLE knowledge_ingestion_repair_queue TYPE option<record<knowledge_ingestion_receipts>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE enqueue_event_id ON TABLE knowledge_ingestion_repair_queue TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_ingestion_repair_queue TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_ingestion_repair_queue TYPE datetime DEFAULT time::now();
-- Stored discriminator for the source partial UNIQUE index
-- `uq_knowledge_ingestion_repair_open_source ... WHERE state IN (queued, retrying)`.
DEFINE FIELD OVERWRITE open_source_key ON TABLE knowledge_ingestion_repair_queue TYPE string
    VALUE IF $this.state = 'queued' OR $this.state = 'retrying' {
        'open:' + type::string($this.source_id)
    } ELSE {
        'closed:' + $this.repair_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_ingestion_repair_queue ON TABLE knowledge_ingestion_repair_queue FIELDS repair_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_ingestion_repair_open_source ON TABLE knowledge_ingestion_repair_queue FIELDS open_source_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_ingestion_repair_workspace_state ON TABLE knowledge_ingestion_repair_queue FIELDS workspace_id, state, created_at;

-- 0170_knowledge_code_files.sql
DEFINE TABLE OVERWRITE knowledge_code_files SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE code_file_id ON TABLE knowledge_code_files TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_code_files TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_code_files TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE file_entity_id ON TABLE knowledge_code_files TYPE option<record<knowledge_entities>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE language ON TABLE knowledge_code_files TYPE 'rust' | 'javascript' | 'typescript' | 'tsx';
DEFINE FIELD OVERWRITE indexed_content_hash ON TABLE knowledge_code_files TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE parser_version ON TABLE knowledge_code_files TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE parse_status ON TABLE knowledge_code_files TYPE 'parsed' | 'partial' | 'failed' DEFAULT 'parsed';
DEFINE FIELD OVERWRITE stale ON TABLE knowledge_code_files TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE symbols_indexed ON TABLE knowledge_code_files TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE edges_indexed ON TABLE knowledge_code_files TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE failure_detail ON TABLE knowledge_code_files TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE failure_detail.* ON TABLE knowledge_code_files TYPE any;
DEFINE FIELD OVERWRITE last_indexed_in_run ON TABLE knowledge_code_files TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE last_index_receipt_event_id ON TABLE knowledge_code_files TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_code_files TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_code_files TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_code_files ON TABLE knowledge_code_files FIELDS code_file_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_code_files_source ON TABLE knowledge_code_files FIELDS source_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_code_files_workspace ON TABLE knowledge_code_files FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_knowledge_code_files_stale ON TABLE knowledge_code_files FIELDS workspace_id, stale;
DEFINE INDEX OVERWRITE idx_knowledge_code_files_parse_status ON TABLE knowledge_code_files FIELDS workspace_id, parse_status;
DEFINE INDEX OVERWRITE idx_knowledge_code_files_file_entity ON TABLE knowledge_code_files FIELDS file_entity_id;

-- 0171_knowledge_code_scip_imports.sql
DEFINE TABLE OVERWRITE knowledge_code_scip_imports SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE scip_import_id ON TABLE knowledge_code_scip_imports TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_code_scip_imports TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE artifact_format ON TABLE knowledge_code_scip_imports TYPE 'scip' | 'lsif';
DEFINE FIELD OVERWRITE tool_name ON TABLE knowledge_code_scip_imports TYPE option<string>;
DEFINE FIELD OVERWRITE tool_version ON TABLE knowledge_code_scip_imports TYPE option<string>;
DEFINE FIELD OVERWRITE artifact_hash ON TABLE knowledge_code_scip_imports TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE status ON TABLE knowledge_code_scip_imports TYPE 'imported' | 'partial' | 'rejected';
DEFINE FIELD OVERWRITE reason ON TABLE knowledge_code_scip_imports TYPE option<string>;
DEFINE FIELD OVERWRITE symbols_imported ON TABLE knowledge_code_scip_imports TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE occurrences_imported ON TABLE knowledge_code_scip_imports TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE edges_imported ON TABLE knowledge_code_scip_imports TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE import_detail ON TABLE knowledge_code_scip_imports TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE import_detail.* ON TABLE knowledge_code_scip_imports TYPE any;
DEFINE FIELD OVERWRITE imported_in_run ON TABLE knowledge_code_scip_imports TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE import_receipt_event_id ON TABLE knowledge_code_scip_imports TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_code_scip_imports TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_code_scip_imports ON TABLE knowledge_code_scip_imports FIELDS scip_import_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_code_scip_imports_workspace ON TABLE knowledge_code_scip_imports FIELDS workspace_id, created_at;
DEFINE INDEX OVERWRITE idx_knowledge_code_scip_imports_status ON TABLE knowledge_code_scip_imports FIELDS status;

-- 0230_knowledge_code_repair_queue.sql. The seed row inserted into
-- `knowledge_schema_registry` is DML and is intentionally omitted: this
-- bootstrap targets an empty database.
DEFINE TABLE OVERWRITE knowledge_code_repair_queue SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE code_repair_id ON TABLE knowledge_code_repair_queue TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'KCRQ-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_code_repair_queue TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_id ON TABLE knowledge_code_repair_queue TYPE record<knowledge_sources> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE relative_path ON TABLE knowledge_code_repair_queue TYPE string;
DEFINE FIELD OVERWRITE reason_class ON TABLE knowledge_code_repair_queue TYPE 'PARSE_ERROR' | 'READ_ERROR' | 'PANIC' | 'CONFIG_PARSE_ERROR';
DEFINE FIELD OVERWRITE reason_detail ON TABLE knowledge_code_repair_queue TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE reason_detail.* ON TABLE knowledge_code_repair_queue TYPE any;
DEFINE FIELD OVERWRITE state ON TABLE knowledge_code_repair_queue TYPE 'queued' | 'retrying' | 'resolved' | 'dead_letter' DEFAULT 'queued';
DEFINE FIELD OVERWRITE attempts ON TABLE knowledge_code_repair_queue TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE max_attempts ON TABLE knowledge_code_repair_queue TYPE int DEFAULT 3 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE last_attempt_at ON TABLE knowledge_code_repair_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE enqueue_event_id ON TABLE knowledge_code_repair_queue TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE resolved_receipt_event_id ON TABLE knowledge_code_repair_queue TYPE option<record<kernel_event_ledger>> ASSERT ($value = NONE AND $this.state != 'resolved') OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_code_repair_queue TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_code_repair_queue TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_code_repair_queue ON TABLE knowledge_code_repair_queue FIELDS code_repair_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_code_repair_open_source ON TABLE knowledge_code_repair_queue FIELDS source_id, state;
DEFINE INDEX OVERWRITE idx_knowledge_code_repair_workspace_state ON TABLE knowledge_code_repair_queue FIELDS workspace_id, state, created_at;

-- 0240_knowledge_memory_ontology.sql
DEFINE TABLE OVERWRITE knowledge_memory_ontology_terms SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE term_id ON TABLE knowledge_memory_ontology_terms TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_ontology_terms TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE term_kind ON TABLE knowledge_memory_ontology_terms TYPE 'entity_class' | 'relation_class' | 'attribute' | 'extraction_pattern';
DEFINE FIELD OVERWRITE term_key ON TABLE knowledge_memory_ontology_terms TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE normalized_label ON TABLE knowledge_memory_ontology_terms TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE maps_to_edge_type ON TABLE knowledge_memory_ontology_terms TYPE option<'defines' | 'references' | 'contains' | 'depends_on' | 'implements' | 'documents' | 'validates' | 'derived_from' | 'mentions' | 'links_to' | 'supersedes' | 'relates_to'>;
DEFINE FIELD OVERWRITE maps_to_entity_kind ON TABLE knowledge_memory_ontology_terms TYPE option<'symbol' | 'concept' | 'file' | 'folder' | 'project' | 'person' | 'role' | 'task' | 'api' | 'schema' | 'command' | 'media' | 'manual_entry' | 'product_primitive' | 'spec_topic' | 'work_packet' | 'micro_task' | 'taskboard_row' | 'rich_document' | 'loom_block' | 'user_manual_page'> ASSERT $value = NONE OR $this.maps_to_edge_type = NONE;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE knowledge_memory_ontology_terms TYPE 'probationary' | 'stable' | 'retired' DEFAULT 'probationary';
DEFINE FIELD OVERWRITE retirement_reason ON TABLE knowledge_memory_ontology_terms TYPE option<'rejected' | 'superseded' | 'stale' | 'operator_retired'>;
DEFINE FIELD OVERWRITE superseded_by_term_id ON TABLE knowledge_memory_ontology_terms TYPE option<record<knowledge_memory_ontology_terms>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE observation_count ON TABLE knowledge_memory_ontology_terms TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE promotion_threshold ON TABLE knowledge_memory_ontology_terms TYPE int DEFAULT 3 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE operator_approved ON TABLE knowledge_memory_ontology_terms TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE promotion_receipt_event_id ON TABLE knowledge_memory_ontology_terms TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE detection_provenance ON TABLE knowledge_memory_ontology_terms TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE detection_provenance.* ON TABLE knowledge_memory_ontology_terms TYPE any;
DEFINE FIELD OVERWRITE first_seen_in_run ON TABLE knowledge_memory_ontology_terms TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE last_seen_in_run ON TABLE knowledge_memory_ontology_terms TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_ontology_terms TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_memory_ontology_terms TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_ontology_terms ON TABLE knowledge_memory_ontology_terms FIELDS term_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_memory_ontology_terms_identity ON TABLE knowledge_memory_ontology_terms FIELDS workspace_id, term_kind, term_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_memory_ontology_terms_ws_kind_state ON TABLE knowledge_memory_ontology_terms FIELDS workspace_id, term_kind, lifecycle_state;
DEFINE EVENT OVERWRITE enforce_stable_promotion_receipt ON TABLE knowledge_memory_ontology_terms
    WHEN ($event = 'CREATE' OR $event = 'UPDATE')
      AND $after.lifecycle_state = 'stable' AND $after.promotion_receipt_event_id = NONE
    THEN { THROW 'knowledge_memory_ontology_terms.promotion_receipt_event_id is required when lifecycle_state is stable'; };
DEFINE EVENT OVERWRITE enforce_ontology_lifecycle_transition ON TABLE knowledge_memory_ontology_terms
    WHEN $event = 'UPDATE'
      AND $before.lifecycle_state != $after.lifecycle_state
      AND !(($before.lifecycle_state = 'probationary'
             AND ($after.lifecycle_state = 'stable' OR $after.lifecycle_state = 'retired'))
            OR ($before.lifecycle_state = 'stable' AND $after.lifecycle_state = 'retired'))
    THEN { THROW 'knowledge_memory_ontology_terms lifecycle transition is illegal'; };

DEFINE TABLE OVERWRITE knowledge_memory_ontology_aliases SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE alias_id ON TABLE knowledge_memory_ontology_aliases TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE term_id ON TABLE knowledge_memory_ontology_aliases TYPE record<knowledge_memory_ontology_terms> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_ontology_aliases TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE alias_surface ON TABLE knowledge_memory_ontology_aliases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE alias_norm_key ON TABLE knowledge_memory_ontology_aliases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE alias_source ON TABLE knowledge_memory_ontology_aliases TYPE 'extraction' | 'operator' | 'spec' | 'import' DEFAULT 'extraction';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_ontology_aliases TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_ontology_aliases ON TABLE knowledge_memory_ontology_aliases FIELDS alias_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_memory_ontology_aliases_norm ON TABLE knowledge_memory_ontology_aliases FIELDS workspace_id, alias_norm_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_memory_ontology_aliases_term ON TABLE knowledge_memory_ontology_aliases FIELDS term_id;

-- 0241_knowledge_memory_facts.sql
DEFINE TABLE OVERWRITE knowledge_memory_facts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE fact_id ON TABLE knowledge_memory_facts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_facts TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_memory_facts TYPE record<knowledge_claims> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE subject_entity_id ON TABLE knowledge_memory_facts TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE predicate_key ON TABLE knowledge_memory_facts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE predicate_term_id ON TABLE knowledge_memory_facts TYPE option<record<knowledge_memory_ontology_terms>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE object_entity_id ON TABLE knowledge_memory_facts TYPE option<record<knowledge_entities>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE object_literal ON TABLE knowledge_memory_facts TYPE option<string> ASSERT ($value = NONE AND $this.object_entity_id != NONE) OR ($value != NONE AND $this.object_entity_id = NONE);
DEFINE FIELD OVERWRITE qualifiers ON TABLE knowledge_memory_facts TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE qualifiers.* ON TABLE knowledge_memory_facts TYPE any;
DEFINE FIELD OVERWRITE authority_label ON TABLE knowledge_memory_facts TYPE 'source' | 'derived' | 'model_suggested' | 'operator_approved' | 'deprecated' | 'superseded' | 'unsupported' DEFAULT 'model_suggested';
DEFINE FIELD OVERWRITE extractor_version ON TABLE knowledge_memory_facts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_in_run ON TABLE knowledge_memory_facts TYPE option<record<knowledge_index_runs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_facts TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_memory_facts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_facts ON TABLE knowledge_memory_facts FIELDS fact_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_memory_facts_claim ON TABLE knowledge_memory_facts FIELDS claim_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_memory_facts_workspace ON TABLE knowledge_memory_facts FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_knowledge_memory_facts_subject ON TABLE knowledge_memory_facts FIELDS subject_entity_id;
DEFINE INDEX OVERWRITE idx_knowledge_memory_facts_object ON TABLE knowledge_memory_facts FIELDS object_entity_id;
DEFINE INDEX OVERWRITE idx_knowledge_memory_facts_predicate ON TABLE knowledge_memory_facts FIELDS workspace_id, predicate_key;
DEFINE INDEX OVERWRITE idx_knowledge_memory_facts_symbolic_key ON TABLE knowledge_memory_facts FIELDS workspace_id, subject_entity_id, predicate_key;

-- 0242_knowledge_memory_conflict_jobs.sql
DEFINE TABLE OVERWRITE knowledge_memory_conflict_detection_jobs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE knowledge_memory_conflict_detection_jobs TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_conflict_detection_jobs TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE detection_kind ON TABLE knowledge_memory_conflict_detection_jobs TYPE 'symbolic' | 'temporal' | 'alias' | 'stale_source' | 'granularity' | 'semantic';
DEFINE FIELD OVERWRITE job_state ON TABLE knowledge_memory_conflict_detection_jobs TYPE 'queued' | 'running' | 'completed' | 'failed' DEFAULT 'completed';
DEFINE FIELD OVERWRITE candidates_scanned ON TABLE knowledge_memory_conflict_detection_jobs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE conflicts_found ON TABLE knowledge_memory_conflict_detection_jobs TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE search_parameters ON TABLE knowledge_memory_conflict_detection_jobs TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE search_parameters.* ON TABLE knowledge_memory_conflict_detection_jobs TYPE any;
DEFINE FIELD OVERWRITE detection_receipt_event_id ON TABLE knowledge_memory_conflict_detection_jobs TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_conflict_detection_jobs TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE completed_at ON TABLE knowledge_memory_conflict_detection_jobs TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_knowledge_memory_conflict_detection_jobs ON TABLE knowledge_memory_conflict_detection_jobs FIELDS job_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kmcdj_workspace ON TABLE knowledge_memory_conflict_detection_jobs FIELDS workspace_id, detection_kind;

DEFINE TABLE OVERWRITE knowledge_memory_conflict_detection_findings SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE knowledge_memory_conflict_detection_findings TYPE record<knowledge_memory_conflict_detection_jobs> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE conflict_id ON TABLE knowledge_memory_conflict_detection_findings TYPE record<knowledge_claim_conflicts> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_conflict_detection_findings TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_conflict_detection_findings ON TABLE knowledge_memory_conflict_detection_findings FIELDS job_id, conflict_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kmcdf_conflict ON TABLE knowledge_memory_conflict_detection_findings FIELDS conflict_id;

DEFINE TABLE OVERWRITE knowledge_memory_conflict_resolution_jobs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE job_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE conflict_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE record<knowledge_claim_conflicts> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE outcome ON TABLE knowledge_memory_conflict_resolution_jobs TYPE 'discard' | 'refine' | 'temporal_qualify' | 'granularity_qualify' | 'merge';
DEFINE FIELD OVERWRITE kept_claim_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE option<record<knowledge_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE discarded_claim_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE option<record<knowledge_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE resolution_detail ON TABLE knowledge_memory_conflict_resolution_jobs TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE resolution_detail.* ON TABLE knowledge_memory_conflict_resolution_jobs TYPE any;
DEFINE FIELD OVERWRITE resolution_receipt_event_id ON TABLE knowledge_memory_conflict_resolution_jobs TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_conflict_resolution_jobs TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_conflict_resolution_jobs ON TABLE knowledge_memory_conflict_resolution_jobs FIELDS job_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kmcrj_workspace ON TABLE knowledge_memory_conflict_resolution_jobs FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_kmcrj_conflict ON TABLE knowledge_memory_conflict_resolution_jobs FIELDS conflict_id;

-- 0243_knowledge_memory_bridge_edges.sql
DEFINE TABLE OVERWRITE knowledge_memory_bridge_decisions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE decision_id ON TABLE knowledge_memory_bridge_decisions TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_memory_bridge_decisions TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE entity_id_a ON TABLE knowledge_memory_bridge_decisions TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE entity_id_b ON TABLE knowledge_memory_bridge_decisions TYPE record<knowledge_entities> ASSERT record::exists($value) AND $value != $this.entity_id_a REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE decision ON TABLE knowledge_memory_bridge_decisions TYPE 'bridged' | 'suppressed_hub' | 'suppressed_no_evidence' | 'suppressed_connected';
DEFINE FIELD OVERWRITE degree_a ON TABLE knowledge_memory_bridge_decisions TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE degree_b ON TABLE knowledge_memory_bridge_decisions TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE hub_degree_threshold ON TABLE knowledge_memory_bridge_decisions TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE evidence_span_id ON TABLE knowledge_memory_bridge_decisions TYPE option<record<knowledge_spans>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE bridge_edge_id ON TABLE knowledge_memory_bridge_decisions TYPE option<record<knowledge_edges>> ASSERT ($value = NONE OR record::exists($value)) AND ($this.decision = 'bridged' OR $value = NONE) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_memory_bridge_decisions TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_memory_bridge_decisions ON TABLE knowledge_memory_bridge_decisions FIELDS decision_id UNIQUE;
DEFINE INDEX OVERWRITE idx_kmbd_workspace ON TABLE knowledge_memory_bridge_decisions FIELDS workspace_id, decision;
DEFINE INDEX OVERWRITE idx_kmbd_edge ON TABLE knowledge_memory_bridge_decisions FIELDS bridge_edge_id;

-- 0260_knowledge_semantic_catalog.sql
DEFINE TABLE OVERWRITE knowledge_semantic_catalog_entries SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE entry_id ON TABLE knowledge_semantic_catalog_entries TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_semantic_catalog_entries TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE entry_kind ON TABLE knowledge_semantic_catalog_entries TYPE 'entity_type' | 'index' | 'view' | 'tool';
DEFINE FIELD OVERWRITE name ON TABLE knowledge_semantic_catalog_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE version ON TABLE knowledge_semantic_catalog_entries TYPE int DEFAULT 1 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE description ON TABLE knowledge_semantic_catalog_entries TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE query_routes ON TABLE knowledge_semantic_catalog_entries TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE supported_selectors ON TABLE knowledge_semantic_catalog_entries TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE default_budgets ON TABLE knowledge_semantic_catalog_entries TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE default_budgets.* ON TABLE knowledge_semantic_catalog_entries TYPE any;
DEFINE FIELD OVERWRITE examples ON TABLE knowledge_semantic_catalog_entries TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE examples.* ON TABLE knowledge_semantic_catalog_entries TYPE any;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE knowledge_semantic_catalog_entries TYPE 'active' | 'retired' DEFAULT 'active';
DEFINE FIELD OVERWRITE last_updated_at ON TABLE knowledge_semantic_catalog_entries TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_semantic_catalog_entries TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_semantic_catalog_entries ON TABLE knowledge_semantic_catalog_entries FIELDS entry_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_semantic_catalog_name_version ON TABLE knowledge_semantic_catalog_entries FIELDS workspace_id, name, version UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_semantic_catalog_kind ON TABLE knowledge_semantic_catalog_entries FIELDS workspace_id, entry_kind, lifecycle_state;

-- 0281_knowledge_document_embeds.sql. The `knowledge_schema_registry` seed row
-- is DML and is intentionally omitted; this bootstrap targets an empty database.
DEFINE TABLE OVERWRITE knowledge_document_embeds SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE embed_id ON TABLE knowledge_document_embeds TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'KEMB-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_document_embeds TYPE record<knowledge_rich_documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE block_id ON TABLE knowledge_document_embeds TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE ref_kind ON TABLE knowledge_document_embeds TYPE 'artifact' | 'media' | 'source' | 'url';
DEFINE FIELD OVERWRITE ref_value ON TABLE knowledge_document_embeds TYPE string ASSERT string::trim($value) != '' AND !string::starts_with($value, '/') AND !string::starts_with($value, '\\\\') AND !string::starts_with(string::lowercase($value), 'file:') AND ($this.ref_kind != 'url' OR string::starts_with(string::lowercase($value), 'http'));
DEFINE FIELD OVERWRITE caption ON TABLE knowledge_document_embeds TYPE option<string>;
DEFINE FIELD OVERWRITE repair_state ON TABLE knowledge_document_embeds TYPE 'ok' | 'broken' DEFAULT 'ok';
DEFINE FIELD OVERWRITE repair_reason ON TABLE knowledge_document_embeds TYPE option<string> ASSERT ($this.repair_state = 'broken') = ($value != NONE);
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_document_embeds TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_document_embeds TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_document_embeds ON TABLE knowledge_document_embeds FIELDS embed_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_document_embeds_block ON TABLE knowledge_document_embeds FIELDS rich_document_id, block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_document_embeds_document ON TABLE knowledge_document_embeds FIELDS rich_document_id;
DEFINE INDEX OVERWRITE idx_knowledge_document_embeds_broken ON TABLE knowledge_document_embeds FIELDS repair_state, rich_document_id;

-- 0282_knowledge_document_backlinks.sql
DEFINE TABLE OVERWRITE knowledge_document_backlinks SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE backlink_id ON TABLE knowledge_document_backlinks TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'KDBL-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_document_backlinks TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE relationship_id ON TABLE knowledge_document_backlinks TYPE string ASSERT string::starts_with($value, 'KDLNK-') AND string::len($value) = 70;
DEFINE FIELD OVERWRITE source_document_id ON TABLE knowledge_document_backlinks TYPE record<knowledge_rich_documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE link_kind ON TABLE knowledge_document_backlinks TYPE 'file' | 'folder' | 'project' | 'spec' | 'wp' | 'symbol' | 'wikilink' | 'mention' | 'tag';
DEFINE FIELD OVERWRITE target ON TABLE knowledge_document_backlinks TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE block_id ON TABLE knowledge_document_backlinks TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_document_backlinks TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_document_backlinks TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_document_backlinks ON TABLE knowledge_document_backlinks FIELDS backlink_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_document_backlinks_relationship ON TABLE knowledge_document_backlinks FIELDS workspace_id, relationship_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_document_backlinks_source ON TABLE knowledge_document_backlinks FIELDS source_document_id;
DEFINE INDEX OVERWRITE idx_knowledge_document_backlinks_target ON TABLE knowledge_document_backlinks FIELDS workspace_id, link_kind, target;

-- 0292_loom_block_knowledge_bridge.sql
DEFINE TABLE OVERWRITE loom_block_knowledge_bridge SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_knowledge_bridge TYPE record<loom_blocks> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_block_knowledge_bridge TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE entity_id ON TABLE loom_block_knowledge_bridge TYPE record<knowledge_entities> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE index_event_id ON TABLE loom_block_knowledge_bridge TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_block_knowledge_bridge TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_block_knowledge_bridge TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_block_knowledge_bridge ON TABLE loom_block_knowledge_bridge FIELDS block_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_block_knowledge_bridge_entity ON TABLE loom_block_knowledge_bridge FIELDS entity_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_block_knowledge_bridge_workspace ON TABLE loom_block_knowledge_bridge FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_loom_block_knowledge_bridge_entity ON TABLE loom_block_knowledge_bridge FIELDS entity_id;

-- 0294_loom_folders.sql
DEFINE TABLE OVERWRITE loom_folders SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE folder_id ON TABLE loom_folders TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_folders TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE parent_folder_id ON TABLE loom_folders TYPE option<record<loom_folders>> ASSERT ($value = NONE) OR (record::exists($value) AND $value != $this.id) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE name ON TABLE loom_folders TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE color ON TABLE loom_folders TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE sort_mode ON TABLE loom_folders TYPE 'name_asc' | 'name_desc' | 'created_desc' | 'updated_desc' | 'manual' DEFAULT 'updated_desc';
DEFINE FIELD OVERWRITE sort_order ON TABLE loom_folders TYPE option<int>;
DEFINE FIELD OVERWRITE project_ref ON TABLE loom_folders TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_folders TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_folders TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_folders ON TABLE loom_folders FIELDS folder_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_folders_sibling_name ON TABLE loom_folders FIELDS workspace_id, parent_folder_id, name UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_folders_workspace_parent ON TABLE loom_folders FIELDS workspace_id, parent_folder_id;

DEFINE TABLE OVERWRITE loom_folder_members SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE folder_id ON TABLE loom_folder_members TYPE record<loom_folders> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_folder_members TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_folder_members TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE sort_order ON TABLE loom_folder_members TYPE option<int>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_folder_members TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_folder_members ON TABLE loom_folder_members FIELDS folder_id, block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_folder_members_block ON TABLE loom_folder_members FIELDS workspace_id, block_id;
DEFINE INDEX OVERWRITE idx_loom_folder_members_folder_order ON TABLE loom_folder_members FIELDS folder_id, sort_order;

-- 0295_loom_wiki_overlays.sql
DEFINE TABLE OVERWRITE loom_wiki_overlays SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE overlay_id ON TABLE loom_wiki_overlays TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE projection_id ON TABLE loom_wiki_overlays TYPE record<knowledge_wiki_projections> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_wiki_overlays TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE annotation ON TABLE loom_wiki_overlays TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE anchor ON TABLE loom_wiki_overlays TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_wiki_overlays TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_wiki_overlays TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_wiki_overlays ON TABLE loom_wiki_overlays FIELDS overlay_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_wiki_overlays_projection ON TABLE loom_wiki_overlays FIELDS projection_id;
DEFINE INDEX OVERWRITE idx_loom_wiki_overlays_workspace ON TABLE loom_wiki_overlays FIELDS workspace_id;

-- 0310_user_manual.sql. UserManual rows are product-level, not workspace
-- scoped: the manual documents the product itself.
DEFINE TABLE OVERWRITE user_manual_pages SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE page_id ON TABLE user_manual_pages TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE slug ON TABLE user_manual_pages TYPE string ASSERT string::trim($value) != '' AND $value = string::lowercase($value);
DEFINE FIELD OVERWRITE title ON TABLE user_manual_pages TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE page_kind ON TABLE user_manual_pages TYPE 'purpose' | 'workflow' | 'tool_catalog' | 'failure_recovery' | 'quickstart' | 'state_recovery' | 'surface_guide' | 'navigation' | 'legacy_bridge' | 'spec_enrichment_seed';
DEFINE FIELD OVERWRITE audience ON TABLE user_manual_pages TYPE 'model' | 'operator' | 'model_and_operator' DEFAULT 'model_and_operator';
DEFINE FIELD OVERWRITE body ON TABLE user_manual_pages TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE body.* ON TABLE user_manual_pages TYPE any;
DEFINE FIELD OVERWRITE content_hash ON TABLE user_manual_pages TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE manual_version ON TABLE user_manual_pages TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE source_kind ON TABLE user_manual_pages TYPE 'builtin_seed' | 'runtime_edit' DEFAULT 'builtin_seed';
DEFINE FIELD OVERWRITE spec_anchors ON TABLE user_manual_pages TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE status ON TABLE user_manual_pages TYPE 'current' | 'deprecated' | 'superseded' DEFAULT 'current';
DEFINE FIELD OVERWRITE superseded_by_slug ON TABLE user_manual_pages TYPE option<string>;
DEFINE FIELD OVERWRITE ledger_event_id ON TABLE user_manual_pages TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE user_manual_pages TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE user_manual_pages TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_user_manual_pages ON TABLE user_manual_pages FIELDS page_id UNIQUE;
DEFINE INDEX OVERWRITE uq_user_manual_pages_slug ON TABLE user_manual_pages FIELDS slug UNIQUE;
DEFINE INDEX OVERWRITE idx_user_manual_pages_kind ON TABLE user_manual_pages FIELDS page_kind, status;

DEFINE TABLE OVERWRITE user_manual_sections SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE section_id ON TABLE user_manual_sections TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE page_id ON TABLE user_manual_sections TYPE record<user_manual_pages> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE position ON TABLE user_manual_sections TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE section_kind ON TABLE user_manual_sections TYPE 'purpose' | 'workflows' | 'startup' | 'run_commands' | 'inputs_outputs' | 'navigation' | 'safety' | 'failure_modes' | 'recovery' | 'hooks' | 'examples' | 'schema';
DEFINE FIELD OVERWRITE title ON TABLE user_manual_sections TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE body_md ON TABLE user_manual_sections TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE body_json ON TABLE user_manual_sections TYPE option<object> FLEXIBLE;
DEFINE FIELD OVERWRITE body_json.* ON TABLE user_manual_sections TYPE any;
DEFINE INDEX OVERWRITE pk_user_manual_sections ON TABLE user_manual_sections FIELDS section_id UNIQUE;
DEFINE INDEX OVERWRITE uq_user_manual_sections_position ON TABLE user_manual_sections FIELDS page_id, position UNIQUE;
DEFINE INDEX OVERWRITE idx_user_manual_sections_page ON TABLE user_manual_sections FIELDS page_id, position;

DEFINE TABLE OVERWRITE user_manual_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_id ON TABLE user_manual_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE page_id ON TABLE user_manual_anchors TYPE record<user_manual_pages> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE anchor_kind ON TABLE user_manual_anchors TYPE 'http_route' | 'tauri_command' | 'ipc_channel' | 'cli_command' | 'spec_anchor' | 'test' | 'event_type' | 'primitive' | 'page_link';
DEFINE FIELD OVERWRITE anchor_value ON TABLE user_manual_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE http_method ON TABLE user_manual_anchors TYPE '' | 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' DEFAULT '';
DEFINE INDEX OVERWRITE pk_user_manual_anchors ON TABLE user_manual_anchors FIELDS anchor_id UNIQUE;
DEFINE INDEX OVERWRITE uq_user_manual_anchors_identity ON TABLE user_manual_anchors FIELDS page_id, anchor_kind, anchor_value, http_method UNIQUE;
DEFINE INDEX OVERWRITE idx_user_manual_anchors_value ON TABLE user_manual_anchors FIELDS anchor_kind, anchor_value;

DEFINE TABLE OVERWRITE user_manual_tool_entries SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE tool_id ON TABLE user_manual_tool_entries TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE page_id ON TABLE user_manual_tool_entries TYPE option<record<user_manual_pages>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE name ON TABLE user_manual_tool_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE status ON TABLE user_manual_tool_entries TYPE 'wired' | 'planned' | 'deprecated';
DEFINE FIELD OVERWRITE ipc_channel ON TABLE user_manual_tool_entries TYPE option<string>;
DEFINE FIELD OVERWRITE tauri_command ON TABLE user_manual_tool_entries TYPE option<string>;
DEFINE FIELD OVERWRITE cli_flag ON TABLE user_manual_tool_entries TYPE option<string>;
DEFINE FIELD OVERWRITE http_route ON TABLE user_manual_tool_entries TYPE option<string>;
DEFINE FIELD OVERWRITE http_method ON TABLE user_manual_tool_entries TYPE '' | 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' DEFAULT '';
DEFINE FIELD OVERWRITE description ON TABLE user_manual_tool_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE expected_input ON TABLE user_manual_tool_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE expected_output ON TABLE user_manual_tool_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE schema_fields ON TABLE user_manual_tool_entries TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE schema_fields.* ON TABLE user_manual_tool_entries TYPE any;
DEFINE FIELD OVERWRITE common_errors ON TABLE user_manual_tool_entries TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE common_errors.* ON TABLE user_manual_tool_entries TYPE any;
DEFINE FIELD OVERWRITE recovery_steps ON TABLE user_manual_tool_entries TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE recovery_steps.* ON TABLE user_manual_tool_entries TYPE any;
DEFINE FIELD OVERWRITE origin ON TABLE user_manual_tool_entries TYPE 'legacy_model_manual' | 'wp009_surface' | 'wp1_model_lane' | 'wp1_mt012_operator_chat' | 'wp1_mt013_embedded_model' | 'wp1_mt014' | 'wp1_mt015_cloud_model_access' | 'wp1_mt016_dedicated_embedding_model';
DEFINE FIELD OVERWRITE content_hash ON TABLE user_manual_tool_entries TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE manual_version ON TABLE user_manual_tool_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at ON TABLE user_manual_tool_entries TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_user_manual_tool_entries ON TABLE user_manual_tool_entries FIELDS tool_id UNIQUE;
DEFINE INDEX OVERWRITE idx_user_manual_tool_entries_route ON TABLE user_manual_tool_entries FIELDS http_route, http_method;

DEFINE TABLE OVERWRITE user_manual_feature_entries SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE feature_id ON TABLE user_manual_feature_entries TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE title ON TABLE user_manual_feature_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE description ON TABLE user_manual_feature_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE tool_ids ON TABLE user_manual_feature_entries TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE origin ON TABLE user_manual_feature_entries TYPE 'legacy_model_manual' | 'wp009_surface' | 'wp1_model_lane' | 'wp1_mt012_operator_chat' | 'wp1_mt013_embedded_model' | 'wp1_mt014' | 'wp1_mt015_cloud_model_access' | 'wp1_mt016_dedicated_embedding_model';
DEFINE FIELD OVERWRITE content_hash ON TABLE user_manual_feature_entries TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE manual_version ON TABLE user_manual_feature_entries TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at ON TABLE user_manual_feature_entries TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_user_manual_feature_entries ON TABLE user_manual_feature_entries FIELDS feature_id UNIQUE;

DEFINE TABLE OVERWRITE user_manual_versions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE manual_version ON TABLE user_manual_versions TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE seeded_at ON TABLE user_manual_versions TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE seed_content_hash ON TABLE user_manual_versions TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE page_count ON TABLE user_manual_versions TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE tool_count ON TABLE user_manual_versions TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE feature_count ON TABLE user_manual_versions TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE ledger_event_id ON TABLE user_manual_versions TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE note ON TABLE user_manual_versions TYPE string DEFAULT '';
DEFINE INDEX OVERWRITE pk_user_manual_versions ON TABLE user_manual_versions FIELDS manual_version UNIQUE;

DEFINE TABLE OVERWRITE user_manual_legacy_aliases SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE alias ON TABLE user_manual_legacy_aliases TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE alias_kind ON TABLE user_manual_legacy_aliases TYPE 'module' | 'tauri_command' | 'ipc_channel' | 'projection' | 'test' | 'constant';
DEFINE FIELD OVERWRITE canonical_kind ON TABLE user_manual_legacy_aliases TYPE 'page' | 'tool' | 'route';
DEFINE FIELD OVERWRITE canonical_ref ON TABLE user_manual_legacy_aliases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE deprecation_note ON TABLE user_manual_legacy_aliases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE manual_version ON TABLE user_manual_legacy_aliases TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at ON TABLE user_manual_legacy_aliases TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_user_manual_legacy_aliases ON TABLE user_manual_legacy_aliases FIELDS alias UNIQUE;

-- 0311_parallel_swarm_state_recovery.sql
DEFINE TABLE OVERWRITE knowledge_agent_worktree_claims SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_agent_worktree_claims TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_agent_worktree_claims TYPE option<string>;
DEFINE FIELD OVERWRITE scope_kind ON TABLE knowledge_agent_worktree_claims TYPE 'worktree' | 'workspace' | 'index_run';
DEFINE FIELD OVERWRITE scope_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE lane_kind ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE attribution_jsonb ON TABLE knowledge_agent_worktree_claims TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE attribution_jsonb.* ON TABLE knowledge_agent_worktree_claims TYPE any;
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE knowledge_agent_worktree_claims TYPE 'active' | 'released' | 'reclaimed';
DEFINE FIELD OVERWRITE reason ON TABLE knowledge_agent_worktree_claims TYPE string;
DEFINE FIELD OVERWRITE claimed_at_utc ON TABLE knowledge_agent_worktree_claims TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE knowledge_agent_worktree_claims TYPE datetime;
DEFINE FIELD OVERWRITE released_at_utc ON TABLE knowledge_agent_worktree_claims TYPE option<datetime>;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_agent_worktree_claims TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE release_event_ledger_event_id ON TABLE knowledge_agent_worktree_claims TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE reclaim_event_ledger_event_id ON TABLE knowledge_agent_worktree_claims TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
-- Stored discriminator for the source partial UNIQUE index
-- `ux_agent_worktree_claims_active_scope ... WHERE status = active AND released_at_utc IS NULL`.
DEFINE FIELD OVERWRITE active_scope_key ON TABLE knowledge_agent_worktree_claims TYPE string
    VALUE IF $this.status = 'active' AND $this.released_at_utc = NONE {
        'active:' + type::string($this.scope_kind) + ':' + $this.scope_id
    } ELSE {
        'inactive:' + $this.claim_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_agent_worktree_claims ON TABLE knowledge_agent_worktree_claims FIELDS claim_id UNIQUE;
DEFINE INDEX OVERWRITE ux_agent_worktree_claims_active_scope ON TABLE knowledge_agent_worktree_claims FIELDS active_scope_key UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_worktree_claims_workspace_active ON TABLE knowledge_agent_worktree_claims FIELDS workspace_id, claimed_at_utc;

DEFINE TABLE OVERWRITE knowledge_agent_role_mailbox_handoffs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE handoff_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE option<record<knowledge_agent_worktree_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE from_lane_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE from_actor_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE from_lane_kind ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE from_attribution_jsonb ON TABLE knowledge_agent_role_mailbox_handoffs TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE from_attribution_jsonb.* ON TABLE knowledge_agent_role_mailbox_handoffs TYPE any;
DEFINE FIELD OVERWRITE to_role ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE mailbox_thread_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE mailbox_message_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE knowledge_agent_role_mailbox_handoffs TYPE 'started' | 'progress' | 'blocked' | 'pass' | 'fail';
DEFINE FIELD OVERWRITE summary ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string;
DEFINE FIELD OVERWRITE body_sha256 ON TABLE knowledge_agent_role_mailbox_handoffs TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_agent_role_mailbox_handoffs TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_agent_role_mailbox_handoffs TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_agent_role_mailbox_handoffs ON TABLE knowledge_agent_role_mailbox_handoffs FIELDS handoff_id UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_role_mailbox_handoffs_wp_mt ON TABLE knowledge_agent_role_mailbox_handoffs FIELDS wp_id, mt_id, created_at_utc;

DEFINE TABLE OVERWRITE knowledge_agent_state_recovery_checkpoints SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE lane_kind ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE attribution_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE attribution_jsonb.* ON TABLE knowledge_agent_state_recovery_checkpoints TYPE any;
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE option<record<knowledge_agent_worktree_claims>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE mailbox_handoff_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE option<record<knowledge_agent_role_mailbox_handoffs>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE navigation_command_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE option<string>;
DEFINE FIELD OVERWRITE resume_pointer_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE resume_pointer_jsonb.* ON TABLE knowledge_agent_state_recovery_checkpoints TYPE any;
DEFINE FIELD OVERWRITE touched_files_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE array<string>;
DEFINE FIELD OVERWRITE tests_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE array<string>;
DEFINE FIELD OVERWRITE hbr_rows_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE array<string>;
DEFINE FIELD OVERWRITE next_step_context ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE payload_jsonb ON TABLE knowledge_agent_state_recovery_checkpoints TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE payload_jsonb.* ON TABLE knowledge_agent_state_recovery_checkpoints TYPE any;
DEFINE FIELD OVERWRITE payload_sha256 ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE compaction_reason ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE git_head ON TABLE knowledge_agent_state_recovery_checkpoints TYPE string;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_agent_state_recovery_checkpoints TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_agent_state_recovery_checkpoints TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_agent_state_recovery_checkpoints ON TABLE knowledge_agent_state_recovery_checkpoints FIELDS checkpoint_id UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_state_recovery_checkpoints_session ON TABLE knowledge_agent_state_recovery_checkpoints FIELDS session_id, created_at_utc;

DEFINE TABLE OVERWRITE knowledge_agent_recovery_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_agent_recovery_receipts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE checkpoint_id ON TABLE knowledge_agent_recovery_receipts TYPE record<knowledge_agent_state_recovery_checkpoints> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE prior_session_id ON TABLE knowledge_agent_recovery_receipts TYPE string;
DEFINE FIELD OVERWRITE new_session_id ON TABLE knowledge_agent_recovery_receipts TYPE string;
DEFINE FIELD OVERWRITE new_lane_id ON TABLE knowledge_agent_recovery_receipts TYPE string;
DEFINE FIELD OVERWRITE new_actor_id ON TABLE knowledge_agent_recovery_receipts TYPE string;
DEFINE FIELD OVERWRITE new_lane_kind ON TABLE knowledge_agent_recovery_receipts TYPE string;
DEFINE FIELD OVERWRITE new_attribution_jsonb ON TABLE knowledge_agent_recovery_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE new_attribution_jsonb.* ON TABLE knowledge_agent_recovery_receipts TYPE any;
DEFINE FIELD OVERWRITE resume_pointer_jsonb ON TABLE knowledge_agent_recovery_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE resume_pointer_jsonb.* ON TABLE knowledge_agent_recovery_receipts TYPE any;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_agent_recovery_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE recovered_at_utc ON TABLE knowledge_agent_recovery_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_agent_recovery_receipts ON TABLE knowledge_agent_recovery_receipts FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_recovery_receipts_checkpoint ON TABLE knowledge_agent_recovery_receipts FIELDS checkpoint_id, recovered_at_utc;

DEFINE TABLE OVERWRITE knowledge_parallel_indexing_lease_queue SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE lease_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE scope_kind ON TABLE knowledge_parallel_indexing_lease_queue TYPE 'worktree' | 'workspace' | 'index_run';
DEFINE FIELD OVERWRITE scope_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE lane_kind ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE attribution_jsonb ON TABLE knowledge_parallel_indexing_lease_queue TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE attribution_jsonb.* ON TABLE knowledge_parallel_indexing_lease_queue TYPE any;
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE index_run_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE string;
DEFINE FIELD OVERWRITE priority ON TABLE knowledge_parallel_indexing_lease_queue TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE ttl_seconds ON TABLE knowledge_parallel_indexing_lease_queue TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE status ON TABLE knowledge_parallel_indexing_lease_queue TYPE 'queued' | 'acquired' | 'completed' | 'cancelled' | 'reclaimed';
DEFINE FIELD OVERWRITE blocked_by_lease_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE option<string>;
DEFINE FIELD OVERWRITE enqueued_at_utc ON TABLE knowledge_parallel_indexing_lease_queue TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE acquired_at_utc ON TABLE knowledge_parallel_indexing_lease_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE expires_at_utc ON TABLE knowledge_parallel_indexing_lease_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE completed_at_utc ON TABLE knowledge_parallel_indexing_lease_queue TYPE option<datetime>;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_parallel_indexing_lease_queue TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
-- 0313_parallel_swarm_quiet_background_work.sql adds the quiet-mode policy.
DEFINE FIELD OVERWRITE quiet_policy_jsonb ON TABLE knowledge_parallel_indexing_lease_queue TYPE object FLEXIBLE DEFAULT { work_kind: 'indexing', no_foreground_window: true, no_focus_steal: true, no_os_shell_window: true, bounded: true, observable: true };
DEFINE FIELD OVERWRITE quiet_policy_jsonb.* ON TABLE knowledge_parallel_indexing_lease_queue TYPE any;
-- Stored discriminator for the source partial UNIQUE index
-- `ux_parallel_indexing_lease_queue_active_scope ... WHERE status = acquired`.
DEFINE FIELD OVERWRITE acquired_scope_key ON TABLE knowledge_parallel_indexing_lease_queue TYPE string
    VALUE IF $this.status = 'acquired' {
        'acquired:' + type::string($this.scope_kind) + ':' + $this.scope_id
    } ELSE {
        'inactive:' + $this.lease_id
    };
DEFINE INDEX OVERWRITE pk_knowledge_parallel_indexing_lease_queue ON TABLE knowledge_parallel_indexing_lease_queue FIELDS lease_id UNIQUE;
DEFINE INDEX OVERWRITE ux_parallel_indexing_lease_queue_active_scope ON TABLE knowledge_parallel_indexing_lease_queue FIELDS acquired_scope_key UNIQUE;
DEFINE INDEX OVERWRITE idx_parallel_indexing_lease_queue_queued ON TABLE knowledge_parallel_indexing_lease_queue FIELDS scope_kind, scope_id, priority, enqueued_at_utc;

-- 0313_parallel_swarm_quiet_background_work.sql
DEFINE TABLE OVERWRITE knowledge_agent_quiet_background_work SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_agent_quiet_background_work TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE work_kind ON TABLE knowledge_agent_quiet_background_work TYPE 'indexing' | 'backend_navigation' | 'visual_capture' | 'test_run';
DEFINE FIELD OVERWRITE subject_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE lane_kind ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE attribution_jsonb ON TABLE knowledge_agent_quiet_background_work TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE attribution_jsonb.* ON TABLE knowledge_agent_quiet_background_work TYPE any;
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_agent_quiet_background_work TYPE string;
DEFINE FIELD OVERWRITE quiet_policy_jsonb ON TABLE knowledge_agent_quiet_background_work TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE quiet_policy_jsonb.* ON TABLE knowledge_agent_quiet_background_work TYPE any;
DEFINE FIELD OVERWRITE evidence_ref ON TABLE knowledge_agent_quiet_background_work TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_agent_quiet_background_work TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_agent_quiet_background_work TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_agent_quiet_background_work ON TABLE knowledge_agent_quiet_background_work FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_quiet_background_work_workspace ON TABLE knowledge_agent_quiet_background_work FIELDS workspace_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_agent_quiet_background_work_subject ON TABLE knowledge_agent_quiet_background_work FIELDS subject_id, created_at_utc;

-- 0314_parallel_swarm_cloud_assistance_receipts.sql
DEFINE TABLE OVERWRITE knowledge_agent_cloud_assistance_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE receipt_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE wp_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE mt_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE claim_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE record<knowledge_agent_worktree_claims> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE handoff_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE record<knowledge_agent_role_mailbox_handoffs> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE handoff_event_ledger_event_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE cloud_assistance_event_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE fallback_basis_event_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE parent_session_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE prompt_sha256 ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE lane_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE lane_kind ON TABLE knowledge_agent_cloud_assistance_receipts TYPE 'cloud';
DEFINE FIELD OVERWRITE provider ON TABLE knowledge_agent_cloud_assistance_receipts TYPE 'open_ai' | 'anthropic' | 'official_cli' | 'other';
DEFINE FIELD OVERWRITE model_label ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE attribution_jsonb ON TABLE knowledge_agent_cloud_assistance_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE attribution_jsonb.* ON TABLE knowledge_agent_cloud_assistance_receipts TYPE any;
DEFINE FIELD OVERWRITE session_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string;
DEFINE FIELD OVERWRITE fallback_reason ON TABLE knowledge_agent_cloud_assistance_receipts TYPE 'local_failed' | 'local_low_confidence' | 'local_overloaded' | 'local_suppressed' | 'hard_reasoning' | 'force_cloud' | 'no_local_model';
DEFINE FIELD OVERWRITE output_kind ON TABLE knowledge_agent_cloud_assistance_receipts TYPE 'analysis' | 'patch_suggestion' | 'validation_summary' | 'handoff_summary';
DEFINE FIELD OVERWRITE output_sha256 ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE body_sha256 ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE output_text ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE output_body_jsonb ON TABLE knowledge_agent_cloud_assistance_receipts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE output_body_jsonb.* ON TABLE knowledge_agent_cloud_assistance_receipts TYPE any;
DEFINE FIELD OVERWRITE target_ref ON TABLE knowledge_agent_cloud_assistance_receipts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE review_state ON TABLE knowledge_agent_cloud_assistance_receipts TYPE 'pending_review' | 'rejected';
DEFINE FIELD OVERWRITE non_authoritative ON TABLE knowledge_agent_cloud_assistance_receipts TYPE bool DEFAULT true ASSERT $value = true;
DEFINE FIELD OVERWRITE requires_promotion ON TABLE knowledge_agent_cloud_assistance_receipts TYPE bool DEFAULT true ASSERT $value = true;
DEFINE FIELD OVERWRITE authority_mutation_allowed ON TABLE knowledge_agent_cloud_assistance_receipts TYPE bool DEFAULT false ASSERT $value = false;
DEFINE FIELD OVERWRITE promotion_event_id ON TABLE knowledge_agent_cloud_assistance_receipts TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE knowledge_agent_cloud_assistance_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_agent_cloud_assistance_receipts ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS receipt_id UNIQUE;
DEFINE INDEX OVERWRITE uq_agent_cloud_assistance_receipts_handoff ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS handoff_id UNIQUE;
DEFINE INDEX OVERWRITE uq_agent_cloud_assistance_receipts_event ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS cloud_assistance_event_id UNIQUE;
DEFINE INDEX OVERWRITE uq_agent_cloud_assistance_receipts_basis ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS fallback_basis_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_agent_cloud_assistance_receipts_workspace ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS workspace_id, created_at_utc;
DEFINE INDEX OVERWRITE idx_agent_cloud_assistance_receipts_review ON TABLE knowledge_agent_cloud_assistance_receipts FIELDS review_state, created_at_utc;

-- 0322_quick_switcher_recents.sql. The `knowledge_schema_registry` seed row is
-- DML and is intentionally omitted; this bootstrap targets an empty database.
DEFINE TABLE OVERWRITE knowledge_quick_switcher_recents SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_quick_switcher_recents TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE hit_key ON TABLE knowledge_quick_switcher_recents TYPE string ASSERT $value = string::concat($this.source_kind, ':', $this.ref_id);
DEFINE FIELD OVERWRITE source_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'symbol' | 'work_packet' | 'micro_task' | 'user_manual_page';
DEFINE FIELD OVERWRITE ref_id ON TABLE knowledge_quick_switcher_recents TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE result_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'knowledge_entity' | 'user_manual_page';
DEFINE FIELD OVERWRITE title ON TABLE knowledge_quick_switcher_recents TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE excerpt ON TABLE knowledge_quick_switcher_recents TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE metadata ON TABLE knowledge_quick_switcher_recents TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE metadata.* ON TABLE knowledge_quick_switcher_recents TYPE any;
DEFINE FIELD OVERWRITE selected_count ON TABLE knowledge_quick_switcher_recents TYPE int DEFAULT 1 ASSERT $value >= 1;
DEFINE FIELD OVERWRITE selected_at ON TABLE knowledge_quick_switcher_recents TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_quick_switcher_recents TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_knowledge_quick_switcher_recents ON TABLE knowledge_quick_switcher_recents FIELDS workspace_id, hit_key UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_quick_switcher_recents_workspace_selected ON TABLE knowledge_quick_switcher_recents FIELDS workspace_id, selected_at, hit_key;
DEFINE INDEX OVERWRITE idx_knowledge_quick_switcher_recents_event ON TABLE knowledge_quick_switcher_recents FIELDS event_ledger_event_id;

-- 0323_workbench_layout_state.sql
DEFINE TABLE OVERWRITE knowledge_workbench_layout_states SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_workbench_layout_states TYPE record<workspaces> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE layout_state ON TABLE knowledge_workbench_layout_states TYPE object FLEXIBLE ASSERT $value.schema_id = 'hsk.workbench_layout_state@1';
DEFINE FIELD OVERWRITE layout_state.* ON TABLE knowledge_workbench_layout_states TYPE any;
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_workbench_layout_states TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_workbench_layout_states TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_knowledge_workbench_layout_states ON TABLE knowledge_workbench_layout_states FIELDS workspace_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_workbench_layout_states_event ON TABLE knowledge_workbench_layout_states FIELDS event_ledger_event_id;

-- 0327_workspace_settings_state.sql
DEFINE TABLE OVERWRITE knowledge_workspace_settings_states SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_workspace_settings_states TYPE record<workspaces> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE settings_state ON TABLE knowledge_workspace_settings_states TYPE object FLEXIBLE ASSERT $value.schema_id = 'hsk.workspace_settings_state@1';
DEFINE FIELD OVERWRITE settings_state.* ON TABLE knowledge_workspace_settings_states TYPE any;
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_workspace_settings_states TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_workspace_settings_states TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_knowledge_workspace_settings_states ON TABLE knowledge_workspace_settings_states FIELDS workspace_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_workspace_settings_states_event ON TABLE knowledge_workspace_settings_states FIELDS event_ledger_event_id;

-- 0328_rich_document_draft_recovery.sql
DEFINE TABLE OVERWRITE knowledge_rich_document_drafts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_rich_document_drafts TYPE record<knowledge_rich_documents> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_drafts TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE base_doc_version ON TABLE knowledge_rich_document_drafts TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE base_content_sha256 ON TABLE knowledge_rich_document_drafts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE draft_content_json ON TABLE knowledge_rich_document_drafts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE draft_content_json.* ON TABLE knowledge_rich_document_drafts TYPE any;
DEFINE FIELD OVERWRITE draft_content_sha256 ON TABLE knowledge_rich_document_drafts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE actor_kind ON TABLE knowledge_rich_document_drafts TYPE 'operator' | 'local_model' | 'cloud_model' | 'validator' | 'system' | 'unauthenticated';
DEFINE FIELD OVERWRITE actor_id ON TABLE knowledge_rich_document_drafts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE kernel_task_run_id ON TABLE knowledge_rich_document_drafts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE session_run_id ON TABLE knowledge_rich_document_drafts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_drafts TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_document_drafts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_drafts ON TABLE knowledge_rich_document_drafts FIELDS rich_document_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_rich_document_drafts_workspace ON TABLE knowledge_rich_document_drafts FIELDS workspace_id, updated_at;

-- 0330_workspace_search_bookmark_state.sql
DEFINE TABLE OVERWRITE knowledge_workspace_search_bookmark_states SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_workspace_search_bookmark_states TYPE record<workspaces> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE bookmark_state ON TABLE knowledge_workspace_search_bookmark_states TYPE object FLEXIBLE ASSERT $value.schema_id = 'hsk.workspace_search_bookmark_state@1' AND type::is::array($value.bookmarks);
DEFINE FIELD OVERWRITE bookmark_state.* ON TABLE knowledge_workspace_search_bookmark_states TYPE any;
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_workspace_search_bookmark_states TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_workspace_search_bookmark_states TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_knowledge_workspace_search_bookmark_states ON TABLE knowledge_workspace_search_bookmark_states FIELDS workspace_id UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_workspace_search_bookmark_states_event ON TABLE knowledge_workspace_search_bookmark_states FIELDS event_ledger_event_id;

-- 0331_debug_breakpoints.sql
DEFINE TABLE OVERWRITE knowledge_debug_breakpoints SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE breakpoint_id ON TABLE knowledge_debug_breakpoints TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_debug_breakpoints TYPE record<knowledge_rich_documents> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_debug_breakpoints TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE source_url ON TABLE knowledge_debug_breakpoints TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE line ON TABLE knowledge_debug_breakpoints TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE condition ON TABLE knowledge_debug_breakpoints TYPE option<string>;
DEFINE FIELD OVERWRITE verified ON TABLE knowledge_debug_breakpoints TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_debug_breakpoints TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_debug_breakpoints TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE knowledge_debug_breakpoints TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_knowledge_debug_breakpoints ON TABLE knowledge_debug_breakpoints FIELDS breakpoint_id UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_debug_breakpoints_doc_src_line ON TABLE knowledge_debug_breakpoints FIELDS rich_document_id, source_url, line UNIQUE;
DEFINE INDEX OVERWRITE idx_knowledge_debug_breakpoints_document ON TABLE knowledge_debug_breakpoints FIELDS rich_document_id, source_url, line;
DEFINE INDEX OVERWRITE idx_knowledge_debug_breakpoints_event ON TABLE knowledge_debug_breakpoints FIELDS event_ledger_event_id;

-- 0332_media_asset_tiers.sql
DEFINE TABLE OVERWRITE media_asset_tiers SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE tier_row_id ON TABLE media_asset_tiers TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE media_asset_tiers TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE asset_id ON TABLE media_asset_tiers TYPE record<assets> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE tier ON TABLE media_asset_tiers TYPE 'thumb' | 'preview' | 'poster' | 'full';
DEFINE FIELD OVERWRITE status ON TABLE media_asset_tiers TYPE 'pending' | 'ready' | 'failed' DEFAULT 'pending';
DEFINE FIELD OVERWRITE tier_asset_id ON TABLE media_asset_tiers TYPE option<record<assets>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE UNSET;
DEFINE FIELD OVERWRITE content_hash ON TABLE media_asset_tiers TYPE option<string>;
DEFINE FIELD OVERWRITE failure_reason ON TABLE media_asset_tiers TYPE option<string>;
DEFINE FIELD OVERWRITE attempt_count ON TABLE media_asset_tiers TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE created_at ON TABLE media_asset_tiers TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE media_asset_tiers TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_media_asset_tiers ON TABLE media_asset_tiers FIELDS tier_row_id UNIQUE;
DEFINE INDEX OVERWRITE uq_media_asset_tiers_asset_tier ON TABLE media_asset_tiers FIELDS asset_id, tier UNIQUE;
DEFINE INDEX OVERWRITE idx_media_asset_tiers_asset ON TABLE media_asset_tiers FIELDS asset_id, tier;
DEFINE INDEX OVERWRITE idx_media_asset_tiers_workspace_status ON TABLE media_asset_tiers FIELDS workspace_id, status;

DEFINE TABLE OVERWRITE loom_collections SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE collection_id ON TABLE loom_collections TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_collections TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title ON TABLE loom_collections TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_collections TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_collections TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_collections ON TABLE loom_collections FIELDS collection_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_collections_workspace ON TABLE loom_collections FIELDS workspace_id;

DEFINE TABLE OVERWRITE loom_collection_members SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE collection_id ON TABLE loom_collection_members TYPE record<loom_collections> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE asset_id ON TABLE loom_collection_members TYPE record<assets> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE position ON TABLE loom_collection_members TYPE int ASSERT $value >= 0;
DEFINE INDEX OVERWRITE pk_loom_collection_members ON TABLE loom_collection_members FIELDS collection_id, asset_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_collection_members_order ON TABLE loom_collection_members FIELDS collection_id, position UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_collection_members_ordered ON TABLE loom_collection_members FIELDS collection_id, position;

-- 0333_loom_ai_suggestions.sql. This migration also widens the
-- `knowledge_crdt_denial_receipts.receipt_kind` allow-list; that superset is
-- applied in the 0150 section above.
DEFINE TABLE OVERWRITE loom_ai_suggestions SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE suggestion_id ON TABLE loom_ai_suggestions TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'LAIS-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE job_id ON TABLE loom_ai_suggestions TYPE string ASSERT string::starts_with($value, 'LAIJ-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_ai_suggestions TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE kind ON TABLE loom_ai_suggestions TYPE 'auto_tag' | 'auto_caption' | 'link_suggest';
DEFINE FIELD OVERWRITE block_id ON TABLE loom_ai_suggestions TYPE string;
DEFINE FIELD OVERWRITE target_block_id ON TABLE loom_ai_suggestions TYPE option<string> ASSERT ($this.kind = 'link_suggest' AND $value != NONE) OR ($this.kind != 'link_suggest' AND $value = NONE);
DEFINE FIELD OVERWRITE suggested_value ON TABLE loom_ai_suggestions TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE suggested_value.* ON TABLE loom_ai_suggestions TYPE any;
DEFINE FIELD OVERWRITE model_attribution ON TABLE loom_ai_suggestions TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE model_attribution.* ON TABLE loom_ai_suggestions TYPE any;
DEFINE FIELD OVERWRITE prompt_sha256 ON TABLE loom_ai_suggestions TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE output_sha256 ON TABLE loom_ai_suggestions TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE review_state ON TABLE loom_ai_suggestions TYPE 'pending' | 'accepted' | 'rejected' | 'promoted' DEFAULT 'pending';
DEFINE FIELD OVERWRITE decided_by ON TABLE loom_ai_suggestions TYPE option<string> ASSERT ($this.review_state = 'pending' AND $value = NONE) OR ($this.review_state != 'pending' AND (string::starts_with($value, 'operator:') OR string::starts_with($value, 'validator:')));
DEFINE FIELD OVERWRITE decided_at_utc ON TABLE loom_ai_suggestions TYPE option<datetime> ASSERT ($this.review_state = 'pending') = ($value = NONE);
DEFINE FIELD OVERWRITE decision_reason ON TABLE loom_ai_suggestions TYPE option<string> ASSERT ($this.review_state = 'pending') = ($value = NONE);
DEFINE FIELD OVERWRITE recorded_event_id ON TABLE loom_ai_suggestions TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE decided_event_id ON TABLE loom_ai_suggestions TYPE option<record<kernel_event_ledger>> ASSERT (($this.review_state = 'pending') = ($value = NONE)) AND ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_requested_event_id ON TABLE loom_ai_suggestions TYPE option<record<kernel_event_ledger>> ASSERT (($this.review_state = 'promoted') = ($value != NONE)) AND ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promotion_accepted_event_id ON TABLE loom_ai_suggestions TYPE option<record<kernel_event_ledger>> ASSERT (($this.review_state = 'promoted') = ($value != NONE)) AND ($value = NONE OR record::exists($value)) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE promoted_artifact_ref ON TABLE loom_ai_suggestions TYPE option<string> ASSERT ($this.review_state = 'promoted') = ($value != NONE);
DEFINE FIELD OVERWRITE value_hash ON TABLE loom_ai_suggestions TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at_utc ON TABLE loom_ai_suggestions TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_ai_suggestions ON TABLE loom_ai_suggestions FIELDS suggestion_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_ai_suggestions_idempotency ON TABLE loom_ai_suggestions FIELDS job_id, block_id, kind, value_hash, target_block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_ai_suggestions_job ON TABLE loom_ai_suggestions FIELDS job_id, kind, review_state, created_at_utc;
DEFINE INDEX OVERWRITE idx_loom_ai_suggestions_workspace ON TABLE loom_ai_suggestions FIELDS workspace_id, review_state, created_at_utc;

-- 0334_loom_canvas_boards.sql. This migration also widens the
-- `loom_blocks.content_type` allow-list with 'canvas'; that superset is applied
-- in the 0013 section above.
DEFINE TABLE OVERWRITE loom_canvas_boards SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_canvas_boards TYPE record<loom_blocks> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_canvas_boards TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE board_state ON TABLE loom_canvas_boards TYPE object FLEXIBLE ASSERT $value.schema_id = 'hsk.loom_canvas_board@1';
DEFINE FIELD OVERWRITE board_state.* ON TABLE loom_canvas_boards TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_canvas_boards TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_canvas_boards TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE loom_canvas_boards TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_loom_canvas_boards ON TABLE loom_canvas_boards FIELDS block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_canvas_boards_workspace ON TABLE loom_canvas_boards FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_loom_canvas_boards_event ON TABLE loom_canvas_boards FIELDS event_ledger_event_id;

DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE placement_id ON TABLE loom_canvas_placements TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'LCP-') AND string::len($value) = 36;
DEFINE FIELD OVERWRITE canvas_block_id ON TABLE loom_canvas_placements TYPE record<loom_canvas_boards> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_canvas_placements TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE placed_block_id ON TABLE loom_canvas_placements TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE x ON TABLE loom_canvas_placements TYPE float;
DEFINE FIELD OVERWRITE y ON TABLE loom_canvas_placements TYPE float;
DEFINE FIELD OVERWRITE w ON TABLE loom_canvas_placements TYPE float ASSERT $value > 0;
DEFINE FIELD OVERWRITE h ON TABLE loom_canvas_placements TYPE float ASSERT $value > 0;
DEFINE FIELD OVERWRITE z_index ON TABLE loom_canvas_placements TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE group_id ON TABLE loom_canvas_placements TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_canvas_placements TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE loom_canvas_placements TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_canvas_placements ON TABLE loom_canvas_placements FIELDS placement_id UNIQUE;
DEFINE INDEX OVERWRITE uq_loom_canvas_placement ON TABLE loom_canvas_placements FIELDS canvas_block_id, placed_block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_canvas_placements_canvas ON TABLE loom_canvas_placements FIELDS canvas_block_id, z_index;
DEFINE INDEX OVERWRITE idx_loom_canvas_placements_placed ON TABLE loom_canvas_placements FIELDS placed_block_id;

DEFINE TABLE OVERWRITE loom_canvas_visual_edges SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE visual_edge_id ON TABLE loom_canvas_visual_edges TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'LCV-') AND string::len($value) = 36;
DEFINE FIELD OVERWRITE canvas_block_id ON TABLE loom_canvas_visual_edges TYPE record<loom_canvas_boards> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_canvas_visual_edges TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE from_placement_id ON TABLE loom_canvas_visual_edges TYPE record<loom_canvas_placements> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE to_placement_id ON TABLE loom_canvas_visual_edges TYPE record<loom_canvas_placements> ASSERT record::exists($value) AND $value != $this.from_placement_id REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE label ON TABLE loom_canvas_visual_edges TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_canvas_visual_edges TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_canvas_visual_edges ON TABLE loom_canvas_visual_edges FIELDS visual_edge_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_canvas_visual_edges_canvas ON TABLE loom_canvas_visual_edges FIELDS canvas_block_id;

-- 0336_loom_search_v2.sql. The PostgreSQL-only artifacts of this migration are
-- projected, not copied: `CREATE EXTENSION pg_trgm/vector` has no Surreal
-- counterpart, and the `search_tsv tsvector GENERATED ALWAYS AS
-- to_tsvector('english', search_text) STORED` column is a PostgreSQL derived
-- column whose Surreal equivalent is the search index over `search_text`
-- itself, so no stored duplicate of `search_text` is defined. `embedding
-- public.vector(768)` becomes a typed float vector; the GIN/trgm/HNSW index
-- shapes are engine-specific and are projected as ordinary indexes carrying the
-- source index names so the reindex layer keeps its named targets.
DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_search_index TYPE record<loom_blocks> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_block_search_index TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE content_type ON TABLE loom_block_search_index TYPE string;
DEFINE FIELD OVERWRITE search_text ON TABLE loom_block_search_index TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE embedding ON TABLE loom_block_search_index TYPE option<array<float>> ASSERT $value = NONE OR array::len($value) = 768;
DEFINE FIELD OVERWRITE embedding_model ON TABLE loom_block_search_index TYPE option<string>;
DEFINE FIELD OVERWRITE indexed_at ON TABLE loom_block_search_index TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_loom_block_search_index ON TABLE loom_block_search_index FIELDS block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_block_search_tsv ON TABLE loom_block_search_index FIELDS search_text;
DEFINE INDEX OVERWRITE idx_loom_block_search_embedding ON TABLE loom_block_search_index FIELDS embedding_model, indexed_at;
DEFINE INDEX OVERWRITE idx_loom_block_search_ws_type ON TABLE loom_block_search_index FIELDS workspace_id, content_type;

-- 0340_calendar_activity_spans.sql
DEFINE TABLE OVERWRITE calendar_activity_spans SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE span_id ON TABLE calendar_activity_spans TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE calendar_activity_spans TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE calendar_event_id ON TABLE calendar_activity_spans TYPE string;
DEFINE FIELD OVERWRITE started_utc ON TABLE calendar_activity_spans TYPE datetime;
DEFINE FIELD OVERWRITE ended_utc ON TABLE calendar_activity_spans TYPE option<datetime>;
DEFINE FIELD OVERWRITE edited_doc_ids ON TABLE calendar_activity_spans TYPE array<string> DEFAULT [];
DEFINE FIELD OVERWRITE created_at ON TABLE calendar_activity_spans TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE calendar_activity_spans TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_calendar_activity_spans ON TABLE calendar_activity_spans FIELDS span_id UNIQUE;
DEFINE INDEX OVERWRITE ix_calendar_activity_spans_event ON TABLE calendar_activity_spans FIELDS workspace_id, calendar_event_id;
DEFINE INDEX OVERWRITE ix_calendar_activity_spans_workspace ON TABLE calendar_activity_spans FIELDS workspace_id;

-- 0341_stage_capture_artifacts.sql
DEFINE TABLE OVERWRITE stage_capture_artifacts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE artifact_id ON TABLE stage_capture_artifacts TYPE string ASSERT $value = record::id($this.id) AND string::starts_with($value, 'STGA-') AND string::len($value) = 37;
DEFINE FIELD OVERWRITE workspace_id ON TABLE stage_capture_artifacts TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE content_kind ON TABLE stage_capture_artifacts TYPE 'document' | 'selection' | 'canvas_node' | 'atelier_item';
DEFINE FIELD OVERWRITE label ON TABLE stage_capture_artifacts TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE content_type ON TABLE stage_capture_artifacts TYPE string;
DEFINE FIELD OVERWRITE content_json ON TABLE stage_capture_artifacts TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE content_json.* ON TABLE stage_capture_artifacts TYPE any;
DEFINE FIELD OVERWRITE content_sha256 ON TABLE stage_capture_artifacts TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE manifest ON TABLE stage_capture_artifacts TYPE object FLEXIBLE DEFAULT {};
DEFINE FIELD OVERWRITE manifest.* ON TABLE stage_capture_artifacts TYPE any;
DEFINE FIELD OVERWRITE manifest_ref ON TABLE stage_capture_artifacts TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE source_ref ON TABLE stage_capture_artifacts TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE stage_capture_artifacts TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE stage_capture_artifacts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_stage_capture_artifacts ON TABLE stage_capture_artifacts FIELDS artifact_id UNIQUE;
DEFINE INDEX OVERWRITE idx_stage_capture_artifacts_workspace ON TABLE stage_capture_artifacts FIELDS workspace_id;
DEFINE INDEX OVERWRITE idx_stage_capture_artifacts_content_sha256 ON TABLE stage_capture_artifacts FIELDS content_sha256;

-- 0343_knowledge_rich_document_loom_projection.sql. Only the durable migration
-- provenance table is transcribed. The backfill body (the plpgsql plain-text
-- helper, the collision guards, and the INSERT/UPDATE passes over
-- knowledge_rich_documents / loom_blocks / loom_block_search_index) is DML over
-- pre-existing rows and is intentionally omitted; this bootstrap targets an
-- empty database, so there is nothing to backfill.
DEFINE TABLE OVERWRITE knowledge_rich_document_loom_projection_0343_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE block_id ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string;
DEFINE FIELD OVERWRITE block_existed ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE bool;
DEFINE FIELD OVERWRITE prior_title ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_content_hash ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_derived_json ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_block_updated_at ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<datetime>;
DEFINE FIELD OVERWRITE prior_search_existed ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE bool;
DEFINE FIELD OVERWRITE prior_search_workspace_id ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_search_content_type ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_search_text ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<string>;
DEFINE FIELD OVERWRITE prior_indexed_at ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<datetime>;
DEFINE FIELD OVERWRITE applied_title ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string;
DEFINE FIELD OVERWRITE applied_content_hash ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string;
DEFINE FIELD OVERWRITE applied_derived_json ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string;
DEFINE FIELD OVERWRITE applied_search_text ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE string;
DEFINE FIELD OVERWRITE applied_block_updated_at ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<datetime>;
DEFINE FIELD OVERWRITE applied_indexed_at ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE option<datetime>;
DEFINE FIELD OVERWRITE applied ON TABLE knowledge_rich_document_loom_projection_0343_state TYPE bool DEFAULT false;
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_loom_projection_0343_state ON TABLE knowledge_rich_document_loom_projection_0343_state FIELDS block_id UNIQUE;

-- 0344_atelier_intake_item_loom_projection.sql
DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE item_id ON TABLE atelier_intake_item_loom_projection TYPE record<atelier_intake_item> ASSERT (record::exists($value)) AND ($value = record::id($this.id)) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE loom_block_id ON TABLE atelier_intake_item_loom_projection TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE workspace_id ON TABLE atelier_intake_item_loom_projection TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE linked_by ON TABLE atelier_intake_item_loom_projection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE linked_at_utc ON TABLE atelier_intake_item_loom_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_intake_item_loom_projection ON TABLE atelier_intake_item_loom_projection FIELDS item_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_intake_item_loom_projection_block ON TABLE atelier_intake_item_loom_projection FIELDS loom_block_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_intake_item_loom_projection_workspace ON TABLE atelier_intake_item_loom_projection FIELDS workspace_id, item_id;

-- 0345_fems_memory_workspace_authority.sql. The historical PostgreSQL
-- backfill DML (legacy request-id derivation) is intentionally omitted: this
-- bootstrap targets an empty database. The workspace foreign keys the same
-- migration installs are retained.
DEFINE TABLE OVERWRITE fems_memory_packs SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE pack_id ON TABLE fems_memory_packs TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_packs TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE scope_key ON TABLE fems_memory_packs TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE pack ON TABLE fems_memory_packs TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE pack.* ON TABLE fems_memory_packs TYPE any;
DEFINE FIELD OVERWRITE generated_at ON TABLE fems_memory_packs TYPE datetime;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_packs TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_memory_packs ON TABLE fems_memory_packs FIELDS pack_id UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_packs_scope ON TABLE fems_memory_packs FIELDS workspace_id, scope_key, created_at;

DEFINE TABLE OVERWRITE fems_memory_proposals SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE fems_memory_proposals TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE request_id ON TABLE fems_memory_proposals TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_proposals TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE document_id ON TABLE fems_memory_proposals TYPE string;
DEFINE FIELD OVERWRITE selection_start ON TABLE fems_memory_proposals TYPE int;
DEFINE FIELD OVERWRITE selection_end ON TABLE fems_memory_proposals TYPE int;
DEFINE FIELD OVERWRITE content_hash ON TABLE fems_memory_proposals TYPE string;
DEFINE FIELD OVERWRITE memory_class ON TABLE fems_memory_proposals TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE fems_memory_proposals TYPE string DEFAULT 'pending_review';
DEFINE FIELD OVERWRITE review_gated ON TABLE fems_memory_proposals TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE proposal ON TABLE fems_memory_proposals TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE proposal.* ON TABLE fems_memory_proposals TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_proposals TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_memory_proposals ON TABLE fems_memory_proposals FIELDS proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_proposals_ws_request ON TABLE fems_memory_proposals FIELDS workspace_id, request_id UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_proposals_ws_status ON TABLE fems_memory_proposals FIELDS workspace_id, status, created_at;

DEFINE TABLE OVERWRITE fems_memory_items SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE memory_id ON TABLE fems_memory_items TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_items TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE item ON TABLE fems_memory_items TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE item.* ON TABLE fems_memory_items TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_items TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE fems_memory_items TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_memory_items ON TABLE fems_memory_items FIELDS memory_id UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_items_ws ON TABLE fems_memory_items FIELDS workspace_id;

-- 0350_fems_memory_commit_authority.sql
DEFINE TABLE OVERWRITE fems_memory_commit_reports SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE commit_id ON TABLE fems_memory_commit_reports TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_commit_reports TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE fems_memory_commit_reports TYPE record<fems_memory_proposals> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE memory_id ON TABLE fems_memory_commit_reports TYPE record<fems_memory_items> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE report ON TABLE fems_memory_commit_reports TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE report.* ON TABLE fems_memory_commit_reports TYPE any;
DEFINE FIELD OVERWRITE report_hash ON TABLE fems_memory_commit_reports TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_commit_reports TYPE datetime;
DEFINE INDEX OVERWRITE pk_fems_memory_commit_reports ON TABLE fems_memory_commit_reports FIELDS commit_id UNIQUE;
DEFINE INDEX OVERWRITE uq_fems_memory_commit_reports_proposal ON TABLE fems_memory_commit_reports FIELDS proposal_id UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_commit_reports_workspace_created ON TABLE fems_memory_commit_reports FIELDS workspace_id, created_at, commit_id;

-- 0351_fems_memory_commit_recovery.sql
DEFINE TABLE OVERWRITE fems_memory_commit_fr_outbox SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE event_id ON TABLE fems_memory_commit_fr_outbox TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_commit_fr_outbox TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE fems_memory_commit_fr_outbox TYPE record<fems_memory_proposals> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE commit_id ON TABLE fems_memory_commit_fr_outbox TYPE record<fems_memory_commit_reports> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE event_code ON TABLE fems_memory_commit_fr_outbox TYPE 'FR-EVT-MEM-003' | 'FR-EVT-MEM-004';
DEFINE FIELD OVERWRITE event ON TABLE fems_memory_commit_fr_outbox TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE event.* ON TABLE fems_memory_commit_fr_outbox TYPE any;
DEFINE FIELD OVERWRITE event_hash ON TABLE fems_memory_commit_fr_outbox TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_commit_fr_outbox TYPE datetime;
DEFINE FIELD OVERWRITE published_at ON TABLE fems_memory_commit_fr_outbox TYPE option<datetime> ASSERT $value = NONE OR $value >= $this.created_at;
DEFINE FIELD OVERWRITE attempt_count ON TABLE fems_memory_commit_fr_outbox TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE last_error ON TABLE fems_memory_commit_fr_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE last_error_at ON TABLE fems_memory_commit_fr_outbox TYPE option<datetime>;
DEFINE FIELD OVERWRITE quarantined_at ON TABLE fems_memory_commit_fr_outbox TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_fems_memory_commit_fr_outbox ON TABLE fems_memory_commit_fr_outbox FIELDS event_id UNIQUE;
DEFINE INDEX OVERWRITE uq_fems_memory_commit_fr_outbox_proposal_event ON TABLE fems_memory_commit_fr_outbox FIELDS proposal_id, event_code UNIQUE;
DEFINE INDEX OVERWRITE uq_fems_memory_commit_fr_outbox_commit_event ON TABLE fems_memory_commit_fr_outbox FIELDS commit_id, event_code UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_commit_fr_outbox_pending ON TABLE fems_memory_commit_fr_outbox FIELDS workspace_id, created_at, event_code, event_id;

-- 0352_fems_memory_lifecycle_outbox.sql
DEFINE TABLE OVERWRITE fems_memory_lifecycle_fr_outbox SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE event_id ON TABLE fems_memory_lifecycle_fr_outbox TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_lifecycle_fr_outbox TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE fems_memory_lifecycle_fr_outbox TYPE record<fems_memory_proposals> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE event_code ON TABLE fems_memory_lifecycle_fr_outbox TYPE 'FR-EVT-MEM-001' | 'FR-EVT-MEM-002';
DEFINE FIELD OVERWRITE event ON TABLE fems_memory_lifecycle_fr_outbox TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE event.* ON TABLE fems_memory_lifecycle_fr_outbox TYPE any;
DEFINE FIELD OVERWRITE event_hash ON TABLE fems_memory_lifecycle_fr_outbox TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at ON TABLE fems_memory_lifecycle_fr_outbox TYPE datetime;
DEFINE FIELD OVERWRITE published_at ON TABLE fems_memory_lifecycle_fr_outbox TYPE option<datetime> ASSERT $value = NONE OR $value >= $this.created_at;
DEFINE FIELD OVERWRITE attempt_count ON TABLE fems_memory_lifecycle_fr_outbox TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE last_error ON TABLE fems_memory_lifecycle_fr_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE last_error_at ON TABLE fems_memory_lifecycle_fr_outbox TYPE option<datetime>;
DEFINE FIELD OVERWRITE quarantined_at ON TABLE fems_memory_lifecycle_fr_outbox TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_fems_memory_lifecycle_fr_outbox ON TABLE fems_memory_lifecycle_fr_outbox FIELDS event_id UNIQUE;
DEFINE INDEX OVERWRITE uq_fems_memory_lifecycle_fr_outbox_proposal_event ON TABLE fems_memory_lifecycle_fr_outbox FIELDS proposal_id, event_code UNIQUE;
DEFINE INDEX OVERWRITE idx_fems_memory_lifecycle_fr_outbox_pending ON TABLE fems_memory_lifecycle_fr_outbox FIELDS workspace_id, created_at, event_id;

-- 0353_calendar_lossless_temporal_contract.sql. The additive `calendar_events`
-- columns (start_date, end_date_exclusive, normalization_note,
-- temporal_contract_version) and the all-day date index this migration adds are
-- folded into the 0015 calendar section above.
DEFINE TABLE OVERWRITE calendar_mutation_outbox SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE idempotency_key ON TABLE calendar_mutation_outbox TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE calendar_mutation_outbox TYPE string;
DEFINE FIELD OVERWRITE source_id ON TABLE calendar_mutation_outbox TYPE string;
DEFINE FIELD OVERWRITE calendar_event_id ON TABLE calendar_mutation_outbox TYPE string;
DEFINE FIELD OVERWRITE job_id ON TABLE calendar_mutation_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE workflow_id ON TABLE calendar_mutation_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE actor_kind ON TABLE calendar_mutation_outbox TYPE string;
DEFINE FIELD OVERWRITE actor_id ON TABLE calendar_mutation_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE edit_event_id ON TABLE calendar_mutation_outbox TYPE string;
DEFINE FIELD OVERWRITE ledger_event_id ON TABLE calendar_mutation_outbox TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE payload ON TABLE calendar_mutation_outbox TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE payload.* ON TABLE calendar_mutation_outbox TYPE any;
DEFINE FIELD OVERWRITE created_at ON TABLE calendar_mutation_outbox TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_calendar_mutation_outbox ON TABLE calendar_mutation_outbox FIELDS idempotency_key UNIQUE;
DEFINE INDEX OVERWRITE uq_calendar_mutation_outbox_edit_event ON TABLE calendar_mutation_outbox FIELDS edit_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_calendar_mutation_outbox_workflow ON TABLE calendar_mutation_outbox FIELDS workflow_id, calendar_event_id, created_at;

-- 0360_preference_records.sql. `value`, `default_value`, `old_value`, and
-- `new_value` hold declared PreferenceValueType payloads (bool | int | float |
-- string | enum | json-object), so they are typed as the corresponding Surreal
-- union rather than forced to an object. `event_ledger_event_id` is the
-- Handshake-native EventLedger record link the SurrealDB preference store
-- writes (`SurrealStorage::preference_*`); the source column carried the same
-- ledger identity as opaque text because PostgreSQL had no record type.
DEFINE TABLE OVERWRITE preference_records SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE preference_id ON TABLE preference_records TYPE string;
DEFINE FIELD OVERWRITE scope_kind ON TABLE preference_records TYPE 'global' | 'workspace' | 'surface';
DEFINE FIELD OVERWRITE scope_ref ON TABLE preference_records TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE namespace ON TABLE preference_records TYPE string;
DEFINE FIELD OVERWRITE value_type ON TABLE preference_records TYPE string;
DEFINE FIELD OVERWRITE value ON TABLE preference_records TYPE bool | int | float | string | object FLEXIBLE;
DEFINE FIELD OVERWRITE value.* ON TABLE preference_records TYPE any;
DEFINE FIELD OVERWRITE default_value ON TABLE preference_records TYPE bool | int | float | string | object FLEXIBLE;
DEFINE FIELD OVERWRITE default_value.* ON TABLE preference_records TYPE any;
DEFINE FIELD OVERWRITE source ON TABLE preference_records TYPE 'default' | 'operator' | 'import' | 'migration';
DEFINE FIELD OVERWRITE redaction_class ON TABLE preference_records TYPE string DEFAULT 'public';
DEFINE FIELD OVERWRITE revision ON TABLE preference_records TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE updated_at ON TABLE preference_records TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_by ON TABLE preference_records TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE preference_records TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE pk_preference_records ON TABLE preference_records FIELDS preference_id, scope_kind, scope_ref UNIQUE;
DEFINE INDEX OVERWRITE preference_records_scope_idx ON TABLE preference_records FIELDS scope_kind, scope_ref;

DEFINE TABLE OVERWRITE preference_change_receipts SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE preference_id ON TABLE preference_change_receipts TYPE string;
DEFINE FIELD OVERWRITE scope_kind ON TABLE preference_change_receipts TYPE 'global' | 'workspace' | 'surface';
DEFINE FIELD OVERWRITE scope_ref ON TABLE preference_change_receipts TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE before_revision ON TABLE preference_change_receipts TYPE option<int>;
DEFINE FIELD OVERWRITE after_revision ON TABLE preference_change_receipts TYPE int;
DEFINE FIELD OVERWRITE old_value ON TABLE preference_change_receipts TYPE option<bool | int | float | string | object> FLEXIBLE;
DEFINE FIELD OVERWRITE old_value.* ON TABLE preference_change_receipts TYPE any;
DEFINE FIELD OVERWRITE new_value ON TABLE preference_change_receipts TYPE bool | int | float | string | object FLEXIBLE;
DEFINE FIELD OVERWRITE new_value.* ON TABLE preference_change_receipts TYPE any;
DEFINE FIELD OVERWRITE value_type ON TABLE preference_change_receipts TYPE string;
DEFINE FIELD OVERWRITE source ON TABLE preference_change_receipts TYPE 'default' | 'operator' | 'import' | 'migration';
DEFINE FIELD OVERWRITE actor ON TABLE preference_change_receipts TYPE string DEFAULT '';
DEFINE FIELD OVERWRITE redaction_class ON TABLE preference_change_receipts TYPE string DEFAULT 'public';
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE preference_change_receipts TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE created_at ON TABLE preference_change_receipts TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE preference_change_receipts_history_idx ON TABLE preference_change_receipts FIELDS preference_id, scope_kind, scope_ref, after_revision;

-- 0361_loom_block_view_fr_outbox.sql
DEFINE TABLE OVERWRITE loom_block_view_fr_outbox SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE event_id ON TABLE loom_block_view_fr_outbox TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_block_view_fr_outbox TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_view_fr_outbox TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE operation ON TABLE loom_block_view_fr_outbox TYPE 'create' | 'update';
DEFINE FIELD OVERWRITE event ON TABLE loom_block_view_fr_outbox TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE event.* ON TABLE loom_block_view_fr_outbox TYPE any;
DEFINE FIELD OVERWRITE event_hash ON TABLE loom_block_view_fr_outbox TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE created_at ON TABLE loom_block_view_fr_outbox TYPE datetime;
DEFINE FIELD OVERWRITE published_at ON TABLE loom_block_view_fr_outbox TYPE option<datetime> ASSERT $value = NONE OR $value >= $this.created_at;
DEFINE FIELD OVERWRITE attempt_count ON TABLE loom_block_view_fr_outbox TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE last_error ON TABLE loom_block_view_fr_outbox TYPE option<string>;
DEFINE FIELD OVERWRITE last_error_at ON TABLE loom_block_view_fr_outbox TYPE option<datetime>;
DEFINE FIELD OVERWRITE quarantined_at ON TABLE loom_block_view_fr_outbox TYPE option<datetime>;
DEFINE INDEX OVERWRITE pk_loom_block_view_fr_outbox ON TABLE loom_block_view_fr_outbox FIELDS event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_loom_block_view_fr_outbox_pending ON TABLE loom_block_view_fr_outbox FIELDS workspace_id, created_at, event_id;

-- 0365_fems_proposal_request_id_canonical_identity.sql. Only the durable
-- rekey-journal table is transcribed. The pgcrypto extension, the three
-- PostgreSQL identity functions (`fems_rust_trim`,
-- `fems_proposal_request_id`, `fems_proposal_request_id_0345`), and the
-- re-keying DML over `fems_memory_proposals` are historical-row repair for a
-- database that already had 0345 rows; this bootstrap targets an empty
-- database, so no proposal can carry a legacy identity and there is nothing to
-- re-key. The canonical identity itself is computed by
-- `stable_proposal_request_id` in the Rust API layer.
DEFINE TABLE OVERWRITE fems_memory_proposal_request_id_rekey SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE proposal_id ON TABLE fems_memory_proposal_request_id_rekey TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE fems_memory_proposal_request_id_rekey TYPE string;
DEFINE FIELD OVERWRITE old_request_id ON TABLE fems_memory_proposal_request_id_rekey TYPE string;
DEFINE FIELD OVERWRITE new_request_id ON TABLE fems_memory_proposal_request_id_rekey TYPE string;
DEFINE FIELD OVERWRITE migration ON TABLE fems_memory_proposal_request_id_rekey TYPE string;
DEFINE FIELD OVERWRITE rekeyed_at ON TABLE fems_memory_proposal_request_id_rekey TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_memory_proposal_request_id_rekey ON TABLE fems_memory_proposal_request_id_rekey FIELDS proposal_id UNIQUE;

UPSERT handshake_schema_state:primary SET
    version = $schema_version,
    revision = $schema_revision,
    namespace = $namespace,
    database = $database,
    source_manifest_sha256 = $source_manifest_sha256,
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = '0000000000000000000000000000000000000000000000000000000000000000',
    apply_state = 'schema_applied',
    target_revision = $schema_revision,
    started_at = time::now(),
    completed_at = time::now(),
    updated_at = time::now();

COMMIT TRANSACTION;
