-- WP-KERNEL-005 MT-016 ArtifactStore media materialization manifest.
-- Media assets carry a durable manifest binding the ArtifactStore handle to
-- hash, size, source provenance, and retention class.

ALTER TABLE atelier_media_asset
    ADD COLUMN IF NOT EXISTS retention_class TEXT NOT NULL DEFAULT 'atelier.media.original.retained';

ALTER TABLE atelier_media_asset
    ADD COLUMN IF NOT EXISTS artifact_manifest JSONB NOT NULL DEFAULT '{}'::jsonb;

UPDATE atelier_media_asset
SET retention_class = 'atelier.media.original.retained'
WHERE retention_class IS NULL
   OR btrim(retention_class) = '';

UPDATE atelier_media_asset
SET artifact_manifest = jsonb_build_object(
        'schema', 'hsk.atelier.media_artifact_manifest@1',
        'asset_id', asset_id,
        'artifact_ref', artifact_ref,
        'content_hash', content_hash,
        'mime', mime,
        'byte_len', byte_len,
        'size_bytes', byte_len,
        'source_provenance_ref', 'sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'),
        'retention_class', retention_class,
        'artifact_store', jsonb_build_object(
            'handle', artifact_ref,
            'content_hash', content_hash,
            'size_bytes', byte_len,
            'retention_class', retention_class
        )
    )
WHERE (artifact_manifest = '{}'::jsonb
   OR artifact_manifest->>'schema' IS DISTINCT FROM 'hsk.atelier.media_artifact_manifest@1'
   OR artifact_manifest->>'asset_id' IS DISTINCT FROM asset_id::text
   OR artifact_manifest->>'artifact_ref' IS DISTINCT FROM artifact_ref
   OR artifact_manifest->>'content_hash' IS DISTINCT FROM content_hash
   OR artifact_manifest->>'mime' IS DISTINCT FROM mime
   OR artifact_manifest->>'byte_len' IS DISTINCT FROM byte_len::text
   OR artifact_manifest->>'size_bytes' IS DISTINCT FROM byte_len::text
   OR artifact_manifest ? 'source_provenance'
   OR artifact_manifest ? 'source'
   OR artifact_manifest->>'source_provenance_ref' IS DISTINCT FROM ('sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'))
   OR artifact_manifest->>'retention_class' IS DISTINCT FROM retention_class
   OR artifact_manifest#>>'{artifact_store,handle}' IS DISTINCT FROM artifact_ref
   OR artifact_manifest#>>'{artifact_store,content_hash}' IS DISTINCT FROM content_hash
   OR artifact_manifest#>>'{artifact_store,size_bytes}' IS DISTINCT FROM byte_len::text
   OR artifact_manifest#>>'{artifact_store,retention_class}' IS DISTINCT FROM retention_class)
  AND artifact_ref ~ '^artifact://\.handshake/artifacts/L[1-4]/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/payload$'
  AND lower(artifact_ref) NOT LIKE '%.gov%'
  AND content_hash ~* '^(sha256:)?[0-9a-f]{64}$'
  AND byte_len > 0
  AND btrim(mime) = mime
  AND btrim(mime) <> ''
  AND btrim(retention_class) = retention_class
  AND btrim(retention_class) <> '';

UPDATE atelier_media_asset
SET artifact_manifest = jsonb_build_object(
        'schema', 'hsk.atelier.media_artifact_manifest@1',
        'asset_id', asset_id,
        'content_hash', content_hash,
        'mime', mime,
        'byte_len', byte_len,
        'size_bytes', byte_len,
        'source_provenance_ref', 'sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'),
        'retention_class', retention_class,
        'validation_state', 'invalid_legacy_artifact_ref',
        'artifact_store', jsonb_build_object(
            'status', 'unresolved',
            'reason', 'legacy artifact_ref is not a native ArtifactStore payload handle'
        )
    )
WHERE NOT (
        artifact_ref ~ '^artifact://\.handshake/artifacts/L[1-4]/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/payload$'
        AND lower(artifact_ref) NOT LIKE '%.gov%'
        AND content_hash ~* '^(sha256:)?[0-9a-f]{64}$'
        AND byte_len > 0
        AND btrim(mime) = mime
        AND btrim(mime) <> ''
        AND btrim(retention_class) = retention_class
        AND btrim(retention_class) <> ''
    )
  AND (
        artifact_manifest = '{}'::jsonb
     OR artifact_manifest->>'schema' IS DISTINCT FROM 'hsk.atelier.media_artifact_manifest@1'
     OR artifact_manifest->>'asset_id' IS DISTINCT FROM asset_id::text
     OR artifact_manifest->>'validation_state' IS DISTINCT FROM 'invalid_legacy_artifact_ref'
     OR artifact_manifest ? 'artifact_ref'
     OR artifact_manifest->>'content_hash' IS DISTINCT FROM content_hash
     OR artifact_manifest->>'mime' IS DISTINCT FROM mime
     OR artifact_manifest->>'byte_len' IS DISTINCT FROM byte_len::text
     OR artifact_manifest->>'size_bytes' IS DISTINCT FROM byte_len::text
     OR artifact_manifest ? 'source_provenance'
     OR artifact_manifest ? 'source'
     OR artifact_manifest->>'source_provenance_ref' IS DISTINCT FROM ('sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'))
     OR artifact_manifest->>'retention_class' IS DISTINCT FROM retention_class
     OR artifact_manifest#>>'{artifact_store,handle}' IS NOT NULL
     OR artifact_manifest#>>'{artifact_store,status}' IS DISTINCT FROM 'unresolved'
     OR artifact_manifest#>>'{artifact_store,reason}' IS DISTINCT FROM 'legacy artifact_ref is not a native ArtifactStore payload handle'
  );

CREATE INDEX IF NOT EXISTS idx_atelier_media_asset_retention
    ON atelier_media_asset(retention_class);
