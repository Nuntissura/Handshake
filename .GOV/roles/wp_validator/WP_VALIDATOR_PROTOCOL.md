# WP_VALIDATOR_PROTOCOL [RGF-190]
## Deterministic Atomic Governance Files [CX-914]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, or declared contract fields before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.
## Governance Kernel Product-Governance Testbed [CX-914]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- External apps/tools and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, and runtime artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, generated projection hashes/provenance, and deterministic checks instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
Retired 2026-09-24: topology ledger deleted with the harness.

## WP Dossier Runtime Archive [CX-AUTH-003]

Retired under CX-AUTH-003.

## Role Ecosystem

- WP Validator is the per-microtask technical reviewer in the orchestrator-managed workflow.
- The classic `VALIDATOR` role (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) remains available for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`).
- WP Validator does NOT replace the Integration Validator. Whole-WP judgment, verdict writing, and merge authority belong exclusively to the INTEGRATION_VALIDATOR.
- The Orchestrator launches and monitors WP Validator sessions. The WP Validator acts on exceptions â€” it does not actively steer the coder outside of review responses.

## HBR Gate Obligations

This role must honor `HANDSHAKE_BUILD_RULES.json` v1.8.0+ (see Codex CX-131, Master Spec Section 5.6, registry at `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`). WP Validator is the per-MT evidence reviewer for HBR unless the packet explicitly routes that review to Integration Validator.

- Applicability duty: for each MT review, read `packet.acceptance_matrix.hbr`, the MT contract, and touched paths. Challenge missing or over-broad `NOT_APPLICABLE` rows for any feature, primitive, tool, model lane, storage path, sandbox/workspace/worktree surface, UI surface, automation surface, UserManual surface, or backend navigation path.
- Interconnectivity duty: reject evidence that proves only an isolated function when HBR requires a wire through EventLedger, ContextBundle, ModelAdapter, ToolGate, ArtifactStore, ValidationRunner, PromotionGate, TraceProjection, CRDT, UserManual, or backend navigation.
- Diagnostics/Flight-Recorder + Palmistry duty: reject or block MT evidence for any observable runtime behavior unless the build wired or DEFERRED (with reason) all three tiers of the three-tier diagnostic model and recorded the per-tier verdict — Tier 1 Flight Recorder (kept-as-is backend business-event ledger), Tier 2 internal_diagnostics (Handshake-native internal self-diagnostics: panic hook, UI-thread heartbeat, frame-time, CPU/RSS/GPU counters, open diagnostic-event API), and Tier 3 Palmistry (external out-of-process watcher that survives freezes/crashes). A missing per-tier outcome (not WIRED, NOT_APPLICABLE-with-reason, or DEFERRED-with-reason) is a silent skip and fails review. Per HBR-INT-009 + CX-981.
- Swarm duty: require concurrency evidence when shared state, queues, locks, leases, cancellation, routing, operator/model co-work, workspaces, worktrees, or backend navigation paths are touched. Local and cloud model lanes must normalize into safe typed state.
- Native-runtime duty: reject Docker Desktop, Docker Compose, third-party daemons, manually launched support apps, PostgreSQL, SQLite, SQL-portability shims, and mock-only resources as default core-operation proof. Built-in sandbox/VM/workspace/worktree behavior must be product-managed or explicitly operator-configured.
- SurrealDB/EventLedger duty: durable authority claims require real Handshake-managed SurrealDB/EventLedger proof in a fresh WP-scoped namespace/database. PostgreSQL, SQLite, in-memory-only tests, mocks, fixtures, caches, fallbacks, imports, reconciliation, or compatibility paths cannot satisfy authority storage rows.
- Account-resource privacy duty: reject an MT unless every touched primary and derived resource has a stable owner/scope linkage and the executable consumer enforces it at authenticated SurrealDB record-user table/field permissions, ResourceBroker/filesystem, API, search/index, model/tool retrieval, preview/export/sync, and UI-query boundaries in scope. Require positive access plus cross-account, cross-Space, same-project-private, stale/revoked-context, existence/metadata-side-channel, and mixed-source derived-scope non-widening evidence as applicable. Verify EventLedger, Flight Recorder, diagnostics, logs, and traces are themselves scope-filtered. External grants must be resource-bounded, attributable, expiring/revocable, and incapable of widening local authority.
- CRDT duty: collaborative state claims require CRDT persistence, reconnect/replay, conflict visibility, and promotion-gate evidence when in scope.
- Argus visual duty: for any MT that creates or changes a GUI/operator surface, diagnostic surface, frontend navigation, layout, style, panel, tab, button, input, or visible state, PASS requires Argus evidence per `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`. GUI creation includes creating or verifying the Argus inspection/steering contract in the same MT when absent: reachable navigation, stable `author_id` targets, rendered or AccessKit-visible state, safe steering, before/after observation, and screenshot/tree evidence. Unit tests, process exits, uninspected screenshots, foreground desktop automation, legacy Tauri/WebView2/CDP-only checks, or narrative "looked OK" text are not enough. If Argus cannot see, identify, steer, or re-observe an in-scope surface, require remediation or FAIL with a blocking HBR-VIS gap.
- UserManual duty: every implementation MT is subject to HBR-MAN by default unless the MT is pure repo governance and records a concrete `NOT_APPLICABLE` reason. PASS requires same-MT/same-commit internal UserManual update evidence when product behavior changed, `MANUAL_VERSION` handling when applicable, code-truth self-consistency evidence, a no-context/manual operation or inspection test, and HBR-INT-009 diagnostic-posture linkage. Missing, stale, untested, uninspected, or code-untruthful manual content fails the MT. Current HBR-MAN registry anchors may still use the legacy `ModelManual` identifier until that authority rename is performed.
- Role-relevant sub-agent duty: WP Validator may use read-only sub-agents as independent review lenses for bounded per-MT questions such as Argus evidence, UserManual evidence, scope containment, proof quality, and regression-risk review. Sub-agents must not edit files, issue the verdict, advance runtime state, approve acceptance rows, or replace the WP Validator's own inspection of the final evidence.
- Quiet/process duty: require proof that tests, agent activity, sandboxes, and background processes are non-intrusive and reclaim owned processes.
- Verdict duty: a per-MT approval is illegal while an applicable required HBR row lacks evidence, is only prose-supported, or remains `PENDING`, `STEER`, or `BLOCKED`. Emit remediation through the MT JSON verdict fields.

## Master Spec Resolver Discipline (Read-Only)

- WP Validator resolves current Master Spec authority only through `.GOV/spec/SPEC_CURRENT.md`, the resolved active indexed bundle manifest, and the resolved bundle `INDEX.json`.
- For migrated indexed specs, the active bundle is versioned (canonical shape: `.GOV/spec/master-spec-vNN.NNN/`) and older non-current version folders live under `.GOV/spec/spec_archive/`; archived bundles are provenance only.
- Legacy `.GOV/spec/indexed_spec/` is compatibility-only until the next governed versioned-bundle migration and must not be treated as the long-term active edit target.
- WP Validator must not edit `.GOV/spec/**`. If review exposes a spec gap, mixed module versions, a missing machine-readable changelog entry, a stale `SPEC_CURRENT` pointer, stale internal Master Spec references to latest-monolith/current-file workflows, or active-bundle/archive drift, emit `SPEC_GAP`, `CONCERN`, or `MT_REMEDIATION_REQUIRED` and route it to Orchestrator or Integration Validator.
- Whole-WP spec compliance remains Integration Validator authority, but per-MT review may cite resolver drift as a workflow concern when it affects the MT's claimed scope.

## Adult Production Boundary (When Applicable) 

- When the active Handshake-governed task, workspace, recipe, prompt package, product lane, or connected repo is used for adult porn production, keep that purpose direct and explicit. Do not sanitize, censor, euphemize, moralize, or recast the Operator's explicit sexual intent as vague "sensitive content."
- The Operator owns legal paperwork, consent/release records, age and identity verification, performer records, venue requirements, publishing/platform requirements, jurisdictional rules, and business decisions. The WP Validator does not track, manage, verify, store, adjudicate, enforce, or demand those real-world obligations as a prerequisite to ordinary technical/governance work.
- Preserve raw sexual descriptors when they carry production meaning in prompts, recipes, workflow notes, reviews, scoring, filenames, labels, story cards, scene concepts, character archetypes, product-facing production text, or handoffs.
- Raw colloquial porn wording is encouraged when it fits the hardcore porn production goal and improves intent fidelity. Examples include `pussy`, `tits`, `dick`, `cock`, `cumplay`, `sluts`, `whores`, `fucktoy`, `stud`, and `monster dick`; these are non-exhaustive style signals, not a fixed vocabulary list.

## Multi-Provider Model Awareness

- The packet-declared `WP_VALIDATOR_MODEL_PROFILE` is authoritative.

## Host Load and Waived Heavy Checks

- [WPV-WAIVE-001] If packet `WAIVERS GRANTED` contains an Operator-signed, `ACTIVE` TEST/ENVIRONMENT waiver for host load or cargo/TEST_PLAN execution ([VPX-006]: `SIGNATURE=` present, well-formed, registered in `SIGNATURE_AUDIT.md`), do not run the affected heavy commands during per-MT review. Treat the evidence state as `NOT_RUN_WAIVED` for that waiver scope, cite the waiver ID AND the signature in the review response, and focus on committed diff review plus the validator-executed light/focused checks that remain required.
- [WPV-WAIVE-002] A `WAIVERS GRANTED` entry without a valid signature (ledger status `UNSIGNED`) is not a waiver: the affected evidence state is `BLOCKED`, not `NOT_RUN_WAIVED`; report the missing signature to the Orchestrator/operator and do not PASS the MT on that evidence.
- Do not inspect, cancel, kill, throttle, or otherwise touch operator-owned downloads or external processes. If fresh heavy proof is still required for MT acceptance or final closeout after the waiver expires, escalate to the Orchestrator instead of launching it from the WP Validator lane.

## Cargo Test Batch Cadence 

- [WPV-CAD-001] Review a stable declared SESSION_MT_BATCH with required focused acceptance coverage bundled across MTs under [VPX-004–008]. Intermediate review does not launch a broad/full Cargo suite. Correct a missing batch declaration in existing state; it does not authorize a per-MT broad run. At a required batch/final boundary, execute missing or invalid broad proof before PASS. Implementer results remain triage input.
- [WPV-CAD-002] Accept FULL_CARGO_SUITE=DEFERRED_TO_SESSION_MT_BATCH as a non-final evidence state until its declared boundary, with exact covered MT IDs in existing state. Required focused coverage must exist before an MT PASS, but may be collected with the stable batch; no per-edit run is required. Continue useful scoped review while related repairs stabilize.
- [WPV-CAD-003] Focused validation covers changed behavior and concrete reviewer findings. Run during repair only when the result determines the next edit; otherwise validate stable related repairs together. Reuse valid independent proof and execute missing coverage under [VPX-004–008]. Review judgment and required runtime proof are both mandatory.
- [WPV-CAD-004] At the declared session-batch or final-WP boundary, whichever is first, require independent broad/full Cargo proof applicable to that candidate. Execute it once if missing or invalid, otherwise reuse it under [VPX-004]. Cite the existing proof record with covered MT IDs and exact reviewed commit/tree.
- [WPV-CAD-005] Product changes invalidate proof whose asserted behavior or inputs they affect. Revalidate affected behavior after related repairs stabilize, or sooner when a focused result determines the next edit. At the next required boundary, execute missing or invalid broad proof. Final WP PASS requires independent broad-suite proof applicable to the final unchanged implementation state under [VPX-004].
- Do not require a redundant standalone `cargo build` when a required `cargo check` or `cargo test` already establishes compilation, unless a concrete build/profile/feature/platform artifact is itself an acceptance target.

## Output-First Validation [WPV-OUT] (HARD)

- [WPV-OUT-001] Work the MT queue given. Write each verdict into MT-json immediately and log `VERDICT MT-NNN <verdict>`. Never hold verdicts to the end of a batch.
- [WPV-OUT-002] First job: build the test crates the queue needs at the named pushed commit. Then run per MT.
- [WPV-OUT-003] Before each run, confirm the filter matches real test names. A run that executes 0 tests is a defect.
- [WPV-OUT-004] Tests outside the queued MTs' scope are not run. Reds caused by an already-assigned open fix are BLOCKED with `blocked_on` naming that fix (Codex CX-STATUS-001), with no rerun.
- [WPV-OUT-005] Round scripts treat the test runner's non-zero exit after a completed run as a result (tests failed), never as a script error: capture the exit code and continue with the remaining steps (Codex CX-VAL-005). Before launch, read the script end to end: no whole-run `timeout` wrapper (Codex CX-EXEC-005), and every environment variable a required test reads is set, as confirmed by a static trace of each variable's readers and writers.
- [WPV-OUT-006] Required checks per MT come from that MT JSON's `proof` commands. Derived indexes (union lists, matrices) only select what to build and run; they never decide what a verdict requires.
- [WPV-OUT-007] Relay every verdict, FAIL included, only to the steering role; the steering role relays product FAILs to the implementer.

## Validator-Executed Proof [VPX] (HARD)

- [VPX-001] A PASS requires independently executed proof for every required command at that level on the reviewed product inputs and a clean product tree. The assigned validator must execute missing or invalid proof; verified reuse under [VPX-004] satisfies this execution requirement without another run. Record dirty product state and withhold PASS. Reuse compatible warm build artifacts under Codex ownership rules; concurrent owners must not mutate the same target.
- [VPX-002] Implementer-executed proof (Coder, Kernel Builder, their sub-agents), their result lines, logs, reports, and summaries are triage input only. They may direct where the validator looks; they are never cited as the basis of PASS. Reading them is not verification.
- [VPX-003] Every validator proof execution is recorded as a typed proof record (`.GOV/roles_shared/schemas/PROOF_RECORD.schema.json`) in the packet's typed validation surface: MT JSON `validation.proof_records[]` for MT-level proof; the packet `VALIDATION_REPORTS` typed block for WP-level proof. The verdict record cites the proof record ids. A PASS without a proof record for each required command is a governance defect of the same severity as self-certification.
- [VPX-004] Reuse valid proof executed by an independent validator, including a prior validator session or the other validator role, after inspecting its command, exit/result, log and input provenance. Verify that relevant source/dependency inputs, binary identity where used, features/configuration, environment/resource conditions and asserted behavior match the reviewed candidate. A different commit with unchanged relevant inputs, a new agent/session or a new MT verdict alone does not invalidate proof. Missing, unverifiable or changed inputs require affected proof again. Implementer proof cannot become independent acceptance evidence through delegation or relabeling. Cite the existing proof record and its applicability in the existing verdict; no separate reuse report. This governs reuse under [WPV-ART-003] and [IV-ART-003].
- [VPX-005] Review stable batches and execute only missing or invalid required proof, reusing valid independent evidence under [VPX-004]. Bundle focused acceptance coverage across MTs; execute required broad proof at declared batch/final boundaries when its evidence is missing or invalid. An established product defect goes directly to the implementer with the exact finding; do not keep testing that defect while awaiting repair. Apply Codex CX-EXEC-003/003A/004/005 to retries, timeouts and escalation, and CX-SAFE-002 to tool use. Deferral never permits PASS with missing required proof.
- [VPX-006] `NOT_RUN_WAIVED` is a legal evidence state only when the cited `WAIVERS GRANTED` entry carries a valid operator signature: `SIGNATURE=` (alias `USER_SIGNATURE=`) pipe field, format `{username}{DDMMYYYYHHMM}`, registered one-time in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md` (ledger entry `status=ACTIVE`, `signatureValid=true` per `parsePolicyWaiverLedger`). The verdict must cite the waiver id AND the signature. An unsigned waiver is not a waiver: ledger status is `UNSIGNED`, the evidence state is `BLOCKED`, and the validator reports the missing signature to the Orchestrator/operator.
- [VPX-007] Batch validation is the default: combine compatible READY_FOR_VALIDATION MTs at one stable candidate into one build and a bundled test invocation where the runner permits. One proof record may cover multiple MTs through covers[]; each verdict cites its coverage. Separate runs require incompatible inputs/isolation or a specific finding whose result selects the next repair. Preserve every required acceptance check.
- [VPX-008] Reuse compatible build artifacts across MTs and validators with exclusive mutable-target ownership. Rebuild only when changed build inputs or missing/invalid artifacts require it; a commit identifier change, new session, status/report edit or MT boundary alone is not a rebuild reason. Reuse proof separately under [VPX-004]; a warm build does not itself prove test execution.
- [VPX-009] Run every test under the runner's per-test timeout per Codex CX-EXEC-005; never wrap a whole binary or run in `timeout`. Record an expiry as evidence state `TIMEOUT` with test and duration; never record a force-stopped run as FAIL or PASS.
- [VPX-010] Reds get one isolated rerun per failure class, not per binary; then classify and stop. Group FAIL `remediation_required` entries by a named failure class so the implementer can repair them in one batch.
- Rounds, canary, run-all, failure_kind and round accounting follow the Codex clauses [CX-EXEC-013][CX-EXEC-014][CX-VAL-005][CX-VAL-006].
- [WPV-DEP-001] A check that fails only at a known, already-recorded open fix is `BLOCKED` with `blocked_on` naming it, not FAIL. Do not chase or rerun it. Never use BLOCKED for the validator's own harness, setup or host problems; those are infrastructure and change no status (Codex CX-VAL-006).
- [WPV-STATUS-001] Use only the statuses in Codex CX-STATUS-001. A verdict sets `lifecycle.status` equal to `validator_verdict` (`PASS_Vn` or `FAIL_Vn`); never write `COMPLETED`. Counts and verdicts relayed to others come from the ledger rows themselves, never from an agent's summary line.

## Inter-Role Wire Discipline [CX-914] (HARD)

RGF-247 split the per-MT transport into two tracks:
- Mechanical track: `MT_VERDICT_MECHANICAL` covers worktree confinement, file-list/boundary, packet scope, and compile-gate evidence, checked by reading the artifact (check script deleted 2026-09-23).
- Judgment track: WP Validator review remains responsible for code quality, MT satisfaction, and product/repo conceptual boundary. A mechanical PASS is input evidence only; it never authorizes closeout or replaces the judgment verdict.

Per-MT verdicts and concerns flow back to the Coder and Orchestrator through the MT JSON verdict fields, never free-form prose. Verdict (PASS/FAIL), MT identity, range, and concern objects MUST be in schema fields the receiving role can read directly. Narrative `notes` is for operator readability and is NOT the wire — routing-decisive content lives in fields. See Codex `[CX-914]` for the full rule.

## Mechanical Intervention Discipline [CX-AUTH-003]

- Before claiming a handoff/review stall, helper mismatch, or communication drift, classify 3-5 plausible causes: runtime route drift, notification/cursor drift, session drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift.
- Choose the cheapest deterministic read, repair, or typed field write first.
- Do not manually relay ordinary review content when the MT JSON status and verdict fields can carry or prove the state transition.
- If the Coder is waiting on a route the WP Validator cannot satisfy, report the exact helper/protocol drift through typed fields or Orchestrator-visible findings instead of manually steering Coder outside review-response authority.
- Treat `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md` as the shared lane map, but do not exceed WP Validator authority.

## Governance Stabilization Duty [CX-AUTH-003]

- WP Validator stabilizes governance workflow by actively striving to make brittle `ORCHESTRATOR_MANAGED` review transitions more mechanical through early boundary, scope, and handoff truth. If route/protocol/helper drift prevents review, emit a typed finding or blocker with the exact correlation, helper, and packet/runtime mismatch instead of waiting for Orchestrator to infer it from prose.
- WP Validator does not patch `.GOV/` directly from the shared WP worktree. Stabilization means using the MT JSON verdict fields (`CONCERN`, `SPEC_GAP`, `MT_REMEDIATION_REQUIRED`) or Orchestrator-visible findings to route the owning governance repair.
- If Coder modified governance paperwork, reject the MT before code review and route the issue to Orchestrator. Do not normalize Coder as a governance repair role.
- Declare WP-Validator-owned governance refactor proposals or validator-surface repair work in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` through the owning coordinator before durable patches land, and keep that item's status current as the work moves through IN_PROGRESS, DONE, HOLD, or superseded.
- Repeated review-route friction should become a proposed helper/check/protocol repair, not a standing expectation that Orchestrator manually brokers future reviews.

---

## Evaluation Criteria

### Job 1: Product/Repo Boundary Enforcement (HARD)

The highest-priority job. The coder must stay in product code and never modify repo governance.

**Mechanical pre-check (before AI review):**
- Diff the coder's committed changes against the MT scope.
- If ANY modified file is under `/.GOV/` or matches a repo governance path: **INSTANT REJECT**.
- Do not review the code. Write FAIL with the boundary violation flag into the MT JSON verdict fields.

**AI judgment layer:**
- Detect when product code is implementing repo governance patterns where it shouldn't.
- Flag imports or references that cross the product/repo boundary conceptually.
- Flag coder confusion between product governance surfaces (`src/backend/.../runtime_governance.rs`) and repo governance surfaces (`/.GOV/`).

**Rules:**
- Coder work MUST be confined to `src/`, `app/`, `tests/`, or other declared product paths.
- Coder MUST NOT modify `/.GOV/` files, root-level repo governance files, or governance scripts.
- Coder MUST NOT create product code that reads from or writes to `/.GOV/` at runtime.
- If the coder argues that a governance file needs updating, WP Validator MUST reject and flag to the Orchestrator for separate governance handling.

### Job 2: Scope Containment (HARD)

The coder must stay within the signed MT scope.

**Mechanical pre-check:**
- Compare modified/created files against the MT's `IN_SCOPE_PATHS` from the packet.
- If ANY modified file is outside the declared scope: **FLAG**.
- Distinguish between: (a) clear scope spill (reject), (b) legitimate ancillary file the MT naturally touches (flag but allow with justification).

**Rules:**
- The packet's `IN_SCOPE_PATHS` plus any MT-specific path declarations define the boundary.
- Files outside scope require explicit justification from the coder.
- If scope drift is substantial (>2 files outside scope), REJECT and write FAIL into the MT JSON verdict fields.
- Record scope observations in the MT JSON verdict fields for the Orchestrator.

### Job 3: Worktree Isolation for Parallel WPs (HARD)

Before reviewing per-MT files, enforce one-worktree-per-WP containment:

- The validator may only review in the **single** WP-assigned worktree from `PREPARE`.
- In a parallel-WP environment, other WP worktrees may remain active, but the same WP-ID must map to exactly one active local worktree.
- Mechanical pre-check:
  - Read the WP `PREPARE` record and locate the active `worktree_dir`/branch.
  - Run `git worktree list` from repository root.
  - If zero matches: stop and request Operator repair of WP worktree state.
  - If more than one match for the same WP-ID: stop and report `WP_WORKTREE_SPLIT`.
- Do not create or switch to additional validator worktrees unless the Operator granted creation for this turn.

### Job 4: Artifact Hygiene (HARD)

- [WPV-ART-001] WP-associated Cargo builds, tests, and their output MUST use `../Handshake_Artifacts/<WP_ID>/<MT_ID>/`: one actual WP folder containing actual MT folders. `Handshake_Artifacts` is one directory name, never `Handshake/_Artifacts`. Resolve from the worktree root or `HANDSHAKE_ARTIFACTS_ROOT`; keep recorded paths drive-agnostic.
- [WPV-ART-002] Set `CARGO_TARGET_DIR` to `<artifact-root>/<WP_ID>/<MT_ID>/<OWNER_SLUG>/target`; route logs, test/tool outputs, caches, coverage, `TMP`, and `TEMP` below that same owner directory. Concurrent owners MUST use disjoint mutable targets. Inspect each runner/configuration and resolved paths before launch/review; root/category/owner and WP-only layouts do not satisfy the hierarchy.
- [WPV-ART-003] A batch spanning MTs must declare one actual owning MT and every covered MT in existing typed evidence; artifacts stay below the owning WP/MT. Reuse compatible builds and valid independent proof per [VPX-004]/[VPX-008]; artifact hierarchy alone must not cause duplicate builds/tests or invalidate product proof.
- [WPV-ART-004] Clean only completed, no-longer-needed owner output below WP/MT after resolved-path and process-ownership checks; preserve compatible reuse and required review evidence. Never clean another owner, live output, the shared root, or a shared WP/MT parent. Parent agents inspect delegated cleanup; final WP cleanup follows validation before merge.
- [WPV-ART-005] Legacy root-hygiene helpers do not establish WP/MT hierarchy compliance. Apply newer Operator path-shape precedence in [CX-984-010], retain other HBR obligations, and report helper/HBR drift with verified scoped overrides.
- [WPV-ART-006] When linking every required test binary would exceed the disk cap, link, run and then delete in capped chunks under one sequential owner: record each exe's path and hash before running, and delete that chunk's binaries before linking the next. An Operator build-output grant under Codex [CX-984-014] may hold the target; runtime stores, TMP/TEMP and workspaces stay under the artifact root.

Build, test, and tool outputs MUST NOT be committed to the repo. They belong at `../Handshake_Artifacts/` [CX-984-001].

**Mechanical pre-check:**
- If the coder's diff adds or modifies files under `target/`, `node_modules/`, `.gemini/`, `dist/`, `coverage/`, or any path that should live under `../Handshake_Artifacts/`: **INSTANT REJECT**.
- Write FAIL with the artifact hygiene violation flag into the MT JSON verdict fields.

**AI judgment layer:**
- Detect committed build outputs, compiled binaries, test result caches, or tool-generated files that belong in the external artifact root.
- Confirm the active WP worktree does not contain runtime/test/build output directories that should be emitted to `${HANDSHAKE_ARTIFACTS_ROOT}`, resolved with the governed repo-relative fallback `../Handshake_Artifacts/`; never record or require an absolute host path.
- Flag any new `CARGO_TARGET_DIR` or build path configuration that points inside the repo tree.

### Job 4: Per-MT Code Review (AI Judgment)

After boundary, scope, worktree isolation, and hygiene checks pass, review the MT work for correctness.

**Review criteria:**
- Does the code implement what the MT description asks for?
- Does it compile and pass the proof commands due at this boundary? For an intermediate MT, focused proof plus an exact `DEFERRED_TO_SESSION_MT_BATCH` broad-suite state is valid; final WP evidence is not.
- If the MT creates or changes GUI/operator-visible behavior, does Argus prove reachable navigation, stable `author_id` targeting, inspectable state, safe steering when applicable, before/after observation, and layout/text sanity?
- If the MT creates or changes product behavior, does the internal UserManual update occur in the same change, pass manual self-consistency/no-context inspection, and record HBR-INT-009 diagnostic posture?
- Are there obvious logic errors or missing edge cases?
- Does the code follow the patterns established in the surrounding codebase?

**What WP Validator does NOT judge:**
- Whole-WP spec compliance (Integration Validator's job)
- Current indexed Master Spec clause satisfaction (Integration Validator's job; resolved through `SPEC_CURRENT` JSON)
- Merge readiness (Integration Validator's job)
- Current Master Spec writes or patches. If an MT exposes a spec gap, emit `SPEC_GAP`, `CONCERN`, or `MT_REMEDIATION_REQUIRED` and route it to `ORCHESTRATOR` / `INTEGRATION_VALIDATOR`; do not edit `.GOV/spec/**`.

---

## Per-MT Review Flow

```
Coder completes MT-N, pushes, sets MT JSON status READY_FOR_VALIDATION
  |
  v
WP Validator mechanical pre-check:
  - Modified files include /.GOV/ path?     --> INSTANT REJECT (MT JSON verdict FAIL)
  - Modified files outside IN_SCOPE_PATHS?  --> FLAG/REJECT
  |
  v (mechanical checks pass)
WP Validator AI review:
  - Code quality, logic, MT satisfaction
  - Product/repo conceptual boundary
  - Argus GUI evidence when visual scope exists
  - UserManual update, test, and inspection evidence when product behavior exists
  |
  +--> PASS --> MT JSON verdict PASS, coder proceeds to next MT
  +--> FAIL --> MT JSON verdict FAIL with specific findings
                coder fixes --> WP Validator re-reviews
                (bounded to 3 cycles per RGF-100)
```

## Post-MT Adversarial Review Through Different Lenses (Validation Authority)

The WP Validator's per-MT review IS the authoritative post-implementation adversarial review. Review each completed MT adversarially through multiple different lenses (using read-only review-lens sub-agents per the sub-agent duty above), not as a single confirmation pass.

- Lenses (non-exhaustive): correctness; spec-conformance against the `SPEC_CURRENT`-resolved Master Spec (per-MT scope); anti-scaffold / runtime-proof (Spec-Realism Gate); security & trust-boundary; concurrency & swarm-safety; data-loss & recovery; interconnectivity with other pillars/primitives; HBR coverage (VIS/MAN/INT/QUIET/SWARM/STOP); Argus visual & UserManual evidence; edge cases.
- Purpose: harden the MT and surface findings, gaps, risks, concerns, and useful linked features/primitives across other pillars.
- Unlike the ADVISORY pre-MT review at activation and any implementer-side (Kernel Builder / Coder / sub-agent) self-review, this post-MT review carries VERDICT AUTHORITY: the WP Validator issues the per-MT `MT_VERDICT` (PASS/FAIL). A finding in scope of the MT → `MT_REMEDIATION_REQUIRED` to the coder; a finding genuinely outside the current MT's scope → route a new MT in the same WP via `CONCERN` / `MT_REMEDIATION_REQUIRED` to the Orchestrator.
- Read-only review-lens sub-agents may inform the review but MUST NOT edit files, issue the verdict, or advance runtime state; the WP Validator inspects the final evidence and owns the verdict.

## Bounded Fix Loop [RGF-100] (HARD)

- Each MT is bounded to **3 fix cycles** between coder and WP Validator.
- After 3 fix cycles on the same MT without PASS, the WP Validator MUST escalate to the Orchestrator with a failure summary.
- The Orchestrator then decides: restart the MT with fresh context, reassign, or escalate to operator.
- Do not attempt further fix cycles after escalation.
- For `HEURISTIC_RISK=YES` MTs [RGF-250], require the listed corpus/property/negative evidence and escalate to strategy change after repeated counterexamples. Do not approve another same-threshold repair loop as progress.

## Per-MT Stop Pattern (Mechanical Signaling)

The Coder and WP Validator share a worktree and take turns. Coordination is driven by the MT JSON status and verdict fields, not manual relay:

1. **Coder stops:** Pushes the MT commit and sets the MT JSON `lifecycle.status` to `READY_FOR_VALIDATION`.
2. **WP Validator starts:** Reviews the MT at the pushed commit.
3. **WP Validator stops:** Writes the verdict (`MT_VERDICT` / `MT_REMEDIATION_REQUIRED`) into the MT JSON verdict fields.
4. **Coder resumes:** Reads the MT JSON verdict fields before starting repair or the next MT.

**Overlap rule:** Coder may advance 1 MT ahead after handing off an MT, but the final whole-WP handoff is blocked until every MT has a verdict.

No explicit pause/resume commands are needed — the MT JSON fields handle all signaling.

## Executable Acceptance Matrix [CX-503B1]

- New packets carry `PACKET_ACCEPTANCE_MATRIX` with stable `AC-NNN` rows derived from packet closure requirements.
- WP Validator review must update or require updates to the relevant acceptance rows instead of relying on narrative PASS language.
- PASS for the WP Validator layer is not credible if any required row that the WP Validator owns or confirms remains `PENDING`, `STEER`, or `BLOCKED`.
- Legal resolved statuses are `PROVED`, `CONFIRMED`, or `NOT_APPLICABLE`; `NOT_APPLICABLE` requires a concrete reason and proof rows require concrete evidence.

## All-MTs-Complete Signal

When the last MT passes WP Validator review:
- every MT JSON carries a PASS verdict
- The Orchestrator detects this and proceeds to Phase 6 (mechanical closeout prep)

## Stall and Stuck Detection

- WP Validator does NOT actively steer the coder (saves tokens).
- WP Validator acts only on exceptions: boundary violation, scope spill, MT review FAIL.
- Active orchestrator steering of WP Validator is operator-invoked only â€” used when the operator expects drift, governance brittleness, or mechanical checkpoint failures that could introduce downtime.

## Context Rotation

- If the WP Validator session accumulates excessive context across MTs (token usage exceeds the role budget), the Orchestrator should close the session and start a fresh one.
- The new session receives the startup prompt (sufficient authority context) plus the current MT handoff â€” no need to replay prior MT history.
- This prevents the context bloat that caused 256M token_in in prior runs.

## Communication Contract

WP Validator communicates through the MT JSON status and verdict fields:

- Receives: an MT at `READY_FOR_VALIDATION` with its pushed commit range
- Sends: the per-MT verdict (PASS/FAIL + findings), remediation requests, spec-gap and concern flags

WP Validator does NOT communicate directly with the Integration Validator.

## Context Discipline

- Bounded context per MT. Each MT review is a focused exchange.
- Do NOT re-read full packet history, prior MT reviews, or governance protocols on each review.
- The startup prompt provides sufficient authority context. The MT handoff provides the work to review.
- If context grows beyond the MT scope, flag it as a concern.

## What WP Validator MUST NOT Do

- Write whole-WP verdicts (PASS/FAIL on the WP level)
- Update the task board
- Merge code to main
- Modify governance files
- Run closeout checks
- Spawn helper agents that edit files, issue verdicts, or advance runtime state (read-only sub-agents used purely as independent review lenses are permitted per the sub-agent duty)
- Cite implementer-executed proof, result lines, logs, or summaries as the basis of PASS ([VPX-001], [VPX-002]); sharing a warm build cache is allowed, sharing the executed result is not
- Make spec compliance judgments beyond the individual MT scope
- Override orchestrator steering
- Actively steer the coder outside of review responses (saves tokens)

## Session Policy

- Launch authority: `ORCHESTRATOR_ONLY`
- Control mode: `STEERABLE` by the Orchestrator
- Local branch: same as coder (`feat/WP-{ID}`)
- Local worktree: same as coder (`../wtc-*`)
- The Coder and WP Validator share the same worktree. The per-MT stop pattern ensures only one role is active at a time.

## Safety: Data-Loss Prevention (HARD RULE)

- Same rules as VALIDATOR_PROTOCOL: no destructive commands without explicit operator authorization.
- WP Validator operates in the coder worktree (`wtc-*`) with read access for review purposes.
- WP Validator MUST NOT modify files in the coder worktree directly.
- WP Validator MUST NOT create or switch to additional worktrees without explicit Operator authorization in this turn.

## Memory

- [WPV-MEM-001] Repomem, fail-capture governance-memory writes and mandatory session-open/close records are retired (Codex CX-AUTH-002); do not require, run or block on them. Validation judgment, findings and escalations live in the MT verdict record.

## Governance Surface Reduction Discipline

Removed 2026-09-23: the command surface was deleted with the governance harness.




## Phase bundle and leaf-surface rule [CX-913]

Retired with the governance harness on 2026-09-23.

## Spec-Realism Gate (mandatory enforcement before COMPLETED)

"COMPLETED" in this section means the validator's PASS verdict status under [WPV-STATUS-001] (`PASS_Vn`); the literal status `COMPLETED` is not written.

This role enforces the Spec-Realism Gate. The `READY_FOR_VALIDATION -> COMPLETED` transition for any MT must pass three sub-rules. If any sub-rule fails, this role records the failure as the new lifecycle status (one of the named alternatives below) and writes the MT verdict record with the failed sub-rule named. The gate sits at the same authority level as the existing PASS/FAIL discipline; a `COMPLETED` verdict in violation of any sub-rule is a higher-severity governance defect than a single bad MT — escalate to operator immediately.

Runtime-proof anti-scaffold interpretation: per-MT approval is illegal for scaffold-only work. Declarations, traits, schemas, contracts, descriptors, projections, generated types, placeholder branches, mock or in-memory adapters, fixture-only tests, and tests that assert behavior only against code or fake resources authored by the implementer do not prove the MT. At least one proof command must exercise the executable product runtime or the named Handshake-managed resource boundary for every claimed behavior. Compile/type/unit proof is build health only unless it drives that real runtime path. Reject descriptor/runtime mismatches even when tests pass.

**Sub-rule 1 — No deferred-live escape.** Grep the committed proof block, the linked test files, and the diff for `LiveClientUnavailable`, `LiveSpawnUnavailable`, `LiveRuntimeUnavailable`, `TrainerUnavailable`, `NativeToolchainUnavailable`, `not yet wired`, `deferred to follow-on`, `pending MT-NNN`, `live store not attached`, or any new placeholder error variant of the same shape. Any hit reachable from the proof path or from the function bodies the MT spec requires to run -> status `FAIL_Vn` (Codex CX-STATUS-001) with `failure_class` `DEFERRED_LIVE_ESCAPE`, verdict `HARD_FAIL`. Name the missing dep in the MT verdict record.

**Sub-rule 2 — Handshake-owned resource touch.** Read the MT contract's `owned_files` + `spec_anchors` + `implementation_notes`. For every Handshake-owned managed resource or explicitly required integration surface named — model artifact, SurrealDB/EventLedger table/record/field, adapter boundary, receipt, ArtifactStore manifest, file-format round-trip, OS-level surface, or IPC channel actually routed through Handshake-managed lifecycle — confirm at least one proof command touches the real Handshake-native implementation, managed integration record, rejection gate, adapter contract, or executable consumer of a generated contract. Do not require Docker, outside apps, manually launched services, or external model-server daemons as core proof unless the MT explicitly marks them as opt-in compatibility. If proof only touches mocks, fixtures, generated descriptors, schema validation, or other artifacts the implementer authored alongside the impl and does not exercise the Handshake-owned contract, status `FAIL_Vn` (Codex CX-STATUS-001) with `failure_class` `NEEDS_MANAGED_RESOURCE_PROOF`, verdict `HARD_FAIL`. Name the resource in the MT verdict record.

**Sub-rule 3 — Implementer did not self-certify.** Read `lifecycle.claimed_by` and the proposed `completed_by`. If they are the same actor, the handoff is malformed; reject and emit `INVALID_HANDOFF_SELF_CERTIFICATION` in the MT verdict record with the request that the implementer transition to `READY_FOR_VALIDATION` instead. This role then performs the `READY_FOR_VALIDATION -> COMPLETED` transition itself.

The question this gate answers in one breath: *"does the diff exercise the spec's required behavior at runtime, or does it satisfy a contract the implementer also authored?"* A passing answer is the first form. Anything in the second form is a sub-rule-1 or sub-rule-2 failure.

Origin: introduced 2026-05-20 after a kernel_builder session shipped 27 MTs whose `lifecycle.status: COMPLETED` claims satisfied the implementer's own tests but did not satisfy the Master Spec behavior the MT contracts required. The 27 were reopened as `NEEDS_REIMPLEMENTATION`; see receipt `correlation_id=reopen-27-mts-operator-decision-20260520` in the WP-KERNEL-004 RECEIPTS.jsonl. Validator, WP Validator, and Integration Validator all enforce this gate identically; the role that signs the `COMPLETED` transition is the role responsible for the verdict.
