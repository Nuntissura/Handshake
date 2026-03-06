## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Locus-Phase1-Integration-Occupancy-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-06
- CREATED_AT: 2026-03-06T18:09:57Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- SPEC_TARGET_SHA1: f3b0715a544ebae689bee2196c0a4041cf4f2798
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060320261915
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Locus-Phase1-Integration-Occupancy-v1
- STUB_WP_IDS: WP-1-Locus-Phase1-QueryContract-Autosync-v1, WP-1-Locus-Phase1-Medallion-Search-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE in the Master Spec Main Body: v02.141 explicitly defines Locus occupancy state, bind/unbind operations, Spec Router auto-creation, MT Executor lifecycle integration, and Flight Recorder mappings.
- Implementation gap: `src/backend/handshake_core/src/workflows.rs` routes Spec Router output into local artifacts only and does not submit `locus_create_wp_v1` when a routed prompt yields a work packet.
- Execution gap: the MT executor loop in `src/backend/handshake_core/src/workflows.rs` records progress artifacts and native FR events, but it does not call `locus_register_mts_v1`, `locus_start_mt_v1`, `locus_record_iteration_v1`, or `locus_complete_mt_v1`.
- Occupancy gap: `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/locus/sqlite_store.rs`, and `src/backend/handshake_core/src/storage/locus_sqlite.rs` do not expose `active_session_ids` or bind/unbind operations for ModelSession occupancy tracking.
- Capability gap: `src/backend/handshake_core/src/capabilities.rs` exposes existing Locus lifecycle operations but does not map `locus_bind_session_v1` or `locus_unbind_session_v1`.
- Persistence gap: current SQLite Locus storage persists MT JSON metadata plus scalar counters, so occupancy updates must be added in a replay-safe way without widening this packet into query/search or Postgres parity work.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 75m
- SEARCH_SCOPE: Master Spec Section 2.3.15.2 through 2.3.15.6, Locus phase-audit evidence, Spec Router workflow path, MT executor loop, Locus job dispatcher, Locus types/parser/storage layers, capability registry, and existing Flight Recorder emission helpers.
- REFERENCES: Handshake_Master_Spec_v02.141.md Section 2.3.15; .GOV/Audits/AUDIT_20260304_PHASE1_CODE_VS_SPEC_v02.139.md; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/models.rs; src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/locus/sqlite_store.rs; src/backend/handshake_core/src/storage/locus_sqlite.rs; src/backend/handshake_core/src/capabilities.rs.
- PATTERNS_EXTRACTED: Existing product code already centralizes Locus side effects through `JobKind::LocusOperation` and `emit_locus_operation_event`; the lowest-risk implementation is to keep Spec Router and MT Executor as producers of typed Locus jobs rather than adding direct storage writes in workflow code. Existing MT progress artifacts and Locus MT metadata already overlap on iteration state, so occupancy can be persisted as part of the tracked MT document plus targeted scalar updates instead of a new relational subsystem in this packet.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT the existing Locus job dispatcher plus FR emitter as the only write path; ADAPT current tracked-MT metadata persistence to carry `active_session_ids` transactionally; REJECT SQLite-only shortcuts, direct workflow-to-storage writes, query/search expansion, Task Board autosync work, and Postgres-parity work in this packet.
- LICENSE/IP_NOTES: Local repository/spec analysis only; no external code reuse is proposed.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The Main Body already names the occupancy field, required operations, router/executor integration hooks, and event mappings with enough specificity to implement this packet without changing the spec.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new Flight Recorder family is required for this packet. The governing Locus catalog already maps `locus_create_wp`, `locus_register_mts`, `locus_start_mt`, `locus_record_iteration`, and `locus_complete_mt` to `FR-EVT-WP-001` and `FR-EVT-MT-001..004`.
- Implementation should reuse the existing Locus emission path in `src/backend/handshake_core/src/workflows.rs` so Spec Router and MT Executor integration produce the same event shapes as direct Locus jobs.
- Session occupancy does not justify a new FR event family in this packet; if extra session context is needed, it should be added as bounded payload fields on existing Locus operation events rather than introducing a parallel audit channel.

