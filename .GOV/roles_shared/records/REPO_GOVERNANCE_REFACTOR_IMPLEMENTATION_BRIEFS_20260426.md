# Repo Governance Refactor Implementation Briefs

**Date:** 2026-04-26  
**Authority:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`  
**Scope:** Companion implementation briefs for `RGF-233` through `RGF-241`  
**Operator driver:** Closeout repair loops still consume too much time and token budget because terminal state is distributed across authoritative, projection, and diagnostic artifacts.

---

## 2026-04-30 Triage Update

Do not implement this brief literally. Later governance tranches, especially `RGF-255` through `RGF-264`, already absorbed much of the original closeout canonicalization scope through compact WP truth, failure-class routing, terminal verdict session finalization, artifact-root preflight, baseline waiver ledgers, and dossier closeout checks.

Current board authority narrows this brief to three queued spine items:

- `RGF-233`: canonical terminal closeout record contract
- `RGF-240`: monotonic state machine and atomic publication for that record
- `RGF-241`: focused breakpoint fixture harness after the record/write model exists

The following rows are intentionally on `HOLD` unless fresh live evidence reactivates them:

- `RGF-234`: fold projection-sync requirements into `RGF-233`; no standalone proof/projection subsystem.
- `RGF-235`: fold product-main head capture into `RGF-233`; no second topology resolver.
- `RGF-236`: superseded by `RGF-260` unless stale terminal residue still reopens verdicts.
- `RGF-237`: fold compact debt formatting into `RGF-233`/`RGF-241`; no separate debt-report system.
- `RGF-238`: hold repair-loop budget work until current closeout repair shows non-convergence after `RGF-255` and `RGF-259`.
- `RGF-239`: no bulk legacy migration; add only lazy enough-evidence fallback inside a terminal-record reader if needed.

---

## Shared Closeout Failure Model

These items assume the following split. Implementers must preserve it in code and user-facing status:

- **Product outcome blocker:** a failure that prevents proving or trusting the reviewed product result. Examples: no validator verdict-of-record, candidate commit mismatch, signed-scope mismatch, reviewed commit absent from product main after claimed merge, current product-main compatibility not proven, or validator finding a real product/spec failure.
- **Governance settlement debt:** a failure in support surfaces after the product outcome can already be trusted. Examples: dossier import failure, repomem coverage gap, stale packet/task-board/build-order projection, route anchor residue, stale READY session row, provenance formatting drift, or token/cost ledger drift.
- **Projection drift:** a generated or mirrored artifact disagrees with canonical runtime/terminal state. Projection drift must be syncable without revalidating product outcome.
- **Legacy absence:** an older WP lacks the new canonical terminal record. This must route through one migration helper, not many ad hoc fallbacks.

If there is any doubt, fail closed only for product outcome proof. Everything else must surface as explicit debt with a next command and must not relaunch Integration Validator.

## RGF-233 - Canonical Terminal Closeout Record

**Problem:** Terminal truth is still spread across runtime authority, packet status, task-board status, validator verdict reports, signed-scope evidence, dossier sync state, repomem coverage, build-order projections, route anchors, and ACP session registry rows. Closeout repair has to read and sometimes rewrite too many artifacts to decide whether the WP is really done.

**Breakpoints this must cover:**

- Integration Validator PASS exists but terminal publication crashed before packet/task-board updates.
- Integration Validator FAIL exists but packet still says closeout pending.
- Packet says DONE but runtime authority says MERGE_PENDING.
- Dossier or repomem import fails after verdict and currently makes the terminal state look undecided.
- A rescue Orchestrator starts after a crash and cannot tell whether it should resume closeout or only sync projections.

**Required implementation:**

1. Add one schema-versioned terminal record as the sole source for final closeout authority. The record may live inside the runtime authority object if that is already the canonical storage path, or in a dedicated terminal artifact if the repo patterns favor a standalone record. The reader API must hide the storage detail.
2. Record at minimum:
   - `schema_version`
   - `wp_id`
   - `terminal_state`
   - `verdict`
   - `verdict_record_path`
   - `verdict_record_hash`
   - `reviewed_candidate_commit`
   - `signed_scope_patch_hash`
   - `product_main_baseline_commit`
   - `product_main_head_commit`
   - `merge_commit`
   - `containment_state`
   - `product_outcome_blockers`
   - `governance_debt`
   - `projection_state`
   - `created_at`
   - `updated_at`
   - `publisher_role`
   - `provenance`
3. Create a single reader library, for example `terminal-closeout-record-lib.mjs`, and make `phase-check CLOSEOUT`, `closeout-repair`, `orchestrator-next`, and validator closeout readers consume that reader.
4. Packet, task-board, dossier, build-order, route-health, and session status must be treated as projections after the terminal record exists.
5. Missing terminal record may block only if the record cannot be migrated from legacy proof or product proof is incomplete.

**Non-goals:**

- Do not add a second record that competes with runtime authority.
- Do not weaken signed-scope, candidate identity, or merge containment proof.
- Do not make dossier/repomem writes authoritative.

**Acceptance criteria:**

- A fixture with validator PASS plus stale packet/task-board projections reports `terminal_state=VERDICT_OF_RECORD` or stronger and projection debt, not unknown state.
- A fixture with missing signed-scope proof reports a product outcome blocker.
- A fixture with dossier import failure records governance debt and does not remove or downgrade the verdict.
- A fresh Orchestrator can recover terminal status from the terminal record without scanning every support artifact.

## RGF-234 - Closeout Proof / Projection Sync Split

**Problem:** `closeout-repair` still mixes product outcome proof with projection maintenance. A stale human-readable artifact can cause another repair pass even when the product verdict is already known.

**Breakpoints this must cover:**

- Stale task-board status after PASS.
- Stale packet header after terminal runtime state.
- Build-order projection lag after governance record changes.
- Route-health still pointing at old anchors after terminal verdict.
- Dossier write lane malformed but product proof intact.

**Required implementation:**

1. Split closeout evaluation into two explicit layers:
   - product proof check: validates verdict, candidate identity, signed scope, product-main compatibility, and containment.
   - projection settlement sync: regenerates or reports packet/task-board/dossier/build-order/route/session projection state.
2. `phase-check CLOSEOUT` should expose both layers in structured output.
3. A projection failure must return a debt key such as `PROJECTION_PACKET_STALE`, not a generic closeout failure.
4. Add an idempotent projection sync path that can be re-run without changing semantic truth.
5. Do not launch Integration Validator again for projection-only drift.

**Acceptance criteria:**

- Product proof can pass while projection sync reports debt.
- Projection sync can repair stale packet/task-board/build-order state from the terminal record.
- Running projection sync twice without semantic changes is file-stable.
- Status output names the exact projection keys and next command.

## RGF-235 - Product-Only Main Compatibility Resolver

**Problem:** Closeout can still falsely block if current-main compatibility uses a guessed sibling path, a stale worktree, or governance-only `.GOV` state instead of real product main. This is distinct from topology-resolved protected worktree discovery: closeout compatibility must prove product code only.

**Breakpoints this must cover:**

- `../handshake_main` guessed path is wrong.
- `wt-gov-kernel` branch is ahead in governance files but product main is compatible.
- Coder worktree carries a symlink or mirror for `.GOV` that should not be part of product diff.
- The backup branch on origin has the product candidate, while local main has governance-only movement.
- Current-main proof records the wrong commit because it reads from the wrong worktree.

**Required implementation:**

1. Add or reuse a topology-backed resolver that returns:
   - product main worktree path
   - product main branch
   - product main commit
   - protected gov-kernel worktree path
   - explicit exclusion rules for `.GOV/` and other repo-governance-only paths
2. Make signed-scope compatibility and closeout repair consume the resolver.
3. Record the product-main head used for compatibility in the terminal closeout record.
4. If product main cannot be resolved, fail with diagnostics listing discovered worktrees and expected branch names.
5. Never use `.GOV` drift as evidence of stale product main.

**Acceptance criteria:**

- Wrong sibling path fixtures fail with a diagnostic, not a silent stale-main blocker.
- Governance-only diffs do not affect product compatibility truth.
- Product-main head in the terminal record matches `git rev-parse` from the resolved product main worktree.
- Tests cover local topology and origin backup branch fallback behavior where practical.

## RGF-236 - Terminal Session Settlement and READY Residue Quarantine

**Problem:** ACP role rows can remain `READY` after completing work. When closeout status sees stale READY rows, it can keep the WP looking live, trigger repeated steering, or confuse rescue Orchestrator takeover decisions.

**Breakpoints this must cover:**

- CODER, WP_VALIDATOR, or ACTIVATION_MANAGER are READY with active=0 and queued=0 after verdict.
- Integration Validator PASS exists but old role sessions still appear in health output.
- Broker result ledger has completed commands but session registry has no terminal settlement marker.
- A rescue Orchestrator sees READY residue and thinks it should re-wake roles.

**Required implementation:**

1. During terminal publication, evaluate governed role sessions for the WP.
2. If a role has active=0, queued=0, and no unfinished required command, classify it as `TERMINAL_RESIDUE` or `SETTLED`.
3. Active runs or queued commands must still block or warn normally.
4. Health/status surfaces must distinguish active work from terminal residue.
5. Terminal residue must be included in governance debt if cleanup is still needed, but it must not reopen verdict state.

**Acceptance criteria:**

- Stale READY rows after PASS do not produce a re-wake recommendation.
- Active/queued sessions still remain visible as live blockers.
- Rescue guard defaults to read-only/status mode for terminal residue unless explicit force authority exists.
- Communication health reports terminal residue separately from live communication failure.

## RGF-237 - Closeout Debt Report and Non-Revalidation Policy

**Problem:** Governance debt after verdict is still too easy to misread as "go validate again" or "closeout is not real." The system needs one compact report that names debt and states whether revalidation is required.

**Breakpoints this must cover:**

- Dossier sync failure after PASS.
- Repomem coverage debt after terminal verdict.
- Packet/task-board projection drift after terminal record exists.
- Build-order lag after governance record mutation.
- Stale route anchors after terminal settlement.
- Token ledger/cost drift during long WP.

**Required implementation:**

1. Add a closeout debt report generator or structured section in existing closeout output.
2. Each debt item must include:
   - stable debt key
   - classification
   - owner surface
   - severity
   - blocks_product_outcome boolean
   - revalidation_required boolean
   - next mechanical command
   - reason
3. Default `revalidation_required=NO` for governance settlement debt.
4. `orchestrator-next`, `orchestrator-health`, and validator closeout text must prefer this compact report over broad artifact rereads.
5. Protocol text must say debt reports do not authorize Orchestrator to write verdicts or approvals.

**Acceptance criteria:**

- Report includes `revalidation_required=NO` for dossier/repomem/projection debt.
- Report includes `revalidation_required=YES` only for product proof blockers.
- Operator-facing status can be answered from the report without rehydrating the whole packet/dossier.
- Regression tests cover at least one debt item from every support-surface class.

## RGF-238 - Closeout Repair Loop Breaker and Escalation Packet

**Problem:** When repair does not converge, Orchestrator can spend many cycles reading artifacts, rewriting projections, retrying checks, and capturing procedural failures. Heavy host load makes this worse because timeouts look like partial failures.

**Breakpoints this must cover:**

- `closeout-repair` fixes one artifact but another projection remains stale.
- A malformed dossier section repeatedly fails import.
- PowerShell quoting breaks a helper call while deriving proof.
- A timeout occurs under heavy load before subcheck attribution is clear.
- One repair pass changes the blocker set but does not reduce it.

**Required implementation:**

1. Give closeout repair a hard budget:
   - one automated repair pass
   - one manual remediation pass when diagnostics identify a safe mechanical fix
   - then stop and escalate
2. Record every repair attempt with:
   - timestamp
   - command
   - pre-repair product blocker keys
   - pre-repair governance debt keys
   - changes attempted
   - post-repair product blocker keys
   - post-repair governance debt keys
   - convergence result
3. If the same product blocker or debt key survives unchanged, do not retry blindly.
4. Emit an escalation packet that contains only canonical terminal record state, product blockers, debt report, attempted repairs, and next safe command.
5. Ensure fail-capture fires for tool failures and command construction failures without causing another closeout repair loop.

**Acceptance criteria:**

- A non-converging fixture stops after the configured budget.
- Escalation output is small enough for a fresh model to act without reading the dossier.
- Re-running after no changes reports the existing escalation instead of starting over.
- Heavy-host timeout fixtures produce attributed `UNKNOWN_DUE_TO_TIMEOUT` or equivalent without rewriting product verdict state.

## RGF-239 - Terminal Authority Migration and Legacy Artifact Compatibility

**Problem:** Old WPs will not have the canonical terminal record. Without one migration path, each script may invent its own fallback and recreate split terminal truth.

**Breakpoints this must cover:**

- Legacy PASS has validator report and packet DONE but no terminal record.
- Legacy FAIL has report but no runtime authority entry.
- Legacy packet has terminal text but validator report path is missing.
- Runtime authority says terminal but task-board history is stale.
- Migration cannot prove signed-scope identity.

**Required implementation:**

1. Add a single migration helper used by all terminal record readers.
2. Migration should derive a record only when enough legacy evidence exists.
3. Migrated records must include `provenance.mode=MIGRATED_LEGACY` and source artifact hashes.
4. If migration cannot prove product outcome, return structured product blockers.
5. Do not let individual readers parse legacy artifacts independently after this helper exists.

**Acceptance criteria:**

- Legacy fixtures migrate to a canonical record or produce explicit product blockers.
- A missing validator report cannot be papered over by packet text.
- Migrated records are stable across repeated reads.
- Reader tests prove all closeout consumers call the same migration path.

## RGF-240 - Monotonic Terminal State Machine and Atomic Publication

**Problem:** Multiple agents or commands can touch terminal surfaces: Orchestrator, visible rescue Orchestrator, Integration Validator, phase-check sync, projection sync, and closeout-repair. Without monotonic state and atomic writes, a stale writer can overwrite stronger authority or race with another terminal publisher.

**Breakpoints this must cover:**

- Rescue Orchestrator starts while original closeout writer is still finishing.
- Projection sync runs after terminal publication and writes older packet/runtime state.
- Integration Validator writes verdict while `closeout-repair` is rechecking.
- Two commands attempt terminal publication under heavy host delays.
- A stale process writes `MERGE_PENDING` after `TERMINAL_SETTLED`.

**Required implementation:**

1. Define a monotonic state machine. Minimum states:
   - `NO_VERDICT`
   - `VERDICT_OF_RECORD`
   - `MERGE_PENDING`
   - `MERGED`
   - `SETTLEMENT_DEBT`
   - `TERMINAL_SETTLED`
2. Add transition rules and reject downgrades unless an explicit invalidation record exists.
3. Use atomic write plus compare-and-swap or file-lock semantics for terminal publication.
4. Projection sync may update projection timestamps/debt but must not downgrade product outcome fields.
5. Stale-writer rejection must produce a clear diagnostic and must be safe to retry.

**Acceptance criteria:**

- Concurrent writer fixtures leave the strongest valid terminal state intact.
- Stale projection sync cannot change product verdict, reviewed commit, or containment proof.
- Invalid transitions produce diagnostics with current state, attempted state, writer, and next safe command.
- Tests cover both pass and fail terminal paths.

## RGF-241 - Closeout Breakpoint Scenario Harness

**Problem:** The governance system keeps learning closeout breakpoints from live WPs. Those scenarios need to become executable fixtures so regressions are caught before another closeout run.

**Required implementation:**

1. Build a focused test harness for closeout scenarios. It may use fixture directories and small JSON/Markdown artifacts rather than full live WPs.
2. Include at least these scenarios:
   - PASS with dossier append/import failure.
   - PASS with stale task-board projection.
   - PASS with stale packet projection.
   - PASS with stale READY sessions but no active runs.
   - PASS with stale route anchors.
   - PASS with build-order projection lag.
   - PASS with repomem coverage debt.
   - FAIL with same-WP remediation required.
   - missing Integration Validator verdict.
   - signed-scope mismatch.
   - candidate commit mismatch.
   - product-main compatibility proof stale.
   - wrong main worktree path.
   - legacy closeout without terminal record.
   - concurrent stale writer attempts downgrade.
   - heavy-host timeout during a subcheck.
3. Every scenario must assert:
   - product blocker keys
   - governance debt keys
   - whether revalidation is required
   - whether projection sync is allowed
   - whether repair may continue or must escalate
4. The harness should run through `node --test` and be included in `gov-check` only if it remains fast and deterministic.

**Acceptance criteria:**

- The harness fails if any support-surface debt blocks product outcome.
- The harness fails if any product proof blocker is demoted to debt.
- The harness fails if a projection-only scenario recommends Integration Validator relaunch.
- The harness output is concise enough to identify the exact failed breakpoint.

## Suggested Implementation Order

1. `RGF-233`: introduce the terminal record reader/writer contract.
2. `RGF-240`: make terminal writes monotonic and atomic before more writers consume the record.
3. `RGF-241`: lock the narrowed breakpoint matrix into fixtures and tests.

The earlier standalone order for `RGF-234` through `RGF-239` is retained in the sections above as historical design context only. Those rows are no longer standalone implementation targets after the 2026-04-30 triage.

## Fresh-Model Starting Points

- Read `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs` first to understand current blocker/debt classification.
- Read `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs` second to see what final-lane closeout consumes.
- Read `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs` for current verdict/settlement preservation behavior.
- Read `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs` for current projection drift handling.
- Read `.GOV/roles_shared/scripts/lib/git-topology-lib.mjs` and `.GOV/roles_shared/scripts/resolve-protected-worktree.mjs` before touching main/worktree resolution.
- Do not start by editing protocols. Implement the mechanical reader/writer/check behavior first, then update protocols and command docs to match the new mechanics.
- Do not route deterministic closeout repair through ACP. Use direct `just`/node calls for phase-check, closeout repair, projection sync, terminal record reads, and test harnesses.
