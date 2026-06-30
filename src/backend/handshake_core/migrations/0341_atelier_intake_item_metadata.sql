-- MT-017: durable metadata for dataset-mining intake decisions.
-- Batch-level Ingest apply keeps operator/agent metadata queryable per item
-- without relying only on event JSON payloads.

CREATE TABLE IF NOT EXISTS atelier_intake_item_metadata (
    item_id UUID PRIMARY KEY REFERENCES atelier_intake_item(item_id) ON DELETE CASCADE,
    batch_id UUID NOT NULL REFERENCES atelier_intake_batch(batch_id) ON DELETE CASCADE,
    asset_id UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    request_id TEXT NOT NULL,
    dataset_ref TEXT,
    character_ref TEXT,
    link_passed BOOLEAN NOT NULL DEFAULT FALSE,
    tags_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    note TEXT,
    event_label TEXT,
    event_date TEXT,
    location TEXT,
    facial_profile TEXT,
    loaded_item_count BIGINT,
    contact_sheet_json JSONB,
    requested_by TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_intake_item_metadata_request_id CHECK (
        btrim(request_id) = request_id AND request_id <> ''
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_requested_by CHECK (
        btrim(requested_by) = requested_by AND requested_by <> ''
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_dataset_ref CHECK (
        dataset_ref IS NULL OR (btrim(dataset_ref) = dataset_ref AND dataset_ref <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_character_ref CHECK (
        character_ref IS NULL OR (btrim(character_ref) = character_ref AND character_ref <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_note CHECK (
        note IS NULL OR (btrim(note) = note AND note <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_event_label CHECK (
        event_label IS NULL OR (btrim(event_label) = event_label AND event_label <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_event_date CHECK (
        event_date IS NULL OR (btrim(event_date) = event_date AND event_date <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_location CHECK (
        location IS NULL OR (btrim(location) = location AND location <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_facial_profile CHECK (
        facial_profile IS NULL OR (btrim(facial_profile) = facial_profile AND facial_profile <> '')
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_loaded_count CHECK (
        loaded_item_count IS NULL OR loaded_item_count >= 0
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_tags CHECK (
        jsonb_typeof(tags_json) = 'array'
    ),
    CONSTRAINT chk_atelier_intake_item_metadata_contact_sheet CHECK (
        contact_sheet_json IS NULL OR jsonb_typeof(contact_sheet_json) = 'object'
    )
);

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_request_id;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_request_id CHECK (
        btrim(request_id) = request_id AND request_id <> ''
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_requested_by;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_requested_by CHECK (
        btrim(requested_by) = requested_by AND requested_by <> ''
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_dataset_ref;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_dataset_ref CHECK (
        dataset_ref IS NULL OR (btrim(dataset_ref) = dataset_ref AND dataset_ref <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_character_ref;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_character_ref CHECK (
        character_ref IS NULL OR (btrim(character_ref) = character_ref AND character_ref <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_note;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_note CHECK (
        note IS NULL OR (btrim(note) = note AND note <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_event_label;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_event_label CHECK (
        event_label IS NULL OR (btrim(event_label) = event_label AND event_label <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_event_date;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_event_date CHECK (
        event_date IS NULL OR (btrim(event_date) = event_date AND event_date <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_location;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_location CHECK (
        location IS NULL OR (btrim(location) = location AND location <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_facial_profile;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_facial_profile CHECK (
        facial_profile IS NULL OR (btrim(facial_profile) = facial_profile AND facial_profile <> '')
    );

ALTER TABLE atelier_intake_item_metadata
    ADD COLUMN IF NOT EXISTS loaded_item_count BIGINT;

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_loaded_count;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_loaded_count CHECK (
        loaded_item_count IS NULL OR loaded_item_count >= 0
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_tags;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_tags CHECK (
        jsonb_typeof(tags_json) = 'array'
    );

ALTER TABLE atelier_intake_item_metadata
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_metadata_contact_sheet;
ALTER TABLE atelier_intake_item_metadata
    ADD CONSTRAINT chk_atelier_intake_item_metadata_contact_sheet CHECK (
        contact_sheet_json IS NULL OR jsonb_typeof(contact_sheet_json) = 'object'
    );

CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_metadata_batch
    ON atelier_intake_item_metadata(batch_id, updated_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_metadata_asset
    ON atelier_intake_item_metadata(asset_id)
    WHERE asset_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_metadata_request
    ON atelier_intake_item_metadata(request_id);
