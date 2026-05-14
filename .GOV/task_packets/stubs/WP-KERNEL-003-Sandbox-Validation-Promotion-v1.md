# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` and the Kernel Builder Activation Mode protocol.
- If this stub is promoted into an official packet, create a refinement first, preserve this source fold map, then generate the active packet and microtasks from the refined contract.

---

# Work Packet Stub: WP-KERNEL-003-Sandbox-Validation-Promotion-v1

## MACHINE_READABLE_GOVERNANCE_ARTIFACT_STANCE
- Machine-readable governance contracts are primary.
- Markdown is a synchronized projection and planning surface for operator/model inspection.
- This stub must produce a `.contract.json` projection before it is treated as registered governance inventory.
- This stub is not executable until it becomes an official signed packet with refinement, packet contract, microtask contracts, and activation approval.
- Source stubs folded here remain retained as historical evidence. Their original goals must be preserved in the fold map and microtask plan.

## STUB_EXECUTION_RULES
- Coder roles MUST NOT implement from this stub directly.
- Kernel Builder may use this stub for Activation Mode planning, refinement, and packet generation only.
- Product implementation begins only after USER_SIGNATURE and active packet generation.
- Validation is by Integration Validator batch review unless the final packet explicitly adds a WP Validator gate.
- Any conflict between a source stub and Kernel V1 law must be resolved by operator deliberation before activation.
- Any source stub not fully owned by Kernel003 is listed as interface or residual dependency, not silently superseded.

## STUB_METADATA
- WP_ID: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- BASE_WP_ID: WP-KERNEL-003-Sandbox-Validation-Promotion
- CREATED_AT: 2026-05-14T19:14:53Z
- STUB_FORMAT_VERSION: 2026-04-06
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-1-Product-Governance-Check-Runner, WP-1-Product-Governance-Artifact-Registry, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Artifact-System-Foundations, WP-1-Micro-Task-Executor, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
- BUILD_ORDER_BLOCKS: WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-MTE-Resource-Caps, WP-1-MTE-Blocked-Decisioning, WP-1-MTE-Summaries, WP-1-MTE-DropBack-Smart, WP-1-MCP-MEX-Evidence-Export, WP-1-Diagnostics-Debug-Bundle-Bridge
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: handshake-v2-kernel-reset-brief.md Week 3 Sandbox Execution; HSK-KERNEL-003 Sandbox Validation Promotion
- ROADMAP_ADD_COVERAGE: Kernel V1 sandbox runner, patch/artifact/log capture, deterministic validation gate, and patch promotion into authority state.
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- PLANNED_EXECUTION_OWNER_RANGE: Coder-A..Coder-Z
- SPEC_ANCHOR_CANDIDATES (Main Body and Reset Brief):
  - `.GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md` section 2.3.13.9 Kernel V1 Authority State.
  - `.GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md` section 2.3.13.10 Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge.
  - `.GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md` section 5.2 Sandboxing & Security.
  - `.GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md` section 10.3.11.5 large or hostile attachments sandboxing requirements.
  - `.GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md` section 11.2 Sandbox Policy vs Hard Isolation.
  - `.GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md` section 11.7.1 Terminal Engine / PTY / Sandbox.
  - `.GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md` section 11.7.4.5 Untrusted input security rule.
  - `.GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md` section 11.8 Engine Sandbox (Code Execution).
  - `.GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md` sections 6.7 Visual Debugging, 6.10 Deterministic Code Checking, 6.12 Minimal Technical Contracts, and Week 3 Sandbox Execution.

## INTENT (DRAFT)
- What: Build the Kernel V1 sandbox, validation, and promotion tranche that turns model or operator proposals into isolated sandbox runs, deterministic evidence, validation reports, and explicit promotion decisions before any authority state changes.
- Why: Kernel V1 must let models propose and test code without mutating real project authority directly. Sandbox outputs are not final until validation and promotion events accept them into product-owned EventLedger authority.
- Product goal: Handshake (Product) should be able to run a candidate patch or artifact job in a bounded sandbox, capture logs/artifacts/diffs/screenshots/check results, classify failure or blocked states, and promote or reject the candidate through durable authority events.
- Repo Governance goal: Repo Governance should collapse the related sandbox/validation/evidence/MTE stubs into this one Kernel003 planning stub while preserving their original intent, risks, and technical goals as traceable source evidence.
- Operator goal: A future no-context model, including a small local model, must be able to execute any single microtask without guessing the target files, contracts, acceptance proof, or validation path.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define KernelSandboxRun, SandboxPolicy, SandboxWorkspace, SandboxAdapter, SandboxArtifactBundle, SandboxPatchProposal, ValidationRun, ValidationFinding, PromotionDecision, and PromotionReceipt contracts.
  - Add a sandbox runner boundary that supports a minimum policy-scoped local adapter and a future hard-isolation adapter behind the same trait/API.
  - Enforce default-deny filesystem, network, process execution, environment variable, and secret behavior for sandbox jobs.
  - Materialize candidate inputs from KB001 EventLedger/ArtifactStore and KB002 WriteBox/CRDT proposal surfaces without mutating authority.
  - Capture stdout, stderr, exit codes, duration, resource usage, command manifests, environment manifests, diff bundles, generated artifacts, validation reports, screenshots, DOM or accessibility evidence where applicable, and redaction reports.
  - Integrate with the already validated Product Governance Check Runner as a deterministic check execution dependency instead of inventing a second check runner.
  - Integrate MTE resource caps, blocked decisioning, summaries, and smart drop-back semantics into Kernel003 sandbox execution where they govern bounded model work.
  - Add promotion logic that accepts only validated candidates and appends EventLedger promotion events with replayable receipt IDs.
  - Add DCC/read-model projections sufficient to inspect sandbox runs, blocked reasons, validation results, patch candidates, and promotion outcomes.
  - Add tests proving direct write denial, sandbox path guardrails, network/process allowlist denial, resource caps, blocked classification, validation result persistence, promotion accept/reject, replay after restart, and no SQLite authority for Kernel V1.
  - Preserve existing source-stub goals by explicit fold, residual, or dependency classification.
- OUT_OF_SCOPE:
  - Kernel004 local model runtime and memory implementation.
  - Full CRDT workspace implementation beyond the handoff already owned by KB002.
  - Replacing all legacy SQLite surfaces unrelated to Kernel V1 authority.
  - Shipping a production-grade VM/container isolation stack as the only supported adapter if local host support is absent.
  - Domain-specific retrieval, Spec Router, AI-ready index, cloud-consent, or calendar/mail evidence exporters except for the generic evidence interfaces Kernel003 must expose.
  - Full DCC UI polish beyond inspectable runtime projections and minimal control affordances needed for validation and promotion.
  - Broad repo-governance workflow repair unrelated to KB003 restartability and traceability.

