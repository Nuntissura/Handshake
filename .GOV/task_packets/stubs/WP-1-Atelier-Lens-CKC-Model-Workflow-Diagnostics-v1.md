# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- This stub is planning-only. It authorizes no product code changes.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics
- CREATED_AT: 2026-05-16T05:05:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- STUB_FORMAT_VERSION: 2026-04-06
- BUILD_ORDER_DOMAIN: ATELIER_LENS
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1, WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1, WP-1-Studio-Runtime-Visibility-v1, WP-1-Dev-Command-Center-MVP-v1, WP-1-Locus-Work-Tracking-System-Phase1-v1
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- PRODUCT_REFERENCE: .GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md
- SOURCE_GREENROOM_ROOT: .GOV/reference/ckc_atelier_lens_consolidation
- SOURCE_CKC_CODE: D:/Projects/LLM projects/CastKit-Codex/CKC_main
- SOURCE_CKC_SPEC: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md
- SOURCE_CKC_TASKBOARD: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md

## INTENT (DRAFT)
- What: Plan the model-facing workflow, manual, diagnostics, command catalog, DCC/Locus/Flight Recorder visibility, and no-focus automation requirements for Atelier/Lens + CKC fold-in.
- Why: CKC proved that the product needs no-context model operation, command/manual consistency, sessions/leases, command logs, visual capture, renderer state, and quiet automation. Handshake already has stronger primitives; this stub translates CKC's model operation layer into Handshake's mechanical separation of LLM and execution.
- No-code stance: This stub creates only future work scope and microtasks. It does not implement diagnostics, DCC UI, command handlers, browser automation, screenshots, tests, or product code.
- Mechanical separation: LLMs are not the executor. They operate through governed prompts, AI Jobs, Workflow Engine nodes, MCP/tool policies, MicroTask/Locus state, and product commands. Runtime execution is performed by Handshake and mechanical engines, with Flight Recorder/DCC evidence.

## SOURCE_COVERAGE_STATUS (DRAFT)
- Coverage audit: `.GOV/reference/ckc_atelier_lens_consolidation/wp-stub-coverage-audit-20260516.md`.
- This stub owns model-facing workflow, manual, command/action catalog, sessions/leases/heartbeats, command logs, DCC/Locus/Flight Recorder visibility, diagnostic bundles, structured state, visual evidence, and non-focus automation rules.
- This stub does not own data schema or PoseKit/ComfyUI domain behavior. It owns the no-context model operation layer over those surfaces.
- This stub remains planning-only and must not implement diagnostics, DCC UI, command handlers, browser automation, screenshots, tests, or product code.
- Repair status: this stub was audited before microtask generation and needed more concrete operational detail. Activation must preserve the model-operation, visual-debugging, build/release evidence, and local-LLM/AI-tagging payload below before any official MT files are generated.

