-- WP-KERNEL-005 MT-041 hardening: direct SQL must not create unreadable refs.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_bracket_link_text_whitespace_guard_v2'
          AND conrelid = 'atelier_bracket_link_projection'::regclass
    ) THEN
        ALTER TABLE atelier_bracket_link_projection
        ADD CONSTRAINT chk_atelier_bracket_link_text_whitespace_guard_v2
            CHECK (
                raw_marker ~ '^\[\[.*\]\]$'
                AND target_id <> ''
                AND target_id !~ '^[[:space:]]|[[:space:]]$'
                AND (
                    target_label IS NULL
                    OR (
                        target_label <> ''
                        AND target_label !~ '^[[:space:]]|[[:space:]]$'
                    )
                )
            );
    END IF;
END $$;
