# Handshake Project Task Board (Phase 1: EXHAUSTIVE STRATEGIC AUDIT)

## Spec Authority Rule [CX-598] (HARD INVARIANT)

**The Roadmap (Section 7.6) is ONLY a pointer. The Master Spec Main Body (Sections 1-6, 9-11) is the SOLE definition of "Done."**

| Principle | Enforcement |
|-----------|-------------|
| **Roadmap = Pointer** | Section 7.6.x items point to Main Body sections where requirements are defined |
| **Main Body = Truth** | Every MUST/SHOULD in Sections 1-6, 9-11 must be implemented - no exceptions |
| **No Debt** | Skipping requirements poisons the project; later phases inherit rotten foundations |
| **No Phase Closes** | Until ALL referenced Main Body requirements are VALIDATED |

**Why:** Handshake is complex software. Treating roadmap bullets as requirements (instead of pointers) leads to surface-level compliance, technical debt, and project failure.

This board provides an exhaustive tracking of all Roadmap items from A7.6.3. Phase 1 cannot close until every item below is validated against the current Master Spec (see `.GOV/spec/SPEC_CURRENT.md`).

**Task Board entry format (enforced for In Progress/Done/Superseded via `just task-board-check`):**
- In Progress: `- **[WP_ID]** - [IN_PROGRESS]`
- Done: `- **[WP_ID]** - [MERGE_PENDING|VALIDATED|FAIL|OUTDATED_ONLY|ABANDONED]`
- Superseded: `- **[WP_ID]** - [SUPERSEDED]`
Keep details (failure reasons, commands, evidence, \"SUPERSEDED by ...\") in the work packet to avoid drift/noise.

`[MERGE_PENDING]` means validator PASS has been appended and the packet is waiting on merge-to-main containment; `[VALIDATED]` is reserved for packets whose approved closure commit is already contained in local `main`.

Historical failed closures that still act as live smoketest baselines are tracked separately under `## Historical Failed Closures Used As Live Smoketest Baselines`. They remain `[SUPERSEDED]` in the archive and do not reopen execution on the historical packet itself.

**Phase 1 closure note:**
- Historical `## Done` entries record closure under the governance/workflow that existed at the time.
- They do **not** by themselves grant final Phase 1 signoff or prove current implementation quality.
- End-of-Phase-1 revalidation will be tracked on this board when that effort begins; until then, treat historical `Done` as prior closure state, not final Phase 1 approval.

**Additional recommended entry format (not currently enforced):**
- Ready for Dev: `- **[WP_ID]** - [READY_FOR_DEV]`
- Stub Backlog: `- **[WP_ID]** - [STUB]`
- Blocked: `- **[WP_ID]** - [BLOCKED] - <reason>`
- Active (Cross-Branch Status): `- **[WP_ID]** - [ACTIVE] - branch: feat/WP-{ID} - coder: <name/model> - last_sync: YYYY-MM-DD`

**Backlog stubs (pre-activation):**
- Track not-yet-activated work as STUB items (no USER_SIGNATURE yet). Details live in `.GOV/task_packets/stubs/`.
- Stubs MUST be activated into official work packets before any coding starts (see `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- Base WP to active packet mapping (v2/v3/v4 and stub revisions) is tracked in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.
- Only the registry-mapped active packet or stub for a `BASE_WP_ID` may remain non-superseded. Any older packet or older stub with the same `BASE_WP_ID` must move to `## Superseded (Archive)`.

---

## Active (Cross-Branch Status)

This section exists to keep the Operator's **main-branch** Task Board up to date when multiple Coders are working in separate WP branches/worktrees.

Rules:
- This section is informational for visibility across branches (who is working on what).
- Do NOT use `[IN_PROGRESS]` here (that token is reserved for the script-checked `## In Progress` list).
- Validator maintains this section on `main` via small docs-only "status sync" commits.
- Status sync commits MUST NOT move WPs to `## Done` or set `[VALIDATED|FAIL|OUTDATED_ONLY]` tokens; those require the canonical Validator report appended to the work packet `## VALIDATION_REPORTS`.

Entry format (recommended):
- `- **[WP_ID]** - [ACTIVE] - branch: feat/WP-{ID} - coder: <name/model> - last_sync: YYYY-MM-DD`


## Current Focus (2026-04-05)

- Self-hosting priority is backend-first. Handshake already has meaningful backend runtime for terminal execution, ModelSession scheduling, Ollama integration, Locus storage, Role Mailbox, and Flight Recorder; the next unblocked tranche is the control-plane and governance substrate that lets those systems work together inside the product.
- Repo governance is being ported into Handshake as an additive software-delivery overlay. It does not replace Handshake-native governance, and it must not collapse Handshake's broader topology, worksurface model, project-profile extension model, or four-layer architecture into repo-only assumptions.
- The oversized "port all repo governance" idea is therefore split into bounded backend-first packets. See `.GOV/roles_shared/docs/PRODUCT_SELF_HOSTING_BACKEND_FOCUS_20260405.md` before activating governance-overlay, session-substrate, or Dev Command Center work.
- Mixed-provider self-hosting is now an explicit backend concern: repo governance can declare GPT and Claude Code role profiles already, but provider-specific governed runtime support is still a follow-on. Keep that split explicit when activating session/governance-overlay packets.

---



## Ready for Dev

A WP is only Ready for Dev if its Active Packet (per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`) is an official packet under `.GOV/task_packets/` (not a stub).
















## Stub Backlog (Not Activated)

Note: This section is an **inventory list**, not a priority order. Do not infer importance from list order; use `.GOV/roles_shared/records/BUILD_ORDER.md` (Priority Views) and the per-WP `BUILD_ORDER_*` metadata instead.
- **[WP-1-Video-Archive-Loom-Integration-v1]** - [STUB]
- **[WP-1-Media-Downloader-Loom-Bridge-v1]** - [STUB]
- **[WP-1-Loom-Preview-VideoPosterFrames-v1]** - [STUB]
- **[WP-1-ASR-Transcribe-Media-v1]** - [STUB]
- **[WP-1-Stage-ASR-Transcript-Lineage-v1]** - [STUB]
- **[WP-1-Governance-Pack-v1]** - [STUB]
- **[WP-1-Spec-Authoring-Rubric-v1]** - [STUB]
- **[WP-1-Spec-Creation-System-v2.2.1-v1]** - [STUB]
- **[WP-1-Locus-Work-Tracking-System-Phase1-v1]** - [STUB]
- **[WP-1-Locus-Debug-Bundle-Bridge-v1]** - [STUB]
- **[WP-1-Locus-Phase1-QueryContract-Autosync-v1]** - [STUB]
- **[WP-1-Locus-Phase1-Medallion-Search-v1]** - [STUB]
- **[WP-1-LocalFirst-Agentic-MCP-Posture-v1]** - [STUB]
- **[WP-1-Storage-No-Runtime-DDL-v1]** - [STUB]
- **[WP-1-Spec-Router-Session-Log]** - [STUB]
- **[WP-1-Spec-Router-CapabilitySnapshot-v1]** - [STUB]
- **[WP-1-Spec-Router-Evidence-Portability-v1]** - [STUB]
- **[WP-1-Spec-Router-SpecLint-v1]** - [STUB]
- **[WP-1-Metrics-OTel-v2]** - [STUB]
- **[WP-1-ACE-Auditability-v2]** - [STUB]
- **[WP-1-ACE-Persist-QueryPlan-Trace-v1]** - [STUB]
- **[WP-1-RAG-Iterative-v2]** - [STUB]
- **[WP-1-RAG-Retrieval-Mode-Policy-v1]** - [STUB]
- **[WP-1-AIReady-CoreMetadata-v1]** - [STUB]
- **[WP-1-AIReady-Index-Evidence-Export-v1]** - [STUB]
- **[WP-1-AIReady-RelationshipIds-GraphRetrieval-v1]** - [STUB]
- **[WP-1-Model-Profiles-v2]** - [STUB]
- **[WP-1-MEX-Safety-Gates-v2]** - [STUB]
- **[WP-1-MEX-Observability-v2]** - [STUB]
- **[WP-1-MCP-MEX-Evidence-Export-v1]** - [STUB]
- **[WP-1-MEX-UX-Bridges-v2]** - [STUB]
- **[WP-1-MTE-Summaries-v1]** - [STUB]
- **[WP-1-MTE-DropBack-Smart-v1]** - [STUB]
- **[WP-1-MTE-LoRA-Wiring-v1]** - [STUB]
- **[WP-1-MTE-Blocked-Decisioning-v1]** - [STUB]
- **[WP-1-MTE-Resource-Caps-v1]** - [STUB]
- **[WP-1-AI-UX-Rewrite-v2]** - [STUB]
- **[WP-1-PDF-Pipeline-v2]** - [STUB]
- **[WP-1-Photo-Studio-v2]** - [STUB]
- **[WP-1-Atelier-Lens-v2]** - [STUB]
- **[WP-1-Studio-Runtime-Visibility-v1]** - [STUB]
- **[WP-1-Calendar-Lens-v3]** - [STUB]
- **[WP-1-Calendar-Sync-Engine-v1]** - [STUB]
- **[WP-1-Calendar-Policy-Integration-v1]** - [STUB]
- **[WP-1-Calendar-Law-Compliance-Tests-v1]** - [STUB]
- **[WP-1-Calendar-Correlation-Export-v1]** - [STUB]
- **[WP-1-Calendar-Mailbox-Correlation-v1]** - [STUB]
- **[WP-1-Distillation-v2]** - [STUB]
- **[WP-1-ContextPack-Recorder-Visibility-v1]** - [STUB]
- **[WP-1-Governance-Hooks-v2]** - [STUB]
- **[WP-1-Governance-Workflow-Mirror-v1]** - [STUB]
- **[WP-1-Product-Governance-Check-Runner-v1]** - [STUB]
- **[WP-1-Workspace-Bundle-v2]** - [STUB]
- **[WP-1-Semantic-Catalog-v2]** - [STUB]
- **[WP-1-Metrics-Traces-v2]** - [STUB]
- **[WP-1-Work-Profiles-v1]** - [STUB]
- **[WP-1-Inbox-Role-Mailbox-Alignment-v1]** - [STUB]
- **[WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1]** - [STUB]
- **[WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1]** - [STUB]
- **[WP-1-FEMS-Acceptance-Replay-Eval-v1]** - [STUB]
- **[WP-1-Provider-Feature-Coverage-Agentic-Ready-v1]** - [STUB]
- **[WP-1-Workspace-Safety-Parallel-Sessions-v1]** - [STUB]
- **[WP-1-Session-Observability-Spans-FR-v1]** - [STUB]
- **[WP-1-Consent-Audit-Projection-v1]** - [STUB]
- **[WP-1-Cloud-Consent-Evidence-Portability-v1]** - [STUB]
- **[WP-1-Diagnostics-Debug-Bundle-Bridge-v1]** - [STUB]
- **[WP-1-Session-Anti-Pattern-Registry-v1]** - [STUB]
- **[WP-1-Layerwise-Inference-Foundations-v1]** - [STUB]
- **[WP-1-Project-Agnostic-Workflow-State-Registry-v1]** - [STUB]
- **[WP-1-Workflow-Transition-Automation-Registry-v1]** - [STUB]
- **[WP-1-Markdown-Mirror-Sync-Drift-Guard-v1]** - [STUB]
- **[WP-1-Dev-Command-Center-Control-Plane-Backend-v1]** - [STUB]
- **[WP-1-Dev-Command-Center-MVP-v1]** - [STUB]
- **[WP-1-Dev-Command-Center-Layout-Projection-Registry-v1]** - [STUB]
- **[WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1]** - [STUB]
- **[WP-1-Handshake-Stage-MVP-v1]** - [STUB]
- **[WP-1-Stage-Media-Artifact-Portability-v1]** - [STUB]
- **[WP-1-Mail-Runtime-Backfill-v1]** - [STUB]
- **[WP-1-Lens-Extraction-Tier-v1]** - [STUB]
- **[WP-1-Role-Turn-Isolation-v1]** - [STUB]
- **[WP-1-Git-Engine-Decision-Gate-v1]** - [STUB]
- **[WP-1-Retrieval-Trace-Bundle-Export-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Message-Thread-Contract-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Triage-Queue-Controls-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1]** - [STUB]
- **[WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1]** - [STUB]
- **[WP-1-Product-Screenshot-Visual-Validation-v1]** - [STUB]
- **[WP-1-Session-Spawn-Tree-DCC-Visualization-v1]** - [STUB]
- **[WP-1-Session-Spawn-Conversation-Distillation-v1]** - [STUB]
- **[WP-1-In-Product-Session-Manager-v1]** - [STUB]
- **[WP-1-Distillation-Training-Pair-Extraction-v1]** - [STUB]
- **[WP-1-Ollama-Local-Model-MT-Routing-v1]** - [STUB]
- **[WP-1-Visual-Debugging-Loop-v1]** - [STUB]
- **[WP-1-Product-Compile-Validation-Gate-v1]** - [STUB]
- **[WP-1-Product-Red-Team-Agent-v1]** - [STUB]
- **[WP-1-Product-MT-Lifecycle-Escalation-v1]** - [STUB]
- **[WP-1-Product-Session-Communication-Database-v1]** - [STUB]
- **[WP-1-Product-MT-Task-Board-v1]** - [STUB]
- **[WP-1-Product-Failure-Knowledge-Base-v1]** - [STUB]
- **[WP-1-Product-Session-Health-Monitor-v1]** - [STUB]
- **[WP-1-Product-Role-Tool-Permissions-v1]** - [STUB]
- **[WP-1-Product-MT-Quality-Gates-v1]** - [STUB]


- **[WP-1-Charts-Dashboards-Backfill-v1]** - [STUB]
- **[WP-1-Docs-Sheets-Runtime-Backfill-v1]** - [STUB]
- **[WP-1-Thinking-Pipeline-Runtime-Backfill-v1]** - [STUB]
- **[WP-1-Canvas-Runtime-Backfill-v1]** - [STUB]
- **[WP-1-Presentations-Decks-Backfill-v1]** - [STUB]
- **[WP-1-Project-Brain-Runtime-Backfill-v1]** - [STUB]

## In Progress

Assignee/model is recorded in the work packet (CODER_MODEL, CODER_REASONING_STRENGTH). Task Board stays minimal.





















































































































































































































## Done
- **[WP-1-Spec-Router-SpecPromptCompiler-v1]** - [VALIDATED]
- **[WP-1-Front-End-Memory-System-v1]** - [VALIDATED]
- **[WP-1-Unified-Tool-Surface-Contract-v1]** - [VALIDATED]
- **[WP-1-Lens-ViewMode-v1]** - [VALIDATED]
- **[WP-1-Cloud-Escalation-Consent-v2]** - [VALIDATED]
- **[WP-1-Autonomous-Governance-Protocol-v2]** - [VALIDATED]
- **[WP-1-Model-Onboarding-ContextPacks-v1]** - [VALIDATED]
- **[WP-1-Spec-Enrichment-Product-Governance-Consistency-v1]** - [VALIDATED]
- **[WP-1-LLM-Provider-Registry-v1]** - [VALIDATED]
- **[WP-1-Runtime-Governance-NoExpect-v1]** - [VALIDATED]
- **[WP-1-Product-Governance-Snapshot-v4]** - [VALIDATED]
- **[WP-1-Flight-Recorder-v4]** - [VALIDATED]
- **[WP-1-Supply-Chain-Cargo-Deny-Clean-v1]** - [VALIDATED]
- **[WP-1-Artifact-System-Foundations-v1]** - [VALIDATED]
- **[WP-1-Model-Swap-Protocol-v1]** - [VALIDATED]
- **[WP-1-ModelSession-Core-Scheduler-v1]** - [VALIDATED]
- **[WP-1-AI-UX-Summarize-Display-v2]** - [VALIDATED]

- **[WP-1-Atelier-Collaboration-Panel-v1]** - [VALIDATED]
- **[WP-1-Response-Behavior-ANS-001]** - [VALIDATED]
- **[WP-1-Global-Silent-Edit-Guard]** - [VALIDATED]
- **[WP-1-AI-Ready-Data-Architecture-v1]** - [VALIDATED]
- **[WP-1-Micro-Task-Executor-v1]** - [VALIDATED]
- **[WP-1-AI-UX-Actions-v2]** - [VALIDATED]
- **[WP-1-Dev-Experience-ADRs-v1]** - [VALIDATED]
- **[WP-1-Editor-Hardening-v2]** - [VALIDATED]
- **[WP-1-Governance-Kernel-Conformance-v1]** - [VALIDATED]
- **[WP-1-Governance-Template-Volume-v1]** - [VALIDATED]
- **[WP-1-Role-Mailbox-v1]** - [VALIDATED]
- **[WP-1-Role-Registry-AppendOnly-v1]** - [VALIDATED]
- **[WP-1-Loom-MVP-v1]** - [VALIDATED]
- **[WP-1-Media-Downloader-v2]** - [VALIDATED]
- **[WP-1-Migration-Framework-v2]** - [VALIDATED]
- **[WP-1-ACE-Validators-v4]** - [VALIDATED]
- **[WP-1-LLM-Core-v3]** - [VALIDATED]
- **[WP-1-OSS-Register-Enforcement-v1]** - [VALIDATED]
- **[WP-1-Tokenization-Service-v3]** - [VALIDATED]
- **[WP-1-Security-Gates-v3]** - [VALIDATED]
- **[WP-1-Gate-Check-Tool-v2]** - [VALIDATED]
- **[WP-1-Workflow-Engine-v4]** - [VALIDATED]
- **[WP-1-Debug-Bundle-v3]** - [VALIDATED]
- **[WP-1-Validator-Error-Codes-v1]** - [VALIDATED]
- **[WP-1-Storage-Foundation-v3]** - [VALIDATED]
- **[WP-1-Storage-Abstraction-Layer-v3]** - [VALIDATED]
- **[WP-1-AppState-Refactoring-v3]** - [VALIDATED]
- **[WP-1-Dual-Backend-Tests-v2]** - [VALIDATED]
- **[WP-1-Terminal-LAW-v3]** - [VALIDATED]
- **[WP-1-MEX-v1.2-Runtime-v3]** - [VALIDATED]
- **[WP-1-Operator-Consoles-v3]** - [VALIDATED]
- **[WP-1-Flight-Recorder-UI-v3]** - [VALIDATED]
- **[WP-1-Supply-Chain-MEX-v2]** - [VALIDATED]
- **[WP-1-Mutation-Traceability-v2]** - [VALIDATED]
- **[WP-1-Metrics-Mock-Tokens]** - [VALIDATED]
- **[WP-1-Canvas-Typography-v2]** - [VALIDATED]
- **[WP-1-ACE-Runtime-v2]** - [VALIDATED]
- **[WP-1-OSS-Governance-v2]** - [VALIDATED]
- **[WP-1-Capability-SSoT-v2]** - [VALIDATED]
- **[WP-1-AI-Job-Model-v4]** - [VALIDATED]
- **[WP-1-Cross-Tool-Interaction-Conformance-v1]** - [VALIDATED]
- **[WP-1-MCP-Skeleton-Gate-v2]** - [VALIDATED]
- **[WP-1-MCP-End-to-End-v2]** - [VALIDATED]



- **[WP-1-Session-Scoped-Capabilities-Consent-Gate-v1]** - [VALIDATED]



- **[WP-1-Spec-Appendices-Backfill-v1]** - [VALIDATED]

- **[WP-1-Postgres-MCP-Durable-Progress-v1]** - [VALIDATED]

- **[WP-1-Locus-Phase1-Integration-Occupancy-v1]** - [VALIDATED]
- **[WP-1-Spec-Enrichment-LLM-Core-v1]** - [OUTDATED_ONLY]
- **[WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2]** - [OUTDATED_ONLY]

- **[WP-1-Structured-Collaboration-Artifact-Family-v1]** - [VALIDATED]



- **[WP-1-Structured-Collaboration-Schema-Registry-v4]** - [VALIDATED]

- **[WP-1-Structured-Collaboration-Contract-Hardening-v1]** - [VALIDATED]

- **[WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1]** - [VALIDATED]

- **[WP-1-Loom-Storage-Portability-v4]** - [VALIDATED]

- **[WP-1-Workflow-Projection-Correlation-v1]** - [VALIDATED]



- **[WP-1-Project-Profile-Extension-Registry-v1]** - [VALIDATED]



- **[WP-1-Storage-Trait-Purity-v1]** - [VALIDATED]



- **[WP-1-Storage-Capability-Boundary-Refactor-v1]** - [VALIDATED]

- **[WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1]** - [VALIDATED]

- **[WP-1-Product-Governance-Artifact-Registry-v1]** - [VALIDATED]

- **[WP-1-Session-Spawn-Contract-v1]** - [VALIDATED]



- **[WP-1-FR-ModelSessionId-v1]** - [VALIDATED]

- **[WP-1-Session-Crash-Recovery-Checkpointing-v1]** - [VALIDATED]

## Blocked
- **[WP-1-Calendar-Storage-v1]** - [BLOCKED]
---
















## Historical Failed Closures Used As Live Smoketest Baselines

Entry format for this section:
`- **[WP_ID]** - [FAILED_HISTORICAL_SMOKETEST_BASELINE] - base_wp_id: <BASE_WP_ID> - active_recovery: <WP_ID> - live_status: <LIVE_SMOKETEST_BASELINE_PENDING|LIVE_SMOKETEST_BASELINE_RECOVERED>`

- **[WP-1-Structured-Collaboration-Schema-Registry-v3]** - [FAILED_HISTORICAL_SMOKETEST_BASELINE] - base_wp_id: WP-1-Structured-Collaboration-Schema-Registry - active_recovery: WP-1-Structured-Collaboration-Schema-Registry-v4 - live_status: LIVE_SMOKETEST_BASELINE_RECOVERED
- **[WP-1-Loom-Storage-Portability-v3]** - [FAILED_HISTORICAL_SMOKETEST_BASELINE] - base_wp_id: WP-1-Loom-Storage-Portability - active_recovery: WP-1-Loom-Storage-Portability-v4 - live_status: LIVE_SMOKETEST_BASELINE_PENDING

---

## Superseded (Archive)
- **[WP-1-Media-Downloader-v1]** - [SUPERSEDED]
- **[WP-1-AppState-Refactoring]** - [SUPERSEDED]
- **[WP-1-AppState-Refactoring-v2]** - [SUPERSEDED]
- **[WP-1-Product-Governance-Snapshot-v3]** - [SUPERSEDED]
- **[WP-1-Tokenization-Service-20251228]** - [SUPERSEDED]
- **[WP-1-Storage-Foundation-20251228]** - [SUPERSEDED]
- **[WP-1-Storage-Foundation]** - [SUPERSEDED]
- **[WP-1-Gate-Check-Tool]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v2]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v1]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles]** - [SUPERSEDED]
- **[WP-1-Diagnostic-Pipe]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-v2]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-v3]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v3]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v2]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v2]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model]** - [SUPERSEDED]
- **[WP-1-ACE-Validators]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v3]** - [SUPERSEDED]
- **[WP-1-Dual-Backend-Tests]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI]** - [SUPERSEDED]
- **[WP-1-LLM-Core]** - [SUPERSEDED]
- **[WP-1-Security-Gates]** - [SUPERSEDED]
- **[WP-1-Security-Gates-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW-v2]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle-v2]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Auditability]** - [SUPERSEDED]
- **[WP-1-ACE-Runtime]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v3]** - [SUPERSEDED]
- **[WP-1-AI-UX-Actions]** - [SUPERSEDED]
- **[WP-1-AI-UX-Rewrite]** - [SUPERSEDED]
- **[WP-1-AI-UX-Summarize-Display]** - [SUPERSEDED]
- **[WP-1-Atelier-Lens]** - [SUPERSEDED]
- **[WP-1-Calendar-Lens]** - [SUPERSEDED]
- **[WP-1-Canvas-Typography]** - [SUPERSEDED]
- **[WP-1-Capability-SSoT]** - [SUPERSEDED]
- **[WP-1-Distillation]** - [SUPERSEDED]
- **[WP-1-Editor-Hardening]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI-v2]** - [SUPERSEDED]
- **[WP-1-Governance-Hooks]** - [SUPERSEDED]
- **[WP-1-MCP-End-to-End]** - [SUPERSEDED]
- **[WP-1-MCP-Skeleton-Gate]** - [SUPERSEDED]
- **[WP-1-Metrics-OTel]** - [SUPERSEDED]
- **[WP-1-Metrics-Traces]** - [SUPERSEDED]
- **[WP-1-MEX-Observability]** - [SUPERSEDED]
- **[WP-1-MEX-Safety-Gates]** - [SUPERSEDED]
- **[WP-1-MEX-UX-Bridges]** - [SUPERSEDED]
- **[WP-1-Migration-Framework]** - [SUPERSEDED]
- **[WP-1-Model-Profiles]** - [SUPERSEDED]
- **[WP-1-Mutation-Traceability]** - [SUPERSEDED]
- **[WP-1-Product-Governance-Snapshot-v1]** - [SUPERSEDED]
- **[WP-1-OSS-Governance]** - [SUPERSEDED]
- **[WP-1-PDF-Pipeline]** - [SUPERSEDED]
- **[WP-1-Photo-Studio]** - [SUPERSEDED]
- **[WP-1-Product-Governance-Snapshot-v2]** - [SUPERSEDED]
- **[WP-1-RAG-Iterative]** - [SUPERSEDED]
- **[WP-1-Semantic-Catalog]** - [SUPERSEDED]
- **[WP-1-Supply-Chain-MEX]** - [SUPERSEDED]
- **[WP-1-Autonomous-Governance-Protocol-v1]** - [SUPERSEDED]
- **[WP-1-Cloud-Escalation-Consent-v1]** - [SUPERSEDED]
- **[WP-1-Dev-Experience-ADRs]** - [SUPERSEDED]
- **[WP-1-Calendar-Lens-v2]** - [SUPERSEDED]
- **[WP-1-Workspace-Bundle]** - [SUPERSEDED]
- **[WP-1-Tokenization-Service]** - [SUPERSEDED]
]

- **[WP-1-Structured-Collaboration-Schema-Registry-v1]** - [SUPERSEDED]
- **[WP-1-Structured-Collaboration-Schema-Registry-v2]** - [SUPERSEDED]
- **[WP-1-Structured-Collaboration-Schema-Registry-v3]** - [SUPERSEDED]

- **[WP-1-Loom-Storage-Portability-v1]** - [SUPERSEDED]
- **[WP-1-Loom-Storage-Portability-v2]** - [SUPERSEDED]
- **[WP-1-Loom-Storage-Portability-v3]** - [SUPERSEDED]
