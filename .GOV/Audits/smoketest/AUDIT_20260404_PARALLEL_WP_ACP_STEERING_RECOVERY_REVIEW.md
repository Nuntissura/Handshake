# AUDIT_20260404_PARALLEL_WP_ACP_STEERING_RECOVERY_REVIEW

## Scope

- Audit ID: `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
- Smoketest review ID: `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- Audit date: 2026-04-04
- Surface: ACP-managed crash recovery for parallel orchestrator-managed WPs
- Worktree: `wt-gov-kernel`
- Branch: `gov_kernel`
- WPs:
  - `WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1`
  - `WP-1-Storage-Capability-Boundary-Refactor-v1`

## Executive Summary

The recovery health check passed for the assigned governance kernel worktree and both active WP worktrees were present at the expected heads. Governance/runtime repair work was required before truthful resumption: the packet/refinement mirror alignment had drifted, `refinement-check.mjs` still mishandled folder packets, `ensure-wp-communications.mjs` needed runtime packet-path canonicalization, and `wp-receipt-append.mjs` could self-deadlock on `.tx.lock` during handoff append flows.

Recovery is now complete. Both WPs are contained in `main` at `ad680e3a4071e05e207ea9d562ee397f0eaded30`, and final-lane closeout sync passes for:

- `WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1`
- `WP-1-Storage-Capability-Boundary-Refactor-v1`

The decisive product-side blocker during recovery was boundary false-negative test portability, not unresolved feature logic. Current `main` failed `storage::tests::database_trait_purity_source_regressions` because `src/backend/handshake_core/src/storage/tests.rs` scanned `mod.rs` assuming LF-only newlines. The recovery repair normalized `include_str!(\"mod.rs\")` with `.replace(\"\\r\\n\", \"\\n\")`, after which the exact boundary proof probes passed on both the WP branch and `main`.

Live ACP prompts for CODER and validator lanes eventually hit usage limits, so the final continuation used the latest settled governed findings plus deterministic local proof. Recorded lane truth still matters: earlier boundary integration-validator failures on `f69f9c5` and `51fe2f0` correctly identified real compatibility gaps, while later validator PASS evidence on `8fa68c7` narrowed the residual issue to contained-main proof and governance sync. The remaining recovery work therefore stayed inside this session as packet truth repair, patch regeneration, closeout resync, and audit correction rather than reopening product design.

The Operator's waiver condition was satisfied for already-performed adjacent-scope state. That waiver was used to continue recovery, but closeout was only considered complete after the signed scope, packet closure matrix, runtime packet path, and contained-main baselines were all brought back to truthful terminal state.

## Final Resolution Update (2026-04-04T23:45Z)

This section supersedes the stale intermediate state elsewhere in this audit.

- Final `main` head containing both WPs: `ad680e3a4071e05e207ea9d562ee397f0eaded30`
- Parity WP:
  - previously contained in `main` at `cc00f1187e233b0bf228522e9f716d3f9ef2478e`
  - resynced onto the newer `main` baseline after the boundary repair advanced `main`
  - `just integration-validator-closeout-sync WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 CONTAINED_IN_MAIN ad680e3a4071e05e207ea9d562ee397f0eaded30` passed
- Boundary WP:
  - recovery repair commit on WP branch: `4cadfb5bed6c88ac88f6feafabe6afecc820c9a2`
  - contained-main repair commit: `ad680e3a4071e05e207ea9d562ee397f0eaded30`
- `just integration-validator-closeout-sync WP-1-Storage-Capability-Boundary-Refactor-v1 CONTAINED_IN_MAIN ad680e3a4071e05e207ea9d562ee397f0eaded30` passed

## Structured Failure Ledger Addendum (2026-04-05T01:30Z)

### SMOKE-FIND-20260404-01

- CATEGORY: ACP_RUNTIME
- SURFACE: `wp-receipt-append.mjs`, notification routing, relay wake path
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-64`
- REGRESSION_HOOKS:
  - `node --test .GOV/roles_shared/tests/wp-auto-relay-paths.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
- Evidence:
  - duplicate review-triggered wake paths were present during ACP-managed parallel WP steering
