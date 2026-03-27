# Audit: WP-1 Loom Storage Portability v4 Smoketest Closeout Review

## METADATA

- AUDIT_ID: AUDIT-20260327-LOOM-STORAGE-PORTABILITY-V4-SMOKETEST-CLOSEOUT-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260327-LOOM-STORAGE-PORTABILITY-V4
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-03-27
- AUTHOR: Codex acting as Orchestrator
- HISTORICAL_BASELINE_PACKET: WP-1-Loom-Storage-Portability-v3
- ACTIVE_RECOVERY_PACKET: WP-1-Loom-Storage-Portability-v4
- LINEAGE_STATUS: LIVE_SMOKETEST_BASELINE_PENDING
- RELATED_PREVIOUS_REVIEWS:
  - AUDIT-20260326-LOOM-STORAGE-PORTABILITY-V4-SMOKETEST-REVIEW
- SCOPE:
  - historical failed-closure lineage from `WP-1-Loom-Storage-Portability-v3` through the 2026-03-26 recovery review
  - current `WP-1-Loom-Storage-Portability-v4` closeout state on local `main` at `18cb2a417534ef8dd7ffa4990e200592c1ade4ba`
  - follow-up remediation now staged on the active `feat/WP-1-Loom-Storage-Portability-v4` worktree at `../wtc-storage-portability-v4`
  - ACP runtime, validator gate, session registry, Task Board, and traceability surfaces after local closeout
- RESULT:
  - PRODUCT_REMEDIATION: PARTIAL
  - MASTER_SPEC_AUDIT: PARTIAL
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PASS
  - MERGE_PROGRESSION: PASS
- KEY_COMMITS_REVIEWED:
  - `e867469` `merge: selective Loom v3 integration from 7aa995b [WP-1-Loom-Storage-Portability-v3]`
  - `0e22102` `docs: bootstrap claim [WP-1-Loom-Storage-Portability-v4]`
  - `277dfa1` `gov: close WP-1 loom portability v4`
  - `18cb2a4` `gov: sync governance kernel d594f0a`
