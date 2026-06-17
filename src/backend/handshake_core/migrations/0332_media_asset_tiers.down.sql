-- WP-KERNEL-009 MT-259 MediaCacheTiers — down migration (replay-safe).
-- (No knowledge_schema_registry rows were inserted; these are Loom/asset-domain
-- tables outside the knowledge namespace.)
DROP INDEX IF EXISTS idx_loom_collection_members_ordered;
DROP TABLE IF EXISTS loom_collection_members;

DROP INDEX IF EXISTS idx_loom_collections_workspace;
DROP TABLE IF EXISTS loom_collections;

DROP INDEX IF EXISTS idx_media_asset_tiers_workspace_status;
DROP INDEX IF EXISTS idx_media_asset_tiers_asset;
DROP TABLE IF EXISTS media_asset_tiers;
