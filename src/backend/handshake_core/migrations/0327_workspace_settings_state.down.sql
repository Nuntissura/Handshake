DELETE FROM knowledge_schema_registry
 WHERE family_key = 'workspace_settings_state';

DROP TABLE IF EXISTS knowledge_workspace_settings_states;
