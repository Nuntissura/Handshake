DROP TABLE IF EXISTS knowledge_parallel_indexing_lease_queue;
DROP TABLE IF EXISTS knowledge_agent_recovery_receipts;
DROP TABLE IF EXISTS knowledge_agent_state_recovery_checkpoints;
DROP TABLE IF EXISTS knowledge_agent_role_mailbox_handoffs;
DROP TABLE IF EXISTS knowledge_agent_worktree_claims;

DELETE FROM knowledge_schema_registry
WHERE family_key IN (
    'parallel_swarm_claims',
    'parallel_swarm_handoffs',
    'parallel_swarm_checkpoints',
    'parallel_swarm_recovery_receipts',
    'parallel_indexing_lease_queue'
);
