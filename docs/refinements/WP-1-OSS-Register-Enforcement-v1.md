## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.

### METADATA
- WP_ID: WP-1-OSS-Register-Enforcement-v1
- CREATED_AT: 2026-01-01T00:00:00Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010120261528

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Enforcement gap: SPEC_CURRENT requires OSS Register Enforcement via a backend unit test that fails when dependencies are missing from `docs/OSS_REGISTER.md` or violate the copyleft isolation rule. This enforcement is not currently guaranteed by an automated test gate.
- Governance gap risk: Without a deterministic enforcement test, license posture can drift (adding a dependency without updating OSS_REGISTER) and escape review until late.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- None required. This is a build/test-time governance gate (unit test) rather than a runtime telemetry emission requirement.

### RED_TEAM_ADVISORY (security failure modes)
- Supply chain drift: a new dependency can be introduced without being recorded, undermining auditability and distribution posture.
- Copyleft contamination: a GPL/AGPL dependency can be linked/embedded accidentally, creating legal/compliance risk if not caught at test time.
- False negatives: naive parsing of Cargo.lock or package.json can miss dependencies (e.g., renamed packages, workspace scopes), resulting in an ineffective gate.

### PRIMITIVES (traits/structs/enums)
- A backend unit test in `src/backend/handshake_core/tests/` that:
  - Parses `src/backend/handshake_core/Cargo.lock` and `app/package.json` dependencies.
  - Parses `docs/OSS_REGISTER.md` entries (component name + license + allowed integration mode).
  - Fails deterministically if a dependency is missing from the register.
  - Fails deterministically if a dependency license is GPL/AGPL and integration mode is not `external_process` or `external_service`.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: SPEC_CURRENT v02.100 explicitly requires maintaining `docs/OSS_REGISTER.md` and adding a backend unit test enforcing completeness and copyleft isolation.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: SPEC_CURRENT v02.100 already defines the OSS Register and the enforcement mechanism (backend unit test) and references the copyleft isolation rule; the work is implementation/enforcement.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.10.4 OSS Register Enforcement (backend unit test)
- CONTEXT_START_LINE: 33900
- CONTEXT_END_LINE: 33907
- CONTEXT_TOKEN: 2.  **OSS Register Enforcement:**
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.10.4 Phase 1 Final Gap Closure (Calendar, OSS, Validators)

  2.  **OSS Register Enforcement:**
      -   **Register:** Maintain `docs/OSS_REGISTER.md` as the source of truth for licensed components.
      -   **Enforcement:** Add a backend unit test that fails if a dependency in `Cargo.lock` or `package.json` is missing from the register or violates the copyleft isolation rule (GPL/AGPL must be `external_process`).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.7.4.3 Copyleft isolation rule
- CONTEXT_START_LINE: 30995
- CONTEXT_END_LINE: 30999
- CONTEXT_TOKEN: #### 11.7.4.3 Copyleft isolation rule
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.7.4.3 Copyleft isolation rule

  - **GPL/AGPL components MUST NOT be linked into the Handshake app binary.**
  - If a GPL/AGPL tool is used at all, it MUST be integrated as `external_process` or `external_service`, with narrow I/O and capability gating.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.7.4.4 OSS Component Register requirement
- CONTEXT_START_LINE: 31000
- CONTEXT_END_LINE: 31007
- CONTEXT_TOKEN: #### 11.7.4.4 OSS Component Register requirement
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.7.4.4 OSS Component Register requirement

  Any third-party component that is shipped, executed, or embedded as part of a Handshake workflow MUST be recorded in the OSS Component Register (see embedded snapshot below), including:
  - license (SPDX where possible),
  - integration mode (`embedded_lib` / `external_process` / `external_service`),
  - pinning policy (version + integrity strategy),
  - required capabilities,
  - compliance notes and fixtures.
  ```
