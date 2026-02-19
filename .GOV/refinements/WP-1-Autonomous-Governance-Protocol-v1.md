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
- WP_ID: WP-1-Autonomous-Governance-Protocol-v1
- CREATED_AT: 2026-02-19T12:49:33Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.131.md
- SPEC_TARGET_SHA1: 9d9c3daea60ed8a00e5ef409c92a3d3c154adbcb
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Autonomous-Governance-Protocol-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Conflicting normative AutomationLevel definitions exist in the current Master Spec:
  - 2.6.8.12.1 defines AutomationLevel as FULL_HUMAN | HYBRID | AUTONOMOUS.
  - 10.13 Stage Spec v0.6 defines AutomationLevel as FULL_HUMAN | ASSISTED | SUPERVISED | AUTONOMOUS | LOCKED.
  This makes implementation ambiguous and breaks schema consistency for any surface that persists/serializes AutomationLevel.
- Conflicting normative GovernanceDecision schemas + schema_version strings exist:
  - 2.6.8.12.3 uses schema_version "hsk.gov_decision@0.4".
  - 10.13 Stage Spec v0.6 4.3 uses schema_version "hsk.governance_decision@0.4".
  They are not the same schema and cannot both be canonical without an explicit mapping rule.
- "LOCKED" is referenced as a governance state in multiple non-Stage sections (e.g., cloud escalation integration and cloud consent rules), but is not defined as a GovernanceMode value. The spec must pin which field/value represents LOCKED and how it interacts with cloud escalation and human intervention.
- Stage Spec v0.6 references governance event IDs that are not defined in the canonical Flight Recorder governance family (11.5.7 FR-EVT-GOV-001..005):
  - FR-EVT-GOV-SELF-APPROVED
  - FR-EVT-GOV-APPROVED-WITH-WARNINGS
  The spec must either (a) define these event IDs or (b) declare an authoritative mapping to FR-EVT-GOV-001..005.
- AutoSignature is referenced as an artifact in both 2.6.8.12.4 and Stage 10.13 4.4, but no canonical AutoSignature schema is defined.

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
- Specific: [ ] FAIL
- Measurable acceptance criteria: [ ] FAIL
- No ambiguity: [ ] FAIL
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: The Master Spec currently contains conflicting normative schemas (AutomationLevel, GovernanceDecision) and inconsistent event ID references across sections, so an implementation cannot be deterministic without an explicit canonicalization/enrichment step.
- AMBIGUITY_FOUND: YES
- AMBIGUITY_REASON: Conflicting definitions between 2.6.8.12 (autonomous governance) and 10.13 (Stage import), plus unpinned LOCKED semantics and governance event ID mismatches.

### ENRICHMENT
- ENRICHMENT_NEEDED: YES

