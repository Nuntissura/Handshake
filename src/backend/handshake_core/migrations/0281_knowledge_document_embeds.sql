-- WP-KERNEL-009 MT-152 EmbedReferenceModel + MT-153 BrokenEmbedRepairState.
--
-- Master Spec anchor: 2.3.13.11 (RichDocument embeds reference media/artifact/
-- source ids, never random absolute paths; a broken target is a repairable
-- typed node). Embeds attached to a RichDocument's embed blocks (image/video/
-- album/slideshow/file-link, MT-146) are stored here as TYPED references:
--   * ref_kind  - 'artifact' | 'media' | 'source' | 'url'
--   * ref_value - the artifact/media/source id, or a typed http(s) URL.
-- A CHECK rejects absolute filesystem paths (leading '/', drive-letter form,
-- UNC '\\', or 'file:' URLs) so an embed can never be a random absolute path.
--
-- Broken-embed repair (MT-153): repair_state defaults to 'ok'; when a target
-- fails to resolve it becomes 'broken' with a repair_reason. The repairable
-- node is DATA the editor renders as a placeholder; the API offers relink /
-- reresolve / remove actions (knowledge_document::embed::EmbedRepairAction).
--
-- Stable block identity: block_id is the MT-148 stable block id of the embed
-- block; (rich_document_id, block_id) is unique so re-saving a document upserts
-- the embed for that block in place.

CREATE TABLE IF NOT EXISTS knowledge_document_embeds (
    embed_id TEXT PRIMARY KEY,
    rich_document_id TEXT NOT NULL
        REFERENCES knowledge_rich_documents(rich_document_id) ON DELETE CASCADE,
    -- MT-148 stable block id of the embed block.
    block_id TEXT NOT NULL,
    ref_kind TEXT NOT NULL,
    ref_value TEXT NOT NULL,
    caption TEXT,
    repair_state TEXT NOT NULL DEFAULT 'ok',
    repair_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_document_embeds_id
        CHECK (embed_id ~ '^KEMB-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_document_embeds_block_id
        CHECK (btrim(block_id) = block_id AND block_id <> ''),
    CONSTRAINT chk_knowledge_document_embeds_ref_kind
        CHECK (ref_kind IN ('artifact', 'media', 'source', 'url')),
    CONSTRAINT chk_knowledge_document_embeds_repair_state
        CHECK (repair_state IN ('ok', 'broken')),
    -- A broken embed must carry a reason; an ok embed must not.
    CONSTRAINT chk_knowledge_document_embeds_repair_pair
        CHECK ((repair_state = 'broken') = (repair_reason IS NOT NULL)),
    -- Embeds are typed ids or typed http(s) URLs, NEVER absolute paths.
    CONSTRAINT chk_knowledge_document_embeds_ref_value_not_path
        CHECK (
            btrim(ref_value) = ref_value AND ref_value <> ''
            AND ref_value NOT LIKE '/%'
            AND ref_value NOT LIKE '\\\\%'
            AND lower(ref_value) NOT LIKE 'file:%'
            AND ref_value !~ '^[A-Za-z]:[\\/]'
        ),
    -- A url-kind embed must be an http(s) URL.
    CONSTRAINT chk_knowledge_document_embeds_url_scheme
        CHECK (ref_kind <> 'url' OR ref_value ~ '^https?://'),
    CONSTRAINT uq_knowledge_document_embeds_block
        UNIQUE (rich_document_id, block_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_document_embeds_document
    ON knowledge_document_embeds (rich_document_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_document_embeds_broken
    ON knowledge_document_embeds (rich_document_id)
    WHERE repair_state = 'broken';

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('document_embeds', 'knowledge_document_embeds', 'RichDocument',
     'authority', '0281_knowledge_document_embeds.sql', 'MT-152')
ON CONFLICT (family_key) DO NOTHING;
