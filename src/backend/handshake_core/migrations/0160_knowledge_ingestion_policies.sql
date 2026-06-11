-- WP-KERNEL-009 MT-081 SourceIngestionAndEvidence-081-ProjectRootAllowlist.
-- Master Spec anchor: 2.3.13.11 KnowledgeSource ("a registered project root
-- ... with stable source id, ... provenance, permission scope") + WP-009
-- constraint "Source ingestion must be allowlisted, hash-based, restartable,
-- and secret-aware".
--
-- Two tables:
--
-- knowledge_ingestion_root_policies is the WORKSPACE-level registration
-- allowlist: which repo-relative paths may become knowledge_source_roots at
-- all. It is the runtime-enforcement layer ABOVE the per-root file allowlist
-- that already lives in knowledge_source_roots.allowlist_policy (0131):
--   * 0160 policy: "may this path be registered/indexed as a root?"
--   * 0131 policy: "which files inside an approved root are eligible?"
-- Typed shape (mirrored by knowledge_ingestion::allowlist::RootRegistrationPolicy):
-- allow patterns (globs over repo-relative POSIX paths), deny patterns
-- (deny wins), and an operator-approval flag for registrations that must be
-- explicitly waved through by the operator.
--
-- knowledge_ingestion_policy_decisions is the durable decision receipt for
-- EVERY evaluation (allowed and denied): a no-context model can replay why a
-- root was accepted or rejected without chat history. Decisions carry an
-- EventLedger receipt ref so the evidence is replayable kernel state, never
-- prose (FK into kernel_event_ledger).

CREATE TABLE IF NOT EXISTS knowledge_ingestion_root_policies (
    policy_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    policy_version INTEGER NOT NULL DEFAULT 1 CHECK (policy_version >= 1),
    -- Glob patterns over normalized repo-relative POSIX paths.
    allow_patterns JSONB NOT NULL DEFAULT '["**"]'::jsonb,
    deny_patterns JSONB NOT NULL DEFAULT '[]'::jsonb,
    require_operator_approval BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_root_policies_id
        CHECK (policy_id ~ '^KIP-[0-9a-f]{32}$'),
    -- Pattern payloads must be JSON arrays (typed policy, not free prose).
    CONSTRAINT chk_knowledge_ingestion_root_policies_allow_shape
        CHECK (jsonb_typeof(allow_patterns) = 'array'),
    CONSTRAINT chk_knowledge_ingestion_root_policies_deny_shape
        CHECK (jsonb_typeof(deny_patterns) = 'array')
);

-- At most one ACTIVE policy per workspace; superseded policies stay as rows
-- (active = FALSE) so old decisions keep their FK context.
CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_ingestion_root_policies_active
    ON knowledge_ingestion_root_policies (workspace_id) WHERE active;

CREATE TABLE IF NOT EXISTS knowledge_ingestion_policy_decisions (
    decision_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- NULL policy_id = the built-in default policy was evaluated.
    policy_id TEXT REFERENCES knowledge_ingestion_root_policies(policy_id) ON DELETE SET NULL,
    candidate_path TEXT NOT NULL,
    root_kind TEXT NOT NULL CHECK (root_kind IN (
        'project_repo', 'governance', 'artifacts', 'media_library',
        'external_import', 'operator_folder'
    )),
    verdict TEXT NOT NULL CHECK (verdict IN (
        'allowed', 'denied_pattern', 'denied_not_allowlisted',
        'denied_requires_approval'
    )),
    -- The deny/allow glob that decided the verdict, when one did.
    matched_pattern TEXT,
    operator_approved BOOLEAN NOT NULL DEFAULT FALSE,
    actor_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    -- EventLedger receipt: decision evidence is a replayable kernel event.
    receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    decided_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_policy_decisions_id
        CHECK (decision_id ~ '^KIPD-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_ingestion_policy_decisions_actor
        CHECK (btrim(actor_kind) = actor_kind AND actor_kind <> ''
               AND btrim(actor_id) = actor_id AND actor_id <> ''),
    -- Candidate paths obey the same portability boundary as roots: no drive
    -- letters, no rooted/UNC paths, no backslashes, no parent escapes.
    CONSTRAINT chk_knowledge_ingestion_policy_decisions_path_portable
        CHECK (
            btrim(candidate_path) = candidate_path
            AND candidate_path !~ '^[A-Za-z]:'
            AND candidate_path !~ '^[/\\]'
            AND candidate_path !~ '\\'
            AND candidate_path !~ '(^|/)\.\.(/|$)'
        )
);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_policy_decisions_workspace
    ON knowledge_ingestion_policy_decisions (workspace_id, decided_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_policy_decisions_verdict
    ON knowledge_ingestion_policy_decisions (verdict);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('ingestion_root_policies', 'knowledge_ingestion_root_policies',
     'KnowledgeSource', 'authority', '0160_knowledge_ingestion_policies.sql', 'MT-081'),
    ('ingestion_policy_decisions', 'knowledge_ingestion_policy_decisions',
     'KnowledgeSource', 'support', '0160_knowledge_ingestion_policies.sql', 'MT-081')
ON CONFLICT (family_key) DO NOTHING;
