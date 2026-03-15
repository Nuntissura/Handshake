# Shared Governance Bundle

This directory holds cross-role truth and the shared implementation surfaces that multiple roles rely on.

Authoritative folder law lives in `Handshake Codex v1.4.md` plus the active role protocols. This README is navigational only.

## Shared Bucket Map

- `docs/`
  - active shared guidance such as onboarding, architecture, debug, workflow, and quality-gate docs
- `records/`
  - authoritative shared ledgers, registries, and pointers
- `runtime/`
  - shared machine-written runtime state only
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

- `records/SPEC_CURRENT.md`
- `records/TASK_BOARD.md`
- `records/WP_TRACEABILITY_REGISTRY.md`
- `records/BUILD_ORDER.md`
- `records/SIGNATURE_AUDIT.md`
- `records/SPEC_DEBT_REGISTRY.md`
- `records/AGENT_REGISTRY.md`

## Shared Runtime

- `runtime/ROLE_SESSION_REGISTRY.json`
- `runtime/SESSION_CONTROL_REQUESTS.jsonl`
- `runtime/SESSION_CONTROL_RESULTS.jsonl`
- `runtime/SESSION_CONTROL_OUTPUTS/`
- `runtime/validator_gates/`
- `runtime/WP_COMMUNICATIONS/`

Historical/reference studies no longer belong in this directory root. Shared non-authoritative reference material belongs under `.GOV/reference/`.
