-- WP-KERNEL-009 MT-259 MediaCacheTiers — GAP-LM-009 / GAP-LM-244a.
-- Master Spec anchor: 2.3.13.11 (Project Knowledge Index & Rich Document
-- authority); DEC-007 (no scaffolding — every capability proven at runtime).
--
-- Two new durable concerns, both PostgreSQL-canonical (UI artifacts are
-- projections; originals are authority; tiers are regenerable derived
-- artifacts):
--
--   1. media_asset_tiers — per-asset, per-tier cache state for the background
--      preview-pyramid generation job (thumb -> preview -> full). Today tier
--      state lived only on loom_blocks.derived_json (block-keyed, single slot,
--      no failure/retry accounting). This table makes tier state per-asset and
--      per-tier with explicit status + failure_reason + attempt_count so the
--      retry queue is visible and never silent. Deleting tier rows MUST NOT
--      touch the original asset blob (tiers are derived).
--
--   2. loom_collections / loom_collection_members — a real backend list-source
--      for ordered album/slideshow membership (GAP-LM-244a). Previously the
--      frontend comma-split an ordered asset-id list out of refValue with no
--      backend entity; this gives an ordered, server-enumerable member list.

CREATE TABLE IF NOT EXISTS media_asset_tiers (
    tier_row_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    -- The ORIGINAL asset this tier derives from. ON DELETE CASCADE: if the
    -- original is removed the derived tier rows go too (the reverse is barred
    -- in storage: deleting tiers never deletes the original blob).
    asset_id TEXT NOT NULL REFERENCES assets(asset_id) ON DELETE CASCADE,
    -- thumb | preview | poster | full
    tier TEXT NOT NULL CHECK (tier IN ('thumb', 'preview', 'poster', 'full')),
    -- pending | ready | failed
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'ready', 'failed')),
    -- The derived asset row holding this tier's blob (NULL until ready / for the
    -- 'full' tier which points back at the original). FK SET NULL so dropping a
    -- derived asset degrades the tier to "needs regeneration" rather than
    -- orphaning a dangling id.
    tier_asset_id TEXT REFERENCES assets(asset_id) ON DELETE SET NULL,
    -- content_hash of the derived blob (lets the view cache-bust deterministically).
    content_hash TEXT,
    failure_reason TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_media_asset_tiers_asset_tier UNIQUE (asset_id, tier)
);

CREATE INDEX IF NOT EXISTS idx_media_asset_tiers_asset
    ON media_asset_tiers (asset_id, tier);
CREATE INDEX IF NOT EXISTS idx_media_asset_tiers_workspace_status
    ON media_asset_tiers (workspace_id, status);

CREATE TABLE IF NOT EXISTS loom_collections (
    collection_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_loom_collections_workspace
    ON loom_collections (workspace_id);

CREATE TABLE IF NOT EXISTS loom_collection_members (
    collection_id TEXT NOT NULL REFERENCES loom_collections(collection_id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets(asset_id) ON DELETE CASCADE,
    -- Dense ordinal; storage re-densifies on set-order so gaps never accumulate.
    position INTEGER NOT NULL CHECK (position >= 0),
    PRIMARY KEY (collection_id, asset_id),
    CONSTRAINT uq_loom_collection_members_order UNIQUE (collection_id, position)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX IF NOT EXISTS idx_loom_collection_members_ordered
    ON loom_collection_members (collection_id, position);

-- NOTE: knowledge_schema_registry is the WP-009 *knowledge namespace* boundary
-- (`knowledge_`-prefixed tables only, per chk_knowledge_schema_registry_prefix).
-- These tables belong to the Loom/asset domain (siblings of `assets`,
-- `loom_blocks`, `loom_edges`, which are likewise NOT registered there), so they
-- are intentionally not inserted into knowledge_schema_registry.
