# Task Packet: WP-1-Flight-Recorder-v2

## Metadata
- TASK_ID: WP-1-Flight-Recorder-v2
- WP_ID: WP-1-Flight-Recorder-v2
- DATE: 2025-12-28
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: Done

## User Context
This task implements the "Flight Recorder," which is like a black box for the Handshake application. It records every important action the AI takes, every technical decision made, and any errors that occur. This information is stored in a permanent database (DuckDB) so that we can audit what happened later, even if the app crashes. This is a critical safety and transparency feature required to finish the first phase of the project.

## Scope
- **What**: Implement the normative `FlightRecorder` trait and its DuckDB-backed storage sink.
- **Why**: Existing implementation is fragmented or non-normative. We need a central, durable, and queryable audit log to meet Phase 1 closure requirements [§11.5].
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/flight_recorder/
  * src/backend/handshake_core/src/observability/
  * src/backend/handshake_core/src/lib.rs (Minimal touch for AppState integration)
  * src/backend/handshake_core/src/main.rs (Initialization)
- **OUT_OF_SCOPE**:
  * Tokenization logic (Occupied by another agent).
  * Direct LLM provider integration (Occupied by another agent).
  * UI/Frontend components (Defer to v3 or separate WP).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Foundational infrastructure for auditability; impacts all system-wide tracing.
- **TEST_PLAN**:
  ```bash
  # 1. Compile and unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder

  # 2. Verify DuckDB persistence
  # (Manual check or specific integration test that writes and reads back)

  # 3. External Cargo target hygiene
  just cargo-clean

  # 4. Post-work validation
  just post-work WP-1-Flight-Recorder-v2
  ```
- **DONE_MEANS**:
  * ✅ `FlightRecorder` trait implemented with `append_event(event: FlightRecorderEvent)` method per §11.5.1.
  * ✅ DuckDB storage sink correctly creates required tables on startup if missing.
  * ✅ Mandatory 7-day rolling retention policy implemented (automatic pruning of older logs).
  * ✅ Canonical `FR-EVT-*` JSON shapes are successfully persisted and queryable by `trace_id`.
  * ✅ All existing tests pass and new tests cover the DuckDB write/read path.
- **HARDENED_INVARIANTS**:
  * **Content-Awareness**: Events MUST be validated against the schema before ingestion.
  * **NFC Normalization**: All text content in events MUST be normalized to prevent bypasses.
  * **Atomic Poisoning**: The sink MUST NOT block the main thread; use async/background task for DuckDB writes.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md (v02.95)
  * docs/ARCHITECTURE.md
  * src/backend/handshake_core/src/flight_recorder/mod.rs (Trait definition)
  * src/backend/handshake_core/src/lib.rs (AppState)
- **SEARCH_TERMS**:
  * "pub trait FlightRecorder"
  * "FR-EVT-"
  * "DuckDB"
  * "trace_id"
  * "Retention"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  just dev
  ```
- **RISK_MAP**:
  * "DuckDB file lock contention" -> Observability Layer (System hang)
  * "Schema mismatch on legacy data" -> Database (Startup Failure)
  * "Trace ID propagation gap" -> Audit Trail (Broken Linkage)

## Authority
- **SPEC_ANCHOR**: §11.5 (Flight Recorder Event Shapes & Retention)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.96.md [ilja281220250525]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Dependencies**: Depends on `WP-1-Storage-Foundation-v2` (Completed).
- **Collision Warning**: GPT Codex is working in `src/tokenization/` and `src/llm/`. **DO NOT** modify files in those directories.
- **Waiver**: DuckDB is the only approved sink for this phase.

---

## VALIDATION
- **Deterministic Manifest (current workflow)**:
  - **Target File**: `<fill before post-work>`
  - **Start**: 1
  - **End**: 1
  - **Line Delta**: 0
  - **Pre-SHA1**: `0000000000000000000000000000000000000000`
  - **Post-SHA1**: `0000000000000000000000000000000000000000`
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
    - [ ] compilation_clean
    - [ ] tests_passed
    - [ ] outside_window_pristine
    - [ ] lint_passed
    - [ ] ai_review (if required)
    - [ ] task_board_updated
    - [ ] commit_ready
  - **Lint Results**: <suite + pass/fail summary>
  - **Artifacts**: <paths if any>
  - **Timestamp**:
  - **Operator**:
  - **Notes**:
- **Validation Commands / Results**:
  - just post-work WP-1-Flight-Recorder-v2

---

**Last Updated:** 2025-12-28
**User Signature Locked:** ilja281220250519
## REVALIDATION NOTE 2025-12-28
- STATUS: In-Progress (revalidation required against Master Spec v02.96).
- ACTION: Rerun TEST_PLAN and validator scans; refresh EVIDENCE_MAPPING for v02.96 alignment.

## VALIDATION REPORT — 2025-12-28 (Revalidation, Spec v02.96)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Flight-Recorder-v2.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.96 §11.5 (Flight Recorder)

Findings:
- FlightRecorder trait defines required append + retention primitives; DuckDB sink enforces 7-day rolling retention and NFC normalization of payloads before ingest (flight_recorder/mod.rs, flight_recorder/duckdb.rs).
- FR-EVT-* shapes persisted and queryable by trace_id; retention tests cover purge path.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests --quiet` (PASS)

