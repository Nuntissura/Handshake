# RUNBOOK_DEBUG

## First 5 minutes
> **WARNING for AI Agents:** Commands like `pnpm -C app tauri dev` or `just dev` start a long-running development server. They MUST NOT be executed with a blocking tool (like `run_shell_command`). These commands should be run in a separate, dedicated terminal by the user or as a true background process.
- Repro fast: `pnpm -C app tauri dev` (frontend + Tauri) and keep terminal output visible; note console errors.
- Check backend health while reproing: `cargo run --bin handshake_core` (or rely on the Tauri spawn) and watch `data/logs/handshake_core.log`.
- Confirm branch/spec alignment: skim `.GOV/spec/SPEC_CURRENT.md` for the exact feature expectation before changing code.
- Isolate layer: decide if the failure is UI, IPC, backend, or data; jump to the matching section below.
- Run the smallest relevant test: `pnpm -C app test <pattern>` for UI, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` for backend.

## Logs and verbosity
- Backend logs: `data/logs/handshake_core.log` (JSON via `tracing_subscriber`). Set `HS_LOG_LEVEL=debug` to increase verbosity; default is `info`.
- Frontend/Tauri: stdout from `pnpm -C app tauri dev`; use browser devtools console for React logs.
- Historical investigation: `Handshake_logger_*` in repo root and `log_archive/` capture prior runs/decisions.

## Common symptom -> where to look
| Symptom | Where to look | Search terms / commands |
| --- | --- | --- |
| UI not rendering / blank window | `app/src/` components & routing | `rg "App" app/src`, `pnpm -C app test` |
| Button/interaction does nothing | `app/src/` handler, Tauri invoke wiring in `app/src-tauri/src/lib.rs` | `rg "invoke" app/src app/src-tauri/src/lib.rs` |
| Backend API error / panic | `src/backend/handshake_core/src/api/*.rs`, `models.rs`, `logging.rs` | `rg "Result<" src/backend/handshake_core/src/api`, check `data/logs/handshake_core.log` |
| IPC/bridge issues (frontend <-> backend) | Tauri orchestrator spawn in `app/src-tauri/src/lib.rs`, backend entry `src/backend/handshake_core/src/main.rs` | `rg "Command::new(\"cargo\")" app/src-tauri/src/lib.rs`, `rg "@tauri" app/src` |
| Data/migration problems | `src/backend/handshake_core/migrations/`, database path under `data/` | `rg "migration" src/backend/handshake_core`, inspect schema diffs |
| Build/test fails | `justfile`, package configs (`app/package.json`, Rust `Cargo.toml`) | Re-run `pnpm -C app test`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` |
| Worktree removal "Filename too long" | Windows MAX_PATH (260 char) limit on deeply nested paths | See **Windows long-path recovery** section below |
| Governance file edit does not appear in `git status` | worktree-local topology and ignore rules | `git ls-files <path>`, `git check-ignore -v <path>`, inspect `.git/info/exclude`, confirm the current worktree/branch before changing public command surfaces |
| `just` wrapper fails before Node sees the arguments | PowerShell metacharacter parsing in variadic flags | Prefer the governed wrappers that use `node-argv-proxy.mjs`; reproduce with the exact `just ... --decisions "..."` or `--metadata '{...}'` text, then inspect the wrapper recipe instead of the downstream Node script first |
| `WORKFLOW_INVALIDITY` says a declared `../handshake_main/...` path is out of scope in a WP worktree | scope comparison drift between packet repo-relative aliases and product-worktree relative paths | Check shared scope classification first; compare the packet `IN_SCOPE_PATHS` alias against the worktree-relative path before widening signed scope or rewriting the packet |
| `wp-receipt-append ... REPAIR` times out but the lane looks like it may have resumed | receipt append wrote the repair and inline auto-relay before the shell boundary timed out | Inspect the WP `RECEIPTS.jsonl`, runtime `RUNTIME_STATUS.json`, and `just session-registry-status WP-{ID}` before retrying; do not assume the timeout means the repair failed |
| `gov-check`, bundled check helpers, or dossier sync appears to fail only because the host is saturated | heavy host load delayed a child process or shell boundary, but the underlying sub-checks may still be healthy | Re-run the targeted sub-checks, inspect emitted artifacts/results directly, and treat the timeout itself as telemetry unless the underlying authority surface actually disagrees |
| Auto-relay stopped retrying and the lane is waiting on Orchestrator repair | runtime-native relay escalation policy spent its budget or shifted strategy away from same-method retry | Check `just session-registry-status WP-{ID}` or `just wp-lane-health WP-{ID}` for `failure_class`, `policy_state`, `next_strategy`, and strategy budget; if the policy says `AUTO_RETRY_BLOCKED`, do not keep re-waking the same path blindly |
| Operator surfaces show a push alert or the lane looks stalled without an obvious terminal error | broker/runtime telemetry now distinguishes run-level state from step-level activity, and the durable alert is the authoritative wake-up clue | Check `just session-registry-status WP-{ID}`, `just wp-lane-health WP-{ID}`, or the operator monitor for `run_telemetry`, `step_telemetry`, and the latest push alert before reopening a role terminal; treat the push alert summary as the first diagnostic surface |
| `gov-flush` stops immediately on artifact-root drift | one or more discovered worktrees have a stale Cargo `target-dir` or a repo-local `target/` directory, so cleanup and NAS backup would be guaranteed to fail later anyway | Run `just artifact-root-preflight` first for the current environment blocker class, then `just artifact-hygiene-check` for deeper cleanup diagnosis. Fix each `.cargo/config.toml` so Cargo resolves to the canonical external artifact root, remove repo-local `target/` residue if present, then rerun `just gov-flush` |
| `orchestrator-next` reports cost governor `WARN`, `RECOVERY_MODE`, or `OVERRIDE_REQUIRED` | the Orchestrator crossed token/command/elapsed/repair-loop budgets and broad rediscovery would waste more context | Run `just wp-truth-bundle WP-{ID}` and act only on the compact next legal command. For `OVERRIDE_REQUIRED`, another steer needs `just orchestrator-steer-next WP-{ID} "<context>" --override-recovery=<operator reason>` |
| A role repeats the same captured procedural mistake despite memory entries | passive memory is not being compiled into startup behavior or deterministic tooling | Run `just role-startup-brief <ROLE>` to inspect current anti-repeat cards, then launch intelligent Memory Manager with `just launch-memory-manager-session` or add a deterministic tooling repair if the failure is mechanical |
| Memory Manager output is treated as approval or gets ignored after hygiene | Memory Manager proposes and orders memory, but coordinator authority must review non-brief governance changes | Inspect the `MEMORY_PROPOSAL`, `MEMORY_FLAG`, or `MEMORY_RGF_CANDIDATE` receipt plus backup proposal. Orchestrator handles `ORCHESTRATOR_MANAGED`; Classic Orchestrator handles `MANUAL_RELAY`. Record accept/reject/defer before editing governance. |
| `PACKET_ACCEPTANCE_MATRIX` blocks PASS closure | one or more required executable acceptance rows are unresolved or lack evidence/reason | Update the packet row status to `PROVED`, `CONFIRMED`, or `NOT_APPLICABLE` only with concrete evidence or reason. Leave unresolved work as `STEER` or `BLOCKED`; do not replace the matrix with narrative PASS language |
| `workflow-dossier-judgment-check` reports placeholders or narrative contradiction | the dossier's judgment/rubric sections lag terminal truth even though mechanical telemetry may be current | Repair the judgment/rubric text or record the debt as diagnostic governance debt. Do not reopen product validation unless the contradiction proves product evidence is wrong |
| `post-work-check` reports out-of-scope drift caused by a broken baseline compile/environment | the coder needs a path-limited exception for baseline repair, but prose waiver authority is insufficient | Orchestrator/Operator records the exception with `just wp-waiver-record WP-{ID} --blocker-command <cmd> --allowed-edit-paths <paths> --operator-authority-ref <ref> ...`; rerun the coder handoff gate after the ledger exists |
| Shared coder/WP-validator worktree is dirty or invalid but the branch must be repaired to packet truth | governed cleanup/reseed path is required; manual reuse risks stale evidence or mixed scope | Snapshot first, use `just delete-local-worktree <worktree_id> "<approval>"` with the governed flags/helpers, then recreate from the packet baseline instead of hand-cleaning or reusing the dirty worktree |
| `phase-check CLOSEOUT` or context status shows a real `verdict_of_record`, but the lane still looks "not settled" | closeout support surfaces still carry settlement debt even though product judgment is already known | Check `just phase-check CLOSEOUT WP-{ID}`, `just integration-validator-context-brief WP-{ID}`, and `just session-registry-status WP-{ID}` for `product_outcome_blockers`, `governance_debt`, and closeout provenance. If `product_outcome_blockers` is empty, repair only the named settlement debt; do not rerun validation just to restate the same verdict |
| `phase-check CLOSEOUT` reports `DIAGNOSTIC_DEBT` for Workflow Dossier sync/import, or a dossier has malformed/duplicate live sections | the dossier is diagnostic evidence and its append/import lane did not fully settle | Inspect `workflow-dossier-inject-repomem`, `workflow-dossier-sync`, and the active dossier sections `LIVE_ORCHESTRATOR_DIAGNOSTIC_LOG`, `LIVE_ACP_SESSION_TRACE`, and `CLOSEOUT_REPOMEM_IMPORT`. Rerun the specific dossier helper if useful, but do not block product outcome or rerun validation only to repair dossier formatting/import debt |

### Windows long-path recovery

`Filename too long` / `ENAMETOOLONG` is a common Windows error when worktrees contain deeply nested `node_modules`, `target/`, or `.GOV/` paths exceeding 260 characters. `delete-local-worktree.mjs` already passes `core.longpaths=true` to git but the Windows filesystem may still refuse.

**Do NOT force-delete or use rm -rf.** Recovery options:

1. **Enable long paths system-wide (preferred, one-time):**
   ```
   reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f
   ```
   Restart the shell, then retry the governance delete script.

2. **Use robocopy /mir to clear the deep tree (per-incident):**
   ```
   mkdir empty_dir
   robocopy empty_dir "<stuck-worktree-path>" /mir /r:1 /w:0
   rmdir empty_dir
   rmdir "<stuck-worktree-path>"
   ```
   Then run `git worktree prune` to clean git's worktree list.

3. **Use `\\?\` prefix paths** in PowerShell/cmd for targeted file operations.

After any manual filesystem recovery, always run `git worktree prune` and `just gov-check` to verify consistency.

## Workflow invalidity and governed repair

- Treat workflow invalidity as a control-plane truth mismatch first, not an automatic packet-widening event.
- When scope invalidity appears after a packet was signed, check shared path aliasing and baseline drift before editing packet scope or steering the coder wider.
- When a coder/WP-validator shared worktree must be repaired, prefer: immutable snapshot -> governed delete/cleanup -> recreate from packet `MERGE_BASE_SHA` -> append `REPAIR` receipt -> verify runtime/session truth before any new steer.

## If you only remember one thing
- Use `rg "<feature or error string>" app/src src/backend/handshake_core` to jump to the owning layer, then open the matching file and cross-check the expected behavior in `.GOV/spec/SPEC_CURRENT.md`.
- When adding new repeatable errors, assign a code/tag like `HSK-####` and note it here with the primary entrypoint to triage.

## Debugging a failed CI check
- product boundary scan: run `just product-scan` and inspect outputs for forbidden product-side patterns or repo-governance boundary drift.
- depcruise: run `pnpm -C app run depcruise` to see layer violations.
- cargo-deny: run `cargo deny check advisories licenses bans sources` (install via `cargo install cargo-deny` if needed).
- gitleaks: rerun in CI or locally with `gitleaks detect --source .` if installed.
- todo-policy: `rg -n --pcre2 "TODO(?!\\(HSK-\\d+\\))" app/src src/backend scripts` to find non-tagged TODOs.
