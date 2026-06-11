-- WP-KERNEL-005 MT-001/MT-002 matrix-scoped source evidence identity.
-- 0041 introduced the runtime matrix tables; this migration makes row identity
-- matrix-scoped so independent matrix snapshots can coexist.

ALTER TABLE atelier_anchor_verification_record
    DROP CONSTRAINT IF EXISTS atelier_anchor_verification_record_source_id_fkey;

ALTER TABLE atelier_anchor_verification_record
    DROP CONSTRAINT IF EXISTS atelier_anchor_verification_record_matrix_id_source_id_fkey;

ALTER TABLE atelier_anchor_verification_record
    DROP CONSTRAINT IF EXISTS atelier_anchor_verification_record_matrix_source_fkey;

ALTER TABLE atelier_anchor_verification_record
    DROP CONSTRAINT IF EXISTS atelier_anchor_verification_record_pkey;

ALTER TABLE atelier_source_evidence_record
    DROP CONSTRAINT IF EXISTS atelier_source_evidence_record_pkey;

ALTER TABLE atelier_source_evidence_record
    ADD CONSTRAINT atelier_source_evidence_record_pkey PRIMARY KEY (matrix_id, source_id);

ALTER TABLE atelier_anchor_verification_record
    ADD CONSTRAINT atelier_anchor_verification_record_pkey PRIMARY KEY (matrix_id, anchor_id);

ALTER TABLE atelier_anchor_verification_record
    ADD CONSTRAINT atelier_anchor_verification_record_matrix_source_fkey
    FOREIGN KEY (matrix_id, source_id)
    REFERENCES atelier_source_evidence_record(matrix_id, source_id)
    ON DELETE CASCADE;
