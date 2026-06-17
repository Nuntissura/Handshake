DELETE FROM knowledge_schema_registry
 WHERE family_key = 'quick_switcher_recents';

DROP TABLE IF EXISTS knowledge_quick_switcher_recents;