## SOURCE_STUB_FOLD_MAP

### Fully Folded Source Stubs

These stubs are fully absorbed into Kernel003. Their old intent must remain visible inside this stub and the future official packet.

| Source Stub | Source Status Before Fold | Kernel003 Carry-Forward |
|-------------|---------------------------|-------------------------|
| WP-1-MTE-Resource-Caps-v1 | STUB | Sandbox jobs and model execution loops must enforce token, storage, time, output, and artifact caps with explicit evidence. |
| WP-1-MTE-Blocked-Decisioning-v1 | STUB | Sandbox and validation blocked states must classify retryable, gate-required, operator-required, environment-missing, policy-denied, and terminal failures. |
| WP-1-MTE-Summaries-v1 | STUB | Every sandboxed microtask run must produce per-MT summaries and aggregate run summaries linked to artifacts and ledger events. |
| WP-1-MTE-DropBack-Smart-v1 | STUB | Retry/escalation/drop-back logic must be explicit, bounded, and recorded for sandboxed model attempts. |
| WP-1-MCP-MEX-Evidence-Export-v1 | STUB | Tool and mechanical engine evidence, including sandbox engine evidence, must be redacted, portable, replayable, and bound to capability decisions. |
| WP-1-Diagnostics-Debug-Bundle-Bridge-v1 | STUB | Diagnostics, validation findings, problems, logs, screenshots, and grouped failures must materialize as canonical debug bundle inputs. |
| WP-1-Packet-Candidate-Range-Truth-v1 | Task Board only; no stub file found | Candidate patch ranges and target file ranges must be explicit and verified before promotion. |
| WP-1-Validator-Command-Surface-Preflight-v1 | Task Board only; no stub file found | Validation commands must be discoverable, preflighted, policy checked, and evidence-linked before execution. |
| WP-1-Worktree-Path-Root-Guardrails-v1 | Task Board only; no stub file found | Sandbox and promotion paths must be rooted, normalized, symlink-safe, and unable to escape declared worktrees/artifact roots. |
| WP-1-Bootstrap-Skeleton-Receipt-Projection-v1 | Task Board only; no stub file found | First skeleton runs must produce receipts/projections good enough for restart and no-context inspection. |
| WP-1-Canonical-Closeout-Bundle-v1 | Task Board only; no stub file found | Promotion closeout must emit one canonical evidence bundle that validators and future models can inspect. |
| WP-1-Receipt-Driven-Lane-Wake-Settlement-v1 | Task Board only; no stub file found | Lane wake/settlement decisions must be driven by durable receipts, not chat state or transient terminal logs. |

### Residual or Reuse Inputs, Not Fully Superseded Here

These items remain important but are not fully owned by Kernel003. Kernel003 must expose interfaces that later packets can reuse without silently absorbing their domain-specific goals.

| Source | Classification | Kernel003 Handling |
|--------|----------------|-------------------|
| WP-1-Product-Governance-Check-Runner-v1 | Validated dependency | Reuse as deterministic check execution layer; do not reimplement. |
| WP-1-Product-Governance-Artifact-Registry-v1 | Validated dependency | Reuse artifact identity and registry behavior for sandbox evidence. |
| WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 | Validated dependency | Reuse split verdict, validation posture, and closeout evidence patterns. |
| WP-1-Product-Screenshot-Visual-Validation-v1 | Already folded into KB002 | Kernel003 consumes screenshot capture and stores screenshot evidence in validation reports. |
| WP-1-Visual-Debugging-Loop-v1 | Already folded into KB002 | Kernel003 uses visual debugging evidence as validation inputs; KB002 owns capture loop hooks. |
| WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Already folded into KB002 | Kernel003 may use residual container/test matrix requirements for sandbox adapter preflight if KB002 leaves them open. |
| WP-1-Cloud-Consent-Evidence-Portability-v1 | Interface dependency | Kernel003 records capability/approval evidence for network/cloud sandbox grants; full cloud portability remains separate. |
| WP-1-Consent-Audit-Projection-v1 | Interface dependency | Kernel003 emits capability/approval evidence IDs; full audit projection remains separate. |
| WP-1-Retrieval-Trace-Bundle-Export-v1 | Interface dependency | Kernel003 can reference trace bundle shapes but does not implement retrieval export. |
| WP-1-ACE-Persist-QueryPlan-Trace-v1 | Interface dependency | Kernel003 preserves trace refs if validation uses ACE context; full ACE persistence remains separate. |
| WP-1-Spec-Router-Evidence-Portability-v1 | Interface dependency | Kernel003 can link packet/spec evidence IDs; full Spec Router portability remains separate. |
| WP-1-AIReady-Index-Evidence-Export-v1 | Interface dependency | Kernel003 preserves evidence bundle compatibility; AI-ready index export remains separate. |

## CONFLICT_REGISTER (DRAFT)
- Potential conflict: Source stubs that assume raw shell execution conflict with Kernel V1 ToolGate, SandboxPolicy, and ValidationRunner requirements. Mitigation: preserve command intent as allowlisted validation descriptors with fail-closed policy denial for unregistered commands.
- Potential conflict: Source stubs that imply direct repo or authority mutation conflict with KB002 WriteBox and Kernel V1 promotion law. Mitigation: preserve mutation intent as PatchProposal or WriteBoxPromotionCandidate only; authority changes require PromotionGate acceptance events.
- Potential conflict: A hard Docker/Podman/container-only requirement could conflict with local Windows availability and disk-agnostic portability. Mitigation: require a SandboxAdapter interface with policy-scoped local proof first and hard-isolation adapters behind capability/preflight gates.
- Potential conflict: Legacy SQLite tests or caches conflict with Kernel V1 no-SQLite-authority law if reused for sandbox authority. Mitigation: Kernel003 tests may inspect legacy SQLite code only as unrelated surface; Kernel003 authority, promotion, replay, and validation records must use Postgres/EventLedger.
- Potential conflict: Domain-specific evidence export stubs could bloat Kernel003 into retrieval, memory, or cloud-consent work. Mitigation: keep generic evidence interfaces in Kernel003 and preserve domain implementations as later packets unless the operator explicitly folds them.

