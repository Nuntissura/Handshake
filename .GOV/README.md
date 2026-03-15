# Governance Repo Map

This folder is the governed control plane for Handshake.

## Ownership Model

- `roles/`
  - role-owned protocols, role-local state, and role-scoped implementation surfaces
  - `roles/orchestrator/`
  - `roles/coder/`
  - `roles/validator/`
- `roles_shared/`
  - shared truth, shared runtime state, shared ledgers, shared checks, and shared scripts
  - `roles_shared/checks/`
  - `roles_shared/scripts/`
  - `roles_shared/exports/`
  - `roles_shared/schemas/`
- `reference/`
  - historical, analytical, roadmap, and non-authoritative reference material
- `task_packets/`
  - executable WP contracts and stub backlog
- `refinements/`
  - signed technical refinements used to hydrate packets
- `templates/`
  - packet/refinement/audit/WP communication templates
- `Audits/`
  - audits and live-smoketest reviews
- `docs/`
  - governance architecture notes and bridge/session-control documentation
- `adr/`
  - architecture decision records
- `tools/`
  - governed tool hosts and plugins

## Start Here

- repo-level navigation:
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/README.md`
  - `Handshake Codex v1.4.md`
  - `.GOV/roles/README.md`
- role bundles:
  - `.GOV/roles/orchestrator/README.md`
  - `.GOV/roles/coder/README.md`
  - `.GOV/roles/validator/README.md`
- shared implementation bundles:
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/scripts/README.md`

## Current Workflow Authority

- manual relay and orchestrator-managed workflows still use the existing `just ...` entrypoints
- role-owned implementation now lives under `roles/<role>/scripts` and `roles/<role>/checks`
- role-owned non-law/support docs now belong under `roles/<role>/docs/`
- validator gate state now lives under `roles_shared/validator_gates`
- repo-shared implementation now lives under `roles_shared/scripts` and `roles_shared/checks`, including git-hook plumbing under `roles_shared/scripts/hooks`
- shared historical/reference material belongs under `.GOV/reference/`, not `roles_shared/`
- task packets and signed refinements remain the authoritative workflow contract

## Folder Law

- `roles/<role>/`
  - root: role protocol, role README, and fixed role-local subfolders
  - `docs/`: role-local guidance, rubrics, roadmaps, protocol gap analysis, and other non-authoritative role docs
  - `runtime/`: role-owned machine state only; new role-owned state belongs here
  - `scripts/`: role-owned executable entrypoints
  - `scripts/lib/`: helper libraries used only by that role's scripts/checks
  - `checks/`: role-owned validation/enforcement entrypoints
  - `tests/`: governance tests for that role's scripts/checks
  - `fixtures/`: role-local test fixtures and golden inputs
- `roles_shared/`
  - root: `README.md` plus the canonical shared buckets
  - `docs/`: active shared guidance
  - `records/`: authoritative shared ledgers, registries, and pointers
  - `runtime/`: shared machine-written runtime state only
  - `exports/`: canonical shared export surfaces
  - `schemas/`: shared governance schemas
  - `scripts/`: shared executable helpers and shared libraries
  - `checks/`: cross-role/shared enforcement
  - `tests/`: shared governance tests spanning multiple roles
  - `fixtures/`: shared fixtures/testdata for shared scripts/checks/tests
- `docs/`
  - repo-level governance docs that do not belong to a single role bundle or the shared bundle
  - temporary/non-authoritative material belongs only in a clearly named scratch subfolder and must not affect workflow execution unless explicitly designated
- `operator/`
  - operator-private workspace; non-authoritative unless the Operator explicitly designates a specific file for the current task
- `reference/`
  - superseded architecture papers, audits, roadmap studies, extracted spec digests, and other non-authoritative material
  - reference material may describe old paths/tokens and must not be treated as live workflow authority

## Deprecations

- See `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md` for active compatibility surfaces and removal triggers.
