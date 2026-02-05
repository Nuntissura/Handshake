## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED before any WP activation/signature.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md Part 2.5.2.

### METADATA
- WP_ID: WP-1-Supply-Chain-MEX-v2
- CREATED_AT: 2026-01-17T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Supply-Chain-MEX-v2
- USER_SIGNATURE: ilja170120262249

### REQUIRED SECTIONS (per .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md Part 2.5.2)

### GAPS_IDENTIFIED
- Master Spec gaps: NONE (CLEARLY_COVERS_VERDICT=PASS; ENRICHMENT_NEEDED=NO).
- Current codebase gap (inspection, do not trust prior attempts): supply-chain MEX engines/jobs are not implemented end-to-end. In particular:
  - src/backend/handshake_core/mechanical_engines.json does not declare engine.supply_chain.* engines.
  - CI has gitleaks + pnpm audit + cargo deny, but there is no MEX Job path producing artifacts with provenance and no Operator-visible Flight Recorder + Problems linkage for supply-chain reports.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-001 (TerminalCommandEvent): every external scanner invocation (gitleaks, osv-scanner, syft, cargo-deny or scancode) MUST produce a TerminalCommand Flight Recorder event with job/workflow correlation fields populated.
- FR-EVT-003 (DiagnosticEvent): when supply-chain policy triggers a BLOCK (see 11.7.5 release build rule), the resulting Diagnostic.id MUST be linkable from Flight Recorder via FR-EVT-003.
- Supply-chain engines MUST emit an additional Flight Recorder System event (event_type=system) with payload that includes:
  - engine_id, operation, op_id
  - tool identity + tool version string used for engine_version in SupplyChainReport
  - input artifact refs (by handle) and output artifact refs (by handle)
  - diagnostic_id when a BLOCK Problem is emitted

### RED_TEAM_ADVISORY (security failure modes)
- Secret leakage: gitleaks findings MUST NOT write raw secrets into Flight Recorder payloads/logs; require redacted reports and avoid embedding secret strings in diagnostics.
- Toolchain trust: scanners are themselves supply-chain risk; versions MUST be pinned and tool versions recorded in artifacts/provenance.
- Non-determinism: scanner results can drift due to advisory DB updates; treat as D0/D1 with evidence artifacts and record versioning inputs (tool version, DB version where available).
- Performance/DoS: scanning can be slow and output-heavy; enforce time/budget caps, output truncation policy, and ensure timeouts emit actionable diagnostics.
- False-positive merge blocks: allowlists MUST be versioned and referenced as artifacts/provenance, not ad-hoc local files.

### PRIMITIVES (traits/structs/enums)
- Mechanical engine IDs (Spec 11.7.5):
  - engine.supply_chain.vuln
  - engine.supply_chain.sbom
  - engine.supply_chain.license
- Validator job kinds (Spec 11.7.5.9.4.4):
  - secret_scan
  - vuln_scan
  - sbom_generate
  - license_scan
- SupplyChainReport artifact schema (Spec 11.7.5):
  - SupplyChainReport { kind: Vuln | SBOM | License, engine_version: String, timestamp: DateTime, findings: JSON }
- Diagnostics policy mapping (Spec 11.7.5):
  - HIGH severity vulnerability or UNKNOWN license in release mode => BLOCK Problem (map to DiagnosticSeverity::Fatal).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.113 explicitly defines supply-chain engines/job names, required artifact schema, and the release-mode BLOCK diagnostics rule; it also defines Flight Recorder logging requirements for mechanical tools and diagnostic linkage (FR-EVT-001/003). No spec enrichment required to implement.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already contains normative requirements for supply-chain MEX engines, validator job names, artifact schema, and the BLOCK-on-release policy, plus the required Flight Recorder logging/linkability rules needed for auditability.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.7.5 Supply Chain Mechanical Gates (MEX v1.2)
- CONTEXT_START_LINE: 35288
- CONTEXT_END_LINE: 35296
- CONTEXT_TOKEN: #### 11.7.5 Supply Chain Mechanical Gates (MEX v1.2)
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 11.7.5 Supply Chain Mechanical Gates (MEX v1.2)
- **Engine IDs**:
  - `engine.supply_chain.vuln`: Wraps `cargo-audit` / `npm audit` / `osv-scanner`.
  - `engine.supply_chain.sbom`: Generates CycloneDX / SPDX via `syft`.
  - `engine.supply_chain.license`: Wraps `scancode-toolkit` or `cargo-deny`.
- **Capability Requirements**: All supply-chain engines require `proc.exec` for their respective binaries and `fs.read:inputs`.
- **Artifact Schemas**:
  - `SupplyChainReport { kind: Vuln | SBOM | License, engine_version: String, timestamp: DateTime, findings: JSON }`.
- **Governance**: Any HIGH severity vulnerability or UNKNOWN license found during a `release` build MUST be emitted as a `BLOCK` problem in the diagnostics registry.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 6 Observability and safety (mechanical tools)
- CONTEXT_START_LINE: 13806
- CONTEXT_END_LINE: 13810
- CONTEXT_TOKEN: All mechanical tool invocations **MUST** be logged in the Flight Recorder
- EXCERPT_ASCII_ESCAPED:
  ```text
6. **Observability and safety.**
   - All mechanical tool invocations **MUST** be logged in the Flight Recorder (Section 2.1.5) with: tool identity, version, inputs (by reference), outputs (by reference), and errors.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md Supply-chain gate mechanical Jobs (CI-gated)
- CONTEXT_START_LINE: 35577
- CONTEXT_END_LINE: 35581
- CONTEXT_TOKEN: Supply-chain gate mechanical Jobs (CI-gated): `secret_scan`
- EXCERPT_ASCII_ESCAPED:
  ```text
- [ADD v02.44] Supply-chain gate mechanical Jobs (CI-gated): `secret_scan` (gitleaks), `vuln_scan` (osv-scanner), `sbom_generate` (syft), `license_scan` (scancode), each emitting artifacts + provenance.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.7.5.9.4.4 Module tech.gates.supply_chain (validator jobs + acceptance criteria)
- CONTEXT_START_LINE: 47005
- CONTEXT_END_LINE: 47012
- CONTEXT_TOKEN: Module: `tech.gates.supply_chain`
- EXCERPT_ASCII_ESCAPED:
  ```text
###### 11.7.5.9.4.4 Module: `tech.gates.supply_chain` (v0.1)
**Validator jobs:** `secret_scan`, `vuln_scan`, `sbom_generate`, `license_scan` with hard fail conditions.
**Acceptance criteria:** deterministic results stored as artifacts; configurable allowlists are versioned.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.7.5.10 Conformance tests (spec-level)
- CONTEXT_START_LINE: 47020
- CONTEXT_END_LINE: 47028
- CONTEXT_TOKEN: #### 11.7.5.10 8. Conformance tests (spec-level)
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 11.7.5.10 8. Conformance tests (spec-level)
1) Register completeness: every integrated component is present in the OSS Register with license + mode + pinning.
5) Gate enforcement: supply-chain validators block promotion when configured to hard fail.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md Flight Recorder event shapes (FR-EVT-003 DiagnosticEvent)
- CONTEXT_START_LINE: 39046
- CONTEXT_END_LINE: 39050
- CONTEXT_TOKEN: - **FR-EVT-003 (DiagnosticEvent)**
- EXCERPT_ASCII_ESCAPED:
  ```text
- **FR-EVT-003 (DiagnosticEvent)**
  Used for Problems/diagnostics; see 11.5 for full shape.
  ```

