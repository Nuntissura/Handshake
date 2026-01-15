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
- WP_ID: WP-1-Role-Mailbox-v1
- CREATED_AT: 2026-01-13T06:20:00Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- SPEC_TARGET_SHA1: 6418250deb1fea1fce63ace65c22657e0398a113
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Role-Mailbox-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec defines the purpose, non-authority rule, normative data model, invariants, deterministic repo export format, and required Flight Recorder + Spec Session Log telemetry.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Role Mailbox message created: MUST emit `FR-EVT-GOV-MAILBOX-001` with correlation fields (`spec_id`, `work_packet_id`, `thread_id`, `message_id`, `governance_mode`, `from_role`, `to_roles[]`, `message_type`, `body_sha256`).
- Mailbox export updated: MUST emit `FR-EVT-GOV-MAILBOX-002` and a Spec Session Log entry `event_type=mailbox_exported`.
- Transcription link created: MUST emit `FR-EVT-GOV-MAILBOX-003` and a Spec Session Log entry `event_type=mailbox_transcribed`.
- Flight Recorder ingestion MUST enforce strict payload shape validation for mailbox events (11.5.3) and reject forbidden inline body fields/unbounded text.

### RED_TEAM_ADVISORY (security failure modes)
- Data leakage: repo exports may contain secrets/PII unless content classification/redaction is enforced; treat `docs/ROLE_MAILBOX/` as potentially sensitive.
- Path traversal/injection: never allow `thread_id` / `message_id` to create arbitrary paths; sanitize and constrain to allowed filename charset.
- Integrity: ensure `body_sha256` matches referenced body artifact; prevent tampering by verifying hashes on load/export.
- DoS/storage blowup: enforce size limits per message/body, per thread, and total export; refuse or chunk large payloads deterministically.
- Concurrency/races: multiple roles writing simultaneously can corrupt exports; require atomic append or write-to-temp + rename, with deterministic ordering.
- Replay/duplication: enforce `idempotency_key` semantics to prevent duplicated messages on retries.

