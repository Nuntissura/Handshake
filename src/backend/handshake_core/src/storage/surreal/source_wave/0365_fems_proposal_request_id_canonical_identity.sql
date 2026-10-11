-- WP-KERNEL-012 MT-112: converge the FEMS memory-proposal stable request identity.
--
-- CANONICAL IDENTITY (AC-112-1). The identity is the SHA-256 of the domain tag
-- 'fems-memory-proposal-request-v2' + NUL, followed by ELEVEN length-prefixed
-- (big-endian u64 byte length + UTF-8 bytes) components in this exact order:
--
--   1  workspace_id                    (route workspace, not trimmed)
--   2  memory_class                    (ProposalClass::wire(), not trimmed)
--   3  content                         (not trimmed)
--   4  source.document_id              (Rust str::trim)
--   5  source.selection_start          (decimal text)
--   6  source.selection_end            (decimal text)
--   7  source.content_hash             (Rust str::trim)
--   8  source.document_content_hash    (Rust str::trim, absent => '')
--   9  source.pane_id                  (Rust str::trim, absent => '')
--  10  source.workspace_id             (Rust str::trim, absent => '')
--  11  sha256_hex(source_document_content)  (absent => '')
--
-- This is exactly `stable_proposal_request_id` in src/api/memory.rs. `actor_id` is
-- DELIBERATELY EXCLUDED: `same_logical_proposal` in src/storage/fems_memory.rs strips
-- actor_id from intake replay equality because the router derives it from the live
-- native binding, so an exact retry from a later authenticated session must converge
-- on the same row. Putting actor_id in the identity would fork that retry into a
-- duplicate proposal and contradict the retry contract. Attribution is not weakened:
-- actor_id remains in the proposal payload and in the immutable EventLedger receipt.
--
-- AC-112-3 (SQL component 11 vs migration 0345 component 12): 0345 hashed TWELVE
-- components - actor_id at 11 and `source.document_content_hash` a second time at 12.
-- The second occurrence is NOT redundant and is NOT removed: component 11 of the
-- canonical list is sha256_hex(source_document_content), and the proposal row never
-- persists source_document_content, so `source.document_content_hash` is the only
-- SQL-derivable expression of it. It is provably the same value: intake
-- (src/api/memory.rs, canonical-code branch) rejects any proposal where
-- sha256_hex(source_document_content) <> source.document_content_hash, and the
-- rich-document and Loom branches reject a proposal that carries either field, so
-- both are ''. Canonical components 8 and 11 are therefore equal for every proposal
-- that can exist, in Rust as well as in SQL. Only actor_id is dropped here.
--
-- AC-112-2: migration 0345 is already applied and its checksum is immutable, so it is
-- NOT edited. This migration supersedes it and re-derives only the rows whose current
-- request_id is exactly what 0345's twelve-component expression produced. Every change
-- is journalled in fems_memory_proposal_request_id_rekey so the down path is exact.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Rust `str::trim` follows Unicode White_Space. PostgreSQL's one-argument `btrim`
-- removes only U+0020, so pin the exact Rust set (U+0009..000D, 0020, 0085, 00A0,
-- 1680, 2000..200A, 2028, 2029, 202F, 205F, 3000).
CREATE OR REPLACE FUNCTION fems_rust_trim(value TEXT)
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

