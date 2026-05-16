---
file_id: mt-suite-wp-1-atelier-lens-ckc-model-workflow-diagnostics-v1
file_kind: draft_microtask_suite
updated_at: 2026-05-16
status: draft_non_executable
wp_id: WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1
official_microtasks_generated: false
replaces_inline_twenty_task_draft: true
---

<topic id="draft-microtask-suite" status="draft" version="v1" wp="WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1" summary="Fresh no-context draft microtasks for Model Workflow Diagnostics" updated_at="2026-05-16">

# Draft Microtask Suite: Model Workflow And Diagnostics

These microtasks replace the prior 20-item draft list. They are not executable until the stub is activated into an official signed work packet with refinement, packet contract, and generated `MT-*.json` / `MT-*.md` contracts.

Every MT below must leave enough evidence for a no-context model to inspect current state, failure state, and recovery path without relying on chat history.

## Shared Execution Rules For Every MT

- Authority: planning draft only; no coder or validator may start from this file before activation.
- Model boundary: LLMs propose or invoke governed jobs/actions; runtime/tool adapters execute and record evidence.
- Quiet operation: no foreground popups, focus theft, keyboard hijack, uncontrolled windows, or unexpected capture of input unless explicitly documented as unavoidable.
- Structured-first rule: use structured state/action surfaces before fragile screen reading when practical; use visual evidence when UI/projection behavior matters.
- Evidence rule: every action has a command receipt, EventLedger/Flight Recorder refs, error taxonomy, and recovery path where applicable.
- Rejection rules: no SQLite, Electron authority, CKC namespace authority, localhost intake authority as truth, `.GOV` product outputs, machine-local path authority, or direct LLM file/runtime execution.

### MT-001 - Diagnostics Source Evidence And Status Matrix
- Objective: Build the source/evidence matrix for automation manual, command map, sessions, leases, visual capture, local LLM, AI tagging, build/release diagnostics, and review hardening.
- Inputs: repaired Diagnostics stub; CKC taskboard `WP-0093`, `WP-0095`, `WP-0099`, `WP-0122`, `WP-0137`; CKC automation files; CKC build rules and package scripts.
- Work slice: create/update activation evidence matrix only.
- Outputs: source matrix with requirement, source path, CKC status, owner boundary, implementation/deferred decision, and fixture/check need.
- Acceptance: local LLM/AI tagging, visual-debug loop, package/release/install evidence, and README/spec drift are represented.
- Verification: compare rows against repaired stub EVOL rows and CKC taskboard anchors.
- Depends on: none.

### MT-002 - Diagnostics Product Anchor Verification
- Objective: Verify Handshake product anchors for model sessions, command catalog, Workflow Engine, DCC, Locus, Flight Recorder, visual capture, and build diagnostics.
- Inputs: activated packet, product worktree, `.GOV/spec/SPEC_CURRENT.md`, Product Reference.
- Work slice: inspect files and produce verified path map or `BLOCKED_MISSING_ANCHOR`.
- Outputs: anchor map with command/state/event/diagnostic/build surfaces.
- Acceptance: no later MT targets an unverified path without a blocker.
- Verification: `rg --files` evidence and anchor-map artifact.
- Depends on: MT-001.

### MT-003 - Non-Intrusive Operation Policy Gate
- Objective: Enforce quiet/background model operation before automation behavior expands.
- Inputs: CKC `automationStealth.js`, `assertBackgroundSafe`, global non-intrusive rules.
- Work slice: implement policy checks for focus stealing, dialogs, show/focus, global shortcuts, uncontrolled windows, and synthetic input.
- Outputs: policy guard and tests.
- Acceptance: foreground behavior is rejected unless an action declares unavoidable foreground interaction and records reason.
- Verification: tests for denied dialog/show/focus/global shortcut and allowed documented foreground exception.
- Depends on: MT-002.

### MT-004 - No-Context Model Manual Structure
- Objective: Define the built-in model manual skeleton for Atelier/Lens CKC surfaces.
- Inputs: CKC `automationManual.js`, Core/Pose manual source rows, global model manual requirements.
- Work slice: create manual schema/sections for purpose, concepts, startup, navigation, workflows, inputs, outputs, safety, failure modes, recovery, evidence paths.
- Outputs: manual source model and projection skeleton.
- Acceptance: a model with no chat history can identify how to operate core, pose, workflow, diagnostics, export, and recovery surfaces.
- Verification: schema/projection test for required sections and no empty sections.
- Depends on: MT-002, Core manual source MT, Pose manual source MT.

### MT-005 - Command Action Catalog Schema
- Objective: Implement typed action catalog entries.
- Inputs: CKC `automationCommandMap.js`, repaired stub minimum command catalog fields.
- Work slice: add action id, owner surface, params schema, capabilities, preconditions, side effects, result receipt schema, errors, evidence refs, DCC/Locus/FR refs, manual refs, foreground flag.
- Outputs: catalog schema/types and validation tests.
- Acceptance: invalid command definitions fail validation; every command has manual and receipt refs.
- Verification: tests for valid command, missing params schema, missing manual ref, missing evidence ref, foreground reason required.
- Depends on: MT-004.

