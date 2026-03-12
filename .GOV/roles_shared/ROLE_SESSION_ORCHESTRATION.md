# ROLE SESSION ORCHESTRATION

This file is the shared law for repo-governed multi-session launch behavior.

## Core Rule
- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions.
- Coder and Validator sessions may resume work, but they do not self-start a fresh repo-governed session.

## Primary Launch Path
- Preferred host: `VSCODE_EXTENSION_TERMINAL`
- Bridge: `handshake.handshake-session-bridge`
- Bridge command: `handshakeSessionBridge.processLaunchQueue`
- Launch queue: `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- Session registry: `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`

## Fallback Law
- Primary path is plugin-first.
- A CLI escalation window is allowed only after the same role/WP session has recorded 2 plugin failures or timeouts.
- Default escalation host: `WINDOWS_TERMINAL`
- Manual `PRINT` output is a repair/debug surface, not the preferred runtime.

## Wake-Up / Notice Protocol
- Primary wake channel: `VS_CODE_FILE_WATCH`
- Fallback wake channel: `WP_HEARTBEAT`
- The plugin bridge should watch:
  - `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
  - `.GOV/roles_shared/WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`
- Roles should not depend on blind continuous polling when a watch event exists.

## Deterministic State
- Launch requests are append-only JSONL records.
- The session registry is the current state projection for active and historical role sessions.
- Packet truth still wins over session state for scope, verdict, and acceptance.

## Session Model Policy
- Primary model: `gpt-5.4`
- Fallback model: `gpt-5.2`
- Reasoning strength: `EXTRA_HIGH`
- Launcher config: `model_reasoning_effort=xhigh`
- Codex model aliases are not allowed in new repo-governed claim fields.

## Operational Commands
- `just launch-coder-session WP-{ID}`
- `just launch-wp-validator-session WP-{ID}`
- `just launch-integration-validator-session WP-{ID}`
- `just session-registry-status [WP-{ID}]`
- `just operator-monitor`
