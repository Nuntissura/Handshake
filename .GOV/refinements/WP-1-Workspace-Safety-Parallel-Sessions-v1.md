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
- WP_ID: WP-1-Workspace-Safety-Parallel-Sessions-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-16
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-04-07T17:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- SPEC_TARGET_SHA1: 7d6558fab2f3df70669fff6f0a6e6ef9ea395194
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja070420262042
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Workspace-Safety-Parallel-Sessions-v1
- STUB_WP_IDS: NONE

### REQUIRED SECTIONS (per the current ORCHESTRATOR_PROTOCOL refinement workflow)

### GAPS_IDENTIFIED
- Session Spawn Contract (VALIDATED) creates ModelSessions and Terminal LAW (VALIDATED) provides command filtering with denied_command_patterns and CWD scoping. But there is no session-scoped workspace isolation.
- No SessionWorktreeRegistry maps session_id to a dedicated git worktree.
- IN_SCOPE_PATHS defined in spec (4.3.9.2.4 and 4.3.9.17 INV-WS-001) but not enforced per-session at Terminal LAW level.
- Cross-session file access (INV-WS-003) is not enforced.
- Command denylist for spawned/background sessions (4.3.9.17.3) is normative but not implemented as session-scoped enforcement.
- Merge-back discipline (4.3.9.17.4) has no implementation surface.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 2h
- SEARCH_SCOPE: Master Spec v02.179 (sections 4.3.9.17, 4.3.9.2.4, 6.0.2, 4.3.9.14), local product code, git worktree management tools, workspace isolation patterns for parallel AI agents
- REFERENCES: .GOV/spec/Handshake_Master_Spec_v02.180.md, .GOV/task_packets/stubs/WP-1-Workspace-Safety-Parallel-Sessions-v1.md, .GOV/task_packets/WP-1-Session-Spawn-Contract-v1/packet.md, https://worktrunk.dev/, https://github.com/coderabbitai/git-worktree-runner, https://github.com/nwiizo/ccswarm, https://developer.nvidia.com/blog/practical-security-guidance-for-sandboxing-agentic-workflows-and-managing-execution-risk/
- PATTERNS_EXTRACTED: Worktrunk uses three-command model for parallel agent worktree management. ccswarm assigns dedicated worktrees per parallel Claude Code session. Terminal LAW already has denied_command_patterns, allowed_cwd_roots, CWD validation, and session isolation checks. NVIDIA guidance warns path-based denylists are bypassable via symlinks.
- DECISIONS ADOPT/ADAPT/REJECT: adopt git worktree isolation as preferred strategy (each writing session gets git worktree add at spawn time); adopt file-scope lock isolation as fallback extending existing Work Unit lock contract; adapt Terminal LAW denied_command_patterns to be session-scoped; adapt Terminal LAW validate_cwd to validate file write targets against per-session IN_SCOPE_PATHS; reject Docker/container workspace isolation (too heavyweight for initial implementation); reject OS-level primitives Landlock/Seccomp (Windows-first product, cross-platform complexity too high now)
- LICENSE/IP_NOTES: Source review informed architectural choices only. No third-party code or copyrighted text is intended for direct reuse.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Section 4.3.9.17 is comprehensive normative text (ADD v02.137) defining both isolation strategies, command denylist, merge-back discipline, and all three invariants. This WP implements the spec as written.

### RESEARCH_CURRENCY (current external signal scan; mandatory unless the WP is strictly internal/mechanical)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON_NO: N/A
- SOURCE_MAX_AGE_DAYS: 30
- SOURCE_LOG:
  - Source: Worktrunk documentation | Kind: OSS_DOC | Date: 2026-04-07 | Retrieved: 2026-04-07T17:00:00Z | URL: https://worktrunk.dev/ | Why: three-command worktree management model for parallel agents demonstrating practical isolation patterns
  - Source: coderabbitai/git-worktree-runner | Kind: GITHUB | Date: 2026-04-07 | Retrieved: 2026-04-07T17:05:00Z | URL: https://github.com/coderabbitai/git-worktree-runner | Why: practical git worktree lifecycle management for parallel code review agents
  - Source: nwiizo/ccswarm | Kind: GITHUB | Date: 2026-04-07 | Retrieved: 2026-04-07T17:10:00Z | URL: https://github.com/nwiizo/ccswarm | Why: parallel Claude Code sessions each with dedicated worktrees and automatic cleanup
  - Source: NVIDIA Sandboxing Guidance | Kind: BIG_TECH | Date: 2026-04-07 | Retrieved: 2026-04-07T17:15:00Z | URL: https://developer.nvidia.com/blog/practical-security-guidance-for-sandboxing-agentic-workflows-and-managing-execution-risk/ | Why: path-based denylist limitations and mitigation strategies for agentic workspace isolation
  - Source: Docker Sandboxes for AI Agents | Kind: BIG_TECH | Date: 2026-04-07 | Retrieved: 2026-04-07T17:20:00Z | URL: https://www.docker.com/blog/docker-sandboxes-run-agents-in-yolo-mode-safely/ | Why: container-level isolation pattern for AI agent workspaces
  - Source: SoK: Lessons Learned from Android Security Research | Kind: PAPER | Date: 2023-05-01 | Retrieved: 2026-04-07T17:25:00Z | URL: https://arxiv.org/abs/2304.14235 | Why: systematic analysis of isolation enforcement pitfalls and bypass vectors in permission-based systems; advisory-only denylists are a well-documented failure mode confirming the need for OS-level enforcement as a future escalation path
- RESEARCH_SYNTHESIS:
  - Worktree-per-session is the simplest correct approach for git-based workspace isolation
  - File-scope locks are the fallback for non-git worksurfaces and when worktree isolation is impractical
  - String-matching denylists are advisory only; real security requires OS-level primitives
  - For a local-first product like Handshake the pragmatic approach is worktree isolation plus advisory enforcement plus complete audit trail
