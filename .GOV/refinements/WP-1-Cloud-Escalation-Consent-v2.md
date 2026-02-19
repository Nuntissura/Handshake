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
- WP_ID: WP-1-Cloud-Escalation-Consent-v2
- CREATED_AT: 2026-02-19T23:41:37Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.133.md
- SPEC_TARGET_SHA1: 9dac473bd1aa01b6d2900874169869c915fc355f
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja200220260034
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Cloud-Escalation-Consent-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Spec gap resolved: Cloud escalation Flight Recorder event definitions are now canonicalized in v02.133:
  - 11.5.8 defines canonical FR-EVT-CLOUD-001..004 typed schema (CloudEscalationEvent).
  - 11.5.8.1 declares 11.5.8 canonical and requires any mirrors (e.g., 9.1.4) to match.
  - 9.1.4 is now an aligned informative mirror (no FR-EVT-CLOUD-005; no consent-presented/received event types).
- No further spec enrichment is required for this WP. Implementation work remains:
  - Enforce WorkProfile.governance.allow_cloud_escalation (fail-closed when false).
  - Enforce GovernanceMode/AutomationLevel LOCKED behavior: cloud escalation blocked (no external transmission).
  - Enforce ProjectionPlan + ConsentReceipt binding (payload_sha256 + projection_plan_id) before any outbound cloud invocation.
  - Emit leak-safe FR-EVT-CLOUD-001..004 events per 11.5.8; include correlation IDs; no raw payloads.
  - Meet conformance tests in 10.5 (T-CLOUD-001..005), especially digest match (T-CLOUD-002).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Canonical Cloud escalation events (11.5.8; schema-validated at ingestion):
  - FR-EVT-CLOUD-001 cloud_escalation_requested: emitted when a CloudEscalationRequest is created/queued (before consent prompt).
  - FR-EVT-CLOUD-002 cloud_escalation_approved: emitted when ConsentReceipt is recorded with approved=true for a ProjectionPlan (bindings validated).
  - FR-EVT-CLOUD-003 cloud_escalation_denied: emitted when consent is denied OR escalation is blocked by policy/LOCKED/allow_cloud_escalation=false.
  - FR-EVT-CLOUD-004 cloud_escalation_executed: emitted immediately before/after the actual outbound cloud invocation (no raw payloads).
- Events MUST include correlation IDs (`job_id`, `wp_id`, `mt_id`, `trace_id`) but MUST NOT include raw transmitted payloads. Use IDs + hashes only.

### RED_TEAM_ADVISORY (security failure modes)
- Consent spoofing: client submits ConsentReceipt that does not match ProjectionPlan.payload_sha256 or mismatched projection_plan_id. Must hard-block server-side.
- Payload mismatch: payload digest shown to user differs from actual transmitted bytes (TOCTOU / serialization mismatch). Must compute digest from the final payload bytes and bind.
- Replay: reuse a ConsentReceipt for a different payload/model. Must bind ConsentReceipt to ProjectionPlan payload_sha256 and projection_plan_id, and tie to CloudEscalationRequest.request_id.
- Leak risk: ProjectionPlan include_artifact_refs or last_error_summary could leak secrets/PII if not redacted/bounded. Keep events leak-safe and ensure redactions_applied is recorded.
- LOCKED bypass: any cloud invocation path that can bypass GovernanceMode/AutomationLevel LOCKED denial is an audit failure. Must fail-closed.
- SSRF / exfil: cloud adapter base_url or request construction could be abused. Must keep existing SSRF/base_url validation and never include raw payload in logs/FR.

### PRIMITIVES (traits/structs/enums)
- ProjectionPlan (hsk.projection_plan@0.4) (11.1.7.1)
- ConsentReceipt (hsk.consent_receipt@0.4) (11.1.7.2)
- CloudEscalationRequest (hsk.cloud_escalation@0.4) (CloudEscalationRequest schema)
- WorkProfile.governance.allow_cloud_escalation and WorkProfile pinning/immutability (4.3.7)
- CloudEscalationEvent (FR-EVT-CLOUD-001..004) typed schema + ingestion validation (11.5.8)

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec v02.133 resolves the prior FR-EVT-CLOUD catalog conflict and provides canonical consent artifacts, CloudEscalationRequest schema, FR event schemas, and conformance tests (10.5) that make implementation deterministic.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.133 canonicalizes cloud escalation FR events (11.5.8 + 11.5.8.1) and aligns the 9.1.4 mirror table; no new normative text is required.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.1.7 Cloud Escalation Consent Artifacts (ProjectionPlan + ConsentReceipt) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 61545
- CONTEXT_END_LINE: 61605
- CONTEXT_TOKEN: payload_sha256: string;              // hash of the final projected payload
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

  A ConsentReceipt records the user\u2019s approval for a specific ProjectionPlan payload hash.

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

  - Cloud escalation MUST require explicit human consent regardless of AutomationLevel (\u00a72.6.8.12).
  - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
  - A `ConsentReceipt` MUST bind to `ProjectionPlan.payload_sha256` (tamper-evident).
  - Flight Recorder MUST record cloud escalation events (`FR-EVT-CLOUD-*`) but MUST NOT include raw payloads (see \u00a711.5).
  - Work Profiles MAY disable cloud escalation entirely (`allow_cloud_escalation = false`, \u00a74.3.7).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md CloudEscalationRequest Schema (Normative)
