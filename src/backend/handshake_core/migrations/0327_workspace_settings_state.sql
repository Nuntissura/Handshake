-- WP-KERNEL-009 MT-248 durable workspace settings state.
-- Settings, themes, and app keybindings are workspace-scoped support state
-- and must round-trip through PostgreSQL with an EventLedger receipt.

CREATE TABLE IF NOT EXISTS knowledge_workspace_settings_states (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    settings_state JSONB NOT NULL CHECK (
        jsonb_typeof(settings_state) = 'object'
        AND settings_state ->> 'schema_id' = 'hsk.workspace_settings_state@1'
    ),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_knowledge_workspace_settings_states_event
    ON knowledge_workspace_settings_states (event_ledger_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('workspace_settings_state', 'knowledge_workspace_settings_states',
     'WorkspaceSettingsState', 'support',
     '0327_workspace_settings_state.sql', 'MT-248')
ON CONFLICT (family_key) DO NOTHING;
