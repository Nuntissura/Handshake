# Kernel Builder Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: KERNEL_BUILDER

## Use

Use this brief after `just kernel-builder-startup`. It is operational memory for the build-reset product-kernel lane.

## Action Cards

### RAM-KERNEL_BUILDER-BUILD_RESET-001

- ACTION: BUILD_RESET_FOCUS
- TRIGGER: before planning, packet authoring, or product implementation for Kernel V1
- FAILURE_PATTERN: spending the session repairing ACP or repo-governance drift instead of moving Handshake Kernel V1 product code forward
- DO: keep repo governance to the minimum needed for Task Board, Build Order, WPs, microtasks, restartability, and packet-declared validator handoff
- DO_NOT: patch governance workflow surfaces for polish, parity, or abstract correctness while product-kernel work can continue safely
- VERIFY: the next action either updates kernel-build artifacts or product code, or names a concrete safety/restartability blocker
- SOURCE: KERNEL_BUILDER_PROTOCOL, Operator reset instruction 2026-05-13

### RAM-KERNEL_BUILDER-WP_DETAIL-001

- ACTION: NO_CONTEXT_WP_DETAIL
- TRIGGER: when creating or updating a large kernel-build WP or microtask set
- FAILURE_PATTERN: creating a broad WP that depends on chat memory, implicit design intent, or unspecified code anchors
- DO: include current code anchors, exact scope, acceptance rows, MT goals, dependencies, tests, validator focus, risks, and out-of-scope notes
- DO_NOT: shrink the WP by deleting implementation-critical detail or compressing microtasks below restartable granularity
- VERIFY: a capable model with no chat context can implement each MT from the packet plus local repo reads
- SOURCE: KERNEL_BUILDER_PROTOCOL, Operator instruction 2026-05-13

### RAM-KERNEL_BUILDER-NO_VALIDATION-001

- ACTION: VALIDATION_BOUNDARY
- TRIGGER: after tests pass, self-checks pass, or product code appears complete
- FAILURE_PATTERN: presenting Kernel Builder self-checks as validator PASS/FAIL or merge readiness
- DO: record tests and self-checks as implementation evidence, then hand off to Integration Validator, Classic Validator, or the Operator-designated validator according to packet topology
- DO_NOT: issue final validation, merge approval, spec compliance verdict, or acceptance-row closure
- VERIFY: final language separates implementation evidence from validator authority
- SOURCE: KERNEL_BUILDER_PROTOCOL

### RAM-KERNEL_BUILDER-WORKTREE-001

- ACTION: PRODUCT_WORKTREE_DISCIPLINE
- TRIGGER: before touching `src/`, `app/`, `tests/`, or product runtime files
- FAILURE_PATTERN: editing product code from `wt-gov-kernel`, direct-main editing by habit, or touching `.GOV` through a WP junction
- DO: use a declared product worktree and branch for product edits; treat `../handshake_main` as reference/integration unless the Operator explicitly authorizes direct-main work
- DO_NOT: edit product code through the gov-kernel worktree or edit governance files through product worktree junctions
- VERIFY: `git status --short --branch` in the product worktree matches the intended branch before and after edits
- SOURCE: KERNEL_BUILDER_PROTOCOL, AGENTS.md worktree law

### RAM-KERNEL_BUILDER-SUBAGENT-001

- ACTION: SUBAGENT_OVERSIGHT
- TRIGGER: before using sub-agents during packet activation or product MT execution
- FAILURE_PATTERN: delegating implementation/review work without `KERNEL_BUILDER` review and acceptance of final delegated output
- DO: use read/write sub-agents where practical when packet rules or Operator instruction explicitly permit; review and verify all sub-agent outputs before advancing any claim, state, or handoff; sub-agents must not create or switch worktrees.
- DO_NOT: treat sub-agent output as finished truth; do not proceed to commit/state advance on unreviewed delegated changes; do not allow sub-agents to perform worktree creation
- VERIFY: a no-context Kernel Builder can identify the delegated action, the review performed, and remaining risks in repomem, receipts, and runtime state
- SOURCE: KERNEL_BUILDER_PROTOCOL

### RAM-KERNEL_BUILDER-PAPERWORK-001

- ACTION: FOLDED_IMPLEMENTATION_PAPERWORK
- TRIGGER: when implementing any ready-for-dev Kernel Builder WP
- FAILURE_PATTERN: product code advances while MT board, receipts, runtime status, repomem, packet projections, task-board/build-order truth, or validator handoff state stays stale
- DO: claim one unblocked MT, emit typed intent/claim, implement inside the declared product worktree, run proof or record blocker, update typed receipts/runtime/MT state, commit on the WP branch, push recovery checkpoints, and hand off through the packet-declared typed review surface
- DO_NOT: rely on chat memory, Markdown-only notes, unstaged local state, or narrative handoff as the source of restart truth
- VERIFY: a fresh no-context Kernel Builder can resume from packet JSON, MT contracts, runtime JSON, receipts JSONL, branch state, and repomem without reading this chat
- SOURCE: KERNEL_BUILDER_PROTOCOL Product Implementation Mode

### RAM-KERNEL_BUILDER-INTEGRATION_BATCH-001

- ACTION: INTEGRATION_BATCH_REVIEW_TOPOLOGY
- TRIGGER: when a folded Kernel Builder WP does not explicitly declare a WP Validator per-MT gate
- FAILURE_PATTERN: stopping each MT for WP Validator review or asking Integration Validator for product-code-vs-Master-Spec review before all MT evidence has been reviewed
- DO: build MT-by-MT, collect implementation evidence, hand off the full MT batch to Integration Validator, mitigate failed MTs, then wait for the scoped Master Spec review only after the MT batch passes
- DO_NOT: treat Kernel Builder self-checks as MT validation, require a WP Validator kickoff by default, or bypass Integration Validator final authority
- VERIFY: runtime status, receipts, and packet fields show `INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1` or the packet's explicit alternative
- SOURCE: KERNEL_BUILDER_PROTOCOL Validation Handoff Topology

