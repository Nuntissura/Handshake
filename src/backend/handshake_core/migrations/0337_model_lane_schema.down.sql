-- Down-migration for 0337_model_lane_schema.sql (WP-1 MT-002).

DROP INDEX IF EXISTS idx_model_lane_messages_record_gin;
DROP INDEX IF EXISTS idx_model_lane_messages_locus;
DROP INDEX IF EXISTS idx_model_lane_messages_payload_hash;
DROP INDEX IF EXISTS idx_model_lane_messages_run_replay;
DROP INDEX IF EXISTS idx_model_lane_messages_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_messages_event_seq;
DROP TABLE IF EXISTS model_lane_messages;

DROP INDEX IF EXISTS idx_model_lanes_locus;
DROP INDEX IF EXISTS idx_model_lanes_run_replay;
DROP INDEX IF EXISTS idx_model_lanes_event_seq;
DROP TABLE IF EXISTS model_lanes;

DROP INDEX IF EXISTS idx_model_lane_runs_locus;
DROP INDEX IF EXISTS idx_model_lane_runs_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_runs_event_seq;
DROP TABLE IF EXISTS model_lane_runs;

DROP TABLE IF EXISTS model_lane_schema_registry;
