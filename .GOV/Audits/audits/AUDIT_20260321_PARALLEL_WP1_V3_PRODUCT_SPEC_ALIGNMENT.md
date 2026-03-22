# Audit: Parallel WP-1 v3 Product Code vs Master Spec Alignment

## METADATA
- AUDIT_ID: AUDIT-20260321-PARALLEL-WP1-V3-PRODUCT-SPEC-ALIGNMENT
- DATE_UTC: 2026-03-21
- AUTHOR: Codex acting as Orchestrator
- SCOPE:
  - Canonical integrated product code on local `main`
  - `WP-1-Structured-Collaboration-Schema-Registry-v3`
  - `WP-1-Loom-Storage-Portability-v3`
- RESULT: FAIL FOR SPEC-TIGHTNESS ON THE SCHEMA-REGISTRY AND MAILBOX VALIDATION LAYER; NO EQUIVALENT NEW LOOM FAILURE IDENTIFIED IN THIS AUDIT PASS
- KEY_PRODUCT_COMMITS_REVIEWED:
  - `fe998e1` `merge: selective Schema Registry v3 integration from 23f4c9a`
  - `e867469` `merge: selective Loom v3 integration from 7aa995b`
- EVIDENCE_SOURCES:
  - `.GOV/spec/Handshake_Master_Spec_v02.178.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v3/packet.md`
  - `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`
  - `../handshake_main/src/backend/handshake_core/src/locus/types.rs`
  - `../handshake_main/src/backend/handshake_core/src/locus/task_board.rs`
  - `../handshake_main/src/backend/handshake_core/src/workflows.rs`
  - `../handshake_main/src/backend/handshake_core/src/role_mailbox.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`
  - `../handshake_main/src/backend/handshake_core/src/api/loom.rs`
  - `../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs`
  - `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs`

---

## 1. EXECUTIVE SUMMARY

This audit re-checked the integrated product code for the two v3 smoketest work packets against the current Master Spec instead of trusting the packet closeout narrative, coder handoff, or earlier validator PASS claims.

The main conclusion is:

- the Loom v3 implementation currently looks materially real in the audited route and storage surfaces
- the structured-collaboration and mailbox validator layer is still under-implemented relative to the Master Spec
- the v3 packets improved the happy path, but the failure-path proof is still too weak
- the remaining weakness is not "feature absent" so much as "contract enforcement shallow"

This means the live smoketest improved delivery, but it did not yet prove that the spec-backed structured artifact layer is resilient against drift, malformed artifacts, or partially broken future edits.

---

## 2. AUDIT ATTACK SURFACES

- Shared structured-collaboration validator in `locus/types.rs`, because v3 claimed schema-registry closure and machine-readable compatibility enforcement
- Runtime Task Board projection emission, because v3 claimed row-field closure and projection integrity
- Runtime Role Mailbox export validation, because v3 claimed field completeness and governed export safety
- Loom graph traversal, metrics, and search route surface, because v3 claimed storage portability and graph-query closure

---

## 3. FINDINGS BY SEVERITY

### 3.1 High: The schema-registry validator still does not enforce the v02.171 workflow contract fields

