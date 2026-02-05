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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Cross-Tool-Interaction-Conformance-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement and validate Cross-Tool Interaction Map conformance by enforcing artifact-first IO, capability-gated side effects, and canonical Flight Recorder logging (`tool.call`/`tool.result` in DuckDB `fr_events`) across tool invocations.
- Why: Prevents "shadow pipelines" that bypass governance/auditability and makes Operator Console narratives deterministic across tools/surfaces.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/terminal/mod.rs
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 6.0.1 Cross-Tool Interaction Map; 11.3.6.4 Canonical Flight Recorder tables (DuckDB); 11.5 Flight Recorder Event Shapes & Retention (FR-EVT-001/002/006)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet artifacts:
  - .GOV/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md (stub; not executable)
- Changes in this v1 activation:
  - Updated SPEC_BASELINE to v02.113 and anchored to 6.0.1 + 11.3.6.4 + 11.5
  - Defined minimum viable conformance proof via DuckDB `fr_events` `tool.call`/`tool.result` rows + required payload keys

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - .GOV/refinements/WP-1-Cross-Tool-Interaction-Conformance-v1.md
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
- Added canonical DuckDB `fr_events` table (11.3.6.4) and `terminal_output` storage; retention purges all relevant tables.
- Emitted `tool.call` and `tool.result` rows around `MexRuntime::execute()` adapter invocation (6.0.1) with required payload keys and correlation fields where available.
- Updated FR-EVT-001 terminal command path to store redacted output in DuckDB and record `stdout_ref`/`stderr_ref` (no inline unbounded output).
- Added automated conformance checks: (1) `fr_events` `tool.*` emission and (2) reject shadow `.invoke(&op)` call sites outside `mex/runtime.rs`.

## HYGIENE
- git branch safety/WP-1-Cross-Tool-Interaction-Conformance-v1 da6349f8 : PASS
- git reset --soft HEAD~1 : PASS
- just pre-work WP-1-Cross-Tool-Interaction-Conformance-v1 : PASSED
- just validator-spec-regression : PASS
- just validator-scan : PASS
- just cargo-clean : PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml : PASS
- just post-work WP-1-Cross-Tool-Interaction-Conformance-v1 : PASSED

