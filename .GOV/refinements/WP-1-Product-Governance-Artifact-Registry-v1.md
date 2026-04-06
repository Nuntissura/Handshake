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
- WP_ID: WP-1-Product-Governance-Artifact-Registry-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-05T12:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- SPEC_TARGET_SHA1: 747a1e77cbe2e1c564d1a99d5c39265edc6aeca2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja050420261939
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Product-Governance-Artifact-Registry-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- The product has a governance pack EXPORT system (`GovernancePackExportRequest/Response` in `governance_pack.rs`) but no IMPORT or REGISTRY system. There is no product-owned surface to hold imported governance artifacts as versioned, typed, provenance-linked structured records.
- Existing structured collaboration record families (`StructuredCollaborationRecordFamily` in `locus/types.rs`) cover work packets, micro-tasks, task board, and role mailbox but have no record family for governance artifact metadata.
- The schema registry function (`structured_collaboration_schema_descriptor`) has no entry for governance artifact records.
- No `GovernanceArtifactKind` enumeration exists to type the different governance artifact categories from spec 7.5.4.3 (codex, protocols, rubrics, checks, templates, schemas).
- The product code does not yet provide a manifest or store contract for versioned governance artifact registries scoped to a project profile.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: current Master Spec v02.179 governance kernel sections, current local product code in `../handshake_main/src/backend/handshake_core/src`, existing structured collaboration and project-profile extension patterns, governance pack export system, and external patterns from Backstage software catalog, OPA policy registries, and Buf schema registries
- REFERENCES: `.GOV/spec/Handshake_Master_Spec_v02.179.md` (sections 1.3, 7.5.4, 7.5.4.8, 10.11), `.GOV/task_packets/stubs/WP-1-Product-Governance-Artifact-Registry-v1.md`, `../handshake_main/src/backend/handshake_core/src/locus/types.rs`, `../handshake_main/src/backend/handshake_core/src/governance_pack.rs`, `../handshake_main/src/backend/handshake_core/src/storage/mod.rs`, `../handshake_main/src/backend/handshake_core/src/capabilities.rs`, `https://backstage.io/docs/features/software-catalog/descriptor-format`, `https://www.openpolicyagent.org/docs/latest/management-bundles/`, `https://buf.build/docs/bsr/overview`, `https://cloud.google.com/artifact-registry/docs/overview`, `https://arxiv.org/abs/2307.10034`
- PATTERNS_EXTRACTED: Backstage uses a descriptor-based catalog with kind/metadata/spec envelope per entity. OPA bundles governance policy as versioned, signed bundles with manifest metadata. Buf schema registry uses typed, versioned schema records with dependency tracking. Google Artifact Registry manages versioned build artifacts across language ecosystems with manifest-based provenance. All four separate definition (what exists) from execution (what runs).
- DECISIONS ADOPT/ADAPT/REJECT: adopt Backstage-style typed artifact descriptors with kind enumeration and provenance metadata; adapt OPA bundle manifest pattern for versioned governance snapshot provenance; adapt Buf registry-first schema contract for governance artifact schema registration; reject dynamic schema resolution and live-watch import patterns that would make the registry non-deterministic
- LICENSE/IP_NOTES: Source review informed architectural choices only. No third-party code or copyrighted text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Handshake_Master_Spec_v02.179.md already defines canonical governance artifacts (7.5.4.3), governance pack project-specific instantiation (7.5.4.8), structured collaboration artifact families ([ADD v02.167]), and project-profile extension contracts ([ADD v02.168]). This WP implements the import/registry side of governance pack instantiation using existing structured collaboration patterns. No new spec law is needed.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 30
- SOURCE_LOG:
  - Source: Google Artifact Registry overview | Kind: BIG_TECH | Date: 2026-04-05 | Retrieved: 2026-04-05T11:00:00Z | URL: https://cloud.google.com/artifact-registry/docs/overview | Why: demonstrates versioned artifact management with manifest-based provenance across language ecosystems, directly analogous to governance artifact versioning and content-hash integrity
  - Source: Backstage descriptor format | Kind: OSS_DOC | Date: 2026-04-05 | Retrieved: 2026-04-05T11:05:00Z | URL: https://backstage.io/docs/features/software-catalog/descriptor-format | Why: demonstrates a stable typed catalog with kind/metadata/spec envelope for software delivery artifacts, directly analogous to governance artifact registry entries
  - Source: OPA Management Bundles | Kind: OSS_DOC | Date: 2026-04-05 | Retrieved: 2026-04-05T11:10:00Z | URL: https://www.openpolicyagent.org/docs/latest/management-bundles/ | Why: governance policy-as-versioned-bundles with manifest metadata and content hashing maps directly to imported governance snapshot provenance
  - Source: Buf Schema Registry | Kind: OSS_DOC | Date: 2026-04-05 | Retrieved: 2026-04-05T11:15:00Z | URL: https://buf.build/docs/bsr/overview | Why: typed versioned schema records with dependency tracking reinforces the registry-first contract for governance schemas and check manifests
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Kind: PAPER | Date: 2024-02-01 | Retrieved: 2026-04-05T11:20:00Z | URL: https://arxiv.org/abs/2307.10034 | Why: warns that overly dynamic schema semantics become hard to reason about and validate, which argues for explicit low-cardinality kind enumeration over free-form schema evolution
  - Source: backstage/backstage | Kind: GITHUB | Date: 2026-04-05 | Retrieved: 2026-04-05T11:25:00Z | URL: https://github.com/backstage/backstage | Why: large OSS implementation surface for descriptor-based extensibility and catalog-style shared parsing, informing the artifact kind + provenance design
