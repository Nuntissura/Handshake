# Shared Structure Rules

This file defines the canonical folder law for `/.GOV/roles_shared/`.

## Root Of `roles_shared/`

Keep the root limited to:

- active shared truth
- active shared runtime/state
- active shared ledgers
- active shared guidance that multiple roles rely on directly

Do not leave historical studies, roadmap audits, superseded analyses, or extracted reference bundles in the `roles_shared/` root.

## Canonical Subfolders

- `checks/`
  - shared enforcement and cross-role governance checks
- `scripts/`
  - shared executable helpers and shared helper libraries
- `schemas/`
  - shared machine-readable governance contracts for shared runtime artifacts and ledgers
- `tests/`
  - shared governance tests spanning multiple roles
- `fixtures/`
  - shared fixtures, golden inputs, and test data
- `exports/`
  - canonical shared export surfaces
- `validator_gates/`
  - shared per-WP validator gate state
- `WP_COMMUNICATIONS/`
  - shared per-WP communication/runtime artifacts

## Historical / Reference Material

- Shared historical/reference material belongs under `/.GOV/reference/`, not `/.GOV/roles_shared/`.
- Reference material may describe superseded paths or migration state and must not be treated as current workflow authority.
