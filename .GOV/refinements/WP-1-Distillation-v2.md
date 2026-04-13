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
- WP_ID: WP-1-Distillation-v2
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-13T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: PENDING
- USER_APPROVAL_EVIDENCE: PENDING
- STUB_WP_IDS: WP-1-MTE-LoRA-Wiring-v1, WP-1-Session-Spawn-Conversation-Distillation-v1

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- SQL schema for Skill Bank tables (skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run) is 100% missing; no migration exists.
- Core data model structs (SkillBankLogEntry, DistillJob, AdapterCheckpoint, EvalRun, QualityMeta, TelemetryMeta, PrivacyMeta) are not implemented; only in-memory PendingDistillationCandidate and DistillationInfo exist in workflows.rs.
- Algorithms (compute_data_trust_score, build_distill_dataset, run_distill_job, evaluate_and_maybe_promote) are entirely absent.
- PII/secret redaction API (redact_entry) is not implemented.
- Capability gates for distillation export/training are not defined in capabilities.rs.
- Distillation observability events beyond FR-EVT-MT-015 (candidate capture) are not wired; stage-level events (select, teacher, student, score, checkpoint, eval, promote) are missing.
- Context Pack hash and PromptEnvelope hash tracking in distillation observability (v02.157 requirement) is absent.
- JobKind::DistillationEval exists as an enum variant but has no execution handler.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 0m (not applicable; internal spec-grounded WP)
- SEARCH_SCOPE: NOT_APPLICABLE; WP implements existing Master Spec Section 9 requirements using local product/runtime code
- REFERENCES: NONE; the Master Spec already encodes distillation research outcomes (LoRA/QLoRA/DoRA posture, teacher/student lineage, collapse indicators, data trust scoring) from v02.115 through v02.157
- PATTERNS_EXTRACTED: NONE
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the spec Section 9 data model, SQL schema, and algorithm definitions as the implementation blueprint; ADAPT the existing ephemeral DistillationCandidate and PendingDistillationCandidate structs into persistent SkillBankLogEntry with full lineage; REJECT external research patterns since the spec already encodes distillation research outcomes through v02.157
- LICENSE/IP_NOTES: NONE; no external code reuse
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Spec Section 9 comprehensively defines all data models, SQL schemas, algorithms, observability requirements, and integration contracts. No Main Body or appendix changes required.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP implements existing Master Spec Section 9 requirements. The spec already encodes distillation research outcomes (LoRA/QLoRA/DoRA posture, teacher/student lineage, data trust scoring, collapse indicators, cross-tokenizer safety) accumulated through spec passes v02.115 to v02.157. No external research would improve implementation beyond spec compliance.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE

### RESEARCH_DEPTH (prevent shallow source logging)
- ADOPT_PATTERNS: N/A
- ADAPT_PATTERNS: N/A
- REJECT_PATTERNS: N/A
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-MT-015: MicroTaskDistillationCandidate (exists in code; extend with persistent storage write)
- FR-EVT-DISTILL-SELECT: Distillation candidate selection/filtering stage
- FR-EVT-DISTILL-TEACHER: Teacher model run with prompt snapshot and output refs
- FR-EVT-DISTILL-STUDENT: Student model run with prompt snapshot and output refs
- FR-EVT-DISTILL-SCORE: Data trust score computation result
- FR-EVT-DISTILL-CHECKPOINT: Adapter checkpoint creation with lineage (parent_checkpoint_id)
- FR-EVT-DISTILL-EVAL: Evaluation run result with pass@k, compile/test rates, collapse indicators
- FR-EVT-DISTILL-PROMOTE: Promotion decision (promote/rollback) with benchmark comparison
- All distillation FR events MUST include: model/tokenizer ids, inference params, context refs, metrics, reward features, lineage, data_signature, job_ids_json (per Section 5.3.6)
- v02.157 addition: Context Pack hashes/freshness decisions and PromptEnvelope hashes MUST be recorded when Context Packs or Spec Router artifacts shape teacher/student inputs

### RED_TEAM_ADVISORY (security failure modes)
- Model collapse: self-distilled data can dominate trusted traces if data_trust_score weighting is bypassed or poorly calibrated; mitigate with collapse indicators in eval gating and minimum trusted-source ratio enforcement
- PII/secret leakage: training data assembled from user sessions can contain sensitive content; mitigate with log-time redaction (redact_entry) and pre-training scrubbing pipeline; enforce capability-based export controls
- Cross-tokenizer corruption: treating token ids as interchangeable between teacher and student models with different tokenizers silently corrupts training data; mitigate by requiring tokenizer metadata in DistillationCandidate and validating tokenizer compatibility at dataset assembly
- Adapter promotion without eval gates: promoting an adapter checkpoint that only looks good on small test sets can introduce silent regressions; mitigate with benchmark-gated promotion requiring pass@k, compile/test rates, and comparison against teacher plus previous checkpoint
- Data poisoning: adversarial or low-quality training examples can degrade adapter quality; mitigate with data trust scoring, quality meta filtering, and bounded candidate queues with recorder visibility
- Export control bypass: local-only checkpoints must not leak off-device; enforce capability gates on export/download of Skill Bank artifacts

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-DistillationCandidate
  - PRIM-PendingDistillationCandidate
  - PRIM-FlightEvent
  - PRIM-JobKind
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DataQualityMetrics
  - PRIM-RedactionMode
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DistillationCandidate
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DistillationCandidate
  - PRIM-DataQualityMetrics
