-- WP-KERNEL-005 MT-145 / MT-144: typed Model-Workflow-Diagnostics runtime
-- surfaces for the command log and stale-session detection.
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_command_log (MT-145): an APPEND-ONLY queryable command log tied to
--     sessions and receipts. Each row pins a command invocation to its session,
--     its status, and an optional receipt/evidence ref. Append-only is enforced
--     in the API (no UPDATE/DELETE method) and reinforced here by re-recording
--     the same command_log_id being rejected at INSERT (the PK conflict is
--     surfaced as a typed Validation error, never an upsert).
--   * atelier_diagnostics_session (MT-144): a small heartbeat-bearing session
--     record. last_heartbeat_utc advances on each heartbeat; detect_stale_sessions
--     flags sessions whose last_heartbeat_utc is older than the timeout as STALE.
--     The KEY INVARIANT is that a stale session's evidence is PRESERVED: marking a
--     session STALE is a status flag only and never deletes the session row or any
--     atelier_command_log evidence rows tied to it.

CREATE TABLE IF NOT EXISTS atelier_command_log (
    command_log_id  TEXT PRIMARY KEY,
    session_ref     TEXT NOT NULL,
    command_id      TEXT NOT NULL,
    status          TEXT NOT NULL,
    receipt_ref     TEXT,
    evidence_ref    TEXT,
    recorded_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_command_log_id CHECK (
        btrim(command_log_id) = command_log_id AND command_log_id <> ''
    ),
    CONSTRAINT chk_atelier_command_log_session_ref CHECK (
        btrim(session_ref) = session_ref AND session_ref <> ''
    ),
    CONSTRAINT chk_atelier_command_log_command_id CHECK (
        btrim(command_id) = command_id AND command_id <> ''
    ),
    CONSTRAINT chk_atelier_command_log_status CHECK (
        btrim(status) = status AND status <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_command_log_session
    ON atelier_command_log(session_ref, recorded_at_utc);

CREATE TABLE IF NOT EXISTS atelier_diagnostics_session (
    session_ref       TEXT PRIMARY KEY,
    status            TEXT NOT NULL DEFAULT 'ACTIVE',
    last_heartbeat_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_diag_session_ref CHECK (
        btrim(session_ref) = session_ref AND session_ref <> ''
    ),
    CONSTRAINT chk_atelier_diag_session_status CHECK (
        status IN ('ACTIVE', 'STALE')
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_diagnostics_session_heartbeat
    ON atelier_diagnostics_session(last_heartbeat_utc);
