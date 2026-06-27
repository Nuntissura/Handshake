# Coder Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: CODER

## Use

Use this brief after `just coder-startup`. It is operational memory for implementation lanes.

## Action Cards

### RAM-CODER-WORKTREE-001

- ACTION: WORKTREE_CONFINEMENT
- TRIGGER: before reading or editing files for a WP
- FAILURE_PATTERN: treating the Operator worktree, gov kernel, or `handshake_main` as a coder worktree
- DO: work only in the assigned WP worktree and branch from the packet/session assignment
- DO_NOT: edit `.GOV` through the junction except for coder-owned packet/MT status/evidence fields; never commit `.GOV` from the feature branch
- VERIFY: `just role-startup-topology-check` and `just phase-check STARTUP WP-{ID} CODER <session>` pass
- SOURCE: CODER_PROTOCOL

### RAM-CODER-BUILD_RULES_AUTHORITY-001

- ACTION: BUILD_RULES_REGISTRY_AUTHORITY
- TRIGGER: before implementing any MT that touches product code or product behavior (`src/`, `app/`, `tests/`, product runtime)
- FAILURE_PATTERN: implementing an MT against the packet text alone while ignoring the `HBR-*` acceptance-matrix rows the packet hydrated, so a build-time gate (interconnectivity, swarm safety, visual proof, quiet operation, manual currency, stop discipline, three-tier diagnostics) is left unproven at handoff
- DO: consult `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` and treat each applicable `HBR-*` row as a mandatory gate; for an MT touching observable runtime behavior, satisfy HBR-INT-009 THREE-TIER diagnostics (Tier 1 Flight Recorder business-event emit kept as-is; Tier 2 internal_diagnostics native INTERNAL self-diagnostics; Tier 3 Palmistry EXTERNAL watcher) and record each tier WIRED / NOT_APPLICABLE-with-reason / DEFERRED-with-reason in the MT evidence
- DO_NOT: claim an MT done while a required HBR row stays PENDING/STEER/BLOCKED; silently skip a diagnostic tier; auto-create a `HANDSHAKE_BUILD_RULES.md` projection (JSON is authority)
- VERIFY: each completed MT's handoff names the applicable HBR rows with PROVED / NOT_APPLICABLE / DEFERRED evidence
- SOURCE: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` (HBR-INT-009 + all HBR-*), CX-503B1, CX-006-VIS, CX-981

### RAM-CODER-VISUAL_DEBUG-001

- ACTION: PER_MT_VISUAL_CAPTURE
- TRIGGER: per MT, when the MT touches the native egui shell or any operator-observable UI surface
- FAILURE_PATTERN: handing off a UI-touching MT on green unit tests only, or using the LEGACY Tauri/WebView2/CDP path (`app/src-tauri/src/visual_debug.rs`, `Page.captureScreenshot`) which does not inspect the current native app
- DO: per MT, capture-and-compare through the NATIVE path in `../wtc-native-editors-v1/src/frontend/handshake_native/` — MCP tools `src/mcp/tools.rs` (`list_widgets` / `click_widget` / `set_value` / `screenshot`), the `egui_kittest` (`0.33`, `wgpu`) `Harness` render harness, and `src/mcp/screenshot.rs`; compare the rendered/AccessKit result against the MT's expected UI state before requesting review
- DO_NOT: use the Tauri CDP path; assume pixel screenshots work headless — `Harness::render()` readback can crash `0xc0000005` on headless-GPU, so run pixel screenshots on a real-GPU host and fall back to `list_widgets`/AccessKit-tree assertions on headless hosts
- VERIFY: the MT review request carries a per-MT visual capture or AccessKit snapshot as HBR-VIS evidence
- SOURCE: HANDSHAKE_BUILD_RULES.json HBR-VIS-001..005, native `src/mcp/tools.rs` + `src/mcp/screenshot.rs`, CX-006-VIS

### RAM-CODER-HANDOFF-001

- ACTION: HANDOFF
- TRIGGER: before claiming implementation complete
- FAILURE_PATTERN: reporting tests or summaries without phase-check handoff proof and spec evidence
- DO: run the packet TEST_PLAN, document evidence, drain required direct-review obligations, then run `just phase-check HANDOFF WP-{ID} CODER`
- DO_NOT: claim done while overlap review items or handoff gate blockers remain
- VERIFY: handoff phase check passes and `## STATUS_HANDOFF` contains required proof
- SOURCE: CODER_PROTOCOL