### MT-006 - Manual/Command Consistency Check
- Objective: Prevent manual and action catalog drift.
- Inputs: CKC `automation_manual_consistency.test.js`.
- Work slice: add a validation check that every manual command exists in catalog and every catalog command appears in manual.
- Outputs: consistency check and fixtures.
- Acceptance: deleting one side fails the check with exact missing command id.
- Verification: positive fixture and two negative fixtures.
- Depends on: MT-004, MT-005.

### MT-007 - State Probe Catalog
- Objective: Define structured state probes before visual/screen inspection.
- Inputs: CKC `getRendererUIState`, Product Reference structured state guidance.
- Work slice: add state probe schema for current character, media selection, intake batch, collection, docs, moodboard, pose context, ComfyUI job, model session, errors.
- Outputs: state probe catalog and validation.
- Acceptance: every major Core/Pose surface has at least one structured state probe.
- Verification: schema test and coverage check against Core/Pose manual source rows.
- Depends on: MT-005.

### MT-008 - Action Receipt Schema
- Objective: Define command/action receipts for every model-visible operation.
- Inputs: CKC command logs, EventLedger/Flight Recorder requirements.
- Work slice: add receipt schema with action id, params hash, actor/session, start/end, status, result refs, error refs, artifact refs, state before/after refs.
- Outputs: receipt schema and tests.
- Acceptance: every action can record success, failure, cancel, denied, blocked, and skipped.
- Verification: tests for each terminal status and required fields.
- Depends on: MT-005.

### MT-009 - Error Taxonomy
- Objective: Define structured errors and recovery hints.
- Inputs: CKC automation errors, Core/Pose failure modes.
- Work slice: add error classes for validation, capability denied, missing state, stale lease, tool timeout, artifact missing, parse failure, visual mismatch, package guard failure, stale docs.
- Outputs: error taxonomy and mapping tests.
- Acceptance: errors include retry/repair/defer/escalate recommendation and evidence refs.
- Verification: tests for each error class and recovery action.
- Depends on: MT-008.

### MT-010 - Diagnostic Bundle Schema
- Objective: Define bundles a no-context model can use to isolate failures.
- Inputs: repaired stub diagnostic bundle requirements.
- Work slice: add bundle manifest schema with command params, sanitized logs, structured state, events, artifacts, screenshots/diffs, environment/tool versions, error taxonomy, recovery actions, Locus/MT/WP refs.
- Outputs: diagnostic bundle schema and builder skeleton.
- Acceptance: failed command, failed tool job, failed ComfyUI registration, missing media, bad sheet parse, and visual mismatch can all produce bundles.
- Verification: schema tests for all six bundle kinds.
- Depends on: MT-007, MT-008, MT-009.

### MT-011 - ModelSession Record
- Objective: Implement durable model sessions.
- Inputs: CKC `automationControl.js` session creation and global parallel model rules.
- Work slice: add ModelSession fields for id, agent name, purpose, metadata, created/updated, state, actor, and close reason.
- Outputs: create/update/close/list session behavior.
- Acceptance: sessions survive restart and are attributable.
- Verification: tests for create, update state, close, list active, restart load.
- Depends on: MT-002, MT-008.

### MT-012 - Lease And Claim Contract
- Objective: Implement leases/claims for parallel model coordination.
- Inputs: CKC leases, global parallel model rules.
- Work slice: add lease name, session id, ttl, acquired/released timestamps, stale state, and conflict errors.
- Outputs: acquire/release/renew/list lease behavior.
- Acceptance: one active lease per lease name unless policy allows sharing; stale leases can be released deterministically.
- Verification: tests for acquire, conflict, renew, release, stale timeout.
- Depends on: MT-011.

### MT-013 - Heartbeat And Timeout Handling
- Objective: Preserve heartbeats and stale session detection.
- Inputs: CKC heartbeat behavior.
- Work slice: implement heartbeat record/update and timeout classification.
- Outputs: heartbeat API and stale session projection.
- Acceptance: missing heartbeat marks session stale but does not delete evidence.
- Verification: tests for heartbeat update, stale threshold, recovery heartbeat, stale lease linkage.
- Depends on: MT-011, MT-012.

### MT-014 - Command Log
- Objective: Persist command logs tied to sessions and receipts.
- Inputs: CKC command logs and automation command dispatch.
- Work slice: implement append/query command log records with action id, session id, params hash, status, receipt ref, error ref.
- Outputs: command log API/query.
- Acceptance: logs are append-only and queryable by session, action id, status, and time range.
- Verification: tests for append success/fail, query filters, no secret leakage.
- Depends on: MT-008, MT-011.

### MT-015 - Cancellation And Recovery Actions
- Objective: Define cancellation and recovery commands for jobs/actions.
- Inputs: CKC cancel/status patterns, Workflow Engine.
- Work slice: add cancel request/receipt and recovery action schema for command/job/session.
- Outputs: cancel/recover action contracts and tests with fake jobs.
- Acceptance: cancellation is visible and does not erase failed/partial evidence.
- Verification: tests for cancel pending, cancel running, recover failed, recover denied.
- Depends on: MT-008, MT-014.

