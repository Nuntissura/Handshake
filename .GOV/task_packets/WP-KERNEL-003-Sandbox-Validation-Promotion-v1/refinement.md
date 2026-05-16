<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.refinement_contract@1 source_file=.GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/refinement.json source_hash=39b104e5a7d92cda projection_hash=395256268766b98a generated_at_utc=2026-05-15T00:58:04.573Z generator=kernel-builder-projection-repair.mjs -->
## TECHNICAL_REFINEMENT

### METADATA
- WP_ID: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-15
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-05-15T00:30:00.000Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json
- SPEC_TARGET_SHA1: 29ae893608ccb3d9ba2bd9fc84a3eca8887de295
- SPEC_TARGET_SHA256: 7286cbee9ce394dc1bb881cf4f20f26eee58a35aebc9a879c8af4a6efc2e7357
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja150520260208
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- STUB_WP_IDS: WP-KERNEL-003-Sandbox-Validation-Promotion-v1
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.refinement_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/refinement.json
- MARKDOWN_PROJECTION_STATUS: SOURCE_REFINEMENT_PENDING_PACKET_GENERATION
- RED_TEAM_REQUIRED: YES
- RED_TEAM_PROFILE: DETERMINISTIC_CONTRACT_MIGRATION_V1

### ACTIVATION_TOPOLOGY_REPAIR
- STATUS: ACTIVATION_READINESS
- IMPLEMENTATION_ROLE: KERNEL_BUILDER
- CODER_COMPATIBILITY_LANE: CODER_A
- WP_VALIDATOR_GATE: DISABLED
- COMMUNICATION_HEALTH_GATE: INTEGRATION_BATCH_REVIEW_BLOCKING
- VALIDATION_TOPOLOGY: INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1
- NOTE: Runtime `next_expected_actor=CODER` means Kernel Builder via coder-compatible tooling, not a separate Primary Coder authority.
- READINESS_STATUS: READY_FOR_PACKET_GENERATION_AFTER_SIGNATURE
- FINAL_CHECKS: packet generation, worktree preparation, Task Board activation, traceability sync, and Integration Validator handoff only; no product PASS/FAIL verdict in this activation session.

### GAPS_IDENTIFIED
  - Kernel003 needs explicit sandbox, validation, promotion, artifact bundle, blocked decisioning, and DCC/read-model contracts before product implementation starts.
  - The folded scope must activate without condensing or removing any of the 80 planned MTs.
  - The source fold includes MTE caps, blocked decisioning, summaries, drop-back, MCP/MEX evidence export, debug bundle bridge, path guardrails, validation preflight, candidate-range truth, closeout bundle, and receipt-driven lane settlement.
  - Kernel003 must reuse KB001 EventLedger/SessionBroker/ArtifactStore/ValidationRunner/PromotionGate and KB002 WriteBox/ActionCatalog/CRDT surfaces instead of creating parallel authority.
  - Kernel003 must keep product authority on Postgres/EventLedger and must not introduce SQLite authority, fallback, offline cache, compatibility path, or test fixture for Kernel V1.
  - This packet uses Kernel Builder consolidated implementation and a separate Integration Validator batch review; no WP Validator gate is allowed.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: Local spec/stub/code-surface scan on 2026-05-15.
- SEARCH_SCOPE: SPEC_CURRENT v02.185, Kernel003 stub contract/projection, Kernel001 packet, Kernel002 packet, Task Board, Build Order, traceability registry, reset brief, and existing product code path inventory.
- REFERENCES: .GOV/spec/SPEC_CURRENT.md; v02.185 modules 02, 03, 05, 10, 11; Kernel003 stub Markdown/contract; Kernel001 and Kernel002 packet contracts; reset brief sandbox section.
- PATTERNS_EXTRACTED: ADOPT EventLedger and Postgres authority; ADOPT ToolGate/ValidationRunner/PromotionGate as authority path; ADOPT default-deny sandbox policy; ADAPT DCC projections for run/status/evidence/promotion controls; REJECT raw shell passthrough, direct authority mutation, container-only activation, and SQLite-backed Kernel V1 authority.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT v02.185 as current authority, ADOPT all 80 preserved MTs, ADAPT policy-scoped local sandbox as first proof with hard-isolation adapter behind preflight, REJECT scope collapse and duplicate check runner.
- LICENSE/IP_NOTES: NONE.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: v02.185 already supplies the topical Kernel V1 authority, sandbox default-deny, validation, promotion, observability, and DCC projection law needed to create the implementation packet; Kernel003-specific schema names are packet-level contracts under that authority.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This activation does not choose a new external sandbox library; it operationalizes local Master Spec v02.185 plus the prepared Kernel003 stub. MT-004 preserves the required current research before product adapter implementation.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Local authority already selects default-deny policy, durable EventLedger evidence, generated artifact bundles, validation descriptors, and PromotionGate receipts.
- RESEARCH_GAPS_TO_TRACK:
  - MT-004 must compare current Docker/Podman, WSL, Deno, Pyodide/WASM, policy-mode, and future VM/container options before adapter implementation.
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE

### RESEARCH_DEPTH (prevent shallow source logging)
- ADOPT_PATTERNS:
  - NONE
- ADAPT_PATTERNS:
  - NONE
- REJECT_PATTERNS:
  - NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
  - Sandbox run requested/started/blocked/completed/failed/cancelled events must link to kernel task run, session run, EventLedger event IDs, policy ID, adapter ID, artifact bundle ID, validation run ID, and promotion receipt ID when applicable.
  - Validation report, finding, preflight, blocked reason, visual evidence, redaction report, promotion accept/reject, lane wake, closeout bundle, and replay verification events must be durable and reconstructable after restart.
  - Flight Recorder remains observability only; EventLedger/Postgres authority decides scheduling, validation, replay, and promotion truth.

### RED_TEAM_ADVISORY (security failure modes)
  - RISK: policy-scoped sandbox is mistaken for strong isolation. CONTROL: label policy mode as best-effort, default-deny sensitive capabilities, expose hard-isolation unsupported state, and require preflight evidence.
  - RISK: validation descriptors become raw shell passthrough. CONTROL: reject undeclared commands, require side-effect declarations, allowed roots, output manifests, and ToolGate policy checks.
  - RISK: path canonicalization misses symlink, junction, absolute path, or relative escape cases. CONTROL: add platform-aware negative tests and typed denial evidence.
  - RISK: sandbox artifacts leak secrets or env values. CONTROL: minimal env, redaction reports, exportability flags, and secret-like test fixtures.
  - RISK: promotion accepts stale, duplicate, or unvalidated candidates. CONTROL: require base refs, validation refs, idempotency keys, approval refs, and replayable rejection receipts.
  - RISK: role consolidation hides validation gaps. CONTROL: Kernel Builder does implementation only; Integration Validator performs batch MT/WP/spec review in a separate session.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - Kernel003-specific names such as KernelSandboxRunV1 and SandboxPolicyV1 are packet implementation contracts under existing primitive families until a later spec-index enrichment explicitly adds new PRIM IDs.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Active v02.185 already contains the primitive families this packet touches; this activation does not add new Appendix 12.4 PRIM IDs.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - No Appendix 12.4 update pending.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: Kernel003 contract names are scoped to this packet and do not require new spec primitive IDs before implementation.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Kernel V1 authority, validation, promotion, artifact, DCC, and sandbox families already exist in v02.185; Kernel003 activates a packet under them.
