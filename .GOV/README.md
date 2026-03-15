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
  - `.GOV/roles_shared/START_HERE.md`
  - `.GOV/roles_shared/README.md`
  - `.GOV/docs/GOVERNANCE_STRUCTURE_TARGET.md`
  - `.GOV/roles/STRUCTURE_RULES.md`
  - `.GOV/roles_shared/STRUCTURE_RULES.md`
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
  - root: only role protocol, role README, stable role state, and narrow role entrypoint docs
  - `docs/`: role-local guidance, rubrics, roadmaps, protocol gap analysis, and other non-authoritative role docs
  - `scripts/`: role-owned executable entrypoints
  - `scripts/lib/`: helper libraries used only by that role's scripts/checks
  - `checks/`: role-owned validation/enforcement entrypoints
  - `tests/`: governance tests for that role's scripts/checks
  - `fixtures/`: role-local test fixtures and golden inputs
- `roles_shared/`
  - root: active shared truth, active shared runtime/state, and active shared ledgers only
  - `scripts/`: shared executable helpers and shared libraries
  - `checks/`: cross-role/shared enforcement
  - `schemas/`: shared machine-readable governance contracts
  - `tests/`: shared governance tests spanning multiple roles
  - `fixtures/`: shared fixtures/testdata for shared scripts/checks/tests
  - `exports/`: canonical shared export surfaces
- `reference/`
  - superseded architecture papers, audits, roadmap studies, extracted spec digests, and other non-authoritative material
  - reference material may describe old paths/tokens and must not be treated as live workflow authority

## Deprecations

- See `.GOV/roles_shared/DEPRECATION_SUNSET_PLAN.md` for active compatibility surfaces and removal triggers.
