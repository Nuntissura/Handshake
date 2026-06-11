-- WP-KERNEL-009 authority-hardening #1/#7 (ADVERSARIAL_CODE_REVIEWER-20260611-PM).
-- Affected MTs: MT-069 (promotion bridge), MT-068 (proposal soft refs), MT-077/MT-080 (proof).
-- Spec anchor: 02-system-architecture.md 2.3.13.11 — "KnowledgeClaim ... Claims
-- MUST carry ... evidence spans". A promoted fact (authority_class 'authority',
-- record family KnowledgeClaim, migration 0153) MUST NOT carry a dangling
-- evidence ref. The proposal table (0152) only CHECKs span-array non-emptiness
-- and permits soft 'pending:<...>' markers and 'KSP-' ids a later re-index may
-- retire; the promotion bridge (MT-069) is the authority gate.
--
-- Before this migration the gate did NOT re-validate spans: claim_promotion.rs
-- copied proposal.source_span_refs verbatim into the authority row, so a
-- proposal citing 'pending:<x>', a non-existent / foreign-workspace / retired
-- 'KSP-' id became durable authority with broken evidence. The Rust resolver
-- (storage::knowledge_crdt::validate_promotion_span_refs) now denies such
-- promotions with a durable receipt; THIS trigger is the schema backstop so the
-- hole is unreachable even via a direct INSERT that bypasses the Rust gate.
--
-- Enforced guarantee (per element of source_span_refs):
--   1. it is a canonical 'KSP-<32 lowercase hex>' id (NO 'pending:' markers,
--      no malformed refs reach authority);
--   2. a knowledge_spans row with that span_id EXISTS;
--   3. its source (knowledge_spans.source_id -> knowledge_sources) is in the
--      SAME workspace_id as the fact;
--   4. that source is NOT stale (knowledge_sources.stale = false) — i.e. the
--      span has not been superseded/retired by a newer index run.
-- Any violation raises and aborts the INSERT (and thus the whole promotion tx).

CREATE OR REPLACE FUNCTION knowledge_crdt_promoted_fact_span_evidence_guard()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    span_ref TEXT;
    span_ws TEXT;
    span_stale BOOLEAN;
BEGIN
    -- 0153 already CHECKs this is a non-empty JSON array; iterate its elements.
    FOR span_ref IN
        SELECT jsonb_array_elements_text(NEW.source_span_refs)
    LOOP
        -- (1) canonical KSP- id only; reject pending:/malformed refs.
        IF span_ref !~ '^KSP-[0-9a-f]{32}$' THEN
            RAISE EXCEPTION
                'promoted fact %: source span ref % is not a canonical KSP- id (pending markers and malformed refs cannot become authority)',
                NEW.fact_id, span_ref
                USING ERRCODE = 'check_violation';
        END IF;

        -- (2)+(3) span must exist and resolve to a same-workspace source.
        SELECT src.workspace_id, src.stale
          INTO span_ws, span_stale
          FROM knowledge_spans sp
          JOIN knowledge_sources src ON src.source_id = sp.source_id
         WHERE sp.span_id = span_ref;

        IF NOT FOUND THEN
            RAISE EXCEPTION
                'promoted fact %: source span ref % does not resolve to an existing knowledge_spans row',
                NEW.fact_id, span_ref
                USING ERRCODE = 'foreign_key_violation';
        END IF;

        IF span_ws IS DISTINCT FROM NEW.workspace_id THEN
            RAISE EXCEPTION
                'promoted fact %: source span ref % belongs to workspace % but the fact is in workspace % (cross-workspace evidence)',
                NEW.fact_id, span_ref, span_ws, NEW.workspace_id
                USING ERRCODE = 'check_violation';
        END IF;

        -- (4) source must not be retired by a newer index run.
        IF span_stale THEN
            RAISE EXCEPTION
                'promoted fact %: source span ref % is anchored to a stale (retired) source and cannot be durable evidence',
                NEW.fact_id, span_ref
                USING ERRCODE = 'check_violation';
        END IF;
    END LOOP;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_crdt_promoted_fact_span_evidence
    ON knowledge_crdt_promoted_facts;

CREATE TRIGGER trg_knowledge_crdt_promoted_fact_span_evidence
    BEFORE INSERT ON knowledge_crdt_promoted_facts
    FOR EACH ROW
    EXECUTE FUNCTION knowledge_crdt_promoted_fact_span_evidence_guard();