- UI_GUIDANCE_ACTION: NO_CHANGE
- UI_GUIDANCE_REASON: DCC action/write-box projection guidance exists in v02.185 and is extended in packet acceptance criteria without requiring appendix mutation.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Existing Kernel V1 action/write-box/promotion edges are sufficient for packet activation; no new IMX ID is required before implementation.
- APPENDIX_MAINTENANCE_NOTES:
  - This pass performs packet-level enrichment and preserves the 80 MTs without opening a new indexed spec bundle.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: Kernel003 touches durable artifact, evidence, and closeout bundle preservation. | STUB_WP_IDS: NONE
- ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: Kernel003 touches evidence lookup, manifest indexing, and no-context inspection paths. | STUB_WP_IDS: NONE
- ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: TOUCHED | NOTES: Kernel003 touches validation findings, blocked taxonomy, summaries, and promotion eligibility analysis. | STUB_WP_IDS: NONE
- ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: TOUCHED | NOTES: Kernel003 touches lane wake, retry, blocked state, and MTE settlement behavior. | STUB_WP_IDS: NONE
- ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Kernel003 touches Postgres-only authority, replay queries, and no-SQLite tripwires. | STUB_WP_IDS: NONE
- ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Kernel003 touches ToolGate, capability grants, policy denial, approval refs, and PromotionGate authority. | STUB_WP_IDS: NONE
- ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: TOUCHED | NOTES: Kernel003 touches DCC/no-context projections and model-facing manual updates. | STUB_WP_IDS: NONE
- ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: Kernel003 touches bounded candidate context, artifact manifests, and validation evidence bundles. | STUB_WP_IDS: NONE
- ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: Kernel003 touches patch proposals, candidate range truth, diff capture, and promotion closeout. | STUB_WP_IDS: NONE
- ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: Kernel003 directly implements sandbox adapter, policy, workspace, guard, resource cap, and run evidence behavior. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Kernel003 emits observability events linked to EventLedger IDs. | STUB_WP_IDS: NONE
- PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Locus | STATUS: TOUCHED | NOTES: Kernel003 emits MT summaries, blocked states, lane settlement, and closeout state for work graph projection. | STUB_WP_IDS: NONE
- PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: Kernel003 preserves 80 MTs and typed packet contracts. | STUB_WP_IDS: NONE
- PILLAR: Task board (product, not repo) | STATUS: TOUCHED | NOTES: Kernel003 moves from stub to active packet and later product projections. | STUB_WP_IDS: NONE
- PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Kernel003 integrates MTE caps, summaries, retry, blocked, and drop-back behavior. | STUB_WP_IDS: NONE
- PILLAR: Command Center | STATUS: TOUCHED | NOTES: Kernel003 exposes sandbox run, validation, promotion, evidence, and blocked projections. | STUB_WP_IDS: NONE
- PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Kernel003 creates sandbox adapter, ValidationRunner, PromotionGate, and bounded execution paths. | STUB_WP_IDS: NONE
- PILLAR: Spec to prompt | STATUS: TOUCHED | NOTES: Kernel003 gives no-context models durable packet, manual, and evidence routes. | STUB_WP_IDS: NONE
- PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Kernel003 reinforces Postgres/EventLedger authority and no-SQLite tripwires. | STUB_WP_IDS: NONE
- PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Kernel003 outputs structured manifests, descriptors, findings, summaries, and receipts. | STUB_WP_IDS: NONE
- PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: Outside Kernel003 activation scope. | STUB_WP_IDS: NONE
- PILLAR: ACE | STATUS: TOUCHED | NOTES: Kernel003 may preserve ACE trace refs when validation evidence uses ACE context, without implementing ACE persistence. | STUB_WP_IDS: NONE
- PILLAR: RAG | STATUS: TOUCHED | NOTES: Kernel003 may preserve retrieval trace refs as evidence handles, without implementing retrieval export. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- PILLAR: Flight Recorder | CAPABILITY_SLICE: Sandbox and promotion event evidence | SUBFEATURES: run events, validation events, promotion receipts, replay verification | PRIMITIVES_FEATURES: PRIM-ValidationExecution | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Locus | CAPABILITY_SLICE: MT summaries and blocked work graph | SUBFEATURES: blocked taxonomy, retries, summaries, lane settlement | PRIMITIVES_FEATURES: PRIM-ValidationRecord | MECHANICAL: engine.wrangler | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Contract-first active packet | SUBFEATURES: 80 MT contracts, packet contract, closeout bundle | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Stub-to-active projection | SUBFEATURES: Ready-for-dev status, traceability registry update, build-order row update | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: MicroTask | CAPABILITY_SLICE: Bounded sandboxed MT attempts | SUBFEATURES: caps, blocked reasons, retry budget, summaries, drop-back | PRIMITIVES_FEATURES: PRIM-ValidationRecord | MECHANICAL: engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Command Center | CAPABILITY_SLICE: Sandbox/validation/promotion projection | SUBFEATURES: run list, run detail, promotion eligibility, evidence links | PRIMITIVES_FEATURES: PRIM-KernelActionCatalogV1 | MECHANICAL: engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Sandboxed execution and validation | SUBFEATURES: adapter, policy, command descriptor, resource caps, preflight | PRIMITIVES_FEATURES: PRIM-ValidationExecution | MECHANICAL: engine.sandbox | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: Spec to prompt | CAPABILITY_SLICE: No-context bounded implementation context | SUBFEATURES: spec anchors, packet context windows, model manual, evidence refs | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres-only kernel authority | SUBFEATURES: run/policy/validation/promotion rows, replay query, no SQLite tripwire | PRIMITIVES_FEATURES: PRIM-PromotionGates | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: LLM-friendly data | CAPABILITY_SLICE: No-context evidence bundles | SUBFEATURES: manifests, findings, summaries, redaction report, closeout bundle | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel003.
- PILLAR: ACE | CAPABILITY_SLICE: Optional ACE evidence reference passthrough | SUBFEATURES: trace refs, query-plan refs, validation evidence links | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Kernel003 preserves references without owning ACE persistence.
- PILLAR: RAG | CAPABILITY_SLICE: Optional retrieval evidence reference passthrough | SUBFEATURES: retrieval trace refs, export-compatible handles, validation evidence links | PRIMITIVES_FEATURES: PRIM-ArtifactManifest | MECHANICAL: engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Kernel003 preserves references without implementing retrieval export.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Capability: Sandbox run execution | JobModel: MECHANICAL_TOOL | Workflow: policy-scoped sandbox run through ToolGate and adapter | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: sandbox_run/requested/started/blocked/completed | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Authority owns state.
- Capability: Validation descriptor execution | JobModel: WORKFLOW | Workflow: descriptors preflight, run checks, normalize findings | ToolSurface: VALIDATION_RUNNER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: validation_started/recorded | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: No raw shell passthrough.
- Capability: Promotion gate | JobModel: WORKFLOW | Workflow: candidate eligibility, accept/reject receipt, EventLedger append | ToolSurface: PROMOTION_GATE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: promotion_requested/accepted/rejected | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: No direct authority mutation.
- Capability: Evidence bundle export | JobModel: ARTIFACT_PIPELINE | Workflow: canonical artifact bundle, redaction report, debug bundle bridge | ToolSurface: ARTIFACT_STORE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: evidence_bundle_recorded | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Export is bounded and redacted.
- Capability: MTE blocked/retry/drop-back | JobModel: WORKFLOW | Workflow: cap checks, blocked classification, retry budget, summaries, lane settlement | ToolSurface: MTE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: mt_blocked/summary/settlement | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Receipts drive wake/settlement.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: Local v02.185 matrix check on 2026-05-15.
- MATRIX_SCAN_NOTES:
  - Existing Kernel V1 primitive and DCC projection matrix coverage is sufficient for packet activation.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: No new interaction edge is required before implementation; Kernel003 uses existing Kernel V1 EventLedger, ToolGate, ValidationRunner, PromotionGate, DCC, and artifact-store relationships.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: Activation uses local spec/stub authority; MT-004 owns external adapter and validation evidence research before implementation.
