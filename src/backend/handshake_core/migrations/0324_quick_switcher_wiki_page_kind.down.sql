DELETE FROM knowledge_quick_switcher_recents
WHERE source_kind = 'wiki_page'
   OR result_kind = 'wiki_page';

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
            'user_manual_page'
        )
    );

ALTER TABLE knowledge_quick_switcher_recents
    DROP CONSTRAINT IF EXISTS knowledge_quick_switcher_recents_result_kind_check;

ALTER TABLE knowledge_quick_switcher_recents
    ADD CONSTRAINT knowledge_quick_switcher_recents_result_kind_check
    CHECK (
        result_kind IN (
            'loom_block',
            'knowledge_entity',
            'user_manual_page'
        )
    );