## PRODUCT_CODE_REALITY_CHECK (DRAFT)
- Current product code evidence from `../handshake_main` shows validated governance check/artifact surfaces exist, but no obvious `kernel`, `EventLedger`, `SandboxRunner`, `ValidationRunner`, or `PromotionGate` product modules are present yet.
- Current storage code includes PostgreSQL and SQLite legacy/test surfaces. Kernel003 must inherit the Kernel V1 prohibition against SQLite authority.
- Current product code has `governance_check_runner.rs`, `governance_artifact_registry.rs`, `runtime_governance.rs`, `flight_recorder`, and ACE promotion/artifact validators that should be inspected before implementation.
- Kernel003 should expect KB001 and KB002 to create or harden the initial kernel authority modules. If those are not landed by activation time, Kernel003 must begin with compatibility checks and block on missing APIs rather than inventing parallel authority.

## RESEARCH_BASIS_REQUIRED_BEFORE_ACTIVATION
- Confirm current field practice for local sandbox execution across Windows, WSL, Docker/Podman, Deno, Pyodide/WASM, and process-level policy sandboxes.
- Confirm how mature systems structure validation evidence: test command descriptors, check runner result schemas, artifact bundles, redaction manifests, trace IDs, and replay hashes.
- Confirm current security constraints for symlink-safe workspace materialization, path canonicalization, network denial, env var redaction, secret handling, process allowlists, and resource accounting.
- Inspect existing code in `../handshake_main` before final packet generation so the microtasks target actual modules and do not create duplicate abstractions.
- Record rejected options in the refinement, especially container-only, raw shell passthrough, SQLite-backed kernel authority, and UI-only validation.

## ACCEPTANCE_CRITERIA (DRAFT)
- AC-001: A no-context model can read the official KB003 packet and implement any single microtask without conversation history.
- AC-002: All fully folded source-stub goals are preserved in the official packet, microtasks, and traceability registry.
- AC-003: No source stub is removed. Folded source stubs remain retained as evidence with a `FOLDED_INTO` pointer.
- AC-004: Kernel003 defines durable product contracts for sandbox runs, policies, artifacts, validation reports, promotion decisions, and promotion receipts.
- AC-005: Sandbox jobs cannot write authority state directly and cannot modify project files outside declared sandbox/output roots.
- AC-006: Sandbox jobs deny filesystem escape, network access, process execution, device access, and secret access unless explicitly granted by policy and recorded as provenance.
- AC-007: Sandbox outputs include canonical artifact bundles with stable hashes, manifest files, stdout/stderr/log references, environment metadata, and redaction state.
- AC-008: Deterministic validation runs before model/LLM review and stores typed PASS/FAIL/BLOCKED/ADVISORY/UNSUPPORTED results.
- AC-009: PromotionGate accepts only validated candidates and appends durable EventLedger events linked to validation and approval evidence.
- AC-010: PromotionGate rejection paths produce replayable receipts for stale candidate, duplicate idempotency key, validation failure, policy denial, missing approval, Postgres failure, and projection rebuild failure.
- AC-011: MTE token/storage/time/output caps deterministically gate sandbox runs and emit evidence.
- AC-012: Blocked decisioning distinguishes retryable, operator-required, environment-required, policy-denied, and terminal failures.
- AC-013: Every sandboxed microtask attempt writes per-MT and aggregate summaries linked to run artifacts.
- AC-014: DCC or equivalent projection can show sandbox run status, blocked reasons, validation reports, promotion decisions, and evidence bundle links.
- AC-015: Visual validation evidence can be attached to a validation report when a GUI/browser check is part of the candidate.
- AC-016: Tests prove direct-write denial, path escape denial, network denial, process allowlist denial, resource cap behavior, validation persistence, promotion accept/reject, and replay after restart.
- AC-017: Kernel003 authority records use Postgres/EventLedger and do not introduce SQLite authority, SQLite fallback, SQLite fixtures, or SQLite compatibility paths.
- AC-018: Validation and promotion evidence remains reconstructable after backend restart without provider chat history, terminal scrollback, or hidden session context.
- AC-019: All generated artifacts, logs, and external tool outputs remain under configured artifact roots and are disk-agnostic.
- AC-020: Activation includes an Integration Validator handoff summary and does not claim PASS/FAIL internally.

## RISKS / UNKNOWNs (DRAFT)
- Risk: KB001/KB002 APIs may not be landed when KB003 activates. Mitigation: first microtasks must run compatibility inventory and block cleanly if EventLedger, WriteBox, or ActionCatalog APIs are missing.
- Risk: Container or VM isolation may not be available on the operator's host. Mitigation: require adapter preflight and policy-scoped proof adapter, while keeping hard-isolation adapter optional until host support is verified.
- Risk: Policy-only sandboxing is not strong security. Mitigation: label policy mode as best-effort, deny sensitive capabilities by default, and keep hard-isolation adapter as a separate implementation surface.
- Risk: Validation commands can mutate the repo if run raw. Mitigation: command descriptors must declare side effects, allowed roots, write outputs, and execution policy before ToolGate dispatch.
- Risk: Path canonicalization errors can escape sandbox roots through symlinks, junctions, or relative paths. Mitigation: add platform-specific path tests, symlink denial, root normalization, and target-root assertions.
- Risk: Evidence bundles can leak sensitive payloads. Mitigation: default-deny export policy, redaction manifest, artifact-level exportability flags, and validation tests for secret/env redaction.
- Risk: Resource caps can be inaccurate. Mitigation: define accounting semantics before implementation and test overage using deterministic fake counters.
- Risk: Promotion can accept stale or duplicate candidates. Mitigation: require idempotency keys, base snapshot/state vector refs, validation report refs, and replay tests.
- Risk: Visual validation can become flaky. Mitigation: store screenshots/DOM/console evidence separately from hard gate verdicts and use deterministic checks where practical.
- Risk: This packet can become too broad. Mitigation: keep domain evidence exporters as interface dependencies unless the operator explicitly folds them fully.

## MICRO_TASK_PLAN (DRAFT, NO-CONTEXT READY)

### MT-001 - Activation Source Inventory
- Objective: Re-scan `.GOV/task_packets/stubs`, `TASK_BOARD.md`, `BUILD_ORDER.md`, and `WP_TRACEABILITY_REGISTRY.md` for every KB003-related stub before packet activation.
- Write scope: governance-only activation notes and future packet refinement.
- Implementation detail: classify each candidate as FULL_FOLD, RESIDUAL_INTERFACE, VALIDATED_DEPENDENCY, ALREADY_FOLDED_TO_KB002, or NOT_KB003.
- Acceptance: the refined packet contains a source fold table at least as detailed as this stub.
- Verification: compare refined fold table against Task Board and traceability registry lines.