- RESEARCH_GAPS_TO_TRACK:
  - Non-git worksurface isolation (Design Studio entity locking) is Phase 2+ and not addressed here
  - OS-level sandbox primitives (Landlock, Seccomp) would strengthen enforcement but require cross-platform work
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH (prevent shallow source logging)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, refinement must record at least one adopted pattern, one adapted pattern, and one rejected pattern grounded in the SOURCE_LOG. Do not only list sources; prove how they changed the WP.
- ADOPT_PATTERNS:
  - Source: Worktrunk documentation | Pattern: three-command worktree management model (switch/list/merge) | Why: directly shapes the session worktree allocation and merge-back discipline lifecycle
  - Source: nwiizo/ccswarm | Pattern: dedicated worktrees per parallel agent with automatic cleanup on completion | Why: validates the session-to-worktree mapping and cleanup-on-completion design
- ADAPT_PATTERNS:
  - Source: NVIDIA Sandboxing Guidance | Pattern: path-based denylist with symlink bypass warnings | Why: Terminal LAW denylist is advisory with canonicalization; future OS-level enforcement is documented as a gap
  - Source: coderabbitai/git-worktree-runner | Pattern: worktree lifecycle management with branch-per-review | Why: the branch-per-session pattern informs how sessions produce merge-ready artifacts but Handshake uses session_id-based naming rather than review-based
- REJECT_PATTERNS:
  - Source: Docker Sandboxes for AI Agents | Pattern: container-level filesystem isolation | Why: too heavyweight for initial implementation; requires Docker runtime dependency which conflicts with local-first product posture
  - Source: NVIDIA Sandboxing Guidance | Pattern: OS-level primitives (Landlock, Seccomp) for enforcement | Why: Windows-first product with cross-platform complexity too high now; advisory enforcement with audit trail is pragmatic first step
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING (same-topic repo exploration; feed useful findings back into governance)
- Rule: if RESEARCH_CURRENCY_REQUIRED=YES, inspect topic-adjacent GitHub projects/repos that touch the same intent, implementation topic, or UI surface. This is for discovering better execution patterns, richer feature combinations, and UI/UX force multipliers. Useful findings MUST flow back into spec/governance through scope expansion, new stubs, spec updates, or UI enrichment.
- SEARCH_QUERIES:
  - site:github.com/nwiizo/ccswarm parallel agent worktree isolation
  - site:github.com/coderabbitai/git-worktree-runner lifecycle management
  - site:github.com workspace isolation parallel AI sessions
- MATCHED_PROJECTS:
  - Source: nwiizo/ccswarm | Repo: nwiizo/ccswarm | URL: https://github.com/nwiizo/ccswarm | Intent: ARCH_PATTERN | Decision: ADOPT | Impact: NONE | Stub: NONE | Notes: parallel Claude Code sessions with dedicated worktrees validates session-to-worktree isolation design
  - Source: coderabbitai/git-worktree-runner | Repo: coderabbitai/git-worktree-runner | URL: https://github.com/coderabbitai/git-worktree-runner | Intent: ARCH_PATTERN | Decision: ADAPT | Impact: NONE | Stub: NONE | Notes: worktree lifecycle management for code review agents informs session cleanup patterns
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Flight Recorder events needed for workspace isolation lifecycle:
  - Reuse existing FR-EVT-SESS-001..005 for session lifecycle (already defined in spec 4.3.9.18.4)
  - Add workspace isolation decision events within FR-EVT-SESS-001 payload (isolation_mode: worktree|file_lock|none)
  - Add denylist violation events as capability action events (existing CapabilityAction pattern)
  - Add merge-back events as governance action events (merge artifact produced, conflict detected, merge completed)
  - No new top-level FR event type IDs required; extend existing session and capability event payloads

### RED_TEAM_ADVISORY (security failure modes)
- Risk: worktree creation fails silently, session proceeds without isolation violating INV-WS-002. Mitigation: fail-closed -- if worktree allocation fails, session MUST NOT start write operations, return BLOCKED.
- Risk: path-based denylist bypass via symlinks, environment variables, or process indirection. Mitigation: document as advisory enforcement, add canonicalization in validate_cwd, full OS-level sandboxing is future concern.
- Risk: session orphans -- if session dies without merge, worktree persists and wastes disk. Mitigation: session lifecycle events trigger cleanup, TTL-based garbage collection as fallback.
- Risk: cross-session reads (INV-WS-003) may be needed for legitimate purposes. Mitigation: deny by default, allow only with explicit operator approval plus FR event.
- Risk: merge conflicts silently resolved by automated tooling, losing work. Mitigation: merge conflicts surface as BLOCKED state per 4.3.9.17.4, no silent resolution allowed.
- Risk: lock contention on shared branches when file-scope fallback is active. Mitigation: lock is advisory with session_id ownership, contention detected and one session blocked deterministically per INV-MM-003.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_CREATED (IDs):
  - NONE
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - New Rust types (SessionWorktreeAllocation, SessionScopedDenylist, MergeBackArtifact) are code-level implementations of the existing workspace safety spec (4.3.9.17). They extend Terminal LAW and Session Spawn Contract, not new primitive families.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already names session, terminal, and capability primitives. Workspace safety types are implementations of existing spec normative text.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - New types follow existing session and terminal primitive patterns and do not require a separate appendix category.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_RESOLUTION: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_ATTACHED_THIS_PASS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_STUB_WP_IDS: NONE
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_REASON: No new primitive family discovered. Workspace safety implements existing spec section 4.3.9.17.

