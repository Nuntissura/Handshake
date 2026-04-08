## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by the current ORCHESTRATOR_PROTOCOL refinement workflow.

### METADATA
- WP_ID: WP-1-Product-Governance-Check-Runner-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-07T17:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja070420262230
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Product-Governance-Check-Runner-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Product Governance Artifact Registry (VALIDATED) stores typed governance artifacts but no execution layer exists
- No check-specific execution descriptor bridging GovernanceArtifactRegistryEntry kind=Checks/Rubrics to bounded gated invocation
- GOV_KERNEL spec section 8 lists auxiliary governance checks as kernel concepts but not as product-runtime-executable surfaces
- No typed check result contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED) in product code
- No tool_id registered under Unified Tool Surface Contract for governance check execution
- No Flight Recorder event type covers check execution lifecycle

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Master Spec v02.179 (sections 1.3, 6.0.2, 7.5.4, 7.5.4.8, 10.11, GOV_KERNEL section 8), local product code in ../handshake_main/src/backend/handshake_core/src, OPA/Conftest, Argo CD sync hooks, Tekton, GitHub Actions
- REFERENCES: .GOV/spec/Handshake_Master_Spec_v02.180.md, .GOV/task_packets/stubs/WP-1-Product-Governance-Check-Runner-v1.md, .GOV/task_packets/WP-1-Product-Governance-Artifact-Registry-v1/packet.md, https://www.openpolicyagent.org/docs/latest/management-bundles/, https://argo-cd.readthedocs.io/en/stable/user-guide/sync-waves/, https://www.conftest.dev/, https://github.com/open-policy-agent/conftest
- PATTERNS_EXTRACTED: OPA policies execute in isolated environments producing structured JSON results. Argo CD hooks use phase-based execution with automatic failure propagation. Conftest produces typed output via multiple outputters. Existing Handshake WorkflowEngine job state machine supports complex outcomes.
- DECISIONS ADOPT/ADAPT/REJECT: adopt OPA-style structured result contract with typed results beyond boolean; adapt Argo-style phase-based lifecycle (PreCheck/Check/PostCheck); adapt existing CapabilityGate for per-check capability validation; reject raw shell execution of repo scripts; reject WASM sandbox (future concern)
- LICENSE/IP_NOTES: Source review informed architectural choices only. No third-party code or copyrighted text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Spec enrichment completed in v02.180. Section 7.5.4.9 now defines the check execution contract, typed result contract, tool surface, and FR events. No further spec changes needed.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 30
- SOURCE_LOG:
  - Source: OPA Management Bundles | Kind: OSS_DOC | Date: 2026-04-07 | Retrieved: 2026-04-07T17:00:00Z | URL: https://www.openpolicyagent.org/docs/latest/management-bundles/ | Why: governance policy execution with structured evaluation output and bundle-level execution boundary
  - Source: Conftest documentation | Kind: OSS_DOC | Date: 2026-04-07 | Retrieved: 2026-04-07T17:05:00Z | URL: https://www.conftest.dev/ | Why: structured output with multiple outputters (JSON, TAP, JUnit) demonstrating typed check result patterns
  - Source: Argo CD Sync Waves and Hooks | Kind: OSS_DOC | Date: 2026-04-07 | Retrieved: 2026-04-07T17:10:00Z | URL: https://argo-cd.readthedocs.io/en/stable/user-guide/sync-waves/ | Why: phase-based execution with declarative hooks as first-class resources with metadata and automatic failure propagation
  - Source: open-policy-agent/conftest | Kind: GITHUB | Date: 2026-04-07 | Retrieved: 2026-04-07T17:15:00Z | URL: https://github.com/open-policy-agent/conftest | Why: practical implementation of policy-as-code execution with structured results
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Kind: PAPER | Date: 2024-02-01 | Retrieved: 2026-04-07T17:20:00Z | URL: https://arxiv.org/abs/2307.10034 | Why: warns that overly dynamic schema semantics become hard to reason about and validate, which argues for explicit low-cardinality result type enumeration over free-form result evolution in check execution contracts
  - Source: Google Cloud Policy Intelligence | Kind: BIG_TECH | Date: 2026-04-07 | Retrieved: 2026-04-07T17:25:00Z | URL: https://cloud.google.com/policy-intelligence/docs/overview | Why: demonstrates enterprise-grade policy evaluation with structured compliance results and audit evidence, directly analogous to the CheckResult evidence payload and audit trail requirements
