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

Post-audit correction:

- both `v2` remediation WPs were genuinely started
- the authoritative packet/task-board surfaces were not reconciled to match runtime truth
- this audit must therefore be read as a live run with stale governance state, not as a run that never advanced beyond `READY_FOR_DEV`

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

The important conclusion is not "ACP failed." The important conclusion is:

- ACP parallel orchestration is viable
- the runtime/check/tooling layer still has failure modes that are only visible under live concurrent use
- those failure modes are now concrete, reproducible, and documented by this audit