- SOURCE_SCAN:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: Sandbox doctor/preflight | Pillars: Execution / Job Runtime | Mechanical: engine.sandbox | Primitives/Features: PRIM-ValidationExecution | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because host isolation uncertainty is already in scope.
  - Combo: Canonical bundle hashing | Pillars: LLM-friendly data | Mechanical: engine.archivist | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because validation, export, replay, and future local memory need stable evidence IDs.
  - Combo: No-op skeleton sandbox run | Pillars: Work packets (product, not repo) | Mechanical: engine.sandbox | Primitives/Features: PRIM-ValidationExecution | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because it proves the full run/validate/promote/replay path before risky candidates.
  - Combo: Receipt-driven lane settlement | Pillars: Locus | Mechanical: engine.wrangler | Primitives/Features: PRIM-ValidationRecord | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because it removes chat/terminal state from wake decisions.
  - Combo: DCC evidence drawer | Pillars: Command Center | Mechanical: engine.guide | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because no-context models need inspectable state.
  - Combo: Flight Recorder run correlation | Pillars: Flight Recorder | Mechanical: engine.sovereign | Primitives/Features: PRIM-ValidationExecution | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because every sandbox/validation/promotion event needs durable authority ids.
  - Combo: Task Board activation projection | Pillars: Task board (product, not repo) | Mechanical: engine.version | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because the packet is already moving from stub to ready state.
  - Combo: MicroTask bounded attempt envelope | Pillars: MicroTask | Mechanical: engine.analyst | Primitives/Features: PRIM-ValidationRecord | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because MTE caps, summaries, blocked reasons, and retries are already folded.
  - Combo: Spec prompt context windows | Pillars: Spec to prompt | Mechanical: engine.context | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because future no-context models need bounded spec basis without chat history.
  - Combo: Postgres replay query | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba | Primitives/Features: PRIM-PromotionGates | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because restart proof is central to sandbox validation and promotion.
  - Combo: ACE trace reference bridge | Pillars: ACE | Mechanical: engine.librarian | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because Kernel003 can preserve ACE refs without absorbing ACE implementation.
  - Combo: Retrieval trace reference bridge | Pillars: RAG | Mechanical: engine.librarian | Primitives/Features: PRIM-ArtifactManifest | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because Kernel003 can preserve retrieval refs without absorbing retrieval export.
  - Combo: Candidate range truth | Pillars: Work packets (product, not repo) | Mechanical: engine.version | Primitives/Features: PRIM-ValidationFinding | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because range validation prevents out-of-scope promotion.
  - Combo: Finding normalization | Pillars: LLM-friendly data | Mechanical: engine.analyst | Primitives/Features: PRIM-ValidationFinding | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because normalized findings drive remediation and Integration Validator review.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI additions are already preserved in the 80-MT scope and do not require new stubs.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: Task Board, Build Order, traceability, Kernel001 packet, Kernel002 packet, Kernel003 stub, folded source stubs, reset brief, and v02.185 anchors.
- MATCHED_STUBS:
  - Artifact: WP-KERNEL-003-Sandbox-Validation-Promotion-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Self stub is promoted without reducing 80 MTs.
- MATCHED_ACTIVE_PACKETS:
  - Artifact: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 | BoardStatus: IN_PROGRESS | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: N/A | Resolution: REUSE_EXISTING | Stub: NONE | Notes: Reuse WriteBox, ActionCatalog, CRDT pre-promotion, and direct-edit denial surfaces when landed.
- MATCHED_COMPLETED_PACKETS:
  - NONE
- CODE_REALITY_EVIDENCE:
  - NONE
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Existing same-intent artifact is the Kernel003 stub being activated; dependency surfaces are reused rather than duplicated.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: YES
- UI_UX_REASON_NO: N/A
- UI_SURFACES:
  - DCC sandbox run list.
  - DCC sandbox run detail.
  - Validation report and findings projection.
  - Promotion eligibility/control state.
  - Evidence/debug bundle drawer.
  - Adapter health/preflight panel.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: sandbox run detail | Type: icon button | Tooltip: Open manifests, logs, validation, promotion, and replay evidence | Notes: disabled only when run ID is missing.
  - Control: adapter mode filter | Type: segmented control | Tooltip: Filter local policy, hard isolation, unsupported, and blocked adapter states | Notes: stable row dimensions.
  - Control: promotion request | Type: action button | Tooltip: Request catalog-backed promotion for an eligible validated candidate | Notes: disabled with visible missing requirements.
  - Control: evidence export | Type: icon button | Tooltip: Export redacted canonical bundle | Notes: exportability flags drive availability.
- UI_STATES (empty/loading/error):
  - Empty run list, adapter unsupported, policy denied, validation blocked, promotion ineligible, artifact missing, stale projection, and replay unavailable states.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use Sandbox run, Adapter, Policy, Validation, Promotion, Evidence, Blocked reason, Replay, Redacted export, and Unsupported isolation.
