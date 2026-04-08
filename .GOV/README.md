# Governance Repo Map

This folder is the governed control plane for Handshake.

This README is navigational only.
Authoritative folder-placement law lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus the active role protocols.

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
  - governance audit artifacts
  - `Audits/audits/` for general audits
  - `Audits/smoke_tests/` for smoke-test reviews
- `docs_repo/`
  - repo-level governance docs, bridge/session-control documentation, and running governance logs
  - `docs_repo/tmp/` for temporary or non-authoritative scratch material only
- `adr/`
  - architecture decision records
- `tools/`
  - governed tool hosts and plugins
- `operator/`
  - operator-private workspace; non-authoritative unless explicitly designated

## Primary Authority

- `.GOV/codex/Handshake_Codex_v1.4.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

## Navigation Entry Points

- repo-level navigation:
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/README.md`
  - `.GOV/roles/README.md`
- role bundles:
  - `.GOV/roles/orchestrator/README.md`
  - `.GOV/roles/coder/README.md`
  - `.GOV/roles/validator/README.md`
- shared implementation bundles:
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/scripts/README.md`

## Deprecations

- See `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md` for active compatibility surfaces and removal triggers.
