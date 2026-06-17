-- WP-KERNEL-009 MT-256 QuickSwitcher durable recents.
-- Recents are support state for the ProjectKnowledgeIndex/Loom navigation
-- workflow: PostgreSQL is the durable store and every update retains a typed
-- Kernel EventLedger receipt.

CREATE TABLE IF NOT EXISTS knowledge_quick_switcher_recents (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    hit_key TEXT NOT NULL,
    source_kind TEXT NOT NULL CHECK (
        source_kind IN (
            'loom_block',
            'symbol',
            'work_packet',
            'micro_task',
            'user_manual_page'
        )
    ),
    ref_id TEXT NOT NULL CHECK (length(btrim(ref_id)) > 0),
    result_kind TEXT NOT NULL CHECK (
        result_kind IN (
            'loom_block',
            'knowledge_entity',
            'user_manual_page'
        )
    ),
    title TEXT NOT NULL CHECK (length(btrim(title)) > 0),
    excerpt TEXT NOT NULL DEFAULT '',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    selected_count BIGINT NOT NULL DEFAULT 1 CHECK (selected_count >= 1),
    selected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    PRIMARY KEY (workspace_id, hit_key),
    CONSTRAINT chk_knowledge_quick_switcher_recents_hit_key
        CHECK (hit_key = source_kind || ':' || ref_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_quick_switcher_recents_workspace_selected
    ON knowledge_quick_switcher_recents (workspace_id, selected_at DESC, hit_key ASC);

CREATE INDEX IF NOT EXISTS idx_knowledge_quick_switcher_recents_event
    ON knowledge_quick_switcher_recents (event_ledger_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('quick_switcher_recents', 'knowledge_quick_switcher_recents',
     'QuickSwitcherRecent', 'support',
     '0322_quick_switcher_recents.sql', 'MT-256')
ON CONFLICT (family_key) DO NOTHING;
