# Task Packet: WP-1-MEX-v1.2-Runtime-v3

## METADATA
- TASK_ID: WP-1-MEX-v1.2-Runtime-v3
- WP_ID: WP-1-MEX-v1.2-Runtime-v3
- DATE: 2026-01-01T22:19:30Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: gpt-5-codex
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010120262219
- SUPERSEDES: WP-1-MEX-v1.2-Runtime-v2 (revalidation FAIL / governance drift; v3 is protocol-clean)

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md
- Rule: Implementation is BLOCKED until refinement is approved/signed and `just pre-work WP-1-MEX-v1.2-Runtime-v3` passes.

## USER_CONTEXT (Non-Technical Explainer)

Mechanical Extensions (MEX) are the "tool runner" side of Handshake: they run deterministic tools under strict safety rules.

This work packet ensures:
- Tool requests follow a strict envelope (PlannedOperation / poe-1.0 engine invocation).
- Tool results follow a strict envelope (EngineResult with provenance/evidence/log refs).
- A fixed set of safety checks (global gates) always run before any tool executes.
- If a tool request is denied, the denial is visible (Problems) and logged (Flight Recorder).

## WHY THIS PACKET WAS RECREATED (STOP-WORK NOTICE)

This packet and its signed refinement file were deleted from disk. Without the packet+refinement pair, the workflow gates cannot run, scope boundaries cannot be enforced, and work cannot be validated deterministically.

Coder instruction: do not start coding until:
- `docs/task_packets/WP-1-MEX-v1.2-Runtime-v3.md` exists on disk,
- `docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md` exists on disk,
- `just pre-work WP-1-MEX-v1.2-Runtime-v3` passes,
- You have written the SKELETON and received explicit approval to implement.

## SCOPE
- What: Remediate and harden the existing MEX v1.2 runtime implementation to match SPEC_CURRENT v02.100 for the Mechanical Tool Bus contract (envelopes + gates + registry + conformance), and ensure gate denials are logged and visible (Problems + Flight Recorder).
- Why: MEX is security-critical (capability gating + sandbox boundary). This WP restores governance-valid artifacts so the work can be performed and validated deterministically.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/mex/mod.rs
  - src/backend/handshake_core/src/mex/envelope.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/mex/registry.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/tests/mex_tests.rs
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (see `docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md`).
  - Any work in app/ or app/src-tauri/ or src/ outside the IN_SCOPE_PATHS above.
  - Implementing real engine adapters beyond conformance test adapters.
  - Any changes to the Terminal WP locked paths:
    - src/backend/handshake_core/src/terminal/mod.rs
    - src/backend/handshake_core/src/terminal/session.rs
    - src/backend/handshake_core/src/terminal/guards.rs
    - src/backend/handshake_core/src/terminal/config.rs
    - src/backend/handshake_core/src/terminal/redaction.rs
    - src/backend/handshake_core/tests/terminal_session_tests.rs
    - src/backend/handshake_core/tests/terminal_guards_tests.rs
  - Do not edit `docs/TASK_BOARD.md` in a two-coder session (orchestrator updates it to avoid collisions).

## WAIVERS GRANTED
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Gate 0: must pass before touching code
just pre-work WP-1-MEX-v1.2-Runtime-v3

# Spec integrity (must remain PASS):
just validator-spec-regression

# Backend format/lint/tests (required):
cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Forbidden patterns (scoped):
rg "unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|dbg!\\(|println!\\(|eprintln!\\(" src/backend/handshake_core/src/mex

# Supply chain (Rust crate only):
powershell -NoProfile -Command "cd src/backend/handshake_core; cargo deny check advisories licenses bans sources"

just cargo-clean
just post-work WP-1-MEX-v1.2-Runtime-v3
```

### DONE_MEANS
- Packet governance: `just pre-work WP-1-MEX-v1.2-Runtime-v3` passes.
- Envelope compliance (Master Spec v02.100 6.3.0 + 11.8):
  - PlannedOperation includes minimum fields and uses schema_version="poe-1.0".
  - EngineResult includes minimum fields (including provenance/evidence/errors/logs_ref).
- Gate pipeline compliance (Master Spec v02.100 6.3.0 + 11.8):
  - Required global gates exist and run for every execution attempt: G-SCHEMA, G-CAP, G-INTEGRITY, G-BUDGET, G-PROVENANCE, G-DET.
  - Artifact-first rule: inline params >32KB are denied by G-INTEGRITY.
  - Evidence rule: D0/D1 without evidence_policy/evidence are denied by G-PROVENANCE/G-DET as required by spec.
- Visibility/audit:
  - Gate denials are surfaced in Problems and logged to Flight Recorder (Diagnostics linkage via FR-EVT-003 where applicable).
  - Capability checks follow HSK-4001 (unknown capability rejects; allow/deny audited).
- Tests/quality: All commands in TEST_PLAN pass and `just post-work WP-1-MEX-v1.2-Runtime-v3` returns PASS with complete COR-701 manifests for every changed non-doc file.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T22:19:30Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.100.md 6.3.0 Mechanical Tool Bus Contract (required global gates; artifact-first; no-bypass)
  - Handshake_Master_Spec_v02.100.md 11.8 Mechanical Extension Specification v1.2 (Verbatim): 4.1, 4.2, 6, 7, 8, 9
  - Handshake_Master_Spec_v02.100.md 11.1 Capabilities & Consent Model (HSK-4001 SSoT + audit requirement)
  - Handshake_Master_Spec_v02.100.md 11.5 Flight Recorder Event Shapes: FR-EVT-003 (DiagnosticEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md
  - src/backend/handshake_core/src/mex/envelope.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/mex/registry.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/tests/mex_tests.rs
- SEARCH_TERMS:
  - "PlannedOperation"
  - "EngineResult"
  - "poe-1.0"
  - "G-SCHEMA"
  - "G-CAP"
  - "G-INTEGRITY"
  - "G-BUDGET"
  - "G-PROVENANCE"
  - "G-DET"
  - "GateDenial"
  - "HSK-4001"
  - "DiagnosticEvent"
  - "FR-EVT-003"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-MEX-v1.2-Runtime-v3
  rg "PlannedOperation|EngineResult|poe-1\\.0|G-SCHEMA|G-CAP|G-INTEGRITY|G-BUDGET|G-PROVENANCE|G-DET|GateDenial|HSK-4001|DiagnosticEvent" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests mex_tests
  ```
