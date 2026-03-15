# Audit: Orchestrator-Managed Live Parallel Smoke Test for WP-1

## METADATA
- AUDIT_ID: AUDIT-20260314-ORCH-MANAGED-LIVE-SMOKETEST-PARALLEL-WP1
- DATE_UTC: 2026-03-14
- AUDITOR: Codex (governance review after live execution)
- SCOPE: Live orchestrator-managed parallel smoke test for `WP-1-Structured-Collaboration-Schema-Registry-v1` and `WP-1-Loom-Storage-Portability-v1`
- WPS_IN_SCOPE:
  - `WP-1-Structured-Collaboration-Schema-Registry-v1`
  - `WP-1-Loom-Storage-Portability-v1`
- RESULT: FORMAL WORKFLOW RECOVERY PASS, BUT POST-HOC CODE/SPEC INSPECTION FOUND RESIDUAL STRUCTURED-COLLABORATION CORRECTNESS GAPS ON INTEGRATED `main`
- EVIDENCE_SOURCES:
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Structured-Collaboration-Schema-Registry-v1/97465c90-5b53-426e-b2aa-67cd09541eec.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Loom-Storage-Portability-v1/0ab53b55-956f-498f-9a6d-0e99f4da9117.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Loom-Storage-Portability-v1/0c721c2b-5cd4-4691-a675-ccff91c5cacd.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v1/1d80242a-f1de-4bc2-9ccd-0a48dcc11432.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v1/ee454935-070a-4f8b-a51a-073b50b5e5d3.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Loom-Storage-Portability-v1/ca6bc9a2-844d-482b-9ead-86caea93991d.jsonl`
  - `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Loom-Storage-Portability-v1/bec1356f-3693-4dd5-8317-e43cf15d8e8b.jsonl`

---

## 1. EXECUTIVE SUMMARY

This smoke test did prove that the orchestrator-managed workflow can run two governed Coder sessions in parallel on separate worktrees with disjoint product-code targets.

It did not prove that the workflow is yet operator-trustworthy for parallel work without heavy manual governance intervention.

What worked:

- two governed Coder sessions were started and steered successfully
- both WPs stayed isolated from each other at the product-code file level
- both WPs produced meaningful in-scope implementation checkpoints
- a governed WP Validator was launched and used to audit the Loom scope leak and the stashed out-of-scope work

What failed:

- the Orchestrator did not launch WP validators early enough
- the Loom coder was allowed to drift into out-of-scope files before correction
- validator worktrees booted from stale `main` state and did not see active packet/refinement truth until manually synced
- validator startup and `gov-check` were blocked by pre-existing governance drift
- session-control wrappers still behave like blocking full-turn calls rather than lightweight steering acknowledgements
- packet/tooling assumptions around Cargo workspace roots were wrong for this repo layout

Overall judgment:

- architecture direction: still correct
- live parallel workflow viability: proven in principle
- workflow implementation quality: not yet operator-safe without stronger automatic sync, validator timing, and packet/test-law enforcement
- later post-hoc code/spec inspection revised the product conclusion:
  - Loom portability still looks materially correct for the inspected storage-portability slice
  - structured-collaboration/schema-registry surfaces on integrated `main` still contain concrete correctness gaps and should not be described as fully master-spec aligned

---

## 2. WHAT WAS PROVEN

### 2.1 Proven Capabilities

- The Orchestrator can create two active feature branches and two governed Coder sessions for separate WPs without sharing a worktree.
- The Orchestrator can drive both WPs through refinement, signature, prepare, packet creation, skeleton checkpoint, skeleton approval, remote backup push, and live implementation start.
- The governance surface is strong enough to preserve deterministic session outputs, runtime ledgers, and audit evidence while multiple role sessions operate on the same repo.
- The Loom packet can be returned to packet scope after a scope leak without losing the removed work, using a safety stash plus validator audit.

### 2.2 What Was Not Proven

- End-to-end validator closeout for both WPs
- automatic validator readiness after coder checkpoint
- safe validator bootstrap from a fresh validator worktree without manual state repair
- fire-and-forget operator confidence in the session-control layer

---

## 3. OUTCOME BY WP

### 3.1 WP-1-Structured-Collaboration-Schema-Registry-v1

State reached:

- official packet created
- skeleton checkpoint committed
- skeleton approval committed
- governed Coder session completed an in-scope implementation pass
- active product diff remains confined to packet scope

Current implementation scope in the coder worktree:

- `src/backend/handshake_core/src/api/role_mailbox.rs`
- `src/backend/handshake_core/src/locus/task_board.rs`
- `src/backend/handshake_core/src/locus/types.rs`
- `src/backend/handshake_core/src/role_mailbox.rs`
- `src/backend/handshake_core/src/runtime_governance.rs`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- `src/backend/handshake_core/tests/role_mailbox_tests.rs`

Validation outcome:

- Coder verification reached parse/format and targeted `--no-run` steps, but full Rust build validation was blocked by the Windows `libduckdb-sys` native build path before target test compilation.
- WP Validator bootstrap did not reach substantive review because governance/spec checks failed first.

### 3.2 WP-1-Loom-Storage-Portability-v1

State reached:

- official packet created
- skeleton checkpoint committed
- skeleton approval committed
- governed Coder session completed a meaningful in-scope Loom portability pass
- out-of-scope drift was corrected and isolated into a safety stash
- WP Validator reviewed both the active worktree and the removed out-of-scope stash

Current implementation scope in the coder worktree after cleanup:

- `src/backend/handshake_core/src/api/loom.rs`
- `src/backend/handshake_core/src/loom_fs.rs`
- `src/backend/handshake_core/src/storage/loom.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `src/backend/handshake_core/tests/storage_conformance.rs`

Validation outcome:

- `cargo test -p handshake_core loom` passed
- `cargo test -p handshake_core --test storage_conformance` passed
- validator concluded the removed out-of-scope work should remain excluded

---

## 4. TECHNICAL FAILURES FOUND AND PATCHED

| # | Severity | Failure | Effect During Run | Resolution / State |
|---|----------|---------|-------------------|--------------------|
| 1 | HIGH | `create-task-packet` transitively imported `EXECUTION_OWNER_VALUES` from `.GOV/scripts/wp-communications-lib.mjs`, but that module did not re-export it | packet creation failed during live startup | patched by exporting `EXECUTION_OWNER_VALUES` from `.GOV/scripts/wp-communications-lib.mjs` |
| 2 | HIGH | packet section replacement in `.GOV/scripts/create-task-packet.mjs` matched `##` headings inside HTML comments | generated packets lost lower metadata and were malformed | patched by anchoring section replacement to real markdown headings; malformed packets were then manually repaired |
| 3 | HIGH | two `record-signature` operations were executed in parallel against shared governance state | `ORCHESTRATOR_GATES.json` lost one signature row even though signature audit and refinement were updated | manually repaired the missing gate entry; workflow remains race-prone |
| 4 | MEDIUM | `SEND_PROMPT` is effectively a full-turn blocking wrapper, not a quick steer/ack call | orchestrator shell calls timed out while governed turns were still legitimately running | operationally worked around by reading registry and output logs; not yet structurally fixed |
| 5 | MEDIUM | first Schema WP Validator startup was rejected with `Governed session ... is not registered` | validator lane appeared broken on first use | retry plus later successful startup; root cause remains unresolved |
| 6 | MEDIUM | validator startup and validator test-plan commands assumed usable local packet/Cargo state from fresh validator worktrees | validators initially operated against stale packet mirrors and invalid repo-root cargo commands | worked around by manually syncing active packet state into validator worktrees and translating to manifest-path cargo commands; underlying workflow defect remains |

### 4.1 Remediations Already Applied During This Smoke Test

The following remediations were not merely proposed. They were actually executed during the run.

#### 4.1.1 Governance/Tooling Patches I Applied

1. Patched `.GOV/scripts/wp-communications-lib.mjs`
   - Added the missing `EXECUTION_OWNER_VALUES` export so task-packet creation could proceed.

2. Patched `.GOV/scripts/create-task-packet.mjs`
   - Corrected the section-replacement logic so packet generation no longer matched comment text and truncated lower metadata blocks.

3. Repaired `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
   - Restored the missing Schema signature gate row after the parallel `record-signature` race.

4. Repaired the two generated official packets by hand after the packet-generation bug
   - restored missing lower metadata
   - restored `USER_SIGNATURE`
   - restored validator/runtime communication fields
   - replaced malformed placeholder sections with packet-safe content

#### 4.1.2 Operational Recoveries I Performed

1. Created and pushed remote backup branches for both active WP feature branches at the skeleton approval boundary.

2. Synced active governance state into the coder worktrees so `pre-work`, packet truth, and WP communications matched the orchestrator worktree.

3. Created validator worktrees and then manually synced active packet/refinement/task-board/traceability/WP communications state into them after discovering that fresh validator worktrees only saw stale `main` truth.

4. Forced Loom back to packet scope
   - identified the out-of-scope files
   - ordered the Loom coder to remove them from the active worktree
   - preserved the removed changes in a safety stash instead of deleting them

5. Used the Loom WP Validator to audit the removed stash against `Handshake_Master_Spec_v02.178.md`
   - obtained an explicit `KEEP_EXCLUDED` disposition
   - prevented reintroduction of formatter/noise churn into the active WP

6. Established that the active Loom worktree is now packet-clean
   - remaining product diff is confined to the packet allowlist
   - narrow Loom verification was re-run after cleanup and remained green

#### 4.1.3 Remediations Still Outstanding

The following issues were discovered but not structurally fixed in this smoke test:

- validator worktrees still bootstrap from stale `main` truth unless manually synced
- `SEND_PROMPT` still behaves like a blocking full-turn wrapper
- Schema validator preflight is still blocked by governance/spec drift and missing validator-ready posture
- task packet / validator command law still assumes an invalid repo-root Cargo layout

---

## 5. WORKFLOW FAILURES BY ROLE

### 5.1 Orchestrator Failures

These were real workflow failures by the Orchestrator role during the smoke test:

1. WP validator sessions were not launched immediately after skeleton approval.
   - This was the most important orchestration miss.
   - Parallel coders were allowed to move into implementation without parallel validator eyes on the work.

2. Loom scope drift was not interrupted early.
   - The first Loom implementation turn had already touched 20 files before intervention.
   - The Orchestrator waited until after the turn completed instead of earlier validator-backed correction.

3. Validator worktree truth was not prepared before validator startup.
   - Fresh validator worktrees were created from `main`, so they saw stale packet/task-board/traceability state.
   - The Orchestrator had to manually copy the active packet/refinement/task-board/traceability/WP communications state into validator worktrees after startup.

4. The live smoke test review itself was not written during the run.
   - Evidence collection happened.
   - Formal review and architecture follow-up were deferred instead of being kept current during execution.

### 5.2 Coder Failures

#### Coder A

- No packet-scope violation was observed.
- Main issue was environment-constrained validation, not scope or governance drift.

#### Coder B

- Ran `cargo fmt -p handshake_core` during a Loom-scoped WP.
- This caused out-of-scope formatting churn in 13 non-Loom files.
- The work was later corrected, but this was still a real packet-scope failure during the live run.

### 5.3 WP Validator Failures

No substantive technical mis-review by the validators was observed. The validator issues were workflow and bootstrap defects:

- fresh validator worktrees started from stale packet truth
- validator startup could be blocked by unrelated governance/spec drift before reaching the WP itself

---

## 6. TOOL FAILURES AND SHARP EDGES

### 6.1 Session-Control Layer

Observed defects:

- `start-wp-validator-session` for Schema first produced `Governed session WP_VALIDATOR:WP-1-Structured-Collaboration-Schema-Registry-v1 is not registered`
- `SEND_PROMPT` calls can outlive shell-level timeouts while still being healthy governed runs
- the session registry is informative, but not sufficient by itself; real status required direct inspection of session output files

Operational consequence:

- the Orchestrator had to treat session-control calls as long-running workflow operations rather than simple steering acks
- the output log became the real progress surface when the wrapper timed out

### 6.2 Packet/Test-Plan Tooling

Observed defect:

- packet test commands for Loom and validator review assumed the repo root contained a usable `Cargo.toml`
- this repo layout requires `--manifest-path src/backend/handshake_core/Cargo.toml`

Operational consequence:

- validator review had to correct the commands on the fly
- the packet/test law was technically wrong for the repo layout

### 6.3 Validator Bootstrap Tooling

Observed defect:

- fresh validator worktrees inherit `main`, not the current orchestrator-managed packet/refinement/task-board state

Operational consequence:

- Loom validator initially concluded the WP was stub-only
- manual sync into validator worktrees was required before meaningful review could start

This is a systemic workflow bug, not operator error.

---

## 7. OUT-OF-SCOPE WORK AND WHY IT WAS REJECTED

### 7.1 The Loom Scope Leak

The first Loom coder implementation turn dirtied these non-Loom files:

- `src/backend/handshake_core/src/api/jobs.rs`
- `src/backend/handshake_core/src/flight_recorder/mod.rs`
- `src/backend/handshake_core/src/llm/guard.rs`
- `src/backend/handshake_core/src/llm/mod.rs`
- `src/backend/handshake_core/src/llm/registry.rs`
- `src/backend/handshake_core/src/mcp/client.rs`
- `src/backend/handshake_core/src/mcp/gate.rs`
- `src/backend/handshake_core/src/mcp/transport/reconnect.rs`
- `src/backend/handshake_core/src/mex/conformance.rs`
- `src/backend/handshake_core/src/mex/runtime.rs`
- `src/backend/handshake_core/tests/mcp_e2e_tests.rs`
- `src/backend/handshake_core/tests/mcp_gate_tests.rs`
- `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`

Immediate cause:

- broad formatter invocation in a narrow-scope WP

Why it was out of scope:

- the Loom packet explicitly scopes storage/API/migration/filesystem/conformance files only
- these files belong to job parsing, Flight Recorder, LLM, MCP, MEX, and unrelated test families
- they do not implement Loom storage portability requirements

### 7.2 Validator Disposition

The WP Validator reviewed the cleaned stash against `Handshake_Master_Spec_v02.178.md` and concluded:

- `ALLOW_IN_CURRENT_WP`: no
- `SALVAGE_AS_SEPARATE_WP`: no
- `KEEP_EXCLUDED`: yes

Why:

- the removed hunks were formatter/noise churn, not material spec implementation
- they touched other spec families, not Loom
- reapplying them would widen scope without value

### 7.3 Safety Handling

The removed out-of-scope work was not destroyed.

It was preserved as:

- `stash@{0}: SAFETY: before Loom packet-scope cleanup`

That was the correct recovery mechanism:

- no product-code loss
- active worktree restored to packet scope
- validator could still audit the removed changes

---

## 8. GOVERNANCE NON-COMPLIANCE AND DRIFT

### 8.1 Traceability Registry Drift

The current orchestrator worktree still shows stale traceability rows for both active WPs:

- `WP-1-Loom-Storage-Portability` is still recorded as `Stub Backlog (Not Activated)` in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `WP-1-Structured-Collaboration-Schema-Registry` is still recorded as `Stub Backlog (Not Activated)` in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`