- EVIDENCE_SOURCES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/Audits/smoketest/AUDIT_20260326_LOOM_STORAGE_PORTABILITY_V4_SMOKETEST_REVIEW.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/task_packets/WP-1-Loom-Storage-Portability-v4/packet.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v4/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v4/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/validator_gates/WP-1-Loom-Storage-Portability-v4.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/loom.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`
  - `../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs`
  - `../wtc-storage-portability-v4/src/backend/handshake_core/src/storage/sqlite.rs`
  - `../wtc-storage-portability-v4/src/backend/handshake_core/src/storage/tests.rs`
- RELATED_GOVERNANCE_ITEMS:
  - RGF-04
  - RGF-05
  - RGF-08
- RELATED_CHANGESETS:
  - active v4 worktree delta in `../wtc-storage-portability-v4/src/backend/handshake_core/src/storage/sqlite.rs`
  - active v4 worktree delta in `../wtc-storage-portability-v4/src/backend/handshake_core/src/storage/tests.rs`
  - governance hardening in `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - governance hardening in `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

---

## 1. Executive Summary

- Fresh code-vs-spec inspection showed that `WP-1-Loom-Storage-Portability-v4` had not fully closed the base WP after `v1` through `v4`: `LoomSearchFilters.backlink_depth` was carried through the API and honored on PostgreSQL, but local `main` SQLite `search_loom_blocks` ignored it. Follow-up remediation is now staged on the active `feat/WP-1-Loom-Storage-Portability-v4` worktree: SQLite consumes `backlink_depth` for recursive graph-filter search, and the shared Loom storage conformance suite now proves that behavior on SQLite instead of short-circuiting it.
- Most of the historical `v3` dispute is still resolved on current local `main`: traversal and metrics routes exist, portable migration checks pass, directional edge readers exist, source-anchor replay survives, and the 10K-block traversal performance probes still pass in the packet harness.
- The new prevention controls are also now explicit in governance: the task-packet template forbids closing portable clauses from a single-backend proof row, and the validator protocol now requires field-by-field tracing across every declared consumer/backend.
- Packet truth, runtime truth, validator-gate truth, and ACP session truth are clean, but the live smoketest baseline should remain pending until the active v4 remediation is formally validated/contained and the closeout projections are reconciled.

## 2. Lineage and What This Run Needed To Prove

- The 2026-03-26 recovery review established that the old `v3` failure story had become stale: current `main` already carried the Loom trait, backend, search, source-anchor, and conformance surfaces that the historical narrative treated as missing.
- This closeout run therefore needed to prove four narrower truths:
  - the signed `v4` scope could close without inventing fresh Loom code churn
  - current local `main` still satisfied the exact packet clause surface on both SQLite and PostgreSQL in the present environment
  - the orchestrator-managed ACP lane could finish with closed packet/runtime/gate/session truth
  - recovery would be recorded honestly as a smoketest closeout rather than left as a stale recovery snapshot

### What Improved vs Previous Smoketest

- The largest historical implementation disputes from the previous smoketest are now substantially narrower. The final-lane validator reran:
  - `sqlite_loom_storage_conformance`
  - `sqlite_loom_traversal_performance_target`
  - `postgres_loom_storage_conformance`
  - `postgres_loom_traversal_performance_target`
  and all four passed on current local `main`.
- Focused follow-up checks also passed:
  - `api::loom::tests::graph_traversal_and_metrics_routes_work`
  - `storage::tests::loom_migration_schema_is_portable_sqlite`
- Follow-up remediation after that audit now exists on the active v4 worktree:
  - SQLite `search_loom_blocks` consumes `backlink_depth` for recursive tag/mention graph filtering
  - the shared helper `loom_search_graph_filter_portable` now runs inside the common conformance suite for SQLite as well as PostgreSQL
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact --nocapture` passed after the fix
- The workflow is materially cleaner than the 2026-03-26 recovery review:
  - `just validator-handoff-check WP-1-Loom-Storage-Portability-v4` passed
  - `just integration-validator-closeout-check WP-1-Loom-Storage-Portability-v4` passed
  - `just validator-packet-complete WP-1-Loom-Storage-Portability-v4` passed
  - `just session-control-runtime-check` passed
  - `just gov-check` passed after packet-template and validator-protocol hardening
  - coder, WP validator, and integration validator ACP sessions were all launched and then closed cleanly
- What did not improve enough:
  - current local `main` and packet closeout truth still predate the active v4 remediation delta
  - the base-WP traceability projection still labels the active packet like a stub-backlog item
  - packet/runtime/gate truth still says `Validated (PASS)` even though the fresh historical-smoketest judgment remains pending

## 3. Product Outcome

