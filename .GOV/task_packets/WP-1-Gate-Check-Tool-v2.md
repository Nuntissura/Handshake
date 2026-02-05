# Task Packet: WP-1-Gate-Check-Tool-v2

## METADATA
- TASK_ID: WP-1-Gate-Check-Tool-v2
- WP_ID: WP-1-Gate-Check-Tool-v2
- DATE: 2025-12-31T18:17:50.968Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja311220251916

## User Context (Non-Technical Explainer)
This work packet fixes the "workflow traffic light" that prevents people (and AI agents) from skipping required steps (BOOTSTRAP -> SKELETON -> approval -> implementation). Right now, that traffic light can misread a packet and block work even when the packet is correct. The goal is to make the gate-check tool strict and reliable so it blocks real protocol violations, but does not create false failures.

## SCOPE
- What: Harden `.GOV/scripts/validation/gate-check.mjs` so it only recognizes explicit phase markers (headings / dedicated marker lines) and ignores prose or fenced code blocks, preventing false-positive gate failures.
- Why: The gate-check tool is wired into `just pre-work` and `just post-work`. False positives cascade into many WPs failing the process gates, stalling Phase 1 remediation.
- IN_SCOPE_PATHS:
  - .GOV/scripts/validation/gate-check.mjs
- OUT_OF_SCOPE:
  - Any changes to other scripts in .GOV/scripts/validation/
  - Any changes to src/, app/, tests/
  - Any changes to .GOV/roles_shared/TASK_BOARD.md (coordination rule for two-coder parallel work: Orchestrator updates the board to avoid file overlap)
  - Any changes to existing task packets (create a new variant if needed)

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Gate-Check-Tool-v2

# Smoke checks (expected PASS):
node .GOV/scripts/validation/gate-check.mjs WP-1-Gate-Check-Tool-v2
node .GOV/scripts/validation/gate-check.mjs WP-1-Workflow-Engine-v4

# Negative check (expected FAIL: implementation evidence without a prior SKELETON APPROVED marker):
node .GOV/scripts/validation/gate-check.mjs WP-1-Tokenization-Service-20251228

