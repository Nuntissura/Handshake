# Validator Bundle

This README is navigational only.
Authoritative folder-placement law for the Validator bundle lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus `VALIDATOR_PROTOCOL.md`.

## Active Docs

- `VALIDATOR_PROTOCOL.md`

## Legacy Reference

- `agentic/AGENTIC_PROTOCOL.md` (legacy reference only; not active validator law)

## Current / Legacy Gate State

- external `../gov_runtime/roles_shared/validator_gates/`
  - current per-WP validator gate state
- `.GOV/reference/legacy/validator/VALIDATOR_GATES.json`
  - migrated read-only legacy archive for older sessions

## Shared Dependencies To Know

- external `../gov_runtime/roles_shared/validator_gates/`
- logical `.GOV/work_packets/` (current physical storage: `.GOV/task_packets/`)
- `.GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md`
- external repo-governance `roles_shared/WP_COMMUNICATIONS/`
- `docs/VALIDATOR_ANTI_GAMING_RUBRIC.md` for support-only independent-review guidance

## Role Layout

- `runtime/`
  - validator-owned machine state only; new validator-owned state belongs here

## Key Commands

- read the Codex, this protocol and the assigned MT, then continue from the MT JSON status
- phase gates (STARTUP, HANDOFF, VERDICT, CLOSEOUT): not available; the corresponding obligation is checked by reading the artifact
