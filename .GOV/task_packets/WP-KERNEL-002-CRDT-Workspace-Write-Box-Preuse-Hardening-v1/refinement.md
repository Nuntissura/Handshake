<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.refinement_contract@1 source_file=.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/refinement.json source_hash=90af097732727cee projection_hash=0c9d9b8257cf3aea generated_at_utc=2026-05-14T08:35:39.870Z generator=kernel-builder-activation-readiness.mjs -->
## TECHNICAL_REFINEMENT

### METADATA
- WP_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-15
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-05-14T04:52:31.378Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json
- SPEC_TARGET_SHA1: 29ae893608ccb3d9ba2bd9fc84a3eca8887de295
- SPEC_TARGET_SHA256: 7286cbee9ce394dc1bb881cf4f20f26eee58a35aebc9a879c8af4a6efc2e7357
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja140520260455
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- STUB_WP_IDS: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.refinement_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/refinement.json
- MARKDOWN_PROJECTION_STATUS: GENERATED_IN_SYNC
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
- READINESS_STATUS: READY_FOR_IMPLEMENTATION
- FINAL_CHECKS: PASS phase-check STARTUP; packet-contract projection; declared topology; truth bundle; communication health.

### GAPS_IDENTIFIED
  - Kernel002 needed explicit CRDT workspace, action catalog, write-box, denial, and promotion bridge authority before implementation.
  - The folded stub scope must activate without condensing the 61 MTs.
  - Legacy SQLite/Markdown/mailbox/UI-local truth assumptions must become Postgres/EventLedger authority, projections, or write-box proposals.
  - This packet uses Kernel Builder consolidated implementation and a separate Integration Validator batch review; no WP Validator gate.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: Local spec/stub scan on 2026-05-14.
- SEARCH_SCOPE: SPEC_CURRENT v02.185, Kernel002 stub, Kernel001 packet, Task Board, Build Order, traceability, and reset brief.
- REFERENCES: .GOV/spec/SPEC_CURRENT.md; v02.185 modules 02, 03, 10, 12; Kernel002 stub Markdown/contract; Kernel001 packet.
- PATTERNS_EXTRACTED: ADOPT action/write-box gating; ADOPT CRDT as pre-promotion state; ADAPT DCC as projection/control surface; REJECT CRDT/Markdown/UI/mailbox/SQLite as final authority.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT v02.185 and 61 MT preservation; ADAPT packet topology to no WP Validator; REJECT scope collapse.
- LICENSE/IP_NOTES: NONE.
- SPEC_IMPACT: YES
- SPEC_IMPACT_REASON: v02.185 was applied before activation; no further update is pending.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: Activation is grounded in local Master Spec v02.185 and prepared stub; MT-003 owns current CRDT ADR research.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Spec basis already selects action catalog/write boxes over direct edits and EventLedger/PromotionGate over CRDT convergence.
- RESEARCH_GAPS_TO_TRACK:
  - MT-003 must compare current CRDT libraries and storage integration before implementation.
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
  - Action, denial, write-box, CRDT, promotion, MT handoff, Integration Validator verdict, screenshot, projection rebuild, and remediation dispatch events must be replayable evidence linked to product authority ids.
  - Flight Recorder is observability only and never authority for status, validation, CRDT, or promotion truth.

### RED_TEAM_ADVISORY (security failure modes)
  - RISK: direct edits bypass action law. CONTROL: deny or wrap raw diffs as ProposalBox/PatchBox with denial evidence.
  - RISK: CRDT convergence is mistaken for authority. CONTROL: require PromotionGate/EventLedger acceptance.
  - RISK: generated projections become stale truth. CONTROL: hashes, stale badges, advisory normalization, and denial.
  - RISK: role consolidation hides validation gaps. CONTROL: Integration Validator remains separate authority.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- NOTES:
  - v02.185 already added these primitive ids; this refinement creates no new primitive index update.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: v02.185 already added Kernel002 primitive coverage.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - No Appendix 12.4 update pending.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: v02.185 attached the high-signal Kernel002 primitives.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: FEAT-KERNEL-WORKSPACE-WRITE-BOX is already present in v02.185.
- UI_GUIDANCE_ACTION: NO_CHANGE
- UI_GUIDANCE_REASON: DCC action/write-box guidance is already present in v02.185.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: IMX-136 and IMX-137 are already active in v02.185.
- APPENDIX_MAINTENANCE_NOTES:
  - v02.185 enrichment was applied copy-first before signature.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: Kernel002 touches this engine through storage, legality, guidance, context, versioning, or sandbox-ready proposal hooks. | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Locus | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Task board (product, not repo) | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: MicroTask | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Command Center | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Spec to prompt | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Kernel002 folds this pillar through action/write-box, DCC, Locus, MT loop, Postgres, model-context, or projection behavior. | STUB_WP_IDS: NONE
- PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: Outside Kernel002 activation scope. | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- PILLAR: Flight Recorder | CAPABILITY_SLICE: Action and denial evidence | SUBFEATURES: action, denial, promotion, screenshot, validation, and replay receipts | PRIMITIVES_FEATURES: PRIM-WriteBoxDirectEditDeniedV1 | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Locus | CAPABILITY_SLICE: MT validation loop graph | SUBFEATURES: MT nodes, remediation edges, leases, blocked/escalated states | PRIMITIVES_FEATURES: PRIM-WriteBoxPromotionReceiptV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Contract-first work authority | SUBFEATURES: StubContractV1, WorkPacketContractV1, MicroTaskContractV1 | PRIMITIVES_FEATURES: FEAT-KERNEL-WORKSPACE-WRITE-BOX | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Projection sync | SUBFEATURES: status projection, drift queue, advisory mirror normalization | PRIMITIVES_FEATURES: PRIM-WriteBoxDirectEditDeniedV1 | MECHANICAL: engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: MicroTask | CAPABILITY_SLICE: Fresh-context MT loop | SUBFEATURES: one-MT bundle, retry budget, handoff, verdict, remediation | PRIMITIVES_FEATURES: PRIM-WriteBoxV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Command Center | CAPABILITY_SLICE: Kernel workbench projections | SUBFEATURES: catalog viewer, write-box queue, artifacts, layouts, visual debug | PRIMITIVES_FEATURES: PRIM-KernelActionCatalogV1 | MECHANICAL: engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Registered transitions | SUBFEATURES: action envelope, transition registry, queues, leases, recovery | PRIMITIVES_FEATURES: PRIM-KernelActionDescriptorV1 | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: Spec to prompt | CAPABILITY_SLICE: Bounded model context | SUBFEATURES: manual, CRDT slices, anchors, prompt-safe extracts | PRIMITIVES_FEATURES: PRIM-CrdtWorkspaceDraftV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Postgres authority reset | SUBFEATURES: CRDT updates, snapshots, no SQLite authority | PRIMITIVES_FEATURES: PRIM-CrdtWorkspaceSnapshotV1 | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR: LLM-friendly data | CAPABILITY_SLICE: Typed model-operable surfaces | SUBFEATURES: catalog schemas, write boxes, CRDT slices, mailbox state, FEMS checkpoints | PRIMITIVES_FEATURES: PRIM-WriteBoxV1 | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Preserved inside Kernel002.
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Capability: Action catalog and denial | JobModel: MECHANICAL_TOOL | Workflow: registered action schemas and denial/proposal receipts | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: action_request/action_denial | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
- Capability: CRDT workspace promotion | JobModel: WORKFLOW | Workflow: CRDT updates promote through ArtifactProposal and PromotionGate | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: crdt_update/promotion_receipt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
- Capability: MT remediation loop | JobModel: WORKFLOW | Workflow: handoff and Integration Validator verdicts create loop records | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: handoff/verdict/remediation | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
- Capability: DCC visual evidence | JobModel: UI_ACTION | Workflow: catalog/write-box/screenshot/stale-state projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: screenshot/projection_rebuilt | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Product authority owns state.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: Local v02.185 matrix check on 2026-05-14.
- MATRIX_SCAN_NOTES:
  - IMX-136 and IMX-137 already exist in active v02.185; no new edge is pending.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Existing v02.185 matrix edges cover this activation; no new IMX id is added.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: Activation uses local spec/stub authority; implementation ADR MT-003 owns external research.
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
  - Combo: Catalog denial evidence | Pillars: Flight Recorder | Mechanical: engine.sovereign | Primitives/Features: PRIM-WriteBoxDirectEditDeniedV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Locus remediation graph | Pillars: Locus | Mechanical: engine.context | Primitives/Features: PRIM-WriteBoxPromotionReceiptV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Packet regeneration | Pillars: Work packets (product, not repo) | Mechanical: engine.version | Primitives/Features: FEAT-KERNEL-WORKSPACE-WRITE-BOX | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Board projection freshness | Pillars: Task board (product, not repo) | Mechanical: engine.version | Primitives/Features: PRIM-WriteBoxDirectEditDeniedV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Fresh-context MTs | Pillars: MicroTask | Mechanical: engine.context | Primitives/Features: PRIM-WriteBoxV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: DCC write-box queue | Pillars: Command Center | Mechanical: engine.guide | Primitives/Features: PRIM-KernelActionCatalogV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Runtime transition law | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-KernelActionDescriptorV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Spec prompt slices | Pillars: Spec to prompt | Mechanical: engine.context | Primitives/Features: PRIM-CrdtWorkspaceDraftV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Postgres CRDT replay | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba | Primitives/Features: PRIM-CrdtWorkspaceSnapshotV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: LLM structured writes | Pillars: LLM-friendly data | Mechanical: engine.context | Primitives/Features: PRIM-WriteBoxV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Sandbox-ready hooks | Pillars: NONE | Mechanical: engine.sandbox | Primitives/Features: PRIM-WriteBoxPromotionRequestV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
  - Combo: Guide manual to catalog | Pillars: NONE | Mechanical: engine.guide | Primitives/Features: PRIM-KernelActionCatalogV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: High ROI because this area is already being activated.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All discovered high-ROI combinations are carried into this WP.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: Task Board, Build Order, traceability, Kernel001 packet, Kernel002 stub, folded stubs, and v02.185 coverage.
- MATCHED_STUBS:
  - Artifact: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: SAME | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: Self stub is promoted without reducing 61 MTs.
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - NONE
- CODE_REALITY_EVIDENCE:
  - NONE
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Existing same-intent artifact is the Kernel002 stub being activated.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: YES
- UI_UX_REASON_NO: N/A
- UI_SURFACES:
  - DCC action catalog viewer.
  - DCC write-box queue.
  - Direct-edit denial/recovery panel.
  - CRDT promotion preview and stale projection badges.
  - Visual debugging evidence viewer.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: action preview | Type: icon button | Tooltip: Show schemas, write boxes, validation hooks, and promotion path | Notes: disabled until actor/action eligibility resolves.
  - Control: write-box filter | Type: segmented control | Tooltip: Filter boxes by lifecycle and authority effect | Notes: stable row dimensions.
  - Control: denial recovery | Type: action button | Tooltip: Convert denied edit into advisory proposal | Notes: denial evidence remains visible.
- UI_STATES (empty/loading/error):
  - Empty catalog, loading queue, denied edit, stale projection, and promotion blocked states.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use Action, Write box, Authority effect, Promotion preview, Denied edit, Stale projection, and Evidence.
- UI_ACCESSIBILITY_NOTES:
  - Tooltips work on hover and keyboard focus and do not obscure queue rows.
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: YES
- GUI_ADVICE_REASON_NO: N/A
- GUI_REFERENCE_SCAN:
  - Surface: Dev Command Center kernel write box projection | Source: NONE | Kind: NONE | Pattern: Typed queue rows with stable ids and preview before mutation | HiddenRequirement: Direct-edit denial, stale projection, and promotion race state must remain visible without reading raw JSON | InteractionContract: action catalog rows, write-box rows, denial receipts, promotion previews, and stale badges expose stable ids before any action | Accessibility: tooltip content must be available through keyboard focus and visible labels must not carry sole meaning | TooltipStrategy: MIXED | EngineeringTrick: bind every control to action_id plus target authority class before enabling it | Resolution: IN_THIS_WP | Stub: NONE | Notes: Verify with visual debugging.
- HANDSHAKE_GUI_ADVICE:
  - Surface: DCC action catalog | Control: Preview action | Type: icon button | Why: inspect authority effect before mutation | Microcopy: Preview action | Tooltip: Show schemas and promotion path.
