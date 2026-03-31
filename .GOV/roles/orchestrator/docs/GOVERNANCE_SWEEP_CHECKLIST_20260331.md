# Governance Sweep Checklist 2026-03-31

**Purpose:** reverse-traceable overnight execution checklist and closeout record for the governance sweeps and operator-viewport refresh

**Primary board:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`

**Primary review:** `.GOV/Audits/smoketest/AUDIT_20260329_WORKFLOW_PROJECTION_CORRELATION_V1_SMOKETEST_PROOF_RUN_REVIEW.md`

## Traceability

- `RGF-32` -> `roles_shared` folder sweep and control-plane hardening
- `RGF-33` -> `roles/orchestrator` folder sweep and lane-mechanics hardening
- `RGF-34` -> `roles/coder` folder sweep and resume/hygiene surface hardening
- `RGF-35` -> `roles/validator` folder sweep and PASS-lane hardening
- `RGF-36` -> ACP workflow and runtime control-plane sweep
- `RGF-37` -> operator viewport modernization and interaction refresh

## Execution Order

1. Finish `RGF-32` residual shared-surface audit and regression coverage.
2. Sweep `RGF-33` orchestrator scripts/checks/tests/docs.
3. Sweep `RGF-34` coder scripts/checks/tests/docs.
4. Sweep `RGF-35` validator scripts/checks/tests/docs.
5. Sweep `RGF-36` ACP/runtime broker, session, notification, registry, and bridge surfaces.
6. Implement `RGF-37` TUI modernization after the orchestration/runtime sweep clarifies the live signal model.

## Completion Summary

- `RGF-32` completed with repo-root-safe shared helpers, repaired relay/session scoping, corrected topology/schema/doc drift, safer shared `.GOV` handling in topology helpers, and expanded regression coverage.
- `RGF-33` completed with repo-root-safe orchestrator gate/packet/worktree reads, safer prepare/launch flow, quieter and more explicit runtime steering, and a refreshed operator viewport.
- `RGF-34` completed with coder pre-work/post-work/bootstrap/skeleton surfaces aligned to compact output, repo-root-safe state reads, and reduced stale startup/command drift.
- `RGF-35` completed with validator product-target resolution, closeout/context helpers, DAL/error-code scans, packet-complete law, and gate state handling made repo-root-safe and mechanically stricter.
- `RGF-36` completed with ACP/runtime registry, relay, token, notification, and resume surfaces audited for cwd drift and stale projection behavior.
- `RGF-37` completed with a TUI-first refresh: stronger top summary, explicit `next_action`, clearer two-line session cards, quieter terminal-WP views, and improved lane/session readability without adding a web control layer.

## Audit Checklist

### `RGF-33` `roles/orchestrator`

- [x] audit startup, steering, resume, and recovery command paths
- [x] audit operator viewport/TUI data sourcing, noise suppression, and stale state presentation
- [x] audit lifecycle helpers for terminal-state, board-state, and packet-truth drift
- [x] audit command-surface docs and tests for parity with `justfile`
- [x] add/extend regression coverage for every bug found

### `RGF-34` `roles/coder`

- [x] audit pre-work and post-work behavior against live packet/runtime/scope law
- [x] audit coder resume / next-action helpers for stale state and repo-root drift
- [x] audit command surface and startup guidance for stale or duplicate instructions
- [x] add/extend regression coverage for every bug found

### `RGF-35` `roles/validator`

- [x] audit WP validator and integration-validator path split
- [x] audit PASS/FAIL/ABANDONED / OUTDATED_ONLY truth handling
- [x] audit closeout/report structure law and anti-rubber-stamp behavior
- [x] audit command-surface parity and final-lane helper correctness
- [x] add/extend regression coverage for every bug found

### `RGF-36` ACP / Runtime

- [x] audit broker start/stop, session registry, control requests/results, and notification routing
- [x] audit projection lag, stale relay behavior, and non-atomic state transitions
- [x] audit waiting-cost surfaces and repeated polling behavior
- [x] add/extend regression coverage for every bug found

### `RGF-37` Operator Viewport

- [x] reduce stale noise and prose density
- [x] strengthen lane/session focus and "next action" visibility
- [x] improve visual hierarchy, grouping, and session-state readability
- [x] keep TUI-first control model; do not add a web control-plane dependency
- [x] align monitor behavior with lessons from `ccmanager`, `coder/mux`, `agentpipe`, and `commander`

## Done Signal

- every item above is either implemented or explicitly proven unnecessary
- task board statuses updated
- smoketest review appendage updated with findings, ROI, and residual risks
- `just gov-check` passes
