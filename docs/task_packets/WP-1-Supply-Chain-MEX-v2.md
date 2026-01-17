# Task Packet: WP-1-Supply-Chain-MEX-v2

## METADATA
- TASK_ID: WP-1-Supply-Chain-MEX-v2
- WP_ID: WP-1-Supply-Chain-MEX-v2
- BASE_WP_ID: WP-1-Supply-Chain-MEX (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-17T22:05:23.155Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja170120262249
- SUPERSEDES: WP-1-Supply-Chain-MEX (historical FAIL; v2 is protocol-clean remediation)

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Supply-Chain-MEX-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Supply-Chain MEX v1.2 requirements: supply-chain mechanical engines + CI-gated validator jobs + artifact/provenance output + Operator-visible diagnostics, all Flight Recorder logged.
- Why: Supply chain is security-critical; we must deterministically detect secrets/vulns/licenses, emit auditable artifacts, and BLOCK release promotion on hard-fail conditions per Spec Main Body.
- IN_SCOPE_PATHS:
  - .github/workflows/ci.yml
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/src/mex/mod.rs
  - src/backend/handshake_core/src/mex/registry.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/mex/supply_chain.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/tests/mex_tests.rs
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (see `docs/refinements/WP-1-Supply-Chain-MEX-v2.md`).
  - Any changes in app/ or app/src-tauri/ or src/ outside the IN_SCOPE_PATHS above.
  - Any modifications to scripts/ (CI checks, deterministic enforcement scripts) in this packet.
  - Do not edit `docs/TASK_BOARD.md` in a two-coder session (orchestrator updates it to avoid collisions).
  - If additional file changes are required beyond IN_SCOPE_PATHS, STOP and request an Orchestrator scope update before proceeding.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Supply-Chain-MEX-v2

# Backend format/lint/tests (required):
just cargo-clean
just fmt
just lint
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Dependency & license checks:
just deny

just cargo-clean
just post-work WP-1-Supply-Chain-MEX-v2
```

### DONE_MEANS
- `src/backend/handshake_core/mechanical_engines.json` declares:
  - `engine.supply_chain.vuln`
  - `engine.supply_chain.sbom`
  - `engine.supply_chain.license`
- Spec job kinds are implemented and gated per `tech.gates.supply_chain`: `secret_scan`, `vuln_scan`, `sbom_generate`, `license_scan`, each emitting artifacts + provenance.
- SupplyChainReport output schema is implemented and produced (kind=Vuln|SBOM|License; includes engine_version + timestamp + findings JSON).
- Release-mode hard-fail policy is enforced: HIGH vulnerabilities or UNKNOWN license => BLOCK (fatal Diagnostic) and is linkable via Flight Recorder FR-EVT-003.
- Every external scanner invocation emits FR-EVT-001 (TerminalCommandEvent) and supply-chain engines emit a System Flight Recorder event with engine_id/op_id/tool version + artifact refs + diagnostic_id (when present).
- `just pre-work WP-1-Supply-Chain-MEX-v2` passes and the checkpoint commit exists (packet + refinement committed on WP branch).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-17T22:05:23.155Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.113.md:35290 Supply-chain engines: engine.supply_chain.vuln/sbom/license
  - Handshake_Master_Spec_v02.113.md:35295 SupplyChainReport schema (kind/engine_version/timestamp/findings)
  - Handshake_Master_Spec_v02.113.md:35579 CI-gated supply-chain jobs list (secret_scan/vuln_scan/sbom_generate/license_scan)
  - Handshake_Master_Spec_v02.113.md:47007 Module `tech.gates.supply_chain` (validator jobs + hard fail conditions)
  - Handshake_Master_Spec_v02.113.md:39043 FR-EVT-001 (TerminalCommandEvent) + FR-EVT-003 (DiagnosticEvent) linkage
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - docs/task_packets/WP-1-Supply-Chain-MEX.md (historical FAIL; incomplete supply-chain gate implementation)
  - docs/task_packets/stubs/WP-1-Supply-Chain-MEX-v2.md (stub backlog; superseded by this official packet)
- Preserved: All Main Body requirements under Spec 11.7.5 (supply-chain engines/jobs, artifacts/provenance, Flight Recorder logging, hard fail policy).
- Changed: v2 is a protocol-clean packet with signed technical refinement and deterministic workflow gates.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - docs/refinements/WP-1-Supply-Chain-MEX-v2.md
  - docs/task_packets/WP-1-Supply-Chain-MEX.md
  - .github/workflows/ci.yml
  - src/backend/handshake_core/mechanical_engines.json
  - src/backend/handshake_core/src/mex/mod.rs
  - src/backend/handshake_core/src/mex/registry.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/mex_tests.rs
- SEARCH_TERMS:
  - "engine.supply_chain"
  - "SupplyChainReport"
  - "tech.gates.supply_chain"
  - "secret_scan"
  - "vuln_scan"
  - "sbom_generate"
  - "license_scan"
  - "FR-EVT-001"
  - "TerminalCommandEvent"
  - "FR-EVT-003"
  - "DiagnosticEvent"
  - "cargo-deny"
  - "gitleaks"
  - "osv-scanner"
  - "syft"
  - "scancode"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Supply-Chain-MEX-v2
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml mex
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "Secrets leak into logs/artifacts" -> "data exfiltration; must redact outputs and avoid raw secret strings in Flight Recorder payloads"
  - "Scanner non-determinism" -> "flaky gate outcomes; must pin tool versions and record tool/DB versions in provenance/artifacts"
  - "Missing/incorrect CI wiring" -> "gates not enforced; must add CI jobs per Spec and ensure hard-fail conditions block promotion"
  - "Incorrect Diagnostic/FR linking" -> "operator cannot debug; must ensure FR-EVT-001/003 correlation and include diagnostic_id in system events"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
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
