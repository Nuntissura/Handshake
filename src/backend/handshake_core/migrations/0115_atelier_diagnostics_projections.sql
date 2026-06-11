-- WP-KERNEL-005 MT-147 / MT-148 / MT-153 / MT-167: typed Model-Workflow-Diagnostics
-- projection surfaces.
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_work_state_projection (MT-147): projects atelier model work state
--     (active MT, owner, status, blocker, receipts, next action, evidence) into a
--     typed Locus/MT diagnostics row so a no-context model can read the live work
--     state without inferring it from prose.
--   * atelier_dcc_panel_projection (MT-148): a typed projection exposing the DCC
--     (diagnostics control center) session / lease / command-log / recovery panel
--     state as JSON, one row per panel instance. Projection only, GUI later.
--   * atelier_screenshot_artifact_storage (MT-153): governs a stealth screenshot
--     capture as a durable artifact with metadata (mime, dimensions, byte length,
--     label) and retention (ttl days, pinned). Extends the existing
--     atelier_stealth_capture receipt (migration referenced by stealth_window.rs)
--     by FK so a capture becomes a retained, described screenshot artifact.
--   * atelier_spec_drift_finding (MT-167): preserves and generalizes the CKC
--     README-vs-spec drift finding -- a typed record of a doc/spec pointer drift
--     (doc_ref, spec_ref, drift_kind, detail) recorded when a doc-claimed surface
--     differs from the spec/code surface it points at.

-- MT-147 ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_work_state_projection (
    projection_id  TEXT PRIMARY KEY,
    active_mt      TEXT NOT NULL,
    owner          TEXT NOT NULL,
    status         TEXT NOT NULL,
    blocker        TEXT,
    receipts_ref   TEXT,
    next_action    TEXT,
    evidence_ref   TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_wsp_projection_id CHECK (
        btrim(projection_id) = projection_id AND projection_id <> ''
    ),
    CONSTRAINT chk_atelier_wsp_active_mt CHECK (
        btrim(active_mt) = active_mt AND active_mt <> ''
    ),
    CONSTRAINT chk_atelier_wsp_owner CHECK (
        btrim(owner) = owner AND owner <> ''
    ),
    CONSTRAINT chk_atelier_wsp_status CHECK (
        btrim(status) = status AND status <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_work_state_projection_active_mt
    ON atelier_work_state_projection(active_mt, created_at_utc);

-- MT-148 ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_dcc_panel_projection (
    panel_id       TEXT PRIMARY KEY,
    panel_kind     TEXT NOT NULL CHECK (
        panel_kind IN ('SESSION', 'LEASE', 'COMMAND_LOG', 'RECOVERY')
    ),
    state_json     JSONB NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_dcc_panel_id CHECK (
        btrim(panel_id) = panel_id AND panel_id <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_dcc_panel_projection_kind
    ON atelier_dcc_panel_projection(panel_kind, created_at_utc);

-- MT-153 ---------------------------------------------------------------------
-- One retained, described screenshot artifact per stealth capture. References
-- the governed ArtifactStore manifest id only (never raw pixels / paths), with
-- diagnostic metadata + retention. content_sha256 ties it to the capture payload.
CREATE TABLE IF NOT EXISTS atelier_screenshot_artifact_storage (
    storage_id           TEXT PRIMARY KEY,
    capture_id           UUID NOT NULL,
    artifact_manifest_id TEXT NOT NULL,
    content_sha256       TEXT NOT NULL,
    mime                 TEXT NOT NULL,
    width_px             INTEGER,
    height_px            INTEGER,
    byte_len             BIGINT,
    label                TEXT,
    retention_ttl_days   INTEGER,
    pinned               BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_sas_storage_id CHECK (
        btrim(storage_id) = storage_id AND storage_id <> ''
    ),
    CONSTRAINT chk_atelier_sas_manifest CHECK (
        btrim(artifact_manifest_id) = artifact_manifest_id
        AND artifact_manifest_id <> ''
        AND artifact_manifest_id NOT ILIKE '%.GOV%'
        AND artifact_manifest_id !~* '^[A-Za-z]:'
        AND artifact_manifest_id NOT ILIKE 'file:%'
        AND artifact_manifest_id NOT ILIKE '%.sqlite%'
        AND artifact_manifest_id NOT ILIKE 'sqlite:%'
        AND artifact_manifest_id NOT ILIKE '%localhost%'
        AND artifact_manifest_id NOT ILIKE '%127.0.0.1%'
    ),
    CONSTRAINT chk_atelier_sas_sha CHECK (
        btrim(content_sha256) = content_sha256 AND content_sha256 <> ''
    ),
    CONSTRAINT chk_atelier_sas_mime CHECK (
        btrim(mime) = mime AND mime <> ''
    ),
    CONSTRAINT chk_atelier_sas_ttl CHECK (
        retention_ttl_days IS NULL OR retention_ttl_days >= 0
    )
);

-- One storage row per capture (idempotent re-record updates in place).
CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_screenshot_artifact_storage_capture
    ON atelier_screenshot_artifact_storage(capture_id);

-- MT-167 ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_spec_drift_finding (
    finding_id     TEXT PRIMARY KEY,
    doc_ref        TEXT NOT NULL,
    spec_ref       TEXT NOT NULL,
    drift_kind     TEXT NOT NULL,
    detail         TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_sdf_finding_id CHECK (
        btrim(finding_id) = finding_id AND finding_id <> ''
    ),
    CONSTRAINT chk_atelier_sdf_doc_ref CHECK (
        btrim(doc_ref) = doc_ref AND doc_ref <> ''
    ),
    CONSTRAINT chk_atelier_sdf_spec_ref CHECK (
        btrim(spec_ref) = spec_ref AND spec_ref <> ''
    ),
    CONSTRAINT chk_atelier_sdf_drift_kind CHECK (
        btrim(drift_kind) = drift_kind AND drift_kind <> ''
    ),
    CONSTRAINT chk_atelier_sdf_detail CHECK (
        btrim(detail) = detail AND detail <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_spec_drift_finding_kind
    ON atelier_spec_drift_finding(drift_kind, created_at_utc);
