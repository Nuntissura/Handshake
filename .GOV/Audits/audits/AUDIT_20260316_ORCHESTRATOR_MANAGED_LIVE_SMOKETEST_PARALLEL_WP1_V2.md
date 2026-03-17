# Audit: Orchestrator-Managed Live Parallel Smoke Test for WP-1 Remediation v2

## METADATA
- AUDIT_ID: AUDIT-20260316-ORCH-MANAGED-LIVE-SMOKETEST-PARALLEL-WP1-V2
- DATE_UTC: 2026-03-16
- AUDITOR: Codex acting as Orchestrator
- SCOPE: Live orchestrator-managed parallel smoke test for `WP-1-Structured-Collaboration-Schema-Registry-v2` and `WP-1-Loom-Storage-Portability-v2`
- RESULT: IN_PROGRESS WITH VERIFIED GOVERNANCE FAILURES, TOOLING FRICTION, AND STRUCTURAL ACP RUNTIME DEFECTS
- ACTIVE_SESSIONS_IN_SCOPE:
  - `CODER:WP-1-Structured-Collaboration-Schema-Registry-v2`
  - `CODER:WP-1-Loom-Storage-Portability-v2`
  - `WP_VALIDATOR:WP-1-Structured-Collaboration-Schema-Registry-v2`
  - `WP_VALIDATOR:WP-1-Loom-Storage-Portability-v2`
- NON-ACTIVE_ROLE_NOTES:
  - `INTEGRATION_VALIDATOR` was not yet active for the v2 remediation run and is therefore out of scope for performance judgment in this audit.
