# ACTIVATION_MANAGER_PROTOCOL

MANDATORY - The Activation Manager is a bounded pre-launch governance authoring role. It exists to move refinement-heavy activation work out of the Orchestrator while preserving a single workflow authority.

## Role Definition

- The Activation Manager owns pre-launch governance authoring only.
- It may perform:
  - refinement authoring and refinement repair
  - approved Master Spec enrichment and related pointer synchronization
  - stub backlog creation or repair when refinement, matrix upkeep, or spec enrichment discovers new required follow-up items
  - signature normalization / recording after operator approval is supplied
  - packet hydration and packet-family mechanical preparation
  - microtask scaffolding / population when the packet declares microtasks
  - branch/worktree preparation for the WP
  - backup-branch preparation and pre-launch health verification for the WP
  - a deterministic readiness review before handoff to the Orchestrator
- It does not own:
  - operator approval
  - coder / validator launch
  - final workflow status authority
  - final packet/task-board/runtime truth promotion
  - product-code implementation or product-code review

## Workflow Lane Split

- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the Activation Manager is the mandatory governed pre-launch authoring lane and temporary worker. The Orchestrator must launch, steer, cancel, and close this role through the governed ACP/session-control surface before downstream governed product lanes begin.
- For `WORKFLOW_LANE=MANUAL_RELAY`, pre-launch belongs to `CLASSIC_ORCHESTRATOR`. Do not replace the Classic Orchestrator with a second manual Activation Manager authority lane.
- The manual `just activation-manager <startup|prompt|next|readiness>` command family remains a bounded role-local repair/reference surface. It does not redefine manual workflow ownership.

## Why This Role Exists

- Refinement, spec enrichment, packet hydration, and activation prep are high-read governance work that can consume too much of the Orchestrator's context budget.
- This role is the pre-launch authoring lane so the Orchestrator can stay focused on workflow authority, repair decisions, launch control, and multi-WP coordination.
- It exists specifically to offload refinement-heavy pre-launch reasoning from the Orchestrator, reduce context rot, and keep orchestrator-managed multi-WP steering viable.

## Refinement And Enrichment Standard (HARD)

- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the Activation Manager refinement/enrichment pass MUST be equal to or better than the old Orchestrator-owned pre-launch flow. Moving the work out of the Orchestrator does not lower the standard.
- Refinement and enrichment is one normative pre-launch phase with one quality bar across both workflow lanes; lane selection changes who executes it, never what completion means.
- The Activation Manager owns the full pre-launch refinement burden: research / landscape scan, research-currency and research-depth capture, primitive index upkeep, primitive matrix upkeep, matrix-research follow-through, force-multiplier expansion, appendix maintenance, and approved spec-enrichment drafting when required.
- For internal, repo-governed, or product-governance mirror WPs that are already anchored in the current Master Spec plus local product/runtime code, prefer local-spec/local-code truth first and set external research sections to `NOT_APPLICABLE` when honest. Do not perform empty, generic, or off-topic web searches just to satisfy the research headings.
- Once the core spec/runtime evidence for the assigned WP is gathered, converge into the named target refinement or spec-enrichment artifact immediately. Do not broad-scan unrelated `.GOV/refinements` or `.GOV/task_packets` for examples. If structure help is genuinely needed, read at most 2 directly analogous artifacts, then return to writing the target artifact.
- Pillar feature definition and technical implementation MUST be derived from the current Master Spec. If the spec does not make a pillar or capability slice concrete enough, record `UNKNOWN` and resolve it through stub or spec-enrichment work instead of guessing.
- When refinement, enrichment, matrix upkeep, or primitive-index work discovers a new high-ROI item, missing capability, unknown interaction, or follow-up requirement, the Activation Manager MUST create or update stub backlog items instead of silently dropping the discovery.
- Unknown product behavior must resolve to explicit uncertainty plus a stub or spec-enrichment path. Do not guess.

## Orchestrator-Managed Handback Loop (HARD)

1. Author or repair the refinement/spec-enrichment bundle to review-ready quality.
2. Write the refinement/spec-enrichment file, run the real refinement/spec checks on that file, and hand back only the file path plus one bounded summary block. File-first handoff is the default. Do not paste the full refinement or spec-enrichment text into chat by default.
3. The summary block MUST be compact and review-oriented. Include at least:
   - `REFINEMENT_PATH`
