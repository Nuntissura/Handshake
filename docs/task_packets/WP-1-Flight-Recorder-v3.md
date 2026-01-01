# Task Packet: WP-1-Flight-Recorder-v3

## METADATA
- TASK_ID: WP-1-Flight-Recorder-v3
- WP_ID: WP-1-Flight-Recorder-v3
- DATE: 2026-01-01T13:50:12.516Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- **Status:** Done
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

SKELETON APPROVED

## IMPLEMENTATION
- Updated FlightRecorderEventType to include canonical terminal_command/editor_edit variants while keeping llm_inference as the approved non-FR-EVT type.
- Added per-type payload validation in FlightRecorderEvent::validate (terminal_command, diagnostic, debug_bundle_export, workflow_recovery, editor_edit, llm_inference) with TERM-API-005 optionality handled as null-acceptable fields.
- Renamed workflow recovery and LLM payload structs; updated terminal, workflow, and Ollama emission paths to use canonical types and TokenUsage (prompt/completion/total).
- Added explicit back-compat mapping for stored event_type strings in DuckDB (including legacy terminal_command payloads stored as capability_action).
- Switched MEX denial logging to System events to avoid Diagnostic schema misuse.

## HYGIENE
- Ran just validator-scan.
- Ran just validator-dal-audit.
- Ran just validator-git-hygiene.
- Ran just cargo-clean.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 6
- **End**: 449
- **Line Delta**: 156
- **Pre-SHA1**: `53ec14e714858a2ba351da4e15e6b6ce1c3892cc`
- **Post-SHA1**: `333bbfecae2bf08f5d72e145aa3482f866feff9c`
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
- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 595
- **End**: 998
- **Line Delta**: 12
- **Pre-SHA1**: `fa375d9bf261a5b23c09bcbb98dddc80fb397aea`
- **Post-SHA1**: `b021ba7144fb87256e641007a62fbe058a415e21`
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
- **Start**: 460
- **End**: 460
- **Line Delta**: 0
- **Pre-SHA1**: `414f3efcbc260fdc59b510019e1c0ead77851584`
- **Post-SHA1**: `4e1ff22861ebfd76ef92ebf7721e729ad3999d94`
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
- **Target File**: `src/backend/handshake_core/src/llm/ollama.rs`
- **Start**: 9
- **End**: 336
- **Line Delta**: 1
- **Pre-SHA1**: `7a1a2ac9a1402c2fd2518bd95a61b0572a397445`
- **Post-SHA1**: `4f0704715123ca099f9e31034834d3975cc10070`
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
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 7
- **End**: 1239
- **Line Delta**: 4
- **Pre-SHA1**: `0573f85d3816700625ce43ae685227b2bf1a0c23`
- **Post-SHA1**: `2d23d66ac96ed11150e6d268043597228068ac25`
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
- **Start**: 113
- **End**: 113
- **Line Delta**: 0
- **Pre-SHA1**: `86c94837da8ade48980af4a9bc77ed11c5d130af`
- **Post-SHA1**: `3fc2b0a9189a74b873154c55288aaf19d3f703a2`
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
- **Lint Results**: see EVIDENCE
- **Artifacts**: see EVIDENCE
- **Timestamp**: 2026-01-01T17:48:24.0463784+01:00
- **Operator**: Codex CLI (Coder-A)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- **Notes**: Manifests cover all modified backend files.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implementation complete; validation commands executed; post-work run (warning: unstaged changes).
- What changed in this update:
  - Canonicalized Flight Recorder event types and payload validation for terminal_command and llm_inference.
  - Updated workflow recovery payload naming and DuckDB legacy type mapping.
  - Adjusted MEX denial logging to avoid Diagnostic schema misuse.
- Next step / handoff hint: Ready for validator review.

