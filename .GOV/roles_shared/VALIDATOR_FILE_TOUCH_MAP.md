# Validator Workflow File-Touch Map (Repo Governance)

This document maps what the Validator workflow reads/writes so the Operator can:
- sanity-check scope boundaries (governance vs product),
- spot drive/host-specific path leaks early,
- and understand what a given `just ...` command depends on.

All paths in this map are repo-relative and must remain drive-agnostic.

## Always (Session Start / Context Check)

Read-only (evidence/context):
- `Handshake Codex v1.4.md`
- `AGENTS.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/SPEC_CURRENT.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/task_packets/WP-*.md` (when validating a specific WP)

Git metadata (read-only, via `git ...`):
- `.git/*` (including worktree metadata under `.git/worktrees/*` when using worktrees)

## Command Map (Validator)

### Governance-only

`just gov-check` (no product scan):
- Reads:
  - `Handshake Codex v1.4.md`
  - `AGENTS.md`
  - `.GOV/roles/**`
  - `.GOV/roles_shared/**`
  - `.GOV/scripts/**`
  - `.github/**`
  - `justfile`
- Writes: none

### WP phase/gate helpers

`just gate-check WP-...`:
- Reads:
  - `.GOV/task_packets/WP-....md`
- Writes: none

`just post-work WP-...`:
- Runs `just gate-check WP-...` first, then performs deterministic manifest checks.
- Reads:
  - `.GOV/task_packets/WP-....md`
  - `.GOV/refinements/WP-....md`
  - `.GOV/scripts/validation/cor701-spec.json`
  - `.GOV/roles_shared/TASK_BOARD.md`
  - `.GOV/roles_shared/SIGNATURE_AUDIT.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
  - `.GOV/roles/validator/VALIDATOR_GATES.json` (legacy back-compat for validator gates)
  - `.GOV/ROLE_MAILBOX/*` (export integrity gate)
  - `.git/*` (diff/status windows)
  - In-scope product files referenced in the packet manifest (typically under `src/` and/or `app/`)
- Writes: none (check-only; does not modify tracked files)

### Product-scanning (still executed by Validator role)

`just product-scan` (alias) / `just validator-scan`:
- Reads:
  - `src/backend/handshake_core/src/**/*.rs`
  - `app/src/**/*.{ts,tsx,js,jsx}`
- Writes: none

`just validator-dal-audit`:
- Reads:
  - `src/backend/handshake_core/src/**`
  - `src/backend/handshake_core/migrations/**`
- Writes: none

`just validator-git-hygiene`:
- Reads:
  - `.gitignore`
  - `.git/*` (via `git ls-files ...`)
  - Untracked file metadata for size checks
- Writes: none

`just validator-hygiene-full`:
- Composite runner that executes:
  - `just validator-scan`
  - `just validator-error-codes`
  - `just validator-traceability`
  - `just validator-git-hygiene`
- Reads/writes: union of the above commands

### Validator gate state (mechanical enforcement)

`just validator-gate-present|acknowledge|append|commit|status|reset WP-...`:
- Reads:
  - `.GOV/validator_gates/WP-....json` (per-WP gate state, if present)
  - `.GOV/roles/validator/VALIDATOR_GATES.json` (legacy back-compat read, if present)
- Writes:
  - `.GOV/validator_gates/WP-....json`