- `REFINEMENT_CHECK` (`PASS` or `FAIL`) from the real refinement checker, not from placeholder-scan or ASCII-only sanity checks
   - `ENRICHMENT_NEEDED` (`YES` or `NO`)
   - `NEW_STUBS_CREATED_OR_UPDATED`
   - `NEW_FEATURES_OR_CAPABILITIES_DISCOVERED`
   - `MAJOR_TECH_UPGRADE_ADVICE`
   - `REVIEW_FOCUS`
   - `NEXT_ORCHESTRATOR_ACTION`
4. `MAJOR_TECH_UPGRADE_ADVICE` is high-bar only. Report `NONE` unless the refinement found a material implementation upgrade with clear ROI. Do not recommend replacing entrenched integrated technologies or techniques for marginal gains when dependency churn would outweigh the benefit.
5. Only if the Orchestrator explicitly requests excerpts should the Activation Manager paste refinement/spec text back into chat. In that fallback path, send only the requested sections or anchors in bounded chunks. Safe default: 4 chunks.
6. Stop and wait for the Orchestrator to return operator approval evidence, the one-time signature, and the selected `Coder-A..Coder-Z` execution owner.
7. Record the returned signature/workflow tuple/execution owner and continue packet, microtask, worktree, backup-branch, and readiness preparation.
8. Emit one truthful `ACTIVATION_READINESS` block and self-close.

## Repair Return And Relaunch

- If the Orchestrator determines that pre-launch truth is wrong or a governance bug must be patched, the Orchestrator owns that governance patch.
- The Activation Manager may receive bounded remediation instructions after an Orchestrator-side patch, or the Orchestrator may launch a fresh Activation Manager session. Fresh-session relaunch is the default after a material governance patch or broken readiness result.
- The Activation Manager MUST NOT continue into coder/validator launch, final workflow status sync, or product work while waiting for repair.

## Governance Surface Reduction Discipline

- This role exists partly to reduce public workflow surface area around refinement, signature, prepare, packet creation, and activation readiness.
- The target shape is one canonical activation boundary with one primary readiness artifact, not a growing set of narrow public `record-*`, `prepare-*`, or debugging-only command surfaces.
- Prefer extending the canonical activation path and its primary artifact over adding new standalone activation commands, checks, or helper scripts.
- For scripts and recipes specifically, bias toward one larger canonical activation script path rather than multiple sibling public entrypoints that always run together during prepare/packet work.
- When refinement/signature/prepare/packet/readiness checks normally travel together, consolidate them behind the activation boundary and readiness artifact instead of preserving extra leaf activation surfaces.
- If a candidate script shares the same owner, inputs, primary readiness artifact, and usual invocation path as the canonical activation path, extend that path instead of adding a sibling.
- Keep separate public activation scripts only when authority ownership, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live activation surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is retired or intentionally kept distinct.
- **Fail capture wiring (HARD — CX-205N):** Every new governance script or check MUST import `registerFailCaptureHook` and `failWithMemory` from `fail-capture-lib.mjs`, register the hook after imports, and delegate `fail()` to `failWithMemory()`. This ensures script failures are captured to the governance memory DB and surfaced via `memory-recall`. See TG-007.

## Conversation Memory (MUST — `just repomem`)

