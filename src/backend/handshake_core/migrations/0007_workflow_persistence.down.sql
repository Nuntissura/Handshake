-- Rollback workflow persistence tables (dev/test/CI only)

DROP TABLE IF EXISTS workflow_node_executions;
