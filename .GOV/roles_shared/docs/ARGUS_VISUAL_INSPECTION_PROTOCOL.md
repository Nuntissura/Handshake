# Argus Visual Inspection Protocol

ARGUS-001: Argus is the named Handshake-native visual inspection and GUI steering capability for model and operator validation.

ARGUS-002: Every role that implements, reviews, validates, hydrates, launches, or closes product work that touches a GUI, operator surface, diagnostic surface, frontend navigation, layout, style, tab, panel, button, input, or visible state must require Argus evidence before claiming completion or PASS.

ARGUS-003: Argus verification must inspect the actual rendered or AccessKit-visible frontend state against the expected behavior; ordinary unit tests, process exit codes, narrative claims, or uninspected screenshots are not enough.

ARGUS-004: The canonical product-facing Argus methods are `argus.inspect`, `argus.click`, `argus.set_value`, and `argus.screenshot`, implemented in the native MCP/AccessKit stack under `src/mcp/argus.rs`, `src/mcp/tools.rs`, `src/mcp/session.rs`, `src/mcp/server.rs`, and `src/mcp/screenshot.rs`. The older MCP primitive names `list_widgets`, `click_widget`, `set_value`, and `screenshot` remain compatibility aliases only.

ARGUS-005: Argus must be headless and non-intrusive by default. It must not bring Handshake to the foreground, must not steal focus, must not hijack keyboard input, must not move or click the OS mouse, and must not use attention-stealing desktop APIs.

ARGUS-006: Foreground desktop interaction is not Argus. If a manual foreground step is genuinely unavoidable, surface that as an exception before running it and do not count it as the normal Argus path.

ARGUS-007: Argus must support parallel agents through stable `author_id` targeting, read-only shared snapshots, explicit leases or receipts for mutating actions, attributed actions, request-level `agent_label` when clients share one binding token, and fresh post-action snapshots.

ARGUS-008: Argus evidence for UI work must cover at least: reachable navigation, visible panels/tabs/buttons, stable `author_id` values for actionable controls, important state visibility, text overlap, bounds/layout coherence, disabled/error/loading/empty states when applicable, and screenshot or AccessKit/tree evidence compared to the expected UI state.

ARGUS-009: If Argus cannot see an in-scope frontend surface, cannot identify it with a stable target, cannot steer it safely, or cannot observe the result after steering, that is HBR-VIS technical debt.

ARGUS-010: HBR-VIS technical debt from ARGUS-009 is an allowed scope increase for the current MT/WP when the missing Argus visibility or steering path blocks proof of the implemented surface. Remediate the minimum needed inspection/steering hook, stable ID, snapshot state, or headless capture path before claiming the original work is complete.

ARGUS-011: If remediation cannot be completed in the current role authority, record a blocking HBR-VIS gap with the exact surface, missing Argus capability, affected proof, and recommended remediation. Do not issue PASS or DONE for that surface.

ARGUS-012: Argus evidence must be recorded in review/handoff artifacts with the tool/path used, snapshot or screenshot reference, target `author_id` values, action sequence when steering occurred, before/after observation, and any unremediated gaps.

ARGUS-013: Observable Argus behavior must record the HBR-INT-009 three-tier diagnostics posture in implementation evidence and the internal UserManual: Tier 1 Flight Recorder/EventLedger, Tier 2 internal_diagnostics, and Tier 3 Palmistry must each be WIRED, NOT_APPLICABLE-with-reason, or DEFERRED-with-reason. Current MT-007 Argus writes native MCP ActionLog receipts, not durable EventLedger persistence; do not claim Flight Recorder/EventLedger wiring until that mirroring exists.
