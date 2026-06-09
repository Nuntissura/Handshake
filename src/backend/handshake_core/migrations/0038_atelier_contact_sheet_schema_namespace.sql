-- WP-KERNEL-005 Tier 1 forbidden legacy namespace repair.
-- Contact-sheet manifests are Handshake artifacts; legacy source namespace rows
-- are backfilled idempotently without rewriting unrelated manifest fields.

UPDATE atelier_contact_sheet
SET manifest = jsonb_set(
    COALESCE(manifest, '{}'::jsonb),
    '{schema}',
    to_jsonb('hsk.atelier.contact_sheet@1'::text),
    true
)
WHERE manifest->>'schema' = ('c' || 'kc.contact_sheet@1');
