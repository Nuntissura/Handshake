-- WP-1-Loom-MVP-v1: Loom core entities + portable schema baseline
-- Spec anchors:
-- - §2.2.1.14 LoomBlock Entity [ADD v02.130]
-- - §2.3.7.1 Loom Relational Edges [ADD v02.130]
-- - §2.3.13.7 Loom Storage Trait + Portable Schema (no triggers) [ADD v02.130]
-- - §10.12 Loom Integration Spec (import/dedup + cache tiers + views + search) [ADD v02.130]

-- =============================================================================
-- Assets (minimal portable subset for Loom import/preview)
-- =============================================================================
CREATE TABLE IF NOT EXISTS assets (
    asset_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    mime TEXT NOT NULL,
    original_filename TEXT,
    content_hash TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    width INTEGER,
    height INTEGER,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    classification TEXT NOT NULL DEFAULT 'low' CHECK (classification IN ('low', 'medium', 'high', 'unknown')),
    exportable INTEGER NOT NULL DEFAULT 1,
    is_proxy_of TEXT,
    proxy_asset_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_assets_workspace ON assets(workspace_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_workspace_content_hash ON assets(workspace_id, content_hash);

-- =============================================================================
-- LoomBlocks
-- =============================================================================
CREATE TABLE IF NOT EXISTS loom_blocks (
    block_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    content_type TEXT NOT NULL CHECK (content_type IN ('note', 'file', 'annotated_file', 'tag_hub', 'journal')),
    document_id TEXT REFERENCES documents(id) ON DELETE SET NULL,
    asset_id TEXT REFERENCES assets(asset_id) ON DELETE SET NULL,
    title TEXT,
    original_filename TEXT,
    content_hash TEXT,
    pinned INTEGER NOT NULL DEFAULT 0,
    journal_date TEXT,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    imported_at TIMESTAMP,
    backlink_count INTEGER NOT NULL DEFAULT 0,
    mention_count INTEGER NOT NULL DEFAULT 0,
    tag_count INTEGER NOT NULL DEFAULT 0,
    derived_json TEXT NOT NULL DEFAULT '{}',
    preview_status TEXT NOT NULL DEFAULT 'none' CHECK (preview_status IN ('none', 'pending', 'generated', 'failed')),
    thumbnail_asset_id TEXT REFERENCES assets(asset_id) ON DELETE SET NULL,
    proxy_asset_id TEXT REFERENCES assets(asset_id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_loom_blocks_workspace_updated_at ON loom_blocks(workspace_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_loom_blocks_workspace_pinned ON loom_blocks(workspace_id, pinned);
CREATE INDEX IF NOT EXISTS idx_loom_blocks_workspace_content_hash ON loom_blocks(workspace_id, content_hash);

-- =============================================================================
-- LoomEdges
-- =============================================================================
CREATE TABLE IF NOT EXISTS loom_edges (
    edge_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_block_id TEXT NOT NULL REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    target_block_id TEXT NOT NULL REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL CHECK (edge_type IN ('mention', 'tag', 'sub_tag', 'parent', 'ai_suggested')),
    created_by TEXT NOT NULL CHECK (created_by IN ('user', 'ai')),
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    crdt_site_id TEXT,
    source_document_id TEXT,
    source_text_block_id TEXT,
    offset_start INTEGER,
    offset_end INTEGER
);

CREATE INDEX IF NOT EXISTS idx_loom_edges_workspace_source ON loom_edges(workspace_id, source_block_id);
CREATE INDEX IF NOT EXISTS idx_loom_edges_workspace_target ON loom_edges(workspace_id, target_block_id);
