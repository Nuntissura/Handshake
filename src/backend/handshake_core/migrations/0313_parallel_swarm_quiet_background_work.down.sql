DELETE FROM knowledge_schema_registry
 WHERE family_key = 'parallel_swarm_quiet_background_work';

DROP TABLE IF EXISTS knowledge_agent_quiet_background_work;

ALTER TABLE knowledge_parallel_indexing_lease_queue
    DROP CONSTRAINT IF EXISTS chk_parallel_indexing_lease_queue_quiet_policy;

ALTER TABLE knowledge_parallel_indexing_lease_queue
    DROP COLUMN IF EXISTS quiet_policy_jsonb;