- NOTES:
  - PRIM-DistillationCandidate: extend from ephemeral in-memory struct to durable Skill Bank artifact with persistent storage
  - PRIM-PendingDistillationCandidate: extend pending queue with recorder-visible persistence per v02.157
  - PRIM-FlightEvent: extend with distillation stage events (FR-EVT-DISTILL-*)
  - PRIM-JobKind: add DistillationEval execution handler
  - PRIM-SkillBankLogEntry: implement full 52-column durable log entry per spec Section 9.1.2
  - PRIM-AdapterCheckpoint: implement lineage-tracked checkpoint with parent chain
  - PRIM-DataQualityMetrics: implement data_trust_score computation per spec Section 9.1.3.1
  - PRIM-RedactionMode: implement redact_entry for PII/secret scrubbing per spec Section 9
  - Implementation-only structures (DistillJob, DistillExample, EvalRun, QualityMeta, PrivacyMeta, DataTrustScore) will be introduced as Rust structs but are not yet registered in Appendix 12.4; post-implementation index update will add them

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: All primitives are already defined in spec Section 9 (v02.115 through v02.180); Appendix 12.4 tracking entries will be added mechanically post-implementation and do not constitute new spec content requiring a version bump
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Post-implementation: add PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint, PRIM-DataQualityMetrics, PRIM-RedactionMode tracking rows
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: All primitives are directly anchored to spec Section 9 data model; no orphans discovered.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Skill Bank distillation features already registered in Feature Registry via v02.115 and v02.157 spec passes
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: This WP is backend-only; no UI surfaces are created or modified
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: Matrix edges for Skill Bank to Flight Recorder, Context Packs, MTE, and Storage Portability were added in v02.157 spec pass; no new edges required
- APPENDIX_MAINTENANCE_NOTES:
  - Primitive index update (Appendix 12.4) will be performed by the coder as part of implementation; this does not constitute a spec version bump since it updates an implementation-tracking appendix
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: No spatial/3D interaction in distillation pipeline | STUB_WP_IDS: NONE
- ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: No mechanical workflow authoring in scope | STUB_WP_IDS: NONE
- ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: No physics simulation in scope | STUB_WP_IDS: NONE
- ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: No simulation in scope | STUB_WP_IDS: NONE
- ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: No hardware interaction in scope | STUB_WP_IDS: NONE
- ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: No multi-agent orchestration in scope | STUB_WP_IDS: NONE
- ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: No content composition in scope | STUB_WP_IDS: NONE
- ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: No visual generation in scope | STUB_WP_IDS: NONE
- ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: No publishing in scope | STUB_WP_IDS: NONE
- ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: No recipe/culinary in scope | STUB_WP_IDS: NONE
- ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: No food safety in scope | STUB_WP_IDS: NONE
- ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: No logistics in scope | STUB_WP_IDS: NONE
- ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: No archival in scope | STUB_WP_IDS: NONE
- ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: No library/catalog in scope | STUB_WP_IDS: NONE
- ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: TOUCHED | NOTES: Eval gating and data trust score computation produce analytical metrics consumed by distillation pipeline | STUB_WP_IDS: NONE
- ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: No data wrangling UI in scope | STUB_WP_IDS: NONE
- ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: New SQL schema (6 tables, 1 view) for Skill Bank storage; storage trait extensions for distillation queries | STUB_WP_IDS: NONE
- ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: Capability gates for distillation export controls; PII/secret redaction enforcement | STUB_WP_IDS: NONE
- ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: No user guidance in scope | STUB_WP_IDS: NONE
- ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: Context Pack hash and PromptEnvelope hash tracking in distillation observability (v02.157 requirement) | STUB_WP_IDS: NONE
- ENGINE: Version | ENGINE_ID: engine.version | STATUS: NOT_TOUCHED | NOTES: No versioning logic in scope | STUB_WP_IDS: NONE
- ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: No sandbox execution in scope | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Distillation stage events (FR-EVT-DISTILL-*) emitted for full pipeline observability; Context Pack and PromptEnvelope hash tracking per v02.157 | STUB_WP_IDS: NONE
- PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: No calendar interaction in distillation pipeline | STUB_WP_IDS: NONE
- PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: No editor interaction in scope | STUB_WP_IDS: NONE
- PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: No document editing in scope | STUB_WP_IDS: NONE
- PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: No spreadsheet in scope | STUB_WP_IDS: NONE
- PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: No Locus UI integration in scope | STUB_WP_IDS: NONE
- PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: No Loom interaction in scope | STUB_WP_IDS: NONE
- PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product work packet changes in scope | STUB_WP_IDS: NONE
- PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: No product task board changes in scope | STUB_WP_IDS: NONE
- PILLAR: MicroTask | STATUS: TOUCHED | NOTES: MTE escalation-driven distillation candidate capture (FR-EVT-MT-015) extended with persistent Skill Bank storage write | STUB_WP_IDS: NONE
- PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: No command center surface in scope (backend-only) | STUB_WP_IDS: NONE
- PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: FEMS is separate memory substrate; not directly involved in training pipeline | STUB_WP_IDS: NONE
- PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: DistillationEval job kind gets execution handler; distillation jobs run through Workflow Engine with capability gates | STUB_WP_IDS: NONE
- PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: Spec Router artifacts consumed as input hashes in observability but not modified | STUB_WP_IDS: NONE
- PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: New skill_log_entry and distillation tables follow SQLITE_NOW_POSTGRES_READY posture; SQL must be portable | STUB_WP_IDS: NONE
- PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: Training data format is spec-defined; no LLM-friendly data changes needed | STUB_WP_IDS: NONE
- PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: No Stage interaction in scope | STUB_WP_IDS: NONE
- PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: No Studio interaction in scope | STUB_WP_IDS: NONE
- PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: No Atelier/Lens interaction in scope | STUB_WP_IDS: NONE
- PILLAR: Skill distillation / LoRA | STATUS: TOUCHED | NOTES: Primary pillar; implements full Skill Bank data model, distillation job lifecycle, adapter checkpoint lineage, eval gating, and promotion pipeline | STUB_WP_IDS: NONE
- PILLAR: ACE | STATUS: TOUCHED | NOTES: Context Pack and PromptEnvelope hash integration for distillation observability and reproducible training inputs | STUB_WP_IDS: NONE
- PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: Retrieval signals feed data_trust_score but RAG pipeline itself is not modified | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Distillation lineage persistence | SUBFEATURES: teacher/student ids, tokenizer metadata, checkpoint parents, eval decisions | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Core durable lineage from spec Section 9.1.2
- PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Benchmark-gated adapter lifecycle | SUBFEATURES: LoRA/QLoRA/DoRA training config, eval suite, promotion/rollback | PRIMITIVES_FEATURES: PRIM-DataQualityMetrics, PRIM-AdapterCheckpoint | MECHANICAL: engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Adapter training outcomes compared against teacher and previous checkpoint
- PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: LoRA inference wiring | SUBFEATURES: lora_id in provider request envelope, adapter selection by task tags | PRIMITIVES_FEATURES: NONE | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-MTE-LoRA-Wiring-v1 | NOTES: Out of scope for this WP; requires provider client changes
- PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Export and replay controls | SUBFEATURES: capability-gated export, deterministic replay metadata | PRIMITIVES_FEATURES: PRIM-RedactionMode | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Prevents off-device leakage of local-only checkpoints
- PILLAR: Flight Recorder | CAPABILITY_SLICE: Distillation stage observability | SUBFEATURES: FR-EVT-DISTILL-* events, Context Pack hash tracking, PromptEnvelope hash tracking | PRIMITIVES_FEATURES: PRIM-FlightEvent | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Per spec Section 5.3.6 and v02.157 additions
- PILLAR: MicroTask | CAPABILITY_SLICE: Persistent candidate capture | SUBFEATURES: escalation-driven distillation candidate write to Skill Bank | PRIMITIVES_FEATURES: PRIM-DistillationCandidate | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Extends existing ephemeral candidate to durable artifact
- PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Distillation job execution | SUBFEATURES: DistillationEval job handler, workflow engine integration, capability gates | PRIMITIVES_FEATURES: PRIM-JobKind, PRIM-DistillationCandidate | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: JobKind::DistillationEval gets full handler
- PILLAR: ACE | CAPABILITY_SLICE: Context provenance in training | SUBFEATURES: Context Pack hashes and PromptEnvelope hashes in distillation observability | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Read-only integration; hashes recorded but ACE runtime not modified
- PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Distillation tables Postgres portability | SUBFEATURES: portable SQL for skill_log_entry, distill_job, adapter_checkpoint, eval_run, distill_example | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: All SQL must be SQLITE_NOW_POSTGRES_READY; avoid SQLite-specific syntax
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Capability: Skill Bank log entry persistence | JobModel: AI_JOB | Workflow: distillation_candidate_capture | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-MT-015 | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extends MTE escalation path to write durable SkillBankLogEntry
- Capability: Distillation job orchestration | JobModel: WORKFLOW | Workflow: distillation_pipeline | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-SELECT, FR-EVT-DISTILL-TEACHER, FR-EVT-DISTILL-STUDENT, FR-EVT-DISTILL-SCORE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Full select-teacher-student-score pipeline
- Capability: Adapter checkpoint management | JobModel: WORKFLOW | Workflow: adapter_training_lifecycle | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-CHECKPOINT | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: LoRA/QLoRA/DoRA checkpoint creation with lineage
- Capability: Eval gating and promotion | JobModel: WORKFLOW | Workflow: eval_promotion_pipeline | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-EVAL, FR-EVT-DISTILL-PROMOTE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Benchmark-gated promotion with rollback safety
- Capability: Data trust scoring | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: FR-EVT-DISTILL-SCORE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Deterministic 0-1 scoring algorithm per spec 9.1.3.1
- Capability: PII/secret redaction | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: Log-time and pre-training scrubbing per spec 9.1.1.3
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 30m
- MATRIX_SCAN_NOTES:
  - Distillation pipeline crosses MTE, Flight Recorder, Artifact System, ACE Runtime, and Storage Portability boundaries
  - Local-model-only posture for training (no cloud model training); cloud fallback only for teacher inference during escalation
  - Checkpoint artifacts use Artifact System foundations (content_hash, retention/GC)
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: PRIM-DistillationCandidate -> PRIM-SkillBankLogEntry
  - Kind: data_flow
  - ROI: HIGH
  - Effort: LOW
  - Spec refs: Section 9.1.2, Section 2.6.6.8.13
  - In-scope for this WP: YES (implementation; matrix edge will be added post-implementation)
  - Edge: PRIM-SkillBankLogEntry -> PRIM-FlightEvent
  - Kind: observability
  - ROI: HIGH
  - Effort: LOW
  - Spec refs: Section 5.3.6
  - In-scope for this WP: YES (implementation; matrix edge will be added post-implementation)
  - Edge: PRIM-AdapterCheckpoint -> PRIM-DataQualityMetrics
  - Kind: lifecycle_gate
  - ROI: HIGH
  - Effort: MEDIUM
  - Spec refs: Section 9.1.4
  - In-scope for this WP: YES (implementation; matrix edge will be added post-implementation)
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The spec already captures subsystem-level distillation matrix edges (MTE to Skill Bank, Skill Bank to Storage Portability) via v02.157. Candidate primitive-level edges above will be added as IMX entries post-implementation alongside the Appendix 12.4 primitive index update.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: Internal spec-grounded WP; all matrix edges derive from existing Master Spec Section 9 integration requirements. No external combo research needed.
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
  - Combo: Distillation + Flight Recorder stage events | Pillars: Skill distillation / LoRA, Flight Recorder | Mechanical: engine.analyst | Primitives/Features: PRIM-FlightEvent, PRIM-DistillationCandidate | Resolution: IN_THIS_WP | Stub: NONE | Notes: Training data quality monitoring via FR events feeding data trust score computation
  - Combo: Distillation + Context Pack provenance | Pillars: Skill distillation / LoRA, ACE | Mechanical: engine.context | Primitives/Features: PRIM-SkillBankLogEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: Context Pack hash tracking for reproducible teacher/student inputs per v02.157
  - Combo: MTE escalation + Skill Bank persistence | Pillars: Skill distillation / LoRA, MicroTask | Mechanical: NONE | Primitives/Features: PRIM-DistillationCandidate, PRIM-SkillBankLogEntry | Resolution: IN_THIS_WP | Stub: NONE | Notes: Automatic training data generation from escalation events
  - Combo: Adapter checkpoint + Artifact System | Pillars: Skill distillation / LoRA, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-AdapterCheckpoint | Resolution: IN_THIS_WP | Stub: NONE | Notes: Checkpoint storage uses artifact system foundations for durability and GC
  - Combo: LoRA inference wiring + trained adapters | Pillars: Skill distillation / LoRA, MicroTask | Mechanical: NONE | Primitives/Features: NONE | Resolution: NEW_STUB | Stub: WP-1-MTE-LoRA-Wiring-v1 | Notes: Promoted adapters need inference-side wiring (already stubbed; blocked by this WP)
  - Combo: Spawn conversation histories + training data | Pillars: Skill distillation / LoRA, Execution / Job Runtime | Mechanical: NONE | Primitives/Features: NONE | Resolution: NEW_STUB | Stub: WP-1-Session-Spawn-Conversation-Distillation-v1 | Notes: High-quality teacher/student pairs from spawn trees (already stubbed)
  - Combo: Distillation tables + Postgres portability | Pillars: SQL to PostgreSQL shift readiness, Skill distillation / LoRA | Mechanical: engine.dba | Primitives/Features: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate | Resolution: IN_THIS_WP | Stub: NONE | Notes: All 6 Skill Bank tables use SQLITE_NOW_POSTGRES_READY SQL; portable schema enables future Postgres migration
  - Combo: Export controls + capability enforcement | Pillars: Skill distillation / LoRA | Mechanical: engine.sovereign | Primitives/Features: PRIM-RedactionMode | Resolution: IN_THIS_WP | Stub: NONE | Notes: Capability-gated export prevents off-device leakage of local-only checkpoints and eval artifacts
  - Combo: Data trust scoring + eval metrics | Pillars: Skill distillation / LoRA, Execution / Job Runtime | Mechanical: engine.analyst | Primitives/Features: PRIM-DataQualityMetrics | Resolution: IN_THIS_WP | Stub: NONE | Notes: Quality-weighted training selection feeds eval gating for benchmark-gated promotion
  - Combo: PII redaction + privacy enforcement | Pillars: Skill distillation / LoRA | Mechanical: engine.sovereign | Primitives/Features: PRIM-RedactionMode, PRIM-DataQualityMetrics | Resolution: IN_THIS_WP | Stub: NONE | Notes: Log-time redaction and pre-training scrubbing prevent PII leakage into training data
  - Combo: Checkpoint lineage + rollback safety | Pillars: Skill distillation / LoRA, Execution / Job Runtime | Mechanical: engine.dba | Primitives/Features: PRIM-AdapterCheckpoint, PRIM-DataQualityMetrics | Resolution: IN_THIS_WP | Stub: NONE | Notes: Parent checkpoint chain enables safe rollback on eval gate failure
  - Combo: Distillation observability + DBA storage | Pillars: Skill distillation / LoRA, Flight Recorder | Mechanical: engine.dba | Primitives/Features: PRIM-FlightEvent, PRIM-DistillationCandidate | Resolution: IN_THIS_WP | Stub: NONE | Notes: Structured SQL storage for eval metrics and promotion decisions enables dashboard queries
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_REASON: All HIGH-ROI combinations resolved; 10 IN_THIS_WP, 2 via existing stubs (WP-1-MTE-LoRA-Wiring-v1, WP-1-Session-Spawn-Conversation-Distillation-v1). No new stubs required beyond existing backlog.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- SCAN_SCOPE: .GOV/task_packets/, .GOV/task_packets/stubs/, .GOV/roles_shared/records/TASK_BOARD.md, src/backend/handshake_core/
- MATCHED_STUBS:
  - Artifact: WP-1-MTE-LoRA-Wiring-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: MISSING | Matrix: MISSING | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: LoRA inference wiring is distinct from training pipeline; this WP produces trained adapters, MTE-LoRA-Wiring consumes them
  - Artifact: WP-1-Session-Spawn-Conversation-Distillation-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: MISSING | Matrix: MISSING | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Spawn tree training data is a data source for distillation pipeline; separate upstream concern
  - Artifact: WP-1-MTE-Summaries-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: MT summaries are audit/observability artifacts, not training data selection
  - Artifact: WP-1-MTE-Resource-Caps-v1 | BoardStatus: STUB | Intent: DISTINCT | PrimitiveIndex: N/A | Matrix: N/A | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Resource caps constrain MT execution cost; orthogonal to distillation pipeline
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Distillation | BoardStatus: SUPERSEDED | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: v1 produced ephemeral PendingDistillationCandidate and FR-EVT-MT-015 event type; this WP completes the full backend pipeline
  - Artifact: WP-1-Micro-Task-Executor-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: MTE provides candidate capture foundation (FR-EVT-MT-015); this WP builds on it
  - Artifact: WP-1-Artifact-System-Foundations-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: Artifact system provides durable storage for checkpoints/eval artifacts
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Distillation | Covers: primitive | Verdict: IMPLEMENTED | Notes: DistillationInfo and PendingDistillationCandidate structs exist and produce candidates during MTE escalation
  - Path: ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs | Artifact: WP-1-Micro-Task-Executor-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: FlightRecorderEventType::MicroTaskDistillationCandidate defined (FR-EVT-MT-015)
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Distillation | Covers: primitive | Verdict: PARTIAL | Notes: JobKind::DistillationEval enum variant exists but has no execution handler
  - Path: ../handshake_main/src/backend/handshake_core/src/capabilities.rs | Artifact: NONE | Covers: execution | Verdict: NOT_PRESENT | Notes: No distillation-specific capability gates
  - Path: ../handshake_main/src/backend/handshake_core/migrations/ | Artifact: NONE | Covers: primitive | Verdict: NOT_PRESENT | Notes: No skill_log_entry, distill_job, adapter_checkpoint, eval_run, or distill_example tables in any migration
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: Prior WP-1-Distillation (SUPERSEDED) provides partial coverage with ephemeral candidate structs. This WP expands into full pipeline implementation with durable storage, algorithms, and observability. No duplicates; all adjacent stubs are KEEP_SEPARATE with distinct concerns.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This WP is strictly backend infrastructure (data model, SQL schema, algorithms, observability). No user-facing UI surfaces are created or modified. Distillation training consoles are explicitly OUT_OF_SCOPE per stub.
- UI_SURFACES: N/A
- UI_CONTROLS (buttons/dropdowns/inputs): N/A
- UI_STATES (empty/loading/error): N/A
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers): N/A
- UI_ACCESSIBILITY_NOTES: N/A
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: Backend-only WP; no GUI surfaces created or modified
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

