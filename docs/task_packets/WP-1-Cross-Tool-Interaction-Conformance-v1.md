# Task Packet: WP-1-Cross-Tool-Interaction-Conformance-v1

## METADATA
- TASK_ID: WP-1-Cross-Tool-Interaction-Conformance-v1
- WP_ID: WP-1-Cross-Tool-Interaction-Conformance-v1
- BASE_WP_ID: WP-1-Cross-Tool-Interaction-Conformance
- DATE: 2026-01-21T20:01:59.574Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: claude-opus-4-5-20251101
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja210120262044

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Cross-Tool-Interaction-Conformance-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement and validate Cross-Tool Interaction Map conformance by enforcing artifact-first IO, capability-gated side effects, and canonical Flight Recorder logging (`tool.call`/`tool.result` in DuckDB `fr_events`) across tool invocations.
- Why: Prevents "shadow pipelines" that bypass governance/auditability and makes Operator Console narratives deterministic across tools/surfaces.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/
  - src/backend/handshake_core/src/mex/
  - src/backend/handshake_core/src/terminal/
  - src/backend/handshake_core/src/llm/
  - src/backend/handshake_core/src/ace/
  - src/backend/handshake_core/src/api/
  - src/backend/handshake_core/src/bundles/
  - src/backend/handshake_core/src/diagnostics/
- OUT_OF_SCOPE:
  - UI changes under app/
  - New tool features beyond conformance proof
  - Spec changes / enrichment (implement against current anchors)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Cross-Tool-Interaction-Conformance-v1

# Targeted backend tests (add/update as needed):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Governance and workflow gates:
just validator-scan
just validator-spec-regression
just cargo-clean
just post-work WP-1-Cross-Tool-Interaction-Conformance-v1
```

### DONE_MEANS
- At least one representative MEX tool invocation emits `tool.call` + `tool.result` rows in DuckDB `fr_events` with required payload keys (tool identity, inputs/outputs refs, status, duration_ms, error_code, correlation fields where available).
- Any tool invocation that causes side effects records `capability_id` in Flight Recorder evidence (event payload and/or event base fields) per spec.
- Terminal command path continues to emit FR-EVT-001 with stdout/stderr references (no unbounded inline output).
- LLM inference path emits FR-EVT-006 with `trace_id` and `model_id`.
- A conformance test (or equivalent automated check) fails when a tool path bypasses the canonical invocation/logging chain.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-21T20:01:59.574Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 6.0.1 Cross-Tool Interaction Map; 11.3.6.4 Canonical Flight Recorder tables (DuckDB); 11.5 Flight Recorder Event Shapes & Retention (FR-EVT-001/002/006)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet artifacts:
  - docs/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md (stub; not executable)
- Changes in this v1 activation:
  - Updated SPEC_BASELINE to v02.113 and anchored to 6.0.1 + 11.3.6.4 + 11.5
  - Defined minimum viable conformance proof via DuckDB `fr_events` `tool.call`/`tool.result` rows + required payload keys

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - docs/refinements/WP-1-Cross-Tool-Interaction-Conformance-v1.md
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/mex/
  - src/backend/handshake_core/src/terminal/
  - src/backend/handshake_core/src/llm/
- SEARCH_TERMS:
  - "fr_events"
  - "tool.call"
  - "tool.result"
  - "mcp.tool_call"
  - "TerminalCommandEvent"
  - "EditorEditEvent"
  - "llm_inference"
  - "capability_id"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Cross-Tool-Interaction-Conformance-v1
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just post-work WP-1-Cross-Tool-Interaction-Conformance-v1
  ```
- RISK_MAP:
  - "shadow pipelines" -> "side effects without capability gates or FR evidence"
  - "log leakage" -> "secrets/PII written to logs instead of refs/hashes"
  - "correlation breakage" -> "Operator Consoles cannot explain causality across tools"

## SKELETON
- Proposed interfaces/types/contracts:

### 1. DuckDB `fr_events` table (Spec 11.3.6.4)

Implement the canonical `fr_events` table in `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
as an additional table (do NOT replace the existing typed `events` table).

```sql
CREATE TABLE fr_events (
    event_id        BIGINT PRIMARY KEY,
    ts_utc          TIMESTAMP NOT NULL,
    session_id      TEXT,
    task_id         TEXT,
    job_id          TEXT,
    workflow_run_id TEXT,
    event_kind      TEXT NOT NULL, -- "tool.call", "tool.result", "mcp.logging", ...
    source          TEXT NOT NULL, -- "mex-runtime", "docling-mcp", "host", ...
    level           TEXT,          -- "DEBUG", "INFO", "WARN", "ERROR"
    message         TEXT,
    payload         JSON
);

