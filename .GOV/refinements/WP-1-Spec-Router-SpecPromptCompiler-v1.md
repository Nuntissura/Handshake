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
- WP_ID: WP-1-Spec-Router-SpecPromptCompiler-v1
- CREATED_AT: 2026-02-27T09:46:09Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.139.md
- SPEC_TARGET_SHA1: 0a5a9069bf8e06654ddf9b647927c2cb8a30aa6f
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja270220261121
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Spec-Router-SpecPromptCompiler-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Handshake_Master_Spec_v02.139.md 2.6.8.9 requires:
  - Governance gate transitions MUST emit `FR-EVT-GOV-GATES-001`.
  - Stub activation (stub -> official packet + traceability mapping) MUST emit `FR-EVT-GOV-WP-001`.
- Handshake_Master_Spec_v02.139.md 2.6.8.5.2 requires Spec Router provenance emission per `spec_router` job:
  - `spec_prompt_pack_id`
  - `spec_prompt_pack_sha256`
  - `context_snapshot_id`
  - `prompt_envelope.stable_prefix_hash`
  - `prompt_envelope.variable_suffix_hash`
  - token counts for stable_prefix + variable_suffix (and truncation flags, if any)

### RED_TEAM_ADVISORY (security failure modes)
- Pack drift/tampering: changing `assets/spec_prompt_packs/spec_router_pack@1.json` changes behavior; mitigate by hashing exact JSON bytes (`spec_prompt_pack_sha256`) and persisting/deep-linking provenance.
- Non-deterministic compilation: unstable truncation/tokenization breaks replay guarantees; mitigate by deterministic truncation rules, hard token caps, and logging token counts + truncation flags.
- Capability hallucination: if CapabilitySnapshot is absent or not injected, models may invent tools/engines; mitigate by injecting the snapshot and requiring the model to reference only what it lists.
- Prompt injection: PROMPT_TEXT and injected context are untrusted; mitigate by stable-prefix hard rules + strict output contract and by recording full ContextSnapshot lineage for audit/replay.