### PACKET_HYDRATION (task packet auto-fill; mandatory for HYDRATED_RESEARCH_V1)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Micro-Task-Executor, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-MTE-LoRA-Wiring
- SPEC_ANCHOR_PRIMARY: Section 9 Continuous Local Skill Distillation (Skill Bank and Pipeline)
- WHAT: Implement the full Skill Bank and distillation backend: durable data model (SkillBankLogEntry, DistillJob, AdapterCheckpoint, EvalRun), SQL schema, data trust scoring, benchmark-gated adapter lifecycle, export controls, PII/secret redaction, and full-pipeline Flight Recorder observability.
- WHY: The spec hardcodes LoRA/QLoRA/DoRA posture, PromptEnvelope/Context Pack reuse, and distillation evidence requirements (v02.115 through v02.157), but the implementation has only ephemeral in-memory candidates and a stub job kind. The learning substrate cannot function without persistent lineage, eval gating, and promotion safety.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/models/ (new skill_bank module)
  - src/backend/handshake_core/src/distillation/ (new module)
  - src/backend/handshake_core/src/workflows.rs (extend candidate persistence)
  - src/backend/handshake_core/src/flight_recorder/ (distillation stage events)
  - src/backend/handshake_core/src/storage/ (storage trait extensions)
  - src/backend/handshake_core/migrations/ (new migration for Skill Bank tables)
  - src/backend/handshake_core/src/capabilities.rs (distillation capability gates)