- What went wrong:
  - review receipts and derived notifications could both steer the next lane, burning prompts and operator attention
- Impact:
  - token and time waste increased on every review transition
- Mechanical fix direction:
  - keep a single authoritative auto-relay source and deliver typed route payloads

### SMOKE-FIND-20260404-02

- CATEGORY: WORKFLOW_DISCIPLINE
- SURFACE: manual relay lane, operator message brokerage
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-67`
- REGRESSION_HOOKS:
  - `node --test .GOV/roles/orchestrator/tests/manual-relay-next.test.mjs`
- Evidence:
  - old manual relay output interwove checks, prompts, and explanations into unreadable walls of text
- What went wrong:
  - handoff vs explainer vs question semantics were not typed
- Impact:
  - operator brokerage remained cheaper than ACP but too ambiguous to be reliable at scale
- Mechanical fix direction:
  - enforce relay envelopes with `ROLE_TO_ROLE_MESSAGE` and `OPERATOR_EXPLAINER` separation

### SMOKE-FIND-20260404-03

- CATEGORY: TIMELINE
- SURFACE: runtime, receipts, control ledger, token ledger
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-65`
- REGRESSION_HOOKS:
  - `just wp-timeline WP-1-Storage-Capability-Boundary-Refactor-v1`
- Evidence:
  - the recovery required manually reconstructing launch, handoff, and closeout timing from multiple files
- What went wrong:
  - no single normalized WP timeline existed
- Impact:
  - cost attribution and stall analysis were slow and lossy
- Mechanical fix direction:
  - expose one merged timeline surface per WP

### SMOKE-FIND-20260404-04

- CATEGORY: PRODUCT_SCOPE
- SURFACE: coder intent and microtask scope budgeting
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-66`
- REGRESSION_HOOKS:
  - `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
- Evidence:
  - coders could still drift into adjacent work because declared microtask scope was not enforced at receipt time
- What went wrong:
  - microtasks existed as narrative artifacts, not as enforced write-budget truth
- Impact:
  - remediation scope widened and review cost increased
- Mechanical fix direction:
  - fail closed when `microtask_contract` scope or file targets do not match declared `MT-*` surfaces

### SMOKE-FIND-20260404-05

- CATEGORY: GOVERNANCE_CHECK
- SURFACE: packet/runtime/task-board/build-order truth
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-68`
  - `RGF-70`
- REGRESSION_HOOKS:
  - `node --test .GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
- Evidence:
  - closeout required late repair of packet status, clause-monitor truth, runtime packet paths, and containment status
- What went wrong:
  - milestone and task-board truth were duplicated across helpers instead of projected from one authority surface
- Impact:
  - closeout spiraled into long repair loops
- Mechanical fix direction:
  - derive runtime milestone and board status from shared packet-authority projection helpers

### SMOKE-FIND-20260404-06

- CATEGORY: ROLE_INTEGRATION_VALIDATOR
- SURFACE: governed validator report law and current-main interaction audit
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-69`
- REGRESSION_HOOKS:
  - `node --test .GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `node --test .GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
- Evidence:
  - earlier workflow law proved many mechanics but did not require explicit primitive-retention and current-main interaction proof
- What went wrong:
  - validators could PASS without a strong enough retained-feature and shared-surface composition audit
- Impact:
  - feature-loss and overwrite risk stayed under-proved for medium/high-risk WPs
- Mechanical fix direction:
  - introduce `SPLIT_DIFF_SCOPED_RIGOR_V4` with primitive-retention and interaction proof sections

### SMOKE-FIND-20260404-07

- CATEGORY: TOOLING
- SURFACE: smoketest review template and audit skeleton
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-73`
- REGRESSION_HOOKS:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
- Evidence:
  - the original review format captured narrative well but did not provide stable finding IDs, board linkage, or positive controls
- What went wrong:
  - postmortem signal was trapped in prose
- Impact:
  - linking smoke findings into governance planning stayed manual and lossy
- Mechanical fix direction:
  - standardize a structured failure ledger and positive-control ledger

## Governance Linkage Addendum (2026-04-05T01:30Z)

