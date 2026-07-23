-- Restore only values whose current state still exactly matches what 0343
-- applied. Operator/runtime changes after upgrade always win.

UPDATE loom_block_search_index s
SET workspace_id = p.prior_search_workspace_id,
    content_type = p.prior_search_content_type,
    search_text = p.prior_search_text,
    indexed_at = p.prior_indexed_at
FROM knowledge_rich_document_loom_projection_0343_state p
WHERE p.applied
  AND p.prior_search_existed
  AND s.block_id = p.block_id
  AND s.workspace_id = p.workspace_id
  AND s.content_type = 'note'
  AND s.search_text = p.applied_search_text
  AND s.indexed_at = p.applied_indexed_at
  AND EXISTS (
      SELECT 1
      FROM loom_blocks b
      WHERE b.block_id = p.block_id
        AND b.workspace_id = p.workspace_id
        AND b.content_type = 'note'
        AND b.title = p.applied_title
        AND b.content_hash = p.applied_content_hash
        AND b.derived_json = p.applied_derived_json
        AND b.updated_at = p.applied_block_updated_at
  );

-- If 0343 added the search projection for a block that already existed, remove
-- only that exact migration-applied row. A later block or search edit makes the
-- predicate false and preserves the runtime-owned projection.
DELETE FROM loom_block_search_index s
USING knowledge_rich_document_loom_projection_0343_state p
WHERE p.applied
  AND p.block_existed
  AND NOT p.prior_search_existed
  AND s.block_id = p.block_id
  AND s.workspace_id = p.workspace_id
  AND s.content_type = 'note'
  AND s.search_text = p.applied_search_text
  AND s.indexed_at = p.applied_indexed_at
  AND EXISTS (
      SELECT 1
      FROM loom_blocks b
      WHERE b.block_id = p.block_id
        AND b.workspace_id = p.workspace_id
        AND b.content_type = 'note'
        AND b.title = p.applied_title
        AND b.content_hash = p.applied_content_hash
        AND b.derived_json = p.applied_derived_json
        AND b.updated_at = p.applied_block_updated_at
  );

UPDATE loom_blocks b
SET title = p.prior_title,
    content_hash = p.prior_content_hash,
    derived_json = p.prior_derived_json,
    updated_at = p.prior_block_updated_at
FROM knowledge_rich_document_loom_projection_0343_state p
WHERE p.applied
  AND p.block_existed
  AND b.block_id = p.block_id
  AND b.workspace_id = p.workspace_id
  AND b.content_type = 'note'
  AND b.title = p.applied_title
  AND b.content_hash = p.applied_content_hash
  AND b.derived_json = p.applied_derived_json
  AND b.updated_at = p.applied_block_updated_at;

-- A block inserted by 0343 is deleted only while both it and its search row
-- still carry the exact migration-applied projection. This cannot erase later
-- edits or a search projection rebuilt by runtime code.
DELETE FROM loom_blocks b
USING knowledge_rich_document_loom_projection_0343_state p
WHERE p.applied
  AND NOT p.block_existed
  AND b.block_id = p.block_id
  AND b.workspace_id = p.workspace_id
  AND b.content_type = 'note'
  AND b.document_id IS NULL
  AND b.asset_id IS NULL
  AND b.title = p.applied_title
  AND b.original_filename IS NULL
  AND b.content_hash = p.applied_content_hash
  AND b.pinned = 0
  AND b.favorite = 0
  AND b.pin_order IS NULL
  AND b.journal_date IS NULL
  AND b.last_actor_kind = 'SYSTEM'
  AND b.derived_json = p.applied_derived_json
  AND b.updated_at = p.applied_block_updated_at
  AND b.last_actor_id = 'knowledge_rich_document_backfill_0343'
  AND b.last_job_id IS NULL
  AND b.last_workflow_id IS NULL
  AND b.edit_event_id = '00000000-0000-0000-0000-000000000000'
  AND b.imported_at IS NULL
  AND b.backlink_count = 0
  AND b.mention_count = 0
  AND b.tag_count = 0
  AND b.preview_status = 'none'
  AND b.thumbnail_asset_id IS NULL
  AND b.proxy_asset_id IS NULL
  AND EXISTS (
      SELECT 1
      FROM loom_block_search_index s
      WHERE s.block_id = p.block_id
        AND s.workspace_id = p.workspace_id
        AND s.content_type = 'note'
        AND s.search_text = p.applied_search_text
        AND s.indexed_at = p.applied_indexed_at
  )
  -- Never let rollback cascade-delete relationships or authority records that
  -- were attached after 0343 created the projection block.
  AND NOT EXISTS (
      SELECT 1 FROM loom_edges e
      WHERE e.source_block_id = p.block_id OR e.target_block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM knowledge_sources ks WHERE ks.loom_block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM loom_block_knowledge_bridge kb WHERE kb.block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM loom_folder_members fm WHERE fm.block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM loom_canvas_boards cb WHERE cb.block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM loom_canvas_placements cp WHERE cp.placed_block_id = p.block_id
  )
  AND NOT EXISTS (
      SELECT 1 FROM atelier_intake_item_loom_projection aip
      WHERE aip.loom_block_id = p.block_id
  );

DROP TABLE knowledge_rich_document_loom_projection_0343_state;
