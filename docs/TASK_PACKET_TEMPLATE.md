# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

## Header
- TASK_ID:
- TITLE:
- REQUESTOR:
- DATE:
- AGENT_ID / ROLE:
- STATUS: Ready-for-Dev | In-Progress | Done

## Goal
- Scope:
- Expected behavior:
- In-scope paths:
- Out-of-scope:

## Quality gate
- RISK_TIER (LOW/MEDIUM/HIGH):
- TEST_PLAN (commands + manual steps, or "None" with reason):
- DONE_MEANS:
- ROLLBACK_HINT:
- SCAFFOLD_USED (yes/no + reason):
- SCAFFOLD_WAIVER_APPROVED_BY (if waived):
- AI_REVIEW (required for MEDIUM/HIGH; attach `ai_review.md` from local `gemini` CLI to this packet):

## Authority
- SPEC_CURRENT:
- Codex:
- Task Board: docs/TASK_BOARD.md
- Logger (optional for milestones/hard bugs):
- ADRs (if relevant):

## Bootstrap
- FILES_TO_OPEN:
- SEARCH_TERMS:
- RUN_COMMANDS:
- RISK_MAP:

## Notes
- Assumptions (if any):
- Open questions (if any):

## Validation
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
  - [ ] compilation_clean
  - [ ] tests_passed
  - [ ] outside_window_pristine
  - [ ] lint_passed
  - [ ] ai_review (if required)
  - [ ] task_board_updated
  - [ ] commit_ready
  - [ ] other:
- **Lint Results**: <suite + pass/fail summary>
- **Artifacts**: <paths if any>
- **Timestamp**:
- **Operator**:
- **Notes**:

## Status / Handoff
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:
