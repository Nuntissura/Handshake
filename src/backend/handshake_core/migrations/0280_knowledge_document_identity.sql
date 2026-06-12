-- WP-KERNEL-009 MT-145 RichDocumentCore RichDocumentIdentityModel.
--
-- Master Spec anchor: 2.3.13.11 (RichDocument is "a versioned ProseMirror/Tiptap
-- document JSON authority record" carrying identity, schema version, CRDT
-- snapshot refs, EventLedger promotion refs, and projection refs) and 7.1.1.8
-- (the authority layer is the versioned RichDocument schema).
--
-- MT-059 (migration 0140) already created knowledge_rich_documents with the
-- core identity: rich_document_id (KRD-...), workspace_id, optional legacy
-- document_id anchor, title, schema_version, doc_version, content_json,
-- content_sha256, CRDT refs, promotion receipt, projection_refs. This migration
-- EXTENDS that authority record with the rest of the RichDocumentIdentityModel:
--
--   * project_ref / folder_ref  - project and folder membership. These are
--     STABLE typed reference strings (e.g. a project id or a workspace-relative
--     folder path token), NEVER random absolute filesystem paths (MT-152
--     embed-reference law applied to identity). They are soft refs: the
--     project/folder surfaces are owned by other WPs, so a hard FK is not
--     created here; membership integrity is enforced at the API/service layer.
--   * authority_label - the authority classification of this document
--     ('draft' | 'promoted' | 'archived'). A RichDocument authority record is
--     the promoted authority; drafts live in the CRDT/write-box and are
--     promoted (spec 2.3.13.11). The label lets a no-context reader see whether
--     a row is a promoted authority document or a draft snapshot landed for
--     recovery.
--   * owner_actor_kind / owner_actor_id - the owning actor identity (operator,
--     local model, cloud model, validator, system). Mirrors the KernelActor
--     taxonomy so document ownership is attributable in the same vocabulary as
--     EventLedger receipts. Permission boundaries (MT-158) key off this.
--
-- All columns are additive and nullable / defaulted so the migration is safe on
-- an existing table; CHECK constraints fail closed on malformed identity.

ALTER TABLE knowledge_rich_documents
    ADD COLUMN IF NOT EXISTS project_ref TEXT,
    ADD COLUMN IF NOT EXISTS folder_ref TEXT,
    ADD COLUMN IF NOT EXISTS authority_label TEXT NOT NULL DEFAULT 'promoted',
    ADD COLUMN IF NOT EXISTS owner_actor_kind TEXT,
    ADD COLUMN IF NOT EXISTS owner_actor_id TEXT;

-- authority_label is a closed vocabulary.
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_authority_label;
ALTER TABLE knowledge_rich_documents
    ADD CONSTRAINT chk_knowledge_rich_documents_authority_label
        CHECK (authority_label IN ('draft', 'promoted', 'archived'));

-- owner_actor_kind, when present, is a closed vocabulary mirroring KernelActor.
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_owner_actor_kind;
ALTER TABLE knowledge_rich_documents
    ADD CONSTRAINT chk_knowledge_rich_documents_owner_actor_kind
        CHECK (owner_actor_kind IS NULL OR owner_actor_kind IN (
            'operator', 'local_model', 'cloud_model', 'validator', 'system'
        ));

-- An owner identity is all-or-nothing: kind without id (or id without kind) is
-- an incomplete owner and fails closed.
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_owner_pair;
ALTER TABLE knowledge_rich_documents
    ADD CONSTRAINT chk_knowledge_rich_documents_owner_pair
        CHECK ((owner_actor_kind IS NULL) = (owner_actor_id IS NULL));

-- project_ref / folder_ref, when present, must be trimmed non-empty tokens and
-- must NOT be absolute filesystem paths (no leading '/', no drive-letter form).
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_project_ref;
ALTER TABLE knowledge_rich_documents
    ADD CONSTRAINT chk_knowledge_rich_documents_project_ref
        CHECK (project_ref IS NULL OR (
            btrim(project_ref) = project_ref AND project_ref <> ''
            AND project_ref NOT LIKE '/%'
            AND project_ref !~ '^[A-Za-z]:[\\/]'
        ));
ALTER TABLE knowledge_rich_documents
    DROP CONSTRAINT IF EXISTS chk_knowledge_rich_documents_folder_ref;
ALTER TABLE knowledge_rich_documents
    ADD CONSTRAINT chk_knowledge_rich_documents_folder_ref
        CHECK (folder_ref IS NULL OR (
            btrim(folder_ref) = folder_ref AND folder_ref <> ''
            AND folder_ref NOT LIKE '/%'
            AND folder_ref !~ '^[A-Za-z]:[\\/]'
        ));

-- Membership lookups: list a project's or folder's rich documents.
CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_project
    ON knowledge_rich_documents (workspace_id, project_ref);
CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_folder
    ON knowledge_rich_documents (workspace_id, folder_ref);
