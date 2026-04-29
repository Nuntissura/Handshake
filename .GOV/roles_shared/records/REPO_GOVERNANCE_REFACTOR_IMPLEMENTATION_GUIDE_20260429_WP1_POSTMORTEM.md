# Repo Governance Refactor Implementation Guide - WP-1 Postmortem Tranche

**Date:** 2026-04-29
**Scope:** Governance-only implementation guide for proposed `RGF-255` through `RGF-264`
**Authority:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
**Evidence driver:** `AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW` / `SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE`
**Primary dossier:** `.GOV/Audits/smoketest/DOSSIER_20260427_SOFTWARE_DELIVERY_PROJECTION_SURFACE_DISCIPLINE_WORKFLOW_DOSSIER.md`

---

## Why This Guide Exists

WP-1 succeeded as a product result but failed as a sustainable orchestration model.

The final product candidate passed WP Validator, passed Integration Validator, and was contained in `main`. The cost of reaching that terminal state was not acceptable. Operator-observed spend exceeded 203M tokens in the first Orchestrator context and 107M tokens in the second Orchestrator context for one WP, with more than 14 hours elapsed. Mechanical telemetry in the dossier also shows high closeout churn: roughly 130 session/control commands, dozens of receipts, repeated closeout repair attempts, stale session residue, and heavy read amplification.

The root issue was not one broken check. The root issue was fragmented lifecycle authority. Product proof, packet truth, runtime truth, validator gate truth, candidate target truth, signed-scope truth, repomem coverage, session lifecycle, dossier projection, task-board projection, and main containment could disagree. The Orchestrator then became the manual reconciler.

This guide converts the WP-1 postmortem into implementation-ready repo-governance refactor items. It is written for a fresh model with no conversation context.

## Non-Goals

- Do not touch Handshake product code.
- Do not edit the Master Spec.
- Do not create a product Work Packet for these items unless the Operator explicitly changes the governance model.
- Do not route deterministic governance implementation or checks through ACP.
- Do not collapse role authority. Coder, WP Validator, Integration Validator, Activation Manager, Memory Manager, and Orchestrator remain separate roles.
- Do not treat governance debt as product failure unless candidate identity, signed scope, verdict, or containment proof is actually broken.

## Startup For The Next Model

Before implementing any item from this guide:

1. Run `just orchestrator-startup`.
2. Run `just repomem open "<specific RGF item or board mutation>" --role ORCHESTRATOR`.
3. Verify the latest `RGF-*` IDs in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`; this guide reserves `RGF-255` through `RGF-264` only if no newer rows were added.
4. Read the current board rows for `RGF-233` through `RGF-254`. This guide assumes the closeout canonicalization and memory follow-on concepts are known or in progress.
5. Implement one item or a tightly coupled pair at a time.
6. Update `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md` for completed work.
7. Run focused tests for the changed helper(s), then `just gov-check`.

If a command fails or a workaround is discovered, immediately run:

```powershell
just memory-capture procedural "<what failed and the fix>" --role ORCHESTRATOR
```

## Proposed Board Rows

Add these to the Post-Refactor Follow-On Board only after confirming the IDs are still free.

```md
| RGF-255 | PLANNED | Compact WP Truth Bundle | RGF-233, RGF-234, RGF-237 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `roles/orchestrator/scripts/orchestrator-next.mjs`, `roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`, `roles_shared/scripts/session/*`, `roles_shared/scripts/audit/workflow-dossier-lib.mjs` | one command emits a bounded current-truth bundle for a WP without rereading packet/runtime/session/dossier surfaces separately |
| RGF-256 | PLANNED | Executable Packet Acceptance Matrix | RGF-248, RGF-250 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `packet-closure-monitor-lib.mjs`, `validator-report-profile-lib.mjs`, packet template, Coder/WP Validator protocols | WP Validator PASS is mechanically blocked unless every packet acceptance row is PROVED/CONFIRMED or NOT_APPLICABLE with evidence |
| RGF-257 | PLANNED | Receipt-Driven Auto-Progression | RGF-245, RGF-248, RGF-247 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `wp-receipt-append.mjs`, `wp-notification-append.mjs`, `nudge-queue-lib.mjs`, `orchestrator-next.mjs`, `session-control-lib.mjs` | Coder handoff, WP Validator verdict, and Integration Validator verdict enqueue the next legal actor exactly once without Orchestrator relay |
| RGF-258 | PLANNED | Orchestrator Cost Governor and Recovery Mode | RGF-18, RGF-22, RGF-253 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `wp-token-budget-lib.mjs`, `wp-token-usage-lib.mjs`, `orchestrator-next.mjs`, `orchestrator-steer-next.mjs`, operator monitor | token/command/time overruns constrain wasteful Orchestrator rediscovery while allowing productive governed role work to continue |
| RGF-259 | PLANNED | Failure-Class Recovery Router | RGF-233, RGF-234, RGF-237, RGF-244 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | Integration Validator closeout libs, validator gate scripts, artifact hygiene libs, runtime invalidity helpers | Integration Validator failures classify as PRODUCT_BLOCKER, ENVIRONMENT_BLOCKER, or GOVERNANCE_BLOCKER with distinct recovery paths |
| RGF-260 | PLANNED | Terminal Verdict Session Finalizer | RGF-236, RGF-240, RGF-251 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | session registry, broker result ledger, `repomem.mjs`, `integration-validator-closeout-sync.mjs` | terminal PASS/FAIL closes or quarantines stale role sessions and records session-close proof before final gov-check |
| RGF-261 | PLANNED | Dossier Closeout Judgment Auto-Fill | RGF-222, RGF-223, RGF-224, RGF-243, RGF-254 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `workflow-dossier.mjs`, `workflow-dossier-lib.mjs`, dossier template, phase-check CLOSEOUT | closeout fails if rubric placeholders or stale narrative claims remain after terminal metrics and repomem import |
| RGF-262 | PLANNED | Artifact Root Preflight Before Final Lane | RGF-235, RGF-244 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `artifact-hygiene-lib.mjs`, validator git hygiene, coder post-work check, handoff phase check | noncanonical artifact roots such as `Handshake Artifacts` fail before Integration Validator final review starts |
| RGF-263 | PLANNED | Baseline Compile Waiver Ledger | RGF-09, RGF-12, RGF-250 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | workflow invalidity helpers, packet scope metadata, Coder pre/post checks, waiver evidence ledger | out-of-scope baseline blockers require one bounded waiver/unblocker record and cannot create repeated informal Operator interruptions |
| RGF-264 | PLANNED | Governance Refactor Board Integrity Check | RGF-254 | AUDIT-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE-SMOKETEST-REVIEW / SMOKETEST-REVIEW-20260427-SOFTWARE-DELIVERY-PROJECTION-SURFACE-DISCIPLINE | `REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`, changelog, gov-check bundle | board summary, row table, execution briefs, and follow-on sequence cannot disagree about implemented or queued RGF IDs |
```

## Recommended Sequence

1. `RGF-264` first if the board has drift. It makes later rows safer to track.
2. `RGF-255` next. Every later cost-control and closeout repair flow should consume the compact truth bundle.
3. `RGF-258` after `RGF-255`. The cost governor needs a compact truth bundle to avoid becoming another verbose status path.
4. `RGF-256` and `RGF-262` before the next large product WP. These reduce late Integration Validator discoveries.
5. `RGF-257` once receipt/verb routing fixtures are understood. It changes progression behavior and must be conservative.
6. `RGF-259` and `RGF-260` after or alongside the closeout canonicalization tranche.
7. `RGF-261` after compact check output and repomem import surfaces are stable.
8. `RGF-263` can run independently if baseline compile blockers keep interrupting orchestrator-managed WPs.

## RGF-255 - Compact WP Truth Bundle

### Reason

WP-1 takeover required reading packet, runtime, session registry, validator gate, closeout logs, repomem import, dossier, task-board state, and main containment. That is not a sustainable takeover or status model.

### Required Behavior

Create one command, recommended:

```powershell
just wp-truth-bundle WP-{ID}
```

Default output must be compact and bounded. It should include:

- `wp_id`
- packet status
- runtime status
- task-board status
- active MT / next MT
- next actor
- waiting_on
- validator gate state
- final verdict if present
- candidate commit and branch
- signed-scope status
- product main containment status
- closeout dependency summary
- session summary: active, queued, stale, terminal residue
- repomem coverage status
- open product blockers
- governance debt keys
- exact next command
- artifact path for full JSON detail

### Likely Surfaces

- `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`
- `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/wp-token-usage-report.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
- `justfile`

