-- Down-migration for 0339_model_lane_promotion_decision.sql (WP-1 MT-004).

DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_record_gin;
DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_locus;
DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_gate_receipt;
DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_run_replay;
DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_promotion_decisions_event_seq;
DROP TABLE IF EXISTS model_lane_promotion_decisions;

DELETE FROM model_lane_schema_registry
WHERE schema_id = 'hsk.model_lane_promotion_decision@1';
