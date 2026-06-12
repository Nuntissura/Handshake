-- WP-KERNEL-009 MT-181 FolderTreeAndColorLabels.
--
-- Master Spec anchor: §7.1.4.3 / MT-181 — Loom offers a persistent folder
-- hierarchy with labels, colors, sort modes, and project membership ("links are
-- the new folders" but an explicit tree is still offered; Obsidian file-tree
-- idiom). Authority is PostgreSQL; this is a navigation/organization projection
-- over LoomBlocks, never a second source of block truth.
--
--   loom_folders          : the tree (self-referential parent_folder_id) +
--                           per-folder color label, sort mode, manual ordinal,
--                           and optional project membership token.
--   loom_folder_members   : which LoomBlocks live in which folder (a block may
--                           appear in multiple folders; the folder tree is an
--                           organizational overlay, not exclusive ownership).

CREATE TABLE IF NOT EXISTS loom_folders (
    folder_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Self-referential hierarchy. NULL parent => a root folder. ON DELETE
    -- CASCADE: deleting a folder removes its subtree (membership rows cascade
    -- from loom_folder_members' FK below).
    parent_folder_id TEXT REFERENCES loom_folders(folder_id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- Optional color label: a short hex token (#rgb / #rrggbb) or a named token.
    color TEXT,
    -- Sort mode for the folder's contents (closed vocabulary).
    sort_mode TEXT NOT NULL DEFAULT 'updated_desc'
        CHECK (sort_mode IN (
            'name_asc', 'name_desc', 'created_desc', 'updated_desc', 'manual'
        )),
    -- Manual ordinal for ordering sibling folders (when the parent uses manual
    -- ordering); NULL sorts after explicitly-ordered siblings.
    sort_order INTEGER,
    -- Optional stable project-membership token (the project surface is owned by
    -- other WPs, so this is a soft ref, never a hard FK — mirrors MT-152
    -- typed-reference discipline; NEVER a random absolute filesystem path).
    project_ref TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_loom_folders_name
        CHECK (btrim(name) = name AND name <> ''),
    CONSTRAINT chk_loom_folders_color
        CHECK (color IS NULL OR (btrim(color) = color AND color <> '')),
    -- A folder cannot be its own parent (deeper cycles are guarded at the
    -- service layer when moving a folder).
    CONSTRAINT chk_loom_folders_not_self_parent
        CHECK (parent_folder_id IS NULL OR parent_folder_id <> folder_id),
    -- Sibling folder names are unique within a parent (and among roots) per
    -- workspace, so the tree has no ambiguous duplicate paths. COALESCE maps the
    -- NULL parent (root) to a sentinel so roots are de-duplicated too.
    CONSTRAINT uq_loom_folders_sibling_name
        UNIQUE (workspace_id, parent_folder_id, name)
);

CREATE INDEX IF NOT EXISTS idx_loom_folders_workspace_parent
    ON loom_folders (workspace_id, parent_folder_id);

CREATE TABLE IF NOT EXISTS loom_folder_members (
    folder_id TEXT NOT NULL REFERENCES loom_folders(folder_id) ON DELETE CASCADE,
    block_id TEXT NOT NULL REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Manual ordinal for ordering blocks within a folder (used when the
    -- folder's sort_mode = 'manual'); NULL sorts after explicitly-ordered.
    sort_order INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (folder_id, block_id)
);

CREATE INDEX IF NOT EXISTS idx_loom_folder_members_block
    ON loom_folder_members (workspace_id, block_id);

CREATE INDEX IF NOT EXISTS idx_loom_folder_members_folder_order
    ON loom_folder_members (folder_id, sort_order);
