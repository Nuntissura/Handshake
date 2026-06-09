-- WP-KERNEL-005 MT-016 follow-up: SQL migrations cannot prove ArtifactStore
-- payload reachability. Native-shaped legacy rows are quarantined as
-- unverified until runtime repair validates the artifact manifest and payload.

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
        'validation_state', 'invalid_artifact_store_binding',
        'artifact_store', jsonb_build_object(
            'status', 'unresolved',
            'reason', 'artifact_ref could not be validated against ArtifactStore'
        )
    )
WHERE artifact_ref ~ '^artifact://\.handshake/artifacts/L[1-4]/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/payload$'
  AND lower(artifact_ref) NOT LIKE '%.gov%'
  AND content_hash ~* '^(sha256:)?[0-9a-f]{64}$'
  AND byte_len > 0
  AND btrim(mime) = mime
  AND btrim(mime) <> ''
  AND btrim(retention_class) = retention_class
  AND btrim(retention_class) <> ''
  AND (
        artifact_manifest = '{}'::jsonb
     OR artifact_manifest->>'schema' IS DISTINCT FROM 'hsk.atelier.media_artifact_manifest@1'
     OR artifact_manifest->>'asset_id' IS DISTINCT FROM asset_id::text
     OR artifact_manifest ? 'artifact_ref'
     OR artifact_manifest->>'content_hash' IS DISTINCT FROM content_hash
     OR artifact_manifest->>'mime' IS DISTINCT FROM mime
     OR artifact_manifest->>'byte_len' IS DISTINCT FROM byte_len::text
     OR artifact_manifest->>'size_bytes' IS DISTINCT FROM byte_len::text
     OR artifact_manifest ? 'source_provenance'
     OR artifact_manifest ? 'source'
     OR artifact_manifest->>'source_provenance_ref' IS DISTINCT FROM ('sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'))
     OR artifact_manifest->>'retention_class' IS DISTINCT FROM retention_class
     OR artifact_manifest->>'validation_state' IS DISTINCT FROM 'invalid_artifact_store_binding'
     OR artifact_manifest#>>'{artifact_store,handle}' IS NOT NULL
     OR artifact_manifest#>>'{artifact_store,status}' IS DISTINCT FROM 'unresolved'
     OR artifact_manifest#>>'{artifact_store,reason}' IS DISTINCT FROM 'artifact_ref could not be validated against ArtifactStore'
  );