## VALIDATION
- (Mechanical manifest for audit. This section records the "What" (hashes/lines) for the Validator's "How/Why" audit. It is NOT a claim of official Validation.)

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 24
- **End**: 559
- **Line Delta**: 73
- **Pre-SHA1**: `41b29b4b24497715dd003bbc9c6698c7024a2e3a`
- **Post-SHA1**: `1684be6b596e4a7e4ca68a775ead4b04a7c78cf6`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 408
- **End**: 1191
- **Line Delta**: 8
- **Pre-SHA1**: `57543c4e1cd9c7b3132f4acc130b68c10f197e15`
- **Post-SHA1**: `66009937a65f1e3031084d1ab0e17923ac380a24`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mex/runtime.rs`
- **Start**: 3
- **End**: 308
- **Line Delta**: 173
- **Pre-SHA1**: `3e3b7378719cd57a466108254c270557ca0ffc01`
- **Post-SHA1**: `2fc722628bade5e57aff8b34ca8cdd1ef0304338`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mex/conformance.rs`
- **Start**: 276
- **End**: 451
- **Line Delta**: 176
- **Pre-SHA1**: `4c4eeb5931995f9f84dbcc7ef4ce1c7ca3e7654d`
- **Post-SHA1**: `3226b9ba401ecb81b52437b670459ac9967a3d6b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/terminal/mod.rs`
- **Start**: 19
- **End**: 589
- **Line Delta**: 42
- **Pre-SHA1**: `0392bee8d3722eaae81cdf58d333df827cf8a02f`
- **Post-SHA1**: `417842f5754b98f6deaa6166cf364493edc130b4`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (implementation complete; awaiting validator review)
- Touched files:
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/terminal/mod.rs
- Next step / handoff hint: Run `just post-work WP-1-Cross-Tool-Interaction-Conformance-v1`, then request Validator review of the manifest + evidence against anchors 6.0.1 / 11.3.6.4 / 11.5.

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

### EVIDENCE_RUNS (2026-01-21)

just pre-work WP-1-Cross-Tool-Interaction-Conformance-v1:
- Pre-work validation PASSED

just validator-spec-regression:
- validator-spec-regression: PASS - Handshake_Master_Spec_v02.113.md present with required anchors.

just validator-scan:
- validator-scan: PASS - no forbidden patterns detected in backend sources.

just cargo-clean:
- cargo clean -p handshake_core (removed 1371 files)

cargo test --manifest-path src/backend/handshake_core/Cargo.toml:
- handshake_core unit tests: 151 passed
- api health tests: 2 passed
- mex_tests: 9 passed; 4 ignored
- oss_register_enforcement_tests: 4 passed
- role_mailbox_tests: 3 passed
- storage_conformance: 2 passed
- terminal_guards_tests: 13 passed
- terminal_session_tests: 5 passed
- tokenization_service_tests: 4 passed
- tokenization_tests: 5 passed
- doc-tests: 0 tests

just post-work WP-1-Cross-Tool-Interaction-Conformance-v1:
- Post-work validation PASSED
- ROLE_MAILBOX_EXPORT_GATE PASS

### EVIDENCE_MAPPING (DONE_MEANS + SPEC_ANCHORS)

DONE_MEANS
- tool.call/tool.result in DuckDB fr_events: src/backend/handshake_core/src/mex/runtime.rs:272 and src/backend/handshake_core/src/mex/conformance.rs:342
- capability_id present for side-effecting tool invocation: src/backend/handshake_core/src/mex/runtime.rs:166 and src/backend/handshake_core/src/mex/runtime.rs:216
- FR-EVT-001 terminal_command stdout_ref/stderr_ref (no inline unbounded output): src/backend/handshake_core/src/terminal/mod.rs:486 and src/backend/handshake_core/src/flight_recorder/mod.rs:1190
- FR-EVT-006 llm_inference emits trace_id + model_id: src/backend/handshake_core/src/llm/ollama.rs:128 and src/backend/handshake_core/src/flight_recorder/mod.rs:964
- conformance check rejects shadow .invoke(&op) call sites: src/backend/handshake_core/src/mex/conformance.rs:308

SPEC_ANCHORS
- 11.3.6.4 fr_events DuckDB table: src/backend/handshake_core/src/flight_recorder/duckdb.rs:271
- 6.0.1 tool.call/tool.result around MexRuntime::execute(): src/backend/handshake_core/src/mex/runtime.rs:272 and src/backend/handshake_core/src/mex/runtime.rs:307
- 11.5 FR-EVT-001 output references: src/backend/handshake_core/src/terminal/mod.rs:506 and src/backend/handshake_core/src/flight_recorder/mod.rs:1190

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATOR_REPORT (2026-01-21)

VALIDATION REPORT - WP-1-Cross-Tool-Interaction-Conformance-v1
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Cross-Tool-Interaction-Conformance-v1.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.113.md 6.0.1, 11.3.6.4, 11.5
- Commit under review: 5c3b3390

Files Checked:
- src/backend/handshake_core/src/flight_recorder/duckdb.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/src/mex/conformance.rs
- src/backend/handshake_core/src/mex/runtime.rs
- src/backend/handshake_core/src/terminal/mod.rs
- src/backend/handshake_core/src/llm/ollama.rs
- .GOV/task_packets/WP-1-Cross-Tool-Interaction-Conformance-v1.md
- .GOV/refinements/WP-1-Cross-Tool-Interaction-Conformance-v1.md

Findings:
- Spec 11.3.6.4 (DuckDB canonical FR tables): DuckDB schema bootstrap creates `fr_events` (src/backend/handshake_core/src/flight_recorder/duckdb.rs:271).
- Spec 6.0.1 (Cross-tool interaction conformance): MEX runtime records `tool.call` + `tool.result` into DuckDB `fr_events` (src/backend/handshake_core/src/mex/runtime.rs:172, src/backend/handshake_core/src/mex/runtime.rs:225).
- Required payload keys enforced by test: `tool_name`, `tool_version`, `inputs`, `outputs`, `status`, `duration_ms`, `error_code`, `job_id`, `workflow_run_id`, `trace_id`, `capability_id` (src/backend/handshake_core/src/mex/conformance.rs:341, src/backend/handshake_core/src/mex/conformance.rs:425).
- Capability evidence: `capability_id` is always present in tool.* payloads (src/backend/handshake_core/src/mex/runtime.rs:166, src/backend/handshake_core/src/mex/runtime.rs:216).
- Spec 11.5 (FR-EVT-001 terminal output refs + bounded logging): terminal stdout/stderr stored as redacted DuckDB refs (`stdout_ref`/`stderr_ref`) with backing `terminal_output` table and payload validation (src/backend/handshake_core/src/terminal/mod.rs:486, src/backend/handshake_core/src/flight_recorder/duckdb.rs:292, src/backend/handshake_core/src/flight_recorder/mod.rs:408).
- Spec 11.5 retention: retention purge covers `events`, `fr_events`, and `terminal_output` (src/backend/handshake_core/src/flight_recorder/duckdb.rs:529).
- Spec 11.5 (FR-EVT-006 llm_inference): llm_inference emission includes `trace_id` and `model_id` (src/backend/handshake_core/src/llm/ollama.rs:128).
- Conformance guard against "shadow pipelines": test fails on `.invoke(&op)` call sites outside `mex/runtime.rs` (src/backend/handshake_core/src/mex/conformance.rs:308).

Forbidden Patterns:
- just validator-scan: PASS (evidence recorded in this packet under EVIDENCE_RUNS).

Tests:
- just pre-work WP-1-Cross-Tool-Interaction-Conformance-v1: PASS (evidence recorded under EVIDENCE_RUNS).
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS (evidence recorded under EVIDENCE_RUNS).
- just post-work WP-1-Cross-Tool-Interaction-Conformance-v1: PASS (evidence recorded under EVIDENCE_RUNS; includes ROLE_MAILBOX_EXPORT_GATE PASS).

### REASON FOR PASS
- DONE_MEANS satisfied with file:line evidence for tool.call/tool.result fr_events logging, capability_id correlation, Terminal FR-EVT-001 output refs, LLM FR-EVT-006 trace_id/model_id, and a conformance test that fails on shadow invocation.
- Deterministic manifests and post-work gate passed (COR-701 discipline recorded in this packet).

Risks & Suggested Actions:
- Workflow note (non-blocking): `just post-work` currently depends on staged/working-tree changes; consider allowing it to validate committed diffs (HEAD) to avoid false FAIL on clean trees.

Task Packet Update (APPEND-ONLY):
- This report appended under ## VALIDATION_REPORTS. Metadata Status line not overwritten.

