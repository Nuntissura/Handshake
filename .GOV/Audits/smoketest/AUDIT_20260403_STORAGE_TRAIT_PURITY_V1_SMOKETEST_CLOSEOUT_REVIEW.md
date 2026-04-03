# Audit: Storage Trait Purity v1 Smoketest Closeout Review

## METADATA

- AUDIT_ID: AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-04-03
- AUTHOR: Codex acting as Orchestrator
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: WP-1-Storage-Trait-Purity-v1
- LINEAGE_STATUS: NONE
- RELATED_PREVIOUS_REVIEWS:
  - NONE
- SCOPE:
  - `WP-1-Storage-Trait-Purity-v1` packet, refinement, receipts, runtime status, and session-control outputs
  - committed product range `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`
  - contained-main merge commit `434c180e9755e7f455ae09b150784ab5412847f0`
  - governance-kernel closeout fixes required to move the packet from `MERGE_PENDING` to `CONTAINED_IN_MAIN`
- RESULT:
  - PRODUCT_REMEDIATION: PASS
  - MASTER_SPEC_AUDIT: PASS
  - WORKFLOW_DISCIPLINE: FAIL
  - ACP_RUNTIME_DISCIPLINE: FAIL
  - MERGE_PROGRESSION: PARTIAL
- KEY_COMMITS_REVIEWED:
  - `d26f46d` `docs: bootstrap claim [WP-1-Storage-Trait-Purity-v1]`
  - `214db04` `refactor: remove storage backend downcasts [WP-1-Storage-Trait-Purity-v1]`
  - `518066a` `fix: gate structured collaboration artifacts`
  - `54978d0` `test: restore named storage trait proof filters`
  - `be50f67` `test: align loom tier proof with flight recorder semantics`
  - `434c180` `merge: storage trait purity [WP-1-Storage-Trait-Purity-v1]`
  - `02f39f3` `gov: restore merge-pending truth [WP-1-Storage-Trait-Purity-v1]`
  - `8f675a5` `gov: fix contained-main signed-scope closeout`
  - `cb16c3f` `gov: honor merge-base signed scope in closeout`
  - `e1c0e51` `gov: allow harmonized contained-main scope`
  - `bc040d3` `gov: close out storage trait purity`
- EVIDENCE_SOURCES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md`
  - `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/signed-scope.patch`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Storage-Trait-Purity-v1/*.jsonl`
  - `.GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs`
  - `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`
  - `../handshake_main/src/backend/handshake_core/src/api/loom.rs`
  - `../handshake_main/src/backend/handshake_core/src/workflows.rs`
  - git history for `wt-gov-kernel` and `handshake_main`
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- RELATED_CHANGESETS:
  - NONE

---

## 1. Executive Summary

- The product packet itself closed correctly. The signed scope removed production storage downcasts, replaced them with explicit capability queries, preserved honest negative-path behavior on PostgreSQL, and merged into `main`.
- The workflow around that packet did not perform well. This was a steering-heavy run with repeated packet repair, review-loop churn, duplicate validator approvals, multiple workflow-invalid states, and live governance-tooling fixes during closeout.
- The strongest positive signal is validator honesty. Both validators repeatedly blocked false progress and forced the packet to prove the real API and negative-path contracts instead of allowing a shallow "downcasts removed" PASS.
- The strongest negative signal is governance execution quality. The packet reached contained-main closure only after the kernel had to be patched to handle multi-commit signed ranges, contained-main harmonization, and closeout syncing correctly.
- The packet is therefore a product PASS and a workflow FAIL. Handshake gained a correct bounded backend remediation, but the way it got there still costs too much operator attention and governance repair.

## 2. Lineage and What This Run Needed To Prove

- There was no earlier dedicated smoketest review for `WP-1-Storage-Trait-Purity-v1`, so this review treats the packet's own launch state as the baseline.
- This run needed to prove five things:
  - production storage code no longer relied on `as_any` or concrete backend downcasts
  - unsupported PostgreSQL paths were denied honestly instead of silently pretending to support SQLite-only collaboration artifacts
  - Loom telemetry still emitted the same `tier_used` contract after the trait cleanup
  - the signed 8-file scope could survive validator scrutiny and contained-main merge harmonization
  - the governed workflow could carry a real backend packet from bootstrap to contained-main closure without status drift or operator-restated rules

