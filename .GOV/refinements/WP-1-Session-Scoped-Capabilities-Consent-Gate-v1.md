## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, REASON_NO_ENRICHMENT and SPEC_EXCERPTS are provided for every anchor.
- If ENRICHMENT_NEEDED=YES, the full Proposed Spec Enrichment text must be verbatim.
- Keep this file ASCII-only.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- CREATED_AT: 2026-03-03T00:48:59Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.139.md
- SPEC_TARGET_SHA1: 0A5A9069BF8E06654DDF9B647927C2CB8A30AA6F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja030320260206
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Session-Scoped-Capabilities-Consent-Gate-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (for this WP scope). The current Master Spec already defines: (a) session-scoped capability intersection in Tool Gate, (b) cloud consent-gate lifecycle for parallel sessions (incl. broadcast receipt enumeration + revocation invariants), and (c) inbound trust boundary rules for SYSTEM provenance and child-session narrowing.
- Note: Some schema fields are defined across multiple sections (ModelSession, Tool Gate envelope, Capability Registry, consent artifacts). This WP is about wiring/enforcing those existing rules end-to-end in runtime and Tool Gate, not creating new policy.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-007 ToolCallEvent: emit for every tool invocation (local + MCP + MEX) with capability_ids[] and ok=false + error.kind="capability" (or policy) when denied by session-scoped intersection.
- FR-EVT-CLOUD-001..004 CloudEscalationEvent: emit for consent request/approval/deny/execute for any cloud escalation path; session consent receipts MUST be linked via consent_receipt_id + projection_plan_id where applicable.
- Session state transitions to BLOCKED due to consent revocation or missing receipt MUST be observable (at minimum via existing session/job + FR correlation by model_session_id).

### RED_TEAM_ADVISORY (security failure modes)
- Confused-deputy risk: global/operator capabilities accidentally applied to a session without intersection enables cross-session privilege escalation.
- Receipt replay risk: reusing a SESSION_SCOPED/WP_SCOPED receipt across sessions not enumerated at issuance violates INV-CONSENT-002 and enables silent fan-out.
- Message provenance bypass: allowing external sources to inject SYSTEM messages (or to spoof trusted source attribution) undermines TRUST-001/002 and makes prompt injection indistinguishable from runtime policy.
- Dangerous bypass flags: any global "skip approvals/sandbox/capabilities" flag violates TRUST-004 and creates a remote action pipeline in practice.
- Partial revocation risk: revocation that fails to cancel pending jobs (INV-CONSENT-003) leaves orphaned cloud calls running outside operator intent.

### PRIMITIVES (traits/structs/enums)
- Tool Gate:
  - Session-scoped effective capability resolution (deny-by-default): (ModelSession.capability_grants + resolved capability_token_ids) intersected with global/operator constraints.
  - Deterministic denial path with structured error + FR-EVT-007.
- Consent gate:
  - ProjectionPlan + ConsentReceipt usage for cloud model calls, including broadcast/fan-out semantics and receipt validity checks before dispatch.
  - ConsentScope enum enforcement (SINGLE_CALL, SESSION_SCOPED, WP_SCOPED, BROADCAST_SCOPED) with INV-CONSENT-001..003 behavior.
- Trust boundary:
  - Inbound message provenance handling enforcing TRUST-001/002 and child-session tool narrowing enforcement per TRUST-003.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec sections 6.0.2.5 (Tool Gate), 4.3.9.14 (Consent Gate), and 4.3.9.20 (Trust Boundary) are normative and define concrete deny-by-default enforcement and invariants for session-scoped capabilities and consent.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The authoritative spec already defines the required invariants, envelopes, and governance rules for session-scoped capability intersection and cloud consent gating. This WP is implementation/wiring of those existing normative requirements.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 6.0.2.5 Canonical invocation envelope (HTC-1.0) (MUST) - session-scoped capability intersection (Normative)
