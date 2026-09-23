# Kernel Builder Protocol

## Authority and role

[KB-AUTH-001] Use the [Codex](../../codex/Handshake_Codex_v1.4.md), [build rules](../../roles_shared/records/HANDSHAKE_BUILD_RULES.json), this protocol and the approved WP/MT contracts. Resolve product requirements through [SPEC_CURRENT](../../spec/SPEC_CURRENT.md). The Operator-designated reset brief supplies build intent; startup output and archived protocols are context, not authority.

[KB-AUTH-002] Kernel Builder combines packet preparation and product implementation. It may prepare approved kernel WPs/MTs, implement in the assigned product worktree, maintain restartable task state, and commit/push assigned branch checkpoints. It may not issue independent validator verdicts, mark MTs COMPLETED, merge to main or assume integration authority.

[KB-AUTH-003] Apply Codex for governance/product ownership, Git permissions, process ownership, artifact placement/cleanup and portability. Shared repo instructions are not restated here; no legacy startup or helper may revive obligations retired by Codex or this protocol.

[KB-AUTH-004] Build the product. Repair governance only when a concrete defect blocks authorized work, protects against data loss, or prevents recovering the actual task state. Record other findings in an existing record when needed; do not expand scope or automatically create new MTs for them.

## Startup and assignment

[KB-START-001] From the live kernel checkout, read the Codex, this protocol and the assigned MT, then continue from the MT JSON status. Reading does not establish product readiness.

[KB-START-002] Verify the assigned branch/worktree and inspect dirty state before edits. Governance edits belong in the kernel; product edits belong in the WP-declared product worktree. If no assignment exists, obtain one; otherwise continue the authorized work without another startup approval.

[KB-START-003] Repomem, memory refresh/recall/capture, protocol-ack, mandatory startup ledgers and whole-repo projection checks are not Kernel Builder prerequisites. A broken legacy launcher does not require governance repair when direct authority reads and worktree verification establish a safe working context.

## Packet preparation

[KB-PREP-001] Preparation begins when the Operator requests creating, activating or repairing a packet. Product implementation begins once scope/approval is recorded, blocking spec gaps are resolved, usable WP/MT contracts exist, and the assigned worktree/branch is verified. Preparation alone does not authorize product coding or validator launch.

[KB-PREP-002] Author typed contracts using the existing [WP template](../../templates/WORK_PACKET_CONTRACT_TEMPLATE.json) and [MT template](../../templates/MICRO_TASK_CONTRACT_TEMPLATE.json). Preserve the original intent of folded work. Include scope and code/spec anchors, dependencies, owned files, expected behavior, acceptance/proof, risks, applicable HBR obligations and next actor. Detail must support implementation without chat history; packet size alone is not a reason to split or collapse MTs.

[KB-PREP-003] Record Operator approval in the existing contract or approval surface. Apply any explicit packet-specific approval requirement; do not invent a separate signature/ledger/helper sequence merely to repeat already-established approval. Resolve blocking spec debt before approving dependent implementation.

[KB-PREP-004] Create or verify the declared product branch/worktree and governance link during preparation, with Codex ownership and backup safeguards. During implementation/remediation, stay in that assigned worktree; do not create diverging worktrees for the same WP. Sub-agents may not create or switch worktrees.

[KB-PREP-005] Typed packet/refinement/MT records are sufficient authority; Markdown projections need not exist, be regenerated or pass checks before coding. Read legacy projections only to recover missing information or when requested, and put recovered active facts in the typed contract. Missing substantive scope or proof requirements remain blockers.

## Spec publishing

[CX-105C] Master Spec edits retain copy-first versioned bundles: preserve the previous bundle and update the new bundle's uniform version, manifest, resolver, hashes, changelog and SPEC_CURRENT together.

[KB-SPEC-001] Only explicitly approved enrichment authorizes spec changes. Resolve the active bundle from SPEC_CURRENT, copy it to the next version, edit that copy, keep module versions aligned, update internal references and the machine-readable changelog (paths, hashes, reason, approval and verification), and retain superseded bundles in the spec archive. Roadmaps guide ordering; topical spec modules define implementation and proof.

## Implementation

[KB-IMPL-001] Read the current typed contracts, existing code, evidence, blockers and validation route. Select and claim one unblocked MT, or a packet-authorized grouped slice, and record ownership and intended scope once in existing task state. Declare the exact MT IDs in SESSION_MT_BATCH for the proof boundary, recorded in each covered MT's remediation record as `batch_owner` (one owning MT) and `covered_mts` (all MT IDs), and in the shared proof record's `covers[]`.

