# WP Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: WP_VALIDATOR

## Use

Use this brief after `just validator-startup WP_VALIDATOR`. It is operational memory for per-WP review.

## Action Cards

### RAM-WP_VALIDATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before steering Coder, responding to a handoff, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: reviewing stale route prose or asking Orchestrator to relay when receipts, notifications, runtime status, or phase checks already identify the next validator action
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, receipt response, or typed helper before writing review prose
- DO_NOT: manually relay ordinary review content when `wp-validator-response`, `wp-review-response`, `wp-spec-gap`, notification ack, or `phase-check` owns the state transition
- VERIFY: the validator response preserves the original correlation, cites packet/runtime authority, and names the deterministic helper used
- SOURCE: CX-218K, WP_VALIDATOR_PROTOCOL, .GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md

### RAM-WP_VALIDATOR-EARLY_REVIEW-001

- ACTION: EARLY_REVIEW
- TRIGGER: coder publishes bootstrap, skeleton, intent, or a microtask review request
- FAILURE_PATTERN: waiting until full handoff before challenging scope, data shape, or spec drift
- DO: use kickoff/intent/review receipts to steer early and keep unresolved overlap review bounded
- DO_NOT: approve based only on coder self-report or passing tests
- VERIFY: pending direct-review receipts are drained or explicitly blocked before final coder handoff
- SOURCE: WP_VALIDATOR_PROTOCOL

### RAM-WP_VALIDATOR-BUILD_RULES_AUTHORITY-001

- ACTION: BUILD_RULES_REGISTRY_AUTHORITY
- TRIGGER: before reviewing any MT that touches product code or product behavior, and before any per-MT PASS
- FAILURE_PATTERN: passing an MT on correctness/scope alone while the packet's hydrated `HBR-*` acceptance rows stay PENDING/STEER/BLOCKED, or accepting a missing three-tier diagnostic consideration without a recorded reason
- DO: consult `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` and enforce each applicable `HBR-*` row as a mandatory gate; for an MT touching observable runtime behavior, require the HBR-INT-009 THREE-TIER outcome on record (Tier 1 Flight Recorder business-event ledger kept as-is; Tier 2 internal_diagnostics native INTERNAL self-diagnostics; Tier 3 Palmistry EXTERNAL watcher), each marked WIRED / NOT_APPLICABLE-with-reason / DEFERRED-with-reason
- DO_NOT: issue per-MT PASS while a required HBR row is PENDING/STEER/BLOCKED [CX-503B1]; accept a silently skipped diagnostic tier; treat green tests as HBR-VIS satisfaction
- VERIFY: the per-MT review receipt cites each applicable HBR row as PROVED / NOT_APPLICABLE / DEFERRED with the coder's evidence
- SOURCE: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` (HBR-INT-009 + all HBR-*), CX-503B1, CX-006-VIS, CX-981

### RAM-WP_VALIDATOR-VISUAL_DEBUG-001

- ACTION: PER_MT_VISUAL_CAPTURE_COMPARE
- TRIGGER: per MT, when reviewing an MT that touches the native egui shell or any operator-observable UI surface
- FAILURE_PATTERN: accepting the coder's self-report or green tests as UI proof, or expecting evidence from the LEGACY Tauri/WebView2/CDP path (`app/src-tauri/src/visual_debug.rs`, `Page.captureScreenshot`) which does not inspect the current native app
- DO: independently capture-and-compare per MT through the NATIVE path in `../wtc-native-editors-v1/src/frontend/handshake_native/` — MCP tools `src/mcp/tools.rs` (`list_widgets` / `click_widget` / `set_value` / `screenshot`), the `egui_kittest` (`0.33`, `wgpu`) `Harness` render harness, and `src/mcp/screenshot.rs`; compare the rendered/AccessKit result against the MT's expected UI state
- DO_NOT: accept Tauri CDP evidence for the native app; assume pixel screenshots work headless — `Harness::render()` readback can crash `0xc0000005` on headless-GPU, so require pixel screenshots from a real-GPU host and accept `list_widgets`/AccessKit-tree assertions as the headless fallback
- VERIFY: the per-MT review receipt records an independent visual capture or AccessKit snapshot compared against expected UI state as HBR-VIS evidence
- SOURCE: HANDSHAKE_BUILD_RULES.json HBR-VIS-001..005, native `src/mcp/tools.rs` + `src/mcp/screenshot.rs`, CX-006-VIS

### RAM-WP_VALIDATOR-SCOPE-001

- ACTION: SCOPE_CONTAINMENT
- TRIGGER: reviewing a microtask or repair
- FAILURE_PATTERN: allowing adjacent shared-surface or out-of-scope fixes to enter the WP without packet authority
- DO: validate against signed scope, declared MT contract, and current diff against main
- DO_NOT: widen the packet by review convenience
- VERIFY: review receipt names in-scope file evidence or blocks with concrete scope reason
- SOURCE: WP_VALIDATOR_PROTOCOL
