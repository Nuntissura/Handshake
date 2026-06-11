-- WP-KERNEL-005 MT-106: versioned ComfyUI/pose workflow spec registry.
-- Preserves versioned workflow specs and handler routing with spec version,
-- read-only source refs, handler id, compatibility pin, and validation. A spec
-- is the durable, replay-stable contract for a workflow graph: the graph/spec
-- JSON, a content hash pin, and the handler that routes the workflow. Identity
-- is (workflow_kind, spec_version): re-registering the same kind+version is an
-- idempotent upsert. spec_hash is unique so a given graph hash never registers
-- twice under conflicting identities. Storage authority is PostgreSQL only
-- (LAW-COMFY-INTAKE-004); SQLite is forbidden.

CREATE TABLE IF NOT EXISTS atelier_comfy_workflow_spec (
    spec_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_kind TEXT NOT NULL,
    spec_version TEXT NOT NULL,
    -- Content-addressed pin over the spec_json graph/contract (replay identity).
    spec_hash TEXT NOT NULL,
    -- Handler that routes this workflow (engine.comfyui adapter handler id).
    handler_id TEXT NOT NULL,
    -- Optional compatibility pin: the bridge protocol / engine version this spec
    -- is pinned to. Null when the spec is version-agnostic.
    compatibility_pin TEXT,
    -- The workflow graph / contract JSON (the spec body).
    spec_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Optional portable, READ-ONLY source ref the spec was lifted from. Never a
    -- legacy/.GOV/SQLite/localhost/machine-local ref (enforced in code).
    source_ref TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_atelier_comfy_workflow_spec_kind_version
        UNIQUE (workflow_kind, spec_version),
    CONSTRAINT uq_atelier_comfy_workflow_spec_hash
        UNIQUE (spec_hash),
    CONSTRAINT chk_atelier_comfy_workflow_spec_kind
        CHECK (
            btrim(workflow_kind) = workflow_kind
            AND workflow_kind <> ''
        ),
    CONSTRAINT chk_atelier_comfy_workflow_spec_version
        CHECK (
            btrim(spec_version) = spec_version
            AND spec_version <> ''
        ),
    CONSTRAINT chk_atelier_comfy_workflow_spec_hash_shape
        CHECK (
            btrim(spec_hash) = spec_hash
            AND spec_hash <> ''
        ),
    CONSTRAINT chk_atelier_comfy_workflow_spec_handler
        CHECK (
            btrim(handler_id) = handler_id
            AND handler_id <> ''
        ),
    CONSTRAINT chk_atelier_comfy_workflow_spec_json
        CHECK (jsonb_typeof(spec_json) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_workflow_spec_kind
    ON atelier_comfy_workflow_spec(workflow_kind, created_at_utc);
