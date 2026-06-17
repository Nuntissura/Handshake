-- WP-KERNEL-009 MT-257: one daily journal LoomBlock per workspace/date.
CREATE UNIQUE INDEX IF NOT EXISTS ux_loom_daily_journal_workspace_date
    ON loom_blocks (workspace_id, journal_date)
    WHERE content_type = 'journal' AND journal_date IS NOT NULL;