- RESEARCH_SYNTHESIS:
  - Governance artifact registries should use typed descriptors with stable identities and version provenance, not raw file copies or live path references.
  - Manifest-level content hashing at import time provides integrity without requiring runtime re-scanning.
  - Separating artifact definition (registry) from artifact execution (runner) is the consensus pattern across OPA, Backstage, Buf, and Google Artifact Registry.
  - Low-cardinality kind enumerations are more deterministically validatable than dynamic schema evolution.
- RESEARCH_GAPS_TO_TRACK:
  - Governance artifact content delivery (how imported artifact bodies are stored and retrieved for execution) is downstream of this registry WP and belongs to Check-Runner and Workflow-Mirror WPs.
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: Backstage descriptor format | Pattern: typed catalog with kind/metadata/spec envelope | Why: directly shapes the `GovernanceArtifactKind` enum and `GovernanceArtifactRegistryEntry` struct design with stable identity, kind, and provenance fields
  - Source: Buf Schema Registry | Pattern: typed versioned schema records with dependency tracking | Why: confirms the schema ID registration approach (`hsk.governance_artifact_registry@1`) for structured collaboration integration
- ADAPT_PATTERNS:
  - Source: OPA Management Bundles | Pattern: versioned signed bundles with manifest metadata | Why: the manifest-level provenance pattern (snapshot version, content hash, import timestamp) is adopted, but OPA's live bundle polling is rejected in favor of snapshot-based import
  - Source: Google Artifact Registry overview | Pattern: manifest-based versioning across ecosystems | Why: the multi-ecosystem manifest approach informs how governance artifacts from different governance kernels can coexist, but the full multi-language packaging is beyond scope
- REJECT_PATTERNS:
  - Source: OPA Management Bundles | Pattern: live bundle discovery and hot-reload | Why: live-watch import would make the registry non-deterministic and violate Handshake's mechanical-layer predictability requirement
  - Source: Validation of Modern JSON Schema: Formalization and Complexity | Pattern: highly dynamic schema behavior and annotation-dependent validation | Why: Handshake should reject extension semantics that are too dynamic to validate deterministically
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - `site:github.com/backstage/backstage software-catalog descriptor kind metadata`
  - `site:github.com/open-policy-agent/opa bundle manifest revision`
  - `site:github.com/bufbuild/buf registry schema module`
