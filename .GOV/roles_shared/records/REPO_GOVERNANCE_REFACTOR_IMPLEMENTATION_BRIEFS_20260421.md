# REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260421

**Status:** Active  
**Scope:** Companion implementation briefs for `RGF-211`, `RGF-215`, and `RGF-216`  
**Authority chain:** `.GOV/roles_shared/docs/REPO_GOVERNANCE_REFACTOR_ROADMAP.md` + `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` + `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md`

## Purpose

This record turns three queued governance-board items into implementation-ready execution contracts for a fresh Orchestrator or coder. The goal is to avoid re-reading the full Calendar Sync Engine dossier just to reconstruct:

- the exact failure shape
- the state contract that must hold after the fix
- the boundaries that must not be widened accidentally
- the regression commands that must pass before the item is marked `DONE`

## Startup Note

These are governance-kernel refactor items, not product work packets. `BUILD_ORDER.md` must remain synced after governance record mutations, but it is not the startup authority for `RGF-*` work. Startup authority for this tranche is:

1. `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
2. this brief record
3. the cited Calendar Sync Engine dossier evidence

Do not block governance startup because `BUILD_ORDER.md` has no ranked `RGF-*` rows.

## Shared Rules

1. Stay inside `/.GOV/`. No product-code widening is part of these items.
2. Prefer shared state/projection library fixes over wrapper-only patches whenever the failure is semantic rather than transport-only.
3. Add or extend regression coverage adjacent to every touched shared library or role-local library.
4. Preserve audit history. Superseded candidates, verdict artifacts, and failed settlement attempts remain visible as history even when a new authoritative truth is adopted.
5. After mutating board/packet/governance records for these items, run `just build-order-sync` and `just gov-check`.

## RGF-211 - Non-Pass Closeout Atomicity and Active-Topology Artifact Hygiene

**Board summary:** A terminal non-pass closeout must settle in one pass with provenance stamped, route anchors cleared, packet/runtime/task-board truth synchronized, and artifact hygiene scoped to the active declared topology instead of unrelated sibling checkouts.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5566`
  The closeout repair chain still consumed additional time after the validator verdict because four mechanical blockers had to be repaired in sequence.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5571`
  The final FAIL verdict already existed and was concrete.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5572`
  Terminal packet-to-runtime sync still left stale `route_anchor_*` projection behind after packet status was already terminal.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5576`
  Artifact-hygiene repair touched sibling checkouts that were outside the active WP lane.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5582`
  The dossier explicitly calls out that unrelated stale worktrees can currently block a lawful terminal sync.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5594`
  The run confirms non-pass closeout is still repair-heavy instead of atomic.

### Problem Statement

The system can already reach a technically sound final FAIL, but terminal publication still behaves like a second workflow. That is the defect. Once a final-lane non-pass verdict is authoritative, the remaining closeout path must be deterministic, single-pass, and scoped to the topology that actually belongs to the active WP.

### Required State Contract

1. A final-lane authoritative non-pass verdict is sufficient to enter closeout settlement. The system must not fall back to `verdict=UNKNOWN` or a pseudo-live review boundary after that point.
2. `phase-check CLOSEOUT ... --sync-mode FAIL` is the canonical mutation surface for terminal non-pass settlement. Support helpers may prepare inputs, but terminal publication truth must settle through the phase-owned path.
3. The terminal mutation must, in one coherent write sequence:
   - stamp closeout provenance
   - project packet/runtime/task-board terminal truth
   - clear `route_anchor_state`, `route_anchor_kind`, `route_anchor_correlation_id`, `route_anchor_target_role`, and `route_anchor_target_session`
   - preserve validator-of-record and governed closeout action provenance
4. Artifact hygiene for terminal non-pass closeout must be scoped to the active declared topology only.
   Active declared topology means:
   - the active WP worktree(s) declared by packet/runtime truth
   - the active governance kernel worktree participating in the run
   - any other worktree explicitly declared for this WP in authoritative packet/runtime topology surfaces
5. Unrelated sibling checkouts may still be reported, but they must not block terminal non-pass publication unless they are part of the active declared topology.
6. Validation-report formatting must be canonical before terminal publication is attempted. Terminal closeout must not require a second pass just to re-shape scalar or list fields into packet-complete format.

### Explicit Non-Goals

- Do not widen this item into PASS-lane merge containment work. `RGF-209` already owns PASS closeout dependency collapse.
- Do not make orphaned sibling worktrees invisible. They may remain advisory or preflight-visible; they just cannot block unrelated non-pass settlement.
- Do not reintroduce a separate public closeout command alongside `phase-check CLOSEOUT`.

### Implementation Surfaces

- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
- `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
- `.GOV/roles_shared/scripts/wp/wp-closeout-format.mjs`
- topology / artifact-hygiene helpers that currently scan sibling worktrees during closeout
- the corresponding regression tests:
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/closeout-repair.test.mjs`
  - `.GOV/roles_shared/tests/wp-closeout-format.test.mjs`

### Acceptance Criteria

1. Given an authoritative final FAIL, one canonical `phase-check CLOSEOUT ... --sync-mode FAIL --context ...` pass settles terminal closeout truth without a follow-on route-anchor repair step.
2. After terminal sync, `wp-communication-health-check` does not continue projecting a live validator-review boundary from stale `route_anchor_*` fields.
3. Unrelated stale `.cargo/config.toml` or artifact-root drift in sibling worktrees outside the active declared topology no longer blocks terminal non-pass closeout.
4. Closeout provenance, publication mode, and verdict are all visible immediately after the terminal closeout pass.
5. Regression coverage proves:
   - route-anchor clearing on terminal non-pass sync
   - topology scoping for artifact hygiene
   - no second-pass formatting repair for terminal report publication

### Regression Commands

```powershell
node --test .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs
node --test .GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs
node --test .GOV/roles/orchestrator/tests/closeout-repair.test.mjs
node --test .GOV/roles_shared/tests/wp-closeout-format.test.mjs
```

### Before / After Example

- Before this fix:
  The Calendar Sync Engine run had a sound final FAIL at dossier line `5571`, but closeout still needed packet truth repair, sibling artifact-hygiene repair, validation-report formatting repair, and terminal route-anchor repair before the authoritative closeout artifact was written.
- After this fix:
  The same state should publish `Validated (FAIL)` truth, closeout provenance, and a route-anchor-free runtime projection in one phase-owned closeout pass.

## RGF-215 - Superseding Candidate and PREPARE Truth Rollover

**Board summary:** When a lawful superseding candidate becomes the validator source of truth, PREPARE truth, packet source metadata, and handoff identity must roll over transactionally instead of leaving stale original-branch expectations behind.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:4561`
  `wp-coder-handoff` was rejected because packet PREPARE truth still expected the original branch while the superseding clean-main candidate lived on `feat/WP-1-Calendar-Sync-Engine-v2-mainproof`.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:4581`
  Runtime and validator-next projections were left unresolved after the candidate source changed.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:4684`
  The Orchestrator had to record superseding PREPARE truth manually after clean-main remediation.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:4685`
  A corrected PREPARE gate-log entry had to be appended manually because stale last-prepare truth blocked normal repair.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5067`
  Packet truth drifted into a policy conflict by overloading `WP_VALIDATOR_LOCAL_BRANCH`.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5446`
  Final-lane startup was blocked because packet truth still carried the wrong branch semantics into validator policy checks.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5447`
  The eventual repair had to restore canonical validator branch policy while preserving the superseding proof candidate elsewhere in packet truth.