- RESEARCH_SYNTHESIS:
  - Governance check runners benefit from structured result contracts beyond boolean pass/fail
  - Phase-based observable lifecycle (PreCheck/Check/PostCheck) enables bounded execution with early failure
  - Descriptor-driven execution where check definition is data not code path ensures determinism
  - Capability gating at invocation time prevents privilege escalation through imported checks
- RESEARCH_GAPS_TO_TRACK:
  - Check descriptor body storage and retrieval format is downstream of this registry and governance pack WPs
  - WASM sandbox execution for untrusted check bodies is a future concern not addressed here
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: OPA Management Bundles | Pattern: structured JSON evaluation output with metadata fields beyond true/false | Why: directly shapes the CheckResult enum with severity, reason, remediation, and evidence payloads
  - Source: Argo CD Sync Waves and Hooks | Pattern: phase-based execution (PreSync/Sync/PostSync/SyncFail) with automatic failure propagation | Why: shapes the PreCheck/Check/PostCheck lifecycle with bounded execution and observable transitions
- ADAPT_PATTERNS:
  - Source: Conftest documentation | Pattern: multiple outputters (JSON, TAP, JUnit) for structured policy results | Why: the outputter pattern informs how check results can be surfaced through different consumers (DCC, FR, API) but Handshake uses a single canonical result type rather than pluggable formatters
  - Source: open-policy-agent/conftest | Pattern: policy-as-code with file-based configuration | Why: the configuration-driven check selection pattern informs how checks are selected from the registry but Handshake uses capability-gated selection rather than file-based config
- REJECT_PATTERNS:
  - Source: OPA Management Bundles | Pattern: live bundle discovery and hot-reload of policy definitions | Why: live check discovery would make the execution surface non-deterministic and violate Handshake mechanical-layer predictability
  - Source: Argo CD Sync Waves and Hooks | Pattern: automatic retry and wave-based ordering of hook execution | Why: automatic retry without operator approval violates fail-closed governance principle
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - site:github.com/open-policy-agent/conftest structured output JSON result
  - site:github.com/open-policy-agent/opa bundle evaluation structured
  - site:github.com/argoproj/argo-cd sync hooks phase lifecycle
- MATCHED_PROJECTS:
  - Source: open-policy-agent/conftest | Repo: open-policy-agent/conftest | URL: https://github.com/open-policy-agent/conftest | Intent: ARCH_PATTERN | Decision: ADOPT | Impact: NONE | Stub: NONE | Notes: structured policy evaluation output pattern directly shapes CheckResult contract design
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Three new Flight Recorder event IDs required:
  - FR-EVT-GOV-CHECK-001: governance.check.started (check_id, session_id, check_descriptor_hash)
  - FR-EVT-GOV-CHECK-002: governance.check.completed (check_id, session_id, result_status, duration_ms, evidence_artifact_id)
  - FR-EVT-GOV-CHECK-003: governance.check.blocked (check_id, session_id, blocked_reason)

