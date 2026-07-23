DROP INDEX IF EXISTS idx_loom_canvas_stage_provenance;

DROP TRIGGER IF EXISTS trg_guard_stage_fems_memory_proposal_reference
    ON fems_memory_proposals;
DROP TRIGGER IF EXISTS trg_guard_stage_document_backlink_reference
    ON knowledge_document_backlinks;
DROP TRIGGER IF EXISTS trg_guard_stage_context_bundle_item_reference
    ON knowledge_context_bundle_items;
DROP TRIGGER IF EXISTS trg_guard_stage_quick_switcher_recent_reference
    ON knowledge_quick_switcher_recents;
DROP TRIGGER IF EXISTS trg_guard_stage_knowledge_source_reference
    ON knowledge_sources;
DROP TRIGGER IF EXISTS trg_guard_stage_loom_ai_suggestion_reference
    ON loom_ai_suggestions;
DROP TRIGGER IF EXISTS trg_guard_stage_loom_edge_text_reference ON loom_edges;

DROP FUNCTION IF EXISTS guard_stage_fems_memory_proposal_reference();
DROP FUNCTION IF EXISTS guard_stage_document_backlink_reference();
DROP FUNCTION IF EXISTS guard_stage_context_bundle_item_reference();
DROP FUNCTION IF EXISTS guard_stage_quick_switcher_recent_reference();
DROP FUNCTION IF EXISTS guard_stage_knowledge_source_reference();
DROP FUNCTION IF EXISTS guard_stage_loom_ai_suggestion_reference();
DROP FUNCTION IF EXISTS guard_stage_loom_edge_text_reference();
DROP FUNCTION IF EXISTS guard_stage_compensated_references(TEXT, TEXT[]);

ALTER TABLE loom_canvas_placements
    DROP CONSTRAINT IF EXISTS loom_canvas_placements_stage_provenance_key_check,
    DROP COLUMN IF EXISTS stage_provenance,
    DROP COLUMN IF EXISTS stage_provenance_key;

-- The legacy Stage metadata repair is intentionally not reversed: restoring a
-- digest/size that no longer describes content_bytes would reintroduce corrupt
-- authority rather than undo schema.
