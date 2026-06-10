-- WP-KERNEL-005 MT-190 / MT-191 / MT-192 / MT-193: DCC Approvals + Visual-Capture
-- panel projections and Flight Recorder workflow event records.
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_dcc_workflow_panel_projection (MT-190): DCC visibility for the
--     APPROVALS and VISUAL_CAPTURE panels only, one typed JSON row per panel
--     instance. Projection only, GUI later. Deliberately a separate table from
--     atelier_dcc_panel_projection (MT-148: SESSION/LEASE/COMMAND_LOG/RECOVERY)
--     so each MT's panel vocabulary stays closed under its own CHECK.
--   * atelier_fr_workflow_event (MT-191..MT-193): one row per WP-KERNEL-005
--     Flight Recorder workflow event. The event_kind vocabulary is the typed
--     FrWorkflowEventKind registry (flight_recorder/workflow_event_kinds.rs):
--       MT-191: FR-EVT-TOOL-CALL / FR-EVT-TOOL-PROPOSAL / FR-EVT-TOOL-APPLY-DECISION
--       MT-192: FR-EVT-VISUAL-CAPTURE / FR-EVT-VISUAL-VALIDATION / FR-EVT-VISUAL-RECOVERY
--       MT-193: FR-EVT-BUILD-GUARD / FR-EVT-PACKAGE-GUARD / FR-EVT-STALE-DOC-DETECTED
--     mt_owner is derived from event_kind in Rust and re-checked here so a row
--     can never claim the wrong owning microtask.

-- MT-190 ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_dcc_workflow_panel_projection (
    panel_id       TEXT PRIMARY KEY,
    panel_kind     TEXT NOT NULL CHECK (
        panel_kind IN ('APPROVALS', 'VISUAL_CAPTURE')
    ),
    state_json     JSONB NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_dcc_workflow_panel_id CHECK (
        btrim(panel_id) = panel_id AND panel_id <> ''
    ),
    -- A panel projection is a structured state object, never a bare scalar.
    CONSTRAINT chk_atelier_dcc_workflow_panel_state_object CHECK (
        jsonb_typeof(state_json) = 'object'
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_dcc_workflow_panel_projection_kind
    ON atelier_dcc_workflow_panel_projection(panel_kind, created_at_utc);

-- MT-191 / MT-192 / MT-193 ---------------------------------------------------
CREATE TABLE IF NOT EXISTS atelier_fr_workflow_event (
    record_id      TEXT PRIMARY KEY,
    event_kind     TEXT NOT NULL CHECK (
        event_kind IN (
            'FR-EVT-TOOL-CALL',
            'FR-EVT-TOOL-PROPOSAL',
            'FR-EVT-TOOL-APPLY-DECISION',
            'FR-EVT-VISUAL-CAPTURE',
            'FR-EVT-VISUAL-VALIDATION',
            'FR-EVT-VISUAL-RECOVERY',
            'FR-EVT-BUILD-GUARD',
            'FR-EVT-PACKAGE-GUARD',
            'FR-EVT-STALE-DOC-DETECTED'
        )
    ),
    mt_owner       TEXT NOT NULL,
    session_ref    TEXT,
    payload        JSONB NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_frwe_record_id CHECK (
        btrim(record_id) = record_id AND record_id <> ''
    ),
    -- Owner pairing: each event kind belongs to exactly one microtask.
    CONSTRAINT chk_atelier_frwe_kind_owner CHECK (
        (
            event_kind IN (
                'FR-EVT-TOOL-CALL', 'FR-EVT-TOOL-PROPOSAL', 'FR-EVT-TOOL-APPLY-DECISION'
            ) AND mt_owner = 'MT-191'
        )
        OR (
            event_kind IN (
                'FR-EVT-VISUAL-CAPTURE', 'FR-EVT-VISUAL-VALIDATION', 'FR-EVT-VISUAL-RECOVERY'
            ) AND mt_owner = 'MT-192'
        )
        OR (
            event_kind IN (
                'FR-EVT-BUILD-GUARD', 'FR-EVT-PACKAGE-GUARD', 'FR-EVT-STALE-DOC-DETECTED'
            ) AND mt_owner = 'MT-193'
        )
    ),
    -- An FR workflow event payload is a structured object, never a scalar.
    CONSTRAINT chk_atelier_frwe_payload_object CHECK (
        jsonb_typeof(payload) = 'object'
    ),
    -- session_ref, when present, must be an unpadded product ref: no .GOV, no
    -- SQLite, no Windows drive, no file: URL, no localhost/loopback authority.
    CONSTRAINT chk_atelier_frwe_session_ref CHECK (
        session_ref IS NULL
        OR (
            btrim(session_ref) = session_ref
            AND session_ref <> ''
            AND session_ref NOT ILIKE '%.GOV%'
            AND session_ref !~ '\s'
            AND session_ref !~* '^[A-Za-z]:'
            AND session_ref NOT ILIKE 'file:%'
            AND session_ref NOT ILIKE '%.sqlite%'
            AND session_ref NOT ILIKE '%.sqlite3%'
            AND session_ref NOT ILIKE 'sqlite:%'
            AND session_ref NOT ILIKE '%localhost%'
            AND session_ref NOT ILIKE '%127.0.0.1%'
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_fr_workflow_event_kind
    ON atelier_fr_workflow_event(event_kind, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_atelier_fr_workflow_event_owner
    ON atelier_fr_workflow_event(mt_owner, created_at_utc);
