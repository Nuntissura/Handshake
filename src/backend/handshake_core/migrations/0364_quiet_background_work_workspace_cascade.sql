-- Match the parent-then-child lock order used by workspace deletion so the
-- live upgrade cannot deadlock with an in-flight cascade.
LOCK TABLE workspaces IN SHARE ROW EXCLUSIVE MODE;
LOCK TABLE knowledge_agent_quiet_background_work IN SHARE ROW EXCLUSIVE MODE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_quiet_background_work_workspace'
          AND conrelid = 'knowledge_agent_quiet_background_work'::regclass
    ) THEN
        ALTER TABLE knowledge_agent_quiet_background_work
            ADD CONSTRAINT fk_quiet_background_work_workspace
            FOREIGN KEY (workspace_id)
            REFERENCES workspaces (id)
            ON DELETE CASCADE
            NOT VALID;
    END IF;
END $$;

-- The NOT VALID constraint protects new writes while legacy rows are repaired.
DELETE FROM knowledge_agent_quiet_background_work AS quiet_work
WHERE NOT EXISTS (
    SELECT 1
    FROM workspaces
    WHERE workspaces.id = quiet_work.workspace_id
);

ALTER TABLE knowledge_agent_quiet_background_work
    VALIDATE CONSTRAINT fk_quiet_background_work_workspace;
