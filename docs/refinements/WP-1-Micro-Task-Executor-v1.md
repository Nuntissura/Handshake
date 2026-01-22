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
- WP_ID: WP-1-Micro-Task-Executor-v1
- CREATED_AT: 2026-01-22
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.115.md
- SPEC_TARGET_SHA1: 61E500454062BACBE70578ADA7989286C0742973
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja220120260926
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Micro-Task-Executor-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- GAP-1 (No implementation present): No backend implementation currently matches the normative Micro-Task Executor profile requirements (job profile, loop controller, persistence, escalation, FR events).
- GAP-2 (MT definition generation): Spec requires MT definitions to be auto-generated from WP scope; no deterministic generator exists yet.
- GAP-3 (Deterministic loop enforcement): No controller enforces bounded iterations, pause points, hard-gate behavior, and completion signal parsing rules.
- GAP-4 (Validation routing + capabilities): No wiring exists to run validation commands via Mechanical Tool Bus with PlannedOperation envelopes and capability checks.
- GAP-5 (State + crash recovery): No persisted ProgressArtifact + RunLedger with atomic writes, idempotency keys, resume_point, and crash recovery procedure.
- GAP-6 (Observability): FR-EVT-MT-001..017 and required correlation fields are not emitted today; Phase 1 requires at least escalation, hard-gate, validation, and distillation-candidate events.
- GAP-7 (FR-EVT-WF-RECOVERY integration detail): Micro-Task crash recovery requires emitting FR-EVT-WF-RECOVERY and also mentions resume_point/step counts. Implementation should emit the schema-required fields and encode recovery specifics in reason and/or in separate artifacts referenced by the ProgressArtifact/RunLedger.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- MT event catalog: FR-EVT-MT-001..017 MUST be emitted per spec (Phase 1 focuses on escalation/hard-gate/validation/distillation-candidate coverage).
- Recovery integration: Micro-Task crash recovery MUST emit FR-EVT-WF-RECOVERY (see event schema anchor).
- Validator posture: treat completion claims as untrusted; emit validation events regardless of claimed completion.

### RED_TEAM_ADVISORY (security failure modes)
- Anti-gaming risk: model can output completion markers without doing work; parser must treat claims as untrusted and always run validation.
- Infinite loop risk: without bounded iteration limits + hard gate, the MT loop can run forever or thrash.
- Capability bypass risk: running validations outside Mechanical Tool Bus bypasses capability gating and evidence capture.
- Evidence integrity risk: without persisted prompt/output snapshots and idempotency keys, the system cannot prove what was attempted or replay crashes deterministically.

### PRIMITIVES (traits/structs/enums)
- Job profile: `micro_task_executor_v1` (AI Job Profile)
- Core schemas: `MicroTaskDefinition`, `ExecutionPolicy`, `ProgressArtifact`, `RunLedger`, `ValidationExecution`, `CompletionSignal`, `DistillationCandidate`
- Escalation: `EscalationLevel[]` and LoRA selection inputs (`task_tags`, available LoRAs)
- Flight Recorder: FR-EVT-MT-001..017, FR-EVT-WF-RECOVERY

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec Section 2.6.6.8 defines the Micro-Task Executor profile with normative loop algorithm, schemas (ValidationExecution, CompletionSignal, ProgressArtifact, RunLedger, DistillationCandidate), escalation chain, crash recovery procedure, and required Flight Recorder events for deterministic implementation.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The spec already defines the Phase 1 requirements in Section 2.6.6.8 and related Flight Recorder schemas; the work is implementation alignment, not spec authoring.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8 (Micro-Task Executor Profile)
- CONTEXT_START_LINE: 9381
- CONTEXT_END_LINE: 9393
- CONTEXT_TOKEN: #### 2.6.6.8 Micro-Task Executor Profile
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.6.6.8 Micro-Task Executor Profile

  [ADD v02.114] This section defines the **Micro-Task Executor Profile**, an AI Job Profile that enables small local models (7B-32B parameters) to execute complex work packets through iterative decomposition, autonomous looping, intelligent escalation, and continuous learning.

  **Status:** NORMATIVE  
  **Profile ID:** `micro_task_executor_v1`  
  **Implements:** AI Job Model (\\u00C2\\u00A72.6.6), Workflow Engine (\\u00C2\\u00A72.6), ACE Runtime (\\u00C2\\u00A72.6.6.7), Mechanical Tool Bus (\\u00C2\\u00A76.3, \\u00C2\\u00A711.8), Skill Bank (\\u00C2\\u00A79)

  **Abstract**

  The Micro-Task Executor Profile integrates with all Handshake subsystems to provide deterministic, auditable, crash-recoverable task execution without requiring large context windows. The design synthesizes proven techniques from autonomous agent patterns (Ralph-Wiggum iterative loops, Get-Shit-Done structured planning) while leveraging Handshake's mechanical infrastructure for validation, context management, and observability.

  **CRITICAL DESIGN PRINCIPLE:** Users NEVER manually create Micro-Task definitions. MT definitions are automatically generated from Work Packet scope by the system. This is enforced throughout the specification.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.7.1 (MicroTaskExecutionLoop algorithm)