## EVIDENCE
- just pre-work WP-1-Flight-Recorder-v3: PASS
- just validator-spec-regression: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder: initial run timed out; rerun PASS (10 tests)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal: PASS (3 tests)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml llm: PASS (6 tests)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS (120 tests)
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just validator-git-hygiene: PASS
- just cargo-clean: completed (removed handshake_core target artifacts)
- Anti-vibe verification (rg split_whitespace|unwrap|expect|todo!|unimplemented!): matches only in existing test/utility code.
- just post-work WP-1-Flight-Recorder-v3: PASS with warning about unstaged changes; manifest corrected for trimmed sha handling.

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- VALIDATION REPORT - WP-1-Flight-Recorder-v3
  - Verdict: PASS
  - Spec checked:
    - Handshake_Master_Spec_v02.100.md:10563 (4.2.3.2)
    - Handshake_Master_Spec_v02.100.md:23408 (TERM-API-005)
    - Handshake_Master_Spec_v02.100.md:30778 (11.5.1 Flight Recorder Ingestion Contract)
    - Handshake_Master_Spec_v02.100.md:30832 (FR-EVT-001 terminal_command)
    - Handshake_Master_Spec_v02.100.md:30856 (FR-EVT-002 editor_edit)
    - Handshake_Master_Spec_v02.100.md:30882 (FR-EVT-003 diagnostic)
    - Handshake_Master_Spec_v02.100.md:30911 (FR-EVT-005 debug_bundle_export)
    - Handshake_Master_Spec_v02.100.md:30930 (FR-EVT-WF-RECOVERY workflow_recovery)
  - Evidence mapping (requirements -> code):
    - Canonical event type strings:
      - src/backend/handshake_core/src/flight_recorder/mod.rs:51 (Display maps to terminal_command/editor_edit/diagnostic/debug_bundle_export/workflow_recovery/llm_inference)
      - src/backend/handshake_core/src/flight_recorder/duckdb.rs:603 (DuckDB event_type string mapping, including legacy capability_action handling)
    - Ingestion validation + stable InvalidEvent error code:
      - src/backend/handshake_core/src/flight_recorder/mod.rs:156 (FlightRecorderEvent::validate enforces per-type payload shape)
      - src/backend/handshake_core/src/flight_recorder/mod.rs:314 (terminal_command payload required fields + nullable numeric fields)
      - src/backend/handshake_core/src/flight_recorder/mod.rs:352 (editor_edit requires editor_surface + ops)
      - src/backend/handshake_core/src/flight_recorder/mod.rs:359 (llm_inference requires model_id + token usage fields)
      - src/backend/handshake_core/src/flight_recorder/mod.rs:486 (RecorderError::InvalidEvent uses HSK-400-INVALID-EVENT)
    - TERM-API-005 deterministic logging as terminal_command (not capability_action):
      - src/backend/handshake_core/src/terminal/mod.rs:415 (TerminalCommandEvent includes command/cwd/exit_code/duration_ms/timed_out/cancelled/truncated_bytes + job_id/model_id/session_id when present)
      - src/backend/handshake_core/src/terminal/mod.rs:446 (TerminalEventEnvelope emits type=terminal_command and FlightRecorderEventType::TerminalCommand)
    - LLM observability + budget enforcement:
      - src/backend/handshake_core/src/llm/mod.rs:32 (trait contract: trace_id + model_id + TokenUsage emission)
      - src/backend/handshake_core/src/llm/ollama.rs:103 (emit_llm_inference_event uses trace_id + model_id + TokenUsage)
      - src/backend/handshake_core/src/llm/ollama.rs:323 (max_tokens enforcement returns BudgetExceeded)
    - Workflow recovery event:
      - src/backend/handshake_core/src/workflows.rs:350 (emits WorkflowRecovery event with actor=system)
      - src/backend/handshake_core/src/flight_recorder/mod.rs:186 (workflow_recovery requires actor=system)
  - Commands executed (validator):
    - just cargo-clean: PASS
    - just validator-spec-regression: PASS
    - just validator-scan: PASS
    - just validator-dal-audit: PASS
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder: PASS (10 tests)
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal: PASS (3 tests)
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml llm: PASS (6 tests)
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS (120 + 2 + 2 + 5 + 2 + 10 + 5 + 4 + 5 tests)
    - just post-work WP-1-Flight-Recorder-v3: PASS (staged-only; warning about unstaged parallel changes)
  - REASON FOR PASS:
    - Task packet DONE_MEANS requirements are satisfied with direct code evidence and passing TEST_PLAN commands, and post-work deterministic manifest gates pass for the staged WP diff set.

## ORCHESTRATOR_HANDOFF_CONFIRMATION
- Packet created: docs/task_packets/WP-1-Flight-Recorder-v3.md
- Refinement approved and signed: docs/refinements/WP-1-Flight-Recorder-v3.md (USER_SIGNATURE: ilja010120261446)
- Signature recorded (one-time use): docs/SIGNATURE_AUDIT.md contains ilja010120261446; docs/ORCHESTRATOR_GATES.json contains refinement+signature records for WP-1-Flight-Recorder-v3
- Task board updated: docs/TASK_BOARD.md includes WP-1-Flight-Recorder-v3 as Ready for Dev; WP-1-Flight-Recorder-v2 marked superseded by v3
- Pre-work gate: just pre-work WP-1-Flight-Recorder-v3 (PASS)

## WAIVERS GRANTED [CX-573F] (APPEND-ONLY)
- [WAIVER-2026-01-01-WORKFLOW-ROLLOUT] Coders not yet aligned to new branch/worktree workflow; allowed for this WP only. Approver: ilja. Expires: on WP closure.
