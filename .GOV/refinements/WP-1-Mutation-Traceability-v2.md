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
- WP_ID: WP-1-Mutation-Traceability-v2
- CREATED_AT: 2026-01-18
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: CF2F5305FC8EEC517D577D87365BD9C072A99B0F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja180120261630
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Mutation-Traceability-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (requirements are present in Master Spec Main Body; this is remediation/revalidation work due to prior packet/spec drift).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (this WP is scoped to StorageGuard enforcement + persistence metadata and uses an error code on rejection; no explicit Flight Recorder event IDs are specified in the anchor text).

### RED_TEAM_ADVISORY (security failure modes)
- If `StorageGuard::validate_write` is not called on every write path in the `Database` trait, AI writes may bypass job/workflow context checks (silent edit).
- If persistence schema columns or constraints are missing/misaligned, AI writes can persist without required metadata (audit bypass).
- If `edit_event_id` is not generated/persisted, downstream traceability/audit chains lose the required anchor.
- If error code is incorrect or inconsistent, validators and tooling cannot deterministically classify silent-edit rejections.

### PRIMITIVES (traits/structs/enums)
- `MutationMetadata` (struct)
- `WriteActor` (enum)
- `StorageGuard` (trait)
- `GuardError::SilentEdit` (error variant; or equivalent mapping to `HSK-403-SILENT-EDIT`)
- Persistence schema columns: `last_actor_kind`, `last_actor_id`, `last_job_id`, `last_workflow_id`, `edit_event_id`

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides explicit primitives (struct/enum/trait), exact DB column names + invariant, and the required error code.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.113 already defines MutationMetadata, persistence schema requirements, StorageGuard requirements, and the required error code for silent-edit rejection.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.9.3 (Mutation Traceability)
- CONTEXT_START_LINE: 9252
- CONTEXT_END_LINE: 9276
- CONTEXT_TOKEN: pub struct MutationMetadata
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.9.3 Mutation Traceability (normative)

  To satisfy the traceability invariant (\\u00A77.6.3.8), every mutation to `RawContent` (e.g., document blocks) MUST persist metadata identifying the source of the change.

  #[derive(Debug, Serialize, Deserialize)]
  pub struct MutationMetadata {
      pub actor: WriteActor, // HUMAN | AI | SYSTEM
      pub job_id: Option<Uuid>,
      pub workflow_id: Option<Uuid>,
      pub timestamp: DateTime<Utc>,
  }

  #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
  pub enum WriteActor {
      Human,
      Ai,
      System,
  }

  1. **Storage Requirement:** Database tables for `blocks`, `cells`, and `nodes` MUST include `last_actor`, `last_job_id`, and `last_workflow_id` columns.
  2. **Audit Invariant:** Any row where `last_actor == 'AI'` MUST have a non-null `last_job_id` referencing a valid AI Job.
  3. **Silent Edit Block:** The storage guard (\\u00A7WP-1-Global-Silent-Edit-Guard) MUST verify that `MutationMetadata` is present and valid for all AI-authored writes.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.9.3.1 (Persistence Schema)
- CONTEXT_START_LINE: 9277
- CONTEXT_END_LINE: 9291
- CONTEXT_TOKEN: last_actor_kind
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.9.3.1 Persistence Schema (Normative)

  To support the `MutationMetadata` struct, the following columns MUST be added to all content tables (`blocks`, `canvas_nodes`, `canvas_edges`, `workspaces`, `documents`):

  | Column Name | SQL Type | Nullable | Description |
  | :--- | :--- | :--- | :--- |
  | `last_actor_kind` | `TEXT` | NO | Enum: "HUMAN", "AI", "SYSTEM" |
  | `last_actor_id` | `TEXT` | YES | User ID or System Component ID |
  | `last_job_id` | `TEXT` | YES | UUID of the AI Job (REQUIRED if kind='AI') |
  | `last_workflow_id` | `TEXT` | YES | UUID of the parent Workflow (REQUIRED if kind='AI') |
  | `edit_event_id` | `TEXT` | NO | UUID of the specific mutation event (traceability anchor) |

  **Invariant:** A database check constraint (or strict application logic) MUST enforce:
  `CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL)`
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.9.3.2 (Storage Guard Trait)
- CONTEXT_START_LINE: 9292
- CONTEXT_END_LINE: 9324
- CONTEXT_TOKEN: pub trait StorageGuard
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.9.3.2 Storage Guard Trait

  The application MUST implement the `StorageGuard` trait for all persistence operations. This trait acts as the final gate against silent edits.

  /// HSK-TRAIT-001: Storage Guard
  #[async_trait]
  pub trait StorageGuard: Send + Sync {
      /// Verifies the write request against the "No Silent Edits" policy.
      /// Returns:
      /// - Ok(MutationMetadata): If allowed. Metadata MUST be returned for DB insertion.
      /// - Err(GuardError::SilentEdit): If AI attempts write without job/approval context.
      async fn validate_write(
          &self,
          actor: &WriteActor,
          resource_id: &str,
          job_id: Option<Uuid>,
          workflow_id: Option<Uuid>
      ) -> Result<MutationMetadata, GuardError>;
  }

  **Guard Implementation Requirements:**

  1.  **AI Write Context:** If `actor == WriteActor::Ai`, the guard MUST fail if `job_id` is `None`.
  2.  **Traceability Anchor:** The guard MUST generate a unique `edit_event_id` (UUID) for every successful validation and return it in `MutationMetadata`.
  3.  **Error Codes:** Use `HSK-403-SILENT-EDIT` for rejection.

  **Integration Invariant:**
  All database persistence methods in the `Database` trait (e.g., `save_blocks`, `update_canvas`) MUST call `validate_write` and persist the returned `MutationMetadata` fields.
  ```

