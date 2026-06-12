-- WP-KERNEL-009 MT-194 UserManualStorageModel.
--
-- Master Spec anchors:
--   * 10.15.8 "UserManual migration bridge" — UserManual is the canonical
--     product concept for operator/model operation guidance; a UserManualRecord
--     MUST explain purpose, workflows, startup, run commands, expected
--     inputs/outputs, navigation, safety constraints, common failure modes,
--     recovery steps, and visual-debug/backend-navigation hooks.
--   * 2.3.13.11 `UserManualRecord` — PostgreSQL/EventLedger authority; manual
--     pages are durable product state, never markdown-vault authority.
--   * 12.7 `PRIM-UserManual` — coverage required for every WP-009 surface.
--
-- Authority model:
--   * user_manual_pages           — the UserManualRecord rows (product-level,
--                                   not workspace-scoped: the manual documents
--                                   the product itself).
--   * user_manual_sections        — ordered typed sections inside a page
--                                   (purpose / startup / failure_modes / ...).
--   * user_manual_anchors         — joins a page to the REAL surfaces it
--                                   documents (HTTP routes, Tauri commands,
--                                   IPC channels, spec anchors, event types).
--                                   The MT-195 build-update gate and the
--                                   MT-204 freshness check run over these.
--   * user_manual_tool_entries    — the machine-readable tool/command catalog
--                                   (MT-197), including rows imported from the
--                                   legacy static ModelManual manifest.
--   * user_manual_feature_entries — feature-group rows over tool entries.
--   * user_manual_versions        — version metadata per seeded corpus
--                                   (MT-194 version metadata requirement).
--   * user_manual_legacy_aliases  — MT-193/MT-203 deterministic legacy-name
--                                   mapping (spec 10.15.8 bridge law).
--
-- Receipts: seed/resync writes append KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED
-- events; receipt ids land in ledger_event_id columns (FK kernel_event_ledger,
-- created by migration 0018, matching every knowledge_* receipt column).

