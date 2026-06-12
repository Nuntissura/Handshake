-- WP-KERNEL-009 MT-242 hardening (projection-isolation law repair).
-- Master Spec 2.3.13.11: "Deleting or editing a projection MUST NOT mutate
-- authority records" + the structural boundary "no FK may point INTO
-- knowledge_wiki_projections" (proven by
-- knowledge_documents_tests::no_authority_table_references_the_projection_table).
--
-- Migration 0295 gave loom_wiki_overlays (operator annotations = AUTHORITY
-- rows) a hard FK into knowledge_wiki_projections with ON DELETE CASCADE.
-- That cascade meant deleting a regenerable projection DESTROYED authority
-- rows — exactly what the law forbids — and the FK itself broke the
-- zero-inbound-FK catalog proof.
--
-- Fix: projection_id becomes a SOFT reference. Overlays survive projection
-- deletion (operator annotations are never destroyed by projection churn);
-- an overlay whose projection was deleted is an orphan the operator can see
-- and clean up, never silent data loss. The add-overlay path still validates
-- the projection exists in the workspace at write time (app-side check in
-- storage/postgres.rs add_loom_wiki_overlay).

ALTER TABLE loom_wiki_overlays
    DROP CONSTRAINT IF EXISTS loom_wiki_overlays_projection_id_fkey;
