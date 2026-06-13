-- WP-KERNEL-009 MT-231 rollback: restore the MT-056/0200 lifecycle guard
-- shape. In reverse-order rollback this runs before 0200.down drops the
-- trigger/function entirely.

DROP TRIGGER IF EXISTS trg_knowledge_claim_conflict_resolution_receipt_guard
    ON knowledge_claim_conflicts;
DROP TRIGGER IF EXISTS trg_knowledge_claim_conflict_unresolved_state_guard
    ON knowledge_claim_conflicts;
DROP FUNCTION IF EXISTS knowledge_claim_conflict_resolution_receipt_guard();
DROP FUNCTION IF EXISTS knowledge_claim_conflict_unresolved_state_guard();
DROP INDEX IF EXISTS uq_knowledge_claim_conflicts_unordered_pair;

CREATE OR REPLACE FUNCTION knowledge_claim_transition_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- No lifecycle movement: allow (metadata-only updates, including on a
    -- terminal retired row).
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