- MATCHED_PROJECTS:
  - Source: backstage/backstage | Repo: backstage/backstage | URL: https://github.com/backstage/backstage | Intent: ARCH_PATTERN | Decision: ADOPT | Impact: NONE | Stub: NONE | Notes: kind/metadata/spec descriptor pattern directly shapes the GovernanceArtifactRegistryEntry struct
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder event IDs are required for this WP.
- Governance artifact registry load/save operations can be FR-span-wrapped by calling code but the registry itself does not mandate new FR events.
- Downstream WPs (Check-Runner, Workflow-Mirror) may add FR events when they execute governance checks.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: if imported governance artifacts replace Handshake-native governance instead of extending it, the product becomes a repo-governance shell and loses its broader multi-domain identity. Mitigation: registry entries are SoftwareDelivery profile extensions, never base-envelope, and cannot overwrite native governance layers.
- Risk: if the registry stores repo file paths as runtime authority, future code can bypass the registry and read `.GOV/**` directly. Mitigation: registry stores artifact identity and content hash, not repo file paths as live authority.
- Risk: if GovernanceArtifactKind grows unbounded, validation complexity becomes unmanageable. Mitigation: use a closed enum with known kinds from spec 7.5.4.3. New kinds require an explicit enum variant and review.
- Risk: if the manifest content hash is not verified at load time, tampered governance artifacts can enter runtime. Mitigation: load contract must verify manifest integrity hash before exposing entries.
- Risk: if schema collisions occur with existing structured collaboration records, downstream consumers may misparse governance artifacts. Mitigation: separate schema namespace (`hsk.governance_artifact_registry@1`) and extension namespace (`hsk.ext.software_delivery.governance_artifact_registry@1`).

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - New Rust types (GovernanceArtifactKind, GovernanceArtifactRegistryEntry, GovernanceArtifactRegistryManifest, GovernanceArtifactRegistryStore) are code-level implementations of the existing governance pack spec concept (PRIM-GovernancePackExport family) and the structured collaboration envelope pattern (PRIM-StructuredCollaborationEnvelopeV1).
  - These do not require new spec-level primitive entries until a later enrichment pass adds them to Appendix 12.4.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already names governance pack, structured collaboration, and project-profile primitives. The new governance artifact registry primitives are implementations of the existing governance pack instantiation concept, not a new primitive family requiring an appendix update.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - New primitives follow the existing structured collaboration record family pattern and do not require a separate appendix category.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family was discovered during this refinement. The governance artifact registry is a concrete implementation of the existing governance pack instantiation spec anchor.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: The governance artifact registry is an implementation of the existing governance pack instantiation concept (7.5.4.8), not a new feature family requiring a feature registry entry.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface is implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: The governance artifact registry creates a new structured data store but does not add a new cross-feature interaction class. Downstream consumers (Check-Runner, DCC) will add interaction edges when they consume the registry.
