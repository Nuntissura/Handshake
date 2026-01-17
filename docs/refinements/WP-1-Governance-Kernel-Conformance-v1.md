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
- WP_ID: WP-1-Governance-Kernel-Conformance-v1
- CREATED_AT: 2026-01-16T20:45:59.2672254Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- SPEC_TARGET_SHA1: 33b50fe7d70381c3eb2a53871f673e1d442633e1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja160120262149
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Governance-Kernel-Conformance-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Repo drift (kernel conformance failure): CI/hooks/docs still reference "Codex v0.8" and/or the file `Handshake Codex v0.8.md`, but the repo's authoritative governance reference is `Handshake Codex v1.4.md` (and `Handshake Codex v0.8.md` does not exist).
  - Evidence (current repo state):
    - `.github/workflows/ci.yml` prints "Docs must reference Codex v0.8..."
    - `scripts/validation/ci-traceability-check.mjs` prints "CI Traceability Check (Codex v0.8)" and hard-fails if `Handshake Codex v0.8.md` is missing.
    - `scripts/hooks/pre-commit` prints "Pre-commit validation (Codex v0.8)" and points to `Handshake Codex v0.8.md` on failure.
    - `docs/task_packets/README.md` links to `Handshake Codex v0.8` (stale pointer).
    - `docs/ORCHESTRATOR_PROTOCOL.md` still contains example references to "Handshake Codex v0.8.md" (stale guidance).
- Concurrency constraint: Coder-A is actively working on `WP-1-Governance-Template-Volume-v1` with a file lock set under `src/backend/handshake_core/**` and `app/**`. This WP must avoid those paths to prevent merge conflicts.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE. This WP is governance enforcement drift remediation (CI/hooks/docs/scripts) and does not add or change runtime Flight Recorder events.

### RED_TEAM_ADVISORY (security failure modes)
- If CI/hook gates hard-fail on a non-existent codex filename, teams tend to bypass or disable gates under pressure, reducing actual governance enforcement (integrity risk).
- Stale references ("Codex v0.8") can mislead agents into following outdated hard rules, creating policy drift and inconsistent enforcement across roles (auditability risk).
- Fix should prefer a single source of truth for the codex filename/version to avoid repeating this drift (e.g., derive from `docs/SPEC_CURRENT.md` governance reference or equivalent pointer).

### PRIMITIVES (traits/structs/enums)
- JavaScript/Node:
  - Add/adjust a single-source-of-truth resolver for the current codex filename/version used by CI/hook scripts (preferred: derive from `docs/SPEC_CURRENT.md`, which already states the authoritative Governance Reference).
  - Update `scripts/validation/ci-traceability-check.mjs` to validate the correct codex filename exists (not `Handshake Codex v0.8.md`) and to report the correct codex version string.
- Shell/CI:
  - Update `scripts/hooks/pre-commit` messaging to reference the correct codex filename and version.
  - Update `.github/workflows/ci.yml` messaging checks to align with the current codex version string and avoid stale version guards.
- Docs:
  - Update any governance-facing docs that still hardcode "Codex v0.8" when they are meant to point at the current Governance Reference.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.112 explicitly defines (a) drift as a first-class failure mode in 7.5.4.7 and (b) the canonical governance file set (including CI + hooks + gate scripts) in 7.5.4.9.2 Template Index; the required change is to bring repo enforcement surfaces in line with those requirements.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The current Master Spec already defines the Governance Kernel drift-control requirement and the canonical CI/hook/gate artifact set; this WP is a repo conformance remediation, not a spec gap.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.7 (CI/hook parity and drift control (kernel))
- CONTEXT_START_LINE: 20506
- CONTEXT_END_LINE: 20515
- CONTEXT_TOKEN: #### 7.5.4.7 CI/hook parity and drift control (kernel)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.5.4.7 CI/hook parity and drift control (kernel)

  Kernel requirements:
  - CI runs the same governance gates as local (or stricter).
  - Determinism config is explicit (EOL policy, toolchain pinning).
  - Drift is treated as a first-class failure mode (old codex/spec references in CI/hooks/docs are detected and remediated explicitly).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.9.2 (Template Index (HARD))
- CONTEXT_START_LINE: 20606
- CONTEXT_END_LINE: 20662
- CONTEXT_TOKEN: ##### 7.5.4.9.2 Template Index (HARD)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 7.5.4.9.2 Template Index (HARD)
  | Path | Intent |
  |------|--------|
  | `{{CODEX_FILENAME}}` | Project Codex: deterministic enforcement laws and invariants (role-agnostic). |
  | `.github/workflows/ci.yml` | CI parity: runs the same (or stricter) mechanical gates as local. |
  | `docs/SPEC_CURRENT.md` | Authoritative pointer to the current Master Spec and Governance Reference (drift guard target). |
  | `docs/ORCHESTRATOR_PROTOCOL.md` | Orchestrator role protocol (refinement loop, signature gate, delegation contract). |
  | `scripts/hooks/pre-commit` | Local git hook enforcing codex checks and quick hygiene at commit time. |
  | `scripts/validation/ci-traceability-check.mjs` | Mechanical governance gate (see filename + internal docstrings). |
  ```