At the same time, the task board shows both as active:

- `WP-1-Structured-Collaboration-Schema-Registry-v1` -> `[READY_FOR_DEV]`
- `WP-1-Loom-Storage-Portability-v1` -> `[READY_FOR_DEV]`

This is real governance inconsistency.

### 8.2 Schema Validator Readiness Drift

Schema validator bootstrap found that the WP was not validator-ready in packet/runtime state:

- packet still says `Status: Ready for Dev`
- WP runtime status still says `next_expected_actor: ORCHESTRATOR`
- `validator_trigger: NONE`

This meant:

- coder had completed a meaningful checkpoint
- validator existed
- but governance state had not advanced to a validator-expected posture

### 8.3 `gov-check` Blocked by Stale Spec Gap Reference

Both coder and validator flows were blocked by the same governance/spec defect:

- `FEAT-LOCUS-WORK-TRACKING` still lists `WP-1-Structured-Collaboration-Artifact-Family-v1` as a `gap_stub_id`
- task board now marks that packet `[VALIDATED]`
- validator gate logic only accepts `gap_stub_id` values that are still `[STUB]` or actively open

This created a governance self-contradiction:

- the feature is no longer an open gap
- but the spec appendix still claims it is
- so `gov-check` fails before validator work can proceed

This was pre-existing, but it materially disrupted the live run and therefore counts as a governance failure discovered in the smoke test.

---

## 9. SYSTEMIC RISKS EXPOSED

| Risk | Severity | Why It Matters | Smoke Test Verdict |
|------|----------|----------------|--------------------|
| Late validator engagement | HIGH | parallel coder work can drift before any review guard engages | real risk observed |
| Stale worktree truth for validators | HIGH | validator may reason over stub/main state instead of active packet truth | real risk observed |
| Packet-scope leakage through broad formatting | HIGH | harmless-looking tooling can create non-local review churn and governance violations | real risk observed |
| Session-control wrapper opacity | MEDIUM | healthy governed runs can look hung or failed at the shell layer | real risk observed |
| Traceability/task-board drift | HIGH | active packet status can disagree across governance surfaces | real risk observed |
| Pre-existing governance defects blocking live ops | HIGH | unrelated spec drift can halt validator or `gov-check` before WP review begins | real risk observed |
| Incorrect packet test-law for repo layout | MEDIUM | validator/coder evidence steps become misleading or invalid | real risk observed |

---

## 10. SELF-ASSESSMENT: ORCHESTRATOR PROTOCOL, REPO GOVERNANCE, AND ROLE-PROTOCOL USE

### 10.1 Orchestrator Protocol Compliance

Overall self-assessment:

- protocol adherence on role boundary and authority: STRONG
- protocol adherence on timing/operational sequencing: MIXED
- protocol adherence on validator timing and live review discipline: BELOW STANDARD

What I followed correctly:

1. I stayed in the Orchestrator role.
   - I did not switch into Coder or Validator authority.
   - I did not directly implement product-code changes in the WP worktrees.

2. I respected the `ORCHESTRATOR_MANAGED` execution model.
   - I created/refined/signed/prepared packets.
   - I launched governed Coder and WP Validator sessions through the governed session commands rather than bypassing them.
   - I used the packet-declared execution owners (`CODER_A`, `CODER_B`) and did not improvise alternate ownership.

3. I stayed non-agentic for repo-role work.
   - I did not use helper agents to impersonate Orchestrator or Validator authority.
   - I did not delegate product-code decisions to sub-agents under my own role.

4. I kept the Orchestrator on governance/workflow surfaces.
   - my direct edits were to `.GOV/**` workflow/governance artifacts and scripts
   - product-code edits were produced by governed coder sessions in their assigned WP worktrees

Where I did not meet the expected standard:

1. I launched validators too late.
   - This is the clearest miss against the actual intent of orchestrator-managed workflow.
   - In a live parallel smoke test, validator coverage should have started as soon as both WPs cleared skeleton approval.

2. I allowed too much time before intervening on Loom scope drift.
   - I waited for the governed coder turn to settle before correction.
   - That was operationally understandable, but it was not the strongest orchestrator posture for a live smoke test.

3. I did not keep the smoke-test review current during execution.
   - Evidence collection happened live.
   - Formal review writing lagged behind the run instead of tracking it as part of the orchestration loop.

### 10.2 Repo Governance Compliance

Overall self-assessment:

- destructive-op safety: STRONG
- branch/worktree concurrency law: STRONG
- governance sync/state accuracy: MIXED
- status/traceability closure discipline: BELOW STANDARD

What I followed correctly:

1. One WP per branch/worktree was enforced.
   - separate feature branches
   - separate coder worktrees
   - no shared implementation worktree between active WPs

2. I avoided destructive cleanup behavior.
   - no `git reset --hard`
   - no `git clean -fd`
   - no direct filesystem deletion of worktrees

3. I preserved recovery state correctly.
   - created checkpoint commits for packet/refinement and skeleton boundaries
   - pushed remote backup branches at the skeleton approval boundary
   - required a safety stash for the Loom out-of-scope cleanup instead of permitting silent loss

4. I preserved the governance/product boundary.
   - Orchestrator edits stayed on governance/workflow surfaces
   - coder product changes stayed in WP worktrees

Where governance handling was weak or incomplete:

1. Traceability and task-board sync remained inconsistent.
   - active packets existed
   - task board showed active state
   - traceability registry still showed both WPs as stub backlog
   - this is governance non-compliance in active state synchronization

2. Validator-ready posture was not advanced cleanly for Schema.
   - packet/runtime status did not move to a validator-ready state even though meaningful coder work existed
   - that left the validator in a technically correct but operationally blocked posture

3. I relied on manual governance copying into validator worktrees.
   - this was permissible as governance-only repair
   - but it is evidence that the workflow law is not self-enforcing enough yet

### 10.3 How I Used Other Role Protocols Through CLI/ACP

This smoke test used governed CLI/ACP sessions for:

- `CODER` on both WPs
- `WP_VALIDATOR` on both WPs

I did not start or use an `INTEGRATION_VALIDATOR` session in this run.

How I used the Coder role protocol:

1. Started coder sessions only through Orchestrator-governed commands:
   - `just start-coder-session WP-{ID}`
   - `just steer-coder-session WP-{ID} "<prompt>"`

2. Kept coders inside the packet law:
   - required `just pre-work`
   - required docs-only skeleton checkpoint
   - used `just skeleton-approved`
   - then resumed implementation only after approval

3. Passed role-specific rails in the steering prompts:
   - packet-scope file isolation
   - no overlap with the sibling WP
   - stop at meaningful checkpoints
   - report changed files, commands, and residual risk

4. Honored the packet’s `SUB_AGENT_DELEGATION: DISALLOWED`
   - I did not instruct or authorize coder sub-agents under these WPs

How I used the WP Validator role protocol:

1. Started validator sessions only through Orchestrator-governed commands:
   - `just start-wp-validator-session WP-{ID}`
   - `just steer-wp-validator-session WP-{ID} "<prompt>"`

2. Used validators in an advisory/review posture only.
   - validators were asked to inspect scope compliance, checkpoint readiness, and spec alignment
   - validators were not used to edit product code

3. Used validators to produce findings-first output.
   - this matched the intended validator/reviewer posture and was especially important for the Loom stash review

4. Correctly kept validator authority subordinate to workflow law.
   - validators surfaced blockers and findings
   - they did not unilaterally mutate packet/workflow authority

Where my use of the other role protocols was weak:

1. I did not start validators early enough.
   - this is both an orchestration timing failure and a weak application of the validator lane

2. Validator bootstrap was not prepared before session start.
   - role startup happened against stale worktree truth
   - that degraded the validator protocol into bootstrap debugging instead of immediate review

### 10.4 ACP / CLI Session-Protocol Assessment

Overall self-assessment:

- lawful use of governed commands: STRONG
- session observability and practical handling: MIXED

What I did correctly:

- used only governed session commands for role starts and prompt steering
- used stable ACP-backed thread identities instead of ad hoc terminal injection as the meaning of session state
- inspected request/result/output ledgers when shell-level behavior was ambiguous

What this run exposed:

- the wrapper semantics are still misleading for long turns
- operator/orchestrator workflow still depends on reading session output logs directly
- validator startup/bootstrap semantics are not robust enough when packet truth differs from `main`

### 10.5 Final Self-Judgment

My role-boundary compliance was good.

My workflow-timing compliance was not good enough.

I followed the non-agentic Orchestrator rule, respected worktree/branch safety, preserved recovery boundaries, and used governed Coder/WP Validator role sessions through the intended CLI/ACP control lane.

I did not meet the bar on:

- early validator engagement
- proactive intervention on scope drift
- keeping governance status surfaces synchronized quickly enough
- keeping the smoke-test review current during execution

If this were graded as an orchestrator performance review for a live smoke test:

- role discipline: PASS
- governance safety: PASS
- orchestration timing: FAIL
- validator utilization: FAIL
- recovery under failure: PASS
- end-to-end operational sharpness: MIXED

---

## 11. RECOMMENDED CORRECTIVE ACTIONS

### Immediate

1. Make validator launch mandatory immediately after skeleton approval for every orchestrator-managed WP.
2. Treat out-of-scope file creation during a coder turn as an automatic hard correction event, not a wait-and-see condition.
3. Require validator worktree sync of active packet/refinement/task-board/traceability state as part of validator startup until the bootstrap model is fixed.
4. Fix the stale traceability rows for:
   - `WP-1-Loom-Storage-Portability`
   - `WP-1-Structured-Collaboration-Schema-Registry`
5. Correct the stale spec appendix `gap_stub_ids` reference that keeps breaking `gov-check`.

### Near-Term

1. Split session-control semantics into:
   - fast command acknowledgement
   - long-running turn monitoring
   so orchestrator shell calls stop looking dead when a governed turn is simply still working
2. Fix packet/test-plan generation so Rust commands are valid from the assigned worktree/root layout.
3. Make validator readiness explicit in packet/runtime state transitions instead of leaving the validator to infer handoff posture.

### Strategic

1. Build an automatic active-packet mirror sync for validator and integration-validator worktrees, or stop bootstrapping them from raw `main`.
2. Add packet-scope enforcement rails for formatting commands and broad repo tools.
3. Update `.GOV/docs/vscode-session-bridge/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md` to record:
   - blocking `SEND_PROMPT` semantics
   - stale validator worktree bootstrap
   - packet/root cargo command mismatch
   - late-validator orchestration risk

---

## 12. FINAL JUDGMENT

This was a useful smoke test, not a clean one.

The orchestrator-managed workflow did succeed at the most important architectural proof point:

- two governed coders can operate in parallel on separate worktrees without direct product overlap

But the workflow is still materially brittle in exactly the places that matter for operator trust:

- validator timing
- stale governance mirrors
- scope enforcement
- tool-wrapper truthfulness
- governance drift blocking live work

The correct high-level reading is:

- parallel orchestrator-managed development is viable
- the current governance/tooling stack is still too manual and too failure-prone to be considered production-ready

The strongest concrete win from this run is the Loom recovery path:

- scope leak happened
- it was preserved safely
- the validator reviewed the removed work against the master spec
- the out-of-scope material was explicitly rejected
- the active worktree was returned to packet scope without losing evidence

That is the behavior we want.

The strongest concrete failure from this run is that the Orchestrator did not bring validator pressure in early enough and had to repair validator truth surfaces by hand.

That should not remain acceptable.

---

## 13. RECOVERY ADDENDUM: WHY THE WORKFLOW STOPPED AND WHY THAT WAS NOT ACCEPTABLE

This addendum records the most important operational failure from the live run:

- both active WPs remained in `Ready for Dev`
- neither WP had reached a formal validator `PASS`
- the Orchestrator still allowed the live effort to drift into reporting, status narration, and postmortem writing

That was a workflow failure by the Orchestrator, not a legitimate completion boundary.

### 13.1 What Actually Happened

After the main coder turns settled:

- Schema had meaningful in-scope implementation work but no substantive validator closeout
- Loom had meaningful in-scope implementation work and a repaired packet-scope cleanup, but no formal validator closeout
- governance surfaces still showed both packets as `Ready for Dev`
- packet verdict blocks still showed `PENDING`

At that point, the run should have remained in active orchestration mode.

Instead, the Orchestrator spent additional time on:

- status relays
- interim explanations
- audit drafting
- post-hoc reasoning about why validation had not completed

Those actions were not wrong in themselves, but they were performed at the wrong time. They displaced the more important work:

- repairing validator-ready governance state
- re-steering both WP validators into live review
- pushing both coders through remaining implementation and evidence gaps
- driving both WPs to explicit validator verdicts

### 13.2 Why This Was a Protocol Failure

The Orchestrator protocol and the live smoke-test intent required active workflow completion, not passive bookkeeping.

The specific failure was a priority inversion:

- blocker resolution should have remained the top priority until the WPs reached validator verdicts
- instead, reporting and commentary were allowed to consume the critical path while the WPs were still operationally incomplete

This was especially unacceptable because the unresolved issues were known, concrete, and actionable:

- stale validator worktree truth
- packet/runtime state not advanced into validator-ready posture
- governance drift preventing clean `gov-check`
- missing end-to-end validator handoff discipline

None of those justified stopping. They required intervention.

### 13.3 Time Waste and Operational Cost

The practical cost of this mistake was several hours of operator-visible stall while the repo remained in a half-finished state:

- active coder output existed
- validator capacity existed
- the Orchestrator had enough evidence to continue
- yet the workflow did not advance to closure

