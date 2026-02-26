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
- WP_ID: WP-1-Front-End-Memory-System-v1
- CREATED_AT: 2026-02-25T23:52:46Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.138.md
- SPEC_TARGET_SHA1: D2A3C38AAC420702C176EE62AF743750366CFBFA
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja260220260100
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Front-End-Memory-System-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE for v02.138 FEMS Phase 1 baseline: Main Body already defines job profile, runtime FEMS policy, session integration, DCC panel, event schema, and test suite.
- Implementation note for packet quality: split execution can remain as one packet with sub-checklists, but done-means must explicitly map to all FEMS v02.138 anchors listed in this refinement.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Required event family: FR-EVT-MEM-001 through FR-EVT-MEM-005.
- Trigger expectations:
  - FR-EVT-MEM-001 on proposal creation (`MemoryWriteProposal` artifact created).
  - FR-EVT-MEM-002 on review decision (approved/rejected/partial).
  - FR-EVT-MEM-003 on committed memory mutation (`MemoryCommitReport` persisted).
  - FR-EVT-MEM-004 on `MemoryPack` build for model invocation.
  - FR-EVT-MEM-005 on item status transitions (pin/unpin/invalidate/tombstone/supersede/merge).
- Privacy hard rule: memory events log IDs/hashes/artifact refs only; no raw memory content.

### RED_TEAM_ADVISORY (security failure modes)
- Memory poisoning: untrusted tool/web/user text promoted into procedural memory without review can weaponize future prompts.
- Drift amplification: uncontrolled pack size and non-deterministic truncation can produce replay mismatch and silent behavior drift.
- Cloud leakage: high-sensitivity or contact-scoped memory can leak to cloud providers without enforced consent/redaction policy.
- UI bypass risk: DCC panel must not mutate memory directly; all writes must flow through proposal -> review -> commit with audit trail.

### PRIMITIVES (traits/structs/enums)
- Job profile and artifacts: `memory_extract_v0.1`, `memory_consolidate_v0.1`, `memory_forget_v0.1`, `MemoryWriteProposal`, `MemoryCommitReport`, `MemoryPack`.
- Session integration: `memory_policy` enum (`EPHEMERAL`, `SESSION_SCOPED`, `WORKSPACE_SCOPED`), `memory_state_ref`, cloud-safe pack variant.
- Event primitives: `MemoryEventCode`, `MemoryWriteProposedEvent`, `MemoryWriteReviewedEvent`, `MemoryWriteCommittedEvent`, `MemoryPackBuiltEvent`, `MemoryItemStatusChangedEvent`.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: v02.138 provides explicit normative sections for FEMS runtime contracts, session integration, operator panel, event schema, and FEMS-EVAL-001 acceptance checks. Requirements are concrete and testable without additional spec enrichment.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Required FEMS Phase 1 behavior and acceptance criteria are already captured in Main Body sections 2.6.6.6.6, 2.6.6.7.6.2, 4.3.9.12.7, 5.4.8, 10.11.5.14, and 11.5.13.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 2.6.6.6.6 Front End Memory Job Profile (FEMS) (Normative)
- CONTEXT_START_LINE: 10288
- CONTEXT_END_LINE: 10303
- CONTEXT_TOKEN: ##### 2.6.6.6.6 Front End Memory Job Profile (FEMS) (Normative) [ADD v02.138]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.6.6 Front End Memory Job Profile (FEMS) (Normative) [ADD v02.138]

  The Front End Memory System (FEMS) is implemented as explicit, auditable AI jobs
  that propose and commit durable MemoryItems.

  It MUST NOT silently mutate LongTermMemory from inside an interactive chat loop.

  Required artifacts:
  - MemoryWriteProposal (canonical JSON; hashable)
  - MemoryCommitReport (diff + ids + policy decisions)
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 2.6.6.7.6.2 Front End Memory System (FEMS) (Normative)
- CONTEXT_START_LINE: 10708
- CONTEXT_END_LINE: 10742
- CONTEXT_TOKEN: ###### 2.6.6.7.6.2.1 Design principles (HARD)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.7.6.2 Front End Memory System (FEMS) (Normative) [ADD v02.138]

  ###### 2.6.6.7.6.2.1 Design principles (HARD)

  - MemoryPack injected tokens \u2264 500 (default).
  - MemoryPack items \u2264 24 (default).
  - If budgets are exceeded, degradation MUST be deterministic and logged.
  - Procedural memory is trusted-only and MUST NOT be created from untrusted text
    or tool output without review.
  - Replay mode MUST persist selected IDs and memory_pack_hash.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 4.3.9.12.7 Front End Memory System integration (FEMS) (Normative)