### RAM-KERNEL_BUILDER-SPEC_RESOLVER-001

- ACTION: MODULE_RESOLVER_NOT_VIEWER
- TRIGGER: before reading indexed Master Spec modules, deriving roadmap order, or creating WPs/microtasks from spec content
- FAILURE_PATTERN: treating `.GOV/spec/indexed_spec/INDEX.json` as an operator surface, document viewer, or repo projection surface
- DO: resolve `.GOV/spec/SPEC_CURRENT.md` through `current_spec.entrypoint_path` and `current_spec.resolver_index_path`; use `INDEX.json` only as a machine-readable module resolver for tools and LLMs
- DO_NOT: create repo-local Markdown indexes, viewer files, summaries, or document projections unless explicitly requested in the current task
- VERIFY: roadmap items are used only for build order, while implementation intent and proof come from topical Master Spec modules, the reset brief, and product-code evidence
- SOURCE: KERNEL_BUILDER_PROTOCOL, RGF-315

### RAM-KERNEL_BUILDER-BUILD_RULES_AUTHORITY-001

- ACTION: BUILD_RULES_REGISTRY_AUTHORITY
- TRIGGER: before planning, packet authoring, or implementing any product-code WP/MT for Kernel V1
- FAILURE_PATTERN: authoring or building a product WP without consulting the `HBR-*` registry, so applicable build-time gates (interconnectivity, swarm safety, visual proof, quiet operation, manual currency, stop discipline, three-tier diagnostics) are missed in the packet and the acceptance matrix
- DO: consult `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` and treat every applicable `HBR-*` rule as a mandatory gate when authoring scope, MT plans, and acceptance rows; for any WP touching observable runtime behavior, plan HBR-INT-009 THREE-TIER diagnostics (Tier 1 Flight Recorder business-event ledger kept as-is; Tier 2 internal_diagnostics native INTERNAL self-diagnostics; Tier 3 Palmistry EXTERNAL out-of-process watcher) and record each tier WIRED / NOT_APPLICABLE-with-reason / DEFERRED-with-reason
- DO_NOT: treat HBR rules as optional, restate them as new authority, or auto-create a `HANDSHAKE_BUILD_RULES.md` projection (JSON is authority, markdown ON_DEMAND_ONLY)
- VERIFY: the packet/MT plan names each applicable HBR row and, for observable-runtime-behavior work, names the three-tier diagnostic outcome (DEFERRED-with-reason while WP-KERNEL-015/014 diagnostics are unshipped)
- SOURCE: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` (HBR-INT-009 + all HBR-*), CX-503B1, CX-006-VIS, CX-981

### RAM-KERNEL_BUILDER-VISUAL_DEBUG-001

- ACTION: WHOLE_WP_VISUAL_PASS
- TRIGGER: when a Kernel V1 WP touches the native egui shell or any operator-observable UI surface
- FAILURE_PATTERN: certifying a UI-touching WP from unit tests / process exit codes alone, or reaching for the LEGACY Tauri/WebView2/CDP path (`app/src-tauri/src/visual_debug.rs`, `Page.captureScreenshot`) which does not inspect the current native app
- DO: drive a whole-WP visual pass through Argus. Until the dedicated Rust-native Argus command exists, the Argus-compatible path is the NATIVE path in `../wtc-native-editors-v1/src/frontend/handshake_native/` — the MCP tool surface `src/mcp/tools.rs` (`list_widgets` / `click_widget` / `set_value` / `screenshot`), the `egui_kittest` (`0.33`, `wgpu`) `Harness` render harness, and `src/mcp/screenshot.rs` capture; inspect the rendered result, not just green tests
- DO_NOT: use the Tauri CDP path; use foreground desktop automation; assume pixel screenshots work on a headless host — `Harness::render()` readback can crash `0xc0000005` on headless-GPU, so run pixel screenshots on a real-GPU host and fall back to `list_widgets`/AccessKit-tree assertions on headless hosts
- VERIFY: Argus evidence exists for each touched UI surface and records stable targets, layout/state observations, before/after state when steering occurred, and any HBR-VIS gap/remediation
- SOURCE: HANDSHAKE_BUILD_RULES.json HBR-VIS-001..005, `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`, native `src/mcp/tools.rs` + `src/mcp/screenshot.rs`, CX-503D1, CX-006-VIS

### RAM-KERNEL_BUILDER-MACHINE_ARTIFACTS-001

- ACTION: MACHINE_CONTRACT_FIRST
- TRIGGER: before creating or updating WPs, refinements, microtasks, task-state records, handoffs, or validation/dossier artifacts
- FAILURE_PATTERN: copying legacy Markdown packets/refinements/microtasks into future work as if they are the intended authority surface
- DO: author or update typed JSON/JSONL/YAML-compatible contracts first, then generate Markdown only as an explicit projection or compatibility bridge
- DO_NOT: create new model-authored Markdown as authority, or treat old Markdown-heavy artifacts as anything more than migration safety rails
- VERIFY: the durable artifact has a machine-readable authority contract, stable IDs, projection metadata when Markdown exists, and `SAFETY_NET_ONLY_DO_NOT_COPY_FORWARD` semantics for legacy Markdown
- SOURCE: CX-914, KERNEL_BUILDER_PROTOCOL, Operator instruction 2026-05-13
