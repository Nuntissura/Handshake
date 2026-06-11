-- WP-KERNEL-009 MT-056 hardening: KnowledgeClaim lifecycle transition guard.
-- Master Spec anchor: 2.3.13.11 KnowledgeClaim ("Claims MUST carry lifecycle
-- state (proposed, accepted, conflicted, retired) ...") — the four-state
-- lifecycle is a MUST, and `retired` is TERMINAL.
--
-- Gap closed (ADVERSARIAL_CODE_REVIEWER-20260611-PM, MEDIUM): transition
-- ordering was enforced ONLY in the app method
-- storage/knowledge.rs::transition_knowledge_claim. The 0137 table CHECKs
-- validate row SHAPE, not transition LEGALITY, so a direct
--   UPDATE knowledge_claims
--   SET lifecycle_state='accepted', retirement_reason=NULL
--   WHERE lifecycle_state='retired'
-- passes every CHECK (accepted + NULL reason is a legal shape) and RESURRECTS a
-- retired claim, violating the terminal-retired MUST. Any raw-SQL writer
-- (migration, ops script, future code path that bypasses the app method) could
-- do this silently.
--
-- Fix: a BEFORE UPDATE trigger that asserts the SAME legal transition table the
-- app enforces, at the DB layer, so the four-state lifecycle holds for every
-- writer:
--   proposed   -> accepted | conflicted | retired
--   accepted   -> conflicted | retired
--   conflicted -> accepted | retired
--   retired    -> (terminal: no transition out)
-- Self-transitions (state unchanged) are allowed so ordinary metadata updates
-- (confidence, receipts, updated_at) on a row keep working, including on a
-- retired row, as long as lifecycle_state is not moved off `retired`.
--
-- Migration range note: 0200-0209 is the WP-KERNEL-009 hardening band; the
-- original PostgresEventLedgerCore chain (0130-0142) is frozen, so this guard
-- ships as an additive migration rather than an edit to 0137.

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

CREATE TRIGGER trg_knowledge_claim_transition_guard
    BEFORE UPDATE OF lifecycle_state ON knowledge_claims
    FOR EACH ROW EXECUTE FUNCTION knowledge_claim_transition_guard();
