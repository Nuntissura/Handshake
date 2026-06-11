-- WP-KERNEL-005 / S6.10: bind media-downloader sessions to capability-profile grants.

ALTER TABLE IF EXISTS atelier_md_download_session
    ADD COLUMN IF NOT EXISTS protocol_id TEXT NOT NULL DEFAULT 'hsk.media_downloader.batch.v0';

ALTER TABLE IF EXISTS atelier_md_download_session
    ADD COLUMN IF NOT EXISTS capability_profile_id TEXT NOT NULL DEFAULT 'MediaDownloader';

ALTER TABLE IF EXISTS atelier_md_download_session
    ADD COLUMN IF NOT EXISTS capability_grant_ref TEXT NOT NULL DEFAULT 'capgrant://media_downloader/MediaDownloader/migrated-session';

CREATE INDEX IF NOT EXISTS idx_atelier_md_session_protocol_profile
    ON atelier_md_download_session(protocol_id, capability_profile_id);