- CONTEXT_START_LINE: 9946
- CONTEXT_END_LINE: 9968
- CONTEXT_TOKEN: ALGORITHM: MicroTaskExecutionLoop
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.7 Execution Model

  ###### 2.6.6.8.7.1 Execution Loop Algorithm

  The following algorithm defines the MT execution loop. Implementations MUST follow this sequence.

  ```
  ALGORITHM: MicroTaskExecutionLoop

  INPUT:
    - wp_id: Work Packet identifier
    - mt_definitions: Array of MicroTaskDefinition
    - policy: ExecutionPolicy

  OUTPUT:
    - progress: FinalProgressArtifact
    - status: "completed" | "completed_with_issues" | "failed" | "cancelled"

  STATE:
    - progress_artifact: ProgressArtifact
    - run_ledger: RunLedger
    - execution_state: ExecutionState
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.8 (MT context compilation uses ACE Runtime)
- CONTEXT_START_LINE: 10196
- CONTEXT_END_LINE: 10232
- CONTEXT_TOKEN: ##### 2.6.6.8.8 Context Compilation
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.8 Context Compilation

  ###### 2.6.6.8.8.1 Integration with ACE Runtime

  MT context compilation MUST use the ACE Runtime (\\u00C3\\u201A\\u00C2\\u00A72.6.6.8.2.6.6.7) with the following constraints:

  ```typescript
  interface MTContextCompilationConfig {
    /** Maximum tokens for compiled context */
    token_budget: number;                    // From MT definition
    
    /** Allocation between sections */
    budget_allocation: {
      system_rules: number;                  // Fixed ~300 tokens
      iteration_context: number;             // Fixed ~200 tokens
      mt_definition: number;                 // Variable, ~300-500 tokens
      file_contents: number;                 // Remainder
      previous_output: number;               // Optional, ~200 tokens if retry
    };
    
    /** Retrieval configuration */
    retrieval: {
      /** Use ContextPacks for efficiency */
      prefer_context_packs: boolean;         // Default: true
      
      /** Shadow Workspace query bounds */
      max_shadow_results: number;            // Default: 10
      
      /** Include neighboring code context */
      include_neighbors: boolean;            // Default: true
      neighbor_lines: number;                // Default: 20
    };
    
    /** Determinism mode */
    determinism: "strict" | "replay";        // Default: "replay"
  }
  ```
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.9.1 (Default escalation chain)
- CONTEXT_START_LINE: 10333
- CONTEXT_END_LINE: 10347
- CONTEXT_TOKEN: const DEFAULT_ESCALATION_CHAIN: EscalationLevel[] = [
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.9 Model Selection and Escalation

  ###### 2.6.6.8.9.1 Escalation Chain Structure

  The escalation chain defines the sequence of models to try:

  ```typescript
  const DEFAULT_ESCALATION_CHAIN: EscalationLevel[] = [
    { level: 0, model_id: "qwen2.5-coder:7b",  lora_selector: "auto", is_cloud: false, is_hard_gate: false },
    { level: 1, model_id: "qwen2.5-coder:7b",  lora_selector: "alternate", is_cloud: false, is_hard_gate: false },
    { level: 2, model_id: "qwen2.5-coder:13b", lora_selector: "auto", is_cloud: false, is_hard_gate: false },
    { level: 3, model_id: "qwen2.5-coder:13b", lora_selector: "alternate", is_cloud: false, is_hard_gate: false },
    { level: 4, model_id: "qwen2.5-coder:32b", lora_selector: "none", is_cloud: false, is_hard_gate: false },
    { level: 5, model_id: "HARD_GATE",         lora_selector: "none", is_cloud: false, is_hard_gate: true },
  ];
  ```
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.10.1 (ValidationExecution via Tool Bus)
- CONTEXT_START_LINE: 10465
- CONTEXT_END_LINE: 10499
- CONTEXT_TOKEN: interface ValidationExecution {
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.10 Validation Pipeline

  ###### 2.6.6.8.10.1 Validation Execution

  Validation commands are executed via the Mechanical Tool Bus (\\u00C3\\u201A\\u00C2\\u00A72.6.6.8.6.3, \\u00C3\\u201A\\u00C2\\u00A72.6.6.8.11.8):

  ```typescript
  interface ValidationExecution {
    /** PlannedOperation for shell execution */
    planned_operation: {
      schema_version: "poe-1.0";
      op_id: UUID;
      engine_id: "engine.shell";
      operation: "exec";
      inputs: [];
      params: {
        command: string;
        cwd: string;
        timeout_ms: number;
        env?: Record<string, string>;
      };
      capabilities_requested: ["proc.exec:cargo", "proc.exec:npm", /* etc */];
      budget: { max_duration_ms: number };
      determinism: "D1";
      evidence_policy: "capture_stdout_stderr";
    };
    
    /** Expected result structure */
    expected_result: {
      exit_code: number;
      stdout_contains?: string;
      stdout_not_contains?: string;
    };
  }
  ```
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.10.3 (CompletionSignal parsing; claims untrusted)
- CONTEXT_START_LINE: 10533
- CONTEXT_END_LINE: 10567
- CONTEXT_TOKEN: ###### 2.6.6.8.10.3 Completion Signal Parsing
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.8.10.3 Completion Signal Parsing

  The model MUST emit a structured completion signal. The parser extracts:

  ```typescript
  interface CompletionSignal {
    /** Whether model claimed completion */
    claimed_complete: boolean;
    
    /** Parsed evidence from completion block */
    evidence?: CompletionEvidence[];
    
    /** Whether model indicated it's blocked */
    blocked: boolean;
    blocked_reason?: string;
    
    /** Raw completion block if present */
    raw_block?: string;
  }

  interface CompletionEvidence {
    criterion: string;
    evidence_location: string;  // file:line format
  }
  ```

  **Parsing Rules:**

  1. Search for `<mt_complete>...</mt_complete>` block
  2. If found and well-formed \\u00C3\\u00A2\\u00E2\\u20AC\\u00A0\\u00E2\\u20AC\\u2122 `claimed_complete = true`
  3. Search for `<blocked>...</blocked>` block
  4. If found \\u00C3\\u00A2\\u00E2\\u20AC\\u00A0\\u00E2\\u20AC\\u2122 `blocked = true`, extract reason
  5. If neither found \\u00C3\\u00A2\\u00E2\\u20AC\\u00A0\\u00E2\\u20AC\\u2122 `claimed_complete = false`, `blocked = false`

  **IMPORTANT:** The model's claim is UNTRUSTED. Validation MUST still run.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.11.1 (ProgressArtifact schema)
- CONTEXT_START_LINE: 10571
- CONTEXT_END_LINE: 10615
- CONTEXT_TOKEN: interface ProgressArtifact {
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.11 State Management

  ###### 2.6.6.8.11.1 Progress Artifact Schema

  The progress artifact is the single source of truth for loop state:

  ```typescript
  interface ProgressArtifact {
    schema_version: "1.0";
    wp_id: string;
    job_id: JobId;
    
    /** Timestamps */
    created_at: ISO8601Timestamp;
    updated_at: ISO8601Timestamp;
    completed_at?: ISO8601Timestamp;
    
    /** Overall status */
    status: "in_progress" | "completed" | "completed_with_issues" | "failed" | "cancelled" | "paused";
    
    /** Execution policy (snapshot at start) */
    policy: ExecutionPolicy;
    
    /** Learning context snapshot */
    learning_context: {
      skill_bank_snapshot_at_start: ISO8601Timestamp;
      loras_available: LoRAInfo[];
      pending_distillation_jobs: number;
    };
    
    /** Current execution state */
    current_state: {
      active_mt?: string;
      active_model_level: number;
      total_iterations: number;
      total_escalations: number;
      total_drop_backs: number;
    };
    
    /** Per-MT status */
    micro_tasks: MTProgressEntry[];
    
    /** Aggregate statistics */
    aggregate_stats: AggregateStats;
  }
  ```
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.11.2 (RunLedger schema)
- CONTEXT_START_LINE: 10674
- CONTEXT_END_LINE: 10709
- CONTEXT_TOKEN: interface RunLedger {
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.8.11.2 Run Ledger Schema

  The run ledger provides idempotency and crash recovery:

  ```typescript
  interface RunLedger {
    ledger_id: UUID;
    wp_id: string;
    job_id: JobId;
    
    created_at: ISO8601Timestamp;
    
    /** Ordered list of execution steps */
    steps: LedgerStep[];
    
    /** Resume information */
    resume_point?: string;  // step_id to resume from
    resume_reason?: string;
  }

  interface LedgerStep {
    step_id: string;        // Format: {mt_id}_iter-{NNN}
    idempotency_key: string; // SHA256 of (mt_id + iteration + model + lora + prompt_hash)
    
    status: "pending" | "in_progress" | "completed" | "failed";
    
    started_at?: ISO8601Timestamp;
    completed_at?: ISO8601Timestamp;
    
    /** For completed steps: output reference */
    output_artifact_ref?: ArtifactHandle;
    
    /** For failed steps: error info */
    error?: string;
    recoverable: boolean;
  }
  ```
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.11.3 (Crash recovery procedure + FR-EVT-WF-RECOVERY emission)
- CONTEXT_START_LINE: 10712
- CONTEXT_END_LINE: 10748
- CONTEXT_TOKEN: ###### 2.6.6.8.11.3 Crash Recovery Procedure
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.8.11.3 Crash Recovery Procedure

  On startup, the Workflow Engine MUST execute:

  ```
  CrashRecoveryProcedure:

  1. SCAN for progress artifacts with status = "in_progress"

  2. FOR EACH stale_progress IN stale_artifacts:
     
     2.1. LOAD associated run_ledger
     
     2.2. FIND last completed step:
          last_completed = ledger.steps.filter(s => s.status == "completed").last()
     
     2.3. FIND any in_progress steps:
          in_progress = ledger.steps.filter(s => s.status == "in_progress")
     
     2.4. FOR EACH ip_step IN in_progress:
          // Check if output exists (step completed but status not updated)
          IF OutputArtifactExists(ip_step.idempotency_key):
              ip_step.status = "completed"
              ip_step.output_artifact_ref = FindOutputArtifact(ip_step.idempotency_key)
          ELSE:
              ip_step.status = "pending"  // Will be retried
     
     2.5. SET ledger.resume_point = FirstPendingOrFailedStep(ledger)
     
     2.6. EMIT FR-EVT-WF-RECOVERY with:
          - job_id
          - resume_point
          - steps_recovered
          - steps_to_retry

  3. RESUME normal execution from resume_point
  ```
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.12.1 (FR-EVT-MT event catalog)
- CONTEXT_START_LINE: 10752
- CONTEXT_END_LINE: 10777
- CONTEXT_TOKEN: | FR-EVT-MT-012 |
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.12 Observability and Flight Recorder

  ###### 2.6.6.8.12.1 Event Catalog

  The following Flight Recorder events MUST be emitted:

  | Event ID | Event Name | Trigger |
  |----------|-----------|---------|
  | FR-EVT-MT-001 | `micro_task_loop_started` | Job begins execution |
  | FR-EVT-MT-002 | `micro_task_iteration_started` | Each iteration begins |
  | FR-EVT-MT-003 | `micro_task_iteration_complete` | Each iteration ends |
  | FR-EVT-MT-004 | `micro_task_complete` | MT successfully completed |
  | FR-EVT-MT-005 | `micro_task_escalated` | Model escalation occurred |
  | FR-EVT-MT-006 | `micro_task_hard_gate` | Hard gate triggered |
  | FR-EVT-MT-007 | `micro_task_pause_requested` | User-defined pause point hit |
  | FR-EVT-MT-008 | `micro_task_resumed` | Execution resumed after pause |
  | FR-EVT-MT-009 | `micro_task_loop_completed` | All MTs finished |
  | FR-EVT-MT-010 | `micro_task_loop_failed` | Job failed |
  | FR-EVT-MT-011 | `micro_task_loop_cancelled` | Job cancelled |
  | FR-EVT-MT-012 | `micro_task_validation` | Validation executed |
  | FR-EVT-MT-013 | `micro_task_lora_selection` | LoRA selected |
  | FR-EVT-MT-014 | `micro_task_drop_back` | Model drop-back occurred |
  | FR-EVT-MT-015 | `micro_task_distillation_candidate` | Distillation data generated |
  | FR-EVT-MT-016 | `micro_task_skipped` | MT skipped |
  | FR-EVT-MT-017 | `micro_task_blocked` | MT blocked, awaiting input |
  ```

