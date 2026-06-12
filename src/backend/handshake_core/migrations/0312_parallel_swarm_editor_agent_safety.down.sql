DO $$
DECLARE
    constraint_name TEXT;
BEGIN
    SELECT conname
      INTO constraint_name
      FROM pg_constraint
     WHERE conrelid = 'knowledge_agent_worktree_claims'::regclass
       AND contype = 'c'
       AND pg_get_constraintdef(oid) LIKE '%scope_kind%'
     LIMIT 1;

    IF constraint_name IS NOT NULL THEN
        EXECUTE format(
            'ALTER TABLE knowledge_agent_worktree_claims DROP CONSTRAINT %I',
            constraint_name
        );
    END IF;

    ALTER TABLE knowledge_agent_worktree_claims
        ADD CONSTRAINT chk_agent_worktree_claims_scope_kind
        CHECK (scope_kind IN ('worktree', 'workspace', 'index_run'));
END $$;
