# Governance Structure Target

This document freezes the target placement model for the governance repo so future moves are driven by one end-state instead of ad hoc reshuffling.

## Root Model

`/.GOV/` root should be limited to long-lived top-level domains:

- `roles/`
- `roles_shared/`
- `reference/`
- `task_packets/`
- `refinements/`
- `templates/`
- `Audits/`
- `docs/`
- `adr/`
- `tools/`
- `README.md`

The following root surfaces are transitional hotspots and should not remain as permanent homes:

- `Papers/`
- `agents/`

## Explicit Exclusion

`/.GOV/operator/` is operator-private workspace material and is not part of the governed repo-structure migration target unless the Operator explicitly asks to move or normalize it.

## Target Buckets

### `roles/<role>/`

Role root should contain only:

- `README.md`
- `<ROLE>_PROTOCOL.md`
- stable role-owned runtime state only when it is truly role-local

Role subfolders:

- `docs/`
- `runtime/`
- `tooling/commands/`
- `tooling/checks/`
- `tooling/tests/`
- `tooling/fixtures/`
- `tooling/lib/`

### `roles_shared/`

`roles_shared/` root should contain almost no loose files. Shared surfaces should be grouped by lifecycle:

- `docs/`
- `records/`
- `runtime/`
- `exports/`
- `schemas/`
- `tooling/commands/`
- `tooling/checks/`
- `tooling/tests/`
- `tooling/fixtures/`
- `tooling/lib/`

Examples:

- active guidance -> `roles_shared/docs/`
- shared ledgers and registries -> `roles_shared/records/`
- JSON/JSONL session state and machine outputs -> `roles_shared/runtime/`
- shared executable governance code -> `roles_shared/tooling/`

### `reference/`

Non-authoritative or historical material belongs under `reference/`, including:

- migration-era notes
- archaeology
- superseded compatibility evidence
- extracted studies and papers

## Current Audit Focus

The structure audit currently reports these hotspot classes:

1. overloaded `roles_shared/` root
2. root-level `Papers/` and `agents/`
3. role-root files that should live under `runtime/` or `reference/legacy/`
4. duplicate template surfaces outside `/.GOV/templates/`

## Command Surface

- report-only audit: `just governance-structure-audit`
- strict mode: `just governance-structure-check`

Report-only mode is for migration planning. Strict mode is for the future end-state after the remaining hotspots are moved.
