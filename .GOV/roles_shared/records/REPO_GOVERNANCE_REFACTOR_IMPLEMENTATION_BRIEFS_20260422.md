# Repo Governance Refactor Implementation Briefs

**Date:** 2026-04-22  
**Authority:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`  
**Scope:** Companion implementation briefs for `RGF-217`, `RGF-218`, `RGF-219`, `RGF-220`, and `RGF-221`

---

## RGF-217 - Blocking Authority Matrix and Non-Canonical Artifact Demotion

**Board summary:** Only artifacts that define or judge product correctness may block product outcome; topology, route, dossier, provenance, repomem, and artifact-hygiene support surfaces must become machine-visible governance debt unless they invalidate the correctness judgment itself.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5566`
  Final-lane closeout paid a four-step repair chain after the validator verdict because support-surface failures were still blocking settlement.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5571`
  The product outcome was already authoritative and specific before the repair chain finished.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5572`
  Stale `route_anchor_*` projection still created a governance bug after packet/runtime truth was already terminal.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5868`
  The run paid a distinct settlement tax after the product outcome was already known.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5869`
  The dossier explicitly says that settlement tax is the least acceptable part of the run.

### Problem Statement

The closeout path still lets support surfaces behave like co-authoritative gates. That means a valid product outcome can remain trapped behind topology drift, route residue, artifact hygiene, provenance formatting, or other governance support defects that do not change the actual correctness judgment.

### Required State Contract

1. Shared closeout dependency evaluation must classify each dependency by authority:
   - `PRODUCT_CORRECTNESS`
   - `GOVERNANCE_SUPPORT`
2. Only `PRODUCT_CORRECTNESS` failures may block product outcome.
3. `GOVERNANCE_SUPPORT` failures must surface as machine-readable debt keys and summaries without erasing or delaying the product verdict of record.
4. `phase-check CLOSEOUT`, validator closeout readers, and any terminal status surface must expose both:
   - product-outcome blockers
   - governance debt
5. Terminal non-pass settlement must not fail solely because active-topology artifact hygiene or similar support surfaces still need repair.
6. This authority split must be shared, not reimplemented separately in each reader or script.

### Explicit Non-Goals

- Do not weaken product-correctness checks such as candidate-vs-signed-scope proof or required PASS compatibility truth.
- Do not make stale or malformed validator findings acceptable if the correctness judgment itself cannot be reconstructed.
- Do not add a second closeout truth mesh; this item is about reducing authority, not adding more parallel projections.

### Implementation Surfaces

- `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`
- any shared closeout authority helper introduced for dependency classification
- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
- `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
- optional closeout status surfaces that need to project governance debt distinctly from outcome blockers
- corresponding regression tests, including at minimum:
  - `.GOV/roles_shared/tests/wp-closeout-dependency-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`

### Acceptance Criteria

1. Shared closeout dependency output distinguishes outcome blockers from governance debt.
2. A terminal non-pass verdict remains authoritative even when non-canonical governance support surfaces still need repair.
3. At least one known support blocker class from the Calendar Sync Engine run is demoted from hard closeout failure to explicit settlement debt.
4. Validator closeout readers expose the new split mechanically.
5. Regression coverage proves:
   - product-correctness failures still block
   - governance-support failures surface as debt
   - the shared authority classification is reused across readers

### Regression Commands

```powershell
node --test .GOV/roles_shared/tests/wp-closeout-dependency-lib.test.mjs
node --test .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs
node --test .GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs
```

### Before / After Example

- Before this fix:
  A terminal FAIL could already exist, but closeout still failed because support surfaces such as artifact hygiene or route projection remained red.
- After this fix:
  The same situation should surface `verdict_of_record=FAIL` plus explicit governance debt, while only correctness-critical failures remain outcome-blocking.

## RGF-218 - Sparse Repomem Event Contract and WP Retrospective Compiler

**Board summary:** Mid-run dossier narration must be retired in favor of sparse structured per-role `repomem` checkpoints, and WP closeout must compile the retrospective mechanically from receipts, gate artifacts, runtime truth, and all role memories.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:341`
  The dossier template still assumes append-only live execution logging during WP execution.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:349`
  Idle-ledger narration is also framed as live append-only maintenance.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:362`
  Governance-change tracking remains a live execution document.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:389`
  Findings logging is likewise treated as something maintained during the run.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5567`
  Token burn on the run was dominated by governance/runtime recovery rather than fresh product implementation.

### Problem Statement

Mid-run narrative maintenance duplicates runtime truth, increases token spend, and pressures the orchestrator to keep a human-readable story synchronized while the workflow is still executing. That is the wrong place to spend tokens.

### Required State Contract