### PRIMITIVES (traits/structs/enums)
- `RoleId`, `RoleMailboxMessageType`, `RoleMailboxContext`, `RoleMailboxThread`, `TranscriptionTargetKind`, `TranscriptionLink`, `RoleMailboxMessage`.
- Canonical JSON serializer (stable key order, `\\uXXXX` escaping, `\\n` newlines) and export manifest schema (`export_manifest.json` + hashes).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS (deterministic export + required FR events + blocking rule on out-of-sync export)
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides normative data model + invariants + deterministic repo export requirements and explicitly blocks relying on mailbox messages as authority.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already defines the required behavior and telemetry precisely enough to implement and validate; remaining choices (storage engine, locking strategy) can be implementation details as long as determinism + invariants hold.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.6.8.10 (Role Mailbox) (Normative)
- CONTEXT_START_LINE: 5987
- CONTEXT_END_LINE: 6200
- CONTEXT_TOKEN: FR-EVT-GOV-MAILBOX-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.6.8.10 Role Mailbox (Normative)

  **Purpose**  
  Reduce Operator copy/paste friction while preserving strict governance by providing a durable role-to-role messaging substrate that is:
  - auditable (Flight Recorder + Spec Session Log)
  - artifact-linked (messages reference artifacts/hashes, not ephemeral chat)
  - repo-exported (machine-friendly shared memory for small-context handoff)

  **Non-authority rule (HARD): chat is not state**  
  Role Mailbox messages are never authoritative by default.  
  Any decision that changes scope, requirements, safety posture, acceptance criteria, waivers, or verdicts MUST be transcribed into canonical governance artifacts (refinement/task packet/task board/waiver/audit logs).  
  Mailbox messages MAY coordinate work, but MUST NOT substitute for signed artifacts, gate state, or validator reports.

  **Data model (normative)**

  pub enum RoleId {
      Operator,
      Orchestrator,
      Coder,
      Validator,
      Advisory(String), // optional helper agents; never replaces the Trinity
  }

  pub enum RoleMailboxMessageType {
      ClarificationRequest,
      ClarificationResponse,
      ScopeRisk,
      ScopeChangeProposal,
      ScopeChangeApproval,
      WaiverProposal,
      WaiverApproval,
      ValidationFinding,
      Handoff,
      Blocker,
      ToolingRequest,
      ToolingResult,
      FYI,
  }

  pub struct RoleMailboxContext {
      pub spec_id: Option<String>,
      pub work_packet_id: Option<String>,
      pub task_board_id: Option<String>,
      pub governance_mode: GovernanceMode,
      pub project_id: Option<String>,
  }

  pub struct RoleMailboxThread {
      pub thread_id: String,          // stable id; UUID or hash-based
      pub subject: String,
      pub context: RoleMailboxContext,
      pub participants: Vec<RoleId>,  // explicit
      pub created_at: DateTime<Utc>,
      pub closed_at: Option<DateTime<Utc>>,
  }

  pub enum TranscriptionTargetKind {
      Refinement,
      TaskPacket,
      TaskBoard,
      GateState,
      SignatureAudit,
      Waiver,
      SpecArtifact,
  }

  pub struct TranscriptionLink {
      pub target_kind: TranscriptionTargetKind,
      pub target_ref: ArtifactHandle, // or repo-path artifact handle
      pub target_sha256: String,
      pub note: String,               // short, machine-safe summary (no new requirements)
  }

  pub struct RoleMailboxMessage {
      pub message_id: String,             // stable id; UUID or hash-based
      pub thread_id: String,
      pub created_at: DateTime<Utc>,
      pub from_role: RoleId,
      pub to_roles: Vec<RoleId>,
      pub message_type: RoleMailboxMessageType,

      // Body MUST be stored as an artifact and referenced
      pub body_ref: ArtifactHandle,
      pub body_sha256: String,

      pub attachments: Vec<ArtifactHandle>,
      pub relates_to_message_id: Option<String>,

      // Required for governance-critical message types:
      pub transcription_links: Vec<TranscriptionLink>,

      // Deterministic idempotency (prevents duplicates on retries)
      pub idempotency_key: String,
  }

  **Invariants (HARD)**
  - Every mailbox message MUST emit a Flight Recorder event `FR-EVT-GOV-MAILBOX-001` with correlation fields (`spec_id`, `work_packet_id`, `thread_id`, `message_id`, `governance_mode`, `from_role`, `to_roles[]`, `message_type`, `body_sha256`).
  - Every mailbox message MUST append a Spec Session Log entry (2.6.8.8) with `event_type=mailbox_message_created`.
  - Any message of type `ScopeChangeApproval`, `WaiverApproval`, or `ValidationFinding` MUST include at least one `TranscriptionLink` to the authoritative artifact where it was recorded.
  - Advisory roles MAY be used, but MUST NOT replace the Trinity roles for GOV_STANDARD and GOV_STRICT (see 11.1.5.1).

  **Flight Recorder mailbox event schemas (HARD)**
  - `FR-EVT-GOV-MAILBOX-001/002/003` MUST be implemented as dedicated Flight Recorder event schemas with strict payload shape validation (see 11.5.3).
  - Ingestion MUST reject any mailbox event that is missing required correlation fields or includes forbidden fields (especially inline message body content).

  **Secret-leak prevention (HARD)**
  - Mailbox message bodies MUST NOT be recorded inline in Flight Recorder event payloads. Only `body_ref` + `body_sha256` are permitted.
  - The repo export under `docs/ROLE_MAILBOX/` MUST NOT contain mailbox message bodies. Only `body_ref` + `body_sha256` are permitted.
  - Any exported free-text fields (thread subject, transcription note) MUST be passed through the Secret Redactor before being written to disk (see 11.5) and MUST be bounded (single-line; <= 160 chars).

  **Repo export (machine-friendly; always-on for GOV_STANDARD and GOV_STRICT)**  
  Handshake MUST maintain an always-on, deterministic mailbox export into the project repo at:
  - `docs/ROLE_MAILBOX/`

  Rules:
  - For GOV_STANDARD and GOV_STRICT, exporting MUST be automatic and continuous:
    - every message creation MUST update the export (append-only where possible),
    - every transcription link creation MUST update the export,
    - handoff/validation MUST be blocked until the export is in sync (no silent drift).
  - For GOV_LIGHT, export is optional.

  Export format (normative):
  - `docs/ROLE_MAILBOX/index.json` (one JSON object; thread inventory)
  - `docs/ROLE_MAILBOX/threads/<thread_id>.jsonl` (one JSON object per line; messages)
  - `docs/ROLE_MAILBOX/export_manifest.json` (export metadata + hashes)

  Export schemas (normative; role_mailbox_export_v1):

  // docs/ROLE_MAILBOX/index.json
  interface RoleMailboxIndexV1 {
    schema_version: 'role_mailbox_export_v1';
    generated_at: string; // RFC3339
    threads: Array<{
      thread_id: string;
      created_at: string; // RFC3339
      closed_at?: string | null; // RFC3339
      participants: string[]; // RoleId rendered as strings
      context: {
        spec_id?: string | null;
        work_packet_id?: string | null;
        task_board_id?: string | null;
        governance_mode: 'gov_strict' | 'gov_standard' | 'gov_light';
        project_id?: string | null;
      };
      subject_redacted: string; // MUST be Secret-Redactor output; bounded
      subject_sha256: string;   // sha256 of original subject bytes (UTF-8)
      message_count: number;
      thread_file: string; // "threads/<thread_id>.jsonl"
    }>;
  }

  // docs/ROLE_MAILBOX/threads/<thread_id>.jsonl (one JSON object per line)
  // This is a canonical JSON encoding of RoleMailboxMessage, but MUST NOT include any inline body.
  type RoleMailboxThreadLineV1 = {
    message_id: string;
    thread_id: string;
    created_at: string; // RFC3339
    from_role: string;
    to_roles: string[];
    message_type: string;
    body_ref: string;    // artifact handle string
    body_sha256: string; // sha256
    attachments: string[];
    relates_to_message_id?: string | null;
    transcription_links: Array<{
      target_kind: string;
      target_ref: string;
      target_sha256: string;
      note_redacted: string; // MUST be Secret-Redactor output; bounded
      note_sha256: string;   // sha256 of original note bytes (UTF-8)
    }>;
    idempotency_key: string;
  };

  // docs/ROLE_MAILBOX/export_manifest.json
  interface RoleMailboxExportManifestV1 {
    schema_version: 'role_mailbox_export_v1';
    export_root: 'docs/ROLE_MAILBOX/';
    generated_at: string; // RFC3339
    index_sha256: string;
    thread_files: Array<{
      path: string;   // "threads/<thread_id>.jsonl"
      sha256: string; // sha256 of file bytes
      message_count: number;
    }>;
  }

  Determinism requirements:
  - JSON serialization MUST be canonical: stable key ordering, `\\uXXXX` escaping for non-ASCII, `\\n` newlines, UTF-8.
  - Messages in each `<thread_id>.jsonl` MUST be ordered by `(created_at, message_id)` ascending.
  - Re-exporting the same mailbox state MUST yield byte-identical files (idempotent).

  Mechanical gate (HARD): RoleMailboxExportGate
  - The runtime MUST provide a mechanical gate that verifies the export is in sync and leak-safe.
  - The gate MUST fail if:
    - `export_manifest.json` hashes do not match current `index.json` / thread files,
    - any thread JSONL line is not valid JSON or violates the RoleMailboxThreadLineV1 field set,
    - any governance-critical message lacks required `transcription_links`,
    - any export file contains forbidden inline body fields (e.g., `body`, `body_text`, `raw_body`).

  Export telemetry:
  - Export MUST emit a Flight Recorder event `FR-EVT-GOV-MAILBOX-002` and a Spec Session Log entry `event_type=mailbox_exported`.

  Transcription telemetry:
  - Creating a transcription link MUST emit a Flight Recorder event `FR-EVT-GOV-MAILBOX-003` and a Spec Session Log entry `event_type=mailbox_transcribed`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 2.6.8.8 (Spec Session Log (Task Board + Work Packets)) (Normative)
