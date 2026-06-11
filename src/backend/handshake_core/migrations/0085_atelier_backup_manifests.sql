-- WP-KERNEL-005 MT-049 backup manifest and restore preflight guards.
-- Backup records preserve version traceability and checksums; restore preflight
-- records the compatibility decision before any restore work is attempted.

CREATE TABLE IF NOT EXISTS atelier_backup_manifest (
    backup_id      UUID PRIMARY KEY,
    app_version    TEXT NOT NULL,
    spec_version   TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    artifact_ref   TEXT NOT NULL,
    content_hash   TEXT NOT NULL,
    byte_len       BIGINT NOT NULL,
    manifest_hash  TEXT NOT NULL UNIQUE,
    manifest_json  JSONB NOT NULL,
    created_by     TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_backup_manifest_versions
        CHECK (
            btrim(app_version) = app_version
            AND app_version <> ''
            AND app_version !~ '[[:space:]]'
            AND btrim(spec_version) = spec_version
            AND spec_version <> ''
            AND spec_version !~ '[[:space:]]'
            AND schema_version > 0
        ),
    CONSTRAINT chk_atelier_backup_manifest_artifact_ref
        CHECK (
            btrim(artifact_ref) = artifact_ref
            AND artifact_ref <> ''
            AND artifact_ref !~ '[[:space:]]'
            AND artifact_ref ~ '^artifact://'
            AND artifact_ref !~* '(^[a-z]:|\.gov|file:|localhost|sqlite|\\|/\.\./)'
        ),
    CONSTRAINT chk_atelier_backup_manifest_checksums
        CHECK (
            btrim(content_hash) = content_hash
            AND content_hash <> ''
            AND content_hash !~ '[[:space:]]'
            AND btrim(manifest_hash) = manifest_hash
            AND manifest_hash <> ''
            AND manifest_hash !~ '[[:space:]]'
            AND byte_len > 0
        ),
    CONSTRAINT chk_atelier_backup_manifest_created_by_trimmed
        CHECK (btrim(created_by) = created_by AND created_by <> ''),
    CONSTRAINT chk_atelier_backup_manifest_json_contract
        CHECK (
            jsonb_typeof(manifest_json) = 'object'
            AND manifest_json->>'schema_id' = 'hsk.atelier.backup_manifest@1'
            AND jsonb_typeof(manifest_json->'artifact') = 'object'
            AND jsonb_typeof(manifest_json->'files') = 'array'
            AND manifest_json::text !~* '\.gov'
        )
);

CREATE INDEX IF NOT EXISTS idx_atelier_backup_manifest_created
    ON atelier_backup_manifest(created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_backup_restore_preflight (
    preflight_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_id              UUID NOT NULL REFERENCES atelier_backup_manifest(backup_id) ON DELETE CASCADE,
    current_app_version    TEXT NOT NULL,
    current_spec_version   TEXT NOT NULL,
    current_schema_version INTEGER NOT NULL,
    status                 TEXT NOT NULL,
    refusal_reason         TEXT,
    requested_by           TEXT NOT NULL,
    created_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_backup_restore_preflight_versions
        CHECK (
            btrim(current_app_version) = current_app_version
            AND current_app_version <> ''
            AND current_app_version !~ '[[:space:]]'
            AND btrim(current_spec_version) = current_spec_version
            AND current_spec_version <> ''
            AND current_spec_version !~ '[[:space:]]'
            AND current_schema_version > 0
        ),
    CONSTRAINT chk_atelier_backup_restore_preflight_status
        CHECK (status IN ('accepted', 'refused')),
    CONSTRAINT chk_atelier_backup_restore_preflight_refusal
        CHECK (
            (status = 'accepted' AND refusal_reason IS NULL)
            OR (status = 'refused' AND refusal_reason IS NOT NULL AND btrim(refusal_reason) = refusal_reason AND refusal_reason <> '')
        ),
    CONSTRAINT chk_atelier_backup_restore_preflight_requested_by_trimmed
        CHECK (btrim(requested_by) = requested_by AND requested_by <> '')
);

CREATE INDEX IF NOT EXISTS idx_atelier_backup_restore_preflight_backup
    ON atelier_backup_restore_preflight(backup_id, created_at_utc DESC);
