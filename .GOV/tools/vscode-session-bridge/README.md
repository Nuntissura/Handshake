# Handshake Session Bridge

Thin VS Code extension bridge for Handshake repo-governed session orchestration.

## Scope
- Watch `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- Watch `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- Use `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/` as the per-command detail source behind steerable completion/cancel notices when the repo tooling requests it
- Create or reuse named integrated terminals
- Send the exact repo-governed Codex command into that terminal
- Record plugin acknowledgment or failure in `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- Surface validator wake-up notifications from `WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`
- Surface steerable session-command completion notices from `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- Stay secondary to the governance ACP bridge in `.GOV/tools/handshake-acp-bridge/`

## Non-Scope
- Do not invent launch policy
- Do not choose models
- Do not mutate packets
- Do not own merge or validator authority

## Why This Exists
- Official VS Code extension APIs can create terminals and send command text.
- The repo scripts remain the authority for launch policy.
- This extension is only the integrated-terminal transport and notification layer. Primary launch path is here; Primary steering lane runs through the governance ACP bridge.
- Only the Orchestrator starts fresh governed sessions. This bridge does not grant Coder or Validator sessions independent start or steering authority.

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
- Steering notice ledger: `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- Steering detail log root: `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/`
- Preferred host: `VSCODE_EXTENSION_TERMINAL`
- Fallback law lives in repo scripts, not here.
- Wake/notice contract: launch queue + registry for bootstrap, `SESSION_CONTROL_RESULTS.jsonl` plus `SESSION_CONTROL_OUTPUTS/` for ACP steering notices, and packet runtime status for validator/operator wake-ups.
- The richer ACP-aware oversight surface is `just operator-monitor`; this extension is not the operator authority surface.

## Safety
- Only requests with `launch_authority=ORCHESTRATOR_ONLY` are accepted.
- Invalid requests are marked as plugin failures in the registry.
- Packet truth still wins over registry state.
