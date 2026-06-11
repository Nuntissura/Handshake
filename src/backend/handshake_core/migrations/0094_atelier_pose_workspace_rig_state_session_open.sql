-- WP-KERNEL-005 MT-096 upgrade: make multi-rig workspace state
-- session-scoped, explicitly closable, and deterministic by open-tab order.

ALTER TABLE atelier_pose_workspace_rig_state
    ADD COLUMN IF NOT EXISTS session_ref TEXT NOT NULL DEFAULT 'pose-session://default',
    ADD COLUMN IF NOT EXISTS open BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE atelier_pose_workspace_rig_state
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_workspace_rig_state_session_ref,
    ADD CONSTRAINT chk_atelier_pose_workspace_rig_state_session_ref CHECK (
        btrim(session_ref) = session_ref
        AND session_ref <> ''
        AND session_ref NOT ILIKE '%.GOV%'
        AND session_ref !~ '\s'
        AND session_ref !~ '\\'
        AND session_ref !~ '^[A-Za-z]:'
        AND session_ref NOT LIKE 'file:%'
    );

ALTER TABLE atelier_pose_workspace_rig_state
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_workspace_rig_state_open_active,
    ADD CONSTRAINT chk_atelier_pose_workspace_rig_state_open_active CHECK (
        open OR active = FALSE
    );

DROP INDEX IF EXISTS uq_atelier_pose_workspace_rig_state_active;
DROP INDEX IF EXISTS uq_atelier_pose_workspace_rig_state_open_order;
DROP INDEX IF EXISTS idx_atelier_pose_workspace_rig_state_order;

ALTER TABLE atelier_pose_workspace_rig_state
    DROP CONSTRAINT IF EXISTS atelier_pose_workspace_rig_state_pkey,
    ADD PRIMARY KEY (workspace_ref, session_ref, rig_id);

CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_pose_workspace_rig_state_active
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref)
    WHERE active AND open;

CREATE UNIQUE INDEX IF NOT EXISTS uq_atelier_pose_workspace_rig_state_open_order
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref, sort_order)
    WHERE open;

CREATE INDEX IF NOT EXISTS idx_atelier_pose_workspace_rig_state_order
    ON atelier_pose_workspace_rig_state(workspace_ref, session_ref, open, sort_order, rig_id);
