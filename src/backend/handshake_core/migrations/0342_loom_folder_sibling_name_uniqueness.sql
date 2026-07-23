-- WP-KERNEL-012 MT-022: make Loom folder sibling-name uniqueness truthful for root folders.
--
-- PostgreSQL UNIQUE constraints treat NULL values as distinct, so migration 0294's
-- UNIQUE(workspace_id, parent_folder_id, name) did not reject two same-name roots. Partial unique
-- indexes express both scopes portably: one key for roots and one for non-root siblings.
--
-- This is a forward migration instead of editing applied 0294, preserving sqlx migration checksums and
-- ensuring existing installations receive the correction. Migration 0294 allowed duplicate roots because
-- NULL values are distinct. Preserve every existing folder while deterministically repairing only the
-- later duplicates before installing the truthful indexes. The suffix includes the stable folder id, and
-- the bounded loop handles the counterfactual where an operator already used the generated recovery name.

ALTER TABLE loom_folders
    DROP CONSTRAINT IF EXISTS uq_loom_folders_sibling_name;

DO $$
DECLARE
    duplicate_row RECORD;
    base_name TEXT;
    candidate_name TEXT;
    candidate_counter INTEGER;
BEGIN
    FOR duplicate_row IN
        SELECT folder_id, workspace_id, parent_folder_id, name
        FROM (
            SELECT folder_id,
                   workspace_id,
                   parent_folder_id,
                   name,
                   ROW_NUMBER() OVER (
                       PARTITION BY workspace_id, parent_folder_id, name
                       ORDER BY created_at ASC, folder_id ASC
                   ) AS duplicate_rank
            FROM loom_folders
        ) ranked
        WHERE duplicate_rank > 1
        ORDER BY workspace_id, parent_folder_id NULLS FIRST, name, folder_id
    LOOP
        base_name := duplicate_row.name || ' [recovered-' || duplicate_row.folder_id || ']';
        candidate_name := base_name;
        candidate_counter := 2;

        WHILE EXISTS (
            SELECT 1
            FROM loom_folders existing
            WHERE existing.workspace_id = duplicate_row.workspace_id
              AND existing.parent_folder_id IS NOT DISTINCT FROM duplicate_row.parent_folder_id
              AND existing.folder_id <> duplicate_row.folder_id
              AND existing.name = candidate_name
        ) LOOP
            candidate_name := base_name || '-' || candidate_counter::TEXT;
            candidate_counter := candidate_counter + 1;
        END LOOP;

        UPDATE loom_folders
        SET name = candidate_name,
            updated_at = NOW()
        WHERE folder_id = duplicate_row.folder_id;

        RAISE NOTICE 'renamed duplicate Loom folder % from % to % during sibling-name recovery',
            duplicate_row.folder_id, duplicate_row.name, candidate_name;
    END LOOP;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS uq_loom_folders_root_name
    ON loom_folders (workspace_id, name)
    WHERE parent_folder_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_loom_folders_child_name
    ON loom_folders (workspace_id, parent_folder_id, name)
    WHERE parent_folder_id IS NOT NULL;
