# GIT_TOPOLOGY_REGISTRY

This file is a deterministic governance registry for the permanent Handshake checkout topology.

- SCHEMA_VERSION: hsk.git_topology_registry@0.1
- CANONICAL_BRANCH: main
- PROTECTED_LOCAL_BRANCHES: main, user_ilja, role_orchestrator, role_validator
- PROTECTED_REMOTE_BRANCHES: origin/main, origin/user_ilja, origin/role_orchestrator, origin/role_validator

## PROTECTED_WORKTREES

| ID | ROLE | REL_PATH | LOCAL_BRANCH | REMOTE_BRANCH | CANONICAL | DESCRIPTION |
| --- | --- | --- | --- | --- | --- | --- |
| handshake_main | CANONICAL | ../handshake_main | main | origin/main | YES | Canonical integrated checkout on disk |
| wt-ilja | OPERATOR | ../wt-ilja | user_ilja | origin/user_ilja | NO | Operator backup worktree |
| wt-orchestrator | ORCHESTRATOR | ../wt-orchestrator | role_orchestrator | origin/role_orchestrator | NO | Orchestrator backup worktree |
| wt-validator | VALIDATOR | ../wt-validator | role_validator | origin/role_validator | NO | Validator backup worktree |

## HELPER_COMMANDS

- backup_snapshot: just backup-snapshot
- backup_status: just backup-status
- sync_all_role_worktrees: just sync-all-role-worktrees
- enumerate_cleanup_targets: just enumerate-cleanup-targets
- ensure_permanent_backup_branches: just ensure-permanent-backup-branches

## BACKUP_POLICY

- BACKUP_PUSH_BEFORE_DESTRUCTIVE_LOCAL_GIT: YES
- IMMUTABLE_SNAPSHOT_BEFORE_TOPOLOGY_DELETION: YES
- NAS_COPY_MODE: timestamped_copy_no_mirror_deletes
- BACKUP_ROOT_ENV_VAR: HANDSHAKE_BACKUP_ROOT
- NAS_BACKUP_ROOT_ENV_VAR: HANDSHAKE_NAS_BACKUP_ROOT

