-- Add traceability metadata columns to RAW content tables.

-- Workspaces
ALTER TABLE workspaces ADD COLUMN last_job_id TEXT;
ALTER TABLE workspaces ADD COLUMN last_workflow_id TEXT;
ALTER TABLE workspaces ADD COLUMN last_actor_id TEXT;
ALTER TABLE workspaces ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE workspaces ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);

-- Documents
ALTER TABLE documents ADD COLUMN last_job_id TEXT;
ALTER TABLE documents ADD COLUMN last_workflow_id TEXT;
ALTER TABLE documents ADD COLUMN last_actor_id TEXT;
ALTER TABLE documents ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE documents ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);

-- Blocks
ALTER TABLE blocks ADD COLUMN last_job_id TEXT;
ALTER TABLE blocks ADD COLUMN last_workflow_id TEXT;
ALTER TABLE blocks ADD COLUMN last_actor_id TEXT;
ALTER TABLE blocks ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE blocks ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);

-- Canvas Nodes
ALTER TABLE canvas_nodes ADD COLUMN last_job_id TEXT;
ALTER TABLE canvas_nodes ADD COLUMN last_workflow_id TEXT;
ALTER TABLE canvas_nodes ADD COLUMN last_actor_id TEXT;
ALTER TABLE canvas_nodes ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE canvas_nodes ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);

-- Canvas Edges
ALTER TABLE canvas_edges ADD COLUMN last_job_id TEXT;
ALTER TABLE canvas_edges ADD COLUMN last_workflow_id TEXT;
ALTER TABLE canvas_edges ADD COLUMN last_actor_id TEXT;
ALTER TABLE canvas_edges ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE canvas_edges ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);