### MT-002 - Conflict Deliberation Record
- Objective: Convert `CONFLICT_REGISTER` into a signed operator decision record before any official KB003 packet is generated.
- Write scope: KB003 refinement and packet metadata.
- Implementation detail: list raw shell execution, direct authority mutation, container-only assumptions, SQLite authority, and domain-evidence bloat as explicit decisions.
- Acceptance: unresolved conflicts are either approved, rejected, or parked; none are silently removed.
- Verification: reviewer can map each conflict to packet constraints or non-goals.

### MT-003 - Current Product API Inventory
- Objective: Inspect `../handshake_main` for landed KB001/KB002 modules and existing check/artifact/governance APIs.
- Write scope: refinement evidence only.
- Implementation detail: record module/file paths for EventLedger, SessionBroker, ArtifactStore, ToolGate, WriteBox, ActionCatalog, check runner, artifact registry, Flight Recorder, and DCC projections.
- Acceptance: packet microtasks target real files or explicitly declare missing upstream blockers.
- Verification: `rg` evidence and file-path list included in refinement.

### MT-004 - Research Basis Update
- Objective: Perform current research needed for sandbox adapter and validation evidence architecture before implementation starts.
- Write scope: refinement research section.
- Implementation detail: compare Docker/Podman, WSL, Deno, Pyodide/WASM, process policy mode, and future VM/container hard isolation for local Windows feasibility.
- Acceptance: selected adapter sequence and rejected options are documented.
- Verification: sources checked, patterns found, reuse opportunities, selected approach, risks, and validation plan are present.

### MT-005 - Official Packet Contract Generation
- Objective: Promote this stub into an official KB003 packet only after operator approval.
- Write scope: `.GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/`.
- Implementation detail: generate packet, contract, microtask contracts, role constraints, dependencies, validation handoff, and status surfaces.
- Acceptance: packet is signed/ready but not self-validated by Kernel Builder.
- Verification: packet contract parses and Task Board/Build Order resolve KB003.

### MT-006 - Product Module Placement Decision
- Objective: Decide where sandbox/validation/promotion modules live in product code based on existing architecture.
- Write scope: no code unless official packet is active; then `src/backend/handshake_core/src/kernel/**` or existing module paths.
- Implementation detail: avoid duplicate parallel authority modules if KB001 created kernel primitives.
- Acceptance: module topology is documented and approved before scaffolding.
- Verification: no new top-level project directories; paths are repo-root-relative.

### MT-007 - Kernel003 Schema Namespace
- Objective: Define stable schema IDs and version names for all Kernel003 entities.
- Write scope: product schema/module files and tests after activation.
- Implementation detail: include `KernelSandboxRunV1`, `SandboxPolicyV1`, `SandboxWorkspaceV1`, `SandboxArtifactBundleV1`, `ValidationRunV1`, `PromotionDecisionV1`, and `PromotionReceiptV1`.
- Acceptance: schema names are stable, versioned, and referenced by EventLedger events.
- Verification: unit tests fail on missing schema version fields.

### MT-008 - EventLedger Event Type Plan
- Objective: Add Kernel003 event type names and payload expectations.
- Write scope: EventLedger event enum/registry files after KB001 is available.
- Implementation detail: define requested, started, blocked, completed, failed, validation_started, validation_recorded, promotion_requested, promotion_accepted, promotion_rejected, and replay_verified events.
- Acceptance: every event carries run ID, actor, session, task, schema version, timestamp, and artifact refs.
- Verification: serialization tests and replay projection tests.

### MT-009 - Artifact Type Plan
- Objective: Define artifact classes for sandbox and validation evidence.
- Write scope: artifact registry schema/module files after activation.
- Implementation detail: include logs, stdout/stderr, command manifest, environment manifest, patch diff, file tree manifest, screenshots, DOM snapshots, validation reports, redaction reports, and promotion receipts.
- Acceptance: each artifact class has content type, hash policy, exportability default, and retention/default root.
- Verification: artifact registry tests and bundle hash tests.

### MT-010 - DCC Projection Contract
- Objective: Define the minimum operator projection for sandbox and promotion state.
- Write scope: backend projection/API contracts and minimal UI hooks after activation.
- Implementation detail: expose run list, active/blocked/completed state, validation verdicts, evidence links, promotion buttons/state, and denial reasons.
- Acceptance: a no-context model can inspect state without reading terminal logs.
- Verification: projection unit test plus frontend/API smoke test if UI is touched.

### MT-011 - Postgres Migration for Sandbox Runs
- Objective: Add Postgres authority tables for sandbox runs if not already provided by KB001.
- Write scope: Postgres migration files and storage layer after activation.
- Implementation detail: store run ID, task/session IDs, policy ref, workspace ref, status, timestamps, adapter ID, and event refs.
- Acceptance: records persist and replay after backend restart.
- Verification: Postgres test using `POSTGRES_TEST_URL`; skip only with explicit blocked evidence.

### MT-012 - Postgres Migration for Sandbox Policies
- Objective: Persist sandbox policies as authority-linked records.
- Write scope: migrations/storage/tests after activation.
- Implementation detail: model filesystem scopes, network grants, process allowlist, env allowlist, secret grants, device grants, timeout/cap values, and approval refs.
- Acceptance: policy changes are versioned and traceable.
- Verification: schema and storage tests reject malformed/default-allow policies.

### MT-013 - Postgres Migration for Validation Runs
- Objective: Persist validation run metadata and result summaries.
- Write scope: migrations/storage/tests after activation.
- Implementation detail: store validation run ID, sandbox run ID, check descriptor IDs, verdict, finding refs, artifact refs, started/ended timestamps, and blocker classification.
- Acceptance: validation results are reconstructable without file-system-only state.
- Verification: storage roundtrip and replay tests.

### MT-014 - Postgres Migration for Promotion Receipts
- Objective: Persist promotion decisions and receipts.
- Write scope: migrations/storage/tests after activation.
- Implementation detail: store candidate ref, validation ref, approval ref, idempotency key, target authority class, accepted/rejected state, error code, and EventLedger IDs.
- Acceptance: duplicate idempotency keys are rejected or idempotently resolved.
- Verification: duplicate/stale/reject/accept storage tests.

### MT-015 - No SQLite Authority Tripwire
- Objective: Prevent Kernel003 authority from using SQLite in production or tests.
- Write scope: tests and product storage guards after activation.
- Implementation detail: add tests scanning Kernel003 modules and migrations for SQLite authority/fallback usage.
- Acceptance: Kernel003 authority fails closed without Postgres/EventLedger authority.
- Verification: targeted test fails if SQLite fixture or fallback appears in Kernel003 authority path.