### Implementation Plan

1. Add a library function `buildWpTruthBundle({ wpId, mode })`.
2. Reuse existing readers. Do not parse giant human logs if structured JSON or runtime state exists.
3. Emit compact text by default and write full JSON to `../gov_runtime/roles_shared/WP_COMMUNICATIONS/<WP_ID>/truth_bundle/<timestamp>.json`.
4. Add `--json` for machine-readable stdout.
5. Update `orchestrator-next` to point to this bundle when a WP is terminal, stalled, or over budget.

### Acceptance Tests

- Fixture: terminal PASS with stale session residue. Bundle reports terminal PASS and governance debt, not live work.
- Fixture: missing signed-scope proof. Bundle reports product blocker.
- Fixture: no active WP. Bundle fails with a clear diagnostic.
- Output length test: default output remains below a fixed line count, for example 80 lines.

## RGF-256 - Executable Packet Acceptance Matrix

### Reason

WP-1 reached Integration Validator with packet-required proof still missing in an earlier final-lane attempt. The governed-action preview payload and exact tripwire should have been blocked by WP Validator before final-lane review.

### Required Behavior

Packet acceptance criteria must become a machine-readable matrix. Coder attaches proof per row. WP Validator must mark every row:

- `PROVED` / `CONFIRMED`
- `NOT_APPLICABLE`
- `STEER`
- `BLOCKED`

WP Validator PASS must fail if any required row lacks proof.

### Likely Surfaces

- `.GOV/roles_shared/scripts/lib/packet-closure-monitor-lib.mjs`
- `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs`
- `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
- `.GOV/templates/TASK_PACKET_TEMPLATE.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`

### Implementation Plan

1. Define an acceptance-row schema with stable IDs, required evidence kind, owner role, and status.
2. Add a parser that extracts existing packet acceptance rows without requiring a packet rewrite first.
3. Add a validator-report section for matrix closure.
4. Update WP Validator PASS checks to require all rows closed.
5. Preserve legacy packet compatibility by deriving missing rows when possible and warning when not possible.

### Acceptance Tests

- Candidate missing governed-action preview proof fails WP Validator PASS.
- Candidate with all rows closed passes the matrix check.
- `NOT_APPLICABLE` requires a reason.
- Matrix closure output lists exact row IDs and missing evidence.

## RGF-257 - Receipt-Driven Auto-Progression

### Reason

WP-1 repeatedly needed Orchestrator re-wakes after Coder handoffs and validator responses. Receipts existed, but the next actor did not reliably self-progress.

### Required Behavior

When a governed receipt is appended, the system should enqueue the next legal actor exactly once if:

- topology is valid
- the next actor has no active/queued equivalent command
- all required gate predicates are satisfied
- the WP is not terminal

### Likely Surfaces

- `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
- `.GOV/roles_shared/scripts/wp/wp-notification-append.mjs`
- `.GOV/roles_shared/scripts/session/nudge-queue-lib.mjs`
- `.GOV/roles_shared/scripts/lib/inter-role-verb-lib.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`

### Implementation Plan

1. Add `deriveNextActionFromReceipt(receipt)` using named verbs from `RGF-248`.
2. Before enqueueing, check active and queued commands for the same role/WP/action.
3. Use the nudge queue, not direct ACP dispatch, for non-emergency progression.
4. Record a small auto-progression receipt with correlation IDs.
5. Add an opt-out flag for repair contexts where automatic progression must be suppressed.

