-- WP-KERNEL-004 cluster X.4 MT-197 down migration.
-- Drops the hardening triggers and CHECK constraints added in
-- 0025_observability_spans.sql. The base tables remain (they belong to
-- 0024_session_checkpoint.sql).

DROP TRIGGER IF EXISTS trg_kernel_activity_span_block_attribute_update
    ON kernel_activity_span;
DROP FUNCTION IF EXISTS kernel_activity_span_block_attribute_update();

DROP TRIGGER IF EXISTS trg_kernel_session_span_block_attribute_update
    ON kernel_model_session_span;
DROP FUNCTION IF EXISTS kernel_session_span_block_attribute_update();

ALTER TABLE kernel_activity_span
    DROP CONSTRAINT IF EXISTS chk_kernel_activity_span_status;
ALTER TABLE kernel_model_session_span
    DROP CONSTRAINT IF EXISTS chk_kernel_model_session_span_status;

ALTER TABLE kernel_activity_span
    DROP CONSTRAINT IF EXISTS chk_kernel_activity_span_end_after_start;
ALTER TABLE kernel_model_session_span
    DROP CONSTRAINT IF EXISTS chk_kernel_model_session_span_end_after_start;
