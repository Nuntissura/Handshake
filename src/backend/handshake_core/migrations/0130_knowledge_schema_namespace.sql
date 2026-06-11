-- WP-KERNEL-009 MT-049 PostgresEventLedgerCore-049-KnowledgeSchemaNamespace.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11
-- "Project Knowledge Index and Rich Document Authority" [ADD v02.192].
--
-- NAMESPACE DECISION (authoritative for every WP-009 migration 0130-0149):
--
-- All ProjectKnowledgeIndex tables use the `knowledge_` table-name prefix in
-- the connection's active schema (search_path). WP-009 does NOT create a
-- dedicated PostgreSQL schema. Rationale:
--
--   1. FK integrity: WP-009 authority rows reference existing same-schema
--      authority tables (workspaces, documents, loom_blocks, assets,
--      kernel_event_ledger). A dedicated PG schema would force
--      schema-qualified FKs and break the established per-test isolated
--      schema pattern (src/storage/tests.rs swaps search_path per test and
--      runs the full migration chain inside that schema).
--   2. Repo convention: every existing Handshake domain namespaces by table
--      prefix in the active schema (loom_*, kb003_*, atelier_*, kernel_*).
--      Schema fingerprint tooling and conformance tests scan
--      `current_schema()` only.
--   3. Rollback/repair (MT-063): sqlx::migrate! applies migrations in the
--      connection search_path, which keeps the WP-009 down-files testable on
--      a scratch schema without touching any other schema.
--
-- Collision audit (2026-06-11): no migration 0001-0129 and no runtime
-- ensure-schema path creates a table starting with `knowledge_`.
--
-- knowledge_schema_registry is the namespace's machine-readable boundary
-- record: every WP-009 migration registers the table families it adds, with
-- its migration file and authority class. Validators and the fail-closed
-- guard (MT-064) read this registry to know which tables form the
-- ProjectKnowledgeIndex authority surface. Projections are registered as
-- 'projection' and MUST NEVER be treated as authority (spec 2.3.13.11).

CREATE TABLE IF NOT EXISTS knowledge_schema_registry (
    family_key TEXT PRIMARY KEY,
    table_name TEXT NOT NULL UNIQUE,
    record_family TEXT NOT NULL,
    authority_class TEXT NOT NULL
        CHECK (authority_class IN ('authority', 'projection', 'support')),
    migration_file TEXT NOT NULL,
    wp_id TEXT NOT NULL DEFAULT 'WP-KERNEL-009',
    mt_id TEXT NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- The namespace boundary itself: every WP-009 table is `knowledge_`-prefixed.
    CONSTRAINT chk_knowledge_schema_registry_prefix
        CHECK (table_name LIKE 'knowledge\_%' ESCAPE '\'),
    CONSTRAINT chk_knowledge_schema_registry_family_key
        CHECK (btrim(family_key) = family_key AND family_key <> ''),
    CONSTRAINT chk_knowledge_schema_registry_mt_id
        CHECK (mt_id ~ '^MT-[0-9]{3}$')
);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('schema_registry', 'knowledge_schema_registry', 'Support',
     'support', '0130_knowledge_schema_namespace.sql', 'MT-049')
ON CONFLICT (family_key) DO NOTHING;