### What Improved vs Previous Smoketest

- There was no prior dedicated storage-trait smoketest to compare against, so "previous smoketest" for this review means the packet's own bootstrap state on 2026-04-03.
- What improved on the product side:
  - the packet moved from a stub-backed bootstrap claim to a contained-main validated merge
  - the signed scope ended with no production storage downcasts in `src/backend/handshake_core/src`
  - the final proof path moved from helper-level coverage to the real `api/loom.rs` Flight Recorder payload contract
- What improved on the workflow side:
  - the validators prevented a false green several times
  - the governance kernel now understands packet-declared `MERGE_BASE_SHA` and harmonized contained-main scope during closeout
- What did not improve enough:
  - the run still needed heavy orchestrator steering and live governance repair
  - the receipt stream still shows churn far above a healthy "one handoff, one review, one closeout" target

## 3. Product Outcome

- The product remediation is real and bounded. The production storage layer no longer uses downcast escape hatches inside the signed scope, the `Database` trait exposes capability methods that downstream callers now use directly, and the contained-main merge is present on local `main`.
- The signed scope is closed for this packet. `TASK_BOARD.md`, `BUILD_ORDER.md`, and the packet now agree that `WP-1-Storage-Trait-Purity-v1` is validated and done.
- The packet did not solve everything adjacent to storage portability, and it should not be credited with more than it actually delivered. In particular, true PostgreSQL structured-collaboration artifact parity still does not exist. The final validator PASS is honest because the packet proves explicit capability denial rather than pretending parity was added.

### Independent Product Findings

- Production downcast removal is confirmed. A repo grep over `src/backend/handshake_core/src` found no production matches for `as_any`, `downcast_ref::<SqliteDatabase>`, or `downcast_ref::<PostgresDatabase>`.
- Capability law is explicit. `Database` now defines `supports_locus_runtime`, `supports_structured_collab_artifacts`, `loom_search_observability_tier`, `supports_loom_graph_filtering`, and `loom_traverse_graph_perf_target_ms`, and PostgreSQL explicitly denies the structured-collaboration and Locus capability surface.
- The Loom proof became materially better by the end of the run. `tier_used` is emitted from `state.storage.loom_search_observability_tier()` in the real API path, and the signed test now asserts that emitted payload contract instead of only checking a backend helper.
- The product concern that remains is not a signed-scope failure. It is maintainability: the base `Database` trait is broader now, and future backend work can still drift if the capability law is not kept symmetric across consumers and implementations.

### INDEPENDENT_CHECKS_RUN

- repo grep over `src/backend/handshake_core/src` for production downcast escape hatches
- direct read of `storage/mod.rs` capability methods in the merged product tree
- direct read of `storage/postgres.rs` negative-path capability law in the merged product tree
- direct read of `api/loom.rs` emitted `tier_used` payload path and `storage/tests.rs` negative-path assertions
- reconciliation check across packet, task board, build order, runtime status, and local `main`

### COUNTERFACTUAL_CHECKS

- If `api/loom.rs` stopped emitting `tier_used` from storage capability law, the final named Loom proof would again degrade into a helper-only assertion and this packet would no longer prove the emitted telemetry contract.
- If PostgreSQL stopped returning explicit `NotImplemented("structured collaboration artifacts")` for those readers, the packet would silently overstate backend parity instead of honestly capability-denying unsupported paths.
- If future consumers in `workflows.rs` or `locus_sqlite.rs` reintroduced concrete backend assumptions, the trait-purity claim would regress even if the base trait still looked clean.

### DIFF_ATTACK_SURFACES

- Producer-consumer drift between `api/loom.rs` emitting `tier_used` and the test that claims to prove the Flight Recorder contract.
- Capability-law drift between `Database` trait defaults and backend implementations, especially around structured collaboration and Locus runtime.
- Consumer drift in `workflows.rs` and `locus_sqlite.rs`, where future edits could silently reintroduce backend-specific assumptions.
- Review-surface drift between the signed patch artifact, the committed range, and the contained-main merge commit.

### BOUNDARY_PROBES

