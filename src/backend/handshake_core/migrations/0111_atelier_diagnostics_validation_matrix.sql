-- WP-KERNEL-005 MT-171..MT-175: typed diagnostics "validation matrix".
--
-- Each row is one required check that a model-workflow diagnostic surface must
-- cover (model manual + action catalog, session lease + heartbeat, command log +
-- error class + state probe, DCC + Flight Recorder projection, visual evidence).
-- This is the typed runtime surface that turns "diagnostics are covered" into a
-- real PostgreSQL row + EventLedger event, never governance markdown. Rows are
-- PostgreSQL authority and are mirrored through the Atelier EventLedger family
-- (DIAGNOSTICS_VALIDATION_ROW_RECORDED) when a row is recorded.
--
-- No local filesystem, no .GOV refs, no SQLite, no localhost in evidence_ref.

CREATE TABLE IF NOT EXISTS atelier_diagnostics_validation_matrix (
    row_id         TEXT PRIMARY KEY,
    matrix_kind    TEXT NOT NULL,
    surface        TEXT NOT NULL,
    check_id       TEXT NOT NULL,
    requirement    TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN ('REQUIRED', 'COVERED', 'DEFERRED')),
    evidence_ref   TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_dvm_row_id CHECK (
        btrim(row_id) = row_id AND row_id <> ''
    ),
    CONSTRAINT chk_atelier_dvm_matrix_kind CHECK (
        btrim(matrix_kind) = matrix_kind AND matrix_kind <> ''
    ),
    CONSTRAINT chk_atelier_dvm_surface CHECK (
        btrim(surface) = surface AND surface <> ''
    ),
    CONSTRAINT chk_atelier_dvm_check_id CHECK (
        btrim(check_id) = check_id AND check_id <> ''
    ),
    CONSTRAINT chk_atelier_dvm_requirement CHECK (
        btrim(requirement) = requirement AND requirement <> ''
    ),
    -- evidence_ref, when present, must be an unpadded product ref: no .GOV, no
    -- SQLite, no Windows drive, no file: URL, no localhost/loopback authority.
    CONSTRAINT chk_atelier_dvm_evidence_ref CHECK (
        evidence_ref IS NULL
        OR (
            btrim(evidence_ref) = evidence_ref
            AND evidence_ref <> ''
            AND evidence_ref NOT ILIKE '%.GOV%'
            AND evidence_ref !~ '\s'
            AND evidence_ref !~* '^[A-Za-z]:'
            AND evidence_ref NOT ILIKE 'file:%'
            AND evidence_ref NOT ILIKE '%.sqlite%'
            AND evidence_ref NOT ILIKE '%.sqlite3%'
            AND evidence_ref NOT ILIKE 'sqlite:%'
            AND evidence_ref NOT ILIKE '%localhost%'
            AND evidence_ref NOT ILIKE '%127.0.0.1%'
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_diagnostics_validation_matrix_kind
    ON atelier_diagnostics_validation_matrix(matrix_kind, surface, created_at_utc);
