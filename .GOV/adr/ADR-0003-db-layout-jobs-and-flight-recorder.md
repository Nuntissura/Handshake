# ADR-0003: DB Layout - Jobs/Workspace (SQLite) and Flight Recorder (DuckDB)

- **Status:** Accepted
- **Date:** 2026-01-23
- **Context:** Handshake stores (a) application/workspace state including AI jobs and workflows, and (b) append-only observability/audit events via the Flight Recorder. These have different access patterns, retention policies, and operational needs.

## Decision
- Use **SQLite** for the primary application database at `data/handshake.db` (via `DATABASE_URL`, defaulting to `sqlite://data/handshake.db`).
- Use **DuckDB** for Flight Recorder event storage at `data/flight_recorder.db`.
- Keep both database files under the repo-local `data/` directory for Phase 1 development.

## Alternatives Considered
- **Single SQLite database for everything:** Rejected; Flight Recorder analytic/event workloads are better served by a separate store and schema contracts.
- **Single DuckDB database for everything:** Rejected; application state/migrations and OLTP access patterns are better served by SQLite.
- **Per-workspace directory layout (e.g., .handshake/):** Deferred; Phase 1 prioritizes deterministic onboarding over multi-workspace packaging.

## Consequences
- **Pros:** Clear separation of concerns; simpler retention/cleanup; avoids coupling operational logging to app-state migrations.
- **Cons:** Multiple DB files to manage/backup; developers must know which DB to inspect for which question.

## Follow-ups
- Ensure docs and tooling consistently reference `data/handshake.db` for app state and `data/flight_recorder.db` for Flight Recorder events.
- Link any future schema changes to ADR updates per "Schema Contracts" governance rules.
