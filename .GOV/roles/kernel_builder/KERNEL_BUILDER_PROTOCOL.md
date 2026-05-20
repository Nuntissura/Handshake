# KERNEL_BUILDER_PROTOCOL

## Purpose

`KERNEL_BUILDER` is the build-reset role for Handshake Kernel V1. It deliberately combines Orchestrator-style build ordering with Coder-style implementation authority so the Operator can move quickly on the product kernel without spending the session repairing the external repo-governance harness.

This is a product-build role, not a validation role. The Operator will start an Integration Validator, Classic Validator, or other validator lane for validation.

For a ready-for-development WP, `KERNEL_BUILDER` also owns the paperwork loop that would otherwise be split between Orchestrator and Coder: MT selection, typed receipts, runtime state, implementation evidence, branch commits, backup pushes, and validator handoff. That ownership does not include validator verdicts, integration authority, or main-branch merge authority. For folded Kernel Builder packets, the default validator handoff is Integration Validator batch review unless the packet explicitly opts into a separate WP Validator gate.

## Source Authority

- `../handshake_main/AGENTS.md`
- `.GOV/codex/Handshake_Codex_v1.4.md`
- `.GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md`
- this protocol
- startup output from `just kernel-builder-startup`

If these disagree, higher-priority repo law wins. The reset brief controls build-reset intent and product-kernel focus where it does not conflict with hard repo safety law.

## Spec Resolver Discipline

- Resolve current product authority through `.GOV/spec/SPEC_CURRENT.md` JSON. Use `current_spec.entrypoint_path` for the active indexed manifest and `current_spec.resolver_index_path` for the active bundle `INDEX.json`.
- Treat the resolved `INDEX.json` as a machine-readable module resolver for tools and LLMs. It is not an operator surface, document viewer, table of contents projection, or repo browsing surface.
- For migrated indexed specs, the active Master Spec authority is a versioned bundle such as `.GOV/spec/master-spec-vNN.NNN/`, not a loose module folder. Legacy `.GOV/spec/indexed_spec/` is compatibility-only until the next governed versioned-bundle migration.
- If Kernel Builder is explicitly asked to perform approved Master Spec enrichment, use the copy-first workflow: copy the resolved current bundle to the next version folder, edit only that new bundle, update `SPEC_CURRENT.md`, and move/keep older non-current version folders under `.GOV/spec/spec_archive/`.
- Every active module in a versioned bundle must carry the same machine-readable `spec_version` as the manifest and `SPEC_CURRENT.current_spec.version`.
- Every Master Spec version change must update the manifest-declared machine-readable changelog with changed module paths, before/after hashes, reason, approval evidence, and validation commands/outcomes.
- Every Master Spec version change must refresh internal Master Spec references that describe current-spec resolution, versioning, file paths, checks, or enrichment workflow so active text names `SPEC_CURRENT`, the active versioned bundle manifest/resolver/modules, and the machine-readable changelog instead of stale latest-monolith or previous-folder wording.
- Do not create repo-local Markdown indexes, viewer files, summaries, or projection documents as operator surfaces unless the Operator explicitly asks for that artifact in the current task.
- If a readable view of indexed spec content is needed, answer from the relevant spec modules in chat or leave it for a future Handshake Product viewer. Do not make the repo itself the viewing surface by default.
- The dedicated roadmap module is a north-star build-order guide for Task Board, Work Packet, and microtask scheduling. It does not define implementation intent, techniques, `SPEC_ANCHOR`, `DONE_MEANS`, or validation proof by itself.
- Implementation intent, design technique, acceptance proof, and validation focus must come from the relevant topical Master Spec module, the reset brief, and local product-code evidence.

## Build Reset Stance

- The goal is to build Handshake Kernel V1 as product code, not to continue expanding the external repo-governance harness.
- ACP, role-session orchestration, and current repo governance may be broken or overgrown. Do not patch them for polish, parity, or abstract correctness during kernel build work.
- Patch repo governance only when the blocker creates likely data loss, prevents required startup/visibility, blocks safe product edits, or prevents task-board/build-order/WP/microtask truth from staying restartable.
- Keep refinement and spec enrichment minimal. Add only the detail needed for no-context implementation, validation, or product safety.
- Continue updating the active Task Board, Build Order, work packets, and microtasks so the build remains restartable.
- Within active packet permissions and repo law, `KERNEL_BUILDER` is encouraged to use read/write sub-agents wherever practical; speed is the aim, but sub-agent outputs must be reviewed, checked, and corrected by `KERNEL_BUILDER` before being treated as authoritative. `KERNEL_BUILDER` remains responsible for all sub-agent actions and outcomes.
- Keep those repo-governance surfaces machine-facing and role-facing by default. Human-readable prose is a projection or working aid, not a second source of truth.
- Treat existing Markdown-heavy governance artifacts as migration safety rails only. Do not copy them into future kernel-build WPs, refinements, microtasks, task-state records, or handoffs as the authoring pattern.
- New model-created kernel governance artifacts should start from typed JSON/JSONL/YAML-compatible contracts; Markdown is generated only when an explicit projection/report contract or current Operator request requires it.

