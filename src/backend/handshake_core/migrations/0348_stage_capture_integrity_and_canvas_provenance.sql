-- WP-KERNEL-012 MT-066: repair the exact-byte integrity metadata recovered by
-- 0346 and make Canvas Stage capture placement idempotency authoritative in
-- PostgreSQL rather than process-local frontend state.

-- 0346 recovered the only available bytes for legacy 0341 rows from
-- content_json::text.  Recompute every byte-derived field from those recovered
-- bytes so native fetch cannot observe the old canonical-JSON digest/size next
-- to a different byte payload.
UPDATE stage_capture_artifacts
SET
    content_sha256 = encode(digest(content_bytes, 'sha256'), 'hex'),
    size_bytes = octet_length(content_bytes),
    manifest = jsonb_set(
        jsonb_set(
            COALESCE(manifest, '{}'::jsonb),
            '{sha256}',
            to_jsonb(encode(digest(content_bytes, 'sha256'), 'hex')),
            true
        ),
        '{size_bytes}',
        to_jsonb(octet_length(content_bytes)),
        true
    )
WHERE idempotency_key LIKE 'legacy:%';

ALTER TABLE loom_canvas_placements
    ADD COLUMN IF NOT EXISTS stage_provenance_key TEXT,
    ADD COLUMN IF NOT EXISTS stage_provenance JSONB;

ALTER TABLE loom_canvas_placements
    DROP CONSTRAINT IF EXISTS loom_canvas_placements_stage_provenance_key_check,
    ADD CONSTRAINT loom_canvas_placements_stage_provenance_key_check
        CHECK (
            (stage_provenance_key IS NULL AND stage_provenance IS NULL)
            OR (
                stage_provenance_key IS NOT NULL
                AND stage_provenance IS NOT NULL
                AND stage_provenance_key ~ '^[0-9a-f]{64}$'
                AND jsonb_typeof(stage_provenance) = 'object'
                AND stage_provenance ?& ARRAY[
                    'schema_id', 'artifact_id', 'sha256', 'manifest_ref',
                    'causal_action_id'
                ]
                AND stage_provenance - ARRAY[
                    'schema_id', 'artifact_id', 'sha256', 'manifest_ref',
                    'causal_action_id'
                ] = '{}'::jsonb
                AND stage_provenance ->> 'schema_id' = 'handshake.canvas-stage-capture-ref.v1'
                AND stage_provenance ->> 'artifact_id' IS NOT NULL
                AND stage_provenance ->> 'artifact_id' = btrim(stage_provenance ->> 'artifact_id')
                AND stage_provenance ->> 'artifact_id' <> ''
                AND stage_provenance ->> 'sha256' ~ '^[0-9a-f]{64}$'
                AND stage_provenance ->> 'manifest_ref' IS NOT NULL
                AND stage_provenance ->> 'manifest_ref' = btrim(stage_provenance ->> 'manifest_ref')
                AND stage_provenance ->> 'manifest_ref' <> ''
                AND stage_provenance ->> 'causal_action_id' IS NOT NULL
                AND stage_provenance ->> 'causal_action_id' = btrim(stage_provenance ->> 'causal_action_id')
                AND stage_provenance ->> 'causal_action_id' <> ''
            )
        );

CREATE UNIQUE INDEX IF NOT EXISTS idx_loom_canvas_stage_provenance
    ON loom_canvas_placements (workspace_id, canvas_block_id, stage_provenance_key)
    WHERE stage_provenance_key IS NOT NULL;

-- Stage Canvas compensation deletes an intentionally narrow authority tuple,
-- but several downstream tables carry logical (non-FK) references to that
-- tuple. Every such writer joins this shared advisory-lock domain and, after
-- any wait, rechecks the append-only compensation tombstone. Compensation
-- holds the matching exclusive locks through event append and deletion. This
-- closes both writer-first and compensation-first races, including direct SQL.
CREATE OR REPLACE FUNCTION guard_stage_compensated_references(
    guarded_workspace_id TEXT,
    guarded_refs TEXT[]
) RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
    guarded_ref TEXT;
    guarded_identity TEXT;