- EVIDENCE_SOURCES:
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Structured-Collaboration-Schema-Registry-v2/044bf061-e53b-409a-8c6a-5a720b44b02f.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Loom-Storage-Portability-v2/7b88448c-2bf8-46fb-ab9d-698a58577ea8.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Structured-Collaboration-Schema-Registry-v2/d7b00021-cb69-40df-a489-251920eb7418.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Loom-Storage-Portability-v2/4c1441e4-ee5b-44f3-a5d1-eaea37646ed3.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/SESSION_CONTROL_OUTPUTS/WP_VALIDATOR_WP-1-Loom-Storage-Portability-v2/96681a3a-9af0-43b6-aba8-99d39e6bff7f.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v2/THREAD.md`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Schema-Registry-v2/RECEIPTS.jsonl`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v2/THREAD.md`
  - `D:/Projects/LLM projects/Handshake/Handshake Runtime/repo-governance/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v2/RECEIPTS.jsonl`

---

## 1. EXECUTIVE SUMMARY

This v2 smoke test successfully launched four governed sessions in parallel on the intended role split:

- two Coders on separate WP worktrees
- two WP Validators on separate validator worktrees
- Orchestrator steering through the ACP/session-control stack

The smoke did not stay clean.

Three governance/runtime defects surfaced immediately enough to affect the live run:

1. historical ACP rows still pointed at the retired repo-local runtime tree and broke `validator-startup`
2. WP communication helpers serialized missing optional fields as the literal string `false`
3. ACP steering commands can time out at the shell layer even while the broker has accepted and is executing the request

Those defects were real workflow faults, not reporting noise. They required in-place governance patching during the smoke.

On the role-performance side:

- Schema coder is doing meaningful current-main remediation work and has already identified live task-board/schema drift.
- Loom coder recovered from a wrong packet test command and is still in the audit/remediation turn.
- Schema WP Validator behaved correctly once started, but its first standby artifacts exposed the nullable-field serialization bug.
- Loom WP Validator initially hit a systemic startup failure unrelated to its own WP because the runtime checker rejected stale historical session-control rows.

Overall judgment:

- session launch and multi-lane isolation: proven
- steady-state ACP/operator ergonomics: not proven
- validator readiness and communication hygiene: partially proven after hotfix
- tooling quality for a no-drama live smoke: not yet sufficient
- authoritative packet/task-board truth maintenance: failed
- coder <-> WP-validator direct review traffic: failed before explicit Orchestrator intervention

Post-audit correction:

- both `v2` remediation WPs were genuinely started
- the authoritative packet/task-board surfaces were not reconciled to match runtime truth
- this audit must therefore be read as a live run with stale governance state, not as a run that never advanced beyond `READY_FOR_DEV`
- later in the same live run, packet/task-board truth was repaired in `wt-orchestrator` and then synced into the active WP worktrees because governed sessions were still reading stale local packet copies
- later in the same live run, direct coder <-> WP-validator review traffic was successfully activated for both WPs after explicit Orchestrator repair steering

---

## 2. CURRENT RUN STATUS

At the audit cut:

- `CODER:WP-1-Structured-Collaboration-Schema-Registry-v2` had started, recorded a bootstrap claim in its worktree, and still held active uncommitted product-code changes in the assigned coder lane.
- `CODER:WP-1-Loom-Storage-Portability-v2` had started, reached a skeleton checkpoint, and recorded the docs-only checkpoint commit `930ee53`.
- both WP validator sessions had been normalized into explicit standby state:
  - `runtime_status=input_required`
  - `current_phase=STATUS_SYNC`
  - `next_expected_actor=ORCHESTRATOR`
  - `waiting_on="coder handoff or validator trigger"`
- no `VALIDATION_REPORTS` closeout had been appended yet for either v2 WP.
- the authoritative task board and task packets still incorrectly showed both WPs as `READY_FOR_DEV` despite live runtime/session evidence proving otherwise.
- no direct coder <-> WP-validator review receipts existed; the communication surface remained hub-and-spoke through the Orchestrator even though structured review helpers already existed.

This means the smoke was live and functioning, but not yet at a coder-handoff or validator-verdict boundary, and the canonical governance surfaces were already behind reality.

---

## 3. FAILURES BY ROLE

### 3.1 Orchestrator

#### Failures

1. I discovered too late that governed commands execute against each worktree's local `.GOV`, not against a central orchestrator-owned governance runtime.
   - Effect: fixing the runtime checker only in `wt-orchestrator` was insufficient; the same patch had to be copied into the active coder and validator worktrees before their local `just` commands stopped failing.

2. I had to diagnose live state from registry and output logs because shell-level `just steer-*` calls can time out while the broker has actually accepted the request.
   - Effect: terminal exit status alone is not trustworthy as the session-control source of truth.

3. I had to repair validator startup mid-smoke instead of relying on pre-existing `gov-check` / startup hard-gates to keep the run clean.
   - Effect: the smoke remained valid, but operator experience was degraded and required manual ACP triage.

4. I did not keep the authoritative packet/task-board state synchronized with the live ACP/runtime state.
   - Effect: the repo later appeared to say both WPs never started even though runtime/session evidence and the WP worktrees showed that they had.

5. I allowed the live run to proceed without forcing direct coder <-> WP-validator review traffic.
   - Effect: the collaboration model regressed into hub-and-spoke messaging through the Orchestrator, which weakens early technical correction and underuses the governed review channel.

6. I allowed authoritative packet/task-board truth in `wt-orchestrator` to drift away from the packet copies inside the active WP worktrees.
   - Effect: runtime status projections in the live lanes continued to report `current_packet_status=Ready for Dev` even after the canonical packet/task-board surfaces were repaired to `In Progress`.

7. I did not catch the review-helper semantic mismatch before using it in the live run.
   - Effect: `wp-validator-query` behavior and naming were ambiguous enough that the first repair attempts produced mis-targeted or misleading review receipts before the helper contract was corrected.

#### What Worked

- session launch across four lanes succeeded
- the standby posture for both validators was eventually made symmetric
- runtime evidence was sufficient to recover truth after shell-level timeouts

### 3.2 Coder A: Schema Registry v2

#### Failures / Friction

1. The governed turn is long-running enough that shell-level observation is poor; the active result still shows `RUNNING` while useful work is happening in the output log.
   - This is more ACP/tooling friction than coder error, but it affects coder supervision directly.

2. The workflow still leaves the validator parked with no structured handoff trigger while the coder is actively making progress.
   - That creates a lag between active coding and active technical review.

#### Product/Implementation Signals

- The coder correctly identified the live task-board contract drift on current `main`.
- The coder traced the current structured artifact emission path before editing.
- The coder moved into formatting and targeted crate-local testing, which is the right direction for this WP.

### 3.3 Coder B: Loom v2

#### Failures / Friction

1. The packet-directed test command was wrong for the repo layout.
   - The run attempted a repo-root Cargo invocation equivalent to `cargo test -p handshake_core loom`.
   - In this repo, that fails because there is no `Cargo.toml` at repo root for that command path.
   - The coder had to discover and shift to crate-local invocation under `src/backend/handshake_core`.

2. The coder spent early turn budget re-auditing packet and repo layout instead of being able to execute a clean first test plan.
   - That is tooling/packet-law waste in a smoke that is supposed to validate fast parallel startup.

#### Positive Signals

- The coder detected the bad test command rather than cargo-culting it.
- The coder recovered by locating the correct manifest path and continuing from there.

### 3.4 WP Validator A: Schema Registry v2

#### Failures / Friction

1. Its first standby thread/receipt artifacts leaked null optional fields as literal `"false"` strings.
   - Evidence:
     - `THREAD.md` recorded `target_session=false | correlation_id=false`
     - `RECEIPTS.jsonl` recorded `"target_session":"false","correlation_id":"false"`
   - This is a data-shape defect in governed communication tooling.

2. The validator was forced into standby because the current workflow still lacks an earlier automatic handoff/trigger phase between live coder work and validator engagement.

#### Positive Signals

- The validator correctly validated worktree identity and packet context before writing anything.
- It did not invent a validation report without a legitimate handoff target.

### 3.5 WP Validator B: Loom v2

#### Failures / Friction

1. `just validator-startup` initially failed for reasons unrelated to Loom v2 itself.
   - The failure came from the session-control runtime check tripping over stale historical rows from older sessions and legacy output-log paths.

2. The first validator response was effectively "WP not ready and validator startup is noisy."
   - That is useful truth, but it shows the validator lane was contaminated by cross-WP historical runtime state.

#### Positive Signals

- After the runtime fix, the validator resumed normally and mirrored the intended standby posture without touching validation-report sections prematurely.

---

## 4. SYSTEMIC FAILURES

### 4.1 Legacy Runtime Path Drift Broke Fresh Validator Startup

The externalized runtime move was incomplete from the perspective of historical ACP evidence. Old session rows still referenced repo-local output logs under `.GOV/roles_shared/runtime/SESSION_CONTROL_OUTPUTS/...`.

`session-control-runtime-check.mjs` treated those old paths as if they had to exist relative to the current worktree, so a fresh validator worktree could fail `validator-startup` because of an unrelated historical row from another WP.

Impact:

- cross-WP contamination of startup checks
- validator lanes blocked before doing any useful work
- live smoke required a governance patch in the middle of execution

### 4.2 Nullable WP Communication Fields Were Not Actually Nullable

`wp-thread-append` / `wp-receipt-append` treated certain CLI arguments as truthy strings and serialized missing optional fields as `"false"`.

Impact:

- bad thread readability
- bad receipt schema hygiene
- downstream ambiguity for structured consumers

### 4.3 ACP Shell Wrapper Semantics Are Misleading

Steering commands can time out in the controlling shell even when the broker has accepted the command and the governed session is executing normally.

Impact:

- misleading local failure signal
- operator forced to inspect registry and output ledgers
- hard to distinguish real command failure from a local wait/timeout issue

### 4.4 Packet Test Commands Still Encode Wrong Repo Assumptions

The Loom packet directed a repo-root Cargo invocation equivalent to `cargo test -p handshake_core loom`, but the actual repo layout requires crate-local or manifest-path execution for that test surface.

Impact:

- false negative friction at the first coder execution step
- wasted smoke-test time on command repair instead of product validation
- increased chance that a coder either broad-runs the wrong surface or silently narrows the test plan without recording why

### 4.5 Direct Review Helper Semantics Were Misleading

The governed helper surface around validator-directed review traffic was semantically inconsistent during the live run.

- `VALIDATOR_QUERY` is intended to be validator -> coder traffic.
- The helper name `wp-validator-query` and its documentation pathing were easy to interpret incorrectly from the coder side.
- That ambiguity contributed to malformed or confusing review traffic and forced manual repair / clarification during the smoke.

Impact:

- direct coder <-> validator review was harder to activate than intended
- early review receipts were noisier than they should have been
- the smoke relied on Orchestrator interpretation instead of a crisp helper contract

### 4.6 Active Worktrees Read Stale Local Governance Truth

Governed role sessions execute against each worktree's local repo copy, not against a central orchestrator-only governance surface.

During the live run:

- `wt-orchestrator` packet/task-board truth was repaired first
- the active coder and validator worktrees still carried `Ready for Dev` packet copies and stale helper wiring
- runtime projections such as `current_packet_status` continued to read stale local packet state until those worktree-local copies were patched too

Impact:

- authoritative truth repair in one worktree did not immediately fix the live lane state
- runtime/readiness projections lagged behind the canonical orchestrator worktree
- governance hotfixes for active sessions must be fanned out into the active worktrees, not just committed centrally

### 4.7 Direct Review Was Eventually Activated, But Only After Repair Steering

After explicit Orchestrator repair steering:

- Loom now has a real coder -> WP validator review request, a validator query, and a correctly routed coder review response on the same correlation id
- Schema now has a validator query, a direct coder review request, and an active WP-validator review turn against that handoff

This proves the governed direct-review channel can work, but it also proves it was not self-starting enough in the original workflow posture.

At least the Loom v2 path still exposed test-law assumptions that behave like a workspace-root Cargo repo, while the real repo requires crate-local manifest-aware commands for `handshake_core`.

Impact:

- first-turn test plans are noisy
- coder time is wasted rediscovering the correct invocation
- smoke-test quality is reduced because the packet cannot be trusted as executable law

### 4.5 Governance Fix Distribution Model Is Weak

When the runtime hotfix was applied, it had to be copied into each already-active coder/validator worktree because their local `just` commands execute the local branch copy of `.GOV`.

Impact:

- active worktrees diverge mid-smoke on governance-only files
- patch deployment is manual and error-prone
- ACP smoke tests are sensitive to branch-local governance skew

### 4.6 Authoritative State Sync Failed

The live ACP/runtime surfaces, the role worktrees, and the canonical packet/task-board surfaces drifted apart during the smoke.

Concrete symptoms:

- both v2 WPs remained listed as `READY_FOR_DEV` on the task board
- both canonical task packets remained at `Ready for Dev`
- Loom v2 had already reached a skeleton checkpoint in the coder worktree
- Schema v2 had already started and accumulated active product-code edits in the coder worktree
- both WP validators had active standby/runtime evidence proving the sessions were launched

Impact:

- later review could falsely conclude the v2 WPs never started
- operator trust in packet/task-board truth was damaged
- ACP/runtime truth required forensic reconstruction from worktrees and ledgers

### 4.7 Direct Review Channel Existed But Was Not Used

The repo already had structured coder/WP-validator review helpers and receipt kinds, but the live v2 run did not actually use them.

Observed behavior:

- Schema v2: only `WP_VALIDATOR -> ORCHESTRATOR` traffic existed in the WP thread/receipts
- Loom v2: `CODER -> ORCHESTRATOR` and `WP_VALIDATOR -> ORCHESTRATOR` traffic existed, but still no direct coder <-> validator review traffic
- searches across both v2 WP communication folders found no `VALIDATOR_QUERY`, `VALIDATOR_RESPONSE`, `REVIEW_REQUEST`, `REVIEW_RESPONSE`, `SPEC_GAP`, or `SPEC_CONFIRMATION` receipts before explicit Orchestrator intervention

Impact:

- WP validators did not actively steer coder thinking early
- coders did not expose their proof surface directly to validators
- the workflow behaved more like manual relay than the intended richer orchestrator-managed review loop

### 4.8 Concurrent Steering Exposed A Session-Registry Write Collision

When I attempted to steer multiple active sessions at once to correct the missing direct review traffic, one command failed with `EPERM` while renaming the external `ROLE_SESSION_REGISTRY.json` temp file into place.

Impact:

- concurrent Orchestrator steering can still collide at the session-registry write boundary
- some steering attempts may fail for tooling reasons even when the intended operator action is valid
- session-registry locking/atomic write behavior needs another hardening pass

---

## 5. WRONG TOOLING, UNCLEAR PATHS, OR UNCLEAR COMMANDS

### 5.1 Wrong Tooling

- `validator-startup` was vulnerable to unrelated historical session rows.
- `wp-thread-append` and `wp-receipt-append` did not normalize nullable CLI values safely.
- shell-level `just steer-*` behavior does not present a clear "accepted vs finished" distinction.

### 5.2 Unclear Paths

- live runtime authority is now external, but historical session rows still carried repo-local `.GOV/roles_shared/runtime/...` paths
- current operators cannot assume that an `output_jsonl_file` path in historical results is directly valid in the worktree they are standing in

### 5.3 Unclear Commands

- packet test-law around Cargo is still not reliably aligned with the actual repo layout for `handshake_core`
- a shell timeout on `just steer-*` does not mean the governed session rejected the command

---

## 6. FAILURES PATCHED DURING THE SMOKE

These were fixed in-place during the live v2 smoke:

1. `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
   - added compatibility resolution for legacy repo-local ACP output paths against the external runtime root