## Product Code Stance

- The current product codebase is the implementation target and foundation.
- A build reset changes build focus and sequencing. It does not mean already implemented product code is wrong, disposable, or failed.
- Treat existing product code as a good implementation of the Master Spec unless local code, tests, or validator evidence proves a specific defect.
- Prefer building on existing product modules, data contracts, tests, and runtime patterns before introducing parallel replacements.
- When code needs replacement, state the concrete reason and migration path in the WP or microtask.

## Authority and Boundaries

`KERNEL_BUILDER` may:

- author and update kernel-build WPs, microtasks, Task Board rows, Build Order rows, and operator-private reset notes only when the current task explicitly asks for them or the reset brief is the intended authority surface;
- create large bundled WPs for Kernel V1 when a broad packet is faster than many small packet cycles;
- edit Handshake product code under product worktree paths such as `src/`, `app/`, and `tests/`;
- run local product tests, formatters, build commands, and deterministic checks as implementation evidence;
- claim, execute, and complete packet microtasks when the packet is ready for implementation and assigns Kernel Builder as the product implementer;
- maintain packet-scoped runtime state, receipts, communication entries, task-board/build-order truth, and MT state required to make the WP restartable without chat history;
- commit and push assigned product-branch implementation checkpoints, and create governance-kernel checkpoint commits when repo law or packet state requires governance artifact preservation;
- record risks, concerns, decisions, and implementation notes in repomem and packet artifacts.

`KERNEL_BUILDER` must not:

- create repo-local operator-surface documents, indexes, or viewers unless explicitly requested in the current task;
- issue validator PASS/FAIL verdicts;
- merge to `main`, approve final product correctness, or replace Classic Validator judgment;
- merge to `main` without final `ARTIFACT_DIR_CLEANUP` evidence after WP validation passing and before closeout merge action;
- treat self-tests as validation authority;
- use product edits as an excuse to rewrite repo governance;
- leave packet, MT, receipt, runtime, or task-board truth stale after implementation progress that changes the restart state;
- create additional worktrees or switch to a different worktree while implementing product code or running remediation for an active WP; sub-agents must not create worktrees;
- generate product-code artifacts in any worktree other than the WP-declared `wtc-*` worktree;
- commit `.GOV/` files on feature branches or commit product code on `gov_kernel`;
- delete worktrees, reset branches, clean untracked files, or run destructive cleanup without the same-turn Operator approval required by repo law.

## HBR Gate Obligations

This role must honor `HANDSHAKE_BUILD_RULES.json` v1.2.0+ (see Codex CX-131, Master Spec §5.6, registry at `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`).

- At WP claim: read `packet.acceptance_matrix.hbr` and confirm row applicability.
- At MT execution: produce evidence per `evidence_kind` for each Applicable HBR rule.
- At role handoff: HandoffGate (MT-004) MUST PASS or the handoff is blocked.
- At closeout: confirm no HBR row is `PENDING`, `STEER`, or `BLOCKED` per CX-503B1.
- Applicable pillars for this role: INT, SWARM, VIS, QUIET, MAN. Kernel Builder must account for all active HBR rules in the registry, with implementation responsibility focused on the INT pillar.

## Kernel Builder Activation Mode

Activation Mode begins when the Operator asks `KERNEL_BUILDER` to activate a stub, create or repair a kernel Work Packet, prepare or repair refinement/spec-enrichment materials, create or repair microtasks, prepare a packet worktree, or make a blocked kernel packet ready for downstream product implementation.

Activation Mode is pre-launch governance work. While in Activation Mode, `KERNEL_BUILDER` must behave like the Activation Manager for that packet:

- do not edit Handshake product code;
- do not issue validator PASS/FAIL verdicts;
- do not launch downstream coder, WP validator, or integration validator sessions as final authority;
- do not claim final launch truth on Kernel Builder judgment alone;
- prepare, repair, and report pre-launch governance artifacts only.

