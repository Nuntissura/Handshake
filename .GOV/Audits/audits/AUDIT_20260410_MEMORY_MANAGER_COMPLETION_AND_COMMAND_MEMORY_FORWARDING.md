# AUDIT_20260410_MEMORY_MANAGER_COMPLETION_AND_COMMAND_MEMORY_FORWARDING

- AUDIT_ID: `AUDIT-20260410-MEMORY-MANAGER-COMPLETION-AND-COMMAND-MEMORY-FORWARDING`
- STATUS: APPLIED
- DATE: `2026-04-10`
- SCOPE: repo-governance maintenance only
- DRIVER: follow-on after the Memory Manager ACP proof run and the operator-directed shell-memory injection work exposed three remaining defects:
  - `gov-check` still treated synthetic `WP-MEMORY-HYGIENE_<timestamp>` lanes as packet/worktree orphans
  - `just repomem ... --decisions "text (with parentheses)"` still broke in PowerShell before Node received the arguments
  - the shell-command wrapper captured structured memory, but the public memory command surface and docs were not yet hardened around that path

## Problem

`RGF-160` had moved past receipt emission, but two completion-path defects remained:

- governed packetless Memory Manager lanes could still fail `gov-check` through packet/worktree communication assumptions
- the Memory Manager closeout path still depended on a fragile `just repomem` variadic-flag wrapper in PowerShell

`RGF-168` had an initial wrapper, but it still needed the safer command-surface forwarding and canonical memory-capture integration to be reliable for real operator use.

## Applied Change

- added a reusable node argument proxy that accepts a quoted raw-flag string from `just` and forwards literal argv tokens to Node scripts without letting PowerShell interpret metacharacters first
- moved the memory command-family recipes that rely on variadic flags onto that proxy, including `repomem`, `memory-capture`, `memory-recall`, and `shell-with-memory`
- extended `governance-memory-cli.mjs capture` with optional `--topic`, `--source`, `--importance`, and `--metadata` so structured shell-command memories can flow through the canonical memory CLI instead of writing directly to SQLite from the wrapper
- updated `shell-with-memory.mjs` to use the canonical memory CLI for structured `shell-command` capture
- clarified the governed Memory Manager startup/steering prompts so completion means:
  - write the truthful `MEMORY_*` receipt(s)
  - run `just repomem close ...`
  - stop and let the governed control lane emit `SESSION_COMPLETION`
  - leave ACP `CLOSE_SESSION` as orchestrator-owned when thread retirement is actually desired
- hardened `wp-communications-check.mjs` so packetless synthetic Memory Manager communication folders are validated as packetless scaffolds instead of being flagged as orphan packet lanes

## Surfaces

- `.GOV/roles_shared/scripts/lib/node-argv-proxy.mjs`
- `.GOV/roles_shared/tests/node-argv-proxy.test.mjs`
- `justfile`
- `.GOV/roles_shared/scripts/memory/governance-memory-cli.mjs`
- `.GOV/roles_shared/scripts/memory/shell-with-memory.mjs`
- `.GOV/roles_shared/tests/shell-with-memory.test.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/tests/session-control-lib.test.mjs`
- `.GOV/roles_shared/checks/wp-communications-check.mjs`
- `.GOV/roles_shared/tests/wp-communications-check.test.mjs`
- `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `node --test .GOV/roles_shared/tests/node-argv-proxy.test.mjs`
- `node --test .GOV/roles_shared/tests/shell-with-memory.test.mjs`
- `node --test .GOV/roles_shared/tests/memory-recall.test.mjs`
- `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
- `node --test .GOV/roles_shared/tests/wp-communications-check.test.mjs`
- `node --check .GOV/roles_shared/checks/wp-communications-check.mjs`
- `just repomem close "<summary>" --decisions "Decision text with parentheses (repro) ..."` now reaches Node and fails only for the honest no-session condition instead of PowerShell parse drift
- `just shell-with-memory CODER write-output "Write-Output shell-memory-wrapper-ok" --shell powershell --on-success "Command-family success note with parentheses (safe) ..."`
- `just gov-check`

## Outcome

`RGF-160` now has an end-to-end green proof that matches the actual governed behavior: packetless Memory Manager ACP lanes emit governed `MEMORY_*` receipts, surface them to the Orchestrator, survive `gov-check`, and close their memory session cleanly without relying on brittle PowerShell flag parsing.

`RGF-168` now has a stable operator-facing shell-command memory path: command-family recall runs before ad hoc shell work, structured shell-command memory capture flows through the canonical memory CLI, and the `just` command surface no longer breaks when flag values contain PowerShell metacharacters such as parentheses.