1. Runtime capture during execution must be sparse and structured.
2. `repomem` becomes the per-role durable learning/event ledger for WP-scoped failures, decisions, escalations, and closures.
3. The workflow dossier becomes a retrospective artifact compiled at or after WP closeout, not a live narrative that must be maintained throughout the run.
4. The retrospective compiler must gather, at minimum:
   - receipts / notifications
   - gate artifacts
   - runtime truth
   - all role `repomem` entries for the WP
5. Roles must not be encouraged to narrate every step in `repomem`; only durable failure/decision/resolution events belong there.
6. Existing repomem coverage checks must remain meaningful after the runtime capture model is simplified.

### Explicit Non-Goals

- Do not remove the dossier as a diagnostic artifact.
- Do not turn `repomem` into another chat transcript.
- Do not require the orchestrator to hand-write a live postmortem during execution.

### Implementation Surfaces

- `roles_shared/scripts/memory/repomem*.mjs`
- `roles_shared/scripts/audit/workflow-dossier*.mjs`
- closeout hooks that currently append live narrative notes
- memory hygiene report / recent WP repomem reporting
- any post-run audit or dossier compiler introduced for cross-role WP export
- corresponding regression tests for the repomem contract and dossier compilation path

### Acceptance Criteria

1. During a normal governed run, roles write only sparse durable `repomem` events instead of maintaining a live narrative dossier.
2. A WP-level aggregation command can export cross-role repomem coverage for one WP mechanically.
3. A dossier or postmortem can be recompiled from runtime artifacts plus repomem without requiring mid-run prose maintenance.
4. Repomem coverage debt remains machine-visible after the change.

### Regression Commands

```powershell
node --test .GOV/roles_shared/tests/repomem-coverage-lib.test.mjs
node --test .GOV/roles_shared/tests/repomem-open-contract-lib.test.mjs
node --test .GOV/roles_shared/tests/workflow-dossier-lib.test.mjs
```

### Before / After Example

- Before this fix:
  The dossier shape encouraged live execution narration while the run was still active.
- After this fix:
  Roles record only durable WP memory, and the retrospective is compiled once from mechanical runtime artifacts plus repomem at closeout.

## RGF-219 - Terminal Settlement Fence and Route Projection Quarantine

**Board summary:** Once a verdict-of-record exists, terminal lanes must not be kept artificially live by stale route anchors, relay projections, or late status mirrors; any remaining repair must stay bounded as settlement debt.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5561`
  The orchestrator still had to close a lingering final-lane thread after runtime truth had already reached terminal `Validated (FAIL)`.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5566`
  Stale `route_anchor_*` projection was one of the blockers in the post-verdict repair chain.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5572`
  The governance bug explicitly says terminal packet-to-runtime sync failed to clear persisted route anchors, so the lane still projected a live review boundary.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5868`
  Settlement tax included final FAIL publication and terminal route-anchor repair.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5869`
  The run should have become close to atomic after the validator produced a sound FAIL, but did not.

### Problem Statement

Even after verdict-of-record exists, stale route projections and lingering session/runtime mirrors can keep the lane looking active. That causes unnecessary wakes, duplicated repair effort, and continued token spend after judgment is already known.

### Required State Contract

1. Verdict-of-record creates a terminal settlement fence for the WP.
2. After that fence exists, stale route anchors or relay projections cannot make the lane look review-active again.
3. Status surfaces must prefer terminal verdict truth over stale route/runtime residue.
4. Lingering session closure after terminal verdict must be treated as bounded settlement debt or automated cleanup, not as renewed live-lane truth.
5. The fence must be shared across route readers, orchestrator-next/steer logic, and terminal status surfaces.

### Explicit Non-Goals

- Do not remove relay diagnostics for genuinely non-terminal lanes.
- Do not suppress legitimate pre-verdict routing or open-review blockers.
- Do not create a second terminal state projection just for display.

### Implementation Surfaces

- route-anchor and communication-health readers
- `orchestrator-next`
- `orchestrator-steer-next`
- terminal status / session-registry readers
- active-lane or closeout brief surfaces that still project live route state after verdict
- corresponding route and closeout regression tests

### Acceptance Criteria

1. Once verdict-of-record exists, stale route state cannot keep the lane looking review-active.
2. Orchestrator relay helpers stop recommending duplicate wake actions for terminal lanes with only settlement debt remaining.
3. Lingering terminal session cleanup no longer masquerades as reopened workflow activity.
4. Regression coverage proves the terminal fence wins over stale route residue.

### Regression Commands

```powershell
node --test .GOV/roles_shared/tests/wp-closeout-dependency-lib.test.mjs
node --test .GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs
node --test .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs
```

### Before / After Example

- Before this fix:
  A final FAIL could already exist while route projection still made the lane look live and steerable.
- After this fix:
  The same lane should stay terminal, with any lingering cleanup exposed only as bounded settlement debt.

## RGF-220 - Heavy-Host Timeout Neutrality and Subcheck Attribution

**Board summary:** Governance must assume the host is under heavy load by default; shell or child-process timeouts are telemetry to inspect, not workflow truth, and bundled checks must attribute the actual failing subcheck instead of collapsing into timeout ambiguity.

### Trigger Evidence

- 2026-04-22 operator directive:
  Remove timeout-based governance assumptions and treat the host PC as under heavy load at all times.
- Procedural memory `#4577`:
  Aggregate `gov-check` failed intermittently under load even though the isolated bundle/sub-check runs passed, indicating the timeout wrapper itself was the noisy failure surface.

