-- WP-KERNEL-009 MT-140 SemanticCatalogBridge (down).
DELETE FROM knowledge_schema_registry WHERE family_key = 'semantic_catalog_entries';
DROP TABLE IF EXISTS knowledge_semantic_catalog_entries;
