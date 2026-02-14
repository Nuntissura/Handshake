## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2
- CREATED_AT: 2026-02-14T14:07:48.3906191Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- SPEC_TARGET_SHA1: 2d7e634514d4e3e2552f9a395c3b40da1184296b
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja140220261758
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. Master Spec v02.126 already defines Phase 1 defaults for MT ContextPacks targeting, policy knobs, stale handling semantics, and a pipeline binding in MT context compilation.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- RetrievalTrace.warnings[] MUST include outcome tokens from ContextPackFreshnessGuard when staleness is encountered:
  - context_pack_stale:fallback
  - context_pack_stale:fail
  - context_pack_stale:degrade_and_fallback
- ContextSnapshot should continue to record selection + hashes + truncations; no provenance weakening is permitted.

### RED_TEAM_ADVISORY (security failure modes)
- RT-STALE-001: Stale ContextPack treated as fresh -> context rot (wrong file chunks / wrong edits).
- RT-SCOPE-001: EntityRef-target packs can pull broader-than-intended context; SourceRef-first reduces accidental scope expansion.
- RT-POLICY-001: Policy knobs not enforced -> silent non-compliance (regen_required ignored; stale packs used).
- RT-PROV-001: Anchors-first payload must not weaken provenance binding; unsourced facts/constraints MUST NOT enter memory.

### PRIMITIVES (traits/structs/enums)
- ContextPackPolicy (normative policy fields): regen_allowed, regen_required, stale_handling.
- ContextPackFreshnessGuard (validator): enforces policy outcomes and emits RetrievalTrace warning tokens.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.126 includes explicit normative defaults + warning tokens; no additional enrichment is required for implementation.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec is already enriched in v02.126 (ContextPacks defaults + FreshnessGuard tokens + MT context compilation binding). This v2 WP is a post-enrichment packet variant for deterministic workflow tracking and implementation alignment.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 2.6.6.7.14.7 ContextPacks (mechanical compaction substrate)
- CONTEXT_START_LINE: 10634
- CONTEXT_END_LINE: 10689
- CONTEXT_TOKEN: interface ContextPackPolicy
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.7 ContextPacks (mechanical compaction substrate)

  **Phase 1 defaults (MT Context compilation)**

  - Target granularity (default): SourceRef-first (per-file / per-source)
  - Minimum payload (anchors-first; Phase 1)

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

  **Normative staleness handling semantics**
  - If a selected ContextPack is stale:
    - If `policy.regen_allowed=true`, the runtime MAY run `refresh_context_pack` and then re-check freshness.
    - If `policy.regen_required=false`:
      - The runtime MUST NOT use a stale pack as evidence; it MUST fall back to non-pack retrieval routes.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 2.6.6.7.14.11 Validators (Normative Traits) - ContextPackFreshnessGuard
- CONTEXT_START_LINE: 10778
- CONTEXT_END_LINE: 10791
- CONTEXT_TOKEN: context_pack_stale:degrade_and_fallback
- EXCERPT_ASCII_ESCAPED:
  ```text
  2) **ContextPackFreshnessGuard**
  - MUST enforce ContextPack staleness behavior using the active `ContextPackPolicy` (\u00a72.6.6.7.14.7).
  - If a selected ContextPack is stale:
    - If `policy.regen_allowed=true`, the runtime MAY attempt `refresh_context_pack` and MUST re-check freshness.
    - If `policy.regen_required=false`:
      - The runtime MUST NOT use a stale pack as evidence; it MUST fall back to non-pack retrieval routes.
  - Observability (normative): `RetrievalTrace.warnings[]` MUST include a warning token indicating the staleness outcome:
    - `context_pack_stale:fallback`
    - `context_pack_stale:fail`
    - `context_pack_stale:degrade_and_fallback`
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 2.6.6.8.8.2 Context Compilation Pipeline (MT ContextPacks usage)
- CONTEXT_START_LINE: 11761
- CONTEXT_END_LINE: 11788
- CONTEXT_TOKEN: target_ref = SourceRef(file)
- EXCERPT_ASCII_ESCAPED:
  ```text
  6. RETRIEVE file contents:
     - FOR EACH file IN MT.files.read \u222a MT.files.modify:
         target_ref = SourceRef(file)                       // SourceRef-first (Phase 1; \u00a72.6.6.7.14.7)
         IF prefer_context_packs=true:
             IF ContextPack(target_ref) exists:
                 IF ContextPack is fresh:
                     Include ContextPack synopsis + anchors[]   // anchors-first minimum payload
                 ELSE: // stale
                     IF policy.regen_allowed=true:
                         MAY refresh_context_pack(target_ref) and re-check freshness
                     IF ContextPack is now fresh:
                         Include ContextPack synopsis + anchors[]
                     ELSE:
                         // Apply ContextPackPolicy + ContextPackFreshnessGuard (\u00a72.6.6.7.14.7 / \u00a72.6.6.7.14.11)
                         // - MUST NOT use stale pack as evidence
                         // - MUST fall back to non-pack retrieval routes
                         // - IF policy.regen_required=true and stale remains:
                         //     * stale_handling=fail => abort compilation for this step
                         //     * stale_handling=degrade_and_fallback => mark degraded and continue fallback
                         // - MUST record RetrievalTrace.warnings[] += "context_pack_stale:{fallback|fail|degrade_and_fallback}"
                         Query Shadow Workspace for relevant chunks
                         Include chunks within budget
             ELSE:
                 Query Shadow Workspace for relevant chunks
                 Include chunks within budget
         ELSE:
             Query Shadow Workspace for relevant chunks
             Include chunks within budget
  ```