## FOLDED_SOURCE_STUBS_FULL_PAYLOAD (DRAFT)
- `WP-1-Studio-Runtime-Visibility-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.md`, `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 23; value high; risk medium; blocked.
  - Preserved original intent: make Studio and Design Studio surfaces explicit runtime citizens.
  - Preserved scope: runtime mappings for Canvas/Excalidraw, Lens/Atelier collaboration panel, Studio jobs, workflow nodes, tool surfaces, DCC/operator projection, Flight Recorder linkage, Locus/task-board/WP linkage, PostgreSQL-only state.
  - Preserved acceptance: Studio-adjacent surfaces have explicit job/workflow/tool mappings; Studio activity is visible in Command Center and Flight Recorder; Locus correlates Studio work; storage posture is specified.
  - Preserved dependencies: `WP-1-Photo-Studio`, `WP-1-Atelier-Lens`, `WP-1-Dev-Command-Center-MVP`, `WP-1-Locus-Work-Tracking-System-Phase1`.
  - Preserved risk: fragmented UI panels, hidden side effects, accidental SQLite reintroduction.
- `WP-1-Product-Screenshot-Visual-Validation-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.md`, `.GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.contract.json`.
  - Lifecycle: superseded; folded into Kernel002; inherited here as validation requirement.
  - Preserved original intent: product-integrated screenshot capture for full app window, individual panels, and module-level views.
  - Preserved scope: programmatic capture, governed artifact storage, CLI/API trigger for coder/validator sessions, screenshot metadata, Tauri/webview/native integration.
  - Preserved acceptance: governed coder/validator sessions can trigger and inspect screenshots; screenshots are stored with metadata; capture works on Tauri + React; usable from cloud and local model sessions.
  - Preserved risk: panel granularity may need DOM capture, `html2canvas` or equivalent, headless/mock UI may differ, screenshot retention policy, native versus DOM uncertainty on Windows 11, and capture APIs may expose full windows more reliably than component-level panels.
  - Handling: consume through Kernel visual evidence, not by reimplementing a CKC-specific capture system.
- `WP-1-Visual-Debugging-Loop-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.md`, `.GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.contract.json`.
  - Lifecycle: superseded; folded into Kernel002; inherited here as validation requirement.
  - Preserved original intent: generate-capture-compare-fix visual debugging loop for GUI work packets.
  - Preserved scope: post-commit screenshot trigger for GUI-bearing WPs, visual comparison against baseline, visual diff artifacts, validator evidence routing, threshold configuration, Tauri app test mode.
  - Preserved acceptance: GUI WPs trigger captures; diffs are stored with WP and commit metadata; validators receive visual evidence; threshold breaches create STEER feedback with visual regression details; comparison may be against an approved design baseline or the previous screenshot when no design baseline exists.
  - Preserved risk: pixel false positives, headless rendering mismatch, threshold tuning noise, structural-vs-pixel comparison choice, baseline drift, and retention budget pressure.
  - Handling: future Atelier/Lens UI packets inherit this requirement; this stub only records model-operation and validation needs.
- `WP-1-Structured-Collaboration-Artifact-Family-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Structured-Collaboration-Artifact-Family-v1.md`, `.GOV/task_packets/stubs/WP-1-Structured-Collaboration-Artifact-Family-v1.contract.json`.
  - Lifecycle: stub backlog/supporting dependency.
  - Preserved original intent: define canonical structured collaboration artifacts for work packets, microtasks, task board projections, and role mailbox exports.
  - Preserved scope: packet, summary, index, and thread JSON/JSONL artifacts; versioned schemas; project-agnostic envelope; profile extensions; compact summaries; validation/migration from Markdown-first records.
  - Preserved acceptance: versioned structured file family; compact summaries for small local models; profile extension separation; migration guidance from Markdown authority to mirrors/sidecars.
  - Preserved dependencies: `WP-1-Locus-Phase1-Integration-Occupancy`, `WP-1-Role-Mailbox`, `WP-1-Micro-Task-Executor`.
  - Preserved risk: overfitting to repository-centric work, summary drift, premature migration complexity.
  - Handling: model-readable substrate for future Atelier/Lens command catalogs, diagnostic bundles, manual projections, and Locus/MT/DCC handoffs.
- `WP-1-Atelier-Collaboration-Panel-v1`
  - Handling here: safety precedent for proposal/apply.
  - Preserved payload consumed by this stub: selection-scoped suggestions, range-bounded patching, provenance, out-of-selection rejection, and visible audit evidence.
  - Owner boundary: Core owns text/sheet data safety; Diagnostics owns model command/proposal/apply workflow that must respect this safety law.
- `WP-1-Lens-ViewMode-v1` and `WP-1-Lens-Extraction-Tier-v1`
  - Handling here: model-facing projection controls.
  - Preserved payload consumed by this stub: no-context model operation must show ViewMode and ExtractionTier as explicit structured state, not hidden UI state.
- `WP-1-Artifact-System-Foundations-v1`
  - Handling here: diagnostic and visual evidence dependency.
  - Preserved payload consumed by this stub: screenshots, debug bundles, command receipts, validation artifacts, and manual exports use ArtifactStore manifests, hashes, retention, and no random filesystem side effects.

## CKC_GREENROOM_PAYLOAD_OWNED (DRAFT)
- `EVOL-019` built-in model manual and command map.
  - Source evidence: `automationManual.js`, `automationCommandMap.js`, `automation_manual_consistency.test.js`.
  - Requirement: produce a Handshake built-in model manual/action catalog for Atelier/Lens/CKC surfaces that a no-context model can use without chat history.
  - Minimum manual content: product purpose, concepts, startup/run commands, navigation paths, command/action IDs, required parameters, expected outputs, state probes, evidence paths, safety constraints, failure modes, recovery steps, and validation recipes for core data/intake, media, docs/stories/moodboards, relationships, pose, ComfyUI, exports, and diagnostics.
  - Minimum command catalog fields: action id, owner surface, parameter schema, capability gates, preconditions, side effects, expected receipt schema, error taxonomy, evidence refs, DCC/Locus/Flight Recorder refs, manual section ref, and whether foreground interaction is unavoidable.
  - Guardrail: manual and command catalog drift must be a validation failure.
- `EVOL-020` sessions, leases, heartbeats, command logs.
  - Source evidence: `automationControl.js`, CKC taskboard `WP-0095`, `WP-0099`.
  - Requirement: preserve multi-agent/session operation, leases, claims, heartbeats, command logs, recovery, cancellation, timeout handling, stale lease release, command start/finish/fail records, session state snapshots, and attribution.
  - Runtime adaptation: replace process-local authority with PostgreSQL/EventLedger-backed ModelSession, WorkflowRun, MicroTask, Locus, and command receipt records.
- `EVOL-021` non-focus-stealing automation and visual capture.
  - Source evidence: `automationStealth.js`, `automationCaptureToFile`, CKC taskboard `WP-0093`, `WP-0099`, `WP-0122`.
  - Requirement: model-driven GUI/projection verification must be quiet, reproducible, bounded, and linked to command/job receipts. CKC evidence includes hidden/background automation, file-based captures, renderer state inspection, command dispatch, synthetic input guarded by background-safe policies, and single-instance/no-focus safeguards.
  - Guardrail: no foreground popups, focus theft, keyboard hijack, or uncontrolled windows unless explicitly documented as unavoidable.
- `EVOL-025` hybrid CRDT/event-log policy.
  - Source evidence: `EntityRevision`, `ProductEvent`, CKC taskboard `WP-0134`, automation manual.
  - Requirement: preserve the discovered boundary: PostgreSQL/EventLedger is authority, sessions/leases coordinate work, CRDT is allowed only for safe merge shapes, and optimistic revisions protect authoritative records.
  - Owner split: Diagnostics owns model/session/parallel operation; Core owns data authority and optimistic revision behavior.
- CKC taskboard `WP-0093`, `WP-0095`, `WP-0099`, `WP-0122`, `WP-0137`.
  - Requirement: preserve automation/manual/debugger/review-batch hardening signals as source evidence; do not drop them because they are operational rather than domain-model features.
- `EVOL-027` local LLM and AI tagging visibility.
  - Source evidence: `app/backend/llm.js`, `app/main.js` `llmChat` and AI tagging job handlers, `src/ui/views/CharacterView.tsx` local model config, `src/ui/components/MediaPane.tsx` AI tag suggestion controls, CKC taskboard `WP-0099`, `WP-0137`.
  - Requirement: preserve local/remote OpenAI-compatible model configuration as a governed command surface, not hidden settings: base URL, model, optional API key, timeout, system prompt, prompt payload, job state, error messages, cancel/status controls, derived tag suggestions, and proposal/apply boundary. Handshake must route this through AI Job/Workflow Engine/tool policy and evidence records rather than process-local ad hoc calls.
- `EVOL-028` build/release/package verification visibility.
  - Source evidence: CKC taskboard build/release rows, `CKC_GOV/build_rules.md`, `CKC_main/scripts/package_win.ps1`, `scripts/package_mac.sh`, `scripts/release.ps1`, `scripts/installer_custom.nsh`, README stale spec pointer.
  - Requirement: preserve release/build/install lessons as model-visible diagnostics: read build rules before implementation WPs, run clean-repo packaging guards, keep build outputs outside repo, verify packaged app asset paths, distinguish dev/debug artifacts from SemVer release artifacts, capture release evidence, expose installer/reset/orphan adoption evidence, and flag stale README/spec pointers.

## GREENROOM_OVERLAP_ROWS_OWNED (DRAFT)
- `OVR-010` automation, model manual, visual diagnostics: fold command catalog, sessions/leases, heartbeats, command log, renderer state, captures, no OS-level input, no focus stealing, manual/command consistency tests; make durable through PostgreSQL/EventLedger.
- `OVR-012` parallel editing / event log / revisions: Diagnostics owns model operation boundary, leases, sessions, attribution, conflict visibility, and safe CRDT guidance; Core owns data authority.
- `OVR-005` sidecars/versioning/recovery: Diagnostics owns visibility of sidecar/recovery commands, deletion previews, recovery receipts, and failed validation evidence; Core/Pose own domain behavior.
- `OVR-007` ComfyUI workflow lineage: Diagnostics owns command catalog, tool policy, DCC/Flight Recorder projection, diagnostic bundle, and failed registration recovery surface; Pose owns workflow receipt domain.

## HANDSHAKE_TRANSLATION (DRAFT)
- Module boundary candidates: `atelier_automation`, `kernel_event_bridge`, DCC projection contracts, Locus/MicroTask projection contracts, ModelSession/WorkflowRun records, Flight Recorder event families, diagnostic bundle schema, and visual evidence hooks.
- CKC automation manual becomes a Handshake built-in model manual for Atelier/Lens/CKC surfaces.
- CKC automation command map becomes a typed Handshake command/action catalog linked to Tauri/Rust command facades and Workflow Engine nodes.
- CKC sessions/leases/heartbeats/command logs become PostgreSQL/EventLedger-backed ModelSession, WorkflowRun, MicroTask, Locus, and command receipt records.
- CKC visual capture/no-focus behavior becomes Handshake visual diagnostic requirements and non-intrusive model-operation policy.
- CKC renderer state commands become structured state projections; no fragile screen scraping when a structured path is practical.
- All tool calls follow Handshake governed flow: capability check, consent check where required, Flight Recorder logging, execution by mechanical/runtime surface, result capture, and diagnostic evidence.
- LLMs are proposers/operators through governed commands, jobs, and tool requests; they are not the executor and may not bypass runtime or write directly into source/artifact roots.
- Manual, command catalog, structured state, diagnostic bundles, DCC projections, and FR events must be sufficient for a no-context model to recover from failed commands without hidden chat state.
- Visual diagnostics use a layered comparison contract: structured state first when practical, screenshots when UI/projection behavior matters, pixel comparison for visual regressions, structural/DOM comparison where native pixel noise is too high, and manual validator review when thresholds are inconclusive.
- Diagnostic bundle records must include command parameters, sanitized logs, structured state, events, artifact refs, screenshots/visual diffs when relevant, environment/tool versions, failure taxonomy, recovery actions, and links to the owning Locus/MT/WP.
- Release/build diagnostics must expose packaging stage roots, artifact output roots, build ids, version/tag metadata, clean-repo guard results, asset-path guard results, installer/reset mode evidence, and stale-doc/spec-drift flags without hardcoding machine-local paths.
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Electron authority, CKC namespace authority, localhost intake authority, `.GOV` product outputs, machine-local runtime paths, and direct LLM execution are rejected.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - No-context model manual requirements.
  - Command/action catalog requirements for core data/intake and pose/ComfyUI surfaces.
  - Model operation state: sessions, leases, heartbeats, command logs, receipts.
  - Locus/MT/DCC/Flight Recorder projection requirements.
  - Visual/debug evidence requirements.
  - Non-focus-stealing automation rules.
  - Structured state and diagnostic bundle requirements.
  - Local LLM / AI tagging command visibility, job state, error taxonomy, and proposal/apply evidence.
  - Release/build/package/install/reset evidence requirements where they affect no-context model operation and verification.
  - Visual-debugging details: baseline versus previous screenshot, STEER feedback, pixel versus structural comparison, capture retention, native/webview/DOM capture uncertainty, and Windows 11 reliability notes.
  - Evidence maturity for CKC operational rows: DONE, REVIEW, BLOCKED, PLANNED, and stale README/spec pointers.
  - Validation matrix for manual/command consistency and model-safe operation.
  - Limited future workbench projection contract: tabs/windows/state surfaces as data model/projection requirements only, not GUI design.
- OUT_OF_SCOPE:
  - Full GUI shell or dockable window implementation.
  - Product command implementation.
  - Full browser automation stack.
  - Direct CKC automation API copy.
  - Direct LLM execution of filesystem/runtime operations.
  - SQLite.

## ACCEPTANCE_CRITERIA (DRAFT)
- A no-context model can understand what Atelier/Lens/CKC surfaces do, what commands exist, what inputs/outputs are expected, what is safe, what failed, and how to recover.
- Every owned CKC EVOL row `EVOL-019`, `EVOL-020`, `EVOL-021`, and `EVOL-025` is represented with source evidence, preserved behavior, runtime adaptation, and guardrails.
- Every source stub in `FOLDED_SOURCE_STUBS_FULL_PAYLOAD` has preserved original intent, scope, acceptance need, lifecycle/dependency notes, and risk/handling notes.
- Every owned overlap row has an owner boundary and required handling.
- Every model action is observable, attributable, recoverable, and tied to Flight Recorder/EventLedger/Locus/DCC where relevant.
- Manual and command catalog cannot drift silently.
- Visual diagnostics can prove important UI/projection states without stealing focus.
- Structured back-end paths are preferred over screen reading.
- Workbench/window/tab requirements are recorded as state/projection contracts, not UI design implementation.
- The model manual/action catalog contract is concrete enough for implementation planning: command IDs, parameter schemas, preconditions, capabilities, side effects, receipt schemas, errors, evidence refs, manual refs, DCC/Locus/FR refs, and recovery steps are required.
- Session/lease/heartbeat logging is concrete enough for parallel model safety: stale lease release, timeouts, command start/finish/fail receipts, cancellation, attribution, and state snapshots are required.
- Visual-debug preservation includes baseline-or-previous screenshot comparison, visual diff artifacts, threshold configuration, STEER feedback, retention policy, pixel/structural comparison choice, DOM/html2canvas/native capture uncertainty, and Windows 11 reliability risk.
- Local LLM/AI tagging behavior is represented as governed AI Job/Workflow Engine/tool-policy work with proposal/apply evidence, not process-local hidden calls.
- Build/release/install diagnostics preserve CKC lessons: clean-repo packaging guard, external/staged artifacts, relative packaged asset guard, dev versus release artifact separation, build id/version/tag metadata, installer/reset/orphan adoption evidence, and README/spec drift detection.
- CKC operational taskboard statuses remain meaningful: REVIEW rows need evidence review, BLOCKED rows remain unresolved, PLANNED rows remain deferred, and DONE rows seed parity fixtures.
- SQLite is rejected in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Electron authority, CKC namespace authority, localhost intake authority, `.GOV` product outputs, machine-local runtime paths, and direct LLM execution are explicitly rejected.

## MICROTASKS (DRAFT)

- Draft MT authority: non-executable planning only; official MT files/contracts are still not generated.
- Replacement rule: the earlier 20-item draft MT list is retired and must not be reused as source for activation.
- DRAFT_MICROTASK_SUITE_PATH: .GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1/MT_SUITE.md
- DRAFT_MICROTASK_COUNT: 66
- OFFICIAL_MICROTASKS_GENERATED: false
- DRAFT_MICROTASK_ACTIVATION_DESTINATION_PATTERN: .GOV/task_packets/<WP_ID>/MT-*.{md,json}
- Fresh no-context MT suite: `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1/MT_SUITE.md`.
- Draft MT count: 66.
- Granularity rule: Activation Manager must split any MT further if it touches unrelated files, crosses owner boundaries, or cannot be executed by a no-context local/small cloud model from that MT alone.
- Activation rule: convert these draft MTs into official `.GOV/task_packets/<WP_ID>/MT-*.json` and `.md` only after refinement, USER_SIGNATURE, and official packet creation.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Model operation becomes hidden UI automation. Mitigation: structured state projection and command catalog first.
- Risk: LLM bypasses execution boundary. Mitigation: tool policy, AI Job/Workflow Engine, and FR/DCC evidence are acceptance criteria.
- Risk: GUI design consumes scope. Mitigation: workbench is projection/state contract only.
- Risk: manual drifts from commands. Mitigation: manual/command consistency gate.
- Risk: visual validation loses the actual loop and becomes generic screenshot advice. Mitigation: activation must preserve baseline/previous comparison, visual diff artifacts, threshold config, STEER feedback, pixel/structural choice, retention, and capture API uncertainty.
- Risk: local LLM and AI tagging become hidden side effects. Mitigation: represent them as governed AI Job/Workflow Engine actions with status, errors, receipts, and proposal/apply evidence.
- Risk: release/build/package lessons are dropped as "not product feature". Mitigation: build/release/install diagnostics are explicit model-operation acceptance rows and link back to Core data-root/reset/orphan behavior.
- Risk: stale CKC README/spec pointer causes no-context models to read v00.063 instead of v00.075. Mitigation: activation must include stale-doc/spec-drift detection and updated manual pointers.
- Risk: SQLite returns through diagnostics fixtures. Mitigation: validation matrix forbids it everywhere.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm operator accepts this as one of no more than three CKC fold-in WP stubs.
- [ ] Verify product worktree anchors for DCC, Locus, AI Job, Workflow Engine, Flight Recorder, ModelSession, MCP/tool governance, visual debugging.
- [ ] Produce Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE.
- [ ] Create official task packet and MT files.
- [ ] Keep SQLite forbidden everywhere.
