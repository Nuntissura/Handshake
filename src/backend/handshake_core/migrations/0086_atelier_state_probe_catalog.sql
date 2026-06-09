-- WP-KERNEL-005 MT-138 model-workflow state probe catalog.
-- Runtime catalog rows are PostgreSQL authority and are mirrored through the
-- Atelier EventLedger event family when a catalog snapshot is recorded.

CREATE TABLE IF NOT EXISTS atelier_state_probe_catalog_entry (
    catalog_id                            TEXT NOT NULL,
    probe_id                              TEXT NOT NULL,
    surface                               TEXT NOT NULL,
    probe_label                           TEXT NOT NULL,
    read_model                            TEXT NOT NULL,
    inspection_phase                      TEXT NOT NULL,
    required_before_visual_inspection      BOOLEAN NOT NULL,
    status                                TEXT NOT NULL,
    probe_fields                          JSONB NOT NULL,
    evidence_refs                         JSONB NOT NULL,
    updated_at_utc                        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (catalog_id, probe_id),
    CONSTRAINT chk_atelier_state_probe_catalog_id_trimmed
        CHECK (catalog_id = btrim(catalog_id) AND catalog_id <> ''),
    CONSTRAINT chk_atelier_state_probe_probe_id_trimmed
        CHECK (probe_id = btrim(probe_id) AND probe_id <> ''),
    CONSTRAINT chk_atelier_state_probe_surface
        CHECK (surface IN (
            'character',
            'media',
            'intake',
            'collection',
            'docs',
            'moodboard',
            'pose',
            'comfyui_job',
            'session',
            'errors'
        )),
    CONSTRAINT chk_atelier_state_probe_phase
        CHECK (inspection_phase IN ('pre_visual_inspection')),
    CONSTRAINT chk_atelier_state_probe_read_model
        CHECK (read_model = 'postgres_event_ledger_projection'),
    CONSTRAINT chk_atelier_state_probe_required_pre_visual
        CHECK (required_before_visual_inspection IS TRUE),
    CONSTRAINT chk_atelier_state_probe_status
        CHECK (status = 'ready'),
    CONSTRAINT chk_atelier_state_probe_fields_object
        CHECK (
            jsonb_typeof(probe_fields) = 'object'
            AND jsonb_typeof(probe_fields->'fields') = 'array'
            AND jsonb_array_length(probe_fields->'fields') > 0
            AND probe_fields->>'schema' = 'hsk.atelier.state_probe.fields@1'
            AND probe_fields->>'state_authority' = 'postgres'
            AND probe_fields->>'event_authority' = 'kernel_event_ledger'
        ),
    CONSTRAINT chk_atelier_state_probe_evidence_refs_array
        CHECK (
            jsonb_typeof(evidence_refs) = 'array'
            AND jsonb_array_length(evidence_refs) > 0
        )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_atelier_state_probe_catalog_surface
    ON atelier_state_probe_catalog_entry(catalog_id, surface);
