-- WP-KERNEL-012 MT-032: backfill stable same-id Loom projections for
-- RichDocuments created before the projection invariant existed.
--
-- The state table is exact migration provenance. It records both prior and
-- applied values so the down migration never broadly deletes or overwrites a
-- projection that an operator changed after upgrade. `applied` makes an
-- explicit second forward execution a no-op.

CREATE TABLE IF NOT EXISTS knowledge_rich_document_loom_projection_0343_state (
    block_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    block_existed BOOLEAN NOT NULL,
    prior_title TEXT,
    prior_content_hash TEXT,
    prior_derived_json TEXT,
    prior_block_updated_at TIMESTAMPTZ,
    prior_search_existed BOOLEAN NOT NULL,
    prior_search_workspace_id TEXT,
    prior_search_content_type TEXT,
    prior_search_text TEXT,
    prior_indexed_at TIMESTAMPTZ,
    applied_title TEXT NOT NULL,
    applied_content_hash TEXT NOT NULL,
    applied_derived_json TEXT NOT NULL,
    applied_search_text TEXT NOT NULL,
    applied_block_updated_at TIMESTAMPTZ,
    applied_indexed_at TIMESTAMPTZ,
    applied BOOLEAN NOT NULL DEFAULT FALSE
);

-- A same id already owned by another workspace/type is not a projection that
-- this migration may adopt. Fail loudly instead of ON CONFLICT-skipping into
-- a cross-workspace or incompatible identity split.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM knowledge_rich_documents d
        JOIN loom_blocks b ON b.block_id = d.rich_document_id
        WHERE d.deleted_at IS NULL
          AND (b.workspace_id <> d.workspace_id OR b.content_type <> 'note')
    ) THEN
        RAISE EXCEPTION '0343 incompatible RichDocument/LoomBlock identity collision';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM knowledge_rich_documents d
        JOIN loom_block_search_index s ON s.block_id = d.rich_document_id
        WHERE d.deleted_at IS NULL
          AND (s.workspace_id <> d.workspace_id OR s.content_type <> 'note')
    ) THEN
        RAISE EXCEPTION '0343 incompatible RichDocument/search identity collision';
    END IF;
END
$$;

-- Migration-local implementation of the Rust search projection: text nodes,
-- Monaco code, typed link labels plus stable hsLink refValue identities, and
-- recursive child separators. The helper is dropped before commit.
CREATE OR REPLACE FUNCTION hsk_0343_rich_document_plain_text(node JSONB)
RETURNS TEXT
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    node_type TEXT;
    separator TEXT;
    child JSONB;
    child_text TEXT;
    ref_kind TEXT;
    ref_value TEXT;
    normalized_locus TEXT;
    result TEXT := '';
BEGIN
    IF node IS NULL OR jsonb_typeof(node) <> 'object' THEN
        RETURN '';
    END IF;
    node_type := COALESCE(node->>'type', '');
    IF node_type = 'text' THEN
        RETURN COALESCE(node->>'text', '');
    ELSIF node_type = 'monacoCodeBlock' THEN
        RETURN COALESCE(node->'attrs'->>'code', '');
    ELSIF node_type = 'hsLink' THEN
        ref_kind := node->'attrs'->>'refKind';
        ref_value := node->'attrs'->>'refValue';
        normalized_locus := CASE
            WHEN ref_kind = 'locus' AND lower(ref_value) LIKE 'locus://wp/%'
                THEN 'locus://wp/' || lower(regexp_replace(btrim(substring(ref_value FROM 12)), '\s+', ' ', 'g'))
            WHEN ref_kind = 'locus' AND lower(ref_value) LIKE 'locus://mt/%'
                THEN 'locus://mt/' || lower(regexp_replace(btrim(substring(ref_value FROM 12)), '\s+', ' ', 'g'))
            WHEN ref_kind = 'locus' AND lower(ref_value) LIKE 'wp/%'
                THEN 'locus://wp/' || lower(regexp_replace(btrim(substring(ref_value FROM 4)), '\s+', ' ', 'g'))
            WHEN ref_kind = 'locus' AND lower(ref_value) LIKE 'mt/%'
                THEN 'locus://mt/' || lower(regexp_replace(btrim(substring(ref_value FROM 4)), '\s+', ' ', 'g'))
            WHEN ref_kind = 'locus' AND upper(ref_value) LIKE 'WP-%'
                THEN 'locus://wp/' || lower(regexp_replace(btrim(ref_value), '\s+', ' ', 'g'))
            WHEN ref_kind = 'locus' AND upper(ref_value) LIKE 'MT-%'
                THEN 'locus://mt/' || lower(regexp_replace(btrim(ref_value), '\s+', ' ', 'g'))
            ELSE NULL
        END;
        RETURN concat_ws(
            ' ',
            NULLIF(node->'attrs'->>'label', ''),
            NULLIF(ref_value, ''),
            normalized_locus
        );
    ELSIF node_type IN ('mention', 'tagMention') THEN
        RETURN COALESCE(
            node->'attrs'->>'label',
            node->'attrs'->>'id',
            ''
        );
    END IF;
    IF jsonb_typeof(node->'content') IS DISTINCT FROM 'array' THEN
        RETURN '';
    END IF;
    separator := CASE
        WHEN node_type IN (
            'listItem', 'bulletList', 'orderedList', 'taskList', 'taskItem',
            'tableRow', 'table'
        ) THEN E'\n'
        ELSE ' '
    END;
    FOR child IN SELECT value FROM jsonb_array_elements(node->'content') LOOP
        child_text := hsk_0343_rich_document_plain_text(child);
        IF child_text <> '' THEN
            IF result <> '' THEN
                result := result || separator;
            END IF;
            result := result || child_text;
        END IF;
    END LOOP;
    RETURN result;
