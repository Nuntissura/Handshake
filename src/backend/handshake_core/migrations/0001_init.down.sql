-- Rollback Phase 0 schema (dev/test/CI only)

DROP TABLE IF EXISTS canvas_edges;
DROP TABLE IF EXISTS canvas_nodes;
DROP TABLE IF EXISTS canvases;
DROP TABLE IF EXISTS blocks;
DROP TABLE IF EXISTS documents;
DROP TABLE IF EXISTS workspaces;