Master Spec v02.178 requires `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on canonical Work Packet, Micro-Task, and Task Board projection records.

Current integrated code state:

- the artifact structs do carry these fields
- the emitters populate these fields
- the validator does not actually check them for Work Packet, Micro-Task, or Task Board entry records

Observed evidence:

- `TrackedWorkPacketArtifactV1`, `TrackedMicroTaskArtifactV1`, and `TaskBoardEntryRecordV1` include the workflow fields
- `validate_structured_collaboration_record(...)` only checks `summary_record_path` for Work Packet / Micro-Task packet families
- the Task Board entry branch checks board identifiers and display fields, but not the v02.171 workflow fields

Impact:

- a drifted or corrupted on-disk record can remain spec-invalid and still pass mechanical validation
- the schema-registry pass does not actually prove portability of the workflow-state contract
- future regressions can silently remove the exact fields the Dev Command Center and routing rules rely on

Assessment:

- this is a real product-code gap, not only a test gap
- this is the strongest remaining reason to reject a full spec-tight PASS on the schema-registry side

### 3.2 High: Nested structured payloads are only validated at "array exists" depth

The spec defines concrete nested structures for:

- Task Board `rows`
- Role Mailbox index `threads`
- Role Mailbox thread-line `transcription_links`

Current integrated code state:

- those fields are only validated by checking that they are arrays
- there is no per-item schema validation in the shared validator for those nested records

Observed evidence:

- `rows`, `threads`, and `transcription_links` are routed through `require_value_array(...)`
- `require_value_array(...)` checks array-ness only and does not validate element shape
- `role_mailbox.rs` relies on `validate_runtime_mailbox_record(...)` before emitting export files

Impact:

- malformed nested payloads can pass validation even when they violate the spec-defined field set
- the Role Mailbox export gate is therefore weaker than it appears
- v3 improved emitted happy-path records, but not the strength of the validator against malformed stored state

Assessment:

- this is exactly the sort of "looks finished, not actually defended" gap that earlier false-PASS history should have made unacceptable
- the validator currently proves field presence at the outer envelope, not full nested contract conformance

### 3.3 Medium: RFC3339 / ISO-8601 timestamp contracts are not mechanically enforced by the shared validator

The spec treats `updated_at`, `generated_at`, and `created_at` as typed timestamp fields, not just arbitrary non-empty strings.

Current integrated code state:

- the emitters generally write RFC3339 timestamps
- the shared validator only checks that timestamp fields are non-empty strings
- there is no generic timestamp-format validation in the schema-registry path

Impact:

- malformed timestamps can pass mechanical validation
- freshness, ordering, and mirror-state logic can silently degrade if a later change emits bad timestamp strings
- the current proof layer is still too shallow for a contract that explicitly encodes freshness and replay safety

Assessment:

- this is less severe than the missing workflow-field enforcement, but still a real spec gap
- the validator is proving "field exists," not "field matches the declared type"

### 3.4 No equivalent new Loom gap was identified in this pass, but the fresh proof scope was narrower

In the audited Loom v3 surface, the following looked materially implemented rather than narrative-only:

- storage trait includes `get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, and `recompute_all_metrics`
- SQLite and PostgreSQL implementations exist
- API route tests for graph traversal, metrics recomputation, and view/search event emission pass

Assessment:

- this audit did not find a new concrete Loom product-code failure comparable to the schema-registry validator gaps
- this is not a full renewed dual-backend portability PASS; it is a narrower "no fresh concrete failure found here" judgment

---

## 4. PROBABLE CODER FAILURE MODES

These are process inferences from the product state, not direct claims about private intent.

### 4.1 Coder likely optimized for emitter correctness more than validator hardness

The code strongly suggests the v3 implementation work focused on:

- making the emitted happy-path records look correct
- adding or restoring the missing fields on produced artifacts
- adding tests that prove the intended runtime path emits those fields

But the code does not show the same rigor on:

- malformed stored artifacts
- nested schema corruption
- v02.171 workflow-field absence in validator paths

Probable failure:

- the coder closed the visible product artifact shape but did not finish the contract-enforcement layer that is supposed to protect that shape from future drift

### 4.2 Coder likely treated "machine-readable validation exists" as sufficient without attacking adversarial inputs

The presence of `StructuredCollaborationValidationResult` and issue codes can create a false sense of closure.

Probable failure:

- once a typed validation object existed, the implementation may have been treated as "registry complete" without verifying that every spec-required field and nested structure was actually enforced

---

## 5. PROBABLE VALIDATOR FAILURE MODES

### 5.1 Validator likely over-weighted happy-path emission tests and under-weighted failure-path attacks

The current product state is consistent with a validator pass that asked:

- does the emitted artifact now contain the expected fields?
- do the happy-path tests pass?
- do schema-id/version negative-path tests fail correctly?

But did not push hard enough on:

- missing v02.171 workflow fields
- malformed nested `rows`, `threads`, or `transcription_links`
- bad timestamp formats on fields the spec treats as typed timestamps

Probable failure:

- the validator verified that the new structure exists, but not that the validator layer can reliably reject broken structure

### 5.2 Validator likely treated packet-scoped done-means as the ceiling instead of the floor

The v3 packet/refinement history clearly improved the work packet family, but the integrated code still shows that some spec obligations remain only partially enforced.

Probable failure:

- the validator may have accepted packet-scoped closure once the named gaps from prior versions looked visibly fixed, instead of asking whether the shared validator now enforces the broader spec contract robustly enough to prevent another false closeout later

---

## 6. PRODUCT CODE CHANGES NEEDED

### 6.1 Strengthen `validate_structured_collaboration_record(...)`

Required changes:

- enforce `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` for Work Packet packet records
- enforce the same fields for Micro-Task packet records
- enforce the same fields for Task Board entry records
- reject missing or malformed values instead of silently accepting them

Why:

- v02.171 made these fields part of the canonical portable workflow contract
- leaving them outside validation keeps the registry pass incomplete

### 6.2 Add nested validators for Task Board and Role Mailbox structures

Required changes:

- validate each Task Board `rows[]` entry as a `task_board_entry` record
- validate each Role Mailbox `threads[]` item against the index-thread schema
- validate each `transcription_links[]` item for required keys and expected scalar formats

Why:

- array presence is not schema conformance
- the current mailbox export gate is too shallow to justify a strong PASS claim

### 6.3 Add real format validators for constrained string fields

Required changes:

- validate RFC3339 / ISO-8601 timestamp fields
- validate sha256 fields as 64-char lowercase hex where the spec declares hashes
- validate artifact-handle string fields where the spec declares artifact handles

Why:

- non-empty-string checks are not enough for replay-safe, audit-safe records

### 6.4 Add negative-path regression tests for the missing validator behavior

Required changes:

- test missing `workflow_state_family`
- test missing `queue_reason_code`
- test missing or malformed `allowed_action_ids`
- test malformed Task Board `rows[]`
- test malformed Role Mailbox `threads[]`
- test malformed `transcription_links[]`
- test malformed timestamp strings

Why:

- the current suite proves some important negative paths
- it does not yet prove the ones that map to the gaps found in this audit

### 6.5 Keep Loom changes limited unless a new concrete failure is found

Recommended stance:

- do not create speculative Loom code churn from this audit alone
- if a new Loom pass is desired, rerun the full SQLite and PostgreSQL conformance / traversal-performance proof before changing product code

Why:

- the concrete failures found in this audit are on the schema-registry and mailbox validation side, not on the currently audited Loom route surface

---

## 7. INDEPENDENT CHECKS RUN

- `cargo test --test role_mailbox_tests role_mailbox_validation_reports_schema_and_authority_drift -- --exact --nocapture`
  - Result: PASS
- `cargo test --test micro_task_executor_tests locus_register_mts_returns_machine_readable_validation_for_unknown_schema_version -- --exact --nocapture`
  - Result: PASS
- `cargo test --lib api::loom::tests::view_and_search_emit_events -- --exact --nocapture`
  - Result: PASS
- `cargo test --lib api::loom::tests::graph_traversal_and_metrics_routes_work -- --exact --nocapture`
  - Result: PASS
- Direct code inspection of the integrated `main` product code against the cited spec clauses
  - Result: FINDINGS RECORDED ABOVE

---

## 8. COUNTERFACTUAL CHECKS

- If a future edit removes `workflow_state_family` from a Work Packet, Micro-Task, or Task Board entry artifact, the current shared validator will not detect that regression.
- If a mailbox export contains `transcription_links: [{}]` or other malformed nested items, the current mailbox validator path can still accept the export because it validates array presence rather than element schema.
- If a future edit emits malformed timestamp strings, the current validator can still mark those records valid because it treats those fields as generic non-empty strings.

---

## 9. BOUNDARY AND NEGATIVE-PATH PROBE SUMMARY

- Mailbox schema-id and authority-drift negative-path coverage exists and passed.
- Micro-Task schema-version mismatch coverage exists and passed.
- Loom route-level graph traversal and metrics recomputation coverage exists and passed.
- Missing negative-path coverage remains for the exact validator weaknesses identified in this audit.

---

## 10. RESIDUAL UNCERTAINTY

- This was not a full repo-wide product-vs-spec audit. It was scoped to the two v3 smoketest product surfaces on integrated `main`.
- This audit did not rerun the full PostgreSQL-backed Loom conformance and traversal-performance suite, so it is not a refreshed total Loom portability sign-off.
- The schema-registry and mailbox findings are still sufficient to reject a clean spec-tight PASS for the combined v3 smoketest result.

---

## 11. FINAL JUDGMENT

The v3 smoketest materially improved product delivery and moved both packets beyond the earlier false-closeout state. But the structured-collaboration validator and mailbox export gate remain too shallow for the current Master Spec.

The strongest remaining issue is not that the main v3 artifact fields are absent. It is that the shared proof layer still fails to enforce several of the exact contracts the spec says are authoritative.

That is why this audit lands on:

- Loom v3: no new concrete failure found in the audited route/storage slice
- Schema Registry v3 and Role Mailbox validation layer: still not strong enough for a clean PASS
- Overall combined product/spec result for this smoketest pair: FAIL pending validator hardening and targeted negative-path test additions

---

## 12. GOVERNANCE REMEDIATION PLAN BEFORE FURTHER PRODUCT WORK

This plan is appended before any new product-code remediation starts. The intent is to stop repeating the same failure shape under new names: improved visible delivery, improved packet wording, but insufficient proof hardness and too much Orchestrator-centered correction.

### 12.1 Governing standard and control-plane doctrine