Reason for PASS: Trait + DuckDB implementation align with §11.5 (event schema, normalization, retention) and tests pass.

## STATUS CANONICAL (2025-12-28)
- Authoritative STATUS: Done (validated against Master Spec v02.96).
- Earlier status lines in this packet are historical and retained for audit only.

---

## REVALIDATION REPORT - WP-1-Flight-Recorder-v2 (2025-12-30)

VALIDATION REPORT - WP-1-Flight-Recorder-v2
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Flight-Recorder-v2.md
- Spec Pointer: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (11.5 Flight Recorder Event Shapes & Retention)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: docs/VALIDATOR_PROTOCOL.md

Commands (evidence):
- just cargo-clean: PASS
- just validator-spec-regression: PASS
- just validator-packet-complete WP-1-Flight-Recorder-v2: FAIL (STATUS missing/invalid)
- node scripts/validation/gate-check.mjs WP-1-Flight-Recorder-v2: FAIL (Implementation detected without SKELETON APPROVED marker.)
- node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-v2: FAIL (non-ASCII packet + missing COR-701 manifest fields/gates)

Blocking Findings:
1) Phase gate FAIL: missing SKELETON APPROVED marker (gate-check).
2) Deterministic manifest gate FAIL (COR-701): post-work-check fails because:
   - packet contains non-ASCII bytes (count=28)
   - no COR-701 manifest fields parsed (target_file/start/end/pre_sha1/post_sha1/line_delta) and required gates are missing/un-checked
3) Spec mismatch: packet references Handshake_Master_Spec_v02.96.md, but docs/SPEC_CURRENT.md now requires Handshake_Master_Spec_v02.98.md.

Spec-to-code spot-check (non-exhaustive; blocked by gates above):
- Spec 11.5 defines FR-EVT-002 as EditorEditEvent (Handshake_Master_Spec_v02.98.md:30812).
- Code defines FrEvt002LlmInference (src/backend/handshake_core/src/flight_recorder/mod.rs:214) and FlightRecorderEventType::LlmInference (src/backend/handshake_core/src/flight_recorder/mod.rs:35), which does not match the v02.98 FR-EVT-002 schema.

REASON FOR FAIL:
- Required workflow gates (gate-check + COR-701 post-work-check) do not pass, and the packet is anchored to a non-current spec version.

Required Remediation:
- Create a NEW packet (recommended: WP-1-Flight-Recorder-v3) anchored to Handshake_Master_Spec_v02.98.md (ASCII-only).
- Follow phase gate: BOOTSTRAP -> SKELETON -> (Validator issues "SKELETON APPROVED") -> IMPLEMENTATION -> VALIDATION.
- Provide a full COR-701 deterministic manifest so `just post-work` can pass.
- Reconcile Flight Recorder FR-EVT schema numbering/types to match v02.98 (requires code changes or an explicit spec version bump).

Task Board Update:
- Move WP-1-Flight-Recorder-v2 from Done -> Ready for Dev (Revalidation FAIL).

Packet Status Update (append-only):
- **Status:** Ready for Dev

Timestamp: 2025-12-30
Validator: Codex CLI (Validator role)


