-- WP-KERNEL-009 authority-hardening #3 (ADVERSARIAL_CODE_REVIEWER-20260611-PM).
-- Affected MT: MT-079 (CrdtRecoveryReceiptFormat).
-- Contract source: MT-041 SwarmCheckpoint semantics. recover_from_checkpoint
-- previously authorized a recovery on lease EXISTENCE alone, so a recovery
-- receipt could be written under a released / expired / foreign-actor /
-- unrelated-scope lease. The Rust path now enforces all four (typed
-- RecoveryFailureV1::Lease{Released,Expired,ForeignActor,ScopeMismatch}); this
-- trigger is the schema backstop so a direct INSERT into
-- knowledge_crdt_recovery_receipts cannot bypass the lease authorization.
--
-- Enforced guarantee at recovery-receipt INSERT time, for new_lease_id:
--   1. the lease row EXISTS;
--   2. it is UNRELEASED (released_at_utc IS NULL);
--   3. it is UNEXPIRED on the DATABASE clock (expires_at_utc > NOW());
--   4. its actor_id == the receipt's new_actor_id (no foreign-actor recovery);
--   5. its scope COVERS the checkpoint scope_ref (exact match, or a
--      workspace:<ws> lease covering '<kind>:<ws>' / '<kind>:<ws>/<child>').
-- Mirrors kernel/crdt/recovery_receipt.rs::lease_scope_covers_checkpoint.

CREATE OR REPLACE FUNCTION knowledge_crdt_recovery_receipt_lease_guard()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    lease_actor TEXT;
    lease_released TIMESTAMPTZ;
    lease_expires TIMESTAMPTZ;
    lease_scope TEXT;
    ckpt_scope TEXT;
    ws TEXT;
    scope_body TEXT;
    covers BOOLEAN := FALSE;
BEGIN
    SELECT actor_id, released_at_utc, expires_at_utc,
           (scope_kind || ':' || scope_id)
      INTO lease_actor, lease_released, lease_expires, lease_scope
      FROM knowledge_crdt_agent_lane_leases
     WHERE lease_id = NEW.new_lease_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION
            'recovery receipt %: new_lease_id % does not exist',
            NEW.receipt_id, NEW.new_lease_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    IF lease_released IS NOT NULL THEN
        RAISE EXCEPTION
            'recovery receipt %: lease % is released and cannot authorize recovery',
            NEW.receipt_id, NEW.new_lease_id
            USING ERRCODE = 'check_violation';
    END IF;

    IF lease_expires <= NOW() THEN
        RAISE EXCEPTION
            'recovery receipt %: lease % is expired on the database clock and cannot authorize recovery',
            NEW.receipt_id, NEW.new_lease_id
            USING ERRCODE = 'check_violation';
    END IF;

    IF lease_actor IS DISTINCT FROM NEW.new_actor_id THEN
        RAISE EXCEPTION
            'recovery receipt %: lease % is held by % but recovery claims actor % (foreign-actor recovery)',
            NEW.receipt_id, NEW.new_lease_id, lease_actor, NEW.new_actor_id
            USING ERRCODE = 'check_violation';
    END IF;

    -- Resolve the checkpoint scope this receipt recovers.
    SELECT scope_ref INTO ckpt_scope
      FROM knowledge_crdt_swarm_checkpoints
     WHERE checkpoint_id = NEW.checkpoint_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION
            'recovery receipt %: checkpoint % does not exist',
            NEW.receipt_id, NEW.checkpoint_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    -- Scope coverage (mirror of lease_scope_covers_checkpoint).
    IF lease_scope = ckpt_scope THEN
        covers := TRUE;
    ELSIF lease_scope LIKE 'workspace:%' THEN
        ws := substring(lease_scope FROM 11);  -- after 'workspace:'
        IF position(':' IN ckpt_scope) > 0 THEN
            scope_body := substring(ckpt_scope FROM position(':' IN ckpt_scope) + 1);
            IF scope_body = ws OR scope_body LIKE (ws || '/%') THEN
                covers := TRUE;
            END IF;
        END IF;
    END IF;

    IF NOT covers THEN
        RAISE EXCEPTION
            'recovery receipt %: lease scope % does not cover checkpoint scope %',
            NEW.receipt_id, lease_scope, ckpt_scope
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_crdt_recovery_receipt_lease_guard
    ON knowledge_crdt_recovery_receipts;

CREATE TRIGGER trg_knowledge_crdt_recovery_receipt_lease_guard
    BEFORE INSERT ON knowledge_crdt_recovery_receipts
    FOR EACH ROW
    EXECUTE FUNCTION knowledge_crdt_recovery_receipt_lease_guard();