- BOARD_LINKS:
  - `SMOKE-FIND-20260404-01 -> RGF-64`
  - `SMOKE-FIND-20260404-02 -> RGF-67`
  - `SMOKE-FIND-20260404-03 -> RGF-65`
  - `SMOKE-FIND-20260404-04 -> RGF-66`
  - `SMOKE-FIND-20260404-05 -> RGF-68, RGF-70`
  - `SMOKE-FIND-20260404-06 -> RGF-69`
  - `SMOKE-FIND-20260404-07 -> RGF-73`
- CHANGESET_LINKS:
  - `SMOKE-FIND-20260404-01 -> GOV-CHANGE-20260405-02`
  - `SMOKE-FIND-20260404-02 -> GOV-CHANGE-20260405-02`
  - `SMOKE-FIND-20260404-03 -> GOV-CHANGE-20260405-02`
  - `SMOKE-FIND-20260404-04 -> GOV-CHANGE-20260405-02`
  - `SMOKE-FIND-20260404-05 -> GOV-CHANGE-20260405-03`
  - `SMOKE-FIND-20260404-06 -> GOV-CHANGE-20260405-03`
  - `SMOKE-FIND-20260404-07 -> GOV-CHANGE-20260405-03`

## Positive Controls Addendum (2026-04-05T01:30Z)

### SMOKE-CONTROL-20260404-01

- SURFACE: contained-main recovery proof
- Why it mattered:
  - the final recovery converged on one real product blocker instead of continuing broad speculative churn
- Evidence:
  - the CRLF normalization fix in `src/backend/handshake_core/src/storage/tests.rs` cleanly removed the false negative and unblocked honest containment proof

### SMOKE-CONTROL-20260404-02

- SURFACE: closeout truth enforcement
- Why it mattered:
  - even after crash recovery, the governed closure checks forced packet/runtime/main truth back into an honest terminal state
- Evidence:
  - both WPs closed only after `integration-validator-closeout-sync` and `gov-check` passed with contained-main truth recorded

## Recorded Failures And Findings Used For Recovery

- Boundary historical validator failures:
  - `f69f9c5` failed because `supports_locus_runtime()` regressed and `workflows.rs` still lagged typed task-board parity
  - `51fe2f0` fixed the Postgres locus-runtime regression but still failed current-main compatibility
- Boundary later settled PASS evidence:
  - WP validator PASS and integration-validator PASS on `8fa68c7`
  - later live retry prompts failed due usage limits rather than new contradictory technical findings
- Boundary final local root cause:
  - `storage::tests::database_trait_purity_source_regressions` failed on current `main` only because the source-scan assertion was newline-sensitive
  - one-line CRLF normalization in `src/backend/handshake_core/src/storage/tests.rs` resolved the false negative without widening product behavior
- Parity recovery finding:
  - after `main` advanced for the boundary repair, parity closeout-check failed only because `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA` was stale
  - parity product code did not need reopening; contained-main truth just needed resync to `ad680e3`

## Additional Governance Defects Exposed During Recovery

- regenerated `signed-scope.patch` once after PowerShell flattened the diff into a single line; repaired by rewriting the artifact with raw redirected `git diff`
- repaired runtime `task_packet` projection for boundary from `../wt-gov-kernel/.GOV/...` to canonical `.GOV/task_packets/...`
- repaired the boundary closed-packet clause monitor so no in-scope `CLAUSE_CLOSURE_MATRIX` row remained at `VALIDATOR_STATUS: PENDING`

## Superseding Continuation Update (2026-04-04T20:38Z)

This section supersedes earlier stale statements in this audit about the active product heads, the parity lane being terminal on `c2af3df`, and the boundary integration-validator lane having already been cleanly reconciled.

- `just orchestrator-startup` now passes again after repairing the boundary integration-validator session registry drift by replaying the already-settled result `45f4cd44-c5bc-49a4-a031-587cbe21320f` onto the stale session entry. The earlier false `RUNNING` state is gone.
- Current product closeout candidates are:
  - parity: `8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af` (`8e90cc8`)
  - boundary: `8fa68c7971694eb646ec0579636acd10c6d88531` (`8fa68c7`)
