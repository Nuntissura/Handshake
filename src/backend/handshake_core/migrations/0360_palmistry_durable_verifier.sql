CREATE TABLE IF NOT EXISTS palmistry_durable_verifier (
    session_id UUID NOT NULL,
    launch_nonce UUID NOT NULL,
    parent_pid BIGINT NOT NULL CHECK (parent_pid > 0),
    watcher_pid BIGINT NOT NULL CHECK (watcher_pid > 0),
    watcher_creation_time_100ns BIGINT NOT NULL CHECK (watcher_creation_time_100ns > 0),
    process_uuid UUID NOT NULL UNIQUE,
    executable_sha256 CHAR(64) NOT NULL CHECK (executable_sha256 ~ '^[0-9a-f]{64}$'),
    verifying_key_hex CHAR(64) NOT NULL CHECK (verifying_key_hex ~ '^[0-9a-f]{64}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retired_at TIMESTAMPTZ,
    PRIMARY KEY (session_id, launch_nonce)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_palmistry_durable_verifier_active
    ON palmistry_durable_verifier (session_id)
    WHERE retired_at IS NULL;
