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
- WP_ID: WP-1-LLM-Provider-Registry-v1
- CREATED_AT: 2026-02-12T02:22:20.947Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120220260340
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-LLM-Provider-Registry-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (No missing normative requirements block implementation; this WP implements adapter/registry wiring to satisfy existing LlmClient + Work Profiles + Cloud consent rules.)

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-LLM-INFERENCE: emit for every LlmClient completion (local or cloud), correlated by trace_id (no raw prompt/payload).
- FR-EVT-PROFILE-001..003: Work Profile selection + model assignment resolution events.
- FR-EVT-CLOUD-001..004: cloud escalation requested/approved/denied/executed.
- FR-EVT-MODEL-001..005: ModelSwapRequest lifecycle events (requested/persist/offload/load/resume).

### RED_TEAM_ADVISORY (security failure modes)
- RT-SECRETS-001: provider API keys/tokens MUST NOT appear in logs, task packets, debug bundles, manifests, or Flight Recorder payloads.
- RT-SSRF-001: OpenAI-compatible base_url MUST be treated as untrusted input; prevent SSRF/internal network access without explicit consent/policy.
- RT-CLOUD-001: cloud invocation without ConsentReceipt (when required) MUST hard-block; LOCKED mode MUST deny cloud escalation.
- RT-LEAK-001: raw prompts/derived payloads MUST NOT be exported to cloud logs/artifacts by default; use local-only payload refs where needed.

### PRIMITIVES (traits/structs/enums)
- ProviderKind enum (ollama | openai_compat | ...), ProviderTier (local | cloud).
- ProviderConfig / ProviderRecord (provider_id, base_url, redacted secrets handles, defaults).
- ProviderRegistry (persist + resolve provider configs; redacted export).
- OpenAiCompatClient implementing LlmClient (http adapter).
- CloudEscalationGuard (ProjectionPlan + ConsentReceipt enforcement prior to cloud calls).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec defines LlmClient (single-client invariant), Work Profiles (role-based routing + allow_cloud_escalation), and Cloud Escalation Consent artifacts/rules; this WP implements the provider/adapter surface required to realize those behaviors.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Required normative requirements already exist in the Main Body (LlmClient adapter, Work Profiles, and Cloud consent artifacts). This WP adds implementation wiring (registry + OpenAI-compatible adapter) without requiring new spec text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 4.2.3 LLM Client Adapter (Normative)
- CONTEXT_START_LINE: 17242
- CONTEXT_END_LINE: 17323
- CONTEXT_TOKEN: ### 4.2.3 LLM Client Adapter (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 4.2.3 LLM Client Adapter (Normative)

  To satisfy the **Single Client Invariant [CX-101]**, all application code MUST interact with LLMs through the `LlmClient` trait. This ensures provider portability and centralized observability.

  #### 4.2.3.1 LlmClient Trait

  ```rust
  /// HSK-TRAIT-004: LLM Client Adapter
  #[async_trait]
  pub trait LlmClient: Send + Sync {
      /// Executes a completion request.
      /// Returns:
      /// - Ok(CompletionResponse): The generated text and usage metadata.
      /// - Err(LlmError): If the request fails or budget is exceeded.
      async fn completion(
          &self,
          req: CompletionRequest
      ) -> Result<CompletionResponse, LlmError>;

      /// Returns the model profile (capabilities, token limits).
      fn profile(&self) -> &ModelProfile;
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct CompletionRequest {
      pub trace_id: Uuid,          // REQUIRED: non-nil
      pub prompt: String,
      pub model_id: String,
      pub max_tokens: Option<u32>,
      pub temperature: f32,
      pub stop_sequences: Vec<String>,
  }
  ```

  #### 4.2.3.2 Implementation Requirements

  1.  **Ollama Adapter:** The primary implementation for Phase 1 MUST use the Ollama API.
  2.  **Budget Enforcement:** The client MUST enforce `max_tokens` and return `BudgetExceeded` if the provider exceeds the limit.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 18038
