-- WP-KERNEL-009 MT-069 rollback. Removes the promoted-fact authority table
-- and its registry row.

DROP TABLE IF EXISTS knowledge_crdt_promoted_facts;
DELETE FROM knowledge_schema_registry WHERE family_key = 'crdt_promoted_facts';