### MT-016 - Locus And MicroTask Projection
- Objective: Project model work state into Locus/MT surfaces.
- Inputs: Locus/MicroTask Product Reference, repaired stub.
- Work slice: add projection rows for active MT, owner, status, blocker, receipts, next action, and evidence.
- Outputs: projection contract and query.
- Acceptance: operator/model can see active/blocked/ready state without reading chat.
- Verification: tests for active MT, blocked MT, completed MT, missing evidence.
- Depends on: MT-011 through MT-015.

### MT-017 - DCC Session Lease And Command Panels
- Objective: Define DCC visibility for sessions, leases, command logs, and command recovery only.
- Inputs: Dev Command Center requirements.
- Work slice: add DCC projection contract for active sessions, held leases, recent commands, command status, command errors, cancellation, and recovery actions.
- Outputs: DCC session/command projection tests; GUI implementation can be later.
- Acceptance: no hidden session or command state is required to debug model operation.
- Verification: tests for active session, held lease, command success, command failure, cancel/recover action, and empty state.
- Depends on: MT-016.

### MT-018 - Flight Recorder Session And Command Events
- Objective: Define Flight Recorder events for session lifecycle and command lifecycle only.
- Inputs: CKC automation/manual evidence and Product Reference.
- Work slice: add event families for session start/close, heartbeat, lease acquire/release/expire, command start, command finish, command fail, and command cancel.
- Outputs: session/command event schema catalog and tests.
- Acceptance: session and command lifecycle actions have replay-grade event evidence.
- Verification: tests for session/command event payload fields, owner attribution, timestamps, and no secret leakage.
- Depends on: MT-008, MT-011.

### MT-019 - Core Structured State Probes
- Objective: Implement state probes for Core/Data surfaces.
- Inputs: Core manual source rows and state probe catalog.
- Work slice: expose current character, media selection, intake batch, pending inbox, collection, docs/story/moodboard, relation graph, search/export states.
- Outputs: state probe implementations or stubs with blockers for missing Core anchors.
- Acceptance: state can be read without screen scraping.
- Verification: tests for each state probe using fake/minimal data.
- Depends on: MT-007 and relevant Core APIs.

### MT-020 - Pose And Workflow Structured State Probes
- Objective: Implement state probes for Pose/ComfyUI surfaces.
- Inputs: Pose manual source rows and state probe catalog.
- Work slice: expose pose context, active rig, source strip count, sidecar strip count, identity profile selection, workflow job status, replay availability, last error.
- Outputs: state probe implementations or blockers for missing Pose anchors.
- Acceptance: models can inspect pose/workflow state before requesting screenshots.
- Verification: tests for blank context, single image, collection context, active workflow job, failed workflow job.
- Depends on: MT-007 and relevant Pose APIs.

### MT-021 - Visual Capture Capability Contract
- Objective: Define programmatic screenshot capture capability for full app, panel, and module targets.
- Inputs: `WP-1-Product-Screenshot-Visual-Validation-v1`.
- Work slice: implement or skeleton capture request schema with target kind, selector/panel id, viewport/window metadata, trigger, session, and artifact refs.
- Outputs: capture schema and fake capture tests.
- Acceptance: capture request is bounded, quiet, and stores metadata.
- Verification: tests for full app, panel, module, invalid target, foreground denial.
- Depends on: MT-003, MT-010.

### MT-022 - Screenshot Artifact Storage
- Objective: Store screenshots as governed artifacts with metadata and retention.
- Inputs: ArtifactStore foundations and screenshot stub.
- Work slice: implement screenshot artifact manifest with dimensions, timestamp, target, trigger, session, WP/MT refs, retention class.
- Outputs: screenshot artifact writer/manifest tests.
- Acceptance: screenshots are not random files and can be referenced by validation bundles.
- Verification: tests for manifest hash, retention, no `.GOV` product output, metadata fields.
- Depends on: MT-021.

### MT-023 - Native Webview DOM Capture Decision Record
- Objective: Preserve capture API uncertainty instead of hiding it.
- Inputs: screenshot stub risk: Tauri webview/native/html2canvas/Windows 11 uncertainty.
- Work slice: create an ADR or config decision record comparing native window capture, webview capture, DOM/html2canvas capture, and headless/test rendering.
- Outputs: decision record with selected first implementation path and fallback.
- Acceptance: tradeoffs and platform risks are explicit; no implementation assumes panel capture is solved if it is not.
- Verification: ADR review and validation of selected path preflight.
- Depends on: MT-021.

### MT-024 - Visual Diff Baseline Contract
- Objective: Preserve baseline-or-previous screenshot comparison behavior.
- Inputs: `WP-1-Visual-Debugging-Loop-v1`, Kimi K2.5 research notes.
- Work slice: implement visual diff request schema with baseline kind, previous screenshot fallback, threshold, target, and metadata.
- Outputs: diff schema and fake diff tests.
- Acceptance: every diff states whether it used approved baseline or previous screenshot.
- Verification: tests for approved baseline, previous fallback, missing both, threshold breach.
- Depends on: MT-022.

