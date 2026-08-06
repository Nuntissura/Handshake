-- WP-KERNEL-012 MT-112 down path (AC-112-2). Exact reversal: every identity this
-- migration changed was journalled with its prior value, so the rollback restores the
-- migration-0345 twelve-component identity byte-for-byte instead of re-deriving it.

DO $$
BEGIN
    IF to_regclass('fems_memory_proposal_request_id_rekey') IS NOT NULL
       AND to_regclass('fems_memory_proposals') IS NOT NULL THEN
        EXECUTE $sql$
            UPDATE fems_memory_proposals p
            SET request_id = r.old_request_id,
                proposal = jsonb_set(
                    p.proposal, '{request_id}', to_jsonb(r.old_request_id), true
                )
            FROM fems_memory_proposal_request_id_rekey r
            WHERE r.proposal_id = p.proposal_id
              AND r.migration = '0365_fems_proposal_request_id_canonical_identity'
              AND p.request_id = r.new_request_id
        $sql$;
    END IF;
END
$$;

DROP TABLE IF EXISTS fems_memory_proposal_request_id_rekey;

DROP FUNCTION IF EXISTS fems_proposal_request_id(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT
);
DROP FUNCTION IF EXISTS fems_proposal_request_id_0345(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT
);
DROP FUNCTION IF EXISTS fems_rust_trim(TEXT);