- APPENDIX_MAINTENANCE_NOTES:
  - Keep the current spec unchanged and implement against existing Main Body governance pack and structured collaboration law.
  - If coding reveals a truly missing primitive or interaction edge, treat that as a separate spec-update flow instead of silently widening this packet.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by governance artifact registry work | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement logic is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes governance artifacts downstream but is not affected by the registry itself | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication or export controllers remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: governance artifact provenance is relevant to archival but the archivist engine is not directly modified | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the registry work | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics surfaces consume stored data later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: the registry uses the storage trait boundary but does not modify database abstraction behavior | STUB_WP_IDS: NONE
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: the governance artifact registry is the product-side instantiation of the sovereign engine's governance authority surface; it defines how imported governance artifacts are typed, versioned, and stored | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: governance artifact metadata becomes available as context for downstream model sessions and execution planning | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: governance artifact provenance tracks source snapshot version and content hash, making version lineage explicit | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: NOT_TOUCHED | NOTES: no sandbox or isolation behavior changes are required | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: NOT_TOUCHED | NOTES: no new FR events required; downstream consumers may add spans when executing governance checks | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: governance artifact import timestamps are plain UTC, not calendar events | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: governance artifact registry entries may eventually be stored through Locus, and the schema descriptor registration in locus/types.rs directly extends Locus record families | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: governance artifacts are not Loom recordings; no collision | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: governance artifact registry entries follow the same structured collaboration pattern as product work packets but do not modify the work packet feature contract directly | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board-specific feature contract is modified | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no micro-task feature contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: DCC will consume the registry downstream but no DCC surface is built here | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: NOT_TOUCHED | NOTES: governance artifact execution is out of scope (Check-Runner WP) | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router surface is altered | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: the registry store trait uses the Database trait boundary, keeping PostgreSQL portability intact | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: governance artifact registry entries are structured JSON with schema IDs, following the LLM-friendly-first data mandate | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage workflow surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is touched | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of governance data | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Locus | CAPABILITY_SLICE: governance artifact schema registration in structured collaboration record families | SUBFEATURES: new schema ID constant, new record family variant, schema descriptor function extension | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, PRIM-GovernancePackExport | MECHANICAL: engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends the existing locus/types.rs structured collaboration pattern
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: governance registry store trait behind Database boundary | SUBFEATURES: GovernanceArtifactRegistryStore trait with load/save/lookup/list | PRIMITIVES_FEATURES: PRIM-Database | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: store trait uses the Database trait boundary, keeping PostgreSQL portability intact
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: governance artifact metadata as structured JSON | SUBFEATURES: JSON-serializable registry entries with schema IDs and provenance | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-GovernancePackExport | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: follows the canonical structured collaboration mandate from [ADD v02.167]
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: governance artifact registry load/store | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal structured data store consumed by downstream governance runners and DCC projections
  - Capability: governance artifact kind enumeration and schema registration | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends the structured collaboration schema registry to include governance artifact records
  - Capability: SoftwareDelivery profile extension for governance artifact references | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: profile extension metadata attaches to structured collaboration records as non-breaking SoftwareDelivery-scoped data
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - This packet creates new governance artifact primitives that participate in the existing structured collaboration ecosystem. The main interaction edge is between the new registry and the existing schema descriptor function.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The governance artifact registry extends existing structured collaboration and governance pack primitives within their current interaction class. A new interaction-matrix edge is not required until downstream consumers (Check-Runner, DCC-Backend) create cross-feature interaction surfaces.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_YES: Governance artifact registry benefits from external patterns for typed registries and versioned artifact management to validate the cross-primitive design.
- SOURCE_SCAN:
  - Source: Backstage descriptor format | Kind: OSS_DOC | Angle: typed catalog entity identity contract | Pattern: apiVersion + kind + metadata.name triple as minimum stable identity | Decision: ADOPT | EngineeringTrick: stable three-field identity triple for catalog entities | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly shapes GovernanceArtifactRegistryEntry identity contract
  - Source: OPA Management Bundles | Kind: OSS_DOC | Angle: versioned governance bundle integrity | Pattern: content hash on full bundle manifest for tamper detection | Decision: ADAPT | EngineeringTrick: manifest-level content hash for import-time integrity | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: manifest provenance adopted; live polling rejected for determinism
  - Source: Google Artifact Registry overview | Kind: BIG_TECH | Angle: multi-ecosystem artifact versioning | Pattern: manifest-based version lineage across language ecosystems | Decision: ADAPT | EngineeringTrick: snapshot version plus content hash pair for version lineage | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: informs source_snapshot_version and source_content_hash design
