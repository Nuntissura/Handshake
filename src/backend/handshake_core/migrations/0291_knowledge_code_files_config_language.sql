-- WP-KERNEL-009 MT-101 hardening (deferred MEDIUM).
-- Affected MT: MT-101 (ConfigAndSchemaExtractor).
--
-- Source: ADVERSARIAL_CODE_REVIEWER-20260611-PM — config files currently get a
-- file: entity + contains edges but NO knowledge_code_files row, so the per-file
-- index-state surface (staleness MT-107, monaco lens) is blind to config
-- sources. The engine now emits a knowledge_code_files row for indexed config
-- files with language 'config'. The 0170 language CHECK only admitted the four
-- tree-sitter code languages; widen it to include 'config'.

ALTER TABLE knowledge_code_files
    DROP CONSTRAINT IF EXISTS knowledge_code_files_language_check;

ALTER TABLE knowledge_code_files
    ADD CONSTRAINT knowledge_code_files_language_check
    CHECK (language IN ('rust', 'javascript', 'typescript', 'tsx', 'config'));
