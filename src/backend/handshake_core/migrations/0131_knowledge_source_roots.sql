-- WP-KERNEL-009 MT-050 PostgresEventLedgerCore-050-ProjectSourceRootTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeSource ("a registered project root,
-- file, asset, ... with stable source id, ... provenance, permission scope").
--
-- knowledge_source_roots persists the managed project roots Handshake is
-- allowed to index: the allowlist policy, the owning workspace, path
-- portability metadata, and indexing eligibility.
--
-- Path portability ([GLOBAL-PORTABILITY], spec portability rules): a root is
-- addressed by a normalized repo-relative POSIX path. Absolute path
-- authority is REJECTED at the database level (no drive letters, no leading
-- slash, no UNC prefix, no parent-dir escapes) so a moved project keeps its
-- knowledge index intact. Machine-local anchoring (which disk the repo lives
-- on today) is runtime configuration, never authority.

CREATE TABLE IF NOT EXISTS knowledge_source_roots (
    root_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    root_kind TEXT NOT NULL CHECK (root_kind IN (
        'project_repo', 'governance', 'artifacts', 'media_library',
        'external_import', 'operator_folder'
    )),
    -- Normalized repo-relative POSIX path ('' = the repo root itself).
    repo_relative_path TEXT NOT NULL,
    -- How repo_relative_path was normalized; v1 token is fixed so future
    -- normalizers can migrate rows knowingly.
    path_normalization TEXT NOT NULL DEFAULT 'repo_relative_posix_v1'
        CHECK (path_normalization = 'repo_relative_posix_v1'),
    -- Allowlist policy: {"include": [glob...], "exclude": [glob...]}.
    allowlist_policy JSONB NOT NULL DEFAULT '{"include": ["**/*"], "exclude": []}'::jsonb,
    indexing_eligibility TEXT NOT NULL DEFAULT 'eligible'
        CHECK (indexing_eligibility IN ('eligible', 'paused', 'excluded')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_source_roots_root_id
        CHECK (root_id ~ '^KSR-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_source_roots_display_name
        CHECK (btrim(display_name) = display_name AND display_name <> ''),
    -- Portability boundary: repo-relative POSIX only. Rejects
    --   * drive letters / Windows absolute ('C:...'),
    --   * leading slash or backslash (rooted paths, UNC),
    --   * backslash separators (must be normalized to '/'),
    --   * parent-directory escapes ('..' segments),
    --   * surrounding whitespace.
    CONSTRAINT chk_knowledge_source_roots_path_portable
        CHECK (
            btrim(repo_relative_path) = repo_relative_path
            AND repo_relative_path !~ '^[A-Za-z]:'
            AND repo_relative_path !~ '^[/\\]'
            AND repo_relative_path !~ '\\'
            AND repo_relative_path !~ '(^|/)\.\.(/|$)'
        ),
    CONSTRAINT uq_knowledge_source_roots_workspace_path
        UNIQUE (workspace_id, repo_relative_path)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_source_roots_workspace
    ON knowledge_source_roots (workspace_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_source_roots_eligibility
    ON knowledge_source_roots (indexing_eligibility);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('source_roots', 'knowledge_source_roots', 'KnowledgeSource',
     'authority', '0131_knowledge_source_roots.sql', 'MT-050')
ON CONFLICT (family_key) DO NOTHING;
