-- Down: WP-KERNEL-009 MT-101 config language. Restore the 0170 code-only
-- language allow-list (without 'config').
ALTER TABLE knowledge_code_files
    DROP CONSTRAINT IF EXISTS knowledge_code_files_language_check;

ALTER TABLE knowledge_code_files
    ADD CONSTRAINT knowledge_code_files_language_check
    CHECK (language IN ('rust', 'javascript', 'typescript', 'tsx'));
