## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by the current ORCHESTRATOR_PROTOCOL refinement workflow.

### METADATA
- WP_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-03T22:33:38.363Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- `PostgresDatabase` in `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs` still reports `supports_structured_collab_artifacts() == false` and returns `StorageError::NotImplemented("structured collaboration artifacts")` for all canonical structured-collaboration readers.
- Workflow and Locus paths in `../handshake_main/src/backend/handshake_core/src/workflows.rs` rely on `ensure_structured_collab_artifacts_supported(db)?`, so PostgreSQL remains an honest capability denial instead of a portable structured work-state backend.
- `execute_locus_sync_task_board` still hard-requires `crate::storage::locus_sqlite::ensure_locus_sqlite(db)?`, so canonical Task Board sync and related projection truth remain SQLite-only.
- Storage tests currently assert PostgreSQL capability denial rather than parity, which means the repo still treats missing Postgres support as the expected state.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 65m
- SEARCH_SCOPE: current Master Spec v02.179 storage-portability and structured-collaboration clauses, the active stub, current SQLite/Postgres storage implementations, workflow/Locus callers, storage tests, and completed structured-collaboration contract packets
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.179.md`, `.GOV/task_packets/stubs/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1.md`, `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md`, `.GOV/task_packets/WP-1-Structured-Collaboration-Artifact-Family-v1.md`, `.GOV/task_packets/WP-1-Structured-Collaboration-Schema-Registry-v4/packet.md`, `.GOV/task_packets/WP-1-Structured-Collaboration-Contract-Hardening-v1/packet.md`, `.GOV/task_packets/WP-1-Dual-Backend-Tests-v2.md`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs`, `../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs`, `../handshake_main/src/backend/handshake_core/src/workflows.rs`, `../handshake_main/src/backend/handshake_core/src/locus/types.rs`, `../handshake_main/src/backend/handshake_core/src/locus/task_board.rs`, `../handshake_main/src/backend/handshake_core/src/storage/tests.rs`, and `../handshake_main/src/backend/handshake_core/migrations/`
- PATTERNS_EXTRACTED: preserve one canonical structured record contract across both backends; keep unsupported operations explicit and narrow instead of blanket backend denial; reuse portable schema/migration discipline; prove parity through dual-backend roundtrip and sync tests rather than backend-specific assumptions
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing canonical structured-collaboration envelope and summary contract on both backends; ADAPT SQLite row and query behavior into PostgreSQL-backed sqlx implementations and migration coverage; REJECT keeping PostgreSQL behind a blanket `structured collaboration artifacts` capability denial
- LICENSE/IP_NOTES: internal governance and product-code review only; no third-party code or copyrighted text will be copied into the implementation
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The current Main Body already requires portable structured collaboration records, project-agnostic workflow-state fields, and dual-backend portability law. This packet is implementation parity work against existing spec truth.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_REASON_NO: This WP is strictly internal and mechanical. The governing truth is the current local Master Spec plus current backend implementation drift, not a time-sensitive external standard or vendor change.
- SOURCE_MAX_AGE_DAYS: N/A
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - The current spec and completed structured-collaboration packets already define the canonical record family, base envelope, summary contract, and workflow-state vocabulary.
  - The missing work is backend implementation parity plus dual-backend proof, not discovery of a new external pattern.
- RESEARCH_GAPS_TO_TRACK:
  - NONE
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - NONE
- ADAPT_PATTERNS:
  - NONE
- REJECT_PATTERNS:
  - NONE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - NONE
- MATCHED_PROJECTS:
  - NONE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event families are required.
