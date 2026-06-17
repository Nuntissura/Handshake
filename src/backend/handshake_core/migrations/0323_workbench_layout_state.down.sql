DELETE FROM knowledge_schema_registry
 WHERE family_key = 'workbench_layout_state';

DROP TABLE IF EXISTS knowledge_workbench_layout_states;
