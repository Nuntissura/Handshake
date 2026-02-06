## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- CREATED_AT: 2026-02-03T07:08:54Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4d406dcc1a75570d2f17659e0ac40d68a22f211a
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja030220260848
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Locus-Work-Tracking-System-Phase1-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE: Master Spec includes a full normative section for Locus Work Tracking System, including schemas, mechanical operations, storage architecture, event sourcing, query interface, and explicit conformance requirements.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Locus operations MUST emit Flight Recorder event families defined in Master Spec 2.3.15.6:
  - FR-EVT-WP-001..005 (Work Packets)
  - FR-EVT-MT-001..006 (Micro-Tasks)
  - FR-EVT-DEP-001..002 (Dependencies)
  - FR-EVT-TB-001..003 (Task Board sync/status)
  - FR-EVT-SYNC-001..003 (Sync lifecycle)
  - FR-EVT-QUERY-001 (Query execution)
- Diagnostics must reject unknown event_type (schema validator update required where Flight Recorder schemas are registered).

### RED_TEAM_ADVISORY (security failure modes)
- Integrity risks: forged/edited work tracking state (WP/MT status) causing false "done" signals or bypassing gates; require append-only notes and deterministic state transitions.
- Injection risks: Task Board sync consumes/produces Markdown; treat as untrusted input, avoid arbitrary execution and avoid path traversal when resolving referenced packet paths.
- Denial-of-service risks: unbounded dependency graphs or large Task Board files; enforce bounds and validate inputs.
- Audit risks: missing event emissions or partial emissions; require fail-fast on emission/validation failures so drift is visible.

### PRIMITIVES (traits/structs/enums)
- TrackedWorkPacket, TrackedMicroTask, TrackedDependency core domain objects.
- WorkPacketStatus and TaskBoardStatus enums (and any mapping between them).
- Locus mechanical operation request/response types for:
  - locus_create_wp, locus_update_wp, locus_gate_wp, locus_close_wp, locus_delete_wp
  - locus_register_mts, locus_start_mt, locus_record_iteration, locus_complete_mt
  - locus_add_dependency, locus_remove_dependency
  - locus_query_ready (and any other query ops included in 2.3.15.7)
- Flight Recorder event payload structs for the FR-EVT-* families listed above (as required by schema registry/validator).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.123 contains an explicit, detailed normative section 2.3.15 (Locus Work Tracking System), including concrete schemas, operation names, and Flight Recorder event IDs.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The referenced Master Spec sections are sufficiently specific to implement without adding new normative text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.15 Locus Work Tracking System [ADD v02.116]
- CONTEXT_START_LINE: 5392
- CONTEXT_END_LINE: 5408
- CONTEXT_TOKEN: ### 2.3.15 Locus Work Tracking System [ADD v02.116]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.15 Locus Work Tracking System [ADD v02.116]

  **Why**
  Handshake requires unified tracking from macro-level governance (Work Packets \\u00e2\\u2020\\u2019 Task Packets \\u00e2\\u2020\\u2019 Gates) through micro-level execution (Micro-Tasks \\u00e2\\u2020\\u2019 Iterations \\u00e2\\u2020\\u2019 Validation). External issue trackers lack governance integration, cannot observe MT execution granularity, and don't provide event-sourced sync. Locus provides a native, mechanical, fully-integrated work tracking system that spans the complete lifecycle from "User says: Build feature" through "MT-003 iteration 2 validated successfully."

  **What**
  Locus is Handshake's native work tracking subsystem that tracks Work Packets (governance-aware work units) and Micro-Tasks (atomic execution units) with full observability, dependency management, and multi-user collaboration. It integrates with Spec Router (auto-creates WPs), MT Executor (tracks iterations), Task Board (bidirectional sync), Task Packets (links to docs/), Flight Recorder (event sourcing), Knowledge Graph (typed dependencies), and Calendar (policy-based queries).

  **Jargon**
  - **Work Packet (WP)**: A governance-tracked work unit with lifecycle states (stub \\u00e2\\u2020\\u2019 ready \\u00e2\\u2020\\u2019 in_progress \\u00e2\\u2020\\u2019 blocked \\u00e2\\u2020\\u2019 gated \\u00e2\\u2020\\u2019 done), gates (pre-work, post-work), and linked Task Packets. Created by Spec Router (\\u00c2\\u00a72.6.8) from user prompts.
  - **Micro-Task (MT)**: An atomic execution unit (1-5 files, single session) with iteration tracking, model escalation, and validation results. Generated and executed by MT Executor (\\u00c2\\u00a72.6.6.8).
  - **Locus**: Latin for "place" or "position"; the system locates work packets by status, dependencies, and execution state.
  - **Task Board**: The markdown table in `docs/TASK_BOARD.md` that provides human-readable project status. Locus syncs bidirectionally with it.
  - **Task Packet**: The structured spec in `docs/task_packets/{WP_ID}.md` with IN_SCOPE_PATHS, DONE_MEANS, TEST_PLAN. Locus links to these.
  - **Ready Work**: The set of all WPs where status=ready AND no open blocking dependencies exist.
  - **Mechanical Operation**: All Locus operations follow Mechanical Tool Bus (\\u00c2\\u00a76.3) patterns with PlannedOperation envelopes, capability gating, and deterministic execution.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 2.3.15.6 Event Sourcing (Flight Recorder event families)
- CONTEXT_START_LINE: 6148
- CONTEXT_END_LINE: 6173
- CONTEXT_TOKEN: FR-EVT-WP-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Flight Recorder Events (21 types)**

  All Locus operations emit Flight Recorder events:

  | Event ID | Event Name | Trigger |
  |----------|------------|---------|
  | FR-EVT-WP-001 | work_packet_created | locus_create_wp |
  | FR-EVT-WP-002 | work_packet_updated | locus_update_wp |
  | FR-EVT-WP-003 | work_packet_gated | locus_gate_wp |
  | FR-EVT-WP-004 | work_packet_completed | locus_close_wp |
  | FR-EVT-WP-005 | work_packet_deleted | locus_delete_wp |
  | FR-EVT-MT-001 | micro_tasks_registered | locus_register_mts |
  | FR-EVT-MT-002 | mt_iteration_completed | locus_record_iteration |
  | FR-EVT-MT-003 | mt_started | locus_start_mt |
  | FR-EVT-MT-004 | mt_completed | locus_complete_mt |
  | FR-EVT-MT-005 | mt_escalated | MT escalation occurs |
  | FR-EVT-MT-006 | mt_failed | MT fails permanently |
  | FR-EVT-DEP-001 | dependency_added | locus_add_dependency |
  | FR-EVT-DEP-002 | dependency_removed | locus_remove_dependency |
  | FR-EVT-TB-001 | task_board_entry_added | WP added to Task Board |
  | FR-EVT-TB-002 | task_board_synced | locus_sync_task_board |
  | FR-EVT-TB-003 | task_board_status_changed | Manual Task Board edit detected |
  | FR-EVT-SYNC-001 | sync_started | Sync operation begins |
  | FR-EVT-SYNC-002 | sync_completed | Sync operation succeeds |
  | FR-EVT-SYNC-003 | sync_failed | Sync operation fails |
  | FR-EVT-QUERY-001 | work_query_executed | Query operation (locus_query_ready, etc.) |
  ```