- CONTEXT_START_LINE: 23090
- CONTEXT_END_LINE: 23160
- CONTEXT_TOKEN: Normative (session-scoped capability intersection):
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6.0.2.5 Canonical invocation envelope (HTC-1.0) (MUST)

  Request envelope includes:
  - session_id: optional (REQUIRED for ModelSession tool calls; set to ModelSession.session_id)

  Normative (ModelSession correlation):
  - session_id MUST be present for ModelSession tool calls.

  Normative (session-scoped capability intersection):
  - Tool Gate evaluates required_capabilities against session-scoped effective grants/tokens (deny-by-default),
    intersected with global/operator constraints; deny or escalate if unsatisfied.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 4.3.9.12 ModelSession (Normative) - consent + capability_token_ids
- CONTEXT_START_LINE: 31064
- CONTEXT_END_LINE: 31138
- CONTEXT_TOKEN: capability_token_ids: string[] | null
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]

  ModelSession includes:
    session_id: string
    parent_session_id: string | null
    spawn_depth: int
    state: enum [CREATED, ACTIVE, PAUSED, BLOCKED, COMPLETED, FAILED, CANCELLED]
    ...
    consent_receipt_id: string | null
    capability_grants: string[]
    capability_token_ids: string[] | null  # deny-by-default; references approval tokens/receipts
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) [ADD v02.137]
- CONTEXT_START_LINE: 31279
- CONTEXT_END_LINE: 31328
- CONTEXT_TOKEN: INV-CONSENT-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) [ADD v02.137]

  Consent flow (normative):
  1) Pre-flight ProjectionPlan (payload disclosure + estimated cost)
  2) Fan-out disclosure for broadcast/fan-out
  3) Operator issues ConsentReceipt bound to ProjectionPlan hash + session_id(s) + validity window
  4) ModelSession stores consent_receipt_id; scheduler verifies before dispatching cloud model_run

  ConsentScope enum includes BROADCAST_SCOPED.

  Invariants:
  - INV-CONSENT-001: No cloud model call without valid ConsentReceipt bound to target session (hard block CX-MM-CONSENT-MISSING)
  - INV-CONSENT-002: BROADCAST_SCOPED receipt enumerates session_ids at issuance; adding sessions requires new receipt
  - INV-CONSENT-003: Revocation cancels pending model_run jobs and transitions sessions to BLOCKED
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 4.3.9.20 Inbound Trust Boundary Rules (Normative) [ADD v02.137]
- CONTEXT_START_LINE: 31682
- CONTEXT_END_LINE: 31702
- CONTEXT_TOKEN: TRUST-001 (system message provenance):
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.20 Inbound Trust Boundary Rules (Normative) [ADD v02.137]

  Rules (HARD):
  - TRUST-001: Only runtime may inject SYSTEM; external sources injected as USER with source attribution.
  - TRUST-002: Cross-session routed messages carry source session_id + role + content_hash + trusted/untrusted flag.
  - TRUST-003: Child sessions have equal-or-narrower tool permissions than parent; no self-escalation.
  - TRUST-004: No global bypass flags for sandbox/approvals/capabilities; session-scoped debug only, logged.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 11.5 FR-EVT-007 ToolCallEvent (Normative)
- CONTEXT_START_LINE: 65148
- CONTEXT_END_LINE: 65220
- CONTEXT_TOKEN: interface ToolCallEvent extends FlightRecorderEventBase
- EXCERPT_ASCII_ESCAPED:
  ```text
  - FR-EVT-007 (ToolCallEvent)

  interface ToolCallEvent extends FlightRecorderEventBase {
    type: 'tool_call';
    trace_id: string;       // REQUIRED
    tool_call_id: string;   // REQUIRED
    tool_id: string;
    tool_version: string;
    ok: boolean;
    error?: { code, kind, message?, retryable? } | null;
    args_ref/result_ref are artifact-first (redacted).
    capability_ids?: string[]; // capabilities asserted/required for this call
  }
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 11.5.8 FR-EVT-CLOUD-001..004 (Cloud Escalation Events) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 65621
- CONTEXT_END_LINE: 65670
- CONTEXT_TOKEN: FR-EVT-CLOUD-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.8 FR-EVT-CLOUD-001..004 (Cloud Escalation Events) (Normative)

  CloudEscalationEventType includes:
  - cloud_escalation_requested   // FR-EVT-CLOUD-001
  - cloud_escalation_approved    // FR-EVT-CLOUD-002
  - cloud_escalation_denied      // FR-EVT-CLOUD-003
  - cloud_escalation_executed    // FR-EVT-CLOUD-004
  ```
