# Orchestrator Restart Smoke Test Note

Date: 2026-03-12
Audience: Orchestrator session after VS Code Insiders restart
Purpose: Resume the plugin-first live smoke test without re-discovering workflow state

## Current Repo State

- Repo/worktree: `D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-orchestrator`
- Current branch before restart: `role_orchestrator`
- Repo status before restart: clean
- Governance verification before restart: `just gov-check` PASS

## What Changed In Governance

The repo now uses plugin-first session orchestration for newly created repo-governed sessions.

Key rules:

- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions.
- Primary host is `VSCODE_EXTENSION_TERMINAL`.
- Launch requests go to `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`.
- Current launch/runtime projection goes to `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`.
- CLI escalation is allowed only after 2 plugin failures or timeouts for the same role/WP session, unless the Operator explicitly waives plugin-first.
- Repo-governed model policy is:
  - primary `gpt-5.4`
  - fallback `gpt-5.2`
  - reasoning `EXTRA_HIGH`
  - launcher config `model_reasoning_effort=xhigh`

Reference files:

- `AGENTS.md`
- `.GOV/roles_shared/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/scripts/session-policy.mjs`
- `.GOV/scripts/launch-cli-session.mjs`
- `.GOV/tools/vscode-session-bridge/extension.js`
- `.GOV/docs/vscode-session-bridge/VSCODE_SESSION_BRIDGE_ARCHITECTURE.md`

## VS Code Bridge Status

Installed for VS Code Insiders only:

- `C:\Users\Ilja Smets\.vscode-insiders\extensions\handshake.handshake-session-bridge-0.0.1`

Expected after restart:

- the extension should activate on startup or when this repo workspace is opened
- the following commands should exist in the Command Palette:
  - `Handshake: Process Session Launch Queue`
  - `Handshake: Open Session Registry`

If the extension does not appear after restart, treat that as a bridge failure and diagnose before using a CLI escalation window.

## Active WP Ready For Live Smoke Test

Packet:

- `.GOV/task_packets/WP-1-Structured-Collaboration-Artifact-Family-v1.md`

Communications:

- `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/THREAD.md`
- `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/RUNTIME_STATUS.json`
- `.GOV/roles_shared/WP_COMMUNICATIONS/WP-1-Structured-Collaboration-Artifact-Family-v1/RECEIPTS.jsonl`

WP status at note time:

- workflow lane: `ORCHESTRATOR_MANAGED`
- execution owner: `CODER_A`
- packet status: `Ready for Dev`
- worktree: `../wt-WP-1-Structured-Collaboration-Artifact-Family-v1`
- branch: `feat/WP-1-Structured-Collaboration-Artifact-Family-v1`
- technical advisor: `WP_VALIDATOR`
- technical authority: `INTEGRATION_VALIDATOR`
- merge authority: `INTEGRATION_VALIDATOR`

Current runtime projection:

- `runtime_status`: `submitted`
- `current_phase`: `BOOTSTRAP`
- `next_expected_actor`: `ORCHESTRATOR`
- `waiting_on`: `INSTRUCTION`

## First Actions After Restart

1. Open this repo in VS Code Insiders.
2. Confirm the bridge is loaded.
3. Confirm repo state:
   - `just orchestrator-startup`
   - `just gov-check`
   - `just session-registry-status WP-1-Structured-Collaboration-Artifact-Family-v1`
4. Start the live smoke test by launching the coder session through the governed path:
   - `just launch-coder-session WP-1-Structured-Collaboration-Artifact-Family-v1`
5. Confirm the launch request is queued/consumed:
   - inspect `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
   - run `just session-registry-status WP-1-Structured-Collaboration-Artifact-Family-v1`
6. If the coder launch succeeds, optionally launch the WP Validator session next:
   - `just launch-wp-validator-session WP-1-Structured-Collaboration-Artifact-Family-v1`

## Important Caveat

The active WP was created before the final `2026-03-12` packet/stub template boundary landed, so its packet metadata still shows an older `PACKET_FORMAT_VERSION`. That does not block the smoke test. The governing launch/session machinery is now active in the repo, and this WP is the current ready target for testing the new live orchestration path.

## Do Not Forget

- Orchestrator may launch sessions, but does not gain merge authority.
- Integration Validator remains the only final merge-to-main authority.
- Packet truth still wins if packet state and runtime/session projection disagree.
