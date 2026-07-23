-- WP-KERNEL-012 E3 MT-022 FolderTreeColorLabels — durable EventLedger receipts.
--
-- FAIL_V2 remediation: folder create/update/delete/member mutations in the live
-- Loom API previously persisted WITHOUT a corresponding transactional
-- EventLedger append, so a committed folder mutation could lack durable
-- evidence. This migration adds a receipt column on both folder-authority tables
-- that references the append-only kernel_event_ledger. The storage layer now
-- appends the KNOWLEDGE_LOOM_FOLDER_MUTATED event and writes the domain row in
-- ONE PostgreSQL transaction; the foreign key makes it schema-impossible for a
-- new folder row (or membership row) to reference a receipt that is not in the
-- ledger. Existing rows created before this migration keep NULL (legacy).

ALTER TABLE loom_folders
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id);

ALTER TABLE loom_folder_members
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id);

CREATE INDEX IF NOT EXISTS idx_loom_folders_event_ledger_event_id
    ON loom_folders (event_ledger_event_id);

CREATE INDEX IF NOT EXISTS idx_loom_folder_members_event_ledger_event_id
    ON loom_folder_members (event_ledger_event_id);
