# REPO_RESILIENCE

This document defines the repo-resilience layer for Handshake governance.

## Goals

- Prevent branch/worktree deletions from becoming unrecoverable events.
- Preserve both committed git history and working-file snapshots outside the repo tree.
- Keep permanent topology deterministic across `handshake_main`, `wt-ilja`, `wt-orchestrator`, and `wt-validator`.

## Commands

- `just topology-registry-sync`
- `just backup-snapshot [label] [out_root] [nas_root]`
- `just sync-all-role-worktrees`
- `just enumerate-cleanup-targets`
- `just ensure-permanent-backup-branches`

## Policy

- `main` is the only canonical integrated branch.
- `user_ilja`, `role_orchestrator`, and `role_validator` are backup branches on GitHub.
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Backup snapshots do two things:
  - create git bundles for committed refs
  - copy current worktree files outside the repo tree so dirty state survives deletion incidents

## Backup Targets

Drive-agnostic rule: do not hardcode machine-local paths in committed governance files.

Use environment variables or explicit command arguments:

- `HANDSHAKE_BACKUP_ROOT`
  - local out-of-repo snapshot root
  - default when unset: sibling directory `../Handshake Backups` next to `Handshake Worktrees`
- `HANDSHAKE_NAS_BACKUP_ROOT`
  - optional NAS destination
  - when set, snapshots are copied there as timestamped directories using `robocopy`
  - copy mode is additive timestamped copy, not destructive mirror delete

## Snapshot Contents

Each snapshot includes:

- `bundles/all_refs.bundle`
- `bundles/protected_branches.bundle`
- `worktrees/<checkout-id>/...` copied working files
- `manifests/repo_resilience_manifest.json`
- `manifests/git_topology_registry.json`
- `manifests/git_topology_snapshot.json`
- `manifests/restore_instructions.txt`

## Restore Model

- Use git bundles to recover committed refs.
- Use copied worktree files to recover dirty/uncommitted state.
- Treat NAS copies as disaster-recovery replicas, not as live working directories.

## Server-Side Protection

GitHub branch protection remains recommended for:

- `main`
- `user_ilja`
- `role_orchestrator`
- `role_validator`

This repo currently relies on local governance plus backup branches until GitHub auth/admin rights are available to apply server-side branch protection.