### RED_TEAM_ADVISORY (security failure modes)
- Risk: runner executes arbitrary repo scripts as subprocesses, recreating repo-governance coupling. Mitigation: checks execute through Tool Gate only; raw shell bypass returns UNSUPPORTED.
- Risk: imported rubrics encode project-specific assumptions that should not be universal product law. Mitigation: checks scoped to SoftwareDelivery profile extension, not base product governance.
- Risk: check with unlimited timeout hangs product runtime. Mitigation: check descriptor includes timeout_ms; runner enforces bounded execution.
- Risk: unsupported checks silently skipped creates false governance assurance. Mitigation: UNSUPPORTED is explicit, logged result -- fail-closed, never silent skip.
- Risk: check result tampering if stored without integrity verification. Mitigation: evidence artifacts use content hash; FR events create immutable audit trail.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-GovernancePackExport
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-Database
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-GovernancePackExport
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-Database
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - New Rust types (CheckDescriptor, CheckResult, CheckRunner) are code-level implementations of the existing governance pack spec concept and extend the governance artifact registry. They do not require new spec-level primitive entries until a later enrichment pass.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already names governance pack and structured collaboration primitives. Check runner primitives are implementations of the existing governance pack execution concept.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - New primitives follow the existing governance artifact registry pattern and do not require a separate appendix category.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family discovered. Check runner is a concrete implementation of existing governance pack execution spec anchor.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Feature registry entry for governance check runner was added as part of spec enrichment v02.180 (section 7.5.4.9). No further appendix action needed.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: New interaction edges (GovernanceArtifactRegistry x CheckRunner, CheckRunner x FlightRecorder, CheckRunner x ToolGate) are identified in DISCOVERY_MATRIX_EDGES but IMX-### IDs will be assigned during the spec enrichment pass (v02.180) that adds section 7.5.4.9. Appendix 12.6 update deferred to that spec bump.
- APPENDIX_MAINTENANCE_NOTES:
  - Spec enrichment completed in v02.180. Section 7.5.4.9 added. IMX edge IDs will be assigned in a future appendix pass.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by governance check runner work | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement logic is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes check results downstream but is not affected by the runner itself | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication or export controllers remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: check result evidence provenance is relevant to archival but the archivist engine is not directly modified | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the check runner work | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics surfaces consume stored check results later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: the runner uses the storage trait boundary but does not modify database abstraction behavior | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: check runner is the execution arm of the sovereign engine governance authority; imported checks extend governance surface | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: check results feed into context for downstream model sessions and governance decisions | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: check descriptors carry version provenance from governance artifact registry | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: check execution enforces bounded execution (timeout, capability gate) which is sandbox-adjacent; future WASM sandbox is a separate concern | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: new FR-EVT-GOV-CHECK-001..003 events for check execution lifecycle | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: check execution timestamps are plain UTC, not calendar events | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: check result evidence stored as governance artifacts through structured collaboration | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: governance check results are not Loom recordings; no collision | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: check runner does not modify the work packet feature contract directly | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board-specific feature contract is modified | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no micro-task feature contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC will consume check results for inspection and debug; UI controls identified in UI_UX_RUBRIC | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: check execution uses the job model (bounded, observable) | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router surface is altered | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: check descriptor and result storage use Database trait boundary | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: check results are structured JSON with typed enums and schema IDs | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage workflow surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is touched | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of check result data | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: check execution lifecycle events | SUBFEATURES: FR-EVT-GOV-CHECK-001..003 event emission per check run | PRIMITIVES_FEATURES: PRIM-GovernancePackExport | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: every check execution emits started, completed, and blocked events
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: bounded check execution through Tool Gate | SUBFEATURES: CheckDescriptor validation, capability-gated invocation, timeout enforcement, typed result capture | PRIMITIVES_FEATURES: PRIM-GovernancePackExport | MECHANICAL: engine.sovereign, engine.sandbox | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: uses existing WorkflowEngine job model with check-specific PreCheck/Check/PostCheck lifecycle
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: check result structured data | SUBFEATURES: CheckResult enum with detail payloads as structured JSON | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: follows canonical structured collaboration mandate
  - PILLAR: Locus | CAPABILITY_SLICE: check result evidence storage as governance artifacts in structured collaboration | SUBFEATURES: evidence artifact records stored with content hash, governance artifact registry lookup during descriptor resolution | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-GovernancePackExport | MECHANICAL: engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: check result evidence artifacts are stored as structured collaboration records through the existing Locus record family pattern
  - PILLAR: Command Center | CAPABILITY_SLICE: check result inspection and run trigger UI surface | SUBFEATURES: check status badge, check run trigger, check result detail expander, batch run controls | PRIMITIVES_FEATURES: PRIM-GovernancePackExport | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC consumes check results from CheckRunner; UI controls are defined here as the authoritative list for downstream DCC WP
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: check descriptor and result persistence through Database trait boundary | SUBFEATURES: CheckDescriptor store operations, CheckResult evidence storage, GovernanceArtifactRegistryStore lookup for descriptor resolution | PRIMITIVES_FEATURES: PRIM-Database | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: all check runner persistence goes through the Database trait boundary for portable SQLite-now/PostgreSQL-ready posture
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: governance check execution | JobModel: WorkflowEngine bounded job | Workflow: PreCheck/Check/PostCheck | ToolSurface: governance.check.run | ModelExposure: BOTH | CommandCenter: CHECK_PANEL | FlightRecorder: FR-EVT-GOV-CHECK-001..003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: primary execution surface for imported governance checks
  - Capability: check descriptor validation | JobModel: NONE | Workflow: PreCheck phase | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: validates check descriptor schema, capabilities, and timeout before execution
  - Capability: check result evidence storage | JobModel: NONE | Workflow: PostCheck phase | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: CHECK_RESULT_DETAIL | FlightRecorder: FR-EVT-GOV-CHECK-002 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: evidence artifacts stored with content hash integrity
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - Check runner creates a new execution surface bridging the governance artifact registry and the workflow engine.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Three conceptual interaction edges identified (GovernanceArtifactRegistry x CheckRunner, CheckRunner x FlightRecorder, CheckRunner x ToolGate) but formal IMX-### IDs deferred to spec enrichment v02.180 when section 7.5.4.9 and Appendix 12.6 are updated together.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_YES: Check runner benefits from external patterns for policy execution frameworks to validate cross-primitive design.
- SOURCE_SCAN:
  - Source: OPA Management Bundles | Kind: OSS_DOC | Angle: policy execution output contract | Pattern: structured JSON result with metadata beyond true/false | Decision: ADOPT | EngineeringTrick: typed result enum with severity, reason, and evidence payload | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly shapes CheckResult enum
  - Source: Argo CD Sync Waves and Hooks | Kind: OSS_DOC | Angle: phase-based bounded execution | Pattern: PreSync/Sync/PostSync with automatic failure propagation | Decision: ADAPT | EngineeringTrick: phase-based lifecycle with early exit on PreCheck failure | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: shapes PreCheck/Check/PostCheck lifecycle
  - Source: Conftest documentation | Kind: OSS_DOC | Angle: multi-format policy result output | Pattern: pluggable outputters for different consumers | Decision: ADAPT | EngineeringTrick: single canonical result type consumed by multiple surfaces (DCC, FR, API) | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: informs multi-consumer result design