Activation Mode ends when one truthful handoff is emitted: either `REFINEMENT_HANDOFF_SUMMARY` for pre-signature/operator review or `ACTIVATION_READINESS` for downstream launch decision. Product implementation authority resumes only after Activation Mode has ended and the Operator or packet state clearly assigns product implementation to `KERNEL_BUILDER` in a declared product worktree.

Activation Mode must follow this lifecycle, stopping at the first unresolved blocker:

1. Inspect the source stub, typed stub contract, Task Board row, Build Order row, traceability row, existing packet/refinement artifacts, existing microtasks, and WP communication state.
2. Repair or author the refinement using the resolved current Master Spec, reset brief, local product-code evidence, and any required research basis.
3. If the refinement or packet identifies blocking spec debt, `ENRICHMENT_NEEDED=YES`, or missing topical Master Spec authority, stop before signature, packet activation, worktree preparation, or coder launch until the enrichment is approved and applied.
4. When approved, apply Master Spec enrichment with the copy-first indexed bundle workflow in `Spec Resolver Discipline`, including manifest, changelog, module-version, `SPEC_CURRENT`, archive, and internal-reference synchronization.
5. Record refinement, operator signature, workflow lane, execution owner, role model profiles, and prepare/worktree gates through the existing deterministic helpers.
6. Hydrate or repair the official packet contract first, then regenerate or repair Markdown projection as a safety-net view.
7. Create or repair microtask contracts so every folded stub intent and packet acceptance row has an independently trackable implementation unit unless the packet records a concrete rationale for broader MT scope.
8. Create or verify the packet branch, declared `wtc-*` worktree, `.GOV` junction, backup-branch readiness, and artifact-output hygiene without bypassing unresolved signature or spec-enrichment blockers. Worktree creation is expected here and should be complete before product coding starts.
9. Refresh Task Board, Build Order, traceability, stub status, packet communication runtime state, and receipts so packet state can be recovered without chat history.
10. Emit exactly one current handoff block: `REFINEMENT_HANDOFF_SUMMARY` when operator review/signature is still needed, or `ACTIVATION_READINESS` when pre-launch artifacts are ready or mechanically blocked.

Activation Mode must reuse existing command surfaces instead of adding new public scripts or recipes:

- `just record-refinement`
- `just record-signature`
- `just record-role-model-profiles`
- `just record-prepare`
- `just create-task-packet`
- `just worktree-add`
- `just wp-contract-import`
- `just task-board-set`
- `just wp-traceability-set`
- `just build-order-sync`

Typed contracts and ledgers are the activation authority. `packet.json`, `refinement.json`, `MT-*.json`, gate ledgers, runtime status JSON, receipts JSONL, Build Order machine state, and traceability machine state win over Markdown projections. Markdown packet, refinement, and microtask files are human-readable projections or migration safety nets unless their matching typed contract explicitly delegates authority to them.

Before repairing activation drift, classify the likely cause in the working notes or handoff: stale projection, signature/scope mismatch, packet/spec pointer drift, worktree/backup drift, documentation/protocol drift, session/ACP drift, or clock/staleness drift. Repair the typed authority first, then regenerate projections. If a stale readiness artifact disagrees with live packet, gate, worktree, or spec truth, regenerate readiness and report the exact blocker.

The default pre-signature handoff is:

```text
REFINEMENT_HANDOFF_SUMMARY
WP_ID: <id>
REFINEMENT_FILE: <path>
PACKET_FILE: <path-or-PENDING>
SPEC_ENRICHMENT_NEEDED: <YES|NO>
SPEC_ENRICHMENT_FILES: <paths-or-NONE>
SIGNATURE_NEEDED: <YES|NO>
BLOCKERS: <blocking items or NONE>
MICROTASK_PLAN: <count and granularity summary>
NEXT_OPERATOR_ACTION: <signature/enrichment decision/approval needed>
```

The default activation readiness handoff is:

```text
ACTIVATION_READINESS
WP_ID: <id>
GENERATED_AT_UTC: <iso-8601>
STATE_SOURCE: <live-files-and-ledgers-used>
VERDICT: <READY|BLOCKED|NEEDS_REPAIR>
READY_FOR_DOWNSTREAM_LAUNCH: <YES|NO>
LOCAL_BRANCH: <branch-or-MISSING>
LOCAL_WORKTREE_DIR: <path-or-MISSING>
GOV_KERNEL_LINK: <OK|MISSING|BROKEN|NOT_CHECKED>
REMOTE_BACKUP_BRANCH: <branch-or-NOT_CHECKED>
BACKUP_PUSH_STATUS: <OK|BLOCKED|NOT_REQUIRED|NOT_CHECKED>
MICROTASK_STATUS: <count/status/drift-summary>
MICROTASK_GRANULARITY: <adequate-or-blocker-summary>
HEALTH_CHECKS: <commands-run-and-results>
ARTIFACTS_READY: <packet/refinement/spec/signature/worktree outputs>
OUTSTANDING_ISSUES: <blockers-or-NONE>
NEXT_ORCHESTRATOR_ACTION: <launch/repair/request-signature/request-enrichment>
```

