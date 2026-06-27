# Shared Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- APPLIES_TO: ALL_ROLES
- CX-218L_BOUNDARY: governance paperwork/workflow stabilization applies to non-CODER roles so `ORCHESTRATOR_MANAGED` workflows become more mechanical where current governance/workflow is still brittle; Coder treats governance drift as report-only blocker context and stays focused on product code

## Use

Read this as operational memory at startup. It shortens the path from repeated failures to correct action. It does not override protocols, signed packets, runtime truth, or Codex law.

## Action Cards

### RAM-SHARED-MECHANICAL_INTERVENTION-001

- ACTION: CX-218K_MECHANICAL_INTERVENTION
- TRIGGER: any stall, handoff delay, relay miss, documentation/protocol drift, ACP/session ambiguity, or repair/steer decision
- FAILURE_PATTERN: patching, steering, relaying, or declaring blocked state after reading only one symptom
- DO: classify 3-5 plausible causes first, including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift; then use the cheapest deterministic read or typed helper, and for non-Coder roles actively strive to turn repeated brittle governance/workflow friction into mechanical surfaces
- DO_NOT: compensate with narrative relay, repeated broad rereads, or another prompt when packet/runtime/receipt truth can answer the next action; do not assign governance-paperwork stabilization to Coder
- VERIFY: the chosen repair names the cause class and either updates the mechanical surface, writes a typed receipt, or records why no patch is needed
- SOURCE: CX-218K, CX-218L, CX-218M, `.GOV/roles_shared/workflow_contracts/orchestrator_managed.workflow.json`, `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md`

### RAM-SHARED-PATHING-001

- ACTION: PATHING
- TRIGGER: reading or writing governance files from any worktree
- FAILURE_PATTERN: treating a role worktree `.GOV` junction or `handshake_main/.GOV` as an isolated copy of governance truth
- DO: resolve live governance through `wt-gov-kernel/.GOV` or `HANDSHAKE_GOV_ROOT`; use repo-relative paths in docs and diagnostics
- DO_NOT: write host-specific absolute paths or assume `handshake_main/.GOV` is authoritative when the kernel root is active
- VERIFY: `just role-startup-topology-check` passes before governed role work
- SOURCE: GOV-CHANGE-20260429-03

### RAM-SHARED-POWERSHELL-001

- ACTION: TOOLCALLING
- TRIGGER: scanning malformed Unicode, command examples, or regex alternation from PowerShell
- FAILURE_PATTERN: inline regexes containing mojibake, pipes, or replacement characters get parsed by PowerShell or Node before the intended scan runs
- DO: use char-code predicates, `Select-String` variables, or simple literal scans; for `rg`, pass `--` before risky patterns
- DO_NOT: paste malformed glyphs directly into inline regex alternation
- VERIFY: the scan exits 0 and reports explicit findings or `no active mojibake markers found`
- SOURCE: TG-017

### RAM-SHARED-FAILCAPTURE-001

- ACTION: GOVERNANCE_SCRIPTING
- TRIGGER: creating or modifying a governance script/check
- FAILURE_PATTERN: scripts exit through `process.exit(1)` or local `fail()` without writing procedural memory
- DO: import `registerFailCaptureHook` and `failWithMemory`, call `registerFailCaptureHook("filename.mjs", { role: "ROLE" })`, and delegate hard failures to `failWithMemory`
- DO_NOT: create standalone `console.error(...); process.exit(1)` failure paths
- VERIFY: `node --test .GOV/roles_shared/tests/fail-capture-lib.test.mjs` passes
- SOURCE: TG-007, GOV-CHANGE-20260429-03

### RAM-SHARED-AUTHORITY-001

- ACTION: AUTHORITY
- TRIGGER: unsure which surface to trust during a governed run
- FAILURE_PATTERN: rereading broad protocols, task boards, runtime ledgers, and dossiers instead of using the compact role-specific digest
- DO: use the role's startup output, active packet, notifications/thread, and compact digest command before broad rediscovery
- DO_NOT: pay repeated read amplification silently; report ambiguity when the compact digest conflicts with live truth
- VERIFY: the selected digest command prints the active route/context without needing broad `just --list` rediscovery
- SOURCE: RGF-255, RGF-253

### RAM-SHARED-BUILD_RULES_AUTHORITY-001

- ACTION: BUILD_RULES_REGISTRY_AUTHORITY
- TRIGGER: at startup, and before planning, authoring, implementing, or reviewing any WP that touches product code or product behavior (`src/`, `app/`, `tests/`, product runtime)
- FAILURE_PATTERN: treating `HBR-*` build rules as optional, advisory, or already-satisfied; skipping the three-tier diagnostic consideration (HBR-INT-009) on observable-runtime-behavior WPs
- DO: read and ACKNOWLEDGE `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` AS AUTHORITATIVE at startup; treat every applicable `HBR-*` rule as a mandatory build-time/handoff-time gate that auto-emits `PACKET_ACCEPTANCE_MATRIX` rows (PROVED / NOT_APPLICABLE-with-reason / BLOCKED-with-cause). For any WP touching observable runtime behavior, evaluate HBR-INT-009 THREE-TIER diagnostics — Flight Recorder (kept-as-is business-event ledger), internal_diagnostics (native INTERNAL self-diagnostics), Palmistry (EXTERNAL out-of-process watcher) — and record each tier as WIRED / NOT_APPLICABLE-with-reason / DEFERRED-with-reason
- DO_NOT: close a WP to PASS while any required HBR row is PENDING/STEER/BLOCKED [CX-503B1]; silently skip a diagnostic tier; auto-create a `HANDSHAKE_BUILD_RULES.md` projection (JSON is authority, markdown is ON_DEMAND_ONLY)
- VERIFY: startup output reflects build-rule acknowledgment, and the next product-code action either proves an applicable HBR row or records its NOT_APPLICABLE/DEFERRED reason
- SOURCE: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json` (HBR-INT-009 + all HBR-* pillars), CX-503B1, CX-006-VIS, CX-981

### RAM-SHARED-STARTUP_TIMEOUT-001

- ACTION: TOOLCALLING
- TRIGGER: running a role `*-startup` command from Codex shell tooling
- FAILURE_PATTERN: using the default shell timeout for startup and truncating authority, checks, memory refresh, or resume recall before the role is ready
- DO: set the shell timeout to at least `600000` ms / 10 minutes for role startup commands; if startup still times out under host load, capture the timeout as procedural memory and rerun with an increased timeout before acting on partial context
- DO_NOT: treat a shell timeout during startup as completed startup or begin governed work from incomplete startup output
- VERIFY: startup reaches its normal checkpoint/resume hint output and any required hard gates have completed or produced explicit failures
- SOURCE: memory-capture #5897, Operator correction 2026-05-03
