-- WP-KERNEL-009 MT-246 durable workbench layout state.
-- The UI workbench layout is projection/support state, but it must restore
-- through PostgreSQL and retain an EventLedger receipt rather than relying on
-- localStorage or process memory.

CREATE TABLE IF NOT EXISTS knowledge_workbench_layout_states (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    layout_state JSONB NOT NULL CHECK (
        jsonb_typeof(layout_state) = 'object'
        AND layout_state ->> 'schema_id' = 'hsk.workbench_layout_state@1'
    ),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_knowledge_workbench_layout_states_event
    ON knowledge_workbench_layout_states (event_ledger_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('workbench_layout_state', 'knowledge_workbench_layout_states',
     'WorkbenchLayoutState', 'support',
     '0323_workbench_layout_state.sql', 'MT-246')
ON CONFLICT (family_key) DO NOTHING;