- Fresh governed coder findings for parity are now settled on command `2c617d98-a1b3-4c1a-b2c3-11c21b28b1d4`:
  - no remaining coder-side product blocker
  - no remaining coder-side governance blocker for final-lane progression on `8e90cc8`
  - fresh `REVIEW_REQUEST` appended on correlation `final-review:wp-1-postgres-structured-collaboration-artifact-parity-v1:8e90cc8`
- Fresh governed parity runtime truth now waits on `INTEGRATION_VALIDATOR`:
  - `next_expected_actor: INTEGRATION_VALIDATOR`
  - `waiting_on: OPEN_REVIEW_ITEM_REVIEW_REQUEST`
  - open review item correlation `final-review:wp-1-postgres-structured-collaboration-artifact-parity-v1:8e90cc8`
- Boundary remains the live merge blocker, but the latest settled packet-scope validator finding is still `PASS` on `8fa68c7`; a fresh final-lane integration-validator pass is actively running under governed command `86554272-b4f3-4e59-ae99-7af63cbb868b`.
- Because that boundary validator command is still running, the boundary token ledger has temporarily re-entered `DRIFT/WARN` on one missing tracked command id. That is a live-ledger artifact of the in-flight review, not a new product-side contradiction.
- Neither WP is contained in `main` yet. Final integration-validator authority, merge, and post-merge governance truth sync are still outstanding for both lanes.

## Health Check

### WT-001

`just hard-gate-wt-001` passed during recovery and confirmed:

- repo root: `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel`
- current branch: `gov_kernel`
- expected active product worktrees present:
  - `../wtc-artifact-parity-v1` at `8e90cc8`
  - `../wtc-boundary-refactor-v1` at `8fa68c7`
  - validator worktrees for both WPs

### Initial Governance State

The kernel worktree was already dirty with live governance edits and one untracked file:

- modified:
  - `.GOV/refinements/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1.md`
  - `.GOV/refinements/WP-1-Storage-Capability-Boundary-Refactor-v1.md`
  - `.GOV/roles_shared/checks/refinement-check.mjs`
  - `.GOV/roles_shared/records/BUILD_ORDER.md`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/packet.md`
  - `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/refinement.md`
  - `.GOV/task_packets/WP-1-Storage-Capability-Boundary-Refactor-v1/packet.md`
  - `.GOV/task_packets/WP-1-Storage-Capability-Boundary-Refactor-v1/refinement.md`
- untracked:
  - `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`

## Recovery Repairs Already Landed

### Governance and Tooling Repairs

- `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - repaired runtime `task_packet` canonicalization so packet references resolve as `.GOV/task_packets/...`
- `.GOV/roles_shared/checks/refinement-check.mjs`
  - repaired folder-packet handling so packet-folder refinements resolve against `packet.md`
- `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - removed the self-deadlocking `.tx.lock` preflight pattern on handoff append
  - switched handoff preflight to call `post-work.mjs` directly instead of `just post-work`

### Packet and Refinement Truth Repairs

- repaired refinement mirror drift for both WPs:
  - `.GOV/refinements/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1.md`
  - `.GOV/refinements/WP-1-Storage-Capability-Boundary-Refactor-v1.md`
  - `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/refinement.md`
  - `.GOV/task_packets/WP-1-Storage-Capability-Boundary-Refactor-v1/refinement.md`
- repaired boundary packet manifest truth:
  - `.GOV/task_packets/WP-1-Storage-Capability-Boundary-Refactor-v1/packet.md`
  - authoritative reviewable range now records `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..51fe2f00ef3c716448ced421905dc132af9358cd`

## Boundary WP Recovery State

### Packet and Runtime Truth

- packet: `.GOV/task_packets/WP-1-Storage-Capability-Boundary-Refactor-v1/packet.md`
- product branch/worktree:
  - branch `feat/WP-1-Storage-Capability-Boundary-Refactor-v1`
  - worktree `../wtc-boundary-refactor-v1`
  - current closeout candidate `8fa68c7`
- runtime status file:
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Capability-Boundary-Refactor-v1/RUNTIME_STATUS.json`
- current runtime projection at recovery:
  - `runtime_status: working`
  - `current_phase: VALIDATION`
  - `next_expected_actor: ORCHESTRATOR`
  - `waiting_on: VERDICT_PROGRESSION`