### MT-016 - Replay Projection Storage Query
- Objective: Implement or specify a query that reconstructs a sandbox-validation-promotion run from durable rows/events.
- Write scope: backend projection module after activation.
- Implementation detail: join EventLedger event IDs to run, artifacts, validation report, and promotion receipt.
- Acceptance: replay does not read provider chat, terminal scrollback, or transient logs.
- Verification: restart/replay test.

### MT-017 - Legacy Compatibility Blocker Check
- Objective: Detect whether existing product surfaces require migration before Kernel003 can run.
- Write scope: compatibility test/handoff note after activation.
- Implementation detail: inspect legacy governance check runner, artifact registry, and runtime governance modules for API compatibility.
- Acceptance: missing prerequisite APIs produce BLOCKED with evidence, not parallel implementations.
- Verification: startup/preflight test and Integration Validator review.

### MT-018 - SandboxAdapter Trait
- Objective: Define a sandbox adapter boundary independent of Docker, WSL, Deno, or WASM.
- Write scope: backend trait/interface module after activation.
- Implementation detail: adapter exposes preflight, prepare_workspace, run, cancel, collect_artifacts, cleanup, and health methods.
- Acceptance: at least one adapter can be implemented without changing caller code.
- Verification: trait mock tests.

### MT-019 - PolicyScopedLocal Adapter
- Objective: Implement the minimum local proof adapter with strict policy checks and clear best-effort labeling.
- Write scope: backend sandbox adapter module after activation.
- Implementation detail: run only allowlisted commands in a temporary sandbox root, never directly in project root.
- Acceptance: policy mode is explicitly not hard isolation and denies sensitive capabilities by default.
- Verification: deny network/process/path escape tests.

### MT-020 - HardIsolation Adapter Stub
- Objective: Add a non-executing adapter slot for Docker/Podman/WSL/WASM/VM hard isolation.
- Write scope: backend adapter registry and preflight code after activation.
- Implementation detail: adapter reports unsupported until configured; it must not silently fall back to unsafe behavior.
- Acceptance: hard isolation absence is a typed BLOCKED/UNSUPPORTED result, not success.
- Verification: unsupported adapter test.

### MT-021 - SandboxPolicy Default Deny
- Objective: Implement default-deny sandbox policy construction.
- Write scope: policy module and tests after activation.
- Implementation detail: default policy grants only declared input reads and artifact-output writes; no network, secrets, devices, or unlisted processes.
- Acceptance: omitted policy fields deny access.
- Verification: unit tests for each capability category.

### MT-022 - Filesystem Scope Guard
- Objective: Enforce read/write roots and prevent path escape.
- Write scope: sandbox filesystem guard module after activation.
- Implementation detail: canonicalize paths, reject `..`, symlink/junction escapes, absolute paths outside roots, and writes outside artifact/sandbox output roots.
- Acceptance: all path escape attempts return typed denial evidence.
- Verification: platform-aware unit tests with temp dirs and symlinks where supported.

### MT-023 - Network Capability Gate
- Objective: Deny network unless policy grants it.
- Write scope: adapter/policy enforcement after activation.
- Implementation detail: for policy mode, block or fail commands requiring network by descriptor; for hard isolation, pass adapter-level network off/on setting.
- Acceptance: network grants require approval/provenance refs.
- Verification: descriptor-based network denial test.

### MT-024 - Process Execution Allowlist
- Objective: Permit only registered commands/checks in sandbox jobs.
- Write scope: command descriptor and adapter modules after activation.
- Implementation detail: commands must declare executable, args schema, side effects, timeout, allowed roots, and artifact outputs.
- Acceptance: raw shell strings without descriptors are rejected.
- Verification: raw command denial and allowlisted command success tests.

### MT-025 - Environment and Secret Redaction
- Objective: Prevent accidental leakage of env vars and secrets into sandbox logs/artifacts.
- Write scope: env assembly/redaction module after activation.
- Implementation detail: pass a minimal env map, redact known sensitive values, and record redaction manifest.
- Acceptance: secret-looking values are not emitted in stored logs or reports.
- Verification: redaction tests with fake secret values.

### MT-026 - Resource Cap Policy
- Objective: Fold MTE resource caps into sandbox execution policy.
- Write scope: policy/accounting module after activation.
- Implementation detail: include token, time, stdout/stderr bytes, artifact bytes, file count, process count, retry count, and validation command count caps.
- Acceptance: overage halts or gates deterministically with evidence.
- Verification: deterministic fake-counter tests.

### MT-027 - Cancellation and Timeout
- Objective: Add run cancellation and timeout handling.
- Write scope: adapter and run-state modules after activation.
- Implementation detail: record cancellation requested/completed events and collect partial logs safely.
- Acceptance: cancelled runs cannot promote and have typed terminal state.
- Verification: timeout and cancellation tests.

### MT-028 - Sandbox Workspace Materializer
- Objective: Materialize candidate inputs into an isolated workspace root.
- Write scope: sandbox workspace module after activation.
- Implementation detail: copy or project only declared artifacts/files, preserve hashes, and create manifest of every input.
- Acceptance: no undeclared project files appear in sandbox input manifest.
- Verification: manifest tests.

### MT-029 - Sandbox Cleanup and Retention
- Objective: Clean temporary execution roots while preserving declared artifacts.
- Write scope: cleanup/retention module after activation.
- Implementation detail: retain manifests/logs/reports under artifact root; delete temp workspace when policy says ephemeral.
- Acceptance: cleanup never deletes project files or authority rows.
- Verification: temp-root cleanup test.

### MT-030 - Sandbox Adapter Health Projection
- Objective: Expose adapter health/preflight state for DCC and no-context diagnostics.
- Write scope: backend projection/API after activation.
- Implementation detail: show adapter ID, mode, available/unavailable state, blocked reason, version, and configured capability ceiling.
- Acceptance: unsupported isolation is visible before run.
- Verification: API/projection test.

### MT-031 - PatchProposal Contract
- Objective: Define the candidate patch envelope.
- Write scope: schema/module files after activation.
- Implementation detail: include base commit/ref, base snapshot/state vector refs, target file ranges, diff ref, write-box refs, author actor, and intent summary.
- Acceptance: proposals without base refs or target ranges cannot enter validation.
- Verification: schema validation tests.

### MT-032 - Candidate Range Truth
- Objective: Implement board-only candidate-range intent as concrete validation.
- Write scope: patch proposal validator after activation.
- Implementation detail: verify changed paths and ranges match declared targets and ownership boundaries.
- Acceptance: unexpected file edits are rejected before promotion.
- Verification: diff with out-of-range path test.

