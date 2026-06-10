-- WP-KERNEL-005 MT-149 EditableSurface live-authority stores.
-- The self-improvement loop's two allow-listed editable surfaces persist
-- their live authority values here:
--   * ModelManual capsule section text (one row per manual section)
--   * RetrievalPolicy parameters (top_k / capsule_budget_bytes per task type)
-- `promote` on the PG-backed surface providers is the only writer; sandbox
-- proposals never touch these tables.

CREATE TABLE IF NOT EXISTS atelier_model_manual_section (
    section_id      TEXT PRIMARY KEY,
    section_text    TEXT NOT NULL,
    revision        BIGINT NOT NULL DEFAULT 1,
    updated_by      TEXT NOT NULL,
    updated_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_model_manual_section_id_trimmed
        CHECK (section_id = btrim(section_id) AND section_id <> ''),
    CONSTRAINT chk_atelier_model_manual_section_text_not_empty
        CHECK (btrim(section_text) <> ''),
    CONSTRAINT chk_atelier_model_manual_section_text_cap
        CHECK (octet_length(section_text) <= 1048576),
    CONSTRAINT chk_atelier_model_manual_section_revision_positive
        CHECK (revision >= 1),
    CONSTRAINT chk_atelier_model_manual_section_updated_by_trimmed
        CHECK (updated_by = btrim(updated_by) AND updated_by <> '')
);

CREATE TABLE IF NOT EXISTS atelier_retrieval_policy (
    task_type       TEXT NOT NULL,
    parameter       TEXT NOT NULL,
    value           BIGINT NOT NULL,
    updated_by      TEXT NOT NULL,
    updated_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (task_type, parameter),
    CONSTRAINT chk_atelier_retrieval_policy_task_type
        CHECK (task_type IN (
            'validator_hbr_test_packet', 'kernel_builder_mt_implementation',
            'integration_validator_batch_review', 'operator_triage',
            'swarm_harness_session', 'process_ledger_inspection',
            'self_improvement_loop_eval', 'general_retrieval'
        )),
    CONSTRAINT chk_atelier_retrieval_policy_parameter
        CHECK (parameter IN ('top_k', 'capsule_budget_bytes')),
    CONSTRAINT chk_atelier_retrieval_policy_value_positive
        CHECK (value > 0),
    CONSTRAINT chk_atelier_retrieval_policy_top_k_cap
        CHECK (parameter <> 'top_k' OR value <= 64),
    CONSTRAINT chk_atelier_retrieval_policy_budget_cap
        CHECK (parameter <> 'capsule_budget_bytes' OR value <= 1048576),
    CONSTRAINT chk_atelier_retrieval_policy_updated_by_trimmed
        CHECK (updated_by = btrim(updated_by) AND updated_by <> '')
);