The runtime route is now sensible again, but the packet header had drifted relative to the latest review history before this audit refresh:

- packet still said `MAIN_CONTAINMENT_STATUS: NOT_STARTED`
- packet still said `CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN`
- packet still said `PACKET_WIDENING_DECISION: NONE`

Those fields no longer matched the recorded integration review evidence and were refreshed in the kernel during this recovery pass.

### Authoritative Review History

- `f69f9c5`
  - first repaired eight-file current-main-compatibility candidate
  - failed final merge-readiness at `2026-04-04T16:35:27.777Z`
  - blockers:
    - `supports_locus_runtime()` regressed to `false` in `storage/postgres.rs`
    - `workflows.rs` still emitted the older task-board projection contract without current-main typed parity
- `51fe2f0`
  - follow-up repair `Repair current-main storage boundary parity`
  - fixed the prior Postgres locus-runtime blocker
  - latest authoritative integration review at `2026-04-04T18:37:42.217Z` still failed current-main compatibility on `workflows.rs`

### Latest Authoritative Governed Finding

The earlier authoritative integration review response on correlation `review:WP-1-Storage-Capability-Boundary-Refactor-v1:review_request:mnko2qu6:2f0229` established the original blocker set:

- the open review request was malformed
  - receipt summary was only `Final`
  - revision suffix `2f0229` does not resolve in the WP worktree or canonical `main`
- `src/backend/handshake_core/src/storage/postgres.rs` is now correct on the previous locus-runtime issue
- `src/backend/handshake_core/src/workflows.rs` is still not current-main compatible because it:
  - builds older typed task-board and structured-artifact records
  - omits current-main typed `profile_extension` handling in constructors/validators
  - backfills `profile_extension` after serialization through `set_profile_extension_field`
- canonical `main` carries typed `profile_extension` directly in the constructors and validators
- the full signed range `f85d767..51fe2f0` still does not apply cleanly to current `main` across all eight packet files

Follow-up targeted governed findings at `2026-04-04T19:03:07.823Z` (CODER) and `2026-04-04T19:03:48.696Z` (INTEGRATION_VALIDATOR) then narrowed the live truth further:

- current-main `src/backend/handshake_core/src/locus/task_board.rs` and `src/backend/handshake_core/src/locus/types.rs` already carry the correct typed `profile_extension` contract for the packet intent
- with those current-main files treated as accepted background state, the remaining repairable blocker is confined to packet-owned `src/backend/handshake_core/src/workflows.rs`
- exact remaining remediation is to stop building older typed records and then patching JSON via `set_profile_extension_field`, and instead populate typed `profile_extension` directly for task-board and structured-artifact records
- honest lane state is still not PASS; it is a waiver-dependent, repairable fail until a fresh repaired branch state and governed validator pass exist

This keeps boundary as the only remaining live product blocker across the two recovered WPs.

### Lane and Session Recovery

- a stale broker-owned `INTEGRATION_VALIDATOR` run remained projected as active after result `45f4cd44-c5bc-49a4-a031-587cbe21320f` had already self-settled to `FAILED`
- the session runtime invariant was repaired by replaying that already-settled result onto the stale registry entry through the existing session-registry mutation path
- after reconciliation:
  - `just orchestrator-startup` returned to green
  - the false `RUNNING` projection cleared
  - later fresh final-lane steering became possible again on command `86554272-b4f3-4e59-ae99-7af63cbb868b`

### Crash-Recovery Steering

An orchestrator recovery note was appended at `2026-04-04T18:42:35.181Z` telling the boundary coder:

- autonomous crash-recovery continuation is active
- the authoritative blocker is `workflows.rs`
- the repair must stay inside the signed eight-file packet surface
- the next return must be a fresh committed `CODER_HANDOFF` plus a valid review request

The follow-up governed prompt landed and settled. Session registry now shows all three boundary lanes back in `READY`, with the latest outputs preserving the narrowed blocker above instead of an in-flight recovery run.

## Parity WP Recovery State

### Packet and Runtime Truth

- packet: `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/packet.md`
- product branch/worktree:
  - branch `feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1`
  - worktree `../wtc-artifact-parity-v1`
  - current closeout candidate `8e90cc8`
