# Script Rationalization Log

Purpose: record governance-script deletions, merges, restorations, archives, and rehomes so cleanup work stays auditable.

This file now serves two purposes:
- event ledger: when a file was kept, merged, rehomed, archived, or restored
- archive index: what is currently archived, where it came from, what it used to do, and what replaced it

External archive root for retired repo files:
- `../../scripts_archive/`
- When a script or test is removed from the repo for rationalization, preserve the source file there for safekeeping and posterity.

Rules:
- Add one row for every script or test script changed as part of rationalization.
- Use `STATUS=PLANNED` before the code change lands.
- Update the same row to `STATUS=DONE` once the change is committed.
- If multiple files collapse into one target, add one row per input file.
- If a script is intentionally retained after review, log `ACTION=KEEP` with the reason.
- If `ACTION` archives a live repo file, add or update the matching row in the `Current Archive Inventory` section below.

## Current Archive Inventory

Purpose: current-state catalog of files already moved under `../../scripts_archive/`.

Interpretation:
- `FORMER_LIVE_ROLE` answers what the file used to do in the governed workflow.
- `REPLACED_BY` names the live surface to use now.
- historical mentions in audits, changelogs, and spec snapshots do not by themselves make an archived file live again.

| ARCHIVE_PATH | FORMER_LIVE_PATH | KIND | REPLACED_BY | FORMER_LIVE_ROLE | RETIRED_IN |
|---|---|---|---|---|---|
| `../../scripts_archive/.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs` | `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs` | test | `.GOV/roles_shared/tests/governance-command-contract.test.mjs` | narrow orchestrator command-name meta coverage; redundant once command-surface assertions were consolidated | rationalization audit wave on `2026-04-08` |
| `../../scripts_archive/.GOV/roles/orchestrator/tests/orchestrator-doc-command-surface.test.mjs` | `.GOV/roles/orchestrator/tests/orchestrator-doc-command-surface.test.mjs` | test | `.GOV/roles_shared/tests/governance-command-contract.test.mjs` | narrow orchestrator doc-command coverage; redundant once the command-contract suite became canonical | rationalization audit wave on `2026-04-08` |
| `../../scripts_archive/.GOV/roles_shared/tests/justfile-gov-root-quoting.test.mjs` | `.GOV/roles_shared/tests/justfile-gov-root-quoting.test.mjs` | test | `.GOV/roles_shared/tests/governance-command-contract.test.mjs` | one-off quoting assertion for `GOV_ROOT` node invocations in the `justfile` | rationalization audit wave on `2026-04-08` |
| `../../scripts_archive/.GOV/roles_shared/tests/protocol-alignment-check.test.mjs` | `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs` | test | `.GOV/roles_shared/tests/governance-command-contract.test.mjs` | single-command protocol-alignment probe; too narrow to justify a separate suite | rationalization audit wave on `2026-04-08` |
| `../../scripts_archive/.GOV/roles_shared/tests/cwd-agnostic-packet-checks.test.mjs` | `.GOV/roles_shared/tests/cwd-agnostic-packet-checks.test.mjs` | test | `.GOV/roles_shared/tests/cwd-agnostic-shared-checks.test.mjs` | narrow cwd portability probe for packet checks; subsumed by broader shared portability coverage | rationalization audit wave on `2026-04-08` |
| `../../scripts_archive/.GOV/roles_shared/checks/active-lane-brief.mjs` | `.GOV/roles_shared/checks/active-lane-brief.mjs` | script | `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs` | thin CLI wrapper that printed lane/next-actor context during startup and phase-boundary flow; retired once `phase-check` and helpers called the library directly | `RGF-153` |
| `../../scripts_archive/.GOV/roles/validator/checks/integration-validator-context-brief.mjs` | `.GOV/roles/validator/checks/integration-validator-context-brief.mjs` | script | `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs` | thin CLI wrapper for the final-lane context bundle used by integration-validator closeout/startup helpers | `RGF-153` |
| `../../scripts_archive/.GOV/roles/validator/checks/validator-handoff-check.mjs` | `.GOV/roles/validator/checks/validator-handoff-check.mjs` | script | `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` | standalone HANDOFF gate that validated committed handoff context, PREPARE worktree truth, and packet hygiene before validator review | `RGF-154` |
| `../../scripts_archive/.GOV/roles/validator/checks/integration-validator-closeout-check.mjs` | `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs` | script | `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs` | standalone CLOSEOUT preflight for integration-validator topology, final-lane governance validity, and WP closeout readiness | `RGF-154` |
| `../../scripts_archive/.GOV/roles/validator/checks/validator-packet-complete.mjs` | `.GOV/roles/validator/checks/validator-packet-complete.mjs` | script | `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` | standalone packet-hygiene and closure-proof gate used before HANDOFF/VERDICT/CLOSEOUT and during final closeout sync; retired after those callers moved in-process | `RGF-155` |

