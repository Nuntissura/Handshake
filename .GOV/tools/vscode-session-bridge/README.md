# Handshake Session Bridge

Thin VS Code extension bridge for Handshake repo-governed session orchestration.

## Scope
- Watch `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- Create or reuse named integrated terminals
- Send the exact repo-governed Codex command into that terminal
- Record plugin acknowledgment or failure in `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- Surface validator wake-up notifications from `WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`

## Non-Scope
- Do not invent launch policy
- Do not choose models
- Do not mutate packets
- Do not own merge or validator authority

## Why This Exists
- Official VS Code extension APIs can create terminals and send command text.
- The repo scripts remain the authority for launch policy.
- This extension is only the integrated-terminal transport layer.

## Install / Run
1. Open this repo in VS Code.
2. Open `.GOV/tools/vscode-session-bridge/`.
3. Use the standard VS Code extension development flow to run the extension in an Extension Development Host.
4. Once active, the bridge will process queued launch requests automatically and you may also run:
   - `Handshake: Process Session Launch Queue`
   - `Handshake: Open Session Registry`

## Runtime Contract
- Queue file: `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- Registry file: `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- Preferred host: `VSCODE_EXTENSION_TERMINAL`
- Fallback law lives in repo scripts, not here.

## Safety
- Only requests with `launch_authority=ORCHESTRATOR_ONLY` are accepted.
- Invalid requests are marked as plugin failures in the registry.
- Packet truth still wins over registry state.
