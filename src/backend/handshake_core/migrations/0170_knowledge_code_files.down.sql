-- Down migration for 0170_knowledge_code_files.
DELETE FROM knowledge_schema_registry WHERE family_key = 'code_files';
DROP TABLE IF EXISTS knowledge_code_files;
