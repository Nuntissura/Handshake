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
- WP_ID: WP-1-Model-Onboarding-ContextPacks-v1
- CREATED_AT: 2026-02-12T02:22:31.408Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- SPEC_TARGET_SHA1: 7260b4ada693263799ff39dd909653863cf0e503
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120220260341
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Model-Onboarding-ContextPacks-v1
 
### DECISIONS_AND_OPEN_QUESTIONS
- OPEN_QUESTION OQ-REGEN-001 (CLOSED 2026-02-14): Is ContextPack regeneration always permitted if a builder exists?
  - Decision: `regen_allowed` is capability/policy/consent-gated (not always permitted).
  - Spec tie-in: stale packs MUST NOT score `pack_score=1.0`; runtime MUST regenerate (if allowed) or fall back. If policy requires regeneration and it is not performed, FreshnessGuard may fail or mark degraded.
 
### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (ContextPacks are defined in the Main Body; this WP focuses on implementing deterministic onboarding/assembly and swap-safe reuse.)

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-LLM-INFERENCE: emit for any model call performed using compiled context from ContextPacks.
- FR-EVT-MT-* (loop/iteration/validation): emit for MT executor iterations that trigger context compilation/refresh decisions.
- FR-EVT-MODEL-001..005: emit on ModelSwapRequest + swap lifecycle; onboarding pack must be re-emitted/re-validated after swap.

### RED_TEAM_ADVISORY (security failure modes)
- RT-INJECT-001: onboarding pack MUST NOT allow prompt text to weaken lock paths, gates, or role identity (treat pack as policy-bearing, not advisory).
- RT-STALE-001: stale ContextPacks (source hash mismatch) MUST NOT be treated as authoritative; must regenerate or fall back per spec.
- RT-SECRETS-001: packs/artifacts MUST NOT contain secrets or raw sensitive payloads; only bounded refs + hashes.
- RT-ROLE-001: wrong role_id / wrong work_unit_id in pack leads to cross-role constraint loss; must hard-fail.

### PRIMITIVES (traits/structs/enums)
- ContextPackRecord / ContextPackPayload (as per spec) + deterministic canonical JSON hashing.
- RoleExecutionIdentity + WorkUnitContextPack (role + WP/MT binding; lock paths + verification contract).
- ContextPackFreshnessGuard (source_hash enforcement) integration point.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec defines ContextPacks (builder job + freshness + provenance binding) and requires fresh context compilation when models swap; this WP implements deterministic onboarding/assembly that satisfies those requirements.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: ContextPacks and swap-driven fresh context compilation are already specified in the Main Body; this WP is implementation-only.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 2.6.6.7.14.7 ContextPacks (mechanical compaction substrate)
- CONTEXT_START_LINE: 10634
- CONTEXT_END_LINE: 10653
- CONTEXT_TOKEN: ContextPacks (mechanical compaction substrate)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.7 ContextPacks (mechanical compaction substrate)

  **ContextPack builder job**
  - Tool ID: `context_pack_builder_v0.1`
  - Inputs: `target: EntityRef | SourceRef`, optional selector allowlist
  - Outputs:
    - `ContextPackRecord` (Derived)
    - `pack_artifact: ArtifactHandle` containing `ContextPackPayload`

  **Freshness**
  - `ContextPackRecord.source_hashes[]` MUST include the hashes of the underlying sources at build time.
  - A ContextPack is **stale** if any referenced source hash differs at retrieval time.
  - Stale packs MUST NOT be treated as pack_score=1.0. The runtime MUST either:
    - regenerate the pack (if allowed), or
    - fall back to non-pack retrieval routes.

  **Provenance binding**
  - Every `fact`, `constraint`, and `open_loop` MUST include `source_refs[]`.
  - A pack item without SourceRefs MUST be dropped or marked `confidence=0` and MUST NOT be promoted to LongTermMemory.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 4.3.3.4.3 ModelSwapRequest (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 17693
- CONTEXT_END_LINE: 17746
- CONTEXT_TOKEN: ModelSwapRequest (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.3.4 Sequential Model Swaps (Runtime Contract) (Normative) [ADD v02.120]

  - The runtime MUST support **sequential model loading** with **state persistence** and **fresh context recompile** on resume.

  ##### 4.3.3.4.3 ModelSwapRequest (Normative)

  ```typescript
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
  }
  ```
  ```