- MATRIX_GROWTH_CANDIDATES:
  - Combo: governance artifact kind x structured collaboration schema descriptor | Sources: Backstage descriptor format | WhatToSteal: stable typed catalog with schema-registered entries | HandshakeCarryOver: extend schema descriptor function with governance artifact schema ID | RuntimeConsequences: governance artifacts become queryable through structured collaboration validation pipeline | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends existing schema descriptor function
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Backstage: stable apiVersion + kind + metadata.name triple as minimum identity contract for any catalog entity
  - OPA: content hash on the entire bundle manifest for tamper detection without per-file scanning
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: GovernanceArtifactRegistry plus StructuredCollaborationEnvelopeV1 | Pillars: Locus, LLM-friendly data | Mechanical: engine.sovereign, engine.context | Primitives/Features: PRIM-StructuredCollaborationEnvelopeV1, PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: governance artifacts join the structured collaboration family as first-class schema-registered records
  - Combo: GovernanceArtifactRegistry plus ProjectProfileExtensionV1 | Pillars: LLM-friendly data | Mechanical: engine.sovereign | Primitives/Features: PRIM-ProjectProfileExtensionV1, PRIM-GovernancePackExportRequest | Resolution: IN_THIS_WP | Stub: NONE | Notes: governance artifact metadata is scoped to SoftwareDelivery profile extension, preserving multi-domain portability
  - Combo: GovernanceArtifactRegistry plus GovernancePackExport | Pillars: Locus, SQL to PostgreSQL shift readiness | Mechanical: engine.sovereign, engine.version | Primitives/Features: PRIM-GovernancePackExport, PRIM-GovernancePackExportRequest | Resolution: IN_THIS_WP | Stub: NONE | Notes: the registry is the import-side complement to the existing governance pack export system
  - Combo: GovernanceArtifactRegistry plus Database trait boundary | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba, engine.version | Primitives/Features: PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: store trait uses the Database trait boundary for portable persistence
  - Combo: GovernanceArtifactRegistry plus StructuredCollaborationSummaryV1 | Pillars: LLM-friendly data, Locus | Mechanical: engine.context | Primitives/Features: PRIM-StructuredCollaborationSummaryV1 | Resolution: IN_THIS_WP | Stub: NONE | Notes: registry entries expose compact summary metadata for local small model consumption
  - Combo: GovernanceArtifactRegistry plus engine.version provenance tracking | Pillars: LLM-friendly data | Mechanical: engine.version, engine.sovereign | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: snapshot version and content hash on the manifest enable deterministic version lineage for imported governance
  - Combo: GovernanceArtifactRegistry plus Check-Runner downstream | Pillars: Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-GovernancePackExport | Resolution: IN_THIS_WP | Stub: NONE | Notes: the registry provides the typed artifact descriptors that Check-Runner will later execute; the CheckManifest kind is defined here but execution is out of scope
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered here belong inside this activation and do not require a new stub or spec update.

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed governance and structured collaboration packets, current Master Spec v02.179, and local product code under `../handshake_main/src/backend/handshake_core`
- MATCHED_STUBS:
  - Artifact: WP-1-Product-Governance-Check-Runner-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: execution of governance checks depends on this registry but is a separate concern
  - Artifact: WP-1-Governance-Workflow-Mirror-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: workflow state mirroring depends on this registry but is a separate concern
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Product-Governance-Snapshot-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: PARTIAL | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the governance pack export system; this WP builds the import/registry complement
  - Artifact: WP-1-Structured-Collaboration-Schema-Registry-v4 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the schema registration pattern this WP extends with governance artifact schema IDs
  - Artifact: WP-1-Structured-Collaboration-Artifact-Family-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the record family pattern this WP extends
  - Artifact: WP-1-Project-Profile-Extension-Registry-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: established the profile extension validation pattern this WP uses for SoftwareDelivery scoping
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/governance_pack.rs | Artifact: WP-1-Product-Governance-Snapshot-v4 | Covers: primitive | Verdict: PARTIAL | Notes: export path exists; import/registry path does not
  - Path: ../handshake_main/src/backend/handshake_core/src/locus/types.rs | Artifact: WP-1-Product-Governance-Snapshot-v4 | Covers: primitive | Verdict: PARTIAL | Notes: schema descriptor function is complete for existing families but does not yet include governance artifact schema ID
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: No duplicate exists. The governance artifact import/registry capability is genuinely missing. Existing export, schema registration, and profile extension patterns provide the foundation this WP builds on. All matched artifacts resolve as KEEP_SEPARATE.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements a backend data store and does not create a new GUI surface directly.
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Snapshot, WP-1-Governance-Kernel-Conformance, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Product-Governance-Check-Runner, WP-1-Governance-Workflow-Mirror, WP-1-Governance-Pack, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
- WHAT: Define and implement a product-owned registry for imported software-delivery governance artifacts (codex, role protocols, rubrics, check manifests, script descriptors, schemas, templates, sync surfaces) as versioned, typed, provenance-linked structured collaboration records scoped to the SoftwareDelivery project profile.
- WHY: Handshake needs a bounded, versioned way to ingest the current repo governance surface so downstream runners and DCC projections can consume governance definitions without treating repo file paths as runtime authority or collapsing Handshake's broader multi-domain identity into software-delivery-only rules.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- OUT_OF_SCOPE:
  - Executing imported checks or scripts (WP-1-Product-Governance-Check-Runner-v1)
  - Workflow state mirroring (WP-1-Governance-Workflow-Mirror-v1)
  - DCC UI projections or typed viewers (WP-1-Dev-Command-Center-Control-Plane-Backend-v1)
  - Replacing or overwriting Handshake-native governance
  - Script content import (only descriptors; content stays in governance pack archives)
  - Multi-provider model execution concerns (downstream session-substrate WPs)
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- DONE_MEANS:
  - GovernanceArtifactKind enum covers all canonical artifact types from spec 7.5.4.3.
  - GovernanceArtifactRegistryManifest can be stored and loaded through the existing storage trait boundary.
  - Schema descriptor for hsk.governance_artifact_registry@1 is registered in the structured collaboration schema registry function.
  - Profile extension validation passes for the governance registry extension on SoftwareDelivery records.
  - Non-SoftwareDelivery project profiles do not see or require governance registry extensions.
  - cargo test and just gov-check pass on the WP branch.
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/lib.rs
- SEARCH_TERMS:
  - GovernanceArtifactKind
  - governance_artifact_registry
  - GovernancePackExportRequest
  - StructuredCollaborationRecordFamily
  - structured_collaboration_schema_descriptor
  - ProjectProfileKind
  - validate_profile_extension
  - hsk.governance_artifact_registry
