# REPO_RESILIENCE

This document defines the repo-resilience layer for Handshake governance.

## Goals

- Prevent branch/worktree deletions from becoming unrecoverable events.
- Preserve both committed git history and working-file snapshots outside the repo tree.
- Keep permanent topology deterministic across `handshake_main`, `wt-ilja`, `wt-orchestrator`, `wt-validator`, and `wt-gov-kernel`.
- Keep offline backups safe from mass-deletion sync by using append-only timestamped snapshots instead of destructive mirrors.

## Commands

- `just topology-registry-sync`
- `just backup-snapshot [label] [out_root] [nas_root]`
- `just backup-status`
- `just backup-snapshot-nas [label]`
- `just sync-all-role-worktrees`
- `just sync-gov-to-main`
- `just enumerate-cleanup-targets`
- `just delete-local-worktree <worktree_id> "<approval>"`
- `just generate-worktree-cleanup-script WP-{ID} CODER|WP_VALIDATOR`
- `just ensure-permanent-backup-branches`

## Policy

- `main` is the only canonical integrated branch.
- `user_ilja`, `role_orchestrator`, `role_validator`, and `gov_kernel` are backup branches on GitHub.
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Worktree deletion must go through `just delete-local-worktree`. Never fall back to `Remove-Item`, `rm`, `del`, or other direct filesystem deletion for worktree paths.
- For orchestrator-managed WP closeout, prefer the generated single-target cleanup script flow:
  - Orchestrator generates the script for the exact CODER or WP_VALIDATOR worktree.
  - The cleanup token is stored in that worktree's git admin dir so the working tree stays clean.
  - The generated script requires both the exact Operator approval text and the matching token, then delegates to the hardened `delete-local-worktree` path.
  - The generated script may only remove the local worktree. Remote backup branch deletion stays separate.
- If `git worktree remove` fails, STOP. Treat that as abnormal repo state, not as permission to continue cleanup manually.
- Role startup should surface `just backup-status` so the assistant can see whether local/NAS backup roots are configured and whether recent immutable snapshots exist.
- Backup snapshots do two things:
  - create git bundles for committed refs
  - copy current worktree files outside the repo tree so dirty state survives deletion incidents
- Backup storage is append-only by default. Each run writes a new timestamped directory and must never mirror-delete older snapshots.
- Live mirrors are allowed for convenience, but they are not disaster recovery. Immutable snapshots are the authoritative recovery layer.

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
  - example operator-provided Handshake NAS root: `\\MIR\home\Backups\project folder backup\Handshake back up` (configure locally; do not reuse blindly for other projects)

## Exact Workflow

1. Keep working repos and worktrees on their normal disks.
2. Keep the backup root outside the repo tree.
3. Run `just backup-snapshot <label>` regularly and before topology deletion or broad cleanup.
4. Run `just backup-status` during role startup or before risky topology work to confirm backup roots and latest snapshots are visible.
5. When `HANDSHAKE_NAS_BACKUP_ROOT` is configured, copy the entire timestamped snapshot directory to the NAS as a second location.
6. Never use a destructive mirror sync against the backup roots.
7. Keep backup cleanup as a separate operator-reviewed action.

## Folder Layout

Backup root layout:

```text
<backup-root>/
  OFFLINE_GIT_BACKUP_SETUP.md
  <timestamp>-<label>/
    bundles/
    worktrees/
    manifests/
```

Each NAS or local backup root receives the reusable `OFFLINE_GIT_BACKUP_SETUP.md` guide so the pattern can be copied to other projects.

## Retention

Recommended default retention:

- dailies: 14 days
- weeklies: 8 weeks
- monthlies: 12 months
- pre-cleanup / pre-topology-change snapshots: keep until manually reviewed

Retention cleanup must be a separate reviewed task. Do not delete old snapshots as part of the snapshot job itself.

## Deletion Quarantine

Mass deletion usually propagates through live sync, not through immutable snapshots.

Rules:

- do not auto-prune snapshot roots
- do not use `robocopy /MIR` or equivalent destructive mirror deletion for backup storage
- do not treat a bare mirror as the only backup
- preserve the last known good snapshot before any topology cleanup

## Restore Model

- Recover committed refs from `bundles/all_refs.bundle`
- Recover protected-branch history quickly from `bundles/protected_branches.bundle`
- Recover dirty state from copied `worktrees/<checkout-id>/...`
- Use `manifests/restore_instructions.txt` and `.GOV/roles_shared/docs/OFFLINE_GIT_BACKUP_SETUP.md` as the operator playbook

## Reusable Guide

The reusable setup guide lives at:

- `.GOV/roles_shared/docs/OFFLINE_GIT_BACKUP_SETUP.md`

The snapshot script copies this guide into the backup root and NAS root so the same pattern can be reused for other projects without reopening the Handshake repo.

## Server-Side Protection

GitHub branch protection remains recommended for:

- `main`
- `user_ilja`
- `role_orchestrator`
- `role_validator`
- `gov_kernel`
