-- WP-KERNEL-005 MT-140 / MT-207: typed Model-Workflow-Diagnostics runtime surfaces.
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_diagnostics_error_taxonomy (MT-140): the structured error-class
--     registry. Each canonical diagnostics error class carries a human
--     description and a mandatory non-empty recovery hint, so a no-context model
--     hitting an error class always has an actionable next step.
--   * atelier_diagnostics_prompt_response_matrix (MT-207): preserves the CKC
--     WP-0118 model prompt-response matrix as a DEFERRED contract -- prompt set,
--     expected-response shape, and scoring schema -- WITHOUT implementing live
--     scoring early. status defaults to 'DEFERRED'.
--
-- MT-166 (Installer Reset And Orphan Evidence Projection) adds no table here: it
-- is a read projection over the existing atelier_reset_operation and
-- atelier_orphan_manifest_item tables (migration 0089).

CREATE TABLE IF NOT EXISTS atelier_diagnostics_error_taxonomy (
    class          TEXT PRIMARY KEY,
    description    TEXT NOT NULL,
    recovery_hint  TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_diag_error_taxonomy_class CHECK (
        btrim(class) = class AND class <> ''
    ),
    CONSTRAINT chk_atelier_diag_error_taxonomy_description CHECK (
        btrim(description) = description AND description <> ''
    ),
    CONSTRAINT chk_atelier_diag_error_taxonomy_recovery CHECK (
        btrim(recovery_hint) = recovery_hint AND recovery_hint <> ''
    )
);

CREATE TABLE IF NOT EXISTS atelier_diagnostics_prompt_response_matrix (
    entry_id                TEXT PRIMARY KEY,
    prompt_text             TEXT NOT NULL,
    expected_response_shape JSONB NOT NULL,
    scoring_schema          JSONB NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'DEFERRED',
    created_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_diag_prm_entry_id CHECK (
        btrim(entry_id) = entry_id AND entry_id <> ''
    ),
    CONSTRAINT chk_atelier_diag_prm_prompt CHECK (
        btrim(prompt_text) = prompt_text AND prompt_text <> ''
    ),
    CONSTRAINT chk_atelier_diag_prm_status CHECK (
        status IN ('DEFERRED', 'ACTIVE')
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_diagnostics_prompt_response_matrix_status
    ON atelier_diagnostics_prompt_response_matrix(status, entry_id);
