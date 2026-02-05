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
- WP_ID: WP-1-Storage-Abstraction-Layer-v3
- CREATED_AT: 2026-01-09T18:33:00.081Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.103.md
- SPEC_TARGET_SHA1: b90ccd962e44fe99e0de7c727166fc98248d7c4c
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja090120261951
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Storage-Abstraction-Layer-v3

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Packet drift: existing WP-1-Storage-Abstraction-Layer packets are legacy format (non-ASCII / missing COR-701 manifest) and fail current workflow gates; requires a v3 revalidation packet anchored to SPEC_CURRENT.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE

### RED_TEAM_ADVISORY (security failure modes)
- Bypass risk: any direct sqlx/pool access outside the storage module reintroduces non-portable coupling and violates CX-DBP-010.
- Partial refactor risk: leaving a single leakage path (handlers, jobs, tests) undermines the mandatory audit acceptance criteria (CX-DBP-030).
- False compliance risk: hiding sqlx usage behind aliases or re-exports; audit must search for literal `sqlx::` and `SqlitePool` across `src/`.

### PRIMITIVES (traits/structs/enums)
- `Database` trait (src/backend/handshake_core/src/storage/mod.rs)
- `AppState` storage field (`Arc<dyn Database>`) and prohibition on exposing `SqlitePool`/`DuckDbConnection`

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Section 2.3.12 provides explicit MUST/FORBIDDEN rules and measurable audit evidence; no additional normative text is required.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Storage portability requirements and acceptance audits are explicit (CX-DBP-010, CX-DBP-040, CX-DBP-030).

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.103.md 2.3.12.5 [CX-DBP-030]
- CONTEXT_START_LINE: 3098
- CONTEXT_END_LINE: 3126
- CONTEXT_TOKEN: 1. **WP-1-Storage-Abstraction-Layer**
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.12.5 Phase 1 Closure Requirements [CX-DBP-030]

  Phase 1 CANNOT close without completing four foundational work packets:

  1. **WP-1-Storage-Abstraction-Layer**
     - Establish trait-based storage API
     - **MANDATORY AUDIT**: The codebase MUST be scanned for `sqlx::` and `SqlitePool` references.
       - **Acceptance Criteria**: Zero occurrences allowed outside of `src/backend/handshake_core/src/storage/`.
       - **Evidence**: `grep -r "sqlx::" src/ | grep -v "src/storage"` must return empty.
     - Audit all database access for compliance
     - Force all DB operations through storage module
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.103.md 2.3.12.1 [CX-DBP-010]
- CONTEXT_START_LINE: 2903
- CONTEXT_END_LINE: 2921
- CONTEXT_TOKEN: **Pillar 1: One Storage API [CX-DBP-010]**
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.12.1 Four Portability Pillars [CX-DBP-002]

  **Pillar 1: One Storage API [CX-DBP-010]**

  All database operations MUST flow through a single storage module boundary. No business logic code may directly access database connections.

  - FORBIDDEN: Direct `sqlx::query()` in API handlers
  - FORBIDDEN: Direct `state.pool` or `state.fr_pool` access outside `src/storage/`
  - REQUIRED: All DB operations via `state.storage.*` interface
  - REQUIRED: AppState MUST NOT expose raw `SqlitePool` or `DuckDbConnection`

  **Enforcement:**
  Pre-commit validation checks for direct pool access in API handlers (FAIL on violation).
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.103.md 2.3.12.3 [CX-DBP-040]
- CONTEXT_START_LINE: 3004
- CONTEXT_END_LINE: 3012
- CONTEXT_TOKEN: **[CX-DBP-040] Trait Purity Invariant (Normative):**
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.12.3 Storage API Abstraction Pattern [CX-DBP-021]

  The storage module MUST define a trait-based interface that hides database differences. This contract is MANDATORY for all storage implementations.

  **[CX-DBP-040] Trait Purity Invariant (Normative):**
  The `Database` trait MUST NOT expose any methods that return concrete, backend-specific types (e.g., `SqlitePool`, `PgPool`, `DuckDbConnection`). All implementations MUST encapsulate their internal connection pools.
  - **Violation:** `fn sqlite_pool(&self) -> Option<&SqlitePool>` is strictly FORBIDDEN.
  - **Remediation:** Any service requiring database access (e.g., Janitor, Search) MUST consume the generic `Database` trait methods or be refactored into a trait-compliant operation.
  ```