BEGIN
    IF guarded_workspace_id IS NULL OR btrim(guarded_workspace_id) = '' THEN
        RETURN;
    END IF;
    FOREACH guarded_ref IN ARRAY ARRAY(
        SELECT DISTINCT candidate
        FROM unnest(guarded_refs) AS refs(candidate)
        WHERE candidate IS NOT NULL AND btrim(candidate) <> ''
        ORDER BY candidate
    ) LOOP
        PERFORM pg_advisory_xact_lock_shared(
            hashtextextended(
                'stage-logical-ref' || chr(31) || guarded_workspace_id || chr(31) || guarded_ref,
                32066::bigint
            )
        );
        guarded_identity := substring(guarded_ref FROM position(':' IN guarded_ref) + 1);
        IF (
            guarded_ref LIKE 'block:%'
            AND EXISTS (
                SELECT 1
                FROM kernel_event_ledger
                WHERE source_component = 'loom_canvas_stage_compensation'
                  AND aggregate_type = 'knowledge_rich_document'
                  AND aggregate_id = guarded_identity
                  AND payload ->> 'workspace_id' = guarded_workspace_id
            )
        ) OR (
            guarded_ref LIKE 'entity:%'
            AND EXISTS (
                SELECT 1
                FROM kernel_event_ledger
                WHERE source_component = 'loom_canvas_stage_compensation'
                  AND payload ->> 'workspace_id' = guarded_workspace_id
                  AND payload ->> 'entity_id' = guarded_identity
            )
        ) OR (
            guarded_ref LIKE 'title:%'
            AND EXISTS (
                SELECT 1
                FROM kernel_event_ledger
                WHERE source_component = 'loom_canvas_stage_compensation'
                  AND payload ->> 'workspace_id' = guarded_workspace_id
                  AND payload ->> 'title' = guarded_identity
            )
            AND NOT EXISTS (
                SELECT 1
                FROM knowledge_rich_documents
                WHERE workspace_id = guarded_workspace_id
                  AND title = guarded_identity
                  AND deleted_at IS NULL
            )
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = '23503',
                MESSAGE = 'reference targets a compensated Stage Canvas authority';
        END IF;
    END LOOP;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_document_backlink_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM guard_stage_compensated_references(
        NEW.workspace_id,
        ARRAY[
            'block:' || NEW.source_document_id,
            'block:' || NEW.target,
            'title:' || NEW.target
        ]
    );
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_fems_memory_proposal_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM guard_stage_compensated_references(
        NEW.workspace_id,
        ARRAY['block:' || NEW.document_id]
    );
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_quick_switcher_recent_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM guard_stage_compensated_references(
        NEW.workspace_id,
        ARRAY[
            CASE WHEN NEW.source_kind = 'loom_block'
                THEN 'block:' || NEW.ref_id ELSE NULL END,
            CASE WHEN NEW.result_kind = 'knowledge_entity'
                THEN 'entity:' || NEW.ref_id ELSE NULL END
        ]
    );
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_context_bundle_item_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    guarded_workspace_id TEXT;
BEGIN
    IF NEW.ref_kind = 'entity' THEN
        SELECT workspace_id INTO guarded_workspace_id
        FROM knowledge_context_bundles
        WHERE bundle_id = NEW.bundle_id;
        PERFORM guard_stage_compensated_references(
            guarded_workspace_id,
            ARRAY['entity:' || NEW.ref_id]
        );
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_knowledge_source_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.source_kind = 'rich_document'
       AND COALESCE(NEW.provenance ->> 'rich_document_id', '') <> '' THEN
        PERFORM guard_stage_compensated_references(
            NEW.workspace_id,
            ARRAY['block:' || (NEW.provenance ->> 'rich_document_id')]
        );
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_loom_ai_suggestion_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM guard_stage_compensated_references(
        NEW.workspace_id,
        ARRAY[
            'block:' || NEW.block_id,
            CASE WHEN NEW.target_block_id IS NULL
                THEN NULL ELSE 'block:' || NEW.target_block_id END
        ]
    );
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION guard_stage_loom_edge_text_reference()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.source_text_block_id IS NOT NULL THEN
        PERFORM guard_stage_compensated_references(
            NEW.workspace_id,
            ARRAY['block:' || NEW.source_text_block_id]
        );
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_guard_stage_fems_memory_proposal_reference
    ON fems_memory_proposals;
CREATE TRIGGER trg_guard_stage_fems_memory_proposal_reference
BEFORE INSERT OR UPDATE OF workspace_id, document_id ON fems_memory_proposals
FOR EACH ROW EXECUTE FUNCTION guard_stage_fems_memory_proposal_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_document_backlink_reference
    ON knowledge_document_backlinks;
CREATE TRIGGER trg_guard_stage_document_backlink_reference
BEFORE INSERT OR UPDATE OF workspace_id, source_document_id, target
ON knowledge_document_backlinks
FOR EACH ROW EXECUTE FUNCTION guard_stage_document_backlink_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_context_bundle_item_reference
    ON knowledge_context_bundle_items;
CREATE TRIGGER trg_guard_stage_context_bundle_item_reference
BEFORE INSERT OR UPDATE OF bundle_id, ref_kind, ref_id ON knowledge_context_bundle_items
FOR EACH ROW EXECUTE FUNCTION guard_stage_context_bundle_item_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_quick_switcher_recent_reference
    ON knowledge_quick_switcher_recents;
CREATE TRIGGER trg_guard_stage_quick_switcher_recent_reference
BEFORE INSERT OR UPDATE OF workspace_id, source_kind, ref_id, result_kind
ON knowledge_quick_switcher_recents
FOR EACH ROW EXECUTE FUNCTION guard_stage_quick_switcher_recent_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_knowledge_source_reference
    ON knowledge_sources;
CREATE TRIGGER trg_guard_stage_knowledge_source_reference
BEFORE INSERT OR UPDATE OF workspace_id, source_kind, provenance ON knowledge_sources
FOR EACH ROW EXECUTE FUNCTION guard_stage_knowledge_source_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_loom_ai_suggestion_reference
    ON loom_ai_suggestions;
CREATE TRIGGER trg_guard_stage_loom_ai_suggestion_reference
BEFORE INSERT OR UPDATE OF workspace_id, block_id, target_block_id ON loom_ai_suggestions
FOR EACH ROW EXECUTE FUNCTION guard_stage_loom_ai_suggestion_reference();

DROP TRIGGER IF EXISTS trg_guard_stage_loom_edge_text_reference ON loom_edges;
CREATE TRIGGER trg_guard_stage_loom_edge_text_reference
BEFORE INSERT OR UPDATE OF workspace_id, source_text_block_id ON loom_edges
FOR EACH ROW EXECUTE FUNCTION guard_stage_loom_edge_text_reference();
