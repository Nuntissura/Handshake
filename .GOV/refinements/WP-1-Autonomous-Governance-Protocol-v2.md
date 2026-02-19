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
- WP_ID: WP-1-Autonomous-Governance-Protocol-v2
- CREATED_AT: 2026-02-19T14:10:53Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.132.md
- SPEC_TARGET_SHA1: ffa3d933b4a21c4677bfe9a06cf29cda59dd34a2
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja190220261548
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Autonomous-Governance-Protocol-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- No additional Master Spec enrichment is required for Autonomous Governance.
- The previously identified cross-section conflicts (AutomationLevel enum mismatch, GovernanceDecision schema_version mismatch, missing AutoSignature schema, and non-canonical governance event IDs in the Stage import) are resolved in Master Spec v02.132 via:
  - 2.6.8.12.6 canonical AutomationLevel set + normalization rules + LOCKED semantics
  - 2.6.8.12.6.2 canonical GovernanceDecision schema_version pinning
  - 2.6.8.12.6.3 AutoSignature artifact schema
  - 2.6.8.12.6.4 FR-EVT-GOV alignment (no new event IDs)
  - 10.13 Merge alignment note pinning canonical definitions

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Governance automation events (11.5.7):
  - FR-EVT-GOV-001 (gov_decision_created): emit whenever a GovernanceDecision is created (human or model).
  - FR-EVT-GOV-002 (gov_decision_applied): emit when the decision is applied to satisfy a gate / transition workflow state.
  - FR-EVT-GOV-003 (gov_auto_signature_created): emit when an AutoSignature artifact is created.
  - FR-EVT-GOV-004 (gov_human_intervention_requested): emit when a decision requires human review (and human review is permitted).
  - FR-EVT-GOV-005 (gov_human_intervention_received): emit when human input is received for a pending intervention.
- Cloud escalation events (11.5.8):
  - FR-EVT-CLOUD-001..004 must be emitted for cloud escalation request/approve/deny/execute, and MUST NOT include raw payloads (ProjectionPlan/ConsentReceipt references only).
- Work profile events (11.5.9):
  - FR-EVT-PROFILE-001..003 must be emitted when a Work Profile is selected/changed/resolved, including the resolved automation_level and allow_cloud_escalation fields.

### RED_TEAM_ADVISORY (security failure modes)
- AutoSignature abuse: an attacker (or buggy policy) uses AutoSignature to satisfy cloud escalation or policy-violation gates. Must be forbidden by construction and validated server-side.
- Forged decision linkage: AutoSignature references a GovernanceDecision that does not match gate_type/target_ref (confused deputy). Must enforce binding checks (decision_id + gate_type + target_ref) before applying.
- Replay: reuse of an old decision_id/AutoSignature to approve a different gate/target. Must ensure decision_id uniqueness and correct target binding.
- Leak risk: GovernanceDecision rationale/evidence_refs may accidentally include sensitive inline payloads. Events must be leak-safe (refs/hashes only).
- LOCKED behavior risk: in LOCKED, human intervention is forbidden. If the system proceeds anyway when confidence is below threshold, it becomes an unreviewable unsafe path. LOCKED must be fail-closed.
- Drift: Work Profile changes mid-job could change automation_level or cloud escalation settings after a decision is made. Profiles must be pinned per job/session and changes logged.