### Problem Statement

The system currently conflates two different concepts:

1. role-local branch/session policy for a validator lane
2. the authoritative candidate branch/range currently under review

When a lawful superseding candidate replaces the earlier source of truth, those two concepts must roll forward together without being written into the same field opportunistically.

### Required State Contract

1. The authoritative candidate-under-review must be represented explicitly and separately from role-local branch policy fields.
2. A superseding-candidate rollover is one transaction, not a sequence of manual note-taking steps. The same mutation must update:
   - PREPARE truth
   - packet source-candidate metadata
   - runtime review / committed-range projection
   - handoff identity surfaces consumed by `wp-coder-handoff`
   - validator and integration-validator context readers
3. Historical candidate truth must remain preserved as superseded history. The fix must not destroy audit visibility into the original candidate.
4. `WP_VALIDATOR_LOCAL_BRANCH` keeps its role-policy meaning. Do not repurpose it to mean "latest candidate branch under review" if session policy still expects a canonical validator branch.
5. After rollover, downstream readers must converge without manual appended PREPARE rows or packet surgery:
   - `validator-next`
   - `integration-validator-context-brief`
   - handoff checks
   - final-lane context bundle
6. The superseding candidate range must become the single authoritative handoff range used by validator and closeout readers once the rollover is committed.

### Explicit Non-Goals

- Do not rewrite session policy so every validator lane follows arbitrary candidate branches automatically.
- Do not erase the original candidate branch from history.
- Do not treat ad hoc packet edits as an acceptable replacement for a typed rollover path.

### Implementation Surfaces

