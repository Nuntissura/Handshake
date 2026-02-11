## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Runtime-Governance-NoExpect-v1
- CREATED_AT: 2026-02-11T17:23:58.730Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja110220261846
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Runtime-Governance-NoExpect-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Implementation divergence: `src/backend/handshake_core/src/runtime_governance.rs` contains test code using `.expect(...)`, which is a "forbidden pattern" for hygiene audits and is currently flagged by the mechanical `just product-scan`/scan workflow.
- This WP removes `.expect(...)` usage by refactoring the tests to propagate errors via `Result` (`?`) instead of panicking.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE. This WP is a hygiene-only refactor of runtime_governance tests; it must not introduce new Flight Recorder event IDs.

### RED_TEAM_ADVISORY (security failure modes)
- Panic-as-control-flow: `expect`/`unwrap` in non-test code can crash the process; this WP removes `expect` in the affected file to align with forbidden pattern audit requirements.
- Audit bypass: ensure the fix removes the forbidden pattern string match (no `expect(...)` left in the file) so mechanical scanners cannot be bypassed by partial refactors.

### PRIMITIVES (traits/structs/enums)
- NONE. This WP changes test error-handling style only (use `Result` + `?` instead of `expect`).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.125 defines a hard Forbidden Pattern Audit requirement [CX-573E] and explicitly includes `unwrap`/`expect` as forbidden patterns to audit for (with limited exceptions). This WP is a narrow remediation to remove `.expect(...)` usage in the targeted runtime_governance file to satisfy that requirement and the repo's mechanical scan gates.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Forbidden Pattern Audit requirement and the list of audited forbidden patterns (including `expect`) already exist in the Master Spec v02.125; this WP is implementation remediation only.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md [CX-573E] FORBIDDEN_PATTERN_AUDIT (HARD)
- CONTEXT_START_LINE: 28899
- CONTEXT_END_LINE: 28902
- CONTEXT_TOKEN: [CX-573E] FORBIDDEN_PATTERN_AUDIT
- EXCERPT_ASCII_ESCAPED:
  ```text
  [CX-573D] ZERO_PLACEHOLDER_POLICY (HARD): Production code under `/src/` MUST NOT contain "placeholder" logic, "hollow" structs, or "mock" implementations for core architectural invariants (Tokenization, Security Gates, Storage Guards). If an external dependency is missing, the task is BLOCKED, not "Baseline."

  [CX-573E] FORBIDDEN_PATTERN_AUDIT (HARD): Before issuing a PASS verdict, the Validator MUST execute a `search_file_content` for "Forbidden Patterns" defined in the Spec (e.g., `split_whitespace`, `unwrap`, `Value`). If a forbidden pattern is found in a production path, the verdict is AUTO-FAIL.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2B) Hygiene & Forbidden Pattern Audit [CX-573E]
- CONTEXT_START_LINE: 34359
- CONTEXT_END_LINE: 34369
- CONTEXT_TOKEN: `expect`
- EXCERPT_ASCII_ESCAPED:
  ```text
  2B) Hygiene & Forbidden Pattern Audit (run before evidence verification)
  - Scope: files in IN_SCOPE_PATHS plus direct importers (one hop) where touched code is used.
  - Grep the touched/impacted code paths for:
    - `split_whitespace`, `unwrap`, `expect`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`, `Value` misuse (serialize/deserialize without validation).
    - `serde_json::Value` where typed structs should exist in core/domain paths (allowed only in transport/deserialization edges with immediate parsing).
    - `Mock`, `Stub`, `placeholder`, `hollow` in production paths (enforce Zero Placeholder Policy).
  - Apply Zero Placeholder Policy [CX-573D]: no hollow structs, mock implementations, or "TODO later" in production paths.
  - Allowed exceptions (must be justified in code + validation notes):
    - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
    - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
  - Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-573E].
  ```