### PRIMITIVES (traits/structs/enums)
- AutomationLevel (enum): canonical values + normalization rules for legacy/imported values.
- GovernanceDecision (struct): canonical schema_version, required fields for audit, and explicit linkage to gates/targets.
- AutoSignature (struct): canonical schema_version and linkage fields; forbidden for cloud/policy gates.
- GateType (enum or canonical string set): stable gate_type strings used across decisions and FR events.
- GovernanceAutomationEvent (struct): FR-EVT-GOV-001..005 schema validation at ingestion time; reject unknown ids/types.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.132 defines canonical schemas/values for AutomationLevel, GovernanceDecision, AutoSignature, and FR-EVT-GOV events (plus conformance tests) and includes an explicit Stage merge alignment note.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: N/A

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.132 now includes 2.6.8.12.6 canonicalization + 10.13 merge alignment pinning; no further normative text is required.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 2.6.8.12.3 GovernanceDecision Artifact (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 10110
- CONTEXT_END_LINE: 10131
- CONTEXT_TOKEN: schema_version: "hsk.gov_decision@0.4";
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.8.12.3 GovernanceDecision Artifact (Normative)

  All autonomous/hybrid gate approvals MUST produce a GovernanceDecision artifact, linked into the Flight Recorder.

  export interface GovernanceDecision {
    schema_version: "hsk.gov_decision@0.4";
    decision_id: string;
    gate_type: string;
    target_ref: string;
    decision: "approve" | "reject" | "defer";
    confidence: number;
    rationale: string;
    evidence_refs?: string[];
    timestamp: string;
    actor: {
      kind: "human" | "model";
    };
  }
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 2.6.8.12.6 Autonomous Governance - Cross-Section Canonicalization (Normative) [ADD v02.132]
- CONTEXT_START_LINE: 10156
- CONTEXT_END_LINE: 10229
- CONTEXT_TOKEN: | "LOCKED";
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.8.12.6 Autonomous Governance - Cross-Section Canonicalization (Normative) [ADD v02.132]

  export type AutomationLevel =
    | "FULL_HUMAN"
    | "HYBRID"
    | "AUTONOMOUS"
    | "LOCKED";

  Normalization rules (required for Stage import compatibility):
  - Implementations MAY accept legacy values "ASSISTED" and "SUPERVISED" as inputs from older/imported specs; both MUST be normalized to "HYBRID" at ingestion boundaries.
  - Any mention in this spec of "governance is `LOCKED`" or "GovernanceMode is `LOCKED`" MUST be interpreted as AutomationLevel="LOCKED".
  - Any schema field named `automation_level` MUST accept the canonical set above (including "LOCKED"), even if an older snippet enumerates a smaller set.

  An AutoSignature is an artifact that satisfies a signature-like gate when AutomationLevel permits auto-approval.

  export interface AutoSignature {
    schema_version: "hsk.auto_signature@0.1";
    auto_signature_id: string;
    decision_id: string;
    gate_type: string;
    target_ref: string;
  }
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 10.13 Handshake Stage - Merge alignment notes (authoritative) [ADD v02.131]
- CONTEXT_START_LINE: 59696
- CONTEXT_END_LINE: 59697
- CONTEXT_TOKEN: Governance automation alignment: for AutomationLevel, GovernanceDecision, AutoSignature, and FR-EVT-GOV event IDs
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Merge alignment notes (authoritative):**
    - Governance automation alignment: for AutomationLevel, GovernanceDecision, AutoSignature, and FR-EVT-GOV event IDs, the canonical definitions are in 2.6.8.12 and 11.5.7. Where the imported Stage spec duplicates or conflicts (e.g., Stage sections 4.1-4.4 and 5.2), treat the imported blocks as informative and follow the canonical schemas/values/event IDs.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 11.5.7 FR-EVT-GOV-001..005 (Governance Automation Events) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 63606
- CONTEXT_END_LINE: 63638
- CONTEXT_TOKEN: type GovernanceAutomationEventType =
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.7 FR-EVT-GOV-001..005 (Governance Automation Events) (Normative) [ADD v02.120]

  type GovernanceAutomationEventType =
    | "gov_decision_created"               // FR-EVT-GOV-001
    | "gov_decision_applied"               // FR-EVT-GOV-002
    | "gov_auto_signature_created"         // FR-EVT-GOV-003
    | "gov_human_intervention_requested"   // FR-EVT-GOV-004
    | "gov_human_intervention_received";   // FR-EVT-GOV-005
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 11.1.7.3 Normative Rules for Cloud Escalation
- CONTEXT_START_LINE: 61601
- CONTEXT_END_LINE: 61601
- CONTEXT_TOKEN: - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
- EXCERPT_ASCII_ESCAPED:
  ```text
  - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
  ```
