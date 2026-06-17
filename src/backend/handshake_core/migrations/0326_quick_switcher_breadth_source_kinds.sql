-- WP-KERNEL-009 MT-256 QuickSwitcher breadth source kinds.
-- Existing databases with 0322/0324 applied need their recents constraint
-- widened for first-class file, tag_hub, and standalone RichDocument hits.

ALTER TABLE knowledge_quick_switcher_recents
    DROP CONSTRAINT IF EXISTS knowledge_quick_switcher_recents_source_kind_check;

ALTER TABLE knowledge_quick_switcher_recents
    ADD CONSTRAINT knowledge_quick_switcher_recents_source_kind_check
    CHECK (
        source_kind IN (
            'loom_block',
            'file',
            'tag_hub',
            'document',
            'symbol',
            'work_packet',
            'micro_task',
            'user_manual_page',
            'wiki_page'
        )
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_rich_document_id
    ON knowledge_rich_documents USING gin (lower(rich_document_id) public.gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_document_id
    ON knowledge_rich_documents USING gin (lower(COALESCE(document_id, '')) public.gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_title
    ON knowledge_rich_documents USING gin (lower(title) public.gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_quick_switcher_fuzzy_content_json
    ON knowledge_rich_documents USING gin (lower(content_json::text) public.gin_trgm_ops);