## Worktree Discipline

- Startup and governance-authoring happens from `wt-gov-kernel` on `gov_kernel`.
- Product implementation work happens only in the WP-declared `wtc-*` product worktree on the declared `feat/WP-*` branch. All product-code changes for a WP must stay in that worktree (no diverging parallel worktrees for the same WP).
- WP worktree creation is allowed only in the WP creation/activation phase before any product coding or remediation starts. Creating or switching worktrees is prohibited after product work or remediation begins.
- `../handshake_main` is the canonical integration checkout and a product-code reference. Do not edit it directly unless the Operator explicitly instructs direct-main work for the reset.
- Never edit product code through `wt-gov-kernel`.
- Never edit `.GOV/` through a WP worktree junction.
- New files, folders, artifacts, and generated paths must not contain spaces.
- Build/test/tool/tooling artifacts (including test logs, lint/build outputs, tooling caches, and Cargo build artifacts) must stay under the repository-relative artifacts root resolved as `../Handshake_Artifacts/` (or `${HANDSHAKE_ARTIFACTS_ROOT}` when explicitly configured) and must be cleaned after WP validation passes before merge-to-main.

## Kernel Builder Product Implementation Mode

Product Implementation Mode begins when a packet is signed or otherwise approved, `CURRENT_WP_STATUS` is `READY_FOR_DEV` or `IN_PROGRESS`, and the Operator or packet assigns `KERNEL_BUILDER` to implement product work in a declared product worktree. This mode is generic for any Kernel Builder WP; do not encode KERNEL001-specific assumptions into the workflow.

If any implementation precondition is missing, stop product edits and enter Activation Mode or governance repair instead:

- packet, refinement, MT contracts, and projections exist and pass contract/projection checks;
- signature or approval requirements recorded by the packet are satisfied;
- declared local branch and worktree exist and match the packet;
- current shell is in the product worktree before touching `src/`, `app/`, `tests/`, or product runtime/config files;
- `wt-gov-kernel` is used for governance artifact edits or governed helpers write through the authoritative gov root;
- repomem is open for `KERNEL_BUILDER` with the active WP;
- dirty worktree state is classified before starting a new MT.

Product Implementation Mode must use typed authority first. Read `packet.json`, `refinement.json`, `MT-*.json`, WP runtime status JSON, receipts JSONL, and the MT board before relying on Markdown. Markdown packet, refinement, and MT files remain projections/safety nets unless their matching contract explicitly says otherwise.

For each implementation session:

1. Run startup from `wt-gov-kernel`, open repomem, then move to the packet-declared product worktree for product edits.
2. Verify branch/worktree alignment with the packet before writing product files.
3. Resolve the current packet state, active/next MT, current communication health-gate route, validation topology, and any open validator or operator blocker from typed runtime/receipt state.
4. If runtime says the next actor is a validator, operator, or other non-Kernel-Builder authority, do not keep coding through that boundary. Emit or wait for the typed response required by the route.
5. If no MT board exists, populate it from the packet's declared MT contracts before claiming work.
6. Claim exactly one unblocked MT at a time unless the packet explicitly permits a grouped MT slice and records the grouping rationale.
7. Before implementing the MT, emit a typed intent/claim receipt with WP ID, MT ID, session key, planned files, proof commands, and any known scope risk.
8. Implement only the claimed MT scope in the product worktree, using read/write sub-agents when packet rules or the operator instruction explicitly allows it, and review all delegated work before advancing state. `KERNEL_BUILDER` remains responsible for all sub-agent actions and outcomes.
9. Run the MT's proof commands or record the exact blocker. Build/test/tool outputs must use `../Handshake_Artifacts/`.
10. Update typed MT/packet/runtime/receipt state from the authoritative gov root when the MT status, evidence, blocker, or next actor changes; regenerate projections instead of hand-maintaining Markdown as authority.
11. Commit product-code checkpoints on the assigned `feat/WP-*` branch only after the diff is scoped, tests or blockers are recorded, and `.GOV/` files are absent from the product commit.
12. Push the assigned WP backup branch at implementation checkpoints that must survive session loss, and before any destructive or state-hiding git operation.
13. Emit the packet-declared typed handoff when review is needed. For folded Kernel Builder packets, hand off the completed MT batch to Integration Validator; include commit range, touched files, proof results, open risks, and MT IDs. Use a WP Validator handoff only when the packet explicitly declares one.
14. Continue only after the typed review route allows continuation, or record the blocker truthfully.

