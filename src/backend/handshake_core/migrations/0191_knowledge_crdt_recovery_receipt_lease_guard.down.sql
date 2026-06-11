-- Down: WP-KERNEL-009 authority-hardening #3 recovery-receipt lease guard.
DROP TRIGGER IF EXISTS trg_knowledge_crdt_recovery_receipt_lease_guard
    ON knowledge_crdt_recovery_receipts;
DROP FUNCTION IF EXISTS knowledge_crdt_recovery_receipt_lease_guard();