-- THE single SQL definition of the canonical identity. Migrations and proofs call this
-- function instead of re-copying the expression; a hand-copied mirror is exactly how
-- the Rust/SQL split that MT-112 repairs was introduced. Arguments are the already
-- normalized components in canonical order.
CREATE OR REPLACE FUNCTION fems_proposal_request_id(
    p_workspace_id           TEXT,
    p_memory_class           TEXT,
    p_content                TEXT,
    p_document_id            TEXT,
    p_selection_start        TEXT,
    p_selection_end          TEXT,
    p_content_hash           TEXT,
    p_document_content_hash  TEXT,
    p_pane_id                TEXT,
    p_source_workspace_id    TEXT,
    p_source_document_hash   TEXT
)
RETURNS TEXT
LANGUAGE SQL
IMMUTABLE
STRICT
AS $$
    SELECT 'derived-sha256:' || encode(
        digest(
            convert_to('fems-memory-proposal-request-v2', 'UTF8') || decode('00', 'hex') ||
            int8send(octet_length(p_workspace_id)::BIGINT)          || convert_to(p_workspace_id, 'UTF8') ||
            int8send(octet_length(p_memory_class)::BIGINT)          || convert_to(p_memory_class, 'UTF8') ||
            int8send(octet_length(p_content)::BIGINT)               || convert_to(p_content, 'UTF8') ||
            int8send(octet_length(p_document_id)::BIGINT)           || convert_to(p_document_id, 'UTF8') ||
            int8send(octet_length(p_selection_start)::BIGINT)       || convert_to(p_selection_start, 'UTF8') ||
            int8send(octet_length(p_selection_end)::BIGINT)         || convert_to(p_selection_end, 'UTF8') ||
            int8send(octet_length(p_content_hash)::BIGINT)          || convert_to(p_content_hash, 'UTF8') ||
            int8send(octet_length(p_document_content_hash)::BIGINT) || convert_to(p_document_content_hash, 'UTF8') ||
            int8send(octet_length(p_pane_id)::BIGINT)               || convert_to(p_pane_id, 'UTF8') ||
            int8send(octet_length(p_source_workspace_id)::BIGINT)   || convert_to(p_source_workspace_id, 'UTF8') ||
            int8send(octet_length(p_source_document_hash)::BIGINT)  || convert_to(p_source_document_hash, 'UTF8'),
            'sha256'
        ),
        'hex'
    )
$$;

-- The exact 0345 twelve-component expression, retained ONLY so this migration can
-- recognise the rows 0345 keyed and so the down path can reproduce them.
CREATE OR REPLACE FUNCTION fems_proposal_request_id_0345(
    p_workspace_id           TEXT,
    p_memory_class           TEXT,
    p_content                TEXT,
    p_document_id            TEXT,
    p_selection_start        TEXT,
    p_selection_end          TEXT,
    p_content_hash           TEXT,
    p_document_content_hash  TEXT,
    p_pane_id                TEXT,
    p_source_workspace_id    TEXT,
    p_actor_id               TEXT,
    p_source_document_hash   TEXT
)
RETURNS TEXT
LANGUAGE SQL
IMMUTABLE
STRICT
AS $$
    SELECT 'derived-sha256:' || encode(
        digest(
            convert_to('fems-memory-proposal-request-v2', 'UTF8') || decode('00', 'hex') ||
            int8send(octet_length(p_workspace_id)::BIGINT)          || convert_to(p_workspace_id, 'UTF8') ||
            int8send(octet_length(p_memory_class)::BIGINT)          || convert_to(p_memory_class, 'UTF8') ||
            int8send(octet_length(p_content)::BIGINT)               || convert_to(p_content, 'UTF8') ||
            int8send(octet_length(p_document_id)::BIGINT)           || convert_to(p_document_id, 'UTF8') ||
            int8send(octet_length(p_selection_start)::BIGINT)       || convert_to(p_selection_start, 'UTF8') ||
            int8send(octet_length(p_selection_end)::BIGINT)         || convert_to(p_selection_end, 'UTF8') ||
            int8send(octet_length(p_content_hash)::BIGINT)          || convert_to(p_content_hash, 'UTF8') ||
            int8send(octet_length(p_document_content_hash)::BIGINT) || convert_to(p_document_content_hash, 'UTF8') ||
            int8send(octet_length(p_pane_id)::BIGINT)               || convert_to(p_pane_id, 'UTF8') ||
            int8send(octet_length(p_source_workspace_id)::BIGINT)   || convert_to(p_source_workspace_id, 'UTF8') ||
            int8send(octet_length(p_actor_id)::BIGINT)              || convert_to(p_actor_id, 'UTF8') ||
            int8send(octet_length(p_source_document_hash)::BIGINT)  || convert_to(p_source_document_hash, 'UTF8'),
            'sha256'
        ),
        'hex'
    )
$$;

