-- Repair rows left by workspace deletion before the quiet-background-work table
-- participated in workspace ownership, then enforce the ownership boundary.
DELETE FROM knowledge_agent_quiet_background_work AS quiet_work
WHERE NOT EXISTS (
    SELECT 1
    FROM workspaces
    WHERE workspaces.id = quiet_work.workspace_id
);

ALTER TABLE knowledge_agent_quiet_background_work
    ADD CONSTRAINT fk_quiet_background_work_workspace
    FOREIGN KEY (workspace_id)
    REFERENCES workspaces (id)
    ON DELETE CASCADE;
