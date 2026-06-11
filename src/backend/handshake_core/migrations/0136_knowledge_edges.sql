-- WP-KERNEL-009 MT-054 PostgresEventLedgerCore-054-KnowledgeEdgeTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeEdge ("a typed relationship between
-- entities or sources. Every edge MUST carry source span refs, extractor
-- version, lifecycle state, confidence, and stable relationship_id.").
--
-- STABLE relationship_id DERIVATION (authoritative copy lives in
-- storage/knowledge.rs::derive_knowledge_relationship_id):
--
--   relationship_id = 'KREL-' || sha256_hex(
--       'knowledge_edge_relationship_v2'
--       || '|' || len(edge_type)   || ':' || edge_type
--       || '|' || len(source_kind) || ':' || source_kind
--       || '|' || len(source_key)  || ':' || source_key
--       || '|' || len(target_kind) || ':' || target_kind
--       || '|' || len(target_key)  || ':' || target_key)
--
-- where len(x) is the UTF-8 byte length of x. Every component is
-- LENGTH-PREFIXED (`{len}:{value}`) so the framing is injective: entity_key is
-- free text under a single non-empty CHECK and legitimately contains `:` and
-- `|` (file paths, FQNs, spec anchors), so a plain delimiter-joined preimage
-- (the prior _v1 scheme) was collision-prone — two distinct (kind,key) tuples
-- could alias onto one relationship_id and silently merge under
-- uq_knowledge_edges_relationship. Length-prefixing makes that impossible: no
-- choice of separators inside a key can reproduce another tuple's preimage.
--
-- The hash input uses the entities' stable natural identities
-- (entity_kind + entity_key, MT-053), NEVER volatile row ids, timestamps, or
-- run ids — so the same logical relationship re-extracted by any later index
-- run derives the same relationship_id. Uniqueness is enforced per workspace
-- (uq_knowledge_edges_relationship); re-extraction upserts in place.
--
-- REQUIRED span evidence: knowledge_edge_spans carries the citeable evidence
-- (MT-055 spans). A deferred constraint trigger rejects, at COMMIT time, any
-- edge without at least one span row, and a delete guard prevents stripping
-- the last span from an existing edge. Application code inserts edge + spans
-- in one transaction (KnowledgeStore::upsert_knowledge_edge).

CREATE TABLE IF NOT EXISTS knowledge_edges (
    edge_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    relationship_id TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK (edge_type IN (
        'defines', 'references', 'contains', 'depends_on', 'implements',
        'documents', 'validates', 'derived_from', 'mentions', 'links_to',
        'supersedes', 'relates_to'
    )),
    source_entity_id TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    target_entity_id TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    extractor_version TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('proposed', 'active', 'conflicted', 'retired')),
    confidence DOUBLE PRECISION NOT NULL
        CHECK (confidence >= 0.0 AND confidence <= 1.0),
    -- Conflict marker: NULL when unconflicted; {"reason": ..., "with": [...]}.
    conflict_marker JSONB,
    created_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    last_seen_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_edges_id CHECK (edge_id ~ '^KED-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_edges_relationship_id
        CHECK (relationship_id ~ '^KREL-[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_edges_extractor_version
        CHECK (btrim(extractor_version) = extractor_version AND extractor_version <> ''),
    CONSTRAINT chk_knowledge_edges_conflict_shape
        CHECK (lifecycle_state <> 'conflicted' OR conflict_marker IS NOT NULL),
    CONSTRAINT uq_knowledge_edges_relationship
        UNIQUE (workspace_id, relationship_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_source_entity
    ON knowledge_edges (source_entity_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_target_entity
    ON knowledge_edges (target_entity_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_type
    ON knowledge_edges (workspace_id, edge_type);

-- REQUIRED evidence span refs for every edge.
CREATE TABLE IF NOT EXISTS knowledge_edge_spans (
    edge_id TEXT NOT NULL
        REFERENCES knowledge_edges(edge_id) ON DELETE CASCADE,
    span_id TEXT NOT NULL
        REFERENCES knowledge_spans(span_id) ON DELETE RESTRICT,
    recorded_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (edge_id, span_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_edge_spans_span
    ON knowledge_edge_spans (span_id);

-- Commit-time guard: every inserted edge must carry >= 1 evidence span.
CREATE OR REPLACE FUNCTION knowledge_edge_requires_span() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM knowledge_edge_spans WHERE edge_id = NEW.edge_id
    ) THEN
        RAISE EXCEPTION
            'knowledge_edges % violates spec 2.3.13.11: every edge MUST carry source span refs (knowledge_edge_spans is empty at commit)',
            NEW.edge_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE CONSTRAINT TRIGGER trg_knowledge_edge_requires_span
    AFTER INSERT ON knowledge_edges
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION knowledge_edge_requires_span();

-- Delete guard: the last evidence span of a surviving edge cannot be removed.
CREATE OR REPLACE FUNCTION knowledge_edge_span_delete_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM knowledge_edges WHERE edge_id = OLD.edge_id)
       AND NOT EXISTS (
           SELECT 1 FROM knowledge_edge_spans WHERE edge_id = OLD.edge_id
       ) THEN
        RAISE EXCEPTION
            'knowledge_edge_spans delete would strip the last evidence span from edge %; retire the edge instead',
            OLD.edge_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN OLD;
END $$;

CREATE CONSTRAINT TRIGGER trg_knowledge_edge_span_delete_guard
    AFTER DELETE ON knowledge_edge_spans
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION knowledge_edge_span_delete_guard();

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('edges', 'knowledge_edges', 'KnowledgeEdge',
     'authority', '0136_knowledge_edges.sql', 'MT-054'),
    ('edge_spans', 'knowledge_edge_spans', 'KnowledgeEdge',
     'authority', '0136_knowledge_edges.sql', 'MT-054')
ON CONFLICT (family_key) DO NOTHING;