- Boundary probe: reviewed the `api/loom.rs` producer path and the corresponding test path to confirm the emitted `tier_used` payload is asserted through the API surface.
- Boundary probe: reviewed `Database` capability methods and their downstream consumers in `workflows.rs` and `locus_sqlite.rs` to confirm the call sites no longer branch on concrete backend type.

### NEGATIVE_PATH_CHECKS

- Negative-path probe: confirmed PostgreSQL still returns `StorageError::NotImplemented("structured collaboration artifacts")` for structured-collaboration artifact access.
- Negative-path probe: confirmed PostgreSQL capability flags remain false for `supports_locus_runtime()` and `supports_structured_collab_artifacts()`.

### RESIDUAL_UNCERTAINTY

- The packet is closed, but the broader product still lacks true PostgreSQL structured-collaboration artifact parity.
- The widened `Database` trait is acceptable for this packet, but it creates a maintenance risk if future backend additions keep accumulating domain-specific capability methods without further factoring.

## 4. Timeline

- 2026-04-03T00:19:07Z: packet/bootstrap artifacts initialized.
- 2026-04-03T00:30:34Z: WP validator opens kickoff and constrains the packet to the signed 8-file scope.
- 2026-04-03T01:22:22Z: coder records initial intent with backend identity plus SQLite-specific trait ideas.
- 2026-04-03T01:26:45Z: WP validator blocks the intent and demands retention coverage, `just gov-check`, and capability-only trait design.
- 2026-04-03T01:52:42Z: coder reports `WORKFLOW_INVALIDITY`; product diff is reviewable, but packet governance is invalid and blocks further progression.
- 2026-04-03T02:49:59Z: orchestrator records packet-governance repair so the lane can continue.
- 2026-04-03T03:05:56Z: WP validator rejects the first reviewable implementation because the trait surface still leaked structured-collaboration readers and PostgreSQL was not capability-denying unsupported paths honestly.
- 2026-04-03T04:16:12Z: WP validator blocks again because the named proof filters and Postgres negative-path proof are still weak.
- 2026-04-03T05:02:48Z: WP validator blocks a third time because the Loom proof does not yet demonstrate the real emitted Flight Recorder contract.
- 2026-04-03T05:55:43Z: coder hands off the repaired `be50f67` range with the API-level Loom proof and explicit Postgres denial proof.
- 2026-04-03T06:13:13Z to 2026-04-03T06:18:35Z: coder and integration validator complete the governed direct-review exchange.
- 2026-04-03T06:30:49Z: packet records the governed PASS report for the committed range.
- 2026-04-03T07:37:12Z and 2026-04-03T07:40:23Z: closeout sync fails in live runtime with active-broker and signed-scope proof mismatches.
- 2026-04-03 morning closeout sequence: governance kernel is patched (`8f675a5`, `cb16c3f`, `e1c0e51`), merge truth is restored, the packet is closed out, and local `main` records merge commit `434c180`.

## 5. Failure Inventory

### 5.1 Critical: Final-lane closeout law was not executable against this real packet

Evidence:

- session-control outputs show `integration-validator-closeout-sync` failing on active broker state plus "candidate target diff" mismatches for all 8 declared files
- contained-main promotion required three governance fixes in the kernel: `8f675a5`, `cb16c3f`, and `e1c0e51`
- the final product merge existed before governance truth could legally settle

Reason:

- closeout logic treated already-contained or harmonized merge targets too rigidly
- packet-declared `MERGE_BASE_SHA` was not being honored correctly for multi-commit signed ranges
- the same final-lane broker run was allowed to collide with closeout syncing

Impact:

- merge progression stalled after the product-side PASS
- the operator paid for live governance-kernel repair in the middle of a supposedly near-complete packet
- the workflow could not honestly progress from PASS report to contained-main closure without changing the tooling

Judgment:

- This is a governance-system failure, not a coder-only failure.
- A real packet should not need kernel surgery to make signed-scope closeout executable.

### 5.2 High: Packet governance became invalid after the first reviewable implementation

Evidence:

- a `WORKFLOW_INVALIDITY` receipt recorded that `214db04` was reviewable, but the packet could not progress because required manifest fields, ASCII cleanliness, evidence mapping, command evidence, and status handoff content were invalid
- orchestrator had to write a `REPAIR` receipt before the lane could continue

