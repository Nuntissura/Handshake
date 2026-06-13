-- WP-KERNEL-009 MT-221 Cloud fallback rules.
-- First-class PostgreSQL lifecycle rows for cloud assistance outputs. The
-- rows are support evidence only: cloud output stays reviewable and
-- non-authoritative. Promotion, when accepted, is recorded by a separate
-- authority surface rather than by mutating this support row.

CREATE TABLE IF NOT EXISTS knowledge_agent_cloud_assistance_receipts (
    receipt_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    claim_id TEXT NOT NULL REFERENCES knowledge_agent_worktree_claims(claim_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    handoff_id TEXT NOT NULL UNIQUE REFERENCES knowledge_agent_role_mailbox_handoffs(handoff_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    handoff_event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    cloud_assistance_event_id TEXT NOT NULL UNIQUE REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    fallback_basis_event_id TEXT NOT NULL UNIQUE REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    parent_session_id TEXT NOT NULL CHECK (length(btrim(parent_session_id)) > 0),
    prompt_sha256 TEXT NOT NULL CHECK (prompt_sha256 ~ '^[0-9a-f]{64}$'),
    lane_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    lane_kind TEXT NOT NULL CHECK (lane_kind = 'cloud'),
    provider TEXT NOT NULL CHECK (provider IN ('open_ai', 'anthropic', 'official_cli', 'other')),
    model_label TEXT NOT NULL CHECK (length(btrim(model_label)) > 0),
    attribution_jsonb JSONB NOT NULL,
    session_id TEXT NOT NULL,
    fallback_reason TEXT NOT NULL CHECK (
        fallback_reason IN (
            'local_failed',
            'local_low_confidence',
            'local_overloaded',
            'local_suppressed',
            'hard_reasoning',
            'force_cloud',
            'no_local_model'
        )
    ),
    output_kind TEXT NOT NULL CHECK (
        output_kind IN (
            'analysis',
            'patch_suggestion',
            'validation_summary',
            'handoff_summary'
        )
    ),
    output_sha256 TEXT NOT NULL CHECK (output_sha256 ~ '^[0-9a-f]{64}$'),
    body_sha256 TEXT NOT NULL CHECK (body_sha256 ~ '^[0-9a-f]{64}$'),
    output_text TEXT NOT NULL CHECK (length(btrim(output_text)) > 0),
    output_body_jsonb JSONB NOT NULL,
    target_ref TEXT NOT NULL CHECK (length(btrim(target_ref)) > 0),
    review_state TEXT NOT NULL CHECK (review_state IN ('pending_review', 'rejected')),
    non_authoritative BOOLEAN NOT NULL DEFAULT TRUE CHECK (non_authoritative IS TRUE),
    requires_promotion BOOLEAN NOT NULL DEFAULT TRUE CHECK (requires_promotion IS TRUE),
    authority_mutation_allowed BOOLEAN NOT NULL DEFAULT FALSE CHECK (authority_mutation_allowed IS FALSE),
    promotion_event_id TEXT REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_agent_cloud_assistance_receipts_id
        CHECK (receipt_id LIKE 'PSR-CLOUD-%'),
    CONSTRAINT chk_agent_cloud_assistance_receipts_promotion_ref
        CHECK (
            promotion_event_id IS NULL
        )
);

CREATE INDEX IF NOT EXISTS idx_agent_cloud_assistance_receipts_workspace
    ON knowledge_agent_cloud_assistance_receipts (workspace_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_agent_cloud_assistance_receipts_review
    ON knowledge_agent_cloud_assistance_receipts (review_state, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_agent_cloud_assistance_receipts_fallback_basis
    ON knowledge_agent_cloud_assistance_receipts (fallback_basis_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('parallel_swarm_cloud_assistance_receipts', 'knowledge_agent_cloud_assistance_receipts',
     'SwarmCloudAssistanceReceipt', 'support',
     '0314_parallel_swarm_cloud_assistance_receipts.sql', 'MT-221')
ON CONFLICT (family_key) DO NOTHING;
