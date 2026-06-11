-- WP-KERNEL-009 MT-074 rollback. Removes the AI edit proposal table and its
-- registry row.

DROP TABLE IF EXISTS knowledge_crdt_ai_edit_proposals;
DELETE FROM knowledge_schema_registry WHERE family_key = 'crdt_ai_edit_proposals';
