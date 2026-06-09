------------------------------------------------------------------------------
-- STEALTH REFERENCE WINDOW UUID v7 BOUNDARY (S10.18.2 / S10.18.3)
------------------------------------------------------------------------------
-- Runtime code must bind stable UUID v7 ids. Drop UUID v4 database defaults so
-- direct SQL insert paths fail closed instead of silently minting v4 ids.

ALTER TABLE IF EXISTS atelier_stealth_window
    ALTER COLUMN window_ref_id DROP DEFAULT;

ALTER TABLE IF EXISTS atelier_stealth_ref
    ALTER COLUMN ref_id DROP DEFAULT;

ALTER TABLE IF EXISTS atelier_stealth_capture
    ALTER COLUMN capture_id DROP DEFAULT;
