Dev and ops scripts live here.

Scaffolding:
- `node .GOV/scripts/new-react-component.mjs <ComponentName>` creates `app/src/components/<ComponentName>.tsx` and a basic test.
- `node .GOV/scripts/new-api-endpoint.mjs <endpoint_name>` creates `src/backend/handshake_core/src/api/<endpoint_name>.rs` and wires it into `api/mod.rs`.
- `node .GOV/scripts/scaffold-check.mjs` validates scaffolding output against a temporary workspace.

Git hooks:
- `.GOV/scripts/hooks/pre-commit` runs local hygiene checks before commits.
- Enable with `git config core.hooksPath .GOV/scripts/hooks`.

Repo resilience:
- `node .GOV/scripts/topology-registry-sync.mjs` regenerates the deterministic permanent-checkout topology registry.
- `node .GOV/scripts/backup-snapshot.mjs --label manual` creates an out-of-repo snapshot with git bundles + copied working files.
- `node .GOV/scripts/backup-snapshot.mjs --label manual --nas-root "\\\\server\\share\\Project Backups"` copies the same timestamped snapshot to a NAS without mirror deletes.
- `node .GOV/scripts/sync-all-role-worktrees.mjs` fast-forwards the permanent local clones when all are clean.
- `node .GOV/scripts/enumerate-cleanup-targets.mjs` lists exact cleanup candidates and approval examples.