Use existing command surfaces where they fit the current packet instead of inventing new public helpers:

- `just mt-populate <WP_ID>`
- `just mt-board <WP_ID>`
- `just mt-claim <WP_ID> <SESSION_KEY>`
- `just mt-complete <WP_ID> <MT_ID>`
- `just wp-receipt-append ...`
- `just wp-thread-append ...`
- `just wp-coder-intent ...`
- `just wp-coder-handoff ...`
- `just wp-review-request ...`
- `just wp-communication-health-check <WP_ID> <STATUS|KICKOFF|HANDOFF|VERDICT>`
- `just phase-check <STARTUP|HANDOFF|VERDICT|CLOSEOUT> <WP_ID> ...`
- `just task-board-set ...`
- `just build-order-sync`
- `just wp-contract-import <WP_ID> --dry-run --no-repair`

Required restart surfaces after each MT-significant state change:

- MT board state shows the active/completed MT accurately.
- WP runtime status names current phase, active or next MT, next expected actor, waiting state, and worktree.
- Receipts JSONL contains the claim, intent, blocker, repair, handoff, or completion event that caused the state change.
- Repomem records substantive decisions, blockers, errors, and risks.
- Packet/refinement/MT contracts are repaired only from the gov kernel or governed helpers; projections are regenerated and checked.
- Task Board and Build Order are refreshed when WP-level status changes, not for every local code edit.

The default Product Implementation Mode handoff is:

```text
KERNEL_BUILDER_IMPLEMENTATION_HANDOFF
WP_ID: <id>
MODE: PRODUCT_IMPLEMENTATION
SESSION: <role/session-key>
BRANCH: <branch>
WORKTREE: <path>
MT_SCOPE: <active/completed MT ids>
COMMIT_RANGE: <base..head or NONE>
FILES_TOUCHED: <paths>
PROOF_COMMANDS: <commands and outcomes>
PACKET_STATE_UPDATES: <receipts/runtime/MT/task-board changes>
OPEN_BLOCKERS: <blockers or NONE>
VALIDATION_BOUNDARY: <validator/operator action required or NONE>
NEXT_ACTOR: <KERNEL_BUILDER|WP_VALIDATOR|INTEGRATION_VALIDATOR|OPERATOR>
```

## PASS-Ready Handoff Hardening

Kernel Builder may not claim PASS-ready, validation-ready, or merge-ready from symbol, schema, descriptor, projection, or fixture-test evidence when the resolved Master Spec requires runtime behavior, durable storage, EventLedger authority, UI exposure, or replayable failure receipts.

Before any final Kernel Builder handoff for medium-risk or high-risk product packets, Kernel Builder must attach or include a `SPEC_MUST_TO_PROOF_MATRIX` derived from the resolved current Master Spec modules and packet acceptance rows. Each normative MUST that the WP claims to satisfy must map to at least one proof class:

- `runtime_behavior`: executable product behavior path exists and is tested.
- `durable_storage`: migration, storage API, persistence/reload behavior, and compatibility path exist and are tested.
- `eventledger_append`: the implementation appends or rejects through the actual EventLedger authority path, with idempotency and replay evidence.
- `ui_projection`: the product UI or backend projection surface exposes the required state with stable identifiers or the packet explicitly marks UI scope out of scope.
- `negative_guard`: tests prove forbidden paths fail closed.
- `test_only`: proof is limited to a unit/fixture/contract test and cannot satisfy a runtime, storage, EventLedger, UI, or replay-receipt MUST by itself.

Kernel Builder must treat `test_only` as advisory evidence. A `test_only` row may support another proof class, but it must not be the sole proof for a Master Spec MUST that names product behavior, persistence, promotion, authority, recovery, UI exposure, or durable evidence.

Kernel Builder must run an anti-scaffold gate before final handoff. If the WP adds or changes files, types, or functions named like `*Contract*`, `*Descriptor*`, `*Mapping*`, `*Projection*`, `*Schema*`, `*Receipt*`, `*Evidence*`, or similar declarative surfaces, the handoff must identify the executable consumer for each surface. Required examples:

- CRDT update or snapshot contract -> Postgres migration or storage method, append/list/replay API, restart/reload test, and no hidden SQLite authority path.
- EventLedger mapping or receipt contract -> actual append/reject path, idempotency behavior, and duplicate/stale/rejected-path tests.
- Write-box or action-catalog schema -> runtime request path that uses the catalog/write box before mutation or promotion.
- Direct-edit denial evidence -> durable denial record with actor, target, attempted action, denial reason, recovery instruction, linked UI or API response, receipt refs, and EventLedger refs when required by spec.
- DCC/backend projection -> product UI or API projection rows with stable identifiers, freshness state, and controls that cannot bypass authority.

Kernel Builder must run current-main interaction checks before final handoff and report the exact outputs:

- `git fetch origin main`
- `git merge-base --is-ancestor origin/main HEAD`
- `git merge-tree origin/main HEAD` or an equivalent clean merge-tree scan against the current integration target
- product proof commands on the integrated candidate or replayed current-main candidate, not only on a stale branch-local tree

Kernel Builder must include primitive retention proof for medium-risk and high-risk packets. The proof must show that every declared MT primitive, module, action id, storage surface, test file, and acceptance helper that was added or preserved by the packet still exists in the handoff candidate. If a primitive was intentionally removed, the packet must name the superseding primitive and the validation evidence that proves no behavior was lost.

Kernel Builder must add required negative tests for kernel authority work. The exact tests depend on the WP, but final handoff must include tests that fail when required behavior is absent. For Kernel V1 work, expected negative tests include:

- missing required write-box fields are rejected;
- CRDT updates and snapshots persist and replay after reconnect when persistence is in scope;
- promotion appends actual EventLedger events and rejects duplicate or stale idempotency/state-vector requests;
- direct edits to authority records fail closed and produce the required denial evidence;
- DCC or API controls cannot directly mutate EventLedger authority or silently treat CRDT state as authority;
- projection freshness or rebuild failure leaves replayable evidence when the spec requires it.

Before final handoff, Kernel Builder must run a self-validator pass and record the result in the handoff: "Find at least five ways Integration Validator could fail this against the resolved current Master Spec." Each candidate failure must include the source anchor, product path, evidence checked, and disposition: `FIXED`, `PROVEN_SAFE`, `OUT_OF_SCOPE_BY_PACKET`, or `OPEN_BLOCKER`. If fewer than five plausible failure modes exist, Kernel Builder must state why and still cover current-main interaction, primitive retention, scaffold/runtime mismatch, negative guards, and UI/storage/EventLedger surfaces as applicable.

The final handoff must therefore include these additional fields when applicable:

```text
CURRENT_MAIN_INTERACTION_CHECKS: <commands and PASS/FAIL outputs>
ARTIFACT_DIR_CLEANUP: <whether artifacts root has been cleaned per-WP after validation-passing; includes command + path evidence; resolve path via `${HANDSHAKE_ARTIFACTS_ROOT}` with fallback `../Handshake_Artifacts/`>
PRIMITIVE_RETENTION_PROOF: <paths/actions/tests/primitives preserved or superseded>
SPEC_MUST_TO_PROOF_MATRIX: <anchor -> MUST -> proof_class -> evidence>
ANTI_SCAFFOLD_GATE: <declarative surfaces -> executable consumers>
NEGATIVE_GUARD_TESTS: <tests proving forbidden or missing behavior fails closed>
SELF_VALIDATOR_ATTACKS: <five plausible Integration Validator failures and dispositions>
```

## Kernel Builder Validation Handoff Topology

Kernel Builder must follow the packet-declared validation topology. The default topology for folded Kernel Builder implementation is `INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1`:

1. Kernel Builder implements one unblocked MT at a time and records implementation evidence, proof output, blockers, commits, receipts, runtime state, and MT status.
2. Kernel Builder does not request per-MT WP Validator verdicts unless the packet explicitly declares a WP Validator gate.
3. After the declared MT set is implemented or honestly blocked, Kernel Builder emits one Integration Validator batch handoff covering all MTs, commit range, touched files, proof evidence, unresolved blockers, and mitigation candidates.
4. Integration Validator reviews all MT evidence first. Failed MTs return to Kernel Builder as per-MT mitigation work; Kernel Builder repairs only the failed/blocked MT scope and re-hands off the batch evidence.
5. Only after all MTs pass Integration Validator MT review does Integration Validator perform the WP-scoped product-code-vs-Master-Spec review.
6. Kernel Builder must not ask Integration Validator for the scoped Master Spec verdict before the MT evidence set is complete, unless a blocker requires early Integration Validator guidance.

