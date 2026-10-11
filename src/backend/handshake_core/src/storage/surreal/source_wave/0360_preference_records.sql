-- WP-KERNEL-012 MT-072 remediation (FAIL_V2): canonical PostgreSQL PreferenceRecord authority.
-- Master Spec v02.201 §10.17 (Settings and Preferences Domain). Editor settings migrate off the
-- opaque workspace-settings JSON document onto typed, validated, revisioned preference records with
-- EventLedger-backed change receipts. PostgreSQL is the only storage authority (SET-STORE-001/002 —
-- no SQLite anywhere). See src/preferences/mod.rs and src/storage/preferences.rs.

-- Canonical current-value store. One row per (preference_id, scope_kind, scope_ref). A row exists only
-- once a preference has been explicitly set/reset; an unset defined preference resolves to its registry
-- default at read time (SET-REC-003), so absence of a row is a valid, meaningful state.
CREATE TABLE IF NOT EXISTS preference_records (
    preference_id          TEXT        NOT NULL,
    scope_kind             TEXT        NOT NULL,
    scope_ref              TEXT        NOT NULL DEFAULT '',
    namespace              TEXT        NOT NULL,
    value_type             TEXT        NOT NULL,
    value                  JSONB       NOT NULL,
    default_value          JSONB       NOT NULL,
    source                 TEXT        NOT NULL,
    redaction_class        TEXT        NOT NULL DEFAULT 'public',
    revision               BIGINT      NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by             TEXT        NOT NULL DEFAULT '',
    event_ledger_event_id  TEXT        NOT NULL,
    PRIMARY KEY (preference_id, scope_kind, scope_ref),
    CONSTRAINT preference_records_scope_kind_ck
        CHECK (scope_kind IN ('global', 'workspace', 'surface')),
    CONSTRAINT preference_records_source_ck
        CHECK (source IN ('default', 'operator', 'import', 'migration')),
    CONSTRAINT preference_records_revision_ck
        CHECK (revision >= 1)
);

CREATE INDEX IF NOT EXISTS preference_records_scope_idx
    ON preference_records (scope_kind, scope_ref);

-- Recoverable change receipts (SET-EVT-002). Append-only history: every set/reset/import/migration
-- writes one immutable row carrying before/after revision + old/new value + a pointer to the EventLedger
-- entry, sufficient to replay or revert. A reset-to-default is recorded as a mutation here, never a
-- provenance-losing delete. The change history surfaced to the operator (SET-UI-003) reads from here.
CREATE TABLE IF NOT EXISTS preference_change_receipts (
    receipt_id             TEXT        PRIMARY KEY,
    preference_id          TEXT        NOT NULL,
    scope_kind             TEXT        NOT NULL,
    scope_ref              TEXT        NOT NULL DEFAULT '',
    before_revision        BIGINT,
    after_revision         BIGINT      NOT NULL,
    old_value              JSONB,
    new_value              JSONB       NOT NULL,
    value_type             TEXT        NOT NULL,
    source                 TEXT        NOT NULL,
    actor                  TEXT        NOT NULL DEFAULT '',
    redaction_class        TEXT        NOT NULL DEFAULT 'public',
    event_ledger_event_id  TEXT        NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT preference_change_receipts_scope_kind_ck
        CHECK (scope_kind IN ('global', 'workspace', 'surface')),
    CONSTRAINT preference_change_receipts_source_ck
        CHECK (source IN ('default', 'operator', 'import', 'migration'))
);

CREATE INDEX IF NOT EXISTS preference_change_receipts_history_idx
    ON preference_change_receipts (preference_id, scope_kind, scope_ref, after_revision);