- CONTEXT_START_LINE: 12268
- CONTEXT_END_LINE: 12303
- CONTEXT_TOKEN: schema_version: "hsk.cloud_escalation@0.4";
- EXCERPT_ASCII_ESCAPED:
  ```text
  **CloudEscalationRequest Schema (Normative)**

  ```typescript
  export interface CloudEscalationRequest {
    schema_version: "hsk.cloud_escalation@0.4";
    request_id: string;
    wp_id: string;
    mt_id: string;

    reason: string;
    local_attempts: number;
    last_error_summary: string;

    requested_model_id: string;   // e.g. "gpt-4o"
    projection_plan_id: string;   // links to ProjectionPlan (what will be transmitted)
    consent_receipt_id: string;   // links to ConsentReceipt (human approval)
  }
  ```

  **Governance Requirements (Normative)**

  - Cloud escalation MUST require **explicit human consent** regardless of AutomationLevel (FULL_HUMAN / HYBRID / AUTONOMOUS).
  - If governance is `LOCKED`, cloud escalation MUST be blocked.
  - A `ProjectionPlan` MUST be generated and shown to the user prior to consent (see \u00a711.1.7).
  - Upon approval, a `ConsentReceipt` MUST be recorded and referenced by `CloudEscalationRequest`.

  **Flight Recorder Events (Normative)**

  The following events MUST be emitted for cloud escalation actions (see \u00a711.5):
  - `FR-EVT-CLOUD-001` cloud_escalation_requested
  - `FR-EVT-CLOUD-002` cloud_escalation_approved
  - `FR-EVT-CLOUD-003` cloud_escalation_denied
  - `FR-EVT-CLOUD-004` cloud_escalation_executed

  Events MUST include correlation IDs (`job_id`, `wp_id`, `mt_id`, `trace_id`) but MUST NOT include raw transmitted payloads.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 19410
- CONTEXT_END_LINE: 19449
- CONTEXT_TOKEN: allow_cloud_escalation: boolean;
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]

  #### 4.3.7.1 WorkProfile Schema (Normative)

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
  }
  ```
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.5.8 FR-EVT-CLOUD-001..004 (Cloud Escalation Events) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 63640
- CONTEXT_END_LINE: 63667
- CONTEXT_TOKEN: | "cloud_escalation_executed";   // FR-EVT-CLOUD-004
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.8 FR-EVT-CLOUD-001..004 (Cloud Escalation Events) (Normative) [ADD v02.120]

  ```ts
  type CloudEscalationEventType =
    | "cloud_escalation_requested"   // FR-EVT-CLOUD-001
    | "cloud_escalation_approved"    // FR-EVT-CLOUD-002
    | "cloud_escalation_denied"      // FR-EVT-CLOUD-003
    | "cloud_escalation_executed";   // FR-EVT-CLOUD-004

  interface CloudEscalationEvent extends FlightRecorderEventBase {
    type: CloudEscalationEventType;

    request_id: string;
    reason: string;

    requested_model_id: string;

    projection_plan_id?: string;
    consent_receipt_id?: string;

    wp_id?: string;
    mt_id?: string;

    local_attempts?: number;
    last_error_summary?: string;

    outcome?: "approved" | "denied" | "executed";
  }
  ```
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 9.1.4 Cloud Escalation Events (Aligned mirror; informative)
- CONTEXT_START_LINE: 67977
- CONTEXT_END_LINE: 67990
- CONTEXT_TOKEN: This table is a quick-reference mirror of the canonical event family defined in 11.5.8.
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 9.1.4 Cloud Escalation Events (Aligned mirror; informative)

  This table is a quick-reference mirror of the canonical event family defined in 11.5.8. The canonical schema and payload fields are defined in 11.5.8.

  | Event ID | Event Type | Trigger | Payload |
  |----------|------------|---------|---------|
  | FR-EVT-CLOUD-001 | `cloud_escalation_requested` | Cloud escalation requested | request_id, reason, requested_model_id, wp_id?, mt_id?, local_attempts?, last_error_summary? |
  | FR-EVT-CLOUD-002 | `cloud_escalation_approved` | Consent recorded (approved) | request_id, projection_plan_id, consent_receipt_id |
  | FR-EVT-CLOUD-003 | `cloud_escalation_denied` | Consent denied OR blocked | request_id, projection_plan_id?, wp_id?, mt_id? |
  | FR-EVT-CLOUD-004 | `cloud_escalation_executed` | Cloud invocation executed | request_id, requested_model_id, wp_id?, mt_id? |
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 10.5 Cloud Escalation (Conformance Tests)
- CONTEXT_START_LINE: 68090
- CONTEXT_END_LINE: 68100
- CONTEXT_TOKEN: | T-CLOUD-002 | Payload digest in UI MUST match transmitted payload |
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 10.5 Cloud Escalation

  | Test ID | Description |
  |---------|-------------|
  | T-CLOUD-001 | Cloud escalation MUST have ConsentReceipt |
  | T-CLOUD-002 | Payload digest in UI MUST match transmitted payload |
  | T-CLOUD-003 | Escalation after 2 failures MUST trigger cloud option |
  | T-CLOUD-004 | `LOCKED` mode MUST block cloud entirely |
  | T-CLOUD-005 | Escalation MUST be recorded as distillation candidate |
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 2.6.6.7.0 Canonical serialization + hashing (Normative)
- CONTEXT_START_LINE: 10271
- CONTEXT_END_LINE: 10274
- CONTEXT_TOKEN: - Hash function: SHA-256 over UTF-8 canonical JSON
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Canonical serialization + hashing**
  - Hash function: SHA-256 over UTF-8 canonical JSON (lexicographic keys, deterministic array order, ISO-8601 UTC timestamps, fixed float precision, NFC).
  - Hashed objects: `ScopeInputs`, `scope_inputs_hash`, `retrieval_candidates.ids_hash`, `selected_sources.ids_hash`, prompt hashes (prefix/suffix/full), and the persisted candidate list used for replay determinism.
  - Any seed used for strict determinism MUST be recorded in ContextSnapshot; `scope_inputs_hash` MUST be logged.
  ```
