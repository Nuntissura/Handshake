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
- WP_ID: WP-1-Terminal-LAW-v3
- CREATED_AT: 2026-01-01T22:14:20.1511391+01:00
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010120262218

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-001 (TerminalCommandEvent): emit on allowed terminal executions (type=terminal_command).
- Capability check audit events: record allow/deny outcomes per HSK-4001 audit requirement (capability_id, actor_id, job_id when present, decision_outcome).

### RED_TEAM_ADVISORY (security failure modes)
- RCE/exfiltration: AI attaching to HUMAN_DEV terminal sessions without capability+consent.
- Audit gap: allow/deny decisions not recorded (no attribution; cannot investigate incidents).
- Unbounded output/log leakage: secrets may be exposed if output handling is not bounded and redacted.

### PRIMITIVES (traits/structs/enums)
- TerminalRequest / TerminalResult (request+result shape) governed by TERM-API-001..005.
- Terminal session typing / isolation: session type discriminator aligned to TERM-UX-001..003.
- Flight Recorder FR-EVT-001 payload shape (TerminalCommandEvent).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.100 contains explicit TERM-API-001..005, TERM-UX-001..003, HSK-4001 audit requirement, and FR-EVT-001 schema; no enrichment needed.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Existing normative blocks define required request fields, isolation rule, audit logging requirement, and FR-EVT-001 event schema in sufficient detail to implement without speculation.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 10.1.1.3 run_command API contract (TERM-API-001..005)
- CONTEXT_START_LINE: 23360
- CONTEXT_END_LINE: 23420
- CONTEXT_TOKEN: TERM-API-001
- EXCERPT_ASCII_ESCAPED:
  ```text
    - The decision MUST be logged in Flight Recorder for that job.

- **TERM-CAP-004 (Expiry)**
  - Long-lived approvals MUST have:
    - Either a time-to-live, or
    - An explicit \u00E2\u20AC\u0153unlimited until revoked\u00E2\u20AC\u009D flag visible in the UI.

#### 10.1.1.3 `run_command` API contract
- **TERM-API-001 (Signature)**
  - The internal `run_command` tool MUST expose at least:
    - `command: string` (full command line or argv-list),
    - `cwd?: string` (workspace-relative path),
    - `mode: 'non_interactive' | 'interactive_session'`,
    - `timeout_ms?: number`,
    - `env_overrides?: Record<string, string | null>`,
    - `max_output_bytes?: number`,
    - `capture_stdout?: boolean`,
    - `capture_stderr?: boolean`,
    - `stdin_chunks?: string[]` (for scripted interactive runs),
    - `idempotency_key?: string` (optional for at-least-once semantics).

- **TERM-API-002 (Timeout & cancellation)**
  - If `timeout_ms` is omitted, a reasonable default MUST be used (recommended: 180_000 ms).
  - When timeout is reached:
    - The backend MUST send a termination signal to the process,
    - Then a kill signal if it doesn\u00E2\u20AC\u2122t exit within a grace period (recommended: 10_000 ms).
    - The result MUST include `timed_out: true`.
  - The orchestrator MUST be able to cancel an in-flight command:
    - Cancellation MUST propagate to the PTY,
    - Result MUST include `cancelled: true`.

- **TERM-API-003 (Output handling)**
  - For `non_interactive` mode:
    - Output MAY be streamed to the caller, but MUST be **bounded** by `max_output_bytes` (recommended default: 1\u00E2\u20AC\u201C2 MB):
      - If truncated, the result MUST indicate truncation and how many bytes were emitted.
  - For `interactive_session`:
    - Output MUST be streamed, but the logging policy (below) still applies.
  - The API MUST separate:
    - Raw unredacted stream (for UI),
    - Redacted/logged stream (for Flight Recorder).

- **TERM-API-004 (Environment rules)**
  - Default environment MUST be inherited from the app\u00E2\u20AC\u2122s process (subject to secrets policy).
  - `env_overrides`:
    - `value` overrides or injects variables,
    - `null` explicitly unsets a variable.
  - Certain variables (e.g., secrets) MAY be suppressed from being passed into AI-run shells if policy dictates.

- **TERM-API-005 (Deterministic logging)**
  - Every `run_command` call MUST emit a log event containing:
    - `job_id` (if any),
    - `model_id` (if any),
    - `session_id` (if bound to a TerminalSession),
    - `command`, `cwd`, `exit_code`, `duration_ms`,
    - `timed_out`, `cancelled`, `truncated_bytes`.
  - This event MUST be stable enough for replay and auditing.

### 10.1.2 Logging, Matchers, UX, Platform (LAW)

#### 10.1.2.1 Output logging & redaction
- **TERM-LOG-001 (Logging levels)**
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 10.1.2.3 Human vs AI terminal invariants (TERM-UX-001..003)
- CONTEXT_START_LINE: 23465
- CONTEXT_END_LINE: 23510
- CONTEXT_TOKEN: TERM-UX-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Parsed output MUST be normalized into the common Diagnostics schema (see 11.4),
  - Then fed into:
    - Monaco,
    - Problems panel,
    - Flight Recorder (as `DiagnosticEvent`).

#### 10.1.2.3 Human vs AI terminal invariants
- **TERM-UX-001 (Session types)**
  - Every terminal session MUST be labeled as:
    - `HUMAN_DEV` (created by user),
    - `AI_JOB` (created by an AI job),
    - `PLUGIN_TOOL` (optional).
  - This type MUST be visible in the UI (badge, color, or icon).

- **TERM-UX-002 (AI attachment rule)**
  - AI models MUST NOT:
    - Attach to,
    - Type into,
    - Or read from `HUMAN_DEV` sessions by default.
  - Any override MUST be:
    - Explicitly requested,
    - Capability-guarded,
    - Confirmed by the user.

- **TERM-UX-003 (Trace linkage)**
  - Every `AI_JOB` session MUST be linked to:
    - A `job_id`,
    - One or more WSIDs,
    - Capability set,
  - And MUST be visible from Flight Recorder with a \u00E2\u20AC\u0153jump to terminal\u00E2\u20AC\u009D link.

#### 10.1.2.4 Platform matrix
- **TERM-PLAT-001 (Baseline support)**
  - v1 MUST support policy-scoped terminals on:
    - Windows (ConPTY-based shells: PowerShell, CMD, WSL),
    - Linux (PTY-based shells: Bash, Zsh),
    - macOS (PTY-based shells: Bash, Zsh).

- **TERM-PLAT-002 (Guaranteed baseline features)**
  - Across all three platforms v1 MUST support:
    - Multiple sessions,
    - Tabs,
    - Workspace-root cwd,
    - Command logging (commands only),
    - File path linkification,
    - Non-interactive `run_command` with timeouts.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.1 Capabilities & Consent Model (HSK-4001 + audit requirement)
- CONTEXT_START_LINE: 29220
- CONTEXT_END_LINE: 29240
- CONTEXT_TOKEN: HSK-4001: UnknownCapability
- EXCERPT_ASCII_ESCAPED:
  ```text

