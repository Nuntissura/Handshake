# Governance Script Family Rationalization Plan

## METADATA
- AUDIT_ID: AUDIT-20260408-GOVERNANCE-SCRIPT-FAMILY-RATIONALIZATION-PLAN
- DATE_UTC: 2026-04-08T10:57:35Z
- AUDITOR: Codex ORCHESTRATOR
- SPEC_CURRENT_POINTER: `.GOV/spec/SPEC_CURRENT.md`
- SPEC_TARGET_RESOLVED: `.GOV/spec/Handshake_Master_Spec_v02.180.md`
- INPUT_ARTIFACTS:
  - `.GOV/Audits/audits/AUDIT_20260408_GOVERNANCE_SCRIPT_RATIONALIZATION_MATRIX.md`
  - `.GOV/Audits/audits/AUDIT_20260408_GOVERNANCE_SCRIPT_RATIONALIZATION_MATRIX_FULL.md`
- SCOPE_SUMMARY: Family-by-family cleanup plan for governance scripts and governance tests. This plan emphasizes deleting obsolete scripts, merging thin wrappers, and consolidating low-value meta tests while protecting core behavioral coverage.
- OUT_OF_SCOPE:
  - stubs
  - work packets / task packets
  - product code
  - large behavior changes to governance flow

## DECISION RULES

- Delete first when a script has no live `just` surface and no meaningful inbound/runtime usage.
- Merge first when a script is a thin alias or composite wrapper over a canonical implementation.
- Keep but split/test first when a script is central to the workflow and large enough that deletion would be reckless.
- Delete old tests only when they are low-value meta coverage and their intent is absorbed into a broader retained suite.
- When a script or governance test is retired, move it to the external archive root `../../scripts_archive/` instead of hard-deleting it.
- Operator review update (2026-04-08): Family A/B are currently treated as live or recovery-adjacent governance surfaces. Do not merge/archive them in the current wave.

## FAMILY MAP

### Family A - Drifted / Orphaned Orchestrator Commands

**Goal:** stabilize command-surface truth for recovery/admin-adjacent scripts without retiring them during active governance usage.