Reason:

- packet and proof hygiene were not enforced tightly enough before implementation hardened
- governance compliance was discovered after real product work was already ready for review

Impact:

- coder time was interrupted by governance repair work outside the coder lane
- the packet accumulated avoidable steering overhead before technical review could continue

Judgment:

- This is a serious workflow design problem.
- Pre-work and packet hydration gates should fail closed earlier than this.

### 5.3 High: Coder intent and first implementation pass were under-scoped and ambiguous

Evidence:

- initial coder intent proposed "explicit backend identity plus sqlite-only trait methods" and omitted explicit `storage/retention.rs` coverage and `just gov-check`
- first validator review rejected `214db04` because the trait surface still carried structured-collaboration readers and PostgreSQL was not capability-denying unsupported paths cleanly

Reason:

- the coder's first plan was pointed in the right direction but not yet spec-tight
- the packet needed stricter early checkpoint discipline than the initial handoff provided

Impact:

- one full implementation loop had to be partially redone
- review effort shifted from confirmation to correction

Judgment:

- The coder eventually delivered the right bounded remediation, but the first pass did not meet the signed portability law.

### 5.4 High: Proof quality lagged behind code quality and required multiple validator interventions

Evidence:

- the validator later blocked because named proof filters resolved to zero tests and the Postgres negative path was not recorded clearly
- the validator then blocked again because `loom_search_backend_tier` was not yet proving the real emitted `tier_used` payload contract

Reason:

- the coder initially proved a storage-helper shape rather than the real API contract
- packet proof expectations were stricter than a superficial refactor-proof bundle

Impact:

- extra repair commits were required even after the main product refactor was functionally close
- operator and validator time increased substantially

Judgment:

- This is exactly the kind of problem adversarial review is supposed to catch.
- The validators were right to block it, but the loop was too expensive.

### 5.5 Medium: Validator communication quality was strong, but the receipt stream was noisy and repetitive

Evidence:

- receipt counts show `CODER_HANDOFF: 6`, `VALIDATOR_REVIEW: 7`, `REPAIR: 4`, `WORKFLOW_INVALIDITY: 4`, and `STATUS: 3`
- several late validator approval receipts repeat essentially the same "approved for final review" conclusion

Reason:

- the lane did not converge cleanly once the end-state became obvious
- review state and runtime state were not collapsing duplicates fast enough

Impact:

- token cost rose sharply
- later readers have to reconstruct the decisive review from near-duplicate approvals

Judgment:

- This is not the main failure, but it is expensive and should be treated as workflow debt.

### 5.6 Medium: Terminal runtime truth still looks sloppy even after successful closure

Evidence:

- `RUNTIME_STATUS.json` still shows `wp_validator_of_record` and `integration_validator_of_record` as `<unassigned>`
- `RUNTIME_STATUS.json` still lists an active coder session with `state: "working"` and an old heartbeat despite terminal packet closure

Reason:

- terminal closeout does not yet normalize every role/session field as cleanly as packet and board truth

Impact:

- the machine-readable authority surface still carries residue that can confuse later audits
- closeout looks less trustworthy than it should

Judgment:

- This did not block final `gov-check`, but it is still a governance concern and should not be normalized.

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- kept the packet bounded to the signed backend scope instead of widening it into unrelated portability work
- repaired broken governance truth when the coder correctly reported a workflow invalidity
- pushed the packet all the way through to contained-main closure instead of stopping at PASS report
- captured the kernel fixes that the live run proved were necessary

Failures:

- allowed the lane to enter real implementation before the packet/governance companion was robust enough
- had to provide repeated manual steering because the packet did not self-progress cleanly through the governed phases
- tolerated too much runtime and projection churn before forcing final truth to converge

Assessment:

- FAIL
- The orchestrator completed the packet, but not with acceptable workflow smoothness or governance quality.

### 6.2 Coder Review

Strengths:

- did the real backend remediation, not just paperwork
- surfaced `WORKFLOW_INVALIDITY` instead of pretending the invalid packet was good enough
- responded to validator objections with concrete repair commits rather than argument
- ended with an honest, bounded solution and a better API-level proof