### MT-033 - Diff Capture
- Objective: Capture candidate diffs as stable artifacts.
- Write scope: patch/diff module and artifact registry after activation.
- Implementation detail: normalize line endings, preserve file modes where relevant, hash content, and attach metadata.
- Acceptance: identical candidate produces identical diff artifact hash.
- Verification: stable hash test.

### MT-034 - Artifact Bundle Manifest
- Objective: Create canonical sandbox artifact bundle format.
- Write scope: artifact bundle module after activation.
- Implementation detail: stable path order, normalized metadata, SHA-256 hashes, output refs, log refs, validation refs, and redaction report refs.
- Acceptance: bundle hash is deterministic for same inputs.
- Verification: deterministic bundle test.

### MT-035 - Stdout/Stderr Log Capture
- Objective: Store bounded command logs as artifacts.
- Write scope: adapter/log capture module after activation.
- Implementation detail: enforce byte caps, truncation markers, redaction, and links to command descriptors.
- Acceptance: logs never live only in terminal output.
- Verification: truncation/redaction tests.

### MT-036 - Environment Manifest
- Objective: Record non-sensitive runtime environment identifiers.
- Write scope: manifest module after activation.
- Implementation detail: include adapter ID, OS family, shell/runtime versions, command descriptors, config hash, and granted capabilities.
- Acceptance: manifest is enough to explain run context without exposing secrets.
- Verification: manifest snapshot test with redacted env.

### MT-037 - Command Manifest
- Objective: Record exactly what commands/checks were run.
- Write scope: command descriptor/report module after activation.
- Implementation detail: include descriptor ID, resolved executable, args hash/projection, cwd inside sandbox root, timeout, and side-effect class.
- Acceptance: validators can replay or reason about command intent.
- Verification: command manifest test.

### MT-038 - Visual Evidence Attachment
- Objective: Attach KB002 screenshot/visual-debug artifacts to validation reports when available.
- Write scope: validation artifact linkage after activation.
- Implementation detail: consume existing screenshot/DOM/console evidence APIs rather than reimplementing capture.
- Acceptance: GUI validation reports can reference screenshots and DOM/log evidence.
- Verification: mocked visual artifact link test.

### MT-039 - Redaction Report
- Objective: Add a redaction report to every exportable bundle.
- Write scope: redaction/bundle module after activation.
- Implementation detail: list redacted fields/classes, denied artifacts, exportability flags, and policy reason.
- Acceptance: default export is redacted and denied artifacts are listed.
- Verification: redaction report tests.

### MT-040 - Artifact Store Integration
- Objective: Store all sandbox artifacts through the validated artifact system.
- Write scope: artifact persistence integration after activation.
- Implementation detail: use existing registry/handle contracts; do not write ad hoc file-only evidence.
- Acceptance: every artifact has stable handle and hash.
- Verification: artifact handle roundtrip test.

### MT-041 - ValidationDescriptor Contract
- Objective: Define validation command/check descriptors.
- Write scope: validation schema/module after activation.
- Implementation detail: descriptor declares tool/check ID, command, args schema, side effects, required capabilities, timeout, blocking posture, and output parser.
- Acceptance: validation runner rejects undeclared raw commands.
- Verification: descriptor schema tests.

### MT-042 - Check Runner Adapter
- Objective: Reuse Product Governance Check Runner as the validation execution backend where applicable.
- Write scope: validation/check-runner integration after activation.
- Implementation detail: map check runner result states into Kernel003 ValidationRun verdicts.
- Acceptance: no duplicate check runner is created.
- Verification: adapter tests with fake check results.

### MT-043 - Validation Result Schema
- Objective: Define validation result states and finding shapes.
- Write scope: validation schema/module after activation.
- Implementation detail: support PASS, FAIL, BLOCKED, ADVISORY_ONLY, UNSUPPORTED, SKIPPED_WITH_REASON, and ERROR.
- Acceptance: every non-PASS has a typed reason and evidence refs.
- Verification: result serialization tests.

### MT-044 - Validation Preflight
- Objective: Implement board-only validator-command preflight intent.
- Write scope: validation preflight module after activation.
- Implementation detail: check descriptor existence, tool availability, capability grants, policy mode, target paths, and resource budget before running checks.
- Acceptance: missing tools produce BLOCKED/UNSUPPORTED, not silent skip.
- Verification: unavailable tool test.

### MT-045 - Deterministic Check Batch
- Objective: Run deterministic validation checks before any model/LLM review.
- Write scope: validation runner module after activation.
- Implementation detail: include format/typecheck/test/lint/security/UI checks as descriptors, not hardcoded shell strings.
- Acceptance: blocking check failure prevents promotion.
- Verification: failing descriptor blocks promotion test.

### MT-046 - Validation Evidence Bundle
- Objective: Store validation outputs as a canonical evidence bundle.
- Write scope: validation/artifact modules after activation.
- Implementation detail: bundle includes check manifests, result JSON, logs, findings, screenshots, redaction report, and hashes.
- Acceptance: validation report can be inspected offline.
- Verification: bundle structure test.

### MT-047 - Finding Normalization
- Objective: Normalize check output into typed findings.
- Write scope: validation parser module after activation.
- Implementation detail: include severity, file path, line/column when safe, check ID, message, remediation hint, and evidence ref.
- Acceptance: raw logs are not the only finding source.
- Verification: parser unit tests.

### MT-048 - Advisory vs Blocking Rules
- Objective: Make blocking posture explicit per validation descriptor.
- Write scope: validation policy module after activation.
- Implementation detail: descriptor declares blocking, advisory, or environment-gated posture; ValidationRun calculates final verdict.
- Acceptance: advisory failure is visible but does not promote-block unless configured.
- Verification: mixed advisory/blocking batch tests.

### MT-049 - Validation Replay
- Objective: Re-run a validation descriptor set against the same candidate when inputs are still available.
- Write scope: validation runner/replay module after activation.
- Implementation detail: use artifact refs and command descriptors, not hidden shell history.
- Acceptance: replay records new run ID linked to original.
- Verification: replay linkage test.

### MT-050 - Validation Report Projection
- Objective: Expose validation report summaries to DCC/projection layer.
- Write scope: backend projection/API after activation.
- Implementation detail: show verdict, blocked reason, findings, artifact refs, and promotion eligibility.
- Acceptance: operator/model can inspect validation without reading raw files first.
- Verification: projection test.

### MT-051 - PromotionCandidate Contract
- Objective: Define promotion candidate shape from patch proposal or write box.
- Write scope: promotion schema/module after activation.
- Implementation detail: include candidate ID, source type, validation run refs, target authority class, base refs, idempotency key, approval posture, and artifact bundle ref.
- Acceptance: missing validation refs block promotion.
- Verification: schema tests.