### RED_TEAM_ADVISORY (security failure modes)
- Stale occupancy risk: if bind/unbind is not idempotent across retries or crash recovery, `active_session_ids` can strand a session on an MT and misrepresent available capacity.
- Dispatcher bypass risk: wiring Spec Router or MT Executor directly into SQLite storage would bypass capability checks, operation provenance, and existing Locus event emission.
- Replay drift risk: repeated executor retries must not duplicate iteration records or append duplicate session IDs; storage updates need deterministic add/remove semantics.
- Provenance loss risk: if router-created Locus WPs omit `task_packet_path` or `spec_session_id`, the system loses the linkage the spec expects between routed prompts, governance artifacts, and tracked work.
- Concurrency risk: occupancy updates touch shared MT state; non-atomic metadata rewrites can drop iteration history or overwrite a concurrent session bind.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-LocusCreateWPJob
  - PRIM-FlightRecorder
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - This packet exercises existing Locus and Flight Recorder primitives already present in Appendix 12.4; the implementation may add product-code structs/enums for bind/unbind params, but no new spec primitive ID is required before packet creation.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: Appendix 12.4 already contains the tracked work packet, tracked micro task, Locus job, and Flight Recorder primitives this packet relies on.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Existing Locus FR events must become live on the Spec Router and MT Executor paths without creating a second audit mechanism. | STUB_WP_IDS: NONE
  - PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: This packet activates the currently-unused router/executor integration points and adds MT occupancy state required for parallel ModelSessions. | STUB_WP_IDS: WP-1-Locus-Phase1-QueryContract-Autosync-v1, WP-1-Locus-Phase1-Medallion-Search-v1
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: Routed prompts that produce work packets must immediately materialize corresponding tracked WPs in Locus. | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: Task Board sync remains out of scope here and stays deferred to the query-contract/autosync packet. | STUB_WP_IDS: WP-1-Locus-Phase1-QueryContract-Autosync-v1
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: The MT executor loop must emit tracked MT lifecycle changes and occupancy state at generation, start, iteration, and completion boundaries. | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: TOUCHED | NOTES: Spec Router becomes a producer of tracked WP state, tightening the handoff from routed prompt intent to executable work tracking. | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: Occupancy state must be stored in a backend-portable shape even though full Postgres parity is explicitly deferred from this packet. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Structured MT iteration and session-occupancy traces improve later retrieval, debugging, and analytics over work execution history. | STUB_WP_IDS: WP-1-Locus-Phase1-Medallion-Search-v1
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Search and query surfaces are deferred; this packet only establishes backend state they will later consume. | STUB_WP_IDS: WP-1-Locus-Phase1-Medallion-Search-v1
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: MT lifecycle calls carry model/lora data, but no new distillation or LoRA behavior is introduced here. | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: This packet uses existing workflow/runtime machinery rather than changing ACE contracts. | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - The highest-value interaction is already the packet's core scope: Spec Router plus MT Executor becoming producers for existing Locus primitives.
  - Search, query, and medallion interactions are real but already isolated into separate Locus Phase 1 stubs.
  - No new Appendix 12.6 edge is required to authorize this packet because the Main Body already names these integration points explicitly.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: NONE
  - Kind: NONE
  - ROI: LOW
  - Effort: LOW
  - Spec refs: NONE
  - In-scope for this WP: NO
  - If NO: create a stub WP and record it in TASK_BOARD Stub Backlog (order is not priority).
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The relevant cross-feature work is already captured by existing Locus stubs, and this packet does not require a new Appendix 12.6 interaction edge before activation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This is backend workflow/storage integration work only; user-facing query/search or console surfaces remain in downstream packets.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: NONE | Type: NONE | Tooltip: NONE | Notes: Backend-only packet
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: OK

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
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

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Main Body explicitly names `active_session_ids`, the bind/unbind operations, Spec Router auto `locus_create_wp`, MT Executor lifecycle calls, and the corresponding Locus event mappings, so this packet is implementation alignment rather than speculative design.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: The engineering choice is where to attach these calls in existing workflow code, but the required behavior itself is already normative and unambiguous in the spec.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Handshake_Master_Spec_v02.141.md already defines the occupancy field, required operations, workflow integration points, and event mappings needed to create this packet.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.4 Integration points summary
- CONTEXT_START_LINE: 5767
- CONTEXT_END_LINE: 5773
- CONTEXT_TOKEN: Auto-invokes `locus_create_wp` when routing prompts
- EXCERPT_ASCII_ESCAPED:
  ```text
  | Subsystem | Integration Type | Locus Operations |
  |-----------|------------------|------------------|
  | **Spec Router (\u00A72.6.8)** | Producer | Auto-invokes `locus_create_wp` when routing prompts |
  | **MT Executor (\u00A72.6.6.8)** | Producer | Auto-invokes `locus_start_mt`, `locus_record_iteration`, `locus_complete_mt` |
  | **Task Board** | Bidirectional Sync | `locus_sync_task_board` reads/writes `.handshake/gov/TASK_BOARD.md` |
  | **Task Packets** | Reference | WP.governance.task_packet_path links to `.handshake/gov/task_packets/{WP_ID}.md` |
  | **Flight Recorder (\u00A711.5)** | Event Source | All operations emit FR-EVT-WP-*, FR-EVT-MT-*, FR-EVT-DEP-*, FR-EVT-TB-* |
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.2 TrackedMicroTask schema
- CONTEXT_START_LINE: 5885
- CONTEXT_END_LINE: 5910
- CONTEXT_TOKEN: active_session_ids
- EXCERPT_ASCII_ESCAPED:
  ```text
  **TrackedMicroTask**

  interface TrackedMicroTask {
    // Identity
    mt_id: string;                       // "MT-001", "MT-002", etc.
    wp_id: string;                       // Parent Work Packet

    // Definition (from MT Executor Section 2.6.6.8)
    name: string;
    scope: string;
    files: {
      read: string[];
      modify: string[];
      create: string[];
    };
    done_criteria: string[];

    // Execution status
    status: "pending" | "in_progress" | "completed" | "failed" | "blocked" | "skipped";
    active_session_ids: string[];        // ModelSession IDs currently working on this MT (parallel allowed)
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.3 Mechanical operation catalog
- CONTEXT_START_LINE: 6013
- CONTEXT_END_LINE: 6024
- CONTEXT_TOKEN: locus_bind_session
- EXCERPT_ASCII_ESCAPED:
  ```text
  | `locus_create_wp` | Create Work Packet | `locus.write` | D1 |
  | `locus_update_wp` | Update WP fields | `locus.write` | D1 |
  | `locus_gate_wp` | Record gate result | `locus.gate` | D1 |
  | `locus_close_wp` | Complete Work Packet | `locus.write` | D1 |
  | `locus_delete_wp` | Delete (tombstone) | `locus.delete` | D1 |
  | `locus_register_mts` | Add MT definitions | `locus.write` | D1 |
  | `locus_start_mt` | Begin MT execution | `locus.write` | D1 |
  | `locus_bind_session` | Bind a ModelSession to an MT (occupancy) | `locus.write` | D1 |
  | `locus_unbind_session` | Unbind a ModelSession from an MT (occupancy) | `locus.write` | D1 |
  | `locus_record_iteration` | Record MT iteration | `locus.write` | D1 |
  | `locus_complete_mt` | Finish MT | `locus.write` | D1 |
  | `locus_add_dependency` | Create dependency edge | `locus.write` | D1 |
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.4 Spec Router integration
- CONTEXT_START_LINE: 6159
- CONTEXT_END_LINE: 6185
- CONTEXT_TOKEN: locus_create_wp_v1
- EXCERPT_ASCII_ESCAPED:
  ```text
  When Spec Router routes a prompt and creates a Task Packet, it automatically creates a Work Packet:

  interface SpecRouterLocusIntegration {
    onWorkPacketCreated(routing_result: SpecRoutingResult) {
      const job = {
        job_kind: "locus_operation",
        protocol_id: "locus_create_wp_v1",
        input: {
          operation: "create_wp",
          params: {
            wp_id: routing_result.wp_id,
            title: routing_result.spec_intent.title,
            description: routing_result.spec_intent.description,
            priority: derivePriority(routing_result.spec_intent),
            type: deriveType(routing_result.spec_intent),
            phase: routing_result.phase,
            routing: routing_result.governance_mode,
            task_packet_path: routing_result.task_packet_path,
            spec_session_id: routing_result.session_id
          }
        }
      };

      submit_job(job);  // WP now tracked in Locus
    }
  }
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.4 MT Executor integration
- CONTEXT_START_LINE: 6190
- CONTEXT_END_LINE: 6248
- CONTEXT_TOKEN: locus_record_iteration
- EXCERPT_ASCII_ESCAPED:
  ```text
  MT Executor calls Locus at every execution step:

  interface MTExecutorLocusIntegration {
    // After MT generation
    onMTsGenerated(wp_id: string, mt_definitions: MicroTaskDefinition[]) {
      locus_register_mts({
        wp_id,
        mt_definitions: mt_definitions.map(mt => ({
          mt_id: mt.mt_id,
          name: mt.name,
          scope: mt.scope,
          files: mt.files,
          done_criteria: mt.done.map(d => d.description)
        }))
      });
    }

    // Start MT execution
    onMTStarted(wp_id: string, mt_id: string, model: string, lora?: string) {
      locus_start_mt({
        wp_id,
        mt_id,
        model_id: model,
        lora_id: lora,
        escalation_level: 0
      });
    }

    // Record each iteration
    onIterationCompleted(
      wp_id: string,
      mt_id: string,
      iteration: number,
      model: string,
      tokens: TokenUsage,
      outcome: "SUCCESS" | "RETRY" | "ESCALATE" | "BLOCKED"
    ) {
      locus_record_iteration({
        wp_id,
        mt_id,
        iteration,
        model_id: model,
        escalation_level: getCurrentEscalationLevel(mt_id),
        tokens_prompt: tokens.prompt,
        tokens_completion: tokens.completion,
        duration_ms: getDuration(),
        outcome
      });
    }

    // Complete MT
    onMTCompleted(wp_id: string, mt_id: string) {
      locus_complete_mt({
        wp_id,
        mt_id,
        final_iteration: getCurrentIteration(mt_id)
      });
    }
  }
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.6 Flight Recorder event catalog
- CONTEXT_START_LINE: 6483
- CONTEXT_END_LINE: 6494
- CONTEXT_TOKEN: FR-EVT-MT-003
- EXCERPT_ASCII_ESCAPED:
  ```text
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
  ```