- CONTEXT_START_LINE: 5950
- CONTEXT_END_LINE: 5974
- CONTEXT_TOKEN: SpecSessionLogEntry
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.6.8.8 Spec Session Log (Task Board + Work Packets) (Normative)

  Task Board items and Work Packets together form a Spec Session Log that runs in parallel to the Flight Recorder. Flight Recorder remains the authoritative system log; the Spec Session Log captures human-facing planning state and is used for context offload.

  pub struct SpecSessionLogEntry {
      pub entry_id: String,
      pub spec_id: String,
      pub task_board_id: String,
      pub work_packet_id: Option<String>,
      pub event_type: String,
      pub governance_mode: GovernanceMode,
      pub actor: String,
      pub timestamp: DateTime<Utc>,
      pub summary: String,
      pub linked_artifacts: Vec<ArtifactHandle>,
  }

  Rules:
  - Every Task Board or Work Packet change MUST emit a SpecSessionLogEntry stored in the workspace and indexed for RAG.
  - The Spec Session Log MUST NOT replace Flight Recorder; it is a parallel, human-facing ledger.
  - Spec Session Log entries MUST reference the same spec_id and work_packet_id used in SpecIntent and WorkPacketBinding.
  - SpecSessionLogEntry.entry_id MUST be unique within the workspace.
  - SpecSessionLogEntry.governance_mode MUST match the active mode at the time of the event; mode transitions require a dedicated entry.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 11.5.3 (FR-EVT-GOV-MAILBOX-001/002/003 (RoleMailbox Events)) (LAW)