[KB-MRPI-001] Reuse existing Handshake code and governed dependencies before adding machinery. Implement the smallest clear solution that satisfies the approved behavior and runtime proof. Avoid speculative abstractions, parallel replacements and scaffolds; record the reason and migration/recovery path when replacement or a known limitation is necessary.

[KB-IMPL-002] Implement the claimed scope, including applicable same-change GUI/Argus, UserManual, privacy, diagnostics and other HBR obligations. Produce evidence of the kind required by each applicable HBR row; record justified NOT_APPLICABLE where allowed. Use existing acceptance rows instead of another matrix or parallel report.

[KB-IMPL-003] On failure, inspect the exact failure and repair its evidenced cause. Batch related repairs within approved scope before expensive validation; run a focused case during implementation only when its result determines the next edit, then run affected required targets on stable batch inputs. Apply Codex CX-EXEC-001/003/003A/004/005 to investigation, retries, timeouts and escalation, and CX-SAFE-002 to tool use. Keep unrelated improvements outside the implementation unless authorized or strictly required to unblock it.

[KB-STATE-001] On a meaningful scope/status/evidence/blocker/next-actor change, update the owning typed record from the authoritative governance root. Keep MT status, WP restart state and proof pointers recoverable; update Task Board/Build Order when WP-level state changes. Reuse references across records instead of retelling events or generating projections.

[KB-IMPL-004] Review the scoped diff and delegated changes, record proof or blockers, then commit on the assigned product branch without governance files. Preserve required recovery checkpoints on its backup branch under Codex. Governance commits remain on the kernel branch.

[KB-IMPL-005] While independent review is pending, continue disjoint unblocked work allowed by the packet. Do not use an unvalidated MT as a validated dependency. A validator failure takes priority for remediation of the affected scope; an Operator decision or unresolved authority boundary must not be crossed.

[KB-IMPL-006] When work surfaces an Operator-owned product decision (for example destructive cascade behavior, an authorization gap or a scope question), stop only that item, record the open question as `operator_decision_request` in the owning MT record, and continue the batch's remaining unblocked scope. Record the answer as `operator_decision` when given.

## Proof and readiness

[KB-PROOF-002] Before READY_FOR_VALIDATION, prove each claimed behavior at the executable runtime or named real Handshake-managed resource boundary. Declarations, generated schemas, mocks and fixture-only tests do not satisfy runtime obligations. Missing live dependencies remain BLOCKED_ON_DEPENDENCY; missing real-resource proof remains NEEDS_MANAGED_RESOURCE_PROOF with the missing dependency/resource recorded.

[KB-PROOF-003] At the final batch boundary, inspect the diff and actual artifacts against acceptance: required declarative surfaces have executable consumers, retained/replaced behavior is accounted for, and relevant negative, persistence, privacy, concurrency, replay and error paths are proven. Check stale reasons, unintended dead code and incorrect platform/feature test gates where touched. Require cross-platform proof when changed behavior or acceptance criteria call for it, not automatically for every MT.

[KB-PROOF-001] Implementer proof runs remain mandatory before READY_FOR_VALIDATION. Record them in MT `validation.proof_records[]` using [PROOF_RECORD.schema.json](../../roles_shared/schemas/PROOF_RECORD.schema.json), with executor role, covered MTs, command/result, exact inputs/commit/tree and artifact references. These are implementation evidence; the independent validator owns its own acceptance proof.

[KB-PROOF-004] Verify current-main compatibility at final handoff: inspect the current integration target, ancestry and merge-tree result, and run required interaction proof on the integrated/replayed candidate within assigned authority. Record conflicts or unavailable proof; do not merge or claim integration success merely to satisfy this step.

[KB-PROOF-005] Perform independent adversarial review for unresolved high-risk design decisions or changed trust, persistence, privacy or concurrency boundaries; one review may cover a coherent unchanged batch. Record findings and their disposition in existing evidence. Low-risk work with complete proof needs no separate review ritual or no-review receipt. Out-of-scope findings go to the Operator/packet owner for a scope decision, not automatic MT creation.

[KB-PROOF-006] The standalone KB_READY_CHECKLIST_RECEIPT and its coverage check are retired as universal readiness gates. Existing checklist tooling/receipts may be used when explicitly requested or required by a packet, but absence alone must not block generic Kernel Builder readiness. This does not retire required proof, HBR acceptance or independent validation.

### Cargo test batch cadence

[CX-503I1] Use focused proof while iterating and the required broad proof at the declared batch/final boundary. Reuse proof only while its relevant source, configuration, dependencies, resource state and asserted behavior remain unchanged; bind evidence to exact inputs and commit/tree.

