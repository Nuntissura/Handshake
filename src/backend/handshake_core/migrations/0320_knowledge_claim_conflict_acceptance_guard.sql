-- WP-KERNEL-009 MT-231 hardening: contradictory claims remain conflicted until
-- their claim-conflict rows carry matching EventLedger resolution receipts.
--
-- The 0200 lifecycle trigger allowed conflicted -> accepted as a shape-level
-- transition. MT-231 tightens that transition with the KnowledgeClaim
-- contract: contradictory claims remain conflicted until every conflict row
-- involving the claim is resolved with an EventLedger receipt.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM (
            SELECT
                LEAST(claim_id, conflicting_claim_id) AS claim_low,
                GREATEST(claim_id, conflicting_claim_id) AS claim_high,
                COUNT(*) AS n
            FROM knowledge_claim_conflicts
            GROUP BY 1, 2
            HAVING COUNT(*) > 1
        ) duplicates
    ) THEN
        RAISE EXCEPTION
            'knowledge_claim_conflicts has reverse duplicate pairs; resolve duplicates before MT-231 unordered-pair guard'
            USING ERRCODE = 'check_violation';
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM knowledge_claim_conflicts kcc
        JOIN kernel_event_ledger kel
          ON kel.event_id = kcc.resolution_receipt_event_id
        WHERE kcc.resolution_receipt_event_id IS NOT NULL
          AND (
              kel.aggregate_type <> 'knowledge_claim_conflict'
              OR kel.aggregate_id <> kcc.conflict_id
          )
    ) THEN
        RAISE EXCEPTION
            'knowledge_claim_conflicts has resolved rows with non-conflict EventLedger receipts; repair receipt aggregates before MT-231 guard'
            USING ERRCODE = 'check_violation';
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_claim_conflicts_unordered_pair
    ON knowledge_claim_conflicts (
        LEAST(claim_id, conflicting_claim_id),
        GREATEST(claim_id, conflicting_claim_id)
    );

-- Repair legacy/stale accepted/proposed claims that already have unresolved
-- conflict rows before installing the trigger for future writers.
UPDATE knowledge_claims kc
SET lifecycle_state = 'conflicted', updated_at = NOW()
WHERE kc.lifecycle_state IN ('proposed', 'accepted')
  AND EXISTS (
      SELECT 1
      FROM knowledge_claim_conflicts kcc
      WHERE kcc.resolved_at IS NULL
        AND (kcc.claim_id = kc.claim_id OR kcc.conflicting_claim_id = kc.claim_id)
  );

CREATE OR REPLACE FUNCTION knowledge_claim_conflict_resolution_receipt_guard()
RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    receipt_aggregate_type TEXT;
    receipt_aggregate_id TEXT;
BEGIN
    IF NEW.resolution_receipt_event_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT aggregate_type, aggregate_id
      INTO receipt_aggregate_type, receipt_aggregate_id
      FROM kernel_event_ledger
     WHERE event_id = NEW.resolution_receipt_event_id;

    -- Let the existing FK report missing ledger events.
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    IF receipt_aggregate_type <> 'knowledge_claim_conflict'
       OR receipt_aggregate_id <> NEW.conflict_id THEN
        RAISE EXCEPTION
            'knowledge_claim_conflicts % violates spec 2.3.13.11: resolution receipt % must target aggregate knowledge_claim_conflict/% (got %/%)',
            NEW.conflict_id,
            NEW.resolution_receipt_event_id,
            NEW.conflict_id,
            receipt_aggregate_type,
            receipt_aggregate_id
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS trg_knowledge_claim_conflict_resolution_receipt_guard
    ON knowledge_claim_conflicts;

CREATE TRIGGER trg_knowledge_claim_conflict_resolution_receipt_guard
    BEFORE INSERT OR UPDATE OF resolution_receipt_event_id, resolved_at
    ON knowledge_claim_conflicts
    FOR EACH ROW
    EXECUTE FUNCTION knowledge_claim_conflict_resolution_receipt_guard();

CREATE OR REPLACE FUNCTION knowledge_claim_conflict_unresolved_state_guard()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.resolved_at IS NOT NULL THEN
        RETURN NEW;
    END IF;

    UPDATE knowledge_claims
    SET lifecycle_state = 'conflicted', updated_at = NOW()
    WHERE claim_id IN (NEW.claim_id, NEW.conflicting_claim_id)
      AND lifecycle_state IN ('proposed', 'accepted');

    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS trg_knowledge_claim_conflict_unresolved_state_guard
    ON knowledge_claim_conflicts;

