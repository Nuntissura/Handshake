# REPO_RESILIENCE

This document defines the repo-resilience layer for Handshake governance.

## Goals

- Prevent branch/worktree deletions from becoming unrecoverable events.
- Preserve both committed git history and working-file snapshots outside the repo tree.
- Keep permanent topology deterministic across `handshake_main`, `wt-ilja`, and `wt-gov-kernel`.
- Keep offline backups safe from mass-deletion sync by using append-only timestamped snapshots instead of destructive mirrors.
- Keep `../Handshake_Artifacts/` bounded by governed cleanup and retention manifests instead of ad hoc manual deletion.

## Commands

Removed 2026-09-23: the command surface was deleted with the governance harness.

## Policy

- `main` is the only canonical integrated branch.
- `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json` is the single authoritative topology ledger. The permanent checkout layout and helper-command contract live under `git_topology_contract`; `GIT_TOPOLOGY_REGISTRY.md/json` are deprecated, non-authoritative compatibility references.
- Post-`RGF-75` evaluation outcome: no separate stable product integration branch is required while `main` remains clean, repo-local artifact leakage stays blocked, and governed worktree hygiene continues to pass.
- `user_ilja` and `gov_kernel` are backup branches on GitHub.
- Permanent non-main worktrees (`wt-ilja`, `wt-gov-kernel`) inherit product code and root-level LLM files from local `main`. Their matching GitHub branches are safety copies, not the refresh source for that base.
- Permanent non-main worktrees with a live `.GOV` kernel junction must suppress `.GOV` git noise locally. The supported model is worktree-local git metadata: add `.GOV/` to that worktree's `info/exclude` for untracked kernel files and mark tracked `.GOV` paths `skip-worktree`. Do not rely on the shared repo `.gitignore` to hide tracked `.GOV` drift.
- Before deleting local branches/worktrees or performing broad topology cleanup, make an out-of-repo backup copy first.
- Worktree deletion must go through `git worktree remove`. Never fall back to `Remove-Item`, `rm`, `del`, or other direct filesystem deletion for worktree paths.
- If `git worktree remove` fails, STOP. Treat that as abnormal repo state, not as permission to continue cleanup manually.
- Backup snapshots do two things:
  - create git bundles for committed refs
  - copy current worktree files outside the repo tree so dirty state survives deletion incidents
- Artifact retention is governed separately from snapshots:
  - canonical roots under `../Handshake_Artifacts/` stay durable
  - cleanup may remove only reclaimable residue
  - every governed artifact cleanup or integration-validator closeout writes a retention manifest under `handshake-tool/artifact-retention/`
  - authority: `.GOV/roles_shared/docs/ARTIFACT_RETENTION_POLICY.md`
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
3. Make an out-of-repo backup copy regularly and before topology deletion or broad cleanup.
4. When `HANDSHAKE_NAS_BACKUP_ROOT` is configured, copy the entire timestamped snapshot directory to the NAS as a second location.
5. Never use a destructive mirror sync against the backup roots.
6. Keep backup cleanup as a separate operator-reviewed action.

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

## Server-Side Protection

GitHub branch protection remains recommended for:

- `main`
- `user_ilja`
- `gov_kernel`