### APPENDIX_MAINTENANCE (spec appendix follow-through)
- Rule: if any appendix action below is `UPDATED`, this refinement is declaring a Master Spec version bump. In that case set `APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE`, set `SPEC_IMPACT=YES`, set `ENRICHMENT_NEEDED=YES`, and include the verbatim appendix update text in `PROPOSED_SPEC_ENRICHMENT`. Packet creation stays blocked until the new spec version exists and `SPEC_CURRENT` is advanced.
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- FEATURE_REGISTRY_REASON_NO_CHANGE: Workspace safety implements existing normative text (4.3.9.17), not a new feature family requiring a feature registry entry.
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- UI_GUIDANCE_REASON: No direct GUI surface implemented in this packet.
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- INTERACTION_MATRIX_REASON_NO_CHANGE: New interaction edges (SessionSpawn x WorkspaceIsolation, TerminalLAW x SessionScopedDenylist, WorkspaceIsolation x FlightRecorder, WorkspaceIsolation x MergeBackArtifact) are identified in DISCOVERY_MATRIX_EDGES but IMX-### IDs will be assigned during a future spec enrichment pass. Appendix 12.6 update deferred.
- APPENDIX_MAINTENANCE_NOTES:
  - IMX edges IMX-WS-001..004 are new interaction edges tracked in this refinement. All workspace safety requirements are already normative in section 4.3.9.17. No spec version bump is required because ENRICHMENT_NEEDED=NO and the interaction edges are implementation-level, not spec-law-level additions.
- APPENDIX_MAINTENANCE_VERDICT: OK

### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)
- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.
- Required rubric lines (one per engine; do not delete lines, fill values):
  - ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: workspace safety is a governance enforcement surface; isolation rules are sovereign-level authority | STUB_WP_IDS: NONE
  - ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: worktree isolation and command denylist ARE sandbox primitives; this WP implements the sandbox engine workspace isolation contract | STUB_WP_IDS: NONE
  - ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: session workspace state (isolation mode, scope paths, denylist) becomes context for session planning | STUB_WP_IDS: NONE
  - ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: worktree-based isolation directly involves git version control; merge-back produces version artifacts | STUB_WP_IDS: NONE
  - ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: no spatial or scene capability is changed by workspace safety work | STUB_WP_IDS: NONE
  - ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: no fabrication or procedure-authoring surface is affected | STUB_WP_IDS: NONE
  - ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: no simulation or measurement logic is involved | STUB_WP_IDS: NONE
  - ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: simulation runtimes are downstream consumers only | STUB_WP_IDS: NONE
  - ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: NOT_TOUCHED | NOTES: no hardware-facing execution surface changes here | STUB_WP_IDS: NONE
  - ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: orchestration consumes workspace state downstream but is not directly modified | STUB_WP_IDS: NONE
  - ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: no media composition surface is involved | STUB_WP_IDS: NONE
  - ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: no creative rendering surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: publication or export controllers remain downstream consumers | STUB_WP_IDS: NONE
  - ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: no recipe or cooking workflow surface is relevant | STUB_WP_IDS: NONE
  - ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: no food-compliance surface is changed | STUB_WP_IDS: NONE
  - ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: no delivery or fulfillment engine behavior is altered directly | STUB_WP_IDS: NONE
  - ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: NOT_TOUCHED | NOTES: merge-back artifacts are relevant to archival but archivist engine is not directly modified | STUB_WP_IDS: NONE
  - ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: NOT_TOUCHED | NOTES: retrieval remains downstream of the workspace safety work | STUB_WP_IDS: NONE
  - ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: NOT_TOUCHED | NOTES: analytics surfaces consume isolation audit data later but are not changed here | STUB_WP_IDS: NONE
  - ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: NOT_TOUCHED | NOTES: no dataset-ingestion or wrangling contract is modified | STUB_WP_IDS: NONE
  - ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: NOT_TOUCHED | NOTES: the registry uses the storage trait boundary but does not modify database abstraction behavior | STUB_WP_IDS: NONE
  - ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: NOT_TOUCHED | NOTES: no tutoring or explanation interface is implemented here | STUB_WP_IDS: NONE
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: isolation decisions, denylist violations, merge-back actions, and cross-session access decisions emit FR events | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC surfaces session workspace status, merge review, and violation alerts | STUB_WP_IDS: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: session spawn lifecycle extended with workspace allocation and deallocation phases | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: merge-back artifacts may be stored through Locus as governance evidence | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: workspace isolation timestamps are plain UTC, not calendar events | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: no code-editor surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: no document editor surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: no spreadsheet surface is changed | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: workspace isolation artifacts are not Loom recordings; no collision | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: workspace safety does not modify the work packet feature contract directly | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: no task-board-specific feature contract is modified | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: no micro-task feature contract is changed directly | STUB_WP_IDS: NONE
  - PILLAR: Front End Memory System | STATUS: NOT_TOUCHED | NOTES: no FEMS surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: no prompt or spec-router surface is altered | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: NOT_TOUCHED | NOTES: session worktree registry uses in-memory runtime state; persistence goes through Database trait boundary already covered by existing primitives | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: NOT_TOUCHED | NOTES: workspace state is structured data but no new LLM-facing schema is introduced directly | STUB_WP_IDS: NONE
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: no stage workflow surface is affected | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: no studio runtime or creative console behavior is touched | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: no design or capture surface is modified | STUB_WP_IDS: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: no training or distillation pipeline depends directly on this work | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: no ACE execution surface is modified directly | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: retrieval layers remain downstream consumers of workspace data | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION (deeper pillar -> subfeature/capability slice mapping)
- Rule: for `REFINEMENT_FORMAT_VERSION >= 2026-03-08`, decompose touched or adjacent pillars into concrete capability slices so Appendix 12 can grow beyond coarse pillar rows. This is where Calendar/Loom/Locus/Stage/Studio/Atelier-Lens/Command Center/Flight Recorder/RAG mixes become explicit. Silent omission is forbidden; every row must resolve through `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`.
- Required row format:
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: session workspace allocation and deallocation | SUBFEATURES: SessionWorktreeAllocation at spawn, cleanup at completion/cancellation, orphan detection | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-TerminalCommandEvent | MECHANICAL: engine.sandbox, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends session spawn lifecycle with workspace isolation phase
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: workspace isolation lifecycle events | SUBFEATURES: isolation decision events in session payload, denylist violation events, merge-back events | PRIMITIVES_FEATURES: PRIM-ModelSession | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends existing FR-EVT-SESS event payloads with workspace isolation state
  - PILLAR: Command Center | CAPABILITY_SLICE: session workspace status display | SUBFEATURES: isolation mode indicator, merge review panel, violation alerts, worktree health | PRIMITIVES_FEATURES: PRIM-ModelSession | MECHANICAL: engine.sovereign | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: backend data surfaces for DCC downstream consumption
  - PILLAR: Locus | CAPABILITY_SLICE: merge-back artifact storage and provenance | SUBFEATURES: merge-ready diff/patch stored as governance evidence, session_id linkage, conflict report persistence | PRIMITIVES_FEATURES: PRIM-Database, PRIM-ModelSession | MECHANICAL: engine.version, engine.sovereign | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: merge-back artifacts stored through Locus via Database trait boundary as governance evidence records
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT (job/workflow/tool/runtime visibility mapping)
- Rule: every new or expanded capability must map to a Handshake runtime execution surface so local models, cloud models, and operators can invoke and observe it. This section is mandatory even when `ENRICHMENT_NEEDED=NO`.
- Required row format:
  - Capability: session worktree allocation | JobModel: SessionSpawn extended | Workflow: spawn phase | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: SESSION_PANEL | FlightRecorder: FR-EVT-SESS-001 extended | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: worktree created at session spawn, tracked in SessionWorktreeRegistry
  - Capability: IN_SCOPE_PATHS per-session enforcement | JobModel: NONE | Workflow: Terminal LAW pre-execution | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: existing CapabilityAction | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends Terminal LAW validate_cwd with per-session file write target validation
  - Capability: session-scoped command denylist | JobModel: NONE | Workflow: Terminal LAW pre-execution | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: VIOLATION_ALERT | FlightRecorder: existing CapabilityAction | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: spawned sessions get spec-mandated denylist injected at creation
  - Capability: merge-back artifact production | JobModel: NONE | Workflow: session completion | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: MERGE_PANEL | FlightRecorder: governance action event | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: session produces merge-ready diff/patch with provenance at completion
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - Workspace safety creates new enforcement surfaces bridging session spawn, Terminal LAW, and merge-back discipline.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: Four conceptual interaction edges identified (SessionSpawn x WorkspaceIsolation, TerminalLAW x SessionScopedDenylist, WorkspaceIsolation x FlightRecorder, WorkspaceIsolation x MergeBackArtifact) but formal IMX-### IDs deferred to a future spec enrichment pass when Appendix 12.6 is updated.