- MATRIX_GROWTH_CANDIDATES:
  - Combo: GovernanceArtifactRegistry x CheckRunner execution | Sources: OPA Management Bundles, Argo CD Sync Waves and Hooks | WhatToSteal: descriptor-driven bounded execution with typed results | HandshakeCarryOver: registry provides check descriptors, runner executes through Tool Gate with capability gating | RuntimeConsequences: imported governance checks become executable through product runtime | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: primary combo for this WP
- ENGINEERING_TRICKS_CARRIED_OVER:
  - OPA: structured JSON result with metadata fields beyond true/false for rich check reporting
  - Argo CD: phase-based execution with early exit on validation failure for bounded resource consumption
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: GovernanceArtifactRegistry plus CheckRunner | Pillars: Execution / Job Runtime, Flight Recorder | Mechanical: engine.sovereign, engine.sandbox | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry provides typed descriptors, runner executes through Tool Gate
  - Combo: CheckRunner plus FlightRecorder | Pillars: Flight Recorder, LLM-friendly data | Mechanical: engine.sovereign | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: every check execution emits FR events for audit trail
  - Combo: CheckRunner plus ToolGate | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign, engine.sandbox | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: capability-gated invocation prevents privilege escalation
  - Combo: CheckRunner plus Database trait boundary | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba | Primitives/Features: PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: check descriptor and result storage use Database trait boundary for portable persistence
  - Combo: CheckRunner plus Locus evidence storage | Pillars: Locus, LLM-friendly data | Mechanical: engine.sovereign, engine.context | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: evidence artifacts from check execution stored as structured collaboration records through Locus family pattern
  - Combo: CheckRunner plus Command Center UI controls | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC check panel consumes CheckResult via check status badge, run trigger, result detail expander, and batch run controls; UI control inventory captured here for downstream DCC WP
  - Combo: CheckRunner plus engine.context feed | Pillars: LLM-friendly data, Command Center | Mechanical: engine.context | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: check results feed into context window for downstream model sessions and governance decisions through structured JSON payloads
  - Combo: CheckRunner plus engine.version provenance | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.version | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: check descriptors carry version provenance (source_snapshot_version, content_hash) from governance artifact registry making execution lineage deterministic
  - Combo: CheckDescriptor validation plus CapabilityGate | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign, engine.sandbox | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: PreCheck phase combines descriptor schema validation with capability gate check ensuring bounded and authorized execution
  - Combo: UNSUPPORTED result plus explicit reason logging | Pillars: Flight Recorder, LLM-friendly data | Mechanical: engine.sovereign | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: fail-closed UNSUPPORTED result with explicit reason prevents silent governance bypass; FR event emitted for UNSUPPORTED same as for BLOCKED
  - Combo: CheckResult evidence plus content hash integrity | Pillars: Locus, SQL to PostgreSQL shift readiness | Mechanical: engine.archivist, engine.sovereign | Primitives/Features: PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: evidence artifacts produced by check execution carry content hash that survives store/load round-trip; tamper detection without per-field scanning
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered belong inside this activation. All touched pillars (Flight Recorder, Locus, Command Center, Execution / Job Runtime, LLM-friendly data, SQL to PostgreSQL shift readiness) and engines (engine.context, engine.version, engine.sovereign, engine.sandbox) are covered.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed governance and structured collaboration packets, current Master Spec v02.179, and local product code
- MATCHED_STUBS:
  - Artifact: WP-1-Governance-Workflow-Mirror-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workflow state mirroring depends on check runner results but is a separate concern
  - Artifact: WP-1-Governance-Pack-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: governance pack bundles depend on both registry and runner but is a separate integration concern
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the artifact import/store layer that this WP executes from
  - Artifact: WP-1-Unified-Tool-Surface-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the Tool Gate this WP routes check execution through
  - Artifact: WP-1-Flight-Recorder-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the FR event infrastructure this WP emits check events to
  - Artifact: WP-1-Workflow-Engine-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the job model this WP uses for bounded check execution
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/governance_artifact_registry.rs | Artifact: WP-1-Product-Governance-Artifact-Registry-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: governance artifact registry with typed kinds including Checks and Rubrics exists; no execution surface
  - Path: ../handshake_main/src/backend/handshake_core/src/mex/gates.rs | Artifact: WP-1-Unified-Tool-Surface-Contract-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: CapabilityGate validates capabilities before operations; reusable for check execution gating
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: No duplicate exists. Check execution capability is genuinely missing. Existing registry, tool surface, FR, and workflow engine provide the foundation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements a backend execution layer and does not create a new GUI surface directly. DCC integration is downstream.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - NONE
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - NONE
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. DCC check panel is a downstream WP.
- GUI_REFERENCE_SCAN:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Patch canonical roadmap sections in place. Do not create addendum-style normative text; use `[ADD v<version>]` markers for new lines/blocks.
- Per phase, include exactly:
  - Goal:
  - MUST deliver:
  - Key risks addressed in Phase n:
  - Acceptance criteria:
  - Explicitly OUT of scope:
  - Mechanical Track:
  - Atelier Track:
  - Distillation Track:
  - Vertical slice:

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Workflow-Engine, WP-1-Flight-Recorder
- BUILD_ORDER_BLOCKS: WP-1-Governance-Workflow-Mirror, WP-1-Governance-Pack, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: 7.5.4.9 Governance Check Runner (new section)
- WHAT: Governed execution layer for imported software-delivery checks through Handshake runtime with typed result contract
- WHY: Importing governance artifacts is not enough; Handshake needs a bounded execution contract so validation happens through capability-gated, recorder-visible, product-owned workflows instead of raw shell bypass
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/Cargo.toml
- OUT_OF_SCOPE:
  - Raw shell execution of arbitrary repo scripts
  - WASM sandbox execution (future stub)
  - DCC UI for check results (downstream WP)
  - Governance Workflow Mirror execution (separate WP)
  - Non-SoftwareDelivery profile check execution
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - CheckDescriptor struct validates imported check definitions from GovernanceArtifactRegistry
  - CheckResult enum implements PASS, FAIL, BLOCKED, ADVISORY_ONLY, UNSUPPORTED with detail payloads
  - CheckRunner service executes checks through Tool Gate with capability gating
  - PreCheck/Check/PostCheck lifecycle is bounded and observable
  - FR-EVT-GOV-CHECK-001..003 events emit for every check execution
  - UNSUPPORTED checks fail closed with explicit reason
  - Evidence artifacts stored with content hash integrity
  - All storage goes through Database trait boundary