- Existing workflow, Locus, and structured-artifact materialization evidence must keep their semantics when PostgreSQL becomes a supported structured-collaboration backend.
- This packet is backend parity for canonical work-state storage, not a telemetry taxonomy expansion.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: PostgreSQL implementations can round-trip records while drifting on base-envelope or summary semantics such as `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, or `mirror_state`. Mitigation: use the same canonical row shapes and dual-backend contract assertions.
- Risk: task-board update or sync can remain SQLite-only while read methods claim parity. Mitigation: prove at least one bounded update/sync path and keep any remaining unsupported operations explicit and narrow.
- Risk: migration coverage can become PostgreSQL-only or SQLite-shaped. Mitigation: keep portable schema discipline and route schema work through the governed migration framework.
- Risk: tests can stop checking PostgreSQL positive paths and keep only negative-path heritage from the current denial posture. Mitigation: replace capability-denial assertions with parity-focused dual-backend tests.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - The packet implements parity through existing storage and Locus primitives plus already-governed structured-collaboration contracts.
  - No new spec primitive family is required at refinement time; the gap is backend implementation, migration coverage, and proof.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already covers the storage and structured-collaboration primitives this packet must preserve across backends.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - Keep the primitive index unchanged and implement parity against the existing structured-collaboration contract family.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No orphan primitive family was discovered; the work is parity implementation for already-governed primitives.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: This packet implements backend parity for an existing structured-collaboration feature family.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface is implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The packet preserves existing structured-collaboration interactions while making PostgreSQL a real backend participant.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement parity against the existing Main Body contract.
  - If coding reveals a genuinely missing workflow-state or structured-record requirement, treat that as a separate governed spec-update path instead of silently widening this packet.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by backend parity work | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement contract is affected | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes remain downstream consumers of structured records | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes the result, but this packet is storage parity rather than orchestration design | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media-composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no rendering or creative-generation capability is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication and export flows remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: canonical structured collaboration artifacts and summaries become durable on PostgreSQL as well as SQLite | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: work-packet and micro-task readers are the concrete parity surface being implemented | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: no analytics or insight-generation surface is changed directly | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: this packet implements the actual PostgreSQL structured-collaboration storage contract | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: NOT_TOUCHED | NOTES: this packet implements existing law and does not add new governance authority | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no explanation or tutoring interface is implemented | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: the packet preserves portable workflow-state and summary semantics across backends | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: portable migration and dual-backend proof posture are part of the parity contract | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: no new event taxonomy is introduced, although existing evidence paths must keep their semantics | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: no calendar-facing feature contract is involved | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document-editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: tracked work-packet, micro-task, and task-board projection parity is part of the Locus storage contract | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Loom is not the direct implementation target of this packet | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no work-packet feature semantics change directly; this packet implements backend parity beneath the existing contract | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board feature semantics change directly; the packet only makes bounded projection storage portable | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: micro-task metadata, status rows, and list readers are core parity surfaces | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: no Command Center UI or feature contract changes directly; the packet only improves backend parity beneath existing consumers | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS contract change is introduced directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: runtime artifact materialization and task-board sync depend on these storage surfaces | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router surface is changed | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: this packet converts honest capability denial into real backend parity for canonical structured work-state storage | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: the packet preserves compact summary and base-envelope semantics for local-small-model consumers on PostgreSQL | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no Stage feature surface is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no Studio runtime or creative console surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no distillation or adapter-training flow is affected directly | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE runtime surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of the structured record family | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: canonical structured-collaboration Postgres parity | SUBFEATURES: portable schema coverage, Postgres query implementations, update/sync wiring, dual-backend proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability objective
  - PILLAR: Locus | CAPABILITY_SLICE: tracked work-packet and micro-task storage parity | SUBFEATURES: work-packet readers, micro-task metadata/status/list readers, bounded task-board sync path | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity is bounded to canonical structured work-state paths
  - PILLAR: MicroTask | CAPABILITY_SLICE: canonical micro-task parity | SUBFEATURES: metadata payloads, status rows, list readers, workflow-state preservation | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.archivist, engine.librarian, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: compact summaries and metadata must stay field-equivalent across backends
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: bounded task-board update or sync parity | SUBFEATURES: update path, sync eligibility, canonical row semantics, workflow-state and queue-reason preservation | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity is only honest if the claimed runtime projection path stops being SQLite-only
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: compact summary and base-envelope parity | SUBFEATURES: `project_profile_kind`, `mirror_state`, `workflow_state_family`, `queue_reason_code`, `allowed_action_ids` | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: local-small-model consumers depend on these fields staying portable
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: PostgreSQL work-packet structured row and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: canonical work-packet state must stop being SQLite-only
  - Capability: PostgreSQL micro-task metadata, status-row, and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: micro-task summary and workflow-state projections depend on these readers
  - Capability: PostgreSQL task-board update and bounded sync parity | JobModel: WORKFLOW | Workflow: locus_task_board_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: a bounded task-board path must be portable for parity to be honest
  - Capability: dual-backend structured-collaboration conformance proofs | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validators need executable proof that PostgreSQL left blanket capability denial behind
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - This packet preserves already-governed structured-collaboration interactions while making PostgreSQL a real backend for those interactions.
  - No new appendix interaction edge is needed if parity stays bounded to existing canonical work-state flows.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Current matrix coverage is sufficient; the packet closes backend implementation drift inside existing structured-collaboration interactions.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_REASON_NO: This packet is strictly internal and mechanical. External combo research is not needed to decide whether PostgreSQL must implement an already-defined structured record contract.
- SOURCE_SCAN:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: portable migration coverage plus PostgreSQL structured readers | Pillars: SQL to PostgreSQL shift readiness, MicroTask | Mechanical: engine.dba, engine.version, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: parity is only durable if schema and readers move together
  - Combo: task-board sync plus workflow-state and queue-reason contract | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: a read-only parity story is insufficient if projection truth remains SQLite-only
  - Combo: compact summary/base-envelope parity plus local-small-model consumers | Pillars: LLM-friendly data, MicroTask | Mechanical: engine.context, engine.archivist, engine.librarian | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: parity must preserve the same bounded model-facing contract on both backends
  - Combo: dual-backend proof plus capability-denial removal | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-Database, PRIM-SqliteDatabase, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: the packet is not done unless PostgreSQL positive-path tests replace the current denial posture
  - Combo: work-packet row parity plus summary-field preservation | Pillars: Locus, LLM-friendly data | Mechanical: engine.archivist, engine.librarian, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: work-packet semantics must survive backend change even if the product pillar itself is not widened
  - Combo: micro-task metadata parity plus task-board summary hydration | Pillars: MicroTask, Execution / Job Runtime | Mechanical: engine.archivist, engine.librarian, engine.context | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: metadata readers and projection flows should agree on the same workflow-state contract
  - Combo: PostgreSQL reader parity plus bounded writer parity | Pillars: Locus, Execution / Job Runtime | Mechanical: engine.dba, engine.context | Primitives/Features: PRIM-Database, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: parity must include at least one bounded mutation or sync path
  - Combo: base-envelope field parity plus contract-hardening validators | Pillars: LLM-friendly data, Locus | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: validators should prove the same fields on both backends
  - Combo: `project_profile_kind` plus `mirror_state` parity | Pillars: LLM-friendly data | Mechanical: engine.context, engine.archivist | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: base-envelope parity is not limited to IDs and timestamps
  - Combo: workflow-state triplet parity plus task-board projection | Pillars: Execution / Job Runtime, MicroTask | Mechanical: engine.context, engine.version | Primitives/Features: PRIM-Database, PRIM-StorageTraits | Resolution: IN_THIS_WP | Stub: NONE | Notes: queue semantics must survive projection on both backends
  - Combo: denial-removal plus negative-path explicitness for remaining unsupported ops | Pillars: SQL to PostgreSQL shift readiness, Locus | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-Database, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: unsupported work must become narrow and explicit rather than disappearing into blanket denial
  - Combo: migration bootstrap plus full-suite backend proof | Pillars: SQL to PostgreSQL shift readiness, Execution / Job Runtime | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-Database, PRIM-SqliteDatabase, PRIM-PostgresDatabase | Resolution: IN_THIS_WP | Stub: NONE | Notes: portability claims fail if schema boot and runtime proof are not both green
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside the current packet activation and do not require a new stub or a spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed storage and structured-collaboration packets, current Master Spec v02.179, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | BoardStatus: STUB | Intent: SAME | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: EXPAND_IN_THIS_WP | Stub: NONE | Notes: this is the correct governed shell for the PostgreSQL parity gap
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: defines the artifact family and portable paths that this packet now has to implement on PostgreSQL
  - Artifact: WP-1-Dual-Backend-Tests-v2 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the dual-backend test posture this packet must extend with structured-collaboration parity cases
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: primitive | Verdict: PARTIAL | Notes: the trait already declares the required structured-collaboration readers, but PostgreSQL still inherits the default denial posture
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: execution | Verdict: NOT_PRESENT | Notes: PostgreSQL returns `NotImplemented(\"structured collaboration artifacts\")` for all canonical readers
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: execution | Verdict: IMPLEMENTED | Notes: SQLite is the current semantic reference implementation for the required row shapes and update path
  - Path: ../handshake_main/src/backend/handshake_core/src/workflows.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: execution | Verdict: PARTIAL | Notes: workflows already consume the canonical structured readers, but only after a backend capability gate that PostgreSQL currently fails
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: execution | Verdict: PARTIAL | Notes: bounded task-board update and ready-query helpers still carry SQLite-only assumptions
  - Path: ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | Artifact: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 | Covers: execution | Verdict: PARTIAL | Notes: tests currently assert PostgreSQL capability denial instead of parity
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: The existing stub is correct, but activation must explicitly include bounded task-board sync/update and replacement of denial-focused tests, not just read-method implementation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements backend storage parity and does not add a new GUI surface.
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
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet.
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
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Capability-Boundary-Refactor, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Dual-Backend-Tests
- BUILD_ORDER_BLOCKS: NONE
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md storage portability plus canonical structured collaboration artifact family, base envelope, workflow-state contract, and structured-record readability [CX-DBP-001]/[ADD v02.167]/[ADD v02.168]/[ADD v02.171]/[ADD v02.166]
- WHAT: Implement real PostgreSQL support for canonical Work Packet, Micro-Task, and Task Board structured artifact readers and bounded sync or update paths instead of blanket capability denial.
- WHY: PostgreSQL still cannot participate in canonical structured work-state persistence, so backend portability is honest but incomplete and Locus or Task Board truth remains effectively SQLite-only.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - full PostgreSQL parity for every Locus operation
  - mailbox or structured-artifact viewer UI work
  - CRDT or realtime multi-user conflict resolution beyond the bounded structured record contract claimed here
  - storage-boundary refactoring unrelated to making PostgreSQL a real structured-collaboration backend
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - PostgreSQL no longer blanket-denies the canonical structured-collaboration flows claimed by this packet.
  - The same base-envelope, summary, and workflow-state semantics round-trip on SQLite and PostgreSQL for work-packet and micro-task records.
  - At least one bounded task-board update or sync path becomes portable on both backends, and any remaining unsupported operations are explicit and narrow.
  - Dual-backend tests fail if PostgreSQL regresses back to blanket denial or semantic drift.
- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- SEARCH_TERMS:
  - supports_structured_collab_artifacts
  - structured_collab_work_packet_row
  - structured_collab_micro_task_metadata
  - structured_collab_micro_task_status_rows
  - structured_collab_micro_task_rows
  - locus_task_board_update_work_packet
  - ensure_locus_sqlite
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
- RUN_COMMANDS:
  ```bash
  rg -n "supports_structured_collab_artifacts|structured_collab_work_packet_row|structured_collab_work_packet_rows|structured_collab_micro_task_metadata|structured_collab_micro_task_status_rows|structured_collab_micro_task_rows|locus_task_board_update_work_packet|ensure_locus_sqlite" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Postgres rows drift on base-envelope or workflow-state fields" -> "portable readers and small-model consumers see inconsistent structured truth"
  - "task-board sync remains SQLite-only while readers are implemented" -> "parity claim becomes false because projection truth still diverges by backend"
  - "migration coverage is not portable" -> "backend portability law is violated even if local tests pass"
  - "packet runs in parallel with boundary refactor on overlapping files" -> "implementation races and invalid governance truth"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - Build Order must also record that this packet is downstream of `WP-1-Storage-Capability-Boundary-Refactor` because both scopes overlap on `storage/mod.rs`, `workflows.rs`, and storage test surfaces.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] | WHY_IN_SCOPE: PostgreSQL is still an honest denial for canonical structured work-state storage instead of a real backend participant | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/migrations/ | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: portability remains theoretical and future backend migration debt keeps compounding
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | WHY_IN_SCOPE: canonical Work Packet, Micro-Task, and Task Board records must stop being effectively SQLite-only | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: PostgreSQL may store data but not the canonical portable artifact family the spec actually governs
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | WHY_IN_SCOPE: work-packet and micro-task rows must preserve base-envelope semantics such as `project_profile_kind` and `mirror_state` across backends | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: consumers can parse different field semantics depending on backend
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | WHY_IN_SCOPE: portable structured records must preserve `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on PostgreSQL as well as SQLite | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: Task Board and queue consumers remain backend-sensitive and semantically lossy
  - CLAUSE: Conformance requirement 11 structured-record readability [ADD v02.166] | WHY_IN_SCOPE: canonical Work Packet, Micro-Task, and Task Board state must be readable as structured records on PostgreSQL without falling back to Markdown-only truth | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | RISK_IF_MISSED: structured collaboration remains effectively single-backend or mirror-dependent

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: structured work-packet row and list readers | PRODUCER: storage/postgres.rs and storage/sqlite.rs | CONSUMER: workflows.rs and Locus readers | SERIALIZER_TRANSPORT: SQL row mapping into typed structs | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | DRIFT_RISK: PostgreSQL can diverge on field presence or semantics while still returning a row
  - CONTRACT: structured micro-task metadata, status-row, and list readers | PRODUCER: storage/postgres.rs and storage/sqlite.rs | CONSUMER: workflows.rs and task-board summary code | SERIALIZER_TRANSPORT: SQL row mapping and JSON metadata payloads | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | DRIFT_RISK: micro-task workflow-state or summary semantics drift by backend
  - CONTRACT: task-board update and bounded sync path | PRODUCER: workflows.rs plus backend storage implementations | CONSUMER: task-board projection readers and Command Center style views | SERIALIZER_TRANSPORT: SQL update plus structured projection export | VALIDATOR_READER: storage/tests.rs and task-board tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | DRIFT_RISK: task-board truth remains SQLite-only or semantically inconsistent
  - CONTRACT: structured-collaboration schema and migration coverage | PRODUCER: numbered migrations and storage implementations | CONSUMER: SQLite/PostgreSQL runtime boot plus validators | SERIALIZER_TRANSPORT: migration framework and sqlx schema | VALIDATOR_READER: storage/tests.rs and migration checks | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | DRIFT_RISK: backend portability claims fail at schema bootstrap rather than at query time

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics
  - the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields
  - a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Add or update portable schema and migration coverage for the structured-collaboration tables needed by the packet.
  - Implement PostgreSQL structured work-packet and micro-task readers plus the bounded task-board update or sync path using the same semantic contract as SQLite.
  - Replace denial-focused tests with dual-backend parity and negative-path assertions for any still-unsupported narrow operations.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- CARRY_FORWARD_WARNINGS:
  - Do not widen the top-level storage boundary to compensate for missing parity; keep any new behavior inside the existing governed contract shape or the downstream boundary-refactor packet.
  - Do not claim full PostgreSQL Locus parity; keep remaining unsupported operations explicit, narrow, and tested.
  - Do not preserve SQLite-only task-board sync by silently downgrading the packet to read-only parity.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013]
  - Canonical structured collaboration artifact family [ADD v02.167]
  - Base structured schema and project-profile extension contract [ADD v02.168]
  - Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
  - Conformance requirement 11 structured-record readability [ADD v02.166]
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - confirm PostgreSQL no longer reports blanket structured-collaboration capability denial on the claimed paths

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - Full PostgreSQL parity for every Locus operation remains outside this packet.
  - Historical data migration complexity and performance characteristics are not proven at refinement time.
  - The exact final bounded update or sync surface may narrow during coding if a specific sub-operation is shown to be non-portable within the signed scope.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Main Body already governs portable structured collaboration records and this refinement turns the stub into a bounded parity packet with explicit code surfaces, proof clauses, and negative-path expectations.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already defines the required structured-collaboration record family, base envelope, workflow-state semantics, and structured-record readability. This packet is backend implementation conformance work against that law.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- CONTEXT_START_LINE: 3243
