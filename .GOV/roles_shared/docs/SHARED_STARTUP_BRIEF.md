# Shared Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- APPLIES_TO: ALL_ROLES

## Use

Read this as operational memory at startup. It shortens the path from repeated failures to correct action. It does not override protocols, signed packets, runtime truth, or Codex law.

## Action Cards

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

### RAM-SHARED-STARTUP_TIMEOUT-001

- ACTION: TOOLCALLING
- TRIGGER: running a role `*-startup` command from Codex shell tooling
- FAILURE_PATTERN: using the default shell timeout for startup and truncating authority, checks, memory refresh, or resume recall before the role is ready
- DO: set the shell timeout to at least `600000` ms / 10 minutes for role startup commands; if startup still times out under host load, capture the timeout as procedural memory and rerun with an increased timeout before acting on partial context
- DO_NOT: treat a shell timeout during startup as completed startup or begin governed work from incomplete startup output
- VERIFY: startup reaches its normal checkpoint/resume hint output and any required hard gates have completed or produced explicit failures
- SOURCE: memory-capture #5897, Operator correction 2026-05-03