END
$$;

INSERT INTO knowledge_rich_document_loom_projection_0343_state (
    block_id, workspace_id, block_existed,
    prior_title, prior_content_hash, prior_derived_json, prior_block_updated_at,
    prior_search_existed, prior_search_workspace_id,
    prior_search_content_type, prior_search_text, prior_indexed_at,
    applied_title, applied_content_hash, applied_derived_json,
    applied_search_text
)
SELECT
    d.rich_document_id,
    d.workspace_id,
    b.block_id IS NOT NULL,
    b.title,
    b.content_hash,
    b.derived_json,
    b.updated_at,
    s.block_id IS NOT NULL,
    s.workspace_id,
    s.content_type,
    s.search_text,
    s.indexed_at,
    d.title,
    d.content_sha256,
    (jsonb_build_object(
        'backlink_count', 0,
        'mention_count', 0,
        'tag_count', 0,
        'preview_status', 'none'
    ) || CASE
        WHEN btrim(hsk_0343_rich_document_plain_text(d.content_json)) = ''
            THEN '{}'::jsonb
        ELSE jsonb_build_object(
            'full_text_index',
            btrim(hsk_0343_rich_document_plain_text(d.content_json))
        )
    END)::text,
    CASE
        WHEN btrim(hsk_0343_rich_document_plain_text(d.content_json)) = ''
            THEN d.title
        ELSE d.title || E'\n' || btrim(hsk_0343_rich_document_plain_text(d.content_json))
    END
FROM knowledge_rich_documents d
LEFT JOIN loom_blocks b ON b.block_id = d.rich_document_id
LEFT JOIN loom_block_search_index s ON s.block_id = d.rich_document_id
WHERE d.deleted_at IS NULL
ON CONFLICT (block_id) DO NOTHING;

INSERT INTO loom_blocks (
    block_id, workspace_id, content_type, document_id, asset_id,
    title, original_filename, content_hash, pinned, journal_date,
    last_actor_kind, last_actor_id, last_job_id, last_workflow_id,
    edit_event_id, created_at, updated_at, imported_at,
    backlink_count, mention_count, tag_count, derived_json,
    preview_status, thumbnail_asset_id, proxy_asset_id
)
SELECT
    p.block_id, p.workspace_id, 'note', NULL, NULL,
    p.applied_title, NULL, p.applied_content_hash, 0, NULL,
    'SYSTEM', 'knowledge_rich_document_backfill_0343', NULL, NULL,
    '00000000-0000-0000-0000-000000000000', NOW(), NOW(), NULL,
    0, 0, 0, p.applied_derived_json,
    'none', NULL, NULL
FROM knowledge_rich_document_loom_projection_0343_state p
WHERE NOT p.applied AND NOT p.block_existed;

UPDATE loom_blocks b
SET title = p.applied_title,
    content_hash = p.applied_content_hash,
    derived_json = p.applied_derived_json,
    updated_at = NOW()
FROM knowledge_rich_document_loom_projection_0343_state p
WHERE NOT p.applied
  AND b.block_id = p.block_id
  AND b.workspace_id = p.workspace_id
  AND b.content_type = 'note';

INSERT INTO loom_block_search_index (
    block_id, workspace_id, content_type, search_text, indexed_at
)
SELECT block_id, workspace_id, 'note', applied_search_text, NOW()
FROM knowledge_rich_document_loom_projection_0343_state
WHERE NOT applied
ON CONFLICT (block_id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
    content_type = EXCLUDED.content_type,
    search_text = EXCLUDED.search_text,
    indexed_at = NOW();

UPDATE knowledge_rich_document_loom_projection_0343_state p
SET applied_block_updated_at = b.updated_at,
    applied_indexed_at = s.indexed_at
FROM loom_blocks b
JOIN loom_block_search_index s ON s.block_id = b.block_id
WHERE NOT p.applied
  AND b.block_id = p.block_id
  AND b.workspace_id = p.workspace_id;

UPDATE knowledge_rich_document_loom_projection_0343_state
SET applied = TRUE
WHERE NOT applied;

DROP FUNCTION hsk_0343_rich_document_plain_text(JSONB);