That created the worst possible middle state:

- enough work had happened to accumulate risk
- not enough workflow law had been applied to produce trustworthy completion

From an operator perspective, that is correctly read as wasted time.

### 13.4 Corrective Actions Being Executed Now

These actions are no longer just recommendations; they are the active recovery path for this same live run:

1. Extend this audit with the explicit stop/failure analysis.
2. Reconcile governance/state blockers that are preventing validator-ready operation.
3. Re-engage both coder sessions with sharper completion criteria.
4. Re-engage both WP validators with explicit instructions to produce findings and verdict-driving guidance instead of bootstrap commentary.
5. Keep the workflow active until each WP reaches:
   - code that survives scrutiny
   - master-spec alignment without unresolved gaps
   - passing governance checks
   - an explicit validator outcome

### 13.5 Final Judgment on This Specific Failure

The failure was not that the repo had blockers.

The failure was that the Orchestrator tolerated those blockers as a reason to pause operational progress instead of treating them as the work.

That is the core orchestration miss being corrected after this addendum.

---

## 14. FINAL RECOVERY OUTCOME (SUPERSEDES THE EARLIER PROVISIONAL NON-CLOSEOUT STATE)

This section records the actual end state reached after the workflow was resumed and driven to technical closeout.

### 14.1 Final Branch Truth

`WP-1-Structured-Collaboration-Schema-Registry-v1`

- final pushed branch head: `127af7046f6eac3f14608b58f71b50db24a34fc3`
- key validator PASS commit in the recovery chain: `ab224c1a1701736c58e9299a2ce5aa41138f6a4b`
- later governance closeout commits synchronized packet/runtimes to the true Gate 3 halt state
- remote backup branch updated: `origin/feat/WP-1-Structured-Collaboration-Schema-Registry-v1`

`WP-1-Loom-Storage-Portability-v1`

- final pushed branch head: `35355abc33eb187a192566776b11899ad4d28052`
- validator PASS commit: `892931a96652336383d4c2552d7637f59cb94e64`
- later governance closeout commits synchronized traceability + runtime truth to the true Gate 3 halt state
- remote backup branch updated: `origin/feat/WP-1-Loom-Storage-Portability-v1`

### 14.2 Final Technical Verification

`Schema`

- detached validation worktree at `127af70`:
  - `just topology-registry-sync`
  - `just build-order-sync`
  - `just gov-check` -> PASS
  - `just post-work WP-1-Structured-Collaboration-Schema-Registry-v1 --range 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8..127af70` -> PASS with recorded waiver warnings only

`Loom`

- detached validation worktree at `35355ab`:
  - `just topology-registry-sync`
  - `just build-order-sync`
  - `just gov-check` -> PASS
  - `just post-work WP-1-Loom-Storage-Portability-v1 --range 1a2568b0842ecc7b9b9aca4efcc9911cc2ce8cc8..35355ab` -> PASS with recorded waiver warnings only

The earlier blocker state therefore no longer reflects the branch heads that exist at the end of this recovery.

### 14.3 Validator Outcome by WP

`Schema`

- integration validator produced PASS and the PASS report was presented
- packet status is `Done`
- packet verdict is `PASS`
- task board row is `[VALIDATED]`
- traceability row is `Done`
- runtime state now reflects that merge is blocked on Gate 4 acknowledgment, not on missing technical work

`Loom`

- integration validator repaired the closure surfaces, revalidated the WP, produced PASS, committed the PASS packet/board/build-order update, and presented the PASS report
- packet status is `Done`
- packet verdict is `PASS`
- task board row is `[VALIDATED]`
- build order row is `VALIDATED | DONE`
- traceability row is `Done`
- runtime state now reflects that merge is blocked on Gate 4 acknowledgment, not on missing technical work

### 14.4 Remaining Formal Blocker

Both WPs remain blocked from merge-to-`main` by the same hard protocol rule:

- Gate 3 has been recorded for both PASS reports
- Gate 4 user acknowledgment has not been recorded
- no lawful bypass was found in `.GOV/scripts/validation/validator_gates.mjs`

That means the technically accurate final state is:

- both WPs are finished at the branch level
- both WPs pass governance + deterministic post-work validation on their final pushed heads
- both WPs have PASS validation reports
- neither WP may be merged until the operator/user explicitly records Gate 4 acknowledgment

### 14.5 Additional Failures Found During Recovery

The recovery itself found more workflow/tooling defects that were not fully visible in the earlier provisional audit state:

- detached validator worktrees still require generator preflights (`topology-registry-sync`, and in some cases `build-order-sync`) before `gov-check` becomes green, even when those preflights do not produce a committed branch diff
- runtime-status JSON is schema-checked more strictly than the workflow initially assumed; RFC3339 UTC formatting had to use `Z` rather than `+00:00`
- branch-closeout truth can still drift after a validator PASS commit unless the Orchestrator explicitly synchronizes:
  - packet `CURRENT_STATE`
  - validator-of-record metadata
  - task board state
  - traceability row
  - runtime wait-state
- session registry/broker projection remained stale after successful validator completion; at the end of recovery, the authoritative output logs were more accurate than the registry projection

### 14.6 Additional Remediation Performed During Recovery

These remediations were executed after the workflow was resumed:

- resumed integration-validator closeout instead of stopping at status narration
- pushed both feature branches to their remote backup branches after the new PASS/closeout commits
- synchronized Schema packet/runtime truth to reflect the Gate 3 wait state
- synchronized Loom packet/runtime/traceability truth to reflect the Gate 3 wait state
- corrected runtime timestamp formatting so `wp-communications-check` would pass
- re-ran detached validation on the actual final branch tips, not on older PASS candidates

### 14.7 Final Judgment

The smoke test should still be judged as a workflow failure mode discovery exercise, not as a clean operational success.

But the final recovered technical state is much stronger than the earlier provisional state:

- both target WPs now survive the formal validator and governance gates that were executed during recovery
- both target WPs pass governance and deterministic post-work checks after the required detached-worktree preflights
- the stronger claim that both WPs were fully master-spec aligned is later revised by Section 14.13 after direct code/spec inspection of integrated `main`
- the remaining stop is a deliberate human-ack gate, not an unresolved engineering defect

### 14.8 Operator Gate 4 Acknowledgment Recorded On March 15, 2026

This audit section supersedes Section 14.4 as the current workflow state.

The operator then gave explicit Gate 4 acknowledgment commands for both WPs:

- `just validator-gate-acknowledge WP-1-Structured-Collaboration-Schema-Registry-v1`
- `just validator-gate-acknowledge WP-1-Loom-Storage-Portability-v1`

At the final post-ack closeout state, the authoritative feature branch heads were:

- `WP-1-Structured-Collaboration-Schema-Registry-v1` -> `9608a422c29577ee15ec592ff23ffd46b1743a7b`
- `WP-1-Loom-Storage-Portability-v1` -> `f97e2a4dbdab9e01ffa924cc45ffcf7c0f48b57f`

This changes the formal workflow state as follows:

- the earlier Gate 4 blocker is removed
- both WPs remain technically PASS at the same validated branch heads
- merge-to-`main` is now protocol-legal for both WPs
- any remaining stop after this point would be an orchestration or operator-choice stop, not a validator-gate stop

The acknowledgment commands were then executed successfully for both authoritative WP worktrees. This addendum therefore records the actual post-command state:

- Gate 4 is recorded for both WPs
- merge-to-`main` is now validator-gate-authorized for both WPs
- the workflow is no longer blocked on user acknowledgment

This addendum still does not itself merge either WP. It records that the acknowledgment was not just supplied, but actually executed, and that the next workflow state is merge-authorized rather than acknowledgment-blocked.

### 14.9 Post-Merge Addendum On March 15, 2026: Selective Main Integration And Worktree-Sprawl Failure

After Gate 4 was recorded, direct branch merge to `main` was attempted through governed Integration Validator sessions and failed for both WPs.

The deeper cause was not just normal merge conflict:

- both WP feature branches were not cleanly based for direct integration to current `main`
- each branch carried substantial governance/tooling ancestry from the orchestrator role branch
- a naive merge would have imported unrelated global governance drift into `main`
- Loom also carried unrelated Schema governance artifacts
- Schema additionally conflicted against newer `main` product code in shared backend surfaces

This forced a second integration strategy:

- Loom was integrated selectively onto `main` from a clean temp main worktree instead of by branch merge
- Schema was then integrated selectively onto the updated `main`, again from a clean temp main worktree, with guided conflict resolution for the shared backend files

Authoritative result:

- Loom selective main integration produced `origin/main` head `4fe9f8efdadd335f5a9b498fbebce0c26e9fde5f`
- Schema selective main integration then advanced `origin/main` to `e4e9eabcaa0767e64176e6f1bbb51754c16fb982`

This part was technically successful, but it exposed a major workflow failure:

- the orchestrator-managed run created far too many transient worktrees
- failed merge-only worktrees were not reaped promptly
- detached validator and revalidation worktrees accumulated across multiple candidate commits
- selective integration required yet more temp worktrees because the earlier branch-merge assumption was invalid
- two off-root temp worktrees were created outside the normal shared worktree cleanup root, which means the standard governed cleanup helper does not enumerate or delete them

I judge that as a substantial operational failure even though it reduced product-risk during integration.

Why it happened:

- no hard orchestration rule limited the number of active temporary validation or merge worktrees per WP
- no mandatory reuse policy existed for detached validation surfaces
- no automatic cleanup checkpoint ran after successful detached validation or failed merge attempts
- branch integration posture was not validated early enough
- the workflow assumed "validated branch head" implied "mergeable branch", which was false here
- the governed cleanup tooling only supports worktrees created under the shared worktree root, so off-root temp surfaces escaped the normal cleanup path

What was remediated during this run:

- both WPs were integrated to `main` without importing the full polluted feature-branch ancestry
- immutable backup snapshots were created before broad cleanup activity
- exact cleanup targets were enumerated before deletion
- the audit now records worktree-sprawl as a first-class workflow defect rather than incidental noise

What remains a defect even after the successful merge:

- worktree lifecycle discipline is still too weak
- off-root temp worktrees are not governed by the normal delete helper
- the orchestrator still lacks a "single temp worktree per purpose per WP" guardrail
- the integration path still allows late discovery that a branch is not directly mergeable to `main`

### 14.10 Required Prevention Controls

This failure should produce concrete workflow changes:

- require an early "integration posture" check before declaring a WP merge-ready:
  - can the branch be merged directly to `main`
  - or does it already require selective integration
- cap temp surfaces per WP by policy:
  - at most one active validator worktree
  - at most one active merge worktree
  - at most one active detached revalidation worktree
- require reuse of the existing temp worktree for the same purpose unless a conflict or corruption reason is recorded in the WP thread/receipts
- require immediate governed cleanup of superseded temp worktrees once a replacement surface is created and the old state is no longer needed
- forbid off-root temp worktree creation unless the governed cleanup tooling can also enumerate and delete that exact path
- extend `.GOV/scripts/delete-local-worktree.mjs` and `just enumerate-cleanup-targets` so off-root git worktrees are governed cleanup targets instead of invisible exceptions
- add a worktree-budget warning to the operator monitor and fail orchestration when a WP exceeds the allowed temp-worktree count without an explicit override receipt
- add an audit receipt whenever integration shifts from direct branch merge to selective integration, because that is a material workflow state change
- treat "validated branch head" and "mergeable integration candidate" as separate truths in protocol and tooling

Final judgment on this specific issue:

- the extra worktrees did reduce integration risk
- but their quantity, lifetime, and unmanaged spread were not acceptable
- this should be treated as a workflow failure that must be fixed in governance/tooling, not as an acceptable cost of cautious validation

### 14.11 Approved Cleanup Execution On March 15, 2026

After the operator supplied explicit assistant deletion approval for the named WP worktrees, governed cleanup was executed under a fresh immutable snapshot:

- snapshot root: `D:\Projects\LLM projects\Handshake\Handshake Backups\20260315-031748Z-pre-cleanup-approved-20260315`

This cleanup pass exposed an additional tooling defect:

- `.GOV/scripts/delete-local-worktree.mjs` already used `git -c core.longpaths=true worktree remove`
- but its dirty-worktree safety path still used plain `git stash push -u`
- on Windows, that asymmetry caused safety-stash failure in dirty Schema worktrees because long `SESSION_CONTROL_OUTPUTS` paths exceeded the default git path handling

Remediation applied during cleanup:

- patched the governed delete helper so the safety stash path also uses `git -c core.longpaths=true stash push -u -m ...`
- retried only the still-approved Schema cleanup targets after the patch
- both live Schema worktrees then deleted successfully under governed cleanup

Actual cleanup result:

- all approved live git-registered Loom worktrees were deleted
- all approved live git-registered Schema worktrees were deleted
- three approved Schema-named paths remained on disk only as orphan directories:
  - `wt-reval-WP-1-Structured-Collaboration-Schema-Registry-v1-00b7715`
  - `wt-reval-WP-1-Structured-Collaboration-Schema-Registry-v1-8d500b5`
  - `wt-VALTMP-WP-1-Structured-Collaboration-Schema-Registry-v1-cf3457f`

Those three orphan directories were not deleted for a governance reason, not because the operator approval was unclear:

- by the time cleanup ran, they were no longer git-registered worktrees
- the governed delete helper correctly refused them with `Target is not a git checkout; direct filesystem deletion is forbidden`
- repo guardrails explicitly forbid falling back to `Remove-Item`, `rm`, or other raw filesystem deletion inside the shared worktree root after a failed `git worktree remove`

This means the cleanup outcome is mixed:

- governed cleanup successfully removed the approved live worktrees
- the repo still has orphan directory debt caused by earlier worktree-remove failures
- the current tooling cannot lawfully finish that last part without either:
  - a new governed orphan-directory recovery flow
  - or explicit operator manual recovery outside assistant deletion authority

This should be treated as a second-order workflow failure produced by the original worktree-sprawl problem:

- too many transient worktrees increased cleanup surface area
- repeated failed remove/revalidate cycles created opportunities for partially-detached orphan directories
- the repository has good protections against unsafe deletion, but no equally-governed orphan reclamation path yet

### 14.12 Post-Cleanup Governance Resynchronization

Approved worktree deletion exposed one more orchestrator failure mode after the product work was already finished:

- the canonical closure mirrors in `wt-orchestrator` were still stale even after both WPs had passed, been gate-acknowledged, and been integrated to `main`
- the Task Board still projected both WPs as active/in-progress
- both official task packets still said `Status: In Progress` with `Verdict: PENDING`
- both runtime status files still projected live coder-remediation states
- this caused `just gov-check` to fail with a false concurrency error after the live WP worktrees were correctly deleted

That was a workflow bookkeeping failure, not a product failure.

Remediation performed:

- moved both WPs on the Task Board to `Done / [VALIDATED]`
- updated both official packets from `In Progress / PENDING` to `Done / PASS`
- updated both runtime status files to completed post-merge state
- updated the two traceability rows from `In Progress` to `Done`
- re-synced generated build-order output with `just build-order-sync`

Final governance result after those fixes:

- `just gov-check` passes in `wt-orchestrator`
- the earlier false concurrent-worktree failure is gone
- the remaining cleanup problem is now limited to orphan directories that governance correctly refused to delete unsafely

This matters because it shows the workflow did not merely suffer from excess temp worktrees; it also lacked a reliable end-of-run truth-sync step after merge and cleanup.

### 14.13 Post-Hoc Code/Spec Inspection On March 15, 2026: Main vs WP Branch Tips vs Master Spec

After the workflow recovery, validation, acknowledgment, integration, and cleanup steps were complete, a separate read-only engineering review was requested against the master spec.

Scope and method of that review:

- this was not another formal validator run
- local `main` in `wt-orchestrator` was stale, so the actual integrated product baseline inspected was `origin/main` head `e4e9eabcaa0767e64176e6f1bbb51754c16fb982` through the preserved temp main worktree `D:\wtsel-4fe9f8e-main-1`
- the feature branch heads inspected were:
  - `WP-1-Structured-Collaboration-Schema-Registry-v1` -> `9608a422c29577ee15ec592ff23ffd46b1743a7b`
  - `WP-1-Loom-Storage-Portability-v1` -> `f97e2a4dbdab9e01ffa924cc45ffcf7c0f48b57f`
- the branch heads are ahead mainly by governance closeout history, so product-correctness judgment should be read primarily against the actual landed code on `origin/main`

Findings from that post-hoc inspection:

1. Structured collaboration artifacts still drop `profile_extension`.
   - `TrackedWorkPacket` and `TrackedMicroTask` source models still carry `profile_extension`.
   - the structured-collaboration validator logic still validates `profile_extension`.
   - but the emitted portable artifact payloads for `TrackedWorkPacketArtifactV1` and `TrackedMicroTaskArtifactV1` do not serialize it into the record that is actually written to disk.
   - this is not a cosmetic omission. The master spec explicitly requires project-specific details to live inside `profile_extension` payloads and requires those payloads to declare `extension_schema_id`, `extension_schema_version`, and `compatibility`.
   - evidence in integrated `main`:
     - model/validator side:
       - `src/backend/handshake_core/src/locus/types.rs:442`
       - `src/backend/handshake_core/src/locus/types.rs:623`
       - `src/backend/handshake_core/src/locus/types.rs:1072`
       - `src/backend/handshake_core/src/locus/types.rs:1628`
     - artifact-serialization side:
       - `src/backend/handshake_core/src/locus/types.rs:185`
       - `src/backend/handshake_core/src/locus/types.rs:239`
       - `src/backend/handshake_core/src/workflows.rs:3380`
       - `src/backend/handshake_core/src/workflows.rs:3467`
   - master-spec anchors:
     - `Handshake_Master_Spec_v02.178.md:6851`
     - `Handshake_Master_Spec_v02.178.md:6853`

2. Integrated task-board structured artifacts are internally inconsistent.
   - the structured-collaboration validator still requires `rows` and `lane_ids` for task-board index/view records.
   - tests also inspect task-board JSON using `rows`.
   - but the current integrated task-board structs and emitters serialize `entries` and `lanes` instead.
   - this means the current integrated structured task-board projection surface is not code-correct even before broader interpretation of spec intent.
   - evidence in integrated `main`:
     - validator expectation:
       - `src/backend/handshake_core/src/locus/types.rs:1105`
       - `src/backend/handshake_core/src/locus/types.rs:1110`
       - `src/backend/handshake_core/src/locus/types.rs:1111`
     - emitted shape:
       - `src/backend/handshake_core/src/locus/task_board.rs:56`
       - `src/backend/handshake_core/src/locus/task_board.rs:75`
       - `src/backend/handshake_core/src/locus/task_board.rs:86`
       - `src/backend/handshake_core/src/locus/task_board.rs:104`
       - `src/backend/handshake_core/src/workflows.rs:3692`
       - `src/backend/handshake_core/src/workflows.rs:3717`
       - `src/backend/handshake_core/src/workflows.rs:3731`
     - test expectation:
       - `src/backend/handshake_core/tests/micro_task_executor_tests.rs:890`
   - relevant spec anchors:
     - `Handshake_Master_Spec_v02.178.md:5825`
     - `Handshake_Master_Spec_v02.178.md:6880`

3. No concrete Loom storage-portability spec defect was found in this pass.
   - the portable contract types required by the spec are present:
     - `src/backend/handshake_core/src/storage/loom.rs:274`
     - `src/backend/handshake_core/src/storage/loom.rs:348`
     - `src/backend/handshake_core/src/storage/loom.rs:360`
   - both SQLite and PostgreSQL implement `search_loom_blocks`:
     - `src/backend/handshake_core/src/storage/sqlite.rs:2755`
     - `src/backend/handshake_core/src/storage/postgres.rs:2336`
   - the conformance suite covers source-anchor round-trip and backend-parity search semantics:
     - `src/backend/handshake_core/src/storage/tests.rs:573`
     - `src/backend/handshake_core/src/storage/tests.rs:815`
     - `src/backend/handshake_core/src/storage/tests.rs:892`
     - `src/backend/handshake_core/src/storage/tests.rs:1128`
     - `src/backend/handshake_core/src/storage/tests.rs:1145`
     - `src/backend/handshake_core/tests/storage_conformance.rs:34`
     - `src/backend/handshake_core/tests/storage_conformance.rs:50`
   - the inspected Loom result is therefore narrower and stronger:
     - no concrete defect found for the storage-portability slice actually targeted by the WP
     - but this review does not claim full Loom master-spec closure outside that slice

Revised judgment after the post-hoc inspection:

- the earlier statement in this audit that both target WPs were "master-spec aligned at the validated branch heads" is too strong and should be treated as superseded
- `WP-1-Structured-Collaboration-Schema-Registry-v1` remains a historical formal validator PASS, but the later direct code/spec review does not support calling it fully code-correct against the master spec
- `WP-1-Loom-Storage-Portability-v1` remains the stronger result from this run; no concrete code/spec defect was found in its targeted portability scope during this inspection
- this divergence between formal validator PASS and later engineering code/spec review is itself an audit-worthy process lesson:
  - validator PASS, governance PASS, and true code/spec correctness are not identical truths
  - future smoke tests should record post-merge code/spec inspection explicitly when validator evidence is constrained by environment or inherited surface complexity

### 14.14 Root Cause Analysis: Why Formal Validation Passed Anyway

The direct answer is that the current workflow compresses several different truths into one word: `PASS`.

What the recovery-time validation actually proved:

- packet/refinement/worktree state was coherent enough for the governed flow to continue
- required workflow checkpoints and deterministic manifests were present
- `just pre-work`, `just post-work`, and `just validator-handoff-check` passed on the committed WP branch heads after the necessary worktree preflights
- Loom had meaningful targeted runtime evidence from passing storage-conformance and Loom-focused tests

What those same validations did **not** actually prove:

- that every changed product-code path was semantically correct against every relevant master-spec clause
- that later selective integration onto `main` preserved all structured-collaboration contracts without drift
- that the validator had exhaustively inspected every normative claim implied by the packet wording
- that environment-blocked areas were compensated by a second independent code/spec inspection

In other words: the system used `PASS` as a workflow/legal verdict, while humans later read it as a product/spec-completeness verdict.

Concrete reasons this happened:

1. `gov-check` is governance-only.
   - `.GOV/scripts/validation/gov-check.mjs` runs governance integrity checks only.
   - it does not inspect product-code semantics in `src/`, `app/`, or `tests/`.
   - a green `gov-check` therefore created confidence about repo truth, not code correctness.

2. `validator-spec-regression` is spec-presence validation, not implementation validation.
   - `.GOV/scripts/validation/validator-spec-regression.mjs` only verifies that `SPEC_CURRENT` points at an existing spec file and that a small list of anchor strings exists.
   - it does not compare the WP diff against the packet's actual `SPEC_ANCHOR`, `DONE_MEANS`, or extracted MUST/SHOULD set.

3. `validator-handoff-check` mainly chains workflow gates, not semantic review.
   - `.GOV/scripts/validation/validator-handoff-check.mjs` proves committed handoff evidence by running:
     - `just pre-work`
     - `just cargo-clean`
     - `just post-work`
   - it records PASS/FAIL for those commands and the committed target.
   - it does **not** itself perform clause-by-clause code/spec reasoning.

4. `post-work` is a deterministic manifest gate, not a spec-completeness gate.
   - `.GOV/scripts/validation/post-work.mjs` only runs:
     - `gate-check.mjs`
     - `post-work-check.mjs`
     - `role_mailbox_export_check.mjs`
   - `post-work-check.mjs` enforces manifest windows, hashes, rails, packet hygiene, and phase discipline.
   - it does not know that `profile_extension` was supposed to be serialized into the portable artifacts, nor that task-board JSON shape had drifted from validator/test expectations.

5. `validator-scan`, `validator-coverage-gaps`, and `validator-traceability` are coarse heuristics.
   - `validator-scan.mjs` looks for forbidden patterns like `unwrap()`, `todo!`, `console.log`, and placeholder text.
   - `validator-coverage-gaps.mjs` only checks whether some tests exist in the target scopes.
   - `validator-traceability.mjs` checks for broad tokens like `job_id` and `trace_id`.
   - none of these tools can detect a wrong artifact contract that still compiles and still has tests nearby.

6. The validator protocol requires stronger review than the scripts enforce.
   - the Validator protocol says "Evidence or Death" and says the validator must block merges unless evidence proves the work meets spec, codex, and packet requirements.
   - in practice, the mechanized gates currently prove packet/gov/determinism strongly, but product semantic review remains manual and therefore vulnerable to overclaim, fatigue, or inherited assumptions.

7. The final recovery narration overclaimed what the evidence supported.
   - the audit and status reporting promoted "formal validator PASS + post-work PASS + targeted tests green" into "master-spec aligned."
   - that wording was stronger than the actual evidence base, especially for the Schema WP.

### 14.15 Bootstrap vs Skeleton: Current Protocol vs Current Enforcement

Yes. The protocol expects both a bootstrap claim checkpoint and a skeleton checkpoint.

Current protocol expectation:

- Coder protocol lists these recovery milestones on the WP backup branch:
  - docs-only bootstrap claim commit
  - docs-only skeleton checkpoint commit
  - skeleton approval commit before implementation continues
- Validator protocol also describes the bootstrap claim as part of the expected sequence before later validation.

Current mechanical enforcement, however, is weaker than the protocol text:

- `pre-work.mjs` mechanically blocks implementation if a skeleton checkpoint exists without a matching skeleton approval commit.
- `coder-skeleton-checkpoint.mjs` creates and enforces the docs-only skeleton checkpoint.
- `skeleton-approved.mjs` creates and enforces the approval marker.
- but the bootstrap claim commit is **not** enforced by `gate-check.mjs`, `pre-work.mjs`, `pre-work-check.mjs`, `post-work.mjs`, or `validator-handoff-check.mjs`.
- bootstrap claim existence is currently visible in helper/resume logic such as `.GOV/scripts/coder-next.mjs`, but it is not yet a hard mechanical closure prerequisite.

There is also a protocol contradiction that should be called out explicitly:

- in `VALIDATOR_PROTOCOL.md`, the bootstrap status-sync section says the bootstrap commit MAY include the initial `## SKELETON` proposal as a fast path
- later in the same protocol, BOOTSTRAP verification says that if the Coder included SKELETON content in the BOOTSTRAP turn, treat it as invalid phase merging and require a separate SKELETON turn/commit

That contradiction makes it easier for different actors to believe they are following the protocol while actually enforcing different phase laws.

That is a real governance/tooling gap:

- the protocol talks as if BOOTSTRAP claim is a mandatory checkpoint
- the mechanized gates actually treat SKELETON + approval as the hard control point
- that mismatch weakens the audit value of the earlier phase and makes it easier to treat "I read the packet" as equivalent to "I produced a committed, reviewable bootstrap claim"
- the protocol contradiction around BOOTSTRAP+SKELETON further erodes confidence in what the phase boundary is supposed to mean

### 14.16 Why Both Coder And Validator Could Honestly Think The Work Was Finished

This was not pure negligence. The current system steers them toward the wrong confidence boundary.

Why the Coder could think it was finished:

- the coder's success criteria are heavily framed around:
  - packet scope clean
  - targeted tests green
  - deterministic gate commands green
  - handoff evidence recorded
- for Loom, that model fit reality reasonably well
- for Schema, the environment issues and later integration drift obscured that some structured-collaboration contracts were still incomplete at the landed `main` state

Why the Validator could think it was finished:

- the validator's strongest mechanical tools are governance/packet/deterministic gates, not semantic diff-to-spec proof
- the validator did not have a forced artifact saying:
  - "these exact spec clauses were reviewed"
  - "these exact clauses remain only partially proven"
  - "this PASS is legal/governance PASS but only partial semantic confidence"
- once pre-work, post-work, handoff check, and targeted test evidence were green, the path of least resistance was to record PASS rather than a split verdict

Why the system amplified that mistake:

- packet `Verdict: PASS`
- audit text saying "master-spec aligned"
- merge authorization
- integration success

All of those created a narrative of completeness even though the supporting proof was mixed in quality.

### 14.17 Prevention Plan: Improve Correctness And Completeness Without Creating More Governance Blockers

The goal should be better separation of truths, not more blanket red tape.

Recommended changes:

1. Split validator verdicts into explicit dimensions.
   - Keep one final legal/workflow verdict if needed, but also require explicit fields:
     - `GOVERNANCE_VERDICT`
     - `PACKET_VERDICT`
     - `TEST_VERDICT`
     - `CODE_REVIEW_VERDICT`
     - `SPEC_ALIGNMENT_VERDICT`
     - `ENVIRONMENT_VERDICT`
   - allowed values can be lightweight:
     - `PASS | FAIL | PARTIAL | BLOCKED`
   - this avoids one overloaded `PASS` swallowing uncertainty.

2. Make spec-alignment review mandatory, but diff-scoped.
   - Do **not** require proving the whole master spec every time.
   - For each WP, require a short append-only validator block:
     - exact packet `SPEC_ANCHOR`
     - extracted MUST/SHOULD list actually in scope for the touched files
     - file:line evidence for each satisfied clause
     - explicit "not proven" list for anything left partial
   - this is a review artifact, not a new giant gate.

3. Add a non-blocking `SPEC_CONFIDENCE` field to packets and audits.
   - suggested values:
     - `NONE`
     - `PARTIAL_DIFF_SCOPED`
     - `REVIEWED_DIFF_SCOPED`
     - `POST_MERGE_RECHECKED`
   - keep merge blocking on existing legal gates, but stop people from reading `PASS` as stronger than it is.

4. Reserve hard blockers for a small number of semantic tripwires.
   - Add targeted validators only for high-value, repeat-failure classes:
     - shared structured artifact contract mismatches
     - serde field-name drift between emitters and validators/tests
     - required extension-envelope fields silently dropped during serialization
   - These should be narrow contract assertions, not generic "prove the whole spec" tools.

5. Enforce bootstrap claim mechanically, but cheaply.
   - Update `pre-work` or `pre-work-check` so a missing `docs: bootstrap claim [WP-{ID}]` is a hard fail before skeleton or implementation.
   - This adds almost no review cost because it is just a commit-subject existence check, but it restores the protocol's intended phase evidence.

6. Require a real validator report in the packet before `Verdict: PASS`.
   - Today the packet can end up in PASS state while `## VALIDATION_REPORTS` is effectively empty.
   - Require one append-only report block that states:
     - what was run
     - what was manually reviewed
     - what was not proven
     - the split verdicts above
   - This is high signal and low bureaucracy.

7. Add a post-merge spot-check mode for shared backend surfaces.
   - Only for WPs that touch highly shared files like:
     - `workflows.rs`
     - shared schema/type registries
     - storage portability layers
   - This is a lightweight re-open/read-only check on integrated `main`, not a full second validation workflow.
   - The current smoke test showed why this matters: selective integration can preserve tests/gates while still shifting the final landed semantics.

8. Treat environment-blocked validation as `PARTIAL`, not `PASS`.
   - If key compile/test surfaces are blocked by external environment issues, keep legal closure possible if policy allows, but force:
     - `ENVIRONMENT_VERDICT: BLOCKED`
     - `SPEC_ALIGNMENT_VERDICT: PARTIAL`
   - This prevents a blocked lane from being narratively upgraded into full correctness.

9. Tighten packet language so DONE_MEANS are spec-claim-safe.
   - DONE_MEANS should say what the WP proves in implementation terms.
   - It should not allow a reviewer to infer broader closure than the WP actually touched.
   - In practice: require "this WP proves X for these families/files" rather than "schema registry complete" style summaries.

10. Add a thin "claim audit" lint over audits and status sync.
   - Audit/status text should fail governance if it claims:
     - "master-spec aligned"
     - "fully correct"
     - "survives technical scrutiny"
   - unless a matching validator report block records the basis for that exact claim.
   - This is not a product-code blocker; it is a wording-discipline check on governance output.

Recommended overall stance:

- do **not** turn every semantic doubt into a new hard blocker
- do force every validator PASS to say what kind of PASS it is
- do add a few narrow semantic tripwire tests for recurring contract classes
- do require explicit "not proven" sections so incomplete confidence is visible instead of hidden

### 14.18 Governance/Workflow Remediations Implemented On March 15, 2026

The following prevention changes are now actually implemented in repo governance/tooling, not merely recommended:

1. Forward-only packet format bump for the stronger validator contract.
   - New packet format version is now `2026-03-15`.
   - Existing `2026-03-12` packets remain valid; session-policy checks were preserved so history did not retro-break.
   - Implemented in:
     - `.GOV/scripts/session-policy.mjs`
     - `.GOV/scripts/validation/session-policy-check.mjs`

2. Bootstrap claim is now mechanically enforced for new-format packets.
   - `pre-work-check` now fails if a new-format WP has entered real work (`In Progress`, claimed coder fields, or skeleton checkpoint present) without `docs: bootstrap claim [WP-{ID}]`.
   - `coder-skeleton-checkpoint` now refuses to create a skeleton checkpoint if the bootstrap claim commit is missing.
   - `pre-work` now gives a specific recovery command instead of generic failure text.
   - Implemented in:
     - `.GOV/scripts/validation/pre-work-check.mjs`
     - `.GOV/scripts/validation/pre-work.mjs`
     - `.GOV/scripts/validation/coder-skeleton-checkpoint.mjs`

3. New governed validator split-verdict contract exists for new-format packets.
   - New packets now declare:
     - `GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_V1`
     - `GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE`
   - The packet template now requires `CLAUSES_REVIEWED` and `NOT_PROVEN` for governed validation reports on the new packet format.
   - Implemented in:
     - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
     - `.GOV/scripts/validation/session-policy-check.mjs`

4. Closed new-format packets now require a real structured validator report.
   - `validator-packet-complete` now refuses closed `2026-03-15` packets unless `## VALIDATION_REPORTS` contains the split verdict fields, `CLAUSES_REVIEWED`, and `NOT_PROVEN`.
   - A new repo-wide checker now scans all closed new-format packets and fails governance if that report structure is missing or internally inconsistent.
   - The new checker is wired into both `gov-check` and `codex-check`.
   - Implemented in:
     - `.GOV/scripts/validation/validator-packet-complete.mjs`
     - `.GOV/scripts/validation/validator-report-structure-check.mjs`
     - `.GOV/scripts/validation/gov-check.mjs`
     - `.GOV/scripts/validation/codex-check.mjs`
     - `justfile`

5. The Validator protocol contradiction on BOOTSTRAP vs SKELETON is now resolved.
   - The protocol no longer permits a BOOTSTRAP + SKELETON fast path.
   - BOOTSTRAP and SKELETON are now explicitly separate turns/commits again.
   - The validator report template now includes split verdicts, diff-scoped clause review, and `NOT_PROVEN`.
   - A short validator completion checklist was added to reduce overclaiming.
   - Implemented in:
     - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

6. The Coder protocol now carries an explicit diff-scoped spec self-check.
   - Before handoff on new-format packets, the coder is now told to re-read the exact claimed clauses, check that required fields are actually emitted end-to-end, and verify shared contract-name consistency.
   - This is protocol guidance, not a new product-code blocker.
   - Implemented in:
     - `.GOV/roles/coder/CODER_PROTOCOL.md`

7. The Orchestrator protocol now states the wording discipline explicitly.
   - The protocol now forbids describing a WP as "master-spec aligned", "fully correct", or equivalent unless the packet report actually supports that claim.
   - Implemented in:
     - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

Verification after landing these changes:

- `just gov-check`: PASS in `wt-orchestrator`

What remains not yet mechanized:

- narrow semantic tripwire tests for recurring product-contract failures (for example: dropped required artifact fields or emitter/validator field-name drift)
- lightweight post-merge spot-check mode for shared backend surfaces
- a full audit/status wording lint beyond the new Orchestrator wording rule

So the repo is materially better protected now, but it still does **not** have automatic semantic proof of master-spec correctness. The implemented changes mainly force better evidence, better honesty about what is and is not proven, and harder workflow discipline around bootstrap and validator closure.

### 14.19 Refinement-Phase Carry-Forward Remediations Implemented On March 15, 2026

The refinement phase is now less lossy downstream for new refinements.

Before this remediation:

- the signed refinement already held the strongest spec evidence
  - anchor windows
  - excerpts
  - upstream scope reasoning
- but the task packet only carried a compressed subset of that work
- coder and validator had to reconstruct too much from the signed refinement manually
- that created avoidable room for clause drift, contract-surface drift, and overclaiming

The following changes are now implemented:

1. New refinements now carry explicit downstream proof/handoff sections.
   - `REFINEMENT_FORMAT_VERSION` advanced to `2026-03-15`.
   - New-format hydrated refinements must now include:
     - `CLAUSE_PROOF_PLAN`
     - `CONTRACT_SURFACES`
     - `CODER_HANDOFF_BRIEF`
     - `VALIDATOR_HANDOFF_BRIEF`
     - `NOT_PROVEN_AT_REFINEMENT_TIME`
   - This was implemented in:
     - `.GOV/templates/REFINEMENT_TEMPLATE.md`
     - `.GOV/scripts/validation/refinement-check.mjs`

2. Signed spec anchor windows are now carried forward into the packet.
   - Packet creation now copies the signed refinement anchor windows/excerpts into:
     - `## SPEC_CONTEXT_WINDOWS`
   - This closes the earlier downstream loss where only `SPEC_ANCHOR_PRIMARY` survived into the packet.
   - Implemented in:
     - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
     - `.GOV/scripts/refinement-brief-lib.mjs`
     - `.GOV/scripts/create-task-packet.mjs`

3. The new refinement sections are now auto-hydrated into new packets.
   - Packet creation now deterministically carries forward:
     - clause proof rows
     - contract-surface rows
     - coder handoff brief
     - validator handoff brief
     - explicit not-proven ledger
   - Implemented in:
     - `.GOV/scripts/refinement-brief-lib.mjs`
     - `.GOV/scripts/create-task-packet.mjs`

4. Packet/refinement drift is now enforced on the carried-forward sections.
   - `pre-work-check` now compares the packet against the signed refinement for:
     - `SPEC_CONTEXT_WINDOWS`
     - `CLAUSE_PROOF_PLAN`
     - `CONTRACT_SURFACES`
     - `CODER_HANDOFF_BRIEF`
     - `VALIDATOR_HANDOFF_BRIEF`
     - `NOT_PROVEN_AT_REFINEMENT_TIME`
   - The packet generator and the drift checker now use the same shared formatter library so the output shape is deterministic.
   - Implemented in:
     - `.GOV/scripts/refinement-brief-lib.mjs`
     - `.GOV/scripts/validation/pre-work-check.mjs`
     - `.GOV/scripts/create-task-packet.mjs`

5. Backward compatibility was preserved deliberately.
   - Existing signed `2026-03-08` refinements were not retro-broken.
   - The stronger handoff sections are enforced only for `REFINEMENT_FORMAT_VERSION >= 2026-03-15`.
   - Legacy hydrated refinements still validate, and their `SPEC_ANCHORS` now at least hydrate into packet `SPEC_CONTEXT_WINDOWS`.
   - This matters because the repo already contains signed `2026-03-08` refinement history that should remain auditable and lawful.

Verification after landing these refinement changes:

- `node --check .GOV/scripts/refinement-brief-lib.mjs`: PASS
- `node --check .GOV/scripts/create-task-packet.mjs`: PASS
- `node --check .GOV/scripts/validation/refinement-check.mjs`: PASS
- `node --check .GOV/scripts/validation/pre-work-check.mjs`: PASS
- `validateRefinementFile(...)` on both existing `2026-03-08` live-smoketest refinements: PASS
- `just gov-check`: PASS in `wt-orchestrator`

Net effect:

- refinement remains the canonical analysis artifact
- the packet now carries materially more of the signed refinement truth forward
- coder and validator have a better diff-scoped proof plan without needing to reconstruct it from scratch
- the system still does **not** automatically prove product semantics, but it now preserves and enforces much better upstream context for that work

### 14.20 Packet-Level Spec-Closure Monitoring Implemented On March 15, 2026

The packet now carries a compact live dashboard for spec closure, rather than forcing the Orchestrator and Validator to infer closure state only from prose, chat, or append-only reports.

Implemented changes:

1. New packets now carry live monitoring sections.
   - `CLAUSE_CLOSURE_MATRIX`
   - `SPEC_DEBT_STATUS`
   - `SHARED_SURFACE_MONITORING`
   - These sections are authoritative packet truth and intentionally mutable.
   - Implemented in:
     - `.GOV/templates/TASK_PACKET_TEMPLATE.md`

2. Packet creation now seeds the monitoring state automatically.
   - `CLAUSE_CLOSURE_MATRIX` is seeded from refinement `CLAUSE_PROOF_PLAN`.
   - `SPEC_DEBT_STATUS` is seeded to no open debt by default.
   - `SHARED_SURFACE_MONITORING` is seeded from coder handoff hot files + tripwire tests, with a conservative shared-surface heuristic.
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/create-task-packet.mjs`

3. `pre-work` now sanity-checks the packet monitoring sections for new-format packets.
   - New packets must contain parseable closure rows, valid debt fields, and valid shared-surface monitoring fields.
   - If shared-surface risk is `YES`, the packet must list concrete hot files and required tripwire tests.
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/validation/pre-work-check.mjs`