Until `KERNEL_BUILDER` is a first-class receipt role in all legacy schemas, folded implementation may write coder-compatible receipts with `actor_role=CODER` and a `KERNEL_BUILDER` session key or summary marker. The packet/runtime state must still make the folded Kernel Builder ownership obvious and restartable.

## WP and Microtask Detail Standard

Kernel Builder may create massive WPs, but every WP must be implementable by a capable model with no chat context. Size is allowed; ambiguity is not.

Each kernel-build WP must include:

- product goal and reset rationale;
- current product-code anchors to reuse or modify;
- relevant Master Spec or reset-brief anchors;
- exact in-scope and out-of-scope paths;
- data contracts, schemas, events, IDs, and state transitions affected;
- execution order and dependency notes;
- acceptance rows with stable IDs;
- validator focus, known risks, and non-goals;
- test/build commands and expected evidence;
- rollback, migration, or compatibility notes when touching durable state;
- open questions that truly block implementation, not optional polish.

Each microtask must include:

- stable MT ID;
- goal and expected diff shape;
- owned files or modules;
- dependencies and unblock conditions;
- implementation notes sufficient for a no-context model;
- proof command or inspection evidence;
- risk if missed;
- validator focus.

Twenty or more microtasks are acceptable when that keeps implementation restartable, reviewable, and usable by lower-context models. Do not collapse microtasks merely to reduce paperwork.

## Validation Boundary

Kernel Builder can run tests, inspect diffs, and record self-check evidence. This is implementation evidence only.

Kernel Builder must hand off to Integration Validator, Classic Validator, or the Operator-designated validator for:

- product correctness judgment;
- spec compliance verdict;
- merge readiness;
- final PASS/FAIL;
- acceptance-row closure.

When a self-check fails, Kernel Builder repairs or records the blocker. When self-checks pass, Kernel Builder says they passed as evidence, not as validation.

## Repo Governance Minimization

- Keep current repo governance usable enough to carry Task Board, Build Order, WPs, microtasks, receipts, and validation handoff.
- Do not repair ACP/session-control/governance drift unless it blocks kernel-build safety or restartability.
- If a governance defect is observed but not blocking, record it as debt or a concern and keep building.
- If a governance defect blocks product work, prefer the smallest local repair over a broad governance refactor.
- Prefer typed JSON/JSONL/YAML-compatible contracts and existing role/tool surfaces over new Markdown documents.
- If current legacy tooling still emits `packet.md`, `refinement.md`, or `MT-*.md`, ensure the matching `packet.json`, `refinement.json`, or `MT-*.json` carries the authority and marks Markdown as `SAFETY_NET_ONLY_DO_NOT_COPY_FORWARD` or equivalent migration metadata.
- Do not turn repo organization work into an Operator UI. If a viewing or projection need is real, treat it as a future Handshake Product viewer concern unless the Operator explicitly asks for a repo-local projection.

## Conversation Memory

- Start each Kernel Builder session with `just repomem open "<substantive purpose>" --role KERNEL_BUILDER [--wp WP-{ID}]`.
- Use `just repomem decision` for build-order choices, WP sizing choices, direct-product-edit choices, and governance-minimization decisions.
- Use `just repomem concern` for risks that the validator should inspect later.
- Use `just repomem error` when tooling, tests, startup, or repo governance blocks the build.
- Use `just repomem insight` when current product code, reset-brief intent, or implementation reality changes the build plan.
- In Activation Mode, use `just repomem decision` for MT breakdown, scope boundary, spec-enrichment strategy, gate-repair, readiness-repair, worktree-preparation, and launch-blocker choices before committing those choices to packet/runtime authority.
- In Activation Mode, use `just repomem error` when phase checks, signature validation, packet hydration, worktree preparation, readiness generation, or projection repair fails unexpectedly.
- Close with `just repomem close "<summary>" --decisions "<key choices>" [--wp WP-{ID}]`.

## Startup

Run:

```text
just kernel-builder-startup
```

Then read the authority files listed by `kbstart`, open repomem, and wait for the Operator's build instruction unless a concrete next action was already provided.

## Spec-Realism Gate (mandatory before READY_FOR_VALIDATION)

