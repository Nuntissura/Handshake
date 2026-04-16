-- WP-1-Distillation-v2 MT-002: Section 9.1.2 SQL schema
-- Skill Bank and distillation pipeline tables.
-- SQLITE_NOW_POSTGRES_READY: TEXT for UUIDs/timestamps, INTEGER for booleans.

-- ---------------------------------------------------------------------------
-- 1.2.1 Core Skill Bank tables
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS skill_log_entry (
    id                          TEXT PRIMARY KEY,   -- UUID
    version                     TEXT NOT NULL,
    created_at                  TEXT NOT NULL,       -- ISO-8601 UTC

    -- Session meta
    session_id                  TEXT NOT NULL,
    session_turn_index          INTEGER NOT NULL,
    task_id                     TEXT,
    user_id_hash                TEXT,
    workspace_id                TEXT,

    -- Task meta
    task_type                   TEXT NOT NULL,
    task_subtype                TEXT,
    task_language               TEXT,
    task_tags                   TEXT,                -- JSON array of strings
    task_request_summary        TEXT,

    -- Engine meta
    actor_role                  TEXT NOT NULL,       -- "student" | "teacher" | ...
    model_name                  TEXT NOT NULL,
    model_family                TEXT,
    model_revision              TEXT,
    provider                    TEXT,
    tokenizer_id                TEXT,
    tokenizer_family            TEXT,
    context_window_tokens       INTEGER,
    precision                   TEXT,
    inference_params            TEXT,                -- JSON object

    -- Context refs and snapshots
    context_refs_json           TEXT NOT NULL,       -- JSON object
    snapshots_input_json        TEXT NOT NULL,       -- ChatSnapshot JSON
    snapshots_output_raw_json   TEXT NOT NULL,
    snapshots_output_final_json TEXT,                -- nullable

    -- Quality meta
    quality_tag                 TEXT NOT NULL,       -- "good" | "bad" | ...
    thumb                       TEXT NOT NULL,       -- "up" | "down" | ...
    quality_score               REAL,
    quality_source              TEXT,
    quality_labels              TEXT,                -- JSON array
    auto_eval_json              TEXT NOT NULL,       -- tests, compile, security flags, scores
    user_edit_stats_json        TEXT NOT NULL,
    data_trust_score            REAL,                -- 0-1, nullable

    -- Auto-eval detail (duplicated for indexing)
    auto_style_score            REAL,
    auto_reasoning_score        REAL,
    auto_factuality_score       REAL,

    -- Telemetry
    latency_ms                  INTEGER,
    prompt_tokens               INTEGER,
    completion_tokens           INTEGER,
    total_tokens                INTEGER,
    truncation_occurred         INTEGER,             -- 0/1
    cache_hit                   INTEGER,             -- 0/1
    output_char_len             INTEGER,
    output_line_count           INTEGER,

    -- Environment
    handshake_version           TEXT,
    orchestrator_build          TEXT,
    git_commit                  TEXT,
    os                          TEXT,
    hardware_profile            TEXT,
    config_profile              TEXT,

    -- Privacy
    contains_secrets            INTEGER NOT NULL DEFAULT 0,
    pii_present                 INTEGER NOT NULL DEFAULT 0,
    can_export_off_device       INTEGER NOT NULL DEFAULT 0,
    redaction_applied           INTEGER NOT NULL DEFAULT 0,

    -- Reward / diagnostic features
    reward_features_json        TEXT                 -- JSON object
);

CREATE INDEX IF NOT EXISTS idx_skill_log_entry_session
    ON skill_log_entry (session_id, session_turn_index);

CREATE INDEX IF NOT EXISTS idx_skill_log_entry_quality
    ON skill_log_entry (quality_tag, thumb);

CREATE INDEX IF NOT EXISTS idx_skill_log_entry_privacy
    ON skill_log_entry (contains_secrets, pii_present, can_export_off_device);

CREATE INDEX IF NOT EXISTS idx_skill_log_entry_created_at
    ON skill_log_entry (created_at);

