-- WP-KERNEL-009 MT-140 SemanticCatalogBridge.
-- Master Spec anchor: 2.6.6.7.14.5 (SemanticCatalogEntry) + 02-system-
-- architecture.md Semantic Catalog anchors ("indexed backend routing
-- contracts") + the folded WP-1-Semantic-Catalog-v2 stub intent: the Semantic
-- Catalog "remains a deterministic backend routing contract for tools, data
-- surfaces, retrieval, and Spec Router planning" and "entries may not remain
-- hidden in prompts, helper labels, or UI-only descriptions". This table is the
-- backend-queryable representation that makes catalog entries first-class
-- authority rather than prompt-only helper text.
--
-- A catalog entry declares, for a named tool/index/view/entity_type, the query
-- routes it supports and the canonical selectors it understands, so the
-- retrieval planner can resolve a query to a route deterministically. Entries
-- are authority rows (PostgreSQL); generated routing hints are projections.

CREATE TABLE IF NOT EXISTS knowledge_semantic_catalog_entries (
    entry_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- entity_type | index | view | tool (spec SemanticCatalogEntry.kind).
    entry_kind TEXT NOT NULL CHECK (entry_kind IN (
        'entity_type', 'index', 'view', 'tool'
    )),
    -- Canonical name; (workspace, name, version) is unique so a name resolves
    -- to exactly one current contract.
    name TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    description TEXT NOT NULL DEFAULT '',
    -- Backend query routes this entry supports (spec route vocabulary):
    -- knowledge_graph | shadow_ws_lexical | shadow_ws_vector | bounded_read |
    -- sql_query. Stored as a JSON array of strings.
    query_routes JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Canonical selector names the entry understands (implementation-defined
    -- mapping); JSON array of strings.
    supported_selectors JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Optional partial default budgets (spec default_budgets); JSON object.
    default_budgets JSONB,
    -- Optional examples [{query, route_hint}]; JSON array.
    examples JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Lifecycle so a superseded contract can be retired without deletion.
    lifecycle_state TEXT NOT NULL DEFAULT 'active' CHECK (lifecycle_state IN (
        'active', 'retired'
    )),
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_semantic_catalog_name
        CHECK (btrim(name) = name AND name <> ''),
    CONSTRAINT chk_knowledge_semantic_catalog_routes_is_array
        CHECK (jsonb_typeof(query_routes) = 'array'),
    CONSTRAINT chk_knowledge_semantic_catalog_selectors_is_array
        CHECK (jsonb_typeof(supported_selectors) = 'array')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_knowledge_semantic_catalog_name_version
    ON knowledge_semantic_catalog_entries (workspace_id, name, version);

CREATE INDEX IF NOT EXISTS idx_knowledge_semantic_catalog_kind
    ON knowledge_semantic_catalog_entries (workspace_id, entry_kind, lifecycle_state);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('semantic_catalog_entries', 'knowledge_semantic_catalog_entries', 'Support',
     'authority', '0260_knowledge_semantic_catalog.sql', 'MT-140')
ON CONFLICT (family_key) DO NOTHING;
