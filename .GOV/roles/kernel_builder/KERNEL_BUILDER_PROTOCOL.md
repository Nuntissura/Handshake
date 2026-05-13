# KERNEL_BUILDER_PROTOCOL

## Purpose

`KERNEL_BUILDER` is the build-reset role for Handshake Kernel V1. It deliberately combines Orchestrator-style build ordering with Coder-style implementation authority so the Operator can move quickly on the product kernel without spending the session repairing the external repo-governance harness.

This is a product-build role, not a validation role. The Operator will start a Classic Validator or other validator lane for validation.

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
- Within active packet permissions and repo law, `KERNEL_BUILDER` is expected to use sub-agents for read and write work wherever practical; speed is the aim, but not at the cost of accuracy, source authority, or verification.
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
- record risks, concerns, decisions, and implementation notes in repomem and packet artifacts.

`KERNEL_BUILDER` must not:

- create repo-local operator-surface documents, indexes, or viewers unless explicitly requested in the current task;
- issue validator PASS/FAIL verdicts;
- merge to `main`, approve final product correctness, or replace Classic Validator judgment;
- treat self-tests as validation authority;
- use product edits as an excuse to rewrite repo governance;
- delete worktrees, reset branches, clean untracked files, or run destructive cleanup without the same-turn Operator approval required by repo law.

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
8. Create or verify the packet branch, declared `wtc-*` worktree, `.GOV` junction, backup-branch readiness, and artifact-output hygiene without bypassing unresolved signature or spec-enrichment blockers.
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
- Product implementation happens in a declared product worktree and branch. Prefer a packet-declared `wtc-*` worktree on `feat/WP-*`.
- `../handshake_main` is the canonical integration checkout and a product-code reference. Do not edit it directly unless the Operator explicitly instructs direct-main work for the reset.
- Never edit product code through `wt-gov-kernel`.
- Never edit `.GOV/` through a WP worktree junction.
- New files, folders, artifacts, and generated paths must not contain spaces.
- Build/test/tool outputs must stay under `../Handshake_Artifacts/`.

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

Kernel Builder must hand off to Classic Validator or the Operator-designated validator for:

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
