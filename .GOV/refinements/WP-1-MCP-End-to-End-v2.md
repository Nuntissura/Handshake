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
- WP_ID: WP-1-MCP-End-to-End-v2
- CREATED_AT: 2026-02-16T08:09:00+01:00
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- SPEC_TARGET_SHA1: 7260b4ada693263799ff39dd909653863cf0e503
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja160220262157
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-MCP-End-to-End-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE in Master Spec Main Body. The spec explicitly defines MCP topology, Gate interception, reference-based binary payload strategy, durable progress mapping, logging sink contracts, and red-team hardening in 11.3.x.
- Implementation remediation gap (codebase): prior packet WP-1-MCP-End-to-End failed validation due to missing MCP modules and missing Flight Recorder MCP evidence. This v2 work is remediation/activation against current spec, not a spec rewrite.
- Dependency/concurrency note: this WP is downstream of WP-1-MCP-Skeleton-Gate-v2 and will overlap backend paths. Do not activate/delegate this WP while WP-1-MCP-Skeleton-Gate-v2 is in progress (file-lock rule [CX-CONC-001]); refinement is allowed.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- End-to-end requirement: at least one MCP-backed job must emit Flight Recorder rows that correlate UI action -> AI job -> MCP tools/call -> response, plus logging/progress notifications where applicable.
- DuckDB sink mapping for MCP traffic (fr_events):
  - tools/call: event_kind "mcp.tool_call"; payload includes server_id, tool_name, job_id, workflow_run_id, trace_id, capability metadata, and gate decision fields.
  - response/result: event_kind "mcp.tool_result" (or equivalent) with status, duration_ms, and typed error_code/error_message when failed.
  - notifications/progress: event_kind "mcp.progress" and persist durable progress mapping to SQLite ai_jobs fields.
  - logging/message: event_kind "mcp.logging" (or fields.event_kind) with full params JSON stored as payload.
- Gate decisions (allow/deny/timeout) must be logged for auditability and later validation.

### RED_TEAM_ADVISORY (security failure modes)
- Symlink/root bypass: canonicalize and enforce allowed-root bounds; no naive prefix checks; no-follow where applicable.
- Prompt injection via sampling/createMessage: treat server-provided content as untrusted; fence sampling contexts; never allow sampling to trigger tool side effects.

### PRIMITIVES (traits/structs/enums)
- MCP end-to-end job primitives:
  - McpJobContext: job_id, workflow_run_id, session_id/task_id (if applicable), server_id, tool_name, capability_profile_id, trace_id.
  - Progress mapping: mcp_progress_token, mcp_call_id, mcp_server_id persisted in SQLite ai_jobs.
  - Reference payloads: ref:// URIs for binary blobs; host-side mapping and cleanup tracking; never trust raw file:// from server.
- Gate middleware primitives (extends WP-1-MCP-Skeleton-Gate-v2):
  - Attach job/capability metadata to outbound tools/call.
  - Validate inbound server requests vs negotiated capabilities; default-deny.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec 11.3.x defines MCP topology + gate + reference protocol + durable progress + logging sink + red-team hardening with concrete sequences/schemas sufficient to implement and test an end-to-end MCP job.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Current Master Spec v02.126 already specifies end-to-end MCP job/log/progress behavior and security hardening. This WP is implementation + evidence remediation, not spec gap closure.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.1.1 The MCP Topology: Inverting the Client-Server Relationship
- CONTEXT_START_LINE: 56166
- CONTEXT_END_LINE: 56170
- CONTEXT_TOKEN: MCP Client
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.3.1.1 The MCP Topology: Inverting the Client-Server Relationship

  In a traditional web application, the "Client" is the frontend and the "Server" is the backend. MCP introduces a nuanced terminology where the "Host" (the AI application) acts as the MCP Client, and the tool-providing entity acts as the MCP Server.
  For Handshake, this topology is critical. The Rust Host (Tauri App) is the MCP Client. It initiates connections, manages the context window of the Large Language Model (LLM), and decides which tools are relevant for the current user intent. The Python Orchestrator is the MCP Server. It exposes capabilities\\u2014tools, resources, and prompts\\u2014that the Host can discover and invoke.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.2 Implementation Target 1: Rust Gate Interceptor (Middleware Design)