### MT-052 - Promotion Eligibility Check
- Objective: Implement promotion preconditions.
- Write scope: promotion gate module after activation.
- Implementation detail: require validated candidate, fresh base refs, actor eligibility, approved capabilities, no duplicate idempotency key, and target authority match.
- Acceptance: ineligible candidate produces typed rejection receipt.
- Verification: table-driven eligibility tests.

### MT-053 - Promotion Accept Path
- Objective: Append accepted promotion events to EventLedger and update target authority through approved path.
- Write scope: promotion gate/storage integration after activation.
- Implementation detail: use KB001/KB002 authority APIs; do not write around EventLedger/WriteBox.
- Acceptance: accepted promotion is replayable from durable events.
- Verification: accept/replay test.

### MT-054 - Promotion Reject Path
- Objective: Record rejected promotion attempts with durable reasons.
- Write scope: promotion gate module after activation.
- Implementation detail: support stale base, duplicate key, validation failure, policy denial, missing approval, missing artifact, Postgres failure, projection failure, and unknown error.
- Acceptance: reject path creates receipt and does not mutate authority.
- Verification: rejection tests.

### MT-055 - Idempotency Key Enforcement
- Objective: Prevent duplicate promotion effects.
- Write scope: promotion storage and tests after activation.
- Implementation detail: idempotency key is unique per target authority/candidate context.
- Acceptance: duplicate accept returns prior receipt or typed duplicate rejection without second mutation.
- Verification: duplicate promotion test.

### MT-056 - Approval Ref Binding
- Objective: Bind operator/validator approval evidence to promotion decisions.
- Write scope: approval/promotion integration after activation.
- Implementation detail: approval ref must identify actor, role, timestamp, candidate, validation report, and scope.
- Acceptance: promotion cannot accept without required approval posture.
- Verification: missing/wrong approval tests.

### MT-057 - Authority Mutation Boundary
- Objective: Ensure sandbox and validation code cannot mutate authority except through PromotionGate.
- Write scope: tests and guard modules after activation.
- Implementation detail: direct writes from sandbox adapter, validation runner, or check runner to authority APIs are forbidden or denied.
- Acceptance: direct mutation attempt produces denial evidence.
- Verification: direct write denial test.

### MT-058 - Promotion Closeout Bundle
- Objective: Implement board-only canonical closeout bundle intent.
- Write scope: promotion closeout/artifact module after activation.
- Implementation detail: closeout bundle includes candidate, validation report, decision, receipts, artifacts, final target refs, and replay pointers.
- Acceptance: Integration Validator can review one bundle for the promotion.
- Verification: closeout bundle structure test.

### MT-059 - MTE Run Cap Integration
- Objective: Wire MTE resource caps into sandboxed microtask execution.
- Write scope: MTE/sandbox integration after activation.
- Implementation detail: cap policy travels with sandbox run and validation runs; caps produce typed BLOCKED or FAILED states.
- Acceptance: cap overage halts bounded run and writes evidence.
- Verification: overage test.

### MT-060 - Blocked Reason Taxonomy
- Objective: Implement MTE blocked decisioning for sandbox/validation.
- Write scope: blocked-state module after activation.
- Implementation detail: classify retryable_tool_missing, environment_missing, policy_denied, approval_required, validation_failed, resource_exhausted, conflict_detected, and terminal_error.
- Acceptance: each blocked reason has retry/escalate/gate semantics.
- Verification: taxonomy unit tests.

### MT-061 - Retry Budget
- Objective: Bound retry behavior for recoverable sandbox/validation failures.
- Write scope: retry policy module after activation.
- Implementation detail: per-run retry count, backoff, same-input idempotency, and evidence links.
- Acceptance: retry exhaustion becomes typed BLOCKED/FAILED.
- Verification: retry budget test.

### MT-062 - Smart DropBack
- Objective: Implement smart drop-back semantics after successful retry/escalation.
- Write scope: MTE policy module after activation.
- Implementation detail: use last escalation, policy, failure class, and validation outcome to decide drop-back.
- Acceptance: smart/always/never modes have test coverage.
- Verification: table-driven drop-back tests.

### MT-063 - Per-MT Summary Artifact
- Objective: Persist per-microtask summaries for sandboxed model attempts.
- Write scope: summary artifact module after activation.
- Implementation detail: summary includes objective, input refs, changed refs, commands, verdicts, blocked reasons, artifacts, and next action.
- Acceptance: every completed/blocked MT attempt has summary ref.
- Verification: summary ref test.

### MT-064 - Aggregate Run Summary
- Objective: Persist an aggregate summary over all attempts in a sandboxed packet run.
- Write scope: summary module after activation.
- Implementation detail: aggregate links per-MT summaries, validation reports, promotion decisions, failures, and unresolved blockers.
- Acceptance: no-context reviewer can inspect aggregate before raw artifacts.
- Verification: aggregate summary test.

### MT-065 - Lane Wake Receipt
- Objective: Implement board-only receipt-driven lane wake/settlement intent.
- Write scope: scheduler/projection integration after activation if scheduler APIs exist.
- Implementation detail: lane wake decisions must cite receipts from validation/promotion state, not chat messages.
- Acceptance: wake/settlement event includes receipt refs and reason.
- Verification: mocked scheduler projection test.

### MT-066 - Bootstrap Skeleton Receipt Projection
- Objective: Ensure first skeleton sandbox run creates enough receipts for restartable inspection.
- Write scope: skeleton run/test fixture after activation.
- Implementation detail: run a harmless candidate through sandbox, validation, rejection or no-op promotion, and projection.
- Acceptance: all receipts visible after restart.
- Verification: end-to-end skeleton test.

### MT-067 - DCC Sandbox Run List
- Objective: Add projection/API for sandbox run list.
- Write scope: DCC backend/API and optional frontend after activation.
- Implementation detail: list run ID, task/session, status, adapter, policy mode, started/ended, blocked reason, and evidence links.
- Acceptance: operator can find current and past sandbox runs.
- Verification: API/projection test.

### MT-068 - DCC Run Detail
- Objective: Add projection/API for sandbox run detail.
- Write scope: DCC backend/API and optional frontend after activation.
- Implementation detail: show command manifests, artifacts, validation reports, promotion decisions, summaries, and replay links.
- Acceptance: detail view has no hidden dependency on terminal scrollback.
- Verification: projection snapshot test.

### MT-069 - DCC Promotion Control State
- Objective: Expose promotion eligibility and required approval state.
- Write scope: DCC backend/API and optional frontend after activation.
- Implementation detail: show eligible/ineligible, missing requirements, approval refs, and action IDs.
- Acceptance: UI/API cannot promote when eligibility is false.
- Verification: eligibility API test.