just cargo-clean
just post-work WP-1-Gate-Check-Tool-v2
```

### DONE_MEANS
- `.GOV/scripts/validation/gate-check.mjs` no longer matches phase markers inside prose paragraphs or inside fenced code blocks.
- gate-check recognizes BOOTSTRAP and SKELETON only via explicit Markdown headings (e.g., `## BOOTSTRAP`, `## SKELETON`), not via mentions in text.
- gate-check recognizes SKELETON approval only via a dedicated marker line starting with `SKELETON APPROVED` (line start), not via mentions in text.
- The smoke checks in TEST_PLAN behave as expected (PASS/PASS/FAIL).
- `just post-work WP-1-Gate-Check-Tool-v2` passes.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.99.md (recorded_at: 2025-12-31T18:17:50.968Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: A2.7.5 (Validation Gates from Diary COR-701); A2.9 (Deterministic Edit Process COR-701)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.99.md (A2.7.5, A2.9)
  - .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md
  - .GOV/roles/coder/CODER_PROTOCOL.md
  - .GOV/roles/validator/VALIDATOR_PROTOCOL.md
  - .GOV/scripts/validation/gate-check.mjs
  - .GOV/scripts/validation/pre-work-check.mjs
  - .GOV/scripts/validation/post-work-check.mjs
  - .GOV/scripts/validation/cor701-spec.json
- SEARCH_TERMS:
  - "APPROVAL_RE"
  - "SKELETON APPROVED"
  - "implementationStarted"
  - "BOOTSTRAP"
  - "SKELETON"
  - "VALIDATION (Coder) heading"
  - "VALIDATION REPORT heading"
  - "```"
- RUN_COMMANDS:
  ```bash
  node .GOV/scripts/validation/gate-check.mjs WP-1-Gate-Check-Tool-v2
  node .GOV/scripts/validation/gate-check.mjs WP-1-Workflow-Engine-v4
  node .GOV/scripts/validation/gate-check.mjs WP-1-Tokenization-Service-20251228
  ```
- RISK_MAP:
  - "False positives block valid work" -> "workflow throughput and deterministic enforcement"
  - "False negatives allow phase skipping" -> "governance compliance failure"
  - "Regex matches prose/code fences" -> "gate-check misclassification"
  - "Breaking gate-check blocks all pre-work/post-work" -> "repo-wide workflow halt"

## SKELETON
- Proposed interfaces/types/contracts:
- A line-based parser that:
  - Tracks whether we are inside a fenced code block and ignores marker-like strings inside fences.
  - Detects phase headings by matching heading lines only.
  - Detects approval marker only from a dedicated marker line starting with `SKELETON APPROVED`.
- Clear error messages that state which required marker is missing or out of order.
- Open questions:
- None.
- Notes:
- Keep behavior compatible with existing packets that already follow the protocol, and avoid introducing new required markers.

SKELETON APPROVED

## IMPLEMENTATION
- Hardened `.GOV/scripts/validation/gate-check.mjs` by switching to line-based parsing that ignores marker-like strings inside fenced code blocks and only recognizes explicit heading lines / marker lines per this packet's DONE_MEANS.

## HYGIENE
- node .GOV/scripts/validation/gate-check.mjs WP-1-Gate-Check-Tool-v2: PASS
- node .GOV/scripts/validation/gate-check.mjs WP-1-Workflow-Engine-v4: PASS
- node .GOV/scripts/validation/gate-check.mjs WP-1-Tokenization-Service-20251228: FAIL (expected negative check)
- just post-work WP-1-Gate-Check-Tool-v2: PASS (after manifest filled; warnings possible if git subprocess checks unavailable)

## VALIDATION
- Target File: `.GOV/scripts/validation/gate-check.mjs`
- Start: 6
- End: 133
- Line Delta: 59
- Pre-SHA1: `49a004580717da35154158d26f912096baf422dc`
- Post-SHA1: `838da666bd64cb98d6299c9a3eea02f6951181fa`
- Gates Passed:
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
- Lint Results: N/A (node script change; smoke checks PASS/PASS/FAIL per TEST_PLAN)
- Artifacts: None
- Timestamp: 2025-12-31
- Operator: claude-code (Coder) + codex-cli (Validator)
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md
- Notes: Pre/Post SHA1 and line window/delta computed from `git show HEAD:` + `git diff` for COR-701 post-work-check.

## STATUS_HANDOFF
- Current WP_STATUS: Done
- What changed in this update: Implemented hardened marker parsing for gate-check and executed smoke checks and deterministic post-work gate.
- Next step / handoff hint: Validator appended PASS report and updated TASK_BOARD to Done (VALIDATED).

---

**Last Updated:** 2025-12-31
**User Signature Locked:** ilja311220251916

IMPORTANT: This packet is locked. No edits allowed.
If changes needed: Create a new packet (WP-1-Gate-Check-Tool-v3). Do not edit this one.

---

## VALIDATION REPORT - 2025-12-31 (Validator)
Verdict: PASS

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Gate-Check-Tool-v2.md` (**Status:** Done)
- Spec (SPEC_CURRENT): `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.99.md` (A2.7.5, A2.9)
- Codex: `Handshake Codex v1.4.md`
- Protocol: `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

Files Checked:
- `.GOV/scripts/validation/gate-check.mjs`
- `.GOV/task_packets/WP-1-Gate-Check-Tool-v2.md`
- `.GOV/roles_shared/TASK_BOARD.md`

Findings:
- Marker detection now ignores fenced code blocks and only recognizes explicit heading lines / the dedicated `SKELETON APPROVED` marker line outside fences, preventing false-positive gate failures from prose/code fences.
- Smoke checks match TEST_PLAN expectations: PASS for `WP-1-Gate-Check-Tool-v2` and `WP-1-Workflow-Engine-v4`; FAIL for `WP-1-Tokenization-Service-20251228` (expected negative check: implementation evidence without approval).

Process Gates:
- `node .GOV/scripts/validation/gate-check.mjs WP-1-Gate-Check-Tool-v2`: PASS
- `node .GOV/scripts/validation/gate-check.mjs WP-1-Workflow-Engine-v4`: PASS
- `node .GOV/scripts/validation/gate-check.mjs WP-1-Tokenization-Service-20251228`: FAIL (expected)
- `just post-work WP-1-Gate-Check-Tool-v2`: PASS (warnings possible if git subprocess checks unavailable in this environment)

REASON FOR PASS:
- DONE_MEANS satisfied: gate-check no longer matches markers inside prose/fenced code blocks, and the TEST_PLAN smoke checks behave as expected.