- UI_ACCESSIBILITY_NOTES:
  - Stable row IDs, focusable tooltips, visible disabled reasons, and no reliance on color alone for verdicts.
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: YES
- GUI_ADVICE_REASON_NO: N/A
- GUI_REFERENCE_SCAN:
  - Surface: DCC sandbox validation projection | Source: NONE | Kind: NONE | Pattern: Typed run rows with explicit blocked/promotion state before action | HiddenRequirement: Adapter unsupported, policy denial, validation blocked, stale candidate, and promotion ineligible must remain visible without reading raw JSON | InteractionContract: run rows, evidence rows, validation findings, promotion controls, and adapter states expose stable ids before any action | Accessibility: tooltip content must be available through keyboard focus and visible labels must not carry sole meaning | TooltipStrategy: MIXED | EngineeringTrick: bind every control to run_id plus validation_run_id or promotion_candidate_id before enabling it | Resolution: IN_THIS_WP | Stub: NONE | Notes: Verify with visual debugging when UI exists.
- HANDSHAKE_GUI_ADVICE:
  - Surface: DCC sandbox run list | Control: Open run evidence | Type: icon button | Why: inspect evidence before promotion | Microcopy: Open evidence | Tooltip: Show manifests, validation, promotion, and replay refs.
  - Surface: DCC promotion state | Control: Request promotion | Type: action button | Why: avoid direct authority mutation | Microcopy: Request promotion | Tooltip: Requires validated candidate and approval refs.
