# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Visual-Debugging-Loop-v1

## STUB_METADATA
- WP_ID: WP-1-Visual-Debugging-Loop-v1
- BASE_WP_ID: WP-1-Visual-Debugging-Loop
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Screenshot-Visual-Validation
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Visual validation loop for GUI-bearing WPs
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-110, Kimi K2.5)

## INTENT (DRAFT)
- What: Implement the generate-capture-compare-fix visual debugging loop for GUI WPs. After a coder commits GUI changes, the post-commit hook captures a screenshot, compares against the design baseline or previous screenshot, identifies visual discrepancies, and sends the visual diff to the validator alongside the code diff. The validator reviews both code and visual output. If visual mismatch exceeds threshold, the coder receives STEER with visual regression details.
- Why: Based on Kimi K2.5 visual debugging loop pattern. Code diffs alone cannot reveal visual regressions. Governed coders working on GUI WPs need automated visual feedback to catch layout breaks, styling mismatches, and rendering issues that pass code review but fail visual inspection. This loop closes the gap between code correctness and visual correctness.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Post-commit screenshot capture trigger for GUI-bearing WPs.
  - Visual comparison against baseline (pixel diff or structural comparison).
  - Visual diff artifact stored in governance artifact system.
  - Validator receives visual evidence alongside code diff.
  - Visual quality threshold configuration per WP.
  - Tauri app headless/test mode for automated screenshots.
- OUT_OF_SCOPE:
  - Screenshot capture tool itself (WP-1-Product-Screenshot-Visual-Validation-v1).
  - Design library or component registry.
  - Video recording or screen recording.

## ACCEPTANCE_CRITERIA (DRAFT)
- Post-commit hook triggers screenshot capture for WPs tagged as GUI-bearing.
- Visual comparison produces a diff artifact highlighting discrepancies against the baseline.
- Visual diff artifact is stored in the governance artifact system with WP and commit metadata.
- Validator session receives both code diff and visual diff for review.
- Visual mismatch exceeding the configured threshold triggers a STEER to the coder with regression details.
- Visual quality threshold is configurable per WP in the task packet or refinement.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Product-Screenshot-Visual-Validation for the screenshot capture tool and baseline infrastructure.
- Requires the Tauri app to support headless or test-mode rendering for automated captures.
- No spec blockers beyond the screenshot prerequisite.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Pixel-level comparison may produce false positives from anti-aliasing, font rendering differences, or minor layout shifts across runs.
- Risk: Headless/test-mode rendering in Tauri may differ from interactive rendering, causing phantom regressions.
- Risk: Visual threshold tuning may require significant iteration to balance sensitivity vs. noise.
- Unknown: Whether structural comparison (DOM-based) or pixel comparison is more reliable for detecting meaningful visual regressions in the Tauri + React stack.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Visual-Debugging-Loop-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Visual-Debugging-Loop-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