- PRIMITIVES_EXPOSED:
  - PRIM-GovernancePackExport
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/workflow_engine.rs
  - src/backend/handshake_core/src/flight_recorder.rs
  - src/backend/handshake_core/src/storage/mod.rs
- SEARCH_TERMS:
  - GovernanceArtifactRegistryEntry
  - GovernanceArtifactKind
  - CapabilityGate
  - GateDenial
  - FlightRecorderEvent
  - WorkflowEngine
- RUN_COMMANDS:
  ```bash
  rg -n "CheckDescriptor|CheckResult|CheckRunner|governance_check_runner" src/backend/handshake_core/src
  rg -n "GovernanceArtifactRegistryEntry|CapabilityGate|FlightRecorderEvent" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
  just gov-check
  ```
- RISK_MAP:
  - Raw shell execution bypass -> product safety regression (HIGH, mitigated by Tool Gate enforcement)
  - Check timeout/hang -> runtime resource exhaustion (MEDIUM, mitigated by bounded timeout)
  - Result tampering -> false governance assurance (MEDIUM, mitigated by content hash + FR audit trail)
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order still shows this WP as the active packet for the base ID and that downstream Governance-Workflow-Mirror, Governance-Pack, and DCC-Backend dependencies continue to point at the base governance-check-runner track.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Governance Pack project-specific instantiation 7.5.4.8 (check execution) | WHY_IN_SCOPE: governance artifact registry exists but no execution layer; this WP builds the execution complement | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs; src/backend/handshake_core/src/governance_artifact_registry.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | RISK_IF_MISSED: downstream Governance-Workflow-Mirror and DCC-Backend remain blocked; governance checks are never actually executed through product runtime
  - CLAUSE: Unified Tool Surface Contract tool registration 6.0.2 | WHY_IN_SCOPE: governance.check.run must be registered as a governed tool with side_effect and capability declarations | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/mex/gates.rs; src/backend/handshake_core/src/governance_check_runner.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | RISK_IF_MISSED: check execution bypasses unified tool surface and capability gating is inconsistent
  - CLAUSE: Check result typed contract (new 7.5.4.9) | WHY_IN_SCOPE: new spec section defines five-variant CheckResult enum as the canonical execution result; must be implemented | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | RISK_IF_MISSED: result contract drifts between producer and consumer; UNSUPPORTED and BLOCKED results may be silently swallowed
  - CLAUSE: Flight Recorder event emission | WHY_IN_SCOPE: spec 11.5 mandates FR events for all governed execution surfaces; check runner creates three new event types | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs; src/backend/handshake_core/src/flight_recorder.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | RISK_IF_MISSED: check execution has no audit trail; governance assurance claims cannot be verified post-execution

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: CheckDescriptor struct | PRODUCER: governance_check_runner.rs (constructed from GovernanceArtifactRegistryEntry) | CONSUMER: CheckRunner service, DCC-Backend (downstream) | SERIALIZER_TRANSPORT: in-process struct; JSON via serde for storage | VALIDATOR_READER: governance_check_runner tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | DRIFT_RISK: schema changes to GovernanceArtifactRegistryEntry can silently break CheckDescriptor construction if kind-to-descriptor mapping is not updated
  - CONTRACT: CheckResult enum | PRODUCER: governance_check_runner.rs | CONSUMER: FlightRecorder (FR events), storage layer, DCC-Backend (downstream) | SERIALIZER_TRANSPORT: JSON via serde | VALIDATOR_READER: governance_check_runner tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | DRIFT_RISK: new enum variants added without updating downstream match arms in FR event builder or DCC consumer
  - CONTRACT: governance.check.run tool surface | PRODUCER: governance_check_runner.rs (registers tool_id) | CONSUMER: ToolGate / CapabilityGate, session-scoped capability intersection | SERIALIZER_TRANSPORT: HTC-1.0 tool invocation JSON | VALIDATOR_READER: gates.rs tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | DRIFT_RISK: tool surface input schema can drift from CheckDescriptor if tool registration is not kept in sync with descriptor struct changes
  - CONTRACT: FR event payloads FR-EVT-GOV-CHECK-001..003 | PRODUCER: governance_check_runner.rs (emits on each phase) | CONSUMER: FlightRecorder append pipeline, audit consumers | SERIALIZER_TRANSPORT: FlightRecorderEvent JSON | VALIDATOR_READER: flight_recorder tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner | DRIFT_RISK: event payload fields can drift if check execution result struct changes without updating FR event builder

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- SEMANTIC_TRIPWIRES:
  - CheckResult enum must be exhaustive (all five variants) and no variant should be constructable without required detail fields
  - UNSUPPORTED result must not be bypassable -- no code path should silently skip an unsupported check
  - Tool Gate capability check must occur before any check execution starts (PreCheck phase)
  - FR events must be emitted even on BLOCKED/UNSUPPORTED results (audit completeness)
  - Additive overlay rule must be enforced -- no code path can disable native governance through imported checks
