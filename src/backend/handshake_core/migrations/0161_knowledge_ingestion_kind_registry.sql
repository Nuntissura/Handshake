-- WP-KERNEL-009 MT-082 SourceIngestionAndEvidence-082-SourceKindRegistry.
-- Master Spec anchor: 2.3.13.11 KnowledgeSource ("source kind") + projection
-- rule ("Generated ... projections only").
--
-- knowledge_ingestion_kind_registry is a PROJECTION of the code-authoritative
-- ingestion kind registry (src/knowledge_ingestion/kinds.rs). Extractors are
-- code, so capability truth lives in code; this table exists so validators,
-- APIs, and no-context models can read the active capability matrix
-- (per-kind span extraction, anchor kinds, partial-extraction support,
-- text-layer detection requirement, secret-scan requirement, extension/MIME
-- mapping) from durable state. It is synced by
-- knowledge_ingestion::kinds::sync_kind_projection and never hand-edited;
-- deleting it loses nothing (spec 2.3.13.11 projection law).

CREATE TABLE IF NOT EXISTS knowledge_ingestion_kind_registry (
    kind_key TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    capabilities JSONB NOT NULL,
    extensions JSONB NOT NULL DEFAULT '[]'::jsonb,
    mime_types JSONB NOT NULL DEFAULT '[]'::jsonb,
    registry_version TEXT NOT NULL,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_kind_registry_key
        CHECK (kind_key ~ '^[a-z][a-z0-9_]*$'),
    CONSTRAINT chk_knowledge_ingestion_kind_registry_caps_shape
        CHECK (jsonb_typeof(capabilities) = 'object'),
    CONSTRAINT chk_knowledge_ingestion_kind_registry_ext_shape
        CHECK (jsonb_typeof(extensions) = 'array'),
    CONSTRAINT chk_knowledge_ingestion_kind_registry_mime_shape
        CHECK (jsonb_typeof(mime_types) = 'array')
);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('ingestion_kind_registry', 'knowledge_ingestion_kind_registry',
     'KnowledgeSource', 'projection', '0161_knowledge_ingestion_kind_registry.sql', 'MT-082')
ON CONFLICT (family_key) DO NOTHING;
