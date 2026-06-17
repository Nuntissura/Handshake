DELETE FROM knowledge_schema_registry
 WHERE family_key = 'workspace_search_bookmark_state';

DROP TABLE IF EXISTS knowledge_workspace_search_bookmark_states;
