# Orchestrator-Managed Workflow Playbook

Status: projection/reference
Scope: `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`
Authority: navigational projection only. The machine-readable contract is `.GOV/roles_shared/workflow_contracts/orchestrator_managed.workflow.json`; ACP/session-control consumes that contract through `workflow_contract` request envelopes and `WORKFLOW_CONTRACT_CAPSULE` prompts. If this playbook conflicts with the workflow contract, role protocols, Codex law, packet truth, receipts, runtime status, or command output, those sources win.

## Purpose

This file is a human-readable projection of the machine workflow contract. It exists for audits and maintenance, not routine role context injection. Roles should receive compact `WORKFLOW_CONTRACT_CAPSULE` state from ACP/session-control rather than rereading this whole document. The machine contract exists to make `ORCHESTRATOR_MANAGED` workflows more mechanical because current governance/workflow can still be brittle, reduce Orchestrator babysitting, and harden autonomous parallel WP runs.

## Authority Boundaries

- Orchestrator owns workflow steering, deterministic governance checks, and role launch/wake decisions.
- Activation Manager owns bounded pre-launch enrichment, packet hydration, readiness, worktree/backup preparation, and self-close.
- Coder owns product implementation in the packet-declared WP worktree.
- WP Validator owns per-MT advisory technical review and early intent checkpoint clearance.
- Integration Validator owns final whole-WP technical verdict and merge authority.
- Packet truth wins over runtime and session projections. `RECEIPTS.jsonl` and `RUNTIME_STATUS.json` are the primary communication runtime. `THREAD.md` is coordination prose only.
- All non-Coder roles share `CX-218L` governance paperwork/workflow stabilization duty within their authority and must actively strive to make brittle handoffs, receipts, projections, and documentation transitions mechanical. Coder is excluded and reports governance blockers instead of patching `.GOV/` or workflow tooling from the product-code lane.
- Governance refactor or stabilization work must be declared in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` with a stable item and current status, then updated as the work moves through IN_PROGRESS, DONE, HOLD, or superseded.

## Healthy Sequence

1. Orchestrator startup/resume
   - `just orchestrator-startup`
   - `just orchestrator-next [WP-{ID}] [--debug]`
   - `just operator-viewport`
   - Open Orchestrator repomem with `--role ORCHESTRATOR --wp WP-{ID}` before governed mutation.

2. Activation Manager pre-launch
   - Launch: `just launch-activation-manager-session WP-{ID}`
   - Activation Manager returns `REFINEMENT_HANDOFF_SUMMARY`.
   - Orchestrator reviews, gets operator approval/signature, and steers the bundle back.
   - Activation Manager writes packet, microtasks, worktree/backups, health evidence, and `ACTIVATION_READINESS`.
   - For large/folded bundles, `ACTIVATION_READINESS` must expose `MICROTASK_STATUS` and `MICROTASK_GRANULARITY`; launch is not healthy if the packet compresses broad subsystem work into a few MTs just to reduce paperwork.
   - Before any model wake on stale readiness, run `just activation-manager readiness WP-{ID} --write` and inspect `just activation-manager next WP-{ID}`.

### Large Bundle MT Discipline

- Bundling related stubs into one WP is allowed when it removes repeated setup/schema/runtime-source decisions, but execution still resumes and validates through official MT files.
- There is no upper MT-count bias. Prefer 20+ narrow MTs over a few broad MTs when that improves deterministic execution, crash/session recovery, validator targeting, or suitability for smaller local/cloud coding models.
- A healthy bundle has a fold map from source stubs to MTs, one reviewable proof target per MT, dependency order that avoids reimplementation, and readiness output naming the declared MT count.
- If a bundled packet has low MT count, think through 3-5 causes before launch: Activation Manager compressed to save paperwork, folded source stubs were not mapped, proof commands span unrelated boundaries, helper/schema work was not assigned to an early dependency MT, or readiness is stale and counting old files.

3. Downstream launch
   - `just phase-check STARTUP WP-{ID} CODER`
   - `just launch-wp-validator-session WP-{ID}`
   - `just launch-coder-session WP-{ID}`
   - WP Validator should be READY before Coder starts MT work.

4. Bootstrap direct-review route
   - WP Validator publishes `VALIDATOR_KICKOFF`.
   - Coder publishes `CODER_INTENT`.
   - Runtime waits on `WP_VALIDATOR_INTENT_CHECKPOINT`.
   - WP Validator clears or rejects this checkpoint with `just wp-validator-response` / `just wp-spec-gap`.
   - `just wp-review-response` is for actual open `REVIEW_REQUEST` or `CODER_HANDOFF` review items, not for clearing `CODER_INTENT`.

5. Per-MT loop
   - Coder implements exactly one MT, commits, and emits `wp-review-request` or `CODER_HANDOFF` as required by the packet route.
   - WP Validator checks notifications, reviews, and emits `wp-review-response` / MT verdict.
   - Runtime alternates between Coder and WP Validator. Orchestrator watches route truth and wakes stalled projected actors; it does not broker ordinary technical content.

6. Whole-WP closeout prep
   - Confirm all MTs have WP Validator PASS receipts.
   - If no whole-WP `CODER_HANDOFF` exists and no `committed_handoff_head_sha` is recorded, steer Coder to publish the final handoff first. Per-MT PASS receipts are not a committed target for Integration Validator closeout.
   - After final `CODER_HANDOFF`, run `just phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>` to write durable committed validation evidence for the exact final range.
   - `just closeout-repair WP-{ID}` may repair deterministic prep drift, but terminal `phase-check CLOSEOUT` waits until the Integration Validator has written its final review/verdict.
   - Do not launch/steer Integration Validator while committed handoff evidence is missing.

7. Final validation
   - `just launch-integration-validator-session WP-{ID}`
   - Integration Validator runs `just phase-check VERDICT WP-{ID} INTEGRATION_VALIDATOR <session>` for the open final handoff, then performs fresh-context whole-WP judgment and emits the review/verdict receipt that resolves the handoff correlation.
   - PASS proceeds through governed closeout/merge path. FAIL returns to same-WP remediation unless scope expansion or operator choice requires a new WP.

8. Main containment after Integration Validator PASS
   - Confirm `just orchestrator-next WP-{ID}` says terminal `MERGE_PENDING` / `NEXT: STOP`, not active Coder or WP Validator work.
   - Treat merge as Integration Validator-owned authority. Orchestrator may babysit the mechanical sequence, but does not create a product verdict.
   - Before `git merge`, preserve committed branch states: push the approved WP feature branch to its remote backup and push current `main` to `origin/main`.
   - Merge only from `../handshake_main` on local `main`, then verify the approved target head is an ancestor of the new local-main HEAD.
   - Run `just phase-check CLOSEOUT WP-{ID} --sync-mode CONTAINED_IN_MAIN --merged-main-sha <MERGED_MAIN_SHA> --context "<why containment is now valid>"`.
   - Run `just gov-check`, then push `origin/main` only after contained-main closeout and governance checks pass.

## Deterministic Contract Migration Red-Team

This playbook is projection/reference. For RGF-286 and later contract migrations, treat the JSON contract as executable authority and treat Markdown as generated projection, frozen legacy reference, or short migration bridge.

Red-team stance before launch or repair:

- Assume projections are stale until source hash/provenance proves they were generated from the authoritative contract.
- Assume sidecars drift when two manually maintained files claim the same authority; prefer one primary typed file per atomic lifecycle object instead.
- Assume prose hides shadow authority; migrate lifecycle, scope, status, assignment, refinement, MT identity, and receipt-routing fields into typed contract keys.
- Assume schema omissions create unsafe fallback behavior; unsupported fields must become explicit migration debt rather than silent Markdown parsing.
- Assume Activation Manager and Classic Orchestrator diverge on prelaunch duties unless refinement, hydration, signature, worktree, backup, and MT preparation are encoded in the packet/refinement contracts.
## Key Artifacts

- Packet contract authority: `.GOV/task_packets/WP-{ID}/packet.json`
- Refinement contract authority: `.GOV/task_packets/WP-{ID}/refinement.json`
- Microtask contract authority: `.GOV/task_packets/WP-{ID}/MT-*.json`
- Packet Markdown projection: `.GOV/task_packets/WP-{ID}/packet.md` generated from or reconciled to the packet contract; legacy Markdown authority must be explicitly classified as `LEGACY_AUTHORITY`.
- Legacy import/repair: `just wp-contract-import WP-{ID}` or `just wp-contract-import --all --dry-run`; this is the governed path for stamping generated projection hashes instead of hand-editing packet/refinement/MT sidecars.
- Communications: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`
- Runtime: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/RUNTIME_STATUS.json`
- Receipts: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/RECEIPTS.jsonl`
- Session registry: `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- ACP ledgers: `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`, `SESSION_CONTROL_RESULTS.jsonl`, and `SESSION_CONTROL_OUTPUTS/`
- Dossier: diagnostic history only; not product-outcome authority.

## Intervention Rule

Before every non-Coder role patch, steer, relay repair, validation blocker, activation repair, closeout settlement, or Memory Manager proposal, classify 3-5 plausible causes and pick the cheapest mechanical action that proves or removes them. Record the durable part in repomem and governance records when it changes future behavior.

Common cause set:

- Runtime route drift: `RUNTIME_STATUS.json` points at an actor or receipt kind that no longer matches the latest receipt.
- Notification drift: target actor has unacknowledged notifications or stale ack cursor.
- Session drift: registry says READY/RUNNING but the output file, nudge queue, or broker state disagrees.
- Documentation/protocol drift: startup prompts tell a role to use the wrong helper or omit the expected helper.
- Clock drift: old `heartbeat_due_at` / `stale_after` values make a fresh route look escalated.
- Scope drift: role memory, worktree, or actor-session lookup inherits another role's state.

## Stall Patterns

### Stale Activation Readiness

Symptom: readiness says blocked or non-ready, but packet/worktree truth may have changed.

First actions:

- `just activation-manager readiness WP-{ID} --write`
- `just activation-manager next WP-{ID}`
- Wake Activation Manager only if refreshed readiness still requires model-owned repair.

### Projected Lane Idle

Symptom: runtime waits on Coder/WP Validator, but no receipt progress appears.

First actions:

- `just wp-lane-health WP-{ID}`
- `just active-lane-brief <ROLE> WP-{ID}`
- `just check-notifications WP-{ID} <ROLE>`
- `just orchestrator-steer-next WP-{ID} "<context>"` only when route truth still points at that role.

### Nudge Queue Backlog

Symptom: `orchestrator-steer-next` reports `NUDGE_QUEUE`, nonzero depth, or session READY but not draining.

First actions:

- Inspect `just session-registry-status WP-{ID}` and `just nudge-depth <session_id>`.
- If the projected target session is `READY`, has no pending control request, and relay status is `ESCALATED`, `just orchestrator-steer-next WP-{ID} "<context>"` should drain queued nudges into one direct safe-boundary `SEND_PROMPT`.
- Avoid duplicate steering while queue depth is nonzero unless the previous nudge is stale and the target run is idle.
- Patch prompt/route bugs if the same role repeatedly completes without consuming the queued route.

### Post-Commit Auto-Relay Does Not Fire

Symptom: Coder commits an MT, but no `REVIEW_REQUEST` notification appears for `WP_VALIDATOR`.

First actions:

- Think through 3-5 causes: effective Git hook path mismatch in linked worktrees, commit subject missing `feat: MT-NNN`, compile gate failed before review emission, mechanical MT review failed, or `wp-review-exchange` path/runtime lock failed.
- Inspect `git -C <coder_worktree> rev-parse --git-path hooks/post-commit`; the hook must be installed at that effective path, not guessed from the `.git` file.
- Inspect the latest commit subject and require `feat: MT-NNN <description>` for hook-driven auto-relay.
- Inspect `COMPILE_GATE_LOG.jsonl`: real compile failures block auto-relay, but host-load timeouts should relay with `HOOK_COMPILE_GATE=TIMEOUT_INCONCLUSIVE` so the Validator sees the proof gap instead of the route going silent.
- If the commit is already valid and the hook missed it, send exactly one manual `just wp-review-request ...` with the route sessions from `active-lane-brief`, then reinstall/fix the hook before the next MT.
- Keep manual review-request summaries shell-safe and short. If a stray summary word lands in `correlation_id` or `spec_anchor`, the helper must fail closed instead of writing a corrupted receipt.

### Active Run With No Output

Symptom: registry says `COMMAND_RUNNING`, but output file and session events are stale.

First actions:

- `just wp-lane-health WP-{ID}`
- `just wp-relay-watchdog WP-{ID} --no-watch-steer`
- Do not cancel by default. Use bounded restart only when watchdog policy allows it and no active output/run timeout guard is fresh.

### Formatter Or Cleanup Spillover

Symptom: a scoped role runs a broad formatter or cleanup and files outside the packet-cleared targets become dirty.

First actions:

- Think through 3-5 causes: broad formatter default, wrong working directory, stale packet file targets, pre-existing dirty worktree, or attempted cleanup after a timed-out test.
- Inspect `git diff --name-only` and compare it to packet-cleared file targets.
- Treat `git restore` / `git checkout --` as destructive/state-hiding worktree rewrites. If cleanup is needed, stop and route a typed blocker/repair note instead of silently discarding spillover.
- Future-proof the workflow by preferring file-targeted formatters (`rustfmt <files>`, prettier/eslint on explicit files) during scoped MTs.

### Wrong Review Helper

Symptom: WP Validator tries `wp-review-response` while runtime waits on `WP_VALIDATOR_INTENT_CHECKPOINT`.

First actions:

- Inspect `route_anchor_kind`. If it is `VALIDATOR_RESPONSE`, use `just wp-validator-response`.
- Reserve `wp-review-response` for open `REVIEW_REQUEST` or `CODER_HANDOFF` review items.
- If the wrong-helper path is unclear, patch the fail message or role prompt so the next session does not spend model turns rediscovering it.

### Final Handoff Missing Before Closeout

Symptom: all declared MTs have WP Validator PASS, `wp-communication-health-check` reports direct review complete, but `phase-check CLOSEOUT --sync-mode MERGE_PENDING` fails with missing governed Integration Validator identity or `candidate target validation requires committed target_head_sha`.

Likely causes:

- Per-MT review completion was mistaken for whole-WP handoff.
- `CODER_HANDOFF` was never emitted after the overlap review queue drained.
- Runtime route handed back to Orchestrator with `VERDICT_PROGRESSION` but no committed handoff base/head.
- `orchestrator-next` tried closeout sync before `phase-check HANDOFF WP_VALIDATOR --range` wrote durable validator-gate evidence.
- Integration Validator was asked to run `phase-check CLOSEOUT` before it answered the final handoff.

First actions:

- Check `just wp-communication-health-check WP-{ID} STATUS --verbose` for `coder_handoffs=0` and `open_review_items=0`.
- Check `RUNTIME_STATUS.json` for null `committed_handoff_head_sha`.
- Send `just session-send CODER WP-{ID} "<final handoff request>"` asking Coder to record `CODER_HANDOFF` with base/head/range, rubric self-audit, proofs, and carry-over risks.
- After handoff, run `just phase-check HANDOFF WP-{ID} WP_VALIDATOR --range <base>..<head>` before steering Integration Validator.
- Do not run closeout sync again until the Integration Validator has resolved the final handoff with a typed review/verdict receipt.

### Final Handoff Closeout Inversion

Symptom: Integration Validator receives a final `CODER_HANDOFF`, acknowledges the notification, runs `phase-check CLOSEOUT`, then records `WORKFLOW_INVALIDITY` because the handoff is still open or committed validation evidence is missing.

Likely causes:

- Closeout was treated as the action that answers the handoff instead of terminal proof after the review response.
- `phase-check VERDICT` was not passed the Integration Validator role/session, so the expected inbox item looked like generic open review debt.
- `phase-check HANDOFF WP_VALIDATOR --range` was skipped, leaving `validator_gates/<WP>.json` without `committed_validation_evidence`.
- The ACP relay prompt allowed acknowledgement/status text to stand in for a correlation-preserving review receipt.
- Protocol docs still said `phase-check CLOSEOUT` before launch after the workflow had shifted to final handoff review.

First actions:

- Patch the prompt/protocol so final handoff routes use `phase-check VERDICT ... INTEGRATION_VALIDATOR <session>` first.
- Run the missing committed handoff validation command from the Orchestrator lane.
- Repair any workflow-invalidity receipt caused by the tooling inversion with a typed `REPAIR`, then steer Integration Validator to emit the review response or product blocker against the original handoff correlation.

### Final Review Response Route Regression

Symptom: Integration Validator records PASS against the final `CODER_HANDOFF`, but runtime/communication health routes back to Coder or an old MT instead of showing final direct review resolved.

Likely causes:

- Final `wp-review-response` fell through to microtask fallback contract derivation.
- Receipt notification logic treated final Integration Validator review response like an ordinary Coder-facing review reply.
- Runtime still had a stale `route_anchor_kind=CODER_HANDOFF` or older MT `REVIEW_REQUEST`.
- Communication health required a direct-authority flag that old packet formats do not carry.
- Orchestrator checkpoint routing was confused with Coder ack routing.

First actions:

- Check `just wp-communication-health-check WP-{ID} STATUS --verbose` for `integration_final_open` and `integration_final_resolution`.
- If both are present, final review is resolved; patch route projection rather than waking Coder.
- Final Integration Validator review responses should notify Orchestrator for checkpoint/routing truth, not Coder for ack debt.
- Record a typed Orchestrator `REPAIR` when stale runtime route truth was corrected after the final PASS receipt.

### Closeout Report Materialization Drift

Symptom: `phase-check CLOSEOUT --sync-mode MERGE_PENDING` has final PASS and communication `COMM_OK`, but `validator-packet-complete` rejects `VALIDATION_REPORTS` or computed policy inputs.

Likely causes:

- Closeout sync tried to materialize a report from a final review receipt but emitted bullet-shaped scalar fields.
- `CLAUSES_REVIEWED` paraphrased closure rows instead of reusing exact `CLAUSE_CLOSURE_MATRIX` labels.
- A list parser swallowed the report instructions after an inline scalar field such as `MECHANICAL_REPORT_SOURCE: ...`.
- Negative proof, counterfactuals, or current-main checks lacked concrete product code references.
- Closeout sync wrote packet/runtime truth before self-validation and did not rollback on deterministic failure.

First actions:

- Run direct packet-complete evaluation or `phase-check CLOSEOUT --verbose` and fix the first deterministic report-shape failure.
- Keep scalar report fields top-level (`VALIDATION_CONTEXT: OK`), list labels top-level (`CLAUSES_REVIEWED:`), and list items indented below them.
- Use exact closure-row names in `CLAUSES_REVIEWED`.
- Make `NEGATIVE_PROOF`, `COUNTERFACTUAL_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS` cite product files or symbols.
- If sync writes before validation, patch the closeout script to throw into rollback on self-validation failures.

### Merge-Pending Terminal Projection

Symptom: terminal packet/task-board truth is `Done` / `MERGE_PENDING`, but `orchestrator-next` says ready to delegate to Coder, `wp-lane-health` reports closed-role/stale-receipt stall issues, or runtime drift says `current_milestone` should still be `VERDICT`.

Likely causes:

- Stale Activation readiness is outranking terminal closeout truth.
- Runtime drift still applies active direct-review rules after terminal merge-pending publication.
- Task Board displays `MERGE_PENDING` while script logic only treats `DONE_MERGE_PENDING` as terminal.
- Closeout projection uses containment milestone while older guidance expected verdict milestone.
- Session registry rows are closed but stale prelaunch state still appears in packet/readiness artifacts.
- Diagnostic code reads a packet resolver object as a string, misses the packet-declared runtime file, and silently loses terminal projection truth.
- Lane-health checks evaluate closed role sessions, stale receipts, or old auto-relay readiness without first fencing terminal packet/task-board state.

First actions:

- Treat `MERGE_PENDING` and `DONE_MERGE_PENDING` as terminal Orchestrator history, not Coder delegation.
- `orchestrator-next` should show `wp-truth-bundle`, packet read, and contained-main closeout command after actual local-main containment.
- `wp-lane-health` should print `Terminal WP` and suppress closed-session, stale-receipt, notification, hook, and auto-relay issues that are only terminal history.
- `wp-relay-watchdog --observe-only` should return `TERMINAL_HISTORY_HIDDEN`, and `wp-autonomous-monitor --once` should log `terminal=YES publication=...` without waking any role.
- Do not wake Coder, WP Validator, or Integration Validator just because old Activation readiness still says ready.
- Keep runtime projection in containment for merge-pending closeout; only active direct-review lanes require `VERDICT`.

### Main Containment Drift

Symptom: Integration Validator final PASS is present and closeout mode is `MERGE_PENDING`, but local `main` does not contain the approved target head or packet/task-board/runtime still say `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`.

Likely causes:

- The final product branch was validated but not merged into `../handshake_main`.
- The merge ran from the wrong worktree or branch.
- The approved target head differs from the branch HEAD being merged.
- Current committed branch states were not pushed before the local merge attempt.
- Contained-main closeout was not rerun with the new local-main SHA.

First actions:

- Check `git -C ../handshake_main branch --show-current`, `git -C ../handshake_main rev-parse HEAD`, and `git -C <wp_worktree> rev-parse HEAD`.
- Prove non-containment with `git -C ../handshake_main merge-base --is-ancestor <target_head> HEAD`.
- Push current committed `main` and WP feature branch state before any `git merge`.
- Merge the approved feature branch from `../handshake_main`, then rerun the ancestor check.
- Run contained-main `phase-check CLOSEOUT` with the merged-main SHA before pushing `origin/main`.

### Repomem Scope Drift

Symptom: Orchestrator mutation context writes under Coder/WP Validator memory session.

First actions:

- Run Orchestrator-owned wrappers with `--role ORCHESTRATOR --wp WP-{ID}`.
- Keep Coder/WP Validator/Integration Validator memory lanes open concurrently.
- Patch wrappers that call `repomem-gate` or `repomem context` without explicit role/WP scope.

### Closeout Mechanical Failure

Symptom: `closeout-repair` or `phase-check CLOSEOUT` fails.

First actions:

- Fix exactly the deterministic diagnostic first.
- Rerun `closeout-repair` and `phase-check CLOSEOUT`.
- Do not launch Integration Validator with broken mechanical truth.

## Primary Cross-Links

- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
- `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
- `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
- `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`

