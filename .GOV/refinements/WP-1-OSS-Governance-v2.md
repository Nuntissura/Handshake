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
- WP_ID: WP-1-OSS-Governance-v2
- CREATED_AT: 2026-01-19T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-OSS-Governance-v2
- USER_SIGNATURE: ilja190120260338

### REQUIRED SECTIONS (per .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md Part 2.5.2)

### GAPS_IDENTIFIED
- Spec drift gap: `.GOV/roles_shared/OSS_REGISTER.md` and `src/backend/handshake_core/tests/oss_register_enforcement_tests.rs` are implemented against the older OSS Register schema (5-column table: Component/License/IntegrationMode/Scope/Purpose) and older spec references.
- Master Spec v02.113 requires an OSS Component Register schema with required columns (component_id, upstream_ref, pinning_policy, capabilities_required, compliance_notes, test_fixture, used_by_modules) and still requires deterministic enforcement against `Cargo.lock` and `package.json`.
- Current risk: updating the register schema without updating the enforcement test will break deterministic governance; leaving the register in the old schema violates the current Master Spec.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- None required for this WP. This is a deterministic build/test-time governance gate (register schema + enforcement test), not a runtime telemetry emission requirement.

### RED_TEAM_ADVISORY (security failure modes)
- Supply-chain drift: dependencies can be introduced without being recorded, undermining auditability and distribution posture.
- Copyleft contamination: GPL/AGPL components accidentally linked/embedded can create legal/compliance risk if not deterministically caught.
- False negatives: naive parsing of Cargo.lock or package.json can miss dependencies (workspace scopes, renamed packages), making the gate ineffective.
- Format brittleness: if the register parser is not strict and schema-versioned, small Markdown edits can silently disable enforcement.

### PRIMITIVES (traits/structs/enums)
- OSS Register schema and parser contract for `.GOV/roles_shared/OSS_REGISTER.md` aligned to the Master Spec required columns.
- Backend enforcement test (`src/backend/handshake_core/tests/oss_register_enforcement_tests.rs`) updated to:
  - Parse the new schema.
  - Enforce dependency coverage for `src/backend/handshake_core/Cargo.lock` and `app/package.json`.
  - Enforce copyleft isolation policy (GPL/AGPL must not be embedded; integration must be external_process or external_service per policy).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec v02.113 explicitly defines OSS licensing policy, a required OSS Component Register schema, and requires deterministic enforcement against Cargo.lock and package.json plus copyleft isolation. No new normative text is required.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec v02.113 already defines (a) OSS policy and register requirements and (b) the enforcement mechanism (backend unit test). This WP is implementation and alignment work.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:46633 11.7.4.2 License preference order
- CONTEXT_START_LINE: 46633
- CONTEXT_END_LINE: 46640
- CONTEXT_TOKEN: #### 11.7.4.2 License preference order
- EXCERPT_ASCII_ESCAPED:
```md
#### 11.7.4.2 License preference order

For components used *in-process* (`embedded_lib`), Handshake MUST follow this preference order:
1) Permissive (MIT / Apache-2.0 / BSD / ISC / Unlicense)
2) Weak copyleft (LGPL-2.1+ / MPL-2.0)
3) Strong copyleft (GPL / AGPL) -- avoid embedding; use isolation
```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:46642 11.7.4.3 Copyleft isolation rule
- CONTEXT_START_LINE: 46642
- CONTEXT_END_LINE: 46646
- CONTEXT_TOKEN: #### 11.7.4.3 Copyleft isolation rule
- EXCERPT_ASCII_ESCAPED:
```md
#### 11.7.4.3 Copyleft isolation rule

- GPL/AGPL components MUST NOT be linked into the Handshake app binary.
- If a GPL/AGPL tool is used at all, it MUST be integrated as `external_process` or `external_service`, with narrow I/O and capability gating.
```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:46647 11.7.4.4 OSS Component Register requirement
- CONTEXT_START_LINE: 46647
- CONTEXT_END_LINE: 46655
- CONTEXT_TOKEN: #### 11.7.4.4 OSS Component Register requirement
- EXCERPT_ASCII_ESCAPED:
```md
#### 11.7.4.4 OSS Component Register requirement

Any third-party component that is shipped, executed, or embedded as part of a Handshake workflow MUST be recorded in the OSS Component Register, including:
- license (SPDX where possible)
- integration mode (`embedded_lib` / `external_process` / `external_service`)
- pinning policy (version + integrity strategy)
- required capabilities
- compliance notes and fixtures
```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:46764 11.7.5.7.1 Register schema (required columns)
- CONTEXT_START_LINE: 46764
- CONTEXT_END_LINE: 46775
- CONTEXT_TOKEN: ##### 11.7.5.7.1 5.1 Register schema (required columns)
- EXCERPT_ASCII_ESCAPED:
```md
##### 11.7.5.7.1 5.1 Register schema (required columns)

- `component_id`
- `name`
- `upstream_ref`
- `license`
- `integration_mode_default`
- `capabilities_required`
- `pinning_policy`
- `compliance_notes`
- `test_fixture`
- `used_by_modules`
```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:49552 11.10.4 (2) OSS Register Enforcement
- CONTEXT_START_LINE: 49547
- CONTEXT_END_LINE: 49554
- CONTEXT_TOKEN: 2.  **OSS Register Enforcement:**
- EXCERPT_ASCII_ESCAPED:
```md
### 11.10.4 Phase 1 Final Gap Closure (Calendar, OSS, Validators)

2.  **OSS Register Enforcement:**
    - **Register:** Maintain `.GOV/roles_shared/OSS_REGISTER.md` as the source of truth for licensed components.
    - **Enforcement:** Add a backend unit test that fails if a dependency in `Cargo.lock` or `package.json` is missing from the register or violates the copyleft isolation rule.
```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:47024 11.7.5.10 Conformance tests (Register completeness + License isolation)
- CONTEXT_START_LINE: 47020
- CONTEXT_END_LINE: 47025
- CONTEXT_TOKEN: 1) **Register completeness:**
- EXCERPT_ASCII_ESCAPED:
```md
#### 11.7.5.10 8. Conformance tests (spec-level)

An implementation claiming conformance to this spec MUST satisfy:

1) **Register completeness:** every integrated component is present in the OSS Register with license + mode + pinning.
2) **License isolation:** any GPL/AGPL component is external_process/external_service only (no linking into binary).
```

