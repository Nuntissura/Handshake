# MIGRATION_GUIDE (LAW) — Portable Migrations with sqlx::migrate!

Authority: Master Spec section 2.3.12 (CX-DBP-011, CX-DBP-022) and Handshake Codex v1.4.

## LAW: Portable SQL Invariants
- Use `$n` placeholders only (`$1`, `$2`, ...). `?1` / `?2` are forbidden.
- Timestamps must be `TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP`. Do not use `strftime`, `datetime('now')`, or precision hacks.
- No `CREATE TRIGGER`, `OLD.`, or `NEW.` usage. Mutation tracking lives in the application layer.
- Migrations are pure DDL; avoid backend-specific pragmas or data transforms that assume SQLite-only behaviour.
- Number migrations sequentially (`0001_`, `0002_`, ...); keep one canonical copy that runs on SQLite and PostgreSQL.

## LAW: Migration Framework Usage
- Run migrations via `sqlx::migrate!("./migrations").run(&pool)` in the storage bootstrap.
- Rely on sqlx’s `_sqlx_migrations` tracking; do not create or maintain a manual `schema_version` table.
- Migrations must be idempotent-safe to re-run; avoid side effects that break on repeated execution.

## LAW: Validation Before Merge
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- `just validator-dal-audit` (portable SQL audit for migrations/)
- `just validator-hygiene-full`
- `just post-work WP-{id}` for the active work packet

## Checklist for New Migrations
- [ ] File name is numbered and ordered (000X_*.sql).
- [ ] Uses `$n` placeholders only.
- [ ] Timestamps use `TIMESTAMP ... DEFAULT CURRENT_TIMESTAMP`.
- [ ] No triggers or DB-specific datetime functions.
- [ ] Tested with the validation commands above.
