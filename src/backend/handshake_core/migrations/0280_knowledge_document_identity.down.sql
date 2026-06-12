-- WP-KERNEL-009 MT-145 rollback (MT-063 RollbackAndRepairMigrations).

DROP INDEX IF EXISTS idx_knowledge_rich_documents_folder;
DROP INDEX IF EXISTS idx_knowledge_rich_documents_project;

ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_folder_ref;
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_project_ref;
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_owner_pair;
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_owner_actor_kind;
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_authority_label;

ALTER TABLE knowledge_rich_documents
    DROP COLUMN IF EXISTS owner_actor_id,
    DROP COLUMN IF EXISTS owner_actor_kind,
    DROP COLUMN IF EXISTS authority_label,
    DROP COLUMN IF EXISTS folder_ref,
    DROP COLUMN IF EXISTS project_ref;