CREATE TRIGGER trg_knowledge_claim_conflict_unresolved_state_guard
    AFTER INSERT OR UPDATE OF claim_id, conflicting_claim_id, resolved_at
    ON knowledge_claim_conflicts
    FOR EACH ROW
    EXECUTE FUNCTION knowledge_claim_conflict_unresolved_state_guard();

CREATE OR REPLACE FUNCTION knowledge_claim_transition_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- No lifecycle movement: allow metadata-only updates, including receipt
    -- fields on a conflicted claim while the conflict-resolution path runs.
    IF NEW.lifecycle_state = OLD.lifecycle_state THEN
        RETURN NEW;
    END IF;

    -- retired is terminal: no transition off it, full stop.
    IF OLD.lifecycle_state = 'retired' THEN
        RAISE EXCEPTION
            'knowledge_claims % violates spec 2.3.13.11: retired is terminal (attempted % -> %)',
            NEW.claim_id, OLD.lifecycle_state, NEW.lifecycle_state
            USING ERRCODE = 'check_violation';
    END IF;

    -- A conflicted claim can only leave `conflicted` after all conflict rows
    -- that mention it are resolved with a matching EventLedger receipt. The
    -- conflict table enforces the receipt FK and aggregate guard, so this binds
    -- raw SQL writers to the same authority as the Rust transition API.
    IF OLD.lifecycle_state = 'conflicted'
       AND NEW.lifecycle_state IN ('accepted', 'retired') THEN
        IF EXISTS (
            SELECT 1
            FROM knowledge_claim_conflicts
            WHERE resolved_at IS NULL
              AND (claim_id = OLD.claim_id OR conflicting_claim_id = OLD.claim_id)
        ) THEN
            RAISE EXCEPTION
                'knowledge_claims % violates spec 2.3.13.11: unresolved conflicts must be receipt-resolved before exiting conflicted',
                NEW.claim_id
                USING ERRCODE = 'check_violation';
        END IF;

        IF NEW.resolution_receipt_event_id IS NULL THEN
            RAISE EXCEPTION
                'knowledge_claims % violates spec 2.3.13.11: exiting conflicted requires a conflict-resolution receipt',
                NEW.claim_id
                USING ERRCODE = 'check_violation';
        END IF;

        IF NOT EXISTS (
            SELECT 1
            FROM knowledge_claim_conflicts
            WHERE resolved_at IS NOT NULL
              AND resolution_receipt_event_id = NEW.resolution_receipt_event_id
              AND (claim_id = OLD.claim_id OR conflicting_claim_id = OLD.claim_id)
        ) THEN
            RAISE EXCEPTION
                'knowledge_claims % violates spec 2.3.13.11: exiting conflicted receipt must match a resolved conflict for the claim',
                NEW.claim_id
                USING ERRCODE = 'check_violation';
        END IF;

        IF NOT EXISTS (
            SELECT 1
            FROM knowledge_claim_conflicts kcc
            JOIN kernel_event_ledger kel
              ON kel.event_id = kcc.resolution_receipt_event_id
            WHERE kcc.resolved_at IS NOT NULL
              AND kcc.resolution_receipt_event_id = NEW.resolution_receipt_event_id
              AND (kcc.claim_id = OLD.claim_id OR kcc.conflicting_claim_id = OLD.claim_id)
              AND kel.aggregate_type = 'knowledge_claim_conflict'
              AND kel.aggregate_id = kcc.conflict_id
        ) THEN
            RAISE EXCEPTION
                'knowledge_claims % violates spec 2.3.13.11: conflict resolution receipt aggregate mismatch',
                NEW.claim_id
                USING ERRCODE = 'check_violation';
        END IF;
    END IF;

    -- Legal forward transitions for non-terminal states.
    IF (OLD.lifecycle_state, NEW.lifecycle_state) IN (
        ('proposed',   'accepted'),
        ('proposed',   'conflicted'),
        ('proposed',   'retired'),
        ('accepted',   'conflicted'),
        ('accepted',   'retired'),
        ('conflicted', 'accepted'),
        ('conflicted', 'retired')
    ) THEN
        RETURN NEW;
    END IF;

    RAISE EXCEPTION
        'knowledge_claims % violates spec 2.3.13.11: illegal lifecycle transition % -> %',
        NEW.claim_id, OLD.lifecycle_state, NEW.lifecycle_state
        USING ERRCODE = 'check_violation';
END $$;