4. Future validator closure now requires the packet monitoring sections to be used honestly.
   - Closed packets must not leave clause rows at `VALIDATOR_STATUS=PENDING`.
   - `SPEC_ALIGNMENT_VERDICT=PASS` now also requires:
     - every clause row to end in `CODER_STATUS=PROVED|NOT_APPLICABLE`
     - every clause row to end in `VALIDATOR_STATUS=CONFIRMED|NOT_APPLICABLE`
     - `SPEC_DEBT_STATUS` to show no open debt
   - This is enforced in the validator closure gate, but was **not** made repo-wide retroactive for historical closed packets.
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/validation/validator-packet-complete.mjs`

5. Protocols were updated so the packet sections are part of normal role behavior.
   - Coder protocol now tells the coder to update `CLAUSE_CLOSURE_MATRIX` and `SPEC_DEBT_STATUS` before handoff.
   - Validator protocol now ties `SPEC_ALIGNMENT_VERDICT=PASS` to the packet closure matrix + debt status.
   - Orchestrator protocol now treats the packet monitoring sections as the packet-scope spec-closure dashboard.
   - Implemented in:
     - `.GOV/roles/coder/CODER_PROTOCOL.md`
     - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
     - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

Important design decision:

- This was **not** added to the repo-wide closed-packet audit retroactively.
- Reason: the two already-closed live-smoketest packets use `PACKET_FORMAT_VERSION=2026-03-15` but were closed before these packet-monitoring sections existed.
- Retroactively failing `just gov-check` on those historical packets would create governance churn instead of improving future correctness.
- Therefore the stronger enforcement is now:
  - packet creation
  - `just pre-work`
  - future validator closure
- not historical packet re-litigation

Additional verification note:

- During implementation, `node --check` exposed an existing UTF-8 BOM at the top of `.GOV/scripts/validation/validator-packet-complete.mjs`.
- That BOM was removed as part of this governance patch so the closure-gate script is syntactically valid under Node.

Verification after landing these changes:

- `node --check .GOV/scripts/packet-closure-monitor-lib.mjs`: PASS
- `node --check .GOV/scripts/create-task-packet.mjs`: PASS
- `node --check .GOV/scripts/validation/pre-work-check.mjs`: PASS
- `node --check .GOV/scripts/validation/validator-packet-complete.mjs`: PASS
- `just gov-check`: PASS in `wt-orchestrator`

### 14.21 Follow-Up Remediations For Open Risks Implemented On March 15, 2026

After the first packet-monitoring rollout, several concrete concerns remained. They are now addressed as follows:

1. Legacy refinement compatibility bridge is now implemented.
   - Problem:
     - new packets use `PACKET_FORMAT_VERSION=2026-03-15`
     - older signed refinements such as the live-smoketest `2026-03-08` refinements did not carry `CLAUSE_PROOF_PLAN`
     - that meant packet creation could seed an empty `CLAUSE_CLOSURE_MATRIX`, and `pre-work` would then fail
   - Fix:
     - clause rows now fall back to signed `SPEC_ANCHORS`, `IN_SCOPE_PATHS`, `DONE_MEANS`, and `TEST_PLAN` when the refinement predates the richer handoff format
     - shared-surface monitoring also falls back to in-scope paths and legacy test-plan context
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/create-task-packet.mjs`
   - Compatibility proof:
     - the old Loom refinement now deterministically seeds 4 legacy bridge clause rows with `shared_risk=YES`

2. Spec debt is now governed truth, not a free-text packet field.
   - Problem:
     - `SPEC_DEBT_STATUS` existed, but there was no canonical debt registry or existence/uniqueness validation
   - Fix:
     - added `.GOV/roles_shared/SPEC_DEBT_REGISTRY.md`
     - added parser/validator for debt rows
     - packet debt IDs now must exist in the registry, belong to the same WP, remain `STATUS=OPEN` while referenced, and match blocking semantics
   - Implemented in:
     - `.GOV/roles_shared/SPEC_DEBT_REGISTRY.md`
     - `.GOV/scripts/spec-debt-registry-lib.mjs`
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/validation/spec-debt-registry-check.mjs`

3. Repo-wide closure-monitor auditing now exists.
   - Problem:
     - packet closure monitoring was enforced only during packet creation / `pre-work` / validator closeout, not by repo-wide governance scans
   - Fix:
     - added `packet-closure-monitor-check`
     - `just gov-check` now audits packets that opt into `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`
   - Important non-retroactive rule:
     - historical closed packets without that profile are intentionally ignored
     - active `2026-03-15+` packets without the profile now fail governance
   - Implemented in:
     - `.GOV/scripts/validation/packet-closure-monitor-check.mjs`
     - `.GOV/scripts/validation/gov-check.mjs`
     - `.GOV/templates/TASK_PACKET_TEMPLATE.md`

4. Matrix/report reconciliation is now mechanically checked.
   - Problem:
     - the packet could contain `CLAUSE_CLOSURE_MATRIX`, while the validator report could still narrate different clause coverage
   - Fix:
     - validator closeout now cross-checks `CLAUSE_CLOSURE_MATRIX` against `CLAUSES_REVIEWED` and `NOT_PROVEN`
     - confirmed rows must appear in `CLAUSES_REVIEWED`
     - partial/rejected rows must appear in `NOT_PROVEN`
     - guidance now explicitly tells validators to reuse the exact matrix clause text
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/validation/validator-packet-complete.mjs`
     - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
     - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

5. Shared-surface monitoring is now seeded more conservatively.
   - Problem:
     - the first version looked only at coder handoff hot files, which was too easy to under-seed on legacy or sparse refinements
   - Fix:
     - shared-surface detection now also considers `IN_SCOPE_PATHS`
     - when legacy refinements do not provide explicit tripwire tests, packet seeding now produces a bridge tripwire reminder instead of an empty list
   - Implemented in:
     - `.GOV/scripts/packet-closure-monitor-lib.mjs`
     - `.GOV/scripts/create-task-packet.mjs`

6. A real runtime bug was found and fixed in `pre-work-check`.
   - Problem:
     - `pre-work-check.mjs` referenced `isModernPacket` before declaration
     - `node --check` would not catch this because it was a runtime, not syntax, failure
   - Fix:
     - the variable order was corrected while adding the new closure-monitor profile gate
   - Implemented in:
     - `.GOV/scripts/validation/pre-work-check.mjs`

7. Non-retroactive behavior was aligned across the gates.
   - Problem:
     - repo-wide governance was intentionally non-retroactive for historical closed packets, but the per-WP validator closeout helper was still stricter
   - Fix:
     - historical closed packets without `CLAUSE_CLOSURE_MONITOR_PROFILE` remain outside the new closure-monitor enforcement
     - future packets still get the stronger path through packet creation, `pre-work`, and closeout
   - Implemented in:
     - `.GOV/scripts/validation/validator-packet-complete.mjs`
     - `.GOV/scripts/validation/packet-closure-monitor-check.mjs`

Verification after landing these follow-up remediations:

- `node --check .GOV/scripts/spec-debt-registry-lib.mjs`: PASS
- `node --check .GOV/scripts/packet-closure-monitor-lib.mjs`: PASS
- `node --check .GOV/scripts/validation/spec-debt-registry-check.mjs`: PASS
- `node --check .GOV/scripts/validation/packet-closure-monitor-check.mjs`: PASS
- `node --check .GOV/scripts/validation/pre-work-check.mjs`: PASS
- `node --check .GOV/scripts/validation/validator-packet-complete.mjs`: PASS
- `node .GOV/scripts/validation/spec-debt-registry-check.mjs`: PASS
- `node .GOV/scripts/validation/packet-closure-monitor-check.mjs`: PASS
- legacy-bridge smoke test against `WP-1-Loom-Storage-Portability-v1` refinement: PASS (`legacy_rows=4`, `shared_risk=YES`)
- `just gov-check`: PASS in `wt-orchestrator`

### 14.22 Semantic Proof And Governed Spec-Debt Automation Implemented On March 15, 2026

The next layer of remediation was then implemented to address two remaining weaknesses:

1. The earlier packet-monitoring rollout improved proof discipline, but it still relied too heavily on human judgment to prove semantic correctness.
2. Spec debt had become governed truth at the registry level, but opening/closing/syncing debt was still manual paperwork rather than deterministic workflow.

#### 14.22.1 What Was Added

1. A new semantic-proof layer now exists for future packets.
   - New packet format baseline is now `PACKET_FORMAT_VERSION=2026-03-16`.
   - New refinement baseline is now `REFINEMENT_FORMAT_VERSION=2026-03-16`.
   - New refinements now carry a `SEMANTIC_PROOF_PLAN` section with:
     - `SEMANTIC_TRIPWIRE_TESTS`
     - `CANONICAL_CONTRACT_EXAMPLES`
   - New packets now carry:
     - `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`
     - `## SEMANTIC_PROOF_ASSETS`

2. Clause rows now carry semantic-backstop fields directly.
   - `CLAUSE_CLOSURE_MATRIX` rows now include:
     - `TESTS`
     - `EXAMPLES`
     - `DEBT_IDS`
   - This makes it mechanically visible whether a clause is backed by:
     - executable proof,
     - canonical example/fixture proof,
     - or explicit governed debt.

3. Repo-wide semantic-proof governance now exists.
   - Added `semantic-proof-check`.
   - `just gov-check` now runs it.
   - Active `2026-03-16+` packets fail if they omit `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`.
   - Packets using the semantic profile fail if:
     - `SEMANTIC_PROOF_ASSETS` is missing,
     - shared-surface packets have no semantic tripwire/example,
     - or any clause row points to neither tests, examples, nor debt.

4. Governed spec-debt helper commands now exist.
   - Added:
     - `just spec-debt-open WP-{ID} "<clause>" "<notes>" <YES|NO>`
     - `just spec-debt-sync WP-{ID}`
     - `just spec-debt-close SPECDEBT-...`
   - These helpers now:
     - allocate deterministic `SPECDEBT-*` ids,
     - append/update `.GOV/roles_shared/SPEC_DEBT_REGISTRY.md`,
     - update packet `SPEC_DEBT_STATUS`,
     - and sync clause-row `DEBT_IDS`.

#### 14.22.2 Files Changed

- Templates / packet/refinement flow:
  - `.GOV/templates/REFINEMENT_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/scripts/create-task-packet.mjs`
  - `.GOV/scripts/refinement-brief-lib.mjs`
  - `.GOV/scripts/packet-closure-monitor-lib.mjs`
  - `.GOV/scripts/semantic-proof-lib.mjs`
  - `.GOV/scripts/validation/refinement-check.mjs`
  - `.GOV/scripts/validation/pre-work-check.mjs`
  - `.GOV/scripts/validation/validator-packet-complete.mjs`
  - `.GOV/scripts/validation/semantic-proof-check.mjs`
  - `.GOV/scripts/validation/gov-check.mjs`
  - `.GOV/scripts/session-policy.mjs`
  - `.GOV/scripts/validation/session-policy-check.mjs`

- Spec-debt automation:
  - `.GOV/scripts/spec-debt-registry-lib.mjs`
  - `.GOV/scripts/spec-debt-packet-lib.mjs`
  - `.GOV/scripts/spec-debt-open.mjs`
  - `.GOV/scripts/spec-debt-sync.mjs`
  - `.GOV/scripts/spec-debt-close.mjs`
  - `.GOV/roles_shared/SPEC_DEBT_REGISTRY.md`
  - `justfile`

- Protocol/help surfaces:
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

#### 14.22.3 Technical Issues Found While Landing This Change

1. Version-compatibility regression was introduced and then fixed.
   - Problem:
     - bumping `PACKET_FORMAT_VERSION` to `2026-03-16` caused `session-policy-check` to reject existing `2026-03-15` packets
   - Fix:
     - `session-policy-check` now explicitly allows the historical `2026-03-15` packet generation
   - Assessment:
     - this was a real backward-compatibility bug in the first pass of the governance patch
     - it was caught before the audit update and before any claimed green closeout

2. The first-pass debt helper crashed on old-format packets.
   - Problem:
     - `spec-debt-packet-lib` initially assumed all packets already had `CLAUSE_CLOSURE_MATRIX` and `SPEC_DEBT_STATUS`
     - smoke testing against an older `2026-03-12` packet threw a raw exception
   - Fix:
     - the helper now fails explicitly with a governed compatibility message instead of crashing
   - Important scope rule:
     - the debt automation is intentionally for packets that already opted into clause/debt monitoring
     - it is not a retroactive mutator for historical packet formats

#### 14.22.4 Verification

- `node --check .GOV/scripts/semantic-proof-lib.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-packet-lib.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-open.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-sync.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-close.mjs`: PASS
- `node --check .GOV/scripts/validation/semantic-proof-check.mjs`: PASS
- `node --check .GOV/scripts/create-task-packet.mjs`: PASS
- `node --check .GOV/scripts/packet-closure-monitor-lib.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-registry-lib.mjs`: PASS
- `node --check .GOV/scripts/validation/refinement-check.mjs`: PASS
- `node --check .GOV/scripts/validation/pre-work-check.mjs`: PASS
- `node --check .GOV/scripts/validation/validator-packet-complete.mjs`: PASS
- `node --check .GOV/scripts/validation/session-policy-check.mjs`: PASS
- `node --check .GOV/scripts/validation/gov-check.mjs`: PASS
- `node .GOV/scripts/validation/spec-debt-registry-check.mjs`: PASS
- `node .GOV/scripts/validation/packet-closure-monitor-check.mjs`: PASS
- `node .GOV/scripts/validation/semantic-proof-check.mjs`: PASS
- legacy semantic-proof bridge smoke test against `WP-1-Loom-Storage-Portability-v1` refinement: PASS (`legacy_semantic_tripwires=1`, `legacy_semantic_examples=0`, `legacy_clause_rows=4`)
- synthetic packet debt-format smoke test: PASS (`synthetic_rows=1`, `synthetic_has_debt=true`)
- `just gov-check`: PASS in `wt-orchestrator`

#### 14.22.5 Residual Constraint

This still does not mean governance automatically proves product semantics end to end. What changed is that:

- semantic proof is now a first-class packet object,
- hidden partial proof is harder to narrate as full completion,
- and spec debt can no longer remain loose free-text if the packet is using the new format.

That is a significant correctness improvement without turning the workflow into a universal hard blocker against all imperfect evidence.

### 14.23 Governance Repo Structure Refactor Implemented On March 15, 2026

The governance repo itself had become difficult to navigate. The biggest problems were:

1. Top-level `.GOV/ROLE_MAILBOX/` looked like active shared authority even though it was only a narrow export artifact plus one gate.
2. Role documentation existed, but it was not bundled with the most relevant commands, scripts, and state files for each role.
3. Scripts and checks were all still callable, but the directory layout did not make discovery easy for humans.

#### 14.23.1 What Changed

1. Added governance navigation bundles and then followed through with a compatibility-safe implementation move.
   - New index documents:
     - `.GOV/README.md`
     - `.GOV/roles/README.md`
     - `.GOV/roles_shared/README.md`
     - `.GOV/scripts/README.md`
     - `.GOV/scripts/validation/README.md`
   - New per-role bundle documents:
     - `.GOV/roles/orchestrator/README.md`
     - `.GOV/roles/coder/README.md`
     - `.GOV/roles/validator/README.md`
   - Real shared-library implementation now lives under:
     - `.GOV/scripts/lib/`
     - `.GOV/scripts/debt/`
   - Root-level `.GOV/scripts/*.mjs` files remain as stable operator-facing entrypoints and compatibility wrappers where needed.
   - This groups protocol, rubric, state, key `just` commands, and relevant scripts/checks per role without breaking existing commands.

