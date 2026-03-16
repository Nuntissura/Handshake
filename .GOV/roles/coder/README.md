# Coder Bundle

This README is navigational only.
Authoritative folder-placement law for the Coder bundle lives in `Handshake Codex v1.4.md` plus `CODER_PROTOCOL.md`.

## Primary Docs

- `CODER_PROTOCOL.md`
- `agentic/AGENTIC_PROTOCOL.md` when sub-agent delegation is explicitly allowed by the packet

## Support / Analysis Docs

- `docs/` is non-authoritative support material and may lag current workflow law
- `docs/CODER_RUBRIC_V2.md` is the current deep-quality support rubric for coder self-review

## Role-Owned Checks / Scripts

- `scripts/coder-next.mjs`
- `checks/pre-work.mjs`
- `checks/pre-work-check.mjs`
- `checks/post-work.mjs`
- `checks/post-work-check.mjs`
- `checks/coder-skeleton-checkpoint.mjs`

## Role Layout

- `runtime/`
  - coder-owned machine state only; new role-owned state belongs here
- `scripts/`
  - coder-owned entrypoints
- `scripts/lib/`
  - coder-only helper libraries
- `checks/`
  - coder-owned enforcement/gate entrypoints
- `tests/`
  - governance tests for coder scripts/checks
- `fixtures/`
  - coder-local test data and golden inputs

## Shared Dependencies To Know

- `.GOV/roles_shared/checks/README.md`
- `.GOV/roles_shared/scripts/README.md`
- `.GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md`
- external repo-governance `roles_shared/WP_COMMUNICATIONS/`

## Key Commands

- `just coder-startup`
- `just coder-next [WP-{ID}]`
- `just pre-work WP-{ID}`
- `just coder-skeleton-checkpoint WP-{ID}`
- `just post-work WP-{ID} --range <MERGE_BASE_SHA>..HEAD`
- `just spec-debt-open WP-{ID} "<clause>" "<notes>" <YES|NO>`
- `just spec-debt-sync WP-{ID}`

## Packet Sections To Watch

- `## BOOTSTRAP`
- `## CLAUSE_CLOSURE_MATRIX`
- `## SPEC_DEBT_STATUS`
- `## SHARED_SURFACE_MONITORING`
- `## SEMANTIC_PROOF_ASSETS`
- `## EVIDENCE`
- `## STATUS_HANDOFF`