- correctness outranks speed, convenience, and narrative smoothness
- proof outranks appearance
- integrated `main` or an explicitly validated candidate commit outranks worktree-local completion claims
- direct coder <-> validator review outranks Orchestrator relay
- one authoritative workflow truth outranks multi-surface interpretation
- repo governance is a live prototype of the future Handshake control plane, not secondary process overhead
- governance defects that weaken proof, authority, validation, or autonomous coordination must be treated as product-grade harness defects
- the current remediation path should strengthen the microtask/locus direction, not abandon it
- the immediate target is not one universal domain definition of "done"; it is one universal completion framework that can support many domains honestly

### 12.2 Universal completion framework

- Handshake does not need one universal semantic definition of domain-level completion for coding, art, writing, design, or research.
- It does need universal completion layers that every governed run must respect:
  - workflow validity
  - scope validity
  - proof completeness
  - integration readiness
  - domain-goal completion
- The first four layers are governance/control-plane law and must be machine-checkable wherever possible.
- The fifth layer is project-specific and must be supplied by the packet, refinement, domain rubric, or acceptance criteria.
- A clean `PASS` is legal only when the run clears the non-domain layers and the packet-defined domain goal is actually proven.
- If workflow, scope, proof, or integration readiness is incomplete, the system must prefer `NOT_PROVEN`, `BLOCKED`, `OUTDATED_ONLY`, or other explicit non-pass states over narrative closure.

### 12.3 Failure taxonomy

- split-authority failure: packet, runtime, task-board, session, and worktree truth disagree
- false-ready or false-progress failure: the system narrates launch or advancement that is not actually supported by the real state
- false-pass or proof-collapse failure: visible completion is rounded up to spec closure without adequate adversarial proof
- direct-review failure: coder and validator do not exchange the required governed challenge/response receipts directly
- scope-leak failure: broad tools or broad edits silently exceed packet scope
- runtime-ledger failure: session, gate, or control ledgers lose, corrupt, or collide on authoritative runtime evidence
- migration residue failure: legacy compatibility paths or stale authority surfaces remain capable of poisoning current workflow checks

### 12.4 Root causes this plan addresses

- the proof model is still too weak and still over-rewards "field exists," "tests passed," and "packet looks closed"
- workflow truth is still fragmented across packet state, runtime state, task board state, session state, and actual worktree state
- the validator is not independent enough from packet framing and still does not attack the broader spec/diff surface hard enough
- the coder can still satisfy the visible artifact shape without fully hardening the enforcement layer that protects it
- the Orchestrator is still absorbing too much routing, interpretation, and repair work that should be handled by workflow law and direct role-to-role exchange
- governance is still being patched too close to live runs, which keeps exposing migration and compatibility drift in production-like execution

### 12.5 Immediate operating rules before the next product pass

- No new product/spec closure attempt should start until the Phase 0 doctrine/completion-model alignment and the first workflow-hardening items below are landed or explicitly waived by the Operator.
- Governance changes during a live smoke should be limited to true blockers. Broad refactors during execution should be treated as workflow risk, not as normal operating style.
- Live runs should be stabilization-first. If a change is not required to keep the governed run truthful or unblocked, defer it out of the active smoke.
- `PASS` must no longer be treated as one compressed word. Every formal validation result should split at least:
  - workflow health
  - packet scope cleanliness
  - semantic/spec proof
  - integration readiness
- domain-goal completion should remain explicit when it matters, but must not be used to hide missing workflow/proof/integration law
- If proof is incomplete, the verdict must be `NOT_PROVEN`, not silently rounded up to `PASS`.
- Any orchestrator-managed run without the required direct coder-validator receipts should be recorded as communication-incomplete even if the product code lands.
- No governed workflow should be considered proven merely because a difficult packet eventually shipped. The workflow itself needs its own proof pass.

### 12.6 Orchestrator remediation

- Add a real smoke-start hard gate that fails unless:
  - `just gov-check` passes
  - `just orchestrator-startup` passes cleanly
  - packet/refinement/spec hashes are current
  - PREPARE targets exist on disk
  - packet/runtime/task-board/session/worktree truth agrees
  - communication artifacts exist only at the packet-authoritative root
- Make `orchestrator-prepare-and-packet` transactional. It must update packet state, runtime state, task-board/traceability state, gate state, and declared worktree/runtime paths together or fail without partial writes.
- Reduce Orchestrator duties after startup to launch, initial lane confirmation, checkpoint approval, and repair-only intervention. Ordinary review routing, wake-ups, and scope clarification must move off the Orchestrator lane.
- Treat out-of-scope file creation or broad formatter/tool spill as an automatic hard-correction event, not a wait-and-see condition.
- Record an explicit workflow-state receipt whenever integration mode changes, for example from direct branch merge to selective integration.