2. De-authoritized the confusing top-level mailbox export path.
   - New authoritative export location:
     - `.GOV/roles_shared/exports/role_mailbox/`
   - Added:
     - `.GOV/roles_shared/exports/README.md`
     - `.GOV/roles_shared/exports/role_mailbox/README.md`
     - canonical placeholder export files under the new path
   - Updated the live gate:
     - `.GOV/scripts/validation/role_mailbox_export_check.mjs`
       now prefers the new authoritative path and only falls back to `.GOV/ROLE_MAILBOX/` for compatibility.

3. Added a clear deprecation marker to the legacy path.
   - Added `.GOV/ROLE_MAILBOX/README.md`
   - The old folder remains compatibility-only and should not receive new dependencies.

4. Added lightweight discovery commands without changing workflow entrypoints.
   - `just governance-map`
   - `just role-bundle <role>`
   - `just validation-map`
   - Existing manual relay and orchestrator-managed commands remain unchanged.

5. Updated active governance docs/invariants to point to the new authority.
   - `.GOV/roles_shared/PROJECT_INVARIANTS.md`
   - `.GOV/roles_shared/EVIDENCE_LEDGER.md`
   - `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`
   - `.gitattributes`

#### 14.23.2 Important Non-Change

I did **not** do a risky command-surface reshuffle.

- I did not rename current `just` recipes that manual relay or orchestrator-managed workflow already depends on.
- I did not relocate validation entrypoints out of `.GOV/scripts/validation/`.
- I did not hard-delete `.GOV/ROLE_MAILBOX/`.

Reason:

- renaming command entrypoints would create avoidable workflow breakage
- hard-deleting the legacy folder would be a destructive cleanup action on a non-temp governed path
- the safer path was to move library/debt implementation first, preserve stable entrypoints, and de-authoritize the legacy mailbox path without deleting it

#### 14.23.3 Verification

- `node --check .GOV/scripts/validation/role_mailbox_export_check.mjs`: PASS
- `just role-mailbox-export-check`: PASS
- `just governance-map`: PASS
- `just role-bundle orchestrator`: PASS
- `just role-bundle validator`: PASS
- `just validation-map`: PASS
- `just orchestrator-next`: PASS
- `just coder-next`: PASS
- `just validator-next`: PASS
- `just gov-check`: PASS

#### 14.23.4 Operator-Facing Outcome

The governance repo is now easier to scan without breaking current workflow law:

- top-level navigation is explicit
- each role has a local bundle view
- shared truth is grouped and documented
- validation scripts remain grouped under `.GOV/scripts/validation/`
- shared implementation is physically bundled under `.GOV/scripts/lib/` and `.GOV/scripts/debt/`
- the misleading top-level mailbox export folder is no longer the active authority

If the Operator later wants the legacy `.GOV/ROLE_MAILBOX/` folder physically deleted, that can now be done as a separate explicit cleanup step with much lower risk because the live gate no longer depends on it.

#### 14.23.5 Second-Pass Structural Correction

After the first governance-repo cleanup pass, the structure was still too cosmetic. The repo map and bundle READMEs existed, but the live implementation was still effectively concentrated in the root `.GOV/scripts/` directory. That meant the Operator’s criticism was correct: the first pass improved labeling more than structure.

I then performed a compatibility-safe physical rebundling:

- moved shared implementation libraries into:
  - `.GOV/scripts/lib/`
- moved governed spec-debt command implementation into:
  - `.GOV/scripts/debt/`
- restored the original root paths as thin compatibility wrappers so existing:
  - `just ...` recipes
  - manual `node .GOV/scripts/...`
  - role workflow entrypoints
  continue to function

This is a real structural change, not just documentation:

- shared library bodies are no longer living only at the root script layer
- debt helper command bodies are no longer living only at the root script layer
- role documentation now points people at the bundle directories, not only the legacy top-level entrypoints

Residual limitation:

- the root `.GOV/scripts/` directory is still visually dense because compatibility wrappers remain in place
- this is intentional for workflow stability
- a future deeper cleanup would require a governed path migration plan, not just file moves

Verification after the second-pass structural move:

- `node --check .GOV/scripts/refinement-brief-lib.mjs`: PASS
- `node --check .GOV/scripts/packet-closure-monitor-lib.mjs`: PASS
- `node --check .GOV/scripts/semantic-proof-lib.mjs`: PASS
- `node --check .GOV/scripts/role-resume-utils.mjs`: PASS
- `node --check .GOV/scripts/wp-communications-lib.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-open.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-close.mjs`: PASS
- `node --check .GOV/scripts/spec-debt-sync.mjs`: PASS
- `just role-mailbox-export-check`: PASS
- `just governance-map`: PASS
- `just role-bundle coder`: PASS
- `just orchestrator-next`: PASS
- `just coder-next`: PASS
- `just validator-next`: PASS
- `just gov-check`: PASS

#### 14.23.6 Third-Pass Repo Structure Correction

The second pass still left one major active role artifact in the top-level `.GOV` namespace:

- validator gate state was still materially stored at `.GOV/validator_gates/{WP_ID}.json`

That meant the repo still looked flatter and more confusing than it really was. The Validator owned the workflow state, but the state directory itself still lived outside the Validator bundle.

This third pass corrected that physically instead of only documenting it.

What actually moved:

- Authoritative validator gate state moved from:
  - `.GOV/validator_gates/{WP_ID}.json`
- To:
  - `.GOV/roles/validator/gates/{WP_ID}.json`

What was added to make that lawful and compatibility-safe:

- New role-owned helper:
  - `.GOV/roles/validator/scripts/validator-gate-paths.mjs`
- Updated live consumers:
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/checks/validator-handoff-check.mjs`
  - `.GOV/roles/validator/checks/external-validator-brief.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles/coder/checks/post-work-check.mjs`
  - `.GOV/roles_shared/scripts/governance-snapshot.mjs`

Compatibility policy after this move:

- new writes go to `.GOV/roles/validator/gates/`
- legacy reads may still fall back to `.GOV/validator_gates/`
- the top-level `.GOV/validator_gates/` folder is now explicitly compatibility-only

Governance documentation updated to match:

- `.GOV/README.md`
- `.GOV/roles/validator/README.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/EVIDENCE_LEDGER.md`
- `.GOV/roles_shared/PROJECT_INVARIANTS.md`
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`
- `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`

Additional truth sync performed:

- regenerated `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json` so the shared snapshot now points at `.GOV/roles/validator/gates/...` instead of the old root path

Why this pass matters more than the earlier ones:

- this is a real ownership move of an active runtime artifact, not just a README or wrapper shuffle
- the Validator role now owns its gate state in its own folder
- the Operator monitor, validator commands, coder post-work logic, and governance snapshot all follow the same new authority path

Residual limits after this pass:

- `.GOV/validator_gates/` still exists as a compatibility directory because hard deletion would be destructive cleanup on a governed non-temp path
- `.GOV/ROLE_MAILBOX/` also still exists as compatibility-only for the same reason
- root `.GOV/scripts/` still contains legacy directory shells and compatibility entrypoints for workflow stability

Verification after the third-pass structural correction:

- `just governance-snapshot`: PASS
- `just role-mailbox-export-check`: PASS
- `just orchestrator-next`: PASS
- `just coder-next`: PASS
- `just validator-next`: PASS
- `just gov-check`: PASS

Final judgment on the repo-structure complaint:

- the original criticism was correct
- the earlier cleanup passes improved navigability but did not move enough active authority
- this pass materially improves structure because one of the most important Validator-owned workflow artifacts is now physically inside the Validator role bundle

#### 14.23.7 Validator Gates Focus Pass

I then did a focused validator-gates pass to make this migration usable as an operator-facing starting point rather than only a file move.

Additional fixes in this pass:

- verified the live gate commands still work against the migrated state:
  - `just validator-gate-status WP-1-Spec-Appendices-Backfill-v1`
- fixed terminal-facing mojibake in the gate status output by replacing the broken glyph markers with ASCII-safe markers in:
  - `.GOV/roles/validator/checks/validator_gates.mjs`

Why this matters:

- the validator-gate surface is one of the few governance surfaces operators interact with directly during live workflow
- if the output itself is noisy or broken, the structural improvement is harder to trust
- the ASCII-safe output now behaves correctly on the Windows terminals used in this repo

Focused verification:

- `node --check .GOV/roles/validator/checks/validator_gates.mjs`: PASS
- `just validator-gate-status WP-1-Spec-Appendices-Backfill-v1`: PASS
- `just gov-check`: PASS

Correction after this pass:

- I briefly added a dedicated `validator-gate-path-check` into `gov-check`.
- That was the wrong tradeoff for this refactor stage.
- It added governance friction without materially improving the validator-gate ownership move itself.
- I removed that extra blocker and kept the structural cutover only.

#### 14.23.8 Validator Gates Ownership Correction And Shared-State Cutover

This section supersedes the placement conclusion in `14.23.6` and `14.23.7`.

The earlier move into `.GOV/roles/validator/gates/` solved only the file-location complaint. It did not solve the ownership problem correctly.

What the follow-up inspection proved:

- Validator gate code is Validator-owned:
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/checks/validator-handoff-check.mjs`
  - `.GOV/roles/validator/checks/external-validator-brief.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
- Validator gate JSON state is shared workflow/runtime state, not Validator-private state.
- Reason:
  - Validator writes the files.
  - Orchestrator reads them in the operator monitor.
  - shared governance snapshot ingests them.
  - coder-side governance logic allowlists them.
  - committed validation evidence stored inside them contains PREPARE worktree and branch truth sourced from Orchestrator state.

Corrected placement after the ownership review:

- shared gate state moved from:
  - `.GOV/roles/validator/gates/{WP_ID}.json`
- to:
  - `.GOV/roles_shared/validator_gates/{WP_ID}.json`
- shared path helper moved from the Validator bundle into shared implementation:
  - `.GOV/roles_shared/scripts/lib/validator-gate-paths.mjs`

Consumers patched to the shared authority path:

- `.GOV/roles/validator/checks/validator_gates.mjs`
- `.GOV/roles/validator/checks/validator-handoff-check.mjs`
- `.GOV/roles/validator/checks/external-validator-brief.mjs`
- `.GOV/roles/validator/scripts/validator-next.mjs`
- `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
- `.GOV/roles_shared/scripts/governance-snapshot.mjs`
- `.GOV/roles/coder/checks/post-work-check.mjs`

Documentation and invariant surfaces updated to match the corrected ownership model:

- `.GOV/README.md`
- `.GOV/roles_shared/README.md`
- `.GOV/roles/validator/README.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/PROJECT_INVARIANTS.md`
- `.GOV/roles_shared/EVIDENCE_LEDGER.md`
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`
- `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`

Important correction on shells / aliases:

- no new compatibility shell was kept for validator gates
- `.GOV/roles/validator/gates/` is no longer the authoritative path and is no longer present on disk in the active worktree
- the live path is only `.GOV/roles_shared/validator_gates/`

Live cutover defect found during this pass:

- I initially patched several imports to `../../roles_shared/...`
- that was one directory too shallow from `roles/validator/*` and `roles/orchestrator/*`
- result:
  - `just validator-gate-status ...` failed
  - `just validator-next` failed
- this was caught immediately by rerunning the live `just` entrypoints instead of trusting `node --check`
- fix:
  - corrected those imports to `../../../roles_shared/...`

Why this pass is materially better:

- role-owned logic stays under `roles/validator/`
- shared runtime state now lives under `roles_shared/`
- Orchestrator, Validator, shared snapshot generation, and coder-side governance now all read the same shared path
- the repo structure now matches real usage rather than only file-writer ownership

Verification after the ownership-correct cutover:

- `just governance-snapshot`: PASS
- `node --check .GOV/roles/validator/checks/validator_gates.mjs`: PASS
- `node --check .GOV/roles/validator/checks/validator-handoff-check.mjs`: PASS
- `node --check .GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`: PASS
- `node --check .GOV/roles_shared/scripts/governance-snapshot.mjs`: PASS
- `just validator-gate-status WP-1-Spec-Appendices-Backfill-v1`: PASS
- `just validator-next`: PASS
- `just orchestrator-next`: PASS
- `just coder-next`: PASS
- `just gov-check`: PASS

Final judgment for validator gates after the ownership review:

- the previous “put it in the Validator role folder” answer was wrong
- the right split is:
  - Validator code in `roles/validator/`
  - shared gate state in `roles_shared/`
- this corrected cutover removes one more misleading governance surface and improves the live manual/orchestrator-managed workflow instead of only tidying documentation

#### 14.23.9 Root Scripts Retirement And Hook Cutover

After the validator-gates correction, I reviewed the remaining root `.GOV/scripts/` surface.

Actual on-disk reality at that point:

- root `.GOV/scripts/` no longer contained active implementation code
- only three tracked files remained:
  - `.GOV/scripts/README.md`
  - `.GOV/scripts/validation/README.md`
  - `.GOV/scripts/hooks/pre-commit`

That meant the root scripts folder had effectively become a confusing compatibility carcass rather than a real runtime surface.

Corrected ownership decision:

- the pre-commit hook is shared repo workflow tooling
- it therefore belongs under shared implementation, not under a root compatibility folder

What changed:

- moved:
  - `.GOV/scripts/hooks/pre-commit`
- to:
  - `.GOV/roles_shared/scripts/hooks/pre-commit`
- added:
  - `.GOV/roles_shared/scripts/hooks/README.md`
- removed tracked root compatibility files:
  - `.GOV/scripts/README.md`
  - `.GOV/scripts/validation/README.md`

Live authority docs updated to the new hook location:

- `.GOV/README.md`
- `.GOV/roles_shared/scripts/README.md`
- `.GOV/roles_shared/START_HERE.md`
- `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
- `.GOV/roles_shared/ARCHITECTURE.md`
- `.GOV/roles_shared/BOUNDARY_RULES.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`
- `.GOV/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`
- `.GOV/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`
- `.GOV/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`
- `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
- `.GOV/roles_shared/workflow_technical_paper.md`

What I intentionally did not rewrite:

- historical task packets
- historical refinements
- older audit evidence

Reason:

- those documents are evidence records and may legitimately reference the path that existed at the time
- rewriting them would blur history instead of clarifying the active repo structure