- CONTEXT_START_LINE: 46421
- CONTEXT_END_LINE: 46472
- CONTEXT_TOKEN: gov_mailbox_message_created
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.3 FR-EVT-GOV-MAILBOX-001/002/003 (RoleMailbox Events)

  // FR-EVT-GOV-MAILBOX-001
  interface RoleMailboxMessageCreatedEvent extends FlightRecorderEventBase {
    type: 'gov_mailbox_message_created';

    // Correlation fields (required keys; may be null where noted)
    spec_id: string | null;
    work_packet_id: string | null;
    governance_mode: 'gov_strict' | 'gov_standard' | 'gov_light';

    thread_id: string;
    message_id: string;

    from_role: string;
    to_roles: string[];
    message_type: string;

    body_ref: string;    // artifact handle string (NOT inline body)
    body_sha256: string; // sha256 of body bytes

    idempotency_key: string;
  }

  // FR-EVT-GOV-MAILBOX-002
  interface RoleMailboxExportedEvent extends FlightRecorderEventBase {
    type: 'gov_mailbox_exported';

    export_root: 'docs/ROLE_MAILBOX/';
    export_manifest_sha256: string;
    thread_count: number;
    message_count: number;
  }

  // FR-EVT-GOV-MAILBOX-003
  interface RoleMailboxTranscribedEvent extends FlightRecorderEventBase {
    type: 'gov_mailbox_transcribed';

    thread_id: string;
    message_id: string;

    transcription_target_kind: string;
    target_ref: string;
    target_sha256: string;
  }

  Validation requirements (HARD):
  - Flight Recorder MUST reject RoleMailbox events that do not match the schema above (missing required keys, wrong types, invalid enum values).
  - Secret-leak prevention: RoleMailbox event payloads MUST NOT include inline message body fields (e.g., `body`, `body_text`, `raw_body`) or any other unbounded text content.
  ```