### MATRIX_RESEARCH_RUBRIC (external combo research; separate from local matrix scan)
- Rule: inspect vendor docs/papers, university/lab work, official design systems, and high-signal GitHub repos when relevant. This section records what those systems combine, what Handshake should steal or reject, and which engineering tricks should carry over into primitives/tools/features/runtime surfaces. Link dumping is forbidden; every useful row must resolve explicitly.
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON_YES: Workspace safety benefits from external patterns for parallel agent isolation to validate cross-primitive design.
- SOURCE_SCAN:
  - Source: Worktrunk documentation | Kind: OSS_DOC | Angle: parallel agent worktree management | Pattern: three-command model (switch/list/merge) for agent isolation | Decision: ADOPT | EngineeringTrick: minimal command surface for worktree lifecycle management | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: directly shapes worktree allocation/deallocation lifecycle
  - Source: nwiizo/ccswarm | Kind: GITHUB | Angle: parallel Claude Code session isolation | Pattern: dedicated worktrees per session with automatic cleanup | Decision: ADOPT | EngineeringTrick: session_id-based worktree naming with TTL-based cleanup | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: validates session-to-worktree registry design
  - Source: NVIDIA Sandboxing Guidance | Kind: BIG_TECH | Angle: agentic workspace safety | Pattern: path-based denylist with OS-level enforcement escalation | Decision: ADAPT | EngineeringTrick: advisory enforcement at application level with audit trail, OS-level as future escalation | ROI: MEDIUM | Resolution: IN_THIS_WP | Stub: NONE | Notes: pragmatic first-step enforcement with documented limitation
