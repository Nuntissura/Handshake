# Strict Cold Governance Script Candidates

## METADATA
- AUDIT_ID: AUDIT-20260408-STRICT-COLD-GOVERNANCE-SCRIPT-CANDIDATES
- DATE_UTC: 2026-04-08T16:20:00Z
- AUDITOR: Codex ORCHESTRATOR
- SCOPE: Repo-governance script surface under `.GOV`
- PURPOSE: Find genuinely cold governance-script candidates only, excluding currently used command, gate, operator, and recovery/admin-adjacent surfaces

## INPUTS
- `.GOV/Audits/audits/AUDIT_20260408_GOVERNANCE_SCRIPT_RATIONALIZATION_MATRIX.md`
- `.GOV/Audits/audits/AUDIT_20260408_GOVERNANCE_SCRIPT_RATIONALIZATION_MATRIX_FULL.md`
- `.GOV/Audits/audits/AUDIT_20260408_GOVERNANCE_SCRIPT_FAMILY_RATIONALIZATION_PLAN.md`
- `justfile`
- live `.GOV/**/*.mjs|.js|.ps1|.sh` executable surface

## EXCLUDED UP FRONT

The Family A/B set was excluded from this pass because operator review established that these files are live or recovery-adjacent:

- `.GOV/roles/orchestrator/scripts/gov-check-feedback.mjs`
- `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
- `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs`
- `.GOV/roles_shared/checks/build-order-check.mjs`
- `.GOV/roles/validator/checks/validator-hygiene-full.mjs`
- `.GOV/operator/scripts/operator-viewport-tui.mjs`

## STRICT FILTER

A file qualified as a strict cold candidate only if all of the following were true:

1. no `justfile` reference
2. no active protocol / README reference
3. no current-doc reference
4. no inbound reference from another executable governance file
5. no active test reference

Current-doc filtering intentionally excluded historical noise:

- `.GOV/Audits/**`
- `.GOV/spec/history/**`
- `.GOV/task_packets/**`
- `.GOV/refinements/**`
- `.GOV/operator/docs_local/**`
- `.GOV/operator/messages_history/**`
- historical `*/history/*` paths
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- `.GOV/reference/PAST_WORK_INDEX.md`

## RESULT

- Total governance script files scanned: `273`
- Candidate pool after excluding Family A/B: `266`
- Strict cold matches: `56`
- Strict cold non-test executable matches: `0`

## JUDGMENT

There are no high-confidence cold executable governance scripts right now outside Family A/B.

Under the strict filter, every remaining match was a test file. That is not enough to treat those files as cold deletion candidates, because governance tests are often unreferenced by design: they are invoked by the test runner rather than imported by runtime code or cited in docs.

So the practical result is:

- No entry script is currently a strict cold candidate.
- No check script is currently a strict cold candidate.
- No helper library is currently a strict cold candidate.
- Cold-only pruning should not target executable governance scripts in the current wave.

## OPERATOR MEANING

The repo does have governance bloat, but it is not currently showing up as obviously dead runtime scripts.

The safer next slimming targets are:

1. low-value meta tests, reviewed semantically rather than by reference graph alone
2. large live monoliths that should be split before any wrapper removal is reconsidered
3. command-surface duplication only after the active surface is explicitly re-baselined

## RECOMMENDATION

- Freeze runtime-script deletion / archive work for now.
- Treat the current runtime script surface as active unless a file is proven cold by stronger evidence than reference absence alone.
- If cleanup should continue immediately, pivot to a test-only rationalization pass focused on meta/contract duplication rather than runtime entrypoints.

## COMMAND LOG

- `rg -n "gov-check-feedback|create-task-packet-stub|session-reset-batch-launch-mode|session-control-cancel|build-order-check|validator-hygiene-full|operator-viewport-tui" .GOV justfile ..\\handshake_main\\AGENTS.md`
- targeted `Get-Content` reads for the Family A/B files
- repo-local Node cross-check over `.GOV` executable files using the strict filter above
- `just gov-check`

## FINAL STATUS

- Family A/B remain deferred.
- Strict cold executable candidates outside Family A/B: `NONE`.