- OUT_OF_SCOPE:
  - End-user UI polish for model-training consoles
  - Full-model fine-tuning beyond adapter-only posture
  - LoRA inference wiring (WP-1-MTE-LoRA-Wiring-v1)
  - Spawn conversation distillation pipeline (WP-1-Session-Spawn-Conversation-Distillation-v1)
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core
  just validator-spec-regression
  just validator-scan WP-1-Distillation-v2
  ```
- DONE_MEANS:
  - skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run tables exist and pass migration
  - SkillBankLogEntry, DistillJob, AdapterCheckpoint, EvalRun structs implement storage trait with SQLite-now-Postgres-ready SQL
  - compute_data_trust_score algorithm produces valid 0-1 scores from multi-signal quality aggregation
  - Distillation candidate persistence pipeline connects MTE escalation to Skill Bank durable storage
  - Checkpoint lineage is queryable (parent chains via parent_checkpoint_id)
  - Eval gating enforces benchmark thresholds (pass@k, compile/test rates, collapse indicators) before promotion
  - Rollback-safe promotion: failed promotion reverts to previous checkpoint without data loss
  - Export controls prevent off-device leakage of local-only checkpoints via capability gates
  - Flight Recorder events emitted for each distillation stage (select, teacher, student, score, checkpoint, eval, promote/rollback)
  - Context Pack hashes and PromptEnvelope hashes recorded in distillation observability per v02.157
  - PII/secret redaction applied at log time (redact_entry) and pre-training scrubbing
  - All tests pass: cargo test -p handshake_core, just validator-spec-regression
- PRIMITIVES_EXPOSED:
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DistillationCandidate
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
- SEARCH_TERMS:
  - SkillBankLogEntry, distill_job, adapter_checkpoint, eval_run
  - DistillationCandidate, PendingDistillationCandidate, DistillationInfo
  - data_trust_score, compute_data_trust_score, enable_distillation
  - MicroTaskDistillationCandidate, DistillationEval
  - LoRA, QLoRA, DoRA, redact_entry
- RUN_COMMANDS:
  ```bash
  cargo test -p handshake_core
  just validator-spec-regression
  just validator-scan WP-1-Distillation-v2
  just gov-check
  ```
- RISK_MAP:
  - "Model collapse from self-distilled data dominance" -> "data_trust_score weighting with collapse indicators monitored in eval gating"
  - "Cross-tokenizer corruption in teacher/student comparisons" -> "Tokenizer metadata required per-candidate; compatibility validated at dataset assembly"
  - "Silent regression from adapter promotion without strict eval" -> "Benchmark-gated promotion with rollback-safe checkpoint lineage"
  - "PII leakage into training data" -> "Log-time redaction plus pre-training scrubbing; capability-gated export controls"
  - "Checkpoint storage exhaustion" -> "Artifact system GC and retention policies apply to distillation artifacts"
- BUILD_ORDER_SYNC_REQUIRED: NO
- BUILD_ORDER_SYNC_NOTES:
  - BUILD_ORDER.md already tracks WP-1-Distillation-v2 via stub metadata; no dependency or blocker changes required

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSE_ROWS:
  - CLAUSE: Section 9.1.1 Data model (SkillBankLogEntry, QualityMeta, TelemetryMeta, PrivacyMeta) | WHY_IN_SCOPE: Core data structures for durable distillation lineage | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/models/skill_bank.rs | EXPECTED_TESTS: cargo test -p handshake_core -- skill_bank | RISK_IF_MISSED: No persistent training data; pipeline cannot operate
  - CLAUSE: Section 9.1.2 SQL schema (skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run) | WHY_IN_SCOPE: Durable storage for Skill Bank artifacts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql | EXPECTED_TESTS: cargo test -p handshake_core -- migration | RISK_IF_MISSED: Data model exists only in memory; no queryable lineage
  - CLAUSE: Section 9.1.3.1 compute_data_trust_score | WHY_IN_SCOPE: Quality-weighted training data selection | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/scoring.rs | EXPECTED_TESTS: cargo test -p handshake_core -- data_trust_score | RISK_IF_MISSED: Training data selection is unweighted; collapse risk increases
  - CLAUSE: Section 9.1.3.2 build_distill_dataset (new + replay batches) | WHY_IN_SCOPE: Dataset assembly for adapter training | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/dataset.rs | EXPECTED_TESTS: cargo test -p handshake_core -- distill_dataset | RISK_IF_MISSED: No dataset assembly; training cannot start
  - CLAUSE: Section 9.1.4 Evaluation and promotion gates | WHY_IN_SCOPE: Benchmark-gated promotion prevents silent regression | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/eval.rs | EXPECTED_TESTS: cargo test -p handshake_core -- eval_promotion | RISK_IF_MISSED: Adapters promoted without quality validation; silent regressions
  - CLAUSE: Section 5.3.6 Distillation observability (FR events per stage) | WHY_IN_SCOPE: Pipeline visibility and debugging | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/mod.rs | EXPECTED_TESTS: cargo test -p handshake_core -- flight_recorder_distill | RISK_IF_MISSED: Silent pipeline failures; no audit trail
  - CLAUSE: Section 2.6.6.8.13 Learning Integration (DistillationCandidate persistence) | WHY_IN_SCOPE: MTE escalation produces training data | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test -p handshake_core -- distillation_candidate | RISK_IF_MISSED: Candidates remain ephemeral; training data lost on process exit
  - CLAUSE: Section 9 PII/secret redaction (redact_entry) | WHY_IN_SCOPE: Privacy safety in training data | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/redaction.rs | EXPECTED_TESTS: cargo test -p handshake_core -- redaction | RISK_IF_MISSED: PII leaks into training data; compliance and safety failure

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CONTRACT_ROWS:
  - CONTRACT: SkillBankLogEntry JSON/SQL | PRODUCER: MTE escalation handler (workflows.rs) | CONSUMER: distillation dataset builder, Flight Recorder | SERIALIZER_TRANSPORT: serde_json / SQLite row | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- skill_bank_serialization | DRIFT_RISK: Field additions in spec vs struct mismatch
  - CONTRACT: DistillationCandidate artifact | PRODUCER: MTE escalation (workflows.rs) | CONSUMER: Skill Bank persistence, FR event logger | SERIALIZER_TRANSPORT: serde_json artifact | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- distillation_candidate_shape | DRIFT_RISK: Existing ephemeral struct vs spec DistillationCandidate interface divergence
  - CONTRACT: AdapterCheckpoint lineage | PRODUCER: adapter training pipeline | CONSUMER: eval gating, promotion logic, export controls | SERIALIZER_TRANSPORT: SQLite row with parent_checkpoint_id FK | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- checkpoint_lineage | DRIFT_RISK: Orphaned checkpoints if parent FK not enforced
  - CONTRACT: FR-EVT-DISTILL-* event payloads | PRODUCER: distillation pipeline stages | CONSUMER: Flight Recorder storage, dashboards | SERIALIZER_TRANSPORT: FlightRecorderEventType enum + JSON payload | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- flight_recorder_distill_events | DRIFT_RISK: Missing required fields (model/tokenizer ids, context refs, lineage)
  - CONTRACT: DataTrustScore computation | PRODUCER: compute_data_trust_score | CONSUMER: dataset builder, training data selection | SERIALIZER_TRANSPORT: f64 in 0.0..=1.0 range | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- data_trust_score_range | DRIFT_RISK: Score outside valid range or missing input signals

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core -- skill_bank_schema_presence (verify all 6 tables created by migration)
  - cargo test -p handshake_core -- distillation_candidate_persistence (verify MTE candidate writes to Skill Bank)
  - cargo test -p handshake_core -- data_trust_score_range (verify score in 0.0..=1.0)
  - cargo test -p handshake_core -- checkpoint_lineage_query (verify parent chain traversal)
  - cargo test -p handshake_core -- eval_gate_promotion (verify benchmark threshold enforcement)
  - cargo test -p handshake_core -- export_control_capability (verify capability gate blocks unauthorized export)
  - cargo test -p handshake_core -- redaction_scrubbing (verify PII removal from training examples)
- CANONICAL_CONTRACT_EXAMPLES:
  - Fixture: SkillBankLogEntry with all 52 columns populated (golden test row)
  - Fixture: DistillationCandidate with teacher/student snapshot refs and trust score
  - Fixture: AdapterCheckpoint with parent lineage chain (3-deep)
  - Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - 1. SQL migration: create skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run tables and replay_candidates view
  - 2. Data model structs: SkillBankLogEntry, QualityMeta, TelemetryMeta, PrivacyMeta, DistillJob, DistillExample, AdapterCheckpoint, EvalRun
  - 3. Storage trait extensions: CRUD operations for all Skill Bank entities
  - 4. PII/secret redaction: redact_entry function with log-time and pre-training scrubbing modes
  - 5. Data trust scoring: compute_data_trust_score with multi-signal quality aggregation
  - 6. Distillation candidate persistence: extend MTE escalation path to write SkillBankLogEntry
  - 7. Dataset assembly: build_distill_dataset with new and replay batch support
  - 8. Adapter training lifecycle: checkpoint creation with parent lineage
  - 9. Eval gating and promotion: benchmark-gated promotion with rollback safety
  - 10. Export controls: capability gates for checkpoint and eval artifact export
  - 11. Flight Recorder events: FR-EVT-DISTILL-* for all pipeline stages
  - 12. Context Pack and PromptEnvelope hash integration in observability
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs (existing DistillationInfo, PendingDistillationCandidate)
  - src/backend/handshake_core/src/flight_recorder/mod.rs (existing MicroTaskDistillationCandidate event)
  - src/backend/handshake_core/src/storage/mod.rs (existing JobKind::DistillationEval)
  - src/backend/handshake_core/src/capabilities.rs (needs distillation gates)
  - src/backend/handshake_core/migrations/ (latest migration is 0016)
- TRIPWIRE_TESTS:
  - Migration applies cleanly: cargo test -p handshake_core -- migration
  - All Skill Bank CRUD: cargo test -p handshake_core -- skill_bank
  - Data trust score produces valid output: cargo test -p handshake_core -- data_trust_score
  - Candidate persistence round-trip: cargo test -p handshake_core -- distillation_candidate
  - Checkpoint lineage traversal: cargo test -p handshake_core -- checkpoint_lineage
  - Eval gate blocks unqualified promotion: cargo test -p handshake_core -- eval_gate
- CARRY_FORWARD_WARNINGS:
  - PendingDistillationCandidate in workflows.rs is ephemeral and JSON-serialized to artifact; migration to SQL-backed SkillBankLogEntry must preserve compatibility during transition
  - JobKind::DistillationEval exists but is dead code; handler implementation must match workflow engine dispatch patterns used by other job kinds
  - SQL must be SQLITE_NOW_POSTGRES_READY: avoid SQLite-specific syntax (e.g., AUTOINCREMENT vs SERIAL)
  - Cross-tokenizer safety: do not assume teacher and student share the same tokenizer; always validate tokenizer_id match or use cross-tokenizer-safe replay

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Section 9.1.2 SQL schema: all 6 tables and 1 view match spec column definitions
  - Section 9.1.3.1 compute_data_trust_score: algorithm matches spec formula
  - Section 9.1.4 eval gating: promotion requires benchmark thresholds
  - Section 5.3.6 observability: all FR-EVT-DISTILL-* events include required fields
  - Section 2.6.6.8.13 candidate persistence: MTE escalation writes durable SkillBankLogEntry
  - PII/secret redaction: redact_entry covers all spec-defined sensitive fields
  - Export controls: capability gate enforcement for local-only artifacts
- FILES_TO_READ:
  - src/backend/handshake_core/src/models/skill_bank.rs
  - src/backend/handshake_core/src/distillation/
  - src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/capabilities.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core
  - just validator-spec-regression
  - just validator-scan WP-1-Distillation-v2
- POST_MERGE_SPOTCHECKS:
  - Verify skill_log_entry table exists after migration
  - Verify FR-EVT-DISTILL-PROMOTE event includes parent_checkpoint_id and promotion decision
  - Verify export capability gate rejects unauthorized checkpoint download

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- NOT_PROVEN_ITEMS:
  - Actual LoRA/QLoRA/DoRA training convergence quality: spec defines adapter-only posture and hyperparameter tracking but real training quality depends on runtime data volume and distribution
  - Cross-tokenizer replay fidelity: spec mandates cross-tokenizer-safe distillation but actual fidelity under diverse tokenizer pairs requires field validation
  - Data trust score calibration: the compute_data_trust_score formula can be implemented per spec but optimal weight calibration requires real-world training data evaluation
  - Collapse indicator sensitivity: spec defines collapse monitoring but threshold tuning requires observed training runs
  - Migration file number (0017 assumed): actual next migration number depends on main branch state at coding time

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Section 9 Continuous Local Skill Distillation provides exhaustive data model definitions (17 dataclasses), SQL schema (6 tables, 1 view), algorithms (data trust scoring, dataset assembly, training, eval gating), and observability requirements. Section 2.6.6.8.13 defines DistillationCandidate interface. Section 5.3.6 defines FR event requirements. All acceptance criteria are measurable through spec compliance tests.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec Section 9 (v02.115 through v02.180) comprehensively defines all data models, SQL schemas, algorithms, observability requirements, and integration contracts needed for this WP. The v02.157 spec pass already incorporated Context Pack hash, PromptEnvelope hash, and pending-candidate queue visibility requirements. No Main Body or appendix gaps remain.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### DISCOVERY (refinement-time discoveries)
- DISCOVERY_PRIMITIVES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint, PRIM-DataQualityMetrics, PRIM-RedactionMode
- DISCOVERY_STUBS: NONE_CREATED
- DISCOVERY_STUBS_REASON: All adjacent distillation work already stubbed (WP-1-MTE-LoRA-Wiring-v1, WP-1-Session-Spawn-Conversation-Distillation-v1); no new gaps found
- DISCOVERY_MATRIX_EDGES: NONE
- DISCOVERY_UI_CONTROLS: NONE_APPLICABLE
- DISCOVERY_UI_CONTROLS_REASON: Backend-only WP; no UI surfaces created or modified
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED
- DISCOVERY_SPEC_ENRICHMENT_REASON: Spec Section 9 (v02.115 through v02.180) comprehensively covers all distillation requirements; no new spec content discovered

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Section 9 Continuous Local Skill Distillation (Skill Bank and Pipeline)
- CONTEXT_START_LINE: 53539
- CONTEXT_END_LINE: 53600
- CONTEXT_TOKEN: data_trust_score
- EXCERPT_ASCII_ESCAPED:
  ```text
  # 9. Continuous Local Skill Distillation (Skill Bank & Pipeline)

  **Why**
  - Capture the complete Skill Bank and distillation pipeline (teacher/student) inside the Master Spec without losing any technical detail.
  - Ensure alignment with AI Job Model, Workflow Engine, Flight Recorder, and capability/privacy controls.

  **How it integrates**
  - Data model fields (messages, snapshots, engines, context refs, telemetry, quality, trust, checkpoints, examples) map to Section 3 storage/indexing and provenance rules; no token logs are stored, tokenization is per-engine at train time.
  - Distillation jobs (sample/select -> teacher -> student -> score -> checkpoint -> eval/promotion) must run through the Workflow Engine with capability gates; Flight Recorder logs models, tokenizers, params, files, tools, metrics, reward features, lineage, and data_signature/job_ids_json.

  **Quality-Weighted Training Data Selection:**
  Training data selection MUST weight samples by signals:
  1. User signal: thumbs up/down, edit ratio (from QualityMeta)
  2. Auto-eval: tests passed, compile success, reasoning score
  3. Retrieval signal: Was retrieved content used? Did it help?
  4. Data trust score: Combined 0-1 weight for training (data_trust_score field)
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.6.6.8.13 Learning Integration
- CONTEXT_START_LINE: 14210
- CONTEXT_END_LINE: 14258
- CONTEXT_TOKEN: enable_distillation
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.13 Learning Integration

  ###### 2.6.6.8.13.1 Skill Bank Integration

  When escalation occurs and policy.enable_distillation = true, a distillation candidate MUST be generated:

  interface DistillationCandidate {
    skill_log_entry_id: UUID;
    mt_id: string;
    wp_id: string;
    student_attempt: { model_id, lora_id?, prompt_snapshot_ref, output_snapshot_ref, outcome, iterations };
    teacher_success: { model_id, lora_id?, prompt_snapshot_ref, output_snapshot_ref, outcome, iterations };
    task_type_tags: string[];
    contributing_factors: string[];
    data_trust_score: number;
    distillation_eligible: boolean;
  }

  [ADD v02.157] Runtime implementations MAY stage candidates in a bounded pending queue before promotion into canonical Skill Bank artifacts, but that queue MUST remain recorder-visible and retain teacher/student prompt refs, tokenizer metadata, task tags, and trust signals.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 5.3.6 Distillation Observability Requirements