Operational note:

- `core.hooksPath` was not set in the active worktree during this cutover
- so there was no live local hook-path break to repair in this repo checkout
- if any external/local clone had the old hook path configured, it would need:
  - `git config core.hooksPath .GOV/roles_shared/scripts/hooks`

Verification after root scripts retirement:

- `bash -n .GOV/roles_shared/scripts/hooks/pre-commit`: PASS
- `just gov-check`: PASS
- `just orchestrator-next`: PASS
- `just coder-next`: PASS
- `just validator-next`: PASS
- `Get-ChildItem .GOV/scripts -Recurse -File`: no tracked runtime files remain

Final judgment on root `.GOV/scripts/`:

- yes, it could be treated the same way as validator gates
- the correct outcome was not “leave a labeled shell”
- the correct outcome was:
  - move the only live remaining runtime artifact to `roles_shared/scripts/hooks/`
  - delete the tracked root-script remnants
  - update current authority docs only

Residual limitation after this pass:

- historical evidence still contains old `.GOV/scripts/...` references by design
- those references are now historical only, not active workflow authority

#### 14.23.10 Codex / Protocol Disk-Agnostic Health Check (2026-03-15)

User-directed objective for this pass:

- determine whether `Handshake Codex v1.4.md` and role/shared protocol files still needed migration updates
- make the active governance surface disk-agnostic where possible
- run a governance-repo health check using an explicit sequence:
  - devise plan
  - let agents scrutinize the plan / current state
  - update plan
  - execute fixes
  - use agents again for final checkup

Execution model used:

- local baseline + live `just gov-check`
- agent-assisted review for:
  - active Codex / protocol / shared-role drift
  - governance health-check perimeter gaps
- local fix pass
- local re-verification
- bounded sub-agent recheck

Important process note:

- broad explorer passes were too slow/noisy for this repo surface
- I narrowed the final agent work to file-bounded review questions so the checkup produced useful answers
- one sub-agent returned no remaining live Codex/protocol inconsistency after the local fix pass
- another sub-agent found real health-perimeter gaps, which I then verified locally before patching

What actually still needed updating:

1. Live Codex / protocol / session-control drift

- `Handshake Codex v1.4.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles_shared/START_HERE.md`
- `.GOV/roles_shared/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/roles_shared/ROLE_WORKTREES.md`
- `.GOV/docs/vscode-session-bridge/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
- `.GOV/tools/handshake-acp-bridge/README.md`

Main corrections applied across those surfaces:

- authoritative governance implementation paths were updated from retired root `.GOV/scripts/` assumptions to:
  - role-owned: `.GOV/roles/<role>/{scripts,checks}/`
  - shared: `.GOV/roles_shared/{scripts,checks}/`
- launch-host wording was normalized to `SYSTEM_TERMINAL`
- `WINDOWS_TERMINAL` was retained only as legacy compatibility, not current guidance
- role-mailbox authority remained:
  - live: `.GOV/roles_shared/exports/role_mailbox/`
  - old top-level path: compatibility-only
- drive/path examples were made more disk-agnostic

2. Real live-doc/operator drift found during the pass

- `.GOV/roles_shared/ROLE_WORKFLOW_QUICKREF.md` still advertised:
  - `WINDOWS_TERMINAL`
  - a mixed Windows-shaped root example
- fix:
  - launch examples now use `SYSTEM_TERMINAL`
  - root example now uses `/workspace/handshake`
- blind-spot fix paired with it:
  - `.GOV/roles_shared/checks/drive-agnostic-check.mjs`
  - `.GOV/roles_shared/checks/migration-path-truth-check.mjs`
  now include `ROLE_WORKFLOW_QUICKREF.md` in the active enforcement perimeter

3. Agent-assisted health-check findings that were real

- CI governance blind spot:
  - CI did not run `just gov-check`
  - meaning local governance hardening could still drift without remote enforcement
- fix:
  - added dedicated `governance` CI job in `.github/workflows/ci.yml`
  - matrix:
    - `ubuntu-latest`
    - `windows-latest`
  - installs:
    - Node
    - `just`
    - `ripgrep`
  - runs:
    - `just gov-check`

- Windows/disk-path blind spot:
  - `drive-agnostic-check` originally only flagged:
    - uppercase drive-letter paths
    - UNC paths
  - that left room for:
    - lowercase drive paths
    - common host absolute POSIX roots
- fix:
  - `drive-agnostic-check` now detects:
    - Windows backslash drive paths
    - multi-segment slash-style drive paths
    - UNC paths
    - common absolute host POSIX roots such as `/mnt/...`, `/home/...`, `/Users/...`, `/Volumes/...`, `/workspace/...`, `/tmp/...`
  - existing explicit `example` allowlist remains in place to avoid blocking legitimate illustrative examples

4. Agent finding that looked plausible but was rejected after local verification

- one agent suggested startup/preflight was still only using the echo-style `hard-gate-wt-001`
- local verification showed that was stale
- current `justfile` wiring already runs:
  - `just gov-check`
  - `just validator-spec-regression`
  after `hard-gate-wt-001` in:
  - `orchestrator-preflight`
  - `validator-preflight`
  - `coder-preflight`
- no patch was applied there because it would have been redundant theater rather than real hardening

Additional implementation detail:

- while strengthening `drive-agnostic-check`, the checker correctly failed on its own explanatory comment
- that self-hit was fixed immediately by rewriting the comment so the matcher remained strict without producing self-noise

Verification after the disk-agnostic / health-check pass:

- `node --check .GOV/roles_shared/checks/drive-agnostic-check.mjs`: PASS
- `node --check .GOV/roles_shared/checks/migration-path-truth-check.mjs`: PASS
- `node .GOV/roles_shared/checks/drive-agnostic-check.mjs`: PASS
- `node .GOV/roles_shared/checks/migration-path-truth-check.mjs`: PASS
- `just gov-check`: PASS

Agent-assisted final checkup outcome:

- active Codex / protocol / shared-role surface:
  - no remaining live inconsistency found after the fix pass
- health-perimeter review:
  - CI governance coverage and disk-path matcher were the meaningful real gaps
  - both were addressed in this pass

Residual risk after this pass:

- runtime ledgers and historical evidence still contain older tokens and older paths, including `WINDOWS_TERMINAL` and retired root `.GOV/scripts/...` references
- that drift remains intentionally excluded from active-governance path checks because those files are evidence/runtime state, not current authority
- the new CI governance job was added but not executed locally in this audit pass; its first true proof will be the next GitHub Actions run
- large historical/reference docs outside the active authority perimeter may still contain old path language by design

Final judgment for this pass:

- yes, Codex and role/shared protocol files did still need updates
- yes, the active governance surface is materially more disk-agnostic now
- yes, the governance-repo health check uncovered real enforcement gaps
- those gaps were fixed in live authority surfaces and local governance verification now passes without touching product code

## 14.19 Governance Repo Structure Hardening (2026-03-15)

This follow-up pass moved from "better labels" to actual governance-structure law:

1. Explicit folder law for `roles/` and `roles_shared/`

- Added governed folder-law docs:
  - `.GOV/roles/STRUCTURE_RULES.md`
  - `.GOV/roles_shared/STRUCTURE_RULES.md`
- Made the landing zones concrete, not just described in prose:
  - role-local:
    - `roles/<role>/docs/`
    - `roles/<role>/scripts/`
    - `roles/<role>/scripts/lib/`
    - `roles/<role>/checks/`
    - `roles/<role>/tests/`
    - `roles/<role>/fixtures/`
  - shared:
    - `roles_shared/scripts/`
    - `roles_shared/checks/`
    - `roles_shared/tests/`
    - `roles_shared/fixtures/`
- Added README landing files to the new test/fixture/helper-lib folders so future governance files have an unambiguous home even before the first real test or fixture is added.
- Updated the role bundle docs so `orchestrator`, `coder`, and `validator` now describe their canonical internal layout instead of only listing a handful of files.

2. Stronger split between active authority and reference material

- Moved non-authoritative study material out of active role/shared roots:
  - role-local roadmaps/rubrics/gap analyses moved under role `docs/`
  - master-spec studies and workflow paper moved under `.GOV/reference/`
- Strengthened the reference boundary further:
  - moved the non-normative Handshake kernel implementation map from `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md` to `.GOV/reference/kernel/REFERENCE_IMPLEMENTATION_HANDSHAKE.md`
  - moved archaeology index `PAST_WORK_INDEX.md` from `.GOV/roles_shared/` to `.GOV/reference/`
- Patched active authority surfaces to point at the new reference locations:
  - `.GOV/GOV_KERNEL/README.md`
  - `.GOV/roles_shared/START_HERE.md`
  - `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
  - `.GOV/roles_shared/checks/spec-governance-reference-check.mjs`
  - `justfile`
  - `Handshake Codex v1.4.md`
  - `.GOV/task_packets/WP-1-Governance-Kernel-Conformance-v1.md`
  - `.GOV/task_packets/stubs/WP-1-Governance-Kernel-Conformance-v1.md`
- Tightened live boundary wording:
  - `Handshake Codex v1.4.md` now treats archaeology/reference as `/.GOV/reference/` material
  - `.GOV/roles_shared/BOUNDARY_RULES.md` no longer blesses "rubrics at role root" as part of the active governance workspace
  - `.GOV/roles_shared/README.md` no longer presents `PAST_WORK_INDEX.md` as shared live truth

3. Deprecation sunset plan converted from vague intent to governed policy

- Added/strengthened `.GOV/roles_shared/DEPRECATION_SUNSET_PLAN.md`
- The plan now records at least:
  - `.GOV/ROLE_MAILBOX/`
  - `WINDOWS_TERMINAL`
  - `.GOV/roles/validator/VALIDATOR_GATES.json`
- Added enforcement:
  - `.GOV/roles_shared/checks/deprecation-sunset-check.mjs`
- The checker now:
  - requires the baseline legacy entries
  - conditionally requires a sunset-plan entry for `VALIDATOR_GATES.json` when that legacy archive still exists on disk
- Updated validator bundle docs so `VALIDATOR_GATES.json` is no longer presented as a primary active validator surface; it is explicitly called a legacy archive.

4. Agent-assisted scrutiny and additional corrections

- A sidecar review found that active law still overstated some helper/reference surfaces.
- Real issues found and corrected in this pass:
  - Codex still implied active archaeology under `roles_shared`; fixed by moving `PAST_WORK_INDEX.md` to `.GOV/reference/`
  - Codex still made all `agentic/AGENTIC_PROTOCOL.md` files sound automatically active; fixed so agentic add-ons are active only when the corresponding role protocol explicitly says so
  - `BOUNDARY_RULES.md` still treated role-root rubrics as if they were part of the active authority root; fixed
- The sidecar also highlighted broader helper/reference drift in `roles_shared/`; that remains a continuing cleanup area, but the most obvious authority contradictions were corrected here.

5. Governance-repo health check results

Commands run after the structure/deprecation/reference refactor:

- `node --check .GOV/roles_shared/checks/deprecation-sunset-check.mjs`
- `node .GOV/roles_shared/checks/deprecation-sunset-check.mjs`
- `node --check .GOV/roles_shared/checks/spec-governance-reference-check.mjs`
- `just build-order-sync`
- `just gov-check`
- `git diff --check`
- `just orchestrator-next`
- `just coder-next`
- `just validator-next`
- `just role-mailbox-export-check`

Results:

- `just gov-check`: PASS
- `git diff --check`: PASS (only non-blocking CRLF warnings from unrelated tracked files)
- `just role-mailbox-export-check`: PASS
- `just orchestrator-next`: PASS as a command surface
- `just coder-next`: PASS as a command surface
- `just validator-next`: PASS as a command surface

6. Additional hygiene fixes discovered during health check

- Fixed trailing whitespace in `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
- Fixed trailing blank lines / EOF hygiene in:
  - `justfile`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`

7. Residual concerns after this pass

- The command surfaces are healthy, but resume-style role commands still surface stale historical WP state for `WP-1-Spec-Appendices-Backfill-v1` (for example a stale `SPEC_TARGET_RESOLVED` mismatch and stale bootstrap/skeleton expectations). This did not break the commands, but it is still governance noise in the live operator experience.
- Runtime/session ledgers still contain old paths/tokens by design and were not normalized in this pass.
- `WINDOWS_TERMINAL`, `.GOV/ROLE_MAILBOX/`, and `VALIDATOR_GATES.json` remain compatibility surfaces until their sunset triggers are actually completed.
- This pass improved repo structure, authority separation, and deprecation discipline; it did not attempt to migrate every remaining helper/reference document out of `roles_shared/`.

Final judgment for this pass:

- folder law is materially clearer and now backed by real directories plus governed README landing points
- active authority vs reference material is more truthful than before
- the deprecation story is now explicit and mechanically checked instead of indefinite prose
- the live manual/orchestrator-managed governance command surface still works after the refactor
- no product code under `src/`, `app/`, or `tests/` was touched

## 14.20 Governance Structure Audit Command (2026-03-15)

I added a machine-readable structure target plus a report-first audit command so governance placement drift is no longer a manual judgment call.

Implemented surfaces:

- `/.GOV/docs/GOVERNANCE_STRUCTURE_TARGET.md`
- `/.GOV/roles_shared/checks/governance-structure-rules.mjs`
- `/.GOV/roles_shared/checks/governance-structure-check.mjs`
- `just governance-structure-audit`
- `just governance-structure-check`

Intent:

- `governance-structure-audit` is report-only and lists current hotspots with suggested destinations.
- `governance-structure-check` is the future strict mode for the end-state once those hotspots are migrated.

Current hotspot classes covered:

- root-level `operator/`, `Papers/`, and `agents/`
- overloaded `roles_shared/` root files that should live under `docs/`, `records/`, `runtime/`, or `templates/`
- role-root state/archive files that should move under `runtime/` or `reference/legacy/`
- stray `.GOV/docs/memory dump.md`

This keeps the remaining governance refactor concrete: the repo can now print the misplaced surfaces directly instead of relying on ad hoc review.

Correction after rollout:

- `/.GOV/operator/` was initially flagged by the structure audit.
- The Operator clarified that `operator/` is operator-private workspace material and not part of the repo-governance migration scope.
- I corrected the target document and the machine-readable audit rules so `operator/` is now excluded unless explicitly requested.