### MT-025 - STEER Feedback From Visual Mismatch
- Objective: Convert threshold breaches into actionable STEER feedback.
- Inputs: Visual Debugging Loop acceptance.
- Work slice: define feedback record with diff artifact, affected target, threshold, changed regions, suspected issue, and coder handoff text.
- Outputs: STEER feedback schema and tests.
- Acceptance: a visual mismatch does not become silent failure or generic prose.
- Verification: tests for diff under threshold, over threshold creates STEER, inconclusive creates manual-review request.
- Depends on: MT-024.

### MT-026 - Pixel Versus Structural Comparison
- Objective: Preserve pixel/structural comparison choice.
- Inputs: visual-debug risks and DOM/html2canvas uncertainty.
- Work slice: implement comparison strategy enum and result fields for pixel, structural/DOM, and manual validator review.
- Outputs: comparison strategy config and tests.
- Acceptance: noisy pixel diff can route to structural/manual without losing evidence.
- Verification: tests for pixel result, structural result, false-positive suppression note, manual review fallback.
- Depends on: MT-024.

### MT-027 - Visual Evidence Retention And Redaction
- Objective: Bound visual artifacts and protect sensitive captures.
- Inputs: screenshot retention risk and ArtifactStore policy.
- Work slice: add retention class, exportable flag, redaction state, and cleanup policy fields to visual artifacts.
- Outputs: retention/redaction manifest fields and tests.
- Acceptance: captures can be retained, pruned, or marked non-exportable without deleting linked evidence unexpectedly.
- Verification: tests for retention class, non-exportable flag, cleanup dry run.
- Depends on: MT-022.

### MT-028 - GUI Verification Checklist
- Objective: Preserve UI verification requirements for future GUI-bearing Atelier/Lens work.
- Inputs: global GUI verification rules and visual-debug loop.
- Work slice: add checklist rows for readability, discoverability, coherent navigation, no overlap, responsive layout, important state visible, stable IDs.
- Outputs: checklist schema and validation hook.
- Acceptance: GUI MTs can require visual evidence without inventing criteria.
- Verification: checklist coverage test and sample PASS/FAIL fixture.
- Depends on: MT-021 through MT-027.

### MT-029 - Local LLM Configuration Surface
- Objective: Preserve CKC local/remote OpenAI-compatible model config as governed state.
- Inputs: CKC `llm.baseUrl`, `llm.model`, `llm.apiKey`, `llm.systemPrompt`, timeout.
- Work slice: implement configuration record with secret handling, validation, and diagnostic status.
- Outputs: config get/set/test behavior and redacted projection.
- Acceptance: API keys are not logged; base URL/model/timeout are visible to diagnostics.
- Verification: tests for set config, redacted key, invalid URL, missing model.
- Depends on: MT-003, MT-008.

### MT-030 - LLM Chat Governed Job
- Objective: Preserve `llmChat` through AI Job/Workflow Engine rather than process-local hidden calls.
- Inputs: CKC `llmChat`, `openAiChatCompletions`.
- Work slice: implement job request with messages, model config ref, timeout, status, result, error, token/tool refs if available.
- Outputs: LLM chat job contract and fake provider tests.
- Acceptance: chat calls produce receipts and can fail with structured errors.
- Verification: tests for success, missing config, timeout, provider error, cancel.
- Depends on: MT-029.

### MT-031 - AI Tagging Governed Job
- Objective: Preserve image AI tag suggestion jobs as governed, cancellable, observable jobs.
- Inputs: CKC AI tagging job handlers and MediaPane suggestion controls.
- Work slice: implement AI tag job with image ids, prompt/model config, status, progress, suggestions, errors, cancel.
- Outputs: AI tag job contract and fake provider tests.
- Acceptance: suggestions are derived/proposed and not applied automatically.
- Verification: tests for start, progress, suggestions, cancel, malformed output.
- Depends on: MT-029, Core AI tag proposal MT.

### MT-032 - Proposal Apply Boundary
- Objective: Ensure model suggestions become product changes only through governed apply.
- Inputs: Atelier Collaboration Panel precedent and CKC AI tag suggestions.
- Work slice: implement proposal state machine: draft, preview, validate, apply, reject, rollback where applicable.
- Outputs: proposal/apply contract and tests.
- Acceptance: LLM/model output cannot mutate authoritative data without an apply receipt.
- Verification: tests for proposal only, validate fail, apply success, reject, rollback record.
- Depends on: MT-008, MT-031.

### MT-033 - Build Rules Read Evidence
- Objective: Preserve CKC build-rule gate as model-visible evidence.
- Inputs: CKC `build_rules.md`, taskboard WP-0120/WP-0121.
- Work slice: add build-rule-read checklist/evidence row required before implementation WPs touching build/test/release surfaces.
- Outputs: evidence schema/check.
- Acceptance: build/release MTs fail preflight if build rules were not read and acknowledged.
- Verification: tests for missing acknowledgment and valid acknowledgment.
- Depends on: MT-008.