- CANONICAL_CONTRACT_EXAMPLES:
  - a CheckRunner invoked with a valid CheckDescriptor executes through Tool Gate, emits FR-EVT-GOV-CHECK-001 on start, and emits FR-EVT-GOV-CHECK-002 on completion with typed CheckResult
  - a CheckRunner invoked with an unsupported check kind returns CheckResult::Unsupported with explicit reason and emits FR-EVT-GOV-CHECK-003
  - a CheckRunner invoked without required capability returns CheckResult::Blocked and emits FR-EVT-GOV-CHECK-003 before any execution begins
  - CheckResult::Pass, Fail, Blocked, AdvisoryOnly, and Unsupported all serialize to and from JSON with correct variant tags
  - evidence artifacts produced by a passing check carry a content hash that survives a store/load round-trip

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - 1. CheckResult enum with all five variants and detail structs (types first)
  - 2. CheckDescriptor struct bridging GovernanceArtifactRegistryEntry to executable form
  - 3. CheckRunner service with PreCheck/Check/PostCheck lifecycle
  - 4. Tool Gate integration (governance.check.run tool_id registration)
  - 5. Flight Recorder event emission (FR-EVT-GOV-CHECK-001..003)
  - 6. Evidence artifact storage with content hash
  - 7. Integration tests validating end-to-end check execution