- The closeout audit did uncover one real remaining product gap on local `main`, and follow-up remediation is now staged on the active `feat/WP-1-Loom-Storage-Portability-v4` worktree rather than already contained on `main`.
- Most of the base-WP surface is present on `main`:
  - storage-trait methods exist for `get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, and `recompute_all_metrics`
  - SQLite and PostgreSQL backend implementations exist
  - `LoomSearchFilters` and `LoomSourceAnchor` contract types exist
  - shared conformance and traversal entrypoints exist and passed in this audit run
  - traversal/metrics API routes are live and route-level tests pass
  - portable migration assertions pass on SQLite
- The original remaining base-WP gap was:
  - `api/loom.rs` accepts `backlink_depth` and forwards it into the shared `LoomSearchFilters` contract
  - `postgres.rs` consumes `filters.backlink_depth` to widen graph-relationship search semantics
  - local `main` `sqlite.rs` never consumed `filters.backlink_depth` inside `search_loom_blocks`
  - the shared conformance suite only probed backlink-depth semantics on PostgreSQL, so this contract drift was not caught by current packet tests
- That gap is now remediated on the active v4 worktree:
  - `src/backend/handshake_core/src/storage/sqlite.rs` adds recursive graph-filter handling for `backlink_depth` on both `tag_ids` and `mention_ids`
  - `src/backend/handshake_core/src/storage/tests.rs` renames the graph-filter helper to `loom_search_graph_filter_portable` and runs it through the shared Loom storage conformance path for SQLite as well as PostgreSQL
  - focused SQLite conformance now passes with that proof in the common suite
- Adjacent debt also remains:
  - PostgreSQL follow-up rerun in this turn is still environment-gated because `POSTGRES_TEST_URL` is unset here
  - downstream Loom bridge and archive integration work remains in separate stubs and was not part of `v4`

## 4. Timeline

- 2026-03-26T14:43:56Z:
  - `WP-1-Loom-Storage-Portability-v4` communication artifacts initialized
- 2026-03-26T18:25:14Z:
  - `WP_VALIDATOR` issued `VALIDATOR_KICKOFF`
- 2026-03-26T18:34:46Z:
  - `CODER` issued `CODER_INTENT`
- 2026-03-26T18:57:09Z:
  - `CODER` issued `CODER_HANDOFF` with proof-only framing
- 2026-03-26T19:02:13Z:
  - `WP_VALIDATOR` issued advisory `VALIDATOR_REVIEW`
- 2026-03-26T19:05:42Z:
  - `INTEGRATION_VALIDATOR` opened final review via `REVIEW_REQUEST`
- 2026-03-26T19:08:04Z:
  - `CODER` issued `REVIEW_RESPONSE`
- 2026-03-26T23:20:06Z:
  - packet records `CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE`
- 2026-03-26T23:39:49Z:
  - packet records `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN` at `18cb2a417534ef8dd7ffa4990e200592c1ade4ba`
- 2026-03-26T23:42:33Z to 2026-03-26T23:47:25Z:
  - runtime and orchestrator heartbeats settle to `completed / STATUS_SYNC / CLOSED`
- 2026-03-26T23:58:27Z:
  - validator gate ledger reaches `USER_ACKNOWLEDGED`
- 2026-03-27:
  - this closeout smoketest review records the post-closeout state, the remaining product/spec gap, and the still-split status projections

## 5. Failure Inventory

### 5.1 High: local `main` still lacks portable `backlink_depth` parity, even though follow-up remediation now exists on the active v4 worktree

Evidence:

- `Handshake_Master_Spec_v02.178.md` states that `LoomSearchFilters` is a canonical portable backend contract whose meaning must survive SQLite and PostgreSQL, and `[LM-SEARCH-001]` requires a backend-agnostic search API
- `src/backend/handshake_core/src/api/loom.rs` forwards `backlink_depth` into `LoomSearchFilters`
- `src/backend/handshake_core/src/storage/postgres.rs` consumes `filters.backlink_depth`
- local `main` `src/backend/handshake_core/src/storage/sqlite.rs` never reads `filters.backlink_depth` in `search_loom_blocks`
- local `main` `src/backend/handshake_core/src/storage/tests.rs` only exercises backlink-depth graph filtering on PostgreSQL
- active v4 worktree `src/backend/handshake_core/src/storage/sqlite.rs` now consumes `filters.backlink_depth`
- active v4 worktree `src/backend/handshake_core/src/storage/tests.rs` now executes the same graph-filter proof as a portable conformance helper
- `WP-1-Loom-Storage-Portability-v2` explicitly defined portable search-filter meaning across both backends as in-scope and required parity to be proven by tests, not inferred

Reason:

- the packet lineage proved PostgreSQL graph-filter semantics and general search parity, but it never re-established the same `backlink_depth` meaning on SQLite before packet closeout
- the follow-up remediation now exists, but it is not yet contained in local `main` or reflected in the packet closeout truth

Impact:

- the same shared `LoomSearchFilters` request still produces different graph-filter behavior on current local `main` depending on backend
- the base WP cannot yet be called historically recovered until the follow-up delta is validated/contained
- the packet and earlier closeout language overstated recovery

Judgment:

- this was the main remaining product/spec gap after `v1`, `v2`, `v3`, and `v4`
- the product fix is now present on the active v4 worktree, but the historical closeout baseline remains pending until that delta is formally carried through

### 5.2 Medium: Packet closeout truth and historical-lineage truth now disagree

Evidence:

- `TASK_BOARD.md` still lists `WP-1-Loom-Storage-Portability-v3` with `live_status: LIVE_SMOKETEST_BASELINE_PENDING`
- `WP_TRACEABILITY_REGISTRY.md` still lists the Loom historical lineage row as `LIVE_SMOKETEST_BASELINE_PENDING`
- packet metadata, runtime status, validator gate ledger, and session registry all show the recovery packet closed

Reason:

- packet/runtime/gate/session surfaces were closed before this fresh product-vs-spec audit uncovered the remaining SQLite portable-search gap
- the repo has no authoritative settlement path for "packet-level PASS exists, but the latest smoketest review still keeps the historical baseline pending"

Impact:

- readers have to choose between packet closure truth and historical-smoketest pending truth
- later review work keeps paying read-amplification cost across multiple surfaces

Judgment:

- this does not create the product gap, but it leaves the repo without one coherent status story

### 5.3 Medium: The base-WP traceability projection still labels the active Loom packet like a stub-backlog item

Evidence:

- `WP_TRACEABILITY_REGISTRY.md` maps `WP-1-Loom-Storage-Portability` to `.GOV/task_packets/WP-1-Loom-Storage-Portability-v4/packet.md`
- the same row still projects `Task Board: Stub Backlog (Not Activated): WP-1-Loom-Storage-Portability-v4`
- `TASK_BOARD.md` itself already lists `WP-1-Loom-Storage-Portability-v4` under `## Done` as `[VALIDATED]`