- CONTEXT_START_LINE: 56177
- CONTEXT_END_LINE: 56195
- CONTEXT_TOKEN: **Crucial Logic Flow:**
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.3.2 Implementation Target 1: The Rust 'Gate' Interceptor (Middleware Design)

  The Gate is the architectural chokepoint of the Handshake system. It is a middleware layer residing within the Rust Host that intercepts every message passing between the frontend UI (or the embedded LLM) and the Python Orchestrator. Its primary function is to transform the "trust" model of the system from implicit to explicit.

  **Crucial Logic Flow:**

  1. Outbound Interception (Client -> Server): When the Rust Host attempts to call a tool (e.g., tools/call), the Gate intercepts the request. It checks the Policy Engine to verify if the tool is allowed or if it requires a "Human-in-the-Loop" confirmation. If confirmation is needed, the request only proceeds upon user approval.
  2. Inbound Interception (Server -> Client): When the Python Orchestrator sends a request (e.g., sampling/createMessage or ping), the Gate acts as a firewall. It validates that the Server has the negotiated capability to make such requests. If the Server attempts to use a capability it did not declare (e.g., roots/list), the Gate rejects it with JSON-RPC -32601.
  3. Response Analysis: When the Python Server returns a result, the Gate inspects the payload. This is critical for Data Loss Prevention (DLP): redact or block sensitive outputs before they reach UI/context.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.3 Implementation Target 2: Reference-Based Binary Protocol
- CONTEXT_START_LINE: 56411
- CONTEXT_END_LINE: 56444
- CONTEXT_TOKEN: ref://
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.3.3 Implementation Target 2: Reference-Based Binary Protocol (Sequence Diagram)

  Instead of embedding binary blobs in JSON-RPC, the server writes bytes to shared storage and returns a Reference URI using ref:// (or blob://). The host recognizes this scheme and loads the bytes directly, avoiding Base64 overhead.

  Example (step excerpt):
  - The server returns a resource object like: { uri: "ref://shm/img_101.png", mimeType: "image/png" }.
  - The host loads the bytes from the shared location and then signals release via notifications/resource_released.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.4 Implementation Target 3: Durable Progress Mapping (SQLite Integration)
- CONTEXT_START_LINE: 56555
- CONTEXT_END_LINE: 56604
- CONTEXT_TOKEN: mcp_progress_token
- EXCERPT_ASCII_ESCAPED:
  ```text
  MCP progress notifications:
  {
    "method": "notifications/progress",
    "params": { "token": "123", "progress": 45, "message": "OCR 34/76 pages" }
  }

  Schema additions in SQLite (ai_jobs):
  - mcp_server_id
  - mcp_call_id
  - mcp_progress_token

  CREATE INDEX idx_ai_jobs_progress_token ON ai_jobs(mcp_progress_token);
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.6 Logging Sink (DuckDB) - MCP event_kind mapping + logging/message payload contract
- CONTEXT_START_LINE: 56962
- CONTEXT_END_LINE: 57028
- CONTEXT_TOKEN: "mcp.tool_call"
- EXCERPT_ASCII_ESCAPED:
  ```text
  CREATE TABLE fr_events (
      event_id        BIGINT PRIMARY KEY,
      ts_utc          TIMESTAMP NOT NULL,
      session_id      TEXT,
      task_id         TEXT,
      job_id          TEXT,
      workflow_run_id TEXT,
      event_kind      TEXT NOT NULL, -- "mcp.logging", "mcp.tool_call", "mcp.progress", ...
      source          TEXT NOT NULL, -- "docling-mcp", "asr-mcp", "teacher-mcp", "host"
      level           TEXT,          -- "DEBUG", "INFO", "WARN", "ERROR"
      message         TEXT,
      payload         JSON
  );

  CREATE INDEX idx_fr_events_job_id ON fr_events(job_id);
  CREATE INDEX idx_fr_events_kind ON fr_events(event_kind);
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md 11.3.7 Red Team Security Audit (Symlinks + Sampling)
- CONTEXT_START_LINE: 57142
- CONTEXT_END_LINE: 57193
- CONTEXT_TOKEN: Symlink Attacks (The "Roots" Bypass)
- EXCERPT_ASCII_ESCAPED:
  ```text
  Vulnerability: naive path.startswith(root) checks can be bypassed via symlinks inside allowed roots.
  Fix: canonicalize the requested path and enforce it stays under the canonical allowed_root; use no-follow policies when opening files (O_NOFOLLOW where available).

  Sampling injection: treat sampling/createMessage as untrusted; tag sampling contexts and forbid tool execution / side effects for sampling jobs.
  ```