- MATRIX_GROWTH_CANDIDATES:
  - Combo: SessionSpawn x WorktreeIsolation | Sources: Worktrunk documentation, nwiizo/ccswarm | WhatToSteal: worktree-per-session allocation at spawn time | HandshakeCarryOver: session spawn lifecycle extended with workspace allocation phase | RuntimeConsequences: every writing session gets deterministic isolation | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: primary combo for this WP
  - Combo: TerminalLAW x SessionScopedDenylist | Sources: NVIDIA Sandboxing Guidance | WhatToSteal: per-session denylist injection with bypass detection | HandshakeCarryOver: Terminal LAW denied_command_patterns extended to session-scoped configuration | RuntimeConsequences: spawned sessions cannot execute destructive commands without operator approval | ROI: HIGH | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends existing Terminal LAW enforcement surface
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Worktrunk: minimal three-command worktree lifecycle model for clean separation
  - ccswarm: session_id-based worktree naming with automatic cleanup on session end
  - NVIDIA: advisory enforcement at application level with full audit trail as pragmatic first step
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION (high-ROI combinations must resolve explicitly)
- Rule: every high-ROI combination found across pillars, mechanical engines, primitives, tools, and features must end in exactly one resolution path: `IN_THIS_WP`, `NEW_STUB`, or `SPEC_UPDATE_NOW`. Silent drop is forbidden.
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: SessionSpawnContract plus WorktreeIsolation | Pillars: Execution / Job Runtime | Mechanical: engine.sandbox, engine.version | Primitives/Features: PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: spawn triggers worktree allocation for writing sessions
  - Combo: TerminalLAW plus SessionScopedDenylist | Pillars: Execution / Job Runtime | Mechanical: engine.sandbox, engine.sovereign | Primitives/Features: PRIM-TerminalCommandEvent, PRIM-CapabilityRegistry | Resolution: IN_THIS_WP | Stub: NONE | Notes: per-session denylist injection for spawned sessions
  - Combo: WorkspaceIsolation plus FlightRecorder | Pillars: Flight Recorder | Mechanical: engine.sovereign | Primitives/Features: PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: all isolation decisions logged for audit trail
  - Combo: WorkspaceIsolation plus MergeBackDiscipline | Pillars: Execution / Job Runtime, Locus | Mechanical: engine.version | Primitives/Features: PRIM-ModelSession, PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: session completion produces merge-ready artifact with provenance
  - Combo: WorkspaceIsolation plus Database trait boundary | Pillars: SQL to PostgreSQL shift readiness | Mechanical: engine.dba | Primitives/Features: PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: session worktree registry and merge artifacts stored through Database trait boundary
  - Combo: SessionScopedDenylist plus CapabilityGate | Pillars: Execution / Job Runtime, Command Center | Mechanical: engine.sandbox, engine.sovereign | Primitives/Features: PRIM-CapabilityRegistry, PRIM-TerminalCommandEvent | Resolution: IN_THIS_WP | Stub: NONE | Notes: denylist violations surface through CapabilityGate BLOCKED state and Command Center violation alerts
  - Combo: WorktreeIsolation plus CommandCenter session panel | Pillars: Command Center, Execution / Job Runtime | Mechanical: engine.sovereign | Primitives/Features: PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: worktree health, isolation mode, and merge review surfaces served by backend data to Command Center downstream
  - Combo: MergeBackArtifact plus Locus governance evidence | Pillars: Locus, Flight Recorder | Mechanical: engine.version, engine.sovereign | Primitives/Features: PRIM-Database, PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: merge-back artifacts stored as governance evidence in Locus via Database trait boundary for audit and replay
  - Combo: SessionWorktreeRegistry plus OrphanDetection | Pillars: Execution / Job Runtime | Mechanical: engine.sandbox, engine.version | Primitives/Features: PRIM-ModelSession, PRIM-Database | Resolution: IN_THIS_WP | Stub: NONE | Notes: TTL-based orphan detection for sessions that die without completing merge-back prevents disk waste
  - Combo: CrossSessionAccessDenial plus OperatorApprovalOverride | Pillars: Execution / Job Runtime, Flight Recorder | Mechanical: engine.sovereign, engine.sandbox | Primitives/Features: PRIM-ModelSession, PRIM-CapabilityRegistry | Resolution: IN_THIS_WP | Stub: NONE | Notes: INV-WS-003 deny-by-default with explicit operator approval plus FR event provides controlled override path
  - Combo: WorkspaceIsolationState plus SessionContext | Pillars: Execution / Job Runtime | Mechanical: engine.context, engine.sandbox | Primitives/Features: PRIM-ModelSession | Resolution: IN_THIS_WP | Stub: NONE | Notes: session workspace state (isolation mode, worktree path, scope paths, denylist config) becomes context for downstream model session planning and governance decisions
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI combinations discovered belong inside this activation and cover all touched pillars (Flight Recorder, Command Center, Execution/Job Runtime, Locus) and engines (engine.sovereign, engine.sandbox, engine.context, engine.version).

### EXISTING_CAPABILITY_ALIGNMENT (dedupe against stubs, packets, UI intent, and product code)
- Rule: before creating a new stub or activating a new packet, scan existing stubs, active packets, completed packets, primitive/index coverage, interaction-matrix coverage, same-intent UI surfaces, and product code. If an equivalent capability already exists and code/UI evidence confirms it, reuse the existing artifact instead of creating a duplicate. If only partial coverage exists, expand this WP. If the gap is real, create a stub and/or spec update.
- SCAN_SCOPE: current stub backlog, completed session and terminal packets, current Master Spec v02.179, and local product code
- MATCHED_STUBS:
  - Artifact: WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: DCC backend depends on workspace safety for session status display but is a separate concern
  - Artifact: WP-1-Session-Observability-Spans-FR-v1 | BoardStatus: STUB | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: N/A | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: session observability spans depend on session lifecycle including workspace allocation but is a separate concern
- MATCHED_ACTIVE_PACKETS:
  - NONE
- MATCHED_COMPLETED_PACKETS:
  - Artifact: WP-1-Session-Spawn-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the session lifecycle this WP extends with workspace allocation
  - Artifact: WP-1-Terminal-LAW-v3 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides the command filtering infrastructure this WP extends with session-scoped denylist
  - Artifact: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides capability gating per session that workspace safety uses for operator approval override
  - Artifact: WP-1-Unified-Tool-Surface-Contract-v1 | BoardStatus: VALIDATED | Intent: PARTIAL | PrimitiveIndex: COVERED | Matrix: COVERED | UI: NONE | CodeReality: IMPLEMENTED | Resolution: KEEP_SEPARATE | Stub: NONE | Notes: provides Tool Gate for command denylist enforcement
- CODE_REALITY_EVIDENCE:
  - Path: ../handshake_main/src/backend/handshake_core/src/terminal/guards.rs | Artifact: WP-1-Terminal-LAW-v3 | Covers: primitive | Verdict: IMPLEMENTED | Notes: TerminalGuard with denied_command_patterns, allowed_cwd_roots, CWD validation, session isolation checks exist; no per-session IN_SCOPE_PATHS enforcement
  - Path: ../handshake_main/src/backend/handshake_core/src/jobs.rs | Artifact: WP-1-Session-Spawn-Contract-v1 | Covers: primitive | Verdict: IMPLEMENTED | Notes: ModelSession with session_id and job lifecycle management exist; no workspace allocation at spawn
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- EXISTING_CAPABILITY_ALIGNMENT_REASON: No duplicate exists. Session-scoped workspace isolation is genuinely missing. Existing session spawn, terminal LAW, and capability gating provide the foundation.