Reason:

- the base-WP projection text was not refreshed when the stub became an activated packet and later validated

Impact:

- the registry sends mixed signals about whether the active Loom packet is real, activated, or closed
- future orchestrators and validators have to re-check packet truth manually instead of trusting the projection layer

Judgment:

- this is a real path/status ambiguity bug, not harmless wording residue

### 5.4 Low: The direct-review receipt chain still reflects the pre-rerun environment story rather than the final proof state

Evidence:

- `RECEIPTS.jsonl` records the 2026-03-26 review loop where PostgreSQL proof was still env-gated because `POSTGRES_TEST_URL` was absent
- the final validator-owned reruns that proved PostgreSQL now live in the packet validation report and closeout evidence rather than in a fresh coder-validator receipt cycle
- the final review request/response correlation id was malformed as `2.3.13.7 Loom Storage Trait`

Reason:

- closeout was completed by validator-owned proof, packet mutation, and gate settlement rather than by minting a fresh governed direct-review exchange after the new PostgreSQL reruns

Impact:

- readers need both the receipt chain and the final validation report to reconstruct the full truth
- the historical review trace is honest, but not as crisp as it should be

Judgment:

- acceptable for this closeout, but still a communication-shape defect worth fixing

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- kept `v4` narrow as a zero-delta proof/status-sync packet instead of inventing new Loom code churn
- launched and closed the ACP coder, WP validator, and integration validator lanes without creating extra worktrees
- carried the packet through final closeout, validator-gate settlement, and runtime/session closure

Failures:

- did not reconcile packet closeout truth with the still-pending historical smoketest judgment after the fresh audit
- still relies on multiple separate governance surfaces to express one real base-WP state

Assessment:

- materially improved from the prior recovery run, but the recovered-baseline claim was still too strong because one portable-contract gap remained uncalled

### 6.2 Coder Review

Strengths:

- did not invent a false product defect
- kept the packet honest as proof-only unless a fresh current-main defect appeared
- preserved the no-extra-worktree, no-speculative-churn boundary

Failures:

- the lineage still accepted `LoomSearchFilters` as portable without checking whether SQLite honored `backlink_depth`
- the final proven PostgreSQL reruns landed in validator-owned closeout evidence rather than a fresh coder handoff cycle

Assessment:

- disciplined and appropriately narrow, but still missed a real shared-contract gap

### 6.3 WP Validator Review

Strengths:

- refused false dual-backend closure in the earlier env-gated state
- kept the packet narrowly framed around current-main proof rather than stale historical narrative
- left a review trail that remained truthful through closeout

Failures:

- the advisory validator let `[LM-SEARCH-001]` close without independent proof that SQLite preserved `backlink_depth` semantics

Assessment:

- strong skepticism on historical false closure, but incomplete on portable search-filter parity

### 6.4 Integration Validator Review

Strengths:

- reran all four packet-level Loom tests on current local `main`
- passed closeout preflight, appended `PASS`, confirmed main containment, and completed the validator gate ledger
- kept the final verdict scoped to what the current environment actually proved

