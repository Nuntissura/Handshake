-- MT-016: reusable CKC sheet-version artifact links.
-- Character sheet versions can hold typed refs to Posekit/OpenPose and ComfyUI
-- artifacts without copying payload bytes into sheet text.

CREATE OR REPLACE FUNCTION atelier_is_native_portable_ref(candidate TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    WITH normalized AS (
        SELECT replace(lower(candidate), E'\\', '/') AS v
    ),
    probes AS (
        SELECT
            v,
            split_part(split_part(v, '?', 1), '#', 1) AS sqlite_probe
        FROM normalized
    )
    SELECT candidate IS NOT NULL
       AND btrim(candidate) = candidate
       AND candidate <> ''
       AND candidate !~ '\s'
       AND position(E'\\' in candidate) = 0
       AND v NOT LIKE '%.gov%'
       AND v NOT LIKE 'sqlite:%'
       AND sqlite_probe NOT LIKE '%.sqlite'
       AND sqlite_probe NOT LIKE '%.sqlite/%'
       AND sqlite_probe NOT LIKE '%.sqlite3'
       AND sqlite_probe NOT LIKE '%.sqlite3/%'
       AND sqlite_probe NOT LIKE '%.db'
       AND sqlite_probe NOT LIKE '%.db/%'
       AND v !~ '^[a-z]:'
       AND v !~ '/[a-z]:'
       AND v NOT LIKE 'file:%'
       AND v NOT LIKE '%file://%'
       AND v NOT LIKE '//%'
       AND v NOT LIKE '/%'
       AND v NOT LIKE '~/%'
       AND v NOT LIKE '%userprofile%'
       AND v NOT LIKE '../%'
       AND v NOT LIKE '%/../%'
       AND v NOT LIKE '%/..'
       AND v NOT LIKE 'electron:%'
       AND v NOT LIKE '%/electron/%'
       AND v !~ '(^|[/:.?#&=@])(ckc|castkit|electron)($|[/:.?#&=@])'
       AND v !~ '^(llm|openai|anthropic|ollama|model-server|model_server):'
       AND v !~ '^[a-z][a-z0-9+.-]*://([^/@]+@)?(localhost|127\.|0\.0\.0\.0|\[?::1\]?|::1|llm|openai|anthropic|ollama|model-server|model_server)([:/?#]|$)'
       AND v NOT LIKE '%//localhost/%'
       AND v NOT LIKE 'localhost:%'
       AND v NOT LIKE 'localhost/%'
       AND v NOT LIKE '127.%'
       AND v NOT LIKE '0.0.0.0%'
       AND v NOT LIKE '[::1]%'
       AND v NOT LIKE '::1%'
    FROM probes;
$$;

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_sheet_version_character_version
    ON atelier_sheet_version(character_internal_id, version_id);

CREATE TABLE IF NOT EXISTS atelier_sheet_artifact_link (
    link_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    sheet_version_id UUID NOT NULL REFERENCES atelier_sheet_version(version_id) ON DELETE CASCADE,
    artifact_kind TEXT NOT NULL CHECK (
        artifact_kind IN (
            'openpose_json',
            'openpose_png',
            'conditioning_png',
            'comfy_render',
            'comfy_receipt'
        )
    ),
    artifact_ref TEXT NOT NULL,
    manifest_ref TEXT,
    source_ref TEXT,
    label TEXT,
    reuse_role TEXT,
    linked_by TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    detached_at_utc TIMESTAMPTZ,
    detached_by TEXT,
    CONSTRAINT chk_atelier_sheet_artifact_link_artifact_ref CHECK (
        atelier_is_native_portable_ref(artifact_ref)
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_manifest_ref CHECK (
        manifest_ref IS NULL
        OR atelier_is_native_portable_ref(manifest_ref)
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_source_ref CHECK (
        source_ref IS NULL
        OR atelier_is_native_portable_ref(source_ref)
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_label CHECK (
        label IS NULL OR (btrim(label) = label AND label <> '')
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_reuse_role CHECK (
        reuse_role IS NULL
        OR (
            btrim(reuse_role) = reuse_role
            AND reuse_role <> ''
            AND reuse_role ~ '^[a-z0-9][a-z0-9._-]*$'
        )
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_linked_by CHECK (
        btrim(linked_by) = linked_by
        AND linked_by <> ''
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_metadata CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT chk_atelier_sheet_artifact_link_detach_actor CHECK (
        (
            detached_at_utc IS NULL
            AND detached_by IS NULL
        )
        OR (
            detached_at_utc IS NOT NULL
            AND detached_by IS NOT NULL
            AND btrim(detached_by) = detached_by
            AND detached_by <> ''
        )
    )
);

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_artifact_ref;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_artifact_ref CHECK (
        atelier_is_native_portable_ref(artifact_ref)
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_manifest_ref;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_manifest_ref CHECK (
        manifest_ref IS NULL
        OR atelier_is_native_portable_ref(manifest_ref)
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_source_ref;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_source_ref CHECK (
        source_ref IS NULL
        OR atelier_is_native_portable_ref(source_ref)
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_label;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_label CHECK (
        label IS NULL OR (btrim(label) = label AND label <> '')
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_reuse_role;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_reuse_role CHECK (
        reuse_role IS NULL
        OR (
            btrim(reuse_role) = reuse_role
            AND reuse_role <> ''
            AND reuse_role ~ '^[a-z0-9][a-z0-9._-]*$'
        )
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_linked_by;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_linked_by CHECK (
        btrim(linked_by) = linked_by
        AND linked_by <> ''
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_metadata;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_metadata CHECK (
        jsonb_typeof(metadata) = 'object'
    );

ALTER TABLE atelier_sheet_artifact_link
    DROP CONSTRAINT IF EXISTS chk_atelier_sheet_artifact_link_detach_actor;
ALTER TABLE atelier_sheet_artifact_link
    ADD CONSTRAINT chk_atelier_sheet_artifact_link_detach_actor CHECK (
        (
            detached_at_utc IS NULL
            AND detached_by IS NULL
        )
        OR (
            detached_at_utc IS NOT NULL
            AND detached_by IS NOT NULL
            AND btrim(detached_by) = detached_by
            AND detached_by <> ''
        )
    );

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_atelier_sheet_artifact_link_sheet_owner'
    ) THEN
        ALTER TABLE atelier_sheet_artifact_link
            ADD CONSTRAINT fk_atelier_sheet_artifact_link_sheet_owner
            FOREIGN KEY (character_internal_id, sheet_version_id)
            REFERENCES atelier_sheet_version(character_internal_id, version_id)
            ON DELETE CASCADE;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_sheet_artifact_link_active_ref
    ON atelier_sheet_artifact_link(sheet_version_id, artifact_kind, artifact_ref)
    WHERE detached_at_utc IS NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_sheet_artifact_link_sheet_active
    ON atelier_sheet_artifact_link(sheet_version_id, created_at_utc, link_id)
    WHERE detached_at_utc IS NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_sheet_artifact_link_character_kind
    ON atelier_sheet_artifact_link(character_internal_id, artifact_kind, created_at_utc);