### MT-034 - Package And Release Diagnostics
- Objective: Preserve package/release evidence from CKC Windows/macOS scripts.
- Inputs: `package_win.ps1`, `package_mac.sh`, `release.ps1`, CKC taskboard release rows.
- Work slice: implement diagnostic record for stage root, artifacts root, build id, kind, version/tag, clean-repo guard, asset-path guard, package command, outputs.
- Outputs: package diagnostic schema and parser/check tests.
- Acceptance: diagnostics do not hardcode machine-local paths as product authority.
- Verification: tests for dev build record, release build record, dirty repo guard fail, absolute `/assets` guard fail.
- Depends on: MT-033.

### MT-035 - Installer Reset And Orphan Evidence Projection
- Objective: Expose Core reset/orphan behavior to model diagnostics.
- Inputs: CKC `installer_custom.nsh`, reset modes, Core reset/orphan MT.
- Work slice: add diagnostic views for update, reinstall preserve, light reset, full reset, orphan manifest, adoption results.
- Outputs: projection schema and tests.
- Acceptance: models can see what reset mode ran and where orphan adoption evidence lives.
- Verification: tests for each reset mode record and orphan adoption projection.
- Depends on: MT-034, Core reset MT.

### MT-036 - Stale README Spec Drift Detector
- Objective: Preserve and generalize the CKC README v00.063 versus spec v00.075 drift finding.
- Inputs: CKC README, CKC spec v00.075, repaired stubs.
- Work slice: add doc/spec pointer drift check for model-facing docs.
- Outputs: drift detector/check and diagnostic event.
- Acceptance: stale current-spec pointers fail with exact old/new values and file path.
- Verification: test fixture with v00.063 pointer against v00.075 current, and valid pointer pass.
- Depends on: MT-033.

### MT-037 - Background Automation Guard
- Objective: Preserve background-safe automation startup and single-instance no-focus safeguards.
- Inputs: CKC automation background mode and stealth guard.
- Work slice: implement runtime guard that routes dialogs/show/focus/global shortcuts through policy.
- Outputs: guard behavior and tests.
- Acceptance: background model operation cannot steal focus or pop windows.
- Verification: tests for hidden startup, show denied, focus denied, dialog denied, single-instance behavior.
- Depends on: MT-003.

### MT-038 - Synthetic Input Guard
- Objective: Preserve synthetic input only as governed, attributed, bounded interaction.
- Inputs: CKC WP-0099 synthetic input commands.
- Work slice: define injectKey/injectMouse/clickElement/typeText action contracts with target, session, preconditions, foreground reason, receipt, and safety limits.
- Outputs: synthetic input action definitions and tests; actual GUI wiring can be later.
- Acceptance: synthetic input cannot run without session, target, and policy approval.
- Verification: tests for missing session denial, invalid target denial, allowed bounded command, receipt emitted.
- Depends on: MT-005, MT-037.

### MT-039 - Parallel Model Coordination View
- Objective: Make parallel model actions observable, attributable, recoverable, and conflict-aware.
- Inputs: sessions, leases, command logs, Core/Pose parallel-editing policy.
- Work slice: implement a coordination projection with active sessions, held leases, recent commands, blockers, recovery actions, and conflict state.
- Outputs: projection API/tests.
- Acceptance: a no-context model can decide whether to proceed, wait, release stale lease, or escalate.
- Verification: tests for no active sessions, active lease conflict, stale lease, blocked command, recovery action.
- Depends on: MT-011 through MT-016.

### MT-040 - Manual And Action Catalog Validation Matrix
- Objective: Create validation rows for the model manual and action catalog only.
- Inputs: MT-004 through MT-010 and manual/action consistency requirements.
- Work slice: add validation rows for manual sections, action ids, command params, output schemas, safety notes, failure modes, recovery steps, and stale manual detection.
- Outputs: manual/action validation matrix rows and runnable check list.
- Acceptance: each manual/action acceptance criterion maps to a proof command, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for manual/action checks.
- Depends on: MT-004 through MT-010.

### MT-041 - Session Lease And Heartbeat Validation Matrix
- Objective: Create validation rows for sessions, leases, and heartbeats only.
- Inputs: MT-011 through MT-013.
- Work slice: add validation rows for session start/close, lease acquire/release/expiry, heartbeat timestamps, stale session detection, and owner attribution.
- Outputs: session/lease/heartbeat validation matrix rows and runnable check list.
- Acceptance: each session/lease/heartbeat acceptance criterion maps to a proof command, test, or artifact.
- Verification: matrix completeness check and dry-run for session/lease/heartbeat checks.
- Depends on: MT-011 through MT-013.

### MT-042 - Command Log Error And State Probe Validation Matrix
- Objective: Create validation rows for command logs, error catalog, recovery, and state probes only.
- Inputs: MT-014 through MT-019.
- Work slice: add validation rows for command log entries, cancellation/recovery, error code schemas, state probe schemas, receipt linkage, and restart reconstruction.
- Outputs: command/error/state validation matrix rows and runnable check list.
- Acceptance: each command/error/state acceptance criterion maps to a proof command, test, or artifact.
- Verification: matrix completeness check and dry-run for command/error/state checks.
- Depends on: MT-014 through MT-019.