#### ANCHOR 11
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 2.6.6.8.13.1 (DistillationCandidate schema)
- CONTEXT_START_LINE: 10832
- CONTEXT_END_LINE: 10876
- CONTEXT_TOKEN: interface DistillationCandidate {
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.8.13 Learning Integration

  ###### 2.6.6.8.13.1 Skill Bank Integration

  When escalation occurs and `policy.enable_distillation = true`, a distillation candidate MUST be generated:

  ```typescript
  interface DistillationCandidate {
    /** Skill Bank entry ID */
    skill_log_entry_id: UUID;
    
    /** MT context */
    mt_id: string;
    wp_id: string;
    
    /** Student attempt (failed) */
    student_attempt: {
      model_id: string;
      lora_id?: string;
      lora_version?: string;
      prompt_snapshot_ref: ArtifactHandle;
      output_snapshot_ref: ArtifactHandle;
      outcome: "VALIDATION_FAILED" | "INCOMPLETE" | "ERROR";
      iterations: number;
    };
    
    /** Teacher success */
    teacher_success: {
      model_id: string;
      lora_id?: string;
      prompt_snapshot_ref: ArtifactHandle;
      output_snapshot_ref: ArtifactHandle;
      outcome: "VALIDATION_PASSED";
      iterations: number;
    };
    
    /** Task classification */
    task_type_tags: string[];
    contributing_factors: string[];
    
    /** Quality signals */
    data_trust_score: number;
    distillation_eligible: boolean;
  }
  ```
  ```

#### ANCHOR 12
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 11.5.2 (FR-EVT-WF-RECOVERY schema)
- CONTEXT_START_LINE: 51092
- CONTEXT_END_LINE: 51110
- CONTEXT_TOKEN: interface WorkflowRecoveryEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-WF-RECOVERY (WorkflowRecoveryEvent)**

  ```ts
  interface WorkflowRecoveryEvent extends FlightRecorderEventBase {
    type: 'workflow_recovery';

    workflow_run_id: string;
    job_id?: string;

    from_state: 'running';
    to_state: 'stalled';

    reason: string;
    last_heartbeat_ts: string; // RFC3339
    threshold_secs: number;
  }
  ```

  FR-EVT-WF-RECOVERY MUST be emitted when HSK-WF-003 transitions a workflow run from Running to Stalled. The actor MUST be 'system'.
  ```
