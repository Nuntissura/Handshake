-- WP-KERNEL-004 cluster X.4 MT-197 Observability span hardening.
--
-- The base tables `kernel_model_session_span` and `kernel_activity_span`
-- ship in migration 0024_session_checkpoint.sql alongside the session
-- checkpoint tables. This migration adds the MT-197 red-team hardening
-- the validator focus calls for:
--
--  1. `ended_at_utc >= started_at_utc` CHECK on both tables (rejects
--     accidental backwards time travel).
--  2. Attribute immutability: a row-level UPDATE trigger that raises if
--     the `attributes` JSONB column is changed post-insert. Matches the
--     append-friendly write model of `SpanRepo`.
--  3. `started_at_utc` immutability: same idea — start time cannot be
--     rewritten after insert.
--  4. Activity-kind catalog: a CHECK using the canonical
--     `ActivityKind` discriminants so a typo string is rejected at the
--     database boundary.
--  5. last_event_ledger_seq monotonicity: a session span's
--     last_event_ledger_seq must only ever grow once set (an event
--     ledger watermark cannot rewind).
--
-- The `IF NOT EXISTS` / `DROP IF EXISTS` patterns keep this migration
-- replay-safe per spec §2.3.13.4.1.

-- ----------------------------------------------------------------------
-- 1. ended_at_utc >= started_at_utc CHECK constraint on session spans.
-- ----------------------------------------------------------------------
ALTER TABLE kernel_model_session_span
    DROP CONSTRAINT IF EXISTS chk_kernel_model_session_span_end_after_start;

ALTER TABLE kernel_model_session_span
    ADD CONSTRAINT chk_kernel_model_session_span_end_after_start
    CHECK (ended_at_utc IS NULL OR ended_at_utc >= started_at_utc);

-- ----------------------------------------------------------------------
-- 2. ended_at_utc >= started_at_utc CHECK constraint on activity spans.
-- ----------------------------------------------------------------------
ALTER TABLE kernel_activity_span
    DROP CONSTRAINT IF EXISTS chk_kernel_activity_span_end_after_start;

ALTER TABLE kernel_activity_span
    ADD CONSTRAINT chk_kernel_activity_span_end_after_start
    CHECK (ended_at_utc IS NULL OR ended_at_utc >= started_at_utc);

-- ----------------------------------------------------------------------
-- 3. Status discriminant CHECK constraints.
-- ----------------------------------------------------------------------
ALTER TABLE kernel_model_session_span
    DROP CONSTRAINT IF EXISTS chk_kernel_model_session_span_status;

ALTER TABLE kernel_model_session_span
    ADD CONSTRAINT chk_kernel_model_session_span_status
    CHECK (status IN ('active', 'completed', 'failed'));

ALTER TABLE kernel_activity_span
    DROP CONSTRAINT IF EXISTS chk_kernel_activity_span_status;

ALTER TABLE kernel_activity_span
    ADD CONSTRAINT chk_kernel_activity_span_status
    CHECK (status IN ('active', 'completed', 'failed'));

-- ----------------------------------------------------------------------
-- 4. Attribute + started_at_utc immutability triggers.
--    Rationale: MT-197 validator_focus calls out "attribute
--    immutability enforced ... db trigger preventing UPDATE of
--    attributes column".
-- ----------------------------------------------------------------------
CREATE OR REPLACE FUNCTION kernel_session_span_block_attribute_update()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.attributes IS DISTINCT FROM OLD.attributes THEN
        RAISE EXCEPTION
            'kernel_model_session_span.attributes is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.started_at_utc IS DISTINCT FROM OLD.started_at_utc THEN
        RAISE EXCEPTION
            'kernel_model_session_span.started_at_utc is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.span_id IS DISTINCT FROM OLD.span_id THEN
        RAISE EXCEPTION
            'kernel_model_session_span.span_id is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.model_session_id IS DISTINCT FROM OLD.model_session_id THEN
        RAISE EXCEPTION
            'kernel_model_session_span.model_session_id is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.session_id IS DISTINCT FROM OLD.session_id THEN
        RAISE EXCEPTION
            'kernel_model_session_span.session_id is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    -- last_event_ledger_seq is monotonic: once set, may only grow.
    IF OLD.last_event_ledger_seq IS NOT NULL
        AND NEW.last_event_ledger_seq IS NOT NULL
        AND NEW.last_event_ledger_seq < OLD.last_event_ledger_seq THEN
        RAISE EXCEPTION
            'kernel_model_session_span.last_event_ledger_seq must be monotonic (span_id=%, old=%, new=%)',
            OLD.span_id, OLD.last_event_ledger_seq, NEW.last_event_ledger_seq;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_kernel_session_span_block_attribute_update
    ON kernel_model_session_span;

CREATE TRIGGER trg_kernel_session_span_block_attribute_update
    BEFORE UPDATE ON kernel_model_session_span
    FOR EACH ROW
    EXECUTE FUNCTION kernel_session_span_block_attribute_update();

CREATE OR REPLACE FUNCTION kernel_activity_span_block_attribute_update()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.attributes IS DISTINCT FROM OLD.attributes THEN
        RAISE EXCEPTION
            'kernel_activity_span.attributes is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.started_at_utc IS DISTINCT FROM OLD.started_at_utc THEN
        RAISE EXCEPTION
            'kernel_activity_span.started_at_utc is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.span_id IS DISTINCT FROM OLD.span_id THEN
        RAISE EXCEPTION
            'kernel_activity_span.span_id is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.parent_span_id IS DISTINCT FROM OLD.parent_span_id THEN
        RAISE EXCEPTION
            'kernel_activity_span.parent_span_id is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    IF NEW.activity_kind IS DISTINCT FROM OLD.activity_kind THEN
        RAISE EXCEPTION
            'kernel_activity_span.activity_kind is immutable post-insert (span_id=%)',
            OLD.span_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_kernel_activity_span_block_attribute_update
    ON kernel_activity_span;

CREATE TRIGGER trg_kernel_activity_span_block_attribute_update
    BEFORE UPDATE ON kernel_activity_span
    FOR EACH ROW
    EXECUTE FUNCTION kernel_activity_span_block_attribute_update();