### MT-043 - DCC And Flight Recorder Validation Matrix
- Objective: Create validation rows for DCC and Flight Recorder projections only.
- Inputs: MT-020 and MT-021.
- Work slice: add validation rows for DCC projection fields, active MT/Locus links, Flight Recorder event ids, source artifact links, and replayable event context.
- Outputs: DCC/FR validation matrix rows and runnable check list.
- Acceptance: each DCC/FR acceptance criterion maps to a proof command, test, or artifact.
- Verification: matrix completeness check and dry-run for DCC/FR checks.
- Depends on: MT-020, MT-021.

### MT-044 - Visual Evidence Validation Matrix
- Objective: Create validation rows for visual capture, diff, ADR, STEER, comparison, and retention only.
- Inputs: MT-022 through MT-029.
- Work slice: add validation rows for capture artifact metadata, native/html2canvas/DOM capture mode, ADR notes, pixel/structural diff reports, STEER rows, comparison reports, and retention policy.
- Outputs: visual-evidence validation matrix rows and runnable check list.
- Acceptance: each visual-evidence acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for visual evidence checks.
- Depends on: MT-022 through MT-029.

### MT-045 - Diagnostic Bundle Validation Matrix
- Objective: Create validation rows for diagnostic bundle creation and contents only.
- Inputs: MT-030.
- Work slice: add validation rows for bundle manifest, command log excerpt, session/lease ids, errors, state snapshot, visual artifacts, Flight Recorder events, and recovery notes.
- Outputs: diagnostic-bundle validation matrix rows and runnable check list.
- Acceptance: each bundle acceptance criterion maps to a proof command, test, or artifact.
- Verification: matrix completeness check and dry-run for diagnostic bundle checks.
- Depends on: MT-030.

### MT-046 - Local LLM And Chat Proposal Validation Matrix
- Objective: Create validation rows for local LLM config and local chat proposal boundaries only.
- Inputs: MT-031 and MT-032.
- Work slice: add validation rows for local model endpoint config, disabled/unavailable state, prompt/response receipt, proposal-only output, explicit apply gate, and failure reporting.
- Outputs: local-LLM/chat validation matrix rows and runnable check list.
- Acceptance: each local-LLM/chat acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for local-LLM/chat checks.
- Depends on: MT-031, MT-032.

### MT-047 - AI Tagging Validation Matrix
- Objective: Create validation rows for AI tagging proposal/apply behavior only.
- Inputs: MT-033 and Core AI tagging rows.
- Work slice: add validation rows for suggested tags, confidence/source metadata, proposal receipt, explicit apply command, reject command, audit record, and no silent media mutation.
- Outputs: AI-tagging validation matrix rows and runnable check list.
- Acceptance: each AI-tagging acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for AI-tagging checks.
- Depends on: MT-033.

### MT-048 - Build And Package Validation Matrix
- Objective: Create validation rows for build diagnostics and packaging/release evidence only.
- Inputs: MT-034.
- Work slice: add validation rows for build path rules, dev/release artifact roots, package/release receipts, version metadata, and no repo-local generated output leakage.
- Outputs: build/package validation matrix rows and runnable check list.
- Acceptance: each build/package acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for build/package checks.
- Depends on: MT-034.

### MT-049 - No-Focus Synthetic Input And Parallel Coordination Validation Matrix
- Objective: Create validation rows for non-intrusive automation, synthetic input guards, and parallel coordination only.
- Inputs: MT-037 through MT-039.
- Work slice: add validation rows for no foreground window/focus stealing, synthetic input target/session/preconditions, coordination projection sessions, lease conflicts, stale lease recovery, and blocked command recovery actions.
- Outputs: no-focus/synthetic/parallel validation matrix rows and runnable check list.
- Acceptance: each safety/coordination acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for no-focus/synthetic/parallel checks.
- Depends on: MT-037 through MT-039.

### MT-050 - Diagnostics Integration Smoke Path
- Objective: Prove one no-context model operation path end to end.
- Inputs: completed Diagnostics command/session/state/bundle pieces and fake Core/Pose actions.
- Work slice: create integration test: start session -> acquire lease -> read state -> run fake command -> emit receipt -> capture fake visual evidence -> bundle failure or success -> release lease.
- Outputs: integration test and evidence bundle.
- Acceptance: all evidence links are reconstructable after restart.
- Verification: run focused integration test and inspect command log, session, lease, FR events, artifact refs.
- Depends on: MT-011, MT-012, MT-018, MT-021, MT-010.

### MT-051 - Diagnostics Red-Team Automation Authority Guards
- Objective: Turn automation-authority risks into focused negative checks.
- Inputs: repaired Diagnostics risks for hidden UI automation, focus stealing, direct LLM execution, and unbounded synthetic input.
- Work slice: add checks that deny foreground/hidden automation without policy, prevent direct LLM product execution, require governed action/session receipts, and bound synthetic input by target/session/preconditions.
- Outputs: focused red-team checks for automation authority.
- Acceptance: each check fails when direct execution, focus stealing, or unbounded input is allowed.
- Verification: run focused red-team checks for no-focus, direct execution denial, session receipt, and synthetic input bounds.
- Depends on: MT-003, MT-005, MT-037, MT-038.