### GUI_IMPLEMENTATION_ADVICE_RUBRIC (research-backed GUI implementation advice)
- Rule: separate hidden interaction requirements and implementation tricks from the concrete UI surface checklist. Inspect reference products/repos/design systems/papers when possible, capture hidden semantics, state models, accessibility/keyboard behavior, tooltip-vs-inline strategy, and spell out what Handshake should copy or adapt.
- GUI_ADVICE_REQUIRED: NO
- GUI_ADVICE_REASON_NO: No direct GUI is implemented in this packet. DCC session panel is downstream.
- GUI_REFERENCE_SCAN:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet implements backend isolation enforcement and does not create a new GUI surface directly.
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Session-Spawn-Contract, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Terminal-LAW, WP-1-Unified-Tool-Surface-Contract
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_ANCHOR_PRIMARY: 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions
- WHAT: Workspace isolation for parallel sessions with worktree allocation, session-scoped denylist, IN_SCOPE_PATHS enforcement, and merge-back discipline
- WHY: Parallel sessions touching the same workspace can silently overwrite or conflict without deterministic isolation and fail-closed execution rules
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workspace_safety.rs
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/Cargo.toml
- OUT_OF_SCOPE:
  - Non-git worksurface isolation (Design Studio entity locking, Phase 2+)
  - OS-level sandbox primitives (Landlock, Seccomp, macOS Seatbelt)
  - Docker/container workspace isolation
  - DCC UI for workspace status (downstream WP)
  - Multi-repo workspace management
- TEST_PLAN:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
  ```
- DONE_MEANS:
  - SessionWorktreeAllocation creates dedicated git worktree per writing session at spawn time
  - SessionWorktreeRegistry maps session_id to worktree_path in runtime state
  - IN_SCOPE_PATHS validated per-session by Terminal LAW for file write targets
  - Session-scoped command denylist injected for spawned/background sessions
  - Cross-session file access denied by default (INV-WS-003) with operator approval override
  - Merge-back produces merge-ready diff/patch artifact with provenance
  - Merge conflicts surface as BLOCKED state with explicit conflict report
  - Worktree cleanup on session completion/cancellation with orphan detection
  - Fail-closed exec enforcement (INV-WS-002) if isolation cannot be established
  - All isolation decisions logged via Flight Recorder
  - All storage goes through Database trait boundary
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-TerminalCommandEvent
  - PRIM-CapabilityRegistry
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/flight_recorder.rs
  - src/backend/handshake_core/src/storage/mod.rs
- SEARCH_TERMS:
  - TerminalGuard
  - TerminalConfig
  - denied_command_patterns
  - allowed_cwd_roots
  - validate_cwd
  - ModelSession
  - session_id
  - CapabilityGate
  - FlightRecorderEvent
  - WorkUnitLock
  - IN_SCOPE_PATHS
- RUN_COMMANDS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety && cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
  ```
- RISK_MAP:
  - Worktree creation failure without fail-closed -> silent isolation bypass (HIGH, mitigated by INV-WS-002 enforcement)
  - Denylist bypass via symlinks/env -> destructive command execution (MEDIUM, mitigated by advisory enforcement plus audit trail)
  - Session orphan worktrees -> disk waste (LOW, mitigated by cleanup on completion plus TTL fallback)
  - Merge conflict silent resolution -> lost work (HIGH, mitigated by BLOCKED state enforcement)
- BUILD_ORDER_SYNC_REQUIRED: YES
- BUILD_ORDER_SYNC_NOTES:
  - Packet activation will move this item out of STUB and just orchestrator-prepare-and-packet will sync Task Board and Build Order truth.
  - After packet creation, verify the Build Order still shows this WP as the active packet for the base ID and that downstream DCC-Backend dependency continues to point at the workspace-safety track.

### CLAUSE_PROOF_PLAN (diff-scoped spec proof seed for coder + validator; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate the exact clauses this WP claims to satisfy, why they are in scope, where the implementation should land, what tests should prove them, and the failure mode if they are missed.
- CLAUSE_ROWS:
  - CLAUSE: Workspace Safety Boundaries 4.3.9.17.2 (isolation strategies) | WHY_IN_SCOPE: parallel sessions touching same workspace have no deterministic isolation contract today | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: parallel sessions silently overwrite each other with no enforcement boundary
  - CLAUSE: Command Denylist 4.3.9.17.3 | WHY_IN_SCOPE: spawned sessions currently inherit full terminal permissions with no session-scoped denylist | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | RISK_IF_MISSED: spawned sessions can execute destructive commands violating spec 4.3.9.17.3
  - CLAUSE: Merge-Back Discipline 4.3.9.17.4 | WHY_IN_SCOPE: no merge artifact production or conflict surfacing exists in session lifecycle today | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/model_session.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: session completion produces no provenance artifact and merge conflicts are silently discarded
  - CLAUSE: INV-WS-001 IN_SCOPE_PATHS enforcement | WHY_IN_SCOPE: Terminal LAW validate_cwd does not currently enforce per-session IN_SCOPE_PATHS at file write target level | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workspace_safety.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal | RISK_IF_MISSED: sessions write outside declared scope violating INV-WS-001
  - CLAUSE: INV-WS-002 fail-closed exec | WHY_IN_SCOPE: no fail-closed guard exists when isolation strategy cannot be established | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: execution proceeds on host without any isolation in place violating INV-WS-002
  - CLAUSE: INV-WS-003 cross-session access denial | WHY_IN_SCOPE: no enforcement exists preventing one session reading another session uncommitted worktree changes | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workspace_safety.rs; src/backend/handshake_core/src/mex/gates.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety | RISK_IF_MISSED: cross-session file reads proceed without operator approval violating INV-WS-003

