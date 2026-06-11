-- WP-KERNEL-005 MT-021: AI tag suggestions are reviewable proposals, not auto-truth.
-- Accepted suggestions are explicitly applied into the existing manual tag surface.

CREATE TABLE IF NOT EXISTS atelier_ai_tag_suggestion (
    suggestion_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    asset_id              UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    tag_text              TEXT NOT NULL,
    confidence            DOUBLE PRECISION CHECK (confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
    model_receipt_ref     TEXT NOT NULL,
    tool_receipt_ref      TEXT NOT NULL,
    suggested_by          TEXT NOT NULL,
    status                TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','accepted','rejected','applied')),
    decided_by            TEXT,
    decision_reason       TEXT,
    applied_tag_id        UUID REFERENCES atelier_tag(tag_id) ON DELETE SET NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_ai_tag_suggestion_character
    ON atelier_ai_tag_suggestion(character_internal_id, status, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_ai_tag_suggestion_asset
    ON atelier_ai_tag_suggestion(asset_id, status, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_ai_tag_suggestion_status
    ON atelier_ai_tag_suggestion(status, updated_at_utc DESC);
