-- WP-KERNEL-005 MT-001/MT-002 source-evidence and anchor-verification matrix.
-- This is product-visible runtime state, not repo-governance paperwork: fresh
-- models can query which legacy-source facts are verified in product code and
-- which anchors are explicitly blocked.

CREATE TABLE IF NOT EXISTS atelier_source_evidence_record (
    source_id             TEXT PRIMARY KEY,
    matrix_id             TEXT NOT NULL,
    source_label          TEXT NOT NULL,
    source_ref            TEXT NOT NULL,
    product_area          TEXT NOT NULL,
    maturity_status       TEXT NOT NULL CHECK (maturity_status IN ('DONE', 'REVIEW', 'BLOCKED')),
    implementation_status TEXT NOT NULL,
    evidence_refs         JSONB NOT NULL DEFAULT '[]'::jsonb,
    proof_refs            JSONB NOT NULL DEFAULT '[]'::jsonb,
    gap_reason            TEXT,
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (btrim(source_id) = source_id AND btrim(source_id) <> ''),
    CHECK (btrim(matrix_id) = matrix_id AND btrim(matrix_id) <> ''),
    CHECK (btrim(source_label) = source_label AND btrim(source_label) <> ''),
    CHECK (btrim(source_ref) = source_ref AND btrim(source_ref) <> ''),
    CHECK (btrim(product_area) = product_area AND btrim(product_area) <> ''),
    CHECK (btrim(implementation_status) = implementation_status AND btrim(implementation_status) <> ''),
    CHECK (jsonb_typeof(evidence_refs) = 'array'),
    CHECK (jsonb_typeof(proof_refs) = 'array')
);

CREATE INDEX IF NOT EXISTS idx_atelier_source_evidence_matrix
    ON atelier_source_evidence_record(matrix_id, maturity_status);

CREATE TABLE IF NOT EXISTS atelier_anchor_verification_record (
    anchor_id              TEXT PRIMARY KEY,
    matrix_id              TEXT NOT NULL,
    source_id              TEXT NOT NULL REFERENCES atelier_source_evidence_record(source_id) ON DELETE CASCADE,
    anchor_label           TEXT NOT NULL,
    expected_product_path  TEXT NOT NULL,
    verification_status    TEXT NOT NULL CHECK (verification_status IN ('VERIFIED', 'BLOCKED_MISSING_ANCHOR')),
    verified_product_paths JSONB NOT NULL DEFAULT '[]'::jsonb,
    blocking_reason        TEXT,
    updated_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (btrim(anchor_id) = anchor_id AND btrim(anchor_id) <> ''),
    CHECK (btrim(matrix_id) = matrix_id AND btrim(matrix_id) <> ''),
    CHECK (btrim(source_id) = source_id AND btrim(source_id) <> ''),
    CHECK (btrim(anchor_label) = anchor_label AND btrim(anchor_label) <> ''),
    CHECK (btrim(expected_product_path) = expected_product_path AND btrim(expected_product_path) <> ''),
    CHECK (jsonb_typeof(verified_product_paths) = 'array')
);

CREATE INDEX IF NOT EXISTS idx_atelier_anchor_verification_matrix
    ON atelier_anchor_verification_record(matrix_id, verification_status);

CREATE INDEX IF NOT EXISTS idx_atelier_anchor_verification_source
    ON atelier_anchor_verification_record(source_id);
