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
| Shared coder/WP-validator worktree is dirty or invalid but the branch must be repaired to packet truth | governed cleanup/reseed path is required; manual reuse risks stale evidence or mixed scope | Snapshot first, use `just delete-local-worktree <worktree_id> "<approval>"` with the governed flags/helpers, then recreate from the packet baseline instead of hand-cleaning or reusing the dirty worktree |

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