- CONTEXT_START_LINE: 23078
- CONTEXT_END_LINE: 23084
- CONTEXT_TOKEN: Flight Recorder events
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 5.3.6 Distillation Observability Requirements
  - Distillation jobs MUST emit Flight Recorder events for each stage (select, teacher run, student run, score, checkpoint, eval, promote/rollback) with trace IDs.
  - Required fields: model/tokenizer ids, inference params, context refs (files/spec sections/tools), metrics (pass@k, compile/test rates, collapse indicators), reward features, lineage (parent_checkpoint_id), data_signature, job_ids_json, promotion decisions.
  - PII/secret handling: apply log-time redaction and pre-training scrubbing; enforce capability-based export controls for Skill Bank artifacts.
  - Dashboards/traces should surface promotion gates vs teacher/previous checkpoints and collapse indicators for regression detection.
  - [ADD v02.157] Distillation observability MUST also record Context Pack hashes/freshness decisions, PromptEnvelope hashes, and pending-candidate queue transitions whenever Context Packs or Spec Router artifacts shape teacher/student inputs.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.5.12 Context Packs AI Job Profile
- CONTEXT_START_LINE: 9305
- CONTEXT_END_LINE: 9354
- CONTEXT_TOKEN: context_pack_builder
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.5.12 Context Packs AI Job Profile

  **Implements:** AI Job Model (Section 2.6.6)
  **Profile ID:** context_pack_builder_v0.1
  **Status:** Draft (internal)

  **Why**
  Retrieval-backed answers and transformations improve correctness and token efficiency when the system prefers mechanical, reusable compactions over raw snippet dumps. A ContextPack is a derived, provenance-bound artifact (facts/constraints/open loops + anchors) that can be retrieved cheaply and assembled deterministically into PromptEnvelopes.

  [ADD v02.156] ContextPack payloads, anchors, coverage, freshness guards, and canonical artifact serialization are portable retrieval contracts.
  [ADD v02.157] ContextPack freshness policy/decision, build/reuse hashes, and recorder-visible build/select/refresh outcomes are canonical backend contracts for later distillation, replay, and model onboarding.
  ```
