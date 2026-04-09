# AUDIT_20260409_TRIGGER_AWARE_MEMORY_RECALL_INJECTION

- AUDIT_ID: `AUDIT-20260409-TRIGGER-AWARE-MEMORY-RECALL-INJECTION`
- STATUS: APPLIED
- DATE: `2026-04-09`
- SCOPE: repo-governance maintenance only
- DRIVER: operator follow-on after the 2026-04-09 governance memory review

## Problem

`memory-recall` already had access to role, trigger, script, and fail-capture metadata, but the live retrieval path only ranked by action keywords plus a small source-artifact boost. That left two gaps:

1. command-specific failures captured through `fail-capture-lib.mjs` were not reliably surfaced before the same command was used again
2. role-authored habit memory (`memory-capture`, intent snapshots, role-scoped repair patterns) was stored but not ranked as a first-class recall signal

## Constraint

The governance-maintenance workflow forbids editing the canonical root `justfile` from `wt-gov-kernel`. This follow-on therefore had to improve retrieval inside `memory-recall.mjs` without widening or rewriting the public command surface from the wrong worktree.

## Applied Change

- refactored `memory-recall.mjs` into an import-safe module with exported helper functions
- added action-to-role/trigger/script default hints so existing action entry points can recover command-specific context without new recipes
- added trigger-sensitive retrieval for fail-capture, trigger-authored memory, and `conversation_log.trigger_ref`
- added role-habit retrieval for role-authored `memory-capture`, intent snapshots, and role-scoped procedural repair patterns
- changed output ordering to surface `TRIGGER PITFALLS`, `ROLE HABITS`, then general findings
- added a focused regression test file for context resolution, trigger matching, role matching, and ranking

## Surfaces

- `.GOV/roles_shared/scripts/memory/memory-recall.mjs`
- `.GOV/roles_shared/tests/memory-recall.test.mjs`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `node --test .GOV/roles_shared/tests/memory-recall.test.mjs`
- `just gov-check`

## Outcome

Existing `just memory-recall <ACTION>` entry points now inject command-specific fail history and role-weighted habit memory before broader action-scoped findings, without adding a new public governance command.