### CONTRACT_SURFACES (serialization/producer/consumer checklist; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: enumerate every contract surface likely to drift silently across producer/consumer/validator/test boundaries.
- CONTRACT_ROWS:
  - CONTRACT: SessionWorktreeAllocation | PRODUCER: workspace_safety.rs session spawn handler | CONSUMER: Terminal LAW validate_cwd, MergeBack, Flight Recorder | SERIALIZER_TRANSPORT: in-process struct via SessionWorktreeRegistry | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety session spawn round-trip test | DRIFT_RISK: worktree path field can drift if session_id naming convention changes across producer and consumer
  - CONTRACT: SessionScopedDenylist | PRODUCER: workspace_safety.rs spawn-time denylist injector | CONSUMER: terminal.rs TerminalGuard, mex/gates.rs Tool Gate | SERIALIZER_TRANSPORT: in-process struct injected into TerminalConfig | VALIDATOR_READER: terminal tests | TRIPWIRE_TESTS: terminal denylist injection unit test for spawned sessions | DRIFT_RISK: new denylist patterns added to spec but not injected at spawn time; Terminal LAW config fields renamed without updating injector
  - CONTRACT: MergeBackArtifact | PRODUCER: workspace_safety.rs session completion handler | CONSUMER: Operator review surface, DCC-Backend (downstream), Flight Recorder, storage layer | SERIALIZER_TRANSPORT: JSON via serde through Database trait boundary | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety merge artifact serialization round-trip test | DRIFT_RISK: provenance fields drift between producer and DCC-Backend consumer if schema version not checked at load time
  - CONTRACT: SessionWorktreeRegistry | PRODUCER: workspace_safety.rs runtime state manager | CONSUMER: all workspace safety enforcement, cleanup, orphan detection | SERIALIZER_TRANSPORT: in-process state map with Database trait boundary for persistence | VALIDATOR_READER: workspace_safety tests | TRIPWIRE_TESTS: workspace_safety registry load/save round-trip test | DRIFT_RISK: registry lookup returns stale worktree_path if session lifecycle events do not update registry atomically

### SEMANTIC_PROOF_PLAN (diff-scoped semantic proof assets; required for REFINEMENT_FORMAT_VERSION >= 2026-03-16)
- Rule: record the concrete semantic proof assets this WP expects to rely on so later phases do not confuse green gates with semantic closure.
- Rule: each in-scope clause should be backed by one or more executable tripwires, canonical contract examples, or explicit governed debt if proof must remain partial.
- SEMANTIC_TRIPWIRE_TESTS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
  ```
- CANONICAL_CONTRACT_EXAMPLES:
  - SessionWorktreeAllocation created at session spawn fails-closed if git worktree add returns error (INV-WS-002)
  - Two parallel sessions with overlapping IN_SCOPE_PATHS get isolated worktrees; direct write conflict is structurally prevented
  - SessionScopedDenylist for a spawned session blocks git reset --hard and rm -rf outside scope; denylist violation logged via FR
  - MergeBackArtifact produced at session completion contains session_id, worktree_path, diff_patch, and provenance timestamp
  - Merge conflict detected at merge-back surfaces BLOCKED state with explicit conflict report; no silent resolution occurs
  - Cross-session file access attempt denied by default; operator approval required plus FR event emitted
  - Worktree cleanup triggered on session completion, cancellation, and timeout; orphan detection fires on TTL expiry

### CODER_HANDOFF_BRIEF (execution brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- IMPLEMENTATION_ORDER:
  - 1. SessionWorktreeRegistry runtime state (session_id to worktree_path mapping)
  - 2. SessionWorktreeAllocation at session spawn (git worktree add)
  - 3. IN_SCOPE_PATHS per-session enforcement in Terminal LAW (extend validate_cwd)
  - 4. SessionScopedDenylist injection for spawned/background sessions
  - 5. Cross-session file access denial (INV-WS-003)
  - 6. Fail-closed exec enforcement (INV-WS-002)
  - 7. MergeBackArtifact production at session completion
  - 8. Merge conflict detection and BLOCKED state
  - 9. Worktree cleanup on session completion/cancellation
  - 10. Integration tests validating parallel session isolation
- HOT_FILES:
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml model_session
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
- CARRY_FORWARD_WARNINGS:
  - Do not allow worktree creation failure to silently proceed with write operations
  - Do not allow cross-session file access without explicit operator approval
  - Do not silently resolve merge conflicts
  - All storage must go through Database trait boundary

### VALIDATOR_HANDOFF_BRIEF (inspection brief copied into packet; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- CLAUSES_TO_INSPECT:
  - 4.3.9.17.2 Session Isolation Strategies
  - 4.3.9.17.3 Command Denylist
  - 4.3.9.17.4 Merge-Back Discipline
  - 4.3.9.17.5 Invariants INV-WS-001, INV-WS-002, INV-WS-003
  - 4.3.9.2.4 Work Unit lock contract
- FILES_TO_READ:
  - src/backend/handshake_core/src/workspace_safety.rs
  - src/backend/handshake_core/src/terminal.rs
  - src/backend/handshake_core/src/model_session.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml workspace_safety
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
  - cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -- -D warnings
- POST_MERGE_SPOTCHECKS:
  - Verify fail-closed enforcement when worktree allocation fails
  - Verify cross-session file access is denied by default
  - Verify merge conflicts surface as BLOCKED not silently resolved
  - Verify session-scoped denylist includes all spec-mandated patterns

### NOT_PROVEN_AT_REFINEMENT_TIME (explicit uncertainty ledger; required for REFINEMENT_FORMAT_VERSION >= 2026-03-15)
- Rule: list what refinement cannot honestly prove yet so later phases cannot silently overclaim completeness.
- NOT_PROVEN_ITEMS:
  - DCC UI for workspace status display (downstream WP-1-Dev-Command-Center-Control-Plane-Backend)
  - Non-git worksurface isolation for Design Studio (Phase 2+)
  - OS-level sandbox enforcement (future concern)

### DISCOVERY
- DISCOVERY_PRIMITIVES: PRIM-SessionWorktreeAllocation, PRIM-SessionScopedDenylist, PRIM-MergeBackArtifact
- DISCOVERY_STUBS: NONE_CREATED | Reason: downstream stubs already exist
- DISCOVERY_MATRIX_EDGES: IMX-WS-001, IMX-WS-002, IMX-WS-003, IMX-WS-004
- DISCOVERY_UI_CONTROLS: session workspace indicator, merge review panel, cross-session approval dialog, denylist violation alert, worktree health status
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED | Reason: section 4.3.9.17 is comprehensive

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec Main Body explicitly defines workspace safety (4.3.9.17), both isolation strategies (4.3.9.17.2), command denylist (4.3.9.17.3), merge-back discipline (4.3.9.17.4), and invariants INV-WS-001 through INV-WS-003 (4.3.9.17.5). The remaining work is implementing the enforcement surface.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- Rule: `ENRICHMENT_NEEDED=YES` is required both for Main Body gaps and for appendix-driven spec version bumps. Appendix-only updates still count as a spec update boundary.
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Section 4.3.9.17 is comprehensive normative text (ADD v02.137) defining both isolation strategies, command denylist, merge-back discipline, and all three invariants. This WP implements the spec as written.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)
- Rule: for REFINEMENT_FORMAT_VERSION >= 2026-03-15, these anchor windows are also copied into the task packet `## SPEC_CONTEXT_WINDOWS` section for coder/validator downstream use.