-- AC-112-6 evidence surface and the exact reversal record for the down path.
CREATE TABLE IF NOT EXISTS fems_memory_proposal_request_id_rekey (
    proposal_id     TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    old_request_id  TEXT NOT NULL,
    new_request_id  TEXT NOT NULL,
    migration       TEXT NOT NULL,
    rekeyed_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

LOCK TABLE fems_memory_proposals IN ACCESS EXCLUSIVE MODE;
LOCK TABLE fems_memory_proposal_request_id_rekey IN ACCESS EXCLUSIVE MODE;

WITH identity_inputs AS (
    SELECT
        p.proposal_id,
        p.workspace_id,
        p.created_at,
        p.request_id AS current_request_id,
        fems_proposal_request_id_0345(
            p.workspace_id,
            p.memory_class,
            COALESCE(p.proposal ->> 'content', ''),
            fems_rust_trim(p.document_id),
            p.selection_start::TEXT,
            p.selection_end::TEXT,
            fems_rust_trim(p.content_hash),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,pane_id}', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,workspace_id}', '')),
            fems_rust_trim(COALESCE(p.proposal ->> 'actor_id', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', ''))
        ) AS legacy_0345_request_id,
        fems_proposal_request_id(
            p.workspace_id,
            p.memory_class,
            COALESCE(p.proposal ->> 'content', ''),
            fems_rust_trim(p.document_id),
            p.selection_start::TEXT,
            p.selection_end::TEXT,
            fems_rust_trim(p.content_hash),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,pane_id}', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,workspace_id}', '')),
            fems_rust_trim(COALESCE(p.proposal #>> '{source,document_content_hash}', ''))
        ) AS canonical_request_id
    FROM fems_memory_proposals p
), classified AS (
    SELECT
        *,
        CASE
            WHEN current_request_id = legacy_0345_request_id
                THEN canonical_request_id
            WHEN current_request_id
                 = 'legacy-duplicate:' || legacy_0345_request_id || ':' || proposal_id
                THEN 'legacy-duplicate:' || canonical_request_id || ':' || proposal_id
            ELSE NULL
        END AS desired_request_id
    FROM identity_inputs
), affected AS (
    SELECT *
    FROM classified
    WHERE desired_request_id IS NOT NULL
      AND desired_request_id <> current_request_id
), ranked AS (
    -- An affected row may want an identity that an untouched row already holds, or that
    -- another affected row wants. Exactly one row may take it; the rest keep a
    -- deterministic per-proposal identity so nothing is lost, merged, or deleted.
    SELECT
        a.*,
        row_number() OVER (
            PARTITION BY a.workspace_id, a.desired_request_id
            ORDER BY a.created_at, a.proposal_id
        ) AS identity_rank,
        EXISTS (
            SELECT 1
            FROM classified c
            WHERE c.desired_request_id IS NULL
              AND c.workspace_id = a.workspace_id
              AND c.current_request_id = a.desired_request_id
        ) AS slot_taken_by_untouched_row
    FROM affected a
), final_identities AS (
    SELECT
        proposal_id,
        workspace_id,
        current_request_id AS old_request_id,
        CASE
            WHEN identity_rank = 1 AND NOT slot_taken_by_untouched_row
                THEN desired_request_id
            ELSE 'legacy-duplicate:' || canonical_request_id || ':' || proposal_id
        END AS new_request_id
    FROM ranked
)
INSERT INTO fems_memory_proposal_request_id_rekey (
    proposal_id, workspace_id, old_request_id, new_request_id, migration
)
SELECT
    proposal_id,
    workspace_id,
    old_request_id,
    new_request_id,
    '0365_fems_proposal_request_id_canonical_identity'
FROM final_identities
WHERE new_request_id <> old_request_id
ON CONFLICT (proposal_id) DO NOTHING;

UPDATE fems_memory_proposals p
SET request_id = r.new_request_id,
    proposal = jsonb_set(p.proposal, '{request_id}', to_jsonb(r.new_request_id), true)
FROM fems_memory_proposal_request_id_rekey r
WHERE r.proposal_id = p.proposal_id
  AND r.migration = '0365_fems_proposal_request_id_canonical_identity'
  AND p.request_id = r.old_request_id;

DROP FUNCTION fems_proposal_request_id_0345(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT
);