Files:
- `.GOV/roles/orchestrator/scripts/gov-check-feedback.mjs`
- `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
- `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs`

Evidence:
- `create-task-packet-stub.mjs` is still listed in `.GOV/roles/orchestrator/README.md:20` and `.GOV/roles_shared/records/BUILD_ORDER.md:35`, but no live `just create-task-packet-stub` recipe exists.
- `gov-check-feedback.mjs` has no live `just` recipe and only appeared in older audit prose during cross-check.
- `session-reset-batch-launch-mode.mjs` is referenced in `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`, but no live `just` recipe exposes it.

Disposition:
- `gov-check-feedback.mjs`: defer. It lacks a current `just` entrypoint, but it is a purpose-built failure-routing helper and should not be retired mid-run without a dedicated recovery/admin-surface review.
- `session-reset-batch-launch-mode.mjs`: defer. It is an explicit batch-reset helper in current orchestration docs and should be treated as admin recovery surface until reviewed in isolation.
- `create-task-packet-stub.mjs`: defer. The stub backlog remains in scope and the script is still documented; resolve supported command truth later, not in the current cleanup wave.

Recommended order:
1. confirm which admin/recovery helpers in Family A need explicit command-surface documentation
2. defer all retirement decisions for Family A until the usage audit is complete

### Family B - Thin Wrappers and Alias Commands

**Goal:** mark live wrapper/alias surfaces that may be merged later, but should remain unchanged during the current wave.

Files:
- `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs`
- `.GOV/roles_shared/checks/build-order-check.mjs`
- `.GOV/roles/validator/checks/validator-hygiene-full.mjs`
- `.GOV/operator/scripts/operator-viewport-tui.mjs`

Evidence:
- `session-control-cancel.mjs` only execs `session-control-command.mjs CANCEL_SESSION`.
- `build-order-check.mjs` only shells into `build-order-sync.mjs --check`.
- `validator-hygiene-full.mjs` only shells four other validator checks in sequence.
- `operator-viewport-tui.mjs` is a 1-hop shim into the real implementation.

Disposition:
- `session-control-cancel.mjs`: defer. It is a live `just session-cancel` entrypoint and is enforced by protocol-alignment checks.
- `build-order-check.mjs`: defer. It is part of the active `gov-check` chain and named in current BUILD_ORDER docs.
- `validator-hygiene-full.mjs`: defer. It is an actively used validator command referenced across validator docs, tests, and many work packets.
- `operator-viewport-tui.mjs`: defer. It is the current operator-facing viewport entrypoint and remains live even if it is only a shim.

Recommended order:
1. keep Family B stable during active governance usage
2. revisit merge opportunities only after the live command surface is explicitly re-baselined

### Family C - Core Orchestrator / Session Monoliths

**Goal:** no deletions yet; split for future deletions/merges around them.

Files:
- `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
- `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
- `.GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs`
- `.GOV/roles_shared/checks/gov-check.mjs`
- `.GOV/roles_shared/checks/refinement-check.mjs`
- `.GOV/roles_shared/checks/session-control-runtime-check.mjs`

Disposition:
- Keep these for now.
- Split by subcommand or concern first.
- Add explicit behavior tests before any attempt to fold or delete adjacent helpers.

Why not delete first:
- These files are hot-path governance infrastructure and have high blast radius.
- The current cleanup win is structural reduction, not raw removal.

### Family D - Shared Session / Topology Helpers

**Goal:** identify helpers that should be reviewed with their parent entrypoints rather than deleted ad hoc.

High-attention areas:
- `.GOV/roles_shared/scripts/session/`
- `.GOV/roles_shared/scripts/topology/`
- `.GOV/roles_shared/scripts/wp/`

Disposition:
- Rationalize these by parent command family, not file-by-file.
- A helper with only one inbound caller is a merge candidate only after its parent surface is stabilized.
- Do not delete session or topology helpers purely because they look low-visibility; many are live via `justfile`.

### Family E - Governance Meta Tests

**Goal:** remove old low-value test scripts from the repo by merging their intent into broader contract suites, then archive the originals externally.

Primary merge/delete candidates:
- `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs`
- `.GOV/roles/orchestrator/tests/orchestrator-doc-command-surface.test.mjs`
- `.GOV/roles_shared/tests/justfile-gov-root-quoting.test.mjs`
- `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs`
- `.GOV/roles_shared/tests/cwd-agnostic-packet-checks.test.mjs`

Evidence:
- `orchestrator-command-surface.test.mjs` only checks for required recipe names in `justfile`.
- `orchestrator-doc-command-surface.test.mjs` only checks whether docs mention commands that exist.
- `justfile-gov-root-quoting.test.mjs` is a tiny formatting/quoting assertion on the `justfile`.
- `protocol-alignment-check.test.mjs` is a single-command success probe.
- `cwd-agnostic-packet-checks.test.mjs` is tiny and overlaps conceptually with the broader `cwd-agnostic-shared-checks.test.mjs`.

Recommended merges:
1. Merge `orchestrator-command-surface.test.mjs` + `orchestrator-doc-command-surface.test.mjs` + `justfile-gov-root-quoting.test.mjs` into one `governance-command-contract.test.mjs`.
2. Merge `protocol-alignment-check.test.mjs` into the broader command-contract or alignment suite.
3. Merge `cwd-agnostic-packet-checks.test.mjs` into a broader runtime/cwd portability suite, likely alongside `cwd-agnostic-shared-checks.test.mjs`.

Keep:
- `operator-monitor-tui.test.mjs`
- `manual-relay-next.test.mjs`
- `manual-relay-envelope-lib.test.mjs`
- `session-launch-governance.test.mjs`
- behavior-heavy shared library tests such as `ensure-wp-communications.test.mjs`, `session-control-lib.test.mjs`, `wp-communications-lib.test.mjs`

### Family F - Behavior Tests To Preserve

**Goal:** protect real runtime and behavior coverage while deleting meta noise.

Keep families:
- relay routing
- session launch governance
- viewport runtime rendering
- runtime path/cwd behavior
- shared library behavior tests with non-trivial fixtures

Rule:
- if a test spins the script, parses machine output, or validates a non-trivial state machine, keep it unless replaced by stronger coverage
- if a test only checks names, docs, quoting, or static surface strings, merge or delete it after replacement

## FIRST EXECUTION WAVE

1. Freeze Family A/B from retire/merge work in the current wave.
2. Build a narrower cold-script inventory outside Family A/B.
3. Consolidate the five low-value meta tests listed in Family E only when they are proven not to protect active command-surface expectations.
4. Split/test the P0 monoliths so later rationalization decisions are based on stronger behavior coverage.
5. For any truly retired source file in a later wave, move the original into `../../scripts_archive/` and log it.

## SCRIPT RATIONALIZATION LOG

- Future deletions, merges, retirements, and restorations should be recorded in:
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
- Every change should log:
  - date
  - file path
  - action (`DELETE`, `MERGE`, `ARCHIVE`, `RESTORE`, `REHOME`, `KEEP`)
  - replacement path if any
  - reason
  - commit sha
  - status

## SUMMARY JUDGMENT

- The low-confidence part of the original audit was Family A/B. Cross-check plus operator review indicates these surfaces should be treated as live or recovery-adjacent for now.
- For this cleanup track, "delete" means "remove from the repo and preserve in `../../scripts_archive/`".
- The strongest current targets are structural/test improvements and genuinely cold scripts outside Family A/B.
- The largest scripts should not be deleted first; they should be split until adjacent surfaces become safely removable.
