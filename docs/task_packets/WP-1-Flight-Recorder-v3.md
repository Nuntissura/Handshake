# Task Packet: WP-1-Flight-Recorder-v3

## METADATA
- TASK_ID: WP-1-Flight-Recorder-v3
- WP_ID: WP-1-Flight-Recorder-v3
- DATE: 2026-01-01T13:50:12.516Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010120261446

## USER_CONTEXT (Non-Technical Explainer)
The Flight Recorder is the app's audit log "black box": it records key actions (terminal commands, diagnostics, recovery events, debug bundle exports) in a durable store so operators can later answer "what happened" with evidence. Right now, some event typing is inconsistent with SPEC_CURRENT, which causes misclassification and blocks revalidation of downstream work that reads these events.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Flight-Recorder-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Align Flight Recorder event taxonomy and ingestion envelope to SPEC_CURRENT v02.100 so stored events use canonical `type` strings and do not conflict with FR-EVT numbering (e.g., FR-EVT-002 is editor_edit, not llm_inference).
- Why: Current revalidation failures show spec-to-code drift in Flight Recorder taxonomy (event type strings and FR-EVT mapping), which blocks downstream packet revalidation (LLM Core, Operator Consoles) and Phase 1 closure.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
- OUT_OF_SCOPE:
  - Any UI changes in app/ or src/frontend/
  - Any Master Spec / Codex edits (spec enrichment is out of scope for this packet)
  - Operator Consoles UI work (separate WP; this packet only fixes the underlying event taxonomy/ingestion)
- Dependencies:
  - This packet is a prerequisite for revalidating downstream work that consumes Flight Recorder event types/shapes, including WP-1-Operator-Consoles and LLM observability, and should be completed before attempting those revalidations.

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Flight-Recorder-v3

# Spec integrity:
just validator-spec-regression

# Targeted backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder
cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal
cargo test --manifest-path src/backend/handshake_core/Cargo.toml llm

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Hygiene:
just validator-scan
just cargo-clean
just post-work WP-1-Flight-Recorder-v3
```

### DONE_MEANS
- Spec alignment (A11.5): Flight Recorder stored event `type` values align with SPEC_CURRENT v02.100 canonical types for:
  - FR-EVT-001: `terminal_command`
  - FR-EVT-002: `editor_edit`
  - FR-EVT-003: `diagnostic`
  - FR-EVT-005: `debug_bundle_export`
  - FR-EVT-WF-RECOVERY: `workflow_recovery`
- Terminal logging correctness: terminal `run_command` events are persisted with `type = terminal_command` (not `capability_action`) and include at least the TERM-API-005 fields: `command`, `cwd`, `exit_code`, `duration_ms`, `timed_out`, `cancelled`, `truncated_bytes` (allowed as additional fields beyond the FR-EVT-001 minimum schema).
- LLM observability correctness: each LLM completion emits a Flight Recorder event containing `trace_id`, `model_id`, and TokenUsage (prompt/completion/total) per 4.2.3.2 using `type = llm_inference`; it MUST NOT claim FR-EVT-002 (reserved for editor_edit).
- Ingestion contract enforcement: `record_event` validates event shape and returns `RecorderError::InvalidEvent` with the stable code prefix `HSK-400-INVALID-EVENT` when required fields are missing/invalid.
- Retention enforcement: `enforce_retention` purges old events and returns a deterministic count (implementation already exists; must remain spec-compliant).
- Tests and gates: all commands in TEST_PLAN pass and `just post-work WP-1-Flight-Recorder-v3` returns PASS (COR-701 manifest filled; ASCII-only packet).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T13:50:12.516Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5.1 (FlightRecorder trait), 11.5 (FR-EVT schemas), 10.1.1.3 TERM-API-005 (Deterministic logging), 4.2.3.2 (Observability requirement)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.100.md (11.5, 10.1.1.3, 4.2.3.2)
  - docs/ORCHESTRATOR_PROTOCOL.md
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
- SEARCH_TERMS:
  - "FlightRecorderEventType"
  - "event_type"
  - "terminal_command"
  - "capability_action"
  - "FrEvt002LlmInference"
  - "FrEvt006WorkflowRecovery"
  - "TerminalEventEnvelope"
  - "record_event"
  - "insert_event"
  - "query_events"
  - "validate("
  - "HSK-400-INVALID-EVENT"
- RUN_COMMANDS:
  ```bash
  rg -n "FlightRecorderEventType|event_type|capability_action|terminal_command" src/backend/handshake_core/src
  rg -n "FrEvt002LlmInference|FrEvt006WorkflowRecovery|TerminalCommandEvent" src/backend/handshake_core/src/flight_recorder src/backend/handshake_core/src/terminal src/backend/handshake_core/src/llm
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder
  ```
- RISK_MAP:
  - "Stored events use non-canonical type strings" -> "Validator/spec compliance failure; Operator Consoles misclassification"
  - "Changing event_type mapping breaks existing queries" -> "Operator Consoles and debug tooling regressions"
  - "Schema validation too strict" -> "Runtime errors (InvalidEvent) and missing logs"
  - "Schema validation too lax" -> "Spec non-compliance; bypassable audit trail"

## SKELETON
- Proposed interfaces/types/contracts:
- Flight Recorder stored `type` string matches SPEC_CURRENT event `type` for canonical events (terminal_command/editor_edit/diagnostic/debug_bundle_export/workflow_recovery).
- Terminal module emits Flight Recorder events using the canonical `type` (not a generic category like capability_action).
- LLM inference event naming is stable and does not reuse FR-EVT-002; shape must include TokenUsage + trace_id/model_id per 4.2.3.2.
- Open questions:
- None (explicit user decisions recorded; do not deviate).
- Notes:
- USER_DECISIONS (2026-01-01):
  - LLM observability event: Treat as allowed non-FR-EVT-numbered event; use `type = llm_inference` and validate via base envelope requirements + local shape checks; FR-EVT list in 11.5 is canonical where defined, but not an exhaustive allowlist.
  - Terminal logging fields: Treat FR-EVT-001 (terminal_command) as a minimum schema; allow additional fields so TERM-API-005 required fields (`timed_out`, `cancelled`, `truncated_bytes`) are present on the same terminal_command payload.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

## ORCHESTRATOR_HANDOFF_CONFIRMATION
- Packet created: docs/task_packets/WP-1-Flight-Recorder-v3.md
- Refinement approved and signed: docs/refinements/WP-1-Flight-Recorder-v3.md (USER_SIGNATURE: ilja010120261446)
- Signature recorded (one-time use): docs/SIGNATURE_AUDIT.md contains ilja010120261446; docs/ORCHESTRATOR_GATES.json contains refinement+signature records for WP-1-Flight-Recorder-v3
- Task board updated: docs/TASK_BOARD.md includes WP-1-Flight-Recorder-v3 as Ready for Dev; WP-1-Flight-Recorder-v2 marked superseded by v3
- Pre-work gate: just pre-work WP-1-Flight-Recorder-v3 (PASS)