### Acceptance Tests

- `MT_HANDOFF` enqueues WP Validator review once.
- Duplicate handoff does not enqueue a second review.
- `MT_VERDICT PASS` enqueues Coder next-MT acknowledgement/implementation once when another MT exists.
- Terminal `INTEGRATION_VERDICT PASS` does not enqueue Coder or WP Validator.

## RGF-258 - Orchestrator Cost Governor and Recovery Mode

### Reason

The Operator observed over 300M tokens across two Orchestrator contexts and over 14 hours elapsed for one WP. The goal is not to ban token burn: autonomous Coder, WP Validator, and Integration Validator work sometimes needs substantial context and time. The goal is to stop wasteful Orchestrator churn after the run crosses cost thresholds.

### Required Behavior

Orchestrator-managed WPs need hard budget bands for:

- gross input tokens
- fresh input tokens
- output tokens
- command count
- elapsed wall time
- repeated closeout-repair attempts
- repeated full-surface rereads

When a budget is exceeded, Orchestrator enters recovery mode. Productive governed role work may continue when the compact truth bundle says it is the next legal action. Wasteful Orchestrator behaviors become constrained:

- compact truth bundle
- closeout repair loop breaker
- explicit escalation packet
- rescue/status command
- Operator-authorized override

Examples of work that should still be allowed:

- Coder implementing a hard feature or bounded remediation.
- WP Validator independently checking a candidate.
- Integration Validator performing final proof.
- One deep recovery pass with stable blocker keys.

Examples of work that should be blocked or require override:

- broad packet/dossier/session rereads without a new blocker key
- repeated closeout repair attempts without blocker-set reduction
- duplicate steering to an already active or queued role
- command-surface rediscovery
- long status narration without mutation, decision, or escalation

### Likely Surfaces

- `.GOV/roles_shared/scripts/session/wp-token-budget-lib.mjs`
- `.GOV/roles_shared/scripts/session/wp-token-usage-lib.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
- `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

### Implementation Plan

1. Add a budget evaluator that returns `OK`, `WARN`, `RECOVERY_MODE`, or `OVERRIDE_REQUIRED`.
2. Feed it token ledger and command/session telemetry.
3. Make `orchestrator-next` display the budget state and permitted next commands.
4. Make `orchestrator-steer-next` refuse broad steering under `RECOVERY_MODE` unless the compact truth bundle names that steering as the next legal action.
5. Add an Operator override flag that records explicit authority and reason.

### Acceptance Tests

- Fixture over command budget blocks `orchestrator-steer-next`.
- Fixture over token budget allows `wp-truth-bundle`.
- Operator override is recorded and visible in status.
- Budget state appears in dossier telemetry.

## RGF-259 - Failure-Class Recovery Router

### Reason

WP-1 had product proof pass but Integration Validator blocked on noncanonical artifact root. The recovery path treated environment hygiene like a terminal validation problem and consumed too much closeout work.

### Required Behavior

Final-lane failures must be classified:

- `PRODUCT_BLOCKER`: product/spec proof failed; revalidation required after remediation.
- `ENVIRONMENT_BLOCKER`: product proof is intact but environment/artifact hygiene prevents merge.
- `GOVERNANCE_BLOCKER`: product proof is intact but governance projection/session/dossier state is inconsistent.

Each class must produce a distinct next command.

### Likely Surfaces

- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
- `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
- `.GOV/roles_shared/scripts/lib/closeout-blocking-authority-lib.mjs`
- `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`

### Implementation Plan

1. Add failure-class enum and classifier helper.
2. Classify validator git/artifact hygiene separately from product proof.
3. Preserve successful product proof metadata when the failure is environment-only.
4. Route environment-only remediation to a minimal hygiene repair path.
5. Include `revalidation_required` in the output.

### Acceptance Tests

- Artifact-root failure becomes `ENVIRONMENT_BLOCKER` with product proof preserved.
- Missing tripwire proof becomes `PRODUCT_BLOCKER`.
- Stale dossier import becomes `GOVERNANCE_BLOCKER`.
- Only `PRODUCT_BLOCKER` recommends Coder product remediation.

## RGF-260 - Terminal Verdict Session Finalizer

### Reason

WP-1 required deterministic cleanup of stale Integration Validator session state after terminal validation. Session cleanup should be part of terminal publication, not a later manual repair.

### Required Behavior

Terminal PASS/FAIL publication must:

- inspect all role sessions for the WP
- close safe active terminal sessions
- quarantine stale READY residue
- preserve active/queued blockers
- record repomem close/debt status
- make `gov-check` pass without an extra manual close command

### Likely Surfaces

- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/scripts/memory/repomem.mjs`
- `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
- `.GOV/roles_shared/checks/session-bundle-check.mjs`

### Implementation Plan

1. Add `finalizeTerminalSessions({ wpId, terminalRecord })`.
2. Treat active/queued commands as live blockers.
3. Treat idle READY state after terminal verdict as terminal residue.
4. Close/quarantine residue and append structured evidence.
5. Make session-bundle check understand the residue state.

### Acceptance Tests

- Terminal PASS with stale READY Integration Validator is finalized without reopening the WP.
- Active Coder command remains a live blocker.
- Repomem debt is reported but does not downgrade product verdict.
- `gov-check` passes after finalizer on terminal fixture.

## RGF-261 - Dossier Closeout Judgment Auto-Fill

### Reason

The WP-1 dossier had rich mechanical evidence, but narrative/rubric sections lagged and still contained placeholders after the WP was terminal. The Operator had to request a postmortem fill manually.

### Required Behavior

At closeout, dossier tooling must:

- import repomem
- append mechanical telemetry
- fill or validate rubric sections
- detect stale narrative claims contradicted by terminal truth
- fail closeout if required postmortem placeholders remain

### Likely Surfaces

- `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
- `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
- `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`
- `.GOV/roles_shared/docs/WORKFLOW_DOSSIER_RUBRIC.md`
- `.GOV/roles_shared/checks/phase-check-lib.mjs`

### Implementation Plan

1. Add a `workflow-dossier judgment-check` mode.
2. Detect placeholders such as `NONE yet`, `<SET_AT_CLOSEOUT>`, and stale MT statuses after terminal state.
3. Generate a compact rubric draft from metrics and debt keys.
4. Require Orchestrator judgment fields only where mechanical inference is insufficient.
5. Make closeout output name missing sections and exact file lines.

### Acceptance Tests

- Terminal dossier with rubric placeholders fails judgment-check.
- Dossier with stale narrative "MT-004 not started" after terminal PASS fails.
- Dossier with all rubric fields and appendix passes.
- Mechanical evidence remains append-only.

## RGF-262 - Artifact Root Preflight Before Final Lane

### Reason

WP-1 hit an Integration Validator failure because Cargo output routed to `Handshake Artifacts` instead of canonical `Handshake_Artifacts`. This should fail before final-lane review.

### Required Behavior

Before Integration Validator starts, artifact hygiene must prove:

- canonical artifact root exists or is creatable
- noncanonical sibling roots are absent or quarantined
- product `.cargo/config.toml` target-dir points to canonical root
- proof command output lands in canonical root

### Likely Surfaces

- `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
- `.GOV/roles/validator/checks/validator-git-hygiene.mjs`
- `.GOV/roles/coder/checks/post-work-check.mjs`
- `.GOV/roles_shared/checks/phase-check.mjs`

### Implementation Plan

1. Add an artifact-root preflight used by HANDOFF and CLOSEOUT phase checks.
2. Make it classify failure as environment blocker.
3. Include exact path diagnostics and remediation command.
4. Avoid deleting artifacts unless an explicit cleanup command is run.

### Acceptance Tests

- `.cargo/config.toml` pointing to `Handshake Artifacts` fails before Integration Validator.
- Correct underscore path passes.
- Existing noncanonical sibling root reports environment debt with remediation.
- Product proof checks are not run after preflight hard fail.

