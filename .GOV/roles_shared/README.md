# Shared Governance Bundle

This directory holds cross-role truth and the shared implementation surfaces that multiple roles rely on.

Authoritative folder law lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus the active role protocols. This README is navigational only.

## Shared Bucket Map

- `docs/`
  - active shared guidance such as onboarding, architecture, debug, workflow, and quality-gate docs
- `records/`
  - authoritative shared ledgers, registries, and pointers
- `runtime/`
  - repo-local machine-written governance state that intentionally remains versioned in-repo (currently governance snapshots and archive-only runtime reference material)
- `exports/`
  - canonical shared export surfaces
- `schemas/`
  - shared governance schemas
- `scripts/`
  - shared executable helpers and shared libraries
- `checks/`
  - cross-role/shared enforcement
- `tests/`
  - shared governance tests spanning multiple roles
- `fixtures/`
  - shared fixtures and golden inputs

## Shared Records

- `records/TASK_BOARD.md`
- `records/WP_TRACEABILITY_REGISTRY.md`
- `records/BUILD_ORDER.md`
- `records/SIGNATURE_AUDIT.md`
- `records/SPEC_DEBT_REGISTRY.md`
- `records/AGENT_REGISTRY.md`

## Shared Runtime

- External repo-governance runtime root:
  - default repo-relative from a repo worktree: `../gov_runtime/roles_shared/`
  - overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`
- external `roles_shared/ROLE_SESSION_REGISTRY.json`
- external `roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- external `roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- external `roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- external `roles_shared/SESSION_CONTROL_OUTPUTS/`
- external `roles_shared/WP_COMMUNICATIONS/`
- external `roles_shared/validator_gates/`
- `runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`

Historical/reference studies no longer belong in this directory root. Shared non-authoritative reference material belongs under `.GOV/reference/`.
