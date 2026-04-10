# AUDIT_20260409_VISIBLE_MEMORY_INJECTION_AND_STARTUP_BOUNDS

- AUDIT_ID: `AUDIT-20260409-VISIBLE-MEMORY-INJECTION-AND-STARTUP-BOUNDS`
- STATUS: APPLIED
- DATE: `2026-04-09`
- SCOPE: repo-governance maintenance only
- DRIVER: operator follow-on after memory-system observability review

## Problem

Governance memory was being written aggressively through `memory-capture`, `fail-capture`, receipts, and snapshots, but two gaps remained:

1. governed `memory-recall` runs did not emit a compact audit line proving what was injected
2. startup prompt memory loaders existed in `session-control-lib.mjs`, but the active startup prompt builder was not appending them, leaving governed role launches effectively memory-blind at session start

## Applied Change

- added `MEMORY_INJECTION_APPLIED` summary output to `memory-recall.mjs` so every recall run reports injected memory counts plus top memory ids/topics before detailed sections
- restored startup prompt memory injection in bounded form through `buildStartupInjectionLines()` with explicit token and line caps
- kept startup prompt injection compact enough to avoid reintroducing Windows command-line bloat
- added regression coverage for the visible recall summary and the bounded startup injection helpers

## Surfaces

- `.GOV/roles_shared/scripts/memory/memory-recall.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/tests/memory-recall.test.mjs`
- `.GOV/roles_shared/tests/session-control-lib.test.mjs`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `node --test .GOV/roles_shared/tests/memory-recall.test.mjs`
- `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
- live smoke: `node .GOV/roles_shared/scripts/memory/memory-recall.mjs STEERING --wp WP-1-Session-Observability-Spans-FR-v1 --budget 280`

## Notes

`just gov-check` is currently blocked by an unrelated `worktree-concurrency-check` environment gate for `WP-1-Governance-Workflow-Mirror-v1` missing linked coder and WP-validator worktrees. That gate is not caused by this patch.

## Outcome

Governed recall is now visibly auditable in terminal output, and governed startup prompts once again receive a small bounded memory/context block instead of starting entirely without memory injection.
