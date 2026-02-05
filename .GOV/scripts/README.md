Dev and ops scripts live here.

Scaffolding:
- `node .GOV/scripts/new-react-component.mjs <ComponentName>` creates `app/src/components/<ComponentName>.tsx` and a basic test.
- `node .GOV/scripts/new-api-endpoint.mjs <endpoint_name>` creates `src/backend/handshake_core/src/api/<endpoint_name>.rs` and wires it into `api/mod.rs`.
- `node .GOV/scripts/scaffold-check.mjs` validates scaffolding output against a temporary workspace.

Git hooks:
- `.GOV/scripts/hooks/pre-commit` runs local hygiene checks before commits.
- Enable with `git config core.hooksPath .GOV/scripts/hooks`.