- HOT_FILES:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner
- CARRY_FORWARD_WARNINGS:
  - Do not introduce raw subprocess/shell execution for imported checks
  - Do not allow imported checks to modify native governance state
  - All storage must go through Database trait boundary (no direct SQLite calls)

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - 7.5.4.8 Governance Pack instantiation check execution
  - 6.0.2 Unified Tool Surface Contract tool registration
  - 7.5.4.9 (new) Check result typed contract
  - 11.5 Flight Recorder event completeness
- FILES_TO_READ:
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/mex/gates.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_check_runner
  - cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
- POST_MERGE_SPOTCHECKS:
  - Verify governance_check_runner.rs does not introduce raw shell execution
  - Verify CheckResult enum is exhaustive with all five variants
  - Verify FR events emit for all result types including BLOCKED and UNSUPPORTED

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - DCC UI integration for check result display (downstream WP-1-Dev-Command-Center-Control-Plane-Backend)
  - WASM sandbox for untrusted check bodies (not in scope; future concern)
  - Non-SoftwareDelivery profile check execution (not in scope)
  - The exact CheckRunner trait method signatures are not proven until coding. The lifecycle surface (run_check, validate_descriptor) is directional but may evolve during implementation.
  - Whether evidence artifact storage requires a separate store trait or shares GovernanceArtifactRegistryStore will need inspection during implementation.

