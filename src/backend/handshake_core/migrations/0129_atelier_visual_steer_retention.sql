-- WP-KERNEL-005 MT-156 / MT-158: visual STEER feedback records + visual
-- evidence retention/redaction hardening.
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_visual_steer_feedback (MT-156): one actionable STEER feedback
--     record per (loop_id, evidence_id) visual threshold breach. Converts a
--     threshold-exceeded visual mismatch into a typed, routable record (target
--     role, receipt kind, code diff ref, visual diff ref, concrete next action)
--     instead of a silent failure or generic prose.
--   * atelier_screenshot_artifact_storage retention/redaction columns (MT-158):
--     bound visual artifacts and protect sensitive captures. `retention_class`
--     names the retention policy bucket, `exportable` gates export (an
--     unredacted capture must never be exportable), `redaction_applied` records
--     whether sensitive regions were blacked out before storage.

-- MT-156 ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_visual_steer_feedback (
    feedback_id            TEXT PRIMARY KEY,
    loop_id                TEXT NOT NULL,
    evidence_id            TEXT NOT NULL,
    wp_id                  TEXT NOT NULL,
    mismatch_basis_points  INTEGER NOT NULL,
    threshold_basis_points INTEGER NOT NULL,
    target_role            TEXT NOT NULL,
    receipt_kind           TEXT NOT NULL,
    code_diff_ref          TEXT NOT NULL,
    visual_diff_ref        TEXT NOT NULL,
    next_action            TEXT NOT NULL,
    created_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_atelier_visual_steer_loop_evidence UNIQUE (loop_id, evidence_id),
    CONSTRAINT chk_atelier_vsf_feedback_id CHECK (
        btrim(feedback_id) = feedback_id AND feedback_id <> ''
    ),
    CONSTRAINT chk_atelier_vsf_loop_id CHECK (
        btrim(loop_id) = loop_id AND loop_id <> ''
    ),
    CONSTRAINT chk_atelier_vsf_evidence_id CHECK (
        btrim(evidence_id) = evidence_id AND evidence_id <> ''
    ),
    CONSTRAINT chk_atelier_vsf_breach CHECK (
        mismatch_basis_points > threshold_basis_points
    ),
    CONSTRAINT chk_atelier_vsf_receipt_kind CHECK (receipt_kind = 'STEER'),
    CONSTRAINT chk_atelier_vsf_next_action CHECK (
        btrim(next_action) = next_action AND next_action <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_visual_steer_feedback_loop
    ON atelier_visual_steer_feedback (loop_id, created_at_utc DESC);

-- MT-158 ---------------------------------------------------------------------
ALTER TABLE atelier_screenshot_artifact_storage
    ADD COLUMN IF NOT EXISTS retention_class TEXT NOT NULL DEFAULT 'visual-validation';
ALTER TABLE atelier_screenshot_artifact_storage
    ADD COLUMN IF NOT EXISTS exportable BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE atelier_screenshot_artifact_storage
    ADD COLUMN IF NOT EXISTS redaction_applied BOOLEAN NOT NULL DEFAULT FALSE;
