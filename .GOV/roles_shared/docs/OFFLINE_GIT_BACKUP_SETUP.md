# OFFLINE_GIT_BACKUP_SETUP

This document describes a deletion-safe offline Git backup pattern that can be reused for Handshake and other projects.

## Goal

Protect a project from:

- accidental branch deletion
- accidental worktree deletion
- bad sync jobs that propagate deletions
- force-push history loss
- disk failure
- network or GitHub outage

## Core Rule

Do not rely on a single live mirror.

A live mirror is useful for speed, but it can also replicate mistakes. The safe design always has three separate layers:

1. mutable mirror for fast Git recovery
2. immutable timestamped snapshots for disaster recovery
3. optional self-hosted Git service for an offline GitHub-like UI

## Recommended Layout

For any project, keep backup data outside the working repo tree.

Example layout:

```text
<Project Workspace>/
  <project-worktrees>/
  <project-backups>/
    OFFLINE_GIT_BACKUP_SETUP.md
    20260308-040000Z-daily/
    20260308-120000Z-manual/
```

Inside each timestamped snapshot:

```text
<timestamp>-<label>/
  bundles/
    all_refs.bundle
    protected_branches.bundle
  worktrees/
    <checkout-id>/
  manifests/
    repo_resilience_manifest.json
    git_topology_registry.json
    git_topology_snapshot.json
    restore_instructions.txt
```

## Required Layers

### 1. Immutable snapshots

Every run must create a new timestamped folder.

Rules:

- never overwrite the previous snapshot
- never prune automatically during the backup run
- never use mirror-delete behavior against the backup root
- keep both committed Git state and copied working files

This is the primary protection against mass deletion sync.

### 2. Bare Git mirror

Create a bare mirror outside the worktree:

```powershell
git clone --mirror <repo-url> <mirror-path>
git -C <mirror-path> remote update
```

Rules:

- use it for quick Git recovery
- do not treat it as the only backup
- do not auto-prune if you want a deletion recovery window

### 3. Optional offline GitHub-like service

Use Forgejo or Gitea on a NAS or small server if you want:

- branch browsing
- issue tracking
- pull requests
- protected branches
- local-only access when GitHub is unavailable

Use it as a secondary remote, not as the only backup target.

## Windows + NAS Setup Pattern

1. Pick a local backup root on a different disk from the working repo if possible.
2. Pick a NAS root for timestamped copies.
3. Store both roots outside the repo tree.
4. Set project-specific environment variables for the backup script.
5. Schedule recurring snapshots.
6. Test restore regularly.

Example environment variables:

```powershell
$env:PROJECT_BACKUP_ROOT = 'D:\Project Backups' # example
$env:PROJECT_NAS_BACKUP_ROOT = '\\NAS\share\Project Backups' # example
```

Persist them for the current user if desired:

```powershell
[Environment]::SetEnvironmentVariable('PROJECT_BACKUP_ROOT', 'D:\Project Backups', 'User') # example
[Environment]::SetEnvironmentVariable('PROJECT_NAS_BACKUP_ROOT', '\\NAS\share\Project Backups', 'User') # example
```

## Scheduling Pattern

Recommended cadence:

- local automated snapshot every 4 hours
- daily automated NAS snapshot
- extra manual snapshot before destructive cleanup or topology changes
- weekly restore drill

Windows Task Scheduler is usually enough.

Pattern:

1. run the snapshot command
2. verify the new timestamped directory exists
3. optionally write a simple log entry

Recommended command split:

- local recurring job:
  - `just backup-snapshot autosnap`
- NAS recurring job:
  - `just backup-snapshot-nas daily`
- visibility/status check:
  - `just backup-status`

The status command should be surfaced during role startup or preflight so humans and agents can see whether backup roots are configured and whether recent snapshots exist. This is safety context only; it does not authorize destructive actions by itself.

## Startup Awareness Pattern

If a project uses role-based agents or strict repo workflows, make backup existence visible at startup.

Minimum pattern:

1. keep the backup implementation in shared scripts/docs, not copied into every role doc in full
2. surface a short status command during startup or preflight
3. show only:
   - local backup configured or not
   - NAS backup configured or not
   - latest snapshot name or `NONE`
4. do not print machine-specific NAS paths in every protocol unless truly required
5. make clear that status visibility is not a substitute for approval gates

## Retention Policy

Start simple:

- keep daily snapshots for 14 days
- keep weekly snapshots for 8 weeks
- keep monthly snapshots for 12 months
- keep pre-cleanup / pre-topology-change snapshots until manually reviewed

Do not delete snapshots during the same job that creates them.

## Deletion Quarantine Policy

Never let upstream deletion immediately delete backup data.

Rules:

- no destructive mirror sync to the backup root
- no automatic `git remote prune` on backup stores
- no automatic deletion of old backup refs during the snapshot job
- if cleanup is needed, do it as a separate reviewed task

If a branch disappears upstream:

1. preserve the last known backup snapshot
2. preserve the last bundle containing that ref
3. keep the branch in the mirror or archive area until the retention window expires

## Restore Playbook

### Recover committed history

```powershell
git clone <repo-url-or-empty-dir> restored-project
git -C restored-project fetch <path-to-all_refs.bundle> "refs/*:refs/*"
```

### Recover dirty/uncommitted files

Copy from the timestamped `worktrees/<checkout-id>/` folder into a safe inspection directory first.

Do not overwrite a live repo blindly. Compare before restoring.

### Recover after mass deletion

1. find the latest good timestamped snapshot
2. restore committed refs from the bundle
3. restore working files from the copied worktree snapshot
4. only after verification, recreate active branches/worktrees

## Adapt This For Another Project

For another project, keep the same model and only change:

- backup root
- NAS root
- protected branch names
- worktree inventory
- scheduling cadence
- project-specific restore notes

Minimal checklist:

- [ ] snapshot command writes timestamped folders
- [ ] snapshot includes `git bundle` archives
- [ ] snapshot includes copied working files
- [ ] backup root is outside the repo
- [ ] NAS copy is additive, not mirror-delete
- [ ] protected branches are documented
- [ ] restore steps are documented
- [ ] restore drill is scheduled

Recommended implementation checklist for another project:

1. Create a shared resilience doc in the repo, for example `docs/REPO_RESILIENCE.md` or `.GOV/roles_shared/docs/REPO_RESILIENCE.md`.
2. Create a reusable setup guide like this file and copy it into the backup root after every snapshot run.
3. Add a script that creates:
   - git bundles for refs
   - copied worktree files outside the repo tree
   - snapshot manifests
4. Add a status script that reports:
   - local backup root configured or not
   - NAS backup root configured or not
   - latest snapshot presence
   - latest manifest presence
5. Add startup/preflight hooks so roles or maintainers see the backup status early.
6. Add scheduled tasks:
   - local recurring snapshots
   - nightly NAS snapshots
   - weekly restore drill or restore check
7. Keep retention cleanup separate from snapshot creation.
8. Keep backup roots outside the repo and outside live mirror-delete jobs.

## Example Windows Task Scheduler Micro-Steps

1. Open Task Scheduler.
2. Create a new task for local recurring snapshots.
3. Run it under the user account that has access to the repo and backup disks.
4. Action:
   - `powershell.exe`
   - arguments: `-NoLogo -NonInteractive -Command "cd '<repo-root>'; just backup-snapshot autosnap"`
5. Trigger:
   - repeat every 4 hours
6. Create a second task for nightly NAS snapshots.
7. Action:
   - `powershell.exe`
   - arguments: `-NoLogo -NonInteractive -Command "cd '<repo-root>'; just backup-snapshot-nas daily"`
8. Create a third optional task or checklist reminder for restore testing.
9. After creating each task, run it once manually and confirm a new timestamped snapshot exists.

## Handshake-Specific Note

Handshake uses a governance workflow with permanent protected branches and worktrees. Reuse the pattern, but do not assume another project has the same branch names or topology registry.
