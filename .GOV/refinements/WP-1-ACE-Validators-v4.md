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
- WP_ID: WP-1-ACE-Validators-v4
- CREATED_AT: 2026-01-06T23:12:42.2661177Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
- SPEC_TARGET_SHA1: 648dfd52b7cd0ad8183b9a037746473b875fa2c8
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja070120260227
- USER_SIGNATURE_PREVIOUS: ilja070120260018 (initial packet creation)
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-ACE-Validators-v4

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec explicitly defines (a) the enforcement invariants and (b) the full list of required ACE runtime validators and their normative requirements.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Prompt injection detection MUST trigger the Atomic Poisoning Directive [HSK-ACE-VAL-101], including emission of `FR-EVT-SEC-VIOLATION` to Flight Recorder.

### RED_TEAM_ADVISORY (security failure modes)
- Hollow guard bypass: validators operate on hashes/handles only and never scan raw content [HSK-ACE-VAL-100].
- Unicode bypass: scanning is not NFC-normalized and case-folded, allowing homoglyph/casing tricks [HSK-ACE-VAL-102].
- Poisoning incomplete: PromptInjectionDetected does not globally poison the job and terminate workflow nodes; workspace continues mutating after detection [HSK-ACE-VAL-101].
- Cloud leakage bypass: CloudLeakageGuard does not recursively inspect bundle/dataset_slice member classifications.
- Determinism bypass: determinism_mode strict/replay invariants not enforced consistently, allowing replay drift or unseeded strict runs.

### PRIMITIVES (traits/structs/enums)
- Ace runtime validator interface (e.g., AceRuntimeValidator trait) and per-guard types:
  - ContextDeterminismGuard
  - ArtifactHandleOnlyGuard
  - CompactionSchemaGuard
  - MemoryPromotionGuard
  - CloudLeakageGuard
  - PromptInjectionGuard
  - JobBoundaryRoutingGuard
  - LocalPayloadGuard
  - RetrievalBudgetGuard
  - ContextPackFreshnessGuard
  - IndexDriftGuard
  - CacheKeyGuard
- Error surfaces:
  - AceError::PromptInjectionDetected (must trigger global poisoning directive)
  - AceError::CloudLeakageBlocked (blocks cloud calls when exportable/sensitivity rules violated)

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: SPEC_CURRENT explicitly defines the validator set and the required enforcement invariants (content awareness, atomic poisoning directive, normalization) with normative rules per validator subsection.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already provides normative requirements for the validators and enforcement invariants in 2.6.6.7.11; the work is implementation/alignment, not spec definition.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.6.6.7.11.0 Enforcement Invariants (MANDATORY) [HSK-ACE-VAL-100..102]
- CONTEXT_START_LINE: 6021
- CONTEXT_END_LINE: 6038
- CONTEXT_TOKEN: [HSK-ACE-VAL-101] Atomic Poisoning Directive:
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.7.11 Validators (runtime-enforced; required)

  The runtime MUST provide validators that reject violations:

  ##### 2.6.6.7.11.0 Enforcement Invariants (MANDATORY)

  **[HSK-ACE-VAL-100] Content Awareness Invariant:**
  Validators MUST NOT operate on hashes or handles alone. For `PromptInjectionGuard` and `CloudLeakageGuard`, the runtime MUST resolve and provide the **raw UTF-8 content** of all `retrieved_snippet` blocks to the validator.

  **[HSK-ACE-VAL-101] Atomic Poisoning Directive:**
  The `WorkflowEngine` MUST implement a global trap for `AceError::PromptInjectionDetected`. Upon detection:
  1.  Immediate commit of `JobState::Poisoned`.
  2.  Abrupt termination of all active workflow nodes.
  3.  Emission of `FR-EVT-SEC-VIOLATION` to Flight Recorder.
  4.  **No further workspace mutations** are permitted for that `job_id`.

  **[HSK-ACE-VAL-102] Normalization Requirement:**
  Scanning for injection patterns MUST be performed on **NFC-normalized, case-folded** text to prevent bypasses via homoglyphs or casing tricks.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.6.6.7.11.5 CloudLeakageGuard (Normative)
- CONTEXT_START_LINE: 6055
- CONTEXT_END_LINE: 6059
- CONTEXT_TOKEN: ##### 2.6.6.7.11.5 CloudLeakageGuard (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.7.11.5 CloudLeakageGuard (Normative)
  - **Requirement:** If `model_tier` is `Cloud`, the guard MUST scan all `artifact_handles` and `SourceRefs`.
  - **Requirement:** MUST block the call if any item has `exportable: false` or a `high` sensitivity classification.
  - **Requirement:** If a `SourceRef` points to a `bundle` or `dataset_slice`, the guard MUST check the classification of **every individual member** within that collection (Recursive Check).
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.6.6.7.11.6 PromptInjectionGuard (Normative)
- CONTEXT_START_LINE: 6060
- CONTEXT_END_LINE: 6063
- CONTEXT_TOKEN: ##### 2.6.6.7.11.6 PromptInjectionGuard (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.7.11.6 PromptInjectionGuard (Normative)
  - **Requirement:** MUST execute a substring scan on the resolved, **NFC-normalized** content of all `retrieved_snippet` blocks.
  - **Requirement:** Scan MUST include patterns: `[ "ignore previous", "new instructions", "system command", "developer mode" ]` and any profile-specific patterns.
  - **Requirement:** Detection MUST trigger the **[HSK-ACE-VAL-101] Atomic Poisoning Directive**.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.6.6.7.12 Logging + Acceptance Tests (minimum)
- CONTEXT_START_LINE: 6085
- CONTEXT_END_LINE: 6113
- CONTEXT_TOKEN: ##### 2.6.6.7.12 Logging + Acceptance Tests (minimum)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.7.12 Logging + Acceptance Tests (minimum)

  For each model call (job step), log to Flight Recorder (A11.5):
  - scope inputs + hashes
  - determinism mode
  - candidate source IDs/hashes (and candidate list artifact ref in replay mode)
  - selected IDs/hashes
  - truncation/compaction decisions
  - prompt envelope hashes
  - ContextSnapshot ID + hash
  - artifact handles referenced
  - QueryPlan ID + hash (ACE-RAG-001)
  - normalized_query_hash (ACE-RAG-001)
  - RetrievalTrace ID + hash (ACE-RAG-001; includes rerank/diversity metadata and spans)
  - per-stage cache hit/miss markers (ACE-RAG-001)
  - drift flags + degraded-mode marker (ACE-RAG-001)

  Minimum tests:
  - strict determinism hash test
  - replay determinism candidate list replay test
  - artifact-first enforcement test
  - compaction reversibility test
  - cloud leakage/default projection test
  - sub-agent isolation test
  - job-boundary routing test
  - local-only prompt payload retention/export test
  - retrieval scoring determinism test (same inputs -> same candidate order and selection)
  - ACE-RAG-001 conformance tests (see A2.6.6.7.14.13)
  ```

