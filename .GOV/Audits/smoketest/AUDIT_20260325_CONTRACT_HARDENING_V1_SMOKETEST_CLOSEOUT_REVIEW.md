# Audit: WP-1 Structured Collaboration Contract Hardening v1 Smoketest Closeout Review

## METADATA
- AUDIT_ID: AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-03-25
- AUTHOR: Codex acting as Orchestrator
- RELATED_PREVIOUS_REVIEWS:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
- SCOPE:
  - `WP-1-Structured-Collaboration-Schema-Registry-v4` smoketest recovery review as the immediate predecessor baseline
  - `WP-1-Structured-Collaboration-Contract-Hardening-v1` as the follow-on closeout pass
  - Integrated product code on `main` at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`
  - Workflow, communication, ACP runtime, and governance behavior observed during the orchestrator-managed run
- RESULT:
  - PRODUCT_REMEDIATION: PASS
  - MASTER_SPEC_AUDIT: PARTIAL; the signed contract-hardening scope is closed, but broader adjacent structured-collaboration spec debt still exists
  - WORKFLOW_DISCIPLINE: PARTIAL; the direct review loop and merge progression were materially stronger than v4, but final status sync still needed governance/runtime repair
  - ACP_RUNTIME_DISCIPLINE: FAIL; session-control truth still required manual settlement to become honest
  - MERGE_PROGRESSION: PASS; the reviewed product fix was merged into `main` inside the governed run
- KEY_COMMITS_REVIEWED:
  - `7467671` initial six-file contract-hardening implementation handed to the WP validator
  - `92d9032` mailbox redaction repair after validator FAIL
  - `c6e8ba2` merged `main` closeout commit
- EVIDENCE_SOURCES:
  - `.GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md`
  - `.GOV/task_packets/WP-1-Structured-Collaboration-Contract-Hardening-v1/packet.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Contract-Hardening-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-1-Structured-Collaboration-Contract-Hardening-v1/*.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Structured-Collaboration-Contract-Hardening-v1/*.jsonl`
  - `../handshake_main/src/backend/handshake_core/src/locus/task_board.rs`
  - `../handshake_main/src/backend/handshake_core/src/locus/types.rs`
  - `../handshake_main/src/backend/handshake_core/src/role_mailbox.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - `../handshake_main/src/backend/handshake_core/src/workflows.rs`
  - `../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs`
  - `../handshake_main/src/backend/handshake_core/tests/role_mailbox_tests.rs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-03`
  - `RGF-04`
  - `RGF-05`
- RELATED_CHANGESETS:
  - `GOV-CHANGE-20260325-04`

---

## 1. Executive Summary

The contract-hardening follow-on run did the product work that the prior Schema Registry v4 smoketest review said still remained. The integrated `main` tree now has registry-backed `allowed_action_ids`, Task Board authoritative-field drift checks, stronger mailbox redaction validation, and the negative-path proof that was missing before.

This was also a better workflow run than v4. The governed coder <-> WP validator loop worked, the validator found one real remaining defect, the coder repaired it in one revision cycle, and the integration validator merged the approved diff to `main` inside the run. That closes the most serious v4 governance failure: validated PASS without actual mainline containment.

The workflow is still not clean enough to call solved. The run required a session-control truth repair for orphaned request `f13de206-76ee-4e8b-aa73-a8218187d9df`, and the final closeout also needed a `validator-packet-complete` parser fix before the repo could truthfully settle green. So the product remediation passed, but ACP/runtime/governance friction remains real.

---

## 2. Lineage and What This WP Needed To Prove

The previous smoketest review for `WP-1-Structured-Collaboration-Schema-Registry-v4` ended with three remaining product gaps against the Master Spec:

1. `allowed_action_ids` still behaved like generic string arrays instead of registry-backed governed action ids.
2. Task Board rows still risked board-status-first heuristics instead of preserving authoritative workflow semantics.
3. Role Mailbox export validation still trusted redacted-field emitters more than it mechanically proved leak-safe outputs.

`WP-1-Structured-Collaboration-Contract-Hardening-v1` was the correct follow-on because it isolated exactly those remaining contract gaps and required negative-path proof for each one.

The packet `DONE_MEANS` required:

- canonical structured-collaboration producers emit only registered `GovernedActionDescriptorV1.action_id` values
- the shared validator rejects unregistered or malformed `allowed_action_ids`
- Task Board rows preserve authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`
- RoleMailbox export validation rejects malformed, unbounded, or non-redacted `subject_redacted` and `note_redacted`
- negative-path tests prove the failures are mechanically blocked

### What Improved vs Previous Smoketest

- The three concrete product gaps left open by the Schema Registry v4 smoketest review are closed here:
  - `allowed_action_ids` is now registry-backed rather than treated as a generic string array.
  - Task Board rows now have authoritative-field drift checks instead of trusting board-status-first semantics.
  - mailbox redaction validation now rejects single-line leak-after-token cases rather than only multiline drift.
- Merge progression is materially improved:
  - unlike the v4 recovery run, this WP reached `main` inside the governed run at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`.
- The validator loop is also stronger:
  - the WP validator caught one real remaining defect, the coder repaired it in one cycle, and the final PASS was narrower and more credible than the earlier v4 closure.
- The remaining failures are narrower than the earlier smoketest:
  - the biggest residual problems are now ACP/session-control settlement and closeout parser reliability, not product incompleteness or merge omission.

---

## 3. Product Outcome

The signed WP scope is materially closed on `main` at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`.

What was fixed:

- `src/backend/handshake_core/src/locus/types.rs`
  - registry-backed `allowed_action_ids` validation replaced generic string-array acceptance
  - canonical redacted-field validation now rejects malformed single-line leak-after-token values as well as multiline drift
- `src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - SQLite micro-task progress metadata now emits governed action ids through the same registry-backed path as the main emitters
- `src/backend/handshake_core/src/workflows.rs`
  - canonical structured-collaboration emitters now use governed action descriptors instead of local ad hoc verb lists
  - authoritative workflow-state triplets flow into Task Board projection surfaces
- `src/backend/handshake_core/src/locus/task_board.rs`
  - Task Board validation now rejects rows whose workflow-state triplet drifts from authoritative backend truth
- `src/backend/handshake_core/src/role_mailbox.rs`
  - mailbox export and validation now preserve bounded redaction semantics more defensively
- tests
  - `micro_task_executor_tests.rs` now proves rejection of unregistered governed actions and authoritative row drift
  - `role_mailbox_tests.rs` now proves rejection of malformed redacted mailbox fields and malformed export thread-line payloads

Judgment:

- compared with the v4 predecessor review, the remaining product debt identified there is closed
- the strongest current product-adjacent gap is broader and outside this packet: some prose `next_action` strings in `workflows.rs` are still not modeled through the governed action descriptor registry

That broader adjacency is real, but it does not invalidate this WP's signed scope.

---

## 4. Timeline

- 2026-03-25 04:34 UTC:
  - packet and runtime communication surfaces initialized for `WP-1-Structured-Collaboration-Contract-Hardening-v1`
- 2026-03-25 05:54 UTC:
  - WP validator issued governed kickoff focused on governed action ids, Task Board workflow-state fidelity, mailbox redaction validation, and deterministic evidence
- 2026-03-25 06:04 UTC:
  - coder handed off initial implementation on `7467671`
- 2026-03-25 06:25 UTC:
  - WP validator issued FAIL
  - concrete defect: `canonical_redacted_secret_output()` still accepted any single-line value containing `[REDACTED]`, so leaked mailbox text could survive validation
- 2026-03-25 06:51 UTC:
  - coder handed off repaired implementation on `92d9032`
- 2026-03-25 07:07 UTC:
  - WP validator issued PASS on the repaired mailbox path, governed action registry, and Task Board authoritative-field probes
- 2026-03-25 07:38 UTC:
  - integration validator issued PASS and merged to `main` at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`
- 2026-03-25 07:50 UTC onward:
  - status sync completed, but honest closeout also required:
    - a settled session-control result row for orphaned request `f13de206-76ee-4e8b-aa73-a8218187d9df`
    - a parser fix in `.GOV/roles/validator/checks/validator-packet-complete.mjs`

---

## 5. Failure Inventory

### 5.1 High: the first product handoff was still incomplete on mailbox leak-safety

Evidence:

- WP validator FAIL receipt at 2026-03-25T06:25:03Z
- thread summary: `canonical_redacted_secret_output()` accepted any single-line value containing `[REDACTED]`

Reason:

- the first implementation closed registry-backed action ids and Task Board projection fidelity, but the mailbox redaction validator still whitelisted leaked surrounding text if a redaction marker was present

Impact:

- the first handoff could not honestly claim leak-safe mechanical validation
- one additional coder revision cycle was required

Judgment:

- this was a real product miss
- the WP validator did the right thing by failing it

### 5.2 High: ACP session-control truth still required manual settlement

Evidence:

- `SESSION_CONTROL_REQUESTS.jsonl` contains request `f13de206-76ee-4e8b-aa73-a8218187d9df`
- the request output log recorded a concurrent-run rejection
- a settled result row only appeared later with summary noting that the broker wrote a rejection event but no settled result row existed

Reason:

- the control plane still allowed a request/output state where the human could infer failure from the output log, but the canonical result ledger remained incomplete

Impact:

- closeout truth depended on repair rather than low-touch automation
- `just session-control-runtime-check` was not honestly green until the result row existed

Judgment:

- this remains an ACP runtime/control-plane failure
- it directly reinforces `RGF-05`

### 5.3 Medium: validator closeout parsing still was not reliable enough

Evidence:

- final closeout required a fix in `.GOV/roles/validator/checks/validator-packet-complete.mjs`

Reason:

- the closeout parser did not robustly accept the validated status format used by the packet

Impact:

- a technically valid packet could still fail the final governance check until parser logic was corrected

Judgment:

- this is governance/runtime harness debt, not product debt
- it directly reinforces `RGF-04`

### 5.4 Medium: runtime closeout still depended on repair work after the technical merge was already correct

Evidence:

- merged `main` commit was already present at `c6e8ba2bf23ff9061b20f83a31567a6e47b322fe`
- status sync still needed runtime/result repair and parser correction before all governance surfaces turned green

Reason:

- product truth and governance-runtime truth still do not settle atomically

Impact:

- the repo remains vulnerable to "product is right, but workflow truth is still noisy"

Judgment:

- this is milder than the v4 merge-omission failure
- it is still a workflow-hardening problem that should not be normalized

---

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- chose the right follow-on packet from the previous smoke review
- kept the scope narrow and product-relevant
- respected the post-v4 helper-agent boundary and did not delegate product code
- carried the run through merge to `main`

Failures:

- still had to repair session-control truth and validator closeout mechanics after the product merge
- allowed the run to depend on governance/runtime repair rather than refusing partial closeout earlier

Assessment:

- much better than the v4 recovery run
- still too tolerant of control-plane imperfection during final settlement

### 6.2 Coder Review

Strengths:

- stayed on the intended product surface
- closed the governed action registry and Task Board fidelity requirements on the first substantial pass
- responded to validator FAIL in one revision cycle
- reran scoped and broader proof after the mailbox repair

Failures:

- first handoff still overstated mailbox redaction safety

Assessment:

- strong run overall
- mailbox redaction semantics needed a stricter self-audit before first handoff

### 6.3 WP Validator Review

Strengths:

- issued a concrete, code-anchored FAIL on the real remaining defect
- did not reopen already-sound governed action registry or Task Board findings unnecessarily
- passed the second handoff narrowly and for the right reason

Failures:

- no material current-run validator miss was observed

Assessment:

- this is the strongest role performance in the run
- the advisory validator added real signal and prevented a shallow PASS

### 6.4 Integration Validator Review

Strengths:

- issued the final PASS on the repaired diff
- merged the approved product fix into `main` during the governed run
- removed the biggest v4 closeout ambiguity by making mainline containment real

Failures:

- final closeout still relied on surrounding governance/runtime repair before everything settled green

Assessment:

- materially better than the v4 integration lane
- merge authority acted correctly, but the surrounding kernel still needs hardening

---

## 7. Review Of Coder and Validator Communication

This was a good governed review loop.

What worked:

- the required direct-review receipts were all present
- the sequence was coherent:
  - `VALIDATOR_KICKOFF`
  - `CODER_INTENT`
  - first `CODER_HANDOFF`
  - `VALIDATOR_REVIEW` FAIL
  - second `CODER_HANDOFF`
  - `VALIDATOR_REVIEW` PASS
  - final coder handoff to integration validator
  - integration-validator PASS and merge receipt
- the validator named one concrete defect instead of replying with generic "not enough proof" language
- the coder's second handoff explicitly described the repaired boundary and the rerun proof

What still concerns me:

- the communication lane itself is now ahead of the control plane around it
- a good direct review contract still cannot fully hide session-control or closeout-settlement defects

Net judgment:

- communication quality is now a positive pattern worth preserving
- the system still needs better final-lane reliability so good communication converts into low-drama completion

---

## 8. ACP Runtime / Session Control Findings

The run did not repeat the v4 merge-omission problem, but it still proved that ACP/runtime truth is not mechanically trustworthy enough yet.

Key findings:

- orphaned request/result truth remains possible
  - request `f13de206-76ee-4e8b-aa73-a8218187d9df` needed a later terminal result row to make runtime truth honest
- closeout helpers still are not atomic
  - product merge, validator packet parsing, runtime status truth, and final governance-green state did not settle in one uninterrupted path
- runtime status itself was coherent at the end
  - `RUNTIME_STATUS.json` records `current_packet_status: "Validated (PASS)"`, `runtime_status: "completed"`, and the orchestrator status-sync heartbeat

Net judgment:

- runtime truth can be recovered
- it is still too easy for truth to require repair instead of emerging cleanly from the governed helpers

---

## 9. Governance Implications

This closeout review changes the governance picture in two important ways.

First, `RGF-03` is now more sharply defined. The repo no longer has to argue from the v4 negative case alone; this run proves the desired positive case too. A WP can and should reach validated PASS with actual `main` containment inside the governed run.

Second, `RGF-04` and `RGF-05` are now reinforced by fresh evidence. Even after a better orchestrator-managed run, final settlement still required:

- parser hardening
- manual session-control result settlement

So the remaining governance work is no longer speculative. It is directly tied to failures observed in an otherwise successful proof run.

---

## 10. Positive Signals Worth Preserving

- the product follow-on scope was correct and economically sized
- the WP validator caught the one real remaining defect
- the coder recovered in one revision cycle
- merge authority actually merged the fix to `main`
- the merged tree on `main` matches the intended contract-hardening surface
- the run produced a much more trustworthy product outcome than the earlier v4 recovery pass

---

## 11. Remaining Product or Spec Debt

This WP closes the remaining product gaps named by the previous Schema Registry v4 smoketest review. It does not prove that all structured-collaboration adjacent spec debt everywhere is gone.

The main adjacent open item still visible from the final validation report is:

- some prose `next_action` strings in `workflows.rs` still are not modeled through the governed action descriptor registry

That is broader than this packet and should not be used to rewrite this WP as incomplete. It should, however, remain visible as adjacent follow-on debt.

---

## 12. Suggested Remediations

### Governance / Runtime

- implement `RGF-03` so validated PASS and main containment stay mechanically linked
- implement `RGF-04` so final closeout either settles completely or fails before writing partial truth
- implement `RGF-05` so every control request gets exactly one terminal result row without repair
- add a closeout self-check that compares request/output/result cardinality before declaring runtime truth clean

### Product / Validation Quality

- keep the direct coder <-> WP validator review contract mandatory for high-risk contract WPs
- require mailbox-leak counterexamples whenever redaction validators are changed
- require at least one explicit "what is still adjacent debt" note in closeout reviews so PASS does not pretend the whole product is complete

### Documentation / Review Practice

- use this review plus the prior Schema Registry v4 review as the baseline for the smoketest review template
- keep stable `AUDIT_ID` and `SMOKETEST_REVIEW_ID` linkage in governance records and WP traceability notes

---

## 13. Command Log

- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests schema_registry` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests task_board` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test role_mailbox_tests role_mailbox` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests -- --test-threads=1` -> PASS
- `CARGO_INCREMENTAL=0 cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
- `just post-work WP-1-Structured-Collaboration-Contract-Hardening-v1` -> PASS
- `just validator-packet-complete WP-1-Structured-Collaboration-Contract-Hardening-v1` -> PASS after parser hardening in `validator-packet-complete.mjs`
- `just wp-communication-health-check WP-1-Structured-Collaboration-Contract-Hardening-v1 STATUS` -> PASS
- `just session-control-runtime-check` -> PASS after settled result repair for request `f13de206-76ee-4e8b-aa73-a8218187d9df`
- `just gov-check` -> PASS
- `git -C ../handshake_main branch --contains c6e8ba2bf23ff9061b20f83a31567a6e47b322fe` -> PASS (`main`)
