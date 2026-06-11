-- Down migration for 0171_knowledge_code_scip_imports.
DELETE FROM knowledge_schema_registry WHERE family_key = 'code_scip_imports';
DROP TABLE IF EXISTS knowledge_code_scip_imports;
