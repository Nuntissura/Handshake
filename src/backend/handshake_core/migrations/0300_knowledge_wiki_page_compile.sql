-- WP-KERNEL-009 MT-241 ProjectWikiBootstrapCompiler /
-- MT-242 WikiProjectionDriftAndStaleness / MT-243 WikiIncrementalIngestFanOut.
-- Master Spec anchors: 11-shared-dev-platform §10.12 Section 17
-- [LM-PWIKI-001..013] "Project Wiki Compile Layer: Knowledge as a Compile
-- Target".
--
-- Extends the EXISTING knowledge_wiki_projections store (0139) — the same
-- store the MT-184 Loom wiki/topic compiler writes to (LM-PWIKI-005: a
-- parallel wiki store is forbidden). All columns added here are PROJECTION
-- metadata: the table stays authority_class = 'projection', no authority
-- table references it, and deleting rows still mutates nothing else
-- (LM-PWIKI-001).
--
--   * page_type      — typed compiled pages (LM-PWIKI-002): module | concept
--                      | flow | entity | decision | index. NULL = an untyped
--                      MT-184 Loom topic page (operator-driven, not produced
--                      by the project-wiki bootstrap compiler).
--   * compile_stamp  — the MT-242 stamp (LM-PWIKI-006): the EventLedger
--                      source version (kernel_event_ledger.event_sequence
--                      watermark) plus the EXACT cited-source set
--                      ({kind, id, content_hash}) the page compiled from.
--                      Never wall-clock based.
--   * compile_recipe — the deterministic compile input descriptor (module
--                      cluster file set, entity ref, index) so MT-243 fan-out
--                      can regenerate ONE page from current authority without
--                      a full bootstrap.
--   * page_links     — outbound wikilink set [{title, projection_id?}] so the
--                      compiled wiki is navigable and backlinks are derivable
--                      by reverse lookup (LM-PWIKI-010 backlink refresh).
--
-- SHIP-TOGETHER GUARD (LM-PWIKI-009 / MT-241 constraint "MUST NOT ship
-- without MT-242"): chk_knowledge_wiki_projections_stamp_guard makes a typed
-- project-wiki page WITHOUT its drift/staleness stamp structurally
-- impossible — compile output without stamps cannot exist as a row.

ALTER TABLE knowledge_wiki_projections
    ADD COLUMN IF NOT EXISTS page_type TEXT,
    ADD COLUMN IF NOT EXISTS compile_stamp JSONB,
    ADD COLUMN IF NOT EXISTS compile_recipe JSONB,
    ADD COLUMN IF NOT EXISTS page_links JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE knowledge_wiki_projections
    ADD CONSTRAINT chk_knowledge_wiki_projections_page_type
        CHECK (page_type IS NULL OR page_type IN (
            'module', 'concept', 'flow', 'entity', 'decision', 'index'
        ));

-- LM-PWIKI-009: a typed compiled page cannot exist unstamped.
ALTER TABLE knowledge_wiki_projections
    ADD CONSTRAINT chk_knowledge_wiki_projections_stamp_guard
        CHECK (page_type IS NULL OR compile_stamp IS NOT NULL);

-- Stamp shape guard: when present, a stamp must carry the ledger version and
-- the cited-source array (fail-closed against malformed stamps).
ALTER TABLE knowledge_wiki_projections
    ADD CONSTRAINT chk_knowledge_wiki_projections_stamp_shape
        CHECK (
            compile_stamp IS NULL
            OR (
                jsonb_typeof(compile_stamp -> 'ledger_version') = 'number'
                AND jsonb_typeof(compile_stamp -> 'cited_sources') = 'array'
            )
        );

CREATE INDEX IF NOT EXISTS idx_knowledge_wiki_projections_page_type
    ON knowledge_wiki_projections (workspace_id, page_type)
    WHERE page_type IS NOT NULL;

-- MT-243 fan-out lookup: which pages cite a changed source. jsonb_path_ops
-- supports the @> containment probe on the cited_sources array.
CREATE INDEX IF NOT EXISTS idx_knowledge_wiki_projections_cited_sources
    ON knowledge_wiki_projections
    USING GIN ((compile_stamp -> 'cited_sources') jsonb_path_ops)
    WHERE compile_stamp IS NOT NULL;