CREATE INDEX idx_fr_events_job_id ON fr_events(job_id);
CREATE INDEX idx_fr_events_kind ON fr_events(event_kind);
```

Implementation detail (deterministic IDs):
- Maintain a monotonic BIGINT `event_id` counter initialized from `MAX(fr_events.event_id)` on startup.
- Increment per insert (thread-safe).
- Apply retention to `fr_events` alongside `events`.

### 2. MEX tool invocation logging (`tool.call` / `tool.result`)

Implement in `src/backend/handshake_core/src/mex/runtime.rs` using `FlightRecorder::duckdb_connection()`.

Trigger points:
- Emit `tool.call` immediately before invoking the engine adapter.
- Emit `tool.result` immediately after completion (success or error).

Minimum payload keys for `tool.*` kinds (per refinement + DONE_MEANS):
```json
{
  "tool_name": "mex:<engine_id>",
  "tool_version": null,
  "inputs": ["artifact:<uuid>:<path>"],
  "outputs": ["artifact:<uuid>:<path>"],
  "status": "success|error|timeout|skipped",
  "duration_ms": 12,
  "error_code": null,
  "job_id": "<op_id>",
  "workflow_run_id": null,
  "trace_id": "<op_id>",
  "capability_id": "fs.write"
}
```

Notes:
- For `tool.call`, set `outputs: []`, `duration_ms: null`, `error_code: null`, `status: "success"` (call emitted).
- For `tool.result`, set `status` to the actual outcome and include `duration_ms` and `error_code` when applicable.

### 3. Terminal FR-EVT-001 stdout/stderr references (no inline unbounded output)

Adjust `src/backend/handshake_core/src/flight_recorder/mod.rs` `TerminalCommandEvent` payload to include:
- `stdout_ref?: string | null`
- `stderr_ref?: string | null`

Store redacted stdout/stderr in DuckDB (new table owned by `DuckDbFlightRecorder`), and set refs like:
- `stdout_ref = "duckdb:terminal_output:<event_id>:stdout"`
- `stderr_ref = "duckdb:terminal_output:<event_id>:stderr"`

### 4. Conformance checks (automated)

Add tests under `src/backend/handshake_core/src/mex/` that:
- Assert a representative `MexRuntime::execute()` writes both `tool.call` and `tool.result` rows into DuckDB `fr_events`.
- Assert "no shadow pipelines" by failing if any `.invoke(` call sites exist outside `src/backend/handshake_core/src/mex/runtime.rs`.

### 5. Files to Modify (in-scope)

1. `src/backend/handshake_core/src/flight_recorder/duckdb.rs` - add `fr_events` (and terminal output) tables + retention
2. `src/backend/handshake_core/src/mex/runtime.rs` - emit `tool.call` + `tool.result` into `fr_events`
3. `src/backend/handshake_core/src/mex/conformance.rs` - add conformance case(s)/tests for `fr_events` tool logging + no-shadow-pipelines audit
4. `src/backend/handshake_core/src/flight_recorder/mod.rs` and `src/backend/handshake_core/src/terminal/mod.rs` - add stdout/stderr refs for FR-EVT-001

- Open questions:
  - None (spec+packet DONE_MEANS require `tool.call` and `tool.result` rows in DuckDB `fr_events`).
- Notes:
  - Existing typed FR events remain (TerminalCommand, LlmInference, Diagnostic, etc.).
  - This WP adds canonical `fr_events` rows for cross-tool invocation evidence.

SKELETON APPROVED

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (SKELETON proposed 2026-01-21)
- What changed in this update:
  - Completed BOOTSTRAP phase: read 12 source files, ran 5 search queries
  - BOOTSTRAP_RISK_MAP identified: No unified ToolInvocation primitive exists, FR events constructed ad-hoc
  - Proposed SKELETON: ToolInvocation struct, FlightRecorderEventType::ToolInvocation, emit_tool_invocation() helper
  - Identified 4 integration points: terminal, mex/supply_chain, llm/ollama, mex/runtime
- Next step / handoff hint: Awaiting operator approval of SKELETON before implementation

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

### BOOTSTRAP_FILES_READ (2026-01-21)

| Path | Key Patterns Observed |
|------|----------------------|
| `flight_recorder/mod.rs` | `FlightRecorderEvent`, `FlightRecorderEventType` enum, `FlightRecorderActor`, payload validators |
| `flight_recorder/duckdb.rs` | `DuckDbFlightRecorder` impl, events table schema |
| `terminal/mod.rs` | `TerminalService::run_command()`, `emit_capability_audit()`, `build_flight_recorder_event()` |
| `mex/mod.rs` | Re-exports: conformance, envelope, gates, registry, runtime, supply_chain |
| `mex/envelope.rs` | `PlannedOperation`, `EngineResult`, `ProvenanceRecord` |
| `mex/supply_chain.rs` | `ToolRunner` trait, `emit_supply_chain_event()` uses `FlightRecorderEvent::new()` directly |
| `llm/mod.rs` | `LlmClient` trait, `CompletionRequest` with trace_id, `ModelTier` enum |
| `ace/mod.rs` | `QueryPlan`, `RetrievalTrace`, `AceError` codes |
| `api/mod.rs` | Router composition |
| `bundles/mod.rs` | `BundleExporter`, `ValBundleValidator` |
| `diagnostics/mod.rs` | `Diagnostic` struct, `DiagnosticsStore` trait |

### BOOTSTRAP_SEARCH_RESULTS (2026-01-21)

| Search Term | Files Found | Key Observations |
|-------------|-------------|------------------|
| `FlightRecorderEvent` | 14 files | Central event type; ad-hoc construction in each module |
| `ToolInvocation` | 1 file | Only as error variant `ToolInvocationFailed` -- NO formal primitive |
| `emit_flight_recorder` | 0 files | No standardized emit helper exists |
| `build_flight_recorder_event` | 1 file | Only terminal has dedicated builder |
| `tool_id` | 3 files | Used in ContextPackBuilder.tool_id |

### BOOTSTRAP_RISK_MAP (2026-01-21)

| Risk Area | Assessment | Mitigation |
|-----------|------------|------------|
| No unified ToolInvocation primitive | HIGH | Define `ToolInvocation` struct in `flight_recorder/` |
| Inconsistent FR event emission | MEDIUM | Create unified `emit_tool_invocation()` helper |
| Cross-tool correlation missing | HIGH | Add `parent_invocation_id` for causality linking |
| FlightRecorderEventType lacks ToolInvocation | HIGH | Add `ToolInvocation` variant to enum |

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
