-- WP-KERNEL-005 MT-048 web portfolio export contract.
-- Web portfolio exports are separate from character-sheet/share-pack exports:
-- they are collection-backed, ArtifactStore-backed, and expose a portable
-- request -> result -> manifest_json contract.

CREATE TABLE IF NOT EXISTS atelier_web_portfolio_export_request (
    portfolio_export_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_collection_id UUID NOT NULL REFERENCES atelier_collection(collection_id) ON DELETE CASCADE,
    slug                 TEXT NOT NULL UNIQUE,
    title                TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'pending',
    requested_by         TEXT NOT NULL,
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_web_portfolio_request_slug
        CHECK (
            btrim(slug) = slug
            AND slug <> ''
            AND slug !~ '[[:space:]]'
            AND slug ~ '^[a-z0-9][a-z0-9_-]*$'
        ),
    CONSTRAINT chk_atelier_web_portfolio_request_title_trimmed
        CHECK (btrim(title) = title AND title <> ''),
    CONSTRAINT chk_atelier_web_portfolio_request_requested_by_trimmed
        CHECK (btrim(requested_by) = requested_by AND requested_by <> ''),
    CONSTRAINT chk_atelier_web_portfolio_request_status
        CHECK (status IN ('pending', 'rendered'))
);

CREATE INDEX IF NOT EXISTS idx_atelier_web_portfolio_export_request_collection
    ON atelier_web_portfolio_export_request(source_collection_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_web_portfolio_export_result (
    result_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    portfolio_export_id UUID NOT NULL REFERENCES atelier_web_portfolio_export_request(portfolio_export_id) ON DELETE CASCADE,
    artifact_ref        TEXT NOT NULL,
    content_hash        TEXT NOT NULL,
    byte_len            BIGINT NOT NULL,
    manifest_json       JSONB NOT NULL,
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (portfolio_export_id, content_hash),
    CONSTRAINT chk_atelier_web_portfolio_result_artifact_ref
        CHECK (
            btrim(artifact_ref) = artifact_ref
            AND artifact_ref <> ''
            AND artifact_ref !~ '[[:space:]]'
            AND artifact_ref ~ '^artifact://'
            AND artifact_ref !~* '(^[a-z]:|\.gov|file:|localhost|sqlite|\\|/\.\./)'
        ),
    CONSTRAINT chk_atelier_web_portfolio_result_content_hash
        CHECK (
            btrim(content_hash) = content_hash
            AND content_hash <> ''
            AND content_hash !~ '[[:space:]]'
        ),
    CONSTRAINT chk_atelier_web_portfolio_result_byte_len
        CHECK (byte_len > 0),
    CONSTRAINT chk_atelier_web_portfolio_result_manifest_contract
        CHECK (
            jsonb_typeof(manifest_json) = 'object'
            AND manifest_json->>'schema_id' = 'hsk.atelier.web_portfolio_export_manifest@1'
            AND jsonb_typeof(manifest_json->'output') = 'object'
            AND jsonb_typeof(manifest_json->'items') = 'array'
            AND manifest_json::text !~* '\.gov'
        )
);

CREATE INDEX IF NOT EXISTS idx_atelier_web_portfolio_export_result_request
    ON atelier_web_portfolio_export_result(portfolio_export_id, created_at_utc DESC);
