# Script Rationalization Log

Purpose: record governance-script deletions, merges, restorations, archives, and rehomes so cleanup work stays auditable.

External archive root for retired repo files:
- `../../scripts_archive/`
- When a script or test is removed from the repo for rationalization, preserve the source file there for safekeeping and posterity.

Rules:
- Add one row for every script or test script changed as part of rationalization.
- Use `STATUS=PLANNED` before the code change lands.
- Update the same row to `STATUS=DONE` once the change is committed.
- If multiple files collapse into one target, add one row per input file.
- If a script is intentionally retained after review, log `ACTION=KEEP` with the reason.

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
