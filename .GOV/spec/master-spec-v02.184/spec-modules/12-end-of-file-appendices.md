---
schema: handshake.indexed_spec.module@1
spec_version: "v02.184"
bundle_id: "master-spec-v02.184"
module_id: "12"
section_id: "12"
title: "12. End-of-File Appendices (Feature Index + Matrix + UI Guidance) [CX-SPEC-APPX-001]"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "9dd972a12685e6d7c41097bcc24bd7ff1e90f7c4da31072eeb47b9a2c7c0eacf"
body_sha256: "f46fe94e26d7688411614d7ab1129a4c9e0ea8cf6c2813d62c431b059521dca6"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 12. End-of-File Appendices (Feature Index + Matrix + UI Guidance) [CX-SPEC-APPX-001]

## 12.1 Why these appendices exist [CX-SPEC-APPX-002]

Handshake is an IDE and an execution harness. As the number of features/primitives/tools/technologies grows, the spec needs an explicit inventory that:
- keeps the Master Spec self-contained (no normative dependence on external derived files),
- forces per-feature UI guidance to exist (so features do not ship without an interaction contract),
- makes interactions explicit (so "everything can use everything" remains safe and coherent),
- reduces cognitive load for humans and external LLMs by providing a stable index and matrix.

These appendices live at the end of the Master Spec so that:
- the Main Body remains the primary reading flow,
- the appendices can be treated as a stable, parseable contract surface,
- derived views can be generated without changing meaning.

## 12.2 Maintenance rules (HARD) [CX-SPEC-APPX-003]

Hard invariants:
1. The Master Spec remains the source of truth. These appendices are inside this file to preserve that.
2. The appendix blocks MUST be the last major section in the file (end-of-file). Do not move them earlier.
3. Each appendix block MUST be a fenced block bracketed by BEGIN/END markers with a unique id and schema version.
4. UI guidance is REQUIRED for new/changed features only:
   - If a feature is introduced or materially changed, its UI guidance entry MUST be added/updated in the UI guidance appendix.
   - Legacy features MAY be missing UI guidance until backfilled; track backfill as a stub WP (do not block unrelated work).
5. External derived files (indexes/matrices extracted into repo folders) are allowed, but MUST be explicitly labeled DERIVED and MUST be regeneratable from this spec. They are never normative.

## 12.3 Appendix Block: FEATURE_REGISTRY (Machine-readable) [CX-SPEC-APPX-010]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-FEATURE-REGISTRY schema=hs_feature_registry@2 -->
```json
{
  "schema": "hs_feature_registry@2",
  "spec_version": "v02.184",
  "last_updated": "2026-05-05",
  "features": [
    {
      "feature_id": "FEAT-ACE-RUNTIME",
      "title": "ACE Runtime (Agentic Context Engineering)",
      "spec_anchor": "Â§2.6.6.7",
      "surfaces": [
        "backend"
      ],
      "primitives": [
        "PRIM-AceRuntimeValidator",
        "PRIM-ContextPackFreshnessPolicyV1",
        "PRIM-ContextPackPayloadV1",
        "PRIM-ContextPackRecord",
        "PRIM-QueryPlan",
        "PRIM-RetrievalTrace",
        "PRIM-TokenizationService"
      ],
      "tools_tech": [
        "TOOL-OLLAMA-API",
        "TECH-JSON",
        "TECH-RUST"
      ],
      "notes": "Context compilation + validator-enforced safety for AI jobs. [ADD v02.157] ACE runtime reuses Context Pack payloads, freshness-policy decisions, and retrieval traces as bounded backend compactions for deterministic job assembly, replay, and later distillation evidence. [ADD v02.178] ACE Runtime also owns explicit retrieval-mode selection and non-hybrid reasons so direct load, exact lookup, graph traversal, and hybrid retrieval remain visible, replayable, and policy-auditable instead of collapsing into one implicit RAG path."
    },
    {
      "feature_id": "FEAT-AI-JOB-MODEL",
      "title": "AI Job Model (Global)",
      "spec_anchor": "#266-ai-job-model-global",
      "surfaces": [
        "backend",
        "operator_consoles",
        "ui"
      ],
      "primitives": [
        "PRIM-AiJob",
        "PRIM-JobKind",
        "PRIM-JobState",
        "PRIM-JobMetrics",
        "PRIM-WorkflowContext",
        "PRIM-AiJobsDrawer",
        "PRIM-JobResultPanel",
        "PRIM-AiJobListFilter",
        "PRIM-CreateJobRequest",
        "PRIM-JobStatusUpdate",
        "PRIM-CloudEscalationConsentRequest"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "Single global job lifecycle used by all AI and mechanical workflows; observable in Operator Consoles. [ADD v02.146] The always-on job drawer and inspector flow are part of the canonical runtime surface, not optional shell chrome. [ADD v02.147] Job list, create, status-update, and consent envelopes are part of the canonical projection surface for jobs, exports, and operator drilldown. [ADD v02.150] Jobs are canonical bounded export anchors for debug bundles and must preserve stable job_id-based scope handoff into bundle/status flows."
    },
    {
      "feature_id": "FEAT-AI-READY-DATA",
      "title": "AI-Ready Data Architecture (Shadow Workspace + Indexing)",
      "spec_anchor": "#2314-ai-ready-data-architecture-add-v02115",
      "surfaces": [
        "backend"
      ],
      "primitives": [
        "PRIM-BronzeRecord",
        "PRIM-SilverRecord",
        "PRIM-EmbeddingRecord",
        "PRIM-ProcessingRecord",
        "PRIM-ValidationRecord",
        "PRIM-EmbeddingArtifact",
        "PRIM-VectorIndexArtifact",
        "PRIM-KeywordIndexArtifact",
        "PRIM-GraphArtifact",
        "PRIM-EmbeddingModelStatus",
        "PRIM-ValidationStatus"
      ],
      "tools_tech": [
        "TOOL-TREE-SITTER",
        "TECH-BM25",
        "TECH-HNSW",
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "Bronzeâ†’Silverâ†’Gold pipeline with hybrid retrieval and explicit metadata for LLM-friendly data. [ADD v02.146] Index artifacts, embedding lifecycle status, and validation status are first-class contracts for deterministic retrieval/runtime reasoning. [ADD v02.151] Index update/rebuild events and persisted embedding/vector/keyword/graph artifacts are canonical backend evidence and portability surfaces; debug-bundle export of those artifacts remains stub-backed until explicit exporter scope rules land. [ADD v02.156] Project Brain retrieval MUST reuse AI-Ready Data hybrid retrieval substrates through explicit QueryPlan and RetrievalTrace contracts instead of inventing a second notebook-only retrieval path. [ADD v02.178] AI-Ready Data is the retrieval substrate, not a mandate to vector-search everything: exact authoritative loads and bounded graph traversal MAY bypass hybrid retrieval, and `QueryPlan` plus `RetrievalTrace` MUST record when no-RAG or lower-cost retrieval modes were chosen."
    },
    {
      "feature_id": "FEAT-ASR",
      "title": "Speech Recognition (ASR)",
      "spec_anchor": "#62-speech-recognition-asr-subsystem",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-MediaSource"
      ],
      "tools_tech": [
        "TOOL-FFMPEG",
        "TOOL-FFPROBE",
        "TECH-WHISPER"
      ],
      "notes": "[ADD v02.144] ASR is a governed transcription surface that turns media into timestamped transcript artifacts for Loom, retrieval, and operator evidence. [ADD v02.158] Transcript payloads, source-media hashes, ffprobe-derived media facts, timing anchors, and bounded failure/progress events are canonical backend portability and Flight Recorder seams instead of UI-only output.",
      "capability_slice_ids": [
        "CAP-014"
      ],
      "runtime_visibility_ids": [
        "RV-014"
      ]
    },
    {
      "feature_id": "FEAT-ATELIER-LENS",
      "title": "Atelier/Lens Runtime",
      "spec_anchor": "Â§6.3.3.5",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-SelectionRange",
        "PRIM-RoleSuggestionV1",
        "PRIM-RoleSuggestionsResponseV1"
      ],
      "tools_tech": [
        "TOOL-COMFYUI",
        "TECH-JSON"
      ],
      "notes": "Always-on Lens claim/glance + governed creative extraction/output with replay and validators. [ADD v02.144] Lens suggestions and collaborator-role hints need deterministic runtime coverage instead of remaining implicit UI behavior. [ADD v02.158] Transcript/time-span and captured-media backend lineage remain stub-backed until explicit Stage/ASR/Lens artifact contracts land."
    },
    {
      "feature_id": "FEAT-CALENDAR",
      "title": "Calendar",
      "spec_anchor": "#104-calendar",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-CalendarMutation",
        "PRIM-CalendarSyncInput",
        "PRIM-CalendarSource",
        "PRIM-CalendarEvent",
        "PRIM-CalendarEventWindowQuery",
        "PRIM-CalendarSourceSyncState",
        "PRIM-CalendarSourceWritePolicy",
        "PRIM-CalendarEventExportMode"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.142] Calendar is both a user calendar surface and a runtime time-window lens for Flight Recorder/Locus correlation and governed local mutations. [ADD v02.150] Calendar event windows, event rows, and source sync state are canonical backend correlation anchors for bounded exports and time-scoped job/activity joins. [ADD v02.151] Calendar-to-Mailbox correlation remains a stub-backed backend follow-on until event-window to thread/message bridge contracts are made explicit. [ADD v02.155] Calendar source sync state, write policy, export mode, capability profiles, and ACE scope hints are first-class backend contracts for storage portability, consent posture, AI-job mutation discipline, and deterministic routing.",
      "capability_slice_ids": [
        "CAP-001",
        "CAP-002",
        "CAP-024"
      ],
      "runtime_visibility_ids": [
        "RV-001",
        "RV-002",
        "RV-024"
      ]
    },
    {
      "feature_id": "FEAT-CANVAS",
      "title": "Freeform Canvas",
      "spec_anchor": "#712-freeform-canvas-milanote-like",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-CanvasView",
        "PRIM-ExcalidrawCanvas"
      ],
      "tools_tech": [
        "TECH-EXCALIDRAW",
        "TECH-JSON"
      ],
      "notes": "[ADD v02.144] Canvas is a first-class spatial reasoning surface that must remain explicit for Thinking Pipeline and Studio runtime flows.",
      "capability_slice_ids": [
        "CAP-009"
      ],
      "runtime_visibility_ids": [
        "RV-009"
      ]
    },
    {
      "feature_id": "FEAT-CAPABILITIES-CONSENT",
      "title": "Capabilities & Consent Model",
      "spec_anchor": "#111-capabilities-consent-model",
      "surfaces": [
        "backend",
        "gov",
        "ui"
      ],
      "primitives": [
        "PRIM-CapabilityRegistry",
        "PRIM-CapabilityRegistryEntry",
        "PRIM-CapabilityKind",
        "PRIM-ConsentDecision",
        "PRIM-RiskClass",
        "PRIM-CapabilityProfile",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-CapabilitySnapshotCapabilityV1",
        "PRIM-CapabilitySnapshotToolV1",
        "PRIM-ConsentProvider",
        "PRIM-ConsentScopeV0_4",
        "PRIM-CloudEscalationGuard"
      ],
      "tools_tech": [
        "TECH-JSON"
      ],
      "notes": "Default-deny capability system with explicit consent decisions and auditability. [ADD v02.147] Capability profiles, emitted capability snapshots, scoped consent, and the cloud escalation guard are first-class runtime enforcement contracts, not hidden glue. [ADD v02.150] Capability and consent decisions must remain exportable and debuggable through bounded bundle scope instead of living only in transient projection surfaces. [ADD v02.153] Capability checks and emitted capability-action events are canonical Flight Recorder-visible audit evidence; portability beyond current bounded bundle scope remains stub-backed until consent receipt manifest/hash rules are explicit."
    },
    {
      "feature_id": "FEAT-CHARTS-DASHBOARDS",
      "title": "Charts & Dashboards",
      "spec_anchor": "#107-charts--dashboards",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-ChartSpec"
      ],
      "tools_tech": [
        "TECH-ECHARTS",
        "TECH-JSON"
      ],
      "notes": "[ADD v02.144] Chart composition is a job-backed analysis surface that should compose with Docs, Decks, DCC, and operator telemetry.",
      "capability_slice_ids": [
        "CAP-015"
      ],
      "runtime_visibility_ids": [
        "RV-015"
      ]
    },
    {
      "feature_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "title": "Cloud Escalation Consent",
      "spec_anchor": "Â§11.1.7",
      "surfaces": [
        "backend",
        "gov",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-CloudEscalationPolicy",
        "PRIM-ConsentReceiptV0_4",
        "PRIM-ProjectionPlanV0_4",
        "PRIM-RuntimeGovernanceMode",
        "PRIM-ConsentScopeV0_4",
        "PRIM-CloudEscalationGuard",
        "PRIM-CloudEscalationUiSurface"
      ],
      "tools_tech": [
        "TECH-JSON"
      ],
      "notes": "[ADD v02.145] Cloud escalation consent is a first-class governance/runtime surface spanning policy selection, projection planning, consent receipts, and operator-visible evidence. [ADD v02.147] Consent scope, guard enforcement, and the UI projection surface are explicit runtime contracts so consent review remains deterministic. [ADD v02.150] Projection plans and consent receipts are canonical audit/export inputs and must retain stable meaning when materialized into bundle evidence. [ADD v02.153] Consent receipts and cloud escalation request artifacts are canonical backend portability inputs, but cross-surface manifest/hash/retention semantics remain stub-backed until a dedicated portability bridge lands.",
      "capability_slice_ids": [
        "CAP-021"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ]
    },
    {
      "feature_id": "FEAT-CONTEXT-PACKS",
      "title": "Context Packs",
      "spec_anchor": "#2512-context-packs-ai-job-profile",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-ContextPackAnchorV1",
        "PRIM-ContextPackCoverageV1",
        "PRIM-ContextPackFreshnessDecision",
        "PRIM-ContextPackFreshnessGuard",
        "PRIM-ContextPackFreshnessPolicyV1",
        "PRIM-ContextPackPayloadV1",
        "PRIM-ContextPackRecord"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.144] Context Packs are the mechanical compaction substrate that makes retrieval/runtime context reusable and explainable. [ADD v02.156] Context Packs are portable retrieval artifacts: anchors, coverage, freshness guards, payload schema, and canonical artifact serialization MUST preserve stable meaning across storage backends and later replay/export flows. [ADD v02.157] Freshness policy/decision, build/reuse hashes, and recorder-visible build/select/refresh outcomes are first-class backend contracts for distillation, replay, and deterministic model onboarding.",
      "capability_slice_ids": [
        "CAP-017"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ]
    },
    {
      "feature_id": "FEAT-DEBUG-BUNDLE",
      "title": "Debug Bundle Export",
      "spec_anchor": "#114-diagnostics-schema-problemsevents",
      "surfaces": [
        "backend",
        "operator_consoles",
        "ui"
      ],
      "primitives": [
        "PRIM-DebugBundleExporter",
        "PRIM-BundleScope",
        "PRIM-RedactionMode",
        "PRIM-BundleExportError",
        "PRIM-BundleManifest",
        "PRIM-BundleValidationReport",
        "PRIM-DebugBundleRequest",
        "PRIM-ArtifactManifest",
        "PRIM-BundleIndexEntry",
        "PRIM-PolicyDecision",
        "PRIM-PolicyDecisionOutcome",
        "PRIM-ExportScope",
        "PRIM-BundleStatus",
        "PRIM-ExportResponse",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-ExportRequest",
        "PRIM-ExportTarget",
        "PRIM-ExportableInventory",
        "PRIM-RetentionReport",
        "PRIM-EventFilter",
        "PRIM-FlightEvent",
        "PRIM-WorkflowRun",
        "PRIM-WorkflowNodeExecution",
        "PRIM-CalendarEventWindowQuery"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-ZIP"
      ],
      "notes": "Deterministic debug bundle export with redaction modes and manifest hashing. [ADD v02.147] Manifest, validation, status, policy-decision, and export request/response contracts are first-class runtime surfaces instead of buried exporter internals. [ADD v02.148] Export request/target/inventory and retention-report contracts are part of the canonical debug-bundle recovery surface, not hidden exporter details. [ADD v02.150] Workflow runs, node executions, recorder filters/events, and calendar event windows are valid bounded export anchors for deterministic backend evidence correlation. [ADD v02.151] Role Mailbox and AI-ready index evidence remain explicit stub-backed bridge tracks until dedicated bundle scope contracts are defined."
    },
    {
      "feature_id": "FEAT-DEV-COMMAND-CENTER",
      "title": "Dev Command Center (Sidecar Integration)",
      "spec_anchor": "#1011-dev-command-center-sidecar-integration",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-WorkPacketBinding",
        "PRIM-PinnedSlice",
        "PRIM-SessionChatLogEntryV0_1",
        "PRIM-GovernancePackExport",
        "PRIM-GovernancePackExportRequest",
        "PRIM-GovernancePackExportResponse",
        "PRIM-GovernancePackExportOutcome",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-BundleManifest",
        "PRIM-BundleStatus",
        "PRIM-DiagnosticsQuery",
        "PRIM-ExportRecord",
        "PRIM-ModelSession",
        "PRIM-MultiModelSession",
        "PRIM-TrackedWorkPacket",
        "PRIM-MicroTaskSummary",
        "PRIM-SpecSessionLogEntry",
        "PRIM-TaskBoardEntry",
        "PRIM-TaskBoardStatus",
        "PRIM-LocusQueryReadyParams",
        "PRIM-LocusGetWpStatusParams",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-SessionRegistry",
        "PRIM-SessionCheckpoint",
        "PRIM-ProviderCapabilities",
        "PRIM-ModelSessionSpanBinding",
        "PRIM-AntiPatternAlert",
        "PRIM-RepositoryEngineDecisionSurface",
        "PRIM-WorkflowRun",
        "PRIM-WorkflowNodeExecution",
        "PRIM-JobStatusUpdate",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-GovernanceDecision",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimMode",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1",
        "PRIM-DevCommandCenterLayoutKind",
        "PRIM-ProjectionActionBindingV1",
        "PRIM-DevCommandCenterViewPresetV1",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimMode",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1"
      ],
      "tools_tech": [
        "TOOL-GIT",
        "TECH-REACT",
        "TECH-TAURI"
      ],
      "notes": "Operator surface to run governed development workflows, map work packets to worktrees and sessions, and review diffs. [ADD v02.144] Dev Command Center is the runtime projection surface for jobs, problems, timeline slices, tool calls, and recovery evidence. [ADD v02.147] Dev Command Center should round-trip export responses and capability and approval projection state instead of treating export actions as fire-and-forget. [ADD v02.159] Dev Command Center is the control and projection umbrella for Flight Recorder, bounded bundle launches, Role Mailbox projection, and worktree or session steering; Operator Consoles remain the specialized evidence and diagnostics views inside that umbrella. [ADD v02.160] Dev Command Center is also the governed control-plane projection for workflow runs, artificial intelligence job state, model-session scheduler state, effective capability snapshots, approval decisions, and work packet or worktree bindings; those backend artifacts MAY NOT remain hidden behind drawer-local state or console-only summaries. [ADD v02.161] Dev Command Center is also the governed evidence and replay projection for Governance Pack exports, Workspace Bundle exports, diagnostics query state, and bounded Debug Bundle validation outcomes; those export lifecycles MAY NOT live only inside console-local polling state or toast notifications. [ADD v02.162] Dev Command Center is also the governed work-orchestration projection for tracked Work Packet status, Task Board sync freshness, ready-query results, Micro-Task summaries, workflow-linked work packet activation, and parallel model session occupancy; those backend artifacts MAY NOT remain hidden behind kanban-only summaries, drawer-local routing state, or session-local heuristics. [ADD v02.163] Dev Command Center is also the governed planning-and-coordination projection for Task Board entries, Work Packet bindings, Spec Session Log continuity, and ready-work planning keyed by stable work-packet, workflow-run, micro-task, and model-session identifiers; those backend artifacts MAY NOT be reconstructed only from kanban ordering or packet-local prose. [ADD v02.164] Dev Command Center is also the governed resilience and repository-decision projection surface for session checkpoints, heartbeat freshness, provider capability readiness, anti-pattern alerts, and declared repository-engine backend policy including required status checks and merge-queue compatibility; those backend artifacts MAY NOT be guessed from console-local badges, adapter logs, or ad hoc git output. [ADD v02.165] Dev Command Center is also the governed operating surface for replay-safe run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots; those backend artifacts MAY NOT be inferred from visible transcript order, tool-list caches, workspace tabs, or optimistic repository badges. [ADD v02.166] Dev Command Center is also the structured record and collaboration-inbox viewing surface for Work Packet records, Micro-Task execution contracts, Task Board projection rows, append-only note timelines, and Role Mailbox triage state; typed fields and authoritative structured artifacts MUST remain primary over Markdown mirrors or transcript-only summaries. [ADD v02.167] Dev Command Center board, queue, roadmap, and Jira-like layouts are derived views over canonical structured collaboration artifacts, and operators MUST be able to inspect the exact authoritative fields and mirror-sync status behind every layout decision. [ADD v02.168] Dev Command Center also depends on a shared structured-collaboration base envelope, compact summary contract, and explicit profile-extension boundary so generic viewers and future project kernels can render the same records without software-delivery assumptions leaking into every view. [ADD v02.169] Dev Command Center is also the canonical mirror-reconciliation surface for authority mode, drift summary, regeneration action, and normalization posture; readable Markdown is never assumed safe when canonical or summary state disagrees. [ADD v02.170] Dev Command Center typed viewers also depend on explicit view presets, governed action bindings, and compact-summary-first execution or inbox queues so board, roadmap, triage, and local-small-model operating layouts can evolve without inventing hidden state transitions. [ADD v02.171] Dev Command Center also depends on one shared workflow-state family, queue-reason vocabulary, governed action descriptor set, and project-profile workflow label override contract so routing and action eligibility never collapse back into lane position or prose. [ADD v02.172] Dev Command Center also depends on explicit workflow transition rules, queue automation rules, and executor eligibility policies so every retry, reroute, review request, approval request, and automatic queue move can be previewed before authoritative state changes occur. [ADD v02.173] Dev Command Center also depends on typed Role Mailbox thread lifecycle, message delivery state, allowed-response envelopes, and mailbox-local-versus-governed action previews so inbox triage and Micro-Task collaboration loops remain queryable without transcript-order heuristics. [ADD v02.174] Dev Command Center also depends on mailbox-linked loop checkpoints, structured verifier outcomes, remaining retry budget, escalation targets, and completion-report transcription state so operator triage can resume, escalate, or close Micro-Task loops without reading full thread histories. [ADD v02.175] Dev Command Center also depends on mailbox triage queue state, reminder schedules, queue age, snooze or expiry posture, dead-letter disposition, and explicit operator remediation controls so queue aging, inbox pressure, and recovery actions remain inspectable before linked work changes. [ADD v02.176] Dev Command Center now also depends on mailbox executor-kind routing, claim mode, current claimant, lease expiry, takeover policy, and response-authority previews so operators can see who may act, who currently owns the thread, and whether takeover is legal before any reply or reroute occurs. [ADD v02.177] Dev Command Center now also depends on mailbox handoff bundle fields, announce-back provenance kind, recommended next actor, and transcription status so operators can resume, review, or close collaboration from compact structured state instead of replaying full threads. [ADD v02.181] Dev Command Center now also projects software-delivery governance overlay runtime truth, validator-gate summaries, governed-action resolution state, derived closeout posture, overlay claim/lease posture, queued steering/follow-up state, and workflow-backed control-plane health/backpressure so start/steer/cancel/close/recover decisions remain grounded in product-owned runtime records; imported repo `/.GOV/**`, packet prose, and mailbox chronology remain evidence or mirror inputs and MAY NOT become the authoritative control surface.",
      "capability_slice_ids": [
        "CAP-018"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ]
    },
    {
      "feature_id": "FEAT-DIAGNOSTICS-SCHEMA",
      "title": "Diagnostics Schema (Problems/Events)",
      "spec_anchor": "#114-diagnostics-schema-problemsevents",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-FindingSeverity",
        "PRIM-ValidationFinding",
        "PRIM-DiagnosticsQuery",
        "PRIM-DiagFilter",
        "PRIM-DiagnosticStatus",
        "PRIM-ProblemGroup"
      ],
      "tools_tech": [
        "TECH-JSON"
      ],
      "notes": "Canonical problems/events schema used for evidence-first debugging and gating. [ADD v02.153] Diagnostics queries, grouped problems, and validation findings are canonical backend export inputs for bounded debug-bundle materialization, not operator-only list views. [ADD v02.161] Diagnostics query state is also a Dev Command Center evidence-and-replay seam and MUST remain projectable by stable query and evidence identifiers rather than console-local filter state."
    },
    {
      "feature_id": "FEAT-DOCS-SHEETS",
      "title": "Docs & Sheets",
      "spec_anchor": "#2510-docs--sheets-ai-job-profile",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-DocsAiJobProfile",
        "PRIM-DocumentView",
        "PRIM-TiptapEditor"
      ],
      "tools_tech": [
        "TECH-BLOCKNOTE",
        "TECH-HYPERFORMULA",
        "TECH-TIPTAP",
        "TECH-WOLF-TABLE"
      ],
      "notes": "[ADD v02.144] Docs and Sheets are job-backed structured editing surfaces, not passive UI shells. [ADD v02.146] The concrete editor surface must remain explicit so future models do not treat Docs as an abstract job shell only.",
      "capability_slice_ids": [
        "CAP-008"
      ],
      "runtime_visibility_ids": [
        "RV-008"
      ]
    },
    {
      "feature_id": "FEAT-FLIGHT-RECORDER",
      "title": "Flight Recorder",
      "spec_anchor": "#115-flight-recorder-event-shapes-retention",
      "surfaces": [
        "backend",
        "operator_consoles",
        "ui"
      ],
      "primitives": [
        "PRIM-FlightRecorder",
        "PRIM-RecorderError",
        "PRIM-EventFilter",
        "PRIM-FlightEvent",
        "PRIM-FlightRecorderEventType"
      ],
      "tools_tech": [
        "TECH-DUCKDB",
        "TECH-JSON"
      ],
      "notes": "Append-only event log for prompts, tool calls, jobs, and governance events with retention. [ADD v02.146] The timeline/filter/deep-link UI is a first-class projection of the recorder contract, not a derived afterthought. [ADD v02.151] Mailbox create/transcribe/export actions and AI-ready index lifecycle changes are canonical backend evidence seams for correlation and later bounded export. [ADD v02.152] Spec Router routing artifacts, Locus event families, and MCP/MEX tool-call evidence are canonical backend seams for projection, replay, and bounded export. [ADD v02.153] Capability-action allow/deny events and MCP tool/result/progress events are first-class recorder correlation seams for policy audit and later bundle materialization."
    },
    {
      "feature_id": "FEAT-GOVERNANCE-PACK",
      "title": "Governance Pack Export",
      "spec_anchor": "#7548-governance-pack-project-specific-instantiation-hard",
      "surfaces": [
        "gov",
        "backend",
        "operator_consoles",
        "ui"
      ],
      "primitives": [
        "PRIM-GovernancePackExport",
        "PRIM-GovernancePackExportRequest",
        "PRIM-GovernancePackExportResponse",
        "PRIM-GovernancePackExportOutcome",
        "PRIM-ExportRecord",
        "PRIM-ExportTarget",
        "PRIM-ArtifactManifest"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.154] Governance Pack export is a governed backend workflow that resolves project invariants, materializes template-volume artifacts, emits dedicated Flight Recorder evidence, and persists export records plus portable manifests for later audit and transfer. [ADD v02.161] Governance Pack export lifecycle and outcomes are also canonical Dev Command Center evidence-and-replay projection state, not operator-console-only export history. [ADD v02.181] Governance Pack import and export also define the software-delivery governance overlay transfer boundary: imported repo-governance artifacts are source material and portable evidence, but live validator-gate, workflow, closeout, claim/lease, queued-instruction, and control-plane authority remain product-owned runtime state.",
      "capability_slice_ids": [
        "CAP-023"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ]
    },
    {
      "feature_id": "FEAT-LOCUS-WORK-TRACKING",
      "title": "Locus Work Tracking System",
      "spec_anchor": "#2315-locus-work-tracking-system-add-v02116",
      "surfaces": [
        "gov",
        "backend",
        "ui"
      ],
      "primitives": [
        "PRIM-TrackedWorkPacket",
        "PRIM-TrackedMicroTask",
        "PRIM-TrackedDependency",
        "PRIM-MicroTaskStatus",
        "PRIM-MicroTaskValidationResult",
        "PRIM-MicroTaskIterationRecord",
        "PRIM-LocusQueryReadyParams",
        "PRIM-LocusGetWpStatusParams",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-MicroTaskSummary",
        "PRIM-TaskBoardStatus",
        "PRIM-GateStatuses",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1",
        "PRIM-DevCommandCenterLayoutKind",
        "PRIM-ProjectionActionBindingV1",
        "PRIM-DevCommandCenterViewPresetV1"
      ],
      "tools_tech": [
        "TECH-CRDT",
        "TECH-JSON",
        "TECH-POSTGRESQL",
        "TECH-SQLITE",
        "TECH-WEBSOCKET"
      ],
      "notes": "End-to-end tracking for Work Packets to Micro-Tasks with dependencies, sync, and event sourcing. [ADD v02.146] Status/query/result contracts must remain explicit so work tracking can be consumed deterministically by operator and model runtimes. [ADD v02.150] Workflow/node execution and task-board sync are backend projection contracts that keep ready-query, progress, and export scopes deterministic. [ADD v02.152] Locus Work Packet, Micro-Task, dependency, task-board, and query events are canonical Flight Recorder evidence seams; dedicated debug-bundle bridging remains stub-backed until explicit bundle-scope contracts land. [ADD v02.166] Locus canonical work-tracking state is structured-record-first for Work Packets, Micro-Tasks, dependencies, and Task Board projection rows; Markdown mirrors and long-form notes remain valuable but MUST stay derived, linked, and non-authoritative for routing and ready-work decisions. [ADD v02.167] Locus canonical collaboration records SHOULD use versioned JavaScript Object Notation envelopes with stable identifiers, compact summaries, and project-agnostic base fields so later project kernels do not inherit repository-specific mandatory fields. [ADD v02.168] Locus is also the shared schema-enforcement surface for the structured collaboration envelope, compact summary joins, and project-profile extension boundaries across Work Packets, Micro-Tasks, and Task Board rows. [ADD v02.169] Locus is also the desired-state reconciliation surface for Markdown mirrors and readable board projections; drift updates change mirror status and reconciliation action instead of mutating canonical packet state through prose. [ADD v02.171] Locus is also the authority for base `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` values so queues and future project kernels can relabel work without forking the underlying routing contract. [ADD v02.172] Locus is also the authority for portable workflow transition rules, automatic queue-move rules, and executor eligibility policies so queue grouping, retries, escalation, review, and approval are driven from durable backend law rather than board gestures or mailbox order. [ADD v02.173] Locus is also the authority that mailbox threads point at when a collaboration request might change linked work; thread lifecycle, due posture, and allowed responses may live in Role Mailbox, but authoritative packet or task state changes still resolve through Locus-backed governed actions or transcriptions. [ADD v02.174] Locus now also joins mailbox loop checkpoints, structured verifier outcomes, remaining retry budget, and completion-report transcription posture to authoritative Micro-Task and Work Packet state so waiting, retrying, escalated, and complete posture remains queryable without thread replay. [ADD v02.175] Locus now also joins mailbox triage queue state, reminder schedules, queue age, expiry posture, and dead-letter disposition to linked Work Packet, Micro-Task, and Task Board projections so mailbox pressure and remediation state remain queryable without relying on unread badges or thread order. [ADD v02.176] Locus now also joins mailbox executor kind, claim mode, current claimant, lease expiry, takeover policy, and response-authority scope to linked Work Packet, Micro-Task, and Task Board projections so temporary mailbox ownership stays queryable without implying that mailbox claims outrank governed work authority. [ADD v02.177] Locus now also joins latest accepted mailbox handoff bundle, announce-back provenance, and transcription-status summaries to linked Work Packet and Micro-Task records so takeover, resume, and review can rely on authoritative joins instead of mailbox chronology. [ADD v02.181] Locus now also owns authoritative software-delivery governance overlay joins for governed actions, validator-gate summaries, derived closeout inputs, overlay claim/lease state, queued steering/follow-up state, and checkpoint-backed recovery posture; imported repo-governance artifacts, Task Board mirrors, and mailbox chronology remain evidence or projection inputs rather than runtime authority.",
      "capability_slice_ids": [
        "CAP-004"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ]
    },
    {
      "feature_id": "FEAT-LOOM-LIBRARY",
      "title": "Loom (Library Surface)",
      "spec_anchor": "#1012-loom-heaper-style-library-surface-add-v02130",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-LoomBlockContentType",
        "PRIM-LoomEdgeType",
        "PRIM-PreviewStatus",
        "PRIM-AssetKind",
        "PRIM-LoomBlock",
        "PRIM-LoomEdge",
        "PRIM-LoomBlockDerived",
        "PRIM-LoomViewFilters",
        "PRIM-LoomSearchFilters",
        "PRIM-LoomBlockSearchResult",
        "PRIM-LoomSourceAnchor"
      ],
      "tools_tech": [
        "TECH-FTS5",
        "TECH-JSON",
        "TECH-SHA256",
        "TECH-SQLITE"
      ],
      "notes": "Import + organize LoomBlocks via views and inline relational tokens (mentions/tags) backed by Knowledge Graph. [ADD v02.146] The persisted Loom block-edge graph is part of the canonical library contract, not just the search/view shell. [ADD v02.156] Loom block-edge records, search/view filters, and source anchors are portable backend library artifacts that must survive storage swaps, export, and replay without semantic drift. [ADD v02.178] Loom tags, mentions, backlinks, pins, and unlinked state are also retrieval-shaping graph signals for Project Brain and Prompt-to-Spec, but exact LoomBlock or asset requests MUST resolve by direct load before semantic search is attempted.",
      "capability_slice_ids": [
        "CAP-005"
      ],
      "runtime_visibility_ids": [
        "RV-005"
      ]
    },
    {
      "feature_id": "FEAT-MAIL-CLIENT",
      "title": "Mail Client",
      "spec_anchor": "#103-mail-client",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-MailMessage",
        "PRIM-MailThread"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "User-facing mail surface; integrates into unified indexing and retrieval. [ADD v02.144] Mail is a governed ingestion, drafting, and event-correlation surface that must remain visible to retrieval, Calendar, Command Center, and operator evidence flows.",
      "capability_slice_ids": [
        "CAP-019"
      ],
      "runtime_visibility_ids": [
        "RV-019"
      ]
    },
    {
      "feature_id": "FEAT-MCP-PRIMITIVES",
      "title": "Auth/Session/MCP Primitives",
      "spec_anchor": "#113-authsessionmcp-primitives",
      "surfaces": [
        "backend"
      ],
      "primitives": [
        "PRIM-McpClient",
        "PRIM-ConsentDecision",
        "PRIM-AccessMode",
        "PRIM-GateConfig",
        "PRIM-McpCall",
        "PRIM-ToolRegistryEntry",
        "PRIM-JsonRpcRequest",
        "PRIM-JsonRpcResponse",
        "PRIM-ToolCallEvent"
      ],
      "tools_tech": [
        "TOOL-MCP",
        "TECH-JSON",
        "TECH-MCP"
      ],
      "notes": "MCP traffic is gated and logged through the same capability and Flight Recorder paths as internal tools. [ADD v02.152] Redacted MCP args/results, JSON-RPC envelopes, and gated tool-call evidence are canonical backend export/projection seams; dedicated debug-bundle export remains stub-backed until MCP evidence-scope contracts are explicit. [ADD v02.153] MCP tool call/result/progress envelopes are direct Flight Recorder-visible correlation seams and MAY NOT remain implied only by the bundle exporter."
    },
    {
      "feature_id": "FEAT-MEDIA-DOWNLOADER",
      "title": "Media Downloader",
      "spec_anchor": "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-MdSessionRecordV0",
        "PRIM-MdSessionsRegistryV0",
        "PRIM-MdAuthMode",
        "PRIM-MediaSource"
      ],
      "tools_tech": [
        "TOOL-FFMPEG",
        "TOOL-YTDLP",
        "TECH-JSON",
        "TECH-SHA256",
        "TOOL-FFPROBE"
      ],
      "notes": "Unified web media archiving surface; job-driven, capability-gated, and artifact/materialization aware. [ADD v02.144] Media capture sessions and source objects are first-class runtime inputs for Stage, ASR, Loom, and retrieval. [ADD v02.148] Auth mode and session registry contracts are shared with Stage-backed capture flows so download/auth reuse remains explicit. [ADD v02.150] Media downloader outputs, auth state, and captured session records are valid bounded debug/export anchors and must preserve portable artifact semantics. [ADD v02.158] Downloaded media artifacts are canonical ASR inputs and MUST preserve source hashes, media-probe facts, and transcript lineage contracts for later replay and retrieval."
    },
    {
      "feature_id": "FEAT-MEX-RUNTIME",
      "title": "Mechanical Extension Runtime",
      "spec_anchor": "Â§11.8",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-EngineResult",
        "PRIM-MexRegistry",
        "PRIM-MexRuntimeError"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-JSON-RPC"
      ],
      "notes": "[ADD v02.145] MEX runtime is the governed mechanical execution surface whose adapter results, safety gates, and runtime envelopes must stay visible to jobs, tool routing, and operator evidence. [ADD v02.152] Redacted tool args/results, denial diagnostics, capability actions, and gate outcomes are canonical backend evidence seams; bounded debug-bundle export remains shared with MCP via a stub-backed bridge until explicit evidence-scope contracts land.",
      "capability_slice_ids": [
        "CAP-022"
      ],
      "runtime_visibility_ids": [
        "RV-022"
      ]
    },
    {
      "feature_id": "FEAT-MICRO-TASK-EXECUTOR",
      "title": "Micro-Task Executor Profile",
      "spec_anchor": "#2668-micro-task-executor-profile",
      "surfaces": [
        "backend",
        "gov"
      ],
      "primitives": [
        "PRIM-MicroTaskDefinition",
        "PRIM-MicroTaskExecutorJob",
        "PRIM-MicroTaskLoopCheckpointV1",
        "PRIM-MicroTaskMetrics",
        "PRIM-MicroTaskVerifierOutcomeV1",
        "PRIM-PendingDistillationCandidate",
        "PRIM-TaskOutcome",
        "PRIM-TaskState",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tools_tech": [
        "TOOL-OLLAMA-API",
        "TECH-JSON",
        "TECH-OLLAMA"
      ],
      "notes": "Auto-generated Micro-Task decomposition plus iterative execution loop with escalation and validation reporting. [ADD v02.157] Pending distillation candidate queues, escalation-derived teacher/student lineage, and trust-tagged candidate evidence are part of the canonical backend learning surface, not optional helper state. [ADD v02.162] Micro-Task progress, hard-gate state, and session-occupancy bindings are canonical Dev Command Center work-orchestration projection inputs and MAY NOT live only in loop-local state, ad hoc progress polling, or isolated test counters. [ADD v02.166] Micro-Task definitions are also bounded structured execution contracts for local small models, with explicit execution tier, allowed tools, retry budget, escalation target, and expected output so delegation and handoff do not depend on packet-length Markdown ingestion. [ADD v02.167] Micro-Task canonical artifacts SHOULD support `packet.json` plus bounded `summary.json` outputs so local small models can read execution-ready fields without loading the full long-form reasoning trail. [ADD v02.168] Micro-Task records also inherit the shared base envelope and explicit profile-extension boundary so domain-specific execution hints do not pollute the generic local-small-model routing contract. [ADD v02.169] Micro-Task readable mirrors remain secondary to canonical execution contracts and summaries; advisory edits or long-form reasoning belong in note sidecars rather than backdoor task-state mutation. [ADD v02.170] Micro-Task execution queues SHOULD project explicit readiness buckets for local-small-model execution, human blockers, escalation, validation, and mailbox-response dependencies rather than relying on heuristic board placement. [ADD v02.171] Micro-Task routing SHOULD also declare one base workflow-state family, one queue reason code, and the currently allowed governed actions so local-small-model execution does not depend on board labels or mailbox ordering. [ADD v02.172] Micro-Task execution also depends on explicit transition rules, queue automation triggers, and executor eligibility policies so retry, escalation, validation, and mailbox-response waits do not silently change who may act next. [ADD v02.173] Micro-Task loops also depend on typed mailbox message families for request, feedback, verification, escalation, and completion reporting so Ralph-style verifier loops and bounded handoffs remain queryable without overloading generic blocker or status traffic. [ADD v02.174] Micro-Task loops now also publish bounded mailbox-loop checkpoints, structured verifier outcomes, remaining retry budget, and escalation posture so Role Mailbox, Work Packet notes, Task Board waits, and Dev Command Center queues can resume or audit loops without transcript replay. [ADD v02.176] Mailbox-linked Micro-Task loops now also surface current claimant, claimant kind, exclusive-versus-shared claim posture, lease expiry, and response-authority scope so execution queues can distinguish work that is truly free to resume from work that is still temporarily owned by another actor. [ADD v02.177] Mailbox-linked Micro-Task loops now also expose compact handoff bundle, announce-back provenance, and last transcription status so a resumed executor can see the safe next action without replaying verifier chatter. [ADD v02.178] Micro-Task loops default to `none`, `direct_load`, or `exact_lookup` retrieval modes and may use hybrid retrieval only for bounded discovery subtasks that are compacted before a local small model sees them."
    },
    {
      "feature_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "title": "Model Session Orchestration",
      "spec_anchor": "Â§4.3.9.12",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-ModelSession",
        "PRIM-MultiModelSession",
        "PRIM-ProviderCapabilities",
        "PRIM-ModelSessionSpanBinding",
        "PRIM-ActivitySpanBinding",
        "PRIM-SessionCheckpoint",
        "PRIM-SessionRegistry",
        "PRIM-SessionSchedulerConfig"
      ],
      "tools_tech": [
        "TOOL-OLLAMA-API",
        "TOOL-OPENAI-COMPAT-API",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.145] Model Session orchestration is the runtime contract for queued model calls, session lifecycles, provider resolution, and safe multi-session steering. [ADD v02.148] MultiModelSession is the explicit runtime substrate for concurrent session steering, snapshotting, and handoff-safe orchestration. [ADD v02.162] Parallel model session steering MUST remain explicitly bound to tracked Work Packets, Micro-Task occupancy, ready-work selection, and workflow-linked routing rather than free-floating queue state. [ADD v02.164] Provider capability coverage, ModelSession span binding, checkpoint recovery state, and anti-pattern evidence are also canonical backend projection inputs for Dev Command Center recovery and governed reroute decisions. [ADD v02.165] Queue-state history, replay eligibility, and checkpoint chronology are also canonical backend projection inputs for Dev Command Center run-history and replay views.",
      "capability_slice_ids": [
        "CAP-020"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ]
    },
    {
      "feature_id": "FEAT-MONACO-EDITOR",
      "title": "Monaco Editor Experience",
      "spec_anchor": "#102-monaco-editor-experience",
      "surfaces": [
        "ui"
      ],
      "primitives": [
        "PRIM-SelectionRange"
      ],
      "tools_tech": [
        "TECH-MONACO"
      ],
      "notes": "Code editor surface with capability-gated actions and patchset/diff review patterns."
    },
    {
      "feature_id": "FEAT-OPERATOR-CONSOLES",
      "title": "Operator Consoles: Debug & Diagnostics",
      "spec_anchor": "#105-operator-consoles-debug-diagnostics",
      "surfaces": [
        "ui"
      ],
      "primitives": [
        "PRIM-DebugBundleExporter",
        "PRIM-BundleScope",
        "PRIM-RedactionMode",
        "PRIM-SessionChatLogEntryV0_1",
        "PRIM-DiagnosticsQuery",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-GovernancePackExportResponse"
      ],
      "tools_tech": [
        "TECH-REACT",
        "TECH-TAURI"
      ],
      "notes": "Evidence-first debugging surfaces (Problems, Jobs, Timeline, and Evidence) with deep links into traces and bundles. [ADD v02.147] Operator Consoles own the query and export response envelopes that drive Problems, Jobs, Timeline, and Evidence flows. [ADD v02.159] Operator Consoles are the specialized evidence and diagnostics cluster inside the Dev Command Center umbrella; they own drilldown, evidence selection, and export-launch behavior rather than global control and orchestration state. [ADD v02.161] Operator Consoles continue to launch governance export, workspace export, diagnostics, and bounded bundle actions, while Dev Command Center owns the durable umbrella projection of those governed evidence flows."
    },
    {
      "feature_id": "FEAT-PHOTO-STUDIO",
      "title": "Photo Studio",
      "spec_anchor": "#1010-photo-studio",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-CanvasView",
        "PRIM-ExcalidrawCanvas"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TOOL-COMFYUI",
        "TECH-EXCALIDRAW"
      ],
      "notes": "Job-driven photo pipeline surface; exports follow the unified export contract. [ADD v02.144] Photo Studio is a Studio-adjacent runtime surface whose job graph and sidecar render path must stay explicit."
    },
    {
      "feature_id": "FEAT-PRESENTATIONS-DECKS",
      "title": "Presentations (Decks)",
      "spec_anchor": "#108-presentations-decks",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-DeckSpec"
      ],
      "tools_tech": [
        "TECH-PPTXGENJS",
        "TECH-REVEALJS",
        "TECH-JSON"
      ],
      "notes": "[ADD v02.144] Deck composition is a governed surface that should compose with Docs, Charts, Project Brain, and Studio workflows.",
      "capability_slice_ids": [
        "CAP-016"
      ],
      "runtime_visibility_ids": [
        "RV-016"
      ]
    },
    {
      "feature_id": "FEAT-PROJECT-BRAIN",
      "title": "Project Brain",
      "spec_anchor": "#258-project-brain-rag-interface",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-ContextPackPayloadV1",
        "PRIM-ContextPackRecord",
        "PRIM-QueryPlan",
        "PRIM-RetrievalTrace"
      ],
      "tools_tech": [
        "TECH-BM25",
        "TECH-FTS5",
        "TECH-HNSW",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.144] Project Brain is the project-scoped retrieval notebook surface over docs, canvas, tables, mail, events, and artifacts. [ADD v02.156] Project Brain is a governed backend retrieval contract over AI-Ready Data that must emit QueryPlan and RetrievalTrace and prefer fresh ContextPack reuse instead of introducing notebook-only retrieval semantics. [ADD v02.178] Project Brain also distinguishes discovery and synthesis from authoritative lookup, records non-hybrid reasons when exact ids or bounded graph neighborhoods suffice, and MUST NOT treat broad hybrid retrieval as the default for every question.",
      "capability_slice_ids": [
        "CAP-010"
      ],
      "runtime_visibility_ids": [
        "RV-010"
      ]
    },
    {
      "feature_id": "FEAT-ROLE-MAILBOX",
      "title": "Role Mailbox (Collab Inbox)",
      "spec_anchor": "Â§2.6.8.10",
      "surfaces": [
        "gov",
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-RoleMailboxContext",
        "PRIM-RoleMailboxThread",
        "PRIM-RoleMailboxMessage",
        "PRIM-RoleMailboxMessageType",
        "PRIM-RoleMailboxThreadLifecycleState",
        "PRIM-RoleMailboxMessageDeliveryState",
        "PRIM-RoleMailboxAllowedResponse",
        "PRIM-RoleMailboxActionRequestV1",
        "PRIM-RoleMailboxTriageQueueState",
        "PRIM-RoleMailboxReminderScheduleV1",
        "PRIM-RoleMailboxDeadLetterDisposition",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimMode",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1",
        "PRIM-MicroTaskLoopCheckpointV1",
        "PRIM-MicroTaskVerifierOutcomeV1",
        "PRIM-CreateRoleMailboxMessageRequest",
        "PRIM-AddTranscriptionLinkRequest",
        "PRIM-RoleMailboxExportSummary",
        "PRIM-RoleMailboxIndexV1",
        "PRIM-RoleMailboxThreadLineV1",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimMode",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tools_tech": [
        "TECH-JSON"
      ],
      "notes": "Non-authoritative coordination channel; messages are exportable and correlated but not binding. [ADD v02.146] Compose, transcription-link, and export-summary contracts must remain explicit for governed coordination and evidence export. [ADD v02.151] Mailbox message creation, transcription linkage, and repository export are backend evidence surfaces that emit Flight Recorder events and portable export manifests; dedicated debug-bundle bridge contracts remain stub-backed until exporter scope semantics are explicit. [ADD v02.159] Role Mailbox is also a backend projection input for the Dev Command Center and Operator Console drilldown surfaces, so message and export events cannot remain hidden in mailbox-only storage. [ADD v02.166] Role Mailbox is also the structured asynchronous collaboration substrate for delegate-work, blocker, review-request, decision-request, escalation, and announce-back flows; expected response, expiry, evidence references, and handoff completeness MUST remain queryable by structured fields. [ADD v02.167] Role Mailbox durable exports SHOULD use a project-agnostic index plus append-only JavaScript Object Notation Lines thread records so collaboration remains compact, filterable, and portable across future Handshake kernels. [ADD v02.168] Role Mailbox export families also inherit the shared base envelope and profile-extension boundary so project-specific routing hints remain portable instead of becoming hard-coded mailbox schema requirements. [ADD v02.169] Role Mailbox readable mirrors or summaries remain derived collaboration aids; advisory edits and mailbox narrative normalization MUST stay visible so async coordination cannot silently rewrite canonical work-state fields. [ADD v02.170] Role Mailbox triage SHOULD support inbox presets grouped by expected response, expiry, blocker posture, and linked work identifiers, while reply or escalation actions stay governed by explicit action bindings rather than thread ordering alone. [ADD v02.171] Role Mailbox posture MAY contribute queue-reason codes such as `mailbox_response_wait` or `escalation_wait`, but mailbox threads remain non-authoritative unless an explicit governed action updates the linked record. [ADD v02.172] Role Mailbox also participates in queue automation and transition law: mailbox replies, expiries, and escalation acknowledgements may trigger queue changes only through explicit automation rules, transition rules, and eligible actor policies. [ADD v02.173] Role Mailbox now also owns typed thread lifecycle, message delivery, allowed-response envelopes, and Micro-Task collaboration message families so asynchronous handoff and verifier loops stay structured without turning mailbox chronology into authority. [ADD v02.174] Role Mailbox now also coordinates verifier-driven Micro-Task loop checkpoints, structured verifier outcomes, remaining retry budget, escalation targets, and completion-report transcription posture so retry or escalation never collapses into open-ended chat. [ADD v02.175] Role Mailbox now also owns triage queue state, reminder schedules, snooze or expiry posture, dead-letter disposition, and explicit operator remediation controls so asynchronous collaboration can age, pause, recover, or archive deterministically without mutating linked authoritative work implicitly. [ADD v02.176] Role Mailbox now also owns executor-kind routing, claim mode, claim or lease records, takeover policy, and response-authority scope so parallel actors do not race to answer the same thread and human-only decisions remain guarded even when local small models or workflow automation can read the thread. [ADD v02.177] Role Mailbox now also owns structured handoff bundles, announce-back provenance kinds, normalized note-transcription references, and compact handoff summaries so asynchronous collaboration can hand work across actors without collapsing into transcript replay or silently implying authoritative completion. [ADD v02.181] Role Mailbox may coordinate software-delivery approvals, reviews, validation follow-up, queued steering, and closeout discussion, but mailbox threads remain projection and evidence only; authoritative gate, workflow, claim/lease, queued-instruction, and closeout state MUST resolve through product-owned runtime records."
    },
    {
      "feature_id": "FEAT-SEMANTIC-CATALOG",
      "title": "Semantic Catalog",
      "spec_anchor": "#267-semantic-catalog-registry-normative",
      "surfaces": [
        "backend",
        "ui",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-SemanticCatalog",
        "PRIM-SemanticCatalogEntry"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.144] Semantic Catalog is the deterministic tool/data dictionary that reduces runtime guessing for local/cloud models. [ADD v02.156] Semantic Catalog entries are indexed backend routing contracts for Spec Router and retrieval-backed runtime planning; deterministic catalog entries MAY NOT remain hidden in prompt helpers or UI labels.",
      "capability_slice_ids": [
        "CAP-013"
      ],
      "runtime_visibility_ids": [
        "RV-013"
      ]
    },
    {
      "feature_id": "FEAT-SKILL-BANK",
      "title": "Skill Bank & Distillation",
      "spec_anchor": "#9-continuous-local-skill-distillation-skill-bank-pipeline",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-AdapterCheckpoint",
        "PRIM-DistillationCandidate",
        "PRIM-SkillBankLogEntry"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.144] Skill Bank is the governed local learning surface for distillation, checkpoints, and evaluation lineage. [ADD v02.157] Teacher/student lineage, adapter-only late-stage training posture, tokenizer metadata, and benchmark-gated promotion rules are part of the canonical backend contract even while full LoRA / QLoRA / DoRA automation remains stub-backed.",
      "capability_slice_ids": [
        "CAP-012"
      ],
      "runtime_visibility_ids": [
        "RV-012"
      ]
    },
    {
      "feature_id": "FEAT-SPEC-APPENDICES",
      "title": "End-of-File Spec Appendices System",
      "spec_anchor": "#12-end-of-file-appendices",
      "surfaces": [
        "spec"
      ],
      "primitives": [],
      "tools_tech": [
        "TECH-JSON",
        "TECH-MARKDOWN"
      ],
      "notes": "Bootstrap entry for the appendix system itself."
    },
    {
      "feature_id": "FEAT-SPEC-ROUTER",
      "title": "Prompt-to-Spec Router",
      "spec_anchor": "Â§2.6.8.5",
      "surfaces": [
        "gov",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-SpecIntent",
        "PRIM-SpecRouterDecision",
        "PRIM-SpecArtifact",
        "PRIM-SpecSessionLogEntry",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-PromptEnvelopeV1",
        "PRIM-SpecPromptPackV1",
        "PRIM-SpecRouterPromptEnvelopeHashesV1",
        "PRIM-LocusCreateWpParams"
      ],
      "tools_tech": [
        "TOOL-GIT",
        "TECH-JSON",
        "TECH-SHA256"
      ],
      "notes": "Deterministic promptâ†’spec routing with session logs, capability snapshots, and SpecLint gating. [ADD v02.152] Prompt envelope hashes, prompt artifacts, decision artifacts, and Locus create-WP params are canonical backend evidence and portability seams; dedicated debug-bundle export remains stub-backed until explicit evidence-scope contracts land. [ADD v02.153] Capability snapshots are also a direct backend capability-governance seam, not only an evidence artifact, and MUST stay explicit when routing/spec-compilation logic evolves. [ADD v02.157] PromptEnvelope and SpecPromptPack are also reusable backend context-assembly contracts; when router jobs consume Context Packs they MUST preserve pack/freshness/hash lineage alongside prompt-envelope evidence. [ADD v02.178] Prompt-to-Spec Router also prefers direct authoritative work-state, policy, and artifact loads before hybrid retrieval; hybrid retrieval is an ambiguity aid, not the default route, and skipped-hybrid reasons must remain visible in routing provenance."
    },
    {
      "feature_id": "FEAT-STAGE",
      "title": "Handshake Stage (Built-in Browser + Stage Apps)",
      "spec_anchor": "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-MdSessionsRegistryV0",
        "PRIM-MdSessionRecordV0",
        "PRIM-MdAuthMode"
      ],
      "tools_tech": [
        "TECH-CSP",
        "TECH-JSON",
        "TECH-TAURI"
      ],
      "notes": "Governed browser surface with isolation, Stage Apps, and capture/import jobs with no bypass. [ADD v02.148] Stage now explicitly owns the shared media-session registry, persisted session records, and auth mode contracts that govern built-in capture/import and Media Downloader reuse. [ADD v02.150] Stage capture/import sessions are portable backend evidence roots and must participate in bounded debug bundle and artifact portability contracts with Media Downloader. [ADD v02.158] Stage-captured/imported media artifacts are also canonical ASR-ready backend evidence roots; transcript-lineage delivery into later Lens/Studio flows remains stub-backed until explicit job and artifact identity contracts land.",
      "capability_slice_ids": [
        "CAP-006"
      ],
      "runtime_visibility_ids": [
        "RV-006"
      ]
    },
    {
      "feature_id": "FEAT-STORAGE-PORTABILITY",
      "title": "Storage Backend Portability Architecture",
      "spec_anchor": "#2312-storage-backend-portability-architecture-cx-dbp-001",
      "surfaces": [
        "backend",
        "ci"
      ],
      "primitives": [
        "PRIM-Database",
        "PRIM-SqliteDatabase",
        "PRIM-PostgresDatabase",
        "PRIM-PostgresPrimaryControlPlane",
        "PRIM-ControlPlaneStorageMode",
        "PRIM-SqliteCacheOfflineBoundary",
        "PRIM-ArtifactManifest",
        "PRIM-BundleIndexEntry",
        "PRIM-PruneReport",
        "PRIM-RetentionPolicy",
        "PRIM-RetentionReport"
      ],
      "tools_tech": [
        "TECH-POSTGRESQL",
        "TECH-SQLITE",
        "TECH-SQLX"
      ],
      "notes": "SQLiteâ†’PostgreSQL portability constraints and Phase 1 closure requirements. [ADD v02.147] Portable artifact manifests, bundle indexes, retention policy, and prune reports are part of the storage portability contract, not backend-only internals. [ADD v02.148] RetentionReport is a shared portability/recovery contract so backend migration does not hide retention evidence behind engine-specific storage internals. [ADD v02.150] Portable manifests, bundle indexes, and retention/prune reports also cover Stage and Media Downloader debug exports so backend evidence survives engine and storage swaps. [ADD v02.151] Workflow artifacts, mailbox export manifests, and AI-ready index artifacts must preserve stable manifest, retention, and evidence semantics across backend swaps and bounded exports. [ADD v02.152] Spec Router prompt/decision artifacts and future MCP/MEX evidence-export bundles must preserve stable manifest/hash semantics across backend swaps and replay tooling. [ADD v02.153] Consent receipt and cloud escalation request artifacts are the next portability bridge and remain stub-backed until manifest/hash/retention semantics are made explicit."
    },
    {
      "feature_id": "FEAT-STUDIO",
      "title": "Studio / Design Studio Shell",
      "spec_anchor": "#633-domain-2-creative-studio",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-CanvasView",
        "PRIM-RoleSuggestionV1"
      ],
      "tools_tech": [
        "TECH-EXCALIDRAW",
        "TECH-JSON",
        "TECH-TAURI"
      ],
      "notes": "[ADD v02.144] Studio is the cross-surface creative shell that must make runtime orchestration visible across Canvas, Lens, Photo, and later design modules. [ADD v02.158] In Phase 1, Studio remains shell-level while Stage/ASR/Lens backend lineage and job identity stay explicit and stub-backed rather than hidden in the shell.",
      "capability_slice_ids": [
        "CAP-007"
      ],
      "runtime_visibility_ids": [
        "RV-007"
      ]
    },
    {
      "feature_id": "FEAT-TASK-BOARD",
      "title": "Task Board",
      "spec_anchor": "Â§2.6.8.8",
      "surfaces": [
        "gov",
        "backend",
        "ui"
      ],
      "primitives": [
        "PRIM-TaskBoardEntry",
        "PRIM-TaskBoardSections",
        "PRIM-TaskBoardStatus",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-SpecSessionLogEntry",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-RoleMailboxExecutorKind",
        "PRIM-RoleMailboxClaimMode",
        "PRIM-RoleMailboxClaimLeaseV1",
        "PRIM-RoleMailboxResponseAuthorityV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1",
        "PRIM-TaskBoardLaneDefinitionV1",
        "PRIM-ProjectionActionBindingV1",
        "PRIM-DevCommandCenterViewPresetV1"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-MARKDOWN"
      ],
      "notes": "[ADD v02.163] Task Board is the human-readable planning mirror over authoritative backend work-tracking artifacts. It preserves operator-readable status, freshness, and ready-work context, but MUST remain synchronized to Locus, Workflow Engine, and Spec Session Log identifiers instead of becoming a second execution authority. [ADD v02.166] Task Board projection rows SHOULD be materialized as structured records first and rendered as Markdown second so filtering, routing, freshness checks, and local-small-model reads remain field-addressable instead of prose-parsed. [ADD v02.167] Kanban, queue, list, roadmap, and future Jira-like Task Board layouts are all derived views over the same structured Task Board and Work Packet records; moving a card in a view MUST NOT become an untracked second authority. [ADD v02.168] Task Board rows also depend on the shared base envelope, compact summary contract, and profile-extension boundary so derived layouts can stay generic while still supporting project-specific grouping semantics. [ADD v02.169] Task Board mirrors also carry explicit authority mode and reconciliation action so manual edits, imported board changes, and regeneration events stay visible before a card is trusted. [ADD v02.170] Task Board board, list, queue, and roadmap layouts SHOULD be versioned view presets with explicit lane definitions and drag-to-lane action bindings so layout experimentation never backdoors workflow authority. [ADD v02.171] Task Board rows SHOULD render one base workflow-state family and queue reason code through project-profile label mappings rather than inventing board-only statuses that local-small-model routing cannot reuse. [ADD v02.172] Task Board projections also depend on explicit transition rules, queue automation rules, and executor eligibility policies so a lane move can preview whether it is view-only, automatic, actor-ineligible, or an allowed governed transition. [ADD v02.173] Task Board projections now also depend on Role Mailbox thread lifecycle and allowed-response posture flowing through linked Work Packets or Micro-Tasks, so queue pressure and waiting reasons stay explainable without treating mailbox chronology as board authority. [ADD v02.174] Task Board projections now also surface mailbox-driven verifier waits, retry-budget exhaustion, escalation checkpoints, and completion-report transcription posture through linked Work Packet or Micro-Task state instead of freeform comments or board-only badges. [ADD v02.175] Task Board projections now also surface mailbox triage backlog, queue age, snooze or expiry posture, and dead-letter remediation pressure through linked identifiers and queue-reason overlays instead of unread counts, manual comments, or lane ordering. [ADD v02.176] Task Board projections now also surface mailbox claimant, claim mode, lease age, lease expiry, and actor-ineligible waiting posture through linked identifiers and queue overlays instead of assignment comments, board-only assignees, or lane naming heuristics. [ADD v02.177] Task Board projections now also surface handoff-ready, announce-back advisory, and transcription-pending overlays through linked identifiers and compact handoff summaries instead of manual checklist comments or latest-message order. [ADD v02.181] Task Board now also projects software-delivery validator-gate and closeout posture from authoritative runtime records; lane order, board badges, imported repo task mirrors, and card comments MAY NOT become the authority for validation or completion."
    },
    {
      "feature_id": "FEAT-TERMINAL",
      "title": "Terminal Experience",
      "spec_anchor": "#101-terminal-experience",
      "surfaces": [
        "ui",
        "backend"
      ],
      "primitives": [
        "PRIM-TerminalCommandEvent"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TOOL-GIT"
      ],
      "notes": "Capability-gated terminal operations with provenance and Flight Recorder logging."
    },
    {
      "feature_id": "FEAT-THINKING-PIPELINE",
      "title": "Thinking Pipeline",
      "spec_anchor": "#259-thinking-pipeline-docs--canvas--workflows",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "primitives": [
        "PRIM-ContextPackPayloadV1",
        "PRIM-WorkflowRun"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.144] Thinking Pipeline is the cyclic handoff loop linking Docs, Canvas, Workflows, and synthesis surfaces.",
      "capability_slice_ids": [
        "CAP-011"
      ],
      "runtime_visibility_ids": [
        "RV-011"
      ]
    },
    {
      "feature_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "title": "Unified Tool Surface Contract (HTC-1.0)",
      "spec_anchor": "Â§6.0.2",
      "surfaces": [
        "backend",
        "gov",
        "ui"
      ],
      "primitives": [
        "PRIM-ToolEntry",
        "PRIM-ToolCallMeta",
        "PRIM-ToolsCallRequest"
      ],
      "tools_tech": [
        "TOOL-MCP",
        "TECH-JSON",
        "TECH-MCP",
        "TOOL-OLLAMA-API",
        "TOOL-OPENAI-COMPAT-API"
      ],
      "notes": "[ADD v02.142] Single governed local+cloud tool-calling contract with explicit runtime visibility into Command Center, Flight Recorder, and operator evidence surfaces. [ADD v02.144] Provider-specific local/cloud runtimes remain subordinate to one discoverable governed tool surface. [ADD v02.164] Repository-engine backend selection, required status checks, and merge-queue compatibility for `engine.version` operations MUST remain explicit governed tool metadata rather than hidden implementation details or command-output heuristics. [ADD v02.165] Tool/server transport kind, health state, permission scope, route policy, fallback policy, and last verification status are also canonical governed metadata for Dev Command Center tool-infrastructure and replay-routing views.",
      "capability_slice_ids": [
        "CAP-003"
      ],
      "runtime_visibility_ids": [
        "RV-003"
      ]
    },
    {
      "feature_id": "FEAT-WORK-PACKET-SYSTEM",
      "title": "Work Packet System",
      "spec_anchor": "Â§7.2",
      "surfaces": [
        "gov",
        "backend",
        "ui"
      ],
      "primitives": [
        "PRIM-TrackedWorkPacket",
        "PRIM-WorkPacketBinding",
        "PRIM-WorkPacketGovernance",
        "PRIM-WorkPacketPhase",
        "PRIM-WorkPacketStatus",
        "PRIM-WorkPacketType",
        "PRIM-SpecSessionLogEntry",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-MARKDOWN",
        "TECH-SQLITE"
      ],
      "notes": "[ADD v02.163] Work Packet System is the governed contract surface for scoped execution, workflow-linked activation, task-packet binding, and parallel-session planning. It MUST remain queryable by stable work_packet_id, workflow_run_id, micro_task_id, and model_session_id values so Dev Command Center and Locus can coordinate execution without relying on packet-local prose. [ADD v02.166] Work Packet canonical state SHOULD be structured-record-first with append-only note streams and Markdown mirrors or sidecars for long-form reasoning, so handoff completeness, blockers, and evidence references remain machine-readable without sacrificing human readability. [ADD v02.167] Work Packet canonical artifacts SHOULD use versioned JavaScript Object Notation records with compact summary companions and project-profile extensions so the same core contract can serve software, research, design, and other future Handshake project kernels. [ADD v02.168] Work Packet records now also define the shared base structured-collaboration envelope and compact summary contract that later project kernels are expected to inherit without repository-specific lock-in. [ADD v02.169] Work Packet mirror contracts now also define whether readable Markdown is purely derived, advisory-editable, or note-sidecar-only so packet narratives cannot silently outrank canonical structured state. [ADD v02.170] Work Packet detail and board surfaces SHOULD expose the governed next actions attached to the current view preset so operators can see what a move, review request, escalation, or promote action will actually mutate before acting. [ADD v02.171] Work Packet records now also declare base `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` values so project-profile-specific labels do not fork activation, review, approval, or completion semantics. [ADD v02.172] Work Packet records also depend on explicit transition rules, queue automation rules, and executor eligibility policies so activation, retry, review, approval, validation, completion, and cancellation law remain portable across project kernels. [ADD v02.173] Work Packet handoff and review posture now also depends on typed Role Mailbox thread lifecycle, delivery state, and allowed-response envelopes so packet-level async collaboration stays structured while authoritative packet state still changes only through governed actions or explicit transcription. [ADD v02.174] Work Packet note streams and detail views now also consume mailbox-linked loop checkpoints, verifier outcomes, escalation summaries, and completion-report transcription targets so remaining work, waiting posture, and announce-back evidence stay compact and auditable. [ADD v02.175] Work Packet note streams and follow-up views now also consume mailbox triage queue state, reminder schedules, expiry posture, and dead-letter remediation summaries so collaboration debt, operator follow-up, and unresolved waiting posture remain visible without reading raw mailbox transcripts. [ADD v02.176] Work Packet follow-up and handoff views now also consume mailbox claimant, claim mode, lease expiry, handback reason, and response-authority summaries so ownership ambiguity and takeover risk stay visible without reading full thread histories. [ADD v02.177] Work Packet note streams and handoff views now also consume mailbox handoff bundle summaries, announce-back provenance, and transcription-status refs so remaining work and next-actor posture stay queryable without replaying full mailbox threads. [ADD v02.178] Work Packet records remain authoritative execution contracts, so known packet ids or bindings MUST resolve by direct load before hybrid retrieval and any related-context search MUST remain advisory instead of overriding blockers, gates, or packet state. [ADD v02.181] Work Packet records now also serve as the product-owned software-delivery governance contract layer that imported repo-governance artifacts map into; validator gates, governed actions, and closeout posture remain runtime-derived rather than packet prose or repo-local ledgers."
    },
    {
      "feature_id": "FEAT-WORKSPACE-BUNDLE",
      "title": "Workspace Bundle Export",
      "spec_anchor": "#1057-workspace-bundle-export-v0",
      "surfaces": [
        "backend",
        "operator_consoles",
        "ui"
      ],
      "primitives": [
        "PRIM-BundleManifest",
        "PRIM-ArtifactManifest",
        "PRIM-ExportRecord",
        "PRIM-ExportTarget"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-ZIP"
      ],
      "notes": "[ADD v02.154] Workspace Bundle export is a normative backend transfer and backup surface with capability-gated export profiles, manifest hashing, and recorder-visible bundle lifecycle. Appendix coverage remains stub-backed until implementation lands. [ADD v02.161] Workspace Bundle export state, manifest identifiers, and export records are also canonical Dev Command Center evidence-and-replay projection state."
    },
    {
      "feature_id": "FEAT-WORKFLOW-ENGINE",
      "title": "Workflow & Automation Engine",
      "spec_anchor": "#26-workflow-automation-engine",
      "surfaces": [
        "backend"
      ],
      "primitives": [
        "PRIM-WorkflowContext",
        "PRIM-JobState",
        "PRIM-WorkflowRun",
        "PRIM-WorkflowNodeExecution",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-AiJobListFilter",
        "PRIM-JobStatusUpdate",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-GateStatuses"
      ],
      "tools_tech": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "notes": "Single execution authority for jobs and workflows; durable state and no bypass for production-path execution. [ADD v02.150] Workflow runs and node executions are canonical backend correlation anchors for Locus sync, Flight Recorder joins, and bounded debug bundle export. [ADD v02.151] Progress artifacts, run ledgers, projection plans, and artifact manifests are portable backend evidence roots that must remain stable across storage/export flows. [ADD v02.153] Workflow execution is also the production enforcement surface for required capability checks and MUST keep that coupling explicit instead of treating capabilities as preflight-only metadata. [ADD v02.162] Workflow execution is also the canonical bridge between Work Packet activation, Task Board synchronization, parallel session steering, and Micro-Task job dispatch; those joins MUST remain queryable by workflow-run and work-packet identifiers. [ADD v02.165] Replay-safe run history, queue-state transitions, and operator reroute decisions are also canonical workflow-backed projection inputs for Dev Command Center operating views. [ADD v02.181] Workflow Engine now also remains the sole execution authority for software-delivery governance overlay actions, validator-gate materialization from Check Runner evidence, overlay claim/lease and queued-instruction application, and closeout-triggering start/steer/cancel/close/recover transitions; imported repo artifacts may inform or mirror but never bypass workflow-backed runtime law."
    }
  ],
  "capability_slices": [
    {
      "capability_slice_id": "CAP-001",
      "feature_id": "FEAT-CALENDAR",
      "title": "[ADD v02.142] Calendar temporal correlation",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-001"
      ],
      "notes": "[ADD v02.142] Calendar time windows project Flight Recorder activity/session spans into operator-visible calendar context."
    },
    {
      "capability_slice_id": "CAP-002",
      "feature_id": "FEAT-CALENDAR",
      "title": "[ADD v02.142] Calendar orchestrated mutation",
      "surfaces": [
        "ui",
        "backend"
      ],
      "runtime_visibility_ids": [
        "RV-002"
      ],
      "notes": "[ADD v02.142] Calendar writes stay job-backed and capability-gated so local edits remain replayable and safe."
    },
    {
      "capability_slice_id": "CAP-003",
      "feature_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "title": "[ADD v02.142] Unified local/cloud governed tool calling",
      "surfaces": [
        "backend",
        "gov",
        "ui"
      ],
      "runtime_visibility_ids": [
        "RV-003"
      ],
      "notes": "[ADD v02.142] Tool Registry + Tool Gate + MCP/local parity is the single runtime path for governed tool invocation. [ADD v02.165] Tool infrastructure health, transport kind, route policy, and last verification status are part of that runtime path and must remain projectable into Dev Command Center operating views."
    },
    {
      "capability_slice_id": "CAP-004",
      "feature_id": "FEAT-LOCUS-WORK-TRACKING",
      "title": "[ADD v02.142] Locus execution correlation",
      "surfaces": [
        "gov",
        "backend",
        "ui"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ],
      "notes": "[ADD v02.142] Locus work packets, microtasks, and task-board status are visible as runtime execution state rather than hidden governance metadata."
    },
    {
      "capability_slice_id": "CAP-005",
      "feature_id": "FEAT-LOOM-LIBRARY",
      "title": "[ADD v02.142] Loom retrieval library",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-005"
      ],
      "notes": "[ADD v02.142] Loom content is a governed retrieval/runtime surface, not only a passive library."
    },
    {
      "capability_slice_id": "CAP-006",
      "feature_id": "FEAT-STAGE",
      "title": "[ADD v02.142] Stage capture/import pipeline",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-006"
      ],
      "notes": "[ADD v02.142] Stage capture/import jobs are first-class runtime flows with governed tooling, artifacts, and evidence. [ADD v02.158] Stage media artifacts remain ASR-eligible backend roots and must preserve capture/session provenance plus portable manifest semantics."
    },
    {
      "capability_slice_id": "CAP-007",
      "feature_id": "FEAT-STUDIO",
      "title": "[ADD v02.144] Studio shell / runtime orchestration",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-007"
      ],
      "notes": "[ADD v02.144] Studio shell actions must project into jobs, tool calls, and operator evidence instead of remaining purely IA."
    },
    {
      "capability_slice_id": "CAP-008",
      "feature_id": "FEAT-DOCS-SHEETS",
      "title": "[ADD v02.144] Docs & Sheets AI operations",
      "surfaces": [
        "ui",
        "backend"
      ],
      "runtime_visibility_ids": [
        "RV-008"
      ],
      "notes": "[ADD v02.144] Structured doc/sheet edits are executed as governed jobs over stable IDs and provenance."
    },
    {
      "capability_slice_id": "CAP-009",
      "feature_id": "FEAT-CANVAS",
      "title": "[ADD v02.144] Canvas spatial composition",
      "surfaces": [
        "ui",
        "backend"
      ],
      "runtime_visibility_ids": [
        "RV-009"
      ],
      "notes": "[ADD v02.144] Canvas moves content through spatial organization that should remain visible to later automation and review."
    },
    {
      "capability_slice_id": "CAP-010",
      "feature_id": "FEAT-PROJECT-BRAIN",
      "title": "[ADD v02.144] Project Brain retrieval interface",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-010"
      ],
      "notes": "[ADD v02.144] Project Brain queries should remain explainable across retrieval, citation, and operator evidence surfaces."
    },
    {
      "capability_slice_id": "CAP-011",
      "feature_id": "FEAT-THINKING-PIPELINE",
      "title": "[ADD v02.144] Thinking Pipeline loop",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-011"
      ],
      "notes": "[ADD v02.144] The Docs -> Canvas -> Workflows -> Docs loop is a runtime-visible orchestration path, not only a product metaphor."
    },
    {
      "capability_slice_id": "CAP-012",
      "feature_id": "FEAT-SKILL-BANK",
      "title": "[ADD v02.144] Skill Bank distillation lifecycle",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-012"
      ],
      "notes": "[ADD v02.144] Distillation candidates, checkpoints, and eval promotions must remain visible in runtime and operator flows. [ADD v02.157] Teacher/student tokenizer metadata, adapter-only late-stage training posture, and benchmark-gated promotion/rollback state are part of the same backend visibility contract."
    },
    {
      "capability_slice_id": "CAP-013",
      "feature_id": "FEAT-SEMANTIC-CATALOG",
      "title": "[ADD v02.144] Semantic tool resolution",
      "surfaces": [
        "backend",
        "ui",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-013"
      ],
      "notes": "[ADD v02.144] Semantic Catalog should provide deterministic routing hints for local/cloud models and operators."
    },
    {
      "capability_slice_id": "CAP-014",
      "feature_id": "FEAT-ASR",
      "title": "[ADD v02.144] ASR transcription pipeline",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-014"
      ],
      "notes": "[ADD v02.144] ASR transcription is a governed pipeline from media source to transcript artifact and downstream retrieval surfaces. [ADD v02.158] Recorder-visible progress/failure semantics, ffprobe-derived media facts, and transcript portability are part of the same backend contract."
    },
    {
      "capability_slice_id": "CAP-015",
      "feature_id": "FEAT-CHARTS-DASHBOARDS",
      "title": "[ADD v02.144] Charts / dashboard composition",
      "surfaces": [
        "ui",
        "backend"
      ],
      "runtime_visibility_ids": [
        "RV-015"
      ],
      "notes": "[ADD v02.144] Charts are derived analytical views that should remain tied to jobs, tables, and evidence."
    },
    {
      "capability_slice_id": "CAP-016",
      "feature_id": "FEAT-PRESENTATIONS-DECKS",
      "title": "[ADD v02.144] Deck composition / export",
      "surfaces": [
        "ui",
        "backend"
      ],
      "runtime_visibility_ids": [
        "RV-016"
      ],
      "notes": "[ADD v02.144] Deck generation should remain governed and traceable across exports, charts, and supporting context."
    },
    {
      "capability_slice_id": "CAP-017",
      "feature_id": "FEAT-CONTEXT-PACKS",
      "title": "[ADD v02.144] ContextPack retrieval compaction",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ],
      "notes": "[ADD v02.144] ContextPack build/select/refresh actions are first-class retrieval/runtime flows, not hidden cache internals. [ADD v02.157] Freshness-policy decisions, pack hashes, and build/select/refresh outcomes must also remain recorder-visible backend evidence for replay, distillation, and model onboarding."
    },
    {
      "capability_slice_id": "CAP-018",
      "feature_id": "FEAT-DEV-COMMAND-CENTER",
      "title": "[ADD v02.144] Dev Command Center runtime projection",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.144] Dev Command Center projects jobs, problems, timeline slices, and approvals as operator-visible runtime state. [ADD v02.160] It also projects workflow runs, model-session scheduler snapshots, capability snapshots, and work packet or worktree bindings as governed control-plane state. [ADD v02.161] It also projects Governance Pack export state, Workspace Bundle export state, diagnostics query state, and bounded evidence packaging outcomes as governed evidence-and-replay state. [ADD v02.162] It also projects tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, and parallel model session occupancy as governed work-orchestration state. [ADD v02.165] It also projects replay-safe run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots as governed operating state."
    },
    {
      "capability_slice_id": "CAP-019",
      "feature_id": "FEAT-MAIL-CLIENT",
      "title": "[ADD v02.144] Mail ingestion / thread correlation",
      "surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-019"
      ],
      "notes": "[ADD v02.144] Mail threads are governed runtime inputs for retrieval, drafting, and Calendar-linked workflows."
    },
    {
      "capability_slice_id": "CAP-020",
      "feature_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "title": "[ADD v02.145] Model session lifecycle orchestration",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.145] Session scheduler, provider routing, spawn constraints, and model swaps must remain explicit orchestration contracts rather than hidden runtime behavior."
    },
    {
      "capability_slice_id": "CAP-021",
      "feature_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "title": "[ADD v02.145] Cloud escalation consent artifacts",
      "surfaces": [
        "backend",
        "gov",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ],
      "notes": "[ADD v02.145] Projection plans, consent receipts, and governance mode outcomes must remain explicit when local workflows escalate to cloud reasoning."
    },
    {
      "capability_slice_id": "CAP-022",
      "feature_id": "FEAT-MEX-RUNTIME",
      "title": "[ADD v02.145] Mechanical engine runtime execution",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-022"
      ],
      "notes": "[ADD v02.145] Planned mechanical operations and engine results must remain visible as governed runtime work rather than hidden adapter internals."
    },
    {
      "capability_slice_id": "CAP-023",
      "feature_id": "FEAT-GOVERNANCE-PACK",
      "title": "[ADD v02.154] Governance Pack export workflow projection",
      "surfaces": [
        "gov",
        "backend",
        "operator_consoles",
        "ui"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ],
      "notes": "[ADD v02.154] Governance Pack export is a governed workflow-run with capability gating, recorder-visible lifecycle, and portable artifact manifests."
    },
    {
      "capability_slice_id": "CAP-024",
      "feature_id": "FEAT-CALENDAR",
      "title": "[ADD v02.155] Calendar policy-scoped routing and mutation discipline",
      "surfaces": [
        "backend",
        "operator_consoles"
      ],
      "runtime_visibility_ids": [
        "RV-024"
      ],
      "notes": "[ADD v02.155] Calendar capability profiles, event policy profiles, and scope hints must remain explicit at job boundaries so routing, consent posture, and mutation discipline stay deterministic and portable."
    }
  ],
  "runtime_visibility_map": [
    {
      "runtime_visibility_id": "RV-001",
      "capability_slice_id": "CAP-001",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Calendar time-window queries must remain visible to operator evidence surfaces and future DCC/Locus overlays."
    },
    {
      "runtime_visibility_id": "RV-002",
      "capability_slice_id": "CAP-002",
      "job_workflow_surface": [
        "MECHANICAL_TOOL",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UI_ONLY"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "NONE",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Calendar writes remain job-backed local mutations with explicit FR evidence and future operator visibility."
    },
    {
      "runtime_visibility_id": "RV-003",
      "capability_slice_id": "CAP-003",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "UNIFIED_TOOL_SURFACE",
        "MCP",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Governed tool calls must be discoverable by local/cloud models and visible in DCC/operator evidence views."
    },
    {
      "runtime_visibility_id": "RV-004",
      "capability_slice_id": "CAP-004",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "VISIBLE",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Locus execution state should project into runtime/operator surfaces instead of remaining hidden in governance artifacts only."
    },
    {
      "runtime_visibility_id": "RV-005",
      "capability_slice_id": "CAP-005",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UI_ONLY",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Loom retrieval/query flows need explicit runtime visibility because they feed AI-ready and lens-style reasoning paths."
    },
    {
      "runtime_visibility_id": "RV-006",
      "capability_slice_id": "CAP-006",
      "job_workflow_surface": [
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "UI_ONLY",
        "COMMAND_CENTER"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.142] Stage capture/import is a governed runtime pipeline with artifacts, approvals, and replayable evidence. [ADD v02.158] Stage media artifacts must preserve capture/session lineage so later ASR jobs and replay/export tooling do not depend on UI-only context."
    },
    {
      "runtime_visibility_id": "RV-007",
      "capability_slice_id": "CAP-007",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Studio shell actions must remain inspectable as runtime work, not disappear into UI-only orchestration."
    },
    {
      "runtime_visibility_id": "RV-008",
      "capability_slice_id": "CAP-008",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UNIFIED_TOOL_SURFACE",
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Docs & Sheets edits must stay visible as governed operations with provenance and later operator replay."
    },
    {
      "runtime_visibility_id": "RV-009",
      "capability_slice_id": "CAP-009",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UI_ONLY",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Canvas spatial organization should remain visible to later automation, provenance, and cross-surface reasoning."
    },
    {
      "runtime_visibility_id": "RV-010",
      "capability_slice_id": "CAP-010",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UNIFIED_TOOL_SURFACE",
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Project Brain retrieval/citation flows must remain explainable across DCC, FR, and later Locus overlays."
    },
    {
      "runtime_visibility_id": "RV-011",
      "capability_slice_id": "CAP-011",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Thinking Pipeline transitions should remain visible as explicit handoffs, not hidden cross-surface jumps."
    },
    {
      "runtime_visibility_id": "RV-012",
      "capability_slice_id": "CAP-012",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Skill Bank promotion and checkpoint lineage must stay queryable for operators and future local-model tooling."
    },
    {
      "runtime_visibility_id": "RV-013",
      "capability_slice_id": "CAP-013",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UNIFIED_TOOL_SURFACE",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Semantic Catalog lookups must remain visible when they influence tool choice, store routing, or prompt compaction."
    },
    {
      "runtime_visibility_id": "RV-014",
      "capability_slice_id": "CAP-014",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] ASR transcription progress, output, and downstream reuse must remain explicit in operator/runtime surfaces. [ADD v02.158] Source-media hashes, media-probe facts, and transcript timing anchors are portable backend evidence and recorder-visible debugging inputs."
    },
    {
      "runtime_visibility_id": "RV-015",
      "capability_slice_id": "CAP-015",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UI_ONLY",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Chart generation should remain tied to inputs, jobs, and export lineage rather than becoming detached presentation output."
    },
    {
      "runtime_visibility_id": "RV-016",
      "capability_slice_id": "CAP-016",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UI_ONLY",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Deck generation/export must remain governed, replayable, and explainable across charts and supporting evidence."
    },
    {
      "runtime_visibility_id": "RV-017",
      "capability_slice_id": "CAP-017",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "UNIFIED_TOOL_SURFACE",
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] ContextPack build/select/freshness outcomes must remain operator-visible because they shape every later prompt/runtime decision."
    },
    {
      "runtime_visibility_id": "RV-018",
      "capability_slice_id": "CAP-018",
      "job_workflow_surface": [
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "VISIBLE",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Dev Command Center is the runtime projection surface for jobs, problems, tool calls, approvals, and pinned timeline slices. [ADD v02.160] It is also the control-plane projection surface for workflow runs, model sessions, capability decisions, and work packet or worktree steering backed by authoritative backend artifacts. [ADD v02.161] It is also the evidence-and-replay projection surface for governance export lifecycles, workspace bundle lifecycles, diagnostics queries, and workflow-linked evidence packaging backed by authoritative backend artifacts. [ADD v02.162] It is also the work-orchestration projection surface for tracked Work Packets, Task Board sync freshness, ready-query state, Micro-Task summaries, and parallel session occupancy backed by authoritative backend artifacts."
    },
    {
      "runtime_visibility_id": "RV-019",
      "capability_slice_id": "CAP-019",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.144] Mail ingestion, drafting, and event correlation must remain visible instead of hiding behind provider-specific sync behavior."
    },
    {
      "runtime_visibility_id": "RV-020",
      "capability_slice_id": "CAP-020",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.145] Model session orchestration must remain visible as queued/runtime work with provider routing, swap requests, and session governance outcomes."
    },
    {
      "runtime_visibility_id": "RV-021",
      "capability_slice_id": "CAP-021",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.145] Cloud escalation consent must remain queryable via projection plans, consent receipts, and governance-mode outcomes instead of ephemeral prompt-time choices."
    },
    {
      "runtime_visibility_id": "RV-022",
      "capability_slice_id": "CAP-022",
      "job_workflow_surface": [
        "MECHANICAL_TOOL",
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.145] MEX runtime actions must remain visible through engine result envelopes, safety gates, and operator evidence rather than disappearing behind adapters."
    },
    {
      "runtime_visibility_id": "RV-023",
      "capability_slice_id": "CAP-023",
      "job_workflow_surface": [
        "WORKFLOW"
      ],
      "tool_surface": [
        "COMMAND_CENTER",
        "UI_ONLY"
      ],
      "model_exposure": "OPERATOR_ONLY",
      "command_center_visibility": "VISIBLE",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.154] Governance Pack export must remain visible as a governed workflow/export lifecycle with project invariants, manifest hashes, and durable evidence."
    },
    {
      "runtime_visibility_id": "RV-024",
      "capability_slice_id": "CAP-024",
      "job_workflow_surface": [
        "AI_JOB",
        "WORKFLOW",
        "MECHANICAL_TOOL"
      ],
      "tool_surface": [
        "COMMAND_CENTER"
      ],
      "model_exposure": "BOTH",
      "command_center_visibility": "PLANNED",
      "flight_recorder_visibility": "VISIBLE",
      "locus_visibility": "PLANNED",
      "storage_posture": "SQLITE_NOW_POSTGRES_READY",
      "notes": "[ADD v02.155] Calendar capability profiles, policy-profile selection, and scope-hint routing must stay queryable as backend execution posture rather than transient view-only state."
    }
  ]
}
```
<!-- HS_APPENDIX:END id=HS-APPX-FEATURE-REGISTRY -->

## 12.4 Appendix Block: PRIMITIVE_TOOL_TECH_MATRIX (Machine-readable) [CX-SPEC-APPX-011]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX schema=hs_primitive_tool_tech_matrix@2 -->
```json
{
  "schema": "hs_primitive_tool_tech_matrix@2",
  "spec_version": "v02.184",
  "last_updated": "2026-05-05",
  "primitives": [
    {
      "primitive_id": "PRIM-AIAssistedAdjustments",
      "title": "AIAssistedAdjustments",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AIGeneratedMetadata",
      "title": "AIGeneratedMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AccessMode",
      "title": "AccessMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-AceRuntimeValidator",
      "title": "AceRuntimeValidator",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-ActivationTrigger",
      "title": "ActivationTrigger",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ActivitySpanBinding",
      "title": "ActivitySpanBinding",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-Actor",
      "title": "Actor",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-AdapterCheckpoint",
      "title": "AdapterCheckpoint",
      "kind": "py_dataclass"
    },
    {
      "primitive_id": "PRIM-AdapterError",
      "title": "AdapterError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-AddTranscriptionLinkRequest",
      "title": "AddTranscriptionLinkRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AdjustmentLayer",
      "title": "AdjustmentLayer",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AffectedEntity",
      "title": "AffectedEntity",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AgentContextAnnotations",
      "title": "AgentContextAnnotations",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AiJob",
      "title": "AiJob",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AiJobListFilter",
      "title": "AiJobListFilter",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AiJobMcpFields",
      "title": "AiJobMcpFields",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AiJobMcpUpdate",
      "title": "AiJobMcpUpdate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AiJobsDrawer",
      "title": "AiJobsDrawer",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-AiReadyDataPipeline",
      "title": "AiReadyDataPipeline",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Alternative",
      "title": "Alternative",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AnchorsPresent",
      "title": "AnchorsPresent",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Ans001Enforcer",
      "title": "Ans001Enforcer",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Ans001TimelineDrawer",
      "title": "Ans001TimelineDrawer",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-Answer",
      "title": "Answer",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AntiPatternAlert",
      "title": "AntiPatternAlert",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-AppState",
      "title": "AppState",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ApproximateControl",
      "title": "ApproximateControl",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ArtifactKind",
      "title": "ArtifactKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ArtifactManifest",
      "title": "ArtifactManifest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ArtifactService",
      "title": "ArtifactService",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AssemblyParams",
      "title": "AssemblyParams",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Asset",
      "title": "Asset",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AssetKind",
      "title": "AssetKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-Assumption",
      "title": "Assumption",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AssumptionManager",
      "title": "AssumptionManager",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-AtelierCollaborationPanel",
      "title": "AtelierCollaborationPanel",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-AtelierFact",
      "title": "AtelierFact",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AtelierScopeError",
      "title": "AtelierScopeError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-AtelierSymbolFact",
      "title": "AtelierSymbolFact",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-AutomationLevel",
      "title": "AutomationLevel",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BasicAdjustments",
      "title": "BasicAdjustments",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BehaviorConfig",
      "title": "BehaviorConfig",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-BoundaryType",
      "title": "BoundaryType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BronzeId",
      "title": "BronzeId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BronzeRecord",
      "title": "BronzeRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BronzeRecordCreatedEvent",
      "title": "BronzeRecordCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BudgetsV1",
      "title": "BudgetsV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-BundleDiagnostic",
      "title": "BundleDiagnostic",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BundleEnv",
      "title": "BundleEnv",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BundleExportError",
      "title": "BundleExportError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-BundleExportRequest",
      "title": "BundleExportRequest",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BundleExportResponse",
      "title": "BundleExportResponse",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BundleIndexEntry",
      "title": "BundleIndexEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-BundleJob",
      "title": "BundleJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BundleJobs",
      "title": "BundleJobs",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-BundleManifest",
      "title": "BundleManifest",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BundleManifestFile",
      "title": "BundleManifestFile",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-BundleScope",
      "title": "BundleScope",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-BundleStatus",
      "title": "BundleStatus",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-BundleValidationReport",
      "title": "BundleValidationReport",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarEvent",
      "title": "CalendarEvent",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarEventExportMode",
      "title": "CalendarEventExportMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarEventStatus",
      "title": "CalendarEventStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarEventUpsert",
      "title": "CalendarEventUpsert",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarEventVisibility",
      "title": "CalendarEventVisibility",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarEventWindowQuery",
      "title": "CalendarEventWindowQuery",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarMutation",
      "title": "CalendarMutation",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarSource",
      "title": "CalendarSource",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarSourceProviderType",
      "title": "CalendarSourceProviderType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarSourceSyncState",
      "title": "CalendarSourceSyncState",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarSourceUpsert",
      "title": "CalendarSourceUpsert",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarSourceWritePolicy",
      "title": "CalendarSourceWritePolicy",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CalendarSyncInput",
      "title": "CalendarSyncInput",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CalendarSyncStateStage",
      "title": "CalendarSyncStateStage",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CanvasView",
      "title": "CanvasView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-CapabilityKind",
      "title": "CapabilityKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-CapabilityProfile",
      "title": "CapabilityProfile",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CapabilityRegistry",
      "title": "CapabilityRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CapabilityRegistryEntry",
      "title": "CapabilityRegistryEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CapabilitySnapshotCapabilityV1",
      "title": "CapabilitySnapshotCapabilityV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CapabilitySnapshotToolV1",
      "title": "CapabilitySnapshotToolV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CapabilitySnapshotV1",
      "title": "CapabilitySnapshotV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ChallengeFirstValidator",
      "title": "ChallengeFirstValidator",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ChangeType",
      "title": "ChangeType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ChartSpec",
      "title": "ChartSpec",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ChunkId",
      "title": "ChunkId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ChunkValidation",
      "title": "ChunkValidation",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ChunkingParams",
      "title": "ChunkingParams",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ChunkingStrategy",
      "title": "ChunkingStrategy",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-CloudEscalationBundleV0_4",
      "title": "CloudEscalationBundleV0_4",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CloudEscalationConsentRequest",
      "title": "CloudEscalationConsentRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CloudEscalationEvent",
      "title": "CloudEscalationEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-CloudEscalationEventType",
      "title": "CloudEscalationEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-CloudEscalationGuard",
      "title": "CloudEscalationGuard",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CloudEscalationPolicy",
      "title": "CloudEscalationPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CloudEscalationRequest",
      "title": "CloudEscalationRequest",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-CloudEscalationUiSurface",
      "title": "CloudEscalationUiSurface",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-CodeMetadata",
      "title": "CodeMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-CommandPalette",
      "title": "CommandPalette",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-CompletionEvidence",
      "title": "CompletionEvidence",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-CompletionRequest",
      "title": "CompletionRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CompletionResponse",
      "title": "CompletionResponse",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CompletionSignal",
      "title": "CompletionSignal",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Conflict",
      "title": "Conflict",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ConsentDecision",
      "title": "ConsentDecision",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ConsentProvider",
      "title": "ConsentProvider",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-ConsentReceiptV0_4",
      "title": "ConsentReceiptV0_4",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ConsentScopeV0_4",
      "title": "ConsentScopeV0_4",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ContentId",
      "title": "ContentId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ContentIntegrityGuard",
      "title": "ContentIntegrityGuard",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContentSensitivity",
      "title": "ContentSensitivity",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContentType",
      "title": "ContentType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ContentUntouchedOutsideWindow",
      "title": "ContentUntouchedOutsideWindow",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextAssembledEvent",
      "title": "ContextAssembledEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContextAssembly",
      "title": "ContextAssembly",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContextBlockV1",
      "title": "ContextBlockV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextBudgetConfig",
      "title": "ContextBudgetConfig",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContextPackAnchorV1",
      "title": "ContextPackAnchorV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackBuilder",
      "title": "ContextPackBuilder",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackCoverageV1",
      "title": "ContextPackCoverageV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackFreshnessDecision",
      "title": "ContextPackFreshnessDecision",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ContextPackFreshnessGuard",
      "title": "ContextPackFreshnessGuard",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackFreshnessPolicyV1",
      "title": "ContextPackFreshnessPolicyV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackPayloadV1",
      "title": "ContextPackPayloadV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPackRecord",
      "title": "ContextPackRecord",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ContextPollutionAlertEvent",
      "title": "ContextPollutionAlertEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContextPollutionMetrics",
      "title": "ContextPollutionMetrics",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ContextualChunk",
      "title": "ContextualChunk",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Cor700Validator",
      "title": "Cor700Validator",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CoreMetadata",
      "title": "CoreMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Corpus",
      "title": "Corpus",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CreateJobRequest",
      "title": "CreateJobRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CreateRoleMailboxMessageRequest",
      "title": "CreateRoleMailboxMessageRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-CurrentMatchesPreimage",
      "title": "CurrentMatchesPreimage",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DataAuditEvent",
      "title": "DataAuditEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DataModeValidator",
      "title": "DataModeValidator",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DataQualityMetrics",
      "title": "DataQualityMetrics",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Database",
      "title": "Database",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-DeadEndDetector",
      "title": "DeadEndDetector",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DebugBundleComplete",
      "title": "DebugBundleComplete",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-DebugBundleExport",
      "title": "DebugBundleExport",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-DebugBundleExportEvent",
      "title": "DebugBundleExportEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DebugBundleExporter",
      "title": "DebugBundleExporter",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-DebugBundleProgress",
      "title": "DebugBundleProgress",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-DebugBundleRequest",
      "title": "DebugBundleRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DebugPanel",
      "title": "DebugPanel",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-DeckSpec",
      "title": "DeckSpec",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-DependencyType",
      "title": "DependencyType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DescriptorRow",
      "title": "DescriptorRow",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DeterminismMode",
      "title": "DeterminismMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-DeterministicEditEngine",
      "title": "DeterministicEditEngine",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DiagFilter",
      "title": "DiagFilter",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Diagnostic",
      "title": "Diagnostic",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DiagnosticEvent",
      "title": "DiagnosticEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DiagnosticLocation",
      "title": "DiagnosticLocation",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DiagnosticRange",
      "title": "DiagnosticRange",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DiagnosticSeverity",
      "title": "DiagnosticSeverity",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DiagnosticSource",
      "title": "DiagnosticSource",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DiagnosticStatus",
      "title": "DiagnosticStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-DiagnosticSurface",
      "title": "DiagnosticSurface",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DiagnosticsQuery",
      "title": "DiagnosticsQuery",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DiagnosticsStore",
      "title": "DiagnosticsStore",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-DistillationCandidate",
      "title": "DistillationCandidate",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PendingDistillationCandidate",
      "title": "PendingDistillationCandidate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DocIngestResult",
      "title": "DocIngestResult",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DocIngestSpec",
      "title": "DocIngestSpec",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DocsAiJobProfile",
      "title": "DocsAiJobProfile",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DocumentFormat",
      "title": "DocumentFormat",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DocumentMetadata",
      "title": "DocumentMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DocumentView",
      "title": "DocumentView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-Domains",
      "title": "Domains",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DevCommandRunHistoryEntry",
      "title": "DevCommandRunHistoryEntry",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-DoneCriterion",
      "title": "DoneCriterion",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DriftHandler",
      "title": "DriftHandler",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-DropBackDecision",
      "title": "DropBackDecision",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DropBackStrategy",
      "title": "DropBackStrategy",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-DualPathGenerator",
      "title": "DualPathGenerator",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EditBehavior",
      "title": "EditBehavior",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EditRecipe",
      "title": "EditRecipe",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EditorEditEvent",
      "title": "EditorEditEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EditorEditOp",
      "title": "EditorEditOp",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EffortDelta",
      "title": "EffortDelta",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-EmbeddingArtifact",
      "title": "EmbeddingArtifact",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EmbeddingComputedEvent",
      "title": "EmbeddingComputedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingModelChangedEvent",
      "title": "EmbeddingModelChangedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingModelRecord",
      "title": "EmbeddingModelRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingModelRegistry",
      "title": "EmbeddingModelRegistry",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingModelStatus",
      "title": "EmbeddingModelStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-EmbeddingQualityJob",
      "title": "EmbeddingQualityJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingRecord",
      "title": "EmbeddingRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EmbeddingRegistry",
      "title": "EmbeddingRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EngineAdapter",
      "title": "EngineAdapter",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-EngineResult",
      "title": "EngineResult",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EntityId",
      "title": "EntityId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-EntryType",
      "title": "EntryType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-EscalationAttempt",
      "title": "EscalationAttempt",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EscalationEngine",
      "title": "EscalationEngine",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EscalationLevel",
      "title": "EscalationLevel",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EscalationRecord",
      "title": "EscalationRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EventFilter",
      "title": "EventFilter",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-EvidenceDrawer",
      "title": "EvidenceDrawer",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-EvidenceRefs",
      "title": "EvidenceRefs",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-EvidenceSelection",
      "title": "EvidenceSelection",
      "kind": "react_type"
    },
    {
      "primitive_id": "PRIM-ExcalidrawCanvas",
      "title": "ExcalidrawCanvas",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-ExecPolicy",
      "title": "ExecPolicy",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecPolicyExactness",
      "title": "ExecPolicyExactness",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ExecPolicyMode",
      "title": "ExecPolicyMode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ExecutionPhase",
      "title": "ExecutionPhase",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ExecutionPolicy",
      "title": "ExecutionPolicy",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecutionPolicyExtension",
      "title": "ExecutionPolicyExtension",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecutionState",
      "title": "ExecutionState",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExportGuard",
      "title": "ExportGuard",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ExportRecord",
      "title": "ExportRecord",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ExportRequest",
      "title": "ExportRequest",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExportResponse",
      "title": "ExportResponse",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExportScope",
      "title": "ExportScope",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ExportTarget",
      "title": "ExportTarget",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ExportableFilter",
      "title": "ExportableFilter",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ExportableInventory",
      "title": "ExportableInventory",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExternalToolPaths",
      "title": "ExternalToolPaths",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ExtractionPipeline",
      "title": "ExtractionPipeline",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FREVTMT001",
      "title": "FREVTMT001",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-FREVTMT003",
      "title": "FREVTMT003",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-FREVTMT005",
      "title": "FREVTMT005",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-FailureCategory",
      "title": "FailureCategory",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-FeasibilityChecker",
      "title": "FeasibilityChecker",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FileAccessSpec",
      "title": "FileAccessSpec",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Finding",
      "title": "Finding",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FindingSeverity",
      "title": "FindingSeverity",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-FitPreCommitGate",
      "title": "FitPreCommitGate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FlightEvent",
      "title": "FlightEvent",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FlightRecorder",
      "title": "FlightRecorder",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FlightRecorderActor",
      "title": "FlightRecorderActor",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-FlightRecorderEntry",
      "title": "FlightRecorderEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-FlightRecorderEventBase",
      "title": "FlightRecorderEventBase",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-FlightRecorderEventType",
      "title": "FlightRecorderEventType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-FlightRecorderView",
      "title": "FlightRecorderView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-FontManagerView",
      "title": "FontManagerView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-FrozenLaw",
      "title": "FrozenLaw",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Gate",
      "title": "Gate",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-GateConfig",
      "title": "GateConfig",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GatePipeline",
      "title": "GatePipeline",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GateRegistry",
      "title": "GateRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GateResult",
      "title": "GateResult",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-GateStatus",
      "title": "GateStatus",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GateStatusKind",
      "title": "GateStatusKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-GateStatuses",
      "title": "GateStatuses",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GatedMcpClient",
      "title": "GatedMcpClient",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GitWorkflowPolicy",
      "title": "GitWorkflowPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GoldLayerComponents",
      "title": "GoldLayerComponents",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GoldenQuery",
      "title": "GoldenQuery",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GoldenQueryFailedEvent",
      "title": "GoldenQueryFailedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GoldenQuerySpec",
      "title": "GoldenQuerySpec",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GovernanceAutomationEvent",
      "title": "GovernanceAutomationEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GovernanceAutomationEventType",
      "title": "GovernanceAutomationEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-GovernanceDecision",
      "title": "GovernanceDecision",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GovernanceGateTransitionEvent",
      "title": "GovernanceGateTransitionEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GovernanceMode",
      "title": "GovernanceMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-GovernanceModePolicy",
      "title": "GovernanceModePolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GovernancePackExport",
      "title": "GovernancePackExport",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-GovernancePackExportOutcome",
      "title": "GovernancePackExportOutcome",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GovernancePackExportRequest",
      "title": "GovernancePackExportRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GovernancePackExportResponse",
      "title": "GovernancePackExportResponse",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-GovernancePolicyDecision",
      "title": "GovernancePolicyDecision",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GraphArtifact",
      "title": "GraphArtifact",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-GrowthItem",
      "title": "GrowthItem",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-HaltManifest",
      "title": "HaltManifest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Handover",
      "title": "Handover",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-HybridQuery",
      "title": "HybridQuery",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-HybridRetrievalParams",
      "title": "HybridRetrievalParams",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-HybridSearchResult",
      "title": "HybridSearchResult",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-HybridWeights",
      "title": "HybridWeights",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ISO8601Timestamp",
      "title": "ISO8601Timestamp",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ImageFormat",
      "title": "ImageFormat",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ImageMetadata",
      "title": "ImageMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-InboxCollabSubtypeV0_5",
      "title": "InboxCollabSubtypeV0_5",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-IndexRebuiltEvent",
      "title": "IndexRebuiltEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-IndexUpdatedEvent",
      "title": "IndexUpdatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-IngestionMethod",
      "title": "IngestionMethod",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-IngestionSource",
      "title": "IngestionSource",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-IngestionValidationJob",
      "title": "IngestionValidationJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-IntegrityChain",
      "title": "IntegrityChain",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-IntegrityChecker",
      "title": "IntegrityChecker",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-IntegrityLink",
      "title": "IntegrityLink",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-IntentConfirmation",
      "title": "IntentConfirmation",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-IterationRecord",
      "title": "IterationRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-JobFilters",
      "title": "JobFilters",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-JobId",
      "title": "JobId",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-JobKind",
      "title": "JobKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-JobMetrics",
      "title": "JobMetrics",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-JobResultPanel",
      "title": "JobResultPanel",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-JobState",
      "title": "JobState",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-JobStatusUpdate",
      "title": "JobStatusUpdate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-JobsView",
      "title": "JobsView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-JsonRpcMcpClient",
      "title": "JsonRpcMcpClient",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-JsonRpcRequest",
      "title": "JsonRpcRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-JsonRpcResponse",
      "title": "JsonRpcResponse",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-KeywordIndexArtifact",
      "title": "KeywordIndexArtifact",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-KeywordIndexConfig",
      "title": "KeywordIndexConfig",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LanguageCode",
      "title": "LanguageCode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-LawSource",
      "title": "LawSource",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-Layer",
      "title": "Layer",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LayerDocument",
      "title": "LayerDocument",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LayerGate",
      "title": "LayerGate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LayerGroup",
      "title": "LayerGroup",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LayerGuard",
      "title": "LayerGuard",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LayerNode",
      "title": "LayerNode",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LayerScope",
      "title": "LayerScope",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LayerScores",
      "title": "LayerScores",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LedgerStep",
      "title": "LedgerStep",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LensExtractionTier",
      "title": "LensExtractionTier",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-LensFilterEnvelope",
      "title": "LensFilterEnvelope",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LensQueryEnvelope",
      "title": "LensQueryEnvelope",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LensResultItem",
      "title": "LensResultItem",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LinkConfidence",
      "title": "LinkConfidence",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-LiveFilterLayer",
      "title": "LiveFilterLayer",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LlmClient",
      "title": "LlmClient",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-LlmError",
      "title": "LlmError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LlmExecPolicyEvent",
      "title": "LlmExecPolicyEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LlmInferenceEvent",
      "title": "LlmInferenceEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoRASelectionStrategy",
      "title": "LoRASelectionStrategy",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-LoadedSpecPromptPack",
      "title": "LoadedSpecPromptPack",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LocalAdjustment",
      "title": "LocalAdjustment",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LocusCreateWPJob",
      "title": "LocusCreateWPJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LocusCreateWpParams",
      "title": "LocusCreateWpParams",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LocusGetWpStatusParams",
      "title": "LocusGetWpStatusParams",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LocusOperation",
      "title": "LocusOperation",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LocusQueryReadyJob",
      "title": "LocusQueryReadyJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LocusQueryReadyParams",
      "title": "LocusQueryReadyParams",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LocusQueryService",
      "title": "LocusQueryService",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LocusSyncTaskBoardParams",
      "title": "LocusSyncTaskBoardParams",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LogEntry",
      "title": "LogEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LoomAiTagAcceptedEvent",
      "title": "LoomAiTagAcceptedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomAiTagRejectedEvent",
      "title": "LoomAiTagRejectedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomAiTagSuggestedEvent",
      "title": "LoomAiTagSuggestedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomBlock",
      "title": "LoomBlock",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomBlockContentType",
      "title": "LoomBlockContentType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LoomBlockCreatedEvent",
      "title": "LoomBlockCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomBlockDeletedEvent",
      "title": "LoomBlockDeletedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomBlockDerived",
      "title": "LoomBlockDerived",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomBlockSearchResult",
      "title": "LoomBlockSearchResult",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LoomBlockUpdatedEvent",
      "title": "LoomBlockUpdatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomDedupHitEvent",
      "title": "LoomDedupHitEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomEdge",
      "title": "LoomEdge",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomEdgeCreatedEvent",
      "title": "LoomEdgeCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomEdgeDeletedEvent",
      "title": "LoomEdgeDeletedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomEdgeType",
      "title": "LoomEdgeType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-LoomPreviewGeneratedEvent",
      "title": "LoomPreviewGeneratedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomSearchExecutedEvent",
      "title": "LoomSearchExecutedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomSearchFilters",
      "title": "LoomSearchFilters",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LoomSourceAnchor",
      "title": "LoomSourceAnchor",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LoomStorage",
      "title": "LoomStorage",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-LoomViewFilters",
      "title": "LoomViewFilters",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-LoomViewQueriedEvent",
      "title": "LoomViewQueriedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-LoomViewResponse",
      "title": "LoomViewResponse",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-MTContextCompilationConfig",
      "title": "MTContextCompilationConfig",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTEventBase",
      "title": "MTEventBase",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTExecutorLocusIntegration",
      "title": "MTExecutorLocusIntegration",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTIterationCompletedEvent",
      "title": "MTIterationCompletedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTIterationNodeExecution",
      "title": "MTIterationNodeExecution",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTIterationRecord",
      "title": "MTIterationRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTMetrics",
      "title": "MTMetrics",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTMetricsExport",
      "title": "MTMetricsExport",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTProgressEntry",
      "title": "MTProgressEntry",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MTStatus",
      "title": "MTStatus",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MailMessage",
      "title": "MailMessage",
      "kind": "spec_entity"
    },
    {
      "primitive_id": "PRIM-MailThread",
      "title": "MailThread",
      "kind": "spec_entity"
    },
    {
      "primitive_id": "PRIM-Manifest",
      "title": "Manifest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ManifestGate",
      "title": "ManifestGate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ManifestScope",
      "title": "ManifestScope",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MaskDefinition",
      "title": "MaskDefinition",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-McpCall",
      "title": "McpCall",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-McpClient",
      "title": "McpClient",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-McpContext",
      "title": "McpContext",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-McpResourceDescriptor",
      "title": "McpResourceDescriptor",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-McpToolDescriptor",
      "title": "McpToolDescriptor",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MdAuthMode",
      "title": "MdAuthMode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MdSessionRecordV0",
      "title": "MdSessionRecordV0",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MdSessionsRegistryV0",
      "title": "MdSessionsRegistryV0",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MediaDownloaderView",
      "title": "MediaDownloaderView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-MediaSource",
      "title": "MediaSource",
      "kind": "spec_entity"
    },
    {
      "primitive_id": "PRIM-MemoryCommitReport",
      "title": "MemoryCommitReport",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryEventCode",
      "title": "MemoryEventCode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryItemStatusChangedEvent",
      "title": "MemoryItemStatusChangedEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryMutationOp",
      "title": "MemoryMutationOp",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryPack",
      "title": "MemoryPack",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryPackBuiltEvent",
      "title": "MemoryPackBuiltEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryWriteCommittedEvent",
      "title": "MemoryWriteCommittedEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryWriteProposal",
      "title": "MemoryWriteProposal",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryWriteProposedEvent",
      "title": "MemoryWriteProposedEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MemoryWriteReviewedEvent",
      "title": "MemoryWriteReviewedEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MexRegistry",
      "title": "MexRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MexRuntimeError",
      "title": "MexRuntimeError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-MicroStepExecutor",
      "title": "MicroStepExecutor",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MicroTaskDefinition",
      "title": "MicroTaskDefinition",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MicroTaskExecutorJob",
      "title": "MicroTaskExecutorJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MicroTaskIterationRecord",
      "title": "MicroTaskIterationRecord",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MicroTaskLoopCheckpointV1",
      "title": "MicroTaskLoopCheckpointV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MicroTaskMetrics",
      "title": "MicroTaskMetrics",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MicroTaskVerifierOutcomeV1",
      "title": "MicroTaskVerifierOutcomeV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MicroTaskStatus",
      "title": "MicroTaskStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-MicroTaskSummary",
      "title": "MicroTaskSummary",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MicroTaskValidationResult",
      "title": "MicroTaskValidationResult",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MissingEvidence",
      "title": "MissingEvidence",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MirrorSyncState",
      "title": "MirrorSyncState",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MirrorAuthorityMode",
      "title": "MirrorAuthorityMode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MirrorReconciliationAction",
      "title": "MirrorReconciliationAction",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-MarkdownMirrorContractV1",
      "title": "MarkdownMirrorContractV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DevCommandCenterLayoutKind",
      "title": "DevCommandCenterLayoutKind",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ProjectionActionBindingV1",
      "title": "ProjectionActionBindingV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TaskBoardLaneDefinitionV1",
      "title": "TaskBoardLaneDefinitionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-DevCommandCenterViewPresetV1",
      "title": "DevCommandCenterViewPresetV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkflowStateFamily",
      "title": "WorkflowStateFamily",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-WorkflowQueueReasonCode",
      "title": "WorkflowQueueReasonCode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-GovernedActionDescriptorV1",
      "title": "GovernedActionDescriptorV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GovernanceClaimLeaseRecord",
      "title": "GovernanceClaimLeaseRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-GovernanceQueuedInstructionRecord",
      "title": "GovernanceQueuedInstructionRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProjectProfileWorkflowExtensionV1",
      "title": "ProjectProfileWorkflowExtensionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkflowTransitionRuleV1",
      "title": "WorkflowTransitionRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QueueAutomationRuleV1",
      "title": "QueueAutomationRuleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ExecutorEligibilityPolicyV1",
      "title": "ExecutorEligibilityPolicyV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ModeContext",
      "title": "ModeContext",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ModelAssignment",
      "title": "ModelAssignment",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ModelAssignmentCompute",
      "title": "ModelAssignmentCompute",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ModelInterface",
      "title": "ModelInterface",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ModelSession",
      "title": "ModelSession",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ModelSwapEvent",
      "title": "ModelSwapEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ModelSwapEventType",
      "title": "ModelSwapEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ModelSwapRequest",
      "title": "ModelSwapRequest",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ModelSwapRequestV0_4",
      "title": "ModelSwapRequestV0_4",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ModelSwapRequesterV0_4",
      "title": "ModelSwapRequesterV0_4",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ModelSessionSpanBinding",
      "title": "ModelSessionSpanBinding",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-Moodboard",
      "title": "Moodboard",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MoodboardElement",
      "title": "MoodboardElement",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Motif",
      "title": "Motif",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MotifActivation",
      "title": "MotifActivation",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-MultiModelSession",
      "title": "MultiModelSession",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MultimodalEmbedding",
      "title": "MultimodalEmbedding",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-MutationMetadata",
      "title": "MutationMetadata",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-NamingPolicy",
      "title": "NamingPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-NewAsset",
      "title": "NewAsset",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-NextAction",
      "title": "NextAction",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-NextSteps",
      "title": "NextSteps",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-OperationPlan",
      "title": "OperationPlan",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-OverlapConfig",
      "title": "OverlapConfig",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PendingGate",
      "title": "PendingGate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PendingId",
      "title": "PendingId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-PhotoAsset",
      "title": "PhotoAsset",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PhotoDerivedState",
      "title": "PhotoDerivedState",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PhotoStackMCPTools",
      "title": "PhotoStackMCPTools",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PingResponse",
      "title": "PingResponse",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PinnedSlice",
      "title": "PinnedSlice",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PinnedSliceQuery",
      "title": "PinnedSliceQuery",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PixelLayer",
      "title": "PixelLayer",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PlaceholderSourceV1",
      "title": "PlaceholderSourceV1",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-PlaceholderV1",
      "title": "PlaceholderV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PlannedOperation",
      "title": "PlannedOperation",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PolicyDecision",
      "title": "PolicyDecision",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PolicyDecisionOutcome",
      "title": "PolicyDecisionOutcome",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-PostgresDatabase",
      "title": "PostgresDatabase",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PrecedenceLevel",
      "title": "PrecedenceLevel",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-PrecedenceResolver",
      "title": "PrecedenceResolver",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PreviewStatus",
      "title": "PreviewStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ProactiveSurfacing",
      "title": "ProactiveSurfacing",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProblemFilters",
      "title": "ProblemFilters",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProblemGroup",
      "title": "ProblemGroup",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProblemsView",
      "title": "ProblemsView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-ProcessedContent",
      "title": "ProcessedContent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProcessingRecord",
      "title": "ProcessingRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProgrammingLanguage",
      "title": "ProgrammingLanguage",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ProgressArtifact",
      "title": "ProgressArtifact",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProhibitionEnforcer",
      "title": "ProhibitionEnforcer",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProjectIdentity",
      "title": "ProjectIdentity",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProjectProfileExtensionV1",
      "title": "ProjectProfileExtensionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProjectionPlanV0_4",
      "title": "ProjectionPlanV0_4",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PromotionGates",
      "title": "PromotionGates",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PromotionGateSnapshot",
      "title": "PromotionGateSnapshot",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-PromotionPath",
      "title": "PromotionPath",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-PromptEnvelopeTruncationV1",
      "title": "PromptEnvelopeTruncationV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PromptEnvelopeV1",
      "title": "PromptEnvelopeV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-PromptType",
      "title": "PromptType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ProviderRecord",
      "title": "ProviderRecord",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProviderRegistry",
      "title": "ProviderRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ProviderCapabilities",
      "title": "ProviderCapabilities",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ProxyGenerateOptions",
      "title": "ProxyGenerateOptions",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ProxySettings",
      "title": "ProxySettings",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-PruneReport",
      "title": "PruneReport",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-QualityDegradationAlertEvent",
      "title": "QualityDegradationAlertEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QualityRubric",
      "title": "QualityRubric",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-QualitySLOs",
      "title": "QualitySLOs",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QueryAnalysis",
      "title": "QueryAnalysis",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-QueryIntent",
      "title": "QueryIntent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-QueryPlan",
      "title": "QueryPlan",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RBCEvent",
      "title": "RBCEvent",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RID",
      "title": "RID",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RateLimitOutcome",
      "title": "RateLimitOutcome",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RateLimitReservation",
      "title": "RateLimitReservation",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RecorderError",
      "title": "RecorderError",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RedactionMode",
      "title": "RedactionMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RedactionReport",
      "title": "RedactionReport",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ReembeddingTriggeredEvent",
      "title": "ReembeddingTriggeredEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ReferenceDiscovery",
      "title": "ReferenceDiscovery",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Refine3Loop",
      "title": "Refine3Loop",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RegionProcessingRequest",
      "title": "RegionProcessingRequest",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Relationship",
      "title": "Relationship",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RelationshipAnnotations",
      "title": "RelationshipAnnotations",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RelationshipExtractedEvent",
      "title": "RelationshipExtractedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RelationshipId",
      "title": "RelationshipId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RelationshipType",
      "title": "RelationshipType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ReplyFormat",
      "title": "ReplyFormat",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RequeryEngine",
      "title": "RequeryEngine",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RequiredOutputV1",
      "title": "RequiredOutputV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ResolvedProvider",
      "title": "ResolvedProvider",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RepositoryEngineDecisionSurface",
      "title": "RepositoryEngineDecisionSurface",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ResponseBehaviorContract",
      "title": "ResponseBehaviorContract",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ResponseProhibitions",
      "title": "ResponseProhibitions",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RetentionPolicy",
      "title": "RetentionPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RetentionReport",
      "title": "RetentionReport",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RetrievalBudgets",
      "title": "RetrievalBudgets",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RetrievalExecutedEvent",
      "title": "RetrievalExecutedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RetrievalFilters",
      "title": "RetrievalFilters",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RetrievalQualityAuditJob",
      "title": "RetrievalQualityAuditJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RetrievalQualityMetrics",
      "title": "RetrievalQualityMetrics",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RetrievalTrace",
      "title": "RetrievalTrace",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Risk",
      "title": "Risk",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RiskClass",
      "title": "RiskClass",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RoleMailboxBodyV0_5",
      "title": "RoleMailboxBodyV0_5",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxContext",
      "title": "RoleMailboxContext",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RoleMailboxExportManifestV1",
      "title": "RoleMailboxExportManifestV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxExportSummary",
      "title": "RoleMailboxExportSummary",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RoleMailboxExportedEvent",
      "title": "RoleMailboxExportedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxIndexV1",
      "title": "RoleMailboxIndexV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxMessage",
      "title": "RoleMailboxMessage",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RoleMailboxMessageCreatedEvent",
      "title": "RoleMailboxMessageCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxMessageType",
      "title": "RoleMailboxMessageType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RoleMailboxThreadLifecycleState",
      "title": "RoleMailboxThreadLifecycleState",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RoleMailboxMessageDeliveryState",
      "title": "RoleMailboxMessageDeliveryState",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RoleMailboxAllowedResponse",
      "title": "RoleMailboxAllowedResponse",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RoleMailboxActionRequestV1",
      "title": "RoleMailboxActionRequestV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxTriageQueueState",
      "title": "RoleMailboxTriageQueueState",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RoleMailboxReminderScheduleV1",
      "title": "RoleMailboxReminderScheduleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxDeadLetterDisposition",
      "title": "RoleMailboxDeadLetterDisposition",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RoleMailboxExecutorKind",
      "title": "RoleMailboxExecutorKind",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RoleMailboxClaimMode",
      "title": "RoleMailboxClaimMode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RoleMailboxClaimLeaseV1",
      "title": "RoleMailboxClaimLeaseV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxResponseAuthorityV1",
      "title": "RoleMailboxResponseAuthorityV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxHandoffBundleV1",
      "title": "RoleMailboxHandoffBundleV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxAnnounceBackProvenanceV1",
      "title": "RoleMailboxAnnounceBackProvenanceV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleMailboxThread",
      "title": "RoleMailboxThread",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RoleMailboxThreadLineV1",
      "title": "RoleMailboxThreadLineV1",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RoleMailboxTranscribedEvent",
      "title": "RoleMailboxTranscribedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleSuggestionV1",
      "title": "RoleSuggestionV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoleSuggestionsResponseV1",
      "title": "RoleSuggestionsResponseV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RoutingPolicy",
      "title": "RoutingPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RoutingStrategy",
      "title": "RoutingStrategy",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RunLedger",
      "title": "RunLedger",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Runtime",
      "title": "Runtime",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-RuntimeChatEventType",
      "title": "RuntimeChatEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RuntimeChatEventV0_1",
      "title": "RuntimeChatEventV0_1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RuntimeGovernanceMode",
      "title": "RuntimeGovernanceMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-RuntimeMailboxEvent",
      "title": "RuntimeMailboxEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-RuntimeMailboxEventType",
      "title": "RuntimeMailboxEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-RuntimeMailboxTelemetryEvent",
      "title": "RuntimeMailboxTelemetryEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SHA256Hash",
      "title": "SHA256Hash",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ScopeBoundary",
      "title": "ScopeBoundary",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ScopeGate",
      "title": "ScopeGate",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SearchStrategy",
      "title": "SearchStrategy",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SelectionRange",
      "title": "SelectionRange",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SemanticCatalog",
      "title": "SemanticCatalog",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SemanticCatalogEntry",
      "title": "SemanticCatalogEntry",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-SemanticCoherenceJob",
      "title": "SemanticCoherenceJob",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SessionChatLogEntryV0_1",
      "title": "SessionChatLogEntryV0_1",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-SessionCheckpoint",
      "title": "SessionCheckpoint",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-SessionHygiene",
      "title": "SessionHygiene",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SessionLayer",
      "title": "SessionLayer",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-SessionMessage",
      "title": "SessionMessage",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SessionRegistry",
      "title": "SessionRegistry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SessionRisk",
      "title": "SessionRisk",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-SessionSchedulerConfig",
      "title": "SessionSchedulerConfig",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SessionState",
      "title": "SessionState",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ShapeLayer",
      "title": "ShapeLayer",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ShotDna",
      "title": "ShotDna",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SilverId",
      "title": "SilverId",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-SilverRecord",
      "title": "SilverRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SilverRecordCreatedEvent",
      "title": "SilverRecordCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SilverRecordUpdatedEvent",
      "title": "SilverRecordUpdatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SkillBankLogEntry",
      "title": "SkillBankLogEntry",
      "kind": "py_dataclass"
    },
    {
      "primitive_id": "PRIM-SpawnLimits",
      "title": "SpawnLimits",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecArtifact",
      "title": "SpecArtifact",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecGateRule",
      "title": "SpecGateRule",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecIntent",
      "title": "SpecIntent",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecPromptPackV1",
      "title": "SpecPromptPackV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecRouterPromptEnvelopeHashesV1",
      "title": "SpecRouterPromptEnvelopeHashesV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecRoleRule",
      "title": "SpecRoleRule",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecRouterDecision",
      "title": "SpecRouterDecision",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecRouterJobProfile",
      "title": "SpecRouterJobProfile",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecRouterLocusIntegration",
      "title": "SpecRouterLocusIntegration",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SpecRouterPolicy",
      "title": "SpecRouterPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecSessionLogEntry",
      "title": "SpecSessionLogEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SpecStatus",
      "title": "SpecStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-SpecTemplateRule",
      "title": "SpecTemplateRule",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SqliteDatabase",
      "title": "SqliteDatabase",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-StablePrefixSectionV1",
      "title": "StablePrefixSectionV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-StartupSequence",
      "title": "StartupSequence",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-StatefulEditTracker",
      "title": "StatefulEditTracker",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Step",
      "title": "Step",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-StorageGuard",
      "title": "StorageGuard",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-StorageTraits",
      "title": "StorageTraits",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-StructuredCollaborationEnvelopeV1",
      "title": "StructuredCollaborationEnvelopeV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-StructuredCollaborationSummaryV1",
      "title": "StructuredCollaborationSummaryV1",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-SymbolicLayer",
      "title": "SymbolicLayer",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-SymbolicPipeline",
      "title": "SymbolicPipeline",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-SystemStatus",
      "title": "SystemStatus",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-TaskBoardStatus",
      "title": "TaskBoardStatus",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-TaskBoardEntry",
      "title": "TaskBoardEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TaskBoardSections",
      "title": "TaskBoardSections",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TaskEntry",
      "title": "TaskEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TaskLedger",
      "title": "TaskLedger",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TaskLedgerQuery",
      "title": "TaskLedgerQuery",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TaskOutcome",
      "title": "TaskOutcome",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-TaskState",
      "title": "TaskState",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-TemporalMetadata",
      "title": "TemporalMetadata",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TerminalCommandEvent",
      "title": "TerminalCommandEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TextDescriptorRow",
      "title": "TextDescriptorRow",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TextLayer",
      "title": "TextLayer",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ThumbnailSpec",
      "title": "ThumbnailSpec",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TimelineFilters",
      "title": "TimelineFilters",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TimelineView",
      "title": "TimelineView",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-TiptapEditor",
      "title": "TiptapEditor",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-TokenUsage",
      "title": "TokenUsage",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TokenizationService",
      "title": "TokenizationService",
      "kind": "rust_trait"
    },
    {
      "primitive_id": "PRIM-ToolCallEvent",
      "title": "ToolCallEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ToolCallMeta",
      "title": "ToolCallMeta",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ToolEntry",
      "title": "ToolEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ToolPolicy",
      "title": "ToolPolicy",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ToolRegistryEntry",
      "title": "ToolRegistryEntry",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ToolInfrastructureStatus",
      "title": "ToolInfrastructureStatus",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ToolTransportBindings",
      "title": "ToolTransportBindings",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ToolingProfileSelection",
      "title": "ToolingProfileSelection",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ToolsCallRequest",
      "title": "ToolsCallRequest",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TopicBoundary",
      "title": "TopicBoundary",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TrackedDependency",
      "title": "TrackedDependency",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TrackedMicroTask",
      "title": "TrackedMicroTask",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TrackedWorkPacket",
      "title": "TrackedWorkPacket",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-TranscriptionLink",
      "title": "TranscriptionLink",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-TranscriptionTargetKind",
      "title": "TranscriptionTargetKind",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ValidationExecution",
      "title": "ValidationExecution",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ValidationFailedEvent",
      "title": "ValidationFailedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ValidationFinding",
      "title": "ValidationFinding",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-ValidationRecord",
      "title": "ValidationRecord",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ValidationResponse",
      "title": "ValidationResponse",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ValidationResult",
      "title": "ValidationResult",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-ValidationStatus",
      "title": "ValidationStatus",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-VectorClock",
      "title": "VectorClock",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-VectorIndexArtifact",
      "title": "VectorIndexArtifact",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-VectorIndexConfig",
      "title": "VectorIndexConfig",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-VerificationSpec",
      "title": "VerificationSpec",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-VerifySpecResult",
      "title": "VerifySpecResult",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-VersionControl",
      "title": "VersionControl",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-ViewMode",
      "title": "ViewMode",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-ViewModeToggle",
      "title": "ViewModeToggle",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-WPBronze",
      "title": "WPBronze",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WPCreatedEvent",
      "title": "WPCreatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WPSilver",
      "title": "WPSilver",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-Window",
      "title": "Window",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkMode",
      "title": "WorkMode",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-WorkNote",
      "title": "WorkNote",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkPacketActivatedEvent",
      "title": "WorkPacketActivatedEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkPacketBinding",
      "title": "WorkPacketBinding",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkPacketGovernance",
      "title": "WorkPacketGovernance",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkPacketPhase",
      "title": "WorkPacketPhase",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-WorkPacketStatus",
      "title": "WorkPacketStatus",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-WorkPacketType",
      "title": "WorkPacketType",
      "kind": "rust_enum"
    },
    {
      "primitive_id": "PRIM-WorkProfile",
      "title": "WorkProfile",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkProfileEvent",
      "title": "WorkProfileEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkProfileEventType",
      "title": "WorkProfileEventType",
      "kind": "ts_type"
    },
    {
      "primitive_id": "PRIM-WorkflowContext",
      "title": "WorkflowContext",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkflowNodeExecution",
      "title": "WorkflowNodeExecution",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkflowRecoveryEvent",
      "title": "WorkflowRecoveryEvent",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkflowRun",
      "title": "WorkflowRun",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-WorkingContextV1",
      "title": "WorkingContextV1",
      "kind": "rust_struct"
    },
    {
      "primitive_id": "PRIM-Workspace",
      "title": "Workspace",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkspaceRuntimeStatus",
      "title": "WorkspaceRuntimeStatus",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-WorkspaceMember",
      "title": "WorkspaceMember",
      "kind": "ts_interface"
    },
    {
      "primitive_id": "PRIM-WorkspaceSidebar",
      "title": "WorkspaceSidebar",
      "kind": "react_component"
    },
    {
      "primitive_id": "PRIM-PostgresPrimaryControlPlane",
      "title": "PostgresPrimaryControlPlane",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ControlPlaneStorageMode",
      "title": "ControlPlaneStorageMode",
      "kind": "spec_enum"
    },
    {
      "primitive_id": "PRIM-SqliteCacheOfflineBoundary",
      "title": "SqliteCacheOfflineBoundary",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ControlPlaneLease",
      "title": "ControlPlaneLease",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-ModelRunQueueWorker",
      "title": "ModelRunQueueWorker",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-FemsPostgresMemoryStore",
      "title": "FemsPostgresMemoryStore",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-WorkflowPostgresDurableExecution",
      "title": "WorkflowPostgresDurableExecution",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-DccPostgresProjection",
      "title": "DccPostgresProjection",
      "kind": "spec_schema"
    },
    {
      "primitive_id": "PRIM-WriteActor",
      "title": "WriteActor",
      "kind": "rust_enum"
    }
  ],
  "tools": [
    {
      "tool_id": "TOOL-COMFYUI",
      "title": "ComfyUI",
      "notes": ""
    },
    {
      "tool_id": "TOOL-FFMPEG",
      "title": "FFmpeg",
      "notes": ""
    },
    {
      "tool_id": "TOOL-FFPROBE",
      "title": "ffprobe",
      "notes": "[ADD v02.143] Media probe/validation surface."
    },
    {
      "tool_id": "TOOL-GIT",
      "title": "Git",
      "notes": ""
    },
    {
      "tool_id": "TOOL-GITLEAKS",
      "title": "Gitleaks",
      "notes": "[ADD v02.144] Secret scanning surface for supply-chain / workspace artifact hygiene."
    },
    {
      "tool_id": "TOOL-MCP",
      "title": "Model Context Protocol (MCP)",
      "notes": ""
    },
    {
      "tool_id": "TOOL-OLLAMA-API",
      "title": "Ollama API",
      "notes": ""
    },
    {
      "tool_id": "TOOL-OPENAI-COMPAT-API",
      "title": "OpenAI-compatible API",
      "notes": "[ADD v02.143] OpenAI-compatible model runtime surface."
    },
    {
      "tool_id": "TOOL-OSV-SCANNER",
      "title": "OSV-Scanner",
      "notes": "[ADD v02.144] Vulnerability scan surface for dependency inventories."
    },
    {
      "tool_id": "TOOL-SCANCODE",
      "title": "ScanCode",
      "notes": "[ADD v02.144] License / provenance scan surface."
    },
    {
      "tool_id": "TOOL-SYFT",
      "title": "Syft",
      "notes": "[ADD v02.144] SBOM generation surface for supply-chain evidence."
    },
    {
      "tool_id": "TOOL-TREE-SITTER",
      "title": "Tree-sitter",
      "notes": ""
    },
    {
      "tool_id": "TOOL-YTDLP",
      "title": "yt-dlp",
      "notes": ""
    }
  ],
  "technologies": [
    {
      "technology_id": "TECH-AXUM",
      "title": "Axum",
      "notes": "[ADD v02.143] Runtime API router."
    },
    {
      "technology_id": "TECH-BLOCKNOTE",
      "title": "BlockNote",
      "notes": "[ADD v02.144] Block-based document editor stack."
    },
    {
      "technology_id": "TECH-BM25",
      "title": "BM25",
      "notes": ""
    },
    {
      "technology_id": "TECH-CRDT",
      "title": "CRDT",
      "notes": ""
    },
    {
      "technology_id": "TECH-CSP",
      "title": "Content Security Policy (CSP)",
      "notes": ""
    },
    {
      "technology_id": "TECH-DUCKDB",
      "title": "DuckDB",
      "notes": ""
    },
    {
      "technology_id": "TECH-ECHARTS",
      "title": "Apache ECharts",
      "notes": "[ADD v02.144] Chart / dashboard rendering tech."
    },
    {
      "technology_id": "TECH-EXCALIDRAW",
      "title": "Excalidraw",
      "notes": "[ADD v02.143] Canvas/whiteboard tech."
    },
    {
      "technology_id": "TECH-FTS5",
      "title": "SQLite FTS5",
      "notes": ""
    },
    {
      "technology_id": "TECH-HNSW",
      "title": "HNSW",
      "notes": ""
    },
    {
      "technology_id": "TECH-HYPERFORMULA",
      "title": "HyperFormula",
      "notes": "[ADD v02.144] Spreadsheet formula engine."
    },
    {
      "technology_id": "TECH-JSON",
      "title": "JSON",
      "notes": ""
    },
    {
      "technology_id": "TECH-JSON-RPC",
      "title": "JSON-RPC",
      "notes": "[ADD v02.143] MCP wire protocol."
    },
    {
      "technology_id": "TECH-MARKDOWN",
      "title": "Markdown",
      "notes": ""
    },
    {
      "technology_id": "TECH-MCP",
      "title": "MCP",
      "notes": ""
    },
    {
      "technology_id": "TECH-MONACO",
      "title": "Monaco Editor",
      "notes": ""
    },
    {
      "technology_id": "TECH-OLLAMA",
      "title": "Ollama",
      "notes": ""
    },
    {
      "technology_id": "TECH-OPENAI-COMPAT",
      "title": "OpenAI-compatible protocol",
      "notes": "[ADD v02.143] Provider abstraction protocol."
    },
    {
      "technology_id": "TECH-POSTGRESQL",
      "title": "PostgreSQL",
      "notes": ""
    },
    {
      "technology_id": "TECH-PPTXGENJS",
      "title": "PptxGenJS",
      "notes": "[ADD v02.144] Deck export / generation tech."
    },
    {
      "technology_id": "TECH-REACT",
      "title": "React",
      "notes": ""
    },
    {
      "technology_id": "TECH-REVEALJS",
      "title": "Reveal.js",
      "notes": "[ADD v02.144] Web deck / presentation runtime."
    },
    {
      "technology_id": "TECH-RUST",
      "title": "Rust",
      "notes": ""
    },
    {
      "technology_id": "TECH-SHA256",
      "title": "SHA-256",
      "notes": ""
    },
    {
      "technology_id": "TECH-SQLITE",
      "title": "SQLite",
      "notes": ""
    },
    {
      "technology_id": "TECH-SQLX",
      "title": "sqlx",
      "notes": "[ADD v02.143] Storage portability layer."
    },
    {
      "technology_id": "TECH-TAURI",
      "title": "Tauri",
      "notes": ""
    },
    {
      "technology_id": "TECH-TIPTAP",
      "title": "Tiptap",
      "notes": "[ADD v02.143] Document editor tech."
    },
    {
      "technology_id": "TECH-WEBSOCKET",
      "title": "WebSocket",
      "notes": ""
    },
    {
      "technology_id": "TECH-WHISPER",
      "title": "Whisper",
      "notes": "[ADD v02.144] Primary ASR model/runtime family."
    },
    {
      "technology_id": "TECH-WOLF-TABLE",
      "title": "Wolf Table",
      "notes": "[ADD v02.144] Spreadsheet grid/table surface."
    },
    {
      "technology_id": "TECH-ZIP",
      "title": "ZIP",
      "notes": ""
    }
  ],
  "feature_links": [
    {
      "feature_id": "FEAT-ACE-RUNTIME",
      "primitive_ids": [
        "PRIM-AceRuntimeValidator",
        "PRIM-ContextPackFreshnessPolicyV1",
        "PRIM-ContextPackPayloadV1",
        "PRIM-ContextPackRecord",
        "PRIM-DeterminismMode",
        "PRIM-QueryPlan",
        "PRIM-RetrievalBudgets",
        "PRIM-RetrievalFilters",
        "PRIM-RetrievalTrace",
        "PRIM-TokenizationService"
      ],
      "tool_ids": [
        "TOOL-OLLAMA-API"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-RUST"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md Appendix 12.3 FEAT-ACE-RUNTIME",
        "src/backend/handshake_core/src/ace/mod.rs",
        "src/backend/handshake_core/src/workflows.rs"
      ],
      "gap_stub_ids": [
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-AI-JOB-MODEL",
      "primitive_ids": [
        "PRIM-AiJob",
        "PRIM-AiJobMcpFields",
        "PRIM-AiJobMcpUpdate",
        "PRIM-AiJobsDrawer",
        "PRIM-JobKind",
        "PRIM-JobMetrics",
        "PRIM-JobResultPanel",
        "PRIM-JobState",
        "PRIM-ModelSession",
        "PRIM-RateLimitOutcome",
        "PRIM-RateLimitReservation",
        "PRIM-RoutingStrategy",
        "PRIM-SessionMessage",
        "PRIM-SessionSchedulerConfig",
        "PRIM-SpawnLimits",
        "PRIM-SystemStatus",
        "PRIM-WorkflowContext",
        "PRIM-WorkflowNodeExecution",
        "PRIM-WorkflowRun",
        "PRIM-ProviderRecord",
        "PRIM-ProviderRegistry",
        "PRIM-ResolvedProvider",
        "PRIM-RoutingPolicy",
        "PRIM-SessionRegistry",
        "PRIM-AiJobListFilter",
        "PRIM-CreateJobRequest",
        "PRIM-JobStatusUpdate",
        "PRIM-CloudEscalationConsentRequest"
      ],
      "tool_ids": [
        "TOOL-OPENAI-COMPAT-API",
        "TOOL-OLLAMA-API"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-OPENAI-COMPAT",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.150.md Appendix 12.3 FEAT-AI-JOB-MODEL",
        "src/backend/handshake_core/src/storage/mod.rs",
        "src/backend/handshake_core/src/api/jobs.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "app/src/components/AiJobsDrawer.tsx",
        "app/src/components/operator/JobsView.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Workflow-Projection-Correlation-v1",
        "WP-1-Consent-Audit-Projection-v1"
      ]
    },
    {
      "feature_id": "FEAT-AI-READY-DATA",
      "primitive_ids": [
        "PRIM-AiReadyDataPipeline",
        "PRIM-BronzeRecord",
        "PRIM-DocIngestResult",
        "PRIM-DocIngestSpec",
        "PRIM-EmbeddingRecord",
        "PRIM-EmbeddingRegistry",
        "PRIM-GoldenQuerySpec",
        "PRIM-HybridRetrievalParams",
        "PRIM-HybridWeights",
        "PRIM-ProcessingRecord",
        "PRIM-QueryPlan",
        "PRIM-RetrievalTrace",
        "PRIM-SilverRecord",
        "PRIM-ValidationRecord",
        "PRIM-EmbeddingArtifact",
        "PRIM-VectorIndexArtifact",
        "PRIM-KeywordIndexArtifact",
        "PRIM-GraphArtifact",
        "PRIM-EmbeddingModelStatus",
        "PRIM-ValidationStatus"
      ],
      "tool_ids": [
        "TOOL-TREE-SITTER"
      ],
      "technology_ids": [
        "TECH-BM25",
        "TECH-HNSW",
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md Appendix 12.3 FEAT-AI-READY-DATA",
        "src/backend/handshake_core/src/ai_ready_data/pipeline.rs",
        "src/backend/handshake_core/src/ai_ready_data/retrieval.rs",
        "src/backend/handshake_core/src/ai_ready_data/embedding.rs",
        "src/backend/handshake_core/src/ai_ready_data/indexing.rs",
        "src/backend/handshake_core/src/ai_ready_data/records.rs"
      ],
      "gap_stub_ids": [
        "WP-1-AIReady-CoreMetadata-v1",
        "WP-1-AIReady-RelationshipIds-GraphRetrieval-v1",
        "WP-1-AIReady-Index-Evidence-Export-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-ASR",
      "primitive_ids": [
        "PRIM-MediaSource"
      ],
      "tool_ids": [
        "TOOL-FFMPEG",
        "TOOL-FFPROBE"
      ],
      "technology_ids": [
        "TECH-WHISPER"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.158.md 6.2 Speech Recognition: ASR Subsystem",
        "Handshake_Master_Spec_v02.158.md Appendix 12.3 FEAT-ASR",
        "src/backend/handshake_core/src/capabilities.rs",
        "src/backend/handshake_core/src/storage/mod.rs",
        ".GOV/task_packets/stubs/WP-1-ASR-Transcribe-Media-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-ASR-Transcribe-Media-v1",
        "WP-1-Stage-ASR-Transcript-Lineage-v1"
      ]
    },
    {
      "feature_id": "FEAT-ATELIER-LENS",
      "primitive_ids": [
        "PRIM-AtelierCollaborationPanel",
        "PRIM-AtelierScopeError",
        "PRIM-DocumentView",
        "PRIM-SelectionRange",
        "PRIM-TiptapEditor",
        "PRIM-ViewModeToggle",
        "PRIM-RoleSuggestionV1",
        "PRIM-RoleSuggestionsResponseV1"
      ],
      "tool_ids": [
        "TOOL-COMFYUI"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-TIPTAP"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-ATELIER-LENS",
        "app/src/components/AtelierCollaborationPanel.tsx",
        "src/backend/handshake_core/src/ace/validators/atelier_scope.rs",
        "src/backend/handshake_core/src/api/workspaces.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Atelier-Lens-v2",
        "WP-1-Studio-Runtime-Visibility-v1"
      ]
    },
    {
      "feature_id": "FEAT-CALENDAR",
      "primitive_ids": [
        "PRIM-CalendarEvent",
        "PRIM-CalendarEventExportMode",
        "PRIM-CalendarEventStatus",
        "PRIM-CalendarEventUpsert",
        "PRIM-CalendarEventVisibility",
        "PRIM-CalendarEventWindowQuery",
        "PRIM-CalendarMutation",
        "PRIM-CalendarSource",
        "PRIM-CalendarSourceProviderType",
        "PRIM-CalendarSourceSyncState",
        "PRIM-CalendarSourceUpsert",
        "PRIM-CalendarSourceWritePolicy",
        "PRIM-CalendarSyncInput",
        "PRIM-CalendarSyncStateStage"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE",
        "TECH-SQLX"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.155.md 10.4 Calendar",
        "Handshake_Master_Spec_v02.155.md 10.4.2 Calendar â†” ACE Integration (v0.1)",
        "src/backend/handshake_core/src/storage/calendar.rs",
        "src/backend/handshake_core/src/storage/tests.rs",
        "src/backend/handshake_core/src/ace/validators/boundary.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Calendar-Law-Compliance-Tests-v1",
        "WP-1-Calendar-Lens-v3",
        "WP-1-Calendar-Policy-Integration-v1",
        "WP-1-Calendar-Sync-Engine-v1",
        "WP-1-Calendar-Correlation-Export-v1",
        "WP-1-Calendar-Mailbox-Correlation-v1"
      ]
    },
    {
      "feature_id": "FEAT-CANVAS",
      "primitive_ids": [
        "PRIM-CanvasView",
        "PRIM-ExcalidrawCanvas"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-EXCALIDRAW",
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 7.1.2 Freeform Canvas (Milanote-like)",
        "app/src/components/CanvasView.tsx",
        ".GOV/task_packets/stubs/WP-1-Canvas-Runtime-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Canvas-Runtime-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-CAPABILITIES-CONSENT",
      "primitive_ids": [
        "PRIM-CapabilityKind",
        "PRIM-CapabilityRegistry",
        "PRIM-CapabilityRegistryEntry",
        "PRIM-ConsentDecision",
        "PRIM-GateConfig",
        "PRIM-RiskClass",
        "PRIM-ToolPolicy",
        "PRIM-CapabilityProfile",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-CapabilitySnapshotCapabilityV1",
        "PRIM-CapabilitySnapshotToolV1",
        "PRIM-ConsentProvider",
        "PRIM-ConsentScopeV0_4",
        "PRIM-CloudEscalationGuard"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.153.md Appendix 12.3 FEAT-CAPABILITIES-CONSENT",
        "src/backend/handshake_core/src/capabilities.rs",
        "src/backend/handshake_core/src/mcp/gate.rs",
        "src/backend/handshake_core/src/llm/guard.rs",
        "src/backend/handshake_core/src/workflows.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Consent-Audit-Projection-v1"
      ]
    },
    {
      "feature_id": "FEAT-CHARTS-DASHBOARDS",
      "primitive_ids": [
        "PRIM-ChartSpec"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-ECHARTS",
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10.7 Charts & Dashboards",
        ".GOV/task_packets/stubs/WP-1-Charts-Dashboards-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Charts-Dashboards-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "primitive_ids": [
        "PRIM-CloudEscalationBundleV0_4",
        "PRIM-CloudEscalationPolicy",
        "PRIM-ConsentReceiptV0_4",
        "PRIM-ProjectionPlanV0_4",
        "PRIM-RuntimeGovernanceMode",
        "PRIM-ConsentScopeV0_4",
        "PRIM-CloudEscalationGuard",
        "PRIM-CloudEscalationUiSurface"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.153.md Appendix 12.3 FEAT-CLOUD-ESCALATION-CONSENT",
        "src/backend/handshake_core/src/llm/guard.rs",
        "src/backend/handshake_core/src/api/jobs.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "app/src/lib/api.ts",
        "app/src/components/operator/JobsView.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Consent-Audit-Projection-v1",
        "WP-1-Cloud-Consent-Evidence-Portability-v1"
      ]
    },
    {
      "feature_id": "FEAT-CONTEXT-PACKS",
      "primitive_ids": [
        "PRIM-ContextPackAnchorV1",
        "PRIM-ContextPackBuilder",
        "PRIM-ContextPackCoverageV1",
        "PRIM-ContextPackFreshnessDecision",
        "PRIM-ContextPackFreshnessGuard",
        "PRIM-ContextPackFreshnessPolicyV1",
        "PRIM-ContextPackPayloadV1",
        "PRIM-ContextPackRecord"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.157.md 2.5.12 Context Packs AI Job Profile",
        "src/backend/handshake_core/src/ace/mod.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/ace/validators/freshness.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Retrieval-Trace-Bundle-Export-v1",
        "WP-1-ContextPack-Recorder-Visibility-v1"
      ]
    },
    {
      "feature_id": "FEAT-DEBUG-BUNDLE",
      "primitive_ids": [
        "PRIM-BundleExportError",
        "PRIM-BundleScope",
        "PRIM-DebugBundleComplete",
        "PRIM-DebugBundleExport",
        "PRIM-DebugBundleExporter",
        "PRIM-DebugBundleProgress",
        "PRIM-RedactionMode",
        "PRIM-BundleManifest",
        "PRIM-BundleValidationReport",
        "PRIM-DebugBundleRequest",
        "PRIM-ArtifactManifest",
        "PRIM-BundleIndexEntry",
        "PRIM-PolicyDecision",
        "PRIM-PolicyDecisionOutcome",
        "PRIM-ExportScope",
        "PRIM-BundleStatus",
        "PRIM-ExportResponse",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-ExportRequest",
        "PRIM-ExportTarget",
        "PRIM-ExportableInventory",
        "PRIM-RetentionReport"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-ZIP"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.152.md Appendix 12.3 FEAT-DEBUG-BUNDLE",
        "src/backend/handshake_core/src/bundles/schemas.rs",
        "src/backend/handshake_core/src/bundles/exporter.rs",
        "src/backend/handshake_core/src/bundles/validator.rs",
        "src/backend/handshake_core/src/api/bundles.rs",
        "src/backend/handshake_core/src/storage/mod.rs",
        "app/src/components/operator/DebugBundleExport.tsx",
        "app/src/lib/api.ts",
        "src/backend/handshake_core/src/role_mailbox.rs",
        "src/backend/handshake_core/src/ai_ready_data/pipeline.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/mcp/gate.rs",
        "src/backend/handshake_core/src/mex/runtime.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Workflow-Projection-Correlation-v1",
        "WP-1-Consent-Audit-Projection-v1",
        "WP-1-Calendar-Correlation-Export-v1",
        "WP-1-Stage-Media-Artifact-Portability-v1",
        "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1",
        "WP-1-AIReady-Index-Evidence-Export-v1",
        "WP-1-Spec-Router-Evidence-Portability-v1",
        "WP-1-Locus-Debug-Bundle-Bridge-v1",
        "WP-1-MCP-MEX-Evidence-Export-v1"
      ]
    },
    {
      "feature_id": "FEAT-DEV-COMMAND-CENTER",
      "primitive_ids": [
        "PRIM-Ans001TimelineDrawer",
        "PRIM-DevCommandRunHistoryEntry",
        "PRIM-CommandPalette",
        "PRIM-DebugPanel",
        "PRIM-EvidenceDrawer",
        "PRIM-GovernancePackExport",
        "PRIM-BundleManifest",
        "PRIM-BundleStatus",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-DiagnosticsQuery",
        "PRIM-ExportRecord",
        "PRIM-JobsView",
        "PRIM-TimelineView",
        "PRIM-WorkPacketBinding",
        "PRIM-JobFilters",
        "PRIM-PinnedSlice",
        "PRIM-PinnedSliceQuery",
        "PRIM-ProblemFilters",
        "PRIM-SessionChatLogEntryV0_1",
        "PRIM-TimelineFilters",
        "PRIM-EvidenceSelection",
        "PRIM-GovernancePackExportRequest",
        "PRIM-GovernancePackExportOutcome",
        "PRIM-GovernancePackExportResponse",
        "PRIM-ModelSession",
        "PRIM-MultiModelSession",
        "PRIM-TrackedWorkPacket",
        "PRIM-MicroTaskSummary",
        "PRIM-SpecSessionLogEntry",
        "PRIM-TaskBoardEntry",
        "PRIM-TaskBoardStatus",
        "PRIM-LocusQueryReadyParams",
        "PRIM-LocusGetWpStatusParams",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-SessionRegistry",
        "PRIM-SessionCheckpoint",
        "PRIM-ProviderCapabilities",
        "PRIM-ModelSessionSpanBinding",
        "PRIM-AntiPatternAlert",
        "PRIM-RepositoryEngineDecisionSurface",
        "PRIM-WorkspaceRuntimeStatus",
        "PRIM-PromotionGateSnapshot",
        "PRIM-WorkflowRun",
        "PRIM-WorkflowNodeExecution",
        "PRIM-ToolInfrastructureStatus",
        "PRIM-JobStatusUpdate",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-GovernanceDecision",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tool_ids": [
        "TOOL-GIT"
      ],
      "technology_ids": [
        "TECH-REACT",
        "TECH-TAURI"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-DEV-COMMAND-CENTER",
        "app/src/components/operator/JobsView.tsx",
        "app/src/components/operator/TimelineView.tsx",
        "app/src/components/operator/ProblemsView.tsx",
        "app/src/components/operator/DebugBundleExport.tsx",
        "app/src/components/operator/Ans001TimelineDrawer.tsx",
        "app/src/components/operator/GovernancePackExport.tsx",
        "src/backend/handshake_core/src/locus/task_board.rs",
        "src/backend/handshake_core/src/role_mailbox.rs",
        "src/backend/handshake_core/src/api/governance_pack.rs",
        "src/backend/handshake_core/src/api/bundles.rs",
        "src/backend/handshake_core/src/api/diagnostics.rs",
        "src/backend/handshake_core/src/bundles/exporter.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/locus/types.rs",
        "src/backend/handshake_core/tests/micro_task_executor_tests.rs",
        "src/backend/handshake_core/src/api/jobs.rs",
        "src/backend/handshake_core/src/api/workspaces.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Dev-Command-Center-MVP-v1",
        "WP-1-Workflow-Projection-Correlation-v1",
        "WP-1-Consent-Audit-Projection-v1",
        "WP-1-Governance-Pack-v1",
        "WP-1-Workspace-Bundle-v2",
        "WP-1-Diagnostics-Debug-Bundle-Bridge-v1",
        "WP-1-MTE-Summaries-v1",
        "WP-1-Provider-Feature-Coverage-Agentic-Ready-v1",
        "WP-1-Session-Crash-Recovery-Checkpointing-v1",
        "WP-1-Session-Observability-Spans-FR-v1",
        "WP-1-Session-Anti-Pattern-Registry-v1",
        "WP-1-Git-Engine-Decision-Gate-v1",
        "WP-1-Workspace-Safety-Parallel-Sessions-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1",
        "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        "WP-1-Role-Mailbox-Triage-Queue-Controls-v1"
      ]
    },
    {
      "feature_id": "FEAT-DIAGNOSTICS-SCHEMA",
      "primitive_ids": [
        "PRIM-FindingSeverity",
        "PRIM-ProblemsView",
        "PRIM-ValidationFinding",
        "PRIM-DiagnosticsQuery",
        "PRIM-DiagnosticStatus",
        "PRIM-DiagFilter",
        "PRIM-ProblemGroup"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.161.md Appendix 12.3 FEAT-DIAGNOSTICS-SCHEMA",
        "app/src/components/operator/ProblemsView.tsx",
        "src/backend/handshake_core/src/diagnostics/mod.rs",
        "src/backend/handshake_core/src/api/diagnostics.rs",
        "src/backend/handshake_core/src/bundles/exporter.rs",
        "app/src/components/operator/Ans001TimelineDrawer.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Diagnostics-Debug-Bundle-Bridge-v1"
      ]
    },
    {
      "feature_id": "FEAT-DOCS-SHEETS",
      "primitive_ids": [
        "PRIM-DocsAiJobProfile",
        "PRIM-DocumentView",
        "PRIM-TiptapEditor"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-BLOCKNOTE",
        "TECH-HYPERFORMULA",
        "TECH-TIPTAP",
        "TECH-WOLF-TABLE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 2.5.10 Docs & Sheets AI Job Profile",
        "Handshake_Master_Spec_v02.144.md 7.1 Rich Content Worksurfaces",
        ".GOV/task_packets/stubs/WP-1-Docs-Sheets-Runtime-Backfill-v1.md",
        "Handshake_Master_Spec_v02.146.md Appendix 12.3 FEAT-DOCS-SHEETS",
        "app/src/components/TiptapEditor.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Docs-Sheets-Runtime-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-FLIGHT-RECORDER",
      "primitive_ids": [
        "PRIM-FlightRecorder",
        "PRIM-FlightRecorderView",
        "PRIM-RecorderError",
        "PRIM-EventFilter",
        "PRIM-FlightRecorderEventType",
        "PRIM-FlightEvent"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-DUCKDB",
        "TECH-JSON"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.161.md Appendix 12.3 FEAT-FLIGHT-RECORDER",
        "app/src/components/FlightRecorderView.tsx",
        "app/src/lib/api.ts",
        "src/backend/handshake_core/src/flight_recorder/mod.rs",
        "src/backend/handshake_core/src/api/flight_recorder.rs",
        "src/backend/handshake_core/src/role_mailbox.rs",
        "src/backend/handshake_core/src/ai_ready_data/pipeline.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/mcp/gate.rs",
        "src/backend/handshake_core/src/mcp/fr_events.rs",
        "src/backend/handshake_core/src/mex/runtime.rs"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-GOVERNANCE-PACK",
      "primitive_ids": [
        "PRIM-GovernancePackExport",
        "PRIM-GovernancePackExportRequest",
        "PRIM-GovernancePackExportResponse",
        "PRIM-GovernancePackExportOutcome",
        "PRIM-ExportRecord",
        "PRIM-ExportTarget",
        "PRIM-ArtifactManifest"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-GOVERNANCE-PACK",
        "src/backend/handshake_core/src/governance_pack.rs",
        "src/backend/handshake_core/src/api/governance_pack.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/flight_recorder/mod.rs",
        "app/src/components/operator/GovernancePackExport.tsx",
        "app/src/components/operator/Ans001TimelineDrawer.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Governance-Pack-v1"
      ]
    },
    {
      "feature_id": "FEAT-LOCUS-WORK-TRACKING",
      "primitive_ids": [
        "PRIM-GateStatus",
        "PRIM-GateStatusKind",
        "PRIM-GateStatuses",
        "PRIM-LocusCreateWpParams",
        "PRIM-LocusOperation",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-MicroTaskSummary",
        "PRIM-TrackedDependency",
        "PRIM-TrackedMicroTask",
        "PRIM-TrackedWorkPacket",
        "PRIM-WorkPacketGovernance",
        "PRIM-WorkPacketPhase",
        "PRIM-WorkPacketType",
        "PRIM-MicroTaskStatus",
        "PRIM-MicroTaskValidationResult",
        "PRIM-MicroTaskIterationRecord",
        "PRIM-LocusQueryReadyParams",
        "PRIM-LocusGetWpStatusParams",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-CRDT",
        "TECH-JSON",
        "TECH-POSTGRESQL",
        "TECH-SQLITE",
        "TECH-SQLX",
        "TECH-WEBSOCKET"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-LOCUS-WORK-TRACKING",
        "src/backend/handshake_core/src/locus/types.rs",
        "src/backend/handshake_core/src/storage/locus_sqlite.rs",
        "src/backend/handshake_core/src/locus/task_board.rs",
        "src/backend/handshake_core/src/flight_recorder/mod.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/tests/micro_task_executor_tests.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Locus-Phase1-Medallion-Search-v1",
        "WP-1-Locus-Phase1-QueryContract-Autosync-v1",
        "WP-1-Workflow-Projection-Correlation-v1",
        "WP-1-Locus-Debug-Bundle-Bridge-v1",
        "WP-1-Structured-Collaboration-Artifact-Family-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        "WP-1-Role-Mailbox-Triage-Queue-Controls-v1"
      ]
    },
    {
      "feature_id": "FEAT-LOOM-LIBRARY",
      "primitive_ids": [
        "PRIM-Asset",
        "PRIM-AssetKind",
        "PRIM-LoomBlockContentType",
        "PRIM-LoomBlockSearchResult",
        "PRIM-LoomEdgeType",
        "PRIM-LoomSearchFilters",
        "PRIM-LoomSourceAnchor",
        "PRIM-LoomViewFilters",
        "PRIM-LoomViewResponse",
        "PRIM-NewAsset",
        "PRIM-PreviewStatus",
        "PRIM-LoomBlock",
        "PRIM-LoomEdge",
        "PRIM-LoomBlockDerived"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-FTS5",
        "TECH-JSON",
        "TECH-SHA256",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md Appendix 12.3 FEAT-LOOM-LIBRARY",
        "Handshake_Master_Spec_v02.179.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example)",
        "src/backend/handshake_core/src/api/loom.rs",
        "src/backend/handshake_core/src/storage/loom.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Loom-Storage-Portability-v4",
        "WP-1-Loom-Preview-VideoPosterFrames-v1",
        "WP-1-Media-Downloader-Loom-Bridge-v1",
        "WP-1-Video-Archive-Loom-Integration-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-MAIL-CLIENT",
      "primitive_ids": [
        "PRIM-MailMessage",
        "PRIM-MailThread"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10.3 Mail Client",
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-MAIL-CLIENT",
        "Handshake_Master_Spec_v02.144.md MailMessage / MailThread schema",
        ".GOV/task_packets/stubs/WP-1-Mail-Runtime-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Mail-Runtime-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-MCP-PRIMITIVES",
      "primitive_ids": [
        "PRIM-AccessMode",
        "PRIM-ConsentDecision",
        "PRIM-GateConfig",
        "PRIM-ToolCallEvent",
        "PRIM-JsonRpcMcpClient",
        "PRIM-McpCall",
        "PRIM-McpClient",
        "PRIM-McpContext",
        "PRIM-McpResourceDescriptor",
        "PRIM-McpToolDescriptor",
        "PRIM-ToolPolicy",
        "PRIM-ToolRegistryEntry",
        "PRIM-ToolTransportBindings",
        "PRIM-JsonRpcRequest",
        "PRIM-JsonRpcResponse"
      ],
      "tool_ids": [
        "TOOL-MCP"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-JSON-RPC",
        "TECH-MCP"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.153.md Appendix 12.3 FEAT-MCP-PRIMITIVES",
        "src/backend/handshake_core/src/mcp/client.rs",
        "src/backend/handshake_core/src/mcp/gate.rs",
        "src/backend/handshake_core/src/mcp/fr_events.rs",
        "src/backend/handshake_core/src/mcp/jsonrpc.rs"
      ],
      "gap_stub_ids": [
        "WP-1-MCP-MEX-Evidence-Export-v1"
      ]
    },
    {
      "feature_id": "FEAT-MEDIA-DOWNLOADER",
      "primitive_ids": [
        "PRIM-MediaDownloaderView",
        "PRIM-MdAuthMode",
        "PRIM-MdSessionRecordV0",
        "PRIM-MdSessionsRegistryV0",
        "PRIM-MediaSource"
      ],
      "tool_ids": [
        "TOOL-FFMPEG",
        "TOOL-FFPROBE",
        "TOOL-YTDLP"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SHA256"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.158.md Appendix 12.3 FEAT-MEDIA-DOWNLOADER",
        "app/src/components/MediaDownloaderView.tsx",
        "app/src-tauri/src/lib.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/mechanical_engines.json",
        "app/src/lib/mediaDownloader.ts"
      ],
      "gap_stub_ids": [
        "WP-1-Media-Downloader-Loom-Bridge-v1",
        "WP-1-Video-Archive-Loom-Integration-v1",
        "WP-1-Stage-Media-Artifact-Portability-v1"
      ]
    },
    {
      "feature_id": "FEAT-MEX-RUNTIME",
      "primitive_ids": [
        "PRIM-AdapterError",
        "PRIM-EngineAdapter",
        "PRIM-EngineResult",
        "PRIM-MexRegistry",
        "PRIM-MexRuntimeError",
        "PRIM-ToolCallEvent"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-JSON-RPC"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.145.md 11.8 Mechanical Extension Specification v1.2 (Verbatim)",
        "Handshake_Master_Spec_v02.152.md Appendix 12.3 FEAT-MEX-RUNTIME",
        "src/backend/handshake_core/src/mex/envelope.rs",
        "src/backend/handshake_core/src/mex/runtime.rs",
        ".GOV/task_packets/stubs/WP-1-MEX-Observability-v2.md"
      ],
      "gap_stub_ids": [
        "WP-1-MEX-Observability-v2",
        "WP-1-MEX-Safety-Gates-v2",
        "WP-1-MEX-UX-Bridges-v2",
        "WP-1-MCP-MEX-Evidence-Export-v1"
      ]
    },
    {
      "feature_id": "FEAT-MICRO-TASK-EXECUTOR",
      "primitive_ids": [
        "PRIM-MicroTaskDefinition",
        "PRIM-MicroTaskExecutorJob",
        "PRIM-MicroTaskLoopCheckpointV1",
        "PRIM-MicroTaskMetrics",
        "PRIM-MicroTaskVerifierOutcomeV1",
        "PRIM-PendingDistillationCandidate",
        "PRIM-TaskOutcome",
        "PRIM-TaskState",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tool_ids": [
        "TOOL-OLLAMA-API"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-OLLAMA"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md Appendix 12.3 FEAT-MICRO-TASK-EXECUTOR",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/tests/micro_task_executor_tests.rs",
        "src/backend/handshake_core/src/locus/types.rs",
        "src/backend/handshake_core/src/flight_recorder/mod.rs"
      ],
      "gap_stub_ids": [
        "WP-1-MTE-Blocked-Decisioning-v1",
        "WP-1-MTE-DropBack-Smart-v1",
        "WP-1-MTE-LoRA-Wiring-v1",
        "WP-1-MTE-Resource-Caps-v1",
        "WP-1-MTE-Summaries-v1",
        "WP-1-Structured-Collaboration-Artifact-Family-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "primitive_ids": [
        "PRIM-ModelSession",
        "PRIM-MultiModelSession",
        "PRIM-ModelSwapRequestV0_4",
        "PRIM-ModelSwapRequesterV0_4",
        "PRIM-ProviderRecord",
        "PRIM-ProviderRegistry",
        "PRIM-ProviderCapabilities",
        "PRIM-ResolvedProvider",
        "PRIM-RoutingPolicy",
        "PRIM-ModelSessionSpanBinding",
        "PRIM-ActivitySpanBinding",
        "PRIM-SessionCheckpoint",
        "PRIM-SessionRegistry",
        "PRIM-SessionSchedulerConfig"
      ],
      "tool_ids": [
        "TOOL-OLLAMA-API",
        "TOOL-OPENAI-COMPAT-API"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.150.md 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]",
        "Handshake_Master_Spec_v02.164.md Appendix 12.3 FEAT-MODEL-SESSION-ORCHESTRATION",
        "src/backend/handshake_core/src/llm/registry.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/locus/types.rs",
        "src/backend/handshake_core/tests/micro_task_executor_tests.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Session-Spawn-Contract-v1",
        "WP-1-Provider-Feature-Coverage-Agentic-Ready-v1",
        "WP-1-Workspace-Safety-Parallel-Sessions-v1",
        "WP-1-Session-Crash-Recovery-Checkpointing-v1",
        "WP-1-Session-Observability-Spans-FR-v1",
        "WP-1-Session-Anti-Pattern-Registry-v1"
      ]
    },
    {
      "feature_id": "FEAT-MONACO-EDITOR",
      "primitive_ids": [
        "PRIM-SelectionRange"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-MONACO"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10.5 Monaco Editor Experience",
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-MONACO-EDITOR"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-OPERATOR-CONSOLES",
      "primitive_ids": [
        "PRIM-Ans001TimelineDrawer",
        "PRIM-BundleScope",
        "PRIM-DebugBundleComplete",
        "PRIM-DebugBundleExport",
        "PRIM-DebugBundleExporter",
        "PRIM-DebugBundleProgress",
        "PRIM-DebugPanel",
        "PRIM-EvidenceDrawer",
        "PRIM-GovernancePackExport",
        "PRIM-JobsView",
        "PRIM-ProblemsView",
        "PRIM-RedactionMode",
        "PRIM-TimelineView",
        "PRIM-EvidenceSelection",
        "PRIM-ExportableFilter",
        "PRIM-ExportRecord",
        "PRIM-GovernancePackExportRequest",
        "PRIM-GovernancePackExportOutcome",
        "PRIM-DiagnosticsQuery",
        "PRIM-BundleExportRequest",
        "PRIM-BundleExportResponse",
        "PRIM-GovernancePackExportResponse"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-REACT",
        "TECH-TAURI"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.159.md Appendix 12.3 FEAT-OPERATOR-CONSOLES",
        "app/src/App.tsx",
        "app/src/components/operator/ProblemsView.tsx",
        "app/src/components/operator/JobsView.tsx",
        "app/src/components/operator/TimelineView.tsx",
        "app/src/components/operator/EvidenceDrawer.tsx",
        "app/src/components/operator/DebugBundleExport.tsx",
        "app/src/components/operator/GovernancePackExport.tsx",
        "src/backend/handshake_core/src/api/diagnostics.rs",
        "src/backend/handshake_core/src/api/bundles.rs",
        "src/backend/handshake_core/src/api/governance_pack.rs"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-PHOTO-STUDIO",
      "primitive_ids": [
        "PRIM-CanvasView",
        "PRIM-ExcalidrawCanvas",
        "PRIM-FontManagerView",
        "PRIM-ViewModeToggle",
        "PRIM-WorkspaceSidebar"
      ],
      "tool_ids": [
        "TOOL-COMFYUI"
      ],
      "technology_ids": [
        "TECH-EXCALIDRAW",
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10.10 Photo Studio",
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-PHOTO-STUDIO",
        "app/src/components/CanvasView.tsx",
        "Handshake_Master_Spec_v02.144.md 10.10.5.2 ComfyUI Integration Scope"
      ],
      "gap_stub_ids": [
        "WP-1-Photo-Studio-v2",
        "WP-1-Studio-Runtime-Visibility-v1"
      ]
    },
    {
      "feature_id": "FEAT-PRESENTATIONS-DECKS",
      "primitive_ids": [
        "PRIM-DeckSpec"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-PPTXGENJS",
        "TECH-REVEALJS",
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10.8 Presentations (Decks)",
        ".GOV/task_packets/stubs/WP-1-Presentations-Decks-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Presentations-Decks-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-PROJECT-BRAIN",
      "primitive_ids": [
        "PRIM-ContextPackPayloadV1",
        "PRIM-QueryPlan",
        "PRIM-RetrievalTrace"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-BM25",
        "TECH-FTS5",
        "TECH-HNSW",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md 2.5.8 Project Brain (RAG Interface)",
        "src/backend/handshake_core/src/workflows.rs",
        ".GOV/task_packets/stubs/WP-1-Project-Brain-Runtime-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Project-Brain-Runtime-Backfill-v1",
        "WP-1-Retrieval-Trace-Bundle-Export-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-ROLE-MAILBOX",
      "primitive_ids": [
        "PRIM-RoleMailboxContext",
        "PRIM-RoleMailboxMessage",
        "PRIM-RoleMailboxMessageType",
        "PRIM-RoleMailboxThreadLifecycleState",
        "PRIM-RoleMailboxMessageDeliveryState",
        "PRIM-RoleMailboxAllowedResponse",
        "PRIM-RoleMailboxActionRequestV1",
        "PRIM-RoleMailboxTriageQueueState",
        "PRIM-RoleMailboxReminderScheduleV1",
        "PRIM-RoleMailboxDeadLetterDisposition",
        "PRIM-RoleMailboxHandoffBundleV1",
        "PRIM-RoleMailboxAnnounceBackProvenanceV1",
        "PRIM-MicroTaskLoopCheckpointV1",
        "PRIM-MicroTaskVerifierOutcomeV1",
        "PRIM-RoleMailboxThread",
        "PRIM-CreateRoleMailboxMessageRequest",
        "PRIM-AddTranscriptionLinkRequest",
        "PRIM-RoleMailboxExportSummary",
        "PRIM-RoleMailboxIndexV1",
        "PRIM-RoleMailboxThreadLineV1",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        ".GOV/ROLE_MAILBOX",
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-ROLE-MAILBOX",
        "src/backend/handshake_core/src/role_mailbox.rs",
        "src/backend/handshake_core/src/api/role_mailbox.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1",
        "WP-1-Structured-Collaboration-Artifact-Family-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        "WP-1-Role-Mailbox-Triage-Queue-Controls-v1",
        "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1",
        "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1"
      ]
    },
    {
      "feature_id": "FEAT-SEMANTIC-CATALOG",
      "primitive_ids": [
        "PRIM-SemanticCatalog",
        "PRIM-SemanticCatalogEntry"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.156.md 2.6.7 Semantic Catalog Registry (Normative)",
        "Handshake_Master_Spec_v02.156.md 2.6.8 Prompt-to-Spec Governance Pipeline (Normative)",
        "src/backend/handshake_core/src/workflows.rs",
        ".GOV/task_packets/stubs/WP-1-Semantic-Catalog-v2.md"
      ],
      "gap_stub_ids": [
        "WP-1-Semantic-Catalog-v2",
        "WP-1-Spec-Router-CapabilitySnapshot-v1"
      ]
    },
    {
      "feature_id": "FEAT-SKILL-BANK",
      "primitive_ids": [
        "PRIM-AdapterCheckpoint",
        "PRIM-DistillationCandidate",
        "PRIM-SkillBankLogEntry"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.157.md 9 Continuous Local Skill Distillation (Skill Bank & Pipeline)",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/tests/micro_task_executor_tests.rs",
        ".GOV/task_packets/stubs/WP-1-Distillation-v2.md"
      ],
      "gap_stub_ids": [
        "WP-1-Distillation-v2",
        "WP-1-MTE-LoRA-Wiring-v1"
      ]
    },
    {
      "feature_id": "FEAT-SPEC-APPENDICES",
      "primitive_ids": [],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-MARKDOWN"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        ".GOV/scripts/validation/spec-eof-appendices-check.mjs",
        "Handshake_Master_Spec_v02.144.md Appendix 12",
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-SPEC-APPENDICES"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-SPEC-ROUTER",
      "primitive_ids": [
        "PRIM-BudgetsV1",
        "PRIM-CapabilitySnapshotV1",
        "PRIM-CommandPalette",
        "PRIM-ContextBlockV1",
        "PRIM-LoadedSpecPromptPack",
        "PRIM-LocusCreateWpParams",
        "PRIM-PlaceholderSourceV1",
        "PRIM-PlaceholderV1",
        "PRIM-PromptEnvelopeTruncationV1",
        "PRIM-PromptEnvelopeV1",
        "PRIM-RequiredOutputV1",
        "PRIM-SpecArtifact",
        "PRIM-SpecIntent",
        "PRIM-SpecPromptPackV1",
        "PRIM-SpecRouterPromptEnvelopeHashesV1",
        "PRIM-SpecRouterDecision",
        "PRIM-SpecSessionLogEntry",
        "PRIM-StablePrefixSectionV1",
        "PRIM-WorkingContextV1"
      ],
      "tool_ids": [
        "TOOL-GIT"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SHA256"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.179.md Appendix 12.3 FEAT-SPEC-ROUTER",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/spec_router/spec_prompt_compiler.rs",
        "src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Spec-Router-Evidence-Portability-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-STAGE",
      "primitive_ids": [
        "PRIM-MdAuthMode",
        "PRIM-MdSessionRecordV0",
        "PRIM-MdSessionsRegistryV0"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-CSP",
        "TECH-JSON",
        "TECH-TAURI"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.158.md 10.13 Handshake Stage (Built-in Browser + Stage Apps)",
        "Handshake_Master_Spec_v02.158.md Appendix 12.3 FEAT-STAGE",
        "app/src-tauri/src/lib.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "app/src/components/MediaDownloaderView.tsx",
        "app/src/lib/mediaDownloader.ts"
      ],
      "gap_stub_ids": [
        "WP-1-Handshake-Stage-MVP-v1",
        "WP-1-Stage-Media-Artifact-Portability-v1",
        "WP-1-Stage-ASR-Transcript-Lineage-v1"
      ]
    },
    {
      "feature_id": "FEAT-STORAGE-PORTABILITY",
      "primitive_ids": [
        "PRIM-Database",
        "PRIM-PostgresDatabase",
        "PRIM-SqliteDatabase",
        "PRIM-PostgresPrimaryControlPlane",
        "PRIM-ControlPlaneStorageMode",
        "PRIM-SqliteCacheOfflineBoundary",
        "PRIM-ControlPlaneLease",
        "PRIM-ManifestScope",
        "PRIM-BundleManifestFile",
        "PRIM-MissingEvidence",
        "PRIM-ArtifactManifest",
        "PRIM-BundleIndexEntry",
        "PRIM-PruneReport",
        "PRIM-RetentionPolicy",
        "PRIM-RetentionReport"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-POSTGRESQL",
        "TECH-SQLITE",
        "TECH-SQLX"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.153.md Appendix 12.3 FEAT-STORAGE-PORTABILITY",
        "src/backend/handshake_core/src/storage/mod.rs",
        "src/backend/handshake_core/src/storage/retention.rs",
        "src/backend/handshake_core/src/storage/sqlite.rs",
        "src/backend/handshake_core/src/storage/postgres.rs",
        "src/backend/handshake_core/src/bundles/schemas.rs",
        "src/backend/handshake_core/src/bundles/validator.rs",
        "src/backend/handshake_core/src/ai_ready_data/embedding.rs",
        "src/backend/handshake_core/src/ai_ready_data/indexing.rs",
        "src/backend/handshake_core/src/role_mailbox.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/mcp/gate.rs",
        "Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-Primary Control-Plane Foundation"
      ],
      "gap_stub_ids": [
        "WP-1-Stage-Media-Artifact-Portability-v1",
        "WP-1-AIReady-Index-Evidence-Export-v1",
        "WP-1-Spec-Router-Evidence-Portability-v1",
        "WP-1-MCP-MEX-Evidence-Export-v1",
        "WP-1-Cloud-Consent-Evidence-Portability-v1",
        "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
        "WP-1-Postgres-Control-Plane-Leases-Backpressure-v1",
        "WP-1-ModelSession-Postgres-Queue-Workers-v1",
        "WP-1-FEMS-Postgres-Memory-Store-v1",
        "WP-1-Workflow-Engine-Postgres-Durable-Execution-v1",
        "WP-1-DCC-Postgres-Control-Plane-Projections-v1",
        "WP-1-SQLite-Cache-Offline-Boundaries-v1"
      ]
    },
    {
      "feature_id": "FEAT-STUDIO",
      "primitive_ids": [
        "PRIM-CanvasView",
        "PRIM-RoleSuggestionV1",
        "PRIM-RoleSuggestionsResponseV1"
      ],
      "tool_ids": [
        "TOOL-COMFYUI"
      ],
      "technology_ids": [
        "TECH-EXCALIDRAW",
        "TECH-JSON",
        "TECH-TAURI"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.158.md 6.3.3 Domain 2: Creative Studio",
        "Handshake_Master_Spec_v02.158.md 10.4 Stage Studio direction (Phase 3/4 MAY)",
        "app/src/components/CanvasView.tsx",
        "app/src/components/AtelierCollaborationPanel.tsx",
        ".GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Studio-Runtime-Visibility-v1"
      ]
    },
    {
      "feature_id": "FEAT-TASK-BOARD",
      "primitive_ids": [
        "PRIM-TaskBoardEntry",
        "PRIM-TaskBoardSections",
        "PRIM-TaskBoardStatus",
        "PRIM-LocusSyncTaskBoardParams",
        "PRIM-SpecSessionLogEntry",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1",
        "PRIM-TaskBoardLaneDefinitionV1",
        "PRIM-ProjectionActionBindingV1",
        "PRIM-DevCommandCenterViewPresetV1"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-MARKDOWN"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md 2.6.8.8 Spec Session Log (Task Board + Work Packets) (Normative)",
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-TASK-BOARD",
        "src/backend/handshake_core/src/locus/task_board.rs",
        "src/backend/handshake_core/src/locus/types.rs",
        "src/backend/handshake_core/src/workflows.rs",
        ".GOV/roles_shared/records/TASK_BOARD.md"
      ],
      "gap_stub_ids": [
        "WP-1-Locus-Work-Tracking-System-Phase1-v1",
        "WP-1-Locus-Phase1-QueryContract-Autosync-v1",
        "WP-1-Dev-Command-Center-MVP-v1",
        "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1",
        "WP-1-Structured-Collaboration-Artifact-Family-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1"
      ]
    },
    {
      "feature_id": "FEAT-TERMINAL",
      "primitive_ids": [
        "PRIM-TerminalCommandEvent"
      ],
      "tool_ids": [
        "TOOL-GIT"
      ],
      "technology_ids": [
        "TECH-JSON"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 10 terminal experience",
        "Handshake_Master_Spec_v02.144.md Appendix 12.3 FEAT-TERMINAL",
        "src/backend/handshake_core/src/terminal/mod.rs",
        "src/backend/handshake_core/src/flight_recorder/mod.rs"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-THINKING-PIPELINE",
      "primitive_ids": [
        "PRIM-CanvasView",
        "PRIM-ContextPackPayloadV1",
        "PRIM-DocumentView",
        "PRIM-WorkflowRun"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.144.md 2.5.9 Thinking Pipeline (Docs ? Canvas ? Workflows)",
        ".GOV/task_packets/stubs/WP-1-Thinking-Pipeline-Runtime-Backfill-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Thinking-Pipeline-Runtime-Backfill-v1"
      ]
    },
    {
      "feature_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "primitive_ids": [
        "PRIM-FlightRecorderView",
        "PRIM-GateConfig",
        "PRIM-JobsView",
        "PRIM-JsonRpcMcpClient",
        "PRIM-McpCall",
        "PRIM-McpContext",
        "PRIM-McpResourceDescriptor",
        "PRIM-McpToolDescriptor",
        "PRIM-ToolCallMeta",
        "PRIM-ToolEntry",
        "PRIM-ToolInfrastructureStatus",
        "PRIM-ToolPolicy",
        "PRIM-ToolRegistryEntry",
        "PRIM-ToolTransportBindings",
        "PRIM-ToolsCallRequest"
      ],
      "tool_ids": [
        "TOOL-MCP",
        "TOOL-OLLAMA-API",
        "TOOL-OPENAI-COMPAT-API"
      ],
      "technology_ids": [
        "TECH-JSON",
        "TECH-JSON-RPC",
        "TECH-MCP"
      ],
      "coverage_status": "SEEDED",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.165.md Appendix 12.3 FEAT-UNIFIED-TOOL-SURFACE",
        "app/src/components/operator/JobsView.tsx",
        "src/backend/handshake_core/src/mcp/gate.rs"
      ],
      "gap_stub_ids": []
    },
    {
      "feature_id": "FEAT-WORK-PACKET-SYSTEM",
      "primitive_ids": [
        "PRIM-TrackedWorkPacket",
        "PRIM-WorkPacketBinding",
        "PRIM-WorkPacketGovernance",
        "PRIM-WorkPacketPhase",
        "PRIM-WorkPacketStatus",
        "PRIM-WorkPacketType",
        "PRIM-SpecSessionLogEntry",
        "PRIM-StructuredCollaborationEnvelopeV1",
        "PRIM-StructuredCollaborationSummaryV1",
        "PRIM-ProjectProfileExtensionV1",
        "PRIM-WorkflowStateFamily",
        "PRIM-WorkflowQueueReasonCode",
        "PRIM-GovernedActionDescriptorV1",
        "PRIM-WorkflowTransitionRuleV1",
        "PRIM-QueueAutomationRuleV1",
        "PRIM-ExecutorEligibilityPolicyV1",
        "PRIM-ProjectProfileWorkflowExtensionV1",
        "PRIM-MirrorSyncState",
        "PRIM-MirrorAuthorityMode",
        "PRIM-MirrorReconciliationAction",
        "PRIM-MarkdownMirrorContractV1"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-MARKDOWN",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md 2.6.8.8 Spec Session Log (Task Board + Work Packets) (Normative)",
        "Handshake_Master_Spec_v02.181.md 7.2 Channel 1: Task Board + Work Packets (Contract Authority)",
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-WORK-PACKET-SYSTEM",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/locus/types.rs",
        ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.md"
      ],
      "gap_stub_ids": [
        "WP-1-Dev-Command-Center-MVP-v1",
        "WP-1-Workflow-Projection-Correlation-v1",
        "WP-1-Work-Profiles-v1",
        "WP-1-Structured-Collaboration-Artifact-Family-v1",
        "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        "WP-1-Structured-Collaboration-Contract-Hardening-v1",
        "WP-1-Project-Profile-Extension-Registry-v1",
        "WP-1-Project-Agnostic-Workflow-State-Registry-v1",
        "WP-1-Workflow-Transition-Automation-Registry-v1",
        "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        "WP-1-RAG-Retrieval-Mode-Policy-v1"
      ]
    },
    {
      "feature_id": "FEAT-WORKSPACE-BUNDLE",
      "primitive_ids": [
        "PRIM-BundleManifest",
        "PRIM-ArtifactManifest",
        "PRIM-ExportRecord",
        "PRIM-ExportTarget"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-JSON",
        "TECH-ZIP"
      ],
      "coverage_status": "GAP",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.161.md 10.5.7 Workspace Bundle Export (v0)",
        "Handshake_Master_Spec_v02.161.md Appendix 12.3 FEAT-WORKSPACE-BUNDLE",
        ".GOV/task_packets/stubs/WP-1-Workspace-Bundle-v2.md",
        "app/src/components/operator/Ans001TimelineDrawer.tsx"
      ],
      "gap_stub_ids": [
        "WP-1-Workspace-Bundle-v2"
      ]
    },
    {
      "feature_id": "FEAT-WORKFLOW-ENGINE",
      "primitive_ids": [
        "PRIM-AdapterError",
        "PRIM-EngineAdapter",
        "PRIM-JobState",
        "PRIM-MexRegistry",
        "PRIM-MexRuntimeError",
        "PRIM-RateLimitOutcome",
        "PRIM-RateLimitReservation",
        "PRIM-RoutingStrategy",
        "PRIM-SpawnLimits",
        "PRIM-WorkflowContext",
        "PRIM-WorkflowNodeExecution",
        "PRIM-WorkflowRun",
        "PRIM-GovernanceClaimLeaseRecord",
        "PRIM-GovernanceQueuedInstructionRecord",
        "PRIM-EngineResult",
        "PRIM-RoutingPolicy",
        "PRIM-SessionRegistry"
      ],
      "tool_ids": [],
      "technology_ids": [
        "TECH-AXUM",
        "TECH-JSON",
        "TECH-SQLITE"
      ],
      "coverage_status": "PARTIAL",
      "coverage_refs": [
        "Handshake_Master_Spec_v02.181.md Appendix 12.3 FEAT-WORKFLOW-ENGINE",
        "src/backend/handshake_core/src/mex/runtime.rs",
        "src/backend/handshake_core/src/workflows.rs",
        "src/backend/handshake_core/src/llm/guard.rs",
        "src/backend/handshake_core/src/storage/mod.rs"
      ],
      "gap_stub_ids": [
        "WP-1-Workflow-Projection-Correlation-v1"
      ]
    }
  ],
  "coverage_status_legend": {
    "SEEDED": "Feature has concrete primitive/tool/technology coverage rows and repo/spec references sufficient for current runtime understanding.",
    "PARTIAL": "Feature has meaningful seed coverage, but unresolved runtime or implementation gaps remain and are tracked as stub work packets.",
    "GAP": "Feature exists in the registry/spec, but Appendix 12.4 coverage is still mostly a gap and must be advanced through explicit stub work."
  }
}
```
<!-- HS_APPENDIX:END id=HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX -->

## 12.5 Appendix Block: UI_GUIDANCE (Per Feature) [CX-SPEC-APPX-012]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-UI-GUIDANCE schema=hs_ui_guidance@1 -->
```json
{
  "schema": "hs_ui_guidance@1",
  "spec_version": "v02.184",
  "last_updated": "2026-05-05",
  "ui_guidance": [
    {
      "feature_id": "FEAT-AI-JOB-MODEL",
      "user_goal": "Inspect, resume, and correlate AI jobs through the always-on drawer and operator inspector without losing runtime identity.",
      "entry_points": [
        "Ai Jobs drawer",
        "Operator Consoles > Jobs",
        "Deep links by job_id from evidence surfaces"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "queued",
          "running",
          "awaiting_user",
          "completed_ok",
          "completed_error",
          "canceled"
        ],
        "errors": [
          "job_not_found",
          "resume_denied_by_policy",
          "cloud_consent_pending"
        ]
      },
      "capability_gates": [
        "Resume, rerun, or escalation-affecting actions require visible policy and consent state."
      ],
      "telemetry": [
        "Job drawer and inspector actions remain correlated to Flight Recorder, Workflow, and consent/evidence surfaces."
      ],
      "tests": [
        "Job-focused deep links resolve the same job across drawer, Jobs view, and evidence surfaces by stable ids."
      ]
    },
    {
      "feature_id": "FEAT-ASR",
      "user_goal": "Turn governed audio/video inputs into timestamped transcripts that can flow into Loom, retrieval, and operator evidence.",
      "entry_points": [
        "Media Downloader > Transcript",
        "Stage capture/import",
        "Loom asset actions"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "queued",
          "transcribing",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "unsupported_media",
          "missing_media_source",
          "decoder_failure",
          "timeout"
        ]
      },
      "capability_gates": [
        "ASR runs require governed media access and must never bypass Flight Recorder."
      ],
      "telemetry": [
        "ASR progress, transcript artifact creation, and failure reasons are visible in operator/runtime surfaces.",
        "Source-media hashes, media-probe facts, and transcript timing anchors remain portable backend evidence."
      ],
      "tests": [
        "Transcript export preserves time anchors and references the originating media source."
      ]
    },
    {
      "feature_id": "FEAT-CALENDAR",
      "user_goal": "View and author local calendar events while understanding the bounded sync, export, capability, and time-window contracts that drive backend routing and evidence queries.",
      "entry_points": [
        "Calendar surface",
        "Create/edit event",
        "Query time range overlap"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "view",
          "editing",
          "sync_pending",
          "synced",
          "sync_failed",
          "busy_only_projected"
        ],
        "errors": [
          "capability_denied",
          "sync_failed",
          "export_mode_restricted"
        ]
      },
      "capability_gates": [
        "Calendar mutations are capability-gated and logged.",
        "Calendar capability profiles, source write policy, and export mode must stay visible enough to explain why a mutation, sync, or projection was allowed, reduced, or blocked."
      ],
      "telemetry": [
        "Calendar mutations emit audit events to Flight Recorder.",
        "Calendar time-window queries and policy-derived routing decisions remain visible as backend evidence posture rather than hidden UI state."
      ],
      "tests": [
        "Range query returns correlated spans with stable IDs.",
        "Calendar sync/export posture survives backend swaps without changing the meaning of local_only, busy_only, or full_export."
      ]
    },
    {
      "feature_id": "FEAT-CANVAS",
      "user_goal": "Organize ideas spatially while keeping AI/job transitions, provenance, and later workflow handoffs explicit.",
      "entry_points": [
        "Canvas worksurface",
        "Thinking Pipeline transitions",
        "Studio shell"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "editing",
          "ai_preview",
          "applying",
          "saved",
          "failed"
        ],
        "errors": [
          "entity_locked",
          "capability_denied",
          "apply_failed"
        ]
      },
      "capability_gates": [
        "AI actions on canvas entities route through the job/runtime contract instead of direct mutation."
      ],
      "telemetry": [
        "Canvas-originated runtime actions are visible in Flight Recorder and later DCC/Locus overlays."
      ],
      "tests": [
        "Canvas handoff into workflows preserves entity binding and provenance references."
      ]
    },
    {
      "feature_id": "FEAT-CHARTS-DASHBOARDS",
      "user_goal": "Generate charts from governed tables and runtime traces while preserving export lineage and supporting later deck reuse.",
      "entry_points": [
        "Charts worksurface",
        "Insert from table",
        "Operator analytics surfaces"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "configuring",
          "rendering",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "missing_table_binding",
          "invalid_chart_spec",
          "render_failed"
        ]
      },
      "capability_gates": [
        "Chart writes remain job-backed and tied to source tables or traces."
      ],
      "telemetry": [
        "Chart specs, source bindings, and export lineage are visible in operator/runtime evidence."
      ],
      "tests": [
        "Chart exports carry source-table provenance and deterministic spec hashes."
      ]
    },
    {
      "feature_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "user_goal": "Approve or inspect cloud escalation decisions with explicit projection plans, consent receipts, and replay-safe evidence.",
      "entry_points": [
        "Dev Command Center > escalation review",
        "Operator Consoles > evidence / governance pack export",
        "Flight Recorder-linked consent drilldown"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "local_only",
          "projection_ready",
          "awaiting_consent",
          "approved",
          "denied",
          "expired"
        ],
        "errors": [
          "projection_missing",
          "consent_receipt_missing",
          "cloud_policy_locked"
        ]
      },
      "capability_gates": [
        "Cloud escalation requires explicit human consent and visible governance mode."
      ],
      "telemetry": [
        "Projection plans, consent receipts, and allow/deny decisions emit Flight Recorder events."
      ],
      "tests": [
        "Consent drilldown links projection plan, receipt, and resulting cloud job/tool activity by ids."
      ]
    },
    {
      "feature_id": "FEAT-DEV-COMMAND-CENTER",
      "user_goal": "Operate governed development work and evidence flows: map work packets, task board state, work packet bindings, worktrees, sessions, ready work, micro-task state, replay-safe run history, tool infrastructure health, workspace runtime readiness, and promotion blockers; review diffs; manage approvals; and track durable export, diagnostics, and replay-safe handoffs.",
      "entry_points": [
        "Dev Command Center surface",
        "Sessions panel",
        "Recovery queue",
        "Provider readiness panel",
        "Repository engine policy panel",
        "run history and replay",
        "tool infrastructure registry",
        "workspace runtime panel",
        "promotion gate snapshot",
        "VCS/diff review",
        "governed export and evidence status",
        "tracked work packet and task board state",
        "Spec Session Log continuity",
        "parallel model session routing and micro-task occupancy",
        "[ADD v02.166] structured work record viewers",
        "[ADD v02.166] Role Mailbox triage queue",
        "[ADD v02.166] append-only note timeline",
        "[ADD v02.167] board or queue layout switcher",
        "[ADD v02.168] base-envelope and profile-extension inspector",
        "[ADD v02.169] mirror reconciliation queue",
        "[ADD v02.170] view preset switcher",
        "[ADD v02.170] execution readiness queue",
        "[ADD v02.170] inbox triage presets",
        "[ADD v02.171] workflow-state and queue-reason inspector",
        "[ADD v02.172] transition and executor preview drawer",
        "[ADD v02.173] mailbox action-request queue",
        "[ADD v02.175] mailbox remediation queue",
        "[ADD v02.176] mailbox claim and takeover panel",
        "[ADD v02.177] mailbox handoff and announce-back inspector"
      ],
      "required_surfaces": [
        "ui"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "reviewing",
          "awaiting_approval",
          "executing",
          "blocked"
        ],
        "errors": [
          "capability_denied",
          "worktree_conflict",
          "approval_expired",
          "session_recovery_required",
          "provider_unready",
          "repository_policy_blocked",
          "tool_infrastructure_unready",
          "workspace_runtime_unready",
          "promotion_gate_blocked"
        ]
      },
      "capability_gates": [
        "No bypass of Tool Gate/Workflow Engine; destructive VCS operations require explicit approval."
      ],
      "telemetry": [
        "All state-changing actions are logged and deep-linkable via Operator Consoles.",
        "Governance Pack exports, Workspace Bundle exports, diagnostics query state, and bounded bundle validation outcomes round-trip by stable backend identifiers.",
        "Tracked work packet state, task board freshness, work-packet binding state, Spec Session Log continuity, ready-query state, micro-task summaries, and active session occupancy round-trip by stable work-packet, micro-task, workflow-run, task-board, and model-session identifiers.",
        "[ADD v02.164] Session checkpoints, heartbeat freshness, provider capability readiness, anti-pattern alerts, and repository-engine decision state round-trip by stable session, workflow-run, work-packet, and workspace identifiers.",
        "[ADD v02.165] Run-history entries, tool-infrastructure status, workspace-runtime state, and promotion-gate snapshots round-trip by stable workflow-run, workflow-node, tool/server, workspace, and repository-policy identifiers.",
        "[ADD v02.166] Structured Work Packet records, Micro-Task execution contracts, Task Board projection rows, note timelines, and Role Mailbox triage state round-trip by stable work-packet, micro-task, task-board, note-stream, and mailbox-thread identifiers before raw Markdown mirrors are consulted.",
        "[ADD v02.167] Board, queue, list, roadmap, and Jira-like layout projections round-trip by stable view configuration identifiers plus the same authoritative field identifiers, and they expose mirror synchronization state instead of silently masking record drift.",
        "[ADD v02.168] Base-envelope fields, compact summaries, profile-extension metadata, and mirror-state semantics round-trip by stable record identifiers before any project-specific viewer logic is applied.",
        "[ADD v02.169] Mirror authority mode, reconciliation action, last reconciliation timestamp, and drift summaries round-trip by stable record identifiers before any regenerate-or-normalize action is offered.",
        "[ADD v02.170] View preset ids, lane ids, and action-binding ids round-trip by stable identifiers before any drag, reorder, quick action, or bulk action is offered.",
        "[ADD v02.171] Workflow-state families, queue-reason codes, and allowed action ids round-trip by stable record identifiers before a queue regroup, review queue move, or routing action is offered.",
        "[ADD v02.172] Transition rule ids, queue automation rule ids, automation trigger ids, and executor eligibility policy ids round-trip by stable record identifiers before any retry, reroute, review-request, or approval-request action is offered.",
        "[ADD v02.173] Mailbox thread lifecycle state, message delivery state, allowed responses, action-request metadata, and mailbox-local versus governed-action previews round-trip by stable thread, message, and linked record identifiers before any reply, snooze, escalation, or transcription action is offered.",
        "[ADD v02.175] Mailbox triage queue state, reminder schedule, queue age, snooze-until, expiry posture, dead-letter disposition, and recommended operator controls round-trip by stable thread, message, linked record, and view identifiers before any reminder, unsnooze, retry-delivery, reroute, or archive action is offered.",
        "[ADD v02.176] Mailbox executor kind, claim mode, claimant identity, lease age, lease expiry, takeover legality, and response-authority scope round-trip by stable thread, claim, linked-record, and view identifiers before any claim, release, renew, takeover, or reply action is offered.",
        "[ADD v02.181] Software-delivery governance overlay records, validator-gate summaries, governed-action resolutions, overlay claim/lease posture, queued steering/follow-up state, checkpoint-backed recovery posture, and derived closeout posture round-trip by stable work_packet_id, workflow_run_id, gate_record_id, action_request_id, claim_id, queued_instruction_id, and checkpoint_id before any repo mirror, packet narrative, or mailbox summary is trusted."
      ],
      "tests": [
        "Diff/accept flows exist for edits; approvals are durable and observable.",
        "Export and diagnostics status remain consistent across Dev Command Center projection, Operator Console drilldown, and backend polling APIs.",
        "Parallel model session steering, ready-work queries, task-board freshness, work-packet bindings, and micro-task summaries remain consistent across Dev Command Center projection, Locus storage, workflow state, Spec Session Log artifacts, and session-registry APIs.",
        "[ADD v02.164] Recovery queue, provider readiness, and repository-engine policy views remain consistent across Dev Command Center projection, session-registry artifacts, workflow state, Tool Gate metadata, and version-control execution policy.",
        "[ADD v02.165] Run history, tool infrastructure registry, workspace runtime, and promotion-gate views remain consistent across Dev Command Center projection, workflow state, Tool Registry metadata, workspace metadata, workspace-safety policy, and repository-policy evidence.",
        "[ADD v02.166] Structured work record viewers, note timelines, and Role Mailbox triage remain consistent across Dev Command Center projection, Role Mailbox artifacts, Locus storage, Task Board projections, and Markdown mirror regeneration.",
        "[ADD v02.167] Switching between board, queue, list, and roadmap layouts does not change authoritative state unless a governed structured-record edit is executed and recorded.",
        "[ADD v02.168] Generic Dev Command Center viewers remain usable when project-profile extensions are unknown, provided the base structured-collaboration envelope is valid.",
        "[ADD v02.169] Mirror-reconciliation views explain whether drift came from canonical field changes, advisory human edits, missing mirror generation, or template mismatch before the operator reads a raw Markdown diff.",
        "[ADD v02.170] Dragging a card, moving a queue row, or triggering a bulk action previews the affected record ids plus the governed field or workflow mutations before authoritative state changes.",
        "[ADD v02.171] Project-profile label overrides can change visible queue names without changing the underlying workflow-state family, queue reason code, or governed action eligibility.",
        "[ADD v02.172] Queue regrouping, retry, escalation, review-request, and approval-request previews explain which transition rule, automation rule, and executor policy made the action legal before authoritative state changes.",
        "[ADD v02.173] Mailbox triage and execution queues explain whether a quick action is mailbox-local, governed, or transcription-required before a reply or acknowledgement can affect linked work.",
        "[ADD v02.175] Mailbox remediation queues explain whether snooze, reminder, retry-delivery, reroute, transcription, or archive actions are mailbox-local, automation-triggering, or governed linked-record mutations before anything changes.",
        "[ADD v02.176] Claim, release, renew, and takeover controls explain whether the result is mailbox-local claim state, automation-triggering queue change, or governed linked-record mutation before any linked work changes.",
        "[ADD v02.181] Dev Command Center, Task Board, and Role Mailbox projections remain consistent with authoritative workflow, validator-gate, claim/lease, queued-instruction, recovery, and closeout state even when repo `/.GOV/**` mirrors or mailbox chronology lag behind."
      ]
    },
    {
      "feature_id": "FEAT-DOCS-SHEETS",
      "user_goal": "Run structured AI edits over docs and sheets without losing stable IDs, provenance, formulas, or later reuse.",
      "entry_points": [
        "Docs editor",
        "Sheets/table surface",
        "Command palette AI actions"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "planning",
          "awaiting_approval",
          "executing",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "invalid_entity_ref",
          "capability_denied",
          "validation_failed"
        ]
      },
      "capability_gates": [
        "All AI edits remain profile-backed Docs & Sheets jobs with deterministic provenance updates."
      ],
      "telemetry": [
        "Docs/Sheets operations emit job, validation, and provenance evidence visible to operator surfaces."
      ],
      "tests": [
        "Block/row/column IDs remain stable across AI operations and replay logs."
      ]
    },
    {
      "feature_id": "FEAT-FLIGHT-RECORDER",
      "user_goal": "Filter, inspect, and deep-link runtime events without losing correlation to jobs, diagnostics, artifacts, or operator actions.",
      "entry_points": [
        "Flight Recorder view",
        "Operator Consoles > Timeline",
        "Deep links from Jobs / Problems / Evidence"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "filtering",
          "results_loaded",
          "empty",
          "event_selected"
        ],
        "errors": [
          "invalid_filter",
          "event_unlinked_or_missing"
        ]
      },
      "capability_gates": [
        "Querying the timeline is read-only; privileged exports or reruns remain governed by separate policy decisions."
      ],
      "telemetry": [
        "Timeline queries preserve event ids and correlation metadata so deep links round-trip across operator surfaces."
      ],
      "tests": [
        "Filter state round-trips through the API contract and selected events deep-link to the same job, diagnostic, or artifact ids."
      ]
    },
    {
      "feature_id": "FEAT-GOVERNANCE-PACK",
      "user_goal": "Export a project-parameterized Governance Pack through a governed workflow without losing capability, artifact, or recorder provenance.",
      "entry_points": [
        "Governance Pack export modal",
        "Operator Consoles > Governance Pack Export",
        "Workflow-linked export completion state"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles",
        "gov"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "validating_invariants",
          "queued",
          "exporting",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "invalid_export_target",
          "missing_invariant",
          "capability_denied",
          "export_failed"
        ]
      },
      "capability_gates": [
        "Governance Pack export remains workflow-run and server-side capability-gated; no direct file export bypass is allowed."
      ],
      "telemetry": [
        "Governance Pack export requests, lifecycle events, artifact manifests, and outcomes remain queryable in Flight Recorder and operator evidence surfaces.",
        "[ADD v02.181] Governance Pack import/export actions preserve stable overlay source identifiers, export ids, gate refs, workflow refs, and any exported claim/lease or queued-instruction refs so repo-governance artifacts remain auditable transfer material without becoming runtime authority."
      ],
      "tests": [
        "Governance Pack export preserves invariant validation results, export target, manifest hashes, and stable export identifiers across export, polling, and download.",
        "[ADD v02.181] Governance Pack import or export never bypasses product-owned workflow authority, validator-gate state, claim/lease state, queued-instruction state, recovery posture, or derived closeout posture."
      ]
    },
    {
      "feature_id": "FEAT-LOOM-LIBRARY",
      "user_goal": "Import notes/files as LoomBlocks and organize them via views, tags/mentions, backlinks, and optional AI suggestions.",
      "entry_points": [
        "Loom views: All/Unlinked/Sorted/Pins",
        "File import (drag/drop or picker)",
        "Inline @mentions/#tags"
      ],
      "required_surfaces": [
        "ui"
      ],
      "interaction_contract": {
        "states": [
          "view_all",
          "view_unlinked",
          "view_sorted",
          "view_pins",
          "preview_pending",
          "preview_generated",
          "preview_failed"
        ],
        "errors": [
          "capability_denied",
          "dedup_hit",
          "preview_failed"
        ]
      },
      "capability_gates": [
        "DerivedContent only for AI suggestions; RawContent never silently mutated."
      ],
      "telemetry": [
        "FR-EVT-LOOM-* (Flight Recorder).",
        "[ADD v02.178] Retrieval-mode badges and graph-bias reasons are visible whenever Loom tags, mentions, backlinks, pins, or unlinked state influence a grounded answer or routing decision."
      ],
      "tests": [
        "Views and filters functional.",
        "Mentions/tags create typed edges; backlinks update.",
        "[ADD v02.178] Exact LoomBlock or asset opens bypass semantic search by default while related-context expansion still logs the retrieval mode and graph-bias reasons."
      ]
    },
    {
      "feature_id": "FEAT-MEDIA-DOWNLOADER",
      "user_goal": "Archive web media into workspace artifacts with a resumable, controllable queue and deterministic materialization.",
      "entry_points": [
        "Media Downloader worksurface",
        "Add URL(s)",
        "Queue controls: pause/resume/cancel/retry"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "queue_idle",
          "queue_running",
          "queue_paused",
          "item_downloading",
          "item_validating",
          "item_succeeded",
          "item_failed"
        ],
        "errors": [
          "auth_required",
          "policy_denied",
          "non_media_payload_rejected"
        ]
      },
      "capability_gates": [
        "Network and proc exec are allowlist-scoped; cookie/session artifacts exportable=false by default."
      ],
      "telemetry": [
        "Ingest/progress events recorded in Flight Recorder."
      ],
      "tests": [
        "Resumable downloads + dedup by SHA-256.",
        "Non-media payloads rejected."
      ]
    },
    {
      "feature_id": "FEAT-MEX-RUNTIME",
      "user_goal": "Inspect governed mechanical engine executions with visible envelopes, safety gates, outputs, and evidence.",
      "entry_points": [
        "Dev Command Center > mechanical runtime feed",
        "Operator Consoles > job inspector",
        "Stage / tool-originated mechanical actions"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "planned",
          "awaiting_gate",
          "engine_running",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "engine_unavailable",
          "gate_denied",
          "result_integrity_failed"
        ]
      },
      "capability_gates": [
        "Mechanical engine execution remains Tool Gate and workflow governed."
      ],
      "telemetry": [
        "EngineResult envelopes, gate outcomes, and artifact refs are visible in Flight Recorder and DCC."
      ],
      "tests": [
        "Mechanical runs expose inputs, outputs, provenance, and failure reasons without bypassing operator evidence surfaces."
      ]
    },
    {
      "feature_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "user_goal": "Steer and inspect concurrent model sessions with explicit provider routing, spawn state, and model-swap history.",
      "entry_points": [
        "Dev Command Center > session steering panel",
        "Operator Consoles > session-linked jobs",
        "Stage / DCC prompts that bind to a model session"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "queued",
          "active",
          "waiting_for_swap",
          "paused",
          "recovered"
        ],
        "errors": [
          "spawn_limit_hit",
          "provider_resolution_failed",
          "session_binding_missing"
        ]
      },
      "capability_gates": [
        "Session steering cannot bypass routing, capability intersection, or cloud escalation governance."
      ],
      "telemetry": [
        "Session lifecycle, model swaps, and provider routing decisions remain visible in DCC, Flight Recorder, and future Locus overlays."
      ],
      "tests": [
        "Session list and inspector preserve stable session ids, provider ids, and swap request lineage."
      ]
    },
    {
      "feature_id": "FEAT-MONACO-EDITOR",
      "user_goal": "Edit code with governed AI actions (patchsets, diff/accept) and deterministic provenance.",
      "entry_points": [
        "Monaco editor",
        "AI code actions",
        "Diff/patchset review"
      ],
      "required_surfaces": [
        "ui"
      ],
      "interaction_contract": {
        "states": [
          "editing",
          "ai_action_pending",
          "ai_action_running",
          "diff_review"
        ],
        "errors": [
          "capability_denied",
          "patch_apply_conflict"
        ]
      },
      "capability_gates": [
        "No silent edits; write operations are capability-gated."
      ],
      "telemetry": [
        "Writes/tool calls correlated to jobs and recorded in Flight Recorder."
      ],
      "tests": [
        "All edits are reviewable (diff/accept) before apply."
      ]
    },
    {
      "feature_id": "FEAT-OPERATOR-CONSOLES",
      "user_goal": "Debug and diagnose jobs/tools with evidence-first drilldown and deterministic Debug Bundle export.",
      "entry_points": [
        "Operator Consoles: Problems",
        "Jobs",
        "Timeline (Flight Recorder)",
        "Debug Bundle export"
      ],
      "required_surfaces": [
        "ui"
      ],
      "interaction_contract": {
        "states": [
          "triage",
          "drilldown",
          "export_bundle"
        ],
        "errors": [
          "export_denied_by_policy",
          "trace_unlinked_or_ambiguous"
        ]
      },
      "capability_gates": [
        "Privileged reruns/actions require visible policy decision."
      ],
      "telemetry": [
        "Console state-changing actions emit Flight Recorder events."
      ],
      "tests": [
        "Deep links across Problemsâ†’Jobsâ†’Timeline by ids.",
        "Debug bundles are deterministic and redacted-by-default."
      ]
    },
    {
      "feature_id": "FEAT-PRESENTATIONS-DECKS",
      "user_goal": "Assemble governed decks from docs, charts, and project context without losing provenance or replayability.",
      "entry_points": [
        "Deck worksurface",
        "Create from chart/doc selection",
        "Export to presentation artifact"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "assembling",
          "rendering",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "missing_source_slide_data",
          "invalid_deck_spec",
          "export_failed"
        ]
      },
      "capability_gates": [
        "Deck generation and export remain governed job flows with explicit source bindings."
      ],
      "telemetry": [
        "Deck creation/export is visible in job history and Flight Recorder."
      ],
      "tests": [
        "Deck exports preserve chart/doc source references and export manifests."
      ]
    },
    {
      "feature_id": "FEAT-PROJECT-BRAIN",
      "user_goal": "Ask project-scoped retrieval questions over docs, canvas, tables, mail, and events with visible citations and bounded context.",
      "entry_points": [
        "Project Brain chat surface",
        "Context-aware answer actions",
        "DCC evidence review"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "retrieving",
          "assembling_context",
          "answer_ready",
          "answer_failed"
        ],
        "errors": [
          "retrieval_budget_exceeded",
          "no_grounded_answer",
          "stale_context_pack"
        ]
      },
      "capability_gates": [
        "Project Brain answers must remain grounded in governed retrieval paths with citation evidence."
      ],
      "telemetry": [
        "QueryPlan, RetrievalTrace, and citation bundles are visible in operator/runtime surfaces.",
        "[ADD v02.178] Retrieval mode and non-hybrid reason are visible whenever Project Brain answers use direct lookup, graph traversal, or choose to avoid hybrid retrieval."
      ],
      "tests": [
        "Project Brain answers cite bounded anchors and log retrieval traces.",
        "[ADD v02.178] Known Work Packet, LoomBlock, or asset identifiers resolve through direct lookup or bounded graph traversal before hybrid retrieval is attempted."
      ]
    },
    {
      "feature_id": "FEAT-ROLE-MAILBOX",
      "user_goal": "Coordinate across roles and sessions through a structured asynchronous collaboration inbox without treating messages as authoritative state.",
      "entry_points": [
        "Role Mailbox inspector",
        "Announce-back messages",
        "Threaded coordination",
        "[ADD v02.166] expected-response queue",
        "[ADD v02.166] handoff completeness review",
        "[ADD v02.170] inbox triage presets",
        "[ADD v02.173] thread lifecycle and action-request badges",
        "[ADD v02.174] verifier loop inspector",
        "[ADD v02.175] snooze and dead-letter remediation queue",
        "[ADD v02.176] claim and lease takeover inspector",
        "[ADD v02.177] handoff bundle and announce-back provenance inspector"
      ],
      "required_surfaces": [
        "ui",
        "gov"
      ],
      "interaction_contract": {
        "states": [
        "new",
        "awaiting_response",
        "escalated",
        "read",
        "snoozed",
        "expired",
        "dead_lettered",
        "needs_remediation",
        "resolved_pending_transcription",
        "archived"
        ],
        "errors": [
          "schema_invalid",
          "missing_wp_id",
        "missing_artifact_ref",
        "expected_response_missing",
        "handoff_incomplete",
        "reminder_schedule_invalid",
        "dead_letter_disposition_missing",
        "handoff_bundle_missing",
        "announce_back_provenance_missing",
        "transcription_status_missing"
        ]
      },
      "capability_gates": [
        "Role Mailbox messages are exportable but non-authoritative by default."
      ],
      "telemetry": [
        "Mailbox events are correlated into Flight Recorder and Debug Bundles (no inline bodies).",
        "[ADD v02.166] Expected response posture, expiry state, evidence references, and handoff completeness remain visible in Dev Command Center triage without transcript-only parsing.",
        "[ADD v02.167] Role Mailbox export index and thread records round-trip by stable thread and message identifiers across structured index views, append-only thread files, and Dev Command Center triage.",
        "[ADD v02.168] Base-envelope fields and mailbox-specific profile extensions remain distinguishable in thread and message views.",
        "[ADD v02.169] Mirror authority mode and reconciliation action remain visible when a readable mailbox summary or Markdown sidecar exists, so operators know whether they are reading derived narrative or canonical export state.",
        "[ADD v02.170] Inbox presets, expected-response queues, and reply or escalation action bindings round-trip by stable thread, message, and linked work identifiers before any triage action is offered.",
        "[ADD v02.171] Mailbox-linked queue reasons round-trip by stable thread, message, and linked record identifiers before a wait, review, or escalation state is shown in triage.",
        "[ADD v02.172] Mailbox-triggered automation rules, transition rule ids, and executor eligibility policy ids round-trip by stable thread, message, trigger, and linked record identifiers before any wait or unblocked state is shown.",
        "[ADD v02.173] Thread lifecycle state, message delivery state, allowed responses, action-request metadata, and mailbox-local versus governed-action previews round-trip by stable thread, message, and linked record identifiers before any reply, snooze, escalation, or transcription action is shown.",
        "[ADD v02.174] Loop checkpoint refs, verifier outcomes, remaining retry budget, escalation targets, and completion-report transcription posture round-trip by stable thread, message, work_packet_id, and micro_task_id before any retry, escalate, verify, or complete quick action is shown.",
        "[ADD v02.175] Triage queue state, reminder schedule, queue age, snooze-until, expiry posture, dead-letter disposition, and operator remediation controls round-trip by stable thread, message, and linked record identifiers before a reminder, unsnooze, reroute, retry-delivery, or archive action is shown.",
        "[ADD v02.176] Executor kind, claim mode, claimant identity, lease age, lease expiry, takeover policy, and response-authority scope round-trip by stable thread, claim, message, and linked record identifiers before a claim, release, renew, takeover, or reply action is shown.",
        "[ADD v02.177] Handoff bundle id, announce-back provenance kind, remaining-work summary, recommended next actor, and transcription status round-trip by stable thread, message, work_packet_id, and micro_task_id before any handoff-complete or done badge is shown.",
        "[ADD v02.181] Mailbox-linked software-delivery approval, review, validation, claim/lease handoff, queued follow-up, recovery, and closeout messages round-trip linked action_request_id, gate_record_id, workflow_run_id, claim_id, queued_instruction_id, and checkpoint_id without turning thread chronology into authority."
      ],
      "tests": [
        "Threads reference wp_id/mt_id when applicable; artifacts are referenced by hash/ref only.",
        "[ADD v02.166] Delegate-work, review-request, decision-request, and escalation messages preserve expected response, evidence references, and handoff fields across Role Mailbox storage and Dev Command Center projection.",
        "[ADD v02.167] Structured Role Mailbox export remains portable when repository-specific routing metadata is absent or moved into project-profile extensions.",
        "[ADD v02.168] Unknown project-profile extensions do not break base mailbox export parsing or triage rendering.",
        "[ADD v02.169] Regenerating readable mailbox summaries never deletes append-only thread exports or advisory note sidecars without an explicit normalization action.",
        "[ADD v02.170] Inbox triage views explain which reply, escalate, or acknowledge action binding is available for each row before mailbox state or linked task state changes.",
        "[ADD v02.171] Mailbox triage can show when a thread contributes `mailbox_response_wait` or `escalation_wait` to a linked record without making the mailbox thread authoritative for that record's workflow-state family.",
        "[ADD v02.172] Mailbox replies, expiries, and escalation acknowledgements can trigger queue recommendations only through explicit automation and transition rules, without bypassing approval boundaries or executor eligibility checks.",
        "[ADD v02.173] Role Mailbox views distinguish mailbox-local actions such as acknowledge or snooze from governed actions and transcription requests before linked Work Packet or Micro-Task state can change.",
        "[ADD v02.174] Retrying, escalating, or completing a Micro-Task from Role Mailbox always shows the latest loop checkpoint, structured verifier outcome, remaining retry budget, and any required transcription target before linked Work Packet or Task Board state changes.",
        "[ADD v02.175] Snoozing, unsnoozing, sending reminders, retrying delivery, rerouting delivery, or archiving a dead-lettered thread never mutates linked authoritative work implicitly and always preserves visible queue age plus dead-letter disposition.",
        "[ADD v02.176] Claiming, releasing, renewing, or taking over a thread never mutates linked authoritative work implicitly and always shows claimant identity, lease expiry, and actor-ineligible actions before a reply or reroute is allowed.",
        "[ADD v02.177] Handoff and announce-back views always distinguish advisory mailbox summaries from transcription-confirmed linked state and preserve enough compact handoff context to resume work without replaying the full thread.",
        "[ADD v02.181] Mailbox replies, approvals, denials, escalations, claim handbacks, and queued follow-up acknowledgements never mutate authoritative work, validator-gate, claim/lease, queued-instruction, recovery, or closeout state unless an explicit governed action or transcription succeeds."
      ]
    },
    {
      "feature_id": "FEAT-SPEC-APPENDICES",
      "user_goal": "Maintain a self-contained spec that scales: explicit feature inventory, explicit interaction model, explicit per-feature UI contract.",
      "entry_points": [
        "Master Spec Section 12"
      ],
      "required_surfaces": [
        "spec"
      ],
      "interaction_contract": {
        "states": [
          "present",
          "stale_derived_views"
        ],
        "errors": [
          "appendix_missing",
          "schema_invalid",
          "derived_drift"
        ]
      },
      "capability_gates": [],
      "telemetry": [],
      "tests": [
        "spec-eof-appendices-check validates presence + schema + spec_version",
        "gov-check includes appendix presence + json-parse validation (repo-level, deterministic)"
      ]
    },
    {
      "feature_id": "FEAT-STAGE",
      "user_goal": "Browse external web with isolation and capture/import governed artifacts via jobs (no UI bypass).",
      "entry_points": [
        "Stage surface",
        "Stage Apps (handshake-stage://)",
        "Stage capture/clip/import actions"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "origin_external_web",
          "origin_stage_app",
          "approval_pending",
          "job_running",
          "job_succeeded",
          "job_failed"
        ],
        "errors": [
          "bridge_denied",
          "capability_denied",
          "approval_expired"
        ]
      },
      "capability_gates": [
        "Stage Bridge injected only on Stage App origin.",
        "All capture/import routes through Workflow Engine + Tool Gate."
      ],
      "telemetry": [
        "Stage allow/deny + capability decisions recorded in Flight Recorder."
      ],
      "tests": [
        "External Web cannot call Stage Bridge.",
        "Capture/import outputs are artifacts with hashes/manifests.",
        "Audio/video capture outputs preserve stable lineage required for later ASR jobs."
      ]
    },
    {
      "feature_id": "FEAT-STUDIO",
      "user_goal": "Operate the cross-surface creative shell with explicit runtime visibility across Canvas, Lens, Photo, and later creative modules.",
      "entry_points": [
        "Studio shell",
        "Atelier/Lens collaboration panel",
        "Studio module switcher"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "switching_surface",
          "planning",
          "job_running",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "surface_binding_missing",
          "capability_denied",
          "runtime_projection_missing"
        ]
      },
      "capability_gates": [
        "Studio actions do not bypass job/tool/runtime governance just because they appear as shell-level UI."
      ],
      "telemetry": [
        "Studio-originated actions are projected into DCC, Flight Recorder, and later Locus overlays."
      ],
      "tests": [
        "Studio surface handoffs preserve runtime identity and visible evidence."
      ]
    },
    {
      "feature_id": "FEAT-TASK-BOARD",
      "user_goal": "Inspect and steer the human-readable planning mirror and its structured projection rows without losing authoritative linkage to tracked work packets, ready-work state, or synchronized workflow activation.",
      "entry_points": [
        "Dev Command Center planning lanes",
        "Spec Session Log view",
        "Task Board status mirror",
        "[ADD v02.170] board or queue preset switcher",
        "[ADD v02.175] mailbox pressure overlay",
        "[ADD v02.176] mailbox claimant overlay"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "gov"
      ],
      "interaction_contract": {
        "states": [
          "synced",
          "freshness_unknown",
          "manual_edit_detected",
          "ready_work_available",
          "stale"
        ],
        "errors": [
          "sync_failed",
          "authority_mismatch",
          "status_unlinked"
        ]
      },
      "capability_gates": [
        "Task Board writes or rewrites must route through governed synchronization or explicit status-sync workflows; ad hoc local edits are never authoritative."
      ],
      "telemetry": [
        "Task Board freshness, authoritative identifiers, and manual-edit detection remain visible through Dev Command Center and Flight Recorder-linked planning surfaces.",
        "[ADD v02.166] Structured Task Board rows and Markdown mirror regeneration stay correlated by stable task_board_id and work_packet_id values.",
        "[ADD v02.167] Derived board, queue, and roadmap layouts round-trip by stable Task Board row identifiers and explicit view configuration identifiers so layout changes remain inspectable.",
        "[ADD v02.168] Task Board rows preserve the shared base envelope and compact summary fields before any board-specific profile extensions are evaluated.",
        "[ADD v02.169] Task Board mirror authority mode, reconciliation action, and last reconciliation timestamp remain visible before a lane move or regenerate action is offered.",
        "[ADD v02.170] View preset ids, lane ids, and action-binding ids remain visible before a card move, queue reorder, or roadmap regroup action is offered.",
        "[ADD v02.171] Base workflow-state families and queue-reason codes remain visible before project-profile board labels or lane aliases are applied.",
        "[ADD v02.172] Transition rule ids, queue automation rule ids, and executor eligibility policy ids remain visible before a lane move, regroup, or automatic board refresh is treated as authoritative.",
        "[ADD v02.175] Mailbox-derived queue age, snooze-until, expiry posture, dead-letter disposition, and linked thread identifiers remain visible before a backlog regroup, waiting overlay, or remediation follow-up is shown.",
        "[ADD v02.176] Mailbox claimant, claim mode, lease age, lease expiry, and actor-ineligible waiting posture remain visible before a backlog regroup, ownership overlay, or takeover follow-up is shown.",
        "[ADD v02.181] Validator-gate summaries, governed-action ids, claim/lease posture, queued follow-up state, checkpoint-backed recovery posture, and derived closeout posture remain visible before a validation badge, completion overlay, ownership overlay, or board-level follow-up is treated as trustworthy."
      ],
      "tests": [
        "Task Board entries round-trip stable task_board_id, work_packet_id, and status values across backend sync, Dev Command Center projection, and Spec Session Log history.",
        "[ADD v02.166] Structured Task Board rows remain authoritative when Markdown mirrors are regenerated, and manual Markdown edits surface as advisory until normalized.",
        "[ADD v02.167] Kanban or Jira-like view regrouping exposes which authoritative fields power each lane and signals drift when a Markdown mirror or imported board no longer matches the structured record.",
        "[ADD v02.168] Generic board viewers still render status, blockers, next action, and mirror state when a project-profile extension is unknown or omitted.",
        "[ADD v02.169] Regenerating a Task Board mirror preserves advisory note sidecars and blocks on manual resolution when the readable mirror contains unnormalized operator edits.",
        "[ADD v02.170] Dragging a card between lanes or applying a bulk board action previews the governed field or workflow mutation and does nothing authoritative when the current preset exposes view-only movement.",
        "[ADD v02.171] Changing a board label or project-profile lane alias does not change the underlying workflow-state family, queue reason code, or governed action eligibility.",
        "[ADD v02.172] A lane move or regroup cannot mutate canonical state unless a valid transition rule and eligible actor permit it, and automatic queue moves stop at approval boundaries.",
        "[ADD v02.175] Task Board overlays for mailbox backlog, snooze, expiry, and dead-letter remediation never mutate canonical work state directly and always identify the linked mailbox thread before a follow-up action is offered.",
        "[ADD v02.176] Task Board claimant overlays never mutate canonical work state directly and always identify the linked mailbox thread, claimant, and lease-expiry posture before a takeover or reroute follow-up is offered.",
        "[ADD v02.181] Board regrouping, mirror refresh, or imported board synchronization cannot create, clear, or complete validator-gate state, claim/lease posture, queued follow-up state, recovery posture, or closeout posture without authoritative runtime mutations."
      ]
    },
    {
      "feature_id": "FEAT-WORK-PACKET-SYSTEM",
      "user_goal": "Inspect and route scoped execution contracts through structured Work Packet records and append-only notes without losing the authoritative work packet binding, workflow activation state, or micro-task coordination context.",
      "entry_points": [
        "Dev Command Center work packet detail",
        "Spec Session Log work packet drilldown",
        "Ready-work selection and activation surfaces",
        "[ADD v02.166] append-only note timeline",
        "[ADD v02.170] governed next-action preview",
        "[ADD v02.172] transition matrix preview",
        "[ADD v02.176] mailbox claimant and lease follow-up"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "gov"
      ],
      "interaction_contract": {
        "states": [
          "stub",
          "ready",
          "in_progress",
          "blocked",
          "gated",
          "done"
        ],
        "errors": [
          "binding_missing",
          "workflow_unlinked",
          "task_packet_missing",
          "activation_conflict"
        ]
      },
      "capability_gates": [
        "Work packet activation, state transitions, and worktree binding must remain governed operations backed by workflow and work-tracking artifacts."
      ],
      "telemetry": [
        "Work packet lifecycle, activation, and session occupancy stay queryable by stable work_packet_id and workflow_run_id across Dev Command Center, Locus, and workflow surfaces.",
        "[ADD v02.166] Structured Work Packet fields, note-stream references, and Markdown mirror regeneration stay correlated by stable work_packet_id, workflow_run_id, and note artifact handles.",
        "[ADD v02.167] Work Packet canonical record and compact summary files round-trip by stable work_packet_id, schema_version, and project_profile_kind before any Markdown mirror is rendered.",
        "[ADD v02.168] Work Packet viewers round-trip base-envelope identity, compact summary payloads, and profile-extension metadata as distinct layers.",
        "[ADD v02.169] Work Packet mirror contracts round-trip authority mode, reconciliation action, manual-edit zones, and last reconciliation timestamps before readable packet views are trusted.",
        "[ADD v02.170] Work Packet view presets and action bindings round-trip by stable work_packet_id, view_id, and action_id before any promote, review-request, escalate, or route action is offered.",
        "[ADD v02.171] Work Packet detail views round-trip workflow-state families, queue-reason codes, and allowed action ids by stable work_packet_id before any project-profile label or view preset is applied.",
        "[ADD v02.172] Transition rule ids, queue automation rule ids, and executor eligibility policy ids round-trip by stable work_packet_id before any activation, retry, review-request, approval-request, or completion action is offered.",
        "[ADD v02.176] Mailbox claimant identity, claim mode, lease age, lease expiry, handback reason, and response-authority summaries round-trip by stable work_packet_id, mailbox_thread_id, and claim_id before any reroute, takeover, or follow-up action is offered.",
        "[ADD v02.178] Direct-load-versus-hybrid-retrieval posture for packet-linked follow-up context round-trips by stable work_packet_id, query_plan_id, and retrieval_trace_id before related-context search is trusted.",
        "[ADD v02.181] Work Packet detail views round-trip governed_action_id, gate_record_id, claim_id, queued_instruction_id, checkpoint_id, and closeout derivation inputs by stable work_packet_id before readable packet narratives or imported repo-governance artifacts are trusted."
      ],
      "tests": [
        "Work packet bindings, activation state, and linked micro-task occupancy remain consistent across workflow state, Locus storage, and Dev Command Center projection.",
        "[ADD v02.166] Structured Work Packet viewers preserve blockers, remaining work, evidence references, and append-only handoff notes without requiring full Markdown packet parsing.",
        "[ADD v02.167] Project-profile extensions do not break the base Work Packet viewer or summary ingestion path for smaller local models.",
        "[ADD v02.168] Base-envelope validation failures are surfaced separately from project-profile-extension validation failures so operators can see whether a record is generically unreadable or only partially specialized.",
        "[ADD v02.169] Regenerating a readable Work Packet mirror never silently overwrites append-only handoff notes or advisory operator narrative that still requires normalization.",
        "[ADD v02.170] Work Packet detail views explain which action binding would mutate state before the operator drags a card, triggers a quick action, or executes a bulk routing change.",
        "[ADD v02.171] Work Packet routing, review, approval, and completion views preserve the same base workflow-state family and queue reason code even when project-profile display labels differ.",
        "[ADD v02.172] Work Packet transitions and automatic queue moves preserve approval boundaries, blocked reasons, and executor eligibility even when project-profile labels differ.",
        "[ADD v02.176] Work Packet follow-up views never treat mailbox claim or lease state as packet authority and always show claimant, lease expiry, and linked thread identifiers before a takeover or reroute follow-up is offered.",
        "[ADD v02.178] Packet-linked follow-up views show when related context came from direct packet load, bounded graph traversal, or hybrid retrieval and never let related-search results override authoritative packet blockers or gate state.",
        "[ADD v02.181] Work Packet detail and note timelines never outrank workflow-backed validator-gate verdicts, claim/lease posture, queued steering/follow-up state, checkpoint-backed recovery posture, or derived closeout posture when readable mirrors or repo-governance artifacts drift."
      ]
    },
    {
      "feature_id": "FEAT-TERMINAL",
      "user_goal": "Run terminal commands with capability gating and provenance; prevent bypass of governance and logging.",
      "entry_points": [
        "Terminal surface",
        "Command palette: run command",
        "View job run history in Operator Consoles"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "running",
          "completed_ok",
          "completed_error",
          "denied"
        ],
        "errors": [
          "capability_denied",
          "sandbox_violation",
          "timeout"
        ]
      },
      "capability_gates": [
        "Command execution requires explicit capability grant; environment secrets are never logged."
      ],
      "telemetry": [
        "Terminal runs emit Flight Recorder events with bounded metadata."
      ],
      "tests": [
        "Denied commands produce visible error and no side effects."
      ]
    },
    {
      "feature_id": "FEAT-THINKING-PIPELINE",
      "user_goal": "Move work through Capture -> Organise -> Refine -> Synthesise with visible transitions rather than opaque cross-view jumps.",
      "entry_points": [
        "Thinking Pipeline action",
        "Project Brain follow-up",
        "Docs/Canvas workflow prompts"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "selecting_handoff",
          "executing_handoff",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "missing_source_binding",
          "handoff_validation_failed",
          "capability_denied"
        ]
      },
      "capability_gates": [
        "Pipeline transitions remain governed job/workflow operations with explicit source/target references."
      ],
      "telemetry": [
        "Each pipeline handoff is visible in Flight Recorder and future DCC/Locus projections."
      ],
      "tests": [
        "Thinking Pipeline transitions preserve source refs and destination entity bindings."
      ]
    },
    {
      "feature_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "user_goal": "Invoke any tool (local/MEX/MCP) through one gated, observable contract with approvals and replay-safe logging.",
      "entry_points": [
        "Any tool-triggering UI action",
        "Dev Command Center > Tool Call Ledger",
        "Operator Consoles > Job Inspector"
      ],
      "required_surfaces": [
        "ui",
        "backend"
      ],
      "interaction_contract": {
        "states": [
          "validated",
          "awaiting_approval",
          "allowed_executing",
          "denied",
          "completed_ok",
          "completed_error"
        ],
        "errors": [
          "capability_denied",
          "unknown_capability",
          "payload_requires_args_ref",
          "timeout",
          "canceled"
        ]
      },
      "capability_gates": [
        "CapabilityRegistry default-deny.",
        "session-scoped capability intersection when session_id present."
      ],
      "telemetry": [
        "FR-EVT-007 tool_call (Flight Recorder)."
      ],
      "tests": [
        "HTC-1.0 schema validation + payload caps.",
        "All tool calls are observable via Operator Consoles."
      ]
    },
    {
      "feature_id": "FEAT-CAPABILITIES-CONSENT",
      "user_goal": "Inspect, compare, and approve capability/consent state without losing the exact allowlist, risk class, or consent scope that governed a runtime action.",
      "entry_points": [
        "Consent review surfaces",
        "Capability inspector",
        "Operator deep links from jobs and tool actions"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "reviewing_snapshot",
          "awaiting_consent",
          "approved",
          "denied"
        ],
        "errors": [
          "snapshot_missing",
          "consent_scope_invalid",
          "policy_state_unavailable"
        ]
      },
      "capability_gates": [
        "No approval surface may imply permission beyond the explicit capability profile, consent scope, and governance mode."
      ],
      "telemetry": [
        "Capability snapshots and consent decisions remain deep-linkable to jobs, tool calls, and Flight Recorder evidence."
      ],
      "tests": [
        "Capability snapshot, consent scope, and resulting decision round-trip across UI and backend ids without drift."
      ]
    },
    {
      "feature_id": "FEAT-DEBUG-BUNDLE",
      "user_goal": "Export, validate, and download a bounded debug bundle with explicit scope, redaction, and manifest status instead of opaque archive generation.",
      "entry_points": [
        "Operator Consoles > Export bundle",
        "Timeline time-window export",
        "Jobs/Problems/Evidence export actions",
        "Workflow run export",
        "Workflow node execution export"
      ],
      "required_surfaces": [
        "ui",
        "backend",
        "operator_consoles"
      ],
      "interaction_contract": {
        "states": [
          "idle",
          "configuring_scope",
          "exporting",
          "ready",
          "validation_failed"
        ],
        "errors": [
          "invalid_scope",
          "workflow_scope_not_found",
          "policy_denied",
          "bundle_not_ready",
          "validation_failed"
        ]
      },
      "capability_gates": [
        "Bundle export stays governed by explicit scope, policy decisions, and redaction mode; no silent broadening of export range is allowed."
      ],
      "telemetry": [
        "Bundle manifest ids, export records, validation outcomes, and bundle status remain queryable in operator/runtime evidence surfaces."
      ],
      "tests": [
        "Time-window export preserves a stable scope hash and validation state across export, status polling, and download.",
        "Workflow-run and workflow-node-execution scope selection preserves explicit ids across export, status polling, validation, and download."
      ]
    }
  ]
}
```
<!-- HS_APPENDIX:END id=HS-APPX-UI-GUIDANCE -->

## 12.6 Appendix Block: INTERACTION_MATRIX (Feature/Primitive edges) [CX-SPEC-APPX-013]

<!-- HS_APPENDIX:BEGIN id=HS-APPX-INTERACTION-MATRIX schema=hs_interaction_matrix@2 -->
```json
{
  "schema": "hs_interaction_matrix@2",
  "spec_version": "v02.184",
  "last_updated": "2026-05-05",
  "edges": [
    {
      "edge_id": "IMX-001",
      "from_kind": "feature",
      "from_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "enforce_capabilities",
      "scope": "normative",
      "tokens": [
        "HTC-1.0",
        "session-scoped capability intersection"
      ],
      "spec_refs": [
        "Â§6.0.2",
        "#111-capabilities-consent-model"
      ],
      "notes": "Tool Gate/Registry enforce default-deny capabilities and consent decisions for all tool invocations."
    },
    {
      "edge_id": "IMX-002",
      "from_kind": "feature",
      "from_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "log_tool_calls",
      "scope": "normative",
      "tokens": [
        "FR-EVT-007"
      ],
      "spec_refs": [
        "Â§6.0.2",
        "#115-flight-recorder-event-shapes-retention"
      ],
      "notes": "Every tool call produces Flight Recorder events with bounded metadata and artifact refs."
    },
    {
      "edge_id": "IMX-003",
      "from_kind": "feature",
      "from_id": "FEAT-MCP-PRIMITIVES",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "route_mcp_through_same_gates",
      "scope": "normative",
      "tokens": [
        "MCP Gate",
        "no bypass"
      ],
      "spec_refs": [
        "#113-authsessionmcp-primitives",
        "Â§6.0.2"
      ],
      "notes": "MCP tool calls must be mediated by the same Tool Gate + capability enforcement + Flight Recorder logging as local tools."
    },
    {
      "edge_id": "IMX-004",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "creates_and_binds_work_packets",
      "scope": "normative",
      "tokens": [
        "WorkPacketBinding"
      ],
      "spec_refs": [
        "Â§2.6.8.5",
        "#2315-locus-work-tracking-system-add-v02116"
      ],
      "notes": "Spec Router creates/binds Work Packets and links governance artifacts into Locus."
    },
    {
      "edge_id": "IMX-005",
      "from_kind": "feature",
      "from_id": "FEAT-MICRO-TASK-EXECUTOR",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "records_iterations",
      "scope": "normative",
      "tokens": [
        "FR-EVT-MT-*"
      ],
      "spec_refs": [
        "#2668-micro-task-executor-profile",
        "#2315-locus-work-tracking-system-add-v02116"
      ],
      "notes": "MT Executor records MT lifecycle/iterations into Locus and emits correlated Flight Recorder events."
    },
    {
      "edge_id": "IMX-006",
      "from_kind": "feature",
      "from_id": "FEAT-ACE-RUNTIME",
      "to_kind": "feature",
      "to_id": "FEAT-AI-READY-DATA",
      "kind": "retrieval_pipeline",
      "scope": "normative",
      "tokens": [
        "ACE-RAG-001"
      ],
      "spec_refs": [
        "Â§2.6.6.7",
        "#2314-ai-ready-data-architecture-add-v02115"
      ],
      "notes": "ACE runtime compiles context via retrieval over Shadow Workspace/Indexes with QueryPlan + RetrievalTrace provenance."
    },
    {
      "edge_id": "IMX-007",
      "from_kind": "feature",
      "from_id": "FEAT-OPERATOR-CONSOLES",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "evidence_drilldown",
      "scope": "normative",
      "tokens": [
        "Jobs",
        "Timeline",
        "Evidence"
      ],
      "spec_refs": [
        "#105-operator-consoles-debug-diagnostics",
        "#115-flight-recorder-event-shapes-retention"
      ],
      "notes": "Operator Consoles are the primary evidence surface for Flight Recorder events, jobs, and tool calls."
    },
    {
      "edge_id": "IMX-008",
      "from_kind": "feature",
      "from_id": "FEAT-LOOM-LIBRARY",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "emit_loom_events",
      "scope": "normative",
      "tokens": [
        "FR-EVT-LOOM-*"
      ],
      "spec_refs": [
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "#115-flight-recorder-event-shapes-retention"
      ],
      "notes": "Loom import/view/tag actions emit Loom event families and are inspectable in Operator Consoles."
    },
    {
      "edge_id": "IMX-009",
      "from_kind": "feature",
      "from_id": "FEAT-MEDIA-DOWNLOADER",
      "to_kind": "feature",
      "to_id": "FEAT-STAGE",
      "kind": "reuse_stage_sessions_for_auth",
      "scope": "principle",
      "tokens": [
        "Stage Sessions"
      ],
      "spec_refs": [
        "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
        "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131"
      ],
      "notes": "Media Downloader may reuse Stage Sessions/auth context, but remains governed by capability and export boundaries."
    },
    {
      "edge_id": "IMX-010",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "query_by_time_window",
      "scope": "normative",
      "tokens": [
        "[ADD v02.142]",
        "ActivitySpan",
        "SessionSpan"
      ],
      "spec_refs": [
        "#104-calendar",
        "#115-flight-recorder-event-shapes-retention",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-001"
      ],
      "notes": "[ADD v02.142] Calendar time windows are a first-class lens over Flight Recorder activity/session spans."
    },
    {
      "edge_id": "IMX-011",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "correlate_workload_windows",
      "scope": "principle",
      "tokens": [
        "[ADD v02.142]",
        "occupancy",
        "time window"
      ],
      "spec_refs": [
        "#104-calendar",
        "#2315-locus-work-tracking-system-add-v02116",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-001",
        "RV-004"
      ],
      "notes": "[ADD v02.142] Calendar and Locus should converge on workload/time-window visibility instead of separate opaque views."
    },
    {
      "edge_id": "IMX-012",
      "from_kind": "feature",
      "from_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "surface_governed_tool_calls",
      "scope": "normative",
      "tokens": [
        "[ADD v02.142]",
        "Tool Ledger"
      ],
      "spec_refs": [
        "?6.0.2",
        "#1011-dev-command-center-sidecar-integration",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-003"
      ],
      "notes": "[ADD v02.142] DCC is the operator/runtime projection surface for governed local+cloud tool calling."
    },
    {
      "edge_id": "IMX-013",
      "from_kind": "feature",
      "from_id": "FEAT-LOCUS-WORK-TRACKING",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "project_execution_status",
      "scope": "principle",
      "tokens": [
        "[ADD v02.142]",
        "WP",
        "MT",
        "occupancy"
      ],
      "spec_refs": [
        "#2315-locus-work-tracking-system-add-v02116",
        "#1011-dev-command-center-sidecar-integration",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ],
      "notes": "[ADD v02.142] DCC should project Locus execution state instead of hiding it behind governance-only artifacts."
    },
    {
      "edge_id": "IMX-014",
      "from_kind": "feature",
      "from_id": "FEAT-LOOM-LIBRARY",
      "to_kind": "feature",
      "to_id": "FEAT-AI-READY-DATA",
      "kind": "retrieve_indexed_library",
      "scope": "normative",
      "tokens": [
        "[ADD v02.142]",
        "retrieval",
        "library"
      ],
      "spec_refs": [
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "#2314-ai-ready-data-architecture-add-v02115",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-005"
      ],
      "notes": "[ADD v02.142] Loom content must remain visible as a retrieval/runtime surface, not only a passive library."
    },
    {
      "edge_id": "IMX-015",
      "from_kind": "feature",
      "from_id": "FEAT-ATELIER-LENS",
      "to_kind": "feature",
      "to_id": "FEAT-LOOM-LIBRARY",
      "kind": "lens_uses_loom_context",
      "scope": "principle",
      "tokens": [
        "[ADD v02.142]",
        "Lens",
        "library context"
      ],
      "spec_refs": [
        "?6.3.3.5",
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-005"
      ],
      "notes": "[ADD v02.142] Lens/Atelier retrieval should be explainable through the same Loom runtime visibility rows used by operator surfaces."
    },
    {
      "edge_id": "IMX-016",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-LOOM-LIBRARY",
      "kind": "capture_into_library",
      "scope": "principle",
      "tokens": [
        "[ADD v02.142]",
        "capture/import",
        "artifact"
      ],
      "spec_refs": [
        "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131",
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-006",
        "RV-005"
      ],
      "notes": "[ADD v02.142] Stage capture/import is a natural upstream path for governed Loom ingestion and later library retrieval."
    },
    {
      "edge_id": "IMX-017",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "calendar_projects_into_dcc",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "calendar",
        "operator runtime projection"
      ],
      "spec_refs": [
        "#104-calendar",
        "#1011-dev-command-center-sidecar-integration",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-001",
        "RV-018"
      ],
      "notes": "[ADD v02.144] Calendar time windows should be projected into DCC as operator-visible workload context rather than a separate passive calendar view."
    },
    {
      "edge_id": "IMX-018",
      "from_kind": "feature",
      "from_id": "FEAT-MAIL-CLIENT",
      "to_kind": "feature",
      "to_id": "FEAT-CALENDAR",
      "kind": "mail_correlates_calendar",
      "scope": "normative",
      "tokens": [
        "[ADD v02.144]",
        "mail",
        "calendar invites"
      ],
      "spec_refs": [
        "#103-mail-client",
        "#104-calendar",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-019",
        "RV-001",
        "RV-002"
      ],
      "notes": "[ADD v02.144] Mail threads and invitations should feed governed Calendar correlation and draft-event flows with explicit evidence."
    },
    {
      "edge_id": "IMX-019",
      "from_kind": "feature",
      "from_id": "FEAT-PROJECT-BRAIN",
      "to_kind": "feature",
      "to_id": "FEAT-CONTEXT-PACKS",
      "kind": "brain_prefers_context_packs",
      "scope": "normative",
      "tokens": [
        "[ADD v02.144]",
        "project brain",
        "context packs"
      ],
      "spec_refs": [
        "#258-project-brain-rag-interface",
        "#2512-context-packs-ai-job-profile",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-010",
        "RV-017"
      ],
      "notes": "[ADD v02.144] Project Brain should prefer bounded ContextPack compactions when they are fresh enough to satisfy retrieval policy."
    },
    {
      "edge_id": "IMX-020",
      "from_kind": "feature",
      "from_id": "FEAT-THINKING-PIPELINE",
      "to_kind": "feature",
      "to_id": "FEAT-CANVAS",
      "kind": "pipeline_organises_into_canvas",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "thinking pipeline",
        "canvas"
      ],
      "spec_refs": [
        "#259-thinking-pipeline-docs--canvas--workflows",
        "#712-freeform-canvas-milanote-like",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-011",
        "RV-009"
      ],
      "notes": "[ADD v02.144] Thinking Pipeline must treat Canvas as an explicit handoff surface instead of a vague brainstorming destination."
    },
    {
      "edge_id": "IMX-021",
      "from_kind": "feature",
      "from_id": "FEAT-THINKING-PIPELINE",
      "to_kind": "feature",
      "to_id": "FEAT-DOCS-SHEETS",
      "kind": "pipeline_synthesises_docs",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "thinking pipeline",
        "docs sheets"
      ],
      "spec_refs": [
        "#259-thinking-pipeline-docs--canvas--workflows",
        "#2510-docs--sheets-ai-job-profile",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-011",
        "RV-008"
      ],
      "notes": "[ADD v02.144] Thinking Pipeline loops back into Docs & Sheets for synthesis, edits, and follow-up planning with explicit provenance."
    },
    {
      "edge_id": "IMX-022",
      "from_kind": "feature",
      "from_id": "FEAT-SEMANTIC-CATALOG",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "semantic_catalog_guides_tool_surface",
      "scope": "normative",
      "tokens": [
        "[ADD v02.144]",
        "semantic catalog",
        "tool routing"
      ],
      "spec_refs": [
        "#267-semantic-catalog-registry-normative",
        "?6.0.2",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-013",
        "RV-003"
      ],
      "notes": "[ADD v02.144] Semantic Catalog should provide deterministic routing hints into the Unified Tool Surface instead of runtime guessing."
    },
    {
      "edge_id": "IMX-023",
      "from_kind": "feature",
      "from_id": "FEAT-SKILL-BANK",
      "to_kind": "feature",
      "to_id": "FEAT-MICRO-TASK-EXECUTOR",
      "kind": "distillation_feedback_loop",
      "scope": "normative",
      "tokens": [
        "[ADD v02.144]",
        "skill bank",
        "micro task"
      ],
      "spec_refs": [
        "#9-continuous-local-skill-distillation-skill-bank-pipeline",
        "#2668-micro-task-executor-profile",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-012"
      ],
      "notes": "[ADD v02.144] Skill Bank distillation candidates and promotions should remain visible as a runtime consequence of Micro-Task execution."
    },
    {
      "edge_id": "IMX-024",
      "from_kind": "feature",
      "from_id": "FEAT-ASR",
      "to_kind": "feature",
      "to_id": "FEAT-LOOM-LIBRARY",
      "kind": "transcribe_into_library",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "asr",
        "loom"
      ],
      "spec_refs": [
        "#62-speech-recognition-asr-subsystem",
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-014",
        "RV-005"
      ],
      "notes": "[ADD v02.144] ASR transcripts should become Loom-searchable artifacts and preserve linkage back to their media sources."
    },
    {
      "edge_id": "IMX-025",
      "from_kind": "feature",
      "from_id": "FEAT-CHARTS-DASHBOARDS",
      "to_kind": "feature",
      "to_id": "FEAT-PRESENTATIONS-DECKS",
      "kind": "charts_flow_into_decks",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "charts",
        "decks"
      ],
      "spec_refs": [
        "#107-charts--dashboards",
        "#108-presentations-decks",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-015",
        "RV-016"
      ],
      "notes": "[ADD v02.144] Chart outputs should remain reusable as deck inputs with preserved provenance and export lineage."
    },
    {
      "edge_id": "IMX-026",
      "from_kind": "feature",
      "from_id": "FEAT-STUDIO",
      "to_kind": "feature",
      "to_id": "FEAT-ATELIER-LENS",
      "kind": "studio_shell_hosts_lens",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "studio",
        "lens"
      ],
      "spec_refs": [
        "#633-domain-2-creative-studio",
        "?6.3.3.5",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-007"
      ],
      "notes": "[ADD v02.144] Studio should present Lens / Atelier capabilities as governed runtime actions instead of isolated collaborator widgets."
    },
    {
      "edge_id": "IMX-027",
      "from_kind": "feature",
      "from_id": "FEAT-MAIL-CLIENT",
      "to_kind": "feature",
      "to_id": "FEAT-LOOM-LIBRARY",
      "kind": "mail_ingests_into_loom",
      "scope": "principle",
      "tokens": [
        "[ADD v02.144]",
        "mail",
        "loom"
      ],
      "spec_refs": [
        "#103-mail-client",
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "?6.0.2.10"
      ],
      "runtime_visibility_ids": [
        "RV-019",
        "RV-005"
      ],
      "notes": "[ADD v02.144] Mail content and attachments should remain first-class Loom/retrieval inputs with preserved thread provenance."
    },
    {
      "edge_id": "IMX-028",
      "from_kind": "feature",
      "from_id": "FEAT-PROJECT-BRAIN",
      "to_kind": "feature",
      "to_id": "FEAT-SEMANTIC-CATALOG",
      "kind": "retrieval_prefers_semantic_catalog",
      "scope": "principle",
      "tokens": [
        "[ADD v02.145]",
        "project brain",
        "semantic catalog"
      ],
      "spec_refs": [
        "#258-project-brain-rag-interface",
        "#267-semantic-catalog-registry-normative",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-011",
        "RV-013"
      ],
      "notes": "[ADD v02.145] Project Brain should use Semantic Catalog hints to keep retrieval, tool resolution, and grounding more deterministic for local/cloud models."
    },
    {
      "edge_id": "IMX-029",
      "from_kind": "feature",
      "from_id": "FEAT-THINKING-PIPELINE",
      "to_kind": "feature",
      "to_id": "FEAT-PROJECT-BRAIN",
      "kind": "pipeline_promotes_project_brain_followups",
      "scope": "principle",
      "tokens": [
        "[ADD v02.145]",
        "thinking pipeline",
        "project brain"
      ],
      "spec_refs": [
        "#259-thinking-pipeline-docs-?-canvas-?-workflows",
        "#258-project-brain-rag-interface",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-012",
        "RV-011"
      ],
      "notes": "[ADD v02.145] Thinking Pipeline should promote Project Brain retrieval follow-ups as explicit runtime handoffs instead of vague research-later transitions."
    },
    {
      "edge_id": "IMX-030",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "stage_routes_through_tool_surface",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "stage",
        "tool surface"
      ],
      "spec_refs": [
        "#1013-handshake-stage",
        "Â§6.0.2",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-006",
        "RV-003"
      ],
      "notes": "[ADD v02.145] Stage capture/import actions must remain visible as Unified Tool Surface executions rather than one-off bridge behavior."
    },
    {
      "edge_id": "IMX-031",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "stage_projects_into_dcc",
      "scope": "principle",
      "tokens": [
        "[ADD v02.145]",
        "stage",
        "dcc"
      ],
      "spec_refs": [
        "#1013-handshake-stage",
        "#1011-dev-command-center-sidecar-integration",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-006",
        "RV-018"
      ],
      "notes": "[ADD v02.145] Stage-originated capture/import work should project into DCC as jobs, approvals, and artifact evidence instead of remaining siloed in the Stage shell."
    },
    {
      "edge_id": "IMX-032",
      "from_kind": "feature",
      "from_id": "FEAT-MAIL-CLIENT",
      "to_kind": "feature",
      "to_id": "FEAT-PROJECT-BRAIN",
      "kind": "mail_threads_feed_project_brain",
      "scope": "principle",
      "tokens": [
        "[ADD v02.145]",
        "mail",
        "project brain"
      ],
      "spec_refs": [
        "#103-mail-client",
        "#258-project-brain-rag-interface",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-019",
        "RV-011"
      ],
      "notes": "[ADD v02.145] Mail threads and attachments should remain first-class Project Brain inputs with preserved provenance and evidence links."
    },
    {
      "edge_id": "IMX-033",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-PROJECT-BRAIN",
      "kind": "calendar_windows_ground_project_brain",
      "scope": "principle",
      "tokens": [
        "[ADD v02.145]",
        "calendar",
        "project brain"
      ],
      "spec_refs": [
        "#104-calendar",
        "#258-project-brain-rag-interface",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-001",
        "RV-011"
      ],
      "notes": "[ADD v02.145] Calendar windows should become a grounded Project Brain context lens instead of a passive scheduling side surface."
    },
    {
      "edge_id": "IMX-034",
      "from_kind": "feature",
      "from_id": "FEAT-AI-JOB-MODEL",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "jobs_materialize_through_workflows",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "job",
        "workflow"
      ],
      "spec_refs": [
        "#266-ai-job-model-global",
        "#26-workflow-automation-engine",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.145] AI Job state should remain explainable through the Workflow Engine nodes and session routing contracts that execute it."
    },
    {
      "edge_id": "IMX-035",
      "from_kind": "feature",
      "from_id": "FEAT-WORKFLOW-ENGINE",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "workflow_steps_emit_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "workflow",
        "flight recorder"
      ],
      "spec_refs": [
        "#26-workflow-automation-engine",
        "#115-flight-recorder-event-shapes-retention",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.145] Workflow execution must remain queryable through Flight Recorder filters and correlated engine/session events."
    },
    {
      "edge_id": "IMX-036",
      "from_kind": "feature",
      "from_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "sessions_constrain_tool_calls",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "session",
        "tool surface"
      ],
      "spec_refs": [
        "Â§4.3.9.12",
        "Â§6.0.2",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-020",
        "RV-003"
      ],
      "notes": "[ADD v02.145] Model sessions should remain explicit tool-call routing and approval context rather than hidden background state."
    },
    {
      "edge_id": "IMX-037",
      "from_kind": "feature",
      "from_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "cloud_consent_is_traceable",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "cloud consent",
        "flight recorder"
      ],
      "spec_refs": [
        "Â§11.1.7",
        "#115-flight-recorder-event-shapes-retention",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ],
      "notes": "[ADD v02.145] Cloud escalation approvals, denials, and projection outcomes must remain traceable in Flight Recorder without leaking raw payloads."
    },
    {
      "edge_id": "IMX-038",
      "from_kind": "feature",
      "from_id": "FEAT-MEX-RUNTIME",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "mex_runtime_flows_through_tool_surface",
      "scope": "normative",
      "tokens": [
        "[ADD v02.145]",
        "mex",
        "tool surface"
      ],
      "spec_refs": [
        "Â§11.8",
        "Â§6.0.2",
        "Â§6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-022",
        "RV-003"
      ],
      "notes": "[ADD v02.145] Mechanical engine execution should remain discoverable through the same governed tool surface used by local/cloud model calls."
    },
    {
      "edge_id": "IMX-039",
      "from_kind": "feature",
      "from_id": "FEAT-MEX-RUNTIME",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "mex_runtime_emits_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.146]",
        "mex",
        "flight recorder"
      ],
      "spec_refs": [
        "11.8",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-022"
      ],
      "notes": "[ADD v02.146] Mechanical engine execution must emit governed, Flight Recorder-visible events so runtime actions remain reviewable across operator surfaces."
    },
    {
      "edge_id": "IMX-040",
      "from_kind": "feature",
      "from_id": "FEAT-AI-JOB-MODEL",
      "to_kind": "feature",
      "to_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "kind": "jobs_pause_on_cloud_consent",
      "scope": "normative",
      "tokens": [
        "[ADD v02.146]",
        "job",
        "cloud consent"
      ],
      "spec_refs": [
        "#266-ai-job-model-global",
        "11.1.7",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ],
      "notes": "[ADD v02.146] Job lifecycle controls must surface pause/resume transitions around explicit cloud escalation consent so operator review and audit stay deterministic."
    },
    {
      "edge_id": "IMX-041",
      "from_kind": "feature",
      "from_id": "FEAT-OPERATOR-CONSOLES",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "operator_surfaces_launch_scoped_bundle_exports",
      "scope": "normative",
      "tokens": [
        "[ADD v02.147]",
        "operator consoles",
        "debug bundle"
      ],
      "spec_refs": [
        "10.5.5",
        "2.3.10",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.147] Problems, Jobs, Timeline, and Evidence surfaces are canonical launch points for bounded debug bundle export and must preserve stable scope/provenance ids."
    },
    {
      "edge_id": "IMX-042",
      "from_kind": "feature",
      "from_id": "FEAT-FLIGHT-RECORDER",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "timeline_filters_materialize_debug_bundle_scope",
      "scope": "normative",
      "tokens": [
        "[ADD v02.147]",
        "flight recorder",
        "time window",
        "debug bundle"
      ],
      "spec_refs": [
        "11.5",
        "2.3.10",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.147] Flight Recorder filters and selected event slices must be able to materialize deterministic debug bundle scopes instead of remaining view-only state."
    },
    {
      "edge_id": "IMX-043",
      "from_kind": "feature",
      "from_id": "FEAT-CAPABILITIES-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "kind": "capability_policy_governs_cloud_consent",
      "scope": "normative",
      "tokens": [
        "[ADD v02.147]",
        "capabilities",
        "cloud consent"
      ],
      "spec_refs": [
        "11.1",
        "11.1.7",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ],
      "notes": "[ADD v02.147] Cloud escalation consent is a specialization of the broader capability/consent system and must preserve explicit scope, policy, and denial semantics."
    },
    {
      "edge_id": "IMX-044",
      "from_kind": "feature",
      "from_id": "FEAT-CAPABILITIES-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "capability_snapshots_project_into_dcc",
      "scope": "principle",
      "tokens": [
        "[ADD v02.147]",
        "capability snapshot",
        "dcc"
      ],
      "spec_refs": [
        "11.1",
        "10.11",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.147] DCC should project effective capability and consent state for governed actions rather than forcing operators to reconstruct it from backend-only artifacts."
    },
    {
      "edge_id": "IMX-045",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-MEDIA-DOWNLOADER",
      "kind": "stage_owns_shared_media_auth_contracts",
      "scope": "principle",
      "tokens": [
        "[ADD v02.148]",
        "MdSessionsRegistryV0",
        "MdAuthMode"
      ],
      "spec_refs": [
        "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131",
        "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.148] Stage explicitly owns the shared media-session registry and auth-mode contracts that Media Downloader reuses for governed capture/auth flows."
    },
    {
      "edge_id": "IMX-046",
      "from_kind": "feature",
      "from_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "to_kind": "feature",
      "to_id": "FEAT-AI-JOB-MODEL",
      "kind": "multi_session_runtime_executes_job_lifecycles",
      "scope": "principle",
      "tokens": [
        "[ADD v02.148]",
        "MultiModelSession",
        "job lifecycle"
      ],
      "spec_refs": [
        "?4.3.9.12",
        "?2.6.6.4",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.148] MultiModelSession is the explicit runtime substrate that binds concurrent session steering to the canonical AI job lifecycle and operator-visible transitions."
    },
    {
      "edge_id": "IMX-047",
      "from_kind": "feature",
      "from_id": "FEAT-DEBUG-BUNDLE",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "retention_and_manifest_contracts_port_across_backends",
      "scope": "principle",
      "tokens": [
        "[ADD v02.148]",
        "RetentionReport",
        "ArtifactManifest"
      ],
      "spec_refs": [
        "#114-diagnostics-schema-problemsevents",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.148] Debug bundle retention reports, artifact manifests, and bundle indexes are portable recovery contracts and must survive backend swaps without losing evidence semantics."
    },
    {
      "edge_id": "IMX-048",
      "from_kind": "feature",
      "from_id": "FEAT-WORKFLOW-ENGINE",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "workflow_projection_materializes_bundle_scope",
      "scope": "normative",
      "tokens": [
        "[ADD v02.150]",
        "workflow run",
        "debug bundle"
      ],
      "spec_refs": [
        "#26-workflow-automation-engine",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.150] Workflow runs and node executions must be materializable as bounded debug bundle scope so backend failures are exportable without reconstructing execution state by hand."
    },
    {
      "edge_id": "IMX-049",
      "from_kind": "feature",
      "from_id": "FEAT-WORKFLOW-ENGINE",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "workflow_progress_projects_into_locus",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "workflow node",
        "locus sync"
      ],
      "spec_refs": [
        "#26-workflow-automation-engine",
        "#2315-locus-work-tracking-system-add-v02116",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ],
      "notes": "[ADD v02.150] Workflow node execution and gate state must project into Locus-ready/progress views through explicit sync contracts instead of ad hoc polling semantics."
    },
    {
      "edge_id": "IMX-050",
      "from_kind": "feature",
      "from_id": "FEAT-AI-JOB-MODEL",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "job_identity_bounds_export_scope",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "job_id",
        "debug bundle"
      ],
      "spec_refs": [
        "#266-ai-job-model-global",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.150] Stable job identity is a canonical export anchor for bounded evidence bundles and must round-trip through export, status, and download flows without scope drift."
    },
    {
      "edge_id": "IMX-051",
      "from_kind": "feature",
      "from_id": "FEAT-CAPABILITIES-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "policy_decisions_export_as_audit_evidence",
      "scope": "normative",
      "tokens": [
        "[ADD v02.150]",
        "capabilities",
        "policy decision",
        "debug bundle"
      ],
      "spec_refs": [
        "#111-capabilities-consent-model",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.150] Capability and consent decisions must be exportable as bounded audit evidence so later review does not depend on transient live projections only."
    },
    {
      "edge_id": "IMX-052",
      "from_kind": "feature",
      "from_id": "FEAT-CLOUD-ESCALATION-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "consent_receipts_bound_bundle_scopes",
      "scope": "normative",
      "tokens": [
        "[ADD v02.150]",
        "cloud consent",
        "consent receipt",
        "debug bundle"
      ],
      "spec_refs": [
        "11.1.7",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-021"
      ],
      "notes": "[ADD v02.150] Projection plans and consent receipts are canonical debug bundle scope inputs whenever cloud escalation behavior must be audited or replayed."
    },
    {
      "edge_id": "IMX-053",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "calendar_windows_bound_time_range_exports",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "calendar",
        "time window",
        "debug bundle"
      ],
      "spec_refs": [
        "#104-calendar",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-001"
      ],
      "notes": "[ADD v02.150] Calendar event windows are canonical backend time-range anchors for bounded exports that correlate jobs, activity spans, and recorder evidence."
    },
    {
      "edge_id": "IMX-054",
      "from_kind": "feature",
      "from_id": "FEAT-MEDIA-DOWNLOADER",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "media_capture_sessions_materialize_evidence_exports",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "media session",
        "debug bundle"
      ],
      "spec_refs": [
        "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.150] Media Downloader session records, auth state, and materialized artifacts are valid bounded debug bundle anchors for backend troubleshooting and replay."
    },
    {
      "edge_id": "IMX-055",
      "from_kind": "feature",
      "from_id": "FEAT-MEDIA-DOWNLOADER",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "media_artifacts_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "artifact manifest",
        "retention report"
      ],
      "spec_refs": [
        "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.150] Media capture outputs must preserve portable manifest, bundle-index, and retention semantics across backend/storage migrations."
    },
    {
      "edge_id": "IMX-056",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "stage_sessions_launch_portable_debug_exports",
      "scope": "principle",
      "tokens": [
        "[ADD v02.150]",
        "stage",
        "session record",
        "debug bundle"
      ],
      "spec_refs": [
        "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-006"
      ],
      "notes": "[ADD v02.150] Stage session records and capture/import artifacts must be exportable through the same bounded debug bundle surface used for backend evidence and portability."
    },
    {
      "edge_id": "IMX-057",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "mailbox_actions_emit_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.151]",
        "role mailbox",
        "flight recorder",
        "export evidence"
      ],
      "spec_refs": [
        "?2.6.8.10",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.151] Role Mailbox create, transcription-link, and export actions must emit Flight Recorder-visible backend evidence so coordination audit and bounded export correlation do not depend on UI state alone."
    },
    {
      "edge_id": "IMX-058",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "mailbox_exports_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.151]",
        "role mailbox",
        "export manifest",
        "storage portability"
      ],
      "spec_refs": [
        "?2.6.8.10",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.151] Role Mailbox repository exports and manifest files must preserve portable storage, retention, and evidence semantics across backend swaps."
    },
    {
      "edge_id": "IMX-059",
      "from_kind": "feature",
      "from_id": "FEAT-AI-READY-DATA",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "index_lifecycle_emits_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.151]",
        "ai-ready data",
        "index rebuild",
        "flight recorder"
      ],
      "spec_refs": [
        "#2314-ai-ready-data-architecture-add-v02115",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.151] AI-ready index update and rebuild events must remain Flight Recorder-visible backend evidence so retrieval posture changes are reviewable and exportable."
    },
    {
      "edge_id": "IMX-060",
      "from_kind": "feature",
      "from_id": "FEAT-AI-READY-DATA",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "index_artifacts_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.151]",
        "ai-ready data",
        "artifact index",
        "storage portability"
      ],
      "spec_refs": [
        "#2314-ai-ready-data-architecture-add-v02115",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.151] Embedding, vector, keyword, and graph index artifacts must preserve portable manifest, retention, and evidence semantics across backend/storage changes."
    },
    {
      "edge_id": "IMX-061",
      "from_kind": "feature",
      "from_id": "FEAT-WORKFLOW-ENGINE",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "workflow_artifacts_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.151]",
        "workflow artifact",
        "run ledger",
        "storage portability"
      ],
      "spec_refs": [
        "#26-workflow-automation-engine",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.151] Workflow progress artifacts, run ledgers, projection plans, and artifact manifests are portable backend evidence roots that must survive storage/backend swaps without semantic drift."
    },
    {
      "edge_id": "IMX-062",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "spec_router_artifacts_emit_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.152]",
        "spec router",
        "flight recorder",
        "prompt envelope hash"
      ],
      "spec_refs": [
        "Â§2.6.8.5",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.152] Spec Router prompt artifacts, prompt-envelope hashes, capability snapshots, and routing decisions must remain Flight Recorder-visible backend evidence so routing and WP-creation outcomes are replayable."
    },
    {
      "edge_id": "IMX-063",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "spec_router_artifacts_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.152]",
        "spec router",
        "decision artifact",
        "storage portability"
      ],
      "spec_refs": [
        "Â§2.6.8.5",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.152] Spec Router prompt and decision artifacts must preserve stable manifest, hash, and retention semantics across backend/storage changes and later replay/export tooling."
    },
    {
      "edge_id": "IMX-064",
      "from_kind": "feature",
      "from_id": "FEAT-LOCUS-WORK-TRACKING",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "locus_operations_emit_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.152]",
        "locus",
        "flight recorder",
        "task board sync"
      ],
      "spec_refs": [
        "#2315-locus-work-tracking-system-add-v02116",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ],
      "notes": "[ADD v02.152] Locus WP/MT/dependency/query/task-board operations must emit Flight Recorder-visible backend evidence so readiness, occupancy, and sync state stay replayable across operator and model surfaces."
    },
    {
      "edge_id": "IMX-065",
      "from_kind": "feature",
      "from_id": "FEAT-MCP-PRIMITIVES",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "mcp_tool_call_evidence_materializes_bundle_scope",
      "scope": "principle",
      "tokens": [
        "[ADD v02.152]",
        "mcp",
        "debug bundle",
        "redacted payload"
      ],
      "spec_refs": [
        "#113-authsessionmcp-primitives",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.152] Redacted MCP args/results, JSON-RPC envelopes, and gate outcomes are valid bounded debug-bundle evidence inputs; detailed export scope rules remain stub-backed until explicit MCP evidence contracts land."
    },
    {
      "edge_id": "IMX-066",
      "from_kind": "feature",
      "from_id": "FEAT-MEX-RUNTIME",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "mex_runtime_evidence_materializes_bundle_scope",
      "scope": "principle",
      "tokens": [
        "[ADD v02.152]",
        "mex runtime",
        "debug bundle",
        "denial diagnostic"
      ],
      "spec_refs": [
        "Â§11.8",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.152] MEX tool-call results, denial diagnostics, capability actions, and gate outcomes are valid bounded debug-bundle evidence inputs; detailed export scope rules remain stub-backed until explicit MEX evidence contracts land."
    },
    {
      "edge_id": "IMX-067",
      "from_kind": "feature",
      "from_id": "FEAT-CAPABILITIES-CONSENT",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "capability_enforcement_emits_audit_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.153]",
        "capability action",
        "flight recorder",
        "allow deny"
      ],
      "spec_refs": [
        "Â§11.1",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.153] Workflow capability allow/deny decisions are canonical Flight Recorder-visible audit evidence and MAY NOT remain implicit inside enforcement helpers."
    },
    {
      "edge_id": "IMX-068",
      "from_kind": "feature",
      "from_id": "FEAT-WORKFLOW-ENGINE",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "workflow_execution_enforces_capability_profiles",
      "scope": "normative",
      "tokens": [
        "[ADD v02.153]",
        "workflow engine",
        "capability profile",
        "required capabilities"
      ],
      "spec_refs": [
        "#26-workflow-automation-engine",
        "Â§11.1",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.153] Workflow execution is the production enforcement surface for required capability checks and MUST keep that coupling explicit instead of treating capabilities as preflight-only metadata."
    },
    {
      "edge_id": "IMX-069",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "spec_router_materializes_capability_snapshots",
      "scope": "principle",
      "tokens": [
        "[ADD v02.153]",
        "spec router",
        "capability snapshot",
        "allowlist"
      ],
      "spec_refs": [
        "Â§2.6.8.5",
        "Â§11.1",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.153] Spec Router capability snapshots are a direct governance seam between prompt/spec compilation and capability enforcement, not only an evidence artifact."
    },
    {
      "edge_id": "IMX-070",
      "from_kind": "feature",
      "from_id": "FEAT-MCP-PRIMITIVES",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "mcp_tool_events_emit_direct_recorder_correlation",
      "scope": "normative",
      "tokens": [
        "[ADD v02.153]",
        "mcp",
        "flight recorder",
        "tool result progress"
      ],
      "spec_refs": [
        "#113-authsessionmcp-primitives",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.153] MCP tool call, result, and progress envelopes are direct Flight Recorder-visible backend evidence and MUST stay explicit even when later bundle/export contracts change."
    },
    {
      "edge_id": "IMX-071",
      "from_kind": "feature",
      "from_id": "FEAT-DIAGNOSTICS-SCHEMA",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "diagnostics_materialize_bounded_bundle_scope",
      "scope": "normative",
      "tokens": [
        "[ADD v02.153]",
        "diagnostics",
        "debug bundle",
        "validation report"
      ],
      "spec_refs": [
        "#114-diagnostics-schema-problemsevents",
        "#2310-debug-bundle-export",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.153] Diagnostics queries, grouped problems, and validation findings are canonical debug-bundle inputs and MAY NOT remain implied only by exporter internals."
    },
    {
      "edge_id": "IMX-072",
      "from_kind": "feature",
      "from_id": "FEAT-GOVERNANCE-PACK",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "governance_pack_runs_as_governed_export_workflow",
      "scope": "normative",
      "tokens": [
        "[ADD v02.154]",
        "governance pack export",
        "workflow engine"
      ],
      "spec_refs": [
        "#7548-governance-pack-project-specific-instantiation-hard",
        "#26-workflow-automation-engine",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ],
      "notes": "[ADD v02.154] Governance Pack export is a first-class workflow-run/export lifecycle and MAY NOT bypass the Workflow Engine through ad hoc file generation. [ADD v02.181] Governance Pack import or export may carry repo-governance overlay source artifacts, but runtime software-delivery authority still resolves through workflow-backed product state."
    },
    {
      "edge_id": "IMX-073",
      "from_kind": "feature",
      "from_id": "FEAT-GOVERNANCE-PACK",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "governance_pack_enforces_export_capabilities",
      "scope": "normative",
      "tokens": [
        "[ADD v02.154]",
        "export.governance_pack",
        "capability profile"
      ],
      "spec_refs": [
        "#7548-governance-pack-project-specific-instantiation-hard",
        "#111-capabilities-consent-model",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ],
      "notes": "[ADD v02.154] Governance Pack export must remain explicitly capability-gated and recorder-auditable instead of implicit in operator tooling."
    },
    {
      "edge_id": "IMX-074",
      "from_kind": "feature",
      "from_id": "FEAT-GOVERNANCE-PACK",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "governance_pack_export_emits_traceable_events",
      "scope": "normative",
      "tokens": [
        "[ADD v02.154]",
        "governance_pack_export",
        "flight recorder"
      ],
      "spec_refs": [
        "#7548-governance-pack-project-specific-instantiation-hard",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ],
      "notes": "[ADD v02.154] Governance Pack export requests, lifecycle updates, and outcomes are canonical Flight Recorder evidence seams for audit and replay."
    },
    {
      "edge_id": "IMX-075",
      "from_kind": "feature",
      "from_id": "FEAT-GOVERNANCE-PACK",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "governance_pack_exports_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.154]",
        "artifact manifest",
        "portable export"
      ],
      "spec_refs": [
        "#7548-governance-pack-project-specific-instantiation-hard",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-023"
      ],
      "notes": "[ADD v02.154] Governance Pack export records and generated artifacts must preserve portable manifest semantics across backend swaps and later transfer tooling."
    },
    {
      "edge_id": "IMX-076",
      "from_kind": "feature",
      "from_id": "FEAT-WORKSPACE-BUNDLE",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "workspace_bundle_exports_preserve_portable_manifests",
      "scope": "principle",
      "tokens": [
        "[ADD v02.154]",
        "workspace bundle",
        "portable manifest"
      ],
      "spec_refs": [
        "#1057-workspace-bundle-export-v0",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.154] Workspace Bundle export is a normative portability surface and must preserve stable manifest/hash semantics even while implementation remains stub-backed."
    },
    {
      "edge_id": "IMX-077",
      "from_kind": "feature",
      "from_id": "FEAT-WORKSPACE-BUNDLE",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "workspace_bundle_export_emits_traceable_events",
      "scope": "principle",
      "tokens": [
        "[ADD v02.154]",
        "workspace bundle",
        "bundle lifecycle",
        "flight recorder"
      ],
      "spec_refs": [
        "#1057-workspace-bundle-export-v0",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "notes": "[ADD v02.154] Workspace Bundle export lifecycle must remain recorder-visible as the bundle surface matures from stub-backed appendix ownership to implemented backend transfer."
    },
    {
      "edge_id": "IMX-078",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "calendar_sync_state_and_export_posture_remain_portable",
      "scope": "normative",
      "tokens": [
        "[ADD v02.155]",
        "sync state",
        "export mode",
        "portable"
      ],
      "spec_refs": [
        "#104-calendar",
        "#312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-001",
        "RV-002",
        "RV-024"
      ],
      "notes": "[ADD v02.155] Calendar source sync state, write policy, and export mode are canonical backend contracts that must preserve stable meaning across SQLite-now / PostgreSQL-ready storage and later export tooling."
    },
    {
      "edge_id": "IMX-079",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "calendar_policy_and_export_modes_enforce_consent_posture",
      "scope": "normative",
      "tokens": [
        "[ADD v02.155]",
        "capability profile",
        "export mode",
        "write policy"
      ],
      "spec_refs": [
        "#104-calendar",
        "#111-capabilities-consent-model",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-002",
        "RV-024"
      ],
      "notes": "[ADD v02.155] Calendar source capability profiles, write policies, and event export modes are backend consent controls and may not be reduced to UI preferences or hidden storage flags."
    },
    {
      "edge_id": "IMX-080",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-AI-JOB-MODEL",
      "kind": "calendar_mutations_and_policy_selection_run_as_jobs",
      "scope": "normative",
      "tokens": [
        "[ADD v02.155]",
        "calendar_sync",
        "WorkflowRun",
        "job boundary"
      ],
      "spec_refs": [
        "#104-calendar",
        "#43-ai-job-model",
        "2.6.6.8.15.3"
      ],
      "runtime_visibility_ids": [
        "RV-002",
        "RV-024"
      ],
      "notes": "[ADD v02.155] Calendar writes and policy-derived mutation posture must remain explicit AI-job / workflow behavior through calendar_sync instead of bypassing the job model with direct storage or view-layer mutations."
    },
    {
      "edge_id": "IMX-081",
      "from_kind": "feature",
      "from_id": "FEAT-CALENDAR",
      "to_kind": "feature",
      "to_id": "FEAT-SPEC-ROUTER",
      "kind": "calendar_scope_hints_shape_prompt_routing",
      "scope": "principle",
      "tokens": [
        "[ADD v02.155]",
        "CalendarScopeHint",
        "scope_hint",
        "policy_profile_id"
      ],
      "spec_refs": [
        "#104-calendar",
        "#268-spec-creation-system",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-024"
      ],
      "notes": "[ADD v02.155] CalendarScopeHint is a backend routing and policy-selection contract for ACE / Spec Router inputs; it must remain bounded, logged, and portable instead of living only in prompt-time prose."
    },
    {
      "edge_id": "IMX-082",
      "from_kind": "feature",
      "from_id": "FEAT-PROJECT-BRAIN",
      "to_kind": "feature",
      "to_id": "FEAT-AI-READY-DATA",
      "kind": "project_brain_reuses_ai_ready_retrieval_substrate",
      "scope": "normative",
      "tokens": [
        "[ADD v02.156]",
        "QueryPlan",
        "RetrievalTrace",
        "hybrid retrieval"
      ],
      "spec_refs": [
        "#258-project-brain-rag-interface",
        "#2314-ai-ready-data-architecture-add-v02115",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-010"
      ],
      "notes": "[ADD v02.156] Project Brain must reuse the AI-Ready Data retrieval substrate through explicit QueryPlan and RetrievalTrace contracts instead of introducing a notebook-only retrieval path."
    },
    {
      "edge_id": "IMX-083",
      "from_kind": "feature",
      "from_id": "FEAT-SEMANTIC-CATALOG",
      "to_kind": "feature",
      "to_id": "FEAT-SPEC-ROUTER",
      "kind": "semantic_catalog_indexes_router_decisions",
      "scope": "normative",
      "tokens": [
        "[ADD v02.156]",
        "deterministic routing",
        "semantic catalog",
        "spec router"
      ],
      "spec_refs": [
        "#267-semantic-catalog-registry-normative",
        "#268-spec-creation-system",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-013"
      ],
      "notes": "[ADD v02.156] Semantic Catalog entries are indexed backend routing contracts for Spec Router and retrieval-backed runtime planning, not optional prompt-time hints."
    },
    {
      "edge_id": "IMX-084",
      "from_kind": "feature",
      "from_id": "FEAT-CONTEXT-PACKS",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "context_packs_preserve_portable_retrieval_artifacts",
      "scope": "principle",
      "tokens": [
        "[ADD v02.156]",
        "ContextPackRecord",
        "freshness guard",
        "portable artifact"
      ],
      "spec_refs": [
        "#2512-context-packs-ai-job-profile",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ],
      "notes": "[ADD v02.156] Context Pack payloads, anchors, freshness guards, and canonical artifact serialization must preserve stable meaning across storage backends and later replay/export flows."
    },
    {
      "edge_id": "IMX-085",
      "from_kind": "feature",
      "from_id": "FEAT-LOOM-LIBRARY",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "loom_library_preserves_portable_graph_library_artifacts",
      "scope": "principle",
      "tokens": [
        "[ADD v02.156]",
        "LoomBlock",
        "LoomEdge",
        "portable graph"
      ],
      "spec_refs": [
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "2.3.13.7"
      ],
      "runtime_visibility_ids": [
        "RV-005"
      ],
      "notes": "[ADD v02.156] Loom block-edge records, search/view filters, and source anchors are portable backend library artifacts that must survive storage swaps, export, and replay without semantic drift."
    },
    {
      "edge_id": "IMX-086",
      "from_kind": "feature",
      "from_id": "FEAT-ACE-RUNTIME",
      "to_kind": "feature",
      "to_id": "FEAT-CONTEXT-PACKS",
      "kind": "ace_runtime_reuses_context_pack_payloads_and_freshness_decisions",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.157]",
        "ContextPackPayloadV1",
        "ContextPackFreshnessPolicyV1",
        "deterministic compaction"
      ],
      "spec_refs": [
        "#2512-context-packs-ai-job-profile",
        "#2667-ace-runtime-agentic-context-engineering",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ],
      "notes": "[ADD v02.157] ACE runtime reuses Context Pack payloads and freshness-policy decisions as bounded backend compactions for deterministic job assembly, replay, and later distillation evidence."
    },
    {
      "edge_id": "IMX-087",
      "from_kind": "feature",
      "from_id": "FEAT-MICRO-TASK-EXECUTOR",
      "to_kind": "feature",
      "to_id": "FEAT-SKILL-BANK",
      "kind": "micro_task_executor_emits_distillation_candidate_lineage_for_skill_bank",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.157]",
        "PendingDistillationCandidate",
        "MicroTaskDistillationCandidate",
        "teacher/student lineage"
      ],
      "spec_refs": [
        "#2668-micro-task-executor-profile",
        "#9-continuous-local-skill-distillation-skill-bank-pipeline",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-012"
      ],
      "notes": "[ADD v02.157] Micro-Task Executor escalation emits pending candidate queues, teacher/student lineage, and trust-tagged distillation evidence that feed the Skill Bank backend contract."
    },
    {
      "edge_id": "IMX-088",
      "from_kind": "feature",
      "from_id": "FEAT-CONTEXT-PACKS",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "context_pack_build_select_refresh_is_recorder_visible",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.157]",
        "ContextPackFreshnessDecision",
        "pack hash",
        "build/select/refresh"
      ],
      "spec_refs": [
        "#2512-context-packs-ai-job-profile",
        "#115-flight-recorder-event-shapes--retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ],
      "notes": "[ADD v02.157] Context Pack build, select, refresh, and freshness-decision outcomes must remain Flight Recorder-visible backend evidence for replay, distillation, and model onboarding."
    },
    {
      "edge_id": "IMX-089",
      "from_kind": "feature",
      "from_id": "FEAT-SKILL-BANK",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "skill_bank_preserves_checkpoint_eval_and_adapter_lineage",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.157]",
        "AdapterCheckpoint",
        "benchmark-gated promotion",
        "rollback lineage"
      ],
      "spec_refs": [
        "#9-continuous-local-skill-distillation-skill-bank-pipeline",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-012"
      ],
      "notes": "[ADD v02.157] Skill Bank checkpoints, eval metadata, and adapter-only late-stage promotion posture are portable backend artifacts that must survive storage swaps and replay without losing lineage or rollback semantics."
    },
    {
      "edge_id": "IMX-090",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-CONTEXT-PACKS",
      "kind": "spec_router_preserves_context_pack_hashes_and_prompt_reuse",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.157]",
        "SpecPromptPackV1",
        "PromptEnvelopeV1",
        "ContextPack hash"
      ],
      "spec_refs": [
        "#266665-spec-router-job-profile-normative",
        "#2512-context-packs-ai-job-profile",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-017"
      ],
      "notes": "[ADD v02.157] Spec Router prompt compilation may reuse Context Packs, but it must preserve effective pack hashes, freshness decisions, and PromptEnvelope evidence so routing remains deterministic and replayable."
    },
    {
      "edge_id": "IMX-091",
      "from_kind": "feature",
      "from_id": "FEAT-ASR",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "asr_pipeline_emits_traceable_transcript_events",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.158]",
        "transcript artifact",
        "progress",
        "failure reason"
      ],
      "spec_refs": [
        "#62-speech-recognition-asr-subsystem",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-014"
      ],
      "notes": "[ADD v02.158] ASR progress, transcript artifact creation, and bounded failure reasons are canonical Flight Recorder-visible backend evidence instead of UI-only progress output."
    },
    {
      "edge_id": "IMX-092",
      "from_kind": "feature",
      "from_id": "FEAT-ASR",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "asr_artifacts_preserve_media_probe_and_timing_lineage",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.158]",
        "transcript artifact",
        "ffprobe",
        "timing anchor"
      ],
      "spec_refs": [
        "#62-speech-recognition-asr-subsystem",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-014"
      ],
      "notes": "[ADD v02.158] Transcript artifacts, source-media hashes, ffprobe-derived media facts, and timing anchors must preserve stable meaning across storage/export/replay flows."
    },
    {
      "edge_id": "IMX-093",
      "from_kind": "feature",
      "from_id": "FEAT-MEDIA-DOWNLOADER",
      "to_kind": "feature",
      "to_id": "FEAT-ASR",
      "kind": "downloaded_media_flows_into_transcript_jobs",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.158]",
        "MediaSource",
        "transcript artifact",
        "time anchors"
      ],
      "spec_refs": [
        "#1014-media-downloader-unified-web-media-archiving-surface-add-v02134",
        "#62-speech-recognition-asr-subsystem",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-014"
      ],
      "notes": "[ADD v02.158] Media Downloader outputs are canonical ASR inputs when policy allows; transcript artifacts must retain provenance back to the originating downloaded media artifact."
    },
    {
      "edge_id": "IMX-094",
      "from_kind": "feature",
      "from_id": "FEAT-STAGE",
      "to_kind": "feature",
      "to_id": "FEAT-STORAGE-PORTABILITY",
      "kind": "stage_capture_artifacts_preserve_portable_manifests",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.158]",
        "capture/import",
        "artifact manifest",
        "replay evidence"
      ],
      "spec_refs": [
        "#1013-handshake-stage-built-in-browser--stage-apps-add-v02131",
        "#2312-storage-backend-portability-architecture-cx-dbp-001",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-006"
      ],
      "notes": "[ADD v02.158] Stage capture/import artifacts must preserve stable capture-session provenance, manifest semantics, and replay evidence across backend/storage changes."
    },
    {
      "edge_id": "IMX-095",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-FLIGHT-RECORDER",
      "kind": "control_surface_projects_recorder_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.159]",
        "Dev Command Center",
        "Flight Recorder",
        "timeline correlation"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#115-flight-recorder-event-shapes-retention",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.159] Dev Command Center is the canonical control and projection surface for Flight Recorder-backed timelines, trace joins, governed execution visibility, and operator correlation workflows."
    },
    {
      "edge_id": "IMX-096",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-DEBUG-BUNDLE",
      "kind": "control_surface_launches_bounded_bundle_exports",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.159]",
        "Dev Command Center",
        "Debug Bundle",
        "bounded export"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#983-debug-bundle",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.159] Dev Command Center is the canonical control and projection umbrella for bounded Debug Bundle launch flows; Operator Consoles remain the specialized evidence and diagnostics views that drive drilldown and evidence selection."
    },
    {
      "edge_id": "IMX-097",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "mailbox_events_project_into_control_surface",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.159]",
        "Role Mailbox",
        "Dev Command Center",
        "projection"
      ],
      "spec_refs": [
        "Â§2.6.8.10",
        "#1011-dev-command-center-sidecar-integration",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.159] Role Mailbox message, transcription, and export events are backend projection inputs for Dev Command Center so coordination audit, approval context, and worktree or session steering do not depend on mailbox-only views. [ADD v02.166] Dev Command Center also triages structured mailbox state including expected response posture, expiry, evidence references, and handoff completeness so collaboration status is visible without transcript-only parsing."
    },
    {
      "edge_id": "IMX-098",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "control_surface_projects_workflow_execution",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.160]",
        "Dev Command Center",
        "workflow run",
        "workflow node execution"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#26-workflow-automation-engine",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.160] Dev Command Center is the canonical control-plane projection surface for workflow runs, workflow node executions, and governed step state; workflow execution MAY NOT remain readable only through backend logs or storage tables."
    },
    {
      "edge_id": "IMX-099",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-AI-JOB-MODEL",
      "kind": "control_surface_projects_jobs_and_resume_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.160]",
        "Dev Command Center",
        "artificial intelligence job",
        "resume"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#266-ai-job-model-global",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.160] Dev Command Center is the governed control-plane projection for artificial intelligence job state, resume actions, and job-to-workflow correlation rather than a passive job viewer."
    },
    {
      "edge_id": "IMX-100",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-CAPABILITIES-CONSENT",
      "kind": "control_surface_routes_capability_and_approval_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.160]",
        "Dev Command Center",
        "capability snapshot",
        "approval decision"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "11.1",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.160] Dev Command Center projects effective capability state and routes governed approval decisions, but the capability system remains authoritative for policy and denial semantics."
    },
    {
      "edge_id": "IMX-101",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "kind": "control_surface_projects_and_steers_model_sessions",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.160]",
        "Dev Command Center",
        "Model Session",
        "session registry"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§4.3.9.12",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.160] Dev Command Center is the canonical projection and steering surface for model-session scheduler state, queue occupancy, and work packet or micro-task session binding backed by the session registry."
    },
    {
      "edge_id": "IMX-102",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-GOVERNANCE-PACK",
      "kind": "control_surface_projects_governance_export_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.161]",
        "Dev Command Center",
        "Governance Pack",
        "export lifecycle"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#7548-governance-pack-project-specific-instantiation-hard",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018",
        "RV-023"
      ],
      "notes": "[ADD v02.161] Dev Command Center is the governed evidence-and-replay projection surface for Governance Pack export requests, lifecycle state, manifest identifiers, and outcomes; export authority remains in the Workflow Engine and capability gates. [ADD v02.181] Imported repo-governance overlay artifacts may also project here as evidence or transfer state, but Dev Command Center remains a projection over product-owned runtime authority rather than an importer-owned truth source."
    },
    {
      "edge_id": "IMX-103",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-WORKSPACE-BUNDLE",
      "kind": "control_surface_projects_workspace_bundle_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.161]",
        "Dev Command Center",
        "Workspace Bundle",
        "export record"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#1057-workspace-bundle-export-v0",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.161] Dev Command Center is the governed evidence-and-replay projection surface for Workspace Bundle export records, manifest identifiers, validation state, and backup or transfer status; delivery remains stub-backed until Workspace Bundle implementation lands."
    },
    {
      "edge_id": "IMX-104",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-DIAGNOSTICS-SCHEMA",
      "kind": "control_surface_projects_diagnostics_query_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.161]",
        "Dev Command Center",
        "Diagnostics Schema",
        "query state"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#114-diagnostics-schema-problemsevents",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.161] Dev Command Center projects diagnostics query state, grouped problem materialization, and evidence references as durable evidence-and-replay state; Operator Consoles remain the specialized drilldown and export-launch surfaces."
    },
    {
      "edge_id": "IMX-105",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "control_surface_projects_work_packet_and_task_board_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.162]",
        "Dev Command Center",
        "tracked Work Packet",
        "Task Board",
        "ready-query"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#2315-locus-work-tracking-system-add-v02116",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.162] Dev Command Center is the canonical work-orchestration projection surface for tracked Work Packet status, Task Board freshness, ready-query results, and workflow-linked work packet activation backed by authoritative Locus artifacts. [ADD v02.166] That projection also includes structured Work Packet records, structured Micro-Task contracts, Task Board projection rows, and append-only note timelines backed by authoritative Locus storage instead of Markdown mirrors. [ADD v02.167] Locus-backed collaboration records also expose compact summaries and project-profile metadata so Dev Command Center can switch layouts without losing authoritative field provenance. [ADD v02.168] Locus and Dev Command Center also share the same base structured-collaboration envelope and mirror-state semantics so generic viewers remain valid across project kernels. [ADD v02.169] Locus-backed mirror contracts now also let Dev Command Center explain whether drift came from canonical changes, advisory human edits, or reconciliation failures before a readable mirror is trusted. [ADD v02.181] This projection now also includes software-delivery validator-gate summaries, governed-action resolution state, overlay claim/lease posture, queued steering/follow-up state, checkpoint-backed recovery posture, and derived closeout posture from product-owned runtime records rather than repo mirrors or mailbox chronology."
    },
    {
      "edge_id": "IMX-106",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-MICRO-TASK-EXECUTOR",
      "kind": "control_surface_projects_micro_task_execution_state",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.162]",
        "Dev Command Center",
        "Micro-Task summary",
        "hard-gate state",
        "iteration progress"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "#2668-micro-task-executor-profile",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.162] Dev Command Center is the canonical work-orchestration projection surface for micro-task summaries, iteration progress, hard-gate state, and session-occupancy bindings backed by Micro-Task Executor and Locus artifacts. [ADD v02.170] That projection also includes compact-summary-first execution queues for local-small-model readiness, escalation-required items, mailbox-response blockers, and governed next actions. [ADD v02.171] That projection also surfaces the base workflow-state family, queue-reason code, and allowed action ids behind each execution bucket so local-small-model routing does not depend on lane naming alone. [ADD v02.172] That projection also surfaces the transition rule ids, automation triggers, and executor eligibility policies behind each execution bucket so local-small-model routing and retry posture remain explainable."
    },
    {
      "edge_id": "IMX-107",
      "from_kind": "feature",
      "from_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "parallel_sessions_bind_to_work_packet_and_micro_task_occupancy",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.162]",
        "parallel model sessions",
        "tracked Work Packets",
        "Micro-Task occupancy"
      ],
      "spec_refs": [
        "Â§4.3.9.12",
        "#2315-locus-work-tracking-system-add-v02116",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.162] Model Session Orchestration is the canonical backend substrate that binds parallel model sessions to tracked Work Packets, ready-work selection, and Micro-Task occupancy through authoritative Locus and session-registry artifacts."
    },
    {
      "edge_id": "IMX-108",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-TASK-BOARD",
      "kind": "control_surface_projects_task_board_authority_and_freshness",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.163]",
        "Dev Command Center",
        "Task Board",
        "freshness",
        "Spec Session Log"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§2.6.8.8",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.163] Dev Command Center is the canonical planning-and-coordination projection surface for Task Board entries, freshness state, and Spec Session Log continuity backed by authoritative backend artifacts rather than kanban-only ordering. [ADD v02.166] Dev Command Center SHOULD render structured Task Board rows first and surface Markdown mirror drift or regeneration state explicitly. [ADD v02.167] Dev Command Center MAY offer kanban, queue, roadmap, or Jira-like board layouts over those rows, but it MUST surface the authoritative fields and view configuration that produce each layout. [ADD v02.168] Task Board projections also carry the shared base envelope and compact summary fields so lane grouping can degrade gracefully when a project-profile extension is unknown. [ADD v02.169] Dev Command Center also surfaces Task Board mirror authority mode, reconciliation action, and last reconciliation time so a readable board never outranks canonical row state. [ADD v02.170] Dev Command Center also surfaces stable view_id, lane_id, and action-binding identifiers so drag-to-lane behavior and preset changes remain inspectable before state changes. [ADD v02.171] Task Board planning views also expose base workflow-state families and queue-reason codes so project-profile lane labels can change without rewriting routing semantics. [ADD v02.172] Task Board planning views also show which transition rules, queue automation rules, and executor eligibility policies explain a lane move, automatic regroup, or actor-ineligible card state. [ADD v02.181] Task Board planning projections also surface validator-gate, claim/lease posture, queued follow-up state, checkpoint-backed recovery posture, and derived closeout posture from authoritative runtime records, and MAY NOT derive validation, ownership, recovery, or completion solely from lane placement, imported board state, or mailbox commentary."
    },
    {
      "edge_id": "IMX-109",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "control_surface_projects_work_packet_contracts_and_activation",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.163]",
        "Dev Command Center",
        "Work Packet",
        "workflow activation",
        "parallel planning"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§7.2",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.163] Dev Command Center is the canonical planning-and-coordination projection surface for Work Packet bindings, workflow-linked activation, and session-aware work routing backed by authoritative backend artifacts instead of packet-local prose. [ADD v02.166] Work Packet detail views also render typed structured fields, note-stream handles, and handoff completeness before any Markdown mirror or packet-local narrative. [ADD v02.167] Work Packet detail surfaces SHOULD default to canonical record plus compact summary views and treat Markdown mirrors as readable derivatives with explicit synchronization state. [ADD v02.168] Work Packet views also distinguish shared base-envelope fields from project-profile extensions so future non-software kernels can reuse the same core record contract. [ADD v02.169] Work Packet detail views also expose mirror contracts, reconciliation actions, and normalization-required posture before the operator regenerates readable packet narratives. [ADD v02.170] Work Packet detail and layout views also expose governed next-action bindings so moves, review requests, escalation, and promotion actions stay explicit. [ADD v02.171] Work Packet detail and queue views also expose base workflow-state families, queue reasons, and allowed action ids so review, approval, and completion posture stay portable across project kernels. [ADD v02.172] Work Packet detail and queue views also expose which transition rules, automation rules, and executor eligibility policies gate activation, retry, review, approval, validation, and completion posture. [ADD v02.181] Work Packet detail projections also surface governed-action resolution, validator-gate, claim/lease posture, queued steering/follow-up state, checkpoint-backed recovery posture, and derived closeout posture from runtime records so packet prose or repo-governance mirrors cannot silently outrank workflow-backed state."
    },
    {
      "edge_id": "IMX-110",
      "from_kind": "feature",
      "from_id": "FEAT-TASK-BOARD",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "human_readable_planning_mirror_syncs_from_authoritative_work_tracking",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.163]",
        "Task Board",
        "Locus Work Tracking",
        "synchronized mirror"
      ],
      "spec_refs": [
        "Â§2.6.8.8",
        "#2315-locus-work-tracking-system-add-v02116",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-004"
      ],
      "notes": "[ADD v02.163] Task Board is the human-readable synchronization mirror for Locus work-tracking state, ready-work results, and tracked Work Packet status; it MUST remain readable without becoming a second execution authority. [ADD v02.166] Structured Task Board rows remain authoritative for field values and freshness semantics, while Markdown mirrors remain readable projections that MAY be regenerated. [ADD v02.167] Derived board layouts remain projections over structured Task Board and Work Packet records, and mirror or imported board drift MUST stay explicit. [ADD v02.168] Task Board rows also inherit the same base structured-collaboration envelope and compact summary joins as Work Packets, so layout logic does not need a board-only schema fork. [ADD v02.169] Task Board reconciliation also records authority mode and reconciliation action so manual board edits outside advisory zones cannot silently backdoor canonical state. [ADD v02.170] Lane definitions, preset ids, and action bindings now keep drag-to-lane and regroup behavior explicit instead of relying on hidden board heuristics. [ADD v02.171] Task Board synchronization also preserves base workflow-state families and queue reasons from Locus so board-only labels never become the routing authority. [ADD v02.172] Task Board synchronization also preserves transition rule ids and queue automation posture from Locus so a board refresh cannot invent a lawful move on its own. [ADD v02.181] Task Board synchronization also preserves validator-gate, claim/lease posture, queued follow-up state, checkpoint-backed recovery posture, and derived closeout posture from authoritative runtime state so readable board summaries cannot silently overrule workflow-backed validation, ownership, recovery, or completion."
    },
    {
      "edge_id": "IMX-111",
      "from_kind": "feature",
      "from_id": "FEAT-WORK-PACKET-SYSTEM",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "work_packet_activation_routes_through_workflow_execution",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.163]",
        "Work Packet System",
        "Workflow Engine",
        "activation"
      ],
      "spec_refs": [
        "Â§7.2",
        "#26-workflow-automation-engine",
        "6.0.2.11"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.163] Work Packet activation, lifecycle projection, and session-aware execution routing MUST remain workflow-backed operations so planning surfaces never infer authoritative activation from packet-local status alone."
    },
    {
      "edge_id": "IMX-112",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "kind": "control_surface_projects_session_recovery_and_provider_readiness",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.164]",
        "Dev Command Center",
        "SessionCheckpoint",
        "ProviderCapabilities",
        "heartbeat freshness"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§4.3.9.16",
        "Â§4.3.9.18",
        "Â§4.3.9.19"
      ],
      "runtime_visibility_ids": [
        "RV-018",
        "RV-020"
      ],
      "notes": "[ADD v02.164] Dev Command Center is the canonical recovery and readiness projection surface for session checkpoints, heartbeat freshness, provider capability coverage, span-linked budget state, and governed resume-or-cancel actions backed by session-registry and workflow artifacts."
    },
    {
      "edge_id": "IMX-113",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "control_surface_projects_repository_engine_policy_and_required_status_checks",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.164]",
        "Dev Command Center",
        "engine.version",
        "required status checks",
        "merge queue"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§6.0.2",
        "Â§6.3.10.1"
      ],
      "runtime_visibility_ids": [
        "RV-003",
        "RV-018"
      ],
      "notes": "[ADD v02.164] Dev Command Center is the canonical repository-decision projection surface for the declared version-control backend, backend version, required status checks, merge-queue compatibility, and explicit no-silent-fallback posture backed by governed tool metadata rather than command-output heuristics. [ADD v02.165] Promotion-gate snapshots also expose required review state, unresolved conversations, required-check provenance, and last verification time before protected-branch actions proceed."
    },
    {
      "edge_id": "IMX-114",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "control_surface_projects_run_history_and_replay",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.165]",
        "Dev Command Center",
        "run history",
        "replay",
        "workflow node execution"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§4.3.9.19",
        "Â§10.11.5.16"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.165] Dev Command Center is the canonical run-history and replay projection surface for workflow runs, workflow node executions, queue-state transitions, retries, and operator reroute decisions backed by authoritative workflow artifacts."
    },
    {
      "edge_id": "IMX-115",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-UNIFIED-TOOL-SURFACE",
      "kind": "control_surface_projects_tool_infrastructure_health",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.165]",
        "Dev Command Center",
        "tool infrastructure",
        "health state",
        "route policy"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Â§6.0.2",
        "Â§10.11.5.16"
      ],
      "runtime_visibility_ids": [
        "RV-003",
        "RV-018"
      ],
      "notes": "[ADD v02.165] Dev Command Center is the canonical tool-infrastructure projection surface for transport kind, health state, permission scope, route policy, fallback policy, and last verification status backed by Tool Registry, Tool Gate, and health-check evidence."
    },
    {
      "edge_id": "IMX-116",
      "from_kind": "feature",
      "from_id": "FEAT-DEV-COMMAND-CENTER",
      "to_kind": "feature",
      "to_id": "FEAT-ROLE-MAILBOX",
      "kind": "control_surface_projects_structured_collaboration_inbox",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.166]",
        "Dev Command Center",
        "Role Mailbox",
        "expected response",
        "handoff completeness"
      ],
      "spec_refs": [
        "#1011-dev-command-center-sidecar-integration",
        "Ã‚Â§2.6.8.10",
        "Ã‚Â§10.11.5.17"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.166] Dev Command Center is the canonical triage projection for structured Role Mailbox messages, expected responses, expiry, evidence references, and handoff completeness backed by Role Mailbox artifacts. [ADD v02.167] Role Mailbox triage SHOULD read from structured thread indexes and append-only thread records first, with Markdown note artifacts linked as secondary drilldown. [ADD v02.168] Mailbox triage also surfaces shared base-envelope fields and profile-extension boundaries so generic collaboration viewers remain reusable beyond software-delivery projects. [ADD v02.169] Mailbox triage also surfaces mirror authority mode and reconciliation action when a readable summary or Markdown sidecar exists, so operators know whether a message view is derived or canonical. [ADD v02.170] Mailbox triage presets now also expose reply, escalate, and acknowledge action bindings so queue actions stay explicit. [ADD v02.171] Mailbox triage now also shows when expected-response or escalation posture contributes to linked queue reasons, without replacing the linked record's authoritative workflow-state family. [ADD v02.172] Mailbox triage now also shows which mailbox events can fire automation rules, which transitions still require approval, and which actor kinds remain eligible to continue the linked work. [ADD v02.173] Mailbox triage now also shows thread lifecycle state, message delivery state, allowed responses, mailbox-local-versus-governed quick actions, and dead-letter posture before an operator or model loop touches linked work. [ADD v02.176] Mailbox triage now also shows executor kind allowlists, current claimant, claim mode, lease expiry, takeover legality, and response-authority scope before an operator, reviewer, validator, or model loop touches linked work. [ADD v02.181] Mailbox triage also shows linked validator-gate, governed-action, claim/lease, queued follow-up, recovery, and closeout context for software-delivery threads, but reply chronology and summary text remain non-authoritative compared with runtime workflow and gate state."
    },
    {
      "edge_id": "IMX-117",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "structured_collaboration_messages_reference_contract_artifacts",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.166]",
        "Role Mailbox",
        "Work Packet",
        "handoff",
        "evidence references"
      ],
      "spec_refs": [
        "Ã‚Â§2.6.8.10",
        "Ã‚Â§7.2",
        "Ã‚Â§10.11.5.17"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.166] Role Mailbox delegate-work, blocker, review, decision, and handoff messages carry stable work-packet and micro-task identifiers, expected-response metadata, and evidence references so collaboration remains queryable without becoming authoritative state. [ADD v02.167] Structured mailbox records SHOULD reference canonical Work Packet and Micro-Task record identifiers plus any Markdown note artifacts, so collaboration remains portable across future project kernels. [ADD v02.168] Role Mailbox export records and Work Packet or Micro-Task records also share the same base structured-collaboration envelope and project-profile extension boundary. [ADD v02.169] Mailbox-related readable summaries also share mirror contracts so advisory human narrative is normalized deliberately instead of leaking into canonical packet state. [ADD v02.170] Mailbox expected-response state also feeds local-small-model execution queues and inbox presets through explicit linked ids and action bindings rather than implicit thread ordering. [ADD v02.171] Mailbox messages now contribute explicit queue reasons such as `mailbox_response_wait` or `escalation_wait` to linked Work Packet records only through stable ids and governed actions, not by thread order alone. [ADD v02.172] Mailbox-triggered queue moves now resolve only through explicit automation rules and transition rules tied to stable ids rather than implicit reply order. [ADD v02.173] Work Packet-facing mailbox threads now also expose thread lifecycle state, delivery state, and allowed responses so handoff, review, and announce-back traffic can remain structured without letting a reply alone mutate packet authority. [ADD v02.176] Work Packet-facing mailbox threads now also expose claimant identity, claim mode, lease expiry, and response-authority scope so handoff, takeover, and reroute traffic can remain structured without letting temporary mailbox ownership outrank packet authority."
    },
    {
      "edge_id": "IMX-118",
      "from_kind": "feature",
      "from_id": "FEAT-TASK-BOARD",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "projected_board_views_render_from_structured_work_packet_contracts",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.167]",
        "Task Board",
        "Work Packet",
        "kanban",
        "queue"
      ],
      "spec_refs": [
        "Â§7.2",
        "Â§2.6.8.8",
        "Â§10.11.5.18"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.167] Future kanban, queue, roadmap, and Jira-like Task Board layouts derive lane, grouping, and sort state from structured Work Packet and Task Board records; rearranging a projection does not change authoritative contract state until a governed structured-record edit is executed. [ADD v02.170] Preset ids, lane definitions, and drag-to-lane action bindings now keep projection behavior explicit before any authoritative mutation occurs. [ADD v02.171] Task Board projections now render base workflow-state families and queue-reason codes from Work Packet records before any project-profile lane alias is applied. [ADD v02.172] Task Board projections now also render transition and executor-eligibility previews from Work Packet records before any lane alias or drag gesture is treated as a legal move."
    },
    {
      "edge_id": "IMX-119",
      "from_kind": "feature",
      "from_id": "FEAT-LOCUS-WORK-TRACKING",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "shared_base_envelope_normalizes_packet_and_tracking_records",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.168]",
        "Locus Work Tracking",
        "Work Packet",
        "base envelope",
        "project profile"
      ],
      "spec_refs": [
        "Â§2.3.15.2",
        "Â§2.3.15.5",
        "Â§7.2"
      ],
      "runtime_visibility_ids": [
        "RV-004",
        "RV-018"
      ],
      "notes": "[ADD v02.168] Locus tracked records and Work Packet contract views share the same base structured-collaboration envelope, compact summary contract, mirror-state semantics, and project-profile extension boundary so tracking, routing, and future project kernels do not fork the core packet schema. [ADD v02.169] Those records now also share mirror authority mode and reconciliation action so readable packet mirrors and task-board projections stay controller-driven instead of prose-driven."
    },
    {
      "edge_id": "IMX-120",
      "from_kind": "feature",
      "from_id": "FEAT-WORK-PACKET-SYSTEM",
      "to_kind": "feature",
      "to_id": "FEAT-TASK-BOARD",
      "kind": "work_packet_mirror_contract_governs_task_board_projection_reconciliation",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.169]",
        "Work Packet",
        "Task Board",
        "mirror contract",
        "reconciliation"
      ],
      "spec_refs": [
        "Â§2.3.15.5",
        "Â§2.6.8.8",
        "Â§7.2",
        "Â§10.11.5.19"
      ],
      "runtime_visibility_ids": [
        "RV-004",
        "RV-018"
      ],
      "notes": "[ADD v02.169] Work Packet canonical records and Task Board readable projections share mirror authority mode, reconciliation action, and normalization rules so regenerating a board never silently overwrites advisory operator edits or packet-linked note sidecars."
    },
    {
      "edge_id": "IMX-121",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-MICRO-TASK-EXECUTOR",
      "kind": "mailbox_response_state_blocks_or_unblocks_execution_queue_readiness",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.170]",
        "Role Mailbox",
        "Micro-Task Executor",
        "expected response",
        "execution queue"
      ],
      "spec_refs": [
        "Ã‚Â§2.6.8.10",
        "#2668-micro-task-executor-profile",
        "Ã‚Â§10.11.5.20"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.170] Role Mailbox expected-response and escalation posture can place Micro-Tasks into awaiting-mailbox-response or ready-for-local-small-model execution buckets through explicit linked identifiers and governed action bindings, without making mailbox thread order authoritative over task state. [ADD v02.171] Mailbox-linked waits now surface as explicit queue-reason codes on Micro-Tasks so execution queues can distinguish mailbox waits from generic blocked state. [ADD v02.172] Mailbox-linked waits now also carry executor-eligibility consequences so local-small-model queues can distinguish a waiting task from one that remains actor-ineligible even after a reply arrives. [ADD v02.173] Micro-Task-linked mailbox threads now also distinguish `MicroTaskRequest`, `MicroTaskFeedback`, `MicroTaskVerificationNeeded`, `MicroTaskEscalation`, and `MicroTaskCompletionReport` so verifier loops, retry hints, and bounded escalate-or-complete outcomes remain structured. [ADD v02.174] Those mailbox-linked loop records now also expose structured verifier outcomes, loop checkpoints, remaining retry budget, escalation targets, and completion-report transcription posture so execution queues can resume or audit a loop without transcript replay. [ADD v02.176] Mailbox-linked Micro-Task queues now also surface claimant identity, claim mode, lease expiry, and response-authority scope so execution readiness does not mistakenly free work that is still temporarily owned by another actor."
    },
    {
      "edge_id": "IMX-122",
      "from_kind": "feature",
      "from_id": "FEAT-LOCUS-WORK-TRACKING",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "workflow_state_registry_drives_operator_queue_projection",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.171]",
        "Locus Work Tracking",
        "Dev Command Center",
        "workflow state family",
        "queue reason"
      ],
      "spec_refs": [
        "#2315-locus-work-tracking-system-add-v02116",
        "#1011-dev-command-center-sidecar-integration",
        "10.11.5.21"
      ],
      "runtime_visibility_ids": [
        "RV-004",
        "RV-018"
      ],
      "notes": "[ADD v02.171] Locus publishes the base workflow-state family, queue-reason code, and allowed action ids that Dev Command Center uses to group queues, relabel lanes through project profiles, and expose portable next actions without treating lane position or prose as authority. [ADD v02.172] Locus also publishes the transition rules, queue automation posture, and executor eligibility policies that Dev Command Center uses to preview whether an action is automatic, approval-bound, actor-ineligible, or ready."
    },
    {
      "edge_id": "IMX-123",
      "from_kind": "feature",
      "from_id": "FEAT-LOCUS-WORK-TRACKING",
      "to_kind": "feature",
      "to_id": "FEAT-MICRO-TASK-EXECUTOR",
      "kind": "workflow_transition_registry_gates_executor_eligibility",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.172]",
        "Locus Work Tracking",
        "Micro-Task Executor",
        "transition rule",
        "executor eligibility"
      ],
      "spec_refs": [
        "#2315-locus-work-tracking-system-add-v02116",
        "#2668-micro-task-executor-profile",
        "10.11.5.22"
      ],
      "runtime_visibility_ids": [
        "RV-004",
        "RV-022"
      ],
      "notes": "[ADD v02.172] Locus publishes the transition rules, queue automation posture, and executor eligibility policies that Micro-Task Executor uses to decide whether local small models may start, retry, wait, escalate, or stop at a review or approval boundary."
    },
    {
      "edge_id": "IMX-124",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-LOCUS-WORK-TRACKING",
      "kind": "mailbox_action_requests_resolve_through_locus_authority",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.173]",
        "Role Mailbox",
        "Locus Work Tracking",
        "allowed response",
        "authority boundary"
      ],
      "spec_refs": [
        "Â§2.3.15",
        "Â§2.6.8.10",
        "Â§10.11.5.23"
      ],
      "runtime_visibility_ids": [
        "RV-004",
        "RV-018"
      ],
      "notes": "[ADD v02.173] Role Mailbox action requests, thread lifecycle changes, and dead-letter posture may recommend or prepare linked work changes, but authoritative Work Packet and Micro-Task state still resolves through Locus-backed governed actions or explicit transcription rather than mailbox chronology alone. [ADD v02.174] Locus also joins mailbox-linked loop checkpoints, structured verifier outcomes, remaining retry budget, and completion-report transcription posture so a Micro-Task retry, escalation, or completion can be explained from authoritative linked state rather than the latest thread message alone. [ADD v02.176] Locus also joins mailbox claimant identity, claim mode, lease expiry, takeover policy, and response-authority scope so a reroute, takeover, or stale-lease warning can be explained from authoritative linked state rather than whichever actor most recently looked at the thread. [ADD v02.177] Locus also joins accepted handoff bundle refs, announce-back provenance kind, and transcription status so resume, handback, and completion posture can be explained from authoritative linked state rather than mailbox summary text alone. [ADD v02.181] Validator-gate, review, approval, claim/lease, queued follow-up, recovery, and closeout changes also resolve through runtime workflow and Locus-backed product records rather than mailbox text or imported repo-governance artifacts."
    },
    {
      "edge_id": "IMX-125",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "micro_task_loop_reports_feed_packet_handoff_and_remaining_work_posture",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.174]",
        "Role Mailbox",
        "Work Packet System",
        "completion report",
        "loop checkpoint"
      ],
      "spec_refs": [
        "§2.6.8.10",
        "§7.2",
        "10.11.5.24"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.174] Role Mailbox Micro-Task feedback, escalation, and completion-report threads may attach loop checkpoints, structured verifier outcomes, remaining retry budget, and transcription targets that Work Packet note streams and handoff views consume so remaining work, waiting posture, and announce-back evidence stay queryable without replaying the full thread. [ADD v02.176] Work Packet follow-up views also consume mailbox claimant identity, claim mode, lease expiry, and last handback reason so packet ownership ambiguity and takeover risk stay visible without replaying the full thread. [ADD v02.177] Role Mailbox handoff, announce-back, and completion-report threads may also attach structured handoff bundles, announce-back provenance, and transcription status that Work Packet note streams and handoff views consume so remaining work and recommended next actor stay queryable without replaying the full thread."
    },
    {
      "edge_id": "IMX-126",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-TASK-BOARD",
      "kind": "mailbox_triage_and_dead_letter_posture_feeds_board_pressure_projection",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.175]",
        "Role Mailbox",
        "Task Board",
        "snooze",
        "dead-letter remediation"
      ],
      "spec_refs": [
        "§2.6.8.10",
        "§2.6.8.8",
        "10.11.5.25"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.175] Role Mailbox triage queue state, queue age, snooze or expiry posture, and dead-letter disposition may contribute mailbox pressure overlays and waiting reasons to Task Board projections, but Task Board remains a derived planning view and MUST NOT become the authority for thread remediation or linked work-state mutation. [ADD v02.176] Mailbox claimant identity, claim mode, lease age, lease expiry, and takeover legality may also contribute board overlays and stale-ownership warnings, but Task Board remains a derived planning view and MUST NOT become the authority for mailbox assignment or takeover state. [ADD v02.177] Mailbox handoff-ready, announce-back advisory, and transcription-pending posture may also contribute board overlays and follow-up warnings, but Task Board remains a derived planning view and MUST NOT become the authority for handoff acceptance or completion state."
    },
    {
      "edge_id": "IMX-127",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "mailbox_claim_lease_and_takeover_posture_drives_operator_reply_authority_projection",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.176]",
        "Role Mailbox",
        "Dev Command Center",
        "claim lease",
        "takeover"
      ],
      "spec_refs": [
        "§2.6.8.10",
        "#1011-dev-command-center-sidecar-integration",
        "10.11.5.26"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.176] Role Mailbox claimant identity, executor kind, claim mode, lease age, lease expiry, takeover legality, and response-authority scope drive Dev Command Center reply and takeover previews so parallel actors do not double-handle a thread or bypass human-only authority."
    },
    {
      "edge_id": "IMX-128",
      "from_kind": "feature",
      "from_id": "FEAT-ROLE-MAILBOX",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "mailbox_handoff_bundle_and_announce_back_provenance_drive_operator_handoff_and_completion_projection",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.177]",
        "Role Mailbox",
        "Dev Command Center",
        "handoff bundle",
        "announce back"
      ],
      "spec_refs": [
        "§2.6.8.10",
        "#1011-dev-command-center-sidecar-integration",
        "10.11.5.27"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.177] Role Mailbox handoff bundle id, announce-back provenance kind, remaining-work summary, recommended next actor, and transcription status drive Dev Command Center handoff review and done-badge safety so operators can resume or close work without confusing advisory mailbox summaries for authoritative completion."
    },
    {
      "edge_id": "IMX-129",
      "from_kind": "feature",
      "from_id": "FEAT-LOOM-LIBRARY",
      "to_kind": "feature",
      "to_id": "FEAT-PROJECT-BRAIN",
      "kind": "loom_graph_biases_project_brain_retrieval_without_replacing_direct_lookup",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.178]",
        "Loom",
        "Project Brain",
        "graph bias",
        "direct load"
      ],
      "spec_refs": [
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "#258-project-brain-rag-interface",
        "Â§2.6.6.7.14"
      ],
      "runtime_visibility_ids": [
        "RV-011",
        "RV-018"
      ],
      "notes": "[ADD v02.178] Loom tags, mentions, backlinks, pins, and unlinked state may bias Project Brain graph expansion, reranking, and related-context discovery, but known LoomBlock or asset identifiers MUST resolve by direct load or bounded graph traversal before hybrid retrieval is attempted."
    },
    {
      "edge_id": "IMX-130",
      "from_kind": "feature",
      "from_id": "FEAT-LOOM-LIBRARY",
      "to_kind": "feature",
      "to_id": "FEAT-SPEC-ROUTER",
      "kind": "loom_context_supports_background_discovery_without_becoming_router_authority",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.178]",
        "Loom",
        "Prompt-to-Spec Router",
        "background discovery",
        "authority boundary"
      ],
      "spec_refs": [
        "#1012-loom-heaper-style-library-surface-add-v02130",
        "Â§2.6.8.5",
        "Â§2.6.6.7.14"
      ],
      "runtime_visibility_ids": [
        "RV-011",
        "RV-018"
      ],
      "notes": "[ADD v02.178] Loom context may support Prompt-to-Spec background discovery for ambiguous prompts, but it cannot outrank policy, capability, Work Packet, or exact artifact state and MUST remain visible as a retrieval bias rather than router authority."
    },
    {
      "edge_id": "IMX-131",
      "from_kind": "feature",
      "from_id": "FEAT-SPEC-ROUTER",
      "to_kind": "feature",
      "to_id": "FEAT-WORK-PACKET-SYSTEM",
      "kind": "router_prefers_authoritative_packet_and_task_state_before_hybrid_retrieval",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.178]",
        "Prompt-to-Spec Router",
        "Work Packet System",
        "direct load",
        "authoritative state"
      ],
      "spec_refs": [
        "Â§2.6.8.5",
        "Â§7.2",
        "Â§2.6.6.7.14"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.178] Prompt-to-Spec Router MUST prefer authoritative Work Packet, Task Board, capability, policy, and exact anchor loads before hybrid retrieval so prompt routing remains grounded in live execution state instead of related-context guesses."
    },
    {
      "edge_id": "IMX-132",
      "from_kind": "feature",
      "from_id": "FEAT-WORK-PACKET-SYSTEM",
      "to_kind": "feature",
      "to_id": "FEAT-MICRO-TASK-EXECUTOR",
      "kind": "packet_direct_load_precedes_hybrid_retrieval_for_micro_task_context",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.178]",
        "Work Packet System",
        "Micro-Task Executor",
        "direct load",
        "bounded context"
      ],
      "spec_refs": [
        "Â§7.2",
        "Â§2.6.6.8",
        "Â§2.6.6.7.14"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.178] When a Micro-Task is bound to a known Work Packet or exact packet-linked artifact, the executor MUST direct-load that authoritative state and compact it before any hybrid retrieval is attempted; related-context search remains secondary and advisory."
    },
    {
      "edge_id": "IMX-133",
      "from_kind": "feature",
      "from_id": "FEAT-STORAGE-PORTABILITY",
      "to_kind": "feature",
      "to_id": "FEAT-WORKFLOW-ENGINE",
      "kind": "postgres_primary_storage_mode_bounds_workflow_runtime_authority",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.182]",
        "postgres_primary",
        "Workflow Engine",
        "fail closed"
      ],
      "spec_refs": [
        "2.3.13.8",
        "#26-workflow-automation-engine"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.182] PostgreSQL-primary storage mode is the authority boundary for workflow-backed control-plane records; if PostgreSQL is required and unavailable, workflow/control-plane writes fail closed instead of falling back to SQLite."
    },
    {
      "edge_id": "IMX-134",
      "from_kind": "feature",
      "from_id": "FEAT-STORAGE-PORTABILITY",
      "to_kind": "feature",
      "to_id": "FEAT-MODEL-SESSION-ORCHESTRATION",
      "kind": "postgres_primary_storage_mode_bounds_model_session_queue_authority",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.182]",
        "postgres_primary",
        "ModelSession",
        "queue state"
      ],
      "spec_refs": [
        "2.3.13.8",
        "4.3.9.12"
      ],
      "runtime_visibility_ids": [
        "RV-020"
      ],
      "notes": "[ADD v02.182] ModelSession scheduling, queue state, persisted messages, checkpoints, cancellation, and provider-profile persistence resolve through PostgreSQL-primary authority as those downstream slices land."
    },
    {
      "edge_id": "IMX-135",
      "from_kind": "feature",
      "from_id": "FEAT-STORAGE-PORTABILITY",
      "to_kind": "feature",
      "to_id": "FEAT-DEV-COMMAND-CENTER",
      "kind": "postgres_primary_authority_projects_into_dcc_with_source_freshness",
      "scope": "backend_runtime",
      "tokens": [
        "[ADD v02.182]",
        "PostgreSQL",
        "Dev Command Center",
        "source/freshness"
      ],
      "spec_refs": [
        "2.3.13.8",
        "#1011-dev-command-center-sidecar-integration"
      ],
      "runtime_visibility_ids": [
        "RV-018"
      ],
      "notes": "[ADD v02.182] Dev Command Center projections over sessions, queues, leases, workflows, memory jobs, and dead-letter state must expose PostgreSQL authority plus source/freshness metadata instead of treating SQLite mirrors or UI state as authority."
    }
  ]
}
```
<!-- HS_APPENDIX:END id=HS-APPX-INTERACTION-MATRIX -->