- HIDDEN_GUI_REQUIREMENTS:
  - Unsupported adapter, denied capability, blocked validation, stale candidate, and missing approval states remain visible.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Key rows by sandbox_run_id, validation_run_id, promotion_candidate_id, artifact_bundle_id, policy_id, and projection_hash.
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- PHASE_SPLIT_REASON: This activation intentionally preserves the complete folded 80-MT Kernel003 scope; implementation can run MTs back to back with local dependency-aware ordering but cannot drop or merge them.

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Kernel Builder
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.185]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-1-Product-Governance-Check-Runner, WP-1-Product-Governance-Artifact-Registry, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Artifact-System-Foundations, WP-1-Micro-Task-Executor, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
- BUILD_ORDER_BLOCKS: WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-MTE-Resource-Caps, WP-1-MTE-Blocked-Decisioning, WP-1-MTE-Summaries, WP-1-MTE-DropBack-Smart, WP-1-MCP-MEX-Evidence-Export, WP-1-Diagnostics-Debug-Bundle-Bridge
- SPEC_ANCHOR_PRIMARY: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.9
- WHAT: Activate Kernel003 as the sandbox, validation, evidence, promotion, MTE controls, DCC projection, debug bundle bridge, candidate-range truth, closeout bundle, and receipt-driven settlement packet with all 80 preserved MTs.
- WHY: Kernel001 supplies durable authority and Kernel002 supplies write-box/pre-promotion surfaces; Kernel003 makes model-produced work safe by running it in bounded sandbox/validation paths before promotion to product authority.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- OUT_OF_SCOPE:
  - No product implementation in this activation session.
  - No WP Validator gate/session.
  - No Integration Validator launch/verdict/merge/pass-fail claim in this activation session.
  - No condensing, merging, dropping, or renumbering the 80 MTs.
  - No Kernel004 local model memory implementation.
  - No full CRDT workspace beyond KB002 dependencies.
  - No broad legacy SQLite replacement unrelated to Kernel V1 authority.
  - No mandatory production-grade VM/container stack as the only supported adapter.
  - No domain-specific retrieval, Spec Router, AI-ready, cloud-consent, calendar, or mail exporters beyond generic evidence interfaces.
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  just spec-eof-appendices-check
  ```
- DONE_MEANS:
  - Active packet/refinement and exactly 80 MT contracts/projections exist.
  - All fully folded source-stub goals are preserved in packet, MTs, and traceability.
  - Sandbox jobs cannot write authority state directly or escape declared sandbox/output roots.
  - Sandbox policy default-denies filesystem escape, network, process, device, env, and secret access unless explicitly granted and recorded.
  - Sandbox outputs include canonical hashed artifact bundles, manifests, logs, environment metadata, and redaction state.
  - Validation descriptors run deterministic checks and store typed PASS, FAIL, BLOCKED, ADVISORY_ONLY, UNSUPPORTED, SKIPPED_WITH_REASON, or ERROR results.
  - PromotionGate accepts only validated candidates and appends durable EventLedger events linked to validation and approval evidence.
  - Promotion rejection receipts cover stale candidate, duplicate idempotency key, validation failure, policy denial, missing approval, missing artifact, Postgres failure, and projection rebuild failure.
  - MTE resource caps, blocked taxonomy, retry budget, smart drop-back, per-MT summaries, aggregate summaries, and lane settlement are typed and durable.
  - DCC or equivalent projection shows sandbox runs, blocked reasons, validation reports, promotion decisions, and evidence links.
  - Visual validation evidence can be attached when GUI/browser checks are in scope.
  - Kernel003 authority uses Postgres/EventLedger and does not introduce SQLite authority, fallback, fixture, compatibility, or offline path.
  - Validation and promotion evidence remains reconstructable after restart without provider chat history, terminal scrollback, or hidden session context.
  - Generated artifacts, logs, and external tool outputs remain under configured artifact roots and disk-agnostic paths.
  - Implementation closeout requests Integration Validator batch review and does not self-claim PASS/FAIL.
- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-ArtifactManifest
  - PRIM-ValidationExecution
  - PRIM-ValidationFinding
  - PRIM-ValidationRecord
  - PRIM-ValidationResult
  - PRIM-ValidationStatus
  - PRIM-PromotionGates
  - PRIM-PromotionGateSnapshot
  - PRIM-PromotionPath
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/task_packets/stubs/WP-KERNEL-003-Sandbox-Validation-Promotion-v1.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md
  - .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md
  - .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md
  - .GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- SEARCH_TERMS:
  - KernelSandboxRunV1
  - SandboxPolicyV1
  - SandboxAdapter
  - ValidationDescriptor
  - ValidationRun
  - PromotionCandidate
  - PromotionReceipt
  - EventLedger
  - ToolGate
  - ArtifactManifest
  - WriteBoxV1
  - Sandbox MUST prevent filesystem escape
  - no SQLite
- RUN_COMMANDS:
  ```bash
  just pre-work WP-KERNEL-003-Sandbox-Validation-Promotion-v1
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  ```
- RISK_MAP:
  - "Policy sandbox mistaken for hard isolation" -> "unsafe operator/model trust"
  - "Raw validation command passthrough" -> "repo mutation or unbounded execution"
  - "Path escape" -> "sandbox can read/write outside declared roots"
  - "Secret leakage in evidence" -> "unsafe artifact export"
  - "Stale or duplicate promotion" -> "authority corruption"
  - "SQLite authority reintroduced" -> "Kernel V1 replay/promotion drift"
  - "MT condensation" -> "folded obligations lost"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Build Order must move Kernel003 from STUB to READY_FOR_DEV once the packet and worktree exist.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Activation Source Inventory: re-scan stubs, Task Board, Build Order, and traceability for every KB003-related source. Acceptance: packet contains a source fold table at least as detailed as the stub. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/**, .GOV/roles_shared/records/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: folded source scope can be lost before implementation.
  - CLAUSE: MT-002 Conflict Deliberation Record: convert conflict register into signed decisions for raw shell, direct mutation, container-only, SQLite, and domain-evidence bloat. Acceptance: conflicts are approved, rejected, or parked, none silently removed. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/refinements/**, .GOV/task_packets/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: implementation reopens unsafe assumptions.
  - CLAUSE: MT-003 Current Product API Inventory: inspect product modules for KB001/KB002 and existing check/artifact/governance APIs. Acceptance: packet targets real files or declares missing upstream blockers. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: rg EventLedger src/backend/handshake_core; rg WriteBox src/backend/handshake_core | RISK_IF_MISSED: coder may invent duplicate authority.
  - CLAUSE: MT-004 Research Basis Update: compare current sandbox adapter and validation evidence options before implementation. Acceptance: selected adapter sequence and rejected options are documented. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, README.md | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: adapter design may be stale or host-incompatible.
  - CLAUSE: MT-005 Official Packet Contract Generation: promote stub into signed official packet with contracts and 80 MTs. Acceptance: packet is ready but not self-validated by Kernel Builder. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: downstream session lacks executable contract.
  - CLAUSE: MT-006 Product Module Placement Decision: decide where sandbox, validation, and promotion modules live. Acceptance: module topology is documented before scaffolding. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: implementation creates duplicate or misplaced modules.
  - CLAUSE: MT-007 Kernel003 Schema Namespace: define stable schema IDs for KernelSandboxRunV1, SandboxPolicyV1, SandboxWorkspaceV1, SandboxArtifactBundleV1, ValidationRunV1, PromotionDecisionV1, and PromotionReceiptV1. Acceptance: schema names are versioned and referenced by EventLedger events. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: contracts drift across producer and validator.
  - CLAUSE: MT-008 EventLedger Event Type Plan: add Kernel003 event type names and payload expectations. Acceptance: every event carries run ID, actor, session, task, schema version, timestamp, and artifact refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_event_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: replay and promotion evidence cannot connect.
  - CLAUSE: MT-009 Artifact Type Plan: define sandbox and validation artifact classes for logs, diffs, manifests, screenshots, reports, redaction, and receipts. Acceptance: each artifact class has content type, hash policy, exportability default, and retention/default root. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence becomes ad hoc and non-replayable.
  - CLAUSE: MT-010 DCC Projection Contract: define minimum operator projection for sandbox and promotion state. Acceptance: no-context model can inspect state without terminal logs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: operator/model cannot inspect state.
  - CLAUSE: MT-011 Postgres Migration for Sandbox Runs: add authority tables for sandbox runs. Acceptance: records persist and replay after backend restart. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run state is not durable.
  - CLAUSE: MT-012 Postgres Migration for Sandbox Policies: persist versioned sandbox policies. Acceptance: policy changes are versioned and traceable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: policy provenance is missing.
  - CLAUSE: MT-013 Postgres Migration for Validation Runs: persist validation run metadata and summaries. Acceptance: validation results reconstruct without file-system-only state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation evidence is not replayable.
  - CLAUSE: MT-014 Postgres Migration for Promotion Receipts: persist decisions and receipts. Acceptance: duplicate idempotency keys are rejected or idempotently resolved. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_storage --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicate promotion corrupts authority.
  - CLAUSE: MT-015 No SQLite Authority Tripwire: prevent Kernel003 authority from using SQLite in production or tests. Acceptance: Kernel003 authority fails closed without Postgres/EventLedger authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_no_sqlite --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: Kernel V1 no-SQLite law regresses.
  - CLAUSE: MT-016 Replay Projection Storage Query: reconstruct a run from durable rows/events. Acceptance: replay does not read provider chat, terminal scrollback, or transient logs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: restart proof is weak.
  - CLAUSE: MT-017 Legacy Compatibility Blocker Check: detect prerequisite API gaps. Acceptance: missing APIs produce BLOCKED with evidence, not parallel implementations. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_compat --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: implementation forks around prerequisites.
  - CLAUSE: MT-018 SandboxAdapter Trait: define adapter boundary independent of Docker, WSL, Deno, or WASM. Acceptance: at least one adapter can be implemented without changing caller code. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: isolation choice leaks through callers.
  - CLAUSE: MT-019 PolicyScopedLocal Adapter: implement minimum local proof adapter with strict policy checks. Acceptance: policy mode is explicitly not hard isolation and denies sensitive capabilities by default. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy_local --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: local proof becomes unsafe or misleading.
  - CLAUSE: MT-020 HardIsolation Adapter Stub: add non-executing adapter slot for hard isolation. Acceptance: hard isolation absence is typed BLOCKED/UNSUPPORTED, not success. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_hard_isolation --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: host capability absence is hidden.
  - CLAUSE: MT-021 SandboxPolicy Default Deny: implement default-deny policy construction. Acceptance: omitted policy fields deny access. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_policy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: unsafe defaults allow access.
  - CLAUSE: MT-022 Filesystem Scope Guard: enforce read/write roots and prevent path escape. Acceptance: all path escape attempts return typed denial evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_fs_guard --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox can access project or host paths.
  - CLAUSE: MT-023 Network Capability Gate: deny network unless policy grants it. Acceptance: network grants require approval/provenance refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_network --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox can leak or fetch untracked data.
  - CLAUSE: MT-024 Process Execution Allowlist: permit only registered commands/checks. Acceptance: raw shell strings without descriptors are rejected. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_command_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation becomes raw shell execution.
  - CLAUSE: MT-025 Environment and Secret Redaction: prevent env/secret leakage. Acceptance: secret-looking values are not emitted in stored logs or reports. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_redaction --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence leaks sensitive data.
  - CLAUSE: MT-026 Resource Cap Policy: fold MTE resource caps into sandbox policy. Acceptance: overage halts or gates deterministically with evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_resource_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: execution is unbounded.
  - CLAUSE: MT-027 Cancellation and Timeout: add cancellation and timeout handling. Acceptance: cancelled runs cannot promote and have typed terminal state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_timeout --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: hung/cancelled work can leak into promotion.
  - CLAUSE: MT-028 Sandbox Workspace Materializer: materialize candidate inputs into isolated root. Acceptance: no undeclared project files appear in sandbox input manifest. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_workspace --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox input is not auditable.
  - CLAUSE: MT-029 Sandbox Cleanup and Retention: clean temp roots while preserving artifacts. Acceptance: cleanup never deletes project files or authority rows. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_cleanup --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: cleanup can destroy data or evidence.
  - CLAUSE: MT-030 Sandbox Adapter Health Projection: expose adapter health/preflight state. Acceptance: unsupported isolation is visible before run. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, app/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_health --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: operators/models cannot diagnose adapter blockers.
  - CLAUSE: MT-031 PatchProposal Contract: define candidate patch envelope. Acceptance: proposals without base refs or target ranges cannot enter validation. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_patch_proposal --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: candidate identity is incomplete.
  - CLAUSE: MT-032 Candidate Range Truth: validate changed paths/ranges against declared targets. Acceptance: unexpected file edits are rejected before promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_candidate_range --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion can include out-of-scope changes.
  - CLAUSE: MT-033 Diff Capture: capture candidate diffs as stable artifacts. Acceptance: identical candidate produces identical diff artifact hash. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_diff_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence hashes drift.
  - CLAUSE: MT-034 Artifact Bundle Manifest: create canonical bundle format. Acceptance: bundle hash is deterministic for same inputs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: artifact identity is unstable.
  - CLAUSE: MT-035 Stdout/Stderr Log Capture: store bounded command logs. Acceptance: logs never live only in terminal output. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_log_capture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence disappears with terminal scrollback.
  - CLAUSE: MT-036 Environment Manifest: record non-sensitive runtime environment identifiers. Acceptance: manifest explains run context without exposing secrets. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_environment_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run context cannot be reconstructed.
  - CLAUSE: MT-037 Command Manifest: record exactly what commands/checks ran. Acceptance: validators can replay or reason about command intent. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_command_manifest --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be audited.
  - CLAUSE: MT-038 Visual Evidence Attachment: attach KB002 screenshot/visual artifacts to validation reports. Acceptance: GUI reports can reference screenshots and DOM/log evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_visual_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: GUI validation loses evidence.
  - CLAUSE: MT-039 Redaction Report: add redaction report to exportable bundles. Acceptance: default export is redacted and denied artifacts are listed. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_redaction_report --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: exported bundles leak or omit policy state.
  - CLAUSE: MT-040 Artifact Store Integration: store sandbox artifacts through validated artifact system. Acceptance: every artifact has stable handle and hash. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_artifact_store --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence bypasses artifact authority.
  - CLAUSE: MT-041 ValidationDescriptor Contract: define validation command/check descriptors. Acceptance: validation runner rejects undeclared raw commands. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be policy checked.
  - CLAUSE: MT-042 Check Runner Adapter: reuse Product Governance Check Runner. Acceptance: no duplicate check runner is created. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_check_runner.rs, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_check_runner_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicate validation engines drift.
  - CLAUSE: MT-043 Validation Result Schema: define result states and finding shapes. Acceptance: every non-PASS has typed reason and evidence refs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_result --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: blockers are ambiguous.
  - CLAUSE: MT-044 Validation Preflight: preflight descriptors, tools, capabilities, policy mode, paths, and budget. Acceptance: missing tools produce BLOCKED/UNSUPPORTED, not silent skip. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_preflight --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation can silently under-run.
  - CLAUSE: MT-045 Deterministic Check Batch: run deterministic validation before model review. Acceptance: blocking check failure prevents promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_batch --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: unvalidated candidate can reach promotion.
  - CLAUSE: MT-046 Validation Evidence Bundle: store validation outputs canonically. Acceptance: validation report can be inspected offline. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validator lacks portable evidence.
  - CLAUSE: MT-047 Finding Normalization: normalize check output into findings. Acceptance: raw logs are not the only finding source. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_finding_normalization --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: findings cannot drive remediation.
  - CLAUSE: MT-048 Advisory vs Blocking Rules: make blocking posture explicit. Acceptance: advisory failure is visible but does not block unless configured. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_posture --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: advisory/blocking semantics drift.
  - CLAUSE: MT-049 Validation Replay: re-run descriptor set against same candidate. Acceptance: replay records new run ID linked to original. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation cannot be reproduced.
  - CLAUSE: MT-050 Validation Report Projection: expose validation summaries to DCC/projection layer. Acceptance: operator/model can inspect validation without reading raw files first. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_validation_projection --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: validation state is hidden.
  - CLAUSE: MT-051 PromotionCandidate Contract: define promotion candidate shape from patch proposal or write box. Acceptance: missing validation refs block promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_candidate --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion input is incomplete.
  - CLAUSE: MT-052 Promotion Eligibility Check: implement promotion preconditions. Acceptance: ineligible candidate produces typed rejection receipt. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_eligibility --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: invalid candidate can promote.
  - CLAUSE: MT-053 Promotion Accept Path: append accepted promotion events to EventLedger. Acceptance: accepted promotion is replayable from durable events. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_accept --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: authority mutation is not durable.
  - CLAUSE: MT-054 Promotion Reject Path: record rejected promotion attempts. Acceptance: reject path creates receipt and does not mutate authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_reject --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: failed promotion is invisible.
  - CLAUSE: MT-055 Idempotency Key Enforcement: prevent duplicate promotion effects. Acceptance: duplicate accept returns prior receipt or typed duplicate rejection without second mutation. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_idempotency --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: duplicates corrupt authority.
  - CLAUSE: MT-056 Approval Ref Binding: bind approval evidence to promotion decisions. Acceptance: promotion cannot accept without required approval posture. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_approval --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: promotion can bypass approval.
  - CLAUSE: MT-057 Authority Mutation Boundary: sandbox and validation cannot mutate authority except through PromotionGate. Acceptance: direct mutation attempt produces denial evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_authority_boundary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox bypasses promotion law.
  - CLAUSE: MT-058 Promotion Closeout Bundle: implement canonical closeout bundle. Acceptance: Integration Validator can review one bundle for promotion. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_closeout_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: review evidence remains scattered.
  - CLAUSE: MT-059 MTE Run Cap Integration: wire resource caps into sandboxed microtask execution. Acceptance: cap overage halts bounded run and writes evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mte_caps --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MTE folded cap scope is lost.
  - CLAUSE: MT-060 Blocked Reason Taxonomy: implement blocked decisioning for sandbox/validation. Acceptance: each blocked reason has retry/escalate/gate semantics. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_blocked_taxonomy --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: blocked work cannot be routed.
  - CLAUSE: MT-061 Retry Budget: bound retry behavior. Acceptance: retry exhaustion becomes typed BLOCKED/FAILED. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_retry_budget --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: loops become unbounded.
  - CLAUSE: MT-062 Smart DropBack: implement smart drop-back semantics. Acceptance: smart/always/never modes have test coverage. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dropback --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: escalation/retry recovery is ad hoc.
  - CLAUSE: MT-063 Per-MT Summary Artifact: persist per-microtask summaries. Acceptance: every completed/blocked MT attempt has summary ref. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: no-context handoff loses MT evidence.
  - CLAUSE: MT-064 Aggregate Run Summary: persist aggregate summary across attempts. Acceptance: no-context reviewer can inspect aggregate before raw artifacts. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_aggregate_summary --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: packet review is too expensive and brittle.
  - CLAUSE: MT-065 Lane Wake Receipt: implement receipt-driven lane wake/settlement. Acceptance: wake/settlement event includes receipt refs and reason. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_lane_wake --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: lane state depends on chat.
  - CLAUSE: MT-066 Bootstrap Skeleton Receipt Projection: first skeleton sandbox run creates restartable receipts. Acceptance: all receipts visible after restart. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_sandbox_skeleton --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: integration path is unproven.
  - CLAUSE: MT-067 DCC Sandbox Run List: add projection/API for sandbox run list. Acceptance: operator can find current and past sandbox runs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_sandbox_list --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: run discovery is missing.
  - CLAUSE: MT-068 DCC Run Detail: add projection/API for sandbox run detail. Acceptance: detail view has no hidden dependency on terminal scrollback. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_run_detail --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: evidence inspection remains manual.
  - CLAUSE: MT-069 DCC Promotion Control State: expose promotion eligibility and approval state. Acceptance: UI/API cannot promote when eligibility is false. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc_promotion_state --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: UI may imply unsafe promotion.
  - CLAUSE: MT-070 Debug Bundle Bridge: fold diagnostics debug bundle into evidence output. Acceptance: diagnostics evidence is bounded and portable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_debug_bundle --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: debug evidence cannot support validation.
  - CLAUSE: MT-071 MCP/MEX Evidence Export Bridge: fold tool/mechanical engine evidence into sandbox evidence. Acceptance: MCP/MEX evidence does not use ad hoc bundle schema. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mex_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: tool evidence is non-portable.
  - CLAUSE: MT-072 Capability Audit Evidence Link: link grants/denials to capability evidence. Acceptance: every sensitive grant has provenance. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_capability_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: grants cannot be audited.
  - CLAUSE: MT-073 Visual Validation Gate Descriptor: define visual evidence validation mapping. Acceptance: visual evidence can block or advise according to descriptor posture. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_visual_descriptor --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: visual evidence cannot influence validation safely.
  - CLAUSE: MT-074 Console and Network Evidence: capture browser/app console and network evidence for GUI checks. Acceptance: GUI validation failures are diagnosable. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_gui_evidence --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: UI failures lack actionable evidence.
  - CLAUSE: MT-075 Security Denial Test Matrix: add negative tests for sandbox boundaries. Acceptance: every denial writes typed evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_security_denial --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: sandbox security claims are unproven.
  - CLAUSE: MT-076 Promotion Failure Test Matrix: test each promotion failure scenario. Acceptance: no failure mutates authority. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_promotion_failure --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: rejection paths can mutate or disappear.
  - CLAUSE: MT-077 Restart Replay Test: prove state survives restart. Acceptance: replay is complete from durable product state. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_restart_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: hidden session state remains required.
  - CLAUSE: MT-078 Disk-Agnostic Path Test: prove paths remain repo-root/config relative. Acceptance: moving workspace root does not break path resolution. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/tests/**, src/backend/handshake_core/src/kernel/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_disk_agnostic --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: local drive paths leak into product.
  - CLAUSE: MT-079 Documentation and Model Manual Update: update product-local model-facing manual. Acceptance: no-context model can run/inspect sandbox workflow from durable docs. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: README.md, src/backend/handshake_core/src/kernel/**, app/** | EXPECTED_TESTS: just gov-check | RISK_IF_MISSED: future sessions depend on chat history.
  - CLAUSE: MT-080 Integration Validator Handoff: prepare final validation bundle without self-validating. Acceptance: Kernel Builder/Coder does not claim PASS/FAIL and validator has sufficient evidence. | WHY_IN_SCOPE: preserved Kernel003 microtask | EXPECTED_CODE_SURFACES: .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: just gov-check; cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: final review lacks batch evidence.

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: KernelSandboxRunV1 and SandboxPolicyV1 | PRODUCER: sandbox runner and policy builder | CONSUMER: ToolGate, ValidationRunner, DCC, Integration Validator | SERIALIZER_TRANSPORT: Postgres records plus EventLedger event payloads | VALIDATOR_READER: sandbox policy and run replay tests | TRIPWIRE_TESTS: omitted grants deny, unsupported adapters block, runs replay after restart | DRIFT_RISK: sandbox grants become implicit.
  - CONTRACT: SandboxWorkspaceV1 and SandboxArtifactBundleV1 | PRODUCER: workspace materializer and artifact store | CONSUMER: validators, DCC, promotion gate, debug bundle exporter | SERIALIZER_TRANSPORT: artifact handles, manifest JSON, hashes, redaction report | VALIDATOR_READER: bundle hash and path guard tests | TRIPWIRE_TESTS: path escape denial, deterministic bundle hash, no terminal-only logs | DRIFT_RISK: evidence becomes file-only.
  - CONTRACT: ValidationDescriptorV1, ValidationRunV1, and ValidationFindingV1 | PRODUCER: validation descriptor registry and check runner adapter | CONSUMER: PromotionGate, DCC, Integration Validator | SERIALIZER_TRANSPORT: JSON descriptors/results and Postgres rows | VALIDATOR_READER: validation preflight, result, replay tests | TRIPWIRE_TESTS: raw command denial, blocked tool, advisory/blocking mix | DRIFT_RISK: raw shell silently runs.
  - CONTRACT: PromotionCandidateV1, PromotionDecisionV1, and PromotionReceiptV1 | PRODUCER: promotion gate | CONSUMER: EventLedger, DCC, Locus, Integration Validator | SERIALIZER_TRANSPORT: EventLedger events, Postgres rows, receipts | VALIDATOR_READER: accept/reject/idempotency tests | TRIPWIRE_TESTS: stale, duplicate, missing approval, validation failure, projection failure | DRIFT_RISK: invalid candidate mutates authority.
  - CONTRACT: MTE bounded-run controls | PRODUCER: sandbox/MTE runtime | CONSUMER: Locus, DCC, summaries, lane settlement | SERIALIZER_TRANSPORT: run cap policy, blocked reason, summary artifact, wake receipt | VALIDATOR_READER: cap, blocked, retry, drop-back, summary tests | TRIPWIRE_TESTS: cap overage, retry exhaustion, settlement by receipt | DRIFT_RISK: MT execution loops become unbounded.
  - CONTRACT: DCC sandbox validation projections | PRODUCER: projection registry | CONSUMER: operator, no-context model, Integration Validator | SERIALIZER_TRANSPORT: structured projection JSON plus stable artifact refs | VALIDATOR_READER: API/projection and visual checks | TRIPWIRE_TESTS: disabled promotion when ineligible, stable row ids, stale badges | DRIFT_RISK: UI hides unsafe state.

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Implement MT-001 through MT-080 back to back in declared order unless dependency proof requires local reordering without dropping, merging, renumbering, or condensing any MT.
- HOT_FILES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/governance_check_runner.rs
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/flight_recorder/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- CARRY_FORWARD_WARNINGS:
  - Do not condense or remove any MT.
  - Do not introduce WP Validator gate.
  - Do not treat Kernel Builder checks as validation.
  - Do not use SQLite authority, cache, offline, fallback, compatibility, or test fixtures for Kernel003.
  - Do not create parallel EventLedger, CheckRunner, ArtifactStore, WriteBox, or PromotionGate systems when KB001/KB002 surfaces exist.
  - PolicyScopedLocal is not hard isolation; label it honestly and expose hard isolation unsupported state.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - All 80 MT clauses and contracts.
  - Fully folded source-stub goals remain present and traceable.
  - Sandbox and validation cannot mutate authority except through PromotionGate.
  - Default-deny policy, path guardrails, caps, blocked taxonomy, and redaction are enforced.
  - Promotion accept/reject/idempotency/replay paths are durable EventLedger/Postgres authority.
  - No SQLite authority, fallback, compatibility, offline, or fixture path exists for Kernel003.
  - No WP Validator gate; Integration Validator batch/spec review is separate.
- FILES_TO_READ:
  - .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/packet.json
  - .GOV/task_packets/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/MT-*.json
  - .GOV/spec/SPEC_CURRENT.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/tests/**
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_sandbox --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_validation --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_promotion --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mte --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
  - just spec-eof-appendices-check
- POST_MERGE_SPOTCHECKS:
  - No-context manual path, sandbox denial harness, validation replay proof, promotion rejection proof, and DCC evidence visibility.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - Product implementation has not started.
  - KB001 and KB002 product APIs may not be fully landed in the implementation worktree.
  - MT-004 current external research has not run yet.
  - Host hard-isolation availability is unknown.
  - Integration Validator verdict does not exist yet.
  - DCC screenshot evidence waits for GUI implementation.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: v02.185 names Kernel V1 EventLedger, ToolGate, ArtifactProposal, ValidationRunner, PromotionGate, sandbox default-deny constraints, observability IDs, DCC projection posture, WriteBox promotion bridge, and no-SQLite authority; the Kernel003 stub supplies 80 measurable MT clauses and the packet-level contract names.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Active v02.185 provides sufficient topical authority for Kernel003 activation; this pass enriches the packet/refinement layer and preserves MT detail without requiring a new indexed Master Spec bundle.
- PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates):
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### FEATURE_DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: existing v02.185 primitive families exposed: PRIM-KernelActionCatalogV1, PRIM-KernelActionDescriptorV1, PRIM-WriteBoxV1, PRIM-ArtifactManifest, PRIM-ValidationExecution, PRIM-ValidationFinding, PRIM-ValidationRecord, PRIM-ValidationResult, PRIM-ValidationStatus, PRIM-PromotionGates, PRIM-PromotionGateSnapshot, PRIM-PromotionPath.
- DISCOVERY_STUBS: NONE_CREATED - existing Kernel003 stub promoted.
- DISCOVERY_MATRIX_EDGES: NONE_FOUND - existing Kernel V1 edges are sufficient for activation.
- DISCOVERY_UI_CONTROLS: sandbox run list, run detail evidence drawer, adapter health filter, validation report projection, promotion eligibility control, redacted export control.
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED - v02.185 is sufficient for activation and packet-level contracts carry Kernel003 details.
- DISCOVERY_JUSTIFICATION: Concrete sandbox, validation, promotion, evidence, MTE, and UI controls are preserved in this packet and do not require a new spec version before implementation.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.9
- CONTEXT_START_LINE: 3688
- CONTEXT_END_LINE: 3724
- CONTEXT_TOKEN: Kernel V1 Authority State [ADD v02.184]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.9 Kernel V1 Authority State [ADD v02.184]
  Kernel V1 runtime authority MUST be product-owned durable state, not provider chat history, terminal transcripts, repo-governance artifacts, or diagnostic mirrors.
  - A Postgres-backed append-only EventLedger for kernel task, session, tool, artifact, validation, and promotion events.
  - ToolGate, ArtifactProposal, ValidationRunner, and PromotionGate events linked to the same run IDs.
  Kernel V1 MUST NOT use SQLite for authority, cache, offline mode, compatibility mode, local fallback, bootstrap convenience, or test fixtures.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
- CONTEXT_START_LINE: 3710
- CONTEXT_END_LINE: 3724
- CONTEXT_TOKEN: Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.13.10 Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
  Kernel V1 CRDT workspace state is pre-promotion working state.
  The Kernel V1 implementation MUST provide a KernelActionCatalogV1 contract that enumerates every write-capable kernel action before it can mutate a draft or request promotion.
  Direct edits to authoritative Kernel V1 records MUST be denied unless they enter through an allowed catalog action and write box path.
  The CRDT-to-EventLedger promotion bridge MUST be explicit.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md#5.2.5
- CONTEXT_START_LINE: 22958
- CONTEXT_END_LINE: 22967
- CONTEXT_TOKEN: Mechanical Runner Sandbox
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 5.2.5 Mechanical Runner Sandbox
  - Mechanical engines run via a constrained runner: explicit allowlist per engine, resource limits (CPU/GPU/mem/time), and capability gates (file/process/network/device).
  - Log command, params, cwd, exit code, stdout/stderr, artifact hashes; refuse/abort when capability is missing or bounds exceeded.
  - Provide refusal paths and tests to ensure engines cannot bypass Workflow/Flight Recorder or capabilities.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/05-security-and-observability.md#5.4.5
- CONTEXT_START_LINE: 23481
- CONTEXT_END_LINE: 23500
- CONTEXT_TOKEN: Kernel V1 Authority Observability Boundary [ADD v02.184]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 5.4.5 Kernel V1 Authority Observability Boundary [ADD v02.184]
  Flight Recorder remains mandatory append-only observability, but Kernel V1 replay and promotion authority MUST come from the Postgres EventLedger defined in Section 2.3.13.9.
  Kernel V1 observability MUST expose enough structured fields for no-context debugging: kernel_task_run_id, session_run_id, event_ledger_id, artifact_proposal_id, validation_run_id, and promotion_gate_id.
  Security and observability tests for Kernel V1 MUST prove that replay still works when Flight Recorder or provider trace history is unavailable and EventLedger rows remain intact.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#10.11.5.28
- CONTEXT_START_LINE: 61703
- CONTEXT_END_LINE: 61719
- CONTEXT_TOKEN: Kernel Action Catalog and Write Box Projections [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]
  The Dev Command Center MUST expose Kernel V1 action-catalog and write-box state as typed product projections, not as raw transcript or repo-governance mirrors.
  Dev Command Center controls MAY request catalog-backed write-box actions or promotion, but they MUST NOT directly mutate EventLedger authority or silently apply CRDT updates as authority.
  Visual debugging and acceptance proof MUST include stable element identifiers for action catalog rows, write-box rows, denial receipts, promotion previews, and stale projection badges.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/11-shared-dev-platform-and-oss-foundations.md#sandbox-minimum
- CONTEXT_START_LINE: 68955
- CONTEXT_END_LINE: 68970
- CONTEXT_TOKEN: Sandbox MUST prevent filesystem escape
- EXCERPT_ASCII_ESCAPED:
  ```text
  Any device.*, net.http, or secrets.use:* MUST require policy approval and be recorded in provenance.
  Sandbox MUST prevent filesystem escape, deny network unless granted, deny exec unless allowlisted, and record environment identifiers.
  Required global gates: G-SCHEMA, G-CAP, G-INTEGRITY, G-BUDGET, G-PROVENANCE, G-DET.
  ```