### MT-052 - Diagnostics Red-Team Manual Visual And Command Drift Guards
- Objective: Turn manual-command and visual-loop drift risks into focused negative checks.
- Inputs: repaired Diagnostics risks for manual-command drift, stale action catalog rows, visual loop dilution, and missing evidence links.
- Work slice: add checks that manual rows match action catalog ids/schemas, stale commands are detected, visual capture cannot be replaced by prose-only status, and comparison reports link to artifacts.
- Outputs: focused red-team checks for manual/action/visual drift.
- Acceptance: each check fails when manual/action drift or visual evidence dilution is introduced.
- Verification: run focused red-team checks for manual/action consistency, stale row detection, visual artifact requirement, and comparison report links.
- Depends on: MT-004 through MT-010, MT-022 through MT-030.

### MT-053 - Diagnostics Red-Team Local LLM Spec And Path Guards
- Objective: Turn local-LLM, stale-source, SQLite, and hardcoded-path risks into focused negative checks.
- Inputs: repaired Diagnostics risks for hidden local LLM side effects, stale spec pointers, SQLite fixtures, build path hardcoding, and non-portable roots.
- Work slice: add checks that local LLM/chat outputs remain proposals until applied, stale README/spec links are reported, SQLite fixtures are rejected, drive-letter/user-profile paths are rejected, and build paths stay configurable.
- Outputs: focused red-team checks for local LLM/spec/path guards.
- Acceptance: each check fails when hidden side effects, stale pointers, SQLite files, or hardcoded paths are present.
- Verification: run focused red-team checks for proposal/apply boundary, stale source report, SQLite tripwire, and path portability.
- Depends on: MT-031 through MT-036, MT-003.

### MT-054 - Manual Source Merge For Core Rows
- Objective: Merge Core/Data manual source rows into the Diagnostics manual dataset only.
- Inputs: Core manual rows from Core draft MT-052 through Core draft MT-060.
- Work slice: import or reference Core character/sheet, media/intake, organization/search, and export/backup/reset manual rows; normalize ids; mark missing Core rows as blockers.
- Outputs: Core portion of the manual source dataset and Core coverage report.
- Acceptance: every required Core surface has purpose, command/action ids, inputs, outputs, failures, recovery, and evidence paths.
- Verification: Core manual coverage report has zero missing required Core sections or explicit blockers.
- Depends on: MT-004 through MT-010 and Core manual rows.

### MT-055 - Manual Source Merge For Pose Rows
- Objective: Merge Pose/ComfyUI manual source rows into the Diagnostics manual dataset only.
- Inputs: Pose manual rows from Pose draft MT-042 through Pose draft MT-045.
- Work slice: import or reference Pose context/rig, sidecar/identity, workflow/replay/failure, and deferred adapter manual rows; normalize ids; mark missing Pose rows as blockers.
- Outputs: Pose portion of the manual source dataset and Pose coverage report.
- Acceptance: every required Pose surface has purpose, command/action ids, inputs, outputs, failures, recovery, status meaning, and evidence paths.
- Verification: Pose manual coverage report has zero missing required Pose sections or explicit blockers.
- Depends on: MT-004 through MT-010 and Pose manual rows.

### MT-056 - Manual Source Merge For Diagnostics Rows
- Objective: Merge Diagnostics-owned manual/action/state/error/bundle rows into the manual dataset only.
- Inputs: MT-004 through MT-039.
- Work slice: import or reference Diagnostics action catalog, session, lease, command log, error, state probe, DCC, Flight Recorder, visual capture, diagnostic bundle, local LLM, AI tagging, build diagnostic, and safety rows; normalize ids; mark missing Diagnostics rows as blockers.
- Outputs: Diagnostics portion of the manual source dataset and Diagnostics coverage report.
- Acceptance: every Diagnostics-owned surface has purpose, command/action ids, inputs, outputs, failures, recovery, and evidence paths.
- Verification: Diagnostics manual coverage report has zero missing required Diagnostics sections or explicit blockers.
- Depends on: MT-004 through MT-039.

### MT-057 - Manual Source Closure Coverage Report
- Objective: Produce the final manual-source closure report without editing product manual content.
- Inputs: MT-054, MT-055, and MT-056 coverage reports.
- Work slice: combine Core, Pose, and Diagnostics coverage reports; list missing rows as blockers; list duplicated/ambiguous action ids; list stale source links; produce activation-ready acceptance notes.
- Outputs: manual source closure report.
- Acceptance: closure report has zero unclassified missing rows; any missing row is an explicit blocker tied to its owning WP stub.
- Verification: run manual source closure check and inspect blocker list.
- Depends on: MT-054, MT-055, MT-056.

### MT-058 - DCC Jobs Tool Calls And Artifact Panels
- Objective: Define DCC visibility for jobs, tool calls, and artifact refs only.
- Inputs: Dev Command Center requirements, Workflow Engine job/tool-call policy, ArtifactStore reference contract.
- Work slice: add DCC projection contract for active jobs, job status, tool-call requests/results, artifact refs, artifact health, and missing-artifact indicators.
- Outputs: DCC job/tool/artifact projection tests; GUI implementation can be later.
- Acceptance: no hidden job/tool/artifact state is required to debug model operation.
- Verification: tests for active job, failed job, tool-call success/failure, artifact ref, missing artifact, and empty state.
- Depends on: MT-016, MT-017.

