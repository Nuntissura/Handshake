-- Revert: restore the 0295 hard FK (CASCADE). NOTE: this re-introduces the
-- projection-isolation violation and fails if orphaned overlays exist; clean
-- them up first.
DELETE FROM loom_wiki_overlays o
WHERE NOT EXISTS (
    SELECT 1 FROM knowledge_wiki_projections p
    WHERE p.projection_id = o.projection_id
);
ALTER TABLE loom_wiki_overlays
    ADD CONSTRAINT loom_wiki_overlays_projection_id_fkey
        FOREIGN KEY (projection_id)
        REFERENCES knowledge_wiki_projections(projection_id) ON DELETE CASCADE;