- RISK_MAP:
  - "Requested caps treated as granted" -> "default-allow tool execution (security)"
  - "Gate denial not visible in Problems" -> "violates spec visibility requirements"
  - "Inline params >32KB not denied" -> "artifact-first violation and context pollution"
  - "D0/D1 without evidence policy/evidence" -> "non-reproducible tool claims"

## SKELETON
- Interfaces / types to adjust (no behavior yet):
  - `PlannedOperation.evidence_policy`: change to `Option<EvidencePolicy>` so missing policy can be detected and denied for D0/D1.
  - `GateDenial`: add `code: Option<String>` to carry HSK-4001 (unknown capability) and allow diagnostics payload to include a code.
  - `MexRegistry`: add `get_operation(engine_id, operation) -> Option<&OperationSpec>` to fetch per-op capabilities.
- Gate contracts (signatures unchanged unless noted):
  - `CapabilityGate::check(op, registry)`:
    - Determine `allowed_caps = union(engine.required_caps, operation.capabilities)` from `MexRegistry` (engine_id + operation).
    - For each `capabilities_requested` entry:
      - If capability is unknown to `CapabilityRegistry`, deny with `GateDenial.code = Some("HSK-4001")`.
      - If capability is not in `allowed_caps`, deny (no code).
    - Keep default-deny when `capabilities_requested` is empty.
  - `ProvenanceGate::check(op, registry)`:
    - If determinism is D0/D1 and `evidence_policy` is `None` or `required == false`, deny.
  - `IntegrityGate`: deny if inline params payload >32KB (artifact-first rule).
  - `DetGate`: deny if determinism rank exceeds engine ceiling (registry lookup).
- Runtime contracts:
  - `MexRuntime::new(registry, flight_recorder, diagnostics, gates)` to accept `Arc<dyn DiagnosticsStore>` for Problems visibility.
  - `record_denial_diagnostic(op, denial) -> Option<Uuid>`:
    - Build `DiagnosticInput` with `source=DiagnosticSource::Engine`, `surface=DiagnosticSurface::System`, `severity` mapped from `DenialSeverity`, `code=denial.code`, `job_id=op.op_id`.
    - Persist via `DiagnosticsStore::record_diagnostic` (DuckDbFlightRecorder will emit FR-EVT-003).
  - `record_gate_outcome(op, gate, outcome, denial_opt, diagnostic_id_opt)`:
    - Always log PASS and DENY to Flight Recorder as `FlightRecorderEventType::System`.
    - Include gate name, outcome, engine_id, operation, reason/code (if deny), severity, diagnostic_id.
  - `execute`:
    - For each gate: on PASS -> `record_gate_outcome`.
    - On DENY -> `record_denial_diagnostic`, then `record_gate_outcome`, then return `MexRuntimeError::Gate`.
- Conformance / tests:
  - `ConformanceHarness::new(...)` accepts diagnostics store and passes into `MexRuntime::new`.
  - Base operation uses `evidence_policy: Some(EvidencePolicy { required: true, ... })`.
  - Provenance case sets `evidence_policy = None` (or `required=false`) to exercise denial.
  - `single_engine_registry` adds `OperationSpec` entries for `conformance.test` and `spatial.build_model` with explicit `capabilities` to support allowed-caps gate.
  - `mex_tests.rs` adds:
    - `gate_pass_logs_outcome` (System event for PASS).
    - `gate_denial_records_diagnostic_and_event` (Diagnostic stored, FR-EVT-003 emitted by recorder, and System gate_outcome event; HSK-4001 asserted for unknown capability).
  - Open questions / confirmations for validator:
  - Refinement file `docs/refinements/WP-1-MEX-v1.2-Runtime-v3.md` needs review for any additional constraints (not yet incorporated).
  - Should `GateDenial.code` be `Some("HSK-4001")` only for unknown capability, or also for disallowed-but-known capability?
  - Confirm mapping from `DenialSeverity` -> `DiagnosticSeverity` (Error -> Error, Warn -> Warning).
  - Confirm whether empty `capabilities_requested` should remain default-deny even when `allowed_caps` is empty.

SKELETON APPROVED

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; commands run + outcomes summary.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'.)
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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