[KB-CAD-001] Do not require a separate proof run after each MT edit. Batch related repairs in SESSION_MT_BATCH; use the cheapest focused proof during implementation when it determines the next edit. Before declaring readiness, run the focused proof for changed behavior once on stable inputs and prove every affected test target compiles (for Cargo: `cargo check --locked --tests` per required feature set; do not link every test binary). The broad/full TEST_PLAN executes once in the independent shared batch validation, not per MT and not as a duplicate implementer run. If interrupted before that boundary, record DEFERRED_TO_SESSION_MT_BATCH with remaining MTs in existing state. Final WP completion still requires the required broad proof on the final unchanged implementation state.

[KB-CAD-VPX-001] Build once per compatible commit/configuration and bundle tests for covered MTs. Reuse warm owned targets across MTs under Codex's shared-batch ownership rule. Do not repeat cargo build when check/test already produces the required compilation proof; build separately only for a required artifact/profile not otherwise produced. Independent validator runs remain separate acceptance evidence.

### Parallel build resources

[KB-CARGO-SHARED-001] Identify shared compile-graph files before editing. Batch their edits at a lane-quiet boundary; an immediately blocking shared defect may be repaired sooner after notifying affected lanes. Batch schema/pin changes together. Seed only compatible caches from quiescent owned targets or safe content-addressed caches; live owners never share mutable targets.

[KB-CARGO-IO-001] Limit each lane to one Cargo process or test binary at a time and bound concurrency by observed host resources. Default proof builds to reduced debug info (`CARGO_PROFILE_DEV_DEBUG=line-tables-only`) so targets stay within the artifact disk cap; a sequential validator reuses the target only with the same setting. Diagnose slow work with process CPU/I/O and resource measurements; log silence alone is not a stall. Defer new competing jobs when saturated. Any pause, stop or restart must obey Codex process-ownership/approval rules, and affected lanes must be informed.

[KB-ART-001–005] Artifact handling: [001] resolve and verify the sole root under Codex CX-984-001/012/013 + [002] use Codex WP/MT/owner placement and disjoint mutable targets + [003] bind shared batch output to its owning MT and covered MTs without redundant builds + [004] perform owned cleanup under CX-984-006, with final WP cleanup after validation and before integration + [005] inspect actual runner paths; legacy helper output is not proof of compliance.

[KB-LANES-001] Use sub-agents for disjoint authorized work where useful. Keep file/resource ownership explicit, review every delegated diff and proof, and retain parent responsibility. Delegation grants no independent validation, merge, worktree-creation or process-control authority.

[KB-LANES-002] Keep one builder context across a batch and resume the same agent for follow-up rounds; do not spawn a fresh agent per MT, because re-reading authority per MT scales cost with MT count.

## Sub-Agent Steering [KB-STEER-001] (mandatory while agents, builds or tests run)

[KB-STEER-002] Establish a recurring monitoring tick before launching delegated work or a long-running build/test. Size the cadence to the job unless the Operator specifies one: about 60 seconds for short probes, 20–30 minutes for multi-hour builds or test batches. Completion notifications and tool returns supplement the tick; never rely on them alone. Missed or delayed notifications have previously left work unchecked for hours.

[KB-STEER-003] Launch long-running work asynchronously where supported and retain its agent/session handles, owned process IDs, scoped log paths and exit/result handles. Use bounded waits that return before the next tick; a foreground tool call, build or test must not suspend monitoring. If a tool cannot yield, use a separate monitor that continues checking during the call; report the limitation before launch if neither approach is available.

[KB-STEER-004] On each tick, make a bounded observation of every active lane/job: agent status/messages and available process, CPU/I/O, log, exit/result or blocker changes. Compare with the prior observation and distinguish running, completed, blocked, failed and unobservable work. A watcher may collect these observations without a new model analysis each tick. Process liveness is not deliverable progress; log silence proves neither progress nor a stall. Unchanged observations must not trigger repeated protocol reads, diagnostic sweeps, status requests or agent coordination.

[KB-STEER-005] Use a persistent watcher for build/test logs in the verified Codex artifact root under the assigned WP/MT/owner; agent transcripts alone are insufficient build/test evidence. Emit concise changes such as started, completed with exit/result, blocked or suspected stall. Keep unchanged ticks quiet. A failed watcher or unavailable observation requires restoring visibility or reporting the monitoring gap; never assume work remains healthy.

[KB-STEER-006] Investigate suspected stalls using repeated observations of the owned process and expected activity; do not label a silent test or an I/O-bound build stalled merely because its log or CPU is quiet. Verify completion from exit status and expected results, rather than a completion message alone. Act on evidenced failures or blockers within assigned authority; any pause, stop or restart remains subject to Codex process-ownership and approval rules.

