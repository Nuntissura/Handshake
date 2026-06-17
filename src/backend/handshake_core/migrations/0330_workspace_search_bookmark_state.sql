-- WP-KERNEL-009 MT-258 durable workspace search bookmarks.
-- Saved searches were a UI-only artifact in localStorage; that is a dropped
-- capability under the Handshake-native durability rule (PostgreSQL + EventLedger
-- is canonical, UI artifacts are projections). Saved searches are workspace-scoped
-- support state and must round-trip through PostgreSQL with an EventLedger receipt,
-- mirroring knowledge_workspace_settings_states (MT-248).

CREATE TABLE IF NOT EXISTS knowledge_workspace_search_bookmark_states (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    bookmark_state JSONB NOT NULL CHECK (
        jsonb_typeof(bookmark_state) = 'object'
        AND bookmark_state ->> 'schema_id' = 'hsk.workspace_search_bookmark_state@1'
        AND jsonb_typeof(bookmark_state -> 'bookmarks') = 'array'
    ),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_knowledge_workspace_search_bookmark_states_event
    ON knowledge_workspace_search_bookmark_states (event_ledger_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('workspace_search_bookmark_state', 'knowledge_workspace_search_bookmark_states',
     'WorkspaceSearchBookmarkState', 'support',
     '0330_workspace_search_bookmark_state.sql', 'MT-258')
ON CONFLICT (family_key) DO NOTHING;
