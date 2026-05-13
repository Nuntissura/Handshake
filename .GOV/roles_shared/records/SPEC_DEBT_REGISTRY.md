# SPEC_DEBT_REGISTRY

Rules:
- Every `SPECDEBT-*` referenced by a work packet must appear exactly once in this registry.
- Use `STATUS: OPEN` while the debt remains unresolved against the current packet/spec truth.
- Use `BLOCKING: YES` only when the debt blocks a full spec PASS for the packet.
- Keep rows append-only for history; close debt by changing `STATUS: CLOSED` in a governed follow-up edit, not by deleting the row.
- Preferred workflow: use `just spec-debt-open`, `just spec-debt-sync`, and `just spec-debt-close` instead of manual freehand edits.

## DEBT_ROWS
- DEBT_ID: SPECDEBT-KERNEL-001 | WP_ID: WP-KERNEL-001-Event-Ledger-Session-Broker-v1 | STATUS: OPEN | BLOCKING: YES | CLAUSE: Kernel V1 product authority is a Postgres EventLedger and must not use SQLite authority, cache, offline, fallback, or test authority for the first kernel slice | NOTES: Reset brief and product inspection require no-SQLite kernel authority, but indexed Master Spec modules still contain older SQLite-local-primary and cache/offline/fallback language. Resolve by enriching the split indexed spec modules before coder launch.
