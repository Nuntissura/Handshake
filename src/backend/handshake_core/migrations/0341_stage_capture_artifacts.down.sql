-- Down: drop the WP-KERNEL-012 MT-066 Stage capture artifact store.
DROP INDEX IF EXISTS idx_stage_capture_artifacts_content_sha256;
DROP INDEX IF EXISTS idx_stage_capture_artifacts_workspace;
DROP TABLE IF EXISTS stage_capture_artifacts;
