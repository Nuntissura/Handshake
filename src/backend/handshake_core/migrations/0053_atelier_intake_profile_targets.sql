-- WP-KERNEL-005 MT-030: distinguish loose-profile intake from
-- character-linked intake and persist target refs for character, sheet
-- version, and collection. PostgreSQL authority only.

ALTER TABLE atelier_intake_batch
    ADD COLUMN IF NOT EXISTS profile_mode TEXT NOT NULL DEFAULT 'loose_profile',
    ADD COLUMN IF NOT EXISTS target_character_id UUID REFERENCES atelier_character(internal_id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS target_sheet_version_id UUID REFERENCES atelier_sheet_version(version_id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS target_collection_id UUID REFERENCES atelier_collection(collection_id) ON DELETE SET NULL;

UPDATE atelier_intake_batch
SET target_character_id = COALESCE(target_character_id, character_internal_id),
    profile_mode = CASE
        WHEN COALESCE(target_character_id, character_internal_id) IS NOT NULL
          OR target_sheet_version_id IS NOT NULL
          OR target_collection_id IS NOT NULL
        THEN 'character_linked'
        ELSE 'loose_profile'
    END;

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_profile_mode;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_profile_mode
    CHECK (profile_mode IN ('loose_profile', 'character_linked'));

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_profile_targets;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_profile_targets
    CHECK (
        (
            profile_mode = 'loose_profile'
            AND target_character_id IS NULL
            AND target_sheet_version_id IS NULL
            AND target_collection_id IS NULL
            AND character_internal_id IS NULL
        )
        OR
        (
            profile_mode = 'character_linked'
            AND target_character_id IS NOT NULL
        )
    );

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_profile_mode
    ON atelier_intake_batch(profile_mode, updated_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_target_character
    ON atelier_intake_batch(target_character_id);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_target_sheet
    ON atelier_intake_batch(target_sheet_version_id);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_target_collection
    ON atelier_intake_batch(target_collection_id);