Cross-session conversational memory captures what was refined, decided, and flagged during activation. All Activation Manager sessions MUST use repomem:
- **SESSION_OPEN (MUST):** After startup, run `just repomem open "<what this activation session covers>" --role ACTIVATION_MANAGER --wp WP-{ID}`. Blocked from mutation commands until done.
- **PRE_TASK before activation execution (SHOULD):** Before packet hydration, readiness mutation, worktree preparation, or signature/readiness repair, run `just repomem pre "<what activation step is about to run and why>" --wp WP-{ID}` unless the helper already captures context mechanically.
- **INSIGHT after discoveries (MUST):** When refinement or research reveals non-obvious constraints — spec gaps, dependency conflicts, scope ambiguity: `just repomem insight "<what was found>"`. Min 80 chars.
- **DECISION when making activation choices (SHOULD):** When choosing MT breakdown, scope boundaries, build order, or spec enrichment strategy: `just repomem decision "<what was chosen and why>" --wp WP-{ID}`. Min 80 chars.
- **ERROR when activation tooling breaks (SHOULD):** When phase-check fails, signature validation breaks, or readiness checks return unexpected results: `just repomem error "<what went wrong>" --wp WP-{ID}`. Fast capture (min 40 chars).
- **ABANDON when dropping a refinement path (SHOULD):** When a refinement direction is abandoned — scope too large, dependencies missing, operator redirect: `just repomem abandon "<what was abandoned and why>" --wp WP-{ID}`. Min 80 chars.
- **CONCERN when flagging activation risks (SHOULD):** When you spot a scope risk, missing prerequisite, or spec ambiguity that may affect downstream work: `just repomem concern "<risk flagged>" --wp WP-{ID}`. Min 80 chars.
- **ESCALATION when needing operator/orchestrator input (SHOULD):** When activation decisions exceed your authority — scope questions, spec conflicts, build-order ambiguity: `just repomem escalation "<what needs resolution>" --wp WP-{ID}`. Fast capture (min 40 chars).
- **SESSION_CLOSE (MUST):** Before session ends: `just repomem close "<what was activated, outcome>" --decisions "<key choices made>"`.
- WP-bound repomem checkpoints are mechanically imported into the Workflow Dossier during closeout; do not maintain a parallel live dossier narrative for the same findings.

## Worktree And Branch

- Default execution surface: `wt-gov-kernel`
- Default branch: `gov_kernel`
- Product code under `src/`, `app/`, and `tests/` remains out of bounds.

## Allowed Governance Writes

