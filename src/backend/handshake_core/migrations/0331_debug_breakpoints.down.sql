-- WP-KERNEL-009 MT-254 DebugAdapterCore — down migration for durable breakpoints.
DELETE FROM knowledge_schema_registry WHERE family_key = 'debug_breakpoints';

DROP INDEX IF EXISTS idx_knowledge_debug_breakpoints_event;
DROP INDEX IF EXISTS idx_knowledge_debug_breakpoints_document;
DROP TABLE IF EXISTS knowledge_debug_breakpoints;
