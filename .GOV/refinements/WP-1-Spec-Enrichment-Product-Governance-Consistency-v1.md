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
- WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1
- CREATED_AT: 2026-02-12T02:22:36.477Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120220260342
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Spec-Enrichment-Product-Governance-Consistency-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Spec drift: Locus and work-tracking sections still reference repo-local `docs/TASK_BOARD.md` and `docs/task_packets/...` as runtime sources of truth, which conflicts with the repo/runtime boundary rule and runtime governance state root `.handshake/gov/`.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (Spec-only consistency correction; no runtime behavior changes required by this refinement.)

### RED_TEAM_ADVISORY (security failure modes)
- RT-BOUNDARY-001: if runtime depends on repo-local `docs/**` paths, product portability breaks and leads to undefined behavior in non-Handshake repos/workspaces.
- RT-BOUNDARY-002: boundary drift can cause accidental runtime reads/writes of `/.GOV/**` (explicitly forbidden) and reintroduce governance leakage risk.
- RT-DETERMINISM-001: inconsistent path vocabulary encourages host-specific/absolute path examples, harming determinism and portability.

### PRIMITIVES (traits/structs/enums)
- NONE (Spec-only correction pass.)

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec explicitly states the repo/runtime boundary (no runtime reads/writes of `/.GOV/**`; runtime state in `.handshake/gov/`) while also containing outdated `docs/**` runtime references; this WP resolves that internal inconsistency.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: This is a consistency correction within already-defined boundary rules; it does not introduce new normative requirements, only aligns stale `docs/**` references with the existing `.handshake/gov/` runtime governance state rule.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.3.15 Locus Work Tracking System (Task Board + Task Packet refs)
- CONTEXT_START_LINE: 5400
- CONTEXT_END_LINE: 5456
- CONTEXT_TOKEN: docs/TASK_BOARD.md
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Task Board**: The markdown table in `docs/TASK_BOARD.md` that provides human-readable project status. Locus syncs bidirectionally with it.
  - **Task Packet**: The structured spec in `docs/task_packets/{WP_ID}.md` with IN_SCOPE_PATHS, DONE_MEANS, TEST_PLAN. Locus links to these.

  | **Task Board** | Bidirectional Sync | `locus_sync_task_board` reads/writes `docs/TASK_BOARD.md` |
  | **Task Packets** | Reference | WP.governance.task_packet_path links to `docs/task_packets/{WP_ID}.md` |
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md Repo/runtime boundary (HARD) + runtime governance state root `.handshake/gov/`
- CONTEXT_START_LINE: 28514
- CONTEXT_END_LINE: 28518
- CONTEXT_TOKEN: .handshake/gov/
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Repo/runtime boundary (HARD)**
  - `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
  - `docs/` MAY exist as a temporary compatibility bundle only (non-authoritative governance state).
  - Handshake product runtime MUST NOT read from or write to `/.GOV/` (hard boundary; enforce via CI/gates).
  - Runtime governance state MUST live in product-owned storage. Handshake default: `.handshake/gov/` (configurable). This directory contains runtime governance state only.
  ```

