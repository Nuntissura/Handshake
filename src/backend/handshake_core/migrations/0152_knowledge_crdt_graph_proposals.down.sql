-- WP-KERNEL-009 MT-068 rollback. Promoted facts (0153) reference proposals;
-- roll back 0153 first. Removes the proposal table and its registry row.

DROP TABLE IF EXISTS knowledge_crdt_graph_proposals;
DELETE FROM knowledge_schema_registry WHERE family_key = 'crdt_graph_proposals';