Failures:

- the final-lane review accepted `MAIN_BODY_GAPS: NONE` despite the SQLite/`backlink_depth` contract hole
- the final-lane completion left packet PASS and historical-lineage pending truth unresolved against each other

Assessment:

- strong technical closeout mechanics, but not fully spec-tight

## 7. Review Of Coder and Validator Communication

- The direct review lane is real and machine-verifiable:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - `CODER_HANDOFF`
  - `VALIDATOR_REVIEW`
  - `REVIEW_REQUEST`
  - `REVIEW_RESPONSE`
- The communication quality remained a positive signal:
  - no one invented a fake Loom implementation gap
  - no one overclaimed dual-backend PASS while PostgreSQL was still unproven
  - the packet stayed truthfully framed as proof-only until validator-owned reruns closed the remaining evidence gap
- The weak parts:
  - the review loop never surfaced the SQLite `backlink_depth` parity hole even though the portable contract was in scope
  - the receipt chain still reflects the pre-rerun env-gated state
  - final proof lives in the packet validation report and gate ledger rather than in a fresh direct-review exchange
  - the malformed final-lane correlation id should not be normalized

## 8. ACP Runtime / Session Control Findings

- ACP runtime truth is now clean for this WP:
  - `RUNTIME_STATUS.json` shows `Validated (PASS)`, `CONTAINED_IN_MAIN`, `completed`, `STATUS_SYNC`, `NONE`, and `CLOSED`
  - `ROLE_SESSION_REGISTRY.json` shows coder, WP validator, and integration validator sessions closed
  - validator gates show `WP_APPENDED`, `COMMITTED`, `REPORT_PRESENTED`, and `USER_ACKNOWLEDGED`
- No extra worktrees were created. The declared topology remained:
  - prepare/coder/WP validator work on `../wtc-storage-portability-v4`
  - integration validation on `../handshake_main`
  - orchestrator on `../wt-gov-kernel`
- Remaining drift is not ACP runtime drift anymore. The unresolved issues are product/spec parity plus projection drift.

## 9. Governance Implications

- This run proves that green packet/runtime/gate/session truth is not enough to call a historical smoketest recovered. A zero-delta packet can still overclose if validator review does not attack the surviving shared-contract surfaces directly.
- It also proves that "all packet tripwire tests passed" is not enough for a portable-contract claim when a shared filter field is only exercised on one backend.
- The repo still lacks an atomic "smoketest recovery closeout sync" that settles:
  - packet status
  - runtime status
  - validator gate status
  - Task Board historical live status
  - traceability live status and latest smoketest review pointer

## 10. Positive Signals Worth Preserving

- zero-delta closure remained legal only because `ZERO_DELTA_PROOF_ALLOWED=YES` and current-main proof was rerun explicitly
- the final-lane validator reran all four packet tests before `PASS`
- focused follow-up checks confirmed traversal/metrics route exposure and SQLite migration portability
- the active v4 follow-up now closes the SQLite `backlink_depth` hole and moves the parity proof into the shared conformance suite
- governance now encodes the prevention rule in both the packet template and validator protocol instead of leaving it as reviewer memory
- the direct-review loop remained honest under pressure and did not manufacture a false product delta
- the ACP topology stayed inside the declared worktrees and all role sessions were closed at the end
- validator-gate `USER_ACKNOWLEDGED` gives this closeout a cleaner auditable end-state than the prior recovery snapshot

## 11. Remaining Product or Spec Debt

- The original main remaining gap is now patched on the active v4 worktree, but current local `main` and packet closeout projections do not yet reflect that remediation.
- PostgreSQL Loom proof is still not turn-key. It requires a reachable local database and `POSTGRES_TEST_URL`, so the repo still does not provide one-command environment provisioning for this portability lane.
- The prevention hardening is landed in governance, but it has not yet been exercised by a fresh packet creation/validation cycle outside this Loom lineage.
- This closeout does not address broader downstream Loom follow-ons such as `WP-1-Media-Downloader-Loom-Bridge-v1` or `WP-1-Video-Archive-Loom-Integration-v1`.
- The remaining product debt is now much narrower than the old `v3` narrative, but the closeout/containment gap is still packet-relevant rather than purely adjacent.

