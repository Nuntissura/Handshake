# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-Screenshot-Visual-Validation-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Screenshot-Visual-Validation-v1
- BASE_WP_ID: WP-1-Product-Screenshot-Visual-Validation
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: NONE
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Visual validation and design library foundation for Dev Command Center and all GUI-bearing WPs
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
  - Handshake_Master_Spec_v02.179.md 1.3 The Four-Layer Architecture (Validation Layer)

## INTENT (DRAFT)
- What: Build a product-integrated screenshot capture tool that programmatically captures the full Handshake app window, individual panels, and module-level views. Expose screenshots to governed coder and validator sessions so they can visually verify UI coherence during implementation and review.
- Why: The current refinement process forces UI/GUI thinking per WP (UI_UX_RUBRIC, GUI_IMPLEMENTATION_ADVICE), but coders and validators operate on code diffs, not visual output. Without a screenshot tool, visual coherence checks are impossible during governed autonomous work. This tool is also a prerequisite for a future Handshake design library and component registry.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Programmatic screenshot capture of the full Tauri app window.
  - Panel-level capture (individual panels, sidebars, modals).
  - Module-level capture (specific UI modules or component regions).
  - Screenshot storage in the governed artifact system (`.handshake/artifacts/` or equivalent).
  - CLI or API surface for triggering captures from governed coder/validator sessions.
  - Screenshot metadata (timestamp, window title, panel id, dimensions, capture trigger).
  - Integration with Tauri's webview screenshot or native window capture APIs.
- OUT_OF_SCOPE:
  - Design library or component registry (downstream WP).
  - Visual diffing or automated visual regression testing (downstream WP).
  - UI design guidelines or style guide creation (downstream WP, informed by screenshot captures).
  - Video recording or screen recording.

## ACCEPTANCE_CRITERIA (DRAFT)
- A governed coder session can trigger a full-app screenshot capture via CLI command or API call.
- A governed validator session can capture and inspect individual panel screenshots.
- Screenshots are stored with metadata in the artifact system.
- Capture works on the current Tauri + React frontend stack.
- The tool is usable from both cloud model sessions (via ACP broker CLI commands) and future local model sessions.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the Tauri app being runnable (existing product runtime).
- No governance or spec dependencies.
- Blocked by nothing; can be activated independently.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Tauri webview screenshot APIs may not expose panel-level granularity natively; may need DOM-based capture via html2canvas or similar.
- Risk: Screenshot capture during test runs may require a headless or mock UI environment.
- Risk: Large screenshot files may strain the artifact storage budget; need retention policy.
- Unknown: Whether Tauri's native screenshot API or the webview's DOM-based capture is more reliable for panel-level captures on Windows 11.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Screenshot-Visual-Validation-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Screenshot-Visual-Validation-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
