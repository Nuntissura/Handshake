-- [HSK-ACE-VAL-100] Add classification columns for ACE runtime validators
-- Sensitivity: "low", "medium", "high", "unknown" (NULL treated as "unknown" -> blocks)
-- Exportable: 0=false (local-only), 1=true (can export to cloud), NULL=true (default exportable)
-- Using INTEGER for exportable ensures portability across SQLite and PostgreSQL

ALTER TABLE blocks ADD COLUMN sensitivity TEXT DEFAULT NULL;
ALTER TABLE blocks ADD COLUMN exportable INTEGER DEFAULT 1;
