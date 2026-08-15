# MIGRATION_GUIDE (LAW) — SurrealDB Schema Rollouts with SurrealKit

Authority: Master Spec section 2.3.12 (CX-DBP-011, CX-DBP-022) and Handshake Codex v1.4.

## LAW: SurrealDB Schema Invariants
- Define authority tables and fields as typed `SCHEMAFULL` records with explicit indexes and field types.
- Bind every SurrealQL parameter through the official SurrealDB Rust SDK; string-interpolated queries are forbidden.
- Default record-user table permissions to `NONE`, then grant only explicit authenticated-record-user expressions intersected with ResourceBroker scope.
- Root, namespace, database, or other system users MUST NOT be used as privacy proof because they bypass record-user table permissions.
- EventLedger mutation history remains semantic authority evidence; schema hooks or live queries MUST NOT create a second mutation history.

## LAW: Migration Framework Usage
- Apply ordered SurrealKit rollouts with explicit `start`, application cutover, `complete`, and `rollback` stages.
- Every rollout MUST be replay-safe, observable, and prove compatible reads/writes at each declared stage.
- Initialize the SurrealDB schema and EventLedger persistence from an empty authority state, verify the genesis event and required typed records, then perform a cold authority activation.
- PostgreSQL and SQLite connectivity are forbidden even for migration, compatibility, reconciliation, fixtures, tests, or proof. Retire their runtime dependencies, configuration keys, credentials, launch tooling, schema files, and database-specific validation paths.

## LAW: Validation Before Merge
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
- `just validator-dal-audit` (SurrealDB-only dependency, schema, permissions, and rollout audit)
- `just validator-hygiene-full`
- `just phase-check HANDOFF WP-{id} CODER` for the active work packet

## Checklist for New Rollouts
- [ ] SurrealKit rollout identifier and ordering are explicit and versioned.
- [ ] Authority definitions use parameterized SurrealQL through the official Rust SDK.
- [ ] Tables and fields are `SCHEMAFULL`, typed, and permissioned for authenticated record users.
- [ ] Start, application cutover, complete, rollback, interruption recovery, and idempotent rerun behavior are tested against a real disposable SurrealDB.
- [ ] Repository tripwires prove PostgreSQL and SQLite authority dependencies, connections, fixtures, and proof paths are absent.
