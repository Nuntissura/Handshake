-- WP-KERNEL-005 MT-096: multi-rig pose workspace tab/panel state.
-- Stores open rigs, active rig, tab order, dirty calibration, and structured
-- panel state for model-safe recovery without screen scraping.

CREATE TABLE IF NOT EXISTS atelier_pose_workspace_rig_state (
    workspace_ref     TEXT NOT NULL,
    session_ref       TEXT NOT NULL,
    rig_id            UUID NOT NULL REFERENCES atelier_pose_rig(rig_id) ON DELETE RESTRICT,
    open              BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order        INTEGER NOT NULL CHECK (sort_order >= 0),
    active            BOOLEAN NOT NULL DEFAULT FALSE,
    dirty_calibration BOOLEAN NOT NULL DEFAULT FALSE,
    panel_state       JSONB NOT NULL DEFAULT '{}'::jsonb,
    requested_by      TEXT NOT NULL,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_ref, session_ref, rig_id),
    CONSTRAINT chk_atelier_pose_workspace_rig_state_workspace_ref CHECK (
        btrim(workspace_ref) = workspace_ref
        AND workspace_ref <> ''
        AND workspace_ref NOT ILIKE '%.GOV%'
        AND workspace_ref !~ '\s'
        AND workspace_ref !~ '\\'
        AND workspace_ref !~ '^[A-Za-z]:'
        AND workspace_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_workspace_rig_state_session_ref CHECK (
        btrim(session_ref) = session_ref
        AND session_ref <> ''
        AND session_ref NOT ILIKE '%.GOV%'
        AND session_ref !~ '\s'
        AND session_ref !~ '\\'
        AND session_ref !~ '^[A-Za-z]:'
        AND session_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_workspace_rig_state_requested_by CHECK (
        btrim(requested_by) = requested_by
        AND requested_by <> ''
        AND requested_by NOT ILIKE '%.GOV%'
        AND requested_by !~ '^[A-Za-z]:'
        AND requested_by NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_workspace_rig_state_panel_state CHECK (
        jsonb_typeof(panel_state) = 'object'
    ),
    CONSTRAINT chk_atelier_pose_workspace_rig_state_open_active CHECK (
        open OR active = FALSE
    )
);

-- Existing MT-096 remediation attempts created this table before session/open
-- state existed. Add those columns here before index DDL references them; the
-- follow-up upgrade migration tightens constraints and primary key shape.
ALTER TABLE atelier_pose_workspace_rig_state
    ADD COLUMN IF NOT EXISTS session_ref TEXT NOT NULL DEFAULT 'pose-session://default',
    ADD COLUMN IF NOT EXISTS open BOOLEAN NOT NULL DEFAULT TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_pose_workspace_rig_state_active
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref)
    WHERE active AND open;

CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_pose_workspace_rig_state_open_order
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref, sort_order)
    WHERE open;

CREATE INDEX IF NOT EXISTS idx_atelier_pose_workspace_rig_state_order
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref, open, sort_order, rig_id);