- CONTEXT_END_LINE: 18088
- CONTEXT_TOKEN: ### 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]

  Work Profiles allow the user (or workspace policy) to define **which models** are used for each runtime role and how autonomous execution is allowed to be (automation level + cloud escalation settings).

  #### 4.3.7.1 WorkProfile Schema (Normative)

  Canonical JSON shape:

  ```typescript
  export interface WorkProfile {
    schema_version: "hsk.work_profile@0.5";
    profile_id: string;
    name: string;
    description?: string;

    // Model role assignments
    model_assignments: {
      frontend: ModelAssignment;
      orchestrator: ModelAssignment;
      worker: ModelAssignment;
      validator: ModelAssignment;
    };

    // Governance settings
    governance: {
      automation_level: "FULL_HUMAN" | "HYBRID" | "AUTONOMOUS";
      allow_cloud_escalation: boolean;
      max_cloud_escalations_per_job?: number;
    };

    // Optional override rules
    overrides?: {
      filetype_rules?: Record<string, Partial<WorkProfile["model_assignments"]>>;
      task_type_rules?: Record<string, Partial<WorkProfile["model_assignments"]>>;
    };
  }

  export interface ModelAssignment {
    primary_model_id: string;
    fallback_model_id?: string;
    local_only: boolean;
    allowed_models?: string[];  // restrict to a whitelist
  }
  ```

  **Normative requirements**
  - Work Profiles MUST be immutable once used by a job (pin-by-id); new edits create a new `profile_id`.
  - A job/session MUST record which `profile_id` was active at execution start.
  - Changing the Work Profile MUST emit a Flight Recorder event (`FR-EVT-PROFILE-001`) and MUST NOT retroactively change already-started jobs.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 11.1.7 Cloud Escalation Consent Artifacts (ProjectionPlan + ConsentReceipt) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 56078
- CONTEXT_END_LINE: 56136
- CONTEXT_TOKEN: ### 11.1.7 Cloud Escalation Consent Artifacts (ProjectionPlan + ConsentReceipt) (Normative) [ADD v02.120]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.1.7 Cloud Escalation Consent Artifacts (ProjectionPlan + ConsentReceipt) (Normative) [ADD v02.120]

  Cloud escalation (sending any user/workspace data to a cloud model) is governed by explicit consent and a machine-readable projection plan.

  #### 11.1.7.1 ProjectionPlan (Normative)

  A ProjectionPlan specifies **exactly what** will be transmitted externally (after redaction/projection).

  ```typescript
  export interface ProjectionPlan {
    schema_version: "hsk.projection_plan@0.4";
    projection_plan_id: string;

    // What will be transmitted
    include_artifact_refs: string[];     // artifact handles or paths
    include_fields?: string[];           // optional explicit fields
    redactions_applied: string[];        // e.g. ["secrets", "pii", "tokens"]
    max_bytes: number;

    // Integrity
    payload_sha256: string;              // hash of the final projected payload
    created_at: string;

    // Correlation
    job_id?: string;
    wp_id?: string;
    mt_id?: string;
  }
  ```

  #### 11.1.7.2 ConsentReceipt (Normative)

  ```typescript
  export interface ConsentReceipt {
    schema_version: "hsk.consent_receipt@0.4";
    consent_receipt_id: string;

    projection_plan_id: string;
    payload_sha256: string;

    approved: boolean;
    approved_at: string;

    // Who approved
    user_id: string;

    // Optional UI metadata (no secrets)
    ui_surface?: "cloud_escalation_modal" | "settings" | "operator_console";
    notes?: string;
  }
  ```

  #### 11.1.7.3 Normative Rules for Cloud Escalation

  - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
  - A `ConsentReceipt` MUST bind to `ProjectionPlan.payload_sha256` (tamper-evident).
  ```

