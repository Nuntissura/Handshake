# Validator Bundle

## Primary Docs

- `VALIDATOR_PROTOCOL.md`
- `agentic/AGENTIC_PROTOCOL.md` (reference only; current repo policy keeps validator duties non-agentic)

## Legacy Archive

- `VALIDATOR_GATES.json`
  - compatibility-only archive for older sessions
  - current per-WP validator gate state lives under `.GOV/roles_shared/validator_gates/`

## Role-Owned Checks / Scripts

- `scripts/validator-next.mjs`
- `checks/validator-handoff-check.mjs`
- `checks/validator-packet-complete.mjs`
- `checks/validator-report-structure-check.mjs`
- `checks/validator_gates.mjs`
- `checks/validator-governance-snapshot.mjs`
- `checks/validator-scan.mjs`
- `checks/validator-dal-audit.mjs`
- `checks/validator-spec-regression.mjs`
- `checks/validator-phase-gate.mjs`
- `checks/validator-error-codes.mjs`
- `checks/validator-coverage-gaps.mjs`
- `checks/validator-traceability.mjs`
- `checks/validator-git-hygiene.mjs`
- `checks/validator-hygiene-full.mjs`
- `checks/external-validator-brief.mjs`

## Shared Dependencies To Know

- `.GOV/roles_shared/checks/README.md`
- `.GOV/roles_shared/scripts/README.md`
- `.GOV/roles_shared/validator_gates/`
- `.GOV/task_packets/`
- `.GOV/roles_shared/SPEC_DEBT_REGISTRY.md`
- `.GOV/roles_shared/WP_COMMUNICATIONS/`

## Role Layout

- `scripts/`
  - validator-owned entrypoints
- `scripts/lib/`
  - validator-only helper libraries
- `checks/`
  - validator-owned enforcement/audit entrypoints
- `tests/`
  - governance tests for validator scripts/checks
- `fixtures/`
  - validator-local test data and golden inputs

## Key Commands

- `just validator-startup`
- `just validator-next [WP-{ID}]`
- `just validator-handoff-check WP-{ID}`
- `just validator-packet-complete WP-{ID}`
- `just validator-gate-present WP-{ID}`
- `just validator-gate-acknowledge WP-{ID}`
- `just validator-gate-append WP-{ID}`
- `just validator-gate-commit WP-{ID}`