## RGF-263 - Baseline Compile Waiver Ledger

### Reason

WP-1 repeatedly encountered out-of-scope baseline compile blockers. Each required Operator interruption and ad hoc waiver reasoning. The system needs a bounded waiver ledger.

### Required Behavior

Out-of-scope baseline blockers must create a waiver or unblocker record with:

- blocker command
- failing files/symbols
- allowed edit paths
- allowed edit kind
- expiry condition
- Operator authority reference
- proof command
- final outcome

Coder may only touch waiver-listed paths for waiver work.

### Likely Surfaces

- `.GOV/roles_shared/scripts/wp/wp-invalidity-flag.mjs`
- `.GOV/roles_shared/scripts/lib/scope-surface-lib.mjs`
- `.GOV/roles/coder/checks/pre-work-check.mjs`
- `.GOV/roles/coder/checks/post-work-check.mjs`
- packet templates and packet scope metadata

### Implementation Plan

1. Add a waiver ledger under WP communications or runtime authority.
2. Add helper `just wp-waiver-record WP-{ID} ...` or equivalent.
3. Update Coder checks to compare touched files against packet scope plus active waiver scope.
4. Add expiry: waiver closes after proof passes or after one failed repair attempt unless renewed.
5. Surface waiver state in active-lane brief.

### Acceptance Tests

- Out-of-scope file edit without waiver fails Coder post-work check.
- Waiver-listed edit passes when proof command succeeds.
- Waiver cannot silently expand to a second file.
- Expired waiver fails further edits.

## RGF-264 - Governance Refactor Board Integrity Check

### Reason

The refactor board is long and already has multiple summary, row, execution-brief, and sequence surfaces. If those drift, the next model cannot trust what is planned, queued, or done.

### Required Behavior

Add a check that validates:

- every `RGF-*` row ID is unique
- status summary matches row statuses
- execution brief references point to existing files
- follow-on sequence references existing rows
- proposed next sequence does not conflict with row status
- no row claims `DONE` without changelog or evidence reference, where existing conventions make that check possible

### Likely Surfaces

- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- `.GOV/roles_shared/checks/gov-check.mjs`
- new check, for example `.GOV/roles_shared/checks/repo-governance-board-check.mjs`

### Implementation Plan

1. Write a markdown parser specific to the board table shape.
2. Extract row IDs, statuses, dependencies, evidence, primary surfaces, and exit signals.
3. Extract execution brief references and sequence lists.
4. Validate consistency and emit compact diagnostics.
5. Wire into `gov-check`.

### Acceptance Tests

- Duplicate row ID fixture fails.
- Missing execution brief file fails.
- Sequence references unknown RGF ID fail.
- Clean current board passes or emits only explicitly grandfathered warnings.

## Cross-Tranche Regression Fixtures

Use fixtures based on WP-1 failure modes:

1. Product PASS, stale runtime waiting_on.
2. Product PASS, stale active microtask.
3. WP Validator PASS, missing Integration Validator final proof.
4. Integration Validator product proof PASS, artifact root environment FAIL.
5. Terminal PASS, stale Integration Validator READY session.
6. Terminal PASS, dossier rubric placeholders remain.
7. Terminal PASS, repomem coverage debt.
8. Baseline compile blocker outside scope with no waiver.
9. Duplicate Coder handoff receipt.
10. Orchestrator over command/token budget.

Each fixture should assert both the compact output and the structured JSON/log output.

## Final Done Criteria For The Tranche

The tranche is done only when:

- board rows are added with final IDs
- each item has tests
- `just gov-check` passes
- `REPO_GOVERNANCE_CHANGELOG.md` records completed items
- a future orchestrator-managed WP can be handed off using `wp-truth-bundle` instead of broad manual rereads
- closeout debt is distinguishable from product blockers in operator-facing output
- Orchestrator cost tripwires can stop a runaway run before another 100M+ token context forms