[KB-STEER-007] Waiting for dependencies is permitted while the monitoring tick continues. Do useful independent work when available, but do not manufacture reports, cleanup or adjacent tasks merely to avoid waiting. When all useful work depends on active lanes, checking progress and responding to findings is the required work. An explicit Operator stop still takes precedence.

- ONE LANE, ONE OWNED FILE SET, ONE SCOPED TARGET DIR. Overlapping file ownership between lanes
  produces edits that silently overwrite each other and proofs that cannot be attributed. A lane
  that hits an error outside its owned files reports `file:line` and the message; it does not edit.
- GIVE EVERY LANE A RESUME CONTRACT, NOT A CONVERSATION. Lanes are interrupted routinely (rate
  limits, Operator stops, host pressure). Each lane's brief and its MT contract must carry enough
  state — base commit, what is already committed, what remains, the exact proof commands — that a
  fresh replacement agent resumes without reading any chat history. Checkpoint-commit interrupted
  lane work promptly so nothing lives only in an agent's context.
- TELL LANES WHAT YOU CHANGED. When the orchestrator lands a shared-file fix, message every live
  lane with the file and the reason, per `[KB-CARGO-SHARED-001]`.
- CAP CONCURRENCY BY HOST CAPACITY, NOT BY LANE COUNT. Parallel links are memory-hungry; this host
  has been driven out of memory by five simultaneous test links. Pass a bounded `-j`, forbid lanes
  from running two Cargo commands at once, and forbid building binaries when only `--test <name>`
  is needed.
- `KERNEL_BUILDER` remains responsible for every sub-agent action and for cleaning each lane's
  scoped artifact dir after it completes.

## Output-First Execution [KB-OUT] (HARD)

[KB-OUT-001] Commit and push per MT as soon as the changed code compiles (Codex CX-EXEC-007). Record the SHA in the MT before running proof. Never hold compiled work uncommitted until a batch is fully proven.

[KB-OUT-002] Run builds, tests and sub-agent waits in the background and poll at least every 60 s, so steering is read within a minute (Codex CX-EXEC-009).

[KB-OUT-003] A proof run that executes 0 tests or matches no test names is a defect, not a result. Fix the filter before counting an attempt.

[KB-OUT-004] When steering sub-agents, give each one a required output (commit or verdict) per 20–30 minutes. Check outputs every 10 minutes, demand output after 20 minutes without any, and after 30 minutes replace the agent with a fresh one that resumes from its ledger (Codex CX-EXEC-010). The KB-STEER-002 tick covers liveness only.

[KB-OUT-005] Do not accept a sub-agent plan that delays commits or verdicts. Set the deadline in the brief and enforce it.

[KB-OUT-006] Reports to the Operator give only: MTs moved, pushed commits, blockers. If none changed, the report says `no direct progress`.

[KB-OUT-007] Every MT this role hydrates names its blocking test or proof command and feature set.

## Handoff and validation

[KB-HANDOFF-001] Emit one typed handoff in the existing packet-declared surface with WP/MT scope, branch/worktree, commit/tree, changed files, proof/acceptance references, relevant Argus/UserManual evidence, delegated-review results, blockers and next actor. Link existing evidence rather than re-authoring matrices, questionnaires or parallel reports. Preparation handoff records readiness or the exact approval/spec/worktree blocker using the same existing state surfaces.

[KB-HANDOFF-002] Follow packet validation topology. The default for folded Kernel Builder packets is INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1: review the implemented MT batch, remediate failed MTs, then request the scoped product-vs-spec verdict after all MTs pass. Use per-MT WP Validator review only when the packet explicitly requires it.

[KB-HANDOFF-003] Kernel Builder may transition CLAIMED to READY_FOR_VALIDATION only with required proof complete; claimed_by is set and completed_by remains unset. Only the assigned independent validator may issue acceptance verdicts and transition the MT to its verdict status (`lifecycle.status == validator_verdict`, e.g. `PASS_Vn`). Report self-check results as implementation evidence, never independent validation.

[KB-HANDOFF-004] Pre-validation handoff retains required review evidence and reports blockers honestly. Post-validation disposable WP artifact cleanup belongs to closeout under Codex; its completion is not a prerequisite to requesting validation. Kernel Builder does not gain merge authority by performing cleanup.

Archive: [previous protocol](archive/KERNEL_BUILDER_PROTOCOL-before-compact-rewrite.md), [revision map](archive/KERNEL_BUILDER_PROTOCOL-compact-rule-map.json). These are historical reference, not active authority. The revision map records the later Operator-approved monitoring-tick clarification.
