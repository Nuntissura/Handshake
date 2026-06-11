-- WP-KERNEL-009 MT-070 rollback. Dependent rows in later 015x migrations
-- must be rolled back first; this file removes the denial-receipt table and
-- its registry row only.

DROP TABLE IF EXISTS knowledge_crdt_denial_receipts;
DELETE FROM knowledge_schema_registry WHERE family_key = 'crdt_denial_receipts';
