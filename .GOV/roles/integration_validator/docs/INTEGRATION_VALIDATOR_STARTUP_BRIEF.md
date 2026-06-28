# Integration Validator Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: INTEGRATION_VALIDATOR

## Use

Use this brief after `just validator-startup INTEGRATION_VALIDATOR`. It is operational memory for final-lane judgment.

## Action Cards

### RAM-INTEGRATION_VALIDATOR-MECHANICAL_INTERVENTION-001

- ACTION: MECHANICAL_INTERVENTION
- TRIGGER: before final verdict, merge containment, status sync, declaring a stall, or treating handoff/documentation/protocol drift as blocked
- FAILURE_PATTERN: re-deriving final-lane truth manually or running terminal closeout before resolving the open final handoff correlation
- DO: classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read, phase check, merge-containment proof, or typed helper before mutating verdict/main truth
- DO_NOT: manually relay ordinary final review content when `phase-check VERDICT`, `wp-review-response`, contained-main closeout, or integration-validator context helpers own the state transition
- VERIFY: final action cites the cause class, helper output, original handoff correlation, packet target head, and current main containment evidence
- SOURCE: CX-218K, INTEGRATION_VALIDATOR_PROTOCOL, .GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md

### RAM-INTEGRATION_VALIDATOR-CLOSEOUT-001

- ACTION: CLOSEOUT
- TRIGGER: before final verdict, merge, or sync-gov-to-main
- FAILURE_PATTERN: rebuilding packet/runtime/main compatibility truth manually or trusting prior role summaries
- DO: run startup, `validator-next`, and `just integration-validator-context-brief WP-{ID}` before broad repo search
- DO_NOT: use `handshake_main/.GOV` as live governance authority when `HANDSHAKE_GOV_ROOT` points to the kernel
- VERIFY: context brief prints packet path, prepare worktree, main compatibility, and closeout blockers
- SOURCE: INTEGRATION_VALIDATOR_PROTOCOL, GOV-CHANGE-20260429-03

### RAM-INTEGRATION_VALIDATOR-BUILD_RULES_AUTHORITY-001

- ACTION: BUILD_RULES_REGISTRY_AUTHORITY
- TRIGGER: before whole-WP judgment and before writing PASS for any WP that touched product code or product behavior
- FAILURE_PATTERN: writing PASS while a required `HBR-*` acceptance-matrix row stays PENDING/STEER/BLOCKED, or letting an observable-runtime-behavior WP close without an HBR-INT-009 three-tier diagnostic outcome on record
- DO: consult `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` and treat every applicable `HBR-*` row as a mandatory closure gate; for any WP touching observable runtime behavior, require the HBR-INT-009 THREE-TIER outcome (Tier 1 Flight Recorder business-event ledger kept as-is; Tier 2 internal_diagnostics native INTERNAL self-diagnostics; Tier 3 Palmistry EXTERNAL watcher) recorded as WIRED / NOT_APPLICABLE-with-reason / DEFERRED-with-reason before PASS
- DO_NOT: write PASS while any required HBR row is PENDING/STEER/BLOCKED [CX-503B1]; accept a silently skipped diagnostic tier; rewrite or weaken an HBR gate to manufacture PASS
- VERIFY: the verdict report maps each applicable HBR row to PROVED / NOT_APPLICABLE / DEFERRED evidence at the whole-WP level
- SOURCE: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` (HBR-INT-009 + all HBR-*), CX-503B1, CX-006-VIS, CX-981

### RAM-INTEGRATION_VALIDATOR-VISUAL_DEBUG-001

- ACTION: WHOLE_WP_VISUAL_PASS
- TRIGGER: before whole-WP PASS, when the WP touched the native egui shell or any operator-observable UI surface
- FAILURE_PATTERN: passing a UI-touching WP on per-MT receipts and green tests alone, or expecting visual evidence from the LEGACY Tauri/WebView2/CDP path (`app/src-tauri/src/visual_debug.rs`, `Page.captureScreenshot`) which does not inspect the current native app
- DO: run an independent whole-WP Argus pass. Until the dedicated Rust-native Argus command exists, the Argus-compatible path is the NATIVE path in `../wtc-native-editors-v1/src/frontend/handshake_native/` — MCP tools `src/mcp/tools.rs` (`list_widgets` / `click_widget` / `set_value` / `screenshot`), the `egui_kittest` (`0.33`, `wgpu`) `Harness` render harness, and `src/mcp/screenshot.rs`; inspect the rendered result across the WP's touched surfaces, not just per-MT green
- DO_NOT: accept Tauri CDP evidence for the native app; accept foreground desktop automation as Argus; assume pixel screenshots work headless — `Harness::render()` readback can crash `0xc0000005` on headless-GPU, so require pixel screenshots from a real-GPU host and accept `list_widgets`/AccessKit-tree assertions as the headless fallback; if Argus cannot see, identify, steer, or re-observe a UI surface, require remediation or record an HBR-VIS-005 blocking verification gap rather than certifying
- VERIFY: the verdict report records whole-WP Argus evidence per touched UI surface, including target `author_id` values, before/after state when steering occurred, and any HBR-VIS remediation or blocker
- SOURCE: HANDSHAKE_BUILD_RULES.json HBR-VIS-001..005, `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`, native `src/mcp/tools.rs` + `src/mcp/screenshot.rs`, CX-503D1, CX-006-VIS

### RAM-INTEGRATION_VALIDATOR-VERDICT-001

- ACTION: VERDICT
- TRIGGER: writing PASS, FAIL, merge, or status-sync truth
- FAILURE_PATTERN: treating WP Validator evidence or green tests as final authority
- DO: perform independent whole-WP review with spec clause map and file:line evidence
- DO_NOT: pass with debt for hard invariants, security, traceability, or spec alignment
- VERIFY: verdict report includes direct evidence and any remediation instructions are packet-visible
- SOURCE: INTEGRATION_VALIDATOR_PROTOCOL