- CONTEXT_START_LINE: 30893
- CONTEXT_END_LINE: 30924
- CONTEXT_TOKEN: ##### 4.3.9.12.7 Front End Memory System integration (FEMS) (Normative) [ADD v02.138]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 4.3.9.12.7 Front End Memory System integration (FEMS) (Normative) [ADD v02.138]

  Read semantics:
  - EPHEMERAL: no MemoryPack injected; no memory write proposals generated.
  - SESSION_SCOPED: inject session-scoped working memory only.
  - WORKSPACE_SCOPED: inject bounded MemoryPack from LongTermMemory/project/WP scope.

  Write semantics:
  - Sessions may produce MemoryWriteProposal artifacts.
  - Commits to LongTermMemory MUST be explicit commit jobs and never implicit.

  Cloud boundary:
  - Build cloud-safe pack variants with consent/redaction policy enforcement.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 5.4.8 Front End Memory System Test Suite (FEMS-EVAL-001) (Normative)
- CONTEXT_START_LINE: 22341
- CONTEXT_END_LINE: 22370
- CONTEXT_TOKEN: ### 5.4.8 Front End Memory System Test Suite (FEMS-EVAL-001) (Normative) [ADD v02.138]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 5.4.8 Front End Memory System Test Suite (FEMS-EVAL-001) (Normative) [ADD v02.138]

  - Budget + truncation: token_estimate \u2264 500 and deterministic truncation warnings.
  - Provenance: committed items and pack items carry bounded SourceRefs.
  - Anti-poisoning: untrusted content cannot promote to procedural memory without review.
  - Determinism: replay mode reproduces pack hash and selected memory IDs.
  - Cloud redaction correctness and consolidation/conflict behavior are validated.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 10.11.5.14 Front End Memory Panel (FEMS)
- CONTEXT_START_LINE: 59182
- CONTEXT_END_LINE: 59204
- CONTEXT_TOKEN: #### 10.11.5.14 Front End Memory Panel (FEMS) [ADD v02.138]
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 10.11.5.14 Front End Memory Panel (FEMS) [ADD v02.138]

  Required views:
  - Memory Browser with scope/class/type/trust/status/classification filters.
  - Memory Write Review with approve/reject and evidence links.
  - MemoryPack Preview with token estimate and pack hash.
  - Conflict/consolidation queue with governed merge jobs.

  Hard rules:
  - UI edits MUST NOT directly mutate MemoryItems.
  - Panel MUST respect classification and consent for preview/export.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.138.md 11.5.13 Front End Memory System events (FR-EVT-MEM-*) (Normative)
- CONTEXT_START_LINE: 55616
- CONTEXT_END_LINE: 55679
- CONTEXT_TOKEN: ### 11.5.13 Front End Memory System events (FR-EVT-MEM-*) (Normative) [ADD v02.138]
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.5.13 Front End Memory System events (FR-EVT-MEM-*) (Normative) [ADD v02.138]

  FEMS MUST emit:
  - FR-EVT-MEM-001 memory_write_proposed
  - FR-EVT-MEM-002 memory_write_reviewed
  - FR-EVT-MEM-003 memory_write_committed
  - FR-EVT-MEM-004 memory_pack_built
  - FR-EVT-MEM-005 memory_item_status_changed

  Privacy rule (HARD): events log IDs/hashes/artifact refs, not raw memory content.
  ```
