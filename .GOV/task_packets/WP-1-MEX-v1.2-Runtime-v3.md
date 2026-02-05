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
- REFINEMENT_FILE: .GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md
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
- `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md` exists on disk,
- `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md` exists on disk,
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
  - Any Master Spec edits/version bumps (see `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md`).
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
  - Do not edit `.GOV/roles_shared/TASK_BOARD.md` in a two-coder session (orchestrator updates it to avoid collisions).

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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.100.md 6.3.0 Mechanical Tool Bus Contract (required global gates; artifact-first; no-bypass)
  - Handshake_Master_Spec_v02.100.md 11.8 Mechanical Extension Specification v1.2 (Verbatim): 4.1, 4.2, 6, 7, 8, 9
  - Handshake_Master_Spec_v02.100.md 11.1 Capabilities & Consent Model (HSK-4001 SSoT + audit requirement)
  - Handshake_Master_Spec_v02.100.md 11.5 Flight Recorder Event Shapes: FR-EVT-003 (DiagnosticEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles/coder/CODER_PROTOCOL.md
  - .GOV/roles/validator/VALIDATOR_PROTOCOL.md
  - .GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md
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
  - Refinement file `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md` needs review for any additional constraints (not yet incorporated).
  - Should `GateDenial.code` be `Some("HSK-4001")` only for unknown capability, or also for disallowed-but-known capability?
  - Confirm mapping from `DenialSeverity` -> `DiagnosticSeverity` (Error -> Error, Warn -> Warning).
  - Confirm whether empty `capabilities_requested` should remain default-deny even when `allowed_caps` is empty.

SKELETON APPROVED

## IMPLEMENTATION
- Updated MEX envelopes so `PlannedOperation.evidence_policy` is optional and gate checks can deny D0/D1 without policy.
- Hardened gates: capability allowlist derived from registry engine+operation, HSK-4001 recorded via `GateDenial.code`, and evidence policy check handles `Option`.
- MexRuntime now accepts DiagnosticsStore, logs gate outcomes, emits per-capability CapabilityAction events, and enforces D0/D1 evidence presence post-execution.
- Conformance harness and registry test data updated for operation-level capabilities; tests expanded for gate outcomes, capability audit, and missing evidence diagnostics.

## HYGIENE
- just pre-work WP-1-MEX-v1.2-Runtime-v3: PASS
- just validator-spec-regression: PASS
- cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check: PASS (after cargo fmt)
- cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- rg "unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|dbg!\\(|println!\\(|eprintln!\\(" src/backend/handshake_core/src/mex: no matches
- powershell -NoProfile -Command "cd src/backend/handshake_core; cargo deny check advisories licenses bans sources": PASS with existing warnings
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just validator-git-hygiene: PASS
- just cargo-clean: completed
- just post-work WP-1-MEX-v1.2-Runtime-v3: PASS (staged)

## VALIDATION
### Manifest Entry 1: src/backend/handshake_core/src/mex/conformance.rs
- **Target File**: `src/backend/handshake_core/src/mex/conformance.rs`
- **Start**: 9
- **End**: 269
- **Line Delta**: 17
- **Pre-SHA1**: `3327e23f83baa6f799f0bad737e8e1df2be24d2b`
- **Post-SHA1**: `cfaf8216574977d64e89618b1495010fcc5c7fe4`
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

### Manifest Entry 2: src/backend/handshake_core/src/mex/envelope.rs
- **Target File**: `src/backend/handshake_core/src/mex/envelope.rs`
- **Start**: 71
- **End**: 71
- **Line Delta**: 0
- **Pre-SHA1**: `56bfea135ee129b1931bab56fd4f5d454c4ac0bb`
- **Post-SHA1**: `6c79e90b9e0c0e6cc028a0893f237ca8d8415482`
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

### Manifest Entry 3: src/backend/handshake_core/src/mex/gates.rs
- **Target File**: `src/backend/handshake_core/src/mex/gates.rs`
- **Start**: 2
- **End**: 295
- **Line Delta**: 55
- **Pre-SHA1**: `c438dda21834960518ce3ab09eda9f8d05fc0842`
- **Post-SHA1**: `751efb1da8f957f0a5c30d6f84470a1af111bbf7`
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

### Manifest Entry 4: src/backend/handshake_core/src/mex/registry.rs
- **Target File**: `src/backend/handshake_core/src/mex/registry.rs`
- **Start**: 75
- **End**: 80
- **Line Delta**: 6
- **Pre-SHA1**: `51ceda6bf0046c9d3b04320ff16bdb5e6c72ea2c`
- **Post-SHA1**: `c88330fadb871b8ee2c5493315316b2b0b227b6a`
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

### Manifest Entry 5: src/backend/handshake_core/src/mex/runtime.rs
- **Target File**: `src/backend/handshake_core/src/mex/runtime.rs`
- **Start**: 8
- **End**: 305
- **Line Delta**: 190
- **Pre-SHA1**: `3fc2b0a9189a74b873154c55288aaf19d3f703a2`
- **Post-SHA1**: `893af22eb951c1162bf9cfa5cf5dbc76f2176774`
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

### Manifest Entry 6: src/backend/handshake_core/tests/mex_tests.rs
- **Target File**: `src/backend/handshake_core/tests/mex_tests.rs`
- **Start**: 5
- **End**: 399
- **Line Delta**: 277
- **Pre-SHA1**: `92efae1254eefbf47a2f2b1078bb30f7d160c840`
- **Post-SHA1**: `9f6fefc7ab57455565651042ca9c3425d1ae91c8`
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

- **Lint Results**: cargo fmt --check PASS; cargo clippy PASS
- **Artifacts**: None
- **Timestamp**: 2026-01-02T01:18:28.1388571+01:00
- **Operator**: Codex CLI (Coder-A)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- **Notes**: Window/start/end and SHA1 values computed from `git diff --unified=0` and `git show HEAD:` vs working tree.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; post-work PASS
- What changed in this update:
  - Capability audit events emitted per requested capability with allow/deny outcomes.
  - D0/D1 evidence enforcement returns error and records diagnostics when evidence is missing.
  - MEX tests expanded to cover capability audit and missing evidence diagnostics.
- Next step / handoff hint: Stage only MEX scope files + this packet and commit for validator review.

## EVIDENCE
- just pre-work WP-1-MEX-v1.2-Runtime-v3: PASS
- just validator-spec-regression: PASS
- cargo fmt --check: PASS (after cargo fmt)
- cargo clippy --all-targets --all-features -- -D warnings: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS (120 + 2 + 5 + 5 + 2 + 13 + 5 + 4 + 5 tests)
- rg unwrap/expect/todo/unimplemented/dbg/println/eprintln in mex: no matches
- pwsh not available; ran powershell -NoProfile -Command "cd src/backend/handshake_core; cargo deny check advisories licenses bans sources"
- cargo deny check advisories licenses bans sources: PASS with existing warnings (license-not-encountered + duplicate crates)
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just validator-git-hygiene: PASS
- just cargo-clean: completed
- just post-work WP-1-MEX-v1.2-Runtime-v3: PASS (staged)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT â€” WP-1-MEX-v1.2-Runtime-v3
Verdict: PASS

WP Artifacts
- Task packet present: `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md:1`
- Refinement present + signed: `.GOV/refinements/WP-1-MEX-v1.2-Runtime-v3.md:1`
- Spec target resolved: `.GOV/roles_shared/SPEC_CURRENT.md:5` -> `Handshake_Master_Spec_v02.100.md`

Scope / Diff Integrity
- Commit validated: `0ed7878` (`feat: audit capability checks and enforce MEX evidence [WP-1-MEX-v1.2-Runtime-v3]`)
- Files changed in `0ed7878` match packet IN_SCOPE + packet file:
  - `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md`
  - `src/backend/handshake_core/src/mex/conformance.rs`
  - `src/backend/handshake_core/src/mex/envelope.rs`
  - `src/backend/handshake_core/src/mex/gates.rs`
  - `src/backend/handshake_core/src/mex/registry.rs`
  - `src/backend/handshake_core/src/mex/runtime.rs`
  - `src/backend/handshake_core/tests/mex_tests.rs`
- Working tree clean on the validation branch after verification.

Spec / DONE_MEANS Requirements -> Evidence Mapping
- Schema discrimination (`schema_version="poe-1.0"`): Spec `Handshake_Master_Spec_v02.100.md:20065`; implemented in `src/backend/handshake_core/src/mex/envelope.rs:8` and enforced by `src/backend/handshake_core/src/mex/gates.rs` (SchemaGate).
- Artifact-first size rule (>32KB not inlined): Spec `Handshake_Master_Spec_v02.100.md:16786` and `Handshake_Master_Spec_v02.100.md:31664`; enforced by `src/backend/handshake_core/src/mex/gates.rs:177`.
- Required global gates exist and run: Spec `Handshake_Master_Spec_v02.100.md:16796`; pipeline includes `G-SCHEMA/G-CAP/G-INTEGRITY/G-BUDGET/G-PROVENANCE/G-DET` in `src/backend/handshake_core/src/mex/conformance.rs:52`.
- Gate outcomes logged + visible on denial: Spec `Handshake_Master_Spec_v02.100.md:16804`; PASS/DENY logged in `src/backend/handshake_core/src/mex/runtime.rs` (`record_gate_outcome`), and denials record Diagnostics via `DiagnosticsStore` (`record_denial_diagnostic`).
- HSK-4001 UnknownCapability: Spec `Handshake_Master_Spec_v02.100.md:29228`; enforced in `src/backend/handshake_core/src/mex/gates.rs:145`.
- Capability audit (allow/deny recorded): Spec `Handshake_Master_Spec_v02.100.md:29229`; emitted as `FlightRecorderEventType::CapabilityAction` per capability in `src/backend/handshake_core/src/mex/runtime.rs:89` and `src/backend/handshake_core/src/mex/runtime.rs:241`.
- D0/D1 evidence required in results: Spec `Handshake_Master_Spec_v02.100.md:16794`; enforced post-execution in `src/backend/handshake_core/src/mex/runtime.rs:128` (records Diagnostic and returns `MexRuntimeError::EvidenceMissing`).
- FR-EVT-003 Diagnostic linkage: Spec `Handshake_Master_Spec_v02.100.md:30882`; validated by tests asserting diagnostic events (see `src/backend/handshake_core/tests/mex_tests.rs`).

Quality Gate (Packet TEST_PLAN) - Re-run Results
- `just pre-work WP-1-MEX-v1.2-Runtime-v3`: PASS
- `just validator-spec-regression`: PASS
- `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check`: PASS
- `cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings`: PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: PASS (mex tests include `d0_missing_evidence_records_diagnostic`)
- Forbidden-pattern scan (`rg ... src/backend/handshake_core/src/mex`): PASS (no matches; `rg` exit code 1)
- Supply chain: `powershell -NoProfile -Command "cd src/backend/handshake_core; cargo deny check advisories licenses bans sources"`: PASS with warnings (existing allowlist + duplicate crate warnings; command exit code 0)
- `just cargo-clean`: completed
- `just post-work WP-1-MEX-v1.2-Runtime-v3`: PASS when run in the required pre-commit state (reconstructed in a dedicated worktree from `0bfc894` and applying `0ed7878` as staged changes). Note: running post-work on a clean post-commit tree fails by design because there is no diff to validate.

Deterministic Manifest (COR-701)
- Packet contains per-file manifest blocks for all changed non-doc files: `.GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md:230` onward.
- `just post-work` PASS confirms SHA1s/windows/gates are consistent for the staged diff.