2. `.GOV/roles_shared/scripts/wp/wp-thread-append.mjs`
   - normalized nullable CLI fields so `false` stops leaking into thread metadata

3. `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
   - normalized nullable receipt fields so `false` stops leaking into structured receipts

Immediate effect:

- `just gov-check` passed again in `wt-orchestrator`
- `just validator-startup` passed in the active Loom validator worktree after the same fix existed locally there
- Loom validator standby artifacts were subsequently written with proper null handling
- follow-up governance hardening upgraded future validator closure from `SPLIT_DIFF_SCOPED_V1` to `SPLIT_DIFF_SCOPED_RIGOR_V3`, and both active `v2` packets were upgraded to that stricter report profile so future PASS claims must include independent checks, counterfactuals, and medium/high-risk boundary + negative-path review

---

## 7. OPEN RISKS

1. Active coder/validator worktrees still require the same governance hotfix locally while the smoke is in progress.
2. The ACP operator experience still depends on log/registry inspection rather than deterministic shell acknowledgements.
3. Packet test-law remains vulnerable to repo-layout drift unless converted to checked, manifest-aware commands.
4. Validator engagement is still too passive before coder handoff; the workflow can park validators cleanly, but does not yet force earlier structured review traffic.

---

## 8. RECOMMENDED NEXT ACTIONS

1. Keep this smoke running, but treat the ACP/runtime layer as under active repair rather than stable.
2. Fold the three-file runtime hotfix into the canonical governance branch immediately.
3. After the current smoke, decide how governance-only hotfixes should propagate into already-active role worktrees without manual copy/paste patching.
4. Add a check or packet-generation rule that rejects repo-root Cargo commands when the packet targets `src/backend/handshake_core`.
5. Tighten validator engagement so WP validators can receive structured review requests earlier than handoff.
6. Add a truth-sync rule so live orchestrator-managed WPs cannot remain `READY_FOR_DEV` once coder or validator sessions have started.
7. Add an explicit enforcement rule that a live orchestrator-managed WP must produce direct coder <-> WP-validator review traffic before validation-ready handoff.
8. Harden session-registry writes so concurrent steering does not fail on temp-file rename collisions.

---

## 8A. CONTINUED STEERING FINDINGS AFTER THE INITIAL AUDIT CUT

Later in the same live smoke, additional concrete findings surfaced:

1. Direct coder <-> WP-validator review traffic was successfully activated for both WPs, but only after explicit Orchestrator repair steering.
   - This confirms the helpers and packet-scoped communication model are usable.
   - It also confirms the default workflow still tolerates passive waiting too long before the direct review loop is enforced.

2. `WP-1-Loom-Storage-Portability-v2` had a real packet command-surface defect after the repo restructure.
   - The signed packet and refinement still named repo-root Cargo commands that do not work from the assigned WP worktree.
   - Canonical packet/refinement plus the live Loom coder and validator worktrees were repaired to the manifest-path command surface:
     - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom`
     - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance`
   - This was a governance/proof defect, not yet a confirmed product-code defect.

3. `WP-1-Loom-Storage-Portability-v2` runtime projection drifted after packet repair.
   - Canonical packet and Task Board truth had already moved to `In Progress`.
   - The packet-scoped `RUNTIME_STATUS.json` still reported `current_packet_status=Ready for Dev` until the Orchestrator refreshed it with a supported heartbeat.
   - This exposed a live rule gap: packet/task-board repair must be followed by runtime-status refresh during active governed work.

4. `WP-1-Structured-Collaboration-Schema-Registry-v2` coder evidence improved materially, but scope hygiene remained a live review problem.
   - The coder eventually completed a broad manifest-path test run whose output included the packet-relevant negative-path and structured-artifact tests.
   - At the same time, the coder worktree still carried a large unstaged product diff far beyond the five-file packet-scoped handoff claim.
   - This means green tests alone were not enough; the validator correctly continued treating scope isolation and diff-truth as live blocker surfaces.

5. `WP-1-Structured-Collaboration-Schema-Registry-v2` validator review hit a real environment ceiling and adapted correctly.
   - Broad verification in the validator lane hit Windows linker OOM (`LNK1102`) rather than a deterministic product regression.
   - The validator then narrowed toward diff-scoped exact tests instead of treating the environment ceiling as a PASS or a silent stall.
   - This is the correct validator behavior and should be codified as expected practice.

6. ACP/broker behavior still showed structural repair pressure under concurrency.
   - New steers during an active governed run were rejected with explicit concurrent-run rejection.
   - The Schema validator lane later emitted `broker.repair` recovery events for an abandoned governed run after prior broker exit.
   - Shell-level `just steer-*` behavior and ACP transport symptoms still do not provide a clean accepted-vs-finished distinction without checking the registry and per-command output logs.

7. Canonical packet/task-board truth and launched-worktree truth were still able to diverge in ways that mislead validators.
   - In canonical `wt-orchestrator`, both `v2` packets and the Task Board were already `In Progress`.
   - The launched Loom integration-validator worktree still surfaced local `Ready for Dev` text plus stale validator-gate evidence from the PREPARE lane.
   - This created a second-order failure mode: the Integration Validator can spend real time proving that its local packet/gate surface is stale instead of spending that time on final technical review.

8. `just steer-*` wrapper timeouts still do not mean the steer failed.
   - A Schema coder steer timed out at the shell boundary, but the broker did accept it and the session registry moved the coder lane to `COMMAND_RUNNING`.
   - Operators still need to inspect `SESSION_CONTROL_REQUESTS.jsonl`, `SESSION_CONTROL_RESULTS.jsonl`, or the session registry before deciding whether to resend a prompt.

9. WP heartbeat trigger taxonomy is still too narrow for active review states.
   - Attempting to record a validator waiting state with `VALIDATOR_TRIGGER=SPEC_GAP_OPEN` failed because the allowed trigger list rejected it.
   - The runtime had to be updated with `validator_trigger=NONE` even though the true state was "validator raised a packet-scoped spec gap and is waiting on coder repair/proof."
   - This is a real observability gap, not just wording polish.

10. Early malformed structured review receipts can still poison runtime open-item tracking.
    - The first Schema validator query was written before the corrected direct-review helper path and ended up with target/session metadata embedded in the wrong fields.
    - Later correct coder/validator exchanges did not automatically close that malformed legacy open item, so runtime still reports review debt that is partially historical noise.
    - The new runtime reconcile helper can rebuild from receipts, but it still cannot infer semantic closure for malformed old correlations without an explicit resolver receipt.

11. Skill/tool loading inside governed validator turns is still vulnerable to unquoted Windows paths.
    - The Loom integration-validator run attempted to read the `adversarial-code-review` skill file, but the generated `Get-Content` command did not quote the `C:\\Users\\Ilja Smets\\...` path correctly.
    - The result was a PowerShell parameter-binding failure before the validator recovered and kept working.
    - This is low-level tooling friction, but it matters because it wastes final-validator time during the most authority-sensitive turn.

12. Active launched-worktree packet truth still needed manual repair after the canonical branch was already correct.
    - The authoritative packet and Task Board in `wt-orchestrator` had already been moved to `In Progress`.
    - The launched validator worktrees still carried stale local packet/task-board copies, including `Ready for Dev` text that misled the Integration Validator into treating stale governance text as a live blocker.
    - The Orchestrator had to sync canonical `.GOV/task_packets/*.md` and `.GOV/roles_shared/records/TASK_BOARD.md` into the active worktrees before final review could proceed from current truth.

13. Schema runtime open-item closure required extra resolver receipts beyond the substantive coder fix.
    - After the Schema coder posted the real negative-path repair, runtime still held onto malformed and stale open review items because the earliest validator query had been written before the corrected structured-review helper path.
    - The coder had to append explicit resolver receipts that acked both the validator-gap correlation and the malformed legacy `DONE_MEANS: ...` correlation before runtime open-item tracking partially normalized.
    - This is a real workflow ergonomics issue: even when the substantive repair is correct, the communication ledger can still require cleanup traffic to recover from malformed early receipts.

14. Schema validator review became materially stronger once the direct blocker loop was active.
    - The validator did not accept the coder's broad green-test claim.
    - It isolated one specific mutation-before-validation defect, forced a narrowed product repair, then re-entered active review with exact packet-scoped inspection and an exact negative-path test rerun.
    - This is the correct validator posture and is the clearest evidence in this smoke that stricter anti-gaming review behavior is feasible in practice.

15. Loom final review is still vulnerable to mixed-context FAILs when stale governance and real product findings appear in the same report.
    - The first Loom integration-validator FAIL combined one credible product defect signal (linked-block delete can leave stale derived counts) with stale launched-worktree governance text and repo-baseline environment noise.
    - The corrected integration-validator turn had to explicitly discard stale-context reasoning and then run a fresh out-of-repo repro probe to settle whether the linked-block-delete counter path is a real current-main defect.
    - This shows a remaining process weakness: final validators need a cleaner separation between product verdict, stale-governance drift, and repo-baseline environment debt.

16. Runtime trigger vocabulary was too narrow for live validator blocker states and had to be patched during the smoke.
    - Attempting to record the real Loom review state with `validator_trigger=SPEC_GAP` failed because the runtime enum rejected it.
    - The canonical governance surface was patched in-place so `SPEC_GAP` is now a supported runtime trigger value.
    - This confirms that the live packet-scoped review loop can surface valid states faster than the runtime taxonomy evolves.

17. Schema runtime projection still required explicit receipt-based reconciliation after the direct review loop was repaired.
    - By the time the Orchestrator re-read the Schema communication ledger, the coder had already posted the substantive `REVIEW_RESPONSE` plus two resolver receipts that closed the malformed legacy correlation and the live validator-gap correlation.
    - The packet-scoped `RUNTIME_STATUS.json` still needed a reconcile pass to rebuild `open_review_items` from receipts and discard historical noise correctly.
    - This is a real workflow fragility: even after structured review traffic exists, the runtime projection can lag behind the communication ledger and misstate remaining review debt.

18. Long-running validator-owned cargo checks still produce ambiguous liveness during active review.
    - The Schema WP-validator exact negative-path rerun stayed in ACP `RUNNING` state for an extended period with multiple live `cargo` processes but no new result row and no direct verdict receipt yet.
    - From the Orchestrator side, this is hard to distinguish from a hung validator turn without inspecting OS process state and the per-command output file.
    - The smoke still lacks a clean, operator-visible distinction between "actively compiling/testing" and "stalled review turn."

19. Loom integration authority and Loom coder remediation briefly ran out of phase.
    - The old Loom integration-validator FAIL was already persisted in `SESSION_CONTROL_RESULTS.jsonl`, but the superseding integration-validator turn was still mid-run and had not appended its corrected report when the Loom coder started the linked-block delete repair.
    - This created a temporary split-brain state: the coder was responding to the narrowed live blocker while the formal integration-validator report surface still lagged behind the real final-review reasoning.
    - The workflow needs a tighter rule that a superseding final-validator turn must materialize its report promptly once the authoritative defect classification changes.

20. ACP cancel flow was too brittle when the broker accepted a request but did not answer within the wrapper timeout.
    - During live recovery, the Orchestrator tried to cancel the stale Schema WP-validator run and the stale Loom integration-validator run after both had gone quiet.
    - The control wrapper failed hard after a 30-second broker timeout even though the cancel request row had already been appended to `SESSION_CONTROL_REQUESTS.jsonl`.
    - The governance surface was patched in-place so cancel/close now tolerate "request logged, broker response slow" and wait longer for settlement instead of reporting an immediate false failure.

21. Broker active-run cleanup still lags after cancel requests and after effectively completed turns.
    - The Schema WP-validator cancel request did settle with `cancel_status=cancellation_requested`, but the target run still remained listed in broker `active_runs` and did not emit a terminal result row immediately.
    - The Loom integration-validator and Loom coder turns also remained in broker `active_runs` with old output-file timestamps and no new result rows, even after the integration-validator turn had already appended a definitive report and the Loom coder had already patched the defect before entering proof.
    - This leaves the Orchestrator in a bad middle state: one-active-run-per-session blocks re-steering, but the broker/operator surfaces do not clearly distinguish "active work", "idle wrapper", and "cancel requested, waiting for child teardown".

22. Broker shutdown recovery still required out-of-band process intervention.
    - The broker accepted a `shutdown force` RPC and returned a success-style acknowledgment, but the broker process did not actually exit.
    - The stale child coder process also survived long enough to keep automatic broker restart logic from reclaiming the session cleanly.
    - The Orchestrator had to stop both the stale governed child process and the broker process manually before a clean broker restart was possible.

23. Automatic broker restart is still too dependent on stale child PID truth.
    - After the stale-run recovery path, `ensureBroker` still treated the old broker state as authoritative because broker state and orphaned child PID truth did not converge quickly enough.
    - The runtime only normalized after the Orchestrator manually launched a fresh broker process and then verified `active_runs=[]` in the external broker-state ledger.
    - This is a real orchestration hazard because the repo can appear "recovered" while automatic restart is still blocked by stale process metadata.

24. Concurrent sequential-steering violations still corrupt the session-registry write path.
    - When the Orchestrator tried to steer multiple governed sessions at the same time after broker recovery, one command failed with `EPERM` during temp-file rename into `ROLE_SESSION_REGISTRY.json`.
    - The paired prompt on the other lane then timed out at the wrapper boundary, leaving ambiguity about which steering action actually landed.
    - This confirms that the external session registry still does not tolerate concurrent read-modify-write turns and that live steering must remain serialized until registry writes are hardened.

25. Direct review helper serialization can still emit malformed target-session metadata under integration-validator traffic.
    - The Loom coder's first integration-validator-targeted `REVIEW_RESPONSE` serialized with an effectively empty `target_session`, forcing a superseding authoritative entry a few seconds later.
    - The substantive repair evidence was still preserved, but the first malformed entry created avoidable ambiguity in the communication ledger.
    - This is low-level tooling friction, but it matters because integration-validator turns are the most authority-sensitive part of the workflow.

26. Live WP truth can keep advancing in the communication ledger after authoritative packet and runtime surfaces stall.
    - After the Schema integration-validator recorded `FAIL`, the coder still posted a substantive packet-scoped blocker repair in the WP thread with new exact tests and line-level proof.
    - At the same time, authoritative runtime and session-control surfaces lagged behind that reality, which means a later operator check could have misread the lane as idle or unchanged.
    - This is another argument for tighter packet/runtime reconciliation and stronger operator-first monitoring of the communication ledger itself during active repair loops.

27. Packet `MERGE_BASE_SHA` parsing was too strict in active role worktrees and silently broke exact-range validation.
    - The live packets store `MERGE_BASE_SHA` with the approved trailing explanatory text.
    - Several active worktrees still used a regex that only accepted a bare 40-char hash at end-of-line, so `post-work` and `validator-handoff-check` silently fell back to the wrong default range.
    - The parser had to be repaired and fanned out into the active coder, validator, and integration-validator worktrees before exact-range validation became trustworthy again.

28. Active worktrees kept stale copies of other live packets and `BUILD_ORDER.md`, which caused cross-WP `gov-check` failures unrelated to the packet under review.
    - The active integration-validator worktrees still carried `Ready for Dev` headers for the opposite `v2` packet even after canonical truth had moved both WPs to `In Progress`.
    - The same active worktrees also drifted on `BUILD_ORDER.md`, which made `just gov-check` fail even when packet truth and task-board truth were otherwise repaired.
    - The Orchestrator had to sync packet headers and run `just build-order-sync` across all active coder and validator worktrees to restore a usable governance baseline.

29. Final validation was still reading dirty PREPARE worktrees even when it claimed to validate the committed candidate.
    - The live `validator-handoff-check` implementation ran `post-work` inside the assigned PREPARE worktree, so unrelated local dirt and historical governance sync commits could still poison the result.
    - This was repaired in-place so the final validator now runs `post-work` against a temporary clean shared clone of the committed target while keeping PREPARE preflight in the assigned worktree.
    - This materially reduced false FAILs in Schema and makes the validator closer to the intended "exact committed candidate" authority model.

30. Skeleton approval enforcement was too rigid for long-lived dirty WP worktrees.
    - Loom still lacked the required `docs: skeleton approved [...]` commit in branch history, and the original helper refused to run if the WP worktree had any local dirt at all.
    - In practice this blocked retroactive repair of the governance lane even when the approval commit itself was allow-empty and would not include the unstaged product changes.
    - The helper was patched to continue refusing staged changes, but to allow unstaged dirt with an explicit note that the approval commit remained empty.

31. Schema exact closure only became green once the final validator used the real packet-scoped committed range instead of raw branch merge-base.
    - `validator-handoff-check` on the default `MERGE_BASE_SHA..HEAD` range still pulled in the broad governance-sync history of the feature branch and failed on out-of-scope changes.
    - The packet-scoped range `5f2f4831b74a06ca763318c70f7149d1f04c8fe7..da05d07d0365d419dce23bbb27788296d52d2ffd` passed cleanly in both `post-work` and `validator-handoff-check`.
    - This exposed a remaining workflow gap: when feature branches carry initial governance-sync commits, final validation needs a first-class way to record and reuse the true packet-scoped committed range instead of relying on the raw branch merge-base.

32. Closure started with missing remote WP backup branches for both active `v2` packets.
    - Neither `origin/feat/WP-1-Loom-Storage-Portability-v2` nor `origin/feat/WP-1-Structured-Collaboration-Schema-Registry-v2` existed when the integration-validator began the governed PASS closeout.
    - Both branches had to be pushed mid-closeout only to satisfy the pre-merge safety rule.
    - This is avoidable workflow drag: backup-branch publication should happen before final validation starts, not during Gate 2/merge prep.

33. Loom closeout temporarily re-opened the legacy `WP-1-Loom-MVP-v1` packet while closing `WP-1-Loom-Storage-Portability-v2`.
    - The live Loom integration-validator turn unexpectedly loaded `WP-1-Loom-MVP-v1` content during the v2 closure flow.
    - That did not become the authoritative closeout surface, but it shows packet lookup/search logic can still pull a superseded base packet into an active remediation closeout.
    - This is a high-noise failure mode because it can mix old scope, old waivers, and old test assumptions into a current remediation run.

34. Loom PASS closure crossed the merge boundary before its final report satisfied the enforced V3 structure.
    - The integration branch did merge the validated feature head, and the validator updated packet/task-board/traceability/build-order state through Gate 2.
    - `gov-check` then blocked legal closeout because the appended validator report did not match the newer `SPLIT_DIFF_SCOPED_RIGOR_V3` structure expected by `validator-report-structure-check`.
    - This proves the report-format contract is now strong enough to stop a weak PASS, but it also shows the validator still lacks a pre-append helper that guarantees V3-compliant report shape before closure work begins.

35. Loom packet-scoped closure on the shared integration worktree required a stash/pop cycle and produced a deterministic `BUILD_ORDER.md` conflict.
    - The validator had to stash the Loom packet, Task Board, and Build Order closure files, merge the validated feature head, then pop the closure state back.
    - `BUILD_ORDER.md` conflicted immediately because the stash held a stale derived snapshot while the merge produced a different autogenerated snapshot.
    - The validator recovered correctly by restoring `BUILD_ORDER.md` to `HEAD` and regenerating it with `just build-order-sync`, but this is still workflow friction created by shared derived files living in the same closure commit boundary as packet-scoped truth.

36. Schema PASS closure could not proceed in the assigned integration worktree and required a second clean helper worktree.
    - The assigned integration-validator worktree carried unrelated shared governance dirt and packet/task-board overlap from the other active WP.
    - The validator had to create `wti-schema-closure-clean` solely to get a clean packet-scoped closure surface.
    - This confirms the current "one live integration worktree per packet" rule is not enough when shared governance files are modified concurrently by multiple active WPs.

37. Schema integration topology was still stale at closure time and had to be repaired before any final closeout work was possible.
    - `integrate/WP-1-Structured-Collaboration-Schema-Registry-v2` was still parked at `5f2f483` while the validated packet-scoped candidate lived on `feat/WP-1-Structured-Collaboration-Schema-Registry-v2` at `da05d07`.
    - The integration-validator had to push the backup branch and then fast-forward the integration branch to the validated head before rerunning closure proof.
    - This is a real orchestration failure: a validator can have a correct PASS report while the designated integration branch still does not contain the validated candidate.

38. Schema closure then stalled inside exact proof reruns with no terminal result row or clear liveness signal.
    - After the fast-forward, the integration-validator restarted exact proof (`validator-handoff-check` plus the packet-scoped negative-path cargo test) and then stopped emitting new output.
    - The session remained marked `RUNNING`, but there was no fresh result row, no fresh receipt, and no cheap operator-visible signal distinguishing "actively compiling/testing" from "hung turn."
    - This is the same ambiguity noted earlier for long validator-owned tests, but it recurred here in the highest-authority closure lane.

39. Cheap process-state inspection is still missing during closure incidents.
    - The Orchestrator attempted to inspect Windows process state for the active integration-validator worktrees using `Win32_Process` queries scoped to the relevant worktree paths.
    - Both process-inspection attempts timed out after 120 seconds, yielding no actionable liveness signal.
    - This means the operator still lacks a lightweight way to answer the most basic closure question: "is the validator still doing work, or is the session just wedged?"

40. Loom closeout still resolved the canonical `main` checkout path incorrectly during merge.
    - The integration-validator looked for `D:\Projects\LLM projects\Handshake\handshake_main` instead of the real permanent checkout at `D:\Projects\LLM projects\Handshake\Handshake Worktrees\handshake_main`.
    - Because of that bad assumption, it created a temporary merge worktree `wt-main-merge-loom-v2` in the shared worktree root instead of using the permanent canonical main checkout.
    - This is not just cosmetic sprawl: it proves the merge path still lacks a single authoritative resolver for the repo's protected `main` worktree.

41. Loom was allowed to perform the `main` merge before verifying the actual integration-branch-to-main diff was packet-scoped.
    - The validator correctly preserved the packet-declared backup branch first, but then merged `integrate/WP-1-Loom-Storage-Portability-v2` into `main` immediately.
    - Only after creating merge commit `3e93d22` did it inspect `HEAD^1..HEAD` and discover that the integration branch carried a much broader ancestor than the Loom repair itself.
    - This is a critical workflow defect: packet-scoped proof existed for the validated candidate, but merge-scope proof did not exist for the branch that was actually merged.

42. The Loom integration branch still carried broad non-packet ancestry relative to `main` at final closeout.
    - `git log --reverse d8edeca..0d078e0` showed many unrelated governance and multi-WP setup commits ahead of `main` before the packet-scoped Loom repair and closure commits.
    - The resulting merge diff pulled in coder/validator protocol hardening, research docs, both `v2` packets, the smoke audit, and other governance files in addition to the three Loom product files and Loom packet closure files.
    - This means the current closeout flow still cannot assume `integrate/WP-{ID}` is packet-clean just because the final validator report is packet-clean.

43. Permanent canonical `main` truth and temporary merge-worktree `main` truth diverged during Loom closeout.
    - After the temporary merge worktree recorded `3e93d22`, the permanent `handshake_main` checkout still remained at `81d16f9`.
    - So even if the validator ultimately repairs the merge scope, the smoketest has now shown that "merged into local main" can silently mean "merged into a temporary clone-local main only" rather than the protected permanent checkout on disk.
    - This is another hard operator-safety issue because a later audit can observe two different local `main` truths at the same time.

44. Schema exact closeout surfaced a real parser contract mismatch inside the governance surface itself.
    - `validator-report-structure-check` and the packet authoring flow accepted plain `CLAUSES_REVIEWED:` / `NOT_PROVEN:` labels, but `packet-closure-monitor-lib.mjs` only parsed the older bullet-prefixed label form.
    - As a result, the clean Schema PASS report could still fail `gov-check` even though the report content satisfied the live validator rigor.
    - The Orchestrator had to patch the parser in-place during the smoke so closure monitoring would accept the same governed report syntax the validator was required to emit.

45. ACP stale-run recovery still required manual operator-style process intervention.
    - Canceling the stale Schema integration-validator run timed out at the wrapper boundary even after the broker accepted the request.
    - The dead child process remained in broker `active_runs`, blocked re-steering, and did not settle into a terminal result row on its own.
    - Recovery required manually killing the stale child process and force-shutting the broker before a clean broker restart and fresh steer were possible.

46. Schema closeout still depended on importing already-closed Loom governance truth just to make repo-wide closure proof honest.
    - The Schema helper could not complete `just gov-check` until the already-closed Loom packet/task-board/build-order truth was present locally.
    - That did not reopen Loom product review, but it did mean the Schema final validator had to carry cross-WP shared governance truth in the closure helper before it could honestly claim a green repo baseline.
    - This confirms that shared closure records are still too entangled across active WPs during parallel closeout.

47. Packet-scoped integration onto permanent `handshake_main` is viable, but the live validator still attempted it with unsafe parallel cherry-picks.
    - After proving the direct branch merge into `handshake_main` was too broad, the Schema integration-validator switched to a packet-scoped carry plan and correctly isolated the exact Schema product commit plus the required shared closure-governance commit.
    - It then launched multiple `git cherry-pick -n` commands against the same `handshake_main` checkout at once, which created `.git/index.lock` contention and a self-inflicted partial apply state.
    - This is a workflow/tooling failure, not a product-code failure: packet-scoped carry is the right recovery path, but it must be executed serially in one critical section.

48. The partial Schema carry into permanent `handshake_main` degraded into a single live packet-header conflict instead of a broad merge-scope failure.
    - The Schema product commit `5f25022` applied cleanly onto `handshake_main`, proving the backend repair itself is portable onto canonical `main`.
    - The remaining conflict was narrowed to the packet status line in `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v2.md` (`Ready for Dev` vs `Done`) while the rest of the carry set staged cleanly.
    - This is still operational friction, but it is a materially better failure mode than the earlier broad branch-ancestry merge and shows the packet-scoped recovery path is close to workable once serialized correctly.

49. Both remediation WPs did ultimately reach authoritative `PASS` on the permanent canonical `handshake_main` checkout, but only through packet-scoped selective integration.
    - Schema final canonical-main state is `d9c7daf6b70506994d067a4d99bfc75b177d5a47` (selective integration commit) followed by `9272063e05a278d97cd6fa00b3bc72588cab02ae` (final gate-closure sync on `handshake_main`).
    - Loom final canonical-main state is `b89995a1fed103a57984d8c1299cabcb96300dfb`, which selectively carried only the three Loom product files onto the already-correct packet/task-board closure state on `handshake_main`.
    - This proves the smoke did not just demonstrate partial remediation; both WPs were actually closed to `PASS` on canonical local `main`.

50. Governed session closeout did not automatically reconcile packet-scoped runtime truth after final `PASS`.
    - All six governed sessions were closed successfully with `CLOSE_SESSION`, and the external role session registry now records the two `v2` coder, WP-validator, and integration-validator lanes as `runtime_state=CLOSED`, `startup_proof_state=CLOSED`.
    - Even after that, both packet-scoped `RUNTIME_STATUS.json` files still remained at `input_required/REVIEW` and kept stale next-actor / waiting-on state until the Orchestrator repaired them explicitly.
    - This is a real ACP/runtime gap: session closeout and packet-scoped runtime closeout are still separate operations.

51. Schema runtime open-review projection still held one stale legacy review item after substantive completion.
    - The communication ledger already contained the real validator-gap closure, WP-validator confirmation, and final integration-validator `PASS`.
    - Runtime still kept the original coder-handoff `REVIEW_REQUEST` open because that correlation had never been explicitly superseded in a way the projection layer could infer automatically.
    - The Orchestrator had to append one final resolver receipt and then rebuild the runtime projection before `open_review_items` finally dropped to zero.

52. A deterministic runtime-closeout repair path was needed and is now part of the governance/tooling surface.
    - `wp-runtime-reconcile.mjs` was extended with a `--finalize` mode so the Orchestrator can rebuild `open_review_items`, set `runtime_status=completed`, set `current_phase=CLOSEOUT_COMPLETE`, clear `active_role_sessions`, and record a final closeout receipt after canonical `PASS`.
    - That helper was then used on both WPs to reconcile runtime truth with the already-closed session registry and canonical packet status.
    - This is a concrete post-smoke improvement: the runtime projection layer now has a supported deterministic recovery/closeout action instead of relying on ad hoc JSON edits.

These later findings strengthen the original audit conclusion:

- the parallel orchestrator-managed shape is viable
- direct coder/validator review can work
- but proactive review enforcement, packet/runtime truth sync, and ACP runtime ergonomics still need additional hardening

---

## 9. CONCLUSION

This smoke did prove the parallel orchestrator-managed shape:

- two Coders
- two WP Validators
- separate worktrees
- governed ACP steering

It also proved that the supporting tooling still has real operational faults.

It also proved a more serious governance point:

- runtime truth without prompt packet/task-board reconciliation is not good enough
- if the authoritative surfaces lag the live run, the system can look idle or orderly while real partial work is already in flight
- if the direct coder/validator review lane is not actively enforced, the system falls back to hub-and-spoke relay through the Orchestrator even when better tooling already exists
- if packet-scoped runtime closeout is not explicit, the session registry and the WP runtime ledger can disagree even after both WPs are technically finished

The important conclusion is not "ACP failed." The important conclusion is:

- ACP parallel orchestration is viable
- both remediation WPs did actually finish `PASS` on permanent canonical local `main`
- packet-scoped selective integration is the safe closeout path; naive branch merge is not
- the runtime/check/tooling layer still has failure modes that are only visible under live concurrent use
- those failure modes are now concrete, reproducible, and documented by this audit