| DATE_UTC | STATUS | ACTION | FILE_PATH | KIND | REPLACED_BY | REASON | SOURCE_AUDIT | COMMIT_SHA | NOTES |
|---|---|---|---|---|---|---|---|---|---|
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles/orchestrator/scripts/gov-check-feedback.mjs` | script | `N/A` | recovery/admin-adjacent helper; not safe to retire during active governance usage | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | defer until dedicated recovery-surface review |
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs` | script | `N/A` | documented batch-reset helper; treat as admin recovery surface for now | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | no merge/archive in current wave |
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs` | script | `N/A` | stub backlog remains in scope; documented script retained pending command-surface review | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | no retire decision in current wave |
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs` | script | `N/A` | live `just session-cancel` entrypoint and protocol-aligned surface | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | merge later only after command-surface rebasing |
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles_shared/checks/build-order-check.mjs` | script | `N/A` | active gov-check chain member and BUILD_ORDER-visible check | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | merge later only after gate-path review |
| 2026-04-08T10:57:35Z | DEFERRED | KEEP | `.GOV/roles/validator/checks/validator-hygiene-full.mjs` | script | `N/A` | active validator command referenced across docs/tests/packets | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | merge later only after validator command review |
| 2026-04-08T16:05:00Z | DEFERRED | KEEP | `.GOV/operator/scripts/operator-viewport-tui.mjs` | script | `N/A` | live operator viewport entrypoint via current just surface | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | shim or not, it is active now |
| 2026-04-08T10:57:35Z | DONE | MERGE_ARCHIVE_SOURCE | `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs` | test | `governance-command-contract.test.mjs` | low-value command-name meta test | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | merged into retained command-contract suite and archived under `../../scripts_archive/` |
| 2026-04-08T10:57:35Z | DONE | MERGE_ARCHIVE_SOURCE | `.GOV/roles/orchestrator/tests/orchestrator-doc-command-surface.test.mjs` | test | `governance-command-contract.test.mjs` | low-value doc command reference test | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | merged into retained command-contract suite and archived under `../../scripts_archive/` |
| 2026-04-08T10:57:35Z | DONE | MERGE_ARCHIVE_SOURCE | `.GOV/roles_shared/tests/justfile-gov-root-quoting.test.mjs` | test | `governance-command-contract.test.mjs` | narrow quoting assertion better covered in one contract suite | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | quoting assertion retained in consolidated suite; source archived under `../../scripts_archive/` |
| 2026-04-08T10:57:35Z | DONE | MERGE_ARCHIVE_SOURCE | `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs` | test | `governance-command-contract.test.mjs` | single-command success probe | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | folded into retained command-contract suite and archived under `../../scripts_archive/` |
| 2026-04-08T10:57:35Z | DONE | MERGE_ARCHIVE_SOURCE | `.GOV/roles_shared/tests/cwd-agnostic-packet-checks.test.mjs` | test | `cwd-agnostic-shared-checks.test.mjs` | overlaps the broader cwd/runtime portability coverage | `AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md` | `TBD` | packet cwd probe retained inside shared portability suite; source archived under `../../scripts_archive/` |
| 2026-04-08T16:40:00Z | DONE | REHOME_ARCHIVE_SOURCE | `.GOV/roles_shared/checks/active-lane-brief.mjs` | script | `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs` | thin CLI wrapper retired after justfile, phase planning, and portability probes moved to the library entrypoint | operator follow-on after `RGF-152` | `TBD` | source archived under `../../scripts_archive/.GOV/roles_shared/checks/active-lane-brief.mjs` |
| 2026-04-08T16:40:00Z | DONE | REHOME_ARCHIVE_SOURCE | `.GOV/roles/validator/checks/integration-validator-context-brief.mjs` | script | `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs` | thin CLI wrapper retired after justfile, phase planning, and validator path-safety coverage moved to the library entrypoint | operator follow-on after `RGF-152` | `TBD` | source archived under `../../scripts_archive/.GOV/roles/validator/checks/integration-validator-context-brief.mjs` |
| 2026-04-08T17:05:00Z | DONE | REHOME_ARCHIVE_SOURCE | `.GOV/roles/validator/checks/validator-handoff-check.mjs` | script | `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` | standalone validator handoff entrypoint retired after phase-check, just recipes, validator gate preflights, and tests switched to the shared library command surface | operator follow-on after `RGF-153` | `TBD` | source archived under `../../scripts_archive/.GOV/roles/validator/checks/validator-handoff-check.mjs` |
| 2026-04-08T17:05:00Z | DONE | REHOME_ARCHIVE_SOURCE | `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs` | script | `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs` | standalone final-lane closeout preflight retired after phase-check, just recipes, validator gates, and alignment tests switched to the shared library command surface | operator follow-on after `RGF-153` | `TBD` | source archived under `../../scripts_archive/.GOV/roles/validator/checks/integration-validator-closeout-check.mjs` |
| 2026-04-08T18:20:00Z | DONE | REHOME_ARCHIVE_SOURCE | `.GOV/roles/validator/checks/validator-packet-complete.mjs` | script | `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` | standalone packet-completeness entrypoint retired after phase-check, closeout sync, just recipes, and validator regressions switched to the shared validator governance library | operator follow-on after `RGF-154` | `TBD` | source archived under `../../scripts_archive/.GOV/roles/validator/checks/validator-packet-complete.mjs` |
