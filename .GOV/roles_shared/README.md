# Shared Governance Bundle

This directory holds cross-role truth and the shared implementation surfaces that multiple roles rely on.

## Stable Shared Truth

- `START_HERE.md`
- `SPEC_CURRENT.md`
- `ARCHITECTURE.md`
- `TASK_BOARD.md`
- `WP_TRACEABILITY_REGISTRY.md`
- `BUILD_ORDER.md`
- `SIGNATURE_AUDIT.md`
- `SPEC_DEBT_REGISTRY.md`

## Shared Runtime / Session State

- `ROLE_SESSION_REGISTRY.json`
- `SESSION_CONTROL_BROKER_STATE.json`
- `SESSION_CONTROL_REQUESTS.jsonl`
- `SESSION_CONTROL_RESULTS.jsonl`
- `SESSION_CONTROL_OUTPUTS/`
- `validator_gates/`
- `WP_COMMUNICATIONS/`

## Shared Implementation

- `checks/`
  - repo-shared governance checks and cross-role hard gates
- `scripts/`
  - repo-shared runtime helpers, topology helpers, dev scaffolds, proof/debt libraries, and WP communication tooling
- `schemas/`
  - shared governance JSON Schemas for WP communication and session-control artifacts
- `tests/`
  - shared governance tests spanning multiple roles
- `fixtures/`
  - shared fixtures and golden inputs for shared scripts/checks/tests
- `exports/role_mailbox/`
  - authoritative governance export path for leak-safe role-mailbox metadata

## Active Shared Guidance

- `ARCHITECTURE.md`
- `BOUNDARY_RULES.md`
- `MIGRATION_GUIDE.md`
- `REPO_RESILIENCE.md`
- `ROLE_SESSION_ORCHESTRATION.md`
- `ROLE_WORKFLOW_QUICKREF.md`
- `ROLE_WORKTREES.md`
- `TOOLING_GUARDRAILS.md`

## Shared Reference / Analysis Surfaces

- `PROJECT_INVARIANTS.md`
- `VALIDATOR_FILE_TOUCH_MAP.md`

Historical/reference studies no longer belong in this directory root. Shared non-authoritative reference material belongs under `.GOV/reference/`.

See:

- `.GOV/roles_shared/STRUCTURE_RULES.md`
- `.GOV/reference/README.md`