### PRIMITIVES (traits/structs/enums)
- SpecPromptPackV1 (`hsk.spec_prompt_pack@1`)
- SpecPromptCompiler (deterministic compiler; runtime component)
- PromptEnvelope + ContextSnapshot (ACE runtime contracts)
- CapabilitySnapshotV1 (`hsk.capability_snapshot@1`) (injected and referenced via handle + hash)
- Provenance fields: `spec_prompt_pack_id`, `spec_prompt_pack_sha256`, `context_snapshot_id`, `prompt_envelope.stable_prefix_hash`, `prompt_envelope.variable_suffix_hash`, token counts + truncation flags

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec Main Body v02.139 explicitly defines asset location, a schema skeleton, a deterministic compiler contract, and required provenance + integration hooks (2.6.8.5.2 and 2.6.8.9).
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: v02.139 includes a full normative contract for SpecPromptPack + SpecPromptCompiler and explicitly states the required provenance/integration hooks.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 2.6.8.5.2 SpecPromptPack + SpecPromptCompiler (Normative) [ADD v02.139]
- CONTEXT_START_LINE: 9550
- CONTEXT_END_LINE: 9677
- CONTEXT_TOKEN: SpecPromptPack + SpecPromptCompiler (Normative) [ADD v02.139]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.8.5.2 SpecPromptPack + SpecPromptCompiler (Normative) [ADD v02.139]

  **Goal**  
  Make Prompt\\u2192Spec routing reproducible, debuggable, and resistant to stale/system-prompt drift by compiling a deterministic **PromptEnvelope** from a versioned, hashed **SpecPromptPack**, and by recording a full **ContextSnapshot**.

  This section is a specialization of the ACE Runtime prompt contracts:
  - ContextSnapshot (\\u00A72.6.6.7.3)  
  - PromptEnvelope (\\u00A72.6.6.7.4)

  **Asset location (HARD)**  
  - Spec prompt packs MUST live under: `assets/spec_prompt_packs/`  
  - The default prompt pack for `spec_router` MUST be:  
    - `assets/spec_prompt_packs/spec_router_pack@1.json`

  **SpecPromptPack (schema skeleton)**

  ```typescript
  export interface SpecPromptPackV1 {
    schema_version: "hsk.spec_prompt_pack@1";
    pack_id: string;                 // e.g. "spec_router_pack@1"
    description: string;
    target_job_kind: "spec_router";

    // Stable prefix is the invariant "SYSTEM RULES" + "OUTPUT CONTRACT" that should rarely change.
    stable_prefix_sections: Array<{
      section_id: string;            // e.g. "SYSTEM_RULES"
      content_md: string;            // markdown; no templating inside this field
    }>;

    // Variable suffix is the deterministic injection of prompt/context/capabilities for a single run.
    variable_suffix_template_md: string; // markdown template with placeholders listed below

    placeholders: Array<{
      name: string;                  // e.g. "PROMPT_TEXT"
      source: "prompt_ref" | "capability_snapshot" | "workflow_context" | "governance_mode";
      max_tokens: number;            // hard cap; enforced by TokenizationService + truncation flags
      required: boolean;
    }>;

    // Output contract references (human + machine)
    required_outputs: Array<{
      artifact_kind: "SpecIntent" | "SpecRouterDecision" | "SpecArtifact";
      schema_ref: string;            // e.g. "hsk.spec_intent@0.2" (string pointer; SSoT is schema repo)
    }>;

    budgets: {
      max_total_tokens: number;      // hard cap for the entire envelope
      max_prompt_excerpt_tokens: number;
      max_capsule_tokens: number;    // for any injected context capsule (if present)
      max_capability_table_tokens: number;
    };
  }
  ```

  **Default prompt pack skeleton (minimal viable)**  
  File: `assets/spec_prompt_packs/spec_router_pack@1.json`

  ```json
  {
    "schema_version": "hsk.spec_prompt_pack@1",
    "pack_id": "spec_router_pack@1",
    "description": "Deterministic prompt envelope skeleton for Spec Router (Prompt\\u2192Spec).",
    "target_job_kind": "spec_router",
    "stable_prefix_sections": [
      {
        "section_id": "SYSTEM_RULES",
        "content_md": "## SYSTEM RULES (HARD)\\n- You are running as Handshake Spec Router / Spec Author.\\n- You MUST NOT invent tools, engines, surfaces, connectors, events, or files.\\n- You MAY only reference items listed in CAPABILITY SNAPSHOT.\\n- If you lack information, record assumptions as NEEDS_CONFIRMATION in ## Assumptions.\\n- Output MUST follow OUTPUT CONTRACT exactly."
      },
      {
        "section_id": "OUTPUT_CONTRACT",
        "content_md": "## REQUIRED OUTPUTS (HARD)\\nYou MUST output, in order:\\n1) SpecIntent (JSON)\\n2) SpecRouterDecision (JSON)\\n3) Spec artifact (Markdown)\\n\\n## OUTPUT CONTRACT (STRICT)\\n- No extra prose outside the three artifacts.\\n- All IDs must be stable and machine-readable."
      }
    ],
    "variable_suffix_template_md": "## INPUTS\\n### User prompt\\n- prompt_ref: {{PROMPT_REF}}\\n- prompt_text: {{PROMPT_TEXT}}\\n\\n### Workspace/workflow context\\n- workspace_id: {{WORKSPACE_ID}}\\n- project_id: {{PROJECT_ID}}\\n- version_control: {{VERSION_CONTROL}}\\n- repo_root: {{REPO_ROOT}}\\n\\n## CAPABILITY SNAPSHOT (ALLOWED ONLY)\\n{{CAPABILITY_SNAPSHOT_TABLE}}\\n\\n## GOVERNANCE\\n- governance_mode: {{GOVERNANCE_MODE}}\\n- required_gates: {{REQUIRED_GATES}}\\n\\n## BEGIN WORK",
    "placeholders": [
      { "name": "PROMPT_REF", "source": "prompt_ref", "max_tokens": 64, "required": true },
      { "name": "PROMPT_TEXT", "source": "prompt_ref", "max_tokens": 900, "required": true },
      { "name": "WORKSPACE_ID", "source": "workflow_context", "max_tokens": 32, "required": true },
      { "name": "PROJECT_ID", "source": "workflow_context", "max_tokens": 32, "required": false },
      { "name": "VERSION_CONTROL", "source": "workflow_context", "max_tokens": 16, "required": true },
      { "name": "REPO_ROOT", "source": "workflow_context", "max_tokens": 64, "required": false },
      { "name": "CAPABILITY_SNAPSHOT_TABLE", "source": "capability_snapshot", "max_tokens": 900, "required": true },
      { "name": "GOVERNANCE_MODE", "source": "governance_mode", "max_tokens": 16, "required": true },
      { "name": "REQUIRED_GATES", "source": "governance_mode", "max_tokens": 128, "required": true }
    ],
    "required_outputs": [
      { "artifact_kind": "SpecIntent", "schema_ref": "hsk.spec_intent@0.2" },
      { "artifact_kind": "SpecRouterDecision", "schema_ref": "hsk.spec_router_decision@0.2" },
      { "artifact_kind": "SpecArtifact", "schema_ref": "hsk.feature_spec@0.2" }
    ],
    "budgets": {
      "max_total_tokens": 8000,
      "max_prompt_excerpt_tokens": 900,
      "max_capsule_tokens": 1200,
      "max_capability_table_tokens": 900
    }
  }
  ```

  **SpecPromptCompiler (contract) (HARD)**  
  The runtime MUST implement a deterministic compiler `SpecPromptCompiler` that:

  1. Loads the selected SpecPromptPack (default `spec_router_pack@1`) and computes:
     - `spec_prompt_pack_id`
     - `spec_prompt_pack_sha256` (hash of the exact JSON bytes)
  2. Generates (or accepts) a CapabilitySnapshot (\\u00A72.6.8.5.3) and injects it into the prompt.
  3. Compiles a PromptEnvelope (\\u00A72.6.6.7.4) where:
     - `stable_prefix` is the concatenation of `stable_prefix_sections` in order
     - `variable_suffix` is the template expansion with deterministic truncation rules
  4. Uses TokenizationService (\\u00A74.6.1) to enforce all placeholder token caps and the envelope total budget.
  5. Emits a ContextSnapshot (\\u00A72.6.6.7.3) that lists every artifact handle/hash used for compilation, at minimum:
     - `prompt_ref` (artifact handle + hash)
     - `capability_snapshot_ref` (artifact handle + hash)
     - `spec_prompt_pack_id` + `spec_prompt_pack_sha256`

  **Provenance emission (HARD)**  
  For every `spec_router` job, the system MUST record the following in Flight Recorder and make it visible in Operator Consoles:

  - `spec_prompt_pack_id`
  - `spec_prompt_pack_sha256`
  - `context_snapshot_id`
  - `prompt_envelope.stable_prefix_hash`
  - `prompt_envelope.variable_suffix_hash`
  - token counts for stable_prefix and variable_suffix (and truncation flags, if any)

  These fields MUST also be copied into SpecIntent and SpecRouterDecision (see \\u00A72.6.8.5 schemas) so downstream decomposition/validation can rehydrate the exact prompt context.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 2.6.8.9 Integration Hooks (Normative)
