DROP INDEX IF EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_content_json;
DROP INDEX IF EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_title;
DROP INDEX IF EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_document_id;
DROP INDEX IF EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_rich_document_id;

DELETE FROM knowledge_quick_switcher_recents
WHERE source_kind IN ('file', 'tag_hub', 'document');

ALTER TABLE knowledge_quick_switcher_recents
    DROP CONSTRAINT IF EXISTS knowledge_quick_switcher_recents_source_kind_check;

ALTER TABLE knowledge_quick_switcher_recents
    ADD CONSTRAINT knowledge_quick_switcher_recents_source_kind_check
    CHECK (
        source_kind IN (
            'loom_block',
            'symbol',
            'work_packet',
            'micro_task',
            'user_manual_page',
            'wiki_page'
        )
    );
