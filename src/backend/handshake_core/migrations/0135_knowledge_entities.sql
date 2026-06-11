-- WP-KERNEL-009 MT-053 PostgresEventLedgerCore-053-KnowledgeEntityTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeEntity ("a typed symbol, concept,
-- file, folder, project, person, role, task, API, schema, command, media
-- object, manual entry, or product primitive detected from one or more
-- spans"). MT-053 contract additionally names: spec topics, WPs, MTs,
-- taskboard rows, rich documents, Loom blocks, UserManual pages.
--
-- Identity model: an entity's stable natural identity is
-- (workspace_id, entity_kind, entity_key) where entity_key is the
-- kind-specific stable key (file path, symbol FQN, WP id, spec anchor, ...).
-- Re-index runs upsert on that identity, so entity_id stays stable across
-- runs and the deterministic relationship_id derivation (MT-054) can hash
-- entity (kind, key) pairs instead of volatile row ids.
--
-- Detection evidence: knowledge_entity_spans links every entity to the spans
-- it was detected from ("detected from one or more spans"), with the
-- detecting run for provenance.

CREATE TABLE IF NOT EXISTS knowledge_entities (
    entity_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    entity_kind TEXT NOT NULL CHECK (entity_kind IN (
        'symbol', 'concept', 'file', 'folder', 'project', 'person', 'role',
        'task', 'api', 'schema', 'command', 'media', 'manual_entry',
        'product_primitive', 'spec_topic', 'work_packet', 'micro_task',
        'taskboard_row', 'rich_document', 'loom_block', 'user_manual_page'
    )),
    -- Stable natural key within (workspace, kind): path, FQN, WP id, ...
    entity_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    -- {"extractor": ..., "extractor_version": ..., "method": ...}
    detection_provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'retired')),
    -- Optional primary source this entity is anchored to.
    primary_source_id TEXT
        REFERENCES knowledge_sources(source_id) ON DELETE SET NULL,
    first_detected_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    last_detected_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_entities_id CHECK (entity_id ~ '^KEN-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_entities_key
        CHECK (btrim(entity_key) = entity_key AND entity_key <> ''),
    CONSTRAINT chk_knowledge_entities_display_name
        CHECK (btrim(display_name) = display_name AND display_name <> ''),
    CONSTRAINT uq_knowledge_entities_identity
        UNIQUE (workspace_id, entity_kind, entity_key)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_workspace_kind
    ON knowledge_entities (workspace_id, entity_kind);

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_primary_source
    ON knowledge_entities (primary_source_id);

-- Detection evidence: which spans an entity was detected from.
CREATE TABLE IF NOT EXISTS knowledge_entity_spans (
    entity_id TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    span_id TEXT NOT NULL
        REFERENCES knowledge_spans(span_id) ON DELETE RESTRICT,
    detected_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, span_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_entity_spans_span
    ON knowledge_entity_spans (span_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('entities', 'knowledge_entities', 'KnowledgeEntity',
     'authority', '0135_knowledge_entities.sql', 'MT-053'),
    ('entity_spans', 'knowledge_entity_spans', 'KnowledgeEntity',
     'authority', '0135_knowledge_entities.sql', 'MT-053')
ON CONFLICT (family_key) DO NOTHING;
