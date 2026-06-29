-- MT-010 CKC media-album hardening.
-- Album names are scoped per CKC character instead of globally unique, and
-- folder/source refs can be carried by the album membership itself so the same
-- media asset can appear in different CKC albums with different provenance.

ALTER TABLE atelier_collection
    DROP CONSTRAINT IF EXISTS atelier_collection_name_key;

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_collection_scoped_name
    ON atelier_collection (COALESCE(character_internal_id, '00000000-0000-0000-0000-000000000000'::uuid), name);

ALTER TABLE atelier_collection_item
    ADD COLUMN IF NOT EXISTS source_path_ref TEXT,
    ADD COLUMN IF NOT EXISTS source_url_ref TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.table_constraints
        WHERE table_name = 'atelier_collection_item'
          AND constraint_name = 'atelier_collection_item_source_ref_trim_ck'
    ) THEN
        ALTER TABLE atelier_collection_item
            ADD CONSTRAINT atelier_collection_item_source_ref_trim_ck CHECK (
                (source_path_ref IS NULL OR (btrim(source_path_ref) = source_path_ref AND source_path_ref <> ''))
                AND (source_url_ref IS NULL OR (btrim(source_url_ref) = source_url_ref AND source_url_ref <> ''))
            );
    END IF;
END $$;

-- Deliberately do not backfill link-scoped refs from old asset-level provenance.
-- Album member reads already fall back to asset-level provenance, while new
-- per-album provenance is written through validated Rust APIs. Copying old
-- values here would risk promoting dirty legacy refs into collection items.

CREATE INDEX IF NOT EXISTS idx_atelier_collection_item_source_path_ref
    ON atelier_collection_item(source_path_ref)
    WHERE source_path_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_collection_item_source_url_ref
    ON atelier_collection_item(source_url_ref)
    WHERE source_url_ref IS NOT NULL;
