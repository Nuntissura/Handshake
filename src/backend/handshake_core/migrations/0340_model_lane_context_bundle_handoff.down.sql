-- Down-migration for 0340_model_lane_context_bundle_handoff.sql (WP-1 MT-005).

DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_record_gin;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_locus;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_source_message;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_selection;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_bundle_replay;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_handoffs_event_seq;
DROP TABLE IF EXISTS model_lane_context_bundle_handoffs;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_artifacts_record_gin;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_artifacts_locus;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_artifacts_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_context_bundle_artifacts_event_seq;
DROP TABLE IF EXISTS model_lane_context_bundle_artifacts;

DELETE FROM model_lane_schema_registry
WHERE schema_id IN (
    'hsk.model_lane_context_bundle_artifact@1',
    'hsk.model_lane_context_bundle_handoff@1'
);