- PREPARE gate writers and `record-prepare` path
- packet branch / source-candidate metadata writers
- `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
- handoff identity / review projection readers
- `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- validator handoff / final-lane context helpers
- corresponding regression tests, including at minimum:
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`

### Acceptance Criteria

1. A lawful superseding candidate can be recorded once and immediately becomes the authoritative candidate source for handoff and final-lane readers.
2. After that rollover, `wp-coder-handoff` is not rejected because of stale original-branch PREPARE truth.
3. `validator-next` and `integration-validator-context-brief` no longer surface `UNKNOWN`, `UNRESOLVED`, or role-policy conflicts when the only change is a lawful superseding candidate.
4. `WP_VALIDATOR_LOCAL_BRANCH` remains semantically correct for validator session policy and is not used as the carrier of superseding-candidate source truth.
5. Regression coverage proves:
   - original candidate preserved as history
   - superseding candidate adopted as authoritative source of truth
   - downstream readers converge on the new candidate range without manual packet edits

### Regression Commands

```powershell
node --test .GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs
node --test .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs
node --test .GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs
```

### Before / After Example

- Before this fix:
  The run created `feat/WP-1-Calendar-Sync-Engine-v2-mainproof` as the lawful clean-main candidate, but packet PREPARE truth and validator policy still expected the original branch, so handoff and final-lane startup diverged until the Orchestrator manually repaired both packet truth and branch semantics.
- After this fix:
  Declaring the superseding candidate should atomically move candidate-under-review truth to the new range while preserving validator branch policy and historical candidate lineage.

## RGF-216 - Verdict-First Dossier Capture and Settlement Debt Ledger

**Board summary:** The first authoritative final-lane PASS/FAIL must be recorded immediately as verdict truth, and any remaining mechanical repair must be tracked as explicit settlement debt rather than leaving the impression that verdict state is still undecided.

### Trigger Evidence From The Calendar Sync Engine Run

- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5312`
  A concrete final-lane FAIL verdict already existed with file-anchored reasons.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5317`
  Canonical FAIL closeout sync was still blocked after that verdict.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5318`
  `phase-check CLOSEOUT` continued to fail even though the verdict had already been established.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5566`
  Additional repair work was still required after the verdict.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5571`
  The final FAIL itself was already authoritative and specific.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5583`
  The dossier explicitly distinguishes lawful closeout from excessive governance tax after the verdict.
- `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md:5667`
  The addendum calls for the final closeout failure chain to be recorded as soon as the first non-pass verdict is authoritative.

### Problem Statement

Today the workflow can know the verdict but still look undecided because verdict truth and settlement truth are collapsed into one surface. That creates rereads, route churn, and misleading `VERDICT: PENDING` or `UNKNOWN` projections after a final review has already concluded.

### Required State Contract

1. Final-lane verdict truth and closeout settlement truth are separate concepts and must be stored and surfaced separately.
2. The first authoritative Integration Validator verdict must immediately stamp:
   - verdict value
   - verdict timestamp
   - verdict session / actor of record
   - verdict evidence pointer or artifact reference
3. If terminal publication or artifact settlement is still blocked, that state must be expressed as settlement debt, not by reverting the verdict to `PENDING` or `UNKNOWN`.
4. Dossier sync, session status, and final-lane context readers must all preserve the same distinction:
   - `verdict_of_record`
   - `settlement_state`
   - `settlement_blockers` or equivalent machine-readable debt keys
5. Settlement debt is additive. It does not weaken, overwrite, or hide an authoritative verdict.
6. Once settlement completes, the debt ledger can resolve to empty, but the original verdict timestamp and actor of record remain stable.

### Explicit Non-Goals

- Do not publish final packet/task-board terminal status before lawful closeout if the existing workflow still requires terminal sync to complete.
- Do not add a second narrative-only ledger. This must be machine-readable and consumed by live status surfaces.
- Do not re-open validator review just because settlement debt exists.

### Implementation Surfaces

- `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
- `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
- `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
- `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
- `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
- `.GOV/roles_shared/scripts/wp/wp-closeout-format.mjs`
- any schema / runtime projection surface that currently collapses verdict and settlement into one field
- corresponding regression tests, including at minimum:
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-closeout-format.test.mjs`

### Acceptance Criteria

1. After the first authoritative final-lane PASS or FAIL, all status surfaces continue to show that verdict even if closeout settlement is still blocked.
2. Settlement blockers are exposed as explicit debt keys or blocker rows rather than as `VERDICT: PENDING` / `UNKNOWN`.
3. The workflow dossier receives a verdict-first entry at the moment the verdict becomes authoritative, with later settlement work appended separately.
4. `session-registry-status` and final-lane context readers distinguish verdict truth from settlement truth in machine-readable form.
5. Regression coverage proves:
   - verdict survives a later settlement failure
   - settlement debt resolves without changing the verdict-of-record
   - status readers stop regressing to `UNKNOWN` after authoritative review

### Regression Commands

```powershell
node --test .GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs
node --test .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs
node --test .GOV/roles_shared/tests/wp-closeout-format.test.mjs
```

### Before / After Example

- Before this fix:
  The Integration Validator had already produced a concrete FAIL, but closeout blockers still made multiple surfaces look like the lane was not yet decided.
- After this fix:
  The same moment should produce `verdict_of_record=FAIL` plus explicit settlement debt, so the workflow looks truthfully closed in judgment even before settlement is complete.