Failures:

- initial intent was under-scoped and not yet aligned with the packet's portability law
- first implementation polluted the trait surface and failed to capability-deny unsupported PostgreSQL paths clearly
- proof quality lagged behind code quality until late in the run

Assessment:

- PARTIAL
- The coder ended strong, but the packet needed too many corrective loops before the work matched the contract.

### 6.3 WP Validator Review

Strengths:

- caught the initial intent gap before full implementation hardened
- caught the real product-scope issue in the first implementation pass
- caught proof drift twice and forced the packet to prove the real contract instead of a convenient proxy
- refused false-green closure

Failures:

- approval receipts became repetitive near the end of the lane
- the communication stream carried more duplicate "cleared" traffic than a healthy loop should need

Assessment:

- PASS
- The WP validator added real quality and prevented several false passes. The noise issue is secondary.

### 6.4 Integration Validator Review

Strengths:

- the diff-scoped PASS report is strong and appropriately adversarial
- the final review preserved honest negative proof about missing PostgreSQL artifact parity
- final-lane scrutiny extended beyond code into signed-scope artifact and merge-surface truth

Failures:

- the closeout path collided with live broker state and exposed final-lane fragility
- promotion from PASS report to contained-main closure was not atomic and needed tooling repair
- terminal runtime cleanup remains imperfect

Assessment:

- PARTIAL
- The technical review quality was good. The lane mechanics were not.

## 7. Review Of Coder and Validator Communication

- The content quality of the review traffic was mostly good. The validators were specific, and the coder's later handoffs became concrete and test-backed.
- The shape of the communication was not good. This was not a clean "intent -> handoff -> review -> final review" progression.
- The quantitative churn is a warning sign:
  - `CODER_HANDOFF: 6`
  - `VALIDATOR_REVIEW: 7`
  - `REPAIR: 4`
  - `WORKFLOW_INVALIDITY: 4`
  - `STATUS: 3`
- The strongest communication pattern was the validators' refusal to overclaim.
- The weakest communication pattern was late-loop duplication. Several validator approvals said nearly the same thing, which made the lane look less decisive than it actually was.

## 8. ACP Runtime / Session Control Findings

- ACP runtime truth was not clean through closeout.
- The most important runtime failure was active-broker self-collision: closeout sync was attempted while the integration-validator broker run was still active and unsettled, which guaranteed preflight failure.
- The second important runtime failure was signed-scope proof handling:
  - candidate-target validation treated contained or harmonized merge state as if the declared signed files had disappeared
  - packet-declared `MERGE_BASE_SHA` was not handled correctly for the signed range
- The third runtime concern is terminal residue:
  - runtime status still shows stale role-session information after final closure
  - validator-of-record fields are still unset in the runtime file
- Positive runtime signal:
  - both worktrees now pass `just gov-check`
  - packet, task board, build order, and local `main` containment now agree

## 9. Governance Implications

- This packet proves that final-lane closeout law was not mature enough before this run. The kernel fixes were not optional polish; they were required to make a real signed-scope contained-main packet closable.
- This packet also proves that packet hygiene gates are still too late. A coder should not be able to produce a valid reviewable product commit before packet manifest invalidity becomes visible.
- Governance projections improved by the end, but convergence was not atomic. The packet had to be walked through `In Progress` -> `Done` / `MERGE_PENDING` -> `Validated (PASS)` with intermediate repair commits rather than one authoritative closeout transition.
- The run leaves a broader governance concern: terminal packet truth can be correct while runtime session truth still looks stale. That is better than the reverse, but it is not the standard to keep.

## 10. Positive Signals Worth Preserving

- Validators did real adversarial review and did not settle for "downcasts removed" as a sufficient claim.
- The packet stayed bounded to an 8-file signed surface even while governance churn was happening around it.
- The final negative proof is honest. Handshake did not pretend PostgreSQL structured-collaboration artifact parity exists when it does not.
- The final state is truly contained in `main` and reconciled across packet, task board, build order, and both `gov-check` surfaces.
- The governance-kernel fixes from this run should reduce future closeout pain for other multi-commit contained-main packets.

## 11. Remaining Product or Spec Debt