- PROPOSED_SPEC_ENRICHMENT VERBATIM required if ENRICHMENT_NEEDED=YES:
```md
##### 2.6.8.12.6 Autonomous Governance - Cross-Section Canonicalization (Normative) [ADD v02.132]

This subsection resolves conflicting "AutomationLevel" and "GovernanceDecision" definitions that exist in multiple parts of the Master Spec (including the verbatim import in 10.13 Handshake Stage).

###### 2.6.8.12.6.1 AutomationLevel canonical set + normalization (Normative)

The canonical AutomationLevel values are:

~~~typescript
export type AutomationLevel =
  | "FULL_HUMAN"    // All gates require explicit human approval
  | "HYBRID"        // Some gates may be auto-approved subject to thresholds; critical gates require human
  | "AUTONOMOUS"    // System self-approves most gates with an audit trail
  | "LOCKED";       // No human intervention is permitted; fail-closed when human intervention would be required
~~~

Normalization rules (required for Stage import compatibility):
- Implementations MAY accept legacy values "ASSISTED" and "SUPERVISED" as inputs from older/imported specs; both MUST be normalized to "HYBRID" at ingestion boundaries.
- Any mention in this spec of "governance is `LOCKED`" or "GovernanceMode is `LOCKED`" MUST be interpreted as AutomationLevel="LOCKED".
- Any schema field named `automation_level` MUST accept the canonical set above (including "LOCKED"), even if an older snippet enumerates a smaller set.

LOCKED rules:
- In LOCKED, cloud escalation MUST be blocked (no consent prompts, no external transmission).
- In LOCKED, if a gate would normally require human intervention (e.g., confidence below threshold), the system MUST NOT proceed; it MUST record a "reject" or "defer" GovernanceDecision and halt the workflow.

###### 2.6.8.12.6.2 GovernanceDecision canonical schema (Normative)

The canonical GovernanceDecision artifact schema is defined in 2.6.8.12.3 and uses:
- schema_version: "hsk.gov_decision@0.4"

Cross-section alignment rules:
- Sections MUST NOT introduce alternate schema_version strings for the same GovernanceDecision concept (e.g., "hsk.governance_decision@0.4"); such duplicates are non-canonical.
- The 10.13 Handshake Stage imported spec's GovernanceDecision block is informative; implementations MUST use the canonical 2.6.8.12.3 schema.

###### 2.6.8.12.6.3 AutoSignature artifact schema (Normative) [ADD v02.132]

An AutoSignature is an artifact that satisfies a signature-like gate when AutomationLevel permits auto-approval.

~~~typescript
export interface AutoSignature {
  schema_version: "hsk.auto_signature@0.1";
  auto_signature_id: string;

  decision_id: string;   // MUST reference GovernanceDecision.decision_id
  gate_type: string;     // MUST match GovernanceDecision.gate_type
  target_ref: string;    // MUST match GovernanceDecision.target_ref

  created_at: string;
  actor: {
    kind: "model";
    model_id: string;
  };
}
~~~

Rules:
- AutoSignature MUST reference the corresponding GovernanceDecision by decision_id.
- AutoSignature MUST NOT be used for cloud escalation or policy violations.
- Creating an AutoSignature MUST emit FR-EVT-GOV-003 (gov_auto_signature_created).
- Applying an auto-approved decision MUST emit FR-EVT-GOV-002 (gov_decision_applied).

###### 2.6.8.12.6.4 Flight Recorder event alignment for self-approval (Normative)

The canonical governance automation event family is 11.5.7 FR-EVT-GOV-001..005.

Stage-imported event labels such as "FR-EVT-GOV-SELF-APPROVED" and "FR-EVT-GOV-APPROVED-WITH-WARNINGS" are deprecated aliases and MUST NOT be introduced as new Flight Recorder event IDs.

Self-approval MUST be represented using the existing family as a sequence:
1. FR-EVT-GOV-001 (gov_decision_created)
2. (optional) FR-EVT-GOV-004 (gov_human_intervention_requested) when human review is required and allowed
3. (optional) FR-EVT-GOV-005 (gov_human_intervention_received) when a human decision is provided
4. FR-EVT-GOV-003 (gov_auto_signature_created) when auto-approval is used
5. FR-EVT-GOV-002 (gov_decision_applied) when the gate is satisfied

##### 10.13 Handshake Stage - Merge alignment notes (authoritative) (ADD v02.132)

Add the following bullet to the existing "Merge alignment notes (authoritative)" list:

- Governance automation alignment: for AutomationLevel, GovernanceDecision, AutoSignature, and FR-EVT-GOV event IDs, the canonical definitions are in 2.6.8.12 and 11.5.7. Where the imported Stage spec duplicates or conflicts (e.g., Stage sections 4.1-4.4 and 5.2), treat the imported blocks as informative and follow the canonical schemas/values/event IDs.
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.131.md 2.6.8.12.3-2.6.8.12.4 Autonomous Governance Protocol (AutomationLevel) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 10079
- CONTEXT_END_LINE: 10149
- CONTEXT_TOKEN: schema_version: "hsk.gov_decision@0.4";
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.6.8.12 Autonomous Governance Protocol (AutomationLevel) (Normative) [ADD v02.120]

  export type AutomationLevel =
    | "FULL_HUMAN"
    | "HYBRID"
    | "AUTONOMOUS";

  export interface GovernanceDecision {
    schema_version: "hsk.gov_decision@0.4";
    decision_id: string;
    gate_type: string;
    target_ref: string;
    decision: "approve" | "reject" | "defer";
    confidence: number;
    rationale: string;
    timestamp: string;
    actor: {
      kind: "human" | "model";
    };
  }

  AutoSignature constraints:
  - AutoSignature MUST reference the corresponding GovernanceDecision.
  - AutoSignature MUST NOT be used for Cloud Escalation or Policy Violations.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.131.md 10.13 Handshake Stage Spec v0.6 4.1 AutomationLevel (normative)
- CONTEXT_START_LINE: 67061
- CONTEXT_END_LINE: 67069
- CONTEXT_TOKEN: type AutomationLevel =
- EXCERPT_ASCII_ESCAPED:
  ```text
  type AutomationLevel =
    | "FULL_HUMAN"
    | "ASSISTED"
    | "SUPERVISED"
    | "AUTONOMOUS"
    | "LOCKED";
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.131.md 10.13 Handshake Stage Spec v0.6 4.3 GovernanceDecision Artifact (normative)
- CONTEXT_START_LINE: 67084
- CONTEXT_END_LINE: 67134
- CONTEXT_TOKEN: schema_version: "hsk.governance_decision@0.4";
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface GovernanceDecision {
    schema_version: "hsk.governance_decision@0.4";
    decision_id: UUID;
    job_id: UUID;
    trace_id: UUID;
    timestamp: ISO8601Timestamp;
    decision_type: "spec_approval" | "refinement_approval" | "wp_approval" | "mt_approval" | "gate_pass" | "escalation";
    decision: "approved" | "rejected" | "deferred" | "escalated";
  }
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.131.md 11.5.7 FR-EVT-GOV-001..005 (Governance Automation Events) (Normative) [ADD v02.120]
- CONTEXT_START_LINE: 63530
- CONTEXT_END_LINE: 63565
- CONTEXT_TOKEN: type GovernanceAutomationEventType =
- EXCERPT_ASCII_ESCAPED:
  ```text
  type GovernanceAutomationEventType =
    | "gov_decision_created"               // FR-EVT-GOV-001
    | "gov_decision_applied"               // FR-EVT-GOV-002
    | "gov_auto_signature_created"         // FR-EVT-GOV-003
    | "gov_human_intervention_requested"   // FR-EVT-GOV-004
    | "gov_human_intervention_received";   // FR-EVT-GOV-005

  interface GovernanceAutomationEvent extends FlightRecorderEventBase {
    type: GovernanceAutomationEventType;
    decision_id: string;
    gate_type: string;
    target_ref: string;
    automation_level: "FULL_HUMAN" | "HYBRID" | "AUTONOMOUS";
  }
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.131.md 11.1.7.3 Normative Rules for Cloud Escalation
- CONTEXT_START_LINE: 61522
- CONTEXT_END_LINE: 61527
- CONTEXT_TOKEN: - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Cloud escalation MUST require explicit human consent regardless of AutomationLevel (2.6.8.12).
  - If GovernanceMode is `LOCKED`, cloud escalation MUST be denied.
  ```
