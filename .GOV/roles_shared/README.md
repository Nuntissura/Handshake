# Shared Governance Bundle

This directory holds cross-role truth and the shared implementation surfaces that multiple roles rely on.

Authoritative folder law lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus the active role protocols. This README is navigational only.

## Shared Bucket Map

- `docs/`
  - active shared guidance such as onboarding, architecture, debug, workflow, and quality-gate docs
- `records/`
  - authoritative shared ledgers, registries, and pointers
- `runtime/`
  - repo-local machine-written governance state that intentionally remains versioned in-repo
  - allowed local exceptions are narrow: `runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json` plus archive-only `runtime/validator_gates/`
  - live launch/control/WP-communication/topology runtime must stay external under `../gov_runtime/roles_shared/`
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
- Runtime classes:
  - authoritative live runtime: external session registry, launch/control ledgers, packet communication ledgers, and live validator gate state under `../gov_runtime/roles_shared/`
  - diagnostic runtime: broker state and per-command control output logs under the same external runtime root
  - versioned repo-local exception: `runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`
  - archive-only local reference: `runtime/validator_gates/`
- Stale live runtime residue does not belong under `/.GOV/roles_shared/runtime/`. Remove it or move it out of the active runtime bucket; do not create new repo-local live runtime side channels.
- external `roles_shared/ROLE_SESSION_REGISTRY.json`
- external `roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- external `roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- external `roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- external `roles_shared/SESSION_CONTROL_OUTPUTS/`
- external `roles_shared/WP_COMMUNICATIONS/`
  - includes per-WP truth bundle detail artifacts under `WP_COMMUNICATIONS/<WP_ID>/truth_bundle/`
  - includes per-WP baseline compile/scope waiver ledgers written by `just wp-waiver-record`
  - includes per-WP cost governor override ledgers when `--override-recovery=<reason>` is used
- external `roles_shared/validator_gates/`
- `runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`
- `runtime/validator_gates/`
  - archive-only legacy reference; new live validator gate writes are forbidden here

Historical/reference studies no longer belong in this directory root. Shared non-authoritative reference material belongs under `.GOV/reference/`.
