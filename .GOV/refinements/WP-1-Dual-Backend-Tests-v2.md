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
- WP_ID: WP-1-Dual-Backend-Tests-v2
- CREATED_AT: 2026-01-06T22:28:17.1506166Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
- SPEC_TARGET_SHA1: 648dfd52b7cd0ad8183b9a037746473b875fa2c8
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060120262333
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Dual-Backend-Tests-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec explicitly defines dual-backend testing requirements (Pillar 4) and names WP-1-Dual-Backend-Tests as a Phase 1 closure requirement.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE. This WP is limited to storage-layer test parameterization and CI coverage; it does not introduce or modify Flight Recorder event shapes.

### RED_TEAM_ADVISORY (security failure modes)
- False confidence: CI only runs SQLite, silently skipping PostgreSQL due to misconfigured matrix or missing service container.
- Flakiness: PostgreSQL readiness not awaited (healthcheck/race), causing intermittent failures and undermining the "block PR merge" invariant.
- Secret leakage: DATABASE_URL or credentials printed in test logs or panic output.

### PRIMITIVES (traits/structs/enums)
- No new primitives required by spec. This WP should reuse the existing storage API (Database trait) and add/extend test harness plumbing to run against both backends.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec explicitly requires a PostgreSQL CI test variant, parameterized storage tests for both backends, and PR-merge blocking on failure; implementation details (CI service container vs docker-compose) are left to engineering choice and do not require spec enrichment.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: SPEC_CURRENT (Handshake_Master_Spec_v02.101.md) already contains explicit normative requirements for dual-backend testing and Phase 1 closure (CX-DBP-013, CX-DBP-030).

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.3.12.1 Pillar 4 (Dual-Backend Testing Early) [CX-DBP-013]
- CONTEXT_START_LINE: 2954
- CONTEXT_END_LINE: 2964
- CONTEXT_TOKEN: Dual-Backend Testing Early [CX-DBP-013]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.3.12.5 Phase 1 Closure Requirements [CX-DBP-030] (WP-1-Dual-Backend-Tests)
- CONTEXT_START_LINE: 3111
- CONTEXT_END_LINE: 3123
- CONTEXT_TOKEN: WP-1-Dual-Backend-Tests
- EXCERPT_ASCII_ESCAPED:
  ```text
  4. **WP-1-Dual-Backend-Tests**
     - Add PostgreSQL to test matrix (Docker container in CI)
     - Parameterize storage layer tests for both backends
     - Block PRs that fail on either backend
  ```

