-- WP-KERNEL-009 MT-070 CRDTAndConcurrencyCore-070-ConcurrentEditorSaveSemantics.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "AI edit
-- proposals, graph mutation proposals, ... and manual edits MUST leave actor,
-- source span, state-vector, validation, denial, or promotion receipts."
--
-- knowledge_crdt_denial_receipts is the single durable home for WP-009 CRDT
-- denial receipts: stale/concurrent draft saves (MT-070), lease write
-- denials (MT-076), index-run slot rejections (MT-071), and proposal
-- promotion denials (MT-069/MT-074). A denied write is never silent: it
-- leaves this row plus a paired EventLedger event (FK below), and the
-- conflict UI state (MT-075) is computed from these rows.
--
-- Numbering: WP-009 CRDTAndConcurrencyCore owns migrations 0150-0159.

CREATE TABLE IF NOT EXISTS knowledge_crdt_denial_receipts (
    receipt_id TEXT PRIMARY KEY,
    receipt_kind TEXT NOT NULL CHECK (receipt_kind IN (
        'stale_draft_save',
        'concurrent_draft_fork',
        'ahead_of_head_save',
        'update_content_mismatch',
        'sequence_slot_race',
        'lease_write_denied',
        'index_run_slot_rejected',
        'graph_promotion_denied',
        'ai_edit_promotion_denied'
    )),
    workspace_id TEXT NOT NULL CHECK (btrim(workspace_id) <> ''),
    -- Draft-document denials carry the document pair; lease/proposal denials
    -- may target a non-document scope and leave these NULL.
    document_id TEXT,
    crdt_document_id TEXT,
    -- Generic typed target of the denied write, e.g.
    -- 'crdt_document:<id>', 'lease_scope:source_root:<id>', 'proposal:<id>'.
    scope_ref TEXT NOT NULL CHECK (btrim(scope_ref) <> ''),
    -- The actor whose write was denied (canonical MT-065 form `kind:ident`).
    actor_id TEXT NOT NULL CHECK (actor_id ~ '^(operator|local_model|cloud_model|validator|system):[A-Za-z0-9._-]+$'),
    actor_kind TEXT NOT NULL CHECK (actor_kind IN (
        'operator', 'local_model', 'cloud_model', 'validator', 'system'
    )),
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    correlation_id TEXT NOT NULL CHECK (btrim(correlation_id) <> ''),
    -- Typed denial reason payload (serialized Rust enum, schema-stamped).
    denial_payload JSONB NOT NULL,
    -- Paired EventLedger receipt: denials are events, never just rows.
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_denial_receipts_id
        CHECK (receipt_id ~ '^KCDR-[0-9a-f]{32}$'),
    CONSTRAINT uq_knowledge_crdt_denial_receipts_idempotency
        UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_denial_receipts_document
    ON knowledge_crdt_denial_receipts (crdt_document_id, created_at);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_denial_receipts_scope
    ON knowledge_crdt_denial_receipts (scope_ref, created_at);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_denial_receipts_actor
    ON knowledge_crdt_denial_receipts (actor_id, created_at);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_denial_receipts', 'knowledge_crdt_denial_receipts', 'CrdtDenialReceipt',
     'support', '0150_knowledge_crdt_denial_receipts.sql', 'MT-070')
ON CONFLICT (family_key) DO NOTHING;
