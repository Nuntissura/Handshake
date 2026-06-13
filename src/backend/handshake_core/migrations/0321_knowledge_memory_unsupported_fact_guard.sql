-- WP-KERNEL-009 MT-233 hardening: unsupported bundle items are typed in the
-- durable item rows, and unsupported memory facts cannot be relabeled into
-- retrieval-trusted authority. Retrieval-trusted labels (`source`, `derived`,
-- `operator_approved`) can enter the stable fact graph when the backing claim
-- is accepted; an unsupported fact has already lost its evidence basis, so
-- re-grounding must create a fresh evidence-backed fact instead of mutating the
-- unsupported row.

ALTER TABLE knowledge_context_bundle_items
    ADD COLUMN IF NOT EXISTS supported BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS unsupported_reason TEXT;

WITH context_items AS (
    SELECT
        item.bundle_id,
        item.item_ordinal,
        (ctx.elem->>'supported')::boolean AS supported,
        NULLIF(btrim(ctx.elem->>'unsupported_reason'), '') AS unsupported_reason
    FROM knowledge_context_bundle_items item
    JOIN knowledge_context_bundles bundle
      ON bundle.bundle_id = item.bundle_id
    CROSS JOIN LATERAL jsonb_array_elements(
        COALESCE(bundle.allowed_context->'items', '[]'::jsonb)
    ) WITH ORDINALITY AS ctx(elem, ord)
    WHERE item.item_ordinal = (ctx.ord - 1)
      AND ctx.elem ? 'supported'
      AND ctx.elem->>'supported' IN ('true', 'false')
)
UPDATE knowledge_context_bundle_items item
SET supported = context_items.supported,
    unsupported_reason = CASE
        WHEN context_items.supported THEN NULL
        ELSE COALESCE(
            context_items.unsupported_reason,
            NULLIF(btrim(item.unsupported_reason), ''),
            CASE
                WHEN item.citation LIKE '%@UNSUPPORTED'
                    THEN 'unsupported citation marker'
                ELSE 'unsupported context bundle item'
            END
        )
    END
FROM context_items
WHERE item.bundle_id = context_items.bundle_id
  AND item.item_ordinal = context_items.item_ordinal;

UPDATE knowledge_context_bundle_items
SET supported = FALSE,
    unsupported_reason = COALESCE(
        NULLIF(btrim(unsupported_reason), ''),
        'unsupported citation marker'
    )
WHERE citation LIKE '%@UNSUPPORTED';

UPDATE knowledge_context_bundle_items
SET unsupported_reason = NULL
WHERE supported = TRUE;

ALTER TABLE knowledge_context_bundle_items
    DROP CONSTRAINT IF EXISTS chk_knowledge_context_bundle_items_support_reason;

ALTER TABLE knowledge_context_bundle_items
    ADD CONSTRAINT chk_knowledge_context_bundle_items_support_reason
    CHECK (
        (
            supported = TRUE
            AND unsupported_reason IS NULL
            AND (citation IS NULL OR citation NOT LIKE '%@UNSUPPORTED')
        )
        OR (
            supported = FALSE
            AND unsupported_reason IS NOT NULL
            AND btrim(unsupported_reason) = unsupported_reason
            AND unsupported_reason <> ''
            AND (citation IS NULL OR citation LIKE '%@UNSUPPORTED')
        )
    );

CREATE OR REPLACE FUNCTION knowledge_memory_fact_authority_transition_guard()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.authority_label = OLD.authority_label THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label = 'operator_approved'
       AND NEW.authority_label IN ('deprecated', 'superseded', 'unsupported') THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label IN ('source', 'derived', 'model_suggested')
       AND NEW.authority_label IN (
           'source', 'derived', 'operator_approved',
           'deprecated', 'superseded', 'unsupported'
       ) THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label = 'deprecated'
       AND NEW.authority_label = 'superseded' THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label = 'superseded'
       AND NEW.authority_label = 'deprecated' THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label = 'unsupported'
       AND NEW.authority_label IN ('deprecated', 'superseded') THEN
        RETURN NEW;
    END IF;

    IF OLD.authority_label = 'unsupported'
       AND NEW.authority_label IN ('source', 'derived', 'operator_approved', 'model_suggested') THEN
        RAISE EXCEPTION
            'knowledge_memory_facts % violates MT-233: unsupported facts cannot become retrieval-trusted or probationary facts without fresh evidence',
            NEW.fact_id
            USING ERRCODE = 'check_violation';
    END IF;

    RAISE EXCEPTION
        'knowledge_memory_facts % violates MT-125/MT-233: illegal authority label transition % -> %',
        NEW.fact_id, OLD.authority_label, NEW.authority_label
        USING ERRCODE = 'check_violation';
END $$;

DROP TRIGGER IF EXISTS trg_knowledge_memory_fact_authority_transition_guard
    ON knowledge_memory_facts;

CREATE TRIGGER trg_knowledge_memory_fact_authority_transition_guard
    BEFORE UPDATE OF authority_label ON knowledge_memory_facts
    FOR EACH ROW
    EXECUTE FUNCTION knowledge_memory_fact_authority_transition_guard();
