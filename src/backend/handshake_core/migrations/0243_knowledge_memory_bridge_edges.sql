-- WP-KERNEL-009 MT-124 BridgeEdgeGenerator.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 (KnowledgeEdge
-- MUST carry source span refs, extractor version, lifecycle, confidence, stable
-- relationship_id) and the WP-009 contract field
-- `translated_memory_system_spec.agent_jobs[BridgeEdgeJob]`:
--   "proposes graph bridge edges across fragmented subgraphs only when evidence
--    and hub-suppression rules allow."
--
-- A bridge edge is a normal knowledge_edges row (type 'relates_to', lifecycle
-- 'proposed', REQUIRED span evidence) produced to connect two entities that sit
-- in DIFFERENT connected components of the existing edge graph but co-occur in
-- evidence. The generator does NOT bypass the committed edge substrate: it
-- calls KnowledgeStore::upsert_knowledge_edge, so the deterministic
-- relationship_id, the >=1 span trigger, and the edge lifecycle all hold.
--
-- This table records the GENERATION DECISION for each candidate pair the
-- generator considered, including candidates it SUPPRESSED (hub suppression,
-- insufficient evidence, already-connected) and the reason. That decision log
-- is the auditable "why did/didn't a bridge appear" record; the edge it
-- produced (when accepted) is linked by edge_id.

CREATE TABLE IF NOT EXISTS knowledge_memory_bridge_decisions (
    decision_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The two entities the generator considered bridging (ordered by id so the
    -- same unordered pair is one decision).
    entity_id_a TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    entity_id_b TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    -- The outcome of the bridge evaluation.
    decision TEXT NOT NULL CHECK (decision IN (
        'bridged', 'suppressed_hub', 'suppressed_no_evidence', 'suppressed_connected'
    )),
    -- Degree of each endpoint at decision time (hub-suppression inputs).
    degree_a INTEGER NOT NULL CHECK (degree_a >= 0),
    degree_b INTEGER NOT NULL CHECK (degree_b >= 0),
    hub_degree_threshold INTEGER NOT NULL CHECK (hub_degree_threshold >= 1),
    -- The shared evidence span that justified the bridge (NULL when suppressed
    -- for no evidence).
    evidence_span_id TEXT
        REFERENCES knowledge_spans(span_id) ON DELETE SET NULL,
    -- The bridge edge produced (NULL unless decision = 'bridged').
    bridge_edge_id TEXT
        REFERENCES knowledge_edges(edge_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kmbd_id CHECK (decision_id ~ '^KBR-[0-9a-f]{32}$'),
    CONSTRAINT chk_kmbd_distinct CHECK (entity_id_a <> entity_id_b),
    -- A 'bridged' decision MUST name the edge it produced and its evidence span.
    CONSTRAINT chk_kmbd_bridged_shape CHECK (
        decision <> 'bridged'
        OR (bridge_edge_id IS NOT NULL AND evidence_span_id IS NOT NULL)
    ),
    -- A hub/connected suppression still recorded its evidence span (it had one);
    -- a no-evidence suppression must NOT name an edge.
    CONSTRAINT chk_kmbd_suppressed_no_edge CHECK (
        decision = 'bridged' OR bridge_edge_id IS NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_kmbd_workspace
    ON knowledge_memory_bridge_decisions (workspace_id, decision);

CREATE INDEX IF NOT EXISTS idx_kmbd_edge
    ON knowledge_memory_bridge_decisions (bridge_edge_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('memory_bridge_decisions', 'knowledge_memory_bridge_decisions',
     'BridgeEdgeJob', 'authority', '0243_knowledge_memory_bridge_edges.sql', 'MT-124')
ON CONFLICT (family_key) DO NOTHING;
