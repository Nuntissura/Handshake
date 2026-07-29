-- WP-KERNEL-012 MT-028 remediation: repair saved-view projections left by
-- the pre-atomic save-as-view path.
--
-- Scope is deliberately anchored to canonical Loom authority:
-- loom_blocks.content_type = 'view_def'. A stranded note is indistinguishable
-- from an ordinary note and is therefore never promoted by this migration.
--
-- The repair is replay-safe:
--   * search and knowledge rows update only when their canonical projection
--     differs;
--   * EventLedger ids/idempotency keys are deterministic per workspace/block;
--   * the bridge timestamp changes only when its authority tuple changes.
--
-- PostgreSQL migrations run transactionally, so the projection repairs,
-- typed EventLedger receipt, and bridge receipt pointer commit together.

CREATE TEMP TABLE mt028_view_projection_repair
ON COMMIT DROP
AS
SELECT
    block.block_id,
    block.workspace_id,
    COALESCE(
        entity.entity_id,
        'KEN-' || md5(block.workspace_id || chr(31) || block.block_id)
    ) AS entity_id,
    COALESCE(
        NULLIF(btrim(block.title), ''),
        NULLIF(btrim(block.original_filename), ''),
        'view_def ' || block.block_id
    ) AS display_name,
    concat_ws(
        E'\n',
        block.title,
        block.original_filename,
        block.derived_json::jsonb ->> 'full_text_index'
    ) AS search_text,
    'KE-MT028-0363-' ||
        md5(block.workspace_id || chr(31) || block.block_id) AS repair_event_id,
    'KEI-MT028-0363-' ||
        md5(block.workspace_id || chr(31) || block.block_id) AS repair_idempotency_key
FROM loom_blocks AS block
LEFT JOIN knowledge_entities AS entity
    ON entity.workspace_id = block.workspace_id
   AND entity.entity_kind = 'loom_block'
   AND entity.entity_key = block.block_id
WHERE block.content_type = 'view_def';

INSERT INTO loom_block_search_index (
    block_id,
    workspace_id,
    content_type,
    search_text,
    indexed_at
)
SELECT
    block_id,
    workspace_id,
    'view_def',
    search_text,
    NOW()
