-- WP-1 MT-014 V5: PostgreSQL-authoritative active ModelRuntime defaults.

CREATE TABLE model_runtime_active_selection (
    schema_id TEXT NOT NULL,
    purpose TEXT NOT NULL,
    runtime_role TEXT NOT NULL,
    artifact_sha256 BYTEA NOT NULL,
    selection_revision BIGINT NOT NULL,
    selection_created_event_id TEXT NOT NULL,
    selection_updated_event_id TEXT NOT NULL,
    selection_created_at_utc TIMESTAMPTZ NOT NULL,
    selection_updated_at_utc TIMESTAMPTZ NOT NULL,
    CONSTRAINT pk_model_runtime_active_selection PRIMARY KEY (purpose),
    CONSTRAINT chk_model_runtime_active_selection_schema_id
        CHECK (schema_id = 'hsk.model_runtime.active_selection@1'),
    CONSTRAINT chk_model_runtime_active_selection_purpose_role
        CHECK (
            (purpose = 'application/default' AND runtime_role = 'completion') OR
            (purpose = 'embeddings/default' AND runtime_role = 'embedding')
        ),
    CONSTRAINT chk_model_runtime_active_selection_artifact_sha256
        CHECK (octet_length(artifact_sha256) = 32),
    CONSTRAINT chk_model_runtime_active_selection_revision
        CHECK (selection_revision >= 1),
    CONSTRAINT fk_model_runtime_active_selection_registry
        FOREIGN KEY (artifact_sha256)
        REFERENCES model_runtime_registry (artifact_sha256),
    CONSTRAINT fk_model_runtime_active_selection_created_event
        FOREIGN KEY (selection_created_event_id)
        REFERENCES kernel_event_ledger (event_id),
    CONSTRAINT fk_model_runtime_active_selection_updated_event
        FOREIGN KEY (selection_updated_event_id)
        REFERENCES kernel_event_ledger (event_id)
);

CREATE INDEX idx_model_runtime_active_selection_artifact
    ON model_runtime_active_selection (artifact_sha256, purpose);
