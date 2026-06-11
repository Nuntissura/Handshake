-- WP-KERNEL-009 MT-076 rollback. Later 015x rows referencing leases must be
-- rolled back first; removes the lease table and its registry row only.

DROP TABLE IF EXISTS knowledge_crdt_agent_lane_leases;
DELETE FROM knowledge_schema_registry WHERE family_key = 'crdt_agent_lane_leases';
