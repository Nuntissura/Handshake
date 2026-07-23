-- WP-KERNEL-012 MT-032/043: project durable RichDocument wikilinks into the
-- same-id Loom graph. knowledge_document_backlinks remains the rebuildable
-- link projection; loom_edges is the graph projection consumed by Loom/Graph.
-- The source must be a live RichDocument projection. The target is deliberately
-- any live LoomBlock in the same workspace (including file/CKC blocks); requiring
-- a target knowledge_rich_documents row would lose cross-surface links on upgrade.

-- Migration 0347 may run while the application is accepting writes. Hold one
-- transaction-scoped writer-excluding lock set across collision preflight,
-- projection, and verification so an independently-authored KDLNK edge or a
-- backlink mutation cannot slip between those phases.
LOCK TABLE knowledge_document_backlinks, knowledge_rich_documents, loom_blocks, loom_edges
    IN SHARE ROW EXCLUSIVE MODE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM knowledge_document_backlinks backlink
        JOIN knowledge_rich_documents source_document
          ON source_document.rich_document_id = backlink.source_document_id
         AND source_document.workspace_id = backlink.workspace_id
         AND source_document.deleted_at IS NULL
        JOIN loom_blocks source_block
          ON source_block.block_id = backlink.source_document_id
         AND source_block.workspace_id = backlink.workspace_id
        JOIN loom_blocks target_block
          ON target_block.block_id = backlink.target
         AND target_block.workspace_id = backlink.workspace_id
        JOIN loom_edges existing ON existing.edge_id = backlink.relationship_id
        WHERE backlink.link_kind = 'wikilink'
          AND backlink.relationship_id LIKE 'KDLNK-%'
          AND NOT (
              existing.workspace_id = backlink.workspace_id
              AND existing.source_block_id = backlink.source_document_id
              AND existing.last_actor_kind = 'SYSTEM'
              AND existing.last_actor_id = 'knowledge_rich_document_backlink_projection'
              AND existing.edit_event_id = '00000000-0000-0000-0000-000000000000'
              AND existing.source_document_id = existing.source_block_id
          )
    ) THEN
        RAISE EXCEPTION 'KDLNK Loom edge identity collision with independently-authored edge';
    END IF;
END
$$;

INSERT INTO loom_edges AS existing (
    edge_id, workspace_id, source_block_id, target_block_id,
    edge_type, created_by, last_actor_kind, last_actor_id,
    edit_event_id, source_document_id, source_text_block_id
)
SELECT
    backlink.relationship_id,
    backlink.workspace_id,
    backlink.source_document_id,
    backlink.target,
    'mention',
    'user',
    'SYSTEM',
    'knowledge_rich_document_backlink_projection',
    '00000000-0000-0000-0000-000000000000',
    backlink.source_document_id,
    backlink.block_id
FROM knowledge_document_backlinks backlink
JOIN knowledge_rich_documents source_document
  ON source_document.rich_document_id = backlink.source_document_id
 AND source_document.workspace_id = backlink.workspace_id
 AND source_document.deleted_at IS NULL
JOIN loom_blocks source_block
  ON source_block.block_id = backlink.source_document_id
 AND source_block.workspace_id = backlink.workspace_id
JOIN loom_blocks target_block
  ON target_block.block_id = backlink.target
 AND target_block.workspace_id = backlink.workspace_id
WHERE backlink.link_kind = 'wikilink'
  AND backlink.relationship_id LIKE 'KDLNK-%'
ON CONFLICT (edge_id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
    source_block_id = EXCLUDED.source_block_id,
    target_block_id = EXCLUDED.target_block_id,
    edge_type = EXCLUDED.edge_type,
    created_by = EXCLUDED.created_by,
    last_actor_kind = EXCLUDED.last_actor_kind,
    last_actor_id = EXCLUDED.last_actor_id,
    edit_event_id = EXCLUDED.edit_event_id,
    source_document_id = EXCLUDED.source_document_id,
    source_text_block_id = EXCLUDED.source_text_block_id,
    offset_start = NULL,
    offset_end = NULL
WHERE existing.workspace_id = EXCLUDED.workspace_id
  AND existing.source_block_id = EXCLUDED.source_block_id
  AND existing.edge_id LIKE 'KDLNK-%'
  AND existing.last_actor_kind = 'SYSTEM'
  AND existing.last_actor_id = 'knowledge_rich_document_backlink_projection'
  AND existing.edit_event_id = '00000000-0000-0000-0000-000000000000'
  AND existing.source_document_id = existing.source_block_id;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM knowledge_document_backlinks backlink
        JOIN knowledge_rich_documents source_document
          ON source_document.rich_document_id = backlink.source_document_id
         AND source_document.workspace_id = backlink.workspace_id
         AND source_document.deleted_at IS NULL
        JOIN loom_blocks source_block
          ON source_block.block_id = backlink.source_document_id
         AND source_block.workspace_id = backlink.workspace_id
        JOIN loom_blocks target_block
          ON target_block.block_id = backlink.target
         AND target_block.workspace_id = backlink.workspace_id
        LEFT JOIN loom_edges projected ON projected.edge_id = backlink.relationship_id
        WHERE backlink.link_kind = 'wikilink'
          AND backlink.relationship_id LIKE 'KDLNK-%'
          AND (
              projected.edge_id IS NULL
              OR projected.workspace_id <> backlink.workspace_id
              OR projected.source_block_id <> backlink.source_document_id
              OR projected.target_block_id <> backlink.target
              OR projected.edge_type <> 'mention'
              OR projected.last_actor_kind <> 'SYSTEM'
              OR projected.last_actor_id <> 'knowledge_rich_document_backlink_projection'
              OR projected.edit_event_id <> '00000000-0000-0000-0000-000000000000'
              OR projected.source_document_id <> projected.source_block_id
          )
    ) THEN
        RAISE EXCEPTION 'KDLNK Loom edge projection verification failed';
    END IF;
END
$$;

UPDATE loom_blocks block
SET
    mention_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.source_block_id = block.block_id AND edge.edge_type = 'mention'),
    tag_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.source_block_id = block.block_id AND edge.edge_type = 'tag'),
    backlink_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.target_block_id = block.block_id AND edge.edge_type IN ('mention', 'tag'))
WHERE EXISTS (
    SELECT 1 FROM loom_edges edge
    WHERE edge.workspace_id = block.workspace_id
      AND (edge.source_block_id = block.block_id OR edge.target_block_id = block.block_id)
      AND edge.edge_id LIKE 'KDLNK-%'
);
