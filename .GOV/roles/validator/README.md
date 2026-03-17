# Validator Bundle

This README is navigational only.
Authoritative folder-placement law for the Validator bundle lives in `Handshake Codex v1.4.md` plus `VALIDATOR_PROTOCOL.md`.

## Active Docs

- `VALIDATOR_PROTOCOL.md`

## Legacy Reference

- `agentic/AGENTIC_PROTOCOL.md` (legacy reference only; not active validator law)

## Current / Legacy Gate State

- `.GOV/roles_shared/runtime/validator_gates/`
  - current per-WP validator gate state
- `.GOV/reference/legacy/validator/VALIDATOR_GATES.json`
  - migrated read-only legacy archive for older sessions

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
- `.GOV/roles_shared/runtime/validator_gates/`
- `.GOV/task_packets/`
- `.GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md`
- external repo-governance `roles_shared/WP_COMMUNICATIONS/`
- `docs/VALIDATOR_ANTI_GAMING_RUBRIC.md` for support-only independent-review guidance

## Role Layout

- `runtime/`
  - validator-owned machine state only; new validator-owned state belongs here
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