- There is no open signed-scope spec debt for this packet. The current master spec clauses named by the packet are satisfied inside the committed range.
- The most important adjacent product debt is still missing PostgreSQL structured-collaboration artifact parity. The packet closed that gap honestly by denying the capability, not by solving parity.
- The second adjacent product concern is trait-surface growth. `Database` is cleaner than a downcast-based design, but it is still a broad cross-domain trait whose capability surface can drift if future work keeps stacking domain-specific methods onto it.
- No new spec-enrichment packet is justified from this review alone. The remaining issues are implementation-surface maintenance and broader backend parity, not current-master-spec ambiguity.

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: FLAT
- CURRENT_STATE: HIGH
- Evidence:
  - the operator burden was high because the packet needed repeated steering and live governance repair
  - `WORKFLOW_INVALIDITY`, `REPAIR`, duplicate review receipts, and closeout sync retries all appeared in one run
  - authoritative closure required kernel fixes during the run, not just disciplined execution of existing tooling
- What improved:
  - the kernel can now handle packet-declared `MERGE_BASE_SHA` and harmonized contained-main scope more honestly
  - the packet did eventually converge to truthful contained-main closure
- What still hurts:
  - the workflow was still far from "one launch, one review loop, one closeout"
  - terminal runtime/session truth remains noisier than packet truth
  - packet hygiene failures were discovered too late
- Next structural fix:
  - make packet-manifest validity and final-lane closeout preflight non-bypassable before coding hardens or contained-main promotion is attempted

### 12.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: LOW
- Evidence:
  - the signed scope is closed and merged
  - validators confirmed the committed range against the current spec clauses with explicit negative proof
  - remaining debt is narrow and clearly outside the packet: PostgreSQL artifact parity and long-term trait-surface maintenance
- What improved:
  - the real packet gap is now materially smaller than at launch
  - the packet converted a vague portability concern into an honest capability-law boundary with proof
- What still hurts:
  - broader backend parity for structured-collaboration artifacts is still absent
  - the trait surface may still become too broad over time
- Next structural fix:
  - only open a follow-on packet when Handshake actually needs PostgreSQL structured-collaboration artifact parity or a narrower storage capability abstraction

### 12.3 Token Cost Pressure

- TREND: REGRESSED
- CURRENT_STATE: HIGH
- Evidence:
  - receipt churn was high: `CODER_HANDOFF: 6`, `VALIDATOR_REVIEW: 7`, `REPAIR: 4`, `WORKFLOW_INVALIDITY: 4`
  - the packet needed repeated review loops plus live closeout-tooling repair
  - duplicate validator approvals and repeated closeout retries consumed operator attention without adding proportional signal
- What improved:
  - once the right proof target was identified, the final code and proof path became clearer and more defensible
- What still hurts:
  - this run still charged humans for procedural ambiguity and tooling immaturity
  - the workflow is still too expensive when a backend packet hits both code review and governance closeout issues
- Next structural fix:
  - collapse duplicate review emissions and add one canonical closeout-sync command path that settles broker state, signed-scope proof, and packet/runtime projection together

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- The first reviewable product commit existed before packet-governance invalidity was surfaced. That is a false-green moment in workflow terms.
- The PASS report was real, but contained-main closeout still failed afterward. That is another false-green moment unless the user reads beyond the PASS paragraph.
- The packet is terminally correct, but runtime still showing an active coder session is a quieter form of false confidence in terminal cleanup quality.

### 13.2 Systematic Wrong Tool or Command Calls

- `integration-validator-closeout-sync` was attempted while the active integration-validator broker run was still unsettled.
- closeout sync initially evaluated the wrong candidate target semantics for an already-contained or harmonized merge situation.
- split-root governance authority around `HANDSHAKE_GOV_ROOT` remained part of the operational complexity rather than being fully hidden by the tooling.

### 13.3 Task and Path Ambiguity

- Early coder intent was ambiguous about whether backend identity methods or capability methods were the right contract.
- The packet's true boundary was "capability-based portability cleanup," but the first implementation drifted toward trait methods that were still too SQLite-shaped.
- Contained-main closeout semantics were ambiguous until the governance kernel was updated to treat merge-base and harmonized merge commits correctly.

### 13.4 Read Amplification / Governance Document Churn

