-- WP-KERNEL-005 MT-160 / MT-163 / MT-169: typed Model-Workflow-Diagnostics
-- runtime surfaces (governed model config, apply state machine, synthetic-input
-- guard).
--
-- These are product/runtime tables for no-context models, never governance
-- markdown. Storage authority is PostgreSQL only (AtelierStore::pool());
-- SQLite is forbidden (MT-004). No local filesystem refs, no .GOV refs.
--
--   * atelier_model_config (MT-160): a governed local/remote OpenAI-compatible
--     model config (base_url, model, system_prompt, timeout). The api_key is
--     NEVER stored in plaintext: only a redacted ref (api_key_ref) is persisted,
--     and the secret never enters any EventLedger payload.
--   * atelier_model_apply (MT-163): the draft/preview/validate/apply/reject/
--     rollback state machine that gates model suggestions becoming product
--     changes. The CHECK constrains the state token set; the legal transition
--     graph is enforced in code (advance_apply_state).
--   * atelier_synthetic_input_guard (MT-169): preserves synthetic-input
--     operations (injectKey/injectMouse/clickElement/typeText) as governed,
--     attributed, auditable rows requiring authorization, so synthetic input is
--     never silent.
--
-- NOTE: this migration is intentionally NOT wired into ensure_schema; the
-- orchestrator wires it after the MT lands. CREATE TABLE IF NOT EXISTS keeps it
-- idempotent and safe once wired in.

CREATE TABLE IF NOT EXISTS atelier_model_config (
    config_id      TEXT PRIMARY KEY,
    base_url       TEXT NOT NULL,
    model          TEXT NOT NULL,
    -- Redacted handle to the api key. NEVER the raw secret.
    api_key_ref    TEXT NOT NULL,
    system_prompt  TEXT NOT NULL,
    timeout_ms     INTEGER NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_model_config_config_id CHECK (
        btrim(config_id) = config_id AND config_id <> ''
    ),
    CONSTRAINT chk_atelier_model_config_base_url CHECK (
        btrim(base_url) = base_url AND base_url <> ''
    ),
    CONSTRAINT chk_atelier_model_config_model CHECK (
        btrim(model) = model AND model <> ''
    ),
    CONSTRAINT chk_atelier_model_config_api_key_ref CHECK (
        btrim(api_key_ref) = api_key_ref AND api_key_ref <> ''
    ),
    CONSTRAINT chk_atelier_model_config_timeout CHECK (
        timeout_ms > 0
    )
);

CREATE TABLE IF NOT EXISTS atelier_model_apply (
    apply_id       TEXT PRIMARY KEY,
    suggestion_ref TEXT NOT NULL,
    state          TEXT NOT NULL,
    evidence_ref   TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_model_apply_apply_id CHECK (
        btrim(apply_id) = apply_id AND apply_id <> ''
    ),
    CONSTRAINT chk_atelier_model_apply_suggestion_ref CHECK (
        btrim(suggestion_ref) = suggestion_ref AND suggestion_ref <> ''
    ),
    CONSTRAINT chk_atelier_model_apply_state CHECK (
        state IN ('DRAFT', 'PREVIEW', 'VALIDATED', 'APPLIED', 'REJECTED', 'ROLLED_BACK')
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_model_apply_state
    ON atelier_model_apply(state, apply_id);

CREATE TABLE IF NOT EXISTS atelier_synthetic_input_guard (
    guard_id       TEXT PRIMARY KEY,
    op             TEXT NOT NULL,
    target_ref     TEXT NOT NULL,
    authorized     BOOLEAN NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_synthetic_input_guard_guard_id CHECK (
        btrim(guard_id) = guard_id AND guard_id <> ''
    ),
    CONSTRAINT chk_atelier_synthetic_input_guard_op CHECK (
        op IN ('INJECT_KEY', 'INJECT_MOUSE', 'CLICK_ELEMENT', 'TYPE_TEXT')
    ),
    CONSTRAINT chk_atelier_synthetic_input_guard_target_ref CHECK (
        btrim(target_ref) = target_ref AND target_ref <> ''
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_synthetic_input_guard_op
    ON atelier_synthetic_input_guard(op, guard_id);