#### ANCHOR 1
- SPEC_ANCHOR: 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions
- CONTEXT_START_LINE: 32655
- CONTEXT_END_LINE: 32700
- CONTEXT_TOKEN: Workspace Safety Boundaries for Parallel Sessions (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions (Normative) [ADD v02.137]

  When multiple sessions run concurrently, the product MUST enforce workspace isolation
  to prevent cross-session file conflicts, destructive command execution, and silent data loss.

  **4.3.9.17.2 Session Isolation Strategies**
  - Primary: git worktree isolation -- each writing session receives a dedicated git worktree
    at spawn time. Sessions write to their own branch and cannot modify the main workspace.
  - Fallback: file-scope lock isolation -- when git worktree is not available, sessions acquire
    Work Unit file-scope locks (4.3.9.2.4) before writing. Two sessions with overlapping
    IN_SCOPE_PATHS MUST NOT run concurrently without explicit operator approval.

  **4.3.9.17.3 Command Denylist**
  - Spawned and background sessions MUST receive a session-scoped command denylist at creation.
  - Denylist MUST include at minimum: git reset --hard, git clean -fd,
    rm -rf outside IN_SCOPE_PATHS, and any modification of .handshake/gov/.
  - Denylist violations MUST be logged via Flight Recorder and surface as BLOCKED state.

  **4.3.9.17.4 Merge-Back Discipline**
  - At session completion, the session MUST produce a merge-ready artifact (diff/patch)
    with session_id and provenance.
  - Merge conflicts MUST surface as BLOCKED state with an explicit conflict report.
  - No automated tooling may silently resolve conflicts.

  **4.3.9.17.5 Invariants**
  - INV-WS-001: Every session MUST declare IN_SCOPE_PATHS before writing.
  - INV-WS-002: If no isolation strategy can be established, execution MUST be denied (fail-closed).
  - INV-WS-003: Cross-session file access MUST be denied by default.
    Override requires explicit operator approval and a Flight Recorder event.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: 4.3.9.2.4 Work Unit lock contract
- CONTEXT_START_LINE: 21320
- CONTEXT_END_LINE: 21333
- CONTEXT_TOKEN: Work Unit lock contract (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.2.4 Work Unit lock contract (normative)

  A Work Unit MUST acquire an advisory lock on its declared file scope before execution begins.
  Lock ownership is identified by session_id.
  Two concurrently executing Work Units MUST NOT modify overlapping file scopes
  unless an explicit operator override with Flight Recorder evidence is present.
  Lock release MUST occur on Work Unit completion, cancellation, or timeout.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: 6.0.2 Unified Tool Surface Contract
- CONTEXT_START_LINE: 23945
- CONTEXT_END_LINE: 24050
- CONTEXT_TOKEN: single canonical tool contract
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 6.0.2 Unified Tool Surface Contract

  All tools available to model sessions MUST be routed through a single canonical tool contract.
  The Tool Gate enforces capability permission checks before any tool execution.
  Command denylist enforcement is a Tool Gate responsibility.
  Sessions may not bypass the Tool Gate to invoke system commands directly.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions
- CONTEXT_START_LINE: 32430
- CONTEXT_END_LINE: 32654
- CONTEXT_TOKEN: Cloud Consent-Gate Lifecycle for Parallel Sessions
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions

  When a session spawns one or more parallel child sessions, each child session MUST go through
  the Cloud Consent-Gate lifecycle independently.
  Session isolation state (worktree path, denylist, IN_SCOPE_PATHS) MUST be established
  before consent is granted and before any tool execution begins.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: INV-MM-003 Strict non-overlap of file scopes
- CONTEXT_START_LINE: 21358
- CONTEXT_END_LINE: 21367
- CONTEXT_TOKEN: Two concurrently executing Work Units MUST NOT modify overlapping file scopes
- EXCERPT_ASCII_ESCAPED:
  ```text
  **INV-MM-003** (Strict non-overlap of file scopes)
  Two concurrently executing Work Units MUST NOT modify overlapping file scopes
  unless an explicit operator override with Flight Recorder evidence is present.
  Violation detection MUST surface as a BLOCKED state, not a silent merge.
  ```
