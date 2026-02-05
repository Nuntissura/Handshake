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
- WP_ID: WP-1-Capability-SSoT-v2
- CREATED_AT: 2026-01-18T14:46:07.1274574Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja180120261552
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Capability-SSoT-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (Master Spec provides sufficient normative text for this WP; no spec enrichment required).
- NOTE: Current repo has no top-level `assets/` directory. Implementing `assets/capability_registry.json` will introduce a new top-level directory and requires explicit user confirmation per Handshake Codex v1.4 [CX-106].

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Capability checks MUST emit a Flight Recorder event on every allow/deny decision (Spec 11.1 [HSK-4001] Audit Requirement).
- Trigger points (minimum):
  - Workflow Engine capability enforcement before running a job.
  - Any other gate that evaluates capabilities (e.g., terminal/mechanical engines/MCP gate), if/when they call the registry.
- Minimum event payload keys (spec-required):
  - capability_id
  - actor_id
  - job_id (when applicable)
  - decision_outcome

### RED_TEAM_ADVISORY (security failure modes)
- Magic-string capability bypass: if code paths accept unregistered capability IDs, an attacker can mint a new capability string and skip gating.
- Audit trail erosion: if allow/deny checks are not logged (or logged without job_id), incident response cannot attribute side effects to a job.
- Actor misattribution: if actor_id is overloaded with capability_profile_id, downstream analysis cannot separate "who acted" from "what was granted".
- Registry drift: if registry content is hardcoded and not generated/validated, capability IDs can diverge from spec and mechanical_engines.json requirements silently.

### PRIMITIVES (traits/structs/enums)
- Rust:
  - CapabilityRegistry (valid_axes, valid_full_ids; can_perform resolution)
  - CapabilityProfile (whitelist of IDs from the registry)
  - RegistryError::UnknownCapability (HSK-4001) and related errors
- Artifacts (from 11.1.6):
  - capability_registry_draft.json
  - capability_registry_diff.json
  - capability_registry_review.json
  - assets/capability_registry.json

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec 11.1 (HSK-4001) defines SSoT + rejection + audit logging requirements; 11.1.3.1/11.1.3.2 defines axis:scope and Rust contract; 11.1.6 defines the registry generation workflow and required artifacts.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec provides sufficient normative requirements and concrete workflows for capability registry enforcement and generation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.1 Capability Registry & SSoT Enforcement (HSK-4001)
- CONTEXT_START_LINE: 44644
- CONTEXT_END_LINE: 44649
- CONTEXT_TOKEN: HSK-4001: UnknownCapability
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Capability Registry & SSoT Enforcement ([HSK-4001]):**
    - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
    - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
    - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
    - **Profile Schema:** `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') MUST be defined solely as whitelists of IDs from the `CapabilityRegistry`.
    - **Registry Generation:** `CapabilityRegistry` MUST be generated from the Master Spec and `mechanical_engines.json` into `assets/capability_registry.json` using an AI-assisted extraction job (local or cloud model allowed) with schema validation and human review; Spec Router and policy evaluation MUST pin `capability_registry_version` in their outputs.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.1.6 Capability Registry Generation Workflow (Normative)
- CONTEXT_START_LINE: 44833
- CONTEXT_END_LINE: 44855
- CONTEXT_TOKEN: profile_id=capability_registry_build
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.1.6 Capability Registry Generation Workflow (Normative)

  The CapabilityRegistry is generated by an AI-assisted extraction workflow (local or cloud model allowed) and then validated and reviewed before publish. The workflow runs as a governed job (job_kind=workflow_run, profile_id=capability_registry_build).

  Inputs:
  - Master spec (Handshake_Master_Spec_v*).
  - `mechanical_engines.json`.
  - Previous `assets/capability_registry.json` (optional).
  - Capability registry schema (JSON Schema).

  Workflow:
  1. Extract: run AI-assisted extraction to produce `capability_registry_draft.json`. The job MUST record model_id, policy decision, and prompt hashes in Flight Recorder. Cloud model usage MUST obey CloudLeakageGuard and use redacted or derived inputs only.
  2. Validate: run schema validation and integrity checks (unique capability_id, valid section_ref, required_capabilities present, risk_class set).
  3. Diff: produce `capability_registry_diff.json` against the previous registry (if any).
  4. Review: require human approval; record decision in Flight Recorder with reviewer_id and diff hash.
  5. Publish: write `assets/capability_registry.json`, bump registry version, and log a publish event.

  Artifacts:
  - `capability_registry_draft.json`
  - `capability_registry_diff.json`
  - `capability_registry_review.json`
  - `assets/capability_registry.json`
  ```