- Scope/time-to-live defaults, approval caching, revocation UX, escalation paths, and capability axes for surfaces (terminal, editor, mail, calendar).
- Mapping to plugins and product surfaces:
  - Plugin manifest permissions MUST map to capability_profile_id entries; no ad-hoc plugin permissions.
  - Mail/Calendar surfaces MUST use the same capability/consent profiles (send_email, read_mail, export_calendar) used by plugin APIs and AI Job profiles.
  - Workflow/AI Job model MUST resolve effective capabilities from: plugin manifest (if tool), job profile, and surface-specific policy; the most restrictive wins.
- **Capability Registry & SSoT Enforcement ([HSK-4001]):**
  - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
  - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
  - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
  - **Profile Schema:** `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') MUST be defined solely as whitelists of IDs from the `CapabilityRegistry`.
- Redaction/safety propagation:
  - Content classification + redaction flags flow from data layer to plugin/tool calls and AI jobs; cloud routing MUST honor projection/redaction defaults per surface (mail/calendar/doc).
- Based on TERM-CAP: axes (model, workspace, command class/action type, time scope), approval types (per-job, per-model-per-workspace), visible \u00E2\u20AC\u0153Capabilities\u00E2\u20AC\u009D UI, revocation without restart, escalation flow with job/model/workspace/command context, decisions logged to Flight Recorder.
- Recommended defaults:
  - Per-job approvals as the default prompt; per-model-per-workspace approvals with 24h TTL by default.
  - \u00E2\u20AC\u0153Until revoked\u00E2\u20AC\u009D only as an explicit opt-in with warning.
  - Define approval classes for non-terminal surfaces (e.g., edit scopes for Monaco; send/read/sync scopes for mail/calendar).
- Canonical calendar capability identifiers (field projections/redaction rules: see \u00C2\u00A710.4):
  - `CALENDAR_READ_BASIC`
  - `CALENDAR_READ_DETAILS`
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 Flight Recorder Event Shapes - FR-EVT-001 (TerminalCommandEvent)
- CONTEXT_START_LINE: 30825
- CONTEXT_END_LINE: 30870
- CONTEXT_TOKEN: interface TerminalCommandEvent
- EXCERPT_ASCII_ESCAPED:
  ```text

  // Governance
  capability_id?: string;
  policy_decision_id?: string;
}
```

- **FR-EVT-001 (TerminalCommandEvent)**

```ts
interface TerminalCommandEvent extends FlightRecorderEventBase {
  type: 'terminal_command';

  session_id: string;               // terminal session identifier
  cwd?: string | null;
  command: string;                  // command line as executed
  args?: string[];                  // optional parsed argv

  // Result
  exit_code?: number | null;
  duration_ms?: number | null;

  // Output references (never inline unbounded output)
  stdout_ref?: string | null;        // pointer/ref to stored output chunk(s)
  stderr_ref?: string | null;

  // Optional: environment and process metadata (redacted per policy)
  env_ref?: string | null;           // ref to redacted env snapshot
}
```

- **FR-EVT-002 (EditorEditEvent)**

```ts
interface EditorEditOp {
  range: { startLine: number; startColumn: number; endLine: number; endColumn: number };
  insert_text?: string;              // MAY be omitted or hashed/redacted by policy
  delete_text?: string;              // MAY be omitted or hashed/redacted by policy
}

interface EditorEditEvent extends FlightRecorderEventBase {
  type: 'editor_edit';

  editor_surface: 'monaco' | 'canvas' | 'sheet';
  document_uri?: string | null;      // file:// or internal uri
  path?: string | null
  ```