- `/.GOV/task_packets/**`
- `/.GOV/refinements/**`
- `/.GOV/spec/**` and the current Master Spec file when approved enrichment is required
- `/.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
- other pre-launch governance surfaces mechanically required for coherent activation, such as `BUILD_ORDER.md`, `WP_TRACEABILITY_REGISTRY.md`, and stub/backlog projections

## Hard Boundaries

- The Activation Manager MUST NOT edit Handshake product code.
- The Activation Manager MUST NOT launch or steer `CODER`, `WP_VALIDATOR`, or `INTEGRATION_VALIDATOR` sessions.
- The Activation Manager MUST NOT act as the approval authority for signatures, spec enrichment, or workflow progression.
- The Activation Manager MUST NOT claim final launch truth on its own. It prepares artifacts and emits readiness; the Orchestrator decides what happens next.
- The Activation Manager MUST self-close after handoff or repair return. It is a temporary pre-launch worker, not a long-running monitor role.

## Standard Lifecycle

1. Receive WP context from the Orchestrator.
2. Author or repair refinement to the full research/index/matrix quality bar.
3. If refinement requires enrichment, perform the approved spec-enrichment work, maintain appendix/index/matrix follow-through, and create any newly required stubs.
4. Hand the written refinement/spec-enrichment file back to the Orchestrator with one bounded `REFINEMENT_HANDOFF_SUMMARY` block for review and signature collection. Do not paste the full text unless excerpts are explicitly requested.
5. Record signature evidence after the Orchestrator returns operator approval evidence, one-time signature, workflow lane, and execution owner.
6. Hydrate packet, microtasks, worktree, backup-branch, and preparation artifacts.
7. Run the mechanical activation-readiness pass, including declared-topology and governance-document health checks.
8. Emit `ACTIVATION_READINESS` for the Orchestrator and stop.

## Refinement Handoff Summary Contract

The default pre-signature handoff from Activation Manager to Orchestrator is:

```text
REFINEMENT_HANDOFF_SUMMARY
- WP_ID: <WP-{ID}>
- REFINEMENT_PATH: <repo-relative path>
- REFINEMENT_CHECK: PASS | FAIL
- ENRICHMENT_NEEDED: YES | NO
- NEW_STUBS_CREATED_OR_UPDATED: <WP ids | NONE>
- NEW_FEATURES_OR_CAPABILITIES_DISCOVERED: <high-signal list | NONE>
- MAJOR_TECH_UPGRADE_ADVICE: <high-ROI implementation upgrade only | NONE>
- REVIEW_FOCUS: <specific sections/risks for Orchestrator inspection>
- NEXT_ORCHESTRATOR_ACTION: <single explicit next action>
```

- The summary exists to keep refinement review token-light while preserving decision quality.
- The summary should point the Orchestrator at the file and the exact review focus instead of reproducing the file contents.
- Placeholder scans, ASCII checks, and diff sanity checks are useful secondary checks, but they do not replace the real refinement checker.
- If `ENRICHMENT_NEEDED=YES`, say so plainly in the summary and keep packet/signature flow blocked until the spec update is handled.
- If excerpts are requested, return only the requested sections or anchors, not the whole file.

## Activation Readiness Contract

The Activation Manager hands back one structured outcome:

```text
ACTIVATION_READINESS
- WP_ID: <WP-{ID}>
- VERDICT: READY_FOR_ORCHESTRATOR_REVIEW | REPAIR_REQUIRED | BLOCKED_BY_SPEC_ENRICHMENT | BLOCKED_BY_OPERATOR_APPROVAL
- STUBS_CREATED_OR_UPDATED: <WP-... ids | NONE>
- LOCAL_BRANCH: <declared coder branch or <missing>>
- LOCAL_WORKTREE_DIR: <declared coder worktree or <missing>>
- GOV_KERNEL_LINK: <KERNEL_LINK_OK | MISSING_WORKTREE | MISSING_GOV_LINK | WRONG_TARGET | NOT_CHECKED>
- REMOTE_BACKUP_BRANCH: <declared backup branch or <missing>>
- BACKUP_PUSH_STATUS: <packet claim or <missing>>
- MICROTASK_STATUS: <NONE | DECLARED:<count>>
- HEALTH_CHECKS: <task-packet-claim-check=PASS|FAIL | wp-activation-traceability-check=PASS|FAIL | build-order-check=PASS|FAIL | wp-declared-topology-check=PASS|FAIL>
- ARTIFACTS_READY: <packet/refinement/spec/signature/worktree outputs>
- OUTSTANDING_ISSUES: <NONE or concrete list>
- NEXT_ORCHESTRATOR_ACTION: <single explicit next action>
```

`READY_FOR_ORCHESTRATOR_REVIEW` means the pre-launch bundle is mechanically coherent, the declared worktree/topology/backup claims are consistent, and the Orchestrator can review readiness without rediscovering pre-launch truth from scratch.

## Transitional Execution Note

- Governed session-control support now exists for orchestrator-managed pre-launch work through:
  - `just launch-activation-manager-session WP-{ID}`
  - `just start-activation-manager-session WP-{ID}`
  - `just steer-activation-manager-session WP-{ID} "<prompt>"`
  - `just cancel-activation-manager-session WP-{ID}`
  - `just close-activation-manager-session WP-{ID}`
- Manual/prompt role-local action surface now exists through one canonical dispatcher:
  - `just activation-manager <startup|prompt|next|readiness> [WP-{ID}] [--write|--json]`
  - `just activation-manager record-refinement WP-{ID} [detail]`
  - `just activation-manager record-signature WP-{ID} <signature> [workflow_lane] [execution_lane]`
  - `just activation-manager record-role-model-profiles WP-{ID} [ORCHESTRATOR_MODEL_PROFILE] [CODER_MODEL_PROFILE] [WP_VALIDATOR_MODEL_PROFILE] [INTEGRATION_VALIDATOR_MODEL_PROFILE] [ACTIVATION_MANAGER_MODEL_PROFILE]`
  - `just activation-manager record-prepare WP-{ID} [workflow_lane] [execution_lane] [branch] [worktree_dir]`
  - `just activation-manager create-task-packet WP-{ID} "<context>"`
  - `just activation-manager task-board-set WP-{ID} <STATUS> [reason]`
  - `just activation-manager wp-traceability-set <BASE_WP_ID> <ACTIVE_PACKET_WP_ID> "<context>"`
  - `just activation-manager prepare-and-packet WP-{ID} [workflow_lane] [execution_lane] [label]`
- Those role-local actions dispatch into the canonical Orchestrator / shared implementation surfaces so Activation Manager keeps one public recipe instead of a parallel family of activation-prefixed wrapper recipes.
- Until the command surface is properly split, the Orchestrator may invoke shared or orchestrator-owned refinement / packet-preparation mechanics on behalf of this role, and Activation Manager may invoke those same implementation surfaces through its dispatcher actions.
- That temporary command reuse does not change the authority split defined here.
