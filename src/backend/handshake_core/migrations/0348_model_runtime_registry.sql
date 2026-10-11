-- WP-1 MT-014 V2: durable selected-adapter authority for ModelRuntime.
--
-- The stable artifact SHA-256 owns the immutable adapter/capability/provider
-- selection. Paths, display labels, operators, and per-boot ModelId values are
-- observations only. First selection and explicit compare-and-swap rebinds
-- reference EventLedger rows written in the same PostgreSQL transaction.

CREATE TABLE IF NOT EXISTS model_runtime_registry (
    schema_id TEXT NOT NULL,
    registry_row_id UUID NOT NULL,
    artifact_sha256 BYTEA NOT NULL,
    artifact_locator TEXT NOT NULL,
    last_observed_runtime_model_id UUID NOT NULL,
    runtime_binding TEXT NOT NULL,
    capabilities_schema_id TEXT NOT NULL,
    capabilities_json JSONB NOT NULL,
    provider TEXT NOT NULL,
    base_model_tag TEXT NOT NULL,
    last_observed_by TEXT NOT NULL,
    selection_revision BIGINT NOT NULL,
    selection_created_event_id TEXT NOT NULL,
    selection_updated_event_id TEXT NOT NULL,
    selection_created_at_utc TIMESTAMPTZ NOT NULL,
    selection_updated_at_utc TIMESTAMPTZ NOT NULL,
    last_observed_at_utc TIMESTAMPTZ NOT NULL,
    CONSTRAINT pk_model_runtime_registry PRIMARY KEY (registry_row_id),
    CONSTRAINT uq_model_runtime_registry_artifact_sha256 UNIQUE (artifact_sha256),
    CONSTRAINT chk_model_runtime_registry_schema_id
        CHECK (schema_id = 'hsk.model_runtime_registry.row@1'),
    CONSTRAINT chk_model_runtime_registry_artifact_sha256
        CHECK (octet_length(artifact_sha256) = 32),
    CONSTRAINT chk_model_runtime_registry_artifact_locator
        CHECK (artifact_locator = 'sha256:' || pg_catalog.encode(artifact_sha256, 'hex')),
    CONSTRAINT chk_model_runtime_registry_runtime_binding
        CHECK (runtime_binding IN ('llama_cpp', 'candle')),
    CONSTRAINT chk_model_runtime_registry_capabilities_schema_id
        CHECK (capabilities_schema_id = 'hsk.model_runtime.capabilities@1'),
    CONSTRAINT chk_model_runtime_registry_capabilities
        CHECK (jsonb_typeof(capabilities_json) = 'object'),
    CONSTRAINT chk_model_runtime_registry_provider
        CHECK (provider = 'local'),
    CONSTRAINT chk_model_runtime_registry_base_model_tag
        CHECK (length(btrim(base_model_tag)) > 0),
    CONSTRAINT chk_model_runtime_registry_last_observed_by
        CHECK (length(btrim(last_observed_by)) > 0),
    CONSTRAINT chk_model_runtime_registry_selection_revision
        CHECK (selection_revision >= 1),
    CONSTRAINT fk_model_runtime_registry_selection_created_event
        FOREIGN KEY (selection_created_event_id) REFERENCES kernel_event_ledger (event_id),
    CONSTRAINT fk_model_runtime_registry_selection_updated_event
        FOREIGN KEY (selection_updated_event_id) REFERENCES kernel_event_ledger (event_id)
);

CREATE INDEX IF NOT EXISTS idx_model_runtime_registry_selection_updated_at
    ON model_runtime_registry (selection_updated_at_utc DESC, registry_row_id ASC);