### MT-070 - Debug Bundle Bridge
- Objective: Fold diagnostics debug bundle bridge into Kernel003 evidence output.
- Write scope: debug bundle integration after activation.
- Implementation detail: bundle validation findings, grouped problems, screenshots, logs, and sandbox manifests into canonical debug bundle inputs.
- Acceptance: diagnostics evidence is bounded and portable.
- Verification: debug bundle bridge test.

### MT-071 - MCP/MEX Evidence Export Bridge
- Objective: Fold MCP/MEX evidence export into sandbox tool/mechanical engine evidence.
- Write scope: evidence export integration after activation.
- Implementation detail: capture redacted JSON-RPC/tool events, gate outcomes, capability decisions, and mechanical engine results as evidence refs.
- Acceptance: MCP/MEX evidence does not use ad hoc bundle schema.
- Verification: mocked tool call evidence test.

### MT-072 - Capability Audit Evidence Link
- Objective: Link sandbox grants and denials to capability/consent evidence without absorbing full cloud-consent packets.
- Write scope: capability evidence integration after activation.
- Implementation detail: record capability snapshot ID, policy decision ID, approval/denial reason, and scope for network/secrets/devices/process grants.
- Acceptance: every sensitive grant has provenance.
- Verification: sensitive grant test.

### MT-073 - Visual Validation Gate Descriptor
- Objective: Define how screenshots, DOM snapshots, console logs, and accessibility checks enter validation.
- Write scope: validation descriptors and visual evidence linkage after activation.
- Implementation detail: reuse KB002 screenshot/visual debugging surfaces; add descriptor result mapping to ValidationRun.
- Acceptance: visual evidence can block or advise according to descriptor posture.
- Verification: mocked visual descriptor test.

### MT-074 - Console and Network Evidence
- Objective: Capture browser/app console and network evidence where validation runs GUI checks.
- Write scope: validation evidence bridge after activation.
- Implementation detail: store console errors, failed requests, status codes, and trace refs as artifacts.
- Acceptance: GUI validation failures are diagnosable.
- Verification: mocked Playwright/trace artifact test.

### MT-075 - Security Denial Test Matrix
- Objective: Add negative tests for sandbox security boundaries.
- Write scope: tests after activation.
- Implementation detail: test filesystem escape, symlink escape, network denied, secret denied, process denied, path outside root, and undeclared artifact output.
- Acceptance: every denial writes typed evidence.
- Verification: negative test suite.

### MT-076 - Promotion Failure Test Matrix
- Objective: Add tests for each promotion failure scenario.
- Write scope: tests after activation.
- Implementation detail: duplicate idempotency, stale base, invalid target authority, validation failure, missing approval, Postgres write error, artifact missing, and projection rebuild failure.
- Acceptance: no failure mutates authority.
- Verification: failure suite.

### MT-077 - Restart Replay Test
- Objective: Prove sandbox-validation-promotion state survives restart.
- Write scope: integration test after activation.
- Implementation detail: create run, collect evidence, validate, promote/reject, restart backend/storage connection, reconstruct projection.
- Acceptance: replay is complete from durable product state.
- Verification: restart/replay integration test.

### MT-078 - Disk-Agnostic Path Test
- Objective: Prove sandbox and artifact paths remain repo-root/config relative.
- Write scope: tests/config after activation.
- Implementation detail: use temp roots and environment/config indirection; reject hardcoded drive/user paths.
- Acceptance: moving workspace root does not break path resolution.
- Verification: temp root relocation test.

### MT-079 - Documentation and Model Manual Update
- Objective: Update the product-local model-facing manual for Kernel003 operation.
- Write scope: existing product documentation/manual path after activation.
- Implementation detail: document purpose, workflows, startup commands, inputs/outputs, navigation, safety constraints, failure modes, recovery, and debug surfaces.
- Acceptance: no-context model can run/inspect sandbox workflow from durable docs.
- Verification: doc review against built-in manual requirements.

### MT-080 - Integration Validator Handoff
- Objective: Prepare the final validation handoff bundle without self-validating.
- Write scope: packet receipts, summaries, and validation handoff after implementation.
- Implementation detail: include changed files, tests run, blocked tests, evidence bundle refs, residual risks, and explicit Integration Validator review request.
- Acceptance: Kernel Builder/Coder does not claim PASS/FAIL; validator has sufficient evidence.
- Verification: handoff checklist complete.

## DEPENDENCIES / BLOCKERS (DRAFT)
- KB001 must provide or be close enough to provide EventLedger, SessionBroker, ArtifactStore, ValidationRunner/PromotionGate event hooks, and TraceProjection authority surfaces.
- KB002 must provide or be close enough to provide WriteBox/ActionCatalog/CRDT pre-promotion bridge hooks and direct-edit denial behavior.
- Validated Product Governance Check Runner and Artifact Registry should be reused rather than reimplemented.
- PostgreSQL test availability may gate full integration validation; absent `POSTGRES_TEST_URL` must be recorded as BLOCKED evidence, not hidden.
- Hard isolation availability is host-dependent and must be adapter-preflighted.

## HIGH_ROI_ADDITIONS (DRAFT)
- Add a `sandbox doctor`/preflight command because host isolation support is uncertain and this gives operators/models an immediate blocker diagnosis.
- Add canonical bundle hashing now because every later validation, memory, export, and self-improvement loop will need stable evidence IDs.
- Add a tiny no-op skeleton sandbox run because it proves the full state path before a real model writes code.
- Add path/symlink denial tests early because one bad path abstraction can invalidate the whole sandbox safety story.
- Add a closeout bundle format now because Integration Validator, DCC, memory, and future replay can reuse it.
- Add visual evidence descriptors now because GUI validation is a normal validation path, not a later polish item.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm source fold set with the operator if any conflict remains unresolved.
- [ ] Complete current research basis and record sources, rejected options, selected approach, risks, mitigations, and validation plan.
- [ ] Inspect product code after KB001 and KB002 land or record missing API blockers.
- [ ] Generate official refinement and packet from this stub.
- [ ] Generate detailed microtask files/contracts from this microtask plan.
- [ ] Update Task Board, Build Order, and Traceability Registry so folded source stubs do not drift.
- [ ] Obtain USER_SIGNATURE before implementation.
- [ ] Declare product worktree and branch for implementation.
- [ ] Open repomem with `--wp WP-KERNEL-003-Sandbox-Validation-Promotion-v1` before product mutation.
- [ ] Preserve Integration Validator batch review as the validation topology unless the packet explicitly adds a WP Validator gate.
