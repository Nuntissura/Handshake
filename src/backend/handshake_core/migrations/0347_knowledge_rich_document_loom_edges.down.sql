-- Roll back only the KDLNK-owned RichDocument graph projection. Independently
-- authored Loom edges are not in this namespace and remain untouched.

LOCK TABLE loom_blocks, loom_edges IN SHARE ROW EXCLUSIVE MODE;

CREATE TEMP TABLE hsk_0347_affected_loom_blocks ON COMMIT DROP AS
SELECT workspace_id, source_block_id AS block_id
FROM loom_edges
WHERE edge_id LIKE 'KDLNK-%'
  AND last_actor_kind = 'SYSTEM'
  AND last_actor_id = 'knowledge_rich_document_backlink_projection'
  AND edit_event_id = '00000000-0000-0000-0000-000000000000'
  AND source_document_id = source_block_id
UNION
SELECT workspace_id, target_block_id AS block_id
FROM loom_edges
WHERE edge_id LIKE 'KDLNK-%'
  AND last_actor_kind = 'SYSTEM'
  AND last_actor_id = 'knowledge_rich_document_backlink_projection'
  AND edit_event_id = '00000000-0000-0000-0000-000000000000'
  AND source_document_id = source_block_id;

DELETE FROM loom_edges
WHERE edge_id LIKE 'KDLNK-%'
  AND last_actor_kind = 'SYSTEM'
  AND last_actor_id = 'knowledge_rich_document_backlink_projection'
  AND edit_event_id = '00000000-0000-0000-0000-000000000000'
  AND source_document_id = source_block_id;

UPDATE loom_blocks block
SET
    mention_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.source_block_id = block.block_id AND edge.edge_type = 'mention'),
    tag_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.source_block_id = block.block_id AND edge.edge_type = 'tag'),
    backlink_count = (SELECT COUNT(*)::INT FROM loom_edges edge WHERE edge.workspace_id = block.workspace_id AND edge.target_block_id = block.block_id AND edge.edge_type IN ('mention', 'tag'))
WHERE (block.workspace_id, block.block_id) IN (
    SELECT workspace_id, block_id FROM hsk_0347_affected_loom_blocks
);