## Post-Smoketest Improvement Rubric

### Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - the 2026-03-26 recovery review ended with split packet/runtime/task-board truth and failed closeout
  - this closeout now has coherent packet/runtime/gate/session truth and clean ACP session shutdown
  - packet/runtime/gate truth still disagrees with the latest historical-smoketest judgment
- What improved:
  - the run reached actual closeout instead of stopping at recovery analysis
  - validator handoff, integration closeout, packet-complete, and session-control checks all passed
  - no extra worktree churn occurred during closeout
- What still hurts:
  - historical-lineage settlement is still not atomic with technical closeout or later review corrections
  - readers still have to compare several governance surfaces to know whether the baseline is actually recovered
- Next structural fix:
  - derive historical smoketest lineage status from the latest smoketest review record, not only from packet closeout gates

### Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - all four packet-level Loom tests passed on current local `main`
  - traversal/metrics route and SQLite migration portability checks passed in this audit
  - current local `main` review showed `backlink_depth` was only consumed on PostgreSQL, not SQLite
  - the active v4 worktree now consumes `backlink_depth` on SQLite and the shared conformance suite proves it there
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact --nocapture` passed after the fix
- What improved:
  - the gap surface is much smaller than the old historical `v3` failure story
  - traversal, metrics, migrations, source-anchor durability, and performance proof are all materially better grounded than before
  - the SQLite `backlink_depth` hole is now fixed on the active v4 worktree instead of only documented as an audit finding
  - future packets/validators now have an explicit field-parity rule in governance
- What still hurts:
  - current local `main` and packet closeout truth still predate the fix
  - PostgreSQL follow-up rerun for the active v4 delta is still blocked by missing `POSTGRES_TEST_URL`
- Next structural fix:
  - validate/contain the active v4 remediation on local `main`, then keep the new field-parity rule mandatory for future packets

### Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - restart-to-closeout was shorter and more direct than the 2026-03-26 recovery run
  - no extra worktrees or speculative product edits were created
  - review creation still required cross-checking packet, runtime, gate ledger, session registry, Task Board, and traceability registry because projection truth is split
- What improved:
  - most of the costlier protocol conflict from the previous run is gone
  - closeout focused on proof and record settlement rather than re-litigating stale implementation narratives
- What still hurts:
  - stale lineage projections mean later readers will still pay read-amplification cost
  - the final proof story is split between receipts, packet validation report, and gate ledger
- Next structural fix:
  - add a single smoketest-closeout sync helper that writes the review pointer and reconciles all lineage/status projections in one step

## Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- packet validation and the first draft of this review both overclaimed closure while SQLite still ignored one shared search-contract field
- packet/runtime/gate/session surfaces can all look green while the historical-smoketest lineage still reads pending in Task Board and traceability views
- the base-WP traceability row can make an active validated packet look like an unactivated stub

### 13.2 Systematic Wrong Tool or Command Calls

- NONE in this closeout pass
- historical residue remains in the malformed final-lane correlation id `2.3.13.7 Loom Storage Trait`

### 13.3 Task and Path Ambiguity

- the base WP and the active packet now disagree at the projection layer about whether Loom `v4` is a real validated packet or a stub-backlog item
- historical-lineage tables and packet/runtime truth disagree about whether the live smoketest baseline is recovered

### 13.4 Read Amplification / Governance Document Churn

- confirming the actual closeout state still required reading:
  - packet metadata and validation report
  - runtime status
  - validator gate ledger
  - session registry
  - Task Board
  - traceability registry
  - receipt history
- that amount of re-reading is evidence that the closeout projection surface is still too fragmented

### 13.5 Hardening Direction

- keep the new packet-template rule that blocks portable-contract closure from a single-backend proof row
- keep the new validator field-consumption rule that traces shared fields across every declared consumer/backend
- keep the new shared `LoomSearchFilters` parity probe that exercises `backlink_depth` in the common conformance suite
- treat historical-lineage status and latest smoketest review ids as required authoritative projections, not passive notes
- keep final review exchange ids machine-generated even for zero-delta proof packets
- add a deterministic projection check that fails when an active validated packet still projects as stub backlog in the traceability registry

## 14. Suggested Remediations

### Governance / Runtime

- sync `TASK_BOARD.md` and `WP_TRACEABILITY_REGISTRY.md` historical-lineage fields from the latest smoketest review outcome, even when that outcome disagrees with an earlier packet-level PASS
- add a closeout projection check that rejects base-WP rows still projecting stub backlog after packet activation
- harden final-lane review wrappers so generated correlation ids are always preserved

### Product / Validation Quality

- carry the active v4 SQLite `backlink_depth` remediation and shared parity test into validated local `main`
- add deterministic local PostgreSQL bootstrap guidance or helper commands for Loom portability proof
- keep zero-delta packet closure restricted to cases with explicit current-main compatibility proof and validator-owned reruns

### Documentation / Review Practice

- use this review, not the 2026-03-26 recovery review, as the current Loom `v4` smoketest reference
- when a recovery packet closes without product diff, require the review to state clearly that the code already lived on `main` and the run earned proof rather than new implementation

## 15. Command Log

- `Get-Content -Raw .GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md` -> PASS
- `Get-Content -Raw .GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md` -> PASS
- `Get-Content -Raw .GOV/Audits/smoketest/AUDIT_20260326_LOOM_STORAGE_PORTABILITY_V4_SMOKETEST_REVIEW.md` -> PASS
- `Get-Content -Raw .GOV/task_packets/WP-1-Loom-Storage-Portability-v4/packet.md` -> PASS
- `Get-Content -Raw ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v4/RUNTIME_STATUS.json` -> PASS
- `Get-Content -Raw ../gov_runtime/roles_shared/validator_gates/WP-1-Loom-Storage-Portability-v4.json` -> PASS
- `Get-Content -Raw ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json` -> PASS
- `Get-Content -Raw .GOV/roles_shared/records/TASK_BOARD.md` -> PASS
- `Get-Content -Raw .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` -> PASS
- `rg -n "backlink_depth" src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/api/loom.rs src/backend/handshake_core/src/storage/tests.rs` -> PASS (revealed that `backlink_depth` is consumed in API/PostgreSQL/tests but not in SQLite)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml api::loom::tests::graph_traversal_and_metrics_routes_work -- --exact --nocapture` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::loom_migration_schema_is_portable_sqlite -- --exact --nocapture` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml sqlite_loom_storage_conformance -- --exact --nocapture` -> PASS (active v4 worktree after SQLite `backlink_depth` remediation)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml sqlite_loom_traversal_performance_target -- --exact --nocapture` -> PASS
- `$env:POSTGRES_TEST_URL='postgres://postgres:postgres@localhost:5432/handshake_test'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_loom_storage_conformance -- --exact --nocapture` -> PASS
- `$env:POSTGRES_TEST_URL='postgres://postgres:postgres@localhost:5432/handshake_test'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_loom_traversal_performance_target -- --exact --nocapture` -> PASS
- `if ($env:POSTGRES_TEST_URL) { 'SET' } else { 'UNSET' }` -> PASS (`UNSET` in the active v4 follow-up turn; no fresh PostgreSQL rerun was available here)
- `just validator-handoff-check WP-1-Loom-Storage-Portability-v4` -> PASS
- `$env:HANDSHAKE_GOV_ROOT='..\\wt-gov-kernel\\.GOV'; just integration-validator-closeout-check WP-1-Loom-Storage-Portability-v4` -> PASS
- `$env:HANDSHAKE_GOV_ROOT='..\\wt-gov-kernel\\.GOV'; just validator-packet-complete WP-1-Loom-Storage-Portability-v4` -> PASS
- `just session-control-runtime-check` -> PASS
- `just gov-check` -> PASS
- `git -C ../handshake_main log --oneline --decorate -n 8` -> PASS
- `git -C ../wt-gov-kernel log --oneline --decorate -n 8` -> PASS
- `rg -n "WP-1-Loom-Storage-Portability-v3|WP-1-Loom-Storage-Portability-v4|LIVE_SMOKETEST_BASELINE_" .GOV` -> PASS