### MT-059 - DCC Approvals And Visual Capture Panels
- Objective: Define DCC visibility for approvals and visual captures only.
- Inputs: Dev Command Center requirements, approval policy, visual capture artifact contract.
- Work slice: add DCC projection contract for pending approvals, approval decisions, visual capture requests, capture artifacts, visual comparison summaries, and blocked capture reasons.
- Outputs: DCC approval/visual projection tests; GUI implementation can be later.
- Acceptance: approval and visual evidence state is observable without reading chat or screenshots manually.
- Verification: tests for pending approval, approved/denied decision, capture success, capture failure, comparison summary, and empty state.
- Depends on: MT-016, MT-022 through MT-029.

### MT-060 - Flight Recorder Tool Proposal And Apply Events
- Objective: Define Flight Recorder events for tool calls, proposals, and apply decisions only.
- Inputs: Product Reference tool-call policy, proposal/apply boundary, CKC automation evidence.
- Work slice: add event families for tool call requested/allowed/denied/result, proposal created, proposal rejected, proposal applied, and apply failure.
- Outputs: tool/proposal/apply event schema catalog and tests.
- Acceptance: tool and proposal/apply actions have replay-grade event evidence and no direct LLM execution ambiguity.
- Verification: tests for tool-call events, proposal events, apply events, denial events, and no secret leakage.
- Depends on: MT-008, MT-018.

### MT-061 - Flight Recorder Visual Validation And Recovery Events
- Objective: Define Flight Recorder events for visual capture, validation, and recovery only.
- Inputs: visual debugging loop requirements, validation matrix policy, cancellation/recovery contracts.
- Work slice: add event families for visual capture requested/completed/failed, comparison generated, validation started/completed/failed, recovery action offered, and recovery action completed/failed.
- Outputs: visual/validation/recovery event schema catalog and tests.
- Acceptance: visual evidence, validation results, and recovery steps have replay-grade event evidence.
- Verification: tests for visual capture events, comparison events, validation events, recovery events, and artifact refs.
- Depends on: MT-015, MT-022 through MT-030.

### MT-062 - Flight Recorder Build Package And Stale-Doc Events
- Objective: Define Flight Recorder events for build/package guard and stale-doc detection only.
- Inputs: build/release diagnostics, stale README/spec drift requirements.
- Work slice: add event families for build guard started/completed/failed, package evidence recorded, stale README/spec scan started/completed/failed, and stale source row reported.
- Outputs: build/package/stale-doc event schema catalog and tests.
- Acceptance: build/package and stale-doc diagnostics have replay-grade event evidence.
- Verification: tests for build guard events, package evidence events, stale-doc scan events, stale source row events, and no hardcoded path leakage.
- Depends on: MT-034, MT-036.

### MT-063 - Reset And Orphan Diagnostic Validation Matrix
- Objective: Create validation rows for reset and orphan diagnostics only.
- Inputs: MT-035.
- Work slice: add validation rows for reset request/result, original media preservation evidence, orphan manifest, orphan adoption receipt, invalid orphan manifest error, and EventLedger linkage.
- Outputs: reset/orphan validation matrix rows and runnable check list.
- Acceptance: each reset/orphan acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for reset/orphan checks.
- Depends on: MT-035.

### MT-064 - Stale Source And Path Portability Validation Matrix
- Objective: Create validation rows for stale README/spec detection and path portability only.
- Inputs: MT-036 and build path rules from MT-034.
- Work slice: add validation rows for stale README link, stale spec pointer, taskboard/spec mismatch, drive-letter path rejection, user-profile path rejection, and configurable artifact/data roots.
- Outputs: stale-source/path-portability validation matrix rows and runnable check list.
- Acceptance: each stale-source/path acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for stale-source/path checks.
- Depends on: MT-034, MT-036.

### MT-065 - Flight Recorder Event Validation Matrix
- Objective: Create validation rows for Flight Recorder event families only.
- Inputs: MT-018, MT-060, MT-061, and MT-062.
- Work slice: add validation rows for session/command events, tool/proposal/apply events, visual/validation/recovery events, build/package/stale-doc events, artifact refs, owner attribution, and secret/path redaction.
- Outputs: Flight Recorder event validation matrix rows and runnable check list.
- Acceptance: each event-family acceptance criterion maps to a proof command, test, artifact, or manual review row.
- Verification: matrix completeness check and dry-run for Flight Recorder event checks.
- Depends on: MT-018, MT-060, MT-061, MT-062.

### MT-066 - Diagnostics Activation Refinement Closure
- Objective: Convert the repaired stub plus this suite into activation-ready refinement content.
- Inputs: source matrix, anchor map, MT suite, validation matrix, red-team controls, manual source closure.
- Work slice: write Diagnostics refinement section with scope, non-goals, dependencies, risks, acceptance, and MT plan.
- Outputs: refinement draft ready for operator signature.
- Acceptance: no MT depends on unverified product anchors without a blocker; build/visual/local-LLM/stale-doc concerns remain explicit.
- Verification: packet/refinement contract check selected at activation.
- Depends on: MT-001 through MT-065.

</topic>
