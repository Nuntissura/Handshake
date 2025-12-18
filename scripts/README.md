Dev and ops scripts live here.

Scaffolding:
- `node scripts/new-react-component.mjs <ComponentName>` creates `app/src/components/<ComponentName>.tsx` and a basic test.
- `node scripts/new-api-endpoint.mjs <endpoint_name>` creates `src/backend/handshake_core/src/api/<endpoint_name>.rs` and wires it into `api/mod.rs`.
- `node scripts/scaffold-check.mjs` validates scaffolding output against a temporary workspace.

AI review:
- `node scripts/ai-review-gemini.mjs` runs the Gemini reviewer via local `gemini` CLI and writes `ai_review.json` and `ai_review.md`.
- If the CLI is not on PATH, set `GEMINI_CLI` to the full path (on Windows, `gemini.ps1`).

Git hooks:
- `scripts/hooks/pre-commit` runs `node scripts/ai-review-gemini.mjs` before commits.
- Enable with `git config core.hooksPath scripts/hooks`.