### Problem Statement

Bundled governance checks can still fail for timing reasons that do not reflect the underlying authority surfaces. When that happens, the workflow burns time repairing or rerunning wrappers instead of inspecting the actual failing subcheck or authoritative artifact.

### Required State Contract

1. Aggregate governance runners must not impose fragile child-process timeouts that convert host slowness into false FAIL state.
2. When a bundled aggregate does fail, it must identify the subcheck that failed instead of collapsing to generic timeout noise.
3. Operator-facing docs and runbooks must treat shell/plugin timeout as advisory telemetry unless receipts, runtime truth, or subcheck artifacts confirm a real failure.
4. Heavy-host assumptions must be reflected in dossier/runbook wording so retry discipline inspects authority surfaces first.

### Explicit Non-Goals

- Do not hide real failing subchecks.
- Do not remove ordinary stall detection from active workflows.
- Do not weaken correctness-critical closeout or validation gates.

### Implementation Surfaces

- `.GOV/roles_shared/checks/bundled-check-runner-lib.mjs`
- `.GOV/roles_shared/checks/gov-check.mjs`
- bundled aggregate check entrypoints
- timeout-facing docs such as `ORCHESTRATOR_PROTOCOL.md`, `RUNBOOK_DEBUG.md`, and startup/operator references

### Acceptance Criteria

1. Aggregate governance checks no longer fail purely because a fixed child timeout expires under load.
2. Failing bundle output names the real failing subcheck.
3. Docs say shell/plugin timeouts are advisory and require artifact inspection before retry loops escalate.

### Regression Commands

```powershell
node --check .GOV/roles_shared/checks/bundled-check-runner-lib.mjs
node .GOV/roles_shared/checks/spec-bundle-check.mjs
node .GOV/roles_shared/checks/wp-comm-bundle-check.mjs
node .GOV/roles_shared/checks/topology-bundle-check.mjs
```

## RGF-221 - Diagnostic-Only Token Cost Policy and Structured Dossier Ledgers

**Board summary:** Token-cost overrun must not block WP continuation or product outcome; it must remain machine-visible as diagnostic telemetry, and dossier output must present the raw cost signal in grouped ledgers instead of cross-cut flat printouts.

### Trigger Evidence

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5567`
  Token burn was dominated by governance/runtime recovery rather than fresh product implementation.
- 2026-04-22 operator directive:
  Token-cost limits have been blocking workpackets; keep the limits as detailed mechanical reporting instead of as a continuation blocker.
- 2026-04-22 operator directive:
  Current dossier raw output is acceptable, but it needs structure so printouts do not cross-cut each other.

### Problem Statement

The current token budget surface still behaves like a lane blocker in some operator flows, and the dossier presents cost and sync output too flatly to be useful under pressure. That makes governance spend harder to interpret and too easy to confuse with workflow truth.

### Required State Contract

1. Token budget and token-ledger drift remain visible, but are diagnostic-only cost telemetry.
2. `orchestrator-next` must not stop an orchestrator-managed lane solely because token cost crossed a threshold.
3. Cost surfaces must distinguish gross input, fresh input, cached replay, output, turns, and command counts.
4. Dossier sync and idle-ledger output must use grouped mechanical ledgers so related signals stay together.
5. Templates and canonical docs must describe token cost as reportable telemetry, not as a continuation-waiver gate.

### Explicit Non-Goals

- Do not remove token accounting or token-ledger drift detection.
- Do not hide high token spend.
- Do not turn the dossier into narrative prose again.

### Implementation Surfaces

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/wp-token-budget-lib.mjs`
- `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
- `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
- `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
- dossier/template/operator doc surfaces that still describe token-cost waivers or flat cost output

### Acceptance Criteria

1. Token-cost overrun no longer blocks WP continuation in orchestrator-managed flow.
2. Dossier cost output reports grouped time and token ledgers with gross/fresh/cached detail.
3. Dossier live sync lines group counts, route, settlement, repomem, tokens, and host-load state.
4. Canonical docs stop telling operators to use governance waivers just to continue after token-cost overrun.

### Regression Commands

```powershell
node --test .GOV/roles_shared/tests/wp-token-budget-lib.test.mjs
node --test .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs
node --test .GOV/roles_shared/tests/wp-timeline-lib.test.mjs
```
