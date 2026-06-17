-- WP-KERNEL-009 MT-262 UnifiedWorkSurface-262-BlockCollectionViews.
-- Master Spec §10.12: saved block-collection views (table / Kanban / calendar)
-- over the REAL Loom query backend. A saved view is itself a typed LoomBlock
-- (content_type='view_def'); its query/columns/grouping live in a dedicated
-- view_definition_json column (NOT a derived_json overload, so the view
-- definition can never be mistaken for full-text index payload). Authority is
-- PostgreSQL + EventLedger; the React table/Kanban/calendar are projections.
--
-- Mutations driven from these views (Kanban drag = tag/field change) route
-- through the existing patch_loom_block / create_loom_edge / delete_loom_edge
-- paths with their own receipts. NO parallel store, NO localStorage authority.
--
-- TRAP GUARD: view_definition_json is dedicated and nullable; non-view blocks
-- keep it NULL. This widens the same loom_blocks content_type allow-list that
-- 0334 (canvas) last extended (additive; mirrors the 0333/0334 widening pattern).

-- A dedicated typed payload column for view definitions (kind, query, columns,
-- group_by, sort, calendar_date_field). NULL for every non-view block.
ALTER TABLE loom_blocks
    ADD COLUMN IF NOT EXISTS view_definition_json TEXT;

-- Widen the loom_blocks content_type allow-list to admit 'view_def'. Drop the
-- existing CHECK and re-add the superset (the 0334 set plus 'view_def').
ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS loom_blocks_content_type_check;

ALTER TABLE loom_blocks
    ADD CONSTRAINT loom_blocks_content_type_check
    CHECK (content_type IN (
        'note',
        'file',
        'annotated_file',
        'tag_hub',
        'journal',
        'canvas',
        'view_def'
    ));

-- A saved view's definition must be present exactly when the block is a view,
-- and absent otherwise. This keeps the dedicated column honest: a view_def
-- block always carries its definition; nothing else ever sets it.
ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS chk_loom_blocks_view_definition;

ALTER TABLE loom_blocks
    ADD CONSTRAINT chk_loom_blocks_view_definition
    CHECK (
        (content_type = 'view_def' AND view_definition_json IS NOT NULL)
        OR (content_type <> 'view_def' AND view_definition_json IS NULL)
    );