CREATE INDEX IF NOT EXISTS idx_skill_log_entry_task_type
    ON skill_log_entry (task_type, task_language);

-- Optional normalized file reference table
CREATE TABLE IF NOT EXISTS skill_log_file_ref (
    id              INTEGER PRIMARY KEY,
    log_entry_id    TEXT NOT NULL REFERENCES skill_log_entry(id) ON DELETE CASCADE,
    path            TEXT NOT NULL,
    hash            TEXT
);

CREATE INDEX IF NOT EXISTS idx_skill_log_file_ref_log
    ON skill_log_file_ref (log_entry_id);

-- ---------------------------------------------------------------------------
-- 1.2.2 Distillation jobs, examples, checkpoints, eval
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS distill_job (
    id              TEXT PRIMARY KEY,       -- UUID
    created_at      TEXT NOT NULL,
    status          TEXT NOT NULL,          -- "pending" | "running" | "completed" | "failed"
    description     TEXT,
    config_json     TEXT NOT NULL           -- adapter hyperparams, data filters, etc.
);

CREATE INDEX IF NOT EXISTS idx_distill_job_status
    ON distill_job (status, created_at);

CREATE TABLE IF NOT EXISTS distill_example (
    job_id          TEXT NOT NULL REFERENCES distill_job(id) ON DELETE CASCADE,
    log_entry_id    TEXT NOT NULL REFERENCES skill_log_entry(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,          -- "teacher" | "student"
    is_replay       INTEGER NOT NULL,      -- 0/1
    sample_weight   REAL NOT NULL,         -- typically = data_trust_score
    PRIMARY KEY (job_id, log_entry_id, role)
);

CREATE INDEX IF NOT EXISTS idx_distill_example_job
    ON distill_example (job_id, is_replay);

CREATE TABLE IF NOT EXISTS adapter_checkpoint (
    id                      TEXT PRIMARY KEY,       -- UUID
    created_at              TEXT NOT NULL,
    parent_checkpoint_id    TEXT REFERENCES adapter_checkpoint(id),

    base_model_name         TEXT NOT NULL,
    adapter_type            TEXT NOT NULL,          -- "lora" | "dora"
    rank_r                  INTEGER NOT NULL,
    alpha                   INTEGER NOT NULL,
    learning_rate           REAL NOT NULL,
    precision               TEXT NOT NULL,          -- e.g. "4bit-nf4"

    path                    TEXT NOT NULL,          -- filesystem path to adapter weights
    ewc_state_json          TEXT,                   -- Fisher diag, lambda, etc.
    eval_summary_json       TEXT,                   -- metrics on fixed eval suite

    -- Lineage / provenance
    data_signature          TEXT,                   -- hash of training data spec
    job_ids_json            TEXT,                   -- JSON array of distill_job ids

    is_approved             INTEGER NOT NULL DEFAULT 0,  -- passed gates
    is_current              INTEGER NOT NULL DEFAULT 0   -- currently served student
);

CREATE INDEX IF NOT EXISTS idx_adapter_checkpoint_current
    ON adapter_checkpoint (is_current);

CREATE INDEX IF NOT EXISTS idx_adapter_checkpoint_parent
    ON adapter_checkpoint (parent_checkpoint_id);

CREATE TABLE IF NOT EXISTS eval_run (
    id              TEXT PRIMARY KEY,
    job_id          TEXT REFERENCES distill_job(id),
    checkpoint_id   TEXT REFERENCES adapter_checkpoint(id),
    created_at      TEXT NOT NULL,
    suite_name      TEXT NOT NULL,          -- e.g. "core_code_eval_v1"
    metrics_json    TEXT NOT NULL           -- pass@k, compile rate, collapse indicators
);

-- ---------------------------------------------------------------------------
-- Replay candidates view
-- ---------------------------------------------------------------------------

CREATE VIEW IF NOT EXISTS replay_candidates AS
SELECT *
FROM skill_log_entry
WHERE quality_tag = 'good'
  AND contains_secrets = 0
  AND pii_present = 0;
