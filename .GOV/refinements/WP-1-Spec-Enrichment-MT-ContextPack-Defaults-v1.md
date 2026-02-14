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
- WP_ID: WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v1
- CREATED_AT: 2026-02-14T01:19:50.082Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja140220260236
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Gap: No explicit Phase 1 defaults for ContextPacks target granularity for MT context compilation (SourceRef-first vs EntityRef).
- Gap: Spec references "regeneration required by policy" for stale ContextPacks, but the policy knobs/fields are not defined (regen_allowed/regen_required) and stale handling outcomes are not enumerated.
- Gap: ContextPackPayload minimum required payload for Phase 1 is not explicit (anchors-first; other arrays may be empty).
- Gap: MT Context Compilation Pipeline uses ContextPacks but does not normatively bind to these defaults/policy, leaving ambiguous implementation choices.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- For each retrieval-backed model call, use existing logging requirements (RetrievalTrace + warnings, ContextSnapshot, and degraded-mode marker) to capture:
  - context_pack_stale_detected: true|false
  - context_pack_regen_allowed: true|false
  - context_pack_regen_required: true|false
  - context_pack_stale_handling: fallback|fail|degrade_and_fallback
  - context_pack_regen_attempted: true|false
  - context_pack_regen_performed: true|false

### RED_TEAM_ADVISORY (security failure modes)
- RT-STALE-001: Stale ContextPack treated as fresh -> context rot (wrong file chunks / wrong edits).
- RT-SCOPE-001: EntityRef-target packs can pull broader-than-intended context; SourceRef-first reduces accidental scope expansion.
- RT-POLICY-001: "Policy required regen" without explicit knobs/outcomes leads to silent non-compliance (hidden drift).
- RT-PROV-001: Anchors-first packs must not weaken provenance binding; no unsourced facts/constraints may enter memory.

### PRIMITIVES (traits/structs/enums)
- ContextPackPolicy (normative policy fields): regen_allowed, regen_required, stale_handling.
- ContextPackStaleHandling (enum): fallback | fail | degrade_and_fallback.
- MT context compilation integration must reference ContextPackPolicy defaults (Phase 1).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: Current Main Body defines ContextPacks and freshness, but does not define Phase 1 defaults for target granularity, policy knobs (regen_allowed/regen_required), stale_handling outcomes, or anchors-first minimum payload; MT section does not bind to defaults.
- AMBIGUITY_FOUND: YES
- AMBIGUITY_REASON: "Regeneration required by policy" is referenced but policy fields and outcomes are unspecified; MT usage does not specify SourceRef-first default or stale handling.

### ENRICHMENT
- ENRICHMENT_NEEDED: YES
- REASON_NO_ENRICHMENT: <not applicable; ENRICHMENT_NEEDED=YES>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
**Phase 1 defaults (MT Context compilation)**

- Target granularity (default): SourceRef-first (per-file / per-source)
  - When compiling context for Micro-Tasks, the runtime MUST prefer ContextPacks whose `target` is a `SourceRef` corresponding to the specific file/source in scope.
  - `EntityRef` targets MAY be used for non-file entities, but MUST NOT replace SourceRef-first targeting for code file compilation in Phase 1.

- Minimum payload (anchors-first; Phase 1)
  - ContextPackPayload MUST include `synopsis` and `anchors[]`.
  - `facts[]`, `constraints[]`, and `open_loops[]` MAY be empty arrays.
  - Provenance rules are unchanged: any included `fact`, `constraint`, or `open_loop` MUST carry `source_refs[]` as already required.

- ContextPack staleness policy knobs (normative)
  - The active policy profile MUST define:

  ```typescript
  interface ContextPackPolicy {
    regen_allowed: boolean;
    regen_required: boolean;
    stale_handling: "fallback" | "fail" | "degrade_and_fallback";
  }
  ```

  - Phase 1 default ContextPackPolicy:
    - regen_allowed: true
    - regen_required: false
    - stale_handling: "fallback"

- Normative staleness handling semantics
  - If a selected ContextPack is stale:
    - If policy.regen_allowed=true, the runtime MAY run `refresh_context_pack` and then re-check freshness.
    - If policy.regen_required=true and a fresh pack was not produced, apply policy.stale_handling:
      - "fail": fail the retrieval-backed call (no compiled context is emitted).
      - "degrade_and_fallback": mark degraded and fall back to non-pack retrieval routes; emit an explicit warning and recovery hint.
    - If policy.regen_required=false:
      - The runtime MUST NOT use a stale pack as evidence; it MUST fall back to non-pack retrieval routes.


**ContextPackFreshnessGuard (policy integration)**
- ContextPackFreshnessGuard MUST enforce staleness behavior using ContextPackPolicy (regen_allowed, regen_required, stale_handling).
- ContextPackFreshnessGuard MUST surface the chosen staleness outcome in RetrievalTrace.warnings[] (e.g., "context_pack_stale:fallback", "context_pack_stale:fail", "context_pack_stale:degrade_and_fallback").


[ADD v02.126] MT ContextPacks defaults (Phase 1)
- When `prefer_context_packs=true`, MT context compilation MUST request packs per file using `target_ref: SourceRef` (SourceRef-first).
- Staleness handling MUST follow ContextPackPolicy defaults and MUST be observable in RetrievalTrace + ContextSnapshot.
- Minimum payload is anchors-first; packs may omit facts/constraints/open_loops (empty arrays are permitted).
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.6.6.7.14.7 ContextPacks (mechanical compaction substrate)
- CONTEXT_START_LINE: 10633
- CONTEXT_END_LINE: 10652
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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.6.6.7.14.11 Validators (Normative Traits)
- CONTEXT_START_LINE: 10711
- CONTEXT_END_LINE: 10751
- CONTEXT_TOKEN: Validators (Normative Traits)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.11 Validators (Normative Traits)

  The runtime MUST implement the `AceRuntimeValidator` trait. All retrieval operations MUST be validated by a pipeline of these guards.

  **Required Implementations:**

  2) **ContextPackFreshnessGuard**
  - Fail or mark degraded if a selected ContextPack is stale and regeneration was required by policy but not performed.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.6.6.8.8.2 Context Compilation Pipeline (MT ContextPacks usage)
- CONTEXT_START_LINE: 11666
- CONTEXT_END_LINE: 11736
- CONTEXT_TOKEN: MTContextCompilationPipeline
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.8.8.2 Context Compilation Pipeline

  The following pipeline MUST be used for each MT iteration:

  MTContextCompilationPipeline:

  1. RESOLVE scope:
     - scope_inputs = {
         entity_refs: MT.files.read \\u222a MT.files.modify,
         task_context: MT.scope + MT.actions,
         iteration_context: current iteration state
       }

  6. RETRIEVE file contents:
     - FOR EACH file IN MT.files.read \\u222a MT.files.modify:
         IF ContextPack exists AND is fresh:
             Include ContextPack summary + anchors
         ELSE:
             Query Shadow Workspace for relevant chunks
             Include chunks within budget
  ```
