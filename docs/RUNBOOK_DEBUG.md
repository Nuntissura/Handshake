# RUNBOOK_DEBUG

## First 5 minutes
- Repro fast: `pnpm -C app tauri dev` (frontend + Tauri) and keep terminal output visible; note console errors.
- Check backend health while reproing: `cargo run --bin handshake_core` (or rely on the Tauri spawn) and watch `data/logs/handshake_core.log`.
- Confirm branch/spec alignment: skim `docs/SPEC_CURRENT.md` for the exact feature expectation before changing code.
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

## If you only remember one thing
- Use `rg "<feature or error string>" app/src src/backend/handshake_core` to jump to the owning layer, then open the matching file and cross-check the expected behavior in `docs/SPEC_CURRENT.md`.
- When adding new repeatable errors, assign a code/tag like `HSK-####` and note it here with the primary entrypoint to triage.

## Debugging a failed CI check
- codex-check: run `just codex-check` and inspect outputs for forbidden `fetch(`, `println!/eprintln!`, or doc drift.
- depcruise: run `pnpm -C app run depcruise` to see layer violations.
- cargo-deny: run `cargo deny check advisories licenses bans sources` (install via `cargo install cargo-deny` if needed).
- gitleaks: rerun in CI or locally with `gitleaks detect --source .` if installed.
- todo-policy: `rg -n --pcre2 "TODO(?!\\(HSK-\\d+\\))" app/src src/backend scripts` to find non-tagged TODOs.
- ai-review: run `just ai-review` locally with the `gemini` CLI and attach `ai_review.md` to the task packet/logger.