- This packet forced repeated rereads of the packet, receipts, runtime status, closeout outputs, and kernel scripts because no single surface stayed authoritative enough during the run.
- Multiple nearly identical validator approvals increased audit-read cost without improving certainty.
- The run required reading both product and governance git histories to explain why the packet took so long.

### 13.5 Hardening Direction

- Pre-work should reject invalid packet-governance companions before reviewable product commits become possible.
- Final-lane closeout should self-settle active broker state or fail earlier with a single clearer diagnostic.
- Terminal closure should stamp packet truth, runtime truth, and role-session retirement atomically.

## 14. Suggested Remediations

### Governance / Runtime

- Make packet-governance validity a stricter early gate so manifest drift cannot survive into reviewable implementation.
- Add one canonical contained-main closeout path that:
  - requires settled broker state
  - honors packet `MERGE_BASE_SHA`
  - accepts signed-surface-preserving harmonized merge commits
  - retires stale role sessions in the same transaction
- Populate validator-of-record fields and clear stale runtime session residue during terminal closeout.

### Product / Validation Quality

- Keep the current API-level Loom proof pattern; do not let future refactors regress to helper-only proof.
- If Handshake later needs PostgreSQL structured-collaboration artifact parity, create a separate bounded packet rather than smuggling parity claims into trait-purity reviews.
- Revisit `Database` trait factoring if more backend capabilities accumulate; the current solution is valid but not necessarily the final architecture.

### Documentation / Review Practice

- Continue writing smoketest reviews that separate product PASS from workflow FAIL. This run is a clear example of why those judgments cannot be merged.
- Add a short review guideline that duplicate validator approvals should be consolidated once the decisive verdict is already known.
- When there is no prior dedicated smoketest baseline for a WP, the review template should explicitly allow "bootstrap state as baseline" wording so authors do not fake lineage.

## 15. Command Log

- `Get-Content .GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md` -> PASS (loaded the canonical smoketest review structure)
- `Get-Content .GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md` -> PASS (loaded the mandatory rubric and ambiguity-scan rules)
- `Get-Content .GOV/Audits/smoketest/AUDIT_20260326_LOOM_STORAGE_PORTABILITY_V4_SMOKETEST_REVIEW.md` -> PASS (used as an output-shape reference only)
- `Get-Content .GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md` -> PASS (reviewed packet truth, status history, and PASS report)
- `Get-Content ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RUNTIME_STATUS.json` -> PASS (reviewed current runtime truth and terminal residue)
- `Get-Content ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RECEIPTS.jsonl` -> PASS (reviewed the full governed review and repair ledger)
- `rg -n -S "active broker|candidate target diff|contained main diff|Closeout sync preflight failed|signed patch artifact|session INTEGRATION_VALIDATOR:WP-1-Storage-Trait-Purity-v1 is READY|closeout sync" ..\\gov_runtime\\roles_shared\\SESSION_CONTROL_OUTPUTS ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS .GOV` -> PASS (captured final-lane and closeout failure evidence)
- `git log --oneline --decorate --max-count=20` in `wt-gov-kernel` -> PASS (confirmed the governance-kernel fixes required by this run)
- `git log --oneline --decorate --graph --max-count=25` in `handshake_main` -> PASS (confirmed the product commit chain and contained-main merge)
- `git grep -n -E "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" HEAD -- src/backend/handshake_core/src` -> PASS (desired zero-match result; `git grep` returns non-zero when nothing matches)
- targeted line reads from `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/storage/postgres.rs`, `src/backend/handshake_core/src/api/loom.rs`, and `src/backend/handshake_core/src/storage/tests.rs` -> PASS (confirmed capability law, emitted payload contract, and explicit negative-path proof)
- `rg -n -S "WP-1-Storage-Trait-Purity-v1" .GOV/roles_shared/records/TASK_BOARD.md .GOV/roles_shared/records/BUILD_ORDER.md .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` -> PASS (confirmed reconciled task-board, build-order, and traceability status)
- `just gov-check` in `wt-gov-kernel` -> PASS
- `just gov-check` in `handshake_main` -> PASS
- receipt count aggregation over `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RECEIPTS.jsonl` -> PASS (quantified churn for the review)