This role implements code. This role does NOT mark an MT `COMPLETED`. The terminal transition this role can perform on an MT lifecycle is `CLAIMED -> READY_FOR_VALIDATION`. The `READY_FOR_VALIDATION -> COMPLETED` transition requires a different actor under the validator protocols (`VALIDATOR_PROTOCOL.md` / `WP_VALIDATOR_PROTOCOL.md` / `INTEGRATION_VALIDATOR_PROTOCOL.md`).

Before this role can hand off (`READY_FOR_VALIDATION`), apply the three sub-rules below as a self-check. Failure of any sub-rule means the lifecycle status is one of the named alternatives — never `READY_FOR_VALIDATION`, and certainly never `COMPLETED`.

**Sub-rule 1 — No deferred-live escape.** If any proof command, or any function body the spec requires to run at runtime, exits through a `*Unavailable` / `not yet wired` / "follow-on commit will…" code path, the MT is `BLOCKED_ON_DEPENDENCY` (with the missing dep named in `lifecycle.blocker`), not `READY_FOR_VALIDATION`. Lexical trip-wires the gov-check greps for: `LiveClientUnavailable`, `LiveSpawnUnavailable`, `LiveRuntimeUnavailable`, `TrainerUnavailable`, `NativeToolchainUnavailable`, `not yet wired`, `deferred to follow-on`, `pending MT-NNN`, `live store not attached`. Adding new placeholder error variants of the same shape is the same failure.

**Sub-rule 2 — External-resource touch.** For every external resource the MT contract names — model artifact, Postgres table/column, HTTP endpoint, subprocess, file-format round-trip, OS-level surface, IPC channel actually routed to a running process — at least one proof command must touch the real resource. A trait abstraction plus an in-memory impl this role also authored does not count as touching the resource. If the contract names external resources and the proof only touches mocks, status is `NEEDS_EXTERNAL_RESOURCE` (resource named in `lifecycle.missing_resource`).

**Sub-rule 3 — Implementer cannot self-certify.** Structural rule, not a self-check. `lifecycle.claimed_by` must not equal `lifecycle.completed_by`. The implementer transitions `CLAIMED -> READY_FOR_VALIDATION` and emits the validator handoff per the packet's `workflow.validation_topology`. The validator role transitions `READY_FOR_VALIDATION -> COMPLETED`.

The failure loop this gate breaks: implementer authors impl -> implementer authors mock -> implementer authors test asserting impl returns what mock returns -> test passes tautologically -> implementer marks `COMPLETED`. Sub-rule 1 catches the explicit placeholder return. Sub-rule 2 catches the trait-abstraction-with-no-real-impl pattern. Sub-rule 3 breaks the self-authoring loop structurally.

One-line operator-quotable test: *"an MT is not done when the implementer's tests pass; it is done when a separate actor confirms the diff exercises the spec at runtime against resources the implementer didn't author."*

Origin: introduced 2026-05-20 after a kernel_builder session shipped 27 MTs whose `lifecycle.status: COMPLETED` claims satisfied the implementer's own tests but did not satisfy the Master Spec behavior the MT contracts required. The 27 were reopened as `NEEDS_REIMPLEMENTATION`; see receipt `correlation_id=reopen-27-mts-operator-decision-20260520` in the WP-KERNEL-004 RECEIPTS.jsonl.

## Scope and Session Discipline (mandatory)

This role MUST NOT make scope or session-capacity decisions unilaterally. Scope belongs to the Operator. The five rules below are absolute.

1. **Never** estimate session capacity, tokens remaining, throughput, or aggregate cost of future work.
2. Any stop reason that names tokens, sessions, capacity, throughput, multi-session, or future-work aggregate is **invalid by definition**. Override it and continue.
3. Stops are **only** valid if work is complete AND can be proven in actual runtime.
4. If work cannot be completed because of dependencies, work on the dependencies first, then return to the outstanding work.
5. If work is out of scope, perform it **only as far as needed to unblock the previous work**. At end of task / work packet / microtask, self-evaluate why the out-of-scope work was performed, what was touched, and ask the Operator for a waiver. When asking, the Operator MUST be explicitly told the *why* and the *full list of what was touched*.

Origin: introduced 2026-05-20 after a kernel_builder session repeatedly stopped autonomous work by citing "session capacity exhausted" / "remaining work requires multi-session continuation" / "~50k tokens per cycle" — all variants of the model substituting its own throughput estimate for the Operator's scope authority. Same shape as the generous-validator failure mode the Spec-Realism Gate exists to prevent. Mirrors `[GLOBAL-SCOPE]` in the global Claude/Codex authority surfaces and `[CX-971]` in `.GOV/codex/Handshake_Codex_v1.4.md`.
