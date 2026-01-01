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
- WP_ID: {{WP_ID}}
- CREATED_AT: {{DATE_ISO}}
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> {{SPEC_TARGET_RESOLVED}}
- SPEC_TARGET_SHA1: {{SPEC_TARGET_SHA1}}
- USER_REVIEW_STATUS: PENDING
- USER_SIGNATURE: <pending>

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- <fill; write NONE if no gaps>

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- <fill; write NONE if not applicable>

### RED_TEAM_ADVISORY (security failure modes)
- <fill; write NONE if not applicable>

### PRIMITIVES (traits/structs/enums)
- <fill; write NONE if not applicable>

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PENDING
- CLEARLY_COVERS_REASON: <fill>

### ENRICHMENT
- ENRICHMENT_NEEDED: PENDING
- REASON_NO_ENRICHMENT: <fill if ENRICHMENT_NEEDED=NO>

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<paste the full normative Markdown text to be inserted into the Master Spec>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: <fill (example: Handshake_Master_Spec_v02.99.md 2.3.12.5 [CX-DBP-030])>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill exact string that must appear between start/end lines in SPEC_TARGET_RESOLVED>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste the relevant excerpt; ASCII-only; use \\uXXXX escapes when needed>
  ```

#### ANCHOR 2
- SPEC_ANCHOR: <fill>
- CONTEXT_START_LINE: <fill integer>
- CONTEXT_END_LINE: <fill integer>
- CONTEXT_TOKEN: <fill>
- EXCERPT_ASCII_ESCAPED:
  ```text
  <paste excerpt>
  ```
