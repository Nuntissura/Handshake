-- WP-KERNEL-004 MT-007:
-- Postgres process lifecycle authority rows for spawned Handshake processes.

CREATE TABLE IF NOT EXISTS kernel_process_lifecycle (
    process_uuid UUID PRIMARY KEY,
    process_id UUID GENERATED ALWAYS AS (process_uuid) STORED,
    os_pid BIGINT CHECK (os_pid IS NULL OR os_pid >= 0),
    parent_session_id TEXT,
    parent_process_id UUID,
    sandbox_adapter_id TEXT,
    adapter_id TEXT GENERATED ALWAYS AS (sandbox_adapter_id) STORED,
    sandbox_internal_id TEXT,
    engine_kind TEXT NOT NULL CHECK (
        engine_kind IN (
            'llamacpp',
            'llama_cpp',
            'candle',
            'abliteration_tool',
            'sandbox_container',
            'mechanical_job',
            'asr_worker',
            'comfyui_worker',
            'plugin_process',
            'helper_subprocess',
            'external_compat',
            'webview2_cdp',
            'official_cli_bridge'
        )
    ),
    started_at TIMESTAMPTZ NOT NULL,
    spawned_at_utc TIMESTAMPTZ GENERATED ALWAYS AS (started_at) STORED,
    stopped_at TIMESTAMPTZ,
    stopped_at_utc TIMESTAMPTZ GENERATED ALWAYS AS (stopped_at) STORED,
    exit_code INTEGER,
    model_artifact_sha256 TEXT,
    work_profile_id TEXT,
    owner_role TEXT NOT NULL,
    owner_wp TEXT,
    role_id TEXT,
    wp_id TEXT,
    mt_id TEXT,
    stop_reason TEXT,
    sandbox_capabilities_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb
);

ALTER TABLE kernel_process_lifecycle
    ADD COLUMN IF NOT EXISTS process_id UUID GENERATED ALWAYS AS (process_uuid) STORED,
    ADD COLUMN IF NOT EXISTS parent_process_id UUID,
    ADD COLUMN IF NOT EXISTS adapter_id TEXT GENERATED ALWAYS AS (sandbox_adapter_id) STORED,
    ADD COLUMN IF NOT EXISTS sandbox_internal_id TEXT,
    ADD COLUMN IF NOT EXISTS spawned_at_utc TIMESTAMPTZ GENERATED ALWAYS AS (started_at) STORED,
    ADD COLUMN IF NOT EXISTS stopped_at_utc TIMESTAMPTZ GENERATED ALWAYS AS (stopped_at) STORED,
    ADD COLUMN IF NOT EXISTS role_id TEXT,
    ADD COLUMN IF NOT EXISTS wp_id TEXT,
    ADD COLUMN IF NOT EXISTS mt_id TEXT,
    ADD COLUMN IF NOT EXISTS stop_reason TEXT,
    ADD COLUMN IF NOT EXISTS sandbox_capabilities_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_parent_session_started
    ON kernel_process_lifecycle (parent_session_id, started_at);

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_engine_started
    ON kernel_process_lifecycle (engine_kind, started_at);

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_os_pid
    ON kernel_process_lifecycle (os_pid)
    WHERE os_pid IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_adapter_spawned
    ON kernel_process_lifecycle (adapter_id, spawned_at_utc);

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_wp_spawned
    ON kernel_process_lifecycle (wp_id, spawned_at_utc);
