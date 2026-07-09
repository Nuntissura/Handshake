-- Down: drop the WP-KERNEL-012 MT-067 calendar activity-span store.
DROP INDEX IF EXISTS ix_calendar_activity_spans_workspace;
DROP INDEX IF EXISTS ix_calendar_activity_spans_event;
DROP TABLE IF EXISTS calendar_activity_spans;
