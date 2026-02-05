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
- WP_ID: WP-1-Model-Swap-Protocol-v1
- CREATED_AT: 2026-02-01T13:59:01Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4D406DCC1A75570D2F17659E0AC40D68A22F211A
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010220261514
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Model-Swap-Protocol-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- The spec requires `state_hash` to match persisted state contents, but does not pin a hash algorithm. Implementation should use a deterministic sha256 over canonical persisted state bytes (consistent with other sha256 usage across the spec) and record it as lowercase hex.
- The spec defines FR-EVT-MODEL-001..005 event ids + event_type strings; implementation must follow these canonical ids/types and reject unknown variants at schema validation time.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-MODEL-001 `model_swap_requested`: emitted when a ModelSwapRequest is issued, including reason + correlation ids.
- FR-EVT-MODEL-002 `model_swap_completed`: emitted on successful swap completion.
- FR-EVT-MODEL-003 `model_swap_failed`: emitted on swap failure, including error_summary + rollback notes (if any).
- FR-EVT-MODEL-004 `model_swap_timeout`: emitted on timeout.
- FR-EVT-MODEL-005 `model_swap_rollback`: emitted when a rollback is executed.
- All ModelSwap events must be schema-validated at Flight Recorder ingestion (unknown ids/types rejected).

### RED_TEAM_ADVISORY (security failure modes)
- Spoofed swap: untrusted callers try to force a target_model_id; enforce server-side allowlists and capability/policy decisions.
- State mismatch: attacker supplies a forged state_hash; must recompute hash over persisted state and compare before proceeding.
- Denial-of-service: repeated swaps thrash VRAM; enforce swap limits and timeouts (per policy/exec extension).
- Cross-trace confusion: swap events missing correlation fields reduce auditability; ensure trace_id is present and include job_id/wp_id/mt_id when available.

### PRIMITIVES (traits/structs/enums)
- `ModelSwapRequest` (struct): typed request envelope (schema_version hsk.model_swap@0.4).
- `ModelSwapState` (struct): persisted state refs + computed state_hash + resume metadata.
- `ModelSwapOutcome` (enum): success|failure|timeout|rollback with error_summary.
- `ModelSwapEngine` (trait): execute swap steps (persist, unload, load, recompile, resume) with budgets.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec provides a canonical ModelSwapRequest schema, a required swap protocol sequence with hard requirements, and a normative FR-EVT-MODEL event family for observability.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The spec already defines the request schema, the protocol steps, and the required FR-EVT-MODEL event shapes; remaining implementation choices (e.g., concrete persistence mechanism) are compatible with the normative constraints.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 4.3.3.4.3 ModelSwapRequest (Normative) + 4.3.3.4.4 Model Swap Protocol (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 17714
- CONTEXT_END_LINE: 17788
- CONTEXT_TOKEN: schema_version: "hsk.model_swap@0.4";
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.3.4.3 ModelSwapRequest (Normative)

  A model swap request is a typed, auditable runtime command. Canonical JSON shape:

  export interface ModelSwapRequest {
    schema_version: "hsk.model_swap@0.4";
    request_id: string;

    // Current and target models
    current_model_id: string;
    target_model_id: string;

    // Role context (orchestrator/worker/validator/frontend)
    role: "frontend" | "orchestrator" | "worker" | "validator";

    // Priority and reason
    priority: "low" | "normal" | "high" | "critical";
    reason: string;   // e.g. "escalation", "profile_switch", "context_overflow"

    // Swap strategy (required)
    swap_strategy: "unload_reload" | "keep_hot_swap" | "disk_offload";

    // State persistence contract
    state_persist_refs: string[];  // Artifact refs (Locus checkpoint, MT state, etc.)
    state_hash: string;            // Hash of persisted state

    // Fresh context compilation requirement
    context_compile_ref: string;   // Reference to ACE context compilation job

    // Resource budgets
    max_vram_mb: number;
    max_ram_mb: number;
    timeout_ms: number;

    // Who requested the swap
    requester: {
      subsystem: "mt_executor" | "governance" | "ui" | "orchestrator";
      job_id?: string;
      wp_id?: string;
      mt_id?: string;
    };

    // Optional metadata
    metadata?: Record<string, any>;
  }

  ##### 4.3.3.4.4 Model Swap Protocol (Normative)

  When a ModelSwapRequest is issued, the runtime MUST execute the following steps:

  1. Persist state (WP/MT state, checkpoints, and pending approvals).
  2. Emit FR-EVT-MODEL-001 (swap requested).
  3. Unload/offload current model per strategy.
  4. Load target model, respecting budgets.
  5. Recompile context: Invoke ACE Runtime context compilation (\u00a72.6.6.7).
  6. Resume execution.
  7. Emit completion event (FR-EVT-MODEL-002 success / FR-EVT-MODEL-003 failure).

  Hard requirements:
  - A swap MUST NOT proceed unless state_hash matches the persisted state contents.
  - A swap MUST NOT exceed the declared resource budgets.
  - A swap MUST fail fast on timeout (timeout_ms).
  - A swap MUST NEVER drop or mutate Locus state; it may only create new checkpoint artifacts.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 11.5.6 FR-EVT-MODEL-001..005 (Model Resource Management Events) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 58038
- CONTEXT_END_LINE: 58077
- CONTEXT_TOKEN: type ModelSwapEventType =
- EXCERPT_ASCII_ESCAPED:
  ```text
  type ModelSwapEventType =
    | "model_swap_requested"     // FR-EVT-MODEL-001
    | "model_swap_completed"     // FR-EVT-MODEL-002
    | "model_swap_failed"        // FR-EVT-MODEL-003
    | "model_swap_timeout"       // FR-EVT-MODEL-004
    | "model_swap_rollback";     // FR-EVT-MODEL-005

  interface ModelSwapEvent extends FlightRecorderEventBase {
    type: ModelSwapEventType;

    request_id: string;

    current_model_id: string;
    target_model_id: string;
    role: "frontend" | "orchestrator" | "worker" | "validator";
    reason: string;

    swap_strategy?: "unload_reload" | "keep_hot_swap" | "disk_offload";

    // Budgets
    max_vram_mb?: number;
    max_ram_mb?: number;
    timeout_ms?: number;

    // State linkage (no inline payloads)
    state_persist_refs?: string[];
    state_hash?: string;
    context_compile_ref?: string;

    // Correlation
    wp_id?: string;
    mt_id?: string;

    // Outcome
    outcome?: "success" | "failure" | "timeout" | "rollback";
    error_summary?: string;
  }
  ```