- CONTEXT_START_LINE: 9901
- CONTEXT_END_LINE: 9914
- CONTEXT_TOKEN: FR-EVT-GOV-WP-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.6.8.9 Integration Hooks (Normative)

  - Flight Recorder logs every router decision, refinement pass, signature, gate outcome, and spec status change.
    - Governance gate transitions MUST emit `FR-EVT-GOV-GATES-001`.
    - Stub activation (stub -> official packet + traceability mapping) MUST emit `FR-EVT-GOV-WP-001`.
  - [ADD v02.139] Spec Router provenance MUST be persisted and deep-linked (Operator Consoles + Debug Bundles): SpecPromptPack id/sha, CapabilitySnapshot ref/hash, ContextSnapshot id, PromptEnvelope hashes, and SpecLintReport refs when present.
  - Calendar integration creates review windows for GOV_STRICT specs and binds ActivitySpans to spec_id.
  - Monaco provides diff review for spec refinements and implementation deltas.
  - Canvas hosts dependency maps and stakeholder views for long-running specs.
  - Tables (Excel-like) hold gate matrices, risk registers, and test matrices per spec_id.
  - ACE runtime consumes SpecIntent and SpecArtifact as ContextSnapshot inputs for subsequent jobs.
  - Operator Consoles include a dedicated Spec Session Log view; entries deep-link to Flight Recorder traces for the same spec_id.
  - Atelier Lens runs on all ingested artifacts and spec-router inputs; claim/glance results are visible in Lens surfaces regardless of governance mode.
  ```