- CONTEXT_END_LINE: 3250
- CONTEXT_TOKEN: Storage Backend Portability Architecture [CX-DBP-001]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]

  **What**
  Defines four mandatory architectural pillars for ensuring database backend flexibility:
  - single storage API
  - portable schema and migrations
  - rebuildable indexes
  - dual-backend testing
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Canonical structured collaboration artifact family [ADD v02.167]
- CONTEXT_START_LINE: 6817
- CONTEXT_END_LINE: 6838
- CONTEXT_TOKEN: Canonical structured collaboration artifact family
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Canonical structured collaboration artifact family** [ADD v02.167]

  - The canonical file standard for Work Packets, Micro-Tasks, and Task Board projections SHALL be versioned JSON documents.
  - Recommended portable Phase 1 layout:
    - `.handshake/gov/work_packets/{wp_id}/packet.json`
    - `.handshake/gov/work_packets/{wp_id}/summary.json`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/packet.json`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/summary.json`
    - `.handshake/gov/task_board/index.json`
  - Every canonical structured collaboration record MUST expose a schema identifier, schema version, stable record identifier, updated timestamp, project profile kind, and references to note or evidence artifacts.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6861
- CONTEXT_TOKEN: Base structured schema and project-profile extension contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope.
  - At minimum that base envelope MUST expose:
    - `schema_id`
    - `schema_version`
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `updated_at`
    - `mirror_state`
    - `authority_refs`
    - `evidence_refs`
  - Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6929
- CONTEXT_END_LINE: 6988
- CONTEXT_TOKEN: Project-agnostic workflow state, queue reason, and governed action contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is.
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.15.10 Conformance requirement 11 structured-record readability [ADD v02.166]
- CONTEXT_START_LINE: 7367
- CONTEXT_END_LINE: 7373
- CONTEXT_TOKEN: Canonical Work Packet, Micro-Task, and Task Board state MUST be readable as structured records
- EXCERPT_ASCII_ESCAPED:
  ```text
  [ADD v02.166] 11. Canonical Work Packet, Micro-Task, and Task Board state MUST be readable as structured records without requiring Markdown parsing as the only machine-readable path.
  [ADD v02.166] 12. Append-only plan, blocker, handoff, review, and decision notes MUST preserve note type, summary, author, and time metadata even when the long-form body is stored in Markdown sidecars.
  ```
