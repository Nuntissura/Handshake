-- WP-KERNEL-012 MT-109 remediation: make workspace ownership and retry identity
-- database-enforced. SQLx runs this migration transactionally; the explicit table
-- locks prevent request traffic from observing or mutating the legacy backfill.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS fems_memory_packs (
    pack_id       TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL,
    scope_key     TEXT NOT NULL DEFAULT '',
    pack          JSONB NOT NULL,
    generated_at  TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS fems_memory_proposals (
    proposal_id      TEXT PRIMARY KEY,
    request_id       TEXT,
    workspace_id     TEXT NOT NULL,
    document_id      TEXT NOT NULL,
    selection_start  BIGINT NOT NULL,
    selection_end    BIGINT NOT NULL,
    content_hash     TEXT NOT NULL,
    memory_class     TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending_review',
    review_gated     BOOLEAN NOT NULL DEFAULT TRUE,
    proposal         JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS fems_memory_items (
    memory_id     TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL,
    item          JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE fems_memory_proposals ADD COLUMN IF NOT EXISTS request_id TEXT;

LOCK TABLE fems_memory_packs IN ACCESS EXCLUSIVE MODE;
LOCK TABLE fems_memory_proposals IN ACCESS EXCLUSIVE MODE;
LOCK TABLE fems_memory_items IN ACCESS EXCLUSIVE MODE;

-- Mutable FEMS projections without a canonical owner cannot be retained. Canonical
-- EventLedger and Flight Recorder receipts are deliberately not modified.
DELETE FROM fems_memory_proposals p
WHERE NOT EXISTS (SELECT 1 FROM workspaces w WHERE w.id = p.workspace_id);
DELETE FROM fems_memory_packs p
WHERE NOT EXISTS (SELECT 1 FROM workspaces w WHERE w.id = p.workspace_id);
DELETE FROM fems_memory_items i
WHERE NOT EXISTS (SELECT 1 FROM workspaces w WHERE w.id = i.workspace_id);

CREATE OR REPLACE FUNCTION pg_temp.fems_len_prefix(value TEXT)
RETURNS BYTEA
LANGUAGE SQL
IMMUTABLE
STRICT
AS $$
    SELECT int8send(octet_length(value)::BIGINT) || convert_to(value, 'UTF8')
$$;

-- Rust `str::trim` follows Unicode White_Space. PostgreSQL's one-argument
-- `btrim` removes only U+0020, so use the exact Rust set for legacy identity
-- convergence (U+0009..000D, 0020, 0085, 00A0, 1680, 2000..200A, 2028,
-- 2029, 202F, 205F, 3000).
CREATE OR REPLACE FUNCTION pg_temp.fems_rust_trim(value TEXT)
RETURNS TEXT
LANGUAGE SQL
IMMUTABLE
STRICT
AS $$
    SELECT btrim(
        value,
        chr(9) || chr(10) || chr(11) || chr(12) || chr(13) || chr(32) ||
        chr(133) || chr(160) || chr(5760) ||
        chr(8192) || chr(8193) || chr(8194) || chr(8195) || chr(8196) ||
        chr(8197) || chr(8198) || chr(8199) || chr(8200) || chr(8201) ||
        chr(8202) || chr(8232) || chr(8233) || chr(8239) || chr(8287) ||
        chr(12288)
    )
$$;

-- Legacy clients had no request_id. Derive the same length-prefixed identity used by
-- the Rust request path. Existing explicit/current identities win collisions; duplicate
-- legacy rows are retained under deterministic suffixes so no historical proposal or
-- append-only receipt correlation is destroyed, while future retries converge to one row.
WITH identity_inputs AS (
    SELECT
        p.proposal_id,
        p.workspace_id,
        p.created_at,
        CASE
            WHEN p.proposal ? 'request_id'
                 AND pg_temp.fems_rust_trim(COALESCE(p.proposal ->> 'request_id', '')) <> ''
                THEN pg_temp.fems_rust_trim(p.proposal ->> 'request_id')
            WHEN p.request_id IS NOT NULL
                 AND pg_temp.fems_rust_trim(p.request_id) <> ''
                 AND p.request_id <> p.proposal_id
                THEN pg_temp.fems_rust_trim(p.request_id)
            ELSE NULL
        END AS preserved_request_id,
        'derived-sha256:' || encode(
            digest(
                convert_to('fems-memory-proposal-request-v2', 'UTF8') || decode('00', 'hex') ||
                pg_temp.fems_len_prefix(p.workspace_id) ||
                pg_temp.fems_len_prefix(p.memory_class) ||
                pg_temp.fems_len_prefix(COALESCE(p.proposal ->> 'content', '')) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(p.document_id)) ||
                pg_temp.fems_len_prefix(p.selection_start::TEXT) ||
                pg_temp.fems_len_prefix(p.selection_end::TEXT) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(p.content_hash)) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', ''))) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(COALESCE(p.proposal #>> '{source,pane_id}', ''))) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(COALESCE(p.proposal #>> '{source,workspace_id}', ''))) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(COALESCE(p.proposal ->> 'actor_id', ''))) ||
                pg_temp.fems_len_prefix(pg_temp.fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', ''))),
                'sha256'
            ),
            'hex'
        ) AS derived_request_id
    FROM fems_memory_proposals p
), desired_identities AS (
    SELECT
        *,
        COALESCE(preserved_request_id, derived_request_id) AS desired_request_id,
        preserved_request_id IS NULL AS is_legacy
    FROM identity_inputs
), ranked_identities AS (
    SELECT
        *,
        row_number() OVER (
            PARTITION BY workspace_id, desired_request_id
            ORDER BY CASE WHEN is_legacy THEN 1 ELSE 0 END, created_at, proposal_id
        ) AS identity_rank
    FROM desired_identities
), final_identities AS (
    SELECT
        proposal_id,
        CASE
            WHEN identity_rank = 1 THEN desired_request_id
            ELSE 'legacy-duplicate:' || desired_request_id || ':' || proposal_id
        END AS final_request_id
    FROM ranked_identities
)
UPDATE fems_memory_proposals p
SET request_id = f.final_request_id,
    proposal = jsonb_set(p.proposal, '{request_id}', to_jsonb(f.final_request_id), true)
FROM final_identities f
WHERE f.proposal_id = p.proposal_id;

ALTER TABLE fems_memory_proposals ALTER COLUMN request_id SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_fems_memory_packs_workspace'
          AND conrelid = to_regclass('fems_memory_packs')
    ) THEN
        ALTER TABLE fems_memory_packs
            ADD CONSTRAINT fk_fems_memory_packs_workspace
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_fems_memory_proposals_workspace'
          AND conrelid = to_regclass('fems_memory_proposals')
    ) THEN
        ALTER TABLE fems_memory_proposals
            ADD CONSTRAINT fk_fems_memory_proposals_workspace
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_fems_memory_items_workspace'
          AND conrelid = to_regclass('fems_memory_items')
    ) THEN
        ALTER TABLE fems_memory_items
            ADD CONSTRAINT fk_fems_memory_items_workspace
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_fems_memory_packs_scope
    ON fems_memory_packs (workspace_id, scope_key, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fems_memory_proposals_ws_status
    ON fems_memory_proposals (workspace_id, status, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fems_memory_proposals_ws_request
    ON fems_memory_proposals (workspace_id, request_id);
CREATE INDEX IF NOT EXISTS idx_fems_memory_items_ws
    ON fems_memory_items (workspace_id);

DROP FUNCTION pg_temp.fems_len_prefix(TEXT);
DROP FUNCTION pg_temp.fems_rust_trim(TEXT);