CREATE TABLE IF NOT EXISTS user_manual_pages (
    page_id TEXT PRIMARY KEY,
    -- Stable kebab-case slug; the model-facing lookup key.
    slug TEXT NOT NULL UNIQUE
        CHECK (btrim(slug) = slug AND slug <> '' AND slug = lower(slug)),
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    page_kind TEXT NOT NULL CHECK (page_kind IN (
        'purpose', 'workflow', 'tool_catalog', 'failure_recovery',
        'quickstart', 'state_recovery', 'surface_guide', 'navigation',
        'legacy_bridge', 'spec_enrichment_seed'
    )),
    audience TEXT NOT NULL DEFAULT 'model_and_operator' CHECK (audience IN (
        'model', 'operator', 'model_and_operator'
    )),
    -- Structured body: { "sections": [...] } mirror of user_manual_sections,
    -- kept for single-fetch reads; sections table is the queryable form.
    body JSONB NOT NULL,
    -- sha256 hex of the canonical body; the MT-204 freshness check compares
    -- this against the compiled-in seed hash.
    content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    manual_version TEXT NOT NULL CHECK (btrim(manual_version) <> ''),
    source_kind TEXT NOT NULL DEFAULT 'builtin_seed' CHECK (source_kind IN (
        'builtin_seed', 'runtime_edit'
    )),
    -- Master Spec anchors backing the page (JSON array of strings).
    spec_anchors JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'current' CHECK (status IN (
        'current', 'deprecated', 'superseded'
    )),
    superseded_by_slug TEXT,
    ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_manual_pages_kind
    ON user_manual_pages (page_kind, status);

CREATE TABLE IF NOT EXISTS user_manual_sections (
    section_id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL
        REFERENCES user_manual_pages(page_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    section_kind TEXT NOT NULL CHECK (section_kind IN (
        'purpose', 'workflows', 'startup', 'run_commands', 'inputs_outputs',
        'navigation', 'safety', 'failure_modes', 'recovery', 'hooks',
        'examples', 'schema'
    )),
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    -- Markdown body (projection-grade prose for operators).
    body_md TEXT NOT NULL CHECK (btrim(body_md) <> ''),
    -- Optional machine-readable payload (schemas, command lists).
    body_json JSONB,
    UNIQUE (page_id, position)
);

CREATE INDEX IF NOT EXISTS idx_user_manual_sections_page
    ON user_manual_sections (page_id, position);

CREATE TABLE IF NOT EXISTS user_manual_anchors (
    anchor_id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL
        REFERENCES user_manual_pages(page_id) ON DELETE CASCADE,
    anchor_kind TEXT NOT NULL CHECK (anchor_kind IN (
        'http_route', 'tauri_command', 'ipc_channel', 'cli_command',
        'spec_anchor', 'test', 'event_type', 'primitive', 'page_link'
    )),
    -- The anchored value: route path, command id, spec anchor string,
    -- event type name, or target page slug (page_link).
    anchor_value TEXT NOT NULL CHECK (btrim(anchor_value) <> ''),
    -- For http_route anchors: GET/POST/PUT/DELETE/PATCH; empty otherwise.
    http_method TEXT NOT NULL DEFAULT '' CHECK (http_method IN (
        '', 'GET', 'POST', 'PUT', 'DELETE', 'PATCH'
    )),
    UNIQUE (page_id, anchor_kind, anchor_value, http_method)
);

CREATE INDEX IF NOT EXISTS idx_user_manual_anchors_value
    ON user_manual_anchors (anchor_kind, anchor_value);

CREATE TABLE IF NOT EXISTS user_manual_tool_entries (
    tool_id TEXT PRIMARY KEY CHECK (btrim(tool_id) = tool_id AND tool_id <> ''),
    -- Optional owning page (a tool can be documented before its page lands,
    -- but the MT-195 gate requires coverage for registry surfaces).
    page_id TEXT
        REFERENCES user_manual_pages(page_id) ON DELETE SET NULL,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    status TEXT NOT NULL CHECK (status IN ('wired', 'planned', 'deprecated')),
    ipc_channel TEXT,
    tauri_command TEXT,
    cli_flag TEXT,
    http_route TEXT,
    http_method TEXT NOT NULL DEFAULT '' CHECK (http_method IN (
        '', 'GET', 'POST', 'PUT', 'DELETE', 'PATCH'
    )),
    description TEXT NOT NULL CHECK (btrim(description) <> ''),
    expected_input TEXT NOT NULL CHECK (btrim(expected_input) <> ''),
    expected_output TEXT NOT NULL CHECK (btrim(expected_output) <> ''),
    schema_fields JSONB NOT NULL DEFAULT '[]'::jsonb,
    common_errors JSONB NOT NULL DEFAULT '[]'::jsonb,
    recovery_steps JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Provenance: 'legacy_model_manual' for rows imported from the static
    -- manifest (MT-193 deterministic mapping), 'wp009_surface' for new rows.
    origin TEXT NOT NULL CHECK (origin IN ('legacy_model_manual', 'wp009_surface')),
    content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    manual_version TEXT NOT NULL CHECK (btrim(manual_version) <> ''),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_manual_tool_entries_route
    ON user_manual_tool_entries (http_route, http_method);

CREATE TABLE IF NOT EXISTS user_manual_feature_entries (
    feature_id TEXT PRIMARY KEY CHECK (btrim(feature_id) <> ''),
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    description TEXT NOT NULL CHECK (btrim(description) <> ''),
    -- JSON array of tool_id strings; integrity enforced by the store + tests
    -- (no per-element FK over JSONB).
    tool_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    origin TEXT NOT NULL CHECK (origin IN ('legacy_model_manual', 'wp009_surface')),
    content_hash TEXT NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    manual_version TEXT NOT NULL CHECK (btrim(manual_version) <> ''),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_manual_versions (
    manual_version TEXT PRIMARY KEY CHECK (btrim(manual_version) <> ''),
    seeded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- sha256 hex over the whole canonical seed corpus.
    seed_content_hash TEXT NOT NULL CHECK (seed_content_hash ~ '^[0-9a-f]{64}$'),
    page_count INTEGER NOT NULL CHECK (page_count >= 0),
    tool_count INTEGER NOT NULL CHECK (tool_count >= 0),
    feature_count INTEGER NOT NULL CHECK (feature_count >= 0),
    ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    note TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS user_manual_legacy_aliases (
    alias TEXT PRIMARY KEY CHECK (btrim(alias) <> ''),
    alias_kind TEXT NOT NULL CHECK (alias_kind IN (
        'module', 'tauri_command', 'ipc_channel', 'projection', 'test', 'constant'
    )),
    canonical_kind TEXT NOT NULL CHECK (canonical_kind IN ('page', 'tool', 'route')),
    canonical_ref TEXT NOT NULL CHECK (btrim(canonical_ref) <> ''),
    deprecation_note TEXT NOT NULL CHECK (btrim(deprecation_note) <> ''),
    manual_version TEXT NOT NULL CHECK (btrim(manual_version) <> ''),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