- HIDDEN_GUI_REQUIREMENTS:
  - Denial, stale projection, and promotion-blocked states remain visible.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Key rows by action_id, write_box_id, target_authority_id, projection_hash, and evidence_ref.
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- PHASE_SPLIT_REASON: This activation intentionally preserves the complete folded 61-MT Kernel002 scope.

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Kernel Builder
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.185]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-1-Global-Silent-Edit-Guard, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Artifact-System-Foundations, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
- BUILD_ORDER_BLOCKS: WP-KERNEL-003-Sandbox-Validation-Promotion, WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-Software-Delivery-Runtime-Truth, WP-1-Workflow-Transition-Automation-Registry, WP-1-Dev-Command-Center-MVP, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Visual-Debugging-Loop
- SPEC_ANCHOR_PRIMARY: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
- WHAT: Activate Kernel002 as the CRDT workspace, action catalog, write-box, direct-edit denial, DCC projection, Role Mailbox, FEMS, Locus, generated-doc, and MT validation-loop hardening packet with all 61 preserved MTs.
- WHY: Kernel001 supplies authority substrate, but safe no-context model use needs registered actions/write boxes and CRDT drafts promoted through EventLedger gates.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/tests/**
  - app/**
  - tests/**
  - README.md
- OUT_OF_SCOPE:
  - No product implementation in this activation session.
  - No WP Validator gate/session.
  - No Integration Validator launch/verdict/merge/pass-fail claim in this activation session.
  - No condensing or reducing the 61 MTs.
  - No SQLite authority or fallback store.
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  just spec-eof-appendices-check
  ```
- DONE_MEANS:
  - Active packet/refinement and exactly 61 MT contracts/projections exist.
  - Action catalog and WriteBoxV1 family prevent or normalize direct edits.
  - CRDT updates/snapshots/promotions persist and remain non-authoritative until EventLedger acceptance.
  - DCC, Role Mailbox, FEMS, Locus, docs/status, visual-debug, and MT loops consume typed authority.
  - No WP Validator gate exists; Integration Validator batch review remains separate.
- PRIMITIVES_EXPOSED:
  - PRIM-KernelActionCatalogV1
  - PRIM-KernelActionDescriptorV1
  - PRIM-WriteBoxV1
  - PRIM-WriteBoxDirectEditDeniedV1
  - PRIM-WriteBoxPromotionRequestV1
  - PRIM-WriteBoxPromotionReceiptV1
  - PRIM-CrdtWorkspaceDraftV1
  - PRIM-CrdtWorkspaceSnapshotV1
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.md
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- SEARCH_TERMS:
  - KernelActionCatalogV1
  - WriteBoxV1
  - WriteBoxDirectEditDeniedV1
  - CRDT workspace
  - PromotionGate
  - EventLedger
  - Role Mailbox
  - FEMS
  - Locus
  - visual debugging
  - MicroTaskContractV1
- RUN_COMMANDS:
  ```bash
  just pre-work WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  ```
- RISK_MAP:
  - "Direct edit bypass" -> "authority mutation outside action law"
  - "CRDT equals authority" -> "invalid promotions"
  - "Projection drift" -> "stale operator/model state"
  - "MT condensation" -> "lost folded obligations"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Build Order must point to v02.185 and this packet once activated.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: MT-001 Fold Preservation Manifest and Source Import: materialize the complete folded-source manifest in the official packet/refinement. Acceptance: every listed source stub has path, pre-fold hash, direct/transitive fold classification, and source-scope import instructions. Activation cannot proceed if any source file is missing or hash mismatch is unexplained. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-001 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-002 Reset Invariant Reconciliation: reconcile folded legacy assumptions with reset invariants. Acceptance: every source obligation that mentions SQLite, Markdown authority, mailbox chronology, or UI-local truth is explicitly converted to Postgres authority, projection/advisory status, or promotion-gated action semantics. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-002 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-003 CRDT Library and Storage ADR: compare Yjs, Loro, Automerge, and existing product dependencies against Handshake runtime needs. Acceptance: ADR selects a CRDT approach, rejected options, sync/storage model, Rust/TypeScript integration boundary, schema compatibility, and validation plan. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-003 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-004 Kernel Action Envelope: define `KernelActionRequestV1`, `KernelActionResultV1`, `KernelActionDenialV1`, and receipt/event mappings. Acceptance: action requests carry actor/session/profile, target ids, input schema id, expected write boxes, authority effect, approval posture, validation requirements, and trace id. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-004 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-005 Action Catalog Registry: implement the durable `KernelActionCatalogV1` registry. Acceptance: every model-facing action has stable id, schemas, role eligibility, capability requirements, write boxes, promotion path, validation hooks, and DCC preview metadata. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-005 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-006 Write Box Schema Family: define `DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, and `PromotionBox`. Acceptance: each write box has lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-006 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-007 Direct Edit Denial Path: route model/tool attempts to mutate authority artifacts through ToolGate denial or proposal wrapping. Acceptance: tests prove raw authority-file edit attempts fail with actionable denial evidence and lawful replacement action ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-007 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-008 Advisory Edit Normalization: convert manual/model edits against generated mirrors into `MirrorAdvisoryBox` records. Acceptance: advisory edits do not mutate authority until a registered normalization/promotion action validates and accepts them. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-008 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-009 No-Context Model Manual: create durable model-facing instructions for using Handshake mechanically. Acceptance: the manual explains purpose, startup, action catalog, write boxes, DCC paths, CRDT workflow, safety constraints, failure modes, denial recovery, and validation evidence for a model with no conversation history. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-009 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-010 CRDT Document Identity and Workspace Model: define document/workspace ids, actor ids, site/client ids, schema ids, and authority links. Acceptance: CRDT records can be linked to work item, action request, artifact proposal, Role Mailbox thread, DCC projection, and EventLedger ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-010 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-011 CRDT Update Persistence: persist CRDT updates in Postgres with ordering, hash, actor/session attribution, and replay metadata. Acceptance: a workspace can be reconstructed from persisted updates after restart without file-system authority assumptions. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-011 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-012 CRDT Snapshot and Compaction: add snapshot/state-vector or equivalent sync cursor support. Acceptance: update replay is bounded by snapshots, old updates remain auditable or compacted according to policy, and compaction never drops promotion evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-012 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-013 CRDT Context Slicing for Models: expose summaries, selected ranges, field digests, and operation deltas. Acceptance: model prompts can request bounded CRDT context without loading entire documents, and extract outputs cite workspace/version/source ids. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-013 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-014 CRDT Schema and Validity Guard: validate CRDT materialized state before promotion. Acceptance: structurally invalid, unauthorized, or schema-drifted CRDT state cannot be promoted into authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-014 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-015 CRDT Promotion Bridge: convert CRDT edits/drafts into ArtifactProposal and PromotionGate inputs. Acceptance: accepted promotions emit EventLedger authority events; rejected promotions keep CRDT/draft state as non-authoritative evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-015 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-016 Conflict and Presence Projection: expose presence, pending conflicts, actor attribution, and merge/proposal state. Acceptance: DCC can show who changed what, which edits are merely merged CRDT state, and which changes are pending promotion. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/crdt/**, src/backend/handshake_core/src/storage/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-016 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-017 Software-Delivery Runtime Truth Records: fold `WP-1-Software-Delivery-Runtime-Truth-v1`. Acceptance: current software-delivery posture is queryable from product-owned stable records and governed actions, not packet prose, mailbox order, or Markdown freshness. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-017 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-018 Workflow Transition Automation Registry: fold `WP-1-Workflow-Transition-Automation-Registry-v1`. Acceptance: every workflow mutation has a registered transition rule, eligible actor, action trigger, approval boundary, and DCC preview. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-018 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-019 Governance Overlay Boundary: fold `WP-1-Software-Delivery-Governance-Overlay-Boundary-v1`. Acceptance: imported repo `.GOV/**` artifacts are evidence/source overlays, not runtime truth, and import/export cannot bypass gates. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-019 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-020 Overlay Coordination Records: fold `WP-1-Software-Delivery-Overlay-Coordination-Records-v1`. Acceptance: claim/lease, queued steering, follow-up, takeover, and actor eligibility are queryable by stable ids without mailbox chronology. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-020 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-021 Overlay Lifecycle and Recovery Control Plane: fold `WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1`. Acceptance: start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture are record-backed and projection-safe. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-021 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-022 Postgres Control-Plane Residual Scope: fold `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` plus its transitive folded stubs. Acceptance: residual live Postgres service proof, leases/backpressure, ModelSession queues, FEMS memory store, durable workflow execution, DCC projections, and SQLite boundary obligations are carried into Kernel002 or explicitly mapped to Kernel003/Kernel004 without reopening the old bundle. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-022 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-023 Locus Work Tracking Reset Migration: fold `WP-1-Locus-Work-Tracking-System-Phase1-v1`. Acceptance: WP/MT tracking, dependencies, occupancy, query, Task Board projection, and Flight Recorder obligations are preserved, but SQLite authority is replaced with Postgres/EventLedger/CRDT-compatible authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-023 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-024 DCC MVP Runtime Surface: fold `WP-1-Dev-Command-Center-MVP-v1`. Acceptance: DCC can select work, view worktree/session/action/proposal state, inspect diffs/evidence, preview approvals, and trigger governed actions through the catalog. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-024 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-025 DCC Structured Artifact Viewer: fold `WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1`. Acceptance: DCC renders canonical fields before mirrors, exposes mirror state, and provides raw structured drilldown as advanced view. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-025 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-026 DCC Layout Projection Registry: fold `WP-1-Dev-Command-Center-Layout-Projection-Registry-v1`. Acceptance: board, queue, list, roadmap, inbox-triage, and execution-queue views derive from registered presets and action bindings. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-026 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-027 Role Mailbox Message and Action Request Contract: fold `WP-1-Role-Mailbox-Message-Thread-Contract-v1`. Acceptance: mailbox lifecycle, delivery state, allowed responses, due/dead-letter posture, and action requests are typed and authority-bounded. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-027 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-028 Role Mailbox Micro-Task Loop Control: fold `WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1`. Acceptance: retry budget, verifier outcome, escalation, completion report, dead-letter, and loop checkpoint state are compact and replayable. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-028 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-029 Role Mailbox Triage Queue Controls: fold `WP-1-Role-Mailbox-Triage-Queue-Controls-v1`. Acceptance: reminder, snooze, expiry, dead-letter, retry/reroute/archive, and Task Board pressure overlays are field-backed projections. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-029 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-030 Role Mailbox Claim and Lease: fold `WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1`. Acceptance: claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility are explicit and queryable. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-030 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-031 Role Mailbox Handoff and Announce-Back: fold `WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1`. Acceptance: handoff bundles, transcription targets, recommended next actor, announce-back provenance, and advisory/completion distinction are typed. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-031 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-032 Role Mailbox Inbox Alignment and Evidence Bridge: fold `WP-1-Inbox-Role-Mailbox-Alignment-v1` and `WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1`. Acceptance: Inbox labels map to Role Mailbox only, mailbox telemetry is leak-safe, and debug bundle exports preserve stable evidence/provenance. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-032 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-033 FEMS Working-Memory Checkpoints: fold `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`. Acceptance: SESSION_OPEN, PRE_TASK, INSIGHT, TASK_COMPLETE, SESSION_CLOSE, memory extract, repeated insight promotion, and GC are typed and quality-gated. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-033 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-034 FEMS Write-Time Safeguards: fold `WP-1-FEMS-Write-Time-Safeguards-v1`. Acceptance: novelty scoring, supersession, contradiction detection, dedup, state validation, and audit trail run mechanically; SQLite/FTS5 references are reworked to reset-approved storage/search primitives. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-034 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-035 FEMS Memory Poisoning and Drift Guardrails: fold `WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1`. Acceptance: trust gates, pack budget, deterministic reduction, proposal/approval/denial events, and effective pack hashes prevent untrusted long-lived memory drift. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-035 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-036 FEMS MT Handoff Memory Context: fold `WP-1-FEMS-MT-Handoff-Memory-Context-v1`. Acceptance: escalated or handed-off MTs carry typed memory context with source/target sessions, failed attempts, recommended items, provenance, and bounded scoring. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/role_mailbox/**, src/backend/handshake_core/src/fems/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_collaboration_memory --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-036 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-037 Role Turn Isolation: fold `WP-1-Role-Turn-Isolation-v1`. Acceptance: role turns default to isolated context, replay pins are recorded, and cross-role bleed is mechanically prevented. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-037 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-038 Work Profiles: fold `WP-1-Work-Profiles-v1`. Acceptance: profile storage, selection, immutable profile ids, per-role routing, autonomy knobs, and profile receipts are wired into action requests. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-038 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-039 Local-First Agentic MCP Posture: fold `WP-1-LocalFirst-Agentic-MCP-Posture-v1`. Acceptance: local-first execution remains default; MCP/cloud paths are capability-gated adapters with cached artifacts and fallback behavior. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-039 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-040 Git Engine Decision Gate: fold `WP-1-Git-Engine-Decision-Gate-v1`. Acceptance: one repo engine path is recorded/enforced, dangerous git actions remain gated, and DCC/action catalog expose only lawful git affordances. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-040 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-041 Session Anti-Pattern Registry: fold `WP-1-Session-Anti-Pattern-Registry-v1`. Acceptance: scheduler/trust/capability/session orchestration anti-patterns have machine-readable detections and deny/downgrade/consent/stop outcomes. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-041 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-042 Governance Pack Instantiation: fold `WP-1-Governance-Pack-v1`. Acceptance: project identity, pack manifest, instantiation, naming/path policy, conformance harness, and imported-overlay boundaries are compatible with Kernel002 action/write-box law. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-042 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-043 Session Spawn Tree DCC Visualization: fold `WP-1-Session-Spawn-Tree-DCC-Visualization-v1`. Acceptance: DCC shows spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from runtime records. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-043 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-044 Session Spawn Conversation Distillation: fold `WP-1-Session-Spawn-Conversation-Distillation-v1`. Acceptance: parent-child request/summary pairs and spawn metadata feed distillation artifacts without making conversation text authority. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-044 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-045 Product Screenshot Capture: fold `WP-1-Product-Screenshot-Visual-Validation-v1`. Acceptance: governed sessions can capture full app, panel, and module screenshots with metadata and artifact refs. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-045 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-046 Visual Debugging Loop: fold `WP-1-Visual-Debugging-Loop-v1`. Acceptance: post-commit or post-action screenshot capture, baseline comparison, visual evidence storage, threshold config, and validator steering are available for GUI work. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: app/**, src/backend/handshake_core/src/kernel/**, tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_dcc --target-dir ../Handshake_Artifacts/handshake-cargo-target; visual DCC screenshot/debug check when UI exists | RISK_IF_MISSED: MT-046 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-047 Markdown Mirror Sync Drift Guard: fold `WP-1-Markdown-Mirror-Sync-Drift-Guard-v1`. Acceptance: deterministic mirror regeneration, drift states, manual advisory handling, reconciliation, DCC mirror queue, and projection banners are implemented. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-047 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-048 Direct-Edit Regression Harness: prove future models cannot bypass write boxes through common edit paths. Acceptance: tests simulate model raw patch, generated file write, mirror edit, CRDT edit, mailbox reply, DCC quick action, and git action; each path either uses registered action/write box or fails with evidence. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-048 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-049 Projection Rebuild and Task Board Sync: regenerate projections and sync Task Board, traceability registry, build order, and stub contracts. Acceptance: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` pass or produce a concrete blocker. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-049 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-050 Pre-Use Kernel Acceptance Run: prove Kernel001 + Kernel002 are usable before real kernel operation. Acceptance: a no-context model follows the manual to draft in CRDT, submit a proposal, trigger validation, receive a promotion/denial, view DCC projections, and inspect evidence without direct authority-file edits. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-050 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-051 Stub, Work Packet, and Microtask Contract Lifecycle: define the machine-readable lifecycle from inactive stub to active work packet to generated microtask contracts. Acceptance: `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` schemas define states, authority rules, required fields, provenance hashes, source imports, lifecycle transitions, receipt events, projection hooks, validation hooks, and failure states. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-051 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-052 Work Packet Full-Detail Authority and Microtask Source Plan: ensure the activated work packet itself carries full implementation detail while also containing a structured MT source plan. Acceptance: a no-context strong model can execute from the work packet alone; the same packet can regenerate MT contracts/files without relying on manually maintained sidecars or hidden chat context. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-052 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-053 Mechanical Stub Promotion and Microtask Extraction: implement deterministic commands or action-catalog entries for stub-to-WP promotion and WP-to-MT extraction. Acceptance: promotion/extraction preserves operator intent, source hashes, folded details, dependencies, constraints, acceptance criteria, verification, and status provenance; every generated artifact records its source contract id and hash. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-053 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-054 Local-Model Fresh-Context Microtask Loop Contract: define the Locus-compatible execution loop for smaller/local models working one MT at a time. Acceptance: the loop contract covers fresh-context input bundle, allowed actions, write boxes, retry budget, verifier handoff, failure requeue, memory checkpoint input, receipt emission, and final MT outcome without requiring the model to inspect unrelated WP scope. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-054 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-055 Generated Documentation and Status Projection: replace manual status/docs maintenance with projections from contracts, receipts, runtime state, and validation outputs. Acceptance: packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries regenerate from machine-readable authority; direct manual status edits are denied or captured as advisory normalization input. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-055 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-056 Coder Handoff and Validation Request Contract: define the structured handoff from coder execution to Handshake-owned validation. Acceptance: `CoderHandoffContractV1` records MT id, parent WP id, actor/session, claimed scope, touched files/actions, receipts, tests, evidence, known blockers, and requested review; Handshake can generate a validator review request from it without a model editing status fields. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-056 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-057 Validator Verdict and Mediation Contract: define structured pass/fail/mediation verdicts from WP Validators. Acceptance: `ValidatorVerdictContractV1` and `MediationInstructionContractV1` encode verdict, failed acceptance criteria, evidence refs, severity, reproducibility, exact remediation instructions, dependency impact, and whether the MT may advance, must loop back, or must escalate. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-057 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports: define machine-readable reports for validator findings that are not simple pass/fail. Acceptance: `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` preserve validator reasoning, source refs, affected surfaces, reproduction or proof, proposed destination, and routing outcome without becoming manual prose-only reports. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-058 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-059 Remediation Microtask and Packet Generation: generate follow-up work from failed verdicts and reports. Acceptance: Handshake can create `RemediationMicroTaskContractV1` or a remediation packet/stub from verdict/report contracts, preserving parent WP/MT links, dependency state, acceptance criteria, allowed actions, write boxes, evidence refs, retry budget, and validator recheck requirements. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-059 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-060 Loop Scheduler and Next-Coder Dispatch: define the mechanical loop that dispatches coders after validation outcomes. Acceptance: Handshake only dispatches a new coder when leases, current coder completion, dependency state, retry budget, and verdict state allow it; failed prerequisites loop to remediation before dependent MTs can advance. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-060 scope is lost and Kernel002 can reintroduce drift or direct mutation.
  - CLAUSE: MT-061 Locus Work Graph Projection for MT Validation Loops: connect the validation/remediation loop to Locus work tracking semantics from the Master Spec. Acceptance: Locus can project MT nodes, validator verdicts, remediation edges, blocked/escalated states, actor leases, and pass/fail history without treating prose reports or chat messages as truth. | WHY_IN_SCOPE: preserved Kernel002 microtask | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/**, src/backend/handshake_core/src/kernel/**, src/backend/handshake_core/tests/** | EXPECTED_TESTS: cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target | RISK_IF_MISSED: MT-061 scope is lost and Kernel002 can reintroduce drift or direct mutation.

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: KernelActionCatalogV1 | PRODUCER: kernel action registry | CONSUMER: ToolGate, DCC, model manual, Integration Validator | SERIALIZER_TRANSPORT: JSON schema over product authority records | VALIDATOR_READER: catalog tests and projection checks | TRIPWIRE_TESTS: each action has ids, schemas, actors, write boxes, hooks, receipts | DRIFT_RISK: undocumented action path bypasses law
  - CONTRACT: WriteBoxV1 family | PRODUCER: CRDT, advisory, patch, artifact, memory, execution, promotion surfaces | CONSUMER: PromotionGate, validators, DCC, Locus | SERIALIZER_TRANSPORT: typed JSON plus artifact hashes | VALIDATOR_READER: write-box transition tests | TRIPWIRE_TESTS: every box records state, owner, authority effect, evidence, validation, projection | DRIFT_RISK: draft/advisory state becomes authority
  - CONTRACT: CRDT workspace records | PRODUCER: CRDT storage layer | CONSUMER: context slicer, promotion bridge, DCC, replay tests | SERIALIZER_TRANSPORT: Postgres update stream and snapshots | VALIDATOR_READER: restart replay and compaction tests | TRIPWIRE_TESTS: reconstruct workspace after restart without SQLite/file authority | DRIFT_RISK: CRDT state is non-replayable
  - CONTRACT: WorkPacketContractV1 and MicroTaskContractV1 | PRODUCER: work graph generator | CONSUMER: Locus, DCC, packet projections, MT loop | SERIALIZER_TRANSPORT: JSON contracts with generated Markdown projections | VALIDATOR_READER: contract import and projection drift checks | TRIPWIRE_TESTS: 61 MT contracts regenerate one-to-one | DRIFT_RISK: packet detail collapses into stale prose
  - CONTRACT: CoderHandoffContractV1 and ValidatorVerdictContractV1 | PRODUCER: Kernel Builder and Integration Validator | CONSUMER: loop scheduler, Locus, DCC | SERIALIZER_TRANSPORT: JSON receipts and review records | VALIDATOR_READER: handoff/verdict tests | TRIPWIRE_TESTS: failed MT creates remediation or block before dependency advances | DRIFT_RISK: validator prose manually drives status
  - CONTRACT: DCC projection and visual evidence payloads | PRODUCER: projection registry and visual debug loop | CONSUMER: operator, models, Integration Validator | SERIALIZER_TRANSPORT: structured projection JSON plus screenshot refs | VALIDATOR_READER: visual-debug checks | TRIPWIRE_TESTS: rows expose stable ids, stale badges, and promotion previews | DRIFT_RISK: UI hides authority or stale state

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Implement MT-001 through MT-061 back to back in declared order unless dependency proof requires local reordering without dropping any MT.
- HOT_FILES:
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/src/storage/**
  - src/backend/handshake_core/src/role_mailbox/**
  - src/backend/handshake_core/src/fems/**
  - src/backend/handshake_core/src/locus/**
  - src/backend/handshake_core/tests/**
  - app/**
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_write_box --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
- CARRY_FORWARD_WARNINGS:
  - Do not condense or remove any MT.
  - Do not introduce WP Validator gate.
  - Do not treat Kernel Builder checks as validation.
  - Do not use SQLite authority or prose/mirror authority.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - All 61 MT clauses and contracts.
  - CRDT is non-authority until promotion.
  - Action catalog/write boxes deny or normalize direct edits.
  - No WP Validator gate; Integration Validator batch/spec review is separate.
- FILES_TO_READ:
  - .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json
  - .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-*.json
  - .GOV/spec/SPEC_CURRENT.md
  - src/backend/handshake_core/src/kernel/**
  - src/backend/handshake_core/tests/**
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_crdt --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - cargo test -p handshake_core kernel_mt_loop --target-dir ../Handshake_Artifacts/handshake-cargo-target
  - just gov-check
  - just spec-eof-appendices-check
- POST_MERGE_SPOTCHECKS:
  - No-context manual path, direct-edit denial harness, CRDT promotion/restart proof, and visual DCC evidence.

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - Product implementation has not started.
  - MT-003 ADR has not selected CRDT library.
  - Integration Validator verdict does not exist yet.
  - DCC screenshot evidence waits for GUI implementation.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: v02.185 names CRDT workspace, KernelActionCatalogV1, WriteBoxV1, direct-edit denial, promotion bridge, DCC projections, feature, primitives, and matrix edges; the stub supplies 61 measurable MT clauses.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Active v02.185 resolves the topical authority gap before packet activation.
- PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates):
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#2.3.13.10
- CONTEXT_START_LINE: 3708
- CONTEXT_END_LINE: 3720
- CONTEXT_TOKEN: Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
  The Kernel V1 EventLedger is the product authority for the first kernel slice. Postgres notifications, process-local locks, UI state, and provider or framework traces MAY accelerate or display work, but recovery and validation MUST be possible from durable product rows alone.
  
  #### 2.3.13.10 Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]
  
  Kernel V1 CRDT workspace state is pre-promotion working state. It MAY hold concurrent human and model drafts, advisory edits, review notes, and normalized proposed operations, but it is not authority until promotion commits through the Postgres EventLedger defined in Section 2.3.13.9.
  
  The Kernel V1 implementation MUST provide a KernelActionCatalogV1 contract that enumerates every write-capable kernel action before it can mutate a draft or request promotion. Each catalog entry MUST declare a stable action id, target authority class, input schema version, actor eligibility, required capability or approval posture, preview behavior, validation checks, idempotency key policy, and resulting event or receipt type. Ad hoc direct writes that do not resolve through this catalog are forbidden.
  
  The Kernel V1 implementation MUST provide a WriteBoxV1 family for draft mutations. A write box MUST carry a stable write_box_id, workspace_id, actor_id, actor kind, CRDT site id, target record refs, base snapshot or state vector refs, intent summary, operation payload refs, schema version, validation state, denial or promotion receipt refs, and replay metadata. Write boxes MAY normalize advisory text, diffs, CRDT transactions, or model proposals into a common envelope, but they MUST preserve actor provenance and source evidence.
  
  Direct edits to authoritative Kernel V1 records MUST be denied unless they enter through an allowed catalog action and write box path. Denials MUST produce durable WriteBoxDirectEditDeniedV1 evidence with actor, target, attempted action, denial reason, recovery instruction, and linked UI or API response. Denial handling is part of product behavior, not only a validation test.
  
  The CRDT-to-EventLedger promotion bridge MUST be explicit. Promotion MUST read a validated write box, confirm actor eligibility and target authority class, verify schema and CRDT state-vector freshness, reject stale or duplicate promotion requests by idempotency key, and append promotion-request, promotion-accepted, or promotion-rejected events to the Postgres EventLedger. A CRDT merge MUST NOT directly mutate EventLedger authority; it can only make a promotion candidate visible for validation.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#KernelActionCatalogV1
- CONTEXT_START_LINE: 65
- CONTEXT_END_LINE: 77
- CONTEXT_TOKEN: KernelActionCatalogV1
- EXCERPT_ASCII_ESCAPED:
  ```text
  | Version | Date | Author | Changes | Approval |
  |---------|------|--------|---------|----------|
  | v02.185 | 2026-05-14 | Kernel Builder | Added Kernel002 authority law: KernelActionCatalogV1, WriteBoxV1, direct-edit denial, advisory edit normalization, CRDT workspace draft persistence, and CRDT-to-EventLedger promotion bridge with DCC projection requirements. [ADD v02.185] | ilja140520260455 |
  | v02.184 | 2026-05-13 | Kernel Builder | Added Kernel V1 authority law: Postgres EventLedger as product runtime authority, SessionBroker/ContextBundle/ModelAdapter boundary, no-SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostics posture for kernel replay and promotion. [ADD v02.184] | Operator approval in chat, 2026-05-13; WP activation signature pending |
  | v02.183 | 2026-05-13 | Orchestrator | Migrated the active indexed Master Spec into the copy-first versioned bundle `.GOV/spec/master-spec-v02.183/`, moved the previous indexed bundle to `.GOV/spec/spec_archive/master-spec-v02.182/`, added uniform module `spec_version` metadata, added a manifest-declared machine-readable changelog module, updated `.GOV/spec/SPEC_CURRENT.md`, and refreshed internal references away from latest-monolith/version-file wording. [ADD v02.183] | Operator approval in chat, 2026-05-13 |
  | v02.182 | 2026-05-05 | Activation Manager | Added PostgreSQL-primary control-plane foundation law: explicit storage modes, PostgreSQL-authoritative self-hosted runtime records, fail-closed behavior when PostgreSQL is required, SQLite cache/offline boundaries, downstream split for queue workers, leases/backpressure, FEMS memory store, workflow durable execution, DCC projections, SQLite fallback boundaries, and developer/test container setup; updated Appendix 12 feature, primitive, and interaction metadata for the pivot. [ADD v02.182] | APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 |
  | v02.181 | 2026-04-17 | Orchestrator | Added software-delivery governance overlay law: product-owned runtime truth over imported repo `/.GOV/**`, validator-gate convergence on top of Governance Check Runner, projection-only Dev Command Center / Task Board / Role Mailbox posture, derived closeout semantics, overlay claim/lease and queued-instruction extension records, explicit overlay lifecycle constraints, and workflow-backed start/steer/cancel/close/recover control-plane law; updated Appendix 12 / roadmap follow-through for the affected feature families. [ADD v02.181] | pending |
  | v02.180 | 2026-04-07 | Orchestrator | Added 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) -- typed CheckResult contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED), tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, additive overlay rule. [ADD v02.180] | pending |
  | v02.179 | 2026-03-28 | Orchestrator | **Workflow-correlation bundle-scope pass:** patched Debug Bundle export law so `workflow_run` and `workflow_node_execution` become first-class bounded scopes, added workflow-node execution inventory plus manifest-count rules, extended exporter and exportable-inventory posture, deepened FEAT-DEBUG-BUNDLE UI guidance for workflow-scoped export, and kept roadmap/cov-matrix scheduling aligned with the existing Workflow Projection Correlation backlog. | ilja280320262308 |
  | v02.178 | 2026-03-11 | Orchestrator | **RAG mode and no-RAG cross-pillar pass:** clarified that RAG is one governed retrieval mode rather than the default context strategy; added retrieval-mode and non-hybrid-reason law across AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for authoritative direct-load, graph-first, and bounded local-model retrieval posture; and materialized a dedicated retrieval-mode policy stub. | ilja110320261228 |
  | v02.177 | 2026-03-11 | Orchestrator | **Role Mailbox handoff-bundle and announce-back provenance pass:** defined structured handoff bundles, announce-back provenance, note-transcription duties, and compact handoff summaries across Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for durable handoffs; and materialized a dedicated mailbox handoff/transcription/announce-back stub. | ilja110320260813 |
  | v02.176 | 2026-03-11 | Orchestrator | **Role Mailbox executor-routing and claim-lease pass:** defined mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Work Packet System, and Task Board; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for claimant-aware parallel work; and materialized a dedicated mailbox executor-routing and claim-lease stub. | ilja110320260021 |
  | v02.175 | 2026-03-11 | Orchestrator | **Role Mailbox triage and queue-control pass:** defined mailbox triage queue state, reminder schedules, snooze and expiry posture, dead-letter remediation, and operator-facing remediation controls across Role Mailbox, Dev Command Center, Task Board, Work Packet System, and Locus Work Tracking; deepened Appendix 12 ownership, coverage, UI guidance, and interaction edges for queue aging and recovery; and materialized a dedicated mailbox-triage-and-queue-controls stub. | ilja110320260002 |
  ```

#### ANCHOR 3
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/02-system-architecture.md#WriteBoxV1
- CONTEXT_START_LINE: 65
- CONTEXT_END_LINE: 77
- CONTEXT_TOKEN: WriteBoxV1
- EXCERPT_ASCII_ESCAPED:
  ```text
  | Version | Date | Author | Changes | Approval |
  |---------|------|--------|---------|----------|
  | v02.185 | 2026-05-14 | Kernel Builder | Added Kernel002 authority law: KernelActionCatalogV1, WriteBoxV1, direct-edit denial, advisory edit normalization, CRDT workspace draft persistence, and CRDT-to-EventLedger promotion bridge with DCC projection requirements. [ADD v02.185] | ilja140520260455 |
  | v02.184 | 2026-05-13 | Kernel Builder | Added Kernel V1 authority law: Postgres EventLedger as product runtime authority, SessionBroker/ContextBundle/ModelAdapter boundary, no-SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostics posture for kernel replay and promotion. [ADD v02.184] | Operator approval in chat, 2026-05-13; WP activation signature pending |
  | v02.183 | 2026-05-13 | Orchestrator | Migrated the active indexed Master Spec into the copy-first versioned bundle `.GOV/spec/master-spec-v02.183/`, moved the previous indexed bundle to `.GOV/spec/spec_archive/master-spec-v02.182/`, added uniform module `spec_version` metadata, added a manifest-declared machine-readable changelog module, updated `.GOV/spec/SPEC_CURRENT.md`, and refreshed internal references away from latest-monolith/version-file wording. [ADD v02.183] | Operator approval in chat, 2026-05-13 |
  | v02.182 | 2026-05-05 | Activation Manager | Added PostgreSQL-primary control-plane foundation law: explicit storage modes, PostgreSQL-authoritative self-hosted runtime records, fail-closed behavior when PostgreSQL is required, SQLite cache/offline boundaries, downstream split for queue workers, leases/backpressure, FEMS memory store, workflow durable execution, DCC projections, SQLite fallback boundaries, and developer/test container setup; updated Appendix 12 feature, primitive, and interaction metadata for the pivot. [ADD v02.182] | APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 |
  | v02.181 | 2026-04-17 | Orchestrator | Added software-delivery governance overlay law: product-owned runtime truth over imported repo `/.GOV/**`, validator-gate convergence on top of Governance Check Runner, projection-only Dev Command Center / Task Board / Role Mailbox posture, derived closeout semantics, overlay claim/lease and queued-instruction extension records, explicit overlay lifecycle constraints, and workflow-backed start/steer/cancel/close/recover control-plane law; updated Appendix 12 / roadmap follow-through for the affected feature families. [ADD v02.181] | pending |
  | v02.180 | 2026-04-07 | Orchestrator | Added 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) -- typed CheckResult contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED), tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, additive overlay rule. [ADD v02.180] | pending |
  | v02.179 | 2026-03-28 | Orchestrator | **Workflow-correlation bundle-scope pass:** patched Debug Bundle export law so `workflow_run` and `workflow_node_execution` become first-class bounded scopes, added workflow-node execution inventory plus manifest-count rules, extended exporter and exportable-inventory posture, deepened FEAT-DEBUG-BUNDLE UI guidance for workflow-scoped export, and kept roadmap/cov-matrix scheduling aligned with the existing Workflow Projection Correlation backlog. | ilja280320262308 |
  | v02.178 | 2026-03-11 | Orchestrator | **RAG mode and no-RAG cross-pillar pass:** clarified that RAG is one governed retrieval mode rather than the default context strategy; added retrieval-mode and non-hybrid-reason law across AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for authoritative direct-load, graph-first, and bounded local-model retrieval posture; and materialized a dedicated retrieval-mode policy stub. | ilja110320261228 |
  | v02.177 | 2026-03-11 | Orchestrator | **Role Mailbox handoff-bundle and announce-back provenance pass:** defined structured handoff bundles, announce-back provenance, note-transcription duties, and compact handoff summaries across Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for durable handoffs; and materialized a dedicated mailbox handoff/transcription/announce-back stub. | ilja110320260813 |
  | v02.176 | 2026-03-11 | Orchestrator | **Role Mailbox executor-routing and claim-lease pass:** defined mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Work Packet System, and Task Board; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for claimant-aware parallel work; and materialized a dedicated mailbox executor-routing and claim-lease stub. | ilja110320260021 |
  | v02.175 | 2026-03-11 | Orchestrator | **Role Mailbox triage and queue-control pass:** defined mailbox triage queue state, reminder schedules, snooze and expiry posture, dead-letter remediation, and operator-facing remediation controls across Role Mailbox, Dev Command Center, Task Board, Work Packet System, and Locus Work Tracking; deepened Appendix 12 ownership, coverage, UI guidance, and interaction edges for queue aging and recovery; and materialized a dedicated mailbox-triage-and-queue-controls stub. | ilja110320260002 |
  ```

#### ANCHOR 4
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/03-local-first-infrastructure.md#kernel-v1-crdt-workspace-addendum
- CONTEXT_START_LINE: 19876
- CONTEXT_END_LINE: 19888
- CONTEXT_TOKEN: Kernel V1 CRDT workspace addendum [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
  Kernel V1 is excluded from the SQLite local-first recommendation: its first authority path is Postgres EventLedger only.
  
  Kernel V1 CRDT workspace addendum [ADD v02.185]: Kernel V1 may use CRDT updates, snapshots, and state vectors for draft workspace collaboration, but those records remain pre-promotion working state. PostgreSQL remains mandatory for EventLedger authority, promotion receipts, replay, and validation. CRDT storage MUST be restart-replayable, snapshot-safe, and joinable to write-box and promotion ids; it MUST NOT become a hidden SQLite authority path.
  
  
  ---
  
  ## 3.4 Conflict Resolution UX
  
  **Why**  
  Even with CRDTs, users sometimes need to understand what changed. Good conflict UX builds trust.
  
  **What**  
  ```

#### ANCHOR 5
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#10.11.5.28
- CONTEXT_START_LINE: 61701
- CONTEXT_END_LINE: 61713
- CONTEXT_TOKEN: Kernel Action Catalog and Write Box Projections [ADD v02.185]
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Bulk handoff or announce-back actions MUST preview whether they only normalize mailbox summaries or also request governed note transcription or linked work mutation.
  
  ### 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]
  
  The Dev Command Center MUST expose Kernel V1 action-catalog and write-box state as typed product projections, not as raw transcript or repo-governance mirrors.
  
  **Required projections**
  - Action catalog viewer: list KernelActionCatalogV1 entries by stable action id, target authority class, input schema version, actor eligibility, approval or capability requirements, preview behavior, and allowed output receipt types.
  - Write box queue: show draft write boxes by write_box_id, actor, CRDT site id, target refs, validation state, stale-state-vector posture, denial receipt, promotion receipt, and linked EventLedger events when promoted.
  - Direct-edit denial view: show attempted actor, target, action, denial reason, recovery instruction, and whether the blocked edit can be normalized into an advisory write box.
  - Promotion preview: before promotion, show affected target refs, current state vector, validation checks, idempotency key, expected EventLedger event types, and stale or duplicate risk.
  - Projection freshness badges: distinguish live CRDT draft state, compacted snapshot state, pending promotion, accepted promotion, rejected promotion, and stale projection.
  
  ```

#### ANCHOR 6
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-KERNEL-WORKSPACE-WRITE-BOX
- CONTEXT_START_LINE: 73924
- CONTEXT_END_LINE: 73936
- CONTEXT_TOKEN: FEAT-KERNEL-WORKSPACE-WRITE-BOX
- EXCERPT_ASCII_ESCAPED:
  ```text
      },
      {
        "feature_id": "FEAT-KERNEL-WORKSPACE-WRITE-BOX",
        "title": "Kernel Workspace Write Box and Promotion Bridge",
        "spec_anchor": "#231310-kernel-v1-crdt-workspace-write-box-and-promotion-bridge-add-v02185",
        "surfaces": [
          "backend",
          "ui"
        ],
        "primitives": [
          "PRIM-KernelActionCatalogV1",
          "PRIM-KernelActionDescriptorV1",
          "PRIM-WriteBoxV1",
  ```

### FEATURE_DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: v02.185 active primitives: PRIM-KernelActionCatalogV1, PRIM-KernelActionDescriptorV1, PRIM-WriteBoxV1, PRIM-WriteBoxDirectEditDeniedV1, PRIM-WriteBoxPromotionRequestV1, PRIM-WriteBoxPromotionReceiptV1, PRIM-CrdtWorkspaceDraftV1, PRIM-CrdtWorkspaceSnapshotV1
- DISCOVERY_STUBS: NONE_CREATED - existing Kernel002 stub promoted.
- DISCOVERY_MATRIX_EDGES: IMX-136, IMX-137 already applied in v02.185.
- DISCOVERY_UI_CONTROLS: action catalog viewer, write-box queue, denial recovery panel, promotion preview, stale badges, visual evidence controls.
- DISCOVERY_SPEC_ENRICHMENT: YES - v02.185 was applied before activation.
- DISCOVERY_JUSTIFICATION: Concrete primitive, matrix, and UI discoveries were folded into v02.185 and preserved here.