- RUN_COMMANDS:
  ```bash
  rg -n "GovernanceArtifactKind|governance_artifact_registry" src/backend/handshake_core/src
  rg -n "StructuredCollaborationRecordFamily|structured_collaboration_schema_descriptor" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Imported governance artifacts replace Handshake-native governance" -> "product becomes repo-governance shell and loses multi-domain identity"
  - "Registry stores repo file paths as runtime authority" -> "product code bypasses registry and reads .GOV directly"
  - "GovernanceArtifactKind grows unbounded" -> "validation complexity becomes unmanageable"
  - "Manifest content hash not verified at load time" -> "tampered governance artifacts enter runtime"
  - "Schema ID collision with existing structured collaboration records" -> "downstream consumers misparse governance artifacts"
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and `just orchestrator-prepare-and-packet` will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order still shows this WP as the active packet for the base ID and that downstream Check-Runner, Workflow-Mirror, Governance-Pack, and DCC-Backend dependencies continue to point at the base governance-artifact-registry track.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Governance Pack project-specific instantiation 7.5.4.8 | WHY_IN_SCOPE: the product has an export path but no import/registry for governance artifacts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | RISK_IF_MISSED: downstream Check-Runner and DCC-Backend remain blocked with no structured governance data to consume
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | WHY_IN_SCOPE: governance artifact records must be versioned JSON with schema IDs per the structured collaboration mandate | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/governance_artifact_registry.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema | RISK_IF_MISSED: governance artifacts bypass the structured collaboration family and become ad hoc unvalidated records
  - CLAUSE: Base envelope + profile extension contract [ADD v02.168] | WHY_IN_SCOPE: governance artifact metadata must be SoftwareDelivery profile extension, not base-envelope, to preserve multi-domain portability | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/locus/types.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | RISK_IF_MISSED: non-software projects inherit governance-only required fields and the base envelope is polluted

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: GovernanceArtifactRegistryManifest JSON serialization | PRODUCER: governance_artifact_registry.rs | CONSUMER: Check-Runner (future), DCC-Backend (future), storage layer | SERIALIZER_TRANSPORT: JSON via serde | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry round-trip serialization test | DRIFT_RISK: manifest fields can drift across producer and consumer if schema version is not checked
  - CONTRACT: GovernanceArtifactKind enum serialization | PRODUCER: governance_artifact_registry.rs | CONSUMER: schema descriptor function, profile extension validation | SERIALIZER_TRANSPORT: JSON string via serde | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry kind exhaustiveness test | DRIFT_RISK: new enum variants can be added without updating downstream match arms
  - CONTRACT: StructuredCollaborationSchemaDescriptor extension for governance artifacts | PRODUCER: locus/types.rs schema descriptor function | CONSUMER: structured collaboration validation pipeline | SERIALIZER_TRANSPORT: in-process struct | VALIDATOR_READER: structured collaboration schema tests | TRIPWIRE_TESTS: structured_collaboration_schema test covering governance artifact schema ID | DRIFT_RISK: schema descriptor function can fall out of sync with new record families
  - CONTRACT: GovernanceArtifactRegistryStore trait boundary | PRODUCER: storage implementations | CONSUMER: governance artifact registry load/save callers | SERIALIZER_TRANSPORT: in-process trait object | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry store round-trip test | DRIFT_RISK: store implementations can drift if trait methods change without updating both backends

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a GovernanceArtifactRegistryManifest with SoftwareDelivery profile can be serialized, stored, loaded, and deserialized with schema ID hsk.governance_artifact_registry@1 and all entries retain their artifact_id, kind, provenance, and content_hash
  - a GovernanceArtifactRegistryEntry with kind Codex correctly serializes to and from JSON with the expected schema version
  - profile extension validation passes for the governance registry extension when attached to a SoftwareDelivery structured collaboration record
  - profile extension validation rejects the governance registry extension when attached to a non-SoftwareDelivery record (e.g. Research or Generic)
  - GovernanceArtifactKind enum covers all canonical types from spec 7.5.4.3 and serialization is exhaustive

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - Create governance_artifact_registry.rs with GovernanceArtifactKind enum, GovernanceArtifactRegistryEntry struct, GovernanceArtifactRegistryManifest struct, GovernanceArtifactProvenance struct, and GovernanceArtifactRegistryStore trait.
  - Register the module in lib.rs.
  - Add schema ID constant hsk.governance_artifact_registry@1 and record family variant in locus/types.rs, extend the schema descriptor function.
  - Add profile extension schema hsk.ext.software_delivery.governance_artifact_registry@1 with non-breaking compatibility.
  - Write unit tests for serialization round-trip, kind exhaustiveness, schema registration, profile extension validation, and store trait contract.
- HOT_FILES:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
- CARRY_FORWARD_WARNINGS:
  - Do not put governance artifact metadata into the base structured collaboration envelope. It MUST be a SoftwareDelivery profile extension only.
  - Do not import script or check content into the registry. Only descriptors with provenance metadata belong here.
  - Do not add repo file paths as runtime authority references. Use artifact identity and content hash instead.
  - Do not widen into check execution, workflow mirroring, or DCC projections.

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - GovernanceArtifactKind covers all canonical artifact types from spec 7.5.4.3
  - Registry entries are versioned JSON with schema IDs per [ADD v02.167]
  - Governance artifact metadata is SoftwareDelivery profile extension, not base-envelope, per [ADD v02.168]
  - Store trait uses the Database trait boundary for PostgreSQL portability
  - Non-SoftwareDelivery project profiles do not see or require governance registry extensions
- FILES_TO_READ:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- COMMANDS_TO_RUN:
  - rg -n "GovernanceArtifactKind|governance_artifact_registry" src/backend/handshake_core/src
  - rg -n "hsk.governance_artifact_registry" src/backend/handshake_core/src
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify GovernanceArtifactKind is a closed enum with no catch-all variant
  - verify schema descriptor function includes hsk.governance_artifact_registry@1
  - verify profile extension uses non-breaking compatibility declaration
  - verify no repo file paths appear as runtime authority in registry entries

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - The exact store trait method signatures are not proven until coding. The trait surface (load/save/lookup/list) is directional but may evolve during implementation.
  - Whether the GovernanceArtifactRegistryStore needs SQLite and PostgreSQL implementations in this WP or can defer to a generic JSON-backed store is not proven.
  - The precise interaction between the governance artifact registry and the existing governance pack export flow is not proven. Import may reuse export format metadata or define its own manifest shape.
  - Whether test-only helpers need direct registry construction without going through the store trait will need inspection during implementation.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec Main Body explicitly defines canonical governance artifacts (7.5.4.3), governance pack project-specific instantiation (7.5.4.8), structured collaboration artifact families ([ADD v02.167]), and project-profile extension contracts ([ADD v02.168]). The remaining work is implementing the import/registry complement to the existing export system.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.179.md already defines the governance pack instantiation, structured collaboration, and project-profile extension law this packet needs. No Main Body or appendix update is required before packet activation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
- CONTEXT_START_LINE: 31837
- CONTEXT_END_LINE: 31857
- CONTEXT_TOKEN: versioned bundle of templates
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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)
- CONTEXT_START_LINE: 31726
- CONTEXT_END_LINE: 31740
- CONTEXT_TOKEN: deterministic multi-role collaboration
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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md [ADD v02.167] Canonical structured collaboration artifact family
- CONTEXT_START_LINE: 6817
- CONTEXT_END_LINE: 6838
- CONTEXT_TOKEN: versioned JavaScript Object Notation documents
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Canonical structured collaboration artifact family** [ADD v02.167]

  - The canonical file standard for Work Packets, Micro-Tasks, and Task Board projections SHALL be versioned JavaScript Object Notation documents.
  - Every canonical structured collaboration record MUST expose:
    - a schema identifier and schema version
    - a stable record identifier
    - an updated timestamp
    - a profile kind such as `software_delivery`, `research`, `worldbuilding`, `design`, `generic`, or `custom`
    - references to note sidecars, mirrors, or evidence artifacts when present
  - Project-specific details MUST live inside profile extensions instead of becoming mandatory base-envelope fields.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md [ADD v02.168] Base structured schema and project-profile extension contract
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6884
- CONTEXT_TOKEN: shared base envelope
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope before any profile-specific fields are applied.
  - The base envelope MUST remain valid even when no project-profile extension is present.
  - `profile_extension` payloads MUST declare `extension_schema_id`, `extension_schema_version`, and `compatibility` so migration and validation tooling can reject unknown breaking extensions deterministically.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 1.3 The Four-Layer Architecture
- CONTEXT_START_LINE: 479
- CONTEXT_END_LINE: 493
- CONTEXT_TOKEN: Mechanical Layer
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 1.3 The Four-Layer Architecture

  Handshake has four layers: LLM (decides what), Orchestrator (enforces rules), Mechanical (executes deterministically), and Validation (confirms correctness).

  - **Mechanical Layer**: Deterministic engines (Word, Excel, Docling) that execute operations.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
- CONTEXT_START_LINE: 60488
- CONTEXT_END_LINE: 60502
- CONTEXT_TOKEN: canonical developer/operator surface
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.11 Dev Command Center (Sidecar Integration)

  The Dev Command Center is the canonical developer/operator surface that binds:
  **work (Locus WP/MT)** - **workspaces (git worktrees)** - **execution sessions (agent/model runs)** - **approvals/logs/diffs**
  ```