FROM mt028_view_projection_repair
ON CONFLICT (block_id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
    content_type = EXCLUDED.content_type,
    search_text = EXCLUDED.search_text,
    indexed_at = NOW()
WHERE (
    loom_block_search_index.workspace_id,
    loom_block_search_index.content_type,
    loom_block_search_index.search_text
) IS DISTINCT FROM (
    EXCLUDED.workspace_id,
    EXCLUDED.content_type,
    EXCLUDED.search_text
);

INSERT INTO knowledge_entities (
    entity_id,
    workspace_id,
    entity_kind,
    entity_key,
    display_name,
    detection_provenance,
    primary_source_id,
    first_detected_in_run,
    last_detected_in_run
)
SELECT
    entity_id,
    workspace_id,
    'loom_block',
    block_id,
    display_name,
    jsonb_build_object(
        'extractor', 'loom_block_knowledge_bridge',
        'extractor_version', 'loom_block_knowledge_bridge_v1',
        'method', 'mt177_bridge',
        'content_type', 'view_def'
    ),
    NULL,
    NULL,
    NULL
FROM mt028_view_projection_repair
ON CONFLICT (workspace_id, entity_kind, entity_key) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    detection_provenance = EXCLUDED.detection_provenance,
    lifecycle_state = 'active',
    updated_at = NOW()
WHERE (
    knowledge_entities.display_name,
    knowledge_entities.detection_provenance,
    knowledge_entities.lifecycle_state
) IS DISTINCT FROM (
    EXCLUDED.display_name,
    EXCLUDED.detection_provenance,
    'active'
);

WITH repair_events AS (
    SELECT
        repair.*,
        jsonb_build_object(
            'type', 'knowledge_loom_block_indexed',
            'workspace_id', repair.workspace_id,
            'block_id', repair.block_id,
            'entity_id', repair.entity_id,
            'content_type', 'view_def',
            'extractor_version', 'loom_block_knowledge_bridge_v1',
            'migration', '0363_loom_block_view_legacy_projection_repair',
            'repair_reason', 'legacy_view_projection_repair'
        ) AS payload,
        '{"block_id":' || to_jsonb(repair.block_id)::text ||
        ',"content_type":"view_def"' ||
        ',"entity_id":' || to_jsonb(repair.entity_id)::text ||
        ',"extractor_version":"loom_block_knowledge_bridge_v1"' ||
        ',"migration":"0363_loom_block_view_legacy_projection_repair"' ||
        ',"repair_reason":"legacy_view_projection_repair"' ||
        ',"type":"knowledge_loom_block_indexed"' ||
        ',"workspace_id":' || to_jsonb(repair.workspace_id)::text ||
        '}' AS canonical_payload
    FROM mt028_view_projection_repair AS repair
)
INSERT INTO kernel_event_ledger (
    event_id,
    event_version,
    kernel_task_run_id,
    session_run_id,
    aggregate_type,
    aggregate_id,
    idempotency_key,
    event_type,
    actor_kind,
    actor_id,
    causation_id,
    correlation_id,
    payload_hash,
    source_component,
    payload
)
SELECT
    repair_event_id,
    'kernel_event_v1',
    'KTR-MT028-0363',
    'SR-MT028-0363',
    'knowledge_loom_block',
    entity_id,
    repair_idempotency_key,
    'KNOWLEDGE_LOOM_BLOCK_INDEXED',
    'system',
    'migration-0363',
    NULL,
    'WP-KERNEL-012/MT-028',
    encode(digest(canonical_payload, 'sha256'), 'hex'),
    'loom_block_view_legacy_projection_repair',
    payload
FROM repair_events
ON CONFLICT (event_id) DO NOTHING;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM mt028_view_projection_repair AS repair
        JOIN kernel_event_ledger AS event
          ON event.event_id = repair.repair_event_id
        WHERE event.event_version <> 'kernel_event_v1'
           OR event.aggregate_type <> 'knowledge_loom_block'
           OR event.aggregate_id <> repair.entity_id
           OR event.idempotency_key <> repair.repair_idempotency_key
           OR event.event_type <> 'KNOWLEDGE_LOOM_BLOCK_INDEXED'
           OR event.actor_kind <> 'system'
           OR event.actor_id <> 'migration-0363'
           OR event.correlation_id <> 'WP-KERNEL-012/MT-028'
           OR event.source_component <> 'loom_block_view_legacy_projection_repair'
           OR event.payload IS DISTINCT FROM jsonb_build_object(
                'type', 'knowledge_loom_block_indexed',
                'workspace_id', repair.workspace_id,
                'block_id', repair.block_id,
                'entity_id', repair.entity_id,
                'content_type', 'view_def',
                'extractor_version', 'loom_block_knowledge_bridge_v1',
                'migration', '0363_loom_block_view_legacy_projection_repair',
                'repair_reason', 'legacy_view_projection_repair'
           )
           OR event.payload_hash IS DISTINCT FROM encode(
                digest(
                    '{"block_id":' || to_jsonb(repair.block_id)::text ||
                    ',"content_type":"view_def"' ||
                    ',"entity_id":' || to_jsonb(repair.entity_id)::text ||
                    ',"extractor_version":"loom_block_knowledge_bridge_v1"' ||
                    ',"migration":"0363_loom_block_view_legacy_projection_repair"' ||
                    ',"repair_reason":"legacy_view_projection_repair"' ||
                    ',"type":"knowledge_loom_block_indexed"' ||
                    ',"workspace_id":' || to_jsonb(repair.workspace_id)::text ||
                    '}',
                    'sha256'
                ),
                'hex'
           )
    ) THEN
        RAISE EXCEPTION
            '0363 repair EventLedger identity conflicts with an existing event';
    END IF;
END
$$;

INSERT INTO loom_block_knowledge_bridge (
    block_id,
    workspace_id,
    entity_id,
    index_event_id
)
SELECT
    block_id,
    workspace_id,
    entity_id,
    repair_event_id
FROM mt028_view_projection_repair
ON CONFLICT (block_id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
    entity_id = EXCLUDED.entity_id,
    index_event_id = EXCLUDED.index_event_id,
    updated_at = NOW()
WHERE (
    loom_block_knowledge_bridge.workspace_id,
    loom_block_knowledge_bridge.entity_id,
    loom_block_knowledge_bridge.index_event_id
) IS DISTINCT FROM (
    EXCLUDED.workspace_id,
    EXCLUDED.entity_id,
    EXCLUDED.index_event_id
);
