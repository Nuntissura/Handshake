-- Phase 0 schema

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS blocks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    raw_content TEXT NOT NULL,
    display_content TEXT NOT NULL,
    derived_content TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    sensitivity TEXT DEFAULT NULL,
    exportable INTEGER DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS canvases (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS canvas_nodes (
    id TEXT PRIMARY KEY,
    canvas_id TEXT NOT NULL REFERENCES canvases(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    position_x REAL NOT NULL,
    position_y REAL NOT NULL,
    data TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS canvas_edges (
    id TEXT PRIMARY KEY,
    canvas_id TEXT NOT NULL REFERENCES canvases(id) ON DELETE CASCADE,
    from_node_id TEXT NOT NULL REFERENCES canvas_nodes(id) ON DELETE CASCADE,
    to_node_id TEXT NOT NULL REFERENCES canvas_nodes(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