### DISCOVERY
- DISCOVERY_PRIMITIVES: PRIM-CheckDescriptor (typed record for executable check definition), PRIM-CheckResult (typed result enum), PRIM-CheckRunner (execution surface)
- DISCOVERY_STUBS: NONE_CREATED | Reason: downstream stubs already exist
- DISCOVERY_MATRIX_EDGES: IMX-GOV-CHECK-001, IMX-GOV-CHECK-002, IMX-GOV-CHECK-003
- DISCOVERY_UI_CONTROLS: check status badge, check run trigger, check result detail expander, batch run controls
- DISCOVERY_SPEC_ENRICHMENT: YES

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.180 Main Body explicitly defines governance pack instantiation (7.5.4.8), governance check runner execution contract (7.5.4.9 [ADD v02.180]), unified tool surface contract (6.0.2), and flight recorder event mandates (11.5). The remaining work implements the execution complement to the existing registry using the spec-defined contracts.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec enrichment completed. Section 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) was added in v02.180. The target spec now contains the typed CheckResult contract, tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, and additive overlay rule.

#### PROPOSED_SPEC_ENRICHMENT
- <not applicable; ENRICHMENT_NEEDED=NO>

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
- CONTEXT_START_LINE: 31837
- CONTEXT_END_LINE: 31900
- CONTEXT_TOKEN: project-parameterized
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)

  **Purpose**
  Handshake MUST implement the project-agnostic Governance Kernel (7.5.4; `.GOV/GOV_KERNEL/*`) as a project-parameterized **Governance Pack** so the same strict workflow can be generated and enforced for arbitrary projects (not Handshake-specific).

  **Definitions**
  - **Governance Pack**: a versioned bundle of templates + gate semantics that instantiate:
    - project codex,
    - role protocols,
    - canonical governance artifacts and templates,
    - mechanical gate tooling (scripts/hooks/CI) and a single command surface (e.g., `just`),
    - deterministic exports (including `.GOV/ROLE_MAILBOX/` when enabled by governance mode).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)
- CONTEXT_START_LINE: 31726
- CONTEXT_END_LINE: 31835
- CONTEXT_TOKEN: Mechanical Gated Workflow (Project-Agnostic)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)

  **Purpose**
  Define a reusable, project-agnostic governance kernel that enables:
  - deterministic multi-role collaboration (Operator / Orchestrator / Coder / Validator)
  - rigorous auditability (evidence-first; append-only logs)
  - reliable handoff between small-context local models and large-context cloud models
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 6.0.2 Unified Tool Surface Contract
- CONTEXT_START_LINE: 23944
- CONTEXT_END_LINE: 24043
- CONTEXT_TOKEN: single canonical tool contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6.0.2 Unified Tool Surface Contract

  Every tool exposed to models MUST register under a single canonical tool contract so capability gating,
  side-effect classification, and audit trail emission are uniform across all execution surfaces.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 1.3 The Four-Layer Architecture
- CONTEXT_START_LINE: 479
- CONTEXT_END_LINE: 530
- CONTEXT_TOKEN: LLM steers, software executes, code validates
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 1.3 The Four-Layer Architecture

  Handshake has four layers: LLM (decides what), Orchestrator (enforces rules), Mechanical (executes deterministically), and Validation (confirms correctness).

  - **Mechanical Layer**: Deterministic engines (Word, Excel, Docling) that execute operations.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md GOV_KERNEL section 8 Auxiliary governance checks
- CONTEXT_START_LINE: 39230
- CONTEXT_END_LINE: 39250
- CONTEXT_TOKEN: Auxiliary governance checks (kernel-recommended)
- EXCERPT_ASCII_ESCAPED:
  ```text
  Auxiliary governance checks (kernel-recommended) are optional validation surfaces that extend the
  base governance kernel with project-specific assertions without replacing or overriding the kernel
  mechanical gates.
  ```