- packet and runtime both truthfully project:
  - `current_packet_status: Done`
  - `main_containment_status: MERGE_PENDING`
  - `current_main_compatibility_status: ADJACENT_SCOPE_REQUIRED`
  - `packet_widening_decision: FOLLOW_ON_WP_REQUIRED`

### Authoritative Outcome

The parity product repair itself is done and final technical review already passed:

- final authoritative integration PASS at `2026-04-04T08:25:20.648Z`
- commit `c2af3df` cleared the remaining task-board ordering regression without widening product scope

However the packet is not honestly closable as contained-main because:

- authoritative `main` baseline is now `cc00f1187e233b0bf228522e9f716d3f9ef2478e`
- packet truth records undeclared `src/backend/handshake_core/src/storage/tests.rs` drift outside this packet's honest scope
- packet therefore records:
  - `CURRENT_MAIN_COMPATIBILITY_STATUS: ADJACENT_SCOPE_REQUIRED`
  - `PACKET_WIDENING_DECISION: FOLLOW_ON_WP_REQUIRED`

This is a governance and signed-scope-accounting blocker, not an unresolved product-code blocker inside the signed parity repair.

### Latest Governed Findings

Targeted governed findings at `2026-04-04T19:06:35.565Z` (INTEGRATION_VALIDATOR) and `2026-04-04T19:03:53.622Z` (CODER session update for the same question) now establish:

- the authoritative-main `src/backend/handshake_core/src/storage/tests.rs` drift looks correct enough versus the Master Spec for the packet's portability and dual-backend proof contract
- the drift is test-facing and `#[cfg(test)]` rather than a new product-path semantic change
- the exact remaining parity action is to record the satisfied waiver and, because closeout enforces signed-scope truth directly, either formally attach `src/backend/handshake_core/src/storage/tests.rs` to the accepted signed surface or port that delta into the parity branch and refresh `signed-scope.patch` plus validation manifest truth before contained-main sync
- waiver alone is not sufficient for closeout because the current signed-scope checker does not waive undeclared files in the enforced surface comparison

### Additional Status Truth

Parity runtime still carries an old coder heartbeat:

- `last_heartbeat_at: 2026-04-04T02:31:15.547Z`

but runtime completion state is already correct:

- `runtime_status: completed`
- `current_phase: STATUS_SYNC`
- `waiting_on: MAIN_CONTAINMENT`
- `attention_required: false`

This stale heartbeat is noisy but not authority-breaking because the packet and validator receipts already describe the actual terminal posture.

## Token Budget and Policy State

### Boundary

`just session-registry-status WP-1-Storage-Capability-Boundary-Refactor-v1` projects:

- token ledger drift currently reappeared because the live recovery prompt request id is not yet settled into the tracked ledger
  - missing tracked command sample: `2a4ad679-22cb-4d58-9f79-05725052ea77`
- fail-budget status remains active:
  - `TOTAL turn_count 40 exceeded fail budget 32`
  - `TOTAL input_tokens 1347089231 exceeded fail budget 260000000`
  - `CODER turn_count 27 exceeded fail budget 14`
  - `CODER input_tokens 1263146432 exceeded fail budget 180000000`

### Parity

The parity token ledger was explicitly settled after recovery using:

- `just wp-token-usage-settle WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 POST_RECOVERY_TERMINAL ORCHESTRATOR`

That removed historical ledger drift, but fail-budget residue still remains:

- `turn_count: 28`
- `input_tokens: 528598780`
- `status: FAIL`

### Consequence

The current `orchestrator-next` law path is explicit:

- on fail-budget WPs under `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the helper stops with `BLOCKER_CLASS: POLICY_CONFLICT`
- boundary and parity both now truthfully meet that condition

This means the governance surface currently lacks a sanctioned "continue under operator-authorized autonomous override" path for over-budget orchestrator-managed lanes.

## Silent Failures, Command Surface Misuse, and Ambiguity Scan

### Confirmed Defects

- Missing audit artifact
  - the requested recovery review file did not exist in the live kernel at recovery start, even though earlier validator output had referenced it as if it existed
- Folder-packet refinement check bug
  - packet-folder refinements were not resolved correctly until `refinement-check.mjs` was repaired
- Runtime packet-path canonicalization bug
  - `ensure-wp-communications.mjs` had allowed invalid packet-path forms to poison communication-health checks
- Receipt append deadlock bug
  - `wp-receipt-append.mjs` could self-deadlock on `.tx.lock`
- Malformed boundary review request
  - boundary coder emitted `REVIEW_REQUEST: Final` with revision suffix `2f0229`, which did not identify a real reviewed head
- Stale active-run projection
  - boundary integration-validator broker state remained projected as running after the effective review had already completed
- Packet/runtime truth drift
  - boundary packet header had remained stale relative to the actual review history until the recovery refresh updated the compatibility fields
- Budget law ambiguity
  - governance tells the orchestrator to stop on `POLICY_CONFLICT`, while the Operator may still explicitly instruct autonomous continuation; no first-class governed override path was found during this recovery

### Token-Amplification Notes

- direct coder-to-validator communication is working and should remain the default
- the remaining budget blowup appears to come from:
  - repeated long-form review loops
  - recovery repair churn
  - packet/runtime drift forcing extra revalidation and resumption work
- the system still lacks a compact sanctioned continuation path once token fail budgets are exceeded

## Current Recommended Next Actions

### Boundary

- steer CODER to repair only `src/backend/handshake_core/src/workflows.rs` against the accepted current-main typed `profile_extension` contract
- require the next coder return to include a fresh committed `CODER_HANDOFF` and valid `REVIEW_REQUEST`
- route the repaired branch immediately through `WP_VALIDATOR` and then `INTEGRATION_VALIDATOR`
- if the coder proves `workflows.rs` still cannot be repaired honestly inside the signed eight-file packet, stop and convert that proof into the next widening or superseding-packet decision

### Parity

- keep product truth as complete and already contained on authoritative `main`
- do not claim contained-main closure until the parity signed-scope surface honestly accounts for the authoritative-main `src/backend/handshake_core/src/storage/tests.rs` delta
- prefer in-session proof-surface repair inside `v1` over creating a new follow-on packet, because the file is already packet-in-scope and the remaining blocker is declared-surface drift rather than new product scope

## Final Assessment

Recovery succeeded in restoring truthful ACP lane state and in repairing several governance/runtime defects that had become hard blockers during the crash-restart window. The parity WP is technically complete and already contained on authoritative `main`, but still blocked on honest signed-scope accounting before contained-main closeout can be synced. The boundary WP remains the only live product blocker, with one remaining packet-owned current-main compatibility defect in `src/backend/handshake_core/src/workflows.rs`.

The governance surface is still not healthy enough to call this parallel ACP flow robust. The missing audit file, malformed review request, stale active-run projection, packet/runtime drift, and missing sanctioned path for operator-authorized continuation after `POLICY_CONFLICT` are all real control-plane defects, not mere operator inconvenience.

## Postmortem Addendum (2026-04-05)

### Findings First

- Finding `SMK-PAR-001`: orchestrator-managed ACP still behaves like manual relay plus extra ledgers, not like true event-driven delegation.
  - Evidence: runtime routing depends on explicit `check-notifications` / `ack-notifications` / `wp-review-exchange` command use, and `orchestrator-steer-next.mjs` still has to inspect packet/runtime state and then actively `START_SESSION` or `SEND_PROMPT` for the next lane.
  - Impact: every handoff adds extra prompts, extra runtime reads, extra self-settlement risk, and extra idle time between productive turns.
- Finding `SMK-PAR-002`: microtasks exist as packet files and receipt metadata, but the governed lane does not schedule on them as the primary unit of work.
  - Evidence: `create-task-packet.mjs` generates `MT-*.md`, while runtime steering and health logic mainly treat `microtask_contract` as optional review metadata on receipts.
  - Impact: coders still reason over whole-WP context windows, validators still review at WP scope by default, and overlap review only trims a small part of the token load.
- Finding `SMK-PAR-003`: fail-budget policy currently dead-ends long-running remediation instead of switching cleanly into an operator-authorized continuation mode.
  - Evidence: boundary and parity both reached `POLICY_CONFLICT`, and the audit already records that no sanctioned override path existed even when the Operator explicitly wanted autonomous continuation.
  - Impact: the orchestrator keeps paying for recovery, settlement, and truth repair around the stop condition instead of transitioning to a cheaper governed override mode.
- Finding `SMK-PAR-004`: governance/runtime drift is now a first-class source of token burn.
  - Evidence: this recovery required repairs to packet/refinement mirrors, packet-path canonicalization, receipt append locking, stale session projection, clause monitor truth, signed-scope patch generation, and closeout sync state before the remaining product delta could close.
  - Impact: the workflow spends large effort proving that control-plane state is truthful again before any new product reasoning can happen.
- Finding `SMK-PAR-005`: non-authoritative runtime/build data still leaks into expensive places despite the external-artifacts policy.
  - Evidence:
    - `handshake_main` is clean in git and its object store is small (`size-pack: 38.96 MiB`), but the checkout is about `27.11 GB` because `src/backend/handshake_core/target/` exists inside the repo tree.
    - `Handshake Artifacts` is about `139 GB`, dominated by `handshake-cargo-target` (`102.11 GB`) plus stale WP-specific targets `validator_wp1_f69f9c5_target` (`25.66 GB`) and `intval-wp1-boundary-target` (`11.33 GB`).
  - Impact: canonical worktrees stay heavy, backups and scans get slower, and cleanup is not happening mechanically at WP closeout.
- Finding `SMK-PAR-006`: terminal/session host lifecycle is not governed tightly enough.
  - Evidence: session registry tracks `active_terminal_title` / kind, but not durable OS PID / window handle ownership. Current runtime can prove dispatch, not exact terminal-window ownership. The desktop still shows many `Code`, `cmd`, `powershell`, and `codex` processes with no safe per-WP closeout binding.
  - Impact: session windows accumulate and cannot be closed mechanically without risking unrelated terminals.
- Finding `SMK-PAR-007`: validator rigor is still split between strong mechanical checks and weaker spec-to-code reading than the Master Spec surface now demands.
  - Evidence: the recovery audit shows several cases where validator outputs were useful, but closeout truth still depended on later direct code/spec reading, adjacent-scope accounting, and contained-main compatibility reasoning.
  - Impact: validators can pass narrow packet proofs while cross-feature integration, primitive retention, or current-main fit still remains under-read.

### Root-Cause Readout

- Root cause `RC-01`: too many workflow transitions are prompt-mediated instead of event-applied.
  - Mechanical route truth exists, but the next lane still needs a new governed prompt to notice it and act.
- Root cause `RC-02`: microtask structure is not yet the main scheduler contract.
  - It helps bounded overlap review, but not planning, budgeting, or closeout segmentation.
- Root cause `RC-03`: governance law, checks, and docs are still evolving faster than the models can keep the whole surface stable in one pass.
  - The repo is still in active control-plane construction, so drift and repair churn are currently expected but too costly.
- Root cause `RC-04`: current packet scope and current-main compatibility surfaces interact badly during remediation.
  - Small product fixes turn into larger governance repairs when signed surface, adjacent scope, and contained-main proof diverge.
- Root cause `RC-05`: external artifact policy is not enforced end-to-end.
  - The policy exists, but nested repo-local `target/` trees and stale per-WP artifact folders still survive.
- Root cause `RC-06`: session launch and session cleanup are asymmetric.
  - Launch is governed; shutdown and window reclamation are only partially governed.

### What Went Well

- Direct coder-validator receipt routing is better than pure manual narration.
- Packet-scoped runtime truth and review queues make many hidden failures visible.
- The system did eventually converge and preserve an auditable trail instead of silently losing history.

### Remediation Themes To Track

- Theme `RT-01`: event-driven relay instead of prompt-driven relay
- Theme `RT-02`: microtask-first orchestration and budgeting
- Theme `RT-03`: operator-authorized fail-budget override lane
- Theme `RT-04`: control-plane drift reduction and audit consolidation
- Theme `RT-05`: artifact and terminal lifecycle enforcement
- Theme `RT-06`: stronger validator spec-reading and primitive-retention review