### 12.7 Coder remediation

- Before editing, the coder must publish a direct response receipt to the validator that states:
  - intended proof surface
  - tests to be run
  - negative-path attacks expected to matter
  - any clause/spec uncertainty
- The coder protocol should treat "emitter correctness" as insufficient. Closure requires hardening the enforcement layer, not only producing the expected happy-path artifact shape.
- Every spec-heavy packet must carry a diff-scoped self-check against the Master Spec clauses actually touched by the implementation.
- Broad repo tools in narrow packet scope must be either:
  - explicitly allowlisted by the packet, or
  - blocked by governance helpers
- The coder should assume that any machine-readable validator object can still be shallow until adversarial inputs prove otherwise.

### 12.8 Validator remediation

- The validator must derive its attack surface from the Master Spec plus the actual diff, not only from the packet's named historical gaps.
- Every active MT or clause group must receive one validator tripwire/checklist receipt before coding starts.
- Every handoff must receive one post-handoff review receipt before verdict.
- Validator review must default to adversarial proof:
  - missing required fields
  - malformed nested structures
  - constrained-string format violations
  - stale caller assumptions after semantic changes
  - counterfactual regressions on shared validators and gates
- Formal validator output must use split verdicts and must keep `NOT_PROVEN` available as a first-class result.
- When feasible, final closure should validate integrated `main` or a candidate commit intended for integration, not only the feature worktree narrative.

### 12.9 Repo governance and workflow remediation

- Make direct coder-validator communication machine-checked at real stop points:
  - pre-edit
  - handoff
  - verdict
- Add governance-specific hardening gates for those stop points so start, handoff, and verdict boundaries are proven rather than narrated.
- Add auto-triggers so the workflow itself routes the next actor:
  - coder MT selection -> notify validator
  - validator checklist posted -> notify coder
  - coder handoff posted -> trigger validator review state
- Treat repeated plugin/bridge instability as a batch-level mode switch. After the threshold is hit, the whole batch should move to explicit CLI mode and record that decision once.
- Harden session-registry writes and any shared runtime ledgers against concurrent rename/write collisions.
- Continue external runtime canonicalization and legacy-path sunset work until old repo-local authority paths can no longer poison live checks.
- Reduce coder-lane `.GOV` noise by default status filtering, explicit startup warnings, and read-only presentation where possible.
- Maintain a compatibility-shim ledger for any legacy bridge or migration patch. Each shim must record:
  - why it exists
  - what supersedes it
  - what condition allows deletion
  - the sunset trigger
- Auto-generate a post-run audit skeleton from runtime, packet, gate, session, and receipt evidence so audit quality does not depend on reconstruction from memory.

### 12.10 Execution order and workflow-proof phase

1. Phase 0: align doctrine and language across codex, role protocols, packets, and validator vocabulary so governance is explicitly treated as prototype control-plane law.
2. Land the smoke-start hard gate and transactional PREPARE/runtime writes.
3. Land machine-checked direct coder-validator communication, boundary gates, and auto-triggers.
4. Land packet-scope tool rails, formatter spill prevention, and explicit hard-correction handling.
5. Land session-registry/runtime write hardening and finish the live-authority path cleanup.
6. Run one governed workflow-proof pass focused on workflow discipline, not on new product breadth. That pass must prove:
   - clean startup
   - unified workflow truth
   - direct coder-validator exchange
   - honest split verdicts
   - materially lower Orchestrator steering load
7. Review that workflow-proof pass as its own audited result and patch only the root-cause defects it exposes.
8. Only after that baseline is stable, resume product-code remediation against the Master Spec.

### 12.11 Exit criteria for this remediation program

- A governed startup can run without live repair of split-brain workflow truth.
- Packet, runtime, task-board, session, and worktree truth agree at restart and at handoff boundaries.
- Every active packet shows the required direct coder-validator exchange on the official governed surface.
- Scope drift is either prevented or forced into explicit hard-correction immediately.
- The workflow has completed at least one workflow-proof pass without requiring broad live governance repair.
- Validator verdicts include negative-path proof and preserve `NOT_PROVEN` when the evidence ceiling is real.
- Orchestrator steering is materially reduced after startup and no longer acts as the normal review relay.
- Remaining compatibility shims are explicit, bounded, and on a documented sunset path.
- At that point, the workflow can be considered strong enough to support the next product-code remediation pass without repeating the same false-closure pattern.
